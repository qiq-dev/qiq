// Hashing for std/crypto (stdlib/crypto.qiq): message digests and HMACs,
// computed with Mbed TLS (already linked into every program for HTTPS).
#include <stdbool.h>
#include <stdint.h>
#include <string.h>
#include <strings.h>

#include "mbedtls/md.h"
#include "qiq_runtime.h"

// The algorithm named `algorithm` (in any case, as in PHP).
static const mbedtls_md_info_t *qiq_digest(const char *function, const char *algorithm) {
    if (algorithm == NULL) qiq_throwf(QIQ_RUNTIME_EXCEPTION, "%s: the algorithm is null", function);
    static const struct {
        const char *name;
        mbedtls_md_type_t type;
    } algorithms[] = {
        {"md5", MBEDTLS_MD_MD5},       {"sha1", MBEDTLS_MD_SHA1},     {"sha224", MBEDTLS_MD_SHA224},
        {"sha256", MBEDTLS_MD_SHA256}, {"sha384", MBEDTLS_MD_SHA384}, {"sha512", MBEDTLS_MD_SHA512},
    };
    for (size_t index = 0; index < sizeof algorithms / sizeof *algorithms; index++) {
        if (strcasecmp(algorithm, algorithms[index].name) == 0) {
            const mbedtls_md_info_t *info = mbedtls_md_info_from_type(algorithms[index].type);
            if (info == NULL) break;
            return info;
        }
    }
    qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION,
               "%s: unknown algorithm \"%s\"; use md5, sha1, sha224, sha256, sha384, or sha512", function, algorithm);
}

static const char *qiq_require_text(const char *function, const char *text, const char *what) {
    if (text == NULL) qiq_throwf(QIQ_RUNTIME_EXCEPTION, "%s: the %s is null", function, what);
    return text;
}

// `length` bytes as lowercase hex, as a new string.
static char *qiq_hex(const unsigned char *bytes, size_t length) {
    static const char digits[] = "0123456789abcdef";
    char *text = qiq_string_new(length * 2);
    for (size_t index = 0; index < length; index++) {
        text[index * 2] = digits[bytes[index] >> 4];
        text[index * 2 + 1] = digits[bytes[index] & 0x0f];
    }
    return text;
}

static char *qiq_hash_hex(const char *function, const mbedtls_md_info_t *info, const char *text) {
    unsigned char digest[MBEDTLS_MD_MAX_SIZE];
    if (mbedtls_md(info, (const unsigned char *)text, strlen(text), digest) != 0) {
        qiq_throwf(QIQ_RUNTIME_EXCEPTION, "%s: hashing failed", function);
    }
    return qiq_hex(digest, mbedtls_md_get_size(info));
}

QIQ_GUARDED(char *, qiq_stdlib_md5, NULL, (const char *text), (text)) {
    return qiq_hash_hex("md5", qiq_digest("md5", "md5"), qiq_require_text("md5", text, "text"));
}

QIQ_GUARDED(char *, qiq_stdlib_sha1, NULL, (const char *text), (text)) {
    return qiq_hash_hex("sha1", qiq_digest("sha1", "sha1"), qiq_require_text("sha1", text, "text"));
}

QIQ_GUARDED(char *, qiq_stdlib_hash, NULL, (const char *algorithm, const char *text), (algorithm, text)) {
    const mbedtls_md_info_t *info = qiq_digest("hash", algorithm);
    return qiq_hash_hex("hash", info, qiq_require_text("hash", text, "text"));
}

QIQ_GUARDED(char *, qiq_stdlib_hash_hmac, NULL, (const char *algorithm, const char *text, const char *key),
            (algorithm, text, key)) {
    const mbedtls_md_info_t *info = qiq_digest("hash_hmac", algorithm);
    qiq_require_text("hash_hmac", text, "text");
    qiq_require_text("hash_hmac", key, "key");
    unsigned char digest[MBEDTLS_MD_MAX_SIZE];
    if (mbedtls_md_hmac(info, (const unsigned char *)key, strlen(key), (const unsigned char *)text, strlen(text), digest) != 0) {
        qiq_throw(QIQ_RUNTIME_EXCEPTION, "hash_hmac: hashing failed");
    }
    return qiq_hex(digest, mbedtls_md_get_size(info));
}

QIQ_GUARDED(void *, qiq_stdlib_hash_bytes, NULL, (const char *algorithm, void *data), (algorithm, data)) {
    const mbedtls_md_info_t *info = qiq_digest("hash_bytes", algorithm);
    if (data == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "hash_bytes: the array is null (declared without a value)");
    size_t size = mbedtls_md_get_size(info);
    void *digest = qiq_array_create((int64_t)size, 1, 0);
    if (mbedtls_md(info, QIQ_ARRAY_DATA(data), (size_t)QIQ_ARRAY_LENGTH(data), QIQ_ARRAY_DATA(digest)) != 0) {
        qiq_release(digest);
        qiq_throw(QIQ_RUNTIME_EXCEPTION, "hash_bytes: hashing failed");
    }
    return digest;
}

QIQ_GUARDED(char *, qiq_stdlib_bin2hex, NULL, (void *data), (data)) {
    if (data == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "bin2hex: the array is null (declared without a value)");
    return qiq_hex(QIQ_ARRAY_DATA(data), (size_t)QIQ_ARRAY_LENGTH(data));
}

// ---------------------------------------------------------------------------
// Base64 (RFC 4648). Encoding uses the standard alphabet with '=' padding,
// as PHP does. Decoding also accepts the URL-safe alphabet ('-' and '_'),
// skips whitespace (encoded data is often wrapped in lines), and doesn't
// require the padding, but throws InvalidArgumentException for anything
// else, where PHP silently drops what it can't read.

static char *qiq_base64_encode(const unsigned char *data, size_t length) {
    static const char alphabet[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    char *text = qiq_string_new((length + 2) / 3 * 4);
    char *out = text;
    size_t index = 0;
    for (; index + 3 <= length; index += 3) {
        uint32_t group = (uint32_t)data[index] << 16 | (uint32_t)data[index + 1] << 8 | data[index + 2];
        *out++ = alphabet[group >> 18];
        *out++ = alphabet[group >> 12 & 63];
        *out++ = alphabet[group >> 6 & 63];
        *out++ = alphabet[group & 63];
    }
    if (index < length) {
        uint32_t group = (uint32_t)data[index] << 16 | (index + 1 < length ? (uint32_t)data[index + 1] << 8 : 0);
        *out++ = alphabet[group >> 18];
        *out++ = alphabet[group >> 12 & 63];
        *out++ = index + 1 < length ? alphabet[group >> 6 & 63] : '=';
        *out++ = '=';
    }
    return text;
}

// The value of a base64 digit, or -1.
static int qiq_base64_digit(unsigned char c) {
    if (c >= 'A' && c <= 'Z') return c - 'A';
    if (c >= 'a' && c <= 'z') return c - 'a' + 26;
    if (c >= '0' && c <= '9') return c - '0' + 52;
    if (c == '+' || c == '-') return 62;
    if (c == '/' || c == '_') return 63;
    return -1;
}

static bool qiq_base64_space(unsigned char c) {
    return c == ' ' || c == '\t' || c == '\r' || c == '\n';
}

// Throws for `text` at byte `offset`, giving the position in characters.
_Noreturn static void qiq_base64_error(const char *function, const char *text, size_t offset, const char *problem) {
    size_t position = 0;
    for (size_t index = 0; index < offset; index++) {
        if (((unsigned char)text[index] & 0xc0) != 0x80) position++;
    }
    qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "%s: %s at position %zu", function, problem, position);
}

// Decodes `text` into a new byte array. Checks all of it before allocating,
// so invalid input throws without anything to free.
static void *qiq_base64_decode(const char *function, const char *text) {
    if (text == NULL) qiq_throwf(QIQ_RUNTIME_EXCEPTION, "%s: the text is null", function);
    size_t digits = 0;
    size_t padding = 0;
    for (size_t index = 0; text[index] != 0; index++) {
        unsigned char c = (unsigned char)text[index];
        if (qiq_base64_space(c)) continue;
        if (c == '=') {
            padding++;
            if (padding > 2 || digits % 4 < 2) qiq_base64_error(function, text, index, "unexpected '='");
            continue;
        }
        if (qiq_base64_digit(c) < 0) {
            qiq_base64_error(function, text, index, c < 0x80 ? "a character that isn't base64" : "a non-ASCII character");
        }
        if (padding > 0) qiq_base64_error(function, text, index, "data after the '=' padding");
        digits++;
    }
    if (digits % 4 == 1) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "%s: the text ends partway through a byte", function);
    if (padding > 0 && (digits + padding) % 4 != 0) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "%s: wrong '=' padding at the end", function);
    }
    size_t length = digits / 4 * 3 + (digits % 4 == 0 ? 0 : digits % 4 - 1);
    void *bytes = qiq_array_create((int64_t)length, 1, 0);
    unsigned char *out = QIQ_ARRAY_DATA(bytes);
    uint32_t accumulated = 0;
    int bits = 0;
    for (const unsigned char *cursor = (const unsigned char *)text; *cursor != 0 && *cursor != '='; cursor++) {
        if (qiq_base64_space(*cursor)) continue;
        accumulated = accumulated << 6 | (uint32_t)qiq_base64_digit(*cursor);
        bits += 6;
        if (bits >= 8) {
            bits -= 8;
            *out++ = (unsigned char)(accumulated >> bits);
        }
    }
    return bytes;
}

QIQ_GUARDED(char *, qiq_stdlib_base64_encode, NULL, (const char *text), (text)) {
    qiq_require_text("base64_encode", text, "text");
    return qiq_base64_encode((const unsigned char *)text, strlen(text));
}

QIQ_GUARDED(char *, qiq_stdlib_base64_encode_bytes, NULL, (void *data), (data)) {
    if (data == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "base64_encode_bytes: the array is null (declared without a value)");
    return qiq_base64_encode(QIQ_ARRAY_DATA(data), (size_t)QIQ_ARRAY_LENGTH(data));
}

QIQ_GUARDED(void *, qiq_stdlib_base64_decode_bytes, NULL, (const char *text), (text)) {
    return qiq_base64_decode("base64_decode_bytes", text);
}

// Whether `length` bytes are UTF-8 text a qiq string can hold: valid, and
// without NUL bytes.
static bool qiq_is_utf8_text(const unsigned char *bytes, size_t length) {
    size_t index = 0;
    while (index < length) {
        unsigned char c = bytes[index];
        if (c == 0) return false;
        if (c < 0x80) {
            index++;
            continue;
        }
        size_t width = (c & 0xe0) == 0xc0 ? 2 : (c & 0xf0) == 0xe0 ? 3 : (c & 0xf8) == 0xf0 ? 4 : 0;
        if (width == 0 || index + width > length) return false;
        for (size_t offset = 1; offset < width; offset++) {
            if ((bytes[index + offset] & 0xc0) != 0x80) return false;
        }
        index += width;
    }
    return true;
}

QIQ_GUARDED(char *, qiq_stdlib_base64_decode, NULL, (const char *text), (text)) {
    void *bytes = qiq_base64_decode("base64_decode", text);
    size_t length = (size_t)QIQ_ARRAY_LENGTH(bytes);
    const unsigned char *data = QIQ_ARRAY_DATA(bytes);
    if (!qiq_is_utf8_text(data, length)) {
        qiq_release(bytes);
        qiq_throw(QIQ_INVALID_UTF8_EXCEPTION, "base64_decode: the decoded bytes aren't UTF-8 text; use base64_decode_bytes for binary data");
    }
    char *result = qiq_string_new(length);
    memcpy(result, data, length);
    qiq_release(bytes);
    return result;
}
