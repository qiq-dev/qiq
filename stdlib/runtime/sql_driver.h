// Shared by sql.c (std/sql, with SQLite) and its network drivers,
// postgres.c and mysql.c: the values for a statement's placeholders, the
// buffers a query's rows are collected in for the qiq side, and the
// functions each driver provides.
#ifndef QIQ_SQL_DRIVER_H
#define QIQ_SQL_DRIVER_H

#include "qiq_runtime.h"

// What a cell holds, as stdlib/sql.qiq reads it. A decimal is PostgreSQL's
// numeric, kept as its exact text.
enum { QIQ_SQL_INT = 1, QIQ_SQL_DOUBLE = 2, QIQ_SQL_TEXT = 3, QIQ_SQL_BYTES = 4, QIQ_SQL_NULL = 5, QIQ_SQL_DECIMAL = 6 };

typedef struct {
    char type;                // 'i' int, 'd' double, 'b' bool, 'y' byte, 's' string, 'a' byte[], 'n' null
    uint64_t payload;         // the value's bits; a pointer for 's' and 'a' (NULL for SQL NULL)
} QiqSqlValue;

extern QiqSqlValue *qiq_sql_pending;
extern size_t qiq_sql_pending_count;
void qiq_sql_drop_pending(void);

// Throws the exception for a database error: UniqueViolationException
// when `unique`, ConstraintViolationException for another SQLSTATE in class
// 23, and DatabaseException otherwise, with the SQLSTATE and the database's
// error number.
_Noreturn void qiq_sql_throw(const char *state, int64_t code, bool unique, const char *message);

// Increases in each process forked after the first database was opened.
extern uint64_t qiq_sql_forks;

void qiq_sql_clear_results(void);
void qiq_sql_add_column(const char *name, size_t length);
// An int cell, or (with QIQ_SQL_NULL) a NULL one.
void qiq_sql_add_cell(int64_t kind, int64_t value);
void qiq_sql_add_double(double value);
// A text or decimal cell; text that isn't valid UTF-8 is stored as bytes.
void qiq_sql_add_text(const char *text, size_t length, int64_t kind);
void qiq_sql_add_bytes(const void *bytes, size_t length);
// The int value of the first cell, for drivers' own queries.
int64_t qiq_sql_first_int(void);

// Whether a statement is a setting the connection keeps (PRAGMA in SQLite,
// SET in PostgreSQL), so it can be run again on a new connection.
bool qiq_sql_is_setting(const char *sql, size_t length, const char *keyword);

// A network database's driver. `run` runs one statement with the pending
// values: mode 0 (execute) returns the rows changed, 1 (query) collects
// every row and 2 (queryOne) the first, returning how many. Each function
// throws DatabaseException or NetworkException, and reconnects first when
// the process was forked or the connection was lost.
typedef struct {
    void *(*open)(const char *dsn);
    int64_t (*run)(void *connection, const char *sql, int64_t mode, const char *caller);
    void (*script)(void *connection, const char *sql);
    int64_t (*last_insert_id)(void *connection);
    bool (*in_transaction)(void *connection);
    void (*close)(void *connection);
    // Cursors, one at a time per connection: open_cursor sends the
    // statement with the pending values and reads its columns into the
    // buffers; fetch reads up to `count` rows into them (fewer at the end,
    // where the connection is free again); close_cursor reads and drops the
    // rest.
    void (*open_cursor)(void *connection, const char *sql, const char *caller);
    int64_t (*fetch)(void *connection, int64_t count);
    void (*close_cursor)(void *connection);
} QiqSqlDriver;

extern const QiqSqlDriver qiq_postgres_driver; // postgres.c
extern const QiqSqlDriver qiq_mysql_driver;    // mysql.c

// The parts of a URL such as scheme://user:password@host:port/database?a=b,
// percent-decoded; NULL for those it doesn't have (sql.c). Options are
// "name=value" strings. Returns false when it isn't such a URL.
typedef struct {
    char *user, *password, *host, *database;
    int64_t port;             // 0 when not given
    char **options;
    size_t option_count;
} QiqSqlUrl;
bool qiq_sql_parse_url(const char *url, QiqSqlUrl *out);
void qiq_sql_free_url(QiqSqlUrl *url);
// Whether a host is this machine, where TLS is off by default.
bool qiq_sql_is_local_host(const char *host);
// How a network driver connects, from a URL's options.
typedef struct {
    bool tls;                 // sslmode; on by default except for this machine
    int64_t timeout;          // connect_timeout, in seconds (10)
    char *ca_file;            // sslrootcert: the CA certificates to trust, or NULL for the system's
    char *socket;             // a Unix socket (host=/dir or socket=/file), or NULL for TCP
} QiqSqlOptions;

// Reads the options the network drivers take: sslmode=disable|require (or
// verify-full), connect_timeout=seconds, sslrootcert=file (which turns TLS
// on), and host=path or socket=path (or a URL host that is a path) for a
// Unix socket, over which TLS is off. Returns NULL, or a message about an
// option that isn't one of these or doesn't fit with the others.
const char *qiq_sql_connection_options(const QiqSqlUrl *url, const char *host, QiqSqlOptions *out);
void qiq_sql_free_options(QiqSqlOptions *options);
// The socket file for `options->socket`: the path itself, or `file_name`
// in it when it's a directory (as with PostgreSQL's host=/var/run/postgresql).
char *qiq_sql_socket_file(const char *socket, const char *file_name);
// Fills `out` with `count` random bytes, or fails with DatabaseException.
void qiq_sql_random(unsigned char *out, size_t count);

#endif
