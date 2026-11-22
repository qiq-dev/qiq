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

// `length` bytes as lowercase hex, as a new string: both digits of a byte
// from one lookup.
static char *qiq_hex(const unsigned char *bytes, size_t length) {
    static char pairs[256][2];
    static bool ready = false;
    if (!ready) {
        static const char digits[] = "0123456789abcdef";
        for (int byte = 0; byte < 256; byte++) {
            pairs[byte][0] = digits[byte >> 4];
            pairs[byte][1] = digits[byte & 0x0f];
        }
        ready = true;
    }
    char *text = qiq_string_new(length * 2);
    for (size_t index = 0; index < length; index++) memcpy(text + index * 2, pairs[bytes[index]], 2);
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
    // The two characters for each 12 bits, so three bytes take two lookups.
    static char pairs[4096][2];
    static bool ready = false;
    if (!ready) {
        for (int bits = 0; bits < 4096; bits++) {
            pairs[bits][0] = alphabet[bits >> 6];
            pairs[bits][1] = alphabet[bits & 63];
        }
        ready = true;
    }
    char *text = qiq_string_new((length + 2) / 3 * 4);
    char *out = text;
    size_t index = 0;
    for (; index + 3 <= length; index += 3) {
        uint32_t group = (uint32_t)data[index] << 16 | (uint32_t)data[index + 1] << 8 | data[index + 2];
        memcpy(out, pairs[group >> 12], 2);
        memcpy(out + 2, pairs[group & 0xfff], 2);
        out += 4;
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

// What each character is in base64 text: a digit's value (0 to 63, from
// either alphabet), or one of these.
#define QIQ_BASE64_INVALID (-1)
#define QIQ_BASE64_SPACE (-2)
#define QIQ_BASE64_PADDING (-3)

static const int8_t *qiq_base64_table(void) {
    static int8_t table[256];
    static bool ready = false;
    if (!ready) {
        memset(table, QIQ_BASE64_INVALID, sizeof table);
        const char *digits = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        for (int value = 0; value < 62; value++) table[(unsigned char)digits[value]] = (int8_t)value;
        table['+'] = table['-'] = 62;
        table['/'] = table['_'] = 63;
        table[' '] = table['\t'] = table['\r'] = table['\n'] = QIQ_BASE64_SPACE;
        table['='] = QIQ_BASE64_PADDING;
        ready = true;
    }
    return table;
}

// Throws for `text` at byte `offset`, giving the position in characters,
// after releasing `owner` (the output being decoded into).
_Noreturn static void qiq_base64_error(const char *function, const char *text, size_t offset, const char *problem, void *owner) {
    qiq_release(owner);
    size_t position = 0;
    for (size_t index = 0; index < offset; index++) {
        if (((unsigned char)text[index] & 0xc0) != 0x80) position++;
    }
    qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "%s: %s at position %zu", function, problem, position);
}

_Noreturn static void qiq_base64_fail(const char *function, const char *problem, void *owner) {
    qiq_release(owner);
    qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "%s: %s", function, problem);
}

// The most bytes `length` characters of base64 can decode to.
static size_t qiq_base64_capacity(size_t length) {
    return length / 4 * 3 + 2;
}

// Decodes the `length` characters of `text` into `out`, which has room for
// qiq_base64_capacity(length) bytes, and returns how many it wrote. Four
// digits at a time while there are no spaces or padding; one character at
// a time otherwise, which also checks the text. On an error, releases
// `owner` and throws.
static size_t qiq_base64_decode_into(const char *function, const char *text, size_t length, unsigned char *out, void *owner) {
    const int8_t *table = qiq_base64_table();
    const unsigned char *in = (const unsigned char *)text;
    const unsigned char *end = in + length;
    unsigned char *start = out;
    uint32_t group = 0;
    int digits = 0;      // in the current group of four
    int padding = 0;
    while (in < end) {
        if (digits == 0 && padding == 0) {
            while (end - in >= 4) {
                int a = table[in[0]], b = table[in[1]], c = table[in[2]], d = table[in[3]];
                if ((a | b | c | d) < 0) break;
                uint32_t bits = (uint32_t)a << 18 | (uint32_t)b << 12 | (uint32_t)c << 6 | (uint32_t)d;
                out[0] = (unsigned char)(bits >> 16);
                out[1] = (unsigned char)(bits >> 8);
                out[2] = (unsigned char)bits;
                out += 3;
                in += 4;
            }
            if (in == end) break;
        }
        int value = table[*in];
        if (value >= 0) {
            if (padding > 0) qiq_base64_error(function, text, (size_t)(in - (const unsigned char *)text), "data after the '=' padding", owner);
            group = group << 6 | (uint32_t)value;
            if (++digits == 4) {
                out[0] = (unsigned char)(group >> 16);
                out[1] = (unsigned char)(group >> 8);
                out[2] = (unsigned char)group;
                out += 3;
                digits = 0;
                group = 0;
            }
        } else if (value == QIQ_BASE64_PADDING) {
            if (++padding > 2 || digits < 2) qiq_base64_error(function, text, (size_t)(in - (const unsigned char *)text), "unexpected '='", owner);
        } else if (value != QIQ_BASE64_SPACE) {
            qiq_base64_error(function, text, (size_t)(in - (const unsigned char *)text),
                             *in < 0x80 ? "a character that isn't base64" : "a non-ASCII character", owner);
        }
        in++;
    }
    if (digits == 1) qiq_base64_fail(function, "the text ends partway through a byte", owner);
    if (padding > 0 && digits + padding != 4) qiq_base64_fail(function, "wrong '=' padding at the end", owner);
    // The last, partial group: 2 digits are 1 byte, 3 are 2.
    if (digits == 2) {
        *out++ = (unsigned char)(group >> 4);
    } else if (digits == 3) {
        *out++ = (unsigned char)(group >> 10);
        *out++ = (unsigned char)(group >> 2);
    }
    return (size_t)(out - start);
}

static void *qiq_base64_decode(const char *function, const char *text) {
    if (text == NULL) qiq_throwf(QIQ_RUNTIME_EXCEPTION, "%s: the text is null", function);
    size_t length = strlen(text);
    void *bytes = qiq_array_create((int64_t)qiq_base64_capacity(length), 1, 0);
    QIQ_ARRAY_LENGTH(bytes) = (int64_t)qiq_base64_decode_into(function, text, length, QIQ_ARRAY_DATA(bytes), bytes);
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
        // Eight bytes at a time while they're ASCII and not NUL (a word
        // with a 0 byte or a byte >= 0x80 has a high bit set here).
        while (index + 8 <= length) {
            uint64_t word;
            memcpy(&word, bytes + index, sizeof word);
            if (((word - UINT64_C(0x0101010101010101)) | word) & UINT64_C(0x8080808080808080)) break;
            index += 8;
        }
        if (index >= length) break;
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

// Decodes straight into the result string, which can be up to two bytes
// longer than the text it ends up holding.
QIQ_GUARDED(char *, qiq_stdlib_base64_decode, NULL, (const char *text), (text)) {
    qiq_require_text("base64_decode", text, "text");
    size_t length = strlen(text);
    char *result = qiq_string_new(qiq_base64_capacity(length));
    size_t decoded = qiq_base64_decode_into("base64_decode", text, length, (unsigned char *)result, result);
    if (!qiq_is_utf8_text((const unsigned char *)result, decoded)) {
        qiq_release(result);
        qiq_throw(QIQ_INVALID_UTF8_EXCEPTION, "base64_decode: the decoded bytes aren't UTF-8 text; use base64_decode_bytes for binary data");
    }
    result[decoded] = '\0';
    return result;
}
