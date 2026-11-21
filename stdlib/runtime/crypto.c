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
