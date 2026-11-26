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

// For fiber.c: each fiber has its own chain of guarded calls.
jmp_buf *qiq_guard_get(void) {
    return qiq_guard;
}

void qiq_guard_set(jmp_buf *guard) {
    qiq_guard = guard;
}

// A database error's details, from qiq_set_database_error until the
// DatabaseException being raised takes them.
static bool qiq_database_error_set = false;
static char qiq_database_error_state[6];
static int64_t qiq_database_error_number;

void qiq_set_database_error(const char *state, int64_t code) {
    snprintf(qiq_database_error_state, sizeof qiq_database_error_state, "%s", state != NULL && state[0] != '\0' ? state : "HY000");
    qiq_database_error_number = code;
    qiq_database_error_set = true;
}

bool qiq_stdlib___database_error_pending(void) {
    return qiq_database_error_set;
}

int64_t qiq_stdlib___database_error_code(void) {
    return qiq_database_error_number;
}

// The pending error's SQLSTATE, which it then stops being, or HY000.
char *qiq_stdlib___database_error_state(void) {
    const char *state = qiq_database_error_set ? qiq_database_error_state : "HY000";
    qiq_database_error_set = false;
    return qiq_string_from(state, strlen(state));
}

void qiq_discard_exception(void) {
    if (qiq_pending_exception != NULL) qiq_release(qiq_pending_exception);
    qiq_pending_exception = NULL;
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
    // An exception already pending (a task's wait was cancelled, and the
    // runtime function then failed for that) is the one that's thrown.
    if (qiq_pending_exception != NULL) longjmp(*guard, 1);
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
    if (qiq_pending_exception != NULL) {
        free(message);
        longjmp(*qiq_guard, 1);
    }
    char *text = qiq_string_from(message, strlen(message));
    free(message);
    jmp_buf *guard = qiq_guard;
    qiq_raise((int32_t)kind, text);
    qiq_release(text);
    longjmp(*guard, 1);
}

// "caller (file:line)": a frame of an exception's stack trace, made by
// Exception::__locate as the exception passes through the program's code.
char *qiq_stdlib___trace_frame(const char *caller, const char *file, int64_t line) {
    int length = snprintf(NULL, 0, "%s (%s:%lld)", caller, file, (long long)line);
    char *text = malloc((size_t)length + 1);
    if (text == NULL) qiq_fatal("out of memory");
    snprintf(text, (size_t)length + 1, "%s (%s:%lld)", caller, file, (long long)line);
    char *frame = qiq_string_from(text, (size_t)length);
    free(text);
    return frame;
}

// The most frames an uncaught exception's trace prints: the innermost ones,
// and then how many more there were (as from deep recursion).
#define QIQ_TRACE_PRINTED 40

// Called by `main` when an exception reaches the top level, with where it
// was thrown (file is "" and line 0 when that isn't known) and its trace (a
// string[] of "name (file:line)", innermost first). The trace is printed
// when it has more than one frame: one would only repeat the first line.
_Noreturn void qiq_uncaught(const char *class_name, const char *message, const char *file, int64_t line, void *trace) {
    fflush(stdout);
    fputs("qiq: ", stderr);
    if (file != NULL && file[0] != 0 && line > 0) {
        fprintf(stderr, "%s:%lld: ", file, (long long)line);
    }
    if (message != NULL && message[0] != 0) {
        fprintf(stderr, "uncaught %s: %s\n", class_name, message);
    } else {
        fprintf(stderr, "uncaught %s\n", class_name);
    }
    int64_t frames = trace == NULL ? 0 : QIQ_ARRAY_LENGTH(trace);
    if (frames > 1) {
        char **data = QIQ_ARRAY_DATA(trace);
        for (int64_t index = 0; index < frames && index < QIQ_TRACE_PRINTED; index++) {
            fprintf(stderr, "    at %s\n", data[index]);
        }
        if (frames > QIQ_TRACE_PRINTED) {
            fprintf(stderr, "    ... and %lld more\n", (long long)(frames - QIQ_TRACE_PRINTED));
        }
    }
    exit(1);
}
