// to_int, to_double, to_byte, and to_string for the conversions that can fail
// or need formatting; the rest are single LLVM instructions.
#include <errno.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "qiq_runtime.h"

// Truncates toward zero; NaN, infinities, and values outside the int range
// throw ArithmeticException.
QIQ_GUARDED(int64_t, qiq_double_to_int, 0, (double value), (value)) {
    // The int range is [-2^63, 2^63); both bounds are exact doubles.
    if (isnan(value) || value < -9223372036854775808.0 || value >= 9223372036854775808.0) {
        qiq_throwf(QIQ_ARITHMETIC_EXCEPTION, "cannot convert %g to int", value);
    }
    return (int64_t)value;
}

QIQ_GUARDED(uint8_t, qiq_int_to_byte, 0, (int64_t value), (value)) {
    if (value < 0 || value > 255) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "cannot convert %lld to byte: bytes are 0 to 255", (long long)value);
    }
    return (uint8_t)value;
}

static const char *qiq_require_text(const char *text) {
    if (text == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "string is null");
    return text;
}

// A whole string of the form [+-]digits, within the int range.
QIQ_GUARDED(int64_t, qiq_string_to_int, 0, (const char *text), (text)) {
    qiq_require_text(text);
    const char *cursor = text;
    if (*cursor == '+' || *cursor == '-') cursor++;
    if (*cursor == 0) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "\"%s\" is not an int", text);
    for (const char *digit = cursor; *digit != 0; digit++) {
        if (*digit < '0' || *digit > '9') qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "\"%s\" is not an int", text);
    }
    errno = 0;
    long long value = strtoll(text, NULL, 10);
    if (errno == ERANGE) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "\"%s\" is outside the int range", text);
    }
    return (int64_t)value;
}

// A whole string that strtod accepts, without leading whitespace.
QIQ_GUARDED(double, qiq_string_to_double, 0.0, (const char *text), (text)) {
    qiq_require_text(text);
    // strtod would skip leading whitespace; a valid double starts with a sign,
    // digit, or point (or "inf"/"nan").
    if (text[0] == 0 || text[0] == ' ' || text[0] == '\t' || text[0] == '\n' || text[0] == '\r') {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "\"%s\" is not a double", text);
    }
    char *end;
    double value = strtod(text, &end);
    if (*end != 0) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "\"%s\" is not a double", text);
    return value;
}

static char *qiq_copy_formatted(const char *formatted, int length) {
    return qiq_string_from(formatted, (size_t)length);
}

char *qiq_int_to_string(int64_t value) {
    char buffer[QIQ_INT_TEXT_SIZE];
    return qiq_copy_formatted(buffer, qiq_format_int(buffer, value));
}

// Formats like print: %g, so 1.5 is "1.5" and 2.0 is "2".
char *qiq_double_to_string(double value) {
    char buffer[64];
    return qiq_copy_formatted(buffer, snprintf(buffer, sizeof buffer, "%g", value));
}
