// std/math (stdlib/math.qiq): rounding, roots, logarithms, trigonometry,
// number_format, and random numbers. abs, min, max, and clamp are compiled
// inline for ints and doubles (see emit_math_generic in src/compiler.rs);
// this file has their array forms and their error cases.
//
// Rounding follows PHP 8.3 (see qiq_round): halves round away from zero,
// and round(1.005, 2) is 1.01, as written, even though the double nearest
// 1.005 is slightly below it.
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "qiq_runtime.h"

double qiq_stdlib_floor(double x) { return floor(x); }
double qiq_stdlib_ceil(double x) { return ceil(x); }
double qiq_stdlib_sqrt(double x) { return sqrt(x); }
double qiq_stdlib_exp(double x) { return exp(x); }
double qiq_stdlib_log10(double x) { return log10(x); }
double qiq_stdlib_log2(double x) { return log2(x); }
double qiq_stdlib_sin(double x) { return sin(x); }
double qiq_stdlib_cos(double x) { return cos(x); }
double qiq_stdlib_tan(double x) { return tan(x); }
double qiq_stdlib_asin(double x) { return asin(x); }
double qiq_stdlib_acos(double x) { return acos(x); }
double qiq_stdlib_atan(double x) { return atan(x); }
double qiq_stdlib_atan2(double y, double x) { return atan2(y, x); }
double qiq_stdlib_hypot(double x, double y) { return hypot(x, y); }
double qiq_stdlib_deg2rad(double degrees) { return degrees * (M_PI / 180.0); }
double qiq_stdlib_rad2deg(double radians) { return radians * (180.0 / M_PI); }
double qiq_stdlib_pi(void) { return M_PI; }
bool qiq_stdlib_is_nan(double x) { return isnan(x); }
bool qiq_stdlib_is_finite(double x) { return isfinite(x); }
bool qiq_stdlib_is_infinite(double x) { return isinf(x); }

// log(x, base): the natural logarithm when `base` is e (the default).
QIQ_GUARDED(double, qiq_stdlib_log, 0, (double x, double base), (x, base)) {
    if (base == M_E) return log(x);
    if (!(base > 0) || base == 1) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "log: the base must be above 0 and not 1, found %g", base);
    return log(x) / log(base);
}

// round(), as PHP 8.3's _php_math_round computes it, step for step, so the
// results are PHP's: when the requested place is within the 15 significant
// digits a double holds reliably, the value is first rounded to those 15
// digits (so round(1.005, 2) is 1.01, and round(2.8499999999999996, 1),
// which is 2.85 computed with a little error, is 2.9); otherwise it is
// scaled and rounded directly. Halves round away from zero. One difference:
// PHP 8.3 rounds with floor(x + 0.5), which makes round(0.49999999999999994)
// 1 (the sum rounds up to 1.0); this uses C's exact round(), as PHP 8.4 does.
static double qiq_pow10(int64_t power) {
    static const double powers[] = {1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11,
                                    1e12, 1e13, 1e14, 1e15, 1e16, 1e17, 1e18, 1e19, 1e20, 1e21, 1e22};
    if (power < 0 || power > 22) return pow(10.0, (double)power);
    return powers[power];
}

static double qiq_scale(double value, int64_t places) {
    double factor = qiq_pow10(places < 0 ? -places : places);
    return places >= 0 ? value * factor : value / factor;
}

static double qiq_round(double x, int64_t places) {
    if (!isfinite(x) || x == 0) return x;
    if (places < INT32_MIN + 1) places = INT32_MIN + 1;
    if (places > INT32_MAX) places = INT32_MAX;
    int64_t precision_places = 14 - (int64_t)floor(log10(fabs(x)));
    double factor = qiq_pow10(places < 0 ? -places : places);
    double scaled;
    if (precision_places > places && precision_places - 15 < places) {
        // Pre-round to 15 significant digits, then move the point to `places`.
        scaled = round(qiq_scale(x, precision_places));
        int64_t shift = places - precision_places;
        scaled = scaled / qiq_pow10(shift < 0 ? -shift : shift);
    } else {
        scaled = places >= 0 ? x * factor : x / factor;
        // Beyond a double's precision: rounding would change nothing.
        if (fabs(scaled) >= 1e15) return x;
    }
    scaled = round(scaled);
    if (places > -23 && places < 23) {
        return places > 0 ? scaled / factor : scaled * factor;
    }
    char text[64];
    snprintf(text, sizeof text, "%15fe%lld", scaled, (long long)-places);
    double result = strtod(text, NULL);
    return isfinite(result) ? result : x;
}

double qiq_stdlib_round(double x, int64_t places) { return qiq_round(x, places); }

// number_format: `x` rounded to `decimals` places, with `thousands` between
// each group of three digits before `point`. A result that rounds to zero
// has no minus sign, as in PHP 8.
static char *qiq_number_format(const char *digits_in, bool negative, int64_t decimals, const char *point, const char *thousands) {
    // digits_in: "1234567.89" (plain digits, '.' before the decimals if any)
    const char *dot = strchr(digits_in, '.');
    size_t integer_length = dot ? (size_t)(dot - digits_in) : strlen(digits_in);
    size_t point_length = strlen(point), thousands_length = strlen(thousands);
    size_t groups = integer_length > 0 ? (integer_length - 1) / 3 : 0;
    size_t length = (negative ? 1 : 0) + integer_length + groups * thousands_length + (decimals > 0 ? point_length + (size_t)decimals : 0);
    char *out = qiq_string_new(length);
    char *at = out;
    if (negative) *at++ = '-';
    for (size_t index = 0; index < integer_length; index++) {
        if (index > 0 && (integer_length - index) % 3 == 0) {
            memcpy(at, thousands, thousands_length);
            at += thousands_length;
        }
        *at++ = digits_in[index];
    }
    if (decimals > 0) {
        memcpy(at, point, point_length);
        at += point_length;
        memcpy(at, dot + 1, (size_t)decimals);
        at += decimals;
    }
    *at = '\0';
    return out;
}

static void qiq_check_format(const char *point, const char *thousands) {
    if (point == NULL || thousands == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "number_format: the separator is null");
}

QIQ_GUARDED(char *, qiq_stdlib_number_format, NULL, (double x, int64_t decimals, const char *point, const char *thousands),
            (x, decimals, point, thousands)) {
    qiq_check_format(point, thousands);
    if (!isfinite(x)) return qiq_string_from(isnan(x) ? "NAN" : x > 0 ? "INF" : "-INF", isnan(x) ? 3 : x > 0 ? 3 : 4);
    // Negative decimals round before the point (PHP 8.3) and show none.
    double rounded = qiq_round(x, decimals);
    if (decimals < 0) decimals = 0;
    if (decimals > 100) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "number_format: at most 100 decimals, found %lld", (long long)decimals);
    char digits[512];
    snprintf(digits, sizeof digits, "%.*f", (int)decimals, fabs(rounded));
    bool zero = true;
    for (const char *p = digits; *p; p++) if (*p >= '1' && *p <= '9') zero = false;
    return qiq_number_format(digits, rounded < 0 && !zero, decimals, point, thousands);
}

// number_format of an int, exact for every int (a double isn't, above 2^53).
QIQ_GUARDED(char *, qiq_stdlib___number_format_int, NULL, (int64_t x, int64_t decimals, const char *point, const char *thousands),
            (x, decimals, point, thousands)) {
    qiq_check_format(point, thousands);
    if (decimals > 100) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "number_format: at most 100 decimals, found %lld", (long long)decimals);
    uint64_t magnitude = x < 0 ? 0 - (uint64_t)x : (uint64_t)x;
    if (decimals < 0) {
        // Round to 10^-decimals, halves away from zero.
        uint64_t unit = 1;
        bool beyond = false;
        for (int64_t i = 0; i < -decimals; i++) {
            if (unit > UINT64_MAX / 10) {
                beyond = true;
                break;
            }
            unit *= 10;
        }
        if (beyond) {
            magnitude = 0;
        } else {
            uint64_t remainder = magnitude % unit;
            magnitude -= remainder;
            if (remainder >= unit - remainder) magnitude += unit;
        }
        decimals = 0;
    }
    char digits[160];
    int length = snprintf(digits, sizeof digits, "%llu", (unsigned long long)magnitude);
    if (decimals > 0) {
        digits[length++] = '.';
        memset(digits + length, '0', (size_t)decimals);
        digits[length + decimals] = '\0';
    }
    return qiq_number_format(digits, x < 0 && magnitude != 0, decimals, point, thousands);
}

// ---------------------------------------------------------------------------
// Random numbers, from the kernel's secure source, as PHP's random_int.
// Nothing is buffered, so processes forked by the HTTP server never share
// numbers.

QIQ_GUARDED(int64_t, qiq_stdlib_random_int, 0, (int64_t min, int64_t max), (min, max)) {
    if (min > max) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "random_int: min (%lld) is greater than max (%lld)", (long long)min, (long long)max);
    uint64_t range = (uint64_t)max - (uint64_t)min;  // the count minus one
    uint64_t value;
    if (range == UINT64_MAX) {
        qiq_random_bytes((unsigned char *)&value, sizeof value, "random_int");
        return (int64_t)value;
    }
    // Rejection sampling, so every value is equally likely: values below
    // 2^64 mod count would make the first few results more common.
    uint64_t count = range + 1;
    uint64_t threshold = (0 - count) % count;
    do {
        qiq_random_bytes((unsigned char *)&value, sizeof value, "random_int");
    } while (value < threshold);
    return (int64_t)((uint64_t)min + value % count);
}

// A double in [0, 1), with 53 random bits: every value a multiple of 2^-53.
QIQ_GUARDED(double, qiq_stdlib_random_double, 0, (void), ()) {
    uint64_t value;
    qiq_random_bytes((unsigned char *)&value, sizeof value, "random_double");
    return (double)(value >> 11) * 0x1.0p-53;
}

// ---------------------------------------------------------------------------
// The array forms of min and max, and clamp's error.

static void *qiq_math_nonempty(void *array, const char *function) {
    if (array == NULL) qiq_throwf(QIQ_RUNTIME_EXCEPTION, "%s: the array is null", function);
    if (QIQ_ARRAY_LENGTH(array) == 0) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "%s: the array is empty", function);
    return array;
}

#define QIQ_MATH_EXTREME(name, shown, type, better)                                                   \
    QIQ_GUARDED(type, qiq_math_##name, 0, (void *array), (array)) {                                   \
        const type *data = QIQ_ARRAY_DATA(qiq_math_nonempty(array, shown));                           \
        type best = data[0];                                                                          \
        for (int64_t index = 1; index < QIQ_ARRAY_LENGTH(array); index++) {                           \
            if (data[index] better best) best = data[index];                                          \
        }                                                                                             \
        return best;                                                                                  \
    }
QIQ_MATH_EXTREME(min_int, "min", int64_t, <)
QIQ_MATH_EXTREME(max_int, "max", int64_t, >)
QIQ_MATH_EXTREME(min_double, "min", double, <)
QIQ_MATH_EXTREME(max_double, "max", double, >)

// clamp's slow path: throws when low > high (the compiled code handles the rest).
QIQ_GUARDED(int64_t, qiq_math_clamp_int, 0, (int64_t x, int64_t low, int64_t high), (x, low, high)) {
    if (low > high) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "clamp: low (%lld) is greater than high (%lld)", (long long)low, (long long)high);
    return x < low ? low : x > high ? high : x;
}

QIQ_GUARDED(double, qiq_math_clamp_double, 0, (double x, double low, double high), (x, low, high)) {
    if (!(low <= high)) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "clamp: low (%g) is greater than high (%g)", low, high);
    return x < low ? low : x > high ? high : x;
}
