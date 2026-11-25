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

// The array a variadic parameter gets from single values followed by a
// spread, `f(a, b, ...tail)`: `head` elements at the start for the compiled
// code to store the values in, then the elements of `tail` (with a reference
// to each, when they're references). One allocation; `tail` is unchanged.
QIQ_GUARDED(void *, qiq_array_with_tail, NULL, (int64_t head, void *tail, int64_t element_size, int32_t references),
            (head, tail, element_size, references)) {
    if (tail == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "the array spread with '...' is null");
    int64_t count = QIQ_ARRAY_LENGTH(tail);
    void *array = qiq_array_create(head + count, element_size, references);
    char *data = QIQ_ARRAY_DATA(array);
    memcpy(data + head * element_size, QIQ_ARRAY_DATA(tail), (size_t)(count * element_size));
    if ((uint32_t)references & QIQ_ELEMENTS_ARE_REFERENCES) {
        void **elements = (void **)data;
        for (int64_t index = head; index < head + count; index++) {
            if (elements[index] != NULL) qiq_retain(elements[index]);
        }
    }
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

// ---------------------------------------------------------------------------
// sort, rsort, usort, in_array, array_search, array_slice, array_reverse,
// array_unique, and array_sum. Each returns a new array (or a value),
// leaving its argument as it was. `size` is the element size (1 for bools
// and bytes, 8 otherwise) and `flags` the array's (references, cycles), as
// the compiler passes them to qiq_array_new.

// From map.c, for array_unique.
void *qiq_map_new(uint32_t key_type, int32_t value_references);
void qiq_map_set(void *map, uint64_t key, uint64_t value);
bool qiq_map_has(void *map, uint64_t key);

// A new array of `count` elements of `array`, from `indexes` (or all of them
// in order when NULL), holding references of its own.
static void *qiq_array_pick(void *array, int64_t size, int32_t flags, const int64_t *indexes, int64_t count) {
    void *result = qiq_array_create(count, size, flags);
    const char *data = QIQ_ARRAY_DATA(array);
    for (int64_t index = 0; index < count; index++) {
        uint64_t value = qiq_load(data, indexes ? indexes[index] : index, size);
        if ((flags & QIQ_ELEMENTS_ARE_REFERENCES) && value != 0) qiq_retain((void *)(uintptr_t)value);
        qiq_store(QIQ_ARRAY_DATA(result), index, size, value);
    }
    return result;
}

// What qiq_array_sort compares: an element kind for sort and rsort, or a
// comparison function compiled for usort.
typedef struct {
    const char *data;
    int64_t size;
    int kind;            // 0 int, 1 double, 2 string or char, 3 byte; -1 for `compare`
    bool descending;
    int64_t (*compare)(void *closure, const void *left, const void *right);
    void *closure;
    bool failed;         // the comparison threw
} QiqSortOrder;

// <0, 0, or >0 for elements `left` and `right` (indexes into the data).
static int qiq_sort_compare(QiqSortOrder *order, int64_t left, int64_t right) {
    const char *a = order->data + left * order->size;
    const char *b = order->data + right * order->size;
    int result;
    if (order->kind < 0) {
        int64_t compared = order->compare(order->closure, a, b);
        if (qiq_pending_exception != NULL) order->failed = true;
        result = compared < 0 ? -1 : compared > 0 ? 1 : 0;
    } else {
        uint64_t x = qiq_load(order->data, left, order->size), y = qiq_load(order->data, right, order->size);
        switch (order->kind) {
        case 0:
            result = (int64_t)x < (int64_t)y ? -1 : (int64_t)x > (int64_t)y;
            break;
        case 1: {
            double p, q;
            memcpy(&p, &x, sizeof p);
            memcpy(&q, &y, sizeof q);
            result = p < q ? -1 : p > q;
            break;
        }
        case 2:
            result = strcmp((const char *)(uintptr_t)(x ? x : (uint64_t)(uintptr_t)""), (const char *)(uintptr_t)(y ? y : (uint64_t)(uintptr_t)""));
            break;
        default:
            result = x < y ? -1 : x > y;
        }
    }
    return order->descending ? -result : result;
}

// A stable merge sort of the indexes `items[0..count)`, using `scratch`.
static void qiq_merge_sort(QiqSortOrder *order, int64_t *items, int64_t *scratch, int64_t count) {
    if (count < 2 || order->failed) return;
    int64_t half = count / 2;
    qiq_merge_sort(order, items, scratch, half);
    qiq_merge_sort(order, items + half, scratch, count - half);
    if (order->failed) return;
    int64_t left = 0, right = half, out = 0;
    while (left < half && right < count) {
        // Equal elements keep their order: the left one first.
        if (qiq_sort_compare(order, items[right], items[left]) < 0) scratch[out++] = items[right++];
        else scratch[out++] = items[left++];
        if (order->failed) return;
    }
    while (left < half) scratch[out++] = items[left++];
    while (right < count) scratch[out++] = items[right++];
    memcpy(items, scratch, (size_t)count * sizeof *items);
}

static void *qiq_array_sorted(void *array, int64_t size, int32_t flags, QiqSortOrder *order) {
    qiq_require_array(array);
    int64_t count = QIQ_ARRAY_LENGTH(array);
    int64_t *items = malloc((size_t)(count > 0 ? count : 1) * sizeof *items * 2);
    if (items == NULL) qiq_fatal("out of memory sorting an array");
    for (int64_t index = 0; index < count; index++) items[index] = index;
    order->data = QIQ_ARRAY_DATA(array);
    order->size = size;
    qiq_merge_sort(order, items, items + count, count);
    void *result = order->failed ? NULL : qiq_array_pick(array, size, flags, items, count);
    free(items);
    return result;
}

// Sorts numbers directly, rather than through indexes: equal numbers can't
// be told apart, so the sort needn't be stable. A merge sort, with insertion
// sort for short runs.
#define QIQ_VALUE_SORT(name, type)                                                                \
    static void name(type *values, type *scratch, int64_t count) {                              \
        if (count <= 24) {                                                                       \
            for (int64_t i = 1; i < count; i++) {                                                \
                type value = values[i];                                                          \
                int64_t j = i;                                                                   \
                while (j > 0 && value < values[j - 1]) {                                         \
                    values[j] = values[j - 1];                                                   \
                    j--;                                                                         \
                }                                                                                \
                values[j] = value;                                                               \
            }                                                                                    \
            return;                                                                              \
        }                                                                                        \
        int64_t half = count / 2;                                                                \
        name(values, scratch, half);                                                             \
        name(values + half, scratch, count - half);                                              \
        if (!(values[half] < values[half - 1])) return;                                          \
        int64_t left = 0, right = half, out = 0;                                                 \
        while (left < half && right < count) {                                                   \
            scratch[out++] = values[right] < values[left] ? values[right++] : values[left++];    \
        }                                                                                        \
        while (left < half) scratch[out++] = values[left++];                                     \
        memcpy(values, scratch, (size_t)out * sizeof(type));                                     \
    }
QIQ_VALUE_SORT(qiq_sort_ints, int64_t)
QIQ_VALUE_SORT(qiq_sort_doubles, double)
QIQ_VALUE_SORT(qiq_sort_bytes, uint8_t)

// sort(array) and rsort(array): ints, doubles, strings and chars (by their
// bytes, so by Unicode code point), or bytes and bools.
QIQ_GUARDED(void *, qiq_array_sort, NULL, (void *array, int64_t size, int32_t flags, int32_t kind, bool descending),
            (array, size, flags, kind, descending)) {
    if (kind == 2) {
        QiqSortOrder order = {NULL, 0, kind, descending, NULL, NULL, false};
        return qiq_array_sorted(array, size, flags, &order);
    }
    qiq_require_array(array);
    int64_t count = QIQ_ARRAY_LENGTH(array);
    void *result = qiq_array_create(count, size, flags);
    if (count == 0) return result;
    void *values = QIQ_ARRAY_DATA(result);
    memcpy(values, QIQ_ARRAY_DATA(array), (size_t)(count * size));
    void *scratch = malloc((size_t)(count * size));
    if (scratch == NULL) qiq_fatal("out of memory sorting an array");
    if (kind == 0) qiq_sort_ints(values, scratch, count);
    else if (kind == 1) qiq_sort_doubles(values, scratch, count);
    else qiq_sort_bytes(values, scratch, count);
    free(scratch);
    if (descending) {
        char *bytes = values;
        for (int64_t i = 0, j = count - 1; i < j; i++, j--) {
            char swap[8];
            memcpy(swap, bytes + i * size, (size_t)size);
            memcpy(bytes + i * size, bytes + j * size, (size_t)size);
            memcpy(bytes + j * size, swap, (size_t)size);
        }
    }
    return result;
}

// usort(array, compare): `compare` is compiled for the element type, and
// calls the closure with two elements. Compiled code doesn't jump out of
// this function when it throws; it leaves the exception pending, which stops
// the sort, and the compiled code that called usort sees it.
QIQ_GUARDED(void *, qiq_array_usort, NULL,
            (void *array, int64_t size, int32_t flags, int64_t (*compare)(void *, const void *, const void *), void *closure),
            (array, size, flags, compare, closure)) {
    qiq_require_array(array);
    QiqSortOrder order = {NULL, 0, -1, false, compare, closure, false};
    return qiq_array_sorted(array, size, flags, &order);
}

// in_array and array_search: the index of the first element equal to
// `value` (as == compares, by `descriptor`), or -1.
QIQ_GUARDED(int64_t, qiq_array_index_of, -1, (void *array, int64_t size, const char *descriptor, uint64_t value),
            (array, size, descriptor, value)) {
    qiq_require_array(array);
    const char *data = QIQ_ARRAY_DATA(array);
    for (int64_t index = 0; index < QIQ_ARRAY_LENGTH(array); index++) {
        if (qiq_payload_equal(descriptor, qiq_load(data, index, size), value)) return index;
    }
    return -1;
}

// array_slice(array, offset, length): as PHP's. A negative offset counts
// from the end; without a length (`has_length` false) the slice runs to the
// end, and a negative length stops that many elements before it. Offsets
// past either end are clamped, so the result can be empty.
QIQ_GUARDED(void *, qiq_array_slice, NULL, (void *array, int64_t size, int32_t flags, int64_t offset, int64_t length, bool has_length),
            (array, size, flags, offset, length, has_length)) {
    qiq_require_array(array);
    int64_t count = QIQ_ARRAY_LENGTH(array);
    int64_t start = offset < 0 ? count + offset : offset;
    if (start < 0) start = 0;
    if (start > count) start = count;
    int64_t end = !has_length ? count : length < 0 ? count + length : start + (length > count ? count : length);
    if (end > count) end = count;
    if (end < start) end = start;
    void *result = qiq_array_create(end - start, size, flags);
    const char *data = QIQ_ARRAY_DATA(array);
    for (int64_t index = start; index < end; index++) {
        uint64_t value = qiq_load(data, index, size);
        if ((flags & QIQ_ELEMENTS_ARE_REFERENCES) && value != 0) qiq_retain((void *)(uintptr_t)value);
        qiq_store(QIQ_ARRAY_DATA(result), index - start, size, value);
    }
    return result;
}

// array_reverse(array): the elements in the opposite order.
QIQ_GUARDED(void *, qiq_array_reverse, NULL, (void *array, int64_t size, int32_t flags), (array, size, flags)) {
    qiq_require_array(array);
    int64_t count = QIQ_ARRAY_LENGTH(array);
    int64_t *indexes = malloc((size_t)(count > 0 ? count : 1) * sizeof *indexes);
    if (indexes == NULL) qiq_fatal("out of memory reversing an array");
    for (int64_t index = 0; index < count; index++) indexes[index] = count - 1 - index;
    void *result = qiq_array_pick(array, size, flags, indexes, count);
    free(indexes);
    return result;
}

// array_unique(array): each value once, where it first appears.
QIQ_GUARDED(void *, qiq_array_unique, NULL, (void *array, int64_t size, int32_t flags, const char *descriptor),
            (array, size, flags, descriptor)) {
    qiq_require_array(array);
    int64_t count = QIQ_ARRAY_LENGTH(array);
    const char *data = QIQ_ARRAY_DATA(array);
    int64_t *kept = malloc((size_t)(count > 0 ? count : 1) * sizeof *kept);
    if (kept == NULL) qiq_fatal("out of memory");
    int64_t kept_count = 0;
    // Values a map can hold as keys are found with one; others compared one by one.
    int key_type = strchr("idbsc", descriptor[0]) && descriptor[1] == '\0' ? (int)(strchr("idbsc", descriptor[0]) - "idbsc") : -1;
    void *seen = key_type >= 0 ? qiq_map_new((uint32_t)key_type, 0) : NULL;
    for (int64_t index = 0; index < count; index++) {
        uint64_t value = qiq_load(data, index, size);
        bool duplicate = false;
        if (seen != NULL) {
            if (key_type >= 3 && value == 0) {
                // A null string: compared by hand, as map keys can't be null.
                for (int64_t other = 0; other < kept_count && !duplicate; other++) duplicate = qiq_load(data, kept[other], size) == 0;
            } else {
                duplicate = qiq_map_has(seen, value);
                if (!duplicate) {
                    if (key_type >= 3) qiq_retain((void *)(uintptr_t)value);
                    qiq_map_set(seen, value, 1);
                }
            }
        } else {
            for (int64_t other = 0; other < kept_count && !duplicate; other++) {
                duplicate = qiq_payload_equal(descriptor, qiq_load(data, kept[other], size), value);
            }
        }
        if (!duplicate) kept[kept_count++] = index;
    }
    if (seen != NULL) qiq_release(seen);
    void *result = qiq_array_pick(array, size, flags, kept, kept_count);
    free(kept);
    return result;
}

// array_sum(array): of ints (wrapping, as + does) or doubles.
QIQ_GUARDED(int64_t, qiq_array_sum_int, 0, (void *array), (array)) {
    qiq_require_array(array);
    const int64_t *data = QIQ_ARRAY_DATA(array);
    uint64_t sum = 0;
    for (int64_t index = 0; index < QIQ_ARRAY_LENGTH(array); index++) sum += (uint64_t)data[index];
    return (int64_t)sum;
}

QIQ_GUARDED(double, qiq_array_sum_double, 0, (void *array), (array)) {
    qiq_require_array(array);
    const double *data = QIQ_ARRAY_DATA(array);
    double sum = 0;
    for (int64_t index = 0; index < QIQ_ARRAY_LENGTH(array); index++) sum += data[index];
    return sum;
}
