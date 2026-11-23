// print(...) for values the compiler can't format with a single printf:
// arrays, maps, objects, and several arguments on one line.
//
// The compiler describes each value's type with a descriptor string:
//   i int   d double   b bool   y byte   s string   c char   n null   o object
//   a<element>        array, e.g. "ai" is int[] and "aai" is int[][]
//   m<key><value>     map, e.g. "msai" is map[string, int[]]
// Values arrive as 64-bit payloads, encoded like map values: ints as they
// are, doubles keep their bits, bools are 0 or 1, and the rest are
// pointers.
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include <stdlib.h>

#include "qiq_runtime.h"

// Each class's name, indexed by class id; defined in every compiled program.
extern const char *qiq_class_names[];


// The descriptor just past the type that starts at `descriptor`.
static const char *qiq_skip_type(const char *descriptor) {
    switch (*descriptor) {
    case 'a': return qiq_skip_type(descriptor + 1);
    case 'm': return qiq_skip_type(qiq_skip_type(descriptor + 1));
    default: return descriptor + 1;
    }
}

// Bytes an array element of this type occupies (see ValueType::size_bytes).
static size_t qiq_element_size(char type) {
    switch (type) {
    case 'b':
    case 'y': return 1;
    default: return 8;
    }
}

// Reads one array element as a payload.
static uint64_t qiq_read_element(const char *data, char type) {
    switch (type) {
    case 'b': return *(const uint8_t *)data != 0;
    case 'y': return *(const uint8_t *)data;
    default: {
        uint64_t bits;
        memcpy(&bits, data, sizeof bits);
        return bits;
    }
    }
}

// Writes `text` between `quote` characters, escaping like a qiq literal.
static void qiq_write_quoted(const char *text, char quote) {
    putchar(quote);
    for (const char *cursor = text; *cursor != 0; cursor++) {
        switch (*cursor) {
        case '\n': fputs("\\n", stdout); break;
        case '\r': fputs("\\r", stdout); break;
        case '\t': fputs("\\t", stdout); break;
        case '\\': fputs("\\\\", stdout); break;
        default:
            if (*cursor == quote) putchar('\\');
            putchar(*cursor);
        }
    }
    putchar(quote);
}

// Writes the value of type `descriptor`. Nested values (inside an array or
// map) are written like qiq literals: quoted strings and chars, true/false.
static void qiq_write_value(const char *descriptor, uint64_t bits, int nested) {
    void *pointer = (void *)(uintptr_t)bits;
    switch (*descriptor) {
    case 'i':
        printf("%lld", (long long)(int64_t)bits);
        return;
    case 'd': {
        double value;
        memcpy(&value, &bits, sizeof value);
        printf("%g", value);
        return;
    }
    case 'y':
        printf("%u", (unsigned)(uint8_t)bits);
        return;
    case 'b':
        fputs(bits ? "true" : "false", stdout);
        return;
    case 'n':
        fputs("null", stdout);
        return;
    }
    if (pointer == NULL) {
        fputs("null", stdout);
        return;
    }
    switch (*descriptor) {
    case 's':
        if (nested) qiq_write_quoted(pointer, '"');
        else fputs(pointer, stdout);
        return;
    case 'c':
        if (nested) qiq_write_quoted(pointer, '\'');
        else fputs(pointer, stdout);
        return;
    case 'o':
        // The object's own class, which may be a subclass of its declared type.
        fputs(qiq_class_names[*(const int64_t *)pointer], stdout);
        return;
    case 'a': {
        const char *element = descriptor + 1;
        size_t size = qiq_element_size(*element);
        const char *data = QIQ_ARRAY_DATA(pointer);
        int64_t length = QIQ_ARRAY_LENGTH(pointer);
        putchar('[');
        for (int64_t index = 0; index < length; index++) {
            if (index > 0) fputs(", ", stdout);
            qiq_write_value(element, qiq_read_element(data + index * size, *element), 1);
        }
        putchar(']');
        return;
    }
    case 'm': {
        const char *key = descriptor + 1;
        const char *value = qiq_skip_type(key);
        size_t length = qiq_map_size(pointer);
        uint64_t *keys = malloc((length + 1) * sizeof *keys);
        uint64_t *values = malloc((length + 1) * sizeof *values);
        if (keys == NULL || values == NULL) qiq_fatal("out of memory");
        qiq_map_collect(pointer, keys, values);
        putchar('{');
        for (size_t index = 0; index < length; index++) {
            if (index > 0) fputs(", ", stdout);
            qiq_write_value(key, keys[index], 1);
            fputs(" => ", stdout);
            qiq_write_value(value, values[index], 1);
        }
        putchar('}');
        free(keys);
        free(values);
        return;
    }
    }
}

// Writes one print argument (without a newline).
void qiq_print_value(const char *descriptor, uint64_t bits) {
    qiq_write_value(descriptor, bits, 0);
}

// Separates print arguments.
void qiq_print_space(void) {
    putchar(' ');
}

// Ends a print statement with several arguments, or with a container.
void qiq_print_newline(void) {
    putchar('\n');
}
