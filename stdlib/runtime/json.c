// JSON: json_encode and json_decode.
//
// Both work from a type descriptor the compiler passes along with the value:
//   i int   d double   b bool   y byte   s string   c char   n null
//   a<element>   m<key><value>   o<class id>;
// and from qiq_class_json, generated per class: "<field count>|" then
// "<name>:<slot>:<descriptor>;" for each public property, in layout order.
//
// json_decode parses straight into the typed value, checking the JSON
// against the descriptor. Errors are returned up the parse (freeing what was
// built) and thrown as JsonException at the top, so bad input doesn't leak.
#include <errno.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "qiq_runtime.h"

extern const char *const qiq_class_json[];
// DateTime's formatter (time.c), or NULL when the program has no DateTimes;
// defined by the compiler.
extern char *(*const qiq_json_datetime_format)(const char *format, int64_t micros, const char *zone);
extern const char *qiq_class_names[];
// The box classes of ?int, ?double, ?bool, and ?byte, in that order.
extern const int64_t qiq_box_classes[];
// Per class, its enum cases' objects, null-terminated (NULL for others).
extern void *const *const qiq_enum_cases[];

// From map.c and memory.c.
void *qiq_map_new(uint32_t key_type, int32_t value_references);
void qiq_map_set(void *map, uint64_t key, uint64_t value);
void *qiq_object_new(int64_t field_count, int64_t class_id);

#define QIQ_JSON_MAX_DEPTH 512

// ---------------------------------------------------------------------------
// Descriptors

static const char *qiq_skip_descriptor(const char *descriptor) {
    switch (*descriptor) {
    case 'a':
        return qiq_skip_descriptor(descriptor + 1);
    case 'm':
        return qiq_skip_descriptor(qiq_skip_descriptor(descriptor + 1));
    case 'o':
        return strchr(descriptor, ';') + 1;
    case 'N':
        return descriptor + 2;
    default:
        return descriptor + 1;
    }
}

static bool qiq_is_reference_descriptor(char kind) {
    return kind == 's' || kind == 'c' || kind == 'a' || kind == 'm' || kind == 'o' || kind == 'N';
}

// Whether values of this type can be part of a cycle: it can hold an object.
static bool qiq_descriptor_may_cycle(const char *descriptor) {
    const char *end = qiq_skip_descriptor(descriptor);
    return memchr(descriptor, 'o', (size_t)(end - descriptor)) != NULL;
}

// Array and map flags for contents of this type (see qiq_array_create).
static int qiq_contents_flags(const char *descriptor) {
    return (qiq_is_reference_descriptor(*descriptor) ? 1 : 0) | (qiq_descriptor_may_cycle(descriptor) ? QIQ_MAY_CYCLE : 0);
}

static int64_t qiq_element_size(char kind) {
    return kind == 'b' || kind == 'y' ? 1 : 8;
}

static uint64_t qiq_load_element(const void *data, int64_t index, char kind) {
    if (kind == 'b' || kind == 'y') return ((const uint8_t *)data)[index];
    return ((const uint64_t *)data)[index];
}

// The type as written in qiq, for error messages.
static void qiq_descriptor_name(const char *descriptor, char *out, size_t size) {
    switch (*descriptor) {
    case 'i': snprintf(out, size, "int"); break;
    case 'd': snprintf(out, size, "double"); break;
    case 'b': snprintf(out, size, "bool"); break;
    case 'y': snprintf(out, size, "byte"); break;
    case 's': snprintf(out, size, "string"); break;
    case 'c': snprintf(out, size, "char"); break;
    case 'n': snprintf(out, size, "null"); break;
    case 'N': {
        char scalar[16];
        qiq_descriptor_name(descriptor + 1, scalar, sizeof scalar);
        snprintf(out, size, "?%s", scalar);
        break;
    }
    case 'a': {
        char element[128];
        qiq_descriptor_name(descriptor + 1, element, sizeof element);
        snprintf(out, size, "%s[]", element);
        break;
    }
    case 'm': {
        char key[64];
        char value[128];
        const char *value_descriptor = qiq_skip_descriptor(descriptor + 1);
        qiq_descriptor_name(descriptor + 1, key, sizeof key);
        qiq_descriptor_name(value_descriptor, value, sizeof value);
        snprintf(out, size, "map[%s, %s]", key, value);
        break;
    }
    default:
        snprintf(out, size, "%s", qiq_class_names[strtol(descriptor + 1, NULL, 10)]);
        break;
    }
}

// Walks a class's public fields: calls `visit` with each name (and its
// length), slot, and descriptor; stops early when it returns false.
typedef bool (*QiqFieldVisitor)(void *context, const char *name, size_t name_length, int64_t slot, const char *descriptor);

static int64_t qiq_class_fields(int64_t class_id, QiqFieldVisitor visit, void *context) {
    const char *cursor = qiq_class_json[class_id];
    int64_t field_count = strtol(cursor, (char **)&cursor, 10);
    cursor++;  // '|'
    while (visit != NULL && *cursor != '\0') {
        const char *name = cursor;
        const char *colon = strchr(name, ':');
        char *after_slot;
        int64_t slot = strtol(colon + 1, &after_slot, 10);
        const char *descriptor = after_slot + 1;
        if (!visit(context, name, (size_t)(colon - name), slot, descriptor)) break;
        cursor = qiq_skip_descriptor(descriptor) + 1;  // ';'
    }
    return field_count;
}

// ---------------------------------------------------------------------------
// Output buffer

typedef struct {
    char *data;
    size_t length;
    size_t capacity;
} QiqBuffer;

static void qiq_buffer_add(QiqBuffer *buffer, const char *bytes, size_t length) {
    if (buffer->length + length + 1 > buffer->capacity) {
        size_t capacity = buffer->capacity ? buffer->capacity : 256;
        while (buffer->length + length + 1 > capacity) capacity *= 2;
        char *grown = realloc(buffer->data, capacity);
        if (grown == NULL) qiq_fatal("out of memory");
        buffer->data = grown;
        buffer->capacity = capacity;
    }
    memcpy(buffer->data + buffer->length, bytes, length);
    buffer->length += length;
}

static void qiq_buffer_text(QiqBuffer *buffer, const char *text) {
    qiq_buffer_add(buffer, text, strlen(text));
}

// ---------------------------------------------------------------------------
// json_encode

typedef struct {
    QiqBuffer out;
    bool pretty;
    int depth;
    const char *error;
} QiqEncoder;

static void qiq_encode_value(QiqEncoder *encoder, const char *descriptor, uint64_t payload);
static void qiq_encode_string(QiqBuffer *out, const char *text);

// std/time's values, as ISO 8601 text: a DateTime as
// "2026-11-22T10:30:00+01:00" (with the microseconds when it has any), a
// Duration as "PT1H30M" (or "P2DT3H", "-PT0.5S").
static void qiq_encode_time_value(QiqEncoder *encoder, const char *json, const int64_t *slots) {
    char *end;
    if (json[1] == 'S') {
        // A value written as one of its strings (a Url).
        qiq_encode_string(&encoder->out, (const char *)(uintptr_t)slots[strtol(json + 2, NULL, 10)]);
        return;
    }
    if (json[1] == 'E') {
        // A backed enum, written as its value: "!ES<slot>" or "!EI<slot>".
        int64_t value = slots[strtol(json + 3, NULL, 10)];
        if (json[2] == 'S') {
            qiq_encode_string(&encoder->out, (const char *)(uintptr_t)value);
        } else {
            char number[24];
            qiq_format_int(number, value);
            qiq_buffer_text(&encoder->out, number);
        }
        return;
    }
    if (json[1] == 'N') {
        encoder->error = "a pure enum can't be written as JSON: only a backed enum (enum Name: string, or enum Name: int) has a value to write";
        return;
    }
    if (json[1] == 'T') {
        int64_t micros = slots[strtol(json + 2, &end, 10)];
        const char *zone = (const char *)(uintptr_t)slots[strtol(end + 1, NULL, 10)];
        if (qiq_json_datetime_format == NULL) {
            encoder->error = "internal error: DateTime's formatter isn't linked in";
            return;
        }
        int64_t micro = micros % 1000000;
        char *text = qiq_json_datetime_format(micro != 0 ? "Y-m-d\\TH:i:s.uP" : "Y-m-d\\TH:i:sP", micros, zone);
        if (text == NULL) {
            encoder->error = "a DateTime couldn't be written";
            return;
        }
        qiq_encode_string(&encoder->out, text);
        qiq_release(text);
        return;
    }
    int64_t micros = slots[strtol(json + 2, NULL, 10)];
    char text[96];
    size_t length = 0;
    uint64_t left = micros < 0 ? (uint64_t)-(micros + 1) + 1 : (uint64_t)micros;
    if (micros < 0) text[length++] = '-';
    text[length++] = 'P';
    uint64_t days = left / 86400000000ULL, rest = left % 86400000000ULL;
    if (days > 0) length += (size_t)snprintf(text + length, sizeof text - length, "%lluD", (unsigned long long)days);
    if (rest > 0 || days == 0) {
        text[length++] = 'T';
        uint64_t hours = rest / 3600000000ULL, minutes = rest / 60000000ULL % 60, seconds = rest / 1000000ULL % 60, fraction = rest % 1000000ULL;
        if (hours > 0) length += (size_t)snprintf(text + length, sizeof text - length, "%lluH", (unsigned long long)hours);
        if (minutes > 0) length += (size_t)snprintf(text + length, sizeof text - length, "%lluM", (unsigned long long)minutes);
        if (seconds > 0 || fraction > 0 || (hours == 0 && minutes == 0)) {
            length += (size_t)snprintf(text + length, sizeof text - length, "%llu", (unsigned long long)seconds);
            if (fraction > 0) {
                char digits[8];
                snprintf(digits, sizeof digits, "%06llu", (unsigned long long)fraction);
                for (int last = 5; last > 0 && digits[last] == '0'; last--) digits[last] = '\0';
                length += (size_t)snprintf(text + length, sizeof text - length, ".%s", digits);
            }
            text[length++] = 'S';
        }
    }
    text[length] = '\0';
    qiq_encode_string(&encoder->out, text);
}

static void qiq_encode_newline(QiqEncoder *encoder) {
    if (!encoder->pretty) return;
    qiq_buffer_add(&encoder->out, "\n", 1);
    for (int level = 0; level < encoder->depth; level++) qiq_buffer_add(&encoder->out, "    ", 4);
}

static void qiq_encode_string(QiqBuffer *out, const char *text) {
    qiq_buffer_add(out, "\"", 1);
    const unsigned char *cursor = (const unsigned char *)text;
    const unsigned char *run = cursor;
    for (; *cursor != '\0'; cursor++) {
        const char *escape = NULL;
        char unicode[8];
        switch (*cursor) {
        case '"': escape = "\\\""; break;
        case '\\': escape = "\\\\"; break;
        case '\b': escape = "\\b"; break;
        case '\f': escape = "\\f"; break;
        case '\n': escape = "\\n"; break;
        case '\r': escape = "\\r"; break;
        case '\t': escape = "\\t"; break;
        default:
            if (*cursor < 0x20) {
                snprintf(unicode, sizeof unicode, "\\u%04x", *cursor);
                escape = unicode;
            }
        }
        if (escape == NULL) continue;
        qiq_buffer_add(out, (const char *)run, (size_t)(cursor - run));
        qiq_buffer_text(out, escape);
        run = cursor + 1;
    }
    qiq_buffer_add(out, (const char *)run, (size_t)(cursor - run));
    qiq_buffer_add(out, "\"", 1);
}

// The shortest text that reads back as the same double, written as PHP's
// json_encode writes it: plain notation unless the exponent is below -4 or
// above 17 ("1.0e+20"), and no ".0" on whole numbers.
static void qiq_format_double(char *out, size_t size, double value) {
    char scientific[40];
    int precision = 1;
    for (; precision < 17; precision++) {
        snprintf(scientific, sizeof scientific, "%.*e", precision - 1, value);
        if (strtod(scientific, NULL) == value) break;
    }
    snprintf(scientific, sizeof scientific, "%.*e", precision - 1, fabs(value));
    char digits[24];
    size_t count = 0;
    const char *cursor = scientific;
    for (; *cursor != 'e'; cursor++) {
        if (*cursor >= '0' && *cursor <= '9') digits[count++] = *cursor;
    }
    while (count > 1 && digits[count - 1] == '0') count--;
    int decimal_point = value == 0.0 ? 1 : atoi(cursor + 1) + 1;
    char *dst = out;
    char *end = out + size - 1;
    if (signbit(value)) *dst++ = '-';
    if (decimal_point < 0 ? decimal_point < -3 : decimal_point > 17) {
        int exponent = decimal_point - 1;
        *dst++ = digits[0];
        *dst++ = '.';
        if (count == 1) *dst++ = '0';
        for (size_t index = 1; index < count; index++) *dst++ = digits[index];
        snprintf(dst, (size_t)(end - dst) + 1, "e%c%d", exponent < 0 ? '-' : '+', exponent < 0 ? -exponent : exponent);
        return;
    }
    if (decimal_point <= 0) {
        *dst++ = '0';
        *dst++ = '.';
        for (int index = decimal_point; index < 0; index++) *dst++ = '0';
        for (size_t index = 0; index < count; index++) *dst++ = digits[index];
    } else {
        for (int index = 0; index < decimal_point; index++) *dst++ = (size_t)index < count ? digits[index] : '0';
        if ((size_t)decimal_point < count) {
            *dst++ = '.';
            for (size_t index = (size_t)decimal_point; index < count; index++) *dst++ = digits[index];
        }
    }
    *dst = '\0';
}

static bool qiq_encode_open(QiqEncoder *encoder) {
    if (++encoder->depth > QIQ_JSON_MAX_DEPTH) {
        encoder->error = "maximum nesting depth of 512 exceeded (is there a reference cycle?)";
        return false;
    }
    return true;
}

typedef struct {
    QiqEncoder *encoder;
    const int64_t *slots;
    bool first;
} QiqObjectEncoding;

static bool qiq_encode_field(void *context, const char *name, size_t name_length, int64_t slot, const char *descriptor) {
    QiqObjectEncoding *object = context;
    QiqEncoder *encoder = object->encoder;
    if (!object->first) qiq_buffer_add(&encoder->out, ",", 1);
    object->first = false;
    qiq_encode_newline(encoder);
    qiq_buffer_add(&encoder->out, "\"", 1);
    qiq_buffer_add(&encoder->out, name, name_length);
    qiq_buffer_text(&encoder->out, encoder->pretty ? "\": " : "\":");
    qiq_encode_value(encoder, descriptor, (uint64_t)object->slots[slot]);
    return encoder->error == NULL;
}

static void qiq_encode_value(QiqEncoder *encoder, const char *descriptor, uint64_t payload) {
    if (encoder->error != NULL) return;
    char number[64];
    switch (*descriptor) {
    case 'i':
        qiq_format_int(number, (int64_t)payload);
        qiq_buffer_text(&encoder->out, number);
        return;
    case 'y':
        snprintf(number, sizeof number, "%u", (unsigned)(payload & 0xff));
        qiq_buffer_text(&encoder->out, number);
        return;
    case 'b':
        qiq_buffer_text(&encoder->out, payload ? "true" : "false");
        return;
    case 'd': {
        double value;
        memcpy(&value, &payload, sizeof value);
        if (!isfinite(value)) {
            encoder->error = "INF and NAN can't be encoded as JSON";
            return;
        }
        qiq_format_double(number, sizeof number, value);
        qiq_buffer_text(&encoder->out, number);
        return;
    }
    case 'n':
        qiq_buffer_text(&encoder->out, "null");
        return;
    case 'N':
        // A nullable scalar: null, or the scalar in its box.
        if (payload == 0) qiq_buffer_text(&encoder->out, "null");
        else qiq_encode_value(encoder, descriptor + 1, ((const uint64_t *)(uintptr_t)payload)[1]);
        return;
    default:
        break;
    }
    if (payload == 0) {
        qiq_buffer_text(&encoder->out, "null");
        return;
    }
    switch (*descriptor) {
    case 's':
    case 'c':
        qiq_encode_string(&encoder->out, (const char *)(uintptr_t)payload);
        return;
    case 'a': {
        void *array = (void *)(uintptr_t)payload;
        int64_t length = QIQ_ARRAY_LENGTH(array);
        if (length == 0) {
            qiq_buffer_text(&encoder->out, "[]");
            return;
        }
        if (!qiq_encode_open(encoder)) return;
        qiq_buffer_add(&encoder->out, "[", 1);
        for (int64_t index = 0; index < length && encoder->error == NULL; index++) {
            if (index > 0) qiq_buffer_add(&encoder->out, ",", 1);
            qiq_encode_newline(encoder);
            qiq_encode_value(encoder, descriptor + 1, qiq_load_element(QIQ_ARRAY_DATA(array), index, descriptor[1]));
        }
        encoder->depth--;
        qiq_encode_newline(encoder);
        qiq_buffer_add(&encoder->out, "]", 1);
        return;
    }
    case 'm': {
        void *map = (void *)(uintptr_t)payload;
        size_t count = qiq_map_size(map);
        if (count == 0) {
            qiq_buffer_text(&encoder->out, "{}");
            return;
        }
        if (!qiq_encode_open(encoder)) return;
        // Entries in insertion order, as PHP writes them.
        uint64_t *keys = malloc(count * sizeof *keys);
        uint64_t *values = malloc(count * sizeof *values);
        if (keys == NULL || values == NULL) qiq_fatal("out of memory");
        qiq_map_collect(map, keys, values);
        const char *key_descriptor = descriptor + 1;
        const char *value_descriptor = qiq_skip_descriptor(key_descriptor);
        qiq_buffer_add(&encoder->out, "{", 1);
        for (size_t index = 0; index < count && encoder->error == NULL; index++) {
            if (index > 0) qiq_buffer_add(&encoder->out, ",", 1);
            qiq_encode_newline(encoder);
            uint64_t key = keys[index];
            // Keys are strings in JSON: numbers and bools are written as text.
            switch (*key_descriptor) {
            case 's':
            case 'c':
                qiq_encode_string(&encoder->out, (const char *)(uintptr_t)key);
                break;
            case 'i': {
                int length = qiq_format_int(number + 1, (int64_t)key);
                number[0] = '"';
                number[length + 1] = '"';
                number[length + 2] = '\0';
                qiq_buffer_text(&encoder->out, number);
                break;
            }
            case 'd': {
                double value;
                memcpy(&value, &key, sizeof value);
                number[0] = '"';
                qiq_format_double(number + 1, sizeof number - 2, value);
                strcat(number, "\"");
                qiq_buffer_text(&encoder->out, number);
                break;
            }
            default:
                qiq_buffer_text(&encoder->out, key ? "\"true\"" : "\"false\"");
                break;
            }
            qiq_buffer_text(&encoder->out, encoder->pretty ? ": " : ":");
            qiq_encode_value(encoder, value_descriptor, values[index]);
        }
        free(keys);
        free(values);
        encoder->depth--;
        qiq_encode_newline(encoder);
        qiq_buffer_add(&encoder->out, "}", 1);
        return;
    }
    default: {
        // An object: its own class's public properties.
        const int64_t *slots = (const int64_t *)(uintptr_t)payload;
        const char *json = qiq_class_json[slots[0]];
        if (json[0] == '!') {
            qiq_encode_time_value(encoder, json, slots);
            return;
        }
        if (!qiq_encode_open(encoder)) return;
        QiqObjectEncoding object = {encoder, slots, true};
        size_t start = encoder->out.length;
        qiq_buffer_add(&encoder->out, "{", 1);
        qiq_class_fields(slots[0], qiq_encode_field, &object);
        encoder->depth--;
        if (object.first) {
            encoder->out.length = start;
            qiq_buffer_text(&encoder->out, "{}");
            return;
        }
        qiq_encode_newline(encoder);
        qiq_buffer_add(&encoder->out, "}", 1);
        return;
    }
    }
}

QIQ_GUARDED(char *, qiq_json_encode, NULL, (const char *descriptor, uint64_t payload, bool pretty),
            (descriptor, payload, pretty)) {
    QiqEncoder encoder = {{0}, pretty, 0, NULL};
    qiq_encode_value(&encoder, descriptor, payload);
    if (encoder.error != NULL) {
        free(encoder.out.data);
        qiq_throw(QIQ_JSON_EXCEPTION, encoder.error);
    }
    char *text = qiq_string_from(encoder.out.data, encoder.out.length);
    free(encoder.out.data);
    return text;
}

// ---------------------------------------------------------------------------
// json_decode

typedef struct {
    const char *text;
    const char *cursor;
    int depth;
    bool failed;
    bool syntax;   // the error is a syntax error
    char error[512];
    char path[512];
    size_t path_length;
} QiqDecoder;

static void qiq_skip_space(QiqDecoder *decoder) {
    while (*decoder->cursor == ' ' || *decoder->cursor == '\t' || *decoder->cursor == '\n' || *decoder->cursor == '\r') {
        decoder->cursor++;
    }
}

static void qiq_syntax_error(QiqDecoder *decoder, const char *expected) {
    if (decoder->failed) return;
    decoder->failed = true;
    decoder->syntax = true;
    int line = 1;
    int column = 1;
    for (const char *cursor = decoder->text; cursor < decoder->cursor; cursor++) {
        if (*cursor == '\n') {
            line++;
            column = 1;
        } else if ((*cursor & 0xc0) != 0x80) {
            column++;
        }
    }
    if (*decoder->cursor == '\0') {
        snprintf(decoder->error, sizeof decoder->error, "syntax error at line %d, column %d: unexpected end of JSON, expected %s",
                 line, column, expected);
    } else {
        snprintf(decoder->error, sizeof decoder->error, "syntax error at line %d, column %d: expected %s", line, column, expected);
    }
}

static void qiq_path_error(QiqDecoder *decoder, const char *message) {
    if (decoder->failed) return;
    decoder->failed = true;
    snprintf(decoder->error, sizeof decoder->error, "$%.*s: %s", (int)decoder->path_length, decoder->path, message);
}

// The kind of JSON value starting at the cursor, for type errors.
static const char *qiq_json_kind(char first) {
    switch (first) {
    case '{': return "an object";
    case '[': return "an array";
    case '"': return "a string";
    case 't':
    case 'f': return "a bool";
    case 'n': return "null";
    default: return "a number";
    }
}

static void qiq_type_error(QiqDecoder *decoder, const char *descriptor, const char *found) {
    char expected[256];
    char message[400];
    qiq_descriptor_name(descriptor, expected, sizeof expected);
    snprintf(message, sizeof message, "expected %s, found %s", expected, found);
    qiq_path_error(decoder, message);
}

static size_t qiq_path_push_index(QiqDecoder *decoder, int64_t index) {
    size_t saved = decoder->path_length;
    int written = snprintf(decoder->path + saved, sizeof decoder->path - saved, "[%lld]", (long long)index);
    if (written > 0 && saved + (size_t)written < sizeof decoder->path) decoder->path_length += (size_t)written;
    return saved;
}

static size_t qiq_path_push_key(QiqDecoder *decoder, const char *key) {
    size_t saved = decoder->path_length;
    bool plain = (*key >= 'a' && *key <= 'z') || (*key >= 'A' && *key <= 'Z') || *key == '_';
    for (const char *c = key; *c && plain; c++) {
        plain = (*c >= 'a' && *c <= 'z') || (*c >= 'A' && *c <= 'Z') || (*c >= '0' && *c <= '9') || *c == '_';
    }
    int written = snprintf(decoder->path + saved, sizeof decoder->path - saved, plain ? ".%s" : "[\"%s\"]", key);
    if (written > 0 && saved + (size_t)written < sizeof decoder->path) decoder->path_length += (size_t)written;
    return saved;
}

static bool qiq_match_literal(QiqDecoder *decoder, const char *literal) {
    size_t length = strlen(literal);
    if (strncmp(decoder->cursor, literal, length) != 0) {
        qiq_syntax_error(decoder, "a JSON value");
        return false;
    }
    decoder->cursor += length;
    return true;
}

static void qiq_add_utf8(QiqBuffer *buffer, uint32_t code) {
    char bytes[4];
    size_t length = code < 0x80 ? 1 : code < 0x800 ? 2 : code < 0x10000 ? 3 : 4;
    if (length == 1) {
        bytes[0] = (char)code;
    } else {
        for (size_t index = length - 1; index > 0; index--) {
            bytes[index] = (char)(0x80 | (code & 0x3f));
            code >>= 6;
        }
        bytes[0] = (char)((length == 2 ? 0xc0 : length == 3 ? 0xe0 : 0xf0) | code);
    }
    qiq_buffer_add(buffer, bytes, length);
}

static int qiq_hex4(const char *cursor) {
    int value = 0;
    for (int index = 0; index < 4; index++) {
        char c = cursor[index];
        int digit = c >= '0' && c <= '9' ? c - '0' : c >= 'a' && c <= 'f' ? c - 'a' + 10 : c >= 'A' && c <= 'F' ? c - 'A' + 10 : -1;
        if (digit < 0) return -1;
        value = value * 16 + digit;
    }
    return value;
}

// Parses a JSON string at the cursor into `out` (NUL-terminated, malloc'd
// buffer the caller frees). Returns false on a syntax error.
static bool qiq_parse_string(QiqDecoder *decoder, QiqBuffer *out) {
    decoder->cursor++;  // opening quote
    for (;;) {
        unsigned char c = (unsigned char)*decoder->cursor;
        if (c == '"') {
            decoder->cursor++;
            qiq_buffer_add(out, "", 0);  // makes sure there is room for the NUL
            out->data[out->length] = '\0';
            return true;
        }
        if (c == '\0') {
            qiq_syntax_error(decoder, "'\"' to end the string");
            return false;
        }
        if (c < 0x20) {
            qiq_syntax_error(decoder, "an escape such as \\n instead of a control character in a string");
            return false;
        }
        if (c != '\\') {
            const char *run = decoder->cursor;
            while ((unsigned char)*decoder->cursor >= 0x20 && *decoder->cursor != '"' && *decoder->cursor != '\\') decoder->cursor++;
            qiq_buffer_add(out, run, (size_t)(decoder->cursor - run));
            continue;
        }
        decoder->cursor++;
        char escape = *decoder->cursor;
        const char *simple = NULL;
        switch (escape) {
        case '"': simple = "\""; break;
        case '\\': simple = "\\"; break;
        case '/': simple = "/"; break;
        case 'b': simple = "\b"; break;
        case 'f': simple = "\f"; break;
        case 'n': simple = "\n"; break;
        case 'r': simple = "\r"; break;
        case 't': simple = "\t"; break;
        case 'u': {
            int code = qiq_hex4(decoder->cursor + 1);
            if (code < 0) {
                decoder->cursor++;
                qiq_syntax_error(decoder, "four hex digits after \\u");
                return false;
            }
            decoder->cursor += 5;
            uint32_t value = (uint32_t)code;
            if (value >= 0xd800 && value <= 0xdbff) {
                int low = decoder->cursor[0] == '\\' && decoder->cursor[1] == 'u' ? qiq_hex4(decoder->cursor + 2) : -1;
                if (low < 0xdc00 || low > 0xdfff) {
                    qiq_syntax_error(decoder, "a low surrogate (\\udc00 to \\udfff) after a high surrogate");
                    return false;
                }
                decoder->cursor += 6;
                value = 0x10000 + ((value - 0xd800) << 10) + ((uint32_t)low - 0xdc00);
            } else if (value >= 0xdc00 && value <= 0xdfff) {
                qiq_syntax_error(decoder, "a high surrogate before a low surrogate");
                return false;
            }
            if (value == 0) {
                qiq_path_error(decoder, "strings can't contain \\u0000");
                return false;
            }
            qiq_add_utf8(out, value);
            continue;
        }
        default:
            qiq_syntax_error(decoder, "a valid escape (\\\" \\\\ \\/ \\b \\f \\n \\r \\t \\uXXXX)");
            return false;
        }
        qiq_buffer_add(out, simple, 1);
        decoder->cursor++;
    }
}

// Reads a number token; `*integer` says whether it has no fraction or
// exponent. Returns its length, or 0 after a syntax error.
static size_t qiq_scan_number(QiqDecoder *decoder, bool *integer) {
    const char *start = decoder->cursor;
    const char *c = start;
    *integer = true;
    if (*c == '-') c++;
    if (*c == '0') {
        c++;
    } else if (*c >= '1' && *c <= '9') {
        while (*c >= '0' && *c <= '9') c++;
    } else {
        decoder->cursor = c;
        qiq_syntax_error(decoder, "a digit");
        return 0;
    }
    if (*c == '.') {
        *integer = false;
        c++;
        if (!(*c >= '0' && *c <= '9')) {
            decoder->cursor = c;
            qiq_syntax_error(decoder, "a digit after the decimal point");
            return 0;
        }
        while (*c >= '0' && *c <= '9') c++;
    }
    if (*c == 'e' || *c == 'E') {
        *integer = false;
        c++;
        if (*c == '+' || *c == '-') c++;
        if (!(*c >= '0' && *c <= '9')) {
            decoder->cursor = c;
            qiq_syntax_error(decoder, "a digit in the exponent");
            return 0;
        }
        while (*c >= '0' && *c <= '9') c++;
    }
    return (size_t)(c - start);
}

static bool qiq_skip_value(QiqDecoder *decoder);
static bool qiq_decode_value(QiqDecoder *decoder, const char *descriptor, uint64_t *out);

// Skips a value of any type (for object keys the class doesn't have),
// checking its syntax.
static bool qiq_skip_value(QiqDecoder *decoder) {
    qiq_skip_space(decoder);
    char first = *decoder->cursor;
    if (first == '"') {
        QiqBuffer text = {0};
        bool ok = qiq_parse_string(decoder, &text);
        free(text.data);
        return ok;
    }
    if (first == 't') return qiq_match_literal(decoder, "true");
    if (first == 'f') return qiq_match_literal(decoder, "false");
    if (first == 'n') return qiq_match_literal(decoder, "null");
    if (first == '-' || (first >= '0' && first <= '9')) {
        bool integer;
        size_t length = qiq_scan_number(decoder, &integer);
        decoder->cursor += length;
        return length > 0;
    }
    if (first != '[' && first != '{') {
        qiq_syntax_error(decoder, "a JSON value");
        return false;
    }
    if (++decoder->depth > QIQ_JSON_MAX_DEPTH) {
        qiq_path_error(decoder, "maximum nesting depth of 512 exceeded");
        return false;
    }
    char close = first == '[' ? ']' : '}';
    decoder->cursor++;
    qiq_skip_space(decoder);
    if (*decoder->cursor == close) {
        decoder->cursor++;
        decoder->depth--;
        return true;
    }
    for (;;) {
        if (close == '}') {
            qiq_skip_space(decoder);
            if (*decoder->cursor != '"') {
                qiq_syntax_error(decoder, "a string key");
                return false;
            }
            if (!qiq_skip_value(decoder)) return false;
            qiq_skip_space(decoder);
            if (*decoder->cursor != ':') {
                qiq_syntax_error(decoder, "':' after the key");
                return false;
            }
            decoder->cursor++;
        }
        if (!qiq_skip_value(decoder)) return false;
        qiq_skip_space(decoder);
        if (*decoder->cursor == ',') {
            decoder->cursor++;
            continue;
        }
        if (*decoder->cursor == close) {
            decoder->cursor++;
            decoder->depth--;
            return true;
        }
        qiq_syntax_error(decoder, close == ']' ? "',' or ']'" : "',' or '}'");
        return false;
    }
}

static bool qiq_decode_array(QiqDecoder *decoder, const char *descriptor, uint64_t *out) {
    const char *element = descriptor + 1;
    decoder->cursor++;  // '['
    uint64_t *items = NULL;
    size_t count = 0;
    size_t capacity = 0;
    bool ok = true;
    qiq_skip_space(decoder);
    if (*decoder->cursor == ']') {
        decoder->cursor++;
    } else {
        for (;;) {
            if (count == capacity) {
                capacity = capacity ? capacity * 2 : 8;
                uint64_t *grown = realloc(items, capacity * sizeof *items);
                if (grown == NULL) qiq_fatal("out of memory");
                items = grown;
            }
            size_t saved = qiq_path_push_index(decoder, (int64_t)count);
            ok = qiq_decode_value(decoder, element, &items[count]);
            decoder->path_length = saved;
            if (!ok) break;
            count++;
            qiq_skip_space(decoder);
            if (*decoder->cursor == ',') {
                decoder->cursor++;
                continue;
            }
            if (*decoder->cursor == ']') {
                decoder->cursor++;
                break;
            }
            qiq_syntax_error(decoder, "',' or ']'");
            ok = false;
            break;
        }
    }
    if (!ok) {
        if (qiq_is_reference_descriptor(*element)) {
            for (size_t index = 0; index < count; index++) qiq_release((void *)(uintptr_t)items[index]);
        }
        free(items);
        return false;
    }
    int64_t size = qiq_element_size(*element);
    void *array = qiq_array_create((int64_t)count, size, qiq_contents_flags(element));
    for (size_t index = 0; index < count; index++) {
        if (size == 1) {
            ((uint8_t *)QIQ_ARRAY_DATA(array))[index] = (uint8_t)items[index];
        } else {
            ((uint64_t *)QIQ_ARRAY_DATA(array))[index] = items[index];
        }
    }
    free(items);
    *out = (uint64_t)(uintptr_t)array;
    return true;
}

// A JSON object key as a map key of the map's key type.
static bool qiq_decode_key(QiqDecoder *decoder, char kind, const char *text, uint64_t *out) {
    char message[300];
    switch (kind) {
    case 's':
        *out = (uint64_t)(uintptr_t)qiq_string_from(text, strlen(text));
        return true;
    case 'c': {
        size_t width = 0;
        if (*text != '\0') {
            unsigned char first = (unsigned char)*text;
            width = first < 0x80 ? 1 : (first & 0xe0) == 0xc0 ? 2 : (first & 0xf0) == 0xe0 ? 3 : 4;
        }
        if (width == 0 || text[width] != '\0') break;
        *out = (uint64_t)(uintptr_t)qiq_string_from(text, width);
        return true;
    }
    case 'i': {
        bool integer;
        QiqDecoder scan = *decoder;
        scan.cursor = text;
        scan.failed = false;
        size_t length = *text == '\0' ? 0 : qiq_scan_number(&scan, &integer);
        if (length == 0 || text[length] != '\0' || !integer) break;
        errno = 0;
        long long value = strtoll(text, NULL, 10);
        if (errno == ERANGE) break;
        *out = (uint64_t)(int64_t)value;
        return true;
    }
    case 'd': {
        char *end;
        double value = strtod(text, &end);
        if (*text == '\0' || *end != '\0' || !isfinite(value)) break;
        memcpy(out, &value, sizeof value);
        return true;
    }
    case 'b':
        if (strcmp(text, "true") == 0 || strcmp(text, "false") == 0) {
            *out = text[0] == 't';
            return true;
        }
        break;
    default:
        break;
    }
    char expected[32];
    char key_descriptor[2] = {kind, '\0'};
    qiq_descriptor_name(key_descriptor, expected, sizeof expected);
    snprintf(message, sizeof message, "key \"%.200s\" is not a valid %s", text, expected);
    qiq_path_error(decoder, message);
    return false;
}

typedef struct {
    const char *key;
    int64_t slot;
    const char *descriptor;
} QiqFieldLookup;

static bool qiq_find_field(void *context, const char *name, size_t name_length, int64_t slot, const char *descriptor) {
    QiqFieldLookup *lookup = context;
    if (strlen(lookup->key) == name_length && memcmp(lookup->key, name, name_length) == 0) {
        lookup->slot = slot;
        lookup->descriptor = descriptor;
        return false;
    }
    return true;
}

static bool qiq_decode_object(QiqDecoder *decoder, const char *descriptor, uint64_t *out) {
    bool is_map = *descriptor == 'm';
    const char *key_descriptor = descriptor + 1;
    const char *value_descriptor = is_map ? qiq_skip_descriptor(key_descriptor) : NULL;
    void *container;
    int64_t class_id = 0;
    if (is_map) {
        static const char key_tags[] = {['i'] = 0, ['d'] = 1, ['b'] = 2, ['s'] = 3, ['c'] = 4};
        container = qiq_map_new((uint32_t)key_tags[(unsigned char)*key_descriptor], qiq_contents_flags(value_descriptor));
    } else {
        class_id = strtol(descriptor + 1, NULL, 10);
        container = qiq_object_new(qiq_class_fields(class_id, NULL, NULL), class_id);
    }
    decoder->cursor++;  // '{'
    qiq_skip_space(decoder);
    bool ok = true;
    if (*decoder->cursor == '}') {
        decoder->cursor++;
    } else {
        for (;;) {
            qiq_skip_space(decoder);
            if (*decoder->cursor != '"') {
                qiq_syntax_error(decoder, "a string key");
                ok = false;
                break;
            }
            QiqBuffer key = {0};
            if (!qiq_parse_string(decoder, &key)) {
                free(key.data);
                ok = false;
                break;
            }
            qiq_skip_space(decoder);
            if (*decoder->cursor != ':') {
                free(key.data);
                qiq_syntax_error(decoder, "':' after the key");
                ok = false;
                break;
            }
            decoder->cursor++;
            size_t saved = qiq_path_push_key(decoder, key.data);
            if (is_map) {
                uint64_t map_key;
                uint64_t value;
                ok = qiq_decode_key(decoder, *key_descriptor, key.data, &map_key);
                if (ok) {
                    ok = qiq_decode_value(decoder, value_descriptor, &value);
                    if (ok) {
                        // The map takes both references; a repeated key keeps the last value.
                        qiq_map_set(container, map_key, value);
                    } else if (qiq_is_reference_descriptor(*key_descriptor)) {
                        qiq_release((void *)(uintptr_t)map_key);
                    }
                }
            } else {
                QiqFieldLookup lookup = {key.data, -1, NULL};
                qiq_class_fields(class_id, qiq_find_field, &lookup);
                if (lookup.slot < 0) {
                    // Keys the class doesn't have are ignored.
                    ok = qiq_skip_value(decoder);
                } else {
                    uint64_t value;
                    ok = qiq_decode_value(decoder, lookup.descriptor, &value);
                    if (ok) {
                        int64_t *slots = container;
                        void *old = qiq_is_reference_descriptor(*lookup.descriptor) ? (void *)(intptr_t)slots[lookup.slot] : NULL;
                        slots[lookup.slot] = (int64_t)value;
                        qiq_release(old);
                    }
                }
            }
            decoder->path_length = saved;
            free(key.data);
            if (!ok) break;
            qiq_skip_space(decoder);
            if (*decoder->cursor == ',') {
                decoder->cursor++;
                continue;
            }
            if (*decoder->cursor == '}') {
                decoder->cursor++;
                break;
            }
            qiq_syntax_error(decoder, "',' or '}'");
            ok = false;
            break;
        }
    }
    if (!ok) {
        qiq_release(container);
        return false;
    }
    *out = (uint64_t)(uintptr_t)container;
    return true;
}

static bool qiq_decode_value(QiqDecoder *decoder, const char *descriptor, uint64_t *out) {
    qiq_skip_space(decoder);
    char first = *decoder->cursor;
    char kind = *descriptor;
    *out = 0;
    // A backed enum: its value, as the case with that value.
    if (kind == 'o' && first != 'n') {
        int64_t class_id = strtol(descriptor + 1, NULL, 10);
        const char *json = qiq_class_json[class_id];
        if (json[0] == '!' && json[1] == 'E') {
            bool text = json[2] == 'S';
            int64_t slot = strtol(json + 3, NULL, 10);
            uint64_t value;
            if (!qiq_decode_value(decoder, text ? "s" : "i", &value)) return false;
            for (void *const *cases = qiq_enum_cases[class_id]; *cases != NULL; cases++) {
                int64_t own = ((const int64_t *)*cases)[slot];
                bool same = text ? strcmp((const char *)(uintptr_t)own, (const char *)(uintptr_t)value) == 0 : own == (int64_t)value;
                if (same) {
                    if (text) qiq_release((void *)(uintptr_t)value);
                    *out = (uint64_t)(uintptr_t)*cases;
                    return true;
                }
            }
            char message[256];
            if (text) {
                snprintf(message, sizeof message, "\"%.100s\" is not a valid value for enum %s", (const char *)(uintptr_t)value, qiq_class_names[class_id]);
                qiq_release((void *)(uintptr_t)value);
            } else {
                snprintf(message, sizeof message, "%lld is not a valid value for enum %s", (long long)value, qiq_class_names[class_id]);
            }
            qiq_path_error(decoder, message);
            return false;
        }
    }
    if (kind == 'N' && first != 'n') {
        // A nullable scalar that isn't null: the scalar, in a new box.
        uint64_t scalar;
        if (!qiq_decode_value(decoder, descriptor + 1, &scalar)) return false;
        int64_t *box = qiq_object_new(2, qiq_box_classes[strchr("idby", descriptor[1]) - "idby"]);
        box[1] = (int64_t)scalar;
        *out = (uint64_t)(uintptr_t)box;
        return true;
    }
    if (first == '\0') {
        qiq_syntax_error(decoder, "a JSON value");
        return false;
    }
    if (first == 'n') {
        if (!qiq_match_literal(decoder, "null")) return false;
        if (qiq_is_reference_descriptor(kind)) return true;
        qiq_type_error(decoder, descriptor, "null");
        return false;
    }
    if (first == 't' || first == 'f') {
        if (!qiq_match_literal(decoder, first == 't' ? "true" : "false")) return false;
        if (kind == 'b') {
            *out = first == 't';
            return true;
        }
        qiq_type_error(decoder, descriptor, "a bool");
        return false;
    }
    if (first == '"') {
        if (kind != 's' && kind != 'c') {
            qiq_type_error(decoder, descriptor, "a string");
            return false;
        }
        QiqBuffer text = {0};
        if (!qiq_parse_string(decoder, &text)) {
            free(text.data);
            return false;
        }
        if (kind == 'c') {
            const char *bytes = text.data;
            size_t width = 0;
            if (*bytes != '\0') {
                unsigned char lead = (unsigned char)*bytes;
                width = lead < 0x80 ? 1 : (lead & 0xe0) == 0xc0 ? 2 : (lead & 0xf0) == 0xe0 ? 3 : 4;
            }
            if (width == 0 || bytes[width] != '\0') {
                free(text.data);
                qiq_path_error(decoder, "expected char, found a string that isn't one character");
                return false;
            }
        }
        *out = (uint64_t)(uintptr_t)qiq_string_from(text.data, text.length);
        free(text.data);
        return true;
    }
    if (first == '-' || (first >= '0' && first <= '9')) {
        bool integer;
        size_t length = qiq_scan_number(decoder, &integer);
        if (length == 0) return false;
        char token[64];
        bool fits = length < sizeof token;
        if (fits) {
            memcpy(token, decoder->cursor, length);
            token[length] = '\0';
        }
        decoder->cursor += length;
        if (kind == 'd') {
            double value = fits ? strtod(token, NULL) : 0.0;
            if (!fits || !isfinite(value)) {
                qiq_path_error(decoder, "number is outside the double range");
                return false;
            }
            memcpy(out, &value, sizeof value);
            return true;
        }
        if (kind == 'i' || kind == 'y') {
            if (!integer) {
                qiq_type_error(decoder, descriptor, "a number with a fraction or exponent");
                return false;
            }
            errno = 0;
            long long value = fits ? strtoll(token, NULL, 10) : 0;
            bool overflow = !fits || errno == ERANGE;
            if (overflow || (kind == 'y' && (value < 0 || value > 255))) {
                qiq_path_error(decoder, kind == 'y' ? "number is outside the byte range (0 to 255)" : "number is outside the int range");
                return false;
            }
            *out = (uint64_t)(int64_t)value;
            return true;
        }
        qiq_type_error(decoder, descriptor, "a number");
        return false;
    }
    if (first != '[' && first != '{') {
        qiq_syntax_error(decoder, "a JSON value");
        return false;
    }
    bool matches = first == '[' ? kind == 'a' : (kind == 'm' || kind == 'o');
    if (!matches) {
        qiq_type_error(decoder, descriptor, qiq_json_kind(first));
        return false;
    }
    if (++decoder->depth > QIQ_JSON_MAX_DEPTH) {
        qiq_path_error(decoder, "maximum nesting depth of 512 exceeded");
        return false;
    }
    bool ok = first == '[' ? qiq_decode_array(decoder, descriptor, out) : qiq_decode_object(decoder, descriptor, out);
    decoder->depth--;
    return ok;
}

// Returns the decoded value as a payload; references hold one reference.
QIQ_GUARDED(uint64_t, qiq_json_decode, 0, (const char *text, const char *descriptor), (text, descriptor)) {
    if (text == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "json_decode: the text is null");
    QiqDecoder decoder = {text, text, 0, false, false, {0}, {0}, 0};
    uint64_t value = 0;
    bool ok = qiq_decode_value(&decoder, descriptor, &value);
    if (ok) {
        qiq_skip_space(&decoder);
        if (*decoder.cursor != '\0') {
            qiq_syntax_error(&decoder, "the end of the JSON");
            if (qiq_is_reference_descriptor(*descriptor)) qiq_release((void *)(uintptr_t)value);
            ok = false;
        }
    }
    if (!ok && !decoder.syntax) {
        // A type error: if the text isn't valid JSON at all, say that instead.
        QiqDecoder check = {text, text, 0, false, false, {0}, {0}, 0};
        if (qiq_skip_value(&check)) {
            qiq_skip_space(&check);
            if (*check.cursor != '\0') qiq_syntax_error(&check, "the end of the JSON");
        }
        if (check.failed && check.syntax) memcpy(decoder.error, check.error, sizeof decoder.error);
    }
    if (!ok) qiq_throw(QIQ_JSON_EXCEPTION, decoder.error);
    return value;
}
