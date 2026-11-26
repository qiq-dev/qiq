// Files: whole-file reading and writing, existence checks, paths, and
// directories. Paths are relative to the directory the program runs in.
// Failures raise FileNotFoundException, PermissionDeniedException, or
// IOException.
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <glob.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

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
    // ENOTDIR is a missing path (a part of it is a file), except where the
    // path itself had to be a directory.
    bool needs_directory = strcmp(action, "list") == 0 || strcmp(action, "remove directory") == 0
        || strcmp(action, "change to directory") == 0;
    if (error == ENOENT || (error == ENOTDIR && !needs_directory)) kind = QIQ_FILE_NOT_FOUND_EXCEPTION;
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
// ---------------------------------------------------------------------------
// Reading and writing whole files. With tasks (std/async), the I/O runs on a
// helper thread (qiq_offload) while other tasks run, so it works on copies
// and C memory only; qiq strings and arrays are made, and errors thrown,
// back on the task's own thread.

int qiq_fibers_active(void);
void qiq_offload(void (*work)(void *), void *context);

typedef struct {
    char *path;      // a copy
    size_t reserve;  // bytes kept free before the contents (a string's header)
    char *block;     // malloc'd: reserve, the contents, and room for a NUL
    size_t length;
    int error;       // errno, or 0
} QiqFileRead;

static void qiq_file_read_work(void *context) {
    QiqFileRead *job = context;
    struct stat info;
    if (stat(job->path, &info) == 0 && S_ISDIR(info.st_mode)) {
        job->error = EISDIR;
        return;
    }
    FILE *file = fopen(job->path, "rb");
    if (file == NULL) {
        job->error = errno;
        return;
    }
    size_t capacity = 4096;
    if (fstat(fileno(file), &info) == 0 && S_ISREG(info.st_mode) && info.st_size > 0) capacity = (size_t)info.st_size + 1;
    size_t length = 0;
    char *block = malloc(job->reserve + capacity + 1);
    if (block == NULL) qiq_fatal("out of memory");
    for (;;) {
        if (length == capacity) {
            capacity *= 2;
            char *grown = realloc(block, job->reserve + capacity + 1);
            if (grown == NULL) qiq_fatal("out of memory");
            block = grown;
        }
        size_t read = fread(block + job->reserve + length, 1, capacity - length, file);
        length += read;
        if (read == 0) break;
    }
    int failed = ferror(file);
    int error = errno;
    fclose(file);
    if (failed) {
        free(block);
        job->error = error;
        return;
    }
    job->block = block;
    job->length = length;
}

// Reads the file at `path` into a block with `reserve` bytes free at the
// start; throws if it can't be read.
static QiqFileRead qiq_file_read(const char *path, size_t reserve) {
    QiqFileRead job = {strdup(path), reserve, NULL, 0, 0};
    if (job.path == NULL) qiq_fatal("out of memory");
    qiq_offload(qiq_file_read_work, &job);
    free(job.path);
    if (job.error != 0) qiq_throw_file_error(job.error, "read", path);
    return job;
}

typedef struct {
    char *path;          // a copy
    const void *data;    // retained by the caller meanwhile
    size_t length;
    bool append;
    size_t written;
    int error;
    bool opened;
} QiqFileWrite;

static void qiq_file_write_work(void *context) {
    QiqFileWrite *job = context;
    FILE *file = fopen(job->path, job->append ? "ab" : "wb");
    if (file == NULL) {
        job->error = errno;
        return;
    }
    job->opened = true;
    job->written = fwrite(job->data, 1, job->length, file);
    job->error = errno;
    if (fclose(file) != 0 && job->written == job->length) {
        job->error = errno;
        job->written = 0;
    }
}

// Writes `length` bytes of `data` (a qiq string or array's memory, held by
// `owner`) to the file; throws if it can't.
static int64_t qiq_file_write(const char *path, void *owner, const void *data, size_t length, bool append) {
    QiqFileWrite job = {strdup(path), data, length, append, 0, 0, false};
    if (job.path == NULL) qiq_fatal("out of memory");
    // Another task can't free the data while a helper thread writes it.
    qiq_retain(owner);
    qiq_offload(qiq_file_write_work, &job);
    qiq_release(owner);
    free(job.path);
    if (job.written != length) qiq_throw_file_error(job.error, "write", path);
    return (int64_t)job.written;
}

QIQ_GUARDED(char *, qiq_stdlib_file_get_contents, NULL, (const char *path), (path)) {
    qiq_require_path(path);
    // Read straight into a string's memory: the header, the text, and a NUL.
    QiqFileRead read = qiq_file_read(path, sizeof(QiqHeader));
    char *block = read.block;
    char *shrunk = realloc(block, sizeof(QiqHeader) + read.length + 1);
    if (shrunk != NULL) block = shrunk;
    // Throws for text that isn't valid UTF-8 (leaking the block).
    qiq_check_text((const unsigned char *)block + sizeof(QiqHeader), read.length, path);
    return qiq_string_adopt(block, read.length);
}

// Writes `data` to the file, replacing it (or adding to the end when
// `append`), and returns the number of bytes written.
QIQ_GUARDED(int64_t, qiq_stdlib_file_put_contents, 0, (const char *path, const char *data, bool append), (path, data, append)) {
    qiq_require_path(path);
    if (data == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "data is null");
    return qiq_file_write(path, (void *)data, data, strlen(data), append);
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
    if (qiq_fibers_active()) {
        // On a helper thread, into C memory, then copied into the array.
        QiqFileRead read = qiq_file_read(path, 0);
        void *bytes = qiq_array_create((int64_t)read.length, 1, 0);
        memcpy(QIQ_ARRAY_DATA(bytes), read.block, read.length);
        free(read.block);
        return bytes;
    }
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
    return qiq_file_write(path, data, QIQ_ARRAY_DATA(data), (size_t)QIQ_ARRAY_LENGTH(data), append);
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

// ---------------------------------------------------------------------------
// Paths, taken apart and put together as text: these don't look at the disk.
// The rules are Go's path/filepath on Unix: "/" separates the parts, and
// empty parts, "." and ".." are resolved by qiq_clean.

// A copy of path[0..length) as a malloc'd C string.
static char *qiq_path_copy(const char *path, size_t length) {
    char *copy = malloc(length + 1);
    if (copy == NULL) qiq_fatal("out of memory");
    memcpy(copy, path, length);
    copy[length] = '\0';
    return copy;
}

// The shortest path naming the same place as path[0..length), as Go's
// filepath.Clean: one "/" between parts, no "." parts, and each ".." removed
// with the part before it (at the root, ".." is the root). "" is ".".
// Malloc'd.
static char *qiq_clean(const char *path, size_t length) {
    if (length == 0) return qiq_path_copy(".", 1);
    bool rooted = path[0] == '/';
    char *out = malloc(length + 2);
    if (out == NULL) qiq_fatal("out of memory");
    size_t read = 0, written = 0, dotdot = 0;
    if (rooted) {
        out[written++] = '/';
        read = 1;
        dotdot = 1;
    }
    while (read < length) {
        if (path[read] == '/') {
            read++;
        } else if (path[read] == '.' && (read + 1 == length || path[read + 1] == '/')) {
            read++;
        } else if (path[read] == '.' && path[read + 1] == '.' && (read + 2 == length || path[read + 2] == '/')) {
            read += 2;
            if (written > dotdot) {
                // Take back the last part.
                written--;
                while (written > dotdot && out[written] != '/') written--;
            } else if (!rooted) {
                // Nothing to take back: keep the "..".
                if (written > 0) out[written++] = '/';
                out[written++] = '.';
                out[written++] = '.';
                dotdot = written;
            }
        } else {
            if ((rooted && written != 1) || (!rooted && written != 0)) out[written++] = '/';
            while (read < length && path[read] != '/') out[written++] = path[read++];
        }
    }
    if (written == 0) out[written++] = '.';
    out[written] = '\0';
    return out;
}

// A malloc'd C string as a qiq string, freeing it.
static char *qiq_path_result(char *path) {
    char *result = qiq_string_from(path, strlen(path));
    free(path);
    return result;
}

// The length of `path` without trailing slashes, keeping a lone "/".
static size_t qiq_trimmed_length(const char *path) {
    size_t length = strlen(path);
    while (length > 1 && path[length - 1] == '/') length--;
    return length;
}

// The last part of the path, ignoring trailing slashes: "c" for "/a/b/c/".
// "/" for "/", and "." for "".
QIQ_GUARDED(char *, qiq_stdlib_basename, NULL, (const char *path), (path)) {
    qiq_require_path(path);
    if (path[0] == '\0') return qiq_string_from(".", 1);
    size_t length = qiq_trimmed_length(path);
    if (length == 1 && path[0] == '/') return qiq_string_from("/", 1);
    size_t start = length;
    while (start > 0 && path[start - 1] != '/') start--;
    return qiq_string_from(path + start, length - start);
}

// Everything but the last part, cleaned: "/a/b" for "/a/b/c/", "." for "c".
QIQ_GUARDED(char *, qiq_stdlib_dirname, NULL, (const char *path), (path)) {
    qiq_require_path(path);
    size_t length = qiq_trimmed_length(path);
    size_t end = length;
    while (end > 0 && path[end - 1] != '/') end--;
    if (end == 0) return qiq_string_from(".", 1);
    return qiq_path_result(qiq_clean(path, end));
}

// The last part's extension, from its last ".": ".gz" for "a.tar.gz", and
// "" for "README", ".bashrc", or "..".
QIQ_GUARDED(char *, qiq_stdlib_path_ext, NULL, (const char *path), (path)) {
    qiq_require_path(path);
    size_t length = qiq_trimmed_length(path);
    size_t start = length;
    while (start > 0 && path[start - 1] != '/') start--;
    size_t dot = length;
    while (dot > start && path[dot - 1] != '.') dot--;
    // No dot, or only a leading one (a hidden file's name, or "..").
    if (dot <= start + 1) return qiq_string_from("", 0);
    bool only_dots = true;
    for (size_t index = start; index < length; index++) {
        if (path[index] != '.') only_dots = false;
    }
    if (only_dots) return qiq_string_from("", 0);
    return qiq_string_from(path + dot - 1, length - (dot - 1));
}

// The parts joined with "/" and cleaned, skipping empty ones; "" when all are.
// An absolute part doesn't restart the path: ("a", "/b") is "a/b".
QIQ_GUARDED(char *, qiq_stdlib_path_join, NULL, (void *parts), (parts)) {
    if (parts == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "array is null (declared without a value)");
    int64_t count = QIQ_ARRAY_LENGTH(parts);
    char **data = QIQ_ARRAY_DATA(parts);
    size_t length = 0;
    for (int64_t index = 0; index < count; index++) {
        if (data[index] == NULL) qiq_throwf(QIQ_RUNTIME_EXCEPTION, "path_join: part %lld is null", (long long)index);
        length += strlen(data[index]) + 1;
    }
    char *joined = malloc(length + 1);
    if (joined == NULL) qiq_fatal("out of memory");
    size_t written = 0;
    for (int64_t index = 0; index < count; index++) {
        size_t part_length = strlen(data[index]);
        if (part_length == 0) continue;
        if (written > 0) joined[written++] = '/';
        memcpy(joined + written, data[index], part_length);
        written += part_length;
    }
    if (written == 0) {
        free(joined);
        return qiq_string_from("", 0);
    }
    char *clean = qiq_clean(joined, written);
    free(joined);
    return qiq_path_result(clean);
}

QIQ_GUARDED(char *, qiq_stdlib_path_clean, NULL, (const char *path), (path)) {
    qiq_require_path(path);
    return qiq_path_result(qiq_clean(path, strlen(path)));
}

QIQ_GUARDED(bool, qiq_stdlib_path_is_absolute, false, (const char *path), (path)) {
    return qiq_require_path(path)[0] == '/';
}

// The path that leads from `base` to `target`, as Go's filepath.Rel: "../c"
// from "a/b" to "a/c". Only the text is compared, so both must be absolute
// or both relative, and `base` can't climb above where it starts ("..").
QIQ_GUARDED(char *, qiq_stdlib_path_rel, NULL, (const char *base_path, const char *target_path), (base_path, target_path)) {
    qiq_require_path(base_path);
    qiq_require_path(target_path);
    char *base = qiq_clean(base_path, strlen(base_path));
    char *target = qiq_clean(target_path, strlen(target_path));
    if (strcmp(base, target) == 0) {
        free(base);
        free(target);
        return qiq_string_from(".", 1);
    }
    const char *b = strcmp(base, ".") == 0 ? "" : base;
    const char *t = target;
    if ((b[0] == '/') != (t[0] == '/')) {
        free(base);
        free(target);
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "path_rel: can't make %s relative to %s: one is absolute and the other isn't", target_path, base_path);
    }
    // Skip the parts they share.
    size_t base_length = strlen(b), target_length = strlen(t);
    size_t b0 = 0, bi = 0, t0 = 0, ti = 0;
    for (;;) {
        while (bi < base_length && b[bi] != '/') bi++;
        while (ti < target_length && t[ti] != '/') ti++;
        if (ti - t0 != bi - b0 || memcmp(t + t0, b + b0, bi - b0) != 0) break;
        if (bi < base_length) bi++;
        if (ti < target_length) ti++;
        b0 = bi;
        t0 = ti;
    }
    if (bi - b0 == 2 && b[b0] == '.' && b[b0 + 1] == '.') {
        free(base);
        free(target);
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "path_rel: can't make %s relative to %s, which is above where it starts", target_path, base_path);
    }
    // Up one ".." for each of base's other parts, then down target's.
    size_t ups = 0;
    if (b0 != base_length) {
        ups = 1;
        for (size_t index = b0; index < base_length; index++) {
            if (b[index] == '/') ups++;
        }
    }
    size_t rest = target_length - t0;
    char *relative = malloc(ups * 3 + rest + 2);
    if (relative == NULL) qiq_fatal("out of memory");
    size_t written = 0;
    for (size_t up = 0; up < ups; up++) {
        if (up > 0) relative[written++] = '/';
        relative[written++] = '.';
        relative[written++] = '.';
    }
    if (rest > 0) {
        if (written > 0) relative[written++] = '/';
        memcpy(relative + written, t + t0, rest);
        written += rest;
    }
    free(base);
    free(target);
    char *clean = qiq_clean(relative, written);
    free(relative);
    return qiq_path_result(clean);
}

// A path from the system as a qiq string, which must be valid UTF-8.
static char *qiq_system_path(const char *path) {
    qiq_check_text((const unsigned char *)path, strlen(path), path);
    return qiq_string_from(path, strlen(path));
}

// The absolute path with symbolic links, ".", and ".." resolved. The path
// must exist.
QIQ_GUARDED(char *, qiq_stdlib_realpath, NULL, (const char *path), (path)) {
    qiq_require_path(path);
    char *resolved = realpath(path, NULL);
    if (resolved == NULL) qiq_throw_file_error(errno, "resolve", path);
    char *result = qiq_system_path(resolved);
    free(resolved);
    return result;
}

// ---------------------------------------------------------------------------
// Directories and file operations

// Creates the directory; with `recursive`, also the missing directories above
// it, and it's not an error if it already exists as a directory.
QIQ_GUARDED_VOID(qiq_stdlib_mkdir, (const char *path, bool recursive), (path, recursive)) {
    qiq_require_path(path);
    if (!recursive) {
        if (mkdir(path, 0777) != 0) qiq_throw_file_error(errno, "create directory", path);
        return;
    }
    size_t length = strlen(path);
    char *partial = qiq_path_copy(path, length);
    for (size_t index = 1; index <= length; index++) {
        if (index < length && partial[index] != '/') continue;
        if (partial[index - 1] == '/') continue;
        char saved = partial[index];
        partial[index] = '\0';
        if (mkdir(partial, 0777) != 0) {
            int error = errno;
            struct stat info;
            if (error != EEXIST || stat(partial, &info) != 0 || !S_ISDIR(info.st_mode)) {
                char *message = qiq_path_copy(partial, strlen(partial));
                free(partial);
                qiq_throw_file_error(error, "create directory", message);
            }
        }
        partial[index] = saved;
    }
    free(partial);
}

// Removes `path` and, if it's a directory (not a link to one), everything in it.
static void qiq_remove_tree(const char *path) {
    struct stat info;
    if (lstat(path, &info) != 0) qiq_throw_file_error(errno, "remove", path);
    if (!S_ISDIR(info.st_mode)) {
        if (unlink(path) != 0) qiq_throw_file_error(errno, "remove", path);
        return;
    }
    DIR *directory = opendir(path);
    if (directory == NULL) qiq_throw_file_error(errno, "remove directory", path);
    size_t path_length = strlen(path);
    struct dirent *entry;
    while ((entry = readdir(directory)) != NULL) {
        if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) continue;
        size_t name_length = strlen(entry->d_name);
        char *child = malloc(path_length + name_length + 2);
        if (child == NULL) qiq_fatal("out of memory");
        memcpy(child, path, path_length);
        size_t at = path_length;
        if (at > 0 && child[at - 1] != '/') child[at++] = '/';
        memcpy(child + at, entry->d_name, name_length + 1);
        qiq_remove_tree(child);
        free(child);
    }
    closedir(directory);
    if (rmdir(path) != 0) qiq_throw_file_error(errno, "remove directory", path);
}

// Removes the directory, which must be empty; with `recursive`, everything in
// it too. Links inside are removed, not followed.
QIQ_GUARDED_VOID(qiq_stdlib_rmdir, (const char *path, bool recursive), (path, recursive)) {
    qiq_require_path(path);
    struct stat info;
    if (lstat(path, &info) != 0) qiq_throw_file_error(errno, "remove directory", path);
    if (!S_ISDIR(info.st_mode)) qiq_throw_file_error(ENOTDIR, "remove directory", path);
    if (recursive) {
        qiq_remove_tree(path);
    } else if (rmdir(path) != 0) {
        qiq_throw_file_error(errno, "remove directory", path);
    }
}

// Deletes a file (or link); directories are removed with rmdir.
QIQ_GUARDED_VOID(qiq_stdlib_unlink, (const char *path), (path)) {
    qiq_require_path(path);
    struct stat info;
    if (lstat(path, &info) == 0 && S_ISDIR(info.st_mode)) qiq_throw_file_error(EISDIR, "delete", path);
    if (unlink(path) != 0) qiq_throw_file_error(errno, "delete", path);
}

// "from to to", for messages about two paths.
static char *qiq_two_paths(const char *from, const char *to) {
    size_t length = strlen(from) + strlen(to) + 5;
    char *text = malloc(length);
    if (text == NULL) qiq_fatal("out of memory");
    snprintf(text, length, "%s to %s", from, to);
    return text;
}

// Moves or renames a file or directory, replacing a file at `to`.
QIQ_GUARDED_VOID(qiq_stdlib_rename, (const char *from, const char *to), (from, to)) {
    qiq_require_path(from);
    qiq_require_path(to);
    if (rename(from, to) != 0) {
        int error = errno;
        qiq_throw_file_error(error, "rename", qiq_two_paths(from, to));
    }
}

// Copies a file's contents and permissions to `to`, replacing it.
QIQ_GUARDED_VOID(qiq_stdlib_copy, (const char *from, const char *to), (from, to)) {
    qiq_require_path(from);
    qiq_require_path(to);
    struct stat source_info, target_info;
    if (stat(from, &source_info) != 0) qiq_throw_file_error(errno, "copy", qiq_two_paths(from, to));
    if (S_ISDIR(source_info.st_mode)) qiq_throw_file_error(EISDIR, "copy", qiq_two_paths(from, to));
    if (stat(to, &target_info) == 0 && target_info.st_dev == source_info.st_dev && target_info.st_ino == source_info.st_ino) {
        qiq_throwf(QIQ_IO_EXCEPTION, "cannot copy %s to %s: they are the same file", from, to);
    }
    int source = open(from, O_RDONLY);
    if (source < 0) qiq_throw_file_error(errno, "copy", qiq_two_paths(from, to));
    int target = open(to, O_WRONLY | O_CREAT | O_TRUNC, source_info.st_mode & 0777);
    if (target < 0) {
        int error = errno;
        close(source);
        qiq_throw_file_error(error, "copy", qiq_two_paths(from, to));
    }
    char buffer[65536];
    for (;;) {
        ssize_t read_count = read(source, buffer, sizeof buffer);
        if (read_count == 0) break;
        if (read_count < 0) {
            if (errno == EINTR) continue;
            int error = errno;
            close(source);
            close(target);
            qiq_throw_file_error(error, "copy", qiq_two_paths(from, to));
        }
        for (ssize_t done = 0; done < read_count;) {
            ssize_t written = write(target, buffer + done, (size_t)(read_count - done));
            if (written < 0) {
                if (errno == EINTR) continue;
                int error = errno;
                close(source);
                close(target);
                qiq_throw_file_error(error, "copy", qiq_two_paths(from, to));
            }
            done += written;
        }
    }
    close(source);
    // Permissions as the source's, even when `to` already existed.
    fchmod(target, source_info.st_mode & 07777);
    if (close(target) != 0) qiq_throw_file_error(errno, "copy", qiq_two_paths(from, to));
}

// The file's size in bytes.
QIQ_GUARDED(int64_t, qiq_stdlib_filesize, 0, (const char *path), (path)) {
    struct stat info;
    if (stat(qiq_require_path(path), &info) != 0) qiq_throw_file_error(errno, "get the size of", path);
    return (int64_t)info.st_size;
}

// When the file was last changed, in seconds since 1970 (a timestamp, as
// std/time's DateTime::fromTimestamp takes).
QIQ_GUARDED(int64_t, qiq_stdlib_filemtime, 0, (const char *path), (path)) {
    struct stat info;
    if (stat(qiq_require_path(path), &info) != 0) qiq_throw_file_error(errno, "get the modification time of", path);
    return (int64_t)info.st_mtime;
}

// The directory the program is running in, which relative paths start from.
QIQ_GUARDED(char *, qiq_stdlib_getcwd, NULL, (void), ()) {
    char *directory = getcwd(NULL, 0);
    if (directory == NULL) qiq_throwf(QIQ_IO_EXCEPTION, "cannot get the current directory: %s", strerror(errno));
    char *result = qiq_system_path(directory);
    free(directory);
    return result;
}

QIQ_GUARDED_VOID(qiq_stdlib_chdir, (const char *path), (path)) {
    qiq_require_path(path);
    if (chdir(path) != 0) qiq_throw_file_error(errno, "change to directory", path);
}

// $TMPDIR, or /tmp when it isn't set.
static const char *qiq_temp_directory(void) {
    const char *directory = getenv("TMPDIR");
    return directory != NULL && directory[0] != '\0' ? directory : "/tmp";
}

QIQ_GUARDED(char *, qiq_stdlib_temp_dir, NULL, (void), ()) {
    return qiq_system_path(qiq_temp_directory());
}

// Creates a new, empty file in temp_dir() that only this user can read and
// write, named `prefix` and six random characters, and returns its path.
QIQ_GUARDED(char *, qiq_stdlib_temp_file, NULL, (const char *prefix), (prefix)) {
    if (prefix == NULL) prefix = "";
    if (strchr(prefix, '/') != NULL) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "temp_file: the prefix can't contain '/': %s", prefix);
    const char *directory = qiq_temp_directory();
    size_t directory_length = qiq_trimmed_length(directory);
    size_t length = directory_length + strlen(prefix) + 8;
    char *path = malloc(length);
    if (path == NULL) qiq_fatal("out of memory");
    snprintf(path, length, "%.*s%s%sXXXXXX", (int)directory_length, directory, directory_length == 1 && directory[0] == '/' ? "" : "/", prefix);
    int descriptor = mkstemp(path);
    if (descriptor < 0) {
        int error = errno;
        qiq_throw_file_error(error, "create a temporary file in", directory);
    }
    close(descriptor);
    char *result = qiq_system_path(path);
    free(path);
    return result;
}

// The paths matching a shell wildcard pattern ("*" and "?" within a part,
// "[abc]" for one of a set), sorted; [] when none do. Names starting with "."
// only match a pattern part that starts with ".".
QIQ_GUARDED(void *, qiq_stdlib_glob, NULL, (const char *pattern), (pattern)) {
    if (pattern == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "pattern is null");
    glob_t found;
    int status = glob(pattern, 0, NULL, &found);
    if (status == GLOB_NOMATCH) {
        globfree(&found);
        return qiq_array_create(0, sizeof(char *), 1);
    }
    if (status == GLOB_NOSPACE) qiq_fatal("out of memory");
    if (status != 0) {
        globfree(&found);
        qiq_throwf(QIQ_IO_EXCEPTION, "glob: cannot read the directories for %s", pattern);
    }
    for (size_t index = 0; index < found.gl_pathc; index++) {
        qiq_check_text((const unsigned char *)found.gl_pathv[index], strlen(found.gl_pathv[index]), found.gl_pathv[index]);
    }
    void *result = qiq_array_create((int64_t)found.gl_pathc, sizeof(char *), 1);
    char **data = QIQ_ARRAY_DATA(result);
    for (size_t index = 0; index < found.gl_pathc; index++) {
        data[index] = qiq_string_from(found.gl_pathv[index], strlen(found.gl_pathv[index]));
    }
    globfree(&found);
    return result;
}
