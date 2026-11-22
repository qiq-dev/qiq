// URLs for std/net's Url class (stdlib/net.qiq): splitting one into its
// parts, percent-escaping each part by its rules, and removing dot segments,
// as RFC 3986 says, with Go's net/url leniency where it helps: characters
// that should have been escaped (a space, "é") are accepted and escaped.
// Invalid URLs throw InvalidArgumentException.
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "qiq_runtime.h"

typedef struct {
    char *data;
    size_t length, capacity;
} QiqUrlText;

static void qiq_url_add(QiqUrlText *text, const char *bytes, size_t length) {
    if (text->length + length + 1 > text->capacity) {
        size_t capacity = text->capacity ? text->capacity * 2 : 64;
        while (capacity < text->length + length + 1) capacity *= 2;
        text->data = realloc(text->data, capacity);
        if (text->data == NULL) qiq_fatal("out of memory");
        text->capacity = capacity;
    }
    memcpy(text->data + text->length, bytes, length);
    text->length += length;
    text->data[text->length] = '\0';
}

static char *qiq_url_finish(QiqUrlText *text) {
    char *result = qiq_string_from(text->data != NULL ? text->data : "", text->length);
    free(text->data);
    return result;
}

static int qiq_url_hex(char c) {
    return c >= '0' && c <= '9' ? c - '0' : c >= 'a' && c <= 'f' ? c - 'a' + 10 : c >= 'A' && c <= 'F' ? c - 'A' + 10 : -1;
}

// The parts, and what each leaves unescaped besides the unreserved
// characters (letters, digits, and -._~).
enum { QIQ_URL_PATH = 0, QIQ_URL_QUERY_COMPONENT = 1, QIQ_URL_FRAGMENT = 2, QIQ_URL_USERINFO = 3, QIQ_URL_QUERY = 4 };

static bool qiq_url_keeps(unsigned char c, int64_t part) {
    if ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '-' || c == '.' || c == '_' || c == '~') return true;
    if (c >= 0x80 || c < 0x21) return false;
    switch (part) {
    case QIQ_URL_PATH: return strchr("!$&'()*+,;=:@/", c) != NULL;
    case QIQ_URL_QUERY_COMPONENT: return false;
    case QIQ_URL_FRAGMENT: return strchr("!$&'()*+,;=:@/?", c) != NULL;
    case QIQ_URL_USERINFO: return strchr("!$&'()*+,;=", c) != NULL;
    case QIQ_URL_QUERY: return strchr("!$&'()*+,;=:@/?", c) != NULL;
    default: return false;
    }
}

static void qiq_url_escape_byte(QiqUrlText *out, unsigned char c) {
    static const char digits[] = "0123456789ABCDEF";
    char escaped[3] = {'%', digits[c >> 4], digits[c & 15]};
    qiq_url_add(out, escaped, 3);
}

// `text` escaped for `part`: every byte it doesn't keep as %XX, and (for a
// query component) a space as "+".
QIQ_GUARDED(char *, qiq_stdlib___url_escape, NULL, (const char *text, int64_t part), (text, part)) {
    if (text == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "Url: the text is null");
    QiqUrlText out = {0};
    for (const unsigned char *c = (const unsigned char *)text; *c; c++) {
        if (qiq_url_keeps(*c, part)) qiq_url_add(&out, (const char *)c, 1);
        else if (*c == ' ' && part == QIQ_URL_QUERY_COMPONENT) qiq_url_add(&out, "+", 1);
        else qiq_url_escape_byte(&out, *c);
    }
    return qiq_url_finish(&out);
}

// `raw` as a part of a URL: its %XX escapes kept (in upper case), and the
// characters the part can't hold as they are escaped. A "%" without two hex
// digits after it is an error.
static bool qiq_url_normalize(QiqUrlText *out, const char *raw, size_t length, int64_t part) {
    for (size_t i = 0; i < length; i++) {
        unsigned char c = (unsigned char)raw[i];
        if (c == '%') {
            if (i + 2 >= length || qiq_url_hex(raw[i + 1]) < 0 || qiq_url_hex(raw[i + 2]) < 0) return false;
            qiq_url_escape_byte(out, (unsigned char)(qiq_url_hex(raw[i + 1]) * 16 + qiq_url_hex(raw[i + 2])));
            i += 2;
        } else if (qiq_url_keeps(c, part)) {
            qiq_url_add(out, (const char *)&raw[i], 1);
        } else {
            qiq_url_escape_byte(out, c);
        }
    }
    return true;
}

// The text an escaped part stands for: %XX as the byte, and (with `plus`)
// "+" as a space. It must be UTF-8 text without NUL, as qiq strings are.
static char *qiq_url_unescape(const char *raw, size_t length, bool plus, const char *what) {
    char *bytes = malloc(length + 1);
    if (bytes == NULL) qiq_fatal("out of memory");
    size_t o = 0;
    for (size_t i = 0; i < length; i++) {
        if (raw[i] == '%') {
            if (i + 2 >= length || qiq_url_hex(raw[i + 1]) < 0 || qiq_url_hex(raw[i + 2]) < 0) {
                free(bytes);
                qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "Url: the %s has a %% that isn't followed by two hex digits", what);
            }
            bytes[o++] = (char)(qiq_url_hex(raw[i + 1]) * 16 + qiq_url_hex(raw[i + 2]));
            i += 2;
        } else {
            bytes[o++] = plus && raw[i] == '+' ? ' ' : raw[i];
        }
    }
    if (!qiq_is_utf8_text((const unsigned char *)bytes, o)) {
        free(bytes);
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "Url: the %s's escapes stand for bytes that aren't UTF-8 text", what);
    }
    char *text = qiq_string_from(bytes, o);
    free(bytes);
    return text;
}

// What an escaped part stands for, as Url's query(), path(), and the rest
// read it: `plus` for query names and values, where "+" is a space.
QIQ_GUARDED(char *, qiq_stdlib___url_unescape, NULL, (const char *raw, bool plus), (raw, plus)) {
    if (raw == NULL) return NULL;
    return qiq_url_unescape(raw, strlen(raw), plus, plus ? "query" : "path");
}

static char *qiq_url_part(const char *start, size_t length, int64_t part, const char *what, const char *url) {
    QiqUrlText out = {0};
    qiq_url_add(&out, "", 0);
    if (!qiq_url_normalize(&out, start, length, part)) {
        free(out.data);
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "Url::parse: the %s of \"%s\" has a %% that isn't followed by two hex digits", what, url);
    }
    return qiq_url_finish(&out);
}

static bool qiq_url_scheme_char(char c, bool first) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (!first && ((c >= '0' && c <= '9') || c == '+' || c == '-' || c == '.'));
}

// Splits `text` into [scheme, user, password, host, port, raw path, raw
// query, fragment]: null for the parts it doesn't have (an empty host is
// "", as in file:///tmp). The scheme is in lower case; the user, password,
// and fragment are unescaped; the path and query stay escaped, normalized.
QIQ_GUARDED(void *, qiq_stdlib___url_parse, NULL, (const char *text), (text)) {
    if (text == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "Url::parse: the text is null");
    const char *c = text;
    char *parts[8] = {NULL};
    void *array = qiq_array_create(8, 8, 1);
    // scheme ":", where the scheme is a letter and letters, digits, +-.
    const char *colon = c;
    while (*colon && qiq_url_scheme_char(*colon, colon == c)) colon++;
    if (*colon == ':' && colon > c) {
        char *scheme = malloc((size_t)(colon - c) + 1);
        if (scheme == NULL) qiq_fatal("out of memory");
        for (const char *s = c; s < colon; s++) scheme[s - c] = (char)(*s >= 'A' && *s <= 'Z' ? *s + 32 : *s);
        parts[0] = qiq_string_from(scheme, (size_t)(colon - c));
        free(scheme);
        c = colon + 1;
    }
    // "//" authority: [user[:password]@]host[:port]
    if (c[0] == '/' && c[1] == '/') {
        c += 2;
        const char *end = c + strcspn(c, "/?#");
        const char *at = NULL;
        for (const char *s = c; s < end; s++) {
            if (*s == '@') at = s; // the last: a password may contain one unescaped
        }
        const char *host = c;
        if (at != NULL) {
            const char *separator = memchr(c, ':', (size_t)(at - c));
            const char *user_end = separator != NULL ? separator : at;
            parts[1] = qiq_url_unescape(c, (size_t)(user_end - c), false, "user");
            if (separator != NULL) parts[2] = qiq_url_unescape(separator + 1, (size_t)(at - separator - 1), false, "password");
            host = at + 1;
        }
        const char *port = NULL;
        const char *host_end = end;
        if (*host == '[') {
            const char *close = memchr(host, ']', (size_t)(end - host));
            if (close == NULL) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "Url::parse: \"%s\" has a [ in its host without a ]", text);
            for (const char *s = host + 1; s < close; s++) {
                if (!(qiq_url_hex(*s) >= 0 || *s == ':' || *s == '.' || *s == '%')) {
                    qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "Url::parse: \"%s\" has a host in [ ] that isn't an IPv6 address", text);
                }
            }
            parts[3] = qiq_string_from(host + 1, (size_t)(close - host - 1));
            if (close + 1 < end) {
                if (close[1] != ':') qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "Url::parse: \"%s\" has something other than a port after its host", text);
                port = close + 2;
            }
        } else {
            const char *separator = NULL;
            for (const char *s = host; s < end; s++) {
                if (*s == ':') separator = s;
            }
            if (separator != NULL) {
                host_end = separator;
                port = separator + 1;
            }
            for (const char *s = host; s < host_end; s++) {
                if ((unsigned char)*s <= ' ' || strchr("<>\"{}|\\^`[]@", *s) != NULL) {
                    qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "Url::parse: \"%s\" has a character its host can't have", text);
                }
            }
            parts[3] = qiq_string_from(host, (size_t)(host_end - host));
        }
        if (port != NULL && port < end) {
            for (const char *s = port; s < end; s++) {
                if (*s < '0' || *s > '9') qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "Url::parse: \"%s\" has a port that isn't a number", text);
            }
            if (end - port > 5 || strtol(port, NULL, 10) > 65535) {
                qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "Url::parse: \"%s\" has a port above 65535", text);
            }
            parts[4] = qiq_string_from(port, (size_t)(end - port));
        }
        c = end;
    }
    size_t path_length = strcspn(c, "?#");
    parts[5] = qiq_url_part(c, path_length, QIQ_URL_PATH, "path", text);
    c += path_length;
    if (*c == '?') {
        c++;
        size_t query_length = strcspn(c, "#");
        parts[6] = qiq_url_part(c, query_length, QIQ_URL_QUERY, "query", text);
        c += query_length;
    }
    if (*c == '#') {
        c++;
        parts[7] = qiq_url_unescape(c, strlen(c), false, "fragment");
    }
    memcpy(QIQ_ARRAY_DATA(array), parts, sizeof parts);
    return array;
}

// A path with its "." and ".." segments removed (RFC 3986, section 5.2.4):
// "/a/b/../c/./d" is "/a/c/d".
QIQ_GUARDED(char *, qiq_stdlib___url_remove_dots, NULL, (const char *path), (path)) {
    if (path == NULL) return NULL;
    size_t length = strlen(path);
    char *in = malloc(length + 1), *out = malloc(length + 2);
    if (in == NULL || out == NULL) qiq_fatal("out of memory");
    memcpy(in, path, length + 1);
    size_t o = 0;
    char *c = in;
    while (*c) {
        if (strncmp(c, "../", 3) == 0) {
            c += 3;
        } else if (strncmp(c, "./", 2) == 0) {
            c += 2;
        } else if (strncmp(c, "/./", 3) == 0) {
            c += 2; // "/./" becomes "/"
        } else if (strcmp(c, "/.") == 0) {
            c[1] = '\0'; // becomes "/"
        } else if (strncmp(c, "/../", 4) == 0 || strcmp(c, "/..") == 0) {
            // Becomes "/", and the last segment written is dropped.
            if (c[3] == '/') c += 3;
            else c[1] = '\0';
            while (o > 0 && out[o - 1] != '/') o--;
            if (o > 0) o--;
        } else if (strcmp(c, ".") == 0 || strcmp(c, "..") == 0) {
            break;
        } else {
            // The first segment, with its leading "/" when it has one.
            size_t segment = *c == '/' ? 1 + strcspn(c + 1, "/") : strcspn(c, "/");
            memcpy(out + o, c, segment);
            o += segment;
            c += segment;
        }
    }
    char *result = qiq_string_from(out, o);
    free(in);
    free(out);
    return result;
}
