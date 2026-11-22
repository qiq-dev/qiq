// The MySQL and MariaDB driver for std/sql (see sql.c): the client/server
// protocol over net.c's connections, with TLS from Mbed TLS and the
// caching_sha2_password (MySQL 8's default, including its RSA exchange on
// unencrypted connections) and mysql_native_password logins. Nothing else
// to install.
//
// Statements are prepared on the server (COM_STMT_PREPARE) and run with
// their values sent separately in binary (COM_STMT_EXECUTE), never pasted
// into the SQL; MySQL's placeholders are `?` already. Each connection keeps
// its most recently used statements prepared, keyed by their SQL text.
// Statements the server can't prepare run as plain queries when they have
// no values. Results are converted by column type: integers and floats to
// the cells Row's getters read, DECIMAL to a decimal kept as its exact text,
// binary strings and BLOBs to bytes, dates and times to text as MySQL
// writes them, and everything else to text.
//
// As in postgres.c, a connection used in a process forked after it was
// opened connects again, as does one whose connection was lost, and the SET
// statements it ran are run again. Errors from the server throw
// DatabaseException; network failures throw NetworkException.
#include <float.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/random.h>

#include "mbedtls/md.h"
#include "mbedtls/pk.h"
#include "mbedtls/rsa.h"
#include "qiq_runtime.h"
#include "sql_driver.h"

// Capability flags.
enum {
    MY_LONG_PASSWORD = 1, MY_FOUND_ROWS = 2, MY_LONG_FLAG = 4, MY_CONNECT_WITH_DB = 8, MY_PROTOCOL_41 = 0x200,
    MY_SSL = 0x800, MY_TRANSACTIONS = 0x2000, MY_SECURE_CONNECTION = 0x8000, MY_MULTI_STATEMENTS = 0x10000,
    MY_MULTI_RESULTS = 0x20000, MY_PS_MULTI_RESULTS = 0x40000, MY_PLUGIN_AUTH = 0x80000,
    MY_PLUGIN_AUTH_LENENC_DATA = 0x200000,
};

// Server status flags.
enum { MY_STATUS_IN_TRANSACTION = 1, MY_STATUS_MORE_RESULTS = 8 };

// Column types.
enum {
    MY_DECIMAL = 0, MY_TINY = 1, MY_SHORT = 2, MY_LONG = 3, MY_FLOAT = 4, MY_DOUBLE = 5, MY_NULL = 6, MY_TIMESTAMP = 7,
    MY_LONGLONG = 8, MY_INT24 = 9, MY_DATE = 10, MY_TIME = 11, MY_DATETIME = 12, MY_YEAR = 13, MY_VARCHAR = 15,
    MY_BIT = 16, MY_JSON = 245, MY_NEWDECIMAL = 246, MY_BLOB = 252, MY_VAR_STRING = 253, MY_GEOMETRY = 255,
};

#define MY_UNSIGNED_FLAG 32
#define MY_BINARY_CHARSET 63
// utf8mb4_general_ci, which MySQL 5.5+ and MariaDB both know.
#define MY_UTF8MB4 45
#define MY_MAX_PACKET 0xffffff
#define MY_CACHE 16
// ER_UNSUPPORTED_PS: "This command is not supported in the prepared statement protocol yet".
#define MY_UNSUPPORTED_PS 1295

typedef struct {
    char *sql;                // NULL when the entry is unused
    uint32_t id;
    int params;
    uint64_t used;
} MyCached;

typedef struct {
    uint8_t type;
    uint16_t flags;
    uint16_t charset;
    uint8_t decimals;
} MyColumn;

typedef struct {
    int64_t socket;           // -1 when not connected
    char *host, *user, *password, *database;
    int64_t port, connect_timeout;
    char *ca_file;            // sslrootcert, or NULL for the system's CAs
    char *socket_file;        // a Unix socket to connect to instead of host:port
    bool tls;
    uint32_t capabilities;    // the ones both sides have
    uint8_t sequence;         // of the next packet to send
    uint16_t status;          // from the last OK or EOF packet
    int64_t last_insert_id;
    uint64_t fork_count;
    bool lost_in_transaction;
    bool in_io;               // talking to the server: a failure now loses the connection
    bool streaming;           // a cursor's rows are still arriving
    bool streaming_binary;    // as binary rows (a prepared statement's) rather than text
    uint32_t connection_id;   // the server's id for this connection, for KILL QUERY
    char *settings;           // the SET statements it ran, to run again on a new connection
    MyCached cache[MY_CACHE];
    uint64_t clock;
    unsigned char *out;
    size_t out_length, out_capacity;
    unsigned char *in;
    size_t in_length, in_capacity;
    MyColumn *columns;
    size_t column_count, columns_capacity;
} QiqMysql;

static void *my_grow(void *data, size_t *capacity, size_t needed, size_t size) {
    if (needed <= *capacity) return data;
    size_t grown = *capacity < 256 ? 256 : *capacity * 2;
    if (grown < needed) grown = needed;
    data = realloc(data, grown * size);
    if (data == NULL) qiq_fatal("out of memory");
    *capacity = grown;
    return data;
}

static char *my_copy(const char *text) {
    char *copy = strdup(text);
    if (copy == NULL) qiq_fatal("out of memory");
    return copy;
}

// ---------------------------------------------------------------------------
// Packets: a 3-byte little-endian length, a sequence number, and up to
// 16 MB - 1 of contents; longer contents continue in the next packets.

static void my_put(QiqMysql *my, const void *data, size_t length) {
    my->out = my_grow(my->out, &my->out_capacity, my->out_length + length, 1);
    memcpy(my->out + my->out_length, data, length);
    my->out_length += length;
}

static void my_put_byte(QiqMysql *my, unsigned char byte) {
    my_put(my, &byte, 1);
}

static void my_put_int(QiqMysql *my, uint64_t value, int bytes) {
    unsigned char out[8];
    for (int i = 0; i < bytes; i++) out[i] = (unsigned char)(value >> (8 * i));
    my_put(my, out, (size_t)bytes);
}

static void my_put_lenenc(QiqMysql *my, uint64_t value) {
    if (value < 251) my_put_byte(my, (unsigned char)value);
    else if (value < 0x10000) my_put_byte(my, 0xfc), my_put_int(my, value, 2);
    else if (value < 0x1000000) my_put_byte(my, 0xfd), my_put_int(my, value, 3);
    else my_put_byte(my, 0xfe), my_put_int(my, value, 8);
}

// Sends what was put as one packet (or several, for 16 MB or more).
static void my_send(QiqMysql *my) {
    size_t sent = 0;
    for (;;) {
        size_t length = my->out_length - sent;
        if (length > MY_MAX_PACKET) length = MY_MAX_PACKET;
        unsigned char header[4] = {(unsigned char)length, (unsigned char)(length >> 8), (unsigned char)(length >> 16), my->sequence++};
        qiq_net_send(my->socket, header, 4);
        if (length > 0) qiq_net_send(my->socket, my->out + sent, length);
        sent += length;
        if (length < MY_MAX_PACKET) break;
    }
    my->out_length = 0;
}

// Starts a command: a new sequence of packets.
static void my_command(QiqMysql *my, unsigned char command) {
    my->sequence = 0;
    my->out_length = 0;
    my_put_byte(my, command);
}

static void my_read(QiqMysql *my) {
    my->in_length = 0;
    for (;;) {
        unsigned char header[4];
        qiq_net_receive(my->socket, header, 4);
        size_t length = (size_t)header[0] | (size_t)header[1] << 8 | (size_t)header[2] << 16;
        my->sequence = (uint8_t)(header[3] + 1);
        my->in = my_grow(my->in, &my->in_capacity, my->in_length + length + 1, 1);
        qiq_net_receive(my->socket, my->in + my->in_length, length);
        my->in_length += length;
        if (length < MY_MAX_PACKET) break;
    }
    my->in[my->in_length] = '\0';
}

typedef struct {
    const unsigned char *at, *end;
} MyReader;

static MyReader my_reader(QiqMysql *my) {
    return (MyReader){my->in, my->in + my->in_length};
}

static _Noreturn void my_malformed(void) {
    qiq_throw(QIQ_DATABASE_EXCEPTION, "the MySQL server sent a malformed packet");
}

static uint64_t my_int(MyReader *reader, int bytes) {
    if (reader->end - reader->at < bytes) my_malformed();
    uint64_t value = 0;
    for (int i = 0; i < bytes; i++) value |= (uint64_t)reader->at[i] << (8 * i);
    reader->at += bytes;
    return value;
}

// A length-encoded integer; *null is set for 0xfb, which stands for NULL
// in text rows.
static uint64_t my_lenenc(MyReader *reader, bool *null) {
    if (null != NULL) *null = false;
    uint64_t first = my_int(reader, 1);
    if (first < 0xfb) return first;
    if (first == 0xfb) {
        if (null != NULL) *null = true;
        return 0;
    }
    if (first == 0xfc) return my_int(reader, 2);
    if (first == 0xfd) return my_int(reader, 3);
    if (first == 0xfe) return my_int(reader, 8);
    my_malformed();
}

static const unsigned char *my_bytes(MyReader *reader, uint64_t length) {
    if ((uint64_t)(reader->end - reader->at) < length) my_malformed();
    const unsigned char *start = reader->at;
    reader->at += length;
    return start;
}

static const char *my_string(MyReader *reader) {
    const unsigned char *start = reader->at;
    while (reader->at < reader->end && *reader->at != 0) reader->at++;
    if (reader->at == reader->end) my_malformed();
    reader->at++;
    return (const char *)start;
}

// The error of the ERR packet just read (0xff, number, '#', SQLSTATE,
// message), for my_throw.
typedef struct {
    int code;
    char state[6];
    char message[1024];
} MyError;

static int my_error_into(QiqMysql *my, MyError *error) {
    MyReader reader = my_reader(my);
    my_int(&reader, 1);
    error->code = (int)my_int(&reader, 2);
    snprintf(error->state, sizeof error->state, "HY000");
    if (reader.at < reader.end && *reader.at == '#') {
        my_bytes(&reader, 1);
        snprintf(error->state, sizeof error->state, "%.5s", (const char *)my_bytes(&reader, 5));
    }
    snprintf(error->message, sizeof error->message, "%.*s", (int)(reader.end - reader.at), (const char *)reader.at);
    return error->code;
}

// The message of an ERR packet, for errors while logging in.
static int my_error(QiqMysql *my, char *out, size_t size) {
    MyError error;
    my_error_into(my, &error);
    snprintf(out, size, "%s", error.message);
    return error.code;
}

// ER_DUP_ENTRY, ER_DUP_ENTRY_WITH_KEY_NAME, and ER_DUP_KEY: the errors for a
// duplicate in a unique key, which MySQL reports with SQLSTATE 23000 like
// every other constraint.
// ER_CHECK_CONSTRAINT_VIOLATED comes with SQLSTATE HY000; it's a constraint
// like the others, so it gets 23000.
static _Noreturn void my_throw(const MyError *error) {
    bool unique = error->code == 1062 || error->code == 1586 || error->code == 1022;
    const char *state = error->code == 3819 ? "23000" : error->state;
    qiq_sql_throw(state, error->code, unique, error->message);
}

static bool my_is_eof(QiqMysql *my) {
    return my->in_length < 9 && my->in_length > 0 && my->in[0] == 0xfe;
}

// An OK packet: rows changed, the last id generated, and the status.
static int64_t my_ok(QiqMysql *my) {
    MyReader reader = my_reader(my);
    my_int(&reader, 1);
    int64_t changed = (int64_t)my_lenenc(&reader, NULL);
    int64_t id = (int64_t)my_lenenc(&reader, NULL);
    if (id != 0) my->last_insert_id = id; // as LAST_INSERT_ID() keeps the last one
    my->status = (uint16_t)my_int(&reader, 2);
    return changed;
}

static void my_eof(QiqMysql *my) {
    MyReader reader = my_reader(my);
    my_int(&reader, 1);
    if (reader.end - reader.at >= 4) {
        my_int(&reader, 2); // warnings
        my->status = (uint16_t)my_int(&reader, 2);
    }
}

// ---------------------------------------------------------------------------
// Logging in

static _Noreturn void my_auth_failed(QiqMysql *my, const char *reason) {
    qiq_throwf(QIQ_DATABASE_EXCEPTION, "logging in to MySQL at %s as %s: %s", my->host, my->user, reason);
}

static void my_digest(mbedtls_md_type_t type, const void *a, size_t a_length, const void *b, size_t b_length, unsigned char *out) {
    mbedtls_md_context_t context;
    mbedtls_md_init(&context);
    mbedtls_md_setup(&context, mbedtls_md_info_from_type(type), 0);
    mbedtls_md_starts(&context);
    mbedtls_md_update(&context, a, a_length);
    if (b != NULL) mbedtls_md_update(&context, b, b_length);
    mbedtls_md_finish(&context, out);
    mbedtls_md_free(&context);
}

// What to send for `plugin` with the server's 20-byte nonce; returns its
// length, or -1 for a plugin qiq doesn't support.
static int my_scramble(QiqMysql *my, const char *plugin, const unsigned char *nonce, unsigned char *out) {
    const char *password = my->password != NULL ? my->password : "";
    size_t length = strlen(password);
    if (strcmp(plugin, "mysql_native_password") == 0) {
        // SHA1(password) XOR SHA1(nonce + SHA1(SHA1(password)))
        if (length == 0) return 0;
        unsigned char hashed[20], twice[20], mixed[20];
        my_digest(MBEDTLS_MD_SHA1, password, length, NULL, 0, hashed);
        my_digest(MBEDTLS_MD_SHA1, hashed, 20, NULL, 0, twice);
        my_digest(MBEDTLS_MD_SHA1, nonce, 20, twice, 20, mixed);
        for (int i = 0; i < 20; i++) out[i] = hashed[i] ^ mixed[i];
        return 20;
    }
    if (strcmp(plugin, "caching_sha2_password") == 0) {
        // SHA256(password) XOR SHA256(SHA256(SHA256(password)) + nonce)
        if (length == 0) return 0;
        unsigned char hashed[32], twice[32], mixed[32];
        my_digest(MBEDTLS_MD_SHA256, password, length, NULL, 0, hashed);
        my_digest(MBEDTLS_MD_SHA256, hashed, 32, NULL, 0, twice);
        my_digest(MBEDTLS_MD_SHA256, twice, 32, nonce, 20, mixed);
        for (int i = 0; i < 32; i++) out[i] = hashed[i] ^ mixed[i];
        return 32;
    }
    if (strcmp(plugin, "mysql_clear_password") == 0) {
        if (!my->tls && my->socket_file == NULL && !qiq_sql_is_local_host(my->host)) {
            my_auth_failed(my, "the server asks for the password unencrypted; connect with TLS (sslmode=require)");
        }
        if (length > 250) my_auth_failed(my, "the password is too long");
        memcpy(out, password, length + 1);
        return (int)length + 1;
    }
    return -1;
}

static int my_random(void *context, unsigned char *out, size_t length) {
    (void)context;
    return getrandom(out, length, 0) == (ssize_t)length ? 0 : -1;
}

// caching_sha2_password's full login on an unencrypted connection: the
// password, XORed with the nonce, encrypted with the server's RSA key.
static void my_send_rsa_password(QiqMysql *my, const unsigned char *nonce) {
    my_put_byte(my, 2); // request the public key
    my_send(my);
    my_read(my);
    if (my->in_length < 2 || my->in[0] != 1) my_auth_failed(my, "the server sent no public key");
    mbedtls_pk_context key;
    mbedtls_pk_init(&key);
    // The PEM text after the 0x01 byte, which my_read ended with a NUL.
    if (mbedtls_pk_parse_public_key(&key, my->in + 1, my->in_length) != 0 || mbedtls_pk_get_type(&key) != MBEDTLS_PK_RSA) {
        mbedtls_pk_free(&key);
        my_auth_failed(my, "the server's public key isn't an RSA key");
    }
    mbedtls_rsa_context *rsa = mbedtls_pk_rsa(key);
    mbedtls_rsa_set_padding(rsa, MBEDTLS_RSA_PKCS_V21, MBEDTLS_MD_SHA1);
    const char *password = my->password != NULL ? my->password : "";
    size_t length = strlen(password) + 1;
    size_t size = mbedtls_rsa_get_len(rsa);
    unsigned char *plain = malloc(length), *encrypted = malloc(size);
    if (plain == NULL || encrypted == NULL) qiq_fatal("out of memory");
    for (size_t i = 0; i < length; i++) plain[i] = (unsigned char)password[i] ^ nonce[i % 20];
    int code = mbedtls_rsa_pkcs1_encrypt(rsa, my_random, NULL, length, plain, encrypted);
    free(plain);
    mbedtls_pk_free(&key);
    if (code != 0) {
        free(encrypted);
        my_auth_failed(my, "encrypting the password for the server failed (is it too long for the key?)");
    }
    my_put(my, encrypted, size);
    free(encrypted);
    my_send(my);
}

static void my_simple_query(QiqMysql *my, const char *sql);

static void my_connect(QiqMysql *my) {
    my->socket = my->socket_file != NULL ? qiq_net_connect_unix(my->socket_file, my->connect_timeout)
                                         : qiq_net_connect(my->host, my->port, my->connect_timeout);
    my->status = 0;
    memset(my->cache, 0, sizeof my->cache);
    my_read(my);
    MyReader reader = my_reader(my);
    if (my->in_length > 0 && my->in[0] == 0xff) {
        char message[512];
        my_error(my, message, sizeof message);
        my_auth_failed(my, message);
    }
    if (my_int(&reader, 1) != 10) my_auth_failed(my, "the server speaks a protocol version qiq doesn't");
    my_string(&reader); // server version
    my->connection_id = (uint32_t)my_int(&reader, 4);
    unsigned char nonce[21] = {0};
    memcpy(nonce, my_bytes(&reader, 8), 8);
    my_int(&reader, 1);
    uint32_t server = (uint32_t)my_int(&reader, 2);
    my_int(&reader, 1); // character set
    my_int(&reader, 2); // status
    server |= (uint32_t)my_int(&reader, 2) << 16;
    size_t data_length = (size_t)my_int(&reader, 1);
    my_bytes(&reader, 10);
    char plugin[64] = "mysql_native_password";
    if (server & MY_SECURE_CONNECTION) {
        size_t rest = data_length > 8 ? data_length - 8 : 13;
        if (rest < 13) rest = 13;
        const unsigned char *more = my_bytes(&reader, rest);
        memcpy(nonce + 8, more, 12);
    }
    if (server & MY_PLUGIN_AUTH) snprintf(plugin, sizeof plugin, "%s", my_string(&reader));
    uint32_t wanted = MY_LONG_PASSWORD | MY_FOUND_ROWS | MY_LONG_FLAG | MY_PROTOCOL_41 | MY_TRANSACTIONS | MY_SECURE_CONNECTION |
                      MY_MULTI_STATEMENTS | MY_MULTI_RESULTS | MY_PS_MULTI_RESULTS | MY_PLUGIN_AUTH | MY_PLUGIN_AUTH_LENENC_DATA;
    if (my->database != NULL) wanted |= MY_CONNECT_WITH_DB;
    if ((server & (MY_PROTOCOL_41 | MY_SECURE_CONNECTION | MY_PLUGIN_AUTH)) != (MY_PROTOCOL_41 | MY_SECURE_CONNECTION | MY_PLUGIN_AUTH)) {
        my_auth_failed(my, "the server is too old (qiq needs MySQL 5.5 or MariaDB 5.5 or later)");
    }
    if (my->tls) {
        if (!(server & MY_SSL)) {
            qiq_throwf(QIQ_DATABASE_EXCEPTION,
                       "the MySQL server at %s doesn't accept TLS connections; if the network to it is trusted (the same machine or a "
                       "private network), add sslmode=disable to the URL",
                       my->host);
        }
        wanted |= MY_SSL;
    }
    my->capabilities = wanted & server;
    if (my->tls) {
        // SSLRequest: the start of the login packet, then TLS.
        my_put_int(my, my->capabilities, 4);
        my_put_int(my, MY_MAX_PACKET, 4);
        my_put_byte(my, MY_UTF8MB4);
        for (int i = 0; i < 23; i++) my_put_byte(my, 0);
        my_send(my);
        qiq_net_start_tls(my->socket, my->host, my->ca_file);
    }
    unsigned char response[256];
    int response_length = my_scramble(my, plugin, nonce, response);
    if (response_length < 0) response_length = 0; // the server will ask to switch
    my_put_int(my, my->capabilities, 4);
    my_put_int(my, MY_MAX_PACKET, 4);
    my_put_byte(my, MY_UTF8MB4);
    for (int i = 0; i < 23; i++) my_put_byte(my, 0);
    my_put(my, my->user, strlen(my->user) + 1);
    my_put_lenenc(my, (uint64_t)response_length);
    my_put(my, response, (size_t)response_length);
    if (my->capabilities & MY_CONNECT_WITH_DB) my_put(my, my->database, strlen(my->database) + 1);
    my_put(my, plugin, strlen(plugin) + 1);
    my_send(my);
    for (;;) {
        my_read(my);
        if (my->in_length == 0) my_malformed();
        unsigned char kind = my->in[0];
        if (kind == 0x00) {
            my_ok(my);
            break;
        }
        if (kind == 0xff) {
            char message[512];
            my_error(my, message, sizeof message);
            my_auth_failed(my, message);
        }
        if (kind == 0xfe) {
            // AuthSwitchRequest: another plugin, with a new nonce.
            MyReader switch_reader = my_reader(my);
            my_int(&switch_reader, 1);
            if (switch_reader.at == switch_reader.end) my_auth_failed(my, "the server asks for MySQL's old password scheme, which qiq doesn't support");
            snprintf(plugin, sizeof plugin, "%s", my_string(&switch_reader));
            size_t left = (size_t)(switch_reader.end - switch_reader.at);
            memset(nonce, 0, sizeof nonce);
            memcpy(nonce, switch_reader.at, left < 20 ? left : 20);
            response_length = my_scramble(my, plugin, nonce, response);
            if (response_length < 0) {
                char reason[160];
                snprintf(reason, sizeof reason, "the server asks for the %s login, which qiq doesn't support", plugin);
                my_auth_failed(my, reason);
            }
            my_put(my, response, (size_t)response_length);
            my_send(my);
            continue;
        }
        if (kind == 0x01 && strcmp(plugin, "caching_sha2_password") == 0 && my->in_length >= 2) {
            if (my->in[1] == 3) continue; // fast login: an OK follows
            if (my->in[1] == 4) {
                // Full login: the password itself, safe only over TLS or a
                // Unix socket, which the server trusts as well.
                if (my->tls || my->socket_file != NULL) {
                    const char *password = my->password != NULL ? my->password : "";
                    my_put(my, password, strlen(password) + 1);
                    my_send(my);
                } else {
                    my_send_rsa_password(my, nonce);
                }
                continue;
            }
        }
        my_auth_failed(my, "unexpected reply while logging in");
    }
    qiq_net_set_timeout_ms(my->socket, -1);
    if (my->settings != NULL) my_simple_query(my, my->settings);
}

// ---------------------------------------------------------------------------
// SQL text

static bool my_is_word(char c) {
    return c == '_' || c == '$' || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || (unsigned char)c >= 0x80;
}

// If sql[i] starts a string, quoted name, or comment, the index just past
// it; otherwise i.
static size_t my_skip(const char *sql, size_t i) {
    char c = sql[i];
    if (c == '\'' || c == '"' || c == '`') {
        for (i++; sql[i] != '\0'; i++) {
            if (c != '`' && sql[i] == '\\' && sql[i + 1] != '\0') i++;
            else if (sql[i] == c) {
                if (sql[i + 1] == c) i++;
                else return i + 1;
            }
        }
        return i;
    }
    // "-- " (with a space or control character after it) and "#" comments.
    if ((c == '-' && sql[i + 1] == '-' && (sql[i + 2] == '\0' || (unsigned char)sql[i + 2] <= ' ')) || c == '#') {
        while (sql[i] != '\0' && sql[i] != '\n') i++;
        return i;
    }
    if (c == '/' && sql[i + 1] == '*') {
        const char *close = strstr(sql + i + 2, "*/");
        return close != NULL ? (size_t)(close - sql) + 2 : strlen(sql);
    }
    return i;
}

// Whether there's another statement after the first `;`.
static bool my_has_more_statements(const char *sql) {
    bool ended = false;
    for (size_t i = 0; sql[i] != '\0';) {
        size_t past = my_skip(sql, i);
        if (past != i) {
            // Comments after the end are fine; a string isn't.
            if (ended && sql[i] != '-' && sql[i] != '#' && sql[i] != '/') return true;
            i = past;
            continue;
        }
        if (sql[i] == ';') ended = true;
        else if (ended && sql[i] != ' ' && sql[i] != '\t' && sql[i] != '\r' && sql[i] != '\n') return true;
        i++;
    }
    return false;
}

static void my_word(const char *sql, size_t length, size_t *at, char *word, size_t size) {
    while (*at < length && (sql[*at] == ' ' || sql[*at] == '\t' || sql[*at] == '\r' || sql[*at] == '\n')) (*at)++;
    size_t o = 0;
    while (*at < length && my_is_word(sql[*at]) && sql[*at] != '$') {
        if (o + 1 < size) word[o++] = (char)(sql[*at] | 0x20);
        (*at)++;
    }
    word[o] = '\0';
}

// Whether a statement is a SET that lasts for the session: not SET GLOBAL
// or PERSIST, which change the server, or SET TRANSACTION, which applies to
// the next transaction only.
static bool my_is_session_setting(const char *sql, size_t length) {
    size_t at = 0;
    char word[16];
    my_word(sql, length, &at, word, sizeof word);
    if (strcmp(word, "set") != 0) return false;
    my_word(sql, length, &at, word, sizeof word);
    return strcmp(word, "global") != 0 && strcmp(word, "persist") != 0 && strcmp(word, "persist_only") != 0 && strcmp(word, "transaction") != 0;
}

static void my_remember_settings(QiqMysql *my, const char *sql) {
    size_t start = 0, length = strlen(sql);
    for (size_t i = 0; i <= length;) {
        size_t past = i < length ? my_skip(sql, i) : i;
        if (past != i) {
            i = past;
            continue;
        }
        if (i == length || sql[i] == ';') {
            const char *statement = sql + start;
            size_t size = i - start;
            if (my_is_session_setting(statement, size)) {
                size_t old = my->settings != NULL ? strlen(my->settings) : 0;
                char *grown = realloc(my->settings, old + size + 3);
                if (grown == NULL) qiq_fatal("out of memory");
                memcpy(grown + old, statement, size);
                memcpy(grown + old + size, ";\n", 3);
                my->settings = grown;
            }
            start = i + 1;
        }
        i++;
    }
}

// ---------------------------------------------------------------------------
// Results

static void my_read_column(QiqMysql *my) {
    MyReader reader = my_reader(my);
    for (int i = 0; i < 4; i++) my_bytes(&reader, my_lenenc(&reader, NULL)); // catalog, schema, table, original table
    uint64_t name_length = my_lenenc(&reader, NULL);
    const unsigned char *name = my_bytes(&reader, name_length);
    my_bytes(&reader, my_lenenc(&reader, NULL)); // original name
    my_lenenc(&reader, NULL);                     // length of the fixed fields
    MyColumn column;
    column.charset = (uint16_t)my_int(&reader, 2);
    my_int(&reader, 4); // display length
    column.type = (uint8_t)my_int(&reader, 1);
    column.flags = (uint16_t)my_int(&reader, 2);
    column.decimals = (uint8_t)my_int(&reader, 1);
    my->columns = my_grow(my->columns, &my->columns_capacity, my->column_count + 1, sizeof(MyColumn));
    my->columns[my->column_count++] = column;
    qiq_sql_add_column((const char *)name, (size_t)name_length);
}

static bool my_is_integer(uint8_t type) {
    return type == MY_TINY || type == MY_SHORT || type == MY_LONG || type == MY_LONGLONG || type == MY_INT24 || type == MY_YEAR;
}

// A value given as text, in a text row or as a binary row's string.
static void my_add_text(MyColumn column, const unsigned char *text, size_t length) {
    if (my_is_integer(column.type)) {
        char buffer[32];
        if (length >= sizeof buffer) length = sizeof buffer - 1;
        memcpy(buffer, text, length);
        buffer[length] = '\0';
        // BIGINT UNSIGNED beyond an int's range stays exact as a decimal.
        if ((column.flags & MY_UNSIGNED_FLAG) && strtoull(buffer, NULL, 10) > INT64_MAX) {
            qiq_sql_add_text(buffer, length, QIQ_SQL_DECIMAL);
        } else {
            qiq_sql_add_cell(QIQ_SQL_INT, strtoll(buffer, NULL, 10));
        }
    } else if (column.type == MY_FLOAT || column.type == MY_DOUBLE) {
        char buffer[64];
        if (length >= sizeof buffer) length = sizeof buffer - 1;
        memcpy(buffer, text, length);
        buffer[length] = '\0';
        qiq_sql_add_double(strtod(buffer, NULL));
    } else if (column.type == MY_DECIMAL || column.type == MY_NEWDECIMAL) {
        qiq_sql_add_text((const char *)text, length, QIQ_SQL_DECIMAL);
    } else if (column.type == MY_JSON) {
        qiq_sql_add_text((const char *)text, length, QIQ_SQL_TEXT);
    } else if (column.type == MY_BIT || column.type == MY_GEOMETRY || column.charset == MY_BINARY_CHARSET) {
        // BINARY, VARBINARY, BLOB, BIT
        qiq_sql_add_bytes(text, length);
    } else {
        qiq_sql_add_text((const char *)text, length, QIQ_SQL_TEXT);
    }
}

static void my_read_text_row(QiqMysql *my) {
    MyReader reader = my_reader(my);
    for (size_t column = 0; column < my->column_count; column++) {
        bool null;
        uint64_t length = my_lenenc(&reader, &null);
        if (null) qiq_sql_add_cell(QIQ_SQL_NULL, 0);
        else my_add_text(my->columns[column], my_bytes(&reader, length), (size_t)length);
    }
}

// A float as the shortest text that reads back as the same float, so
// FLOAT 3.14 is 3.14 rather than 3.140000104904175.
static double my_float(float value) {
    char buffer[32];
    for (int digits = 6; digits <= 9; digits++) {
        snprintf(buffer, sizeof buffer, "%.*g", digits, (double)value);
        if (strtof(buffer, NULL) == value) break;
    }
    return strtod(buffer, NULL);
}

// DATE, DATETIME, and TIMESTAMP: a length, then the fields it covers.
static void my_add_datetime(MyColumn column, MyReader *reader) {
    size_t length = (size_t)my_int(reader, 1);
    unsigned year = 0, month = 0, day = 0, hour = 0, minute = 0, second = 0, micro = 0;
    if (length >= 4) year = (unsigned)my_int(reader, 2), month = (unsigned)my_int(reader, 1), day = (unsigned)my_int(reader, 1);
    if (length >= 7) hour = (unsigned)my_int(reader, 1), minute = (unsigned)my_int(reader, 1), second = (unsigned)my_int(reader, 1);
    if (length >= 11) micro = (unsigned)my_int(reader, 4);
    char text[40];
    int written = snprintf(text, sizeof text, "%04u-%02u-%02u", year, month, day);
    if (column.type != MY_DATE) {
        written += snprintf(text + written, sizeof text - (size_t)written, " %02u:%02u:%02u", hour, minute, second);
        if (column.decimals > 0 && column.decimals <= 6) {
            char fraction[8];
            snprintf(fraction, sizeof fraction, "%06u", micro);
            written += snprintf(text + written, sizeof text - (size_t)written, ".%.*s", column.decimals, fraction);
        }
    }
    qiq_sql_add_text(text, (size_t)written, QIQ_SQL_TEXT);
}

static void my_add_time(MyColumn column, MyReader *reader) {
    size_t length = (size_t)my_int(reader, 1);
    unsigned negative = 0, days = 0, hour = 0, minute = 0, second = 0, micro = 0;
    if (length >= 8) {
        negative = (unsigned)my_int(reader, 1);
        days = (unsigned)my_int(reader, 4);
        hour = (unsigned)my_int(reader, 1), minute = (unsigned)my_int(reader, 1), second = (unsigned)my_int(reader, 1);
    }
    if (length >= 12) micro = (unsigned)my_int(reader, 4);
    char text[40];
    int written = snprintf(text, sizeof text, "%s%02u:%02u:%02u", negative ? "-" : "", days * 24 + hour, minute, second);
    if (column.decimals > 0 && column.decimals <= 6) {
        char fraction[8];
        snprintf(fraction, sizeof fraction, "%06u", micro);
        written += snprintf(text + written, sizeof text - (size_t)written, ".%.*s", column.decimals, fraction);
    }
    qiq_sql_add_text(text, (size_t)written, QIQ_SQL_TEXT);
}

static void my_read_binary_row(QiqMysql *my) {
    MyReader reader = my_reader(my);
    my_int(&reader, 1); // 0x00
    size_t bitmap_size = (my->column_count + 7 + 2) / 8;
    const unsigned char *nulls = my_bytes(&reader, bitmap_size);
    for (size_t index = 0; index < my->column_count; index++) {
        MyColumn column = my->columns[index];
        if (nulls[(index + 2) / 8] & (1 << ((index + 2) % 8))) {
            qiq_sql_add_cell(QIQ_SQL_NULL, 0);
            continue;
        }
        bool is_unsigned = column.flags & MY_UNSIGNED_FLAG;
        switch (column.type) {
        case MY_TINY: {
            uint64_t bits = my_int(&reader, 1);
            qiq_sql_add_cell(QIQ_SQL_INT, is_unsigned ? (int64_t)bits : (int64_t)(int8_t)bits);
            break;
        }
        case MY_SHORT:
        case MY_YEAR: {
            uint64_t bits = my_int(&reader, 2);
            qiq_sql_add_cell(QIQ_SQL_INT, is_unsigned ? (int64_t)bits : (int64_t)(int16_t)bits);
            break;
        }
        case MY_LONG:
        case MY_INT24: {
            uint64_t bits = my_int(&reader, 4);
            qiq_sql_add_cell(QIQ_SQL_INT, is_unsigned ? (int64_t)bits : (int64_t)(int32_t)bits);
            break;
        }
        case MY_LONGLONG: {
            uint64_t bits = my_int(&reader, 8);
            if (is_unsigned && bits > INT64_MAX) {
                char text[24];
                int written = snprintf(text, sizeof text, "%llu", (unsigned long long)bits);
                qiq_sql_add_text(text, (size_t)written, QIQ_SQL_DECIMAL);
            } else {
                qiq_sql_add_cell(QIQ_SQL_INT, (int64_t)bits);
            }
            break;
        }
        case MY_FLOAT: {
            uint32_t bits = (uint32_t)my_int(&reader, 4);
            float value;
            memcpy(&value, &bits, sizeof value);
            qiq_sql_add_double(my_float(value));
            break;
        }
        case MY_DOUBLE: {
            uint64_t bits = my_int(&reader, 8);
            double value;
            memcpy(&value, &bits, sizeof value);
            qiq_sql_add_double(value);
            break;
        }
        case MY_DATE:
        case MY_DATETIME:
        case MY_TIMESTAMP:
            my_add_datetime(column, &reader);
            break;
        case MY_TIME:
            my_add_time(column, &reader);
            break;
        default: {
            uint64_t length = my_lenenc(&reader, NULL);
            my_add_text(column, my_bytes(&reader, length), (size_t)length);
        }
        }
    }
}

// ---------------------------------------------------------------------------
// Connections

static void my_free(QiqMysql *my) {
    free(my->ca_file);
    free(my->socket_file);
    for (int i = 0; i < MY_CACHE; i++) free(my->cache[i].sql);
    free(my->host);
    free(my->user);
    free(my->password);
    free(my->database);
    free(my->settings);
    free(my->out);
    free(my->in);
    free(my->columns);
    free(my);
}

static void *qiq_my_open(const char *dsn) {
    QiqSqlUrl url;
    if (!qiq_sql_parse_url(dsn, &url)) {
        qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "Database::open: expected a MySQL URL such as \"mysql://user:password@host:3306/database\"");
    }
    QiqMysql *my = calloc(1, sizeof *my);
    if (my == NULL) qiq_fatal("out of memory");
    my->socket = -1;
    my->fork_count = qiq_sql_forks;
    my->host = my_copy(url.host != NULL ? url.host : "localhost");
    my->user = my_copy(url.user != NULL ? url.user : "root");
    if (url.password != NULL) my->password = my_copy(url.password);
    else if (getenv("MYSQL_PWD") != NULL) my->password = my_copy(getenv("MYSQL_PWD"));
    if (url.database != NULL) my->database = my_copy(url.database);
    my->port = url.port != 0 ? url.port : 3306;
    QiqSqlOptions options;
    const char *problem = qiq_sql_connection_options(&url, url.host, &options);
    qiq_sql_free_url(&url);
    if (problem != NULL) {
        my_free(my);
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "Database::open: in a MySQL URL, %s", problem);
    }
    my->tls = options.tls;
    my->connect_timeout = options.timeout;
    my->ca_file = options.ca_file;
    options.ca_file = NULL;
    if (options.socket != NULL) {
        // A directory holds mysqld.sock, the usual name.
        my->socket_file = qiq_sql_socket_file(options.socket, "mysqld.sock");
        free(my->host);
        my->host = my_copy(my->socket_file); // for messages
    }
    qiq_sql_free_options(&options);
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        if (my->socket >= 0) qiq_net_abandon(my->socket);
        my_free(my);
        qiq_rethrow(previous);
    }
    my_connect(my);
    qiq_guard_leave(previous);
    return my;
}

static void qiq_my_close(QiqMysql *my) {
    if (my->socket >= 0) {
        if (my->fork_count == qiq_sql_forks) {
            jmp_buf environment;
            jmp_buf *previous = qiq_guard_enter(&environment);
            if (!setjmp(environment)) {
                my_command(my, 0x01); // COM_QUIT
                my_send(my);
                qiq_net_close(my->socket);
            } else {
                qiq_net_abandon(my->socket);
            }
            qiq_guard_leave(previous);
        } else {
            qiq_net_abandon(my->socket);
        }
    }
    my_free(my);
}

static bool qiq_my_in_transaction(QiqMysql *my) {
    return my->socket >= 0 && my->fork_count == qiq_sql_forks && (my->status & MY_STATUS_IN_TRANSACTION);
}

static int64_t qiq_my_last_insert_id(QiqMysql *my) {
    return my->last_insert_id;
}

static void my_lose(QiqMysql *my) {
    my->streaming = false;
    if (my->socket < 0) return;
    if (my->status & MY_STATUS_IN_TRANSACTION) my->lost_in_transaction = true;
    qiq_net_abandon(my->socket);
    my->socket = -1;
    my->status = 0;
    for (int i = 0; i < MY_CACHE; i++) {
        free(my->cache[i].sql);
        my->cache[i].sql = NULL;
    }
}

static void my_failed(QiqMysql *my) {
    qiq_sql_drop_pending();
    qiq_sql_clear_results();
    my->out_length = 0;
    if (my->in_io) my_lose(my);
    my->in_io = false;
}

static void my_ready(QiqMysql *my) {
    if (my->fork_count != qiq_sql_forks) {
        // The parent's connection and statements: left alone, and opened again here.
        if (my->socket >= 0) qiq_net_abandon(my->socket);
        my->socket = -1;
        my->fork_count = qiq_sql_forks;
        my->status = 0;
        my->streaming = false;
        for (int i = 0; i < MY_CACHE; i++) {
            free(my->cache[i].sql);
            my->cache[i].sql = NULL;
        }
    }
    if (my->lost_in_transaction) {
        my->lost_in_transaction = false;
        qiq_throw(QIQ_DATABASE_EXCEPTION, "the connection to MySQL was lost during a transaction, which the server rolled back; start it again");
    }
    if (my->socket < 0) {
        my->in_io = true;
        my_connect(my);
        my->in_io = false;
    }
}

// Reads one result: an OK, an error, or a result set (columns, then rows
// until an EOF). Collects rows when `collect` is set (at most one when
// `first_only`). Returns the rows changed (for an OK, when `collect` isn't
// set) or collected; on an error, writes it to `message` and returns -1.
static int64_t my_read_result(QiqMysql *my, bool binary, bool collect, bool first_only, MyError *error) {
    my_read(my);
    if (my->in_length == 0) my_malformed();
    if (my->in[0] == 0x00) {
        int64_t changed = my_ok(my);
        return collect ? 0 : changed;
    }
    if (my->in[0] == 0xff) {
        my_error_into(my, error);
        return -1;
    }
    MyReader reader = my_reader(my);
    uint64_t columns = my_lenenc(&reader, NULL);
    my->column_count = 0;
    for (uint64_t i = 0; i < columns; i++) {
        my_read(my);
        if (collect) my_read_column(my);
        else {
            my->columns = my_grow(my->columns, &my->columns_capacity, my->column_count + 1, sizeof(MyColumn));
            my->column_count++;
        }
    }
    my_read(my); // EOF after the columns
    int64_t rows = 0;
    for (;;) {
        my_read(my);
        if (my_is_eof(my)) {
            my_eof(my);
            break;
        }
        if (my->in[0] == 0xff) {
            my_error_into(my, error);
            return -1;
        }
        if (collect && !(first_only && rows > 0)) {
            if (binary) my_read_binary_row(my);
            else my_read_text_row(my);
            rows++;
        }
    }
    return rows;
}

// Runs `sql` with COM_QUERY: for scripts, and statements that can't be
// prepared. Reads every result; throws the first error.
static int64_t my_text_query(QiqMysql *my, const char *sql, bool collect, bool first_only) {
    my_command(my, 0x03);
    my_put(my, sql, strlen(sql));
    my_send(my);
    MyError error;
    int64_t result = 0;
    bool first = true;
    do {
        int64_t got = my_read_result(my, false, collect && first, first_only, &error);
        if (got < 0) {
            // The server stops at an error and is ready for the next command.
            my->in_io = false;
            qiq_sql_clear_results();
            my_throw(&error);
        }
        if (first) result = got;
        first = false;
    } while (my->status & MY_STATUS_MORE_RESULTS);
    return result;
}

static void my_simple_query(QiqMysql *my, const char *sql) {
    my_text_query(my, sql, false, false);
}

static void qiq_my_script(QiqMysql *my, const char *sql) {
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        my_failed(my);
        qiq_rethrow(previous);
    }
    my_ready(my);
    my->in_io = true;
    my_simple_query(my, sql);
    my->in_io = false;
    my_remember_settings(my, sql);
    qiq_guard_leave(previous);
}

// The prepared statement for `sql`, from the cache or prepared now (closing
// the least recently used). NULL when the server can't prepare it, with
// *unsupported set.
static MyCached *my_prepare(QiqMysql *my, const char *sql, const char *caller, bool *unsupported) {
    *unsupported = false;
    my->clock++;
    MyCached *oldest = &my->cache[0];
    for (int i = 0; i < MY_CACHE; i++) {
        MyCached *cached = &my->cache[i];
        if (cached->sql != NULL && strcmp(cached->sql, sql) == 0) {
            cached->used = my->clock;
            return cached;
        }
        if (cached->sql == NULL || cached->used < oldest->used) oldest = cached;
        if (cached->sql == NULL) break;
    }
    if (my_has_more_statements(sql)) {
        qiq_throwf(QIQ_DATABASE_EXCEPTION, "%s runs one SQL statement, but this has more; run several with executeScript", caller);
    }
    my->in_io = true;
    my_command(my, 0x16); // COM_STMT_PREPARE
    my_put(my, sql, strlen(sql));
    my_send(my);
    my_read(my);
    if (my->in_length > 0 && my->in[0] == 0xff) {
        MyError error;
        my_error_into(my, &error);
        my->in_io = false;
        if (error.code == MY_UNSUPPORTED_PS) {
            *unsupported = true;
            return NULL;
        }
        my_throw(&error);
    }
    MyReader reader = my_reader(my);
    my_int(&reader, 1);
    uint32_t id = (uint32_t)my_int(&reader, 4);
    int columns = (int)my_int(&reader, 2);
    int params = (int)my_int(&reader, 2);
    // The parameters' and columns' definitions, each list ending with an EOF.
    if (params > 0) {
        for (int i = 0; i <= params; i++) my_read(my);
    }
    if (columns > 0) {
        for (int i = 0; i <= columns; i++) my_read(my);
    }
    if (oldest->sql != NULL) {
        // COM_STMT_CLOSE has no reply.
        my_command(my, 0x19);
        my_put_int(my, oldest->id, 4);
        my_send(my);
        free(oldest->sql);
    }
    my->in_io = false;
    *oldest = (MyCached){my_copy(sql), id, params, my->clock};
    return oldest;
}

// Sends COM_STMT_EXECUTE for `statement` with the pending values: no
// cursor, one run, and the values in binary.
static void my_send_execute(QiqMysql *my, MyCached *statement, const char *caller) {
    size_t given = qiq_sql_pending_count;
    if ((size_t)statement->params != given) {
        qiq_throwf(QIQ_DATABASE_EXCEPTION, "%s: the SQL has %d placeholder%s (?), but %zu value%s given", caller, statement->params,
                   statement->params == 1 ? "" : "s", given, given == 1 ? " was" : "s were");
    }
    my_command(my, 0x17);
    my_put_int(my, statement->id, 4);
    my_put_byte(my, 0);
    my_put_int(my, 1, 4);
    if (given > 0) {
        size_t bitmap_at = my->out_length;
        for (size_t i = 0; i < (given + 7) / 8; i++) my_put_byte(my, 0);
        my_put_byte(my, 1); // types follow
        for (size_t i = 0; i < given; i++) {
            QiqSqlValue value = qiq_sql_pending[i];
            bool null = value.type == 'n' || ((value.type == 's' || value.type == 'a') && value.payload == 0);
            if (null) my->out[bitmap_at + i / 8] |= (unsigned char)(1 << (i % 8));
            unsigned char type = null ? MY_NULL : value.type == 'd' ? MY_DOUBLE : value.type == 's' ? MY_VAR_STRING : value.type == 'a' ? MY_BLOB : MY_LONGLONG;
            my_put_byte(my, type);
            my_put_byte(my, 0);
        }
        for (size_t i = 0; i < given; i++) {
            QiqSqlValue value = qiq_sql_pending[i];
            if (value.type == 'n' || ((value.type == 's' || value.type == 'a') && value.payload == 0)) continue;
            if (value.type == 's') {
                const char *text = (const char *)(uintptr_t)value.payload;
                size_t length = strlen(text);
                my_put_lenenc(my, length);
                my_put(my, text, length);
            } else if (value.type == 'a') {
                void *array = (void *)(uintptr_t)value.payload;
                my_put_lenenc(my, (uint64_t)QIQ_ARRAY_LENGTH(array));
                my_put(my, QIQ_ARRAY_DATA(array), (size_t)QIQ_ARRAY_LENGTH(array));
            } else if (value.type == 'b') {
                my_put_int(my, value.payload ? 1 : 0, 8);
            } else {
                my_put_int(my, value.payload, 8); // the int's or double's bits
            }
        }
    }
    qiq_sql_drop_pending();
    my->in_io = true;
    my_send(my);
}

static int64_t qiq_my_run(QiqMysql *my, const char *sql, int64_t mode, const char *caller) {
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        my_failed(my);
        qiq_rethrow(previous);
    }
    my_ready(my);
    bool unsupported;
    MyCached *statement = my_prepare(my, sql, caller, &unsupported);
    size_t given = qiq_sql_pending_count;
    int64_t result;
    if (statement == NULL) {
        // Not preparable (such as some administrative statements): run as
        // text, which only works without values.
        if (given > 0) qiq_throwf(QIQ_DATABASE_EXCEPTION, "%s: MySQL can't take values for this kind of statement", caller);
        my->in_io = true;
        result = my_text_query(my, sql, mode != 0, mode == 2);
        my->in_io = false;
    } else {
        my_send_execute(my, statement, caller);
        MyError error;
        result = my_read_result(my, true, mode != 0, mode == 2, &error);
        bool failed = result < 0;
        // A procedure's further results, which nothing reads.
        while (!failed && (my->status & MY_STATUS_MORE_RESULTS)) {
            if (my_read_result(my, true, false, false, &error) < 0) failed = true;
        }
        my->in_io = false;
        if (failed) {
            qiq_sql_clear_results();
            my_throw(&error);
        }
    }
    qiq_sql_drop_pending();
    my_remember_settings(my, sql);
    qiq_guard_leave(previous);
    return result;
}

// ---------------------------------------------------------------------------
// Cursors: the rows are read off the connection as they're asked for.

static void qiq_my_open_cursor(QiqMysql *my, const char *sql, const char *caller) {
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        my_failed(my);
        qiq_rethrow(previous);
    }
    my_ready(my);
    bool unsupported;
    MyCached *statement = my_prepare(my, sql, caller, &unsupported);
    if (statement == NULL) {
        if (qiq_sql_pending_count > 0) qiq_throwf(QIQ_DATABASE_EXCEPTION, "%s: MySQL can't take values for this kind of statement", caller);
        my_command(my, 0x03);
        my_put(my, sql, strlen(sql));
        my->in_io = true;
        my_send(my);
    } else {
        my_send_execute(my, statement, caller);
    }
    my->streaming_binary = statement != NULL;
    // The first response: an OK (no rows), an error, or the columns.
    my_read(my);
    if (my->in_length == 0) my_malformed();
    if (my->in[0] == 0x00) {
        my_ok(my);
        // Further results (a procedure's) are read and dropped.
        MyError error;
        while (my->status & MY_STATUS_MORE_RESULTS) {
            if (my_read_result(my, my->streaming_binary, false, false, &error) < 0) {
                my->in_io = false;
                my_throw(&error);
            }
        }
    } else if (my->in[0] == 0xff) {
        MyError error;
        my_error_into(my, &error);
        my->in_io = false;
        my_throw(&error);
    } else {
        MyReader reader = my_reader(my);
        uint64_t columns = my_lenenc(&reader, NULL);
        my->column_count = 0;
        for (uint64_t i = 0; i < columns; i++) {
            my_read(my);
            my_read_column(my);
        }
        my_read(my); // EOF after the columns
        my->streaming = true;
    }
    my->in_io = false;
    qiq_guard_leave(previous);
}

// Reads the end of a cursor's result: the further results a procedure can
// have, then the connection is free.
static void my_finish_stream(QiqMysql *my) {
    my_eof(my);
    my->streaming = false;
    MyError error;
    while (my->status & MY_STATUS_MORE_RESULTS) {
        if (my_read_result(my, my->streaming_binary, false, false, &error) < 0) {
            my->in_io = false;
            my_throw(&error);
        }
    }
}

static int64_t qiq_my_fetch(QiqMysql *my, int64_t count) {
    if (!my->streaming || my->socket < 0 || my->fork_count != qiq_sql_forks) {
        my->streaming = false;
        return 0;
    }
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        my_failed(my);
        qiq_rethrow(previous);
    }
    my->in_io = true;
    int64_t rows = 0;
    while (rows < count) {
        my_read(my);
        if (my_is_eof(my)) {
            my_finish_stream(my);
            break;
        }
        if (my->in_length > 0 && my->in[0] == 0xff) {
            MyError error;
            my_error_into(my, &error);
            my->streaming = false;
            my->in_io = false;
            qiq_sql_clear_results();
            my_throw(&error);
        }
        if (my->streaming_binary) my_read_binary_row(my);
        else my_read_text_row(my);
        rows++;
    }
    my->in_io = false;
    qiq_guard_leave(previous);
    return rows;
}

// Stops the statement this connection is running: KILL QUERY, from a
// connection of its own with the same settings. MySQL returns once the
// statement is marked, and a statement that already ended isn't affected
// (the mark is cleared when the next one starts). A failure here only means
// the rest of the rows are read instead.
static void my_kill_query(QiqMysql *my) {
    QiqMysql *killer = calloc(1, sizeof *killer);
    if (killer == NULL) qiq_fatal("out of memory");
    killer->socket = -1;
    killer->fork_count = qiq_sql_forks;
    killer->host = my_copy(my->host);
    killer->user = my_copy(my->user);
    if (my->password != NULL) killer->password = my_copy(my->password);
    if (my->ca_file != NULL) killer->ca_file = my_copy(my->ca_file);
    if (my->socket_file != NULL) killer->socket_file = my_copy(my->socket_file);
    killer->port = my->port;
    killer->connect_timeout = my->connect_timeout;
    killer->tls = my->tls;
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        qiq_guard_leave(previous);
        if (killer->socket >= 0) qiq_net_abandon(killer->socket);
        my_free(killer);
        qiq_discard_exception();
        return;
    }
    my_connect(killer);
    char kill[40];
    snprintf(kill, sizeof kill, "KILL QUERY %u", my->connection_id);
    my_simple_query(killer, kill);
    qiq_guard_leave(previous);
    qiq_my_close(killer);
}

static void qiq_my_close_cursor(QiqMysql *my) {
    if (!my->streaming || my->socket < 0 || my->fork_count != qiq_sql_forks) {
        my->streaming = false;
        return;
    }
    jmp_buf environment;
    jmp_buf *previous = qiq_guard_enter(&environment);
    if (setjmp(environment)) {
        my_failed(my);
        qiq_rethrow(previous);
    }
    my->in_io = true;
    // The rows left are read and dropped. After a few thousand, the
    // statement is stopped: logging in to do that takes a few milliseconds,
    // which reading a short rest of a result doesn't. The server then sends
    // an error in place of the rows it hadn't sent.
    for (int read = 0;; read++) {
        if (read == 5000) my_kill_query(my);
        my_read(my);
        if (my_is_eof(my)) {
            my_finish_stream(my);
            break;
        }
        if (my->in_length > 0 && my->in[0] == 0xff) {
            my->streaming = false;
            break;
        }
    }
    my->in_io = false;
    qiq_guard_leave(previous);
}

const QiqSqlDriver qiq_mysql_driver = {
    .open = qiq_my_open,
    .run = (int64_t (*)(void *, const char *, int64_t, const char *))qiq_my_run,
    .script = (void (*)(void *, const char *))qiq_my_script,
    .last_insert_id = (int64_t (*)(void *))qiq_my_last_insert_id,
    .in_transaction = (bool (*)(void *))qiq_my_in_transaction,
    .close = (void (*)(void *))qiq_my_close,
    .open_cursor = (void (*)(void *, const char *, const char *))qiq_my_open_cursor,
    .fetch = (int64_t (*)(void *, int64_t))qiq_my_fetch,
    .close_cursor = (void (*)(void *))qiq_my_close_cursor,
};
