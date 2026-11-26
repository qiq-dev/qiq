// Running other programs: exec and shell in std/os (stdlib/program.qiq).
//
// The child is started with fork and exec. Between them it only calls
// async-signal-safe functions: it connects its standard input, output, and
// error to pipes, changes directory, closes every other file descriptor (so
// it can't inherit the program's files, sockets, or database connections),
// and restores the default SIGPIPE handling that net.c turns off. If exec
// fails, the child writes errno to a close-on-exec pipe, so the parent can
// tell "not found" from a program that ran and failed.
//
// The parent writes the input and reads both outputs at once with poll, so
// a child that fills one pipe while the parent waits on the other can't
// deadlock it.
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

#include "qiq_runtime.h"

// From fiber.c: poll() that lets other tasks (std/async) run while waiting,
// and sleeping that does.
int qiq_poll(struct pollfd *fds, nfds_t count, int timeout_ms);
int qiq_fibers_active(void);
void qiq_fiber_sleep(uint64_t nanoseconds);

#ifndef SYS_close_range
#define SYS_close_range 436
#endif

extern char **environ;

// A growing byte buffer for one of the child's outputs.
typedef struct {
    char *data;
    size_t length;
    size_t capacity;
} QiqBuffer;

// The outputs of the last command, until __process_output takes them.
static QiqBuffer qiq_process_outputs[2];

static void qiq_buffer_append(QiqBuffer *buffer, const char *bytes, size_t count) {
    if (buffer->length + count > buffer->capacity) {
        size_t capacity = buffer->capacity == 0 ? 4096 : buffer->capacity;
        while (capacity < buffer->length + count) capacity *= 2;
        char *data = realloc(buffer->data, capacity);
        if (data == NULL) qiq_fatal("out of memory");
        buffer->data = data;
        buffer->capacity = capacity;
    }
    memcpy(buffer->data + buffer->length, bytes, count);
    buffer->length += count;
}

static void qiq_buffer_free(QiqBuffer *buffer) {
    free(buffer->data);
    *buffer = (QiqBuffer){0};
}

// What a failed child reports through its error pipe.
typedef struct {
    int stage;  // 1: changing directory, 2: exec
    int error;  // errno
} QiqChildError;

static int64_t qiq_milliseconds(void) {
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    return (int64_t)now.tv_sec * 1000 + now.tv_nsec / 1000000;
}

static void qiq_close(int *fd) {
    if (*fd >= 0) close(*fd);
    *fd = -1;
}

// In the child: closes every descriptor from `low` (3) up, except `keep`
// (which is at least `low`). Uses close_range (Linux 5.9), or else closes
// them one at a time up to `limit`.
static void qiq_close_from(int low, int keep, int limit) {
    bool closed = true;
    if (keep > low) closed = syscall(SYS_close_range, (unsigned)low, (unsigned)keep - 1, 0) == 0;
    if (closed) closed = syscall(SYS_close_range, (unsigned)keep + 1, ~0u, 0) == 0;
    if (closed) return;
    for (int fd = low; fd < limit; fd++) {
        if (fd != keep) close(fd);
    }
}

// The environment for the child: the program's, with `names` set to `values`.
static char **qiq_child_environment(void *names, void *values) {
    int64_t count = QIQ_ARRAY_LENGTH(names);
    char **name_data = QIQ_ARRAY_DATA(names);
    char **value_data = QIQ_ARRAY_DATA(values);
    size_t existing = 0;
    while (environ[existing] != NULL) existing++;
    char **result = calloc(existing + (size_t)count + 1, sizeof(char *));
    if (result == NULL) qiq_fatal("out of memory");
    size_t used = 0;
    for (size_t index = 0; index < existing; index++) {
        const char *entry = environ[index];
        const char *equals = strchr(entry, '=');
        size_t length = equals == NULL ? strlen(entry) : (size_t)(equals - entry);
        bool replaced = false;
        for (int64_t name = 0; name < count && !replaced; name++) {
            replaced = strlen(name_data[name]) == length && memcmp(name_data[name], entry, length) == 0;
        }
        if (!replaced) result[used++] = (char *)entry;
    }
    for (int64_t index = 0; index < count; index++) {
        size_t length = strlen(name_data[index]) + strlen(value_data[index]) + 2;
        char *entry = malloc(length);
        if (entry == NULL) qiq_fatal("out of memory");
        snprintf(entry, length, "%s=%s", name_data[index], value_data[index]);
        result[used++] = entry;
    }
    return result;
}

// Frees what qiq_child_environment allocated: the array and the `count`
// entries it added at its end.
static void qiq_free_environment(char **environment, int64_t count) {
    if (environment == NULL) return;
    size_t length = 0;
    while (environment[length] != NULL) length++;
    for (size_t index = length - (size_t)count; index < length; index++) free(environment[index]);
    free(environment);
}

static void qiq_check_arguments(void *command, void *names, void *values, int64_t timeout) {
    if (command == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "exec: the command is null");
    int64_t length = QIQ_ARRAY_LENGTH(command);
    char **data = QIQ_ARRAY_DATA(command);
    if (length == 0) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "exec: the command is empty; give the program and its arguments, as in [\"ls\", \"-l\"]");
    for (int64_t index = 0; index < length; index++) {
        if (data[index] == NULL) qiq_throwf(QIQ_RUNTIME_EXCEPTION, "exec: element %lld of the command is null", (long long)index);
    }
    if (data[0][0] == '\0') qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "exec: the program name is empty");
    if (timeout < 0) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "exec: the timeout must be 0 (none) or more seconds, found %lld", (long long)timeout);
    if (names != NULL) {
        int64_t count = QIQ_ARRAY_LENGTH(names);
        char **name_data = QIQ_ARRAY_DATA(names);
        char **value_data = QIQ_ARRAY_DATA(values);
        for (int64_t index = 0; index < count; index++) {
            const char *name = name_data[index];
            if (name == NULL || name[0] == '\0' || strchr(name, '=') != NULL) {
                qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "exec: \"%s\" isn't an environment variable name (it's empty or contains =)", name == NULL ? "" : name);
            }
            if (value_data[index] == NULL) qiq_throwf(QIQ_RUNTIME_EXCEPTION, "exec: the value of environment variable %s is null", name);
        }
    }
}

// __process_run(command, input, cwd, envNames, envValues, timeout): runs the
// command, waits for it, and returns its exit status (128 + the signal's
// number if a signal ended it). Its outputs wait in qiq_process_outputs.
QIQ_GUARDED(int64_t, qiq_stdlib___process_run, 0,
            (void *command, const char *input, const char *cwd, void *names, void *values, int64_t timeout),
            (command, input, cwd, names, values, timeout)) {
    qiq_check_arguments(command, names, values, timeout);
    for (int stream = 0; stream < 2; stream++) qiq_buffer_free(&qiq_process_outputs[stream]);

    char **arguments = QIQ_ARRAY_DATA(command);
    // A copy with a terminating NULL, as exec needs.
    int64_t argument_count = QIQ_ARRAY_LENGTH(command);
    char **argv = calloc((size_t)argument_count + 1, sizeof(char *));
    if (argv == NULL) qiq_fatal("out of memory");
    memcpy(argv, arguments, (size_t)argument_count * sizeof(char *));
    int64_t added = names == NULL ? 0 : QIQ_ARRAY_LENGTH(names);
    char **environment = names == NULL ? NULL : qiq_child_environment(names, values);
    long open_max = sysconf(_SC_OPEN_MAX);
    int limit = open_max < 0 || open_max > 65536 ? 65536 : (int)open_max;

    // [0] read end, [1] write end.
    int in[2] = {-1, -1}, out[2] = {-1, -1}, err[2] = {-1, -1}, report[2] = {-1, -1};
    int null_fd = -1;
    bool ok = pipe2(out, O_CLOEXEC) == 0 && pipe2(err, O_CLOEXEC) == 0 && pipe2(report, O_CLOEXEC) == 0;
    if (ok) ok = input != NULL ? pipe2(in, O_CLOEXEC) == 0 : (null_fd = open("/dev/null", O_RDONLY | O_CLOEXEC)) >= 0;
    pid_t pid = ok ? fork() : -1;
    if (pid == 0) {
        // The child: only async-signal-safe calls from here on.
        int child_in = input != NULL ? in[0] : null_fd;
        if (dup2(child_in, 0) < 0 || dup2(out[1], 1) < 0 || dup2(err[1], 2) < 0) _exit(127);
        QiqChildError failure = {0, 0};
        if (cwd != NULL && chdir(cwd) != 0) {
            failure = (QiqChildError){1, errno};
        } else {
            qiq_close_from(3, report[1], limit);
            struct sigaction action;
            memset(&action, 0, sizeof action);
            action.sa_handler = SIG_DFL;
            sigaction(SIGPIPE, &action, NULL);
            sigset_t none;
            sigemptyset(&none);
            sigprocmask(SIG_SETMASK, &none, NULL);
            if (environment != NULL) {
                execvpe(argv[0], argv, environment);
            } else {
                execvp(argv[0], argv);
            }
            failure = (QiqChildError){2, errno};
        }
        ssize_t ignored = write(report[1], &failure, sizeof failure);
        (void)ignored;
        _exit(127);
    }
    int fork_error = errno;
    qiq_close(&in[0]);
    qiq_close(&out[1]);
    qiq_close(&err[1]);
    qiq_close(&report[1]);
    qiq_close(&null_fd);
    free(argv);
    qiq_free_environment(environment, added);
    const char *program = arguments[0];
    if (pid < 0) {
        int fds[] = {in[1], out[0], err[0], report[0]};
        for (int index = 0; index < 4; index++) qiq_close(&fds[index]);
        qiq_throwf(QIQ_PROCESS_EXCEPTION, "could not run %s: %s", program, strerror(fork_error));
    }

    // Did exec work? The pipe closes on exec, so this read ends then.
    QiqChildError failure;
    ssize_t got;
    do {
        got = read(report[0], &failure, sizeof failure);
    } while (got < 0 && errno == EINTR);
    qiq_close(&report[0]);
    if (got == (ssize_t)sizeof failure) {
        qiq_close(&in[1]);
        qiq_close(&out[0]);
        qiq_close(&err[0]);
        while (waitpid(pid, NULL, 0) < 0 && errno == EINTR) {}
        if (failure.stage == 1) {
            qiq_throwf(QIQ_PROCESS_EXCEPTION, "could not run %s: working directory %s: %s", program, cwd, strerror(failure.error));
        }
        if (failure.error == ENOENT) {
            if (strchr(program, '/') != NULL) qiq_throwf(QIQ_PROCESS_EXCEPTION, "could not run %s: no such file", program);
            qiq_throwf(QIQ_PROCESS_EXCEPTION, "command not found: %s", program);
        }
        if (failure.error == EACCES) qiq_throwf(QIQ_PROCESS_EXCEPTION, "could not run %s: permission denied", program);
        qiq_throwf(QIQ_PROCESS_EXCEPTION, "could not run %s: %s", program, strerror(failure.error));
    }

    // Feed the input and collect the outputs.
    struct sigaction ignore, previous_pipe;
    memset(&ignore, 0, sizeof ignore);
    ignore.sa_handler = SIG_IGN;
    sigaction(SIGPIPE, &ignore, &previous_pipe);
    size_t input_length = input == NULL ? 0 : strlen(input);
    size_t input_sent = 0;
    if (in[1] >= 0) {
        fcntl(in[1], F_SETFL, fcntl(in[1], F_GETFL) | O_NONBLOCK);
        if (input_length == 0) qiq_close(&in[1]);
    }
    int64_t deadline = timeout > 0 ? qiq_milliseconds() + timeout * 1000 : -1;
    bool timed_out = false;
    char chunk[65536];
    // Collected here, not in qiq_process_outputs: other tasks may run commands
    // while this one waits.
    QiqBuffer outputs[2] = {{0}, {0}};
    while (out[0] >= 0 || err[0] >= 0) {
        struct pollfd fds[3];
        int watched = 0;
        int *sources[3];
        if (out[0] >= 0) { fds[watched] = (struct pollfd){out[0], POLLIN, 0}; sources[watched++] = &out[0]; }
        if (err[0] >= 0) { fds[watched] = (struct pollfd){err[0], POLLIN, 0}; sources[watched++] = &err[0]; }
        if (in[1] >= 0) { fds[watched] = (struct pollfd){in[1], POLLOUT, 0}; sources[watched++] = &in[1]; }
        int wait_ms = -1;
        if (deadline >= 0) {
            int64_t left = deadline - qiq_milliseconds();
            if (left <= 0) { timed_out = true; break; }
            wait_ms = left > 1000000 ? 1000000 : (int)left;
        }
        int ready = qiq_poll(fds, (nfds_t)watched, wait_ms);
        if (ready < 0) {
            if (errno == EINTR) continue;
            break;
        }
        for (int index = 0; index < watched; index++) {
            if (fds[index].revents == 0) continue;
            int *fd = sources[index];
            if (fd == &in[1]) {
                ssize_t written = write(in[1], input + input_sent, input_length - input_sent);
                if (written > 0) input_sent += (size_t)written;
                // Done, or the child closed its input (EPIPE): stop writing.
                if (input_sent == input_length || (written < 0 && errno != EAGAIN && errno != EINTR)) qiq_close(&in[1]);
                continue;
            }
            ssize_t count = read(*fd, chunk, sizeof chunk);
            if (count > 0) {
                qiq_buffer_append(&outputs[fd == &out[0] ? 0 : 1], chunk, (size_t)count);
            } else if (count == 0 || (errno != EAGAIN && errno != EINTR)) {
                qiq_close(fd);
            }
        }
    }
    qiq_close(&in[1]);
    qiq_close(&out[0]);
    qiq_close(&err[0]);
    sigaction(SIGPIPE, &previous_pipe, NULL);
    // The task was cancelled while it waited: the command is stopped.
    if (qiq_pending_exception != NULL) {
        kill(pid, SIGKILL);
        while (waitpid(pid, NULL, 0) < 0 && errno == EINTR) {}
        for (int stream = 0; stream < 2; stream++) qiq_buffer_free(&outputs[stream]);
        qiq_throw(QIQ_CANCELLED_EXCEPTION, "the task was cancelled");
    }

    // Wait for it to exit, within the deadline. With other tasks, it checks
    // every few milliseconds, letting them run in between.
    int status = 0;
    while (!timed_out) {
        bool tasks = qiq_fibers_active();
        pid_t done = waitpid(pid, &status, deadline >= 0 || tasks ? WNOHANG : 0);
        if (done == pid) break;
        if (done < 0 && errno != EINTR) break;
        if (done == 0) {
            if (deadline >= 0 && qiq_milliseconds() >= deadline) {
                timed_out = true;
                break;
            }
            if (tasks) {
                qiq_fiber_sleep(2 * 1000000);
                if (qiq_pending_exception != NULL) {
                    kill(pid, SIGKILL);
                    while (waitpid(pid, NULL, 0) < 0 && errno == EINTR) {}
                    for (int stream = 0; stream < 2; stream++) qiq_buffer_free(&outputs[stream]);
                    qiq_throw(QIQ_CANCELLED_EXCEPTION, "the task was cancelled");
                }
            } else {
                struct timespec pause = {0, 5 * 1000000};
                nanosleep(&pause, NULL);
            }
        }
    }
    for (int stream = 0; stream < 2; stream++) {
        qiq_buffer_free(&qiq_process_outputs[stream]);
        qiq_process_outputs[stream] = outputs[stream];
    }
    if (timed_out) {
        kill(pid, SIGKILL);
        while (waitpid(pid, NULL, 0) < 0 && errno == EINTR) {}
        for (int stream = 0; stream < 2; stream++) qiq_buffer_free(&qiq_process_outputs[stream]);
        qiq_throwf(QIQ_PROCESS_EXCEPTION, "%s timed out after %lld second%s and was killed", program, (long long)timeout, timeout == 1 ? "" : "s");
    }

    static const char *stream_names[] = {"output", "error output"};
    for (int stream = 0; stream < 2; stream++) {
        QiqBuffer *buffer = &qiq_process_outputs[stream];
        if (!qiq_is_utf8_text((const unsigned char *)buffer->data, buffer->length)) {
            for (int other = 0; other < 2; other++) qiq_buffer_free(&qiq_process_outputs[other]);
            qiq_throwf(QIQ_INVALID_UTF8_EXCEPTION, "the %s of %s is not UTF-8 text", stream_names[stream], program);
        }
    }
    if (WIFEXITED(status)) return WEXITSTATUS(status);
    if (WIFSIGNALED(status)) return 128 + WTERMSIG(status);
    return 0;
}

// __process_output(stream): the last command's output (1) or error output
// (2) as a string, which is then forgotten.
QIQ_GUARDED(char *, qiq_stdlib___process_output, NULL, (int64_t stream), (stream)) {
    if (stream != 1 && stream != 2) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "__process_output: stream must be 1 or 2");
    QiqBuffer *buffer = &qiq_process_outputs[stream - 1];
    char *text = qiq_string_from(buffer->data == NULL ? "" : buffer->data, buffer->length);
    qiq_buffer_free(buffer);
    return text;
}

// ---------------------------------------------------------------------------
// Worker processes, for std/async's parallel: a forked copy of the program
// runs a function on another processor core and sends its result back
// through a pipe, as text (JSON, from the qiq side).

void qiq_fiber_become_root(void);

typedef struct {
    pid_t pid;
    int fd;   // the pipe's read end; -1 for a free slot
} QiqWorkerProcess;

static QiqWorkerProcess *qiq_workers = NULL;
static size_t qiq_worker_count = 0;
// In a worker process: where its result goes.
static int qiq_worker_result = -1;

// Forks a worker process: 0 in the worker, and in this process a handle for
// __process_result.
QIQ_GUARDED(int64_t, qiq_stdlib___process_fork, 0, (void), ()) {
    // Output not yet written would otherwise be written by both.
    fflush(NULL);
    int pipe_ends[2];
    if (pipe2(pipe_ends, O_CLOEXEC) != 0) qiq_throwf(QIQ_PROCESS_EXCEPTION, "could not start a worker process: %s", strerror(errno));
    pid_t pid = fork();
    if (pid < 0) {
        int error = errno;
        close(pipe_ends[0]);
        close(pipe_ends[1]);
        qiq_throwf(QIQ_PROCESS_EXCEPTION, "could not start a worker process: %s", strerror(error));
    }
    if (pid == 0) {
        close(pipe_ends[0]);
        qiq_worker_result = pipe_ends[1];
        // Its workers are its own, not the parent's.
        for (size_t index = 0; index < qiq_worker_count; index++) {
            if (qiq_workers[index].fd >= 0) close(qiq_workers[index].fd);
        }
        qiq_worker_count = 0;
        qiq_fiber_become_root();
        return 0;
    }
    close(pipe_ends[1]);
    fcntl(pipe_ends[0], F_SETFL, fcntl(pipe_ends[0], F_GETFL) | O_NONBLOCK);
    size_t slot = 0;
    while (slot < qiq_worker_count && qiq_workers[slot].fd >= 0) slot++;
    if (slot == qiq_worker_count) {
        QiqWorkerProcess *grown = realloc(qiq_workers, (qiq_worker_count + 1) * sizeof *grown);
        if (grown == NULL) qiq_fatal("out of memory");
        qiq_workers = grown;
        qiq_worker_count++;
    }
    qiq_workers[slot] = (QiqWorkerProcess){pid, pipe_ends[0]};
    return (int64_t)slot + 1;
}

// In a worker process: sends `payload` to the parent and ends the process
// (without the parent's exit handlers, which aren't the worker's).
void qiq_stdlib___process_finish(const char *payload) {
    size_t length = payload == NULL ? 0 : strlen(payload);
    size_t sent = 0;
    while (sent < length) {
        ssize_t wrote = write(qiq_worker_result, payload + sent, length - sent);
        if (wrote < 0) {
            if (errno == EINTR) continue;
            break;
        }
        sent += (size_t)wrote;
    }
    close(qiq_worker_result);
    fflush(NULL);
    _exit(0);
}

// Stops worker `slot`'s process and frees the slot.
static void qiq_end_worker(size_t slot, bool kill_it, int *status) {
    QiqWorkerProcess *worker = &qiq_workers[slot];
    if (worker->fd >= 0) close(worker->fd);
    worker->fd = -1;
    if (kill_it) kill(worker->pid, SIGKILL);
    // It has closed its end (or been killed), so it exits soon: with other
    // tasks, check every few milliseconds, letting them run.
    for (;;) {
        pid_t done = waitpid(worker->pid, status, qiq_fibers_active() ? WNOHANG : 0);
        if (done == worker->pid || (done < 0 && errno != EINTR)) break;
        if (done == 0) qiq_fiber_sleep(1000000);
    }
}

// The worker's result, once its process ends: what it sent (see
// std/async), or, if it ended without sending anything (it crashed or was
// killed), "e" and why. Other tasks run while it waits; cancelled, it stops
// the worker.
QIQ_GUARDED(char *, qiq_stdlib___process_result, NULL, (int64_t handle), (handle)) {
    if (handle < 1 || (size_t)handle > qiq_worker_count || qiq_workers[handle - 1].fd < 0) {
        qiq_throw(QIQ_PROCESS_EXCEPTION, "not a running worker process");
    }
    size_t slot = (size_t)handle - 1;
    QiqBuffer received = {0};
    char chunk[65536];
    for (;;) {
        ssize_t got = read(qiq_workers[slot].fd, chunk, sizeof chunk);
        if (got > 0) {
            qiq_buffer_append(&received, chunk, (size_t)got);
            continue;
        }
        if (got == 0) break;
        if (errno == EINTR) continue;
        if (errno != EAGAIN && errno != EWOULDBLOCK) break;
        struct pollfd poller = {qiq_workers[slot].fd, POLLIN, 0};
        if (qiq_poll(&poller, 1, -1) < 0 && qiq_pending_exception != NULL) {
            int ignored;
            qiq_end_worker(slot, true, &ignored);
            qiq_buffer_free(&received);
            qiq_throw(QIQ_CANCELLED_EXCEPTION, "the task was cancelled");
        }
    }
    int status = 0;
    qiq_end_worker(slot, false, &status);
    if (received.length == 0) {
        qiq_buffer_free(&received);
        char reason[200];
        if (WIFSIGNALED(status)) {
            snprintf(reason, sizeof reason, "eWorkerException: the worker process was killed by signal %d (%s)", WTERMSIG(status), strsignal(WTERMSIG(status)));
        } else {
            snprintf(reason, sizeof reason, "eWorkerException: the worker process exited with status %d without a result", WEXITSTATUS(status));
        }
        return qiq_string_from(reason, strlen(reason));
    }
    char *result = qiq_string_from(received.data, received.length);
    qiq_buffer_free(&received);
    return result;
}

int64_t qiq_stdlib___process_cpu_count(void) {
    long count = sysconf(_SC_NPROCESSORS_ONLN);
    return count > 0 ? count : 1;
}
