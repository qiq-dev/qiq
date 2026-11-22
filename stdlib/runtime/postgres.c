// The PostgreSQL driver for std/sql (see sql.c): the frontend/backend
// protocol (version 3) over net.c's connections, with TLS from Mbed TLS and
// SCRAM-SHA-256, MD5, or plain password logins. Nothing else to install.
//
// Each statement is one round trip in the extended query protocol: Parse
// (with the `?` placeholders rewritten as $1, $2, ...), Bind (the values,
// sent separately from the SQL), Describe, Execute, and Sync. Results come
// back as text and are converted by column type: integers, floats, and
// booleans to the cells Row's getters read, numeric to a decimal kept as
// its exact text, bytea to bytes, and everything else to text.
//
// A connection used in a process forked after it was opened (the HTTP
// server's workers) leaves the parent's socket alone and connects again; one
// whose connection was lost connects again on the next statement. Either
// way the SET statements it ran are run again on the new connection.
// Errors from the server throw DatabaseException; network failures throw
// NetworkException.
#include <math.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>

#include "mbedtls/md.h"
#include "mbedtls/pkcs5.h"
#include "qiq_runtime.h"
#include "sql_driver.h"

typedef struct QiqPostgres QiqPostgres;

// PostgreSQL's type ids (pg_type.oid) for the types converted specially.
enum {
    PG_BOOL = 16, PG_BYTEA = 17, PG_INT8 = 20, PG_INT2 = 21, PG_INT4 = 23, PG_OID = 26,
    PG_FLOAT4 = 700, PG_FLOAT8 = 701, PG_NUMERIC = 1700,
};

struct QiqPostgres {
    int64_t socket;           // -1 when not connected
    char *host, *user, *password, *database;
    int64_t port;
    bool tls;
    int64_t connect_timeout;  // seconds
    char status;              // from ReadyForQuery: 'I' idle, 'T' in a transaction, 'E' in a failed one
    uint64_t fork_count;      // qiq_sql_forks when it connected
    bool lost_in_transaction; // the connection was lost with a transaction open
    bool in_io;               // talking to the server: a failure now loses the connection
    bool streaming;           // a cursor's rows are still arriving
    char *settings;           // the SET statements it ran, to run again on a new connection
    // The message being written, and the one being read.
    unsigned char *out;
    size_t out_length, out_capacity;
    unsigned char *in;
    size_t in_length, in_capacity;
    // The result columns' types.
    int32_t *types;
    size_t type_count, types_capacity;
};

static void *pg_grow(void *data, size_t *capacity, size_t needed, size_t size) {
    if (needed <= *capacity) return data;
    size_t grown = *capacity < 256 ? 256 : *capacity * 2;
    if (grown < needed) grown = needed;
    data = realloc(data, grown * size);
    if (data == NULL) qiq_fatal("out of memory");
    *capacity = grown;
    return data;
}

static char *pg_strdup(const char *text, size_t length) {
    char *copy = malloc(length + 1);
    if (copy == NULL) qiq_fatal("out of memory");
    memcpy(copy, text, length);
    copy[length] = '\0';
    return copy;
}

// ---------------------------------------------------------------------------
// Writing messages: a type byte (none for the startup message), a 32-bit
// big-endian length that counts itself, and the contents.

static void pg_put(QiqPostgres *pg, const void *data, size_t length) {
    pg->out = pg_grow(pg->out, &pg->out_capacity, pg->out_length + length, 1);
    memcpy(pg->out + pg->out_length, data, length);
    pg->out_length += length;
}

static void pg_put_byte(QiqPostgres *pg, unsigned char byte) {
    pg_put(pg, &byte, 1);
}

static void pg_put_int16(QiqPostgres *pg, int value) {
    unsigned char bytes[2] = {(unsigned char)(value >> 8), (unsigned char)value};
    pg_put(pg, bytes, 2);
}

static void pg_put_int32(QiqPostgres *pg, int64_t value) {
    uint32_t bits = (uint32_t)value;
    unsigned char bytes[4] = {(unsigned char)(bits >> 24), (unsigned char)(bits >> 16), (unsigned char)(bits >> 8), (unsigned char)bits};
    pg_put(pg, bytes, 4);
}

static void pg_put_string(QiqPostgres *pg, const char *text) {
    pg_put(pg, text, strlen(text) + 1);
}

// Starts a message; returns where its length goes, for pg_end.
static size_t pg_begin(QiqPostgres *pg, char type) {
    if (type != 0) pg_put_byte(pg, (unsigned char)type);
    size_t at = pg->out_length;
    pg_put_int32(pg, 0);
    return at;
}

static void pg_end(QiqPostgres *pg, size_t at) {
    uint32_t length = (uint32_t)(pg->out_length - at);
    pg->out[at] = (unsigned char)(length >> 24);
    pg->out[at + 1] = (unsigned char)(length >> 16);
    pg->out[at + 2] = (unsigned char)(length >> 8);
    pg->out[at + 3] = (unsigned char)length;
}

static void pg_flush(QiqPostgres *pg) {
    qiq_net_send(pg->socket, pg->out, pg->out_length);
    pg->out_length = 0;
}

// ---------------------------------------------------------------------------
// Reading messages

static char pg_read(QiqPostgres *pg) {
    unsigned char header[5];
    qiq_net_receive(pg->socket, header, 5);
    uint32_t length = (uint32_t)header[1] << 24 | (uint32_t)header[2] << 16 | (uint32_t)header[3] << 8 | header[4];
    if (length < 4 || length > 0x40000000) qiq_throw(QIQ_DATABASE_EXCEPTION, "the PostgreSQL server sent a malformed message");
    pg->in_length = length - 4;
    pg->in = pg_grow(pg->in, &pg->in_capacity, pg->in_length + 1, 1);
    qiq_net_receive(pg->socket, pg->in, pg->in_length);
    pg->in[pg->in_length] = '\0';
    return (char)header[0];
}

typedef struct {
    const unsigned char *at, *end;
} PgReader;

static PgReader pg_reader(QiqPostgres *pg) {
    return (PgReader){pg->in, pg->in + pg->in_length};
}

static _Noreturn void pg_malformed(void) {
    qiq_throw(QIQ_DATABASE_EXCEPTION, "the PostgreSQL server sent a malformed message");
}

static int32_t pg_int32(PgReader *reader) {
    if (reader->end - reader->at < 4) pg_malformed();
    const unsigned char *b = reader->at;
    reader->at += 4;
    return (int32_t)((uint32_t)b[0] << 24 | (uint32_t)b[1] << 16 | (uint32_t)b[2] << 8 | b[3]);
}

static int pg_int16(PgReader *reader) {
    if (reader->end - reader->at < 2) pg_malformed();
    const unsigned char *b = reader->at;
    reader->at += 2;
    return (int16_t)((uint16_t)b[0] << 8 | b[1]);
}

static const char *pg_string(PgReader *reader) {
    const unsigned char *start = reader->at;
    while (reader->at < reader->end && *reader->at != 0) reader->at++;
    if (reader->at == reader->end) pg_malformed();
    reader->at++;
    return (const char *)start;
}

// An ErrorResponse's text: the message, then the detail when there is one,
// as in `duplicate key value violates unique constraint "users_email_key":
// Key (email)=(ada@example.com) already exists.`
// Returns whether the error is FATAL (or PANIC): the server is closing the
// connection.
static bool pg_error_text(QiqPostgres *pg, char *out, size_t size, char *state) {
    PgReader reader = pg_reader(pg);
    const char *message = "the server reported an error", *detail = NULL;
    bool fatal = false;
    state[0] = '\0';
    while (reader.at < reader.end && *reader.at != 0) {
        char field = (char)*reader.at++;
        const char *value = pg_string(&reader);
        if (field == 'M') message = value;
        else if (field == 'D') detail = value;
        else if (field == 'C') snprintf(state, 6, "%s", value);
        else if (field == 'V') fatal = strcmp(value, "FATAL") == 0 || strcmp(value, "PANIC") == 0;
    }
    if (detail != NULL) snprintf(out, size, "%s: %s", message, detail);
    else snprintf(out, size, "%s", message);
    return fatal;
}

// ---------------------------------------------------------------------------
// Base64, for SCRAM's messages.

static size_t pg_base64_encode(const unsigned char *data, size_t length, char *out) {
    static const char alphabet[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    size_t o = 0;
    for (size_t i = 0; i < length; i += 3) {
        uint32_t bits = (uint32_t)data[i] << 16 | (i + 1 < length ? (uint32_t)data[i + 1] << 8 : 0) | (i + 2 < length ? data[i + 2] : 0);
        out[o++] = alphabet[bits >> 18 & 63];
        out[o++] = alphabet[bits >> 12 & 63];
        out[o++] = i + 1 < length ? alphabet[bits >> 6 & 63] : '=';
        out[o++] = i + 2 < length ? alphabet[bits & 63] : '=';
    }
    out[o] = '\0';
    return o;
}

// Decodes up to `size` bytes; returns how many, or -1 for invalid base64.
static long pg_base64_decode(const char *text, size_t length, unsigned char *out, size_t size) {
    uint32_t bits = 0;
    int count = 0;
    size_t o = 0;
    for (size_t i = 0; i < length; i++) {
        char c = text[i];
        int value = c >= 'A' && c <= 'Z' ? c - 'A' : c >= 'a' && c <= 'z' ? c - 'a' + 26 : c >= '0' && c <= '9' ? c - '0' + 52
                  : c == '+' ? 62 : c == '/' ? 63 : c == '=' ? -2 : -1;
        if (value == -2) break;
        if (value < 0) return -1;
        bits = bits << 6 | (uint32_t)value;
        if (++count == 4) {
            if (o + 3 > size) return -1;
            out[o++] = (unsigned char)(bits >> 16);
            out[o++] = (unsigned char)(bits >> 8);
            out[o++] = (unsigned char)bits;
            bits = 0;
            count = 0;
        }
    }
    if (count == 2 || count == 3) {
        if (o + (size_t)(count - 1) > size) return -1;
        bits <<= 6 * (4 - count);
        out[o++] = (unsigned char)(bits >> 16);
        if (count == 3) out[o++] = (unsigned char)(bits >> 8);
    } else if (count == 1) {
        return -1;
    }
    return (long)o;
}

static void pg_hmac(const unsigned char *key, size_t key_length, const void *data, size_t length, unsigned char out[32]) {
    mbedtls_md_hmac(mbedtls_md_info_from_type(MBEDTLS_MD_SHA256), key, key_length, data, length, out);
}

// ---------------------------------------------------------------------------
// Logging in

static _Noreturn void pg_auth_failed(QiqPostgres *pg, const char *reason) {
    qiq_throwf(QIQ_DATABASE_EXCEPTION, "logging in to PostgreSQL at %s as %s: %s", pg->host, pg->user, reason);
}

// SCRAM-SHA-256 (RFC 5802 and 7677), without channel binding.
static void pg_scram(QiqPostgres *pg) {
    unsigned char random[18];
    qiq_sql_random(random, sizeof random);
    char nonce[32];
    pg_base64_encode(random, sizeof random, nonce);
    // The user name is the one in the startup message; SCRAM's is ignored.
    char first_bare[64];
    snprintf(first_bare, sizeof first_bare, "n=,r=%s", nonce);
    size_t at = pg_begin(pg, 'p');
    pg_put_string(pg, "SCRAM-SHA-256");
    pg_put_int32(pg, (int64_t)(3 + strlen(first_bare)));
    pg_put(pg, "n,,", 3);
    pg_put(pg, first_bare, strlen(first_bare));
    pg_end(pg, at);
    pg_flush(pg);

    char type = pg_read(pg);
    if (type == 'E') {
        char message[512], state[6];
        pg_error_text(pg, message, sizeof message, state);
        pg_auth_failed(pg, message);
    }
    PgReader reader = pg_reader(pg);
    if (type != 'R' || pg_int32(&reader) != 11) pg_auth_failed(pg, "unexpected reply during SCRAM");
    size_t server_first_length = (size_t)(reader.end - reader.at);
    char *server_first = pg_strdup((const char *)reader.at, server_first_length);
    // r=<nonce>,s=<salt>,i=<iterations>
    const char *server_nonce = NULL, *salt_text = NULL;
    size_t server_nonce_length = 0, salt_length = 0;
    long iterations = 0;
    for (char *field = server_first; *field != '\0';) {
        char *comma = strchr(field, ',');
        size_t length = comma != NULL ? (size_t)(comma - field) : strlen(field);
        if (length > 2 && field[1] == '=') {
            if (field[0] == 'r') server_nonce = field + 2, server_nonce_length = length - 2;
            else if (field[0] == 's') salt_text = field + 2, salt_length = length - 2;
            else if (field[0] == 'i') iterations = strtol(field + 2, NULL, 10);
        }
        field += length + (comma != NULL ? 1 : 0);
    }
    unsigned char salt[128];
    long salt_bytes = salt_text != NULL ? pg_base64_decode(salt_text, salt_length, salt, sizeof salt) : -1;
    if (server_nonce == NULL || strncmp(server_nonce, nonce, strlen(nonce)) != 0 || salt_bytes <= 0 || iterations < 1 || iterations > 10000000) {
        free(server_first);
        pg_auth_failed(pg, "the server's SCRAM reply isn't valid");
    }
    unsigned char salted[32], client_key[32], stored_key[32], signature[32], proof[32], server_key[32], server_signature[32];
    const char *password = pg->password != NULL ? pg->password : "";
    mbedtls_pkcs5_pbkdf2_hmac_ext(MBEDTLS_MD_SHA256, (const unsigned char *)password, strlen(password), salt, (size_t)salt_bytes,
                                  (unsigned int)iterations, 32, salted);
    pg_hmac(salted, 32, "Client Key", 10, client_key);
    mbedtls_md(mbedtls_md_info_from_type(MBEDTLS_MD_SHA256), client_key, 32, stored_key);
    // c=biws is base64("n,,").
    size_t final_capacity = server_nonce_length + 128;
    char *final_message = malloc(final_capacity);
    size_t auth_capacity = strlen(first_bare) + server_first_length + final_capacity + 8;
    char *auth = malloc(auth_capacity);
    if (final_message == NULL || auth == NULL) qiq_fatal("out of memory");
    int final_length = snprintf(final_message, final_capacity, "c=biws,r=%.*s", (int)server_nonce_length, server_nonce);
    int auth_length = snprintf(auth, auth_capacity, "%s,%s,%s", first_bare, server_first, final_message);
    free(server_first);
    pg_hmac(stored_key, 32, auth, (size_t)auth_length, signature);
    for (int i = 0; i < 32; i++) proof[i] = client_key[i] ^ signature[i];
    char proof_text[48];
    pg_base64_encode(proof, 32, proof_text);
    at = pg_begin(pg, 'p');
    pg_put(pg, final_message, (size_t)final_length);
    pg_put(pg, ",p=", 3);
    pg_put(pg, proof_text, strlen(proof_text));
    pg_end(pg, at);
    free(final_message);
    pg_flush(pg);
    pg_hmac(salted, 32, "Server Key", 10, server_key);
    pg_hmac(server_key, 32, auth, (size_t)auth_length, server_signature);
    free(auth);

    type = pg_read(pg);
    if (type == 'E') {
        char message[512], state[6];
        pg_error_text(pg, message, sizeof message, state);
        pg_auth_failed(pg, message);
    }
    reader = pg_reader(pg);
    if (type != 'R' || pg_int32(&reader) != 12) pg_auth_failed(pg, "unexpected reply during SCRAM");
    // v=<base64 ServerSignature>: proves the server knows the password too.
    unsigned char verifier[64];
    long verifier_length = reader.end - reader.at > 2 && reader.at[0] == 'v' && reader.at[1] == '='
                               ? pg_base64_decode((const char *)reader.at + 2, (size_t)(reader.end - reader.at - 2), verifier, sizeof verifier)
                               : -1;
    if (verifier_length != 32 || memcmp(verifier, server_signature, 32) != 0) {
        pg_auth_failed(pg, "the server's SCRAM signature is wrong: it may not be the server you meant");
    }
}

static void pg_md5_hex(const char *a, size_t a_length, const char *b, size_t b_length, char out[33]) {
    unsigned char digest[16];
    mbedtls_md_context_t context;
    mbedtls_md_init(&context);
    mbedtls_md_setup(&context, mbedtls_md_info_from_type(MBEDTLS_MD_MD5), 0);
    mbedtls_md_starts(&context);
    mbedtls_md_update(&context, (const unsigned char *)a, a_length);
    mbedtls_md_update(&context, (const unsigned char *)b, b_length);
    mbedtls_md_finish(&context, digest);
    mbedtls_md_free(&context);
    for (int i = 0; i < 16; i++) snprintf(out + i * 2, 3, "%02x", digest[i]);
}

static void pg_send_password(QiqPostgres *pg, const char *password) {
    size_t at = pg_begin(pg, 'p');
    pg_put_string(pg, password);
    pg_end(pg, at);
    pg_flush(pg);
}

// Reads messages until ReadyForQuery, throwing for an ErrorResponse.
static void pg_until_ready(QiqPostgres *pg) {
    for (;;) {
        char type = pg_read(pg);
        if (type == 'Z') {
            pg->status = pg->in_length > 0 ? (char)pg->in[0] : 'I';
            return;
        }
        if (type == 'E') {
            char message[512], state[6];
            pg_error_text(pg, message, sizeof message, state);
            qiq_throwf(QIQ_DATABASE_EXCEPTION, "PostgreSQL at %s: %s", pg->host, message);
        }
        // ParameterStatus, BackendKeyData, notices, and notifications.
    }
}

static void pg_simple_query(QiqPostgres *pg, const char *sql);

// Connects, logs in, and runs the remembered SET statements.
static void pg_connect(QiqPostgres *pg) {
    pg->socket = qiq_net_connect(pg->host, pg->port, pg->connect_timeout);
    pg->status = 'I';
    if (pg->tls) {
        // SSLRequest: the server answers with one byte, before any TLS.
        pg_put_int32(pg, 8);
        pg_put_int32(pg, 80877103);
        pg_flush(pg);
        char answer;
        qiq_net_receive(pg->socket, &answer, 1);
        if (answer != 'S') {
            qiq_throwf(QIQ_DATABASE_EXCEPTION,
                       "the PostgreSQL server at %s doesn't accept TLS connections; if the network to it is trusted (the same machine "
                       "or a private network), add sslmode=disable to the URL",
                       pg->host);
        }
        qiq_net_start_tls(pg->socket, pg->host);
    }
    size_t at = pg_begin(pg, 0);
    pg_put_int32(pg, 196608); // protocol 3.0
    pg_put_string(pg, "user");
    pg_put_string(pg, pg->user);
    pg_put_string(pg, "database");
    pg_put_string(pg, pg->database);
    pg_put_string(pg, "client_encoding");
    pg_put_string(pg, "UTF8");
    pg_put_string(pg, "application_name");
    pg_put_string(pg, "qiq");
    pg_put_byte(pg, 0);
    pg_end(pg, at);
    pg_flush(pg);
    for (;;) {
        char type = pg_read(pg);
        if (type == 'E') {
            char message[512], state[6];
            pg_error_text(pg, message, sizeof message, state);
            pg_auth_failed(pg, message);
        }
        if (type == 'v') continue; // NegotiateProtocolVersion: fine with 3.0
        if (type != 'R') pg_auth_failed(pg, "unexpected reply to the startup message");
        PgReader reader = pg_reader(pg);
        int32_t code = pg_int32(&reader);
        if (code == 0) break;
        if (code == 3) {
            if (pg->password == NULL) pg_auth_failed(pg, "the server asks for a password; add one to the URL or set PGPASSWORD");
            pg_send_password(pg, pg->password);
        } else if (code == 5) {
            if (pg->password == NULL) pg_auth_failed(pg, "the server asks for a password; add one to the URL or set PGPASSWORD");
            if (reader.end - reader.at < 4) pg_malformed();
            // "md5" + md5(md5(password + user) + salt)
            char inner[33], outer[36] = "md5";
            pg_md5_hex(pg->password, strlen(pg->password), pg->user, strlen(pg->user), inner);
            pg_md5_hex(inner, 32, (const char *)reader.at, 4, outer + 3);
            pg_send_password(pg, outer);
        } else if (code == 10) {
            bool scram = false;
            while (reader.at < reader.end && *reader.at != 0) {
                if (strcmp(pg_string(&reader), "SCRAM-SHA-256") == 0) scram = true;
            }
            if (!scram) pg_auth_failed(pg, "the server offers no login method qiq supports");
            if (pg->password == NULL) pg_auth_failed(pg, "the server asks for a password; add one to the URL or set PGPASSWORD");
            pg_scram(pg);
        } else {
            pg_auth_failed(pg, "the server asks for a login method qiq doesn't support (Kerberos, GSSAPI, or SSPI); use a password");
        }
    }
    pg_until_ready(pg);
    // Statements can take as long as they need.
    qiq_net_set_timeout_ms(pg->socket, -1);
    if (pg->settings != NULL) pg_simple_query(pg, pg->settings);
}

// ---------------------------------------------------------------------------
// Opening: postgres://user:password@host:port/database?sslmode=...

static int pg_hex_digit(char c) {
    return c >= '0' && c <= '9' ? c - '0' : c >= 'a' && c <= 'f' ? c - 'a' + 10 : c >= 'A' && c <= 'F' ? c - 'A' + 10 : -1;
}

static void pg_free(QiqPostgres *pg) {
    free(pg->host);
    free(pg->user);
    free(pg->password);
    free(pg->database);
    free(pg->settings);
    free(pg->out);
    free(pg->in);
    free(pg->types);
    free(pg);
}

static char *pg_copy(const char *text) {
    return pg_strdup(text, strlen(text));
}

QiqPostgres *qiq_pg_open(const char *dsn) {
    QiqSqlUrl url;
    if (!qiq_sql_parse_url(dsn, &url)) {
        qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "Database::open: expected a PostgreSQL URL such as \"postgres://user:password@host:5432/database\"");
    }
    QiqPostgres *pg = calloc(1, sizeof *pg);
    if (pg == NULL) qiq_fatal("out of memory");
    pg->socket = -1;
    pg->fork_count = qiq_sql_forks;
    pg->host = pg_copy(url.host != NULL ? url.host : "localhost");
    pg->user = pg_copy(url.user != NULL ? url.user : "postgres");
    if (url.password != NULL) pg->password = pg_copy(url.password);
    else if (getenv("PGPASSWORD") != NULL) pg->password = pg_copy(getenv("PGPASSWORD"));
    pg->database = pg_copy(url.database != NULL ? url.database : pg->user);
    pg->port = url.port != 0 ? url.port : 5432;
    bool known = qiq_sql_connection_options(&url, pg->host, &pg->tls, &pg->connect_timeout);
    qiq_sql_free_url(&url);
    if (!known) {
        pg_free(pg);
        qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION,
                  "Database::open: a PostgreSQL URL's options are sslmode=disable, sslmode=require (or verify-full), and connect_timeout=seconds");
    }
    {
        // A failure to connect frees everything before passing the exception on.
        jmp_buf environment;
        jmp_buf *previous = qiq_guard_enter(&environment);
        if (setjmp(environment)) {
            if (pg->socket >= 0) qiq_net_abandon(pg->socket);
            pg_free(pg);
            qiq_rethrow(previous);
        }
        pg_connect(pg);
        qiq_guard_leave(previous);
    }
    return pg;
}

void qiq_pg_close(QiqPostgres *pg) {
    if (pg->socket >= 0) {
        if (pg->fork_count == qiq_sql_forks) {
            // Terminate, ignoring a connection that's already gone.
            jmp_buf environment;
            jmp_buf *previous = qiq_guard_enter(&environment);
            if (!setjmp(environment)) {
                size_t at = pg_begin(pg, 'X');
                pg_end(pg, at);
                pg_flush(pg);
                qiq_net_close(pg->socket);
            } else {
                qiq_net_abandon(pg->socket);
            }
            qiq_guard_leave(previous);
        } else {
            qiq_net_abandon(pg->socket);
        }
    }
    pg_free(pg);
}

bool qiq_pg_in_transaction(QiqPostgres *pg) {
    return pg->socket >= 0 && pg->fork_count == qiq_sql_forks && pg->status != 'I';
}

// ---------------------------------------------------------------------------
// SQL text

// If sql[i] starts a string, quoted identifier, comment, or dollar-quoted
// string, the index just past it; otherwise i.
static size_t pg_skip(const char *sql, size_t i) {
    char c = sql[i];
    if (c == '\'') {
        // E'...' strings have backslash escapes.
        bool escapes = i > 0 && (sql[i - 1] == 'E' || sql[i - 1] == 'e');
        for (i++; sql[i] != '\0'; i++) {
            if (escapes && sql[i] == '\\' && sql[i + 1] != '\0') i++;
            else if (sql[i] == '\'') {
                if (sql[i + 1] == '\'') i++;
                else return i + 1;
            }
        }
        return i;
    }
    if (c == '"') {
        for (i++; sql[i] != '\0'; i++) {
            if (sql[i] == '"') {
                if (sql[i + 1] == '"') i++;
                else return i + 1;
            }
        }
        return i;
    }
    if (c == '-' && sql[i + 1] == '-') {
        while (sql[i] != '\0' && sql[i] != '\n') i++;
        return i;
    }
    if (c == '/' && sql[i + 1] == '*') {
        // Block comments nest.
        int depth = 0;
        while (sql[i] != '\0') {
            if (sql[i] == '/' && sql[i + 1] == '*') depth++, i += 2;
            else if (sql[i] == '*' && sql[i + 1] == '/') {
                i += 2;
                if (--depth == 0) return i;
            } else i++;
        }
        return i;
    }
    if (c == '$') {
        // $tag$ ... $tag$, unless the $ is part of an identifier.
        if (i > 0 && (sql[i - 1] == '_' || (sql[i - 1] >= 'a' && sql[i - 1] <= 'z') || (sql[i - 1] >= 'A' && sql[i - 1] <= 'Z') ||
                      (sql[i - 1] >= '0' && sql[i - 1] <= '9'))) {
            return i;
        }
        size_t tag_end = i + 1;
        if (sql[tag_end] >= '0' && sql[tag_end] <= '9') return i;
        while (sql[tag_end] == '_' || (sql[tag_end] >= 'a' && sql[tag_end] <= 'z') || (sql[tag_end] >= 'A' && sql[tag_end] <= 'Z') ||
               (sql[tag_end] >= '0' && sql[tag_end] <= '9')) {
            tag_end++;
        }
        if (sql[tag_end] != '$') return i;
        size_t tag_length = tag_end - i + 1;
        for (size_t j = tag_end + 1; sql[j] != '\0'; j++) {
            if (sql[j] == '$' && strncmp(sql + j, sql + i, tag_length) == 0) return j + tag_length;
        }
        return strlen(sql);
    }
    return i;
}

// `sql` with each `?` written as $1, $2, ... (and `??` as a literal ?);
// sets *count to how many there are.
static char *pg_placeholders(const char *sql, size_t *count, const char *caller) {
    size_t length = strlen(sql);
    char *out = malloc(length * 4 + 1);
    if (out == NULL) qiq_fatal("out of memory");
    size_t o = 0;
    *count = 0;
    for (size_t i = 0; i < length;) {
        size_t past = pg_skip(sql, i);
        if (past != i) {
            memcpy(out + o, sql + i, past - i);
            o += past - i;
            i = past;
        } else if (sql[i] == '?') {
            if (sql[i + 1] == '?') {
                out[o++] = '?';
                i += 2;
            } else {
                o += (size_t)sprintf(out + o, "$%zu", ++*count);
                i++;
            }
        } else if (sql[i] == '$' && sql[i + 1] >= '0' && sql[i + 1] <= '9' &&
                   !(i > 0 && (sql[i - 1] == '_' || (sql[i - 1] >= 'a' && sql[i - 1] <= 'z') || (sql[i - 1] >= 'A' && sql[i - 1] <= 'Z')))) {
            free(out);
            qiq_throwf(QIQ_DATABASE_EXCEPTION, "%s: write placeholders as ?, not $%c (the values are given in order after the SQL)", caller, sql[i + 1]);
        } else {
            out[o++] = sql[i++];
        }
    }
    out[o] = '\0';
    return out;
}

// The word starting at sql[*at] (after spaces), lowercased into `word`;
// advances *at past it.
static void pg_word(const char *sql, size_t length, size_t *at, char *word, size_t size) {
    while (*at < length && (sql[*at] == ' ' || sql[*at] == '\t' || sql[*at] == '\r' || sql[*at] == '\n')) (*at)++;
    size_t o = 0;
    while (*at < length && ((sql[*at] >= 'a' && sql[*at] <= 'z') || (sql[*at] >= 'A' && sql[*at] <= 'Z') || sql[*at] == '_')) {
        if (o + 1 < size) word[o++] = (char)(sql[*at] | 0x20);
        (*at)++;
    }
    word[o] = '\0';
}

// Whether a statement is a SET that lasts for the session: not SET LOCAL
// or SET TRANSACTION, which last only for a transaction.
static bool pg_is_session_setting(const char *sql, size_t length) {
    size_t at = 0;
    char word[16];
    pg_word(sql, length, &at, word, sizeof word);
    if (strcmp(word, "set") != 0) return false;
    pg_word(sql, length, &at, word, sizeof word);
    return strcmp(word, "local") != 0 && strcmp(word, "transaction") != 0;
}

// Remembers the SET statements among `sql`'s, to run again on a new
// connection; SET LOCAL and SET TRANSACTION last only for a transaction.
static void pg_remember_settings(QiqPostgres *pg, const char *sql) {
    size_t start = 0, length = strlen(sql);
    for (size_t i = 0; i <= length;) {
        size_t past = i < length ? pg_skip(sql, i) : i;
        if (past != i) {
            i = past;
            continue;
        }
        if (i == length || sql[i] == ';') {
            const char *statement = sql + start;
            size_t size = i - start;
            if (pg_is_session_setting(statement, size)) {
                size_t old = pg->settings != NULL ? strlen(pg->settings) : 0;
                char *grown = realloc(pg->settings, old + size + 3);
                if (grown == NULL) qiq_fatal("out of memory");
                memcpy(grown + old, statement, size);
                memcpy(grown + old + size, ";\n", 3);
                pg->settings = grown;
            }
            start = i + 1;
        }
        i++;
    }
}

// ---------------------------------------------------------------------------
// Results

static int64_t pg_parse_int(const unsigned char *text, size_t length) {
    char buffer[32];
    if (length >= sizeof buffer) length = sizeof buffer - 1;
    memcpy(buffer, text, length);
    buffer[length] = '\0';
    return strtoll(buffer, NULL, 10);
}

static void pg_add_value(int32_t type, const unsigned char *text, size_t length) {
    switch (type) {
    case PG_INT2:
    case PG_INT4:
    case PG_INT8:
    case PG_OID:
        qiq_sql_add_cell(QIQ_SQL_INT, pg_parse_int(text, length));
        return;
    case PG_BOOL:
        qiq_sql_add_cell(QIQ_SQL_INT, length > 0 && text[0] == 't');
        return;
    case PG_FLOAT4:
    case PG_FLOAT8: {
        char buffer[64];
        if (length >= sizeof buffer) length = sizeof buffer - 1;
        memcpy(buffer, text, length);
        buffer[length] = '\0';
        qiq_sql_add_double(strtod(buffer, NULL));
        return;
    }
    case PG_NUMERIC:
        qiq_sql_add_text((const char *)text, length, QIQ_SQL_DECIMAL);
        return;
    case PG_BYTEA:
        // Hex output: \x0102ff
        if (length >= 2 && text[0] == '\\' && text[1] == 'x') {
            size_t count = (length - 2) / 2;
            unsigned char *bytes = malloc(count > 0 ? count : 1);
            if (bytes == NULL) qiq_fatal("out of memory");
            for (size_t i = 0; i < count; i++) bytes[i] = (unsigned char)(pg_hex_digit((char)text[2 + i * 2]) * 16 + pg_hex_digit((char)text[3 + i * 2]));
            qiq_sql_add_bytes(bytes, count);
            free(bytes);
        } else {
            qiq_sql_add_bytes(text, length);
        }
        return;
    default:
        qiq_sql_add_text((const char *)text, length, QIQ_SQL_TEXT);
    }
}

static void pg_read_columns(QiqPostgres *pg) {
    PgReader reader = pg_reader(pg);
    int count = pg_int16(&reader);
    pg->type_count = 0;
    pg->types = pg_grow(pg->types, &pg->types_capacity, (size_t)(count > 0 ? count : 1), sizeof(int32_t));
    for (int column = 0; column < count; column++) {
        const char *name = pg_string(&reader);
        pg_int32(&reader); // table
        pg_int16(&reader); // column number
        pg->types[pg->type_count++] = pg_int32(&reader);
        pg_int16(&reader); // size
        pg_int32(&reader); // modifier
        pg_int16(&reader); // format
        qiq_sql_add_column(name, strlen(name));
    }
}

static void pg_read_row(QiqPostgres *pg) {
    PgReader reader = pg_reader(pg);
    int count = pg_int16(&reader);
    if ((size_t)count != pg->type_count) pg_malformed();
    for (int column = 0; column < count; column++) {
        int32_t length = pg_int32(&reader);
        if (length < 0) {
            qiq_sql_add_cell(QIQ_SQL_NULL, 0);
            continue;
        }
        if (reader.end - reader.at < length) pg_malformed();
        pg_add_value(pg->types[column], reader.at, (size_t)length);
        reader.at += length;
    }
}

// The number of rows a command changed, from its tag ("UPDATE 3").
static int64_t pg_changes(const char *tag) {
    if (strncmp(tag, "INSERT ", 7) != 0 && strncmp(tag, "UPDATE ", 7) != 0 && strncmp(tag, "DELETE ", 7) != 0 &&
        strncmp(tag, "MERGE ", 6) != 0 && strncmp(tag, "COPY ", 5) != 0) {
        return 0;
    }
    const char *last = strrchr(tag, ' ');
    return last != NULL ? strtoll(last + 1, NULL, 10) : 0;
}

// ---------------------------------------------------------------------------
// Running statements

// Connects for the first statement after a fork or a lost connection.
static void pg_ready(QiqPostgres *pg) {
    if (pg->fork_count != qiq_sql_forks) {
        // The parent's connection: left alone, and opened again here.
        if (pg->socket >= 0) qiq_net_abandon(pg->socket);
        pg->socket = -1;
        pg->fork_count = qiq_sql_forks;
        pg->status = 'I';
        pg->streaming = false;
    }
    if (pg->lost_in_transaction) {
        pg->lost_in_transaction = false;
        qiq_throw(QIQ_DATABASE_EXCEPTION,
                  "the connection to PostgreSQL was lost during a transaction, which the server rolled back; start it again");
    }
    if (pg->socket < 0) {
        pg->in_io = true;
        pg_connect(pg);
        pg->in_io = false;
    }
}

// Gives up a connection that failed; the next statement connects again.
static void pg_lose(QiqPostgres *pg) {
    pg->streaming = false;
    if (pg->socket < 0) return;
    if (pg->status != 'I') pg->lost_in_transaction = true;
    qiq_net_abandon(pg->socket);
    pg->socket = -1;
    pg->status = 'I';
}

// What a failure does: cleans up, and when it happened while talking to the
// server, gives up the connection.
static void pg_failed(QiqPostgres *pg) {
    qiq_sql_drop_pending();
    qiq_sql_clear_results();
    pg->out_length = 0;
    if (pg->in_io) pg_lose(pg);
    pg->in_io = false;
}

static void pg_simple_query(QiqPostgres *pg, const char *sql) {
    size_t at = pg_begin(pg, 'Q');
    pg_put_string(pg, sql);
    pg_end(pg, at);
    pg_flush(pg);
    char message[1024] = "", state[6] = "";
    bool failed = false;
    for (;;) {
        char type = pg_read(pg);
        if (type == 'Z') {
            pg->status = pg->in_length > 0 ? (char)pg->in[0] : 'I';
            break;
        }
        if (type == 'E' && !failed) {
            failed = true;
            if (pg_error_text(pg, message, sizeof message, state)) {
                pg_lose(pg);
                break;
            }
        }
    }
    if (failed) qiq_sql_throw(state, 0, strcmp(state, "23505") == 0, message);
}

void qiq_pg_script(QiqPostgres *pg, const char *sql) {
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        pg_failed(pg);
        qiq_rethrow(previous);
    }
    pg_ready(pg);
    pg->in_io = true;
    pg_simple_query(pg, sql);
    pg->in_io = false;
    pg_remember_settings(pg, sql);
    qiq_guard_leave(previous);
}

static void pg_format_double(double value, char *out, size_t size) {
    if (isnan(value)) snprintf(out, size, "NaN");
    else if (isinf(value)) snprintf(out, size, value > 0 ? "Infinity" : "-Infinity");
    else snprintf(out, size, "%.17g", value);
}

// Sends `sql` with the pending values: Parse, Bind, Describe, Execute (all
// the rows, or one for mode 2), and Sync, in one write.
static void pg_send_statement(QiqPostgres *pg, const char *sql, int64_t mode, const char *caller) {
    size_t expected;
    char *text = pg_placeholders(sql, &expected, caller);
    size_t given = qiq_sql_pending_count;
    if (expected != given) {
        free(text);
        qiq_throwf(QIQ_DATABASE_EXCEPTION, "%s: the SQL has %zu placeholder%s (?), but %zu value%s given", caller, expected,
                   expected == 1 ? "" : "s", given, given == 1 ? " was" : "s were");
    }
    // Parse, with the values' types: bytea for bytes, sent as they are, and
    // otherwise unknown, so the server reads the text as whatever type the
    // column, operator, or function needs (as PDO and libpq-based drivers
    // do). Typing ints as bigint would rule out functions that take an
    // integer, such as pg_terminate_backend.
    size_t at = pg_begin(pg, 'P');
    pg_put_string(pg, "");
    pg_put_string(pg, text);
    pg_put_int16(pg, (int)given);
    for (size_t i = 0; i < given; i++) {
        char type = qiq_sql_pending[i].type;
        pg_put_int32(pg, type == 'a' ? PG_BYTEA : 0);
    }
    pg_end(pg, at);
    free(text);
    // Bind: text values, except bytes, which go as they are.
    at = pg_begin(pg, 'B');
    pg_put_string(pg, "");
    pg_put_string(pg, "");
    pg_put_int16(pg, (int)given);
    for (size_t i = 0; i < given; i++) pg_put_int16(pg, qiq_sql_pending[i].type == 'a' ? 1 : 0);
    pg_put_int16(pg, (int)given);
    for (size_t i = 0; i < given; i++) {
        QiqSqlValue value = qiq_sql_pending[i];
        char number[40];
        switch (value.type) {
        case 's':
            if (value.payload == 0) {
                pg_put_int32(pg, -1);
            } else {
                const char *string = (const char *)(uintptr_t)value.payload;
                size_t length = strlen(string);
                pg_put_int32(pg, (int64_t)length);
                pg_put(pg, string, length);
            }
            break;
        case 'a': {
            void *array = (void *)(uintptr_t)value.payload;
            if (array == NULL) {
                pg_put_int32(pg, -1);
            } else {
                pg_put_int32(pg, QIQ_ARRAY_LENGTH(array));
                pg_put(pg, QIQ_ARRAY_DATA(array), (size_t)QIQ_ARRAY_LENGTH(array));
            }
            break;
        }
        case 'n':
            pg_put_int32(pg, -1);
            break;
        case 'd': {
            double d;
            memcpy(&d, &value.payload, sizeof d);
            pg_format_double(d, number, sizeof number);
            pg_put_int32(pg, (int64_t)strlen(number));
            pg_put(pg, number, strlen(number));
            break;
        }
        case 'b':
            pg_put_int32(pg, 1);
            pg_put(pg, value.payload ? "t" : "f", 1);
            break;
        default:
            snprintf(number, sizeof number, "%lld", (long long)(int64_t)value.payload);
            pg_put_int32(pg, (int64_t)strlen(number));
            pg_put(pg, number, strlen(number));
        }
    }
    pg_put_int16(pg, 0); // results as text
    pg_end(pg, at);
    at = pg_begin(pg, 'D');
    pg_put_byte(pg, 'P');
    pg_put_string(pg, "");
    pg_end(pg, at);
    at = pg_begin(pg, 'E');
    pg_put_string(pg, "");
    pg_put_int32(pg, mode == 2 ? 1 : 0);
    pg_end(pg, at);
    at = pg_begin(pg, 'S');
    pg_end(pg, at);
    qiq_sql_drop_pending();
    pg->in_io = true;
    pg_flush(pg);
}

int64_t qiq_pg_run(QiqPostgres *pg, const char *sql, int64_t mode, const char *caller) {
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        pg_failed(pg);
        qiq_rethrow(previous);
    }
    pg_ready(pg);
    pg_send_statement(pg, sql, mode, caller);
    int64_t rows = 0, changes = 0;
    char message[1024] = "", state[6] = "";
    bool failed = false, empty = false;
    pg->type_count = 0;
    for (;;) {
        char type = pg_read(pg);
        if (type == 'Z') {
            pg->status = pg->in_length > 0 ? (char)pg->in[0] : 'I';
            break;
        }
        if (failed) continue; // the server skips to the Sync
        switch (type) {
        case 'T':
            if (mode != 0) pg_read_columns(pg);
            break;
        case 'D':
            if (mode != 0) {
                pg_read_row(pg);
                rows++;
            }
            break;
        case 'C':
            changes = pg_changes((const char *)pg->in);
            break;
        case 'I':
            empty = true;
            break;
        case 'E':
            failed = true;
            if (pg_error_text(pg, message, sizeof message, state)) {
                // The server is closing the connection, and says why.
                pg_lose(pg);
                pg->in_io = false;
                qiq_sql_clear_results();
                qiq_sql_throw(state, 0, false, message);
            }
            break;
        default: // ParseComplete, BindComplete, NoData, PortalSuspended, notices
            break;
        }
    }
    pg->in_io = false;
    if (failed || empty) {
        qiq_sql_clear_results();
        if (empty) qiq_throwf(QIQ_DATABASE_EXCEPTION, "%s: the SQL has no statement to run", caller);
        if (strcmp(state, "42601") == 0 && strstr(message, "multiple commands") != NULL) {
            qiq_throwf(QIQ_DATABASE_EXCEPTION, "%s runs one SQL statement, but this has more; run several with executeScript", caller);
        }
        qiq_sql_throw(state, 0, strcmp(state, "23505") == 0, message);
    }
    pg_remember_settings(pg, sql);
    qiq_guard_leave(previous);
    return mode == 0 ? changes : rows;
}

// ---------------------------------------------------------------------------
// Cursors: the rows are read off the connection as they're asked for.

static void qiq_pg_open_cursor(QiqPostgres *pg, const char *sql, const char *caller) {
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        pg_failed(pg);
        qiq_rethrow(previous);
    }
    pg_ready(pg);
    pg_send_statement(pg, sql, 1, caller);
    pg->type_count = 0;
    char message[1024] = "", state[6] = "";
    bool failed = false;
    // Up to the columns; a statement without rows (or an error) runs to the end.
    for (;;) {
        char type = pg_read(pg);
        if (type == 'T' && !failed) {
            pg_read_columns(pg);
            pg->streaming = true;
            break;
        }
        if (type == 'Z') {
            pg->status = pg->in_length > 0 ? (char)pg->in[0] : 'I';
            break;
        }
        if (type == 'E' && !failed) {
            failed = true;
            if (pg_error_text(pg, message, sizeof message, state)) {
                pg_lose(pg);
                break;
            }
        }
    }
    pg->in_io = false;
    if (failed) {
        qiq_sql_clear_results();
        qiq_sql_throw(state, 0, strcmp(state, "23505") == 0, message);
    }
    qiq_guard_leave(previous);
}

static int64_t qiq_pg_fetch(QiqPostgres *pg, int64_t count) {
    if (!pg->streaming || pg->socket < 0 || pg->fork_count != qiq_sql_forks) {
        pg->streaming = false;
        return 0;
    }
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        pg_failed(pg);
        qiq_rethrow(previous);
    }
    pg->in_io = true;
    int64_t rows = 0;
    char message[1024] = "", state[6] = "";
    bool failed = false;
    while (failed || rows < count) {
        char type = pg_read(pg);
        if (type == 'Z') {
            pg->status = pg->in_length > 0 ? (char)pg->in[0] : 'I';
            pg->streaming = false;
            break;
        }
        if (failed) continue;
        if (type == 'D') {
            pg_read_row(pg);
            rows++;
        } else if (type == 'E') {
            failed = true;
            if (pg_error_text(pg, message, sizeof message, state)) {
                pg_lose(pg);
                break;
            }
        }
    }
    pg->in_io = false;
    if (failed) {
        pg->streaming = false;
        qiq_sql_clear_results();
        qiq_sql_throw(state, 0, strcmp(state, "23505") == 0, message);
    }
    qiq_guard_leave(previous);
    return rows;
}

static void qiq_pg_close_cursor(QiqPostgres *pg) {
    if (!pg->streaming || pg->socket < 0 || pg->fork_count != qiq_sql_forks) {
        pg->streaming = false;
        return;
    }
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        pg_failed(pg);
        qiq_rethrow(previous);
    }
    pg->in_io = true;
    // The rows left, and the end, are read and dropped.
    while (pg_read(pg) != 'Z') {}
    pg->status = pg->in_length > 0 ? (char)pg->in[0] : 'I';
    pg->streaming = false;
    pg->in_io = false;
    qiq_guard_leave(previous);
}

// ---------------------------------------------------------------------------

// PostgreSQL has no rowid: the value the session's last sequence (such as a
// serial or identity column's) gave, as PDO does.
static int64_t qiq_pg_last_insert_id(QiqPostgres *pg) {
    qiq_sql_drop_pending();
    qiq_sql_clear_results();
    qiq_pg_run(pg, "select lastval()", 2, "lastInsertId");
    int64_t id = qiq_sql_first_int();
    qiq_sql_clear_results();
    return id;
}

const QiqSqlDriver qiq_postgres_driver = {
    .open = (void *(*)(const char *))qiq_pg_open,
    .run = (int64_t (*)(void *, const char *, int64_t, const char *))qiq_pg_run,
    .script = (void (*)(void *, const char *))qiq_pg_script,
    .last_insert_id = (int64_t (*)(void *))qiq_pg_last_insert_id,
    .in_transaction = (bool (*)(void *))qiq_pg_in_transaction,
    .close = (void (*)(void *))qiq_pg_close,
    .open_cursor = (void (*)(void *, const char *, const char *))qiq_pg_open_cursor,
    .fetch = (int64_t (*)(void *, int64_t))qiq_pg_fetch,
    .close_cursor = (void (*)(void *))qiq_pg_close_cursor,
};
