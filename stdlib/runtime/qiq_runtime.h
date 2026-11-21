// Shared declarations for the qiq C runtime.
#ifndef QIQ_RUNTIME_H
#define QIQ_RUNTIME_H

#include <setjmp.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

// Which exception class a runtime failure raises. The compiler maps each kind
// to a class in stdlib/exceptions.qiq; keep the two lists in the same order.
typedef enum {
    QIQ_RUNTIME_EXCEPTION = 0,
    QIQ_ARITHMETIC_EXCEPTION = 1,
    QIQ_DIVISION_BY_ZERO_EXCEPTION = 2,
    QIQ_KEY_NOT_FOUND_EXCEPTION = 3,
    QIQ_INVALID_UTF8_EXCEPTION = 4,
    QIQ_INVALID_ARGUMENT_EXCEPTION = 5,
    QIQ_IO_EXCEPTION = 6,
    QIQ_FILE_NOT_FOUND_EXCEPTION = 7,
    QIQ_PERMISSION_DENIED_EXCEPTION = 8,
    QIQ_OUT_OF_RANGE_EXCEPTION = 9,
    QIQ_JSON_EXCEPTION = 10,
    QIQ_NETWORK_EXCEPTION = 11,
} QiqExceptionKind;

// Raises a qiq exception of `kind` and abandons the current runtime call: the
// guarded entry point (see QIQ_GUARDED) returns a placeholder, and compiled
// code sees the pending exception right after the call.
_Noreturn void qiq_throw(QiqExceptionKind kind, const char *message);

// Like qiq_throw, with a printf-style message.
_Noreturn void qiq_throwf(QiqExceptionKind kind, const char *format, ...);

// Prints an error and exits; for failures no exception can report, such as
// running out of memory.
_Noreturn void qiq_fatal(const char *message);

// Writes `value` in decimal and a NUL to `buffer`, which must hold at least
// QIQ_INT_TEXT_SIZE bytes, and returns the number of digits and sign written.
// Several times faster than musl's snprintf("%lld").
#define QIQ_INT_TEXT_SIZE 21
int qiq_format_int(char *buffer, int64_t value);

// ---------------------------------------------------------------------------
// Memory (memory.c). Every value a qiq program can hold a reference to (a
// string, char, array, map, or object) has a QiqHeader just before the
// address the program uses, so a string is still a plain char pointer.
// Values are reference counted: a new value starts at 1, and the last
// qiq_release frees it, releasing the values it holds. String literals have
// static headers and are never freed.

typedef struct {
    int64_t references;   // > 0 while in use; QIQ_STATIC_REFERENCES for literals
    uint32_t kind;        // QIQ_KIND_*, which includes QIQ_KIND_MAGIC
    uint32_t flags;       // QIQ_*_ARE_REFERENCES
} QiqHeader;

#define QIQ_HEADER(value) ((QiqHeader *)((char *)(value) - sizeof(QiqHeader)))
#define QIQ_STATIC_REFERENCES (-1)

#define QIQ_KIND_MAGIC 0x71690000u
#define QIQ_KIND_STRING (QIQ_KIND_MAGIC | 1u)
#define QIQ_KIND_ARRAY (QIQ_KIND_MAGIC | 2u)
#define QIQ_KIND_MAP (QIQ_KIND_MAGIC | 3u)
#define QIQ_KIND_OBJECT (QIQ_KIND_MAGIC | 4u)

// Array flag: the elements are references (strings, arrays, objects, ...).
#define QIQ_ELEMENTS_ARE_REFERENCES 1u
// Map flags.
#define QIQ_KEYS_ARE_REFERENCES 1u
#define QIQ_VALUES_ARE_REFERENCES 2u
// The value can be part of a reference cycle: an object with a field that
// can hold an object, or an array (elements) or map (values) whose type can.
// Only these are tracked by the cycle collector.
#define QIQ_MAY_CYCLE 4u
// The cycle collector's state, in the upper bits of the flags.
#define QIQ_COLOR_MASK (3u << 8)
#define QIQ_BLACK 0u                 // in use (or not yet examined)
#define QIQ_GRAY (1u << 8)           // being examined
#define QIQ_WHITE (2u << 8)          // garbage, unless found to be reachable
#define QIQ_PURPLE (3u << 8)         // a possible root of a garbage cycle
#define QIQ_BUFFERED (1u << 10)      // in the list of possible roots

// A zero-filled value of `size` bytes with one reference.
void *qiq_allocate(size_t size, uint32_t kind, uint32_t flags);
// A string of `length` bytes plus a NUL; the caller fills in the bytes.
char *qiq_string_new(size_t length);
// A copy of `length` bytes as a string.
char *qiq_string_from(const char *bytes, size_t length);
// Turns a block from malloc(sizeof(QiqHeader) + length + 1), whose text
// starts after the header, into a string without copying it.
char *qiq_string_adopt(void *block, size_t length);

void qiq_retain(void *value);
void qiq_release(void *value);

// Frees a map's entries, releasing keys and values it holds (map.c).
void qiq_map_drop(void *map);
// For the cycle collector: calls `visit` with each non-null value of a map
// whose values may be in a cycle, and frees a garbage map's entries,
// releasing only the keys and values that can't be in a cycle.
void qiq_map_visit_values(void *map, void (*visit)(void *value));
void qiq_map_free_collected(void *map);
// The number of entries, and copies of the keys and values into arrays of
// that length (in table order), for code that walks a whole map.
size_t qiq_map_size(void *map);
void qiq_map_collect(void *map, uint64_t *keys, uint64_t *values);
// The key type tag of a map (0 int, 1 double, 2 bool, 3 string, 4 char).
uint32_t qiq_map_key_type(void *map);

// Finds and frees garbage reference cycles; returns how many values it
// freed. Runs automatically as possible roots accumulate (memory.c).
int64_t qiq_collect_cycles(void);

// Arrays are a pointer to {int64 length; void *data; int64 capacity},
// followed by room for the elements the array was created with; `data`
// points there, or to a separate buffer once the array has grown (array.c).
// A null pointer is an array without a value.
#define QIQ_ARRAY_INLINE 24
#define QIQ_ARRAY_LENGTH(array) (*(int64_t *)(array))
#define QIQ_ARRAY_DATA(array) (*(void **)((char *)(array) + 8))
#define QIQ_ARRAY_CAPACITY(array) (*(int64_t *)((char *)(array) + 16))
#define QIQ_ARRAY_INLINE_DATA(array) ((void *)((char *)(array) + QIQ_ARRAY_INLINE))

// Frees a grown array's separate buffer.
void qiq_array_free_storage(void *array);

// Allocates a zero-filled array of `count` elements of `element_size` bytes,
// whose elements are references when `references` is true. For runtime
// functions that build arrays; `count` must not be negative.
void *qiq_array_create(int64_t count, int64_t element_size, int references);

jmp_buf *qiq_guard_enter(jmp_buf *environment);
void qiq_guard_leave(jmp_buf *previous);

// Defines a public runtime function `name` that can raise exceptions. Write
// the body after the macro; it becomes `name##_impl`. If the body raises, the
// function returns `fallback`.
//
//   QIQ_GUARDED(int32_t, qiq_int_divide, 0, (int32_t a, int32_t b), (a, b)) {
//       if (b == 0) qiq_throw(QIQ_DIVISION_BY_ZERO_EXCEPTION, "division by zero");
//       return a / b;
//   }
#define QIQ_GUARDED(return_type, name, fallback, parameters, arguments) \
    static return_type name##_impl parameters;                              \
    return_type name parameters {                                           \
        jmp_buf environment;                                                \
        jmp_buf *previous = qiq_guard_enter(&environment);                  \
        if (setjmp(environment)) {                                          \
            qiq_guard_leave(previous);                                      \
            return fallback;                                                \
        }                                                                   \
        return_type result = name##_impl arguments;                         \
        qiq_guard_leave(previous);                                          \
        return result;                                                      \
    }                                                                       \
    static return_type name##_impl parameters

// QIQ_GUARDED for functions returning void.
#define QIQ_GUARDED_VOID(name, parameters, arguments)                       \
    static void name##_impl parameters;                                     \
    void name parameters {                                                  \
        jmp_buf environment;                                                \
        jmp_buf *previous = qiq_guard_enter(&environment);                  \
        if (setjmp(environment)) {                                          \
            qiq_guard_leave(previous);                                      \
            return;                                                         \
        }                                                                   \
        name##_impl arguments;                                              \
        qiq_guard_leave(previous);                                          \
    }                                                                       \
    static void name##_impl parameters

#endif
