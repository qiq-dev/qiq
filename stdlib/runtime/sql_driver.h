// Shared by sql.c (std/sql, with SQLite) and postgres.c (its PostgreSQL
// driver): the values for a statement's placeholders, and the buffers a
// query's rows are collected in for the qiq side.
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

// The PostgreSQL driver (postgres.c).
typedef struct QiqPostgres QiqPostgres;
QiqPostgres *qiq_pg_open(const char *dsn);
int64_t qiq_pg_run(QiqPostgres *pg, const char *sql, int64_t mode, const char *caller);
void qiq_pg_script(QiqPostgres *pg, const char *sql);
bool qiq_pg_in_transaction(QiqPostgres *pg);
void qiq_pg_close(QiqPostgres *pg);

#endif
