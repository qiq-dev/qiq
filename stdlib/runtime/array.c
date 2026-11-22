// Arrays. An array is a pointer to {int64 length; void *data; int64 capacity}
// followed by room for the elements it was created with (see
// QIQ_ARRAY_LENGTH and friends in qiq_runtime.h). `data` points to that room
// until the array grows past it; then the elements move to a separately
// allocated buffer that doubles as needed. The array's address never
// changes, so every variable that refers to it sees the same elements.
//
// Element sizes are 1 (bool, byte) or 8 (everything else). Indexes run from
// 0 to length - 1 with no gaps: removing an element shifts the later ones
// down, as with Python lists.
#include <math.h>
#include <stdlib.h>
#include <string.h>

#include "qiq_runtime.h"

// The Unicode code point of a char (a one-character UTF-8 string).
static int64_t qiq_code_point(const char *text) {
    const unsigned char *bytes = (const unsigned char *)text;
    if (bytes[0] < 0x80) return bytes[0];
    if ((bytes[0] & 0xe0) == 0xc0) return ((bytes[0] & 0x1f) << 6) | (bytes[1] & 0x3f);
    if ((bytes[0] & 0xf0) == 0xe0) return ((bytes[0] & 0x0f) << 12) | ((bytes[1] & 0x3f) << 6) | (bytes[2] & 0x3f);
    return ((int64_t)(bytes[0] & 0x07) << 18) | ((bytes[1] & 0x3f) << 12) | ((bytes[2] & 0x3f) << 6) | (bytes[3] & 0x3f);
}

void *qiq_array_create(int64_t count, int64_t element_size, int references) {
    if (count > (INT64_MAX - QIQ_ARRAY_INLINE - 64) / (element_size > 0 ? element_size : 1)) {
        qiq_fatal("array is too large");
    }
    // `references` is 1 when elements are references, plus QIQ_MAY_CYCLE.
    void *array = qiq_allocate((size_t)(QIQ_ARRAY_INLINE + count * element_size), QIQ_KIND_ARRAY,
                               (uint32_t)references & (QIQ_ELEMENTS_ARE_REFERENCES | QIQ_MAY_CYCLE));
    QIQ_ARRAY_LENGTH(array) = count;
    QIQ_ARRAY_DATA(array) = QIQ_ARRAY_INLINE_DATA(array);
    QIQ_ARRAY_CAPACITY(array) = count;
    return array;
}

// Frees the buffer of an array that has grown (memory.c frees the rest).
void qiq_array_free_storage(void *array) {
    if (QIQ_ARRAY_DATA(array) != QIQ_ARRAY_INLINE_DATA(array)) free(QIQ_ARRAY_DATA(array));
}

// `new type[count]` and array literals.
QIQ_GUARDED(void *, qiq_array_new, NULL, (int64_t count, int64_t element_size, int32_t references),
            (count, element_size, references)) {
    if (count < 0) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "array size must not be negative, found %lld", (long long)count);
    return qiq_array_create(count, element_size, references);
}

static void *qiq_require_array(void *array) {
    if (array == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "array is null");
    return array;
}

// Length of an array, for foreach.
QIQ_GUARDED(int64_t, qiq_array_length, 0, (void *array), (array)) {
    return QIQ_ARRAY_LENGTH(qiq_require_array(array));
}

// Bounds check before reading or writing `array[index]`.
QIQ_GUARDED_VOID(qiq_array_check_index, (void *array, int64_t index), (array, index)) {
    int64_t length = QIQ_ARRAY_LENGTH(qiq_require_array(array));
    if (index < 0 || index >= length) {
        qiq_throwf(QIQ_OUT_OF_RANGE_EXCEPTION, "index %lld is out of range for an array of length %lld",
                   (long long)index, (long long)length);
    }
}

// len() of an array.
QIQ_GUARDED(int64_t, qiq_len_array, 0, (void *array), (array)) {
    if (array == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "len() of a null array");
    return QIQ_ARRAY_LENGTH(array);
}

// ---------------------------------------------------------------------------
// Growing and shrinking. Values passed in are references the array takes
// over; values returned are references the caller takes over.

static uint64_t qiq_load(const void *data, int64_t index, int64_t size) {
    return size == 1 ? ((const uint8_t *)data)[index] : ((const uint64_t *)data)[index];
}

static void qiq_store(void *data, int64_t index, int64_t size, uint64_t value) {
    if (size == 1) {
        ((uint8_t *)data)[index] = (uint8_t)value;
    } else {
        ((uint64_t *)data)[index] = value;
    }
}

// Makes room for `needed` elements, doubling the capacity.
static void qiq_reserve(void *array, int64_t needed, int64_t size) {
    int64_t capacity = QIQ_ARRAY_CAPACITY(array);
    if (needed <= capacity) return;
    int64_t grown = capacity < 4 ? 4 : capacity;
    while (grown < needed) {
        if (grown > INT64_MAX / 2 / size) qiq_fatal("array is too large");
        grown *= 2;
    }
    void *data = QIQ_ARRAY_DATA(array);
    void *buffer;
    if (data == QIQ_ARRAY_INLINE_DATA(array)) {
        buffer = malloc((size_t)(grown * size));
        if (buffer != NULL) memcpy(buffer, data, (size_t)(QIQ_ARRAY_LENGTH(array) * size));
    } else {
        buffer = realloc(data, (size_t)(grown * size));
    }
    if (buffer == NULL) qiq_fatal("out of memory growing array");
    QIQ_ARRAY_DATA(array) = buffer;
    QIQ_ARRAY_CAPACITY(array) = grown;
}

// array_push(array, value): adds to the end.
QIQ_GUARDED_VOID(qiq_array_push, (void *array, int64_t size, uint64_t value), (array, size, value)) {
    qiq_require_array(array);
    int64_t length = QIQ_ARRAY_LENGTH(array);
    qiq_reserve(array, length + 1, size);
    qiq_store(QIQ_ARRAY_DATA(array), length, size, value);
    QIQ_ARRAY_LENGTH(array) = length + 1;
}

// array_pop(array): removes and returns the last element.
QIQ_GUARDED(uint64_t, qiq_array_pop, 0, (void *array, int64_t size), (array, size)) {
    qiq_require_array(array);
    int64_t length = QIQ_ARRAY_LENGTH(array);
    if (length == 0) qiq_throw(QIQ_OUT_OF_RANGE_EXCEPTION, "array_pop: the array is empty");
    QIQ_ARRAY_LENGTH(array) = length - 1;
    return qiq_load(QIQ_ARRAY_DATA(array), length - 1, size);
}

// array_shift(array): removes and returns the first element; the rest move
// down one place.
QIQ_GUARDED(uint64_t, qiq_array_shift, 0, (void *array, int64_t size), (array, size)) {
    qiq_require_array(array);
    int64_t length = QIQ_ARRAY_LENGTH(array);
    if (length == 0) qiq_throw(QIQ_OUT_OF_RANGE_EXCEPTION, "array_shift: the array is empty");
    char *data = QIQ_ARRAY_DATA(array);
    uint64_t first = qiq_load(data, 0, size);
    memmove(data, data + size, (size_t)((length - 1) * size));
    QIQ_ARRAY_LENGTH(array) = length - 1;
    return first;
}

// Inserts at `index` (0 to length), moving later elements up: array_unshift.
QIQ_GUARDED_VOID(qiq_array_insert, (void *array, int64_t size, int64_t index, uint64_t value), (array, size, index, value)) {
    qiq_require_array(array);
    int64_t length = QIQ_ARRAY_LENGTH(array);
    if (index < 0 || index > length) {
        qiq_throwf(QIQ_OUT_OF_RANGE_EXCEPTION, "index %lld is out of range for inserting into an array of length %lld",
                   (long long)index, (long long)length);
    }
    qiq_reserve(array, length + 1, size);
    char *data = QIQ_ARRAY_DATA(array);
    memmove(data + (index + 1) * size, data + index * size, (size_t)((length - index) * size));
    qiq_store(data, index, size, value);
    QIQ_ARRAY_LENGTH(array) = length + 1;
}

// array_splice(array, offset, length, replacement), as in PHP: removes
// `length` elements from `offset` (all the rest when `has_length` is false),
// puts the elements of `replacement` (which may be null) in their place, and
// returns the removed elements as a new array. A negative offset counts from
// the end; a negative length leaves that many elements at the end.
QIQ_GUARDED(void *, qiq_array_splice, NULL,
            (void *array, int64_t size, int64_t offset, int64_t count, bool has_length, void *replacement),
            (array, size, offset, count, has_length, replacement)) {
    qiq_require_array(array);
    int64_t length = QIQ_ARRAY_LENGTH(array);
    if (offset < 0) offset = offset + length < 0 ? 0 : offset + length;
    if (offset > length) offset = length;
    if (!has_length) count = length - offset;
    if (count < 0) count = length - offset + count < 0 ? 0 : length - offset + count;
    if (count > length - offset) count = length - offset;

    uint32_t flags = QIQ_HEADER(array)->flags & (QIQ_ELEMENTS_ARE_REFERENCES | QIQ_MAY_CYCLE);
    char *data = QIQ_ARRAY_DATA(array);
    // The removed elements move to the new array, with their references.
    void *removed = qiq_array_create(count, size, (int)flags);
    memcpy(QIQ_ARRAY_DATA(removed), data + offset * size, (size_t)(count * size));

    // Copy the replacement first: it may be this same array.
    int64_t added = replacement == NULL ? 0 : QIQ_ARRAY_LENGTH(replacement);
    char *incoming = NULL;
    if (added > 0) {
        incoming = malloc((size_t)(added * size));
        if (incoming == NULL) qiq_fatal("out of memory");
        memcpy(incoming, QIQ_ARRAY_DATA(replacement), (size_t)(added * size));
        if (flags & QIQ_ELEMENTS_ARE_REFERENCES) {
            for (int64_t index = 0; index < added; index++) qiq_retain((void *)(uintptr_t)qiq_load(incoming, index, size));
        }
    }
    int64_t new_length = length - count + added;
    qiq_reserve(array, new_length, size);
    data = QIQ_ARRAY_DATA(array);
    memmove(data + (offset + added) * size, data + (offset + count) * size, (size_t)((length - offset - count) * size));
    if (added > 0) memcpy(data + offset * size, incoming, (size_t)(added * size));
    free(incoming);
    QIQ_ARRAY_LENGTH(array) = new_length;
    return removed;
}

// ---------------------------------------------------------------------------
// range(start, end, step), as in PHP 8.3: the end is included, the direction
// comes from start and end, and the step must be non-zero, no bigger than the
// span, and (for an increasing range) positive.

#define QIQ_RANGE_MAX_ELEMENTS (INT64_C(1) << 31)

// The number of ints from `start` to `end`, and the signed step between
// them. `foreach (range(...) as ...)` uses this to count without an array.
QIQ_GUARDED(int64_t, qiq_range_plan, 0, (int64_t start, int64_t end, int64_t step, bool has_step, int64_t *out_step),
            (start, end, step, has_step, out_step)) {
    if (!has_step) step = 1;
    if (step == 0) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "range(): the step cannot be 0");
    if (start == end) {
        *out_step = 1;
        return 1;
    }
    bool increasing = start < end;
    if (increasing && step < 0) {
        qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "range(): the step must be greater than 0 for increasing ranges");
    }
    uint64_t span = increasing ? (uint64_t)end - (uint64_t)start : (uint64_t)start - (uint64_t)end;
    uint64_t magnitude = step < 0 ? (uint64_t)0 - (uint64_t)step : (uint64_t)step;
    if (magnitude > span) {
        qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "range(): the step must be less than the range spanned by start and end");
    }
    uint64_t count = span / magnitude + 1;
    if (count > (uint64_t)INT64_MAX) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "range(): the range is too large");
    *out_step = increasing ? (int64_t)magnitude : (int64_t)((uint64_t)0 - magnitude);
    return (int64_t)count;
}

static void qiq_check_range_size(int64_t count) {
    if (count > QIQ_RANGE_MAX_ELEMENTS) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION,
                   "range(): %lld elements are too many for an array; use foreach (range(...) as i), which doesn't build one",
                   (long long)count);
    }
}

QIQ_GUARDED(void *, qiq_range_int, NULL, (int64_t start, int64_t end, int64_t step, bool has_step),
            (start, end, step, has_step)) {
    int64_t signed_step;
    int64_t count = qiq_range_plan_impl(start, end, step, has_step, &signed_step);
    qiq_check_range_size(count);
    void *array = qiq_array_create(count, 8, 0);
    int64_t *data = QIQ_ARRAY_DATA(array);
    for (int64_t index = 0; index < count; index++) {
        data[index] = (int64_t)((uint64_t)start + (uint64_t)index * (uint64_t)signed_step);
    }
    return array;
}

// range(count): 0 up to count - 1, or no elements when count <= 0.
QIQ_GUARDED(void *, qiq_range_count, NULL, (int64_t count), (count)) {
    if (count < 0) count = 0;
    qiq_check_range_size(count);
    void *array = qiq_array_create(count, 8, 0);
    int64_t *data = QIQ_ARRAY_DATA(array);
    for (int64_t index = 0; index < count; index++) data[index] = index;
    return array;
}

// Doubles: element i is start + i * step, stopping at the end, as in PHP.
QIQ_GUARDED(void *, qiq_range_double, NULL, (double start, double end, double step, bool has_step),
            (start, end, step, has_step)) {
    if (!has_step) step = 1.0;
    if (!isfinite(start) || !isfinite(end)) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "range(): start and end must be finite numbers");
    if (!isfinite(step)) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "range(): the step must be a finite number");
    if (step == 0.0) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "range(): the step cannot be 0");
    if (start == end) {
        void *array = qiq_array_create(1, 8, 0);
        ((double *)QIQ_ARRAY_DATA(array))[0] = start;
        return array;
    }
    bool increasing = start < end;
    if (increasing && step < 0) {
        qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "range(): the step must be greater than 0 for increasing ranges");
    }
    double magnitude = fabs(step);
    double span = increasing ? end - start : start - end;
    if (magnitude > span) {
        qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "range(): the step must be less than the range spanned by start and end");
    }
    double estimate = floor(span / magnitude + 1.0 + 0.5);
    if (estimate > (double)QIQ_RANGE_MAX_ELEMENTS) qiq_check_range_size(QIQ_RANGE_MAX_ELEMENTS + 1);
    int64_t limit = (int64_t)estimate;
    void *array = qiq_array_create(limit, 8, 0);
    double *data = QIQ_ARRAY_DATA(array);
    int64_t count = 0;
    for (; count < limit; count++) {
        // The first element is `start` itself (so -0.0 stays -0.0, as in PHP).
        double element = count == 0 ? start : increasing ? start + (double)count * magnitude : start - (double)count * magnitude;
        if (increasing ? element > end : element < end) break;
        data[count] = element;
    }
    QIQ_ARRAY_LENGTH(array) = count;
    return array;
}

// Chars: by Unicode code point, so range('a', 'e') and range('α', 'ε') work.
QIQ_GUARDED(void *, qiq_range_char, NULL, (const char *start, const char *end, int64_t step, bool has_step),
            (start, end, step, has_step)) {
    if (start == NULL || end == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "range(): a char is null");
    int64_t first = qiq_code_point(start);
    int64_t last = qiq_code_point(end);
    int64_t signed_step;
    int64_t count = qiq_range_plan_impl(first, last, step, has_step, &signed_step);
    qiq_check_range_size(count);
    void *array = qiq_array_create(count, 8, QIQ_ELEMENTS_ARE_REFERENCES);
    char **data = QIQ_ARRAY_DATA(array);
    for (int64_t index = 0; index < count; index++) {
        int64_t code = first + index * signed_step;
        if (code >= 0xd800 && code <= 0xdfff) {
            qiq_release(array);
            qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "range(): the range includes U+D800 to U+DFFF, which aren't characters");
        }
        char bytes[4];
        size_t length = code < 0x80 ? 1 : code < 0x800 ? 2 : code < 0x10000 ? 3 : 4;
        if (length == 1) {
            bytes[0] = (char)code;
        } else {
            int64_t rest = code;
            for (size_t byte = length - 1; byte > 0; byte--) {
                bytes[byte] = (char)(0x80 | (rest & 0x3f));
                rest >>= 6;
            }
            bytes[0] = (char)((length == 2 ? 0xc0 : length == 3 ? 0xe0 : 0xf0) | rest);
        }
        data[index] = qiq_string_from(bytes, length);
    }
    return array;
}
