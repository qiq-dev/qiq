// Exceptions raised by the runtime, and reporting of uncaught exceptions.
#include <setjmp.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "qiq_runtime.h"

int qiq_format_int(char *buffer, int64_t value) {
    static const char pairs[201] =
        "00010203040506070809101112131415161718192021222324252627282930313233343536373839"
        "40414243444546474849505152535455565758596061626364656667686970717273747576777879"
        "8081828384858687888990919293949596979899";
    // Digits are written backwards from the end of `digits`. Negating as
    // unsigned also covers INT64_MIN, whose magnitude has no int64_t.
    char digits[20];
    char *end = digits + sizeof digits;
    char *start = end;
    uint64_t magnitude = value < 0 ? 0 - (uint64_t)value : (uint64_t)value;
    while (magnitude >= 100) {
        unsigned pair = (unsigned)(magnitude % 100) * 2;
        magnitude /= 100;
        *--start = pairs[pair + 1];
        *--start = pairs[pair];
    }
    if (magnitude >= 10) {
        *--start = pairs[magnitude * 2 + 1];
        *--start = pairs[magnitude * 2];
    } else {
        *--start = (char)('0' + magnitude);
    }
    int length = 0;
    if (value < 0) buffer[length++] = '-';
    memcpy(buffer + length, start, (size_t)(end - start));
    length += (int)(end - start);
    buffer[length] = '\0';
    return length;
}

// The exception being propagated, or NULL. Compiled code checks it after
// every call that can throw.
void *qiq_pending_exception = NULL;

// Defined in every compiled program: creates the exception object for `kind`
// with `message` (a qiq string) and stores it in qiq_pending_exception.
extern void qiq_raise(int32_t kind, const char *message);

// The innermost guarded runtime call (see QIQ_GUARDED).
static jmp_buf *qiq_guard = NULL;

jmp_buf *qiq_guard_enter(jmp_buf *environment) {
    jmp_buf *previous = qiq_guard;
    qiq_guard = environment;
    return previous;
}

void qiq_guard_leave(jmp_buf *previous) {
    qiq_guard = previous;
}

_Noreturn void qiq_rethrow(jmp_buf *previous) {
    qiq_guard = previous;
    if (previous == NULL) qiq_fatal("internal error: an exception was rethrown with no handler");
    longjmp(*previous, 1);
}

_Noreturn void qiq_fatal(const char *message) {
    fprintf(stderr, "qiq runtime error: %s\n", message);
    exit(1);
}

_Noreturn void qiq_throw(QiqExceptionKind kind, const char *message) {
    if (qiq_guard == NULL) qiq_fatal(message);
    jmp_buf *guard = qiq_guard;
    // The exception keeps its own reference to the message.
    char *text = qiq_string_from(message, strlen(message));
    qiq_raise((int32_t)kind, text);
    qiq_release(text);
    longjmp(*guard, 1);
}

_Noreturn void qiq_throwf(QiqExceptionKind kind, const char *format, ...) {
    va_list arguments;
    va_start(arguments, format);
    va_list copy;
    va_copy(copy, arguments);
    int length = vsnprintf(NULL, 0, format, copy);
    va_end(copy);
    char *message = malloc(length < 0 ? 1 : (size_t)length + 1);
    if (message == NULL) qiq_fatal("out of memory");
    vsnprintf(message, (size_t)length + 1, format, arguments);
    va_end(arguments);
    if (qiq_guard == NULL) qiq_fatal(message);
    char *text = qiq_string_from(message, strlen(message));
    free(message);
    jmp_buf *guard = qiq_guard;
    qiq_raise((int32_t)kind, text);
    qiq_release(text);
    longjmp(*guard, 1);
}

// Called by `main` when an exception reaches the top level.
_Noreturn void qiq_uncaught(const char *class_name, const char *message) {
    fflush(stdout);
    if (message != NULL && message[0] != 0) {
        fprintf(stderr, "qiq: uncaught %s: %s\n", class_name, message);
    } else {
        fprintf(stderr, "qiq: uncaught %s\n", class_name);
    }
    exit(1);
}
