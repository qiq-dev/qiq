// Files: whole-file reading and writing, and existence checks. Paths are
// relative to the directory the program runs in. Failures raise
// FileNotFoundException, PermissionDeniedException, or IOException.
#include <dirent.h>
#include <errno.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#include "qiq_runtime.h"

// Whether the bytes are valid UTF-8 without NUL bytes, as qiq strings require.
static bool qiq_is_text(const unsigned char *bytes, size_t length) {
    size_t index = 0;
    while (index < length) {
        unsigned char first = bytes[index];
        if (first == 0) return false;
        size_t width = first < 0x80 ? 1
            : (first & 0xe0) == 0xc0 ? 2
            : (first & 0xf0) == 0xe0 ? 3
            : (first & 0xf8) == 0xf0 ? 4
            : 0;
        if (width == 0 || index + width > length) return false;
        for (size_t offset = 1; offset < width; offset++) {
            if ((bytes[index + offset] & 0xc0) != 0x80) return false;
        }
        index += width;
    }
    return true;
}

static const char *qiq_require_path(const char *path) {
    if (path == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "path is null");
    return path;
}

// Raises the exception matching `error` (an errno value) for `action` on `path`.
_Noreturn static void qiq_throw_file_error(int error, const char *action, const char *path) {
    QiqExceptionKind kind = QIQ_IO_EXCEPTION;
    if (error == ENOENT || (error == ENOTDIR && strcmp(action, "list") != 0)) kind = QIQ_FILE_NOT_FOUND_EXCEPTION;
    if (error == EACCES || error == EPERM) kind = QIQ_PERMISSION_DENIED_EXCEPTION;
    qiq_throwf(kind, "cannot %s %s: %s", action, path, strerror(error));
}

static bool qiq_is_text(const unsigned char *bytes, size_t length);

// qiq strings are NUL-terminated UTF-8 (the same rules as strings.c), so the
// contents must not contain NUL bytes or invalid UTF-8.
static void qiq_check_text(const unsigned char *bytes, size_t length, const char *path) {
    size_t index = 0;
    while (index < length) {
        unsigned char first = bytes[index];
        if (first == 0) {
            qiq_throwf(QIQ_INVALID_UTF8_EXCEPTION, "%s contains a NUL byte, which qiq strings cannot hold", path);
        }
        size_t width = first < 0x80 ? 1
            : (first & 0xe0) == 0xc0 ? 2
            : (first & 0xf0) == 0xe0 ? 3
            : (first & 0xf8) == 0xf0 ? 4
            : 0;
        if (width == 0 || index + width > length) {
            qiq_throwf(QIQ_INVALID_UTF8_EXCEPTION, "%s is not valid UTF-8", path);
        }
        for (size_t offset = 1; offset < width; offset++) {
            if ((bytes[index + offset] & 0xc0) != 0x80) {
                qiq_throwf(QIQ_INVALID_UTF8_EXCEPTION, "%s is not valid UTF-8", path);
            }
        }
        index += width;
    }
}

// Reads the whole file. Works for regular files and for files whose size
// isn't known in advance, such as pipes.
QIQ_GUARDED(char *, qiq_stdlib_file_get_contents, NULL, (const char *path), (path)) {
    qiq_require_path(path);
    struct stat info;
    if (stat(path, &info) == 0 && S_ISDIR(info.st_mode)) qiq_throw_file_error(EISDIR, "read", path);
    FILE *file = fopen(path, "rb");
    if (file == NULL) qiq_throw_file_error(errno, "read", path);
    // Read straight into a string's memory: the header, the text, and a NUL.
    size_t capacity = 4096;
    size_t length = 0;
    char *block = malloc(sizeof(QiqHeader) + capacity + 1);
    if (block == NULL) qiq_fatal("out of memory");
    for (;;) {
        if (length == capacity) {
            capacity *= 2;
            char *grown = realloc(block, sizeof(QiqHeader) + capacity + 1);
            if (grown == NULL) qiq_fatal("out of memory");
            block = grown;
        }
        char *contents = block + sizeof(QiqHeader);
        size_t read = fread(contents + length, 1, capacity - length, file);
        length += read;
        if (read == 0) break;
    }
    int failed = ferror(file);
    int error = errno;
    fclose(file);
    if (failed) {
        free(block);
        qiq_throw_file_error(error, "read", path);
    }
    char *shrunk = realloc(block, sizeof(QiqHeader) + length + 1);
    if (shrunk != NULL) block = shrunk;
    // Throws for text that isn't valid UTF-8 (leaking the block).
    qiq_check_text((const unsigned char *)block + sizeof(QiqHeader), length, path);
    return qiq_string_adopt(block, length);
}

// Writes `data` to the file, replacing it (or adding to the end when
// `append`), and returns the number of bytes written.
QIQ_GUARDED(int64_t, qiq_stdlib_file_put_contents, 0, (const char *path, const char *data, bool append), (path, data, append)) {
    qiq_require_path(path);
    if (data == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "data is null");
    FILE *file = fopen(path, append ? "ab" : "wb");
    if (file == NULL) qiq_throw_file_error(errno, "write", path);
    size_t length = strlen(data);
    size_t written = fwrite(data, 1, length, file);
    int error = errno;
    if (fclose(file) != 0 && written == length) {
        error = errno;
        written = 0;
    }
    if (written != length) qiq_throw_file_error(error, "write", path);
    return (int64_t)written;
}

QIQ_GUARDED(bool, qiq_stdlib_file_exists, false, (const char *path), (path)) {
    struct stat info;
    return stat(qiq_require_path(path), &info) == 0;
}

QIQ_GUARDED(bool, qiq_stdlib_is_file, false, (const char *path), (path)) {
    struct stat info;
    return stat(qiq_require_path(path), &info) == 0 && S_ISREG(info.st_mode);
}

QIQ_GUARDED(bool, qiq_stdlib_is_dir, false, (const char *path), (path)) {
    struct stat info;
    return stat(qiq_require_path(path), &info) == 0 && S_ISDIR(info.st_mode);
}

// The file's lines, without their line endings ("\n" or "\r\n"). A final line
// ending doesn't start another (empty) line.
QIQ_GUARDED(void *, qiq_stdlib_file, NULL, (const char *path), (path)) {
    char *text = qiq_stdlib_file_get_contents_impl(path);
    size_t length = strlen(text);
    int64_t count = 0;
    for (size_t index = 0; index < length; index++) {
        if (text[index] == '\n') count++;
    }
    if (length > 0 && text[length - 1] != '\n') count++;
    void *lines = qiq_array_create(count, sizeof(char *), 1);
    char **data = QIQ_ARRAY_DATA(lines);
    char *start = text;
    for (int64_t line = 0; line < count; line++) {
        char *end = strchr(start, '\n');
        size_t line_length = end == NULL ? strlen(start) : (size_t)(end - start);
        if (line_length > 0 && start[line_length - 1] == '\r') line_length--;
        data[line] = qiq_string_from(start, line_length);
        start = end == NULL ? start + strlen(start) : end + 1;
    }
    qiq_release(text);
    return lines;
}

static int qiq_compare_names(const void *left, const void *right) {
    return strcmp(*(char *const *)left, *(char *const *)right);
}

// The names in a directory, sorted by byte order, without "." and "..".
QIQ_GUARDED(void *, qiq_stdlib_scandir, NULL, (const char *path), (path)) {
    qiq_require_path(path);
    struct stat info;
    if (stat(path, &info) == 0 && !S_ISDIR(info.st_mode)) qiq_throw_file_error(ENOTDIR, "list", path);
    DIR *directory = opendir(path);
    if (directory == NULL) qiq_throw_file_error(errno, "list", path);
    size_t capacity = 16;
    size_t count = 0;
    char **names = malloc(capacity * sizeof(char *));
    if (names == NULL) qiq_fatal("out of memory");
    struct dirent *entry;
    while ((entry = readdir(directory)) != NULL) {
        if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) continue;
        if (count == capacity) {
            capacity *= 2;
            char **grown = realloc(names, capacity * sizeof(char *));
            if (grown == NULL) qiq_fatal("out of memory");
            names = grown;
        }
        size_t name_length = strlen(entry->d_name);
        names[count] = malloc(name_length + 1);
        if (names[count] == NULL) qiq_fatal("out of memory");
        memcpy(names[count], entry->d_name, name_length + 1);
        count++;
    }
    closedir(directory);
    for (size_t index = 0; index < count; index++) {
        qiq_check_text((const unsigned char *)names[index], strlen(names[index]), names[index]);
    }
    qsort(names, count, sizeof(char *), qiq_compare_names);
    void *result = qiq_array_create((int64_t)count, sizeof(char *), 1);
    char **data = QIQ_ARRAY_DATA(result);
    for (size_t index = 0; index < count; index++) {
        data[index] = qiq_string_from(names[index], strlen(names[index]));
        free(names[index]);
    }
    free(names);
    return result;
}

// The file's raw bytes, for binary files.
QIQ_GUARDED(void *, qiq_stdlib_file_get_bytes, NULL, (const char *path), (path)) {
    qiq_require_path(path);
    struct stat info;
    if (stat(path, &info) == 0 && S_ISDIR(info.st_mode)) qiq_throw_file_error(EISDIR, "read", path);
    FILE *file = fopen(path, "rb");
    if (file == NULL) qiq_throw_file_error(errno, "read", path);
    // A regular file is read straight into an array of its size. Reading into
    // a growing buffer and copying that into the array took ten times as long
    // for a 1 MB file, mostly in page faults for the extra memory.
    if (fstat(fileno(file), &info) == 0 && S_ISREG(info.st_mode) && info.st_size > 0) {
        size_t expected = (size_t)info.st_size;
        void *bytes = qiq_array_create((int64_t)expected, 1, 0);
        size_t read = fread(QIQ_ARRAY_DATA(bytes), 1, expected, file);
        if (read == expected && fgetc(file) == EOF && !ferror(file)) {
            fclose(file);
            return bytes;
        }
        // It changed size while being read, or a read failed: read it again
        // below, which handles both.
        qiq_release(bytes);
        rewind(file);
    }
    size_t capacity = 4096;
    size_t length = 0;
    unsigned char *contents = malloc(capacity);
    if (contents == NULL) qiq_fatal("out of memory");
    for (;;) {
        if (length == capacity) {
            capacity *= 2;
            unsigned char *grown = realloc(contents, capacity);
            if (grown == NULL) qiq_fatal("out of memory");
            contents = grown;
        }
        size_t read = fread(contents + length, 1, capacity - length, file);
        length += read;
        if (read == 0) break;
    }
    int failed = ferror(file);
    int error = errno;
    fclose(file);
    if (failed) {
        free(contents);
        qiq_throw_file_error(error, "read", path);
    }
    void *bytes = qiq_array_create((int64_t)length, 1, 0);
    memcpy(QIQ_ARRAY_DATA(bytes), contents, length);
    free(contents);
    return bytes;
}

// Writes the bytes, replacing the file or (with `append`) adding to its end;
// returns the number of bytes written.
QIQ_GUARDED(int64_t, qiq_stdlib_file_put_bytes, 0, (const char *path, void *data, bool append), (path, data, append)) {
    qiq_require_path(path);
    if (data == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "array is null (declared without a value)");
    FILE *file = fopen(path, append ? "ab" : "wb");
    if (file == NULL) qiq_throw_file_error(errno, "write", path);
    size_t length = (size_t)QIQ_ARRAY_LENGTH(data);
    size_t written = fwrite(QIQ_ARRAY_DATA(data), 1, length, file);
    int error = errno;
    if (fclose(file) != 0 && written == length) {
        error = errno;
        written = 0;
    }
    if (written != length) qiq_throw_file_error(error, "write", path);
    return (int64_t)written;
}

// ---------------------------------------------------------------------------
// Open files, for the File class in stdlib/file.qiq. A File holds a handle:
// an index into `qiq_files`. Handles are never reused, so a File that was
// closed can't reach a file opened later. Handles 0 to 2 are standard input,
// output, and error; closing one of them only flushes it.

typedef struct {
    FILE *stream;
    char *path;   // for error messages
} QiqOpenFile;

static QiqOpenFile *qiq_files = NULL;
static int64_t qiq_file_count = 0;
static int64_t qiq_file_capacity = 0;

static void qiq_files_init(void) {
    if (qiq_files != NULL) return;
    qiq_file_capacity = 16;
    qiq_files = calloc((size_t)qiq_file_capacity, sizeof *qiq_files);
    if (qiq_files == NULL) qiq_fatal("out of memory");
    qiq_files[0] = (QiqOpenFile){stdin, "standard input"};
    qiq_files[1] = (QiqOpenFile){stdout, "standard output"};
    qiq_files[2] = (QiqOpenFile){stderr, "standard error"};
    qiq_file_count = 3;
}

static QiqOpenFile *qiq_open_file(int64_t handle) {
    qiq_files_init();
    if (handle < 0 || handle >= qiq_file_count) qiq_throw(QIQ_IO_EXCEPTION, "invalid file handle");
    QiqOpenFile *file = &qiq_files[handle];
    if (file->stream == NULL) qiq_throwf(QIQ_IO_EXCEPTION, "%s is closed", file->path);
    return file;
}

QIQ_GUARDED(int64_t, qiq_stdlib___file_open, 0, (const char *path, const char *mode), (path, mode)) {
    qiq_require_path(path);
    if (mode == NULL) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "mode is null");
    static const char *const modes[][2] = {
        {"r", "rb"}, {"r+", "r+b"}, {"w", "wb"}, {"w+", "w+b"},
        {"a", "ab"}, {"a+", "a+b"}, {"x", "wbx"}, {"x+", "w+bx"},
    };
    const char *stdio_mode = NULL;
    for (size_t i = 0; i < sizeof modes / sizeof *modes; i++) {
        if (strcmp(mode, modes[i][0]) == 0) stdio_mode = modes[i][1];
    }
    if (stdio_mode == NULL) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION,
                   "invalid mode \"%s\" (expected \"r\", \"w\", \"a\", \"x\", or one of them followed by \"+\")", mode);
    }
    struct stat info;
    if (stat(path, &info) == 0 && S_ISDIR(info.st_mode)) qiq_throw_file_error(EISDIR, "open", path);
    FILE *stream = fopen(path, stdio_mode);
    if (stream == NULL) {
        int error = errno;
        if (error == EEXIST) qiq_throwf(QIQ_IO_EXCEPTION, "cannot open %s: it already exists", path);
        qiq_throw_file_error(error, "open", path);
    }
    qiq_files_init();
    if (qiq_file_count == qiq_file_capacity) {
        qiq_file_capacity *= 2;
        QiqOpenFile *grown = realloc(qiq_files, (size_t)qiq_file_capacity * sizeof *qiq_files);
        if (grown == NULL) qiq_fatal("out of memory");
        qiq_files = grown;
    }
    char *copy = strdup(path);
    if (copy == NULL) qiq_fatal("out of memory");
    qiq_files[qiq_file_count] = (QiqOpenFile){stream, copy};
    return qiq_file_count++;
}

// The next line without its line ending ("\n" or "\r\n"), or null at the end
// of the file. A last line without a line ending is still returned.
QIQ_GUARDED(char *, qiq_stdlib___file_read_line, NULL, (int64_t handle), (handle)) {
    QiqOpenFile *file = qiq_open_file(handle);
    char *line = NULL;
    size_t capacity = 0;
    errno = 0;
    ssize_t length = getline(&line, &capacity, file->stream);
    if (length < 0) {
        int error = errno;
        bool failed = ferror(file->stream);
        free(line);
        if (failed) {
            clearerr(file->stream);
            qiq_throw_file_error(error, "read", file->path);
        }
        return NULL;
    }
    if (length > 0 && line[length - 1] == '\n') length--;
    if (length > 0 && line[length - 1] == '\r') length--;
    if (!qiq_is_text((const unsigned char *)line, (size_t)length)) {
        free(line);
        qiq_throwf(QIQ_INVALID_UTF8_EXCEPTION, "%s contains a line that isn't valid UTF-8 text", file->path);
    }
    char *text = qiq_string_from(line, (size_t)length);
    free(line);
    return text;
}

// Up to `length` bytes; fewer only at the end of the file, and none after it.
QIQ_GUARDED(void *, qiq_stdlib___file_read, NULL, (int64_t handle, int64_t length), (handle, length)) {
    QiqOpenFile *file = qiq_open_file(handle);
    if (length < 0) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "read length must not be negative, found %lld", (long long)length);
    unsigned char *buffer = malloc(length > 0 ? (size_t)length : 1);
    if (buffer == NULL) qiq_fatal("out of memory");
    size_t read = fread(buffer, 1, (size_t)length, file->stream);
    if (read < (size_t)length && ferror(file->stream)) {
        int error = errno;
        clearerr(file->stream);
        free(buffer);
        qiq_throw_file_error(error, "read", file->path);
    }
    void *bytes = qiq_array_create((int64_t)read, 1, 0);
    memcpy(QIQ_ARRAY_DATA(bytes), buffer, read);
    free(buffer);
    return bytes;
}

static void qiq_file_write_raw(QiqOpenFile *file, const void *data, size_t length) {
    if (fwrite(data, 1, length, file->stream) != length) {
        int error = errno;
        clearerr(file->stream);
        qiq_throw_file_error(error, "write", file->path);
    }
}

QIQ_GUARDED_VOID(qiq_stdlib___file_write, (int64_t handle, const char *text), (handle, text)) {
    QiqOpenFile *file = qiq_open_file(handle);
    if (text == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "cannot write a null string");
    qiq_file_write_raw(file, text, strlen(text));
}

QIQ_GUARDED_VOID(qiq_stdlib___file_write_bytes, (int64_t handle, void *data), (handle, data)) {
    QiqOpenFile *file = qiq_open_file(handle);
    if (data == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "cannot write a null array");
    qiq_file_write_raw(file, QIQ_ARRAY_DATA(data), (size_t)QIQ_ARRAY_LENGTH(data));
}

// Whether nothing is left to read. Looks ahead one byte, so it's true before
// a read would come back empty (for standard input, it waits for input).
QIQ_GUARDED(bool, qiq_stdlib___file_eof, false, (int64_t handle), (handle)) {
    QiqOpenFile *file = qiq_open_file(handle);
    int next = getc(file->stream);
    if (next == EOF) {
        if (ferror(file->stream)) {
            int error = errno;
            clearerr(file->stream);
            qiq_throw_file_error(error, "read", file->path);
        }
        return true;
    }
    ungetc(next, file->stream);
    return false;
}

QIQ_GUARDED_VOID(qiq_stdlib___file_flush, (int64_t handle), (handle)) {
    QiqOpenFile *file = qiq_open_file(handle);
    if (fflush(file->stream) != 0) qiq_throw_file_error(errno, "write", file->path);
}

// Closing twice does nothing, so close() is safe in `finally`.
QIQ_GUARDED_VOID(qiq_stdlib___file_close, (int64_t handle), (handle)) {
    qiq_files_init();
    if (handle < 0 || handle >= qiq_file_count) qiq_throw(QIQ_IO_EXCEPTION, "invalid file handle");
    QiqOpenFile *file = &qiq_files[handle];
    if (file->stream == NULL) return;
    if (handle <= 2) {
        fflush(file->stream);
        return;
    }
    FILE *stream = file->stream;
    file->stream = NULL;
    if (fclose(stream) != 0) qiq_throw_file_error(errno, "write", file->path);
}
