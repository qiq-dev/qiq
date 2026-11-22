// Network connections, for the Socket class (stdlib/net.qiq) and the HTTP
// server (stdlib/server.qiq): TCP, with TLS from Mbed TLS. A socket is
// referred to by a handle: a slot index in the low 32 bits and the slot's
// generation above them. Closing a socket frees it and makes its slot
// reusable with the next generation, so a stale handle is detected rather
// than reaching a newer socket.
//
// Sockets are non-blocking, and every connect, read, and write waits with
// poll() for at most the socket's timeout, so a stalled server can't hang a
// program. TLS always verifies the server's certificate and hostname
// against the system's CA certificates (or SSL_CERT_FILE / SSL_CERT_DIR).
// Failures throw NetworkException.
#include <errno.h>
#include <fcntl.h>
#include <netdb.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <poll.h>
#include <pthread.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <sys/sendfile.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

#include "mbedtls/ctr_drbg.h"
#include "mbedtls/entropy.h"
#include "mbedtls/error.h"
#include "mbedtls/net_sockets.h"
#include "mbedtls/ssl.h"
#include "mbedtls/x509_crt.h"
#include "psa/crypto.h"
#include "qiq_runtime.h"

#define QIQ_SOCKET_BUFFER 16384

typedef struct {
    int fd;
    bool tls;
    bool open;
    bool eof;
    int timeout_ms;
    char peer[300];  // "host:port", for messages
    bool unix_listener;  // listening on the Unix socket file "unix:<path>" in peer
    mbedtls_ssl_context ssl;
    unsigned char buffer[QIQ_SOCKET_BUFFER];
    size_t start;
    size_t end;
    int64_t served;      // requests a server connection has carried
} QiqSocket;

typedef struct {
    QiqSocket *socket;
    uint32_t generation;
} QiqSocketSlot;

static QiqSocketSlot *qiq_slots = NULL;
static int64_t qiq_slot_count = 0;
static int64_t qiq_slot_capacity = 0;
static int64_t *qiq_free_slots = NULL;
static int64_t qiq_free_count = 0;

// ---------------------------------------------------------------------------
// TLS setup, done once: random numbers, the CA certificates, and the client
// configuration (TLS 1.2 and 1.3, certificates required).

static bool qiq_tls_ready = false;
static mbedtls_entropy_context qiq_entropy;
static mbedtls_ctr_drbg_context qiq_random;
static mbedtls_x509_crt qiq_authorities;
static mbedtls_ssl_config qiq_tls_config;

// Where Linux distributions keep their CA bundle (the list Go uses).
static const char *const qiq_ca_files[] = {
    "/etc/ssl/certs/ca-certificates.crt",                // Debian, Ubuntu, Arch, Alpine
    "/etc/pki/tls/certs/ca-bundle.crt",                  // Fedora, RHEL
    "/etc/ssl/ca-bundle.pem",                            // openSUSE
    "/etc/pki/tls/cacert.pem",                           // OpenELEC
    "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem", // CentOS, RHEL 7
    "/etc/ssl/cert.pem",                                 // Alpine
};

static void qiq_tls_error(int code, char *out, size_t size) {
    mbedtls_strerror(code, out, size);
}

// After a fork, the new process would otherwise draw the same random
// numbers as its parent and every other worker: Mbed TLS's generator and
// PSA's are both copied with the memory. Both are seeded again from the
// system's entropy, so each process's TLS keys and nonces are its own.
// (Connections the parent had open aren't usable in the child anyway.)
static bool qiq_tls_forked = false;

static void qiq_tls_after_fork(void) {
    if (!qiq_tls_ready) return;
    mbedtls_psa_crypto_free();
    qiq_tls_forked = psa_crypto_init() != PSA_SUCCESS;
    static const unsigned char process[] = "qiq child";
    if (mbedtls_ctr_drbg_reseed(&qiq_random, process, sizeof process - 1) != 0) qiq_tls_forked = true;
}

static void qiq_tls_setup(void) {
    // A reseed that failed in a forked process leaves TLS unusable there.
    if (qiq_tls_forked) qiq_throw(QIQ_NETWORK_EXCEPTION, "TLS: could not seed the random number generator after fork");
    if (qiq_tls_ready) return;
    if (psa_crypto_init() != PSA_SUCCESS) qiq_throw(QIQ_NETWORK_EXCEPTION, "TLS: could not initialize cryptography");
    pthread_atfork(NULL, NULL, qiq_tls_after_fork);
    mbedtls_entropy_init(&qiq_entropy);
    mbedtls_ctr_drbg_init(&qiq_random);
    static const unsigned char personal[] = "qiq";
    int code = mbedtls_ctr_drbg_seed(&qiq_random, mbedtls_entropy_func, &qiq_entropy, personal, sizeof personal - 1);
    if (code != 0) qiq_throw(QIQ_NETWORK_EXCEPTION, "TLS: could not seed the random number generator");

    mbedtls_x509_crt_init(&qiq_authorities);
    int loaded = 0;
    const char *file = getenv("SSL_CERT_FILE");
    const char *directory = getenv("SSL_CERT_DIR");
    if (file != NULL && file[0] != '\0') {
        if (mbedtls_x509_crt_parse_file(&qiq_authorities, file) < 0) {
            qiq_throwf(QIQ_NETWORK_EXCEPTION, "TLS: could not read the certificates in SSL_CERT_FILE (%s)", file);
        }
        loaded = 1;
    } else if (directory != NULL && directory[0] != '\0') {
        if (mbedtls_x509_crt_parse_path(&qiq_authorities, directory) < 0) {
            qiq_throwf(QIQ_NETWORK_EXCEPTION, "TLS: could not read the certificates in SSL_CERT_DIR (%s)", directory);
        }
        loaded = 1;
    } else {
        for (size_t index = 0; index < sizeof qiq_ca_files / sizeof *qiq_ca_files && !loaded; index++) {
            if (access(qiq_ca_files[index], R_OK) == 0 && mbedtls_x509_crt_parse_file(&qiq_authorities, qiq_ca_files[index]) >= 0) {
                loaded = 1;
            }
        }
        if (!loaded && mbedtls_x509_crt_parse_path(&qiq_authorities, "/etc/ssl/certs") >= 0 && qiq_authorities.version != 0) {
            loaded = 1;
        }
    }
    if (!loaded) {
        qiq_throw(QIQ_NETWORK_EXCEPTION,
                  "TLS: no CA certificates found; install your system's ca-certificates package or set SSL_CERT_FILE");
    }

    mbedtls_ssl_config_init(&qiq_tls_config);
    code = mbedtls_ssl_config_defaults(&qiq_tls_config, MBEDTLS_SSL_IS_CLIENT, MBEDTLS_SSL_TRANSPORT_STREAM,
                                       MBEDTLS_SSL_PRESET_DEFAULT);
    if (code != 0) qiq_throw(QIQ_NETWORK_EXCEPTION, "TLS: could not configure");
    mbedtls_ssl_conf_authmode(&qiq_tls_config, MBEDTLS_SSL_VERIFY_REQUIRED);
    mbedtls_ssl_conf_ca_chain(&qiq_tls_config, &qiq_authorities, NULL);
    mbedtls_ssl_conf_rng(&qiq_tls_config, mbedtls_ctr_drbg_random, &qiq_random);
    qiq_tls_ready = true;
}

// ---------------------------------------------------------------------------
// Waiting with a timeout

// Waits until the socket can be read (POLLIN) or written (POLLOUT).
static void qiq_wait(QiqSocket *socket, short events, const char *action) {
    struct pollfd poller = {socket->fd, events, 0};
    for (;;) {
        int ready = poll(&poller, 1, socket->timeout_ms);
        if (ready > 0) return;
        if (ready == 0) {
            qiq_throwf(QIQ_NETWORK_EXCEPTION, "timed out %s %s after %d second%s", action, socket->peer,
                       socket->timeout_ms / 1000, socket->timeout_ms == 1000 ? "" : "s");
        }
        if (errno != EINTR) qiq_throwf(QIQ_NETWORK_EXCEPTION, "%s %s: %s", action, socket->peer, strerror(errno));
    }
}

// Mbed TLS's I/O callbacks: non-blocking, so it reports when to wait.
static int qiq_tls_send(void *context, const unsigned char *data, size_t length) {
    QiqSocket *socket = context;
    ssize_t sent = send(socket->fd, data, length, MSG_NOSIGNAL);
    if (sent >= 0) return (int)sent;
    if (errno == EAGAIN || errno == EWOULDBLOCK || errno == EINTR) return MBEDTLS_ERR_SSL_WANT_WRITE;
    return MBEDTLS_ERR_NET_SEND_FAILED;
}

static int qiq_tls_receive(void *context, unsigned char *data, size_t length) {
    QiqSocket *socket = context;
    ssize_t received = recv(socket->fd, data, length, 0);
    if (received >= 0) return (int)received;
    if (errno == EAGAIN || errno == EWOULDBLOCK || errno == EINTR) return MBEDTLS_ERR_SSL_WANT_READ;
    return MBEDTLS_ERR_NET_RECV_FAILED;
}

// Waits as Mbed TLS asks, or returns false for any other result.
static bool qiq_tls_retry(QiqSocket *socket, int code, const char *action) {
    if (code == MBEDTLS_ERR_SSL_WANT_READ) {
        qiq_wait(socket, POLLIN, action);
        return true;
    }
    if (code == MBEDTLS_ERR_SSL_WANT_WRITE) {
        qiq_wait(socket, POLLOUT, action);
        return true;
    }
#ifdef MBEDTLS_ERR_SSL_RECEIVED_NEW_SESSION_TICKET
    if (code == MBEDTLS_ERR_SSL_RECEIVED_NEW_SESSION_TICKET) return true;
#endif
    return false;
}

// ---------------------------------------------------------------------------
// Handles

// Gives a new socket a slot, reusing a freed one when there is one.
static int64_t qiq_register_socket(QiqSocket *socket) {
    int64_t index;
    if (qiq_free_count > 0) {
        index = qiq_free_slots[--qiq_free_count];
    } else {
        if (qiq_slot_count == qiq_slot_capacity) {
            qiq_slot_capacity = qiq_slot_capacity ? qiq_slot_capacity * 2 : 16;
            QiqSocketSlot *grown = realloc(qiq_slots, (size_t)qiq_slot_capacity * sizeof *grown);
            int64_t *free_grown = realloc(qiq_free_slots, (size_t)qiq_slot_capacity * sizeof *free_grown);
            if (grown == NULL || free_grown == NULL) qiq_fatal("out of memory");
            qiq_slots = grown;
            qiq_free_slots = free_grown;
        }
        index = qiq_slot_count++;
        qiq_slots[index].generation = 0;
    }
    qiq_slots[index].socket = socket;
    return ((int64_t)qiq_slots[index].generation << 32) | index;
}

// The open socket for a handle, or NULL when it's closed or invalid.
static QiqSocket *qiq_lookup(int64_t handle) {
    int64_t index = handle & 0xffffffff;
    uint32_t generation = (uint32_t)((uint64_t)handle >> 32);
    if (handle < 0 || index >= qiq_slot_count) return NULL;
    QiqSocketSlot *slot = &qiq_slots[index];
    if (slot->generation != generation || slot->socket == NULL) return NULL;
    return slot->socket;
}

static QiqSocket *qiq_socket(int64_t handle) {
    QiqSocket *socket = qiq_lookup(handle);
    if (socket == NULL || !socket->open) qiq_throw(QIQ_NETWORK_EXCEPTION, "the socket is closed");
    return socket;
}

static void qiq_release_socket(QiqSocket *socket) {
    if (socket->tls) mbedtls_ssl_free(&socket->ssl);
    socket->tls = false;
    if (socket->fd >= 0) close(socket->fd);
    socket->fd = -1;
    socket->open = false;
}

// Closes and frees a socket, freeing its slot for reuse.
static void qiq_free_socket(int64_t handle) {
    QiqSocket *socket = qiq_lookup(handle);
    if (socket == NULL) return;
    qiq_release_socket(socket);
    free(socket);
    int64_t index = handle & 0xffffffff;
    qiq_slots[index].socket = NULL;
    qiq_slots[index].generation++;
    qiq_free_slots[qiq_free_count++] = index;
}

// Connects to the first address that accepts, waiting at most the timeout.
// Returns the descriptor, or -1 with the reason in `message`.
static int qiq_connect_tcp(QiqSocket *connection, const char *host, int64_t port, char *message, size_t size) {
    char service[16];
    snprintf(service, sizeof service, "%lld", (long long)port);
    struct addrinfo hints = {0};
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    struct addrinfo *addresses = NULL;
    int found = getaddrinfo(host, service, &hints, &addresses);
    if (found != 0) {
        if (found == EAI_NONAME || found == EAI_FAIL || found == EAI_NODATA) {
            snprintf(message, size, "could not find host %s", host);
        } else {
            snprintf(message, size, "could not look up host %s: %s", host, gai_strerror(found));
        }
        return -1;
    }
    int error = 0;
    bool timed_out = false;
    for (struct addrinfo *address = addresses; address != NULL; address = address->ai_next) {
        int fd = socket(address->ai_family, address->ai_socktype | SOCK_CLOEXEC | SOCK_NONBLOCK, address->ai_protocol);
        if (fd < 0) {
            error = errno;
            continue;
        }
        if (connect(fd, address->ai_addr, address->ai_addrlen) == 0) {
            freeaddrinfo(addresses);
            return fd;
        }
        if (errno == EINPROGRESS) {
            struct pollfd poller = {fd, POLLOUT, 0};
            int ready;
            do {
                ready = poll(&poller, 1, connection->timeout_ms);
            } while (ready < 0 && errno == EINTR);
            if (ready > 0) {
                socklen_t length = sizeof error;
                getsockopt(fd, SOL_SOCKET, SO_ERROR, &error, &length);
                if (error == 0) {
                    freeaddrinfo(addresses);
                    return fd;
                }
            } else if (ready == 0) {
                timed_out = true;
            } else {
                error = errno;
            }
        } else {
            error = errno;
        }
        close(fd);
    }
    freeaddrinfo(addresses);
    if (timed_out && error == 0) {
        snprintf(message, size, "timed out connecting to %s after %d second%s", connection->peer,
                 connection->timeout_ms / 1000, connection->timeout_ms == 1000 ? "" : "s");
    } else if (error == ECONNREFUSED) {
        snprintf(message, size, "connection refused by %s", connection->peer);
    } else {
        snprintf(message, size, "could not connect to %s: %s", connection->peer, strerror(error));
    }
    return -1;
}

// Runs the TLS handshake. On failure, writes the reason to `message` and
// returns false, so the caller can release the socket before throwing.
static bool qiq_tls_handshake(QiqSocket *socket, const char *host, const mbedtls_ssl_config *config, char *message, size_t size) {
    mbedtls_ssl_init(&socket->ssl);
    socket->tls = true;
    char reason[200];
    int code = mbedtls_ssl_setup(&socket->ssl, config);
    if (code == 0) code = mbedtls_ssl_set_hostname(&socket->ssl, host);
    if (code != 0) {
        qiq_tls_error(code, reason, sizeof reason);
        snprintf(message, size, "TLS setup for %s failed: %s", socket->peer, reason);
        return false;
    }
    mbedtls_ssl_set_bio(&socket->ssl, socket, qiq_tls_send, qiq_tls_receive, NULL);
    while ((code = mbedtls_ssl_handshake(&socket->ssl)) != 0) {
        if (code == MBEDTLS_ERR_SSL_WANT_READ || code == MBEDTLS_ERR_SSL_WANT_WRITE) {
            struct pollfd poller = {socket->fd, code == MBEDTLS_ERR_SSL_WANT_READ ? POLLIN : POLLOUT, 0};
            int ready;
            do {
                ready = poll(&poller, 1, socket->timeout_ms);
            } while (ready < 0 && errno == EINTR);
            if (ready > 0) continue;
            snprintf(message, size, "timed out during the TLS handshake with %s after %d second%s", socket->peer,
                     socket->timeout_ms / 1000, socket->timeout_ms == 1000 ? "" : "s");
            return false;
        }
        uint32_t flags = mbedtls_ssl_get_verify_result(&socket->ssl);
        if (code == MBEDTLS_ERR_X509_CERT_VERIFY_FAILED && flags != 0) {
            char details[400];
            mbedtls_x509_crt_verify_info(details, sizeof details, "", flags);
            // One line: the first reason Mbed TLS gives.
            char *newline = strchr(details, '\n');
            if (newline != NULL) *newline = '\0';
            snprintf(message, size, "TLS certificate of %s is not trusted: %s", socket->peer, details);
            return false;
        }
        qiq_tls_error(code, reason, sizeof reason);
        snprintf(message, size, "TLS handshake with %s failed: %s", socket->peer, reason);
        return false;
    }
    return true;
}

// Opens a connection to host:port, with TLS when `tls` is true. Returns a
// handle for the other functions.
QIQ_GUARDED(int64_t, qiq_stdlib___socket_connect, 0, (const char *host, int64_t port, bool tls, int64_t timeout),
            (host, port, tls, timeout)) {
    if (host == NULL || host[0] == '\0') qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "the host is empty");
    if (port < 1 || port > 65535) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "port %lld is not between 1 and 65535", (long long)port);
    if (timeout < 1) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "the timeout must be at least 1 second");
    QiqSocket *socket = calloc(1, sizeof *socket);
    if (socket == NULL) qiq_fatal("out of memory");
    socket->fd = -1;
    socket->timeout_ms = timeout > 2000000 ? 2000000000 : (int)(timeout * 1000);
    snprintf(socket->peer, sizeof socket->peer, strchr(host, ':') ? "[%s]:%lld" : "%s:%lld", host, (long long)port);
    char message[600];
    socket->fd = qiq_connect_tcp(socket, host, port, message, sizeof message);
    if (socket->fd < 0) {
        free(socket);
        qiq_throw(QIQ_NETWORK_EXCEPTION, message);
    }
    socket->open = true;
    int on = 1;
    setsockopt(socket->fd, IPPROTO_TCP, TCP_NODELAY, &on, sizeof on);
    if (tls) {
        qiq_tls_setup();
        if (!qiq_tls_handshake(socket, host, &qiq_tls_config, message, sizeof message)) {
            qiq_release_socket(socket);
            free(socket);
            qiq_throw(QIQ_NETWORK_EXCEPTION, message);
        }
    }
    return qiq_register_socket(socket);
}

// ---------------------------------------------------------------------------
// Writing

// Sends all of `data`. With `more`, the kernel holds a short write back
// until the next one (MSG_MORE), so a response's headers and body leave in
// the same packet rather than two.
static void qiq_send_flagged(QiqSocket *socket, const unsigned char *data, size_t length, bool more) {
    size_t sent = 0;
    while (sent < length) {
        if (socket->tls) {
            int code = mbedtls_ssl_write(&socket->ssl, data + sent, length - sent);
            if (code > 0) {
                sent += (size_t)code;
                continue;
            }
            if (qiq_tls_retry(socket, code, "writing to")) continue;
            char reason[200];
            qiq_tls_error(code, reason, sizeof reason);
            qiq_throwf(QIQ_NETWORK_EXCEPTION, "writing to %s failed: %s", socket->peer, reason);
        }
        ssize_t written = send(socket->fd, data + sent, length - sent, MSG_NOSIGNAL | (more ? MSG_MORE : 0));
        if (written > 0) {
            sent += (size_t)written;
        } else if (written < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) {
            qiq_wait(socket, POLLOUT, "writing to");
        } else if (written < 0 && errno != EINTR) {
            qiq_throwf(QIQ_NETWORK_EXCEPTION, "writing to %s failed: %s", socket->peer, strerror(errno));
        }
    }
}

static void qiq_send_all(QiqSocket *socket, const unsigned char *data, size_t length) {
    qiq_send_flagged(socket, data, length, false);
}

// Like __socket_write, for text that more will follow straight away (a
// response's headers before its body).
QIQ_GUARDED_VOID(qiq_stdlib___socket_write_more, (int64_t handle, const char *text), (handle, text)) {
    QiqSocket *socket = qiq_socket(handle);
    if (text == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "cannot write a null string");
    qiq_send_flagged(socket, (const unsigned char *)text, strlen(text), true);
}

QIQ_GUARDED_VOID(qiq_stdlib___socket_write, (int64_t handle, const char *text), (handle, text)) {
    QiqSocket *socket = qiq_socket(handle);
    if (text == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "cannot write a null string");
    qiq_send_all(socket, (const unsigned char *)text, strlen(text));
}

// For http_file_server, in two system calls: the length of the regular file
// at `path`, or -1 when there's no file or directory there, -2 for a
// directory, and -3 for a file that can't be read. A readable file is then
// streamed with __socket_send_file.
int64_t qiq_stdlib___server_file_info(const char *path) {
    struct stat info;
    if (path == NULL || stat(path, &info) != 0) return -1;
    if (S_ISDIR(info.st_mode)) return -2;
    if (!S_ISREG(info.st_mode)) return -1;
    return access(path, R_OK) == 0 ? (int64_t)info.st_size : -3;
}

// Sends the first `length` bytes of the file at `path`, straight from the
// file to the socket with sendfile (or, over TLS, through a 64 KB buffer),
// so a large file is never read into memory.
QIQ_GUARDED_VOID(qiq_stdlib___socket_send_file, (int64_t handle, const char *path, int64_t length),
                 (handle, path, length)) {
    QiqSocket *socket = qiq_socket(handle);
    if (path == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "file path is null");
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) qiq_throwf(QIQ_NETWORK_EXCEPTION, "sending %s failed: %s", path, strerror(errno));
    // Closes the file when anything below throws, then passes the exception on.
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        qiq_guard_leave(previous);
        close(fd);
        longjmp(*previous, 1);
    }
    off_t offset = 0;
    bool use_sendfile = !socket->tls;
    while (offset < length) {
        if (use_sendfile) {
            ssize_t sent = sendfile(socket->fd, fd, &offset, (size_t)(length - offset));
            if (sent > 0) continue;
            if (sent < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) {
                qiq_wait(socket, POLLOUT, "writing to");
                continue;
            }
            if (sent < 0 && errno == EINTR) continue;
            if (sent < 0 && (errno == EINVAL || errno == ENOSYS)) {
                // Not supported for this file or socket: copy it instead.
                use_sendfile = false;
                continue;
            }
            if (sent == 0) qiq_throwf(QIQ_NETWORK_EXCEPTION, "sending %s failed: it got shorter", path);
            qiq_throwf(QIQ_NETWORK_EXCEPTION, "writing to %s failed: %s", socket->peer, strerror(errno));
        }
        unsigned char buffer[65536];
        size_t wanted = (size_t)(length - offset) < sizeof buffer ? (size_t)(length - offset) : sizeof buffer;
        ssize_t got = pread(fd, buffer, wanted, offset);
        if (got < 0 && errno == EINTR) continue;
        if (got < 0) qiq_throwf(QIQ_NETWORK_EXCEPTION, "sending %s failed: %s", path, strerror(errno));
        if (got == 0) qiq_throwf(QIQ_NETWORK_EXCEPTION, "sending %s failed: it got shorter", path);
        qiq_send_all(socket, buffer, (size_t)got);
        offset += got;
    }
    qiq_guard_leave(previous);
    close(fd);
}

QIQ_GUARDED_VOID(qiq_stdlib___socket_write_bytes, (int64_t handle, void *data), (handle, data)) {
    QiqSocket *socket = qiq_socket(handle);
    if (data == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "cannot write a null array");
    qiq_send_all(socket, QIQ_ARRAY_DATA(data), (size_t)QIQ_ARRAY_LENGTH(data));
}

// ---------------------------------------------------------------------------
// Reading, through the socket's buffer

// Reads more into the buffer; returns false at the end of the stream.
static bool qiq_fill(QiqSocket *socket) {
    if (socket->eof) return false;
    if (socket->start == socket->end) socket->start = socket->end = 0;
    if (socket->end == QIQ_SOCKET_BUFFER) {
        memmove(socket->buffer, socket->buffer + socket->start, socket->end - socket->start);
        socket->end -= socket->start;
        socket->start = 0;
    }
    unsigned char *into = socket->buffer + socket->end;
    size_t room = QIQ_SOCKET_BUFFER - socket->end;
    for (;;) {
        if (socket->tls) {
            int code = mbedtls_ssl_read(&socket->ssl, into, room);
            if (code > 0) {
                socket->end += (size_t)code;
                return true;
            }
            // Servers often close without TLS's close_notify; both mean the end.
            if (code == 0 || code == MBEDTLS_ERR_SSL_PEER_CLOSE_NOTIFY || code == MBEDTLS_ERR_NET_CONN_RESET) {
                socket->eof = true;
                return false;
            }
            if (qiq_tls_retry(socket, code, "reading from")) continue;
            char reason[200];
            qiq_tls_error(code, reason, sizeof reason);
            qiq_throwf(QIQ_NETWORK_EXCEPTION, "reading from %s failed: %s", socket->peer, reason);
        }
        ssize_t received = recv(socket->fd, into, room, 0);
        if (received > 0) {
            socket->end += (size_t)received;
            return true;
        }
        if (received == 0) {
            socket->eof = true;
            return false;
        }
        if (errno == EAGAIN || errno == EWOULDBLOCK) {
            qiq_wait(socket, POLLIN, "reading from");
        } else if (errno != EINTR) {
            qiq_throwf(QIQ_NETWORK_EXCEPTION, "reading from %s failed: %s", socket->peer, strerror(errno));
        }
    }
}

// A string from bytes that should be UTF-8; invalid sequences become
// U+FFFD, since network data can't be trusted to be well formed.
static char *qiq_lenient_string(const unsigned char *bytes, size_t length) {
    char *text = malloc(length * 3 + 1);
    if (text == NULL) qiq_fatal("out of memory");
    size_t out = 0;
    size_t index = 0;
    while (index < length) {
        unsigned char first = bytes[index];
        size_t width = first < 0x80 ? 1 : (first & 0xe0) == 0xc0 ? 2 : (first & 0xf0) == 0xe0 ? 3 : (first & 0xf8) == 0xf0 ? 4 : 0;
        bool valid = width > 0 && first != 0 && index + width <= length;
        for (size_t offset = 1; valid && offset < width; offset++) valid = (bytes[index + offset] & 0xc0) == 0x80;
        if (valid) {
            memcpy(text + out, bytes + index, width);
            out += width;
            index += width;
        } else {
            memcpy(text + out, "\xef\xbf\xbd", 3);
            out += 3;
            index++;
        }
    }
    char *result = qiq_string_from(text, out);
    free(text);
    return result;
}

// The next line without "\n" or "\r\n", or null at the end of the stream.
QIQ_GUARDED(char *, qiq_stdlib___socket_read_line, NULL, (int64_t handle), (handle)) {
    QiqSocket *socket = qiq_socket(handle);
    unsigned char *line = NULL;
    size_t length = 0;
    size_t capacity = 0;
    for (;;) {
        unsigned char *newline = memchr(socket->buffer + socket->start, '\n', socket->end - socket->start);
        size_t take = newline != NULL ? (size_t)(newline - (socket->buffer + socket->start)) + 1 : socket->end - socket->start;
        if (length + take > capacity) {
            capacity = (length + take) * 2 + 64;
            unsigned char *grown = realloc(line, capacity);
            if (grown == NULL) qiq_fatal("out of memory");
            line = grown;
        }
        memcpy(line + length, socket->buffer + socket->start, take);
        length += take;
        socket->start += take;
        if (newline != NULL) break;
        if (!qiq_fill(socket)) {
            if (length == 0) {
                free(line);
                return NULL;
            }
            break;
        }
    }
    if (length > 0 && line[length - 1] == '\n') length--;
    if (length > 0 && line[length - 1] == '\r') length--;
    char *text = qiq_lenient_string(line, length);
    free(line);
    return text;
}

// Up to `limit` bytes: what's available, waiting only if nothing is. An
// empty array at the end of the stream.
QIQ_GUARDED(void *, qiq_stdlib___socket_read, NULL, (int64_t handle, int64_t limit), (handle, limit)) {
    QiqSocket *socket = qiq_socket(handle);
    if (limit < 0) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "read length must not be negative");
    if (socket->start == socket->end && limit > 0) qiq_fill(socket);
    size_t available = socket->end - socket->start;
    size_t count = (size_t)limit < available ? (size_t)limit : available;
    void *bytes = qiq_array_create((int64_t)count, 1, 0);
    memcpy(QIQ_ARRAY_DATA(bytes), socket->buffer + socket->start, count);
    socket->start += count;
    return bytes;
}

// Exactly `count` bytes, or all the rest of the stream when `count` is -1.
static void *qiq_read_to(QiqSocket *socket, int64_t count) {
    size_t capacity = count >= 0 ? (size_t)count : 65536;
    unsigned char *data = malloc(capacity > 0 ? capacity : 1);
    if (data == NULL) qiq_fatal("out of memory");
    size_t length = 0;
    for (;;) {
        size_t wanted = count >= 0 ? (size_t)count - length : SIZE_MAX;
        size_t available = socket->end - socket->start;
        size_t take = available < wanted ? available : wanted;
        if (length + take > capacity) {
            while (length + take > capacity) capacity *= 2;
            unsigned char *grown = realloc(data, capacity);
            if (grown == NULL) qiq_fatal("out of memory");
            data = grown;
        }
        memcpy(data + length, socket->buffer + socket->start, take);
        length += take;
        socket->start += take;
        if (count >= 0 && length == (size_t)count) break;
        if (!qiq_fill(socket)) {
            if (count >= 0) {
                free(data);
                qiq_throwf(QIQ_NETWORK_EXCEPTION, "%s closed the connection after %zu of %lld bytes", socket->peer, length,
                           (long long)count);
            }
            break;
        }
    }
    void *bytes = qiq_array_create((int64_t)length, 1, 0);
    memcpy(QIQ_ARRAY_DATA(bytes), data, length);
    free(data);
    return bytes;
}

QIQ_GUARDED(void *, qiq_stdlib___socket_read_exactly, NULL, (int64_t handle, int64_t count), (handle, count)) {
    QiqSocket *socket = qiq_socket(handle);
    if (count < 0) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "read length must not be negative");
    return qiq_read_to(socket, count);
}

QIQ_GUARDED(void *, qiq_stdlib___socket_read_all, NULL, (int64_t handle), (handle)) {
    return qiq_read_to(qiq_socket(handle), -1);
}

// Closes the connection (telling a TLS peer first) and frees the socket;
// closing twice does nothing.
QIQ_GUARDED_VOID(qiq_stdlib___socket_close, (int64_t handle), (handle)) {
    QiqSocket *socket = qiq_lookup(handle);
    if (socket == NULL) return;
    if (socket->open && socket->tls) mbedtls_ssl_close_notify(&socket->ssl);
    qiq_free_socket(handle);
}

// "address:port" of the other end.
QIQ_GUARDED(char *, qiq_stdlib___socket_peer, NULL, (int64_t handle), (handle)) {
    QiqSocket *socket = qiq_socket(handle);
    return qiq_string_from(socket->peer, strlen(socket->peer));
}

// Changes how long each read and write may wait.
QIQ_GUARDED_VOID(qiq_stdlib___socket_set_timeout, (int64_t handle, int64_t timeout), (handle, timeout)) {
    QiqSocket *socket = qiq_socket(handle);
    if (timeout < 1) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "the timeout must be at least 1 second");
    socket->timeout_ms = timeout > 2000000 ? 2000000000 : (int)(timeout * 1000);
}

// ---------------------------------------------------------------------------
// Connections for other runtime files (the PostgreSQL driver in
// postgres.c). They use the same handles and throw NetworkException.

int64_t qiq_net_connect(const char *host, int64_t port, int64_t timeout) {
    return qiq_stdlib___socket_connect_impl(host, port, false, timeout);
}

// Connects to the Unix socket file `path`, waiting at most `timeout` seconds.
int64_t qiq_net_connect_unix(const char *path, int64_t timeout) {
    struct sockaddr_un address = {0};
    address.sun_family = AF_UNIX;
    if (strlen(path) >= sizeof address.sun_path) qiq_throwf(QIQ_NETWORK_EXCEPTION, "the socket path %s is too long", path);
    strcpy(address.sun_path, path);
    int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC | SOCK_NONBLOCK, 0);
    if (fd < 0) qiq_throwf(QIQ_NETWORK_EXCEPTION, "could not create a socket: %s", strerror(errno));
    int error = 0;
    if (connect(fd, (struct sockaddr *)&address, sizeof address) != 0) {
        error = errno;
        if (error == EINPROGRESS || error == EAGAIN) {
            // The server's queue is full: wait for room.
            struct pollfd poller = {fd, POLLOUT, 0};
            int ready;
            do {
                ready = poll(&poller, 1, timeout > 2000000 ? 2000000000 : (int)(timeout * 1000));
            } while (ready < 0 && errno == EINTR);
            error = ETIMEDOUT;
            if (ready > 0) {
                socklen_t length = sizeof error;
                getsockopt(fd, SOL_SOCKET, SO_ERROR, &error, &length);
            }
        }
    }
    if (error != 0) {
        close(fd);
        if (error == ENOENT) qiq_throwf(QIQ_NETWORK_EXCEPTION, "no socket at %s: is the server running, and is that its socket?", path);
        if (error == ECONNREFUSED) qiq_throwf(QIQ_NETWORK_EXCEPTION, "nothing is accepting connections on %s", path);
        qiq_throwf(QIQ_NETWORK_EXCEPTION, "could not connect to %s: %s", path, strerror(error));
    }
    QiqSocket *connection = calloc(1, sizeof *connection);
    if (connection == NULL) qiq_fatal("out of memory");
    connection->fd = fd;
    connection->open = true;
    connection->timeout_ms = timeout > 2000000 ? 2000000000 : (int)(timeout * 1000);
    snprintf(connection->peer, sizeof connection->peer, "unix:%s", path);
    return qiq_register_socket(connection);
}

// TLS configurations that trust only the certificates in one file, for
// connections that name their own CA (a database's sslrootcert). Each file
// is read once and kept.
typedef struct {
    char *path;
    mbedtls_x509_crt authorities;
    mbedtls_ssl_config config;
} QiqTlsAuthority;

// Each is allocated once and never moves: connections point at its config.
static QiqTlsAuthority **qiq_tls_authorities;
static size_t qiq_tls_authority_count;

static const mbedtls_ssl_config *qiq_tls_config_for(const char *ca_file) {
    qiq_tls_setup();
    if (ca_file == NULL) return &qiq_tls_config;
    for (size_t i = 0; i < qiq_tls_authority_count; i++) {
        if (strcmp(qiq_tls_authorities[i]->path, ca_file) == 0) return &qiq_tls_authorities[i]->config;
    }
    QiqTlsAuthority *authority = calloc(1, sizeof *authority);
    if (authority == NULL) qiq_fatal("out of memory");
    mbedtls_x509_crt_init(&authority->authorities);
    if (mbedtls_x509_crt_parse_file(&authority->authorities, ca_file) < 0) {
        mbedtls_x509_crt_free(&authority->authorities);
        free(authority);
        qiq_throwf(QIQ_NETWORK_EXCEPTION, "TLS: could not read the certificates in %s", ca_file);
    }
    mbedtls_ssl_config_init(&authority->config);
    if (mbedtls_ssl_config_defaults(&authority->config, MBEDTLS_SSL_IS_CLIENT, MBEDTLS_SSL_TRANSPORT_STREAM, MBEDTLS_SSL_PRESET_DEFAULT) != 0) {
        qiq_throw(QIQ_NETWORK_EXCEPTION, "TLS: could not configure");
    }
    mbedtls_ssl_conf_authmode(&authority->config, MBEDTLS_SSL_VERIFY_REQUIRED);
    mbedtls_ssl_conf_ca_chain(&authority->config, &authority->authorities, NULL);
    mbedtls_ssl_conf_rng(&authority->config, mbedtls_ctr_drbg_random, &qiq_random);
    authority->path = strdup(ca_file);
    QiqTlsAuthority **grown = realloc(qiq_tls_authorities, (qiq_tls_authority_count + 1) * sizeof *grown);
    if (authority->path == NULL || grown == NULL) qiq_fatal("out of memory");
    qiq_tls_authorities = grown;
    qiq_tls_authorities[qiq_tls_authority_count++] = authority;
    return &authority->config;
}

// Starts TLS on a connected socket, for protocols that agree to it first,
// checking the server's certificate against `ca_file`'s certificates, or
// the system's when it's NULL.
void qiq_net_start_tls(int64_t handle, const char *host, const char *ca_file) {
    QiqSocket *socket = qiq_socket(handle);
    const mbedtls_ssl_config *config = qiq_tls_config_for(ca_file);
    char message[600];
    if (!qiq_tls_handshake(socket, host, config, message, sizeof message)) {
        qiq_free_socket(handle);
        qiq_throw(QIQ_NETWORK_EXCEPTION, message);
    }
}

// How long each read and write may wait, in milliseconds; -1 for no limit.
void qiq_net_set_timeout_ms(int64_t handle, int timeout_ms) {
    qiq_socket(handle)->timeout_ms = timeout_ms;
}

void qiq_net_send(int64_t handle, const void *data, size_t length) {
    qiq_send_all(qiq_socket(handle), data, length);
}

// Exactly `length` bytes into `out`.
void qiq_net_receive(int64_t handle, void *out, size_t length) {
    QiqSocket *socket = qiq_socket(handle);
    unsigned char *into = out;
    while (length > 0) {
        if (socket->start == socket->end && !qiq_fill(socket)) {
            qiq_throwf(QIQ_NETWORK_EXCEPTION, "%s closed the connection", socket->peer);
        }
        size_t available = socket->end - socket->start;
        size_t take = available < length ? available : length;
        memcpy(into, socket->buffer + socket->start, take);
        socket->start += take;
        into += take;
        length -= take;
    }
}

void qiq_net_close(int64_t handle) {
    QiqSocket *socket = qiq_lookup(handle);
    if (socket == NULL) return;
    if (socket->open && socket->tls) mbedtls_ssl_close_notify(&socket->ssl);
    qiq_free_socket(handle);
}

// Frees a socket without telling the other end, for one inherited from the
// process this one was forked from: it's still the parent's connection.
void qiq_net_abandon(int64_t handle) {
    qiq_free_socket(handle);
}

// ---------------------------------------------------------------------------
// Servers: listening, accepting, and pre-forked worker processes. SIGINT and
// SIGTERM ask the server to stop: workers finish the request in progress,
// stop accepting, and exit; the supervising process waits for them.

static volatile sig_atomic_t qiq_stopping = 0;

static void qiq_on_stop(int signal_number) {
    (void)signal_number;
    qiq_stopping = 1;
}

static void qiq_handle_stop_signals(void) {
    struct sigaction action = {0};
    action.sa_handler = qiq_on_stop;
    sigemptyset(&action.sa_mask);
    // No SA_RESTART: a signal interrupts poll() and waitpid().
    sigaction(SIGINT, &action, NULL);
    sigaction(SIGTERM, &action, NULL);
    signal(SIGPIPE, SIG_IGN);
}

// Listens on host:port; an empty host means every address (IPv6 and IPv4).
QIQ_GUARDED(int64_t, qiq_stdlib___server_listen, 0, (const char *host, int64_t port), (host, port)) {
    if (port < 0 || port > 65535) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "port %lld is not between 0 and 65535", (long long)port);
    bool any = host == NULL || host[0] == '\0';
    char service[16];
    snprintf(service, sizeof service, "%lld", (long long)port);
    char where[300];
    if (any) {
        snprintf(where, sizeof where, ":%lld", (long long)port);
    } else {
        snprintf(where, sizeof where, strchr(host, ':') ? "[%s]:%lld" : "%s:%lld", host, (long long)port);
    }
    struct addrinfo hints = {0};
    hints.ai_family = any ? AF_INET6 : AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = AI_PASSIVE;
    struct addrinfo *addresses = NULL;
    int found = getaddrinfo(any ? NULL : host, service, &hints, &addresses);
    if (found != 0 && any) {
        hints.ai_family = AF_INET;  // no IPv6 on this machine
        found = getaddrinfo(NULL, service, &hints, &addresses);
    }
    if (found != 0) qiq_throwf(QIQ_NETWORK_EXCEPTION, "could not listen on %s: %s", where, gai_strerror(found));
    int fd = -1;
    int error = 0;
    for (struct addrinfo *address = addresses; address != NULL && fd < 0; address = address->ai_next) {
        fd = socket(address->ai_family, address->ai_socktype | SOCK_CLOEXEC | SOCK_NONBLOCK, address->ai_protocol);
        if (fd < 0) {
            error = errno;
            continue;
        }
        int on = 1;
        int off = 0;
        setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &on, sizeof on);
        if (address->ai_family == AF_INET6 && any) setsockopt(fd, IPPROTO_IPV6, IPV6_V6ONLY, &off, sizeof off);
        if (bind(fd, address->ai_addr, address->ai_addrlen) != 0 || listen(fd, 511) != 0) {
            error = errno;
            close(fd);
            fd = -1;
        }
    }
    freeaddrinfo(addresses);
    if (fd < 0) qiq_throwf(QIQ_NETWORK_EXCEPTION, "could not listen on %s: %s", where, strerror(error));
    QiqSocket *listener = calloc(1, sizeof *listener);
    if (listener == NULL) qiq_fatal("out of memory");
    listener->fd = fd;
    listener->open = true;
    listener->timeout_ms = 30000;
    snprintf(listener->peer, sizeof listener->peer, "%s", where);
    return qiq_register_socket(listener);
}

// Listens on a Unix domain socket file at `path`, replacing a stale one left
// by a server that didn't stop cleanly. Every user may connect, as with a
// port on 127.0.0.1; limit who can with the permissions of its directory.
QIQ_GUARDED(int64_t, qiq_stdlib___server_listen_unix, 0, (const char *path), (path)) {
    struct sockaddr_un address = {0};
    address.sun_family = AF_UNIX;
    size_t length = strlen(path);
    if (length == 0 || length >= sizeof address.sun_path) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "a Unix socket path must be 1 to %zu bytes long", sizeof address.sun_path - 1);
    }
    memcpy(address.sun_path, path, length + 1);
    struct stat info;
    if (lstat(path, &info) == 0) {
        if (!S_ISSOCK(info.st_mode)) qiq_throwf(QIQ_NETWORK_EXCEPTION, "could not listen on unix:%s: the file exists and isn't a socket", path);
        // A socket file is in use if something accepts connections on it.
        int probe = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0);
        bool in_use = probe >= 0 && connect(probe, (struct sockaddr *)&address, sizeof address) == 0;
        if (probe >= 0) close(probe);
        if (in_use) qiq_throwf(QIQ_NETWORK_EXCEPTION, "could not listen on unix:%s: another server is listening on it", path);
        unlink(path);
    }
    int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC | SOCK_NONBLOCK, 0);
    if (fd < 0) qiq_throwf(QIQ_NETWORK_EXCEPTION, "could not listen on unix:%s: %s", path, strerror(errno));
    if (bind(fd, (struct sockaddr *)&address, sizeof address) != 0 || listen(fd, 511) != 0) {
        int error = errno;
        close(fd);
        qiq_throwf(QIQ_NETWORK_EXCEPTION, "could not listen on unix:%s: %s", path, strerror(error));
    }
    chmod(path, 0666);
    QiqSocket *listener = calloc(1, sizeof *listener);
    if (listener == NULL) qiq_fatal("out of memory");
    listener->fd = fd;
    listener->open = true;
    listener->unix_listener = true;
    listener->timeout_ms = 30000;
    snprintf(listener->peer, sizeof listener->peer, "unix:%s", path);
    return qiq_register_socket(listener);
}

// Stops listening; for a Unix socket, also removes its file.
QIQ_GUARDED_VOID(qiq_stdlib___server_close, (int64_t handle), (handle)) {
    QiqSocket *listener = qiq_lookup(handle);
    if (listener == NULL) return;
    if (listener->unix_listener) unlink(listener->peer + 5);
    qiq_free_socket(handle);
}

// The port a listening socket got (useful after listening on port 0).
QIQ_GUARDED(int64_t, qiq_stdlib___server_port, 0, (int64_t handle), (handle)) {
    QiqSocket *listener = qiq_socket(handle);
    struct sockaddr_storage address;
    socklen_t length = sizeof address;
    if (getsockname(listener->fd, (struct sockaddr *)&address, &length) != 0) return 0;
    if (address.ss_family == AF_INET6) return ntohs(((struct sockaddr_in6 *)&address)->sin6_port);
    return ntohs(((struct sockaddr_in *)&address)->sin_port);
}

// Accepts a connection waiting on `listener`: its handle, or -1 if there was
// none (another worker took it) or accepting failed for now. Each read and
// write on it waits at most `timeout` seconds.
static int64_t qiq_accept_connection(QiqSocket *listener, int64_t timeout) {
    struct sockaddr_storage address;
    socklen_t length = sizeof address;
    int fd = accept4(listener->fd, (struct sockaddr *)&address, &length, SOCK_NONBLOCK | SOCK_CLOEXEC);
    if (fd < 0) {
        // A temporary shortage, such as too many open files: wait a
        // moment rather than spin.
        if (errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR && errno != ECONNABORTED) {
            struct timespec pause = {0, 50000000};
            nanosleep(&pause, NULL);
        }
        return -1;
    }
    QiqSocket *socket = calloc(1, sizeof *socket);
    if (socket == NULL) qiq_fatal("out of memory");
    socket->fd = fd;
    socket->open = true;
    socket->timeout_ms = timeout > 2000000 ? 2000000000 : (int)(timeout < 1 ? 1000 : timeout * 1000);
    if (address.ss_family == AF_UNIX) {
        snprintf(socket->peer, sizeof socket->peer, "unix");  // clients of a Unix socket have no address
    } else {
        char host[NI_MAXHOST] = "?";
        char service[NI_MAXSERV] = "?";
        getnameinfo((struct sockaddr *)&address, length, host, sizeof host, service, sizeof service, NI_NUMERICHOST | NI_NUMERICSERV);
        const char *shown = strncmp(host, "::ffff:", 7) == 0 ? host + 7 : host;  // IPv4 over IPv6
        snprintf(socket->peer, sizeof socket->peer, strchr(shown, ':') ? "[%s]:%s" : "%s:%s", shown, service);
        int on = 1;
        setsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &on, sizeof on);
    }
    return qiq_register_socket(socket);
}

// The connections a worker keeps open between requests. A worker serves
// one request at a time, but not one connection at a time: it waits on all
// of these and the listening socket at once, and serves whichever request
// comes first. So a client that keeps a connection open, as nginx does with
// `keepalive`, doesn't hold a worker while it's idle, and a client that
// connects and sends nothing doesn't either.
typedef struct {
    int64_t handle;
    int64_t since_ms;   // when it last became idle
} QiqIdleConnection;

static QiqIdleConnection *qiq_idle = NULL;
static size_t qiq_idle_count = 0;
static size_t qiq_idle_capacity = 0;
static struct pollfd *qiq_idle_pollers = NULL;
static size_t qiq_next_idle = 0;   // where the next search starts, so each connection gets its turn

// Idle connections a worker keeps; when there are more, the one idle the
// longest is closed. And how long one may be idle before it's closed.
#define QIQ_IDLE_MAX 1024
#define QIQ_IDLE_TIMEOUT_MS 5000

static int64_t qiq_now_ms(void) {
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    return (int64_t)now.tv_sec * 1000 + now.tv_nsec / 1000000;
}

static void qiq_forget_idle(size_t index) {
    qiq_idle[index] = qiq_idle[--qiq_idle_count];
}

static void qiq_add_idle(int64_t handle) {
    if (qiq_idle_count == QIQ_IDLE_MAX) {
        size_t oldest = 0;
        for (size_t index = 1; index < qiq_idle_count; index++) {
            if (qiq_idle[index].since_ms < qiq_idle[oldest].since_ms) oldest = index;
        }
        qiq_free_socket(qiq_idle[oldest].handle);
        qiq_forget_idle(oldest);
    }
    if (qiq_idle_count == qiq_idle_capacity) {
        qiq_idle_capacity = qiq_idle_capacity ? qiq_idle_capacity * 2 : 16;
        QiqIdleConnection *grown = realloc(qiq_idle, qiq_idle_capacity * sizeof *grown);
        struct pollfd *pollers = realloc(qiq_idle_pollers, (qiq_idle_capacity + 1) * sizeof *pollers);
        if (grown == NULL || pollers == NULL) qiq_fatal("out of memory");
        qiq_idle = grown;
        qiq_idle_pollers = pollers;
    }
    qiq_idle[qiq_idle_count++] = (QiqIdleConnection){handle, qiq_now_ms()};
}

// The next connection with a request to read: one of the idle connections
// once data (or its close) arrives, or a new one, which waits with the idle
// ones until its first request comes. -1 once the server is stopping. New
// connections' reads and writes wait at most `timeout` seconds.
QIQ_GUARDED(int64_t, qiq_stdlib___server_next, -1, (int64_t handle, int64_t timeout), (handle, timeout)) {
    QiqSocket *listener = qiq_socket(handle);
    if (qiq_idle_pollers == NULL) {
        qiq_idle_pollers = malloc(sizeof *qiq_idle_pollers);
        if (qiq_idle_pollers == NULL) qiq_fatal("out of memory");
    }
    for (;;) {
        if (qiq_stopping) return -1;
        // Close what has been idle too long, and serve a request that
        // arrived with the last one (pipelined) straight away.
        int64_t now = qiq_now_ms();
        int64_t wait_ms = 500;   // at most, to notice a stop request
        for (size_t index = 0; index < qiq_idle_count;) {
            QiqSocket *socket = qiq_lookup(qiq_idle[index].handle);
            int64_t left = qiq_idle[index].since_ms + QIQ_IDLE_TIMEOUT_MS - now;
            if (socket == NULL || left <= 0) {
                qiq_free_socket(qiq_idle[index].handle);
                qiq_forget_idle(index);
                continue;
            }
            if (socket->start < socket->end) {
                int64_t ready = qiq_idle[index].handle;
                qiq_forget_idle(index);
                return ready;
            }
            if (left < wait_ms) wait_ms = left;
            index++;
        }
        qiq_idle_pollers[0] = (struct pollfd){listener->fd, POLLIN, 0};
        for (size_t index = 0; index < qiq_idle_count; index++) {
            qiq_idle_pollers[index + 1] = (struct pollfd){qiq_lookup(qiq_idle[index].handle)->fd, POLLIN, 0};
        }
        int ready = poll(qiq_idle_pollers, (nfds_t)qiq_idle_count + 1, (int)wait_ms);
        if (ready <= 0) continue;  // a timeout or a signal: look again
        // A connection with something to read, starting after the last one
        // served so that none waits behind the others.
        for (size_t step = 0; step < qiq_idle_count; step++) {
            size_t index = (qiq_next_idle + step) % qiq_idle_count;
            if (qiq_idle_pollers[index + 1].revents != 0) {
                int64_t chosen = qiq_idle[index].handle;
                qiq_forget_idle(index);
                qiq_next_idle = index + 1;
                return chosen;
            }
        }
        // Otherwise a new connection. Its request has usually arrived with
        // it: read what's there without waiting, and serve it straight away
        // (the parser then starts from this buffer, so this costs no extra
        // system call). A client that hasn't sent anything yet waits with
        // the idle connections instead of holding the worker.
        if (qiq_idle_pollers[0].revents != 0) {
            int64_t connection = qiq_accept_connection(listener, timeout);
            if (connection < 0) continue;
            QiqSocket *socket = qiq_lookup(connection);
            ssize_t got = recv(socket->fd, socket->buffer, QIQ_SOCKET_BUFFER, MSG_DONTWAIT);
            if (got < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) {
                // Not here yet: give it a moment, as it's usually on its way.
                struct pollfd arriving = {socket->fd, POLLIN, 0};
                if (poll(&arriving, 1, 2) > 0) got = recv(socket->fd, socket->buffer, QIQ_SOCKET_BUFFER, MSG_DONTWAIT);
            }
            if (got > 0) {
                socket->start = 0;
                socket->end = (size_t)got;
                return connection;
            }
            if (got == 0 || (errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR)) {
                qiq_free_socket(connection);   // closed before saying anything
                continue;
            }
            qiq_add_idle(connection);
        }
    }
}

// Keeps a connection open for its next request, after a response.
QIQ_GUARDED_VOID(qiq_stdlib___server_keep, (int64_t handle), (handle)) {
    qiq_socket(handle);
    qiq_add_idle(handle);
}

// Closes the idle connections, when a worker stops.
void qiq_stdlib___server_close_idle(void) {
    while (qiq_idle_count > 0) {
        qiq_free_socket(qiq_idle[0].handle);
        qiq_forget_idle(0);
    }
}

// Counts a request on a connection; returns how many it has carried.
QIQ_GUARDED(int64_t, qiq_stdlib___server_count_request, 0, (int64_t handle), (handle)) {
    return ++qiq_socket(handle)->served;
}

// Whether SIGINT or SIGTERM asked the server to stop.
bool qiq_stdlib___server_stopping(void) {
    return qiq_stopping;
}

int64_t qiq_stdlib___server_cpu_count(void) {
    long count = sysconf(_SC_NPROCESSORS_ONLN);
    return count > 0 ? count : 1;
}

// Starts the worker processes. Returns the worker's number (1 to `workers`)
// in each worker. The calling process supervises them, starting a new one
// when one dies, and returns 0 once they've all stopped after SIGINT or
// SIGTERM. With one worker, nothing is forked: returns 1 in this process.
QIQ_GUARDED(int64_t, qiq_stdlib___server_start, 0, (int64_t workers), (workers)) {
    qiq_handle_stop_signals();
    if (workers <= 1) return 1;
    if (workers > 1024) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "at most 1024 workers");
    pid_t *pids = calloc((size_t)workers, sizeof *pids);
    time_t *started = calloc((size_t)workers, sizeof *started);
    if (pids == NULL || started == NULL) qiq_fatal("out of memory");
    fflush(NULL);  // so buffered output isn't written again by each worker
    for (int64_t index = 0; index < workers; index++) {
        pid_t pid = fork();
        if (pid == 0) return index + 1;
        if (pid < 0) qiq_throwf(QIQ_NETWORK_EXCEPTION, "could not start worker processes: %s", strerror(errno));
        pids[index] = pid;
        started[index] = time(NULL);
    }
    while (!qiq_stopping) {
        int status;
        pid_t pid = waitpid(-1, &status, 0);
        if (pid < 0) {
            if (errno == EINTR) continue;
            break;
        }
        for (int64_t index = 0; index < workers; index++) {
            if (pids[index] != pid) continue;
            pids[index] = 0;
            if (qiq_stopping) break;
            char reason[64];
            if (WIFSIGNALED(status)) {
                snprintf(reason, sizeof reason, "was killed by signal %d", WTERMSIG(status));
            } else {
                snprintf(reason, sizeof reason, "exited with status %d", WEXITSTATUS(status));
            }
            fprintf(stderr, "qiq: worker %lld %s; starting another\n", (long long)index + 1, reason);
            // A worker that dies right away would otherwise restart in a tight loop.
            if (time(NULL) - started[index] < 1) sleep(1);
            if (qiq_stopping) break;
            fflush(NULL);
            pid_t replacement = fork();
            if (replacement == 0) return index + 1;
            if (replacement > 0) {
                pids[index] = replacement;
                started[index] = time(NULL);
            }
            break;
        }
    }
    // Stopping: ask the workers to finish (they also get Ctrl-C from the
    // terminal), then wait for them.
    for (int64_t index = 0; index < workers; index++) {
        if (pids[index] > 0) kill(pids[index], SIGTERM);
    }
    for (int64_t index = 0; index < workers; index++) {
        while (pids[index] > 0 && waitpid(pids[index], NULL, 0) < 0 && errno == EINTR) {
        }
    }
    free(pids);
    free(started);
    return 0;
}
