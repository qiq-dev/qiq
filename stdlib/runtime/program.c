// The running program: command-line arguments, exiting with a status, and
// environment variables.
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "qiq_runtime.h"

static int qiq_argument_count = 0;
static char **qiq_arguments = NULL;

// Called first thing by every program's main.
void qiq_set_arguments(int32_t argc, char **argv) {
    qiq_argument_count = argc;
    qiq_arguments = argv;
}

// Whether the text is valid UTF-8, as every qiq string must be.
static bool qiq_valid_utf8(const unsigned char *text) {
    while (*text != '\0') {
        unsigned char first = *text;
        size_t width = first < 0x80 ? 1
            : (first & 0xe0) == 0xc0 ? 2
            : (first & 0xf0) == 0xe0 ? 3
            : (first & 0xf8) == 0xf0 ? 4
            : 0;
        if (width == 0) return false;
        for (size_t offset = 1; offset < width; offset++) {
            if ((text[offset] & 0xc0) != 0x80) return false;
        }
        text += width;
    }
    return true;
}

// argv(): the program (element 0) and its arguments, as a new array.
QIQ_GUARDED(void *, qiq_stdlib_argv, NULL, (void), ()) {
    void *array = qiq_array_create(qiq_argument_count, sizeof(char *), QIQ_ELEMENTS_ARE_REFERENCES);
    char **data = QIQ_ARRAY_DATA(array);
    for (int index = 0; index < qiq_argument_count; index++) {
        const char *argument = qiq_arguments[index];
        if (!qiq_valid_utf8((const unsigned char *)argument)) {
            qiq_release(array);
            qiq_throwf(QIQ_INVALID_UTF8_EXCEPTION, "command-line argument %d is not valid UTF-8", index);
        }
        data[index] = qiq_string_from(argument, strlen(argument));
    }
    return array;
}

// exit(status): flushes output and ends the program. finally blocks don't run.
QIQ_GUARDED_VOID(qiq_stdlib_exit, (int64_t status), (status)) {
    if (status < 0 || status > 255) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "exit status must be 0 to 255, found %lld", (long long)status);
    }
    fflush(stdout);
    exit((int)status);
}

// getenv(name): the variable's value, or null when it isn't set.
QIQ_GUARDED(char *, qiq_stdlib_getenv, NULL, (const char *name), (name)) {
    if (name == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "getenv: the name is null");
    const char *value = getenv(name);
    if (value == NULL) return NULL;
    if (!qiq_valid_utf8((const unsigned char *)value)) {
        qiq_throwf(QIQ_INVALID_UTF8_EXCEPTION, "environment variable %s is not valid UTF-8", name);
    }
    return qiq_string_from(value, strlen(value));
}
