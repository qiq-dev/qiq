// SQL databases for std/sql (stdlib/sql.qiq): SQLite (vendor/, built by
// build.rs and linked into programs that use it), and PostgreSQL through
// postgres.c, which shares this file's handles, values, and result buffers
// (sql_driver.h).
//
// A query runs to completion in one call (qiq_stdlib___sql_run): its
// statement is prepared, given the values the compiled code passed just
// before the call (qiq_bind_values), stepped through, and reset, with the
// rows' cells collected in this file's buffers. The qiq side then takes
// them as arrays (__sql_kinds, __sql_values, ...), so a statement never
// stays open in qiq code and can't leak when that code throws.
//
// Connections are referred to by handles, like sockets in net.c: a slot
// index in the low 32 bits and the slot's generation above them, so a
// handle to a closed database is detected. Each keeps its most recently
// used statements prepared, keyed by their SQL text.
//
// SQLite connections must not be used in a process forked after they were
// opened (as the HTTP server's workers are), so a connection notices a fork
// and reopens its database in the new process the first time it's used,
// running the PRAGMA statements it had run again, since settings such as
// `synchronous` belong to the connection rather than the file.
// Errors throw DatabaseException.
#include <pthread.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/random.h>
#include <sys/stat.h>

#include "qiq_runtime.h"
#include "sql_driver.h"
#include "sqlite3.h"

// Prepared statements each connection keeps.
#define QIQ_SQL_CACHE 16
// Milliseconds a statement waits for another process's lock (such as
// another server worker writing) before failing with "database is locked".
#define QIQ_SQL_BUSY_TIMEOUT 5000

typedef struct {
    char *sql;                // NULL when the entry is unused
    sqlite3_stmt *statement;
    uint64_t used;            // when it was last used, for evicting the oldest
} QiqSqlCached;

typedef struct {
    sqlite3 *db;              // NULL for other databases, and when the slot is free
    const QiqSqlDriver *driver; // for a network database, its driver
    void *pg;                 // and its connection (NULL otherwise)
    int64_t streaming;        // a network connection's cursor still reading rows, or 0
    char *filename;           // to reopen it after a fork
    char *pragmas;            // the PRAGMA statements it ran, to run again then
    uint32_t generation;
    uint64_t fork_count;      // the value of qiq_sql_forks when it was opened
    uint64_t clock;
    QiqSqlCached cache[QIQ_SQL_CACHE];
} QiqSqlConnection;

static QiqSqlConnection *qiq_sql_connections;
static size_t qiq_sql_connection_count;
uint64_t qiq_sql_forks;

// The network drivers, defined by the compiler: each one's QiqSqlDriver when
// the program's Database::open calls could open that database, or NULL, so
// a program that only uses SQLite doesn't include them (or TLS).
extern const QiqSqlDriver *const qiq_sql_postgres;
extern const QiqSqlDriver *const qiq_sql_mysql;

static sqlite3_stmt *qiq_sql_prepare(QiqSqlConnection *connection, const char *sql, const char *caller, bool keep);
static void qiq_sql_bind(sqlite3_stmt *statement, sqlite3_destructor_type keep);
static void qiq_sql_close_cursors(int64_t connection);

static void *qiq_sql_grow(void *data, size_t *capacity, size_t needed, size_t size) {
    if (needed <= *capacity) return data;
    size_t grown = *capacity < 16 ? 16 : *capacity * 2;
    if (grown < needed) grown = needed;
    data = realloc(data, grown * size);
    if (data == NULL) qiq_fatal("out of memory");
    *capacity = grown;
    return data;
}

// ---------------------------------------------------------------------------
// The values for the next statement's placeholders, from the compiled code
// (see Function::binds_values in src/compiler.rs). Strings and byte arrays
// are retained until the statement has run.

QiqSqlValue *qiq_sql_pending;
size_t qiq_sql_pending_count;
static size_t qiq_sql_pending_capacity;

void qiq_sql_drop_pending(void) {
    for (size_t index = 0; index < qiq_sql_pending_count; index++) {
        QiqSqlValue *value = &qiq_sql_pending[index];
        if ((value->type == 's' || value->type == 'a' || value->type == 'T') && value->payload != 0) qiq_release((void *)(uintptr_t)value->payload);
    }
    qiq_sql_pending_count = 0;
}

void qiq_bind_values(const char *types, const uint64_t *payloads, int64_t count) {
    qiq_sql_drop_pending();
    qiq_sql_pending = qiq_sql_grow(qiq_sql_pending, &qiq_sql_pending_capacity, (size_t)count, sizeof(QiqSqlValue));
    for (int64_t index = 0; index < count; index++) {
        QiqSqlValue value = {types[index], payloads[index]};
        if ((value.type == 's' || value.type == 'a' || value.type == 'T') && value.payload != 0) qiq_retain((void *)(uintptr_t)value.payload);
        qiq_sql_pending[index] = value;
    }
    qiq_sql_pending_count = (size_t)count;
}

// ---------------------------------------------------------------------------
// The cells of the last query's rows, row by row, until the qiq side takes
// them. Each cell has a kind and a value: an int's value, or the index of
// its double, string, or byte array in the lists below.

static int64_t *qiq_sql_kinds, *qiq_sql_values, *qiq_sql_blob_starts;
static double *qiq_sql_doubles;
static char **qiq_sql_texts;
static unsigned char *qiq_sql_blobs;
static size_t qiq_sql_cells, qiq_sql_kinds_capacity, qiq_sql_values_capacity;
static size_t qiq_sql_double_count, qiq_sql_doubles_capacity;
static size_t qiq_sql_text_count, qiq_sql_texts_capacity;
static size_t qiq_sql_blob_count, qiq_sql_blob_starts_capacity;
static size_t qiq_sql_blob_length, qiq_sql_blobs_capacity;
static char **qiq_sql_columns;
static size_t qiq_sql_column_count, qiq_sql_columns_capacity;

void qiq_sql_clear_results(void) {
    for (size_t index = 0; index < qiq_sql_text_count; index++) qiq_release(qiq_sql_texts[index]);
    for (size_t index = 0; index < qiq_sql_column_count; index++) qiq_release(qiq_sql_columns[index]);
    qiq_sql_cells = qiq_sql_double_count = qiq_sql_text_count = 0;
    qiq_sql_blob_count = qiq_sql_blob_length = qiq_sql_column_count = 0;
}

void qiq_sql_add_cell(int64_t kind, int64_t value) {
    qiq_sql_kinds = qiq_sql_grow(qiq_sql_kinds, &qiq_sql_kinds_capacity, qiq_sql_cells + 1, sizeof(int64_t));
    qiq_sql_values = qiq_sql_grow(qiq_sql_values, &qiq_sql_values_capacity, qiq_sql_cells + 1, sizeof(int64_t));
    qiq_sql_kinds[qiq_sql_cells] = kind;
    qiq_sql_values[qiq_sql_cells] = value;
    qiq_sql_cells++;
}

void qiq_sql_add_bytes(const void *bytes, size_t length) {
    qiq_sql_blob_starts = qiq_sql_grow(qiq_sql_blob_starts, &qiq_sql_blob_starts_capacity, qiq_sql_blob_count + 1, sizeof(int64_t));
    qiq_sql_blobs = qiq_sql_grow(qiq_sql_blobs, &qiq_sql_blobs_capacity, qiq_sql_blob_length + length, 1);
    if (length > 0) memcpy(qiq_sql_blobs + qiq_sql_blob_length, bytes, length);
    qiq_sql_blob_starts[qiq_sql_blob_count] = (int64_t)qiq_sql_blob_length;
    qiq_sql_add_cell(QIQ_SQL_BYTES, (int64_t)qiq_sql_blob_count);
    qiq_sql_blob_count++;
    qiq_sql_blob_length += length;
}

void qiq_sql_add_double(double value) {
    qiq_sql_doubles = qiq_sql_grow(qiq_sql_doubles, &qiq_sql_doubles_capacity, qiq_sql_double_count + 1, sizeof(double));
    qiq_sql_doubles[qiq_sql_double_count] = value;
    qiq_sql_add_cell(QIQ_SQL_DOUBLE, (int64_t)qiq_sql_double_count++);
}

void qiq_sql_add_text(const char *text, size_t length, int64_t kind) {
    // Text a qiq string can't hold reads as bytes (getBytes).
    if (text == NULL || !qiq_is_utf8_text((const unsigned char *)text, length)) {
        qiq_sql_add_bytes(text, text == NULL ? 0 : length);
        return;
    }
    qiq_sql_texts = qiq_sql_grow(qiq_sql_texts, &qiq_sql_texts_capacity, qiq_sql_text_count + 1, sizeof(char *));
    qiq_sql_texts[qiq_sql_text_count] = qiq_string_from(text, length);
    qiq_sql_add_cell(kind, (int64_t)qiq_sql_text_count++);
}

void qiq_sql_add_column(const char *name, size_t length) {
    qiq_sql_columns = qiq_sql_grow(qiq_sql_columns, &qiq_sql_columns_capacity, qiq_sql_column_count + 1, sizeof(char *));
    qiq_sql_columns[qiq_sql_column_count++] = qiq_string_from(name, length);
}

int64_t qiq_sql_first_int(void) {
    return qiq_sql_cells > 0 && qiq_sql_kinds[0] == QIQ_SQL_INT ? qiq_sql_values[0] : 0;
}

// ---------------------------------------------------------------------------
// Errors

_Noreturn void qiq_sql_throw(const char *state, int64_t code, bool unique, const char *message) {
    QiqExceptionKind kind = QIQ_DATABASE_EXCEPTION;
    if (state != NULL && state[0] == '2' && state[1] == '3') kind = unique ? QIQ_UNIQUE_VIOLATION_EXCEPTION : QIQ_CONSTRAINT_VIOLATION_EXCEPTION;
    qiq_set_database_error(state, code);
    qiq_throw(kind, message);
}

// A SQLite error, with the SQLSTATE for its extended result code where
// there's a standard one.
static _Noreturn void qiq_sqlite_throw(int code, const char *message) {
    const char *state = "HY000";
    bool unique = false;
    switch (code) {
    case SQLITE_CONSTRAINT_UNIQUE:
    case SQLITE_CONSTRAINT_PRIMARYKEY:
        state = "23505";
        unique = true;
        break;
    case SQLITE_CONSTRAINT_NOTNULL:
        state = "23502";
        break;
    case SQLITE_CONSTRAINT_FOREIGNKEY:
        state = "23503";
        break;
    case SQLITE_CONSTRAINT_CHECK:
        state = "23514";
        break;
    default:
        if ((code & 0xff) == SQLITE_CONSTRAINT) state = "23000";
    }
    qiq_sql_throw(state, code, unique, message);
}

// ---------------------------------------------------------------------------
// For the network drivers: URLs and random bytes.

static int qiq_sql_hex(char c) {
    return c >= '0' && c <= '9' ? c - '0' : c >= 'a' && c <= 'f' ? c - 'a' + 10 : c >= 'A' && c <= 'F' ? c - 'A' + 10 : -1;
}

// A percent-decoded copy of `length` bytes.
static char *qiq_sql_decode(const char *text, size_t length) {
    char *out = malloc(length + 1);
    if (out == NULL) qiq_fatal("out of memory");
    size_t o = 0;
    for (size_t i = 0; i < length; i++) {
        if (text[i] == '%' && i + 2 < length && qiq_sql_hex(text[i + 1]) >= 0 && qiq_sql_hex(text[i + 2]) >= 0) {
            out[o++] = (char)(qiq_sql_hex(text[i + 1]) * 16 + qiq_sql_hex(text[i + 2]));
            i += 2;
        } else {
            out[o++] = text[i];
        }
    }
    out[o] = '\0';
    return out;
}

void qiq_sql_free_url(QiqSqlUrl *url) {
    free(url->user);
    free(url->password);
    free(url->host);
    free(url->database);
    for (size_t i = 0; i < url->option_count; i++) free(url->options[i]);
    free(url->options);
    memset(url, 0, sizeof *url);
}

bool qiq_sql_parse_url(const char *text, QiqSqlUrl *url) {
    memset(url, 0, sizeof *url);
    const char *scheme_end = strstr(text, "://");
    if (scheme_end == NULL) return false;
    const char *rest = scheme_end + 3;
    const char *end = rest + strcspn(rest, "?");
    const char *slash = memchr(rest, '/', (size_t)(end - rest));
    const char *authority_end = slash != NULL ? slash : end;
    // The last @ ends the user and password (a password may contain one).
    const char *at = NULL;
    for (const char *c = rest; c < authority_end; c++) {
        if (*c == '@') at = c;
    }
    const char *host = rest;
    if (at != NULL) {
        const char *colon = memchr(rest, ':', (size_t)(at - rest));
        url->user = qiq_sql_decode(rest, (size_t)((colon != NULL ? colon : at) - rest));
        if (colon != NULL) url->password = qiq_sql_decode(colon + 1, (size_t)(at - colon - 1));
        host = at + 1;
    }
    const char *port = NULL;
    if (*host == '[') {
        const char *close = memchr(host, ']', (size_t)(authority_end - host));
        if (close == NULL) goto invalid;
        url->host = qiq_sql_decode(host + 1, (size_t)(close - host - 1));
        if (close + 1 < authority_end) {
            if (close[1] != ':') goto invalid;
            port = close + 2;
        }
    } else {
        const char *colon = memchr(host, ':', (size_t)(authority_end - host));
        url->host = qiq_sql_decode(host, (size_t)((colon != NULL ? colon : authority_end) - host));
        if (colon != NULL) port = colon + 1;
    }
    if (port != NULL) {
        char *after;
        long number = strtol(port, &after, 10);
        if (after != authority_end || number < 1 || number > 65535) goto invalid;
        url->port = number;
    }
    if (url->host[0] == '\0') {
        free(url->host);
        url->host = NULL;
    }
    if (url->user != NULL && url->user[0] == '\0') {
        free(url->user);
        url->user = NULL;
    }
    if (slash != NULL && slash + 1 < end) url->database = qiq_sql_decode(slash + 1, (size_t)(end - slash - 1));
    for (const char *option = *end == '?' ? end + 1 : end; *option != '\0';) {
        size_t length = strcspn(option, "&");
        if (memchr(option, '=', length) == NULL) goto invalid;
        char **grown = realloc(url->options, (url->option_count + 1) * sizeof(char *));
        if (grown == NULL) qiq_fatal("out of memory");
        url->options = grown;
        url->options[url->option_count++] = qiq_sql_decode(option, length);
        option += length + (option[length] == '&' ? 1 : 0);
    }
    return true;
invalid:
    qiq_sql_free_url(url);
    return false;
}

bool qiq_sql_is_local_host(const char *host) {
    return host == NULL || strcmp(host, "localhost") == 0 || strncmp(host, "127.", 4) == 0 || strcmp(host, "::1") == 0;
}

void qiq_sql_free_options(QiqSqlOptions *options) {
    free(options->ca_file);
    free(options->socket);
    memset(options, 0, sizeof *options);
}

static char *qiq_sql_copy(const char *text) {
    char *copy = strdup(text);
    if (copy == NULL) qiq_fatal("out of memory");
    return copy;
}

const char *qiq_sql_connection_options(const QiqSqlUrl *url, const char *host, QiqSqlOptions *out) {
    memset(out, 0, sizeof *out);
    out->timeout = 10;
    // A host that is a path ("postgres://%2Fvar%2Frun%2Fpostgresql/db") is a socket.
    if (host != NULL && host[0] == '/') out->socket = qiq_sql_copy(host);
    int sslmode = -1; // -1 not given, 0 disable, 1 require
    for (size_t i = 0; i < url->option_count; i++) {
        const char *option = url->options[i];
        if (strcmp(option, "sslmode=disable") == 0) sslmode = 0;
        else if (strcmp(option, "sslmode=require") == 0 || strcmp(option, "sslmode=verify-full") == 0) sslmode = 1;
        else if (strncmp(option, "connect_timeout=", 16) == 0 && strtol(option + 16, NULL, 10) >= 1) out->timeout = strtol(option + 16, NULL, 10);
        else if (strncmp(option, "sslrootcert=", 12) == 0 && option[12] != '\0') {
            free(out->ca_file);
            out->ca_file = qiq_sql_copy(option + 12);
        } else if ((strncmp(option, "host=", 5) == 0 && option[5] == '/') || (strncmp(option, "socket=", 7) == 0 && option[7] == '/')) {
            free(out->socket);
            out->socket = qiq_sql_copy(strchr(option, '=') + 1);
        } else {
            qiq_sql_free_options(out);
            return "the options are sslmode=disable or require (or verify-full), sslrootcert=file, connect_timeout=seconds, and "
                   "host=/path or socket=/path for a Unix socket";
        }
    }
    if (out->socket != NULL) {
        if (sslmode == 1 || out->ca_file != NULL) {
            qiq_sql_free_options(out);
            return "TLS isn't used over a Unix socket, which only reaches this machine: leave out sslmode and sslrootcert";
        }
        out->tls = false;
    } else if (out->ca_file != NULL) {
        if (sslmode == 0) {
            qiq_sql_free_options(out);
            return "sslrootcert is for checking the server's certificate, which sslmode=disable turns off";
        }
        out->tls = true;
    } else {
        out->tls = sslmode == -1 ? !qiq_sql_is_local_host(host) : sslmode == 1;
    }
    return NULL;
}

char *qiq_sql_socket_file(const char *socket, const char *file_name) {
    struct stat info;
    if (stat(socket, &info) == 0 && S_ISDIR(info.st_mode)) {
        size_t length = strlen(socket) + strlen(file_name) + 2;
        char *path = malloc(length);
        if (path == NULL) qiq_fatal("out of memory");
        snprintf(path, length, "%s%s%s", socket, socket[strlen(socket) - 1] == '/' ? "" : "/", file_name);
        return path;
    }
    return qiq_sql_copy(socket);
}

void qiq_sql_random(unsigned char *out, size_t count) {
    if (getrandom(out, count, 0) != (ssize_t)count) qiq_throw(QIQ_DATABASE_EXCEPTION, "no random numbers from the system");
}

bool qiq_sql_is_setting(const char *sql, size_t length, const char *keyword) {
    size_t at = 0;
    while (at < length && (sql[at] == ' ' || sql[at] == '\t' || sql[at] == '\n' || sql[at] == '\r')) at++;
    size_t size = strlen(keyword);
    if (length - at <= size || sqlite3_strnicmp(sql + at, keyword, (int)size) != 0) return false;
    char after = sql[at + size];
    return after == ' ' || after == '\t' || after == '\n' || after == '\r';
}

static void qiq_sql_add_row(sqlite3_stmt *statement, int columns) {
    for (int column = 0; column < columns; column++) {
        switch (sqlite3_column_type(statement, column)) {
        case SQLITE_INTEGER:
            qiq_sql_add_cell(QIQ_SQL_INT, sqlite3_column_int64(statement, column));
            break;
        case SQLITE_FLOAT:
            qiq_sql_add_double(sqlite3_column_double(statement, column));
            break;
        case SQLITE_TEXT: {
            const char *text = (const char *)sqlite3_column_text(statement, column);
            qiq_sql_add_text(text, (size_t)sqlite3_column_bytes(statement, column), QIQ_SQL_TEXT);
            break;
        }
        case SQLITE_BLOB: {
            const void *bytes = sqlite3_column_blob(statement, column);
            qiq_sql_add_bytes(bytes, bytes == NULL ? 0 : (size_t)sqlite3_column_bytes(statement, column));
            break;
        }
        default:
            qiq_sql_add_cell(QIQ_SQL_NULL, 0);
        }
    }
}

// ---------------------------------------------------------------------------
// Connections.

static void qiq_sql_after_fork(void) {
    qiq_sql_forks++;
}

static sqlite3 *qiq_sql_connect(const char *filename) {
    sqlite3 *db = NULL;
    int code = sqlite3_open_v2(filename, &db, SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_URI, NULL);
    if (code != SQLITE_OK) {
        char message[512];
        snprintf(message, sizeof message, "could not open the SQLite database '%s': %s", filename,
                 db != NULL ? sqlite3_errmsg(db) : sqlite3_errstr(code));
        sqlite3_close(db);
        qiq_throw(QIQ_DATABASE_EXCEPTION, message);
    }
    sqlite3_extended_result_codes(db, 1);
    sqlite3_busy_timeout(db, QIQ_SQL_BUSY_TIMEOUT);
    return db;
}

QIQ_GUARDED(int64_t, qiq_stdlib___sql_open, 0, (const char *dsn), (dsn)) {
    static bool registered = false;
    if (!registered) {
        pthread_atfork(NULL, NULL, qiq_sql_after_fork);
        registered = true;
    }
    sqlite3 *db = NULL;
    void *pg = NULL;
    const QiqSqlDriver *driver = NULL;
    const char *filename = NULL;
    bool network = true;
    if (strncmp(dsn, "postgres://", 11) == 0 || strncmp(dsn, "postgresql://", 13) == 0) {
        driver = qiq_sql_postgres;
    } else if (strncmp(dsn, "mysql://", 8) == 0 || strncmp(dsn, "mariadb://", 10) == 0) {
        driver = qiq_sql_mysql;
    } else {
        network = false;
    }
    if (network && driver == NULL) {
        // The compiler leaves a driver out only when no URL could need it.
        qiq_throw(QIQ_DATABASE_EXCEPTION, "internal error: this program was built without the driver for this database");
    }
    if (driver != NULL) {
        pg = driver->open(dsn);
    } else if (strncmp(dsn, "sqlite:", 7) == 0) {
        filename = dsn + 7;
        if (*filename == '\0') qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "Database::open: \"sqlite:\" needs a file name, or :memory:");
        db = qiq_sql_connect(filename);
    } else {
        qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION,
                  "Database::open: expected a database such as \"sqlite:path/to/file.db\", \"sqlite::memory:\", \"postgres://user:password@host/database\", or \"mysql://user:password@host/database\"");
    }
    size_t slot = 0;
    while (slot < qiq_sql_connection_count && (qiq_sql_connections[slot].db != NULL || qiq_sql_connections[slot].pg != NULL)) slot++;
    if (slot == qiq_sql_connection_count) {
        QiqSqlConnection *grown = realloc(qiq_sql_connections, (slot + 1) * sizeof(QiqSqlConnection));
        if (grown == NULL) qiq_fatal("out of memory");
        qiq_sql_connections = grown;
        memset(&qiq_sql_connections[slot], 0, sizeof(QiqSqlConnection));
        qiq_sql_connection_count++;
    }
    QiqSqlConnection *connection = &qiq_sql_connections[slot];
    uint32_t generation = connection->generation;
    memset(connection, 0, sizeof *connection);
    connection->generation = generation;
    connection->db = db;
    connection->driver = driver;
    connection->pg = pg;
    if (filename != NULL) {
        connection->filename = strdup(filename);
        if (connection->filename == NULL) qiq_fatal("out of memory");
    }
    connection->fork_count = qiq_sql_forks;
    return (int64_t)((uint64_t)generation << 32 | (uint64_t)(slot + 1));
}

// The connection for `handle`, or NULL when it has been closed.
static QiqSqlConnection *qiq_sql_find(int64_t handle) {
    size_t slot = (size_t)(uint32_t)handle;
    uint32_t generation = (uint32_t)((uint64_t)handle >> 32);
    if (slot == 0 || slot > qiq_sql_connection_count) return NULL;
    QiqSqlConnection *connection = &qiq_sql_connections[slot - 1];
    if ((connection->db == NULL && connection->pg == NULL) || connection->generation != generation) return NULL;
    return connection;
}

static QiqSqlConnection *qiq_sql_connection(int64_t handle) {
    QiqSqlConnection *connection = qiq_sql_find(handle);
    if (connection == NULL) qiq_throw(QIQ_DATABASE_EXCEPTION, "the database is closed");
    // PostgreSQL connections reconnect themselves (postgres.c).
    if (connection->pg == NULL && connection->fork_count != qiq_sql_forks) {
        // Opened before this process was forked: the parent's connection
        // and statements are left alone (closing them here could release
        // the parent's locks) and the database is opened again.
        memset(connection->cache, 0, sizeof connection->cache);
        connection->db = NULL;
        connection->db = qiq_sql_connect(connection->filename);
        connection->fork_count = qiq_sql_forks;
        char *error = NULL;
        if (connection->pragmas != NULL && sqlite3_exec(connection->db, connection->pragmas, NULL, NULL, &error) != SQLITE_OK) {
            char message[512];
            snprintf(message, sizeof message, "reopening the database in a new process: %s", error != NULL ? error : "PRAGMA failed");
            sqlite3_free(error);
            qiq_throw(QIQ_DATABASE_EXCEPTION, message);
        }
    }
    return connection;
}

// Records a statement that ran if it's a PRAGMA (see the top of the file).
static void qiq_sql_remember(QiqSqlConnection *connection, sqlite3_stmt *statement) {
    const char *sql = sqlite3_sql(statement);
    if (!qiq_sql_is_setting(sql, strlen(sql), "pragma")) return;
    while (*sql == ' ' || *sql == '\t' || *sql == '\n' || *sql == '\r') sql++;
    size_t length = strlen(sql);
    while (length > 0 && (sql[length - 1] == ';' || sql[length - 1] == ' ' || sql[length - 1] == '\n')) length--;
    size_t old = connection->pragmas != NULL ? strlen(connection->pragmas) : 0;
    char *grown = realloc(connection->pragmas, old + length + 3);
    if (grown == NULL) qiq_fatal("out of memory");
    memcpy(grown + old, sql, length);
    memcpy(grown + old + length, ";\n", 3);
    connection->pragmas = grown;
}

QIQ_GUARDED_VOID(qiq_stdlib___sql_close, (int64_t handle), (handle)) {
    QiqSqlConnection *connection = qiq_sql_find(handle);
    if (connection == NULL) return;
    qiq_sql_close_cursors(handle);
    if (connection->pg != NULL) {
        connection->driver->close(connection->pg);
    } else if (connection->fork_count == qiq_sql_forks) {
        for (int index = 0; index < QIQ_SQL_CACHE; index++) {
            if (connection->cache[index].sql == NULL) continue;
            sqlite3_finalize(connection->cache[index].statement);
            free(connection->cache[index].sql);
        }
        sqlite3_close_v2(connection->db);
    }
    free(connection->filename);
    free(connection->pragmas);
    uint32_t generation = connection->generation + 1;
    memset(connection, 0, sizeof *connection);
    connection->generation = generation;
}

// ---------------------------------------------------------------------------
// Cursors: a statement whose rows are read a batch at a time, for results
// too big to hold at once. A SQLite cursor keeps its own prepared statement;
// a network database's streams its rows from the connection, which can't
// run anything else until they've been read or the cursor is closed.
// Handles work like connections'.

typedef struct {
    int64_t connection;       // its database's handle
    sqlite3_stmt *statement;  // SQLite's, or NULL
    char **columns;           // the column names
    size_t column_count;
    bool open;                // rows may be left
    uint32_t generation;
} QiqSqlCursor;

static QiqSqlCursor *qiq_sql_cursors;
static size_t qiq_sql_cursor_count;

static void qiq_sql_check_idle(QiqSqlConnection *connection) {
    if (connection->streaming == 0) return;
    qiq_sql_drop_pending();
    qiq_throw(QIQ_DATABASE_EXCEPTION,
              "a cursor is still reading rows from this database: read them all, or close() it, before running another statement");
}

static QiqSqlCursor *qiq_sql_find_cursor(int64_t handle) {
    size_t slot = (size_t)(uint32_t)handle;
    uint32_t generation = (uint32_t)((uint64_t)handle >> 32);
    if (slot == 0 || slot > qiq_sql_cursor_count) return NULL;
    QiqSqlCursor *cursor = &qiq_sql_cursors[slot - 1];
    return cursor->generation == generation && cursor->columns != NULL ? cursor : NULL;
}

// Ends a cursor: finalizes SQLite's statement, and for a network database,
// reads what's left (when `drain`) and frees the connection for other work.
static void qiq_sql_end_cursor(QiqSqlCursor *cursor, bool drain) {
    if (!cursor->open) return;
    cursor->open = false;
    if (cursor->statement != NULL) {
        sqlite3_finalize(cursor->statement);
        cursor->statement = NULL;
    }
    QiqSqlConnection *connection = qiq_sql_find(cursor->connection);
    if (connection != NULL && connection->pg != NULL && connection->streaming != 0) {
        connection->streaming = 0;
        if (drain) connection->driver->close_cursor(connection->pg);
    }
}

// Frees a cursor's slot (and names) once it's closed.
static void qiq_sql_free_cursor(QiqSqlCursor *cursor) {
    for (size_t i = 0; i < cursor->column_count; i++) free(cursor->columns[i]);
    free(cursor->columns);
    uint32_t generation = cursor->generation + 1;
    memset(cursor, 0, sizeof *cursor);
    cursor->generation = generation;
}

// When a database closes: its cursors end, without reading the rest.
static void qiq_sql_close_cursors(int64_t connection) {
    for (size_t i = 0; i < qiq_sql_cursor_count; i++) {
        QiqSqlCursor *cursor = &qiq_sql_cursors[i];
        if (cursor->columns != NULL && cursor->connection == connection) {
            qiq_sql_end_cursor(cursor, false);
            qiq_sql_free_cursor(cursor);
        }
    }
}

QIQ_GUARDED(int64_t, qiq_stdlib___sql_cursor, 0, (int64_t handle, const char *sql), (handle, sql)) {
    qiq_sql_clear_results();
    QiqSqlConnection *connection = qiq_sql_find(handle) != NULL ? qiq_sql_connection(handle) : NULL;
    if (connection == NULL) {
        qiq_sql_drop_pending();
        qiq_throw(QIQ_DATABASE_EXCEPTION, "the database is closed");
    }
    sqlite3_stmt *statement = NULL;
    if (connection->pg != NULL) {
        qiq_sql_check_idle(connection);
        // Sends the statement and reads the columns into the buffers.
        connection->driver->open_cursor(connection->pg, sql, "cursor");
    } else {
        jmp_buf environment;
        jmp_buf *previous = qiq_guard_enter(&environment);
        if (setjmp(environment)) {
            qiq_guard_leave(previous);
            qiq_sql_drop_pending();
            return 0;
        }
        statement = qiq_sql_prepare(connection, sql, "cursor", false);
        qiq_guard_leave(previous);
        int expected = sqlite3_bind_parameter_count(statement);
        if ((size_t)expected != qiq_sql_pending_count) {
            size_t given = qiq_sql_pending_count;
            qiq_sql_drop_pending();
            sqlite3_finalize(statement);
            qiq_throwf(QIQ_DATABASE_EXCEPTION, "cursor: the SQL has %d placeholder%s (?), but %zu value%s given", expected,
                       expected == 1 ? "" : "s", given, given == 1 ? " was" : "s were");
        }
        // Copied: the statement outlives the values.
        qiq_sql_bind(statement, SQLITE_TRANSIENT);
        qiq_sql_drop_pending();
        for (int column = 0; column < sqlite3_column_count(statement); column++) {
            const char *name = sqlite3_column_name(statement, column);
            qiq_sql_add_column(name, strlen(name));
        }
    }
    size_t slot = 0;
    while (slot < qiq_sql_cursor_count && qiq_sql_cursors[slot].columns != NULL) slot++;
    if (slot == qiq_sql_cursor_count) {
        QiqSqlCursor *grown = realloc(qiq_sql_cursors, (slot + 1) * sizeof(QiqSqlCursor));
        if (grown == NULL) qiq_fatal("out of memory");
        qiq_sql_cursors = grown;
        memset(&qiq_sql_cursors[slot], 0, sizeof(QiqSqlCursor));
        qiq_sql_cursor_count++;
    }
    QiqSqlCursor *cursor = &qiq_sql_cursors[slot];
    cursor->connection = handle;
    cursor->statement = statement;
    cursor->open = true;
    cursor->column_count = qiq_sql_column_count;
    cursor->columns = malloc((qiq_sql_column_count + 1) * sizeof(char *));
    if (cursor->columns == NULL) qiq_fatal("out of memory");
    for (size_t i = 0; i < qiq_sql_column_count; i++) {
        cursor->columns[i] = strdup(qiq_sql_columns[i]);
        if (cursor->columns[i] == NULL) qiq_fatal("out of memory");
    }
    qiq_sql_clear_results();
    int64_t id = (int64_t)((uint64_t)cursor->generation << 32 | (uint64_t)(slot + 1));
    if (connection->pg != NULL) connection->streaming = id;
    return id;
}

// Reads up to `count` more rows into the buffers; fewer means the end, after
// which the cursor is closed.
QIQ_GUARDED(int64_t, qiq_stdlib___sql_fetch, 0, (int64_t handle, int64_t count), (handle, count)) {
    qiq_sql_clear_results();
    QiqSqlCursor *cursor = qiq_sql_find_cursor(handle);
    if (cursor == NULL) qiq_throw(QIQ_DATABASE_EXCEPTION, "the cursor's database is closed");
    if (count < 1) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "Cursor::fetch: the count must be at least 1");
    for (size_t i = 0; i < cursor->column_count; i++) qiq_sql_add_column(cursor->columns[i], strlen(cursor->columns[i]));
    if (!cursor->open) return 0;
    int64_t rows = 0;
    if (cursor->statement != NULL) {
        int code = SQLITE_ROW;
        while (rows < count && (code = sqlite3_step(cursor->statement)) == SQLITE_ROW) {
            qiq_sql_add_row(cursor->statement, (int)cursor->column_count);
            rows++;
        }
        if (code != SQLITE_ROW) {
            QiqSqlConnection *connection = qiq_sql_find(cursor->connection);
            int error = connection != NULL ? sqlite3_extended_errcode(connection->db) : 0;
            char message[512];
            snprintf(message, sizeof message, "%s", connection != NULL ? sqlite3_errmsg(connection->db) : "the database is closed");
            qiq_sql_end_cursor(cursor, false);
            if (code != SQLITE_DONE) {
                qiq_sql_clear_results();
                qiq_sqlite_throw(error, message);
            }
        }
        return rows;
    }
    QiqSqlConnection *connection = qiq_sql_find(cursor->connection);
    if (connection == NULL || connection->streaming != handle) {
        cursor->open = false;
        return 0;
    }
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        // An error ends the cursor; the driver has already read to the end.
        cursor->open = false;
        connection->streaming = 0;
        qiq_rethrow(previous);
    }
    rows = connection->driver->fetch(connection->pg, count);
    qiq_guard_leave(previous);
    if (rows < count) {
        cursor->open = false;
        connection->streaming = 0;
    }
    return rows;
}

QIQ_GUARDED_VOID(qiq_stdlib___sql_close_cursor, (int64_t handle), (handle)) {
    QiqSqlCursor *cursor = qiq_sql_find_cursor(handle);
    if (cursor == NULL) return;
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        qiq_sql_free_cursor(cursor);
        qiq_rethrow(previous);
    }
    qiq_sql_end_cursor(cursor, true);
    qiq_guard_leave(previous);
    qiq_sql_free_cursor(cursor);
}

// ---------------------------------------------------------------------------
// Running statements.

// A prepared statement for `sql`, from the cache or newly prepared and
// added to it (evicting the least recently used). `caller` names the method
// for errors.
static sqlite3_stmt *qiq_sql_prepare(QiqSqlConnection *connection, const char *sql, const char *caller, bool keep) {
    connection->clock++;
    QiqSqlCached *oldest = &connection->cache[0];
    for (int index = 0; keep && index < QIQ_SQL_CACHE; index++) {
        QiqSqlCached *cached = &connection->cache[index];
        if (cached->sql != NULL && strcmp(cached->sql, sql) == 0) {
            cached->used = connection->clock;
            return cached->statement;
        }
        if (cached->sql == NULL || cached->used < oldest->used) oldest = cached;
        if (cached->sql == NULL) break;
    }
    sqlite3_stmt *statement = NULL;
    const char *tail = NULL;
    if (sqlite3_prepare_v3(connection->db, sql, -1, SQLITE_PREPARE_PERSISTENT, &statement, &tail) != SQLITE_OK) {
        qiq_sqlite_throw(sqlite3_extended_errcode(connection->db), sqlite3_errmsg(connection->db));
    }
    if (statement == NULL) qiq_throwf(QIQ_DATABASE_EXCEPTION, "%s: the SQL has no statement to run", caller);
    // Anything after the statement must be only spaces and comments.
    sqlite3_stmt *more = NULL;
    if (tail != NULL && *tail != '\0' && sqlite3_prepare_v2(connection->db, tail, -1, &more, NULL) == SQLITE_OK && more != NULL) {
        sqlite3_finalize(more);
        sqlite3_finalize(statement);
        qiq_throwf(QIQ_DATABASE_EXCEPTION, "%s runs one SQL statement, but this has more; run several with executeScript", caller);
    }
    for (int index = 1; index <= sqlite3_bind_parameter_count(statement); index++) {
        const char *name = sqlite3_bind_parameter_name(statement, index);
        if (name != NULL) {
            char message[256];
            snprintf(message, sizeof message, "%s: write placeholders as ?, not %.64s (the values are given in order after the SQL)", caller, name);
            sqlite3_finalize(statement);
            qiq_throw(QIQ_DATABASE_EXCEPTION, message);
        }
    }
    if (!keep) return statement;
    char *copy = strdup(sql);
    if (copy == NULL) qiq_fatal("out of memory");
    if (oldest->sql != NULL) {
        sqlite3_finalize(oldest->statement);
        free(oldest->sql);
    }
    *oldest = (QiqSqlCached){copy, statement, connection->clock};
    return statement;
}

// Binds the pending values. With SQLITE_STATIC, they must stay alive until
// the statement is reset; SQLITE_TRANSIENT copies them.
static void qiq_sql_bind(sqlite3_stmt *statement, sqlite3_destructor_type keep) {
    for (size_t index = 0; index < qiq_sql_pending_count; index++) {
        QiqSqlValue value = qiq_sql_pending[index];
        int at = (int)index + 1;
        switch (value.type) {
        case 'd': {
            double number;
            memcpy(&number, &value.payload, sizeof number);
            sqlite3_bind_double(statement, at, number);
            break;
        }
        case 's':
        case 'T': // with its offset, which SQLite's date functions understand
            if (value.payload == 0) sqlite3_bind_null(statement, at);
            else sqlite3_bind_text(statement, at, (const char *)(uintptr_t)value.payload, -1, keep);
            break;
        case 'a': {
            void *array = (void *)(uintptr_t)value.payload;
            if (array == NULL) sqlite3_bind_null(statement, at);
            // A null pointer would bind NULL rather than an empty blob.
            else if (QIQ_ARRAY_LENGTH(array) == 0) sqlite3_bind_zeroblob(statement, at, 0);
            else sqlite3_bind_blob64(statement, at, QIQ_ARRAY_DATA(array), (sqlite3_uint64)QIQ_ARRAY_LENGTH(array), keep);
            break;
        }
        case 'n':
            sqlite3_bind_null(statement, at);
            break;
        default: // 'i', 'b', 'y'
            sqlite3_bind_int64(statement, at, (sqlite3_int64)value.payload);
        }
    }
}

// Runs `sql` with the pending values. `mode` 0 (execute) returns the number
// of rows changed; 1 (query) collects every row and 2 (queryOne) the first,
// returning how many were collected.
QIQ_GUARDED(int64_t, qiq_stdlib___sql_run, 0, (int64_t handle, const char *sql, int64_t mode), (handle, sql, mode)) {
    const char *caller = mode == 0 ? "execute" : mode == 1 ? "query" : "queryOne";
    qiq_sql_clear_results();
    QiqSqlConnection *connection = qiq_sql_find(handle) != NULL ? qiq_sql_connection(handle) : NULL;
    if (connection == NULL) {
        qiq_sql_drop_pending();
        qiq_throw(QIQ_DATABASE_EXCEPTION, "the database is closed");
    }
    if (connection->pg != NULL) {
        qiq_sql_check_idle(connection);
        return connection->driver->run(connection->pg, sql, mode, caller);
    }
    sqlite3_stmt *statement;
    {
        // Prepare errors throw from inside; the pending values go first.
        jmp_buf environment;
        jmp_buf *previous = qiq_guard_enter(&environment);
        if (setjmp(environment)) {
            qiq_guard_leave(previous);
            qiq_sql_drop_pending();
            return 0;
        }
        statement = qiq_sql_prepare(connection, sql, caller, true);
        qiq_guard_leave(previous);
    }
    int expected = sqlite3_bind_parameter_count(statement);
    if ((size_t)expected != qiq_sql_pending_count) {
        size_t given = qiq_sql_pending_count;
        qiq_sql_drop_pending();
        qiq_throwf(QIQ_DATABASE_EXCEPTION, "%s: the SQL has %d placeholder%s (?), but %zu value%s given", caller, expected,
                   expected == 1 ? "" : "s", given, given == 1 ? " was" : "s were");
    }
    qiq_sql_bind(statement, SQLITE_STATIC);
    int columns = sqlite3_column_count(statement);
    if (mode != 0) {
        qiq_sql_columns = qiq_sql_grow(qiq_sql_columns, &qiq_sql_columns_capacity, (size_t)columns, sizeof(char *));
        for (int column = 0; column < columns; column++) {
            const char *name = sqlite3_column_name(statement, column);
            qiq_sql_columns[column] = qiq_string_from(name, strlen(name));
        }
        qiq_sql_column_count = (size_t)columns;
    }
    int64_t rows = 0;
    int code;
    while ((code = sqlite3_step(statement)) == SQLITE_ROW) {
        if (mode == 0) continue;
        qiq_sql_add_row(statement, columns);
        rows++;
        if (mode == 2) {
            code = SQLITE_DONE;
            break;
        }
    }
    // A statement that changes nothing (a SELECT) would report the last change's count.
    int64_t result = mode != 0 ? rows : sqlite3_stmt_readonly(statement) ? 0 : (int64_t)sqlite3_changes64(connection->db);
    char message[512];
    int error = code != SQLITE_DONE ? sqlite3_extended_errcode(connection->db) : 0;
    if (code != SQLITE_DONE) snprintf(message, sizeof message, "%s", sqlite3_errmsg(connection->db));
    if (code == SQLITE_DONE) qiq_sql_remember(connection, statement);
    sqlite3_reset(statement);
    sqlite3_clear_bindings(statement);
    qiq_sql_drop_pending();
    if (code != SQLITE_DONE) {
        qiq_sql_clear_results();
        qiq_sqlite_throw(error, message);
    }
    return result;
}

// Runs several statements separated by semicolons, without values.
QIQ_GUARDED_VOID(qiq_stdlib___sql_execute_script, (int64_t handle, const char *sql), (handle, sql)) {
    QiqSqlConnection *connection = qiq_sql_connection(handle);
    if (connection->pg != NULL) {
        qiq_sql_check_idle(connection);
        connection->driver->script(connection->pg, sql);
        return;
    }
    const char *next = sql;
    while (*next != '\0') {
        sqlite3_stmt *statement = NULL;
        if (sqlite3_prepare_v2(connection->db, next, -1, &statement, &next) != SQLITE_OK) {
            qiq_sqlite_throw(sqlite3_extended_errcode(connection->db), sqlite3_errmsg(connection->db));
        }
        if (statement == NULL) break; // only spaces and comments were left
        int code;
        while ((code = sqlite3_step(statement)) == SQLITE_ROW) {}
        if (code != SQLITE_DONE) {
            char message[512];
            int error = sqlite3_extended_errcode(connection->db);
            snprintf(message, sizeof message, "%s", sqlite3_errmsg(connection->db));
            sqlite3_finalize(statement);
            qiq_sqlite_throw(error, message);
        }
        qiq_sql_remember(connection, statement);
        sqlite3_finalize(statement);
    }
}

QIQ_GUARDED(int64_t, qiq_stdlib___sql_last_insert_id, 0, (int64_t handle), (handle)) {
    QiqSqlConnection *connection = qiq_sql_connection(handle);
    if (connection->pg != NULL) {
        qiq_sql_check_idle(connection);
        return connection->driver->last_insert_id(connection->pg);
    }
    return (int64_t)sqlite3_last_insert_rowid(connection->db);
}

QIQ_GUARDED(bool, qiq_stdlib___sql_in_transaction, false, (int64_t handle), (handle)) {
    QiqSqlConnection *connection = qiq_sql_connection(handle);
    if (connection->pg != NULL) return connection->driver->in_transaction(connection->pg);
    return sqlite3_get_autocommit(connection->db) == 0;
}

// ---------------------------------------------------------------------------
// Handing the last query's results to the qiq side. Each list is taken
// once: strings move into the array rather than being copied.

static void *qiq_sql_int_array(const int64_t *data, size_t count) {
    void *array = qiq_array_create((int64_t)count, 8, 0);
    if (count > 0) memcpy(QIQ_ARRAY_DATA(array), data, count * 8);
    return array;
}

void *qiq_stdlib___sql_columns(void) {
    void *array = qiq_array_create((int64_t)qiq_sql_column_count, 8, 1);
    if (qiq_sql_column_count > 0) memcpy(QIQ_ARRAY_DATA(array), qiq_sql_columns, qiq_sql_column_count * sizeof(char *));
    qiq_sql_column_count = 0;
    return array;
}

void *qiq_stdlib___sql_kinds(void) {
    return qiq_sql_int_array(qiq_sql_kinds, qiq_sql_cells);
}

void *qiq_stdlib___sql_values(void) {
    return qiq_sql_int_array(qiq_sql_values, qiq_sql_cells);
}

void *qiq_stdlib___sql_doubles(void) {
    void *array = qiq_array_create((int64_t)qiq_sql_double_count, 8, 0);
    if (qiq_sql_double_count > 0) memcpy(QIQ_ARRAY_DATA(array), qiq_sql_doubles, qiq_sql_double_count * sizeof(double));
    return array;
}

void *qiq_stdlib___sql_texts(void) {
    void *array = qiq_array_create((int64_t)qiq_sql_text_count, 8, 1);
    if (qiq_sql_text_count > 0) memcpy(QIQ_ARRAY_DATA(array), qiq_sql_texts, qiq_sql_text_count * sizeof(char *));
    qiq_sql_text_count = 0;
    return array;
}

void *qiq_stdlib___sql_blobs(void) {
    void *array = qiq_array_create((int64_t)qiq_sql_blob_length, 1, 0);
    if (qiq_sql_blob_length > 0) memcpy(QIQ_ARRAY_DATA(array), qiq_sql_blobs, qiq_sql_blob_length);
    return array;
}

// Where each byte array starts in __sql_blobs, then where the last ends.
void *qiq_stdlib___sql_blob_starts(void) {
    qiq_sql_blob_starts = qiq_sql_grow(qiq_sql_blob_starts, &qiq_sql_blob_starts_capacity, qiq_sql_blob_count + 1, sizeof(int64_t));
    qiq_sql_blob_starts[qiq_sql_blob_count] = (int64_t)qiq_sql_blob_length;
    return qiq_sql_int_array(qiq_sql_blob_starts, qiq_sql_blob_count + 1);
}

// Bytes `start` to `end` of `data`, for Row::getBytes.
void *qiq_stdlib___sql_slice(void *data, int64_t start, int64_t end) {
    void *array = qiq_array_create(end - start, 1, 0);
    if (end > start) memcpy(QIQ_ARRAY_DATA(array), (const char *)QIQ_ARRAY_DATA(data) + start, (size_t)(end - start));
    return array;
}
