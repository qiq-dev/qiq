// CSV for std/csv (stdlib/csv.qiq): reading and writing RFC 4180 CSV, the
// format of Excel and every CSV library.
//
// Reading: fields may be quoted, "" inside quotes is a quote, and quoted
// fields may hold delimiters and line breaks. Lines end with \n or \r\n,
// blank lines are skipped (as Go's encoding/csv does), and a UTF-8 byte
// order mark at the start, which Excel writes, is ignored. Text after a
// closing quote is kept, as Python's csv module does, so a slightly
// malformed file still reads. A quoted field that's never closed is an
// error. Errors throw CsvException naming the line.
//
// Writing: a field is quoted only when it must be: when it holds the
// delimiter, a quote, or a line break (and a row of one empty field, which
// would otherwise read as a blank line).
//
// csv_decode and csv_encode, compiled by the compiler for the class given
// (see emit_csv_decode in src/compiler.rs), read rows into objects by
// header name and write objects as rows, using the class's public
// properties from qiq_class_json, as json.c does. csv_decode converts each
// row to JSON text and lets json_decode build the objects.
#include <errno.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>

#include "qiq_runtime.h"

extern const char *const qiq_class_json[];
extern const char *qiq_class_names[];
void *qiq_map_new(uint32_t key_type, int32_t value_references);
void qiq_map_set(void *map, uint64_t key, uint64_t value);
uint64_t qiq_json_decode(const char *text, const char *descriptor);

// ---------------------------------------------------------------------------
// Growing buffers.

typedef struct {
    char *data;
    size_t length;
    size_t capacity;
} QiqCsvText;

static void qiq_csv_add(QiqCsvText *text, const char *bytes, size_t count) {
    if (text->length + count + 1 > text->capacity) {
        size_t capacity = text->capacity ? text->capacity : 256;
        while (capacity < text->length + count + 1) capacity *= 2;
        char *data = realloc(text->data, capacity);
        if (data == NULL) qiq_fatal("out of memory");
        text->data = data;
        text->capacity = capacity;
    }
    memcpy(text->data + text->length, bytes, count);
    text->length += count;
    text->data[text->length] = '\0';
}

static void qiq_csv_add_char(QiqCsvText *text, char c) {
    qiq_csv_add(text, &c, 1);
}

// A list of pointers (strings, or rows).
typedef struct {
    void **items;
    size_t count;
    size_t capacity;
} QiqCsvList;

static void qiq_csv_push(QiqCsvList *list, void *item) {
    if (list->count == list->capacity) {
        list->capacity = list->capacity ? list->capacity * 2 : 16;
        list->items = realloc(list->items, list->capacity * sizeof(void *));
        if (list->items == NULL) qiq_fatal("out of memory");
    }
    list->items[list->count++] = item;
}

// Releases the qiq values a list holds and empties it.
static void qiq_csv_release_all(QiqCsvList *list) {
    for (size_t index = 0; index < list->count; index++) {
        if (list->items[index] != NULL) qiq_release(list->items[index]);
    }
    list->count = 0;
}

// ---------------------------------------------------------------------------
// Reading.

// Where records come from: a string, or a file read a chunk at a time.
typedef struct {
    const char *data;
    size_t length;
    size_t position;
    FILE *file;            // NULL for a string
    char *chunk;           // the file's current chunk
    int64_t line;          // the line being read, from 1
    bool started;          // whether the byte order mark was checked
} QiqCsvSource;

// The next byte, or -1 at the end of the input.
static int qiq_csv_peek(QiqCsvSource *source) {
    if (source->position >= source->length) {
        if (source->file == NULL) return -1;
        if (source->chunk == NULL) {
            source->chunk = malloc(65536);
            if (source->chunk == NULL) qiq_fatal("out of memory");
        }
        source->length = fread(source->chunk, 1, 65536, source->file);
        source->data = source->chunk;
        source->position = 0;
        if (source->length == 0) return -1;
    }
    return (unsigned char)source->data[source->position];
}

static int qiq_csv_take(QiqCsvSource *source) {
    int c = qiq_csv_peek(source);
    if (c >= 0) source->position++;
    return c;
}

// The delimiter a caller gave: one ASCII character that isn't a quote or a
// line break.
static char qiq_csv_delimiter(const char *delimiter, const char *caller) {
    if (delimiter == NULL) qiq_throwf(QIQ_RUNTIME_EXCEPTION, "%s: the delimiter is null", caller);
    unsigned char c = (unsigned char)delimiter[0];
    if (c == 0 || delimiter[1] != '\0' || c >= 0x80 || c == '"' || c == '\n' || c == '\r') {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "%s: the delimiter must be one character other than a quote or a line break, such as \",\", \";\", or \"\\t\"; found \"%s\"", caller, delimiter);
    }
    return (char)c;
}

// The current record's fields, as qiq strings.
static QiqCsvList qiq_csv_fields;
static QiqCsvText qiq_csv_field;

// The field read so far as a qiq string, added to the record.
static void qiq_csv_end_field(int64_t line) {
    if (!qiq_is_utf8_text((const unsigned char *)(qiq_csv_field.data ? qiq_csv_field.data : ""), qiq_csv_field.length)) {
        qiq_csv_release_all(&qiq_csv_fields);
        qiq_throwf(QIQ_CSV_EXCEPTION, "line %lld: a field isn't UTF-8 text", (long long)line);
    }
    qiq_csv_push(&qiq_csv_fields, qiq_string_from(qiq_csv_field.data ? qiq_csv_field.data : "", qiq_csv_field.length));
    qiq_csv_field.length = 0;
}

// Reads the next record's fields into qiq_csv_fields, and the line it
// starts on into *line. Returns false at the end of the input.
static bool qiq_csv_record(QiqCsvSource *source, char delimiter, int64_t *line) {
    qiq_csv_release_all(&qiq_csv_fields);
    qiq_csv_field.length = 0;
    if (!source->started) {
        source->started = true;
        if (qiq_csv_peek(source) == 0xEF && source->length - source->position >= 3 &&
            memcmp(source->data + source->position, "\xEF\xBB\xBF", 3) == 0) {
            source->position += 3;
        }
    }
    // Blank lines don't make records.
    for (;;) {
        int c = qiq_csv_peek(source);
        if (c == '\n') {
            qiq_csv_take(source);
            source->line++;
        } else if (c == '\r') {
            qiq_csv_take(source);
            if (qiq_csv_peek(source) == '\n') qiq_csv_take(source);
            source->line++;
        } else if (c < 0) {
            return false;
        } else {
            break;
        }
    }
    *line = source->line;
    for (;;) {
        // One field.
        if (qiq_csv_peek(source) == '"') {
            qiq_csv_take(source);
            int64_t opened = source->line;
            for (;;) {
                int c = qiq_csv_take(source);
                if (c < 0) {
                    qiq_csv_release_all(&qiq_csv_fields);
                    qiq_throwf(QIQ_CSV_EXCEPTION, "line %lld: a quoted field isn't closed", (long long)opened);
                }
                if (c == '"') {
                    if (qiq_csv_peek(source) != '"') break;
                    qiq_csv_take(source);
                }
                if (c == '\n') source->line++;
                qiq_csv_add_char(&qiq_csv_field, (char)c);
            }
        }
        // The (rest of the) field, up to the delimiter or the line's end.
        for (;;) {
            int c = qiq_csv_peek(source);
            if (c < 0 || c == delimiter || c == '\n' || c == '\r') break;
            qiq_csv_add_char(&qiq_csv_field, (char)qiq_csv_take(source));
        }
        qiq_csv_end_field(*line);
        int c = qiq_csv_peek(source);
        if (c == delimiter) {
            qiq_csv_take(source);
            continue;
        }
        if (c == '\r') {
            qiq_csv_take(source);
            if (qiq_csv_peek(source) == '\n') qiq_csv_take(source);
            source->line++;
        } else if (c == '\n') {
            qiq_csv_take(source);
            source->line++;
        }
        return true;
    }
}

// The current record as a new string[], taking its strings.
static void *qiq_csv_row(void) {
    void *row = qiq_array_create((int64_t)qiq_csv_fields.count, sizeof(char *), QIQ_ELEMENTS_ARE_REFERENCES);
    memcpy(QIQ_ARRAY_DATA(row), qiq_csv_fields.items, qiq_csv_fields.count * sizeof(char *));
    qiq_csv_fields.count = 0;
    return row;
}

// A list of qiq values as a new array of them, taking them.
static void *qiq_csv_array(QiqCsvList *list) {
    void *array = qiq_array_create((int64_t)list->count, sizeof(void *), QIQ_ELEMENTS_ARE_REFERENCES);
    if (list->count > 0) memcpy(QIQ_ARRAY_DATA(array), list->items, list->count * sizeof(void *));
    free(list->items);
    *list = (QiqCsvList){0};
    return array;
}

// __csv_parse(text, delimiter): every record, as string[][].
QIQ_GUARDED(void *, qiq_stdlib___csv_parse, NULL, (const char *text, const char *delimiter), (text, delimiter)) {
    char separator = qiq_csv_delimiter(delimiter, "csv_parse");
    if (text == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "csv_parse: the text is null");
    QiqCsvSource source = {text, strlen(text), 0, NULL, NULL, 1, false};
    QiqCsvList rows = {0};
    int64_t line;
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        qiq_guard_leave(previous);
        qiq_csv_release_all(&rows);
        free(rows.items);
        qiq_rethrow(previous);
    }
    while (qiq_csv_record(&source, separator, &line)) qiq_csv_push(&rows, qiq_csv_row());
    qiq_guard_leave(previous);
    return qiq_csv_array(&rows);
}

// The header record: its names, which must be different.
static void qiq_csv_check_header(char **names, size_t count, const char *caller) {
    for (size_t index = 0; index < count; index++) {
        for (size_t other = 0; other < index; other++) {
            if (strcmp(names[index], names[other]) == 0) {
                qiq_throwf(QIQ_CSV_EXCEPTION, "%s: line 1: the header has the column \"%s\" twice", caller, names[index]);
            }
        }
    }
}

// __csv_parse_maps(text, delimiter): the records after the header, each as
// a map from the header's names to its fields.
QIQ_GUARDED(void *, qiq_stdlib___csv_parse_maps, NULL, (const char *text, const char *delimiter), (text, delimiter)) {
    char separator = qiq_csv_delimiter(delimiter, "csv_parse_maps");
    if (text == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "csv_parse_maps: the text is null");
    QiqCsvSource source = {text, strlen(text), 0, NULL, NULL, 1, false};
    QiqCsvList rows = {0};
    void *header = NULL;
    int64_t line;
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        qiq_guard_leave(previous);
        qiq_csv_release_all(&rows);
        free(rows.items);
        if (header != NULL) qiq_release(header);
        qiq_rethrow(previous);
    }
    if (qiq_csv_record(&source, separator, &line)) {
        header = qiq_csv_row();
        size_t columns = (size_t)QIQ_ARRAY_LENGTH(header);
        char **names = QIQ_ARRAY_DATA(header);
        qiq_csv_check_header(names, columns, "csv_parse_maps");
        while (qiq_csv_record(&source, separator, &line)) {
            if (qiq_csv_fields.count != columns) {
                size_t found = qiq_csv_fields.count;
                qiq_csv_release_all(&qiq_csv_fields);
                qiq_throwf(QIQ_CSV_EXCEPTION, "line %lld: %zu field%s, but the header has %zu", (long long)line, found, found == 1 ? "" : "s", columns);
            }
            void *map = qiq_map_new(3, 1);
            for (size_t index = 0; index < columns; index++) {
                qiq_retain(names[index]);
                qiq_map_set(map, (uint64_t)(uintptr_t)names[index], (uint64_t)(uintptr_t)qiq_csv_fields.items[index]);
            }
            qiq_csv_fields.count = 0;
            qiq_csv_push(&rows, map);
        }
    }
    qiq_guard_leave(previous);
    if (header != NULL) qiq_release(header);
    return qiq_csv_array(&rows);
}

// ---------------------------------------------------------------------------
// Streaming: CsvReader.

typedef struct {
    QiqCsvSource source;
    char delimiter;
    int64_t line;   // where the last record started
} QiqCsvReader;

static QiqCsvReader **qiq_csv_readers;
static int64_t qiq_csv_reader_count;

QIQ_GUARDED(int64_t, qiq_stdlib___csv_open, 0, (const char *path, const char *delimiter), (path, delimiter)) {
    char separator = qiq_csv_delimiter(delimiter, "CsvReader::open");
    if (path == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "CsvReader::open: the path is null");
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        int error = errno;
        QiqExceptionKind kind = error == ENOENT ? QIQ_FILE_NOT_FOUND_EXCEPTION : error == EACCES ? QIQ_PERMISSION_DENIED_EXCEPTION : QIQ_IO_EXCEPTION;
        qiq_throwf(kind, "could not open %s: %s", path, strerror(error));
    }
    QiqCsvReader *reader = calloc(1, sizeof *reader);
    if (reader == NULL) qiq_fatal("out of memory");
    reader->source = (QiqCsvSource){NULL, 0, 0, file, NULL, 1, false};
    reader->delimiter = separator;
    QiqCsvReader **grown = realloc(qiq_csv_readers, (size_t)(qiq_csv_reader_count + 1) * sizeof *grown);
    if (grown == NULL) qiq_fatal("out of memory");
    qiq_csv_readers = grown;
    qiq_csv_readers[qiq_csv_reader_count] = reader;
    return ++qiq_csv_reader_count;
}

static QiqCsvReader *qiq_csv_reader(int64_t handle) {
    if (handle < 1 || handle > qiq_csv_reader_count || qiq_csv_readers[handle - 1] == NULL) {
        qiq_throw(QIQ_IO_EXCEPTION, "the CSV reader is closed");
    }
    return qiq_csv_readers[handle - 1];
}

// The next record, or null at the end.
QIQ_GUARDED(void *, qiq_stdlib___csv_next, NULL, (int64_t handle), (handle)) {
    QiqCsvReader *reader = qiq_csv_reader(handle);
    if (!qiq_csv_record(&reader->source, reader->delimiter, &reader->line)) return NULL;
    return qiq_csv_row();
}

QIQ_GUARDED(int64_t, qiq_stdlib___csv_line, 0, (int64_t handle), (handle)) {
    return qiq_csv_reader(handle)->line;
}

QIQ_GUARDED_VOID(qiq_stdlib___csv_close, (int64_t handle), (handle)) {
    if (handle < 1 || handle > qiq_csv_reader_count || qiq_csv_readers[handle - 1] == NULL) return;
    QiqCsvReader *reader = qiq_csv_readers[handle - 1];
    fclose(reader->source.file);
    free(reader->source.chunk);
    free(reader);
    qiq_csv_readers[handle - 1] = NULL;
}

// ---------------------------------------------------------------------------
// Writing.

// Adds `field` to `out`, quoted if it must be.
static void qiq_csv_write_field(QiqCsvText *out, const char *field, char delimiter, bool alone) {
    if (field == NULL) field = "";
    bool quote = alone && field[0] == '\0';
    for (const char *p = field; *p && !quote; p++) {
        quote = *p == delimiter || *p == '"' || *p == '\n' || *p == '\r';
    }
    if (!quote) {
        qiq_csv_add(out, field, strlen(field));
        return;
    }
    qiq_csv_add_char(out, '"');
    for (const char *p = field; *p; p++) {
        if (*p == '"') qiq_csv_add_char(out, '"');
        qiq_csv_add_char(out, *p);
    }
    qiq_csv_add_char(out, '"');
}

static void qiq_csv_write_row(QiqCsvText *out, void *row, char delimiter, const char *line_ending, int64_t number, const char *caller) {
    if (row == NULL) qiq_throwf(QIQ_RUNTIME_EXCEPTION, "%s: row %lld is null", caller, (long long)number);
    int64_t count = QIQ_ARRAY_LENGTH(row);
    char **fields = QIQ_ARRAY_DATA(row);
    for (int64_t index = 0; index < count; index++) {
        if (index > 0) qiq_csv_add_char(out, delimiter);
        qiq_csv_write_field(out, fields[index], delimiter, count == 1);
    }
    qiq_csv_add(out, line_ending, strlen(line_ending));
}

static const char *qiq_csv_line_ending(const char *line_ending, const char *caller) {
    if (line_ending == NULL || (strcmp(line_ending, "\n") != 0 && strcmp(line_ending, "\r\n") != 0)) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "%s: the line ending must be \"\\n\" or \"\\r\\n\"", caller);
    }
    return line_ending;
}

static char *qiq_csv_text_string(QiqCsvText *out) {
    char *text = qiq_string_from(out->data ? out->data : "", out->length);
    free(out->data);
    return text;
}

// __csv_format(rows, delimiter, lineEnding): the rows as CSV text.
QIQ_GUARDED(char *, qiq_stdlib___csv_format, NULL, (void *rows, const char *delimiter, const char *line_ending),
            (rows, delimiter, line_ending)) {
    char separator = qiq_csv_delimiter(delimiter, "csv_format");
    line_ending = qiq_csv_line_ending(line_ending, "csv_format");
    if (rows == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "csv_format: the rows are null");
    QiqCsvText out = {0};
    void **data = QIQ_ARRAY_DATA(rows);
    for (int64_t index = 0; index < QIQ_ARRAY_LENGTH(rows); index++) {
        if (data[index] == NULL) {
            free(out.data);
            qiq_throwf(QIQ_RUNTIME_EXCEPTION, "csv_format: row %lld is null", (long long)index);
        }
        qiq_csv_write_row(&out, data[index], separator, line_ending, index, "csv_format");
    }
    return qiq_csv_text_string(&out);
}

// __csv_format_row(row, delimiter, lineEnding): one row as a line of CSV.
QIQ_GUARDED(char *, qiq_stdlib___csv_format_row, NULL, (void *row, const char *delimiter, const char *line_ending),
            (row, delimiter, line_ending)) {
    char separator = qiq_csv_delimiter(delimiter, "CsvWriter");
    line_ending = qiq_csv_line_ending(line_ending, "CsvWriter");
    QiqCsvText out = {0};
    if (row == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "CsvWriter::write: the row is null");
    qiq_csv_write_row(&out, row, separator, line_ending, 0, "CsvWriter::write");
    return qiq_csv_text_string(&out);
}

// ---------------------------------------------------------------------------
// Objects: csv_decode and csv_encode.

typedef struct {
    char name[128];
    int64_t slot;
    char kind;     // i, d, b, or s
} QiqCsvProperty;

// The public properties of class `class_id`, from qiq_class_json:
// "<field count>|name:slot:descriptor;...". Returns how many.
static size_t qiq_csv_properties(int64_t class_id, QiqCsvProperty *properties, size_t limit, const char *caller) {
    const char *cursor = strchr(qiq_class_json[class_id], '|') + 1;
    size_t count = 0;
    while (*cursor && count < limit) {
        const char *colon = strchr(cursor, ':');
        const char *second = strchr(colon + 1, ':');
        const char *end = strchr(second + 1, ';');
        QiqCsvProperty *property = &properties[count++];
        size_t length = (size_t)(colon - cursor) < sizeof property->name ? (size_t)(colon - cursor) : sizeof property->name - 1;
        memcpy(property->name, cursor, length);
        property->name[length] = '\0';
        property->slot = strtoll(colon + 1, NULL, 10);
        property->kind = second[1];
        if (end != second + 2 || !strchr("idbs", property->kind)) {
            qiq_throwf(QIQ_CSV_EXCEPTION, "%s: %s's property '%s' isn't an int, double, bool, or string, which is all CSV holds", caller,
                       qiq_class_names[class_id], property->name);
        }
        cursor = end + 1;
    }
    return count;
}

static int64_t qiq_csv_class(const char *descriptor) {
    // "ao<class id>"
    return strtoll(descriptor + 2, NULL, 10);
}

// Adds `text` to `out` as a JSON string.
static void qiq_csv_json_string(QiqCsvText *out, const char *text) {
    qiq_csv_add_char(out, '"');
    for (const unsigned char *p = (const unsigned char *)text; *p; p++) {
        char escaped[8];
        if (*p == '"' || *p == '\\') {
            qiq_csv_add_char(out, '\\');
            qiq_csv_add_char(out, (char)*p);
        } else if (*p < 0x20) {
            snprintf(escaped, sizeof escaped, "\\u%04x", *p);
            qiq_csv_add(out, escaped, 6);
        } else {
            qiq_csv_add_char(out, (char)*p);
        }
    }
    qiq_csv_add_char(out, '"');
}

// `field` without spaces around it, into `out` (of `size` bytes).
static const char *qiq_csv_trimmed(const char *field, char *out, size_t size) {
    while (*field == ' ' || *field == '\t') field++;
    size_t length = strlen(field);
    while (length > 0 && (field[length - 1] == ' ' || field[length - 1] == '\t')) length--;
    if (length >= size) length = size - 1;
    memcpy(out, field, length);
    out[length] = '\0';
    return out;
}

// Adds field `field` (of `property`, in the record at `line`) to `out` as a
// JSON value of the property's type, or returns an error message.
static bool qiq_csv_json_value(QiqCsvText *out, const QiqCsvProperty *property, const char *field, int64_t line, char *message, size_t size) {
    char trimmed[128];
    const char *value = qiq_csv_trimmed(field, trimmed, sizeof trimmed);
    char *end;
    switch (property->kind) {
    case 's':
        qiq_csv_json_string(out, field);
        return true;
    case 'i': {
        errno = 0;
        long long number = strtoll(value, &end, 10);
        if (*value == '\0' || *end != '\0' || errno == ERANGE) break;
        char text[32];
        qiq_csv_add(out, text, (size_t)snprintf(text, sizeof text, "%lld", number));
        return true;
    }
    case 'd': {
        double number = strtod(value, &end);
        if (*value == '\0' || *end != '\0' || !isfinite(number)) break;
        char text[40];
        qiq_csv_add(out, text, (size_t)snprintf(text, sizeof text, "%.17g", number));
        return true;
    }
    case 'b':
        if (strcasecmp(value, "true") == 0 || strcmp(value, "1") == 0 || strcasecmp(value, "yes") == 0) {
            qiq_csv_add(out, "true", 4);
            return true;
        }
        if (strcasecmp(value, "false") == 0 || strcmp(value, "0") == 0 || strcasecmp(value, "no") == 0) {
            qiq_csv_add(out, "false", 5);
            return true;
        }
        break;
    }
    const char *wanted = property->kind == 'i' ? "an int" : property->kind == 'd' ? "a number" : "a bool (true, false, 1, 0, yes, or no)";
    snprintf(message, size, "line %lld, column '%s': \"%s\" isn't %s", (long long)line, property->name, field, wanted);
    return false;
}

// qiq_csv_decode(text, delimiter, descriptor): the records after the header,
// as objects of the class in `descriptor` ("ao<id>"), by header name.
QIQ_GUARDED(uint64_t, qiq_csv_decode, 0, (const char *text, const char *delimiter, const char *descriptor),
            (text, delimiter, descriptor)) {
    char separator = qiq_csv_delimiter(delimiter, "csv_decode");
    if (text == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "csv_decode: the text is null");
    int64_t class_id = qiq_csv_class(descriptor);
    QiqCsvProperty properties[256];
    size_t property_count = qiq_csv_properties(class_id, properties, 256, "csv_decode");
    QiqCsvSource source = {text, strlen(text), 0, NULL, NULL, 1, false};
    QiqCsvText json = {0};
    void *header = NULL;
    int64_t line;
    char message[512];
    message[0] = '\0';
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        qiq_guard_leave(previous);
        free(json.data);
        if (header != NULL) qiq_release(header);
        qiq_rethrow(previous);
    }
    qiq_csv_add_char(&json, '[');
    if (qiq_csv_record(&source, separator, &line)) {
        header = qiq_csv_row();
        size_t columns = (size_t)QIQ_ARRAY_LENGTH(header);
        char **names = QIQ_ARRAY_DATA(header);
        qiq_csv_check_header(names, columns, "csv_decode");
        // Each property's column.
        size_t column_of[256];
        for (size_t index = 0; index < property_count; index++) {
            column_of[index] = columns;
            for (size_t column = 0; column < columns; column++) {
                if (strcmp(names[column], properties[index].name) == 0) column_of[index] = column;
            }
            if (column_of[index] == columns) {
                qiq_throwf(QIQ_CSV_EXCEPTION, "csv_decode: the CSV has no column '%s' for %s's property", properties[index].name, qiq_class_names[class_id]);
            }
        }
        bool first = true;
        while (qiq_csv_record(&source, separator, &line)) {
            if (qiq_csv_fields.count != columns) {
                size_t found = qiq_csv_fields.count;
                qiq_csv_release_all(&qiq_csv_fields);
                qiq_throwf(QIQ_CSV_EXCEPTION, "line %lld: %zu field%s, but the header has %zu", (long long)line, found, found == 1 ? "" : "s", columns);
            }
            qiq_csv_add(&json, first ? "{" : ",{", first ? 1 : 2);
            first = false;
            for (size_t index = 0; index < property_count; index++) {
                if (index > 0) qiq_csv_add_char(&json, ',');
                qiq_csv_json_string(&json, properties[index].name);
                qiq_csv_add_char(&json, ':');
                if (!qiq_csv_json_value(&json, &properties[index], qiq_csv_fields.items[column_of[index]], line, message, sizeof message)) {
                    qiq_csv_release_all(&qiq_csv_fields);
                    qiq_throw(QIQ_CSV_EXCEPTION, message);
                }
            }
            qiq_csv_add_char(&json, '}');
            qiq_csv_release_all(&qiq_csv_fields);
        }
    }
    qiq_csv_add_char(&json, ']');
    qiq_guard_leave(previous);
    if (header != NULL) qiq_release(header);
    uint64_t objects = qiq_json_decode(json.data, descriptor);
    free(json.data);
    return objects;
}

// A double as the shortest text that reads back as it.
static void qiq_csv_double(QiqCsvText *out, double value) {
    char text[40];
    for (int precision = 1; precision <= 17; precision++) {
        snprintf(text, sizeof text, "%.*g", precision, value);
        if (precision == 17 || strtod(text, NULL) == value) break;
    }
    qiq_csv_add(out, text, strlen(text));
}

// qiq_csv_encode(objects, descriptor, delimiter, lineEnding): a header of the
// class's public property names, then a row for each object.
QIQ_GUARDED(char *, qiq_csv_encode, NULL, (void *objects, const char *descriptor, const char *delimiter, const char *line_ending),
            (objects, descriptor, delimiter, line_ending)) {
    char separator = qiq_csv_delimiter(delimiter, "csv_encode");
    line_ending = qiq_csv_line_ending(line_ending, "csv_encode");
    if (objects == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "csv_encode: the array is null");
    int64_t class_id = qiq_csv_class(descriptor);
    QiqCsvProperty properties[256];
    size_t count = qiq_csv_properties(class_id, properties, 256, "csv_encode");
    QiqCsvText out = {0};
    for (size_t index = 0; index < count; index++) {
        if (index > 0) qiq_csv_add_char(&out, separator);
        qiq_csv_write_field(&out, properties[index].name, separator, count == 1);
    }
    qiq_csv_add(&out, line_ending, strlen(line_ending));
    void **data = QIQ_ARRAY_DATA(objects);
    QiqCsvText field = {0};
    for (int64_t row = 0; row < QIQ_ARRAY_LENGTH(objects); row++) {
        const uint64_t *slots = data[row];
        if (slots == NULL) {
            free(out.data);
            free(field.data);
            qiq_throwf(QIQ_RUNTIME_EXCEPTION, "csv_encode: element %lld is null", (long long)row);
        }
        for (size_t index = 0; index < count; index++) {
            if (index > 0) qiq_csv_add_char(&out, separator);
            uint64_t bits = slots[properties[index].slot];
            field.length = 0;
            qiq_csv_add(&field, "", 0);
            switch (properties[index].kind) {
            case 'i': {
                char text[32];
                qiq_csv_add(&field, text, (size_t)snprintf(text, sizeof text, "%lld", (long long)(int64_t)bits));
                break;
            }
            case 'd': {
                double value;
                memcpy(&value, &bits, sizeof value);
                qiq_csv_double(&field, value);
                break;
            }
            case 'b':
                qiq_csv_add(&field, bits ? "true" : "false", bits ? 4 : 5);
                break;
            default: {
                const char *text = (const char *)(uintptr_t)bits;
                if (text != NULL) qiq_csv_add(&field, text, strlen(text));
            }
            }
            qiq_csv_write_field(&out, field.data, separator, count == 1);
        }
        qiq_csv_add(&out, line_ending, strlen(line_ending));
    }
    free(field.data);
    return qiq_csv_text_string(&out);
}
