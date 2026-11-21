// Strings: NUL-terminated UTF-8. Every position and length is counted in
// characters (Unicode scalar values), never bytes. Strings are immutable;
// functions that produce text return newly allocated strings.
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "qiq_runtime.h"

// ---------------------------------------------------------------------------
// UTF-8 helpers
// ---------------------------------------------------------------------------

// A new string with room for `size` bytes, including the NUL. Every
// allocation in this file is a result string.
static void *qiq_alloc(size_t size) {
    return qiq_string_new(size - 1);
}

static const char *qiq_require_string(const char *text) {
    if (text == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "string is null");
    return text;
}

static size_t qiq_utf8_width(unsigned char first) {
    if (first < 0x80) return 1;
    if ((first & 0xe0) == 0xc0) return 2;
    if ((first & 0xf0) == 0xe0) return 3;
    if ((first & 0xf8) == 0xf0) return 4;
    qiq_throw(QIQ_INVALID_UTF8_EXCEPTION, "invalid UTF-8 string");
    return 0;
}

// Returns the width of the character at `cursor`, validating its continuation bytes.
static size_t qiq_utf8_char_width(const unsigned char *cursor) {
    size_t width = qiq_utf8_width(*cursor);
    for (size_t offset = 1; offset < width; offset++) {
        if ((cursor[offset] & 0xc0) != 0x80) qiq_throw(QIQ_INVALID_UTF8_EXCEPTION, "invalid UTF-8 string");
    }
    return width;
}

static uint32_t qiq_utf8_decode(const unsigned char *cursor, size_t *width) {
    *width = qiq_utf8_char_width(cursor);
    switch (*width) {
    case 1: return cursor[0];
    case 2: return ((uint32_t)(cursor[0] & 0x1f) << 6) | (cursor[1] & 0x3f);
    case 3:
        return ((uint32_t)(cursor[0] & 0x0f) << 12) | ((uint32_t)(cursor[1] & 0x3f) << 6)
            | (cursor[2] & 0x3f);
    default:
        return ((uint32_t)(cursor[0] & 0x07) << 18) | ((uint32_t)(cursor[1] & 0x3f) << 12)
            | ((uint32_t)(cursor[2] & 0x3f) << 6) | (cursor[3] & 0x3f);
    }
}

static size_t qiq_utf8_encode(uint32_t code_point, char *out) {
    if (code_point < 0x80) {
        out[0] = (char)code_point;
        return 1;
    }
    if (code_point < 0x800) {
        out[0] = (char)(0xc0 | (code_point >> 6));
        out[1] = (char)(0x80 | (code_point & 0x3f));
        return 2;
    }
    if (code_point < 0x10000) {
        out[0] = (char)(0xe0 | (code_point >> 12));
        out[1] = (char)(0x80 | ((code_point >> 6) & 0x3f));
        out[2] = (char)(0x80 | (code_point & 0x3f));
        return 3;
    }
    out[0] = (char)(0xf0 | (code_point >> 18));
    out[1] = (char)(0x80 | ((code_point >> 12) & 0x3f));
    out[2] = (char)(0x80 | ((code_point >> 6) & 0x3f));
    out[3] = (char)(0x80 | (code_point & 0x3f));
    return 4;
}

// Text is mostly ASCII, and an ASCII byte is a whole character that needs no
// validating, so the functions below skip runs of ASCII 8 bytes at a time
// and decode only the other characters one by one: counting the characters
// of a 12 MB ASCII string takes about a tenth of the time.
typedef uint64_t __attribute__((__may_alias__)) qiq_word;
#define QIQ_ONES UINT64_C(0x0101010101010101)
#define QIQ_HIGHS UINT64_C(0x8080808080808080)

// Whether any byte of `word` is 0 or 0x80 and above (sometimes true for a
// byte after one of those, which only ends a run early).
static bool qiq_word_ends_ascii(uint64_t word) {
    return ((word - QIQ_ONES) | word) & QIQ_HIGHS;
}

// The number of ASCII bytes (1 to 0x7f) at the start of `text`. Reads whole
// aligned words, which never cross into another page, so reading past the
// terminating NUL within one is safe.
static size_t qiq_ascii_prefix(const unsigned char *text) {
    const unsigned char *cursor = text;
    while ((uintptr_t)cursor % sizeof(qiq_word) != 0) {
        if (*cursor == 0 || *cursor >= 0x80) return (size_t)(cursor - text);
        cursor++;
    }
    while (!qiq_word_ends_ascii(*(const qiq_word *)cursor)) cursor += sizeof(qiq_word);
    while (*cursor != 0 && *cursor < 0x80) cursor++;
    return (size_t)(cursor - text);
}

// Number of characters in `text`, validating it as UTF-8.
static int64_t qiq_char_count(const char *text) {
    const unsigned char *cursor = (const unsigned char *)text;
    int64_t count = 0;
    for (;;) {
        size_t ascii = qiq_ascii_prefix(cursor);
        cursor += ascii;
        count += (int64_t)ascii;
        if (*cursor == 0) return count;
        cursor += qiq_utf8_char_width(cursor);
        count++;
    }
}

// Number of characters in `text` before `end`, a character boundary in it,
// validating them as UTF-8.
static int64_t qiq_char_count_to(const char *text, const char *end) {
    const unsigned char *cursor = (const unsigned char *)text;
    int64_t count = 0;
    while (cursor < (const unsigned char *)end) {
        size_t ascii = qiq_ascii_prefix(cursor);
        if (cursor + ascii >= (const unsigned char *)end) return count + ((const unsigned char *)end - cursor);
        cursor += ascii;
        count += (int64_t)ascii;
        cursor += qiq_utf8_char_width(cursor);
        count++;
    }
    return count;
}

// Pointer to the character at index `index`, or to the terminating NUL when
// `index` is at or past the end.
static const char *qiq_char_offset(const char *text, int64_t index) {
    const unsigned char *cursor = (const unsigned char *)text;
    while (*cursor != 0 && index > 0) {
        size_t ascii = qiq_ascii_prefix(cursor);
        if ((int64_t)ascii >= index) return (const char *)cursor + index;
        cursor += ascii;
        index -= (int64_t)ascii;
        if (*cursor == 0) break;
        cursor += qiq_utf8_char_width(cursor);
        index--;
    }
    return (const char *)cursor;
}

static char *qiq_copy_bytes(const char *start, size_t length) {
    char *copy = qiq_alloc(length + 1);
    memcpy(copy, start, length);
    copy[length] = 0;
    return copy;
}

// ---------------------------------------------------------------------------
// Used by compiled code (foreach over strings, `+` concatenation)
// ---------------------------------------------------------------------------

QIQ_GUARDED(uint64_t, qiq_string_len, 0, (const char *text), (text)) {
    if (text == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "cannot iterate a null string");
    return (uint64_t)qiq_char_count(text);
}

// `==` and `!=` on strings and chars: 0 when equal. Null equals only null.
int32_t qiq_string_equality(const char *left, const char *right) {
    if (left == NULL || right == NULL) return left == right ? 0 : 1;
    return strcmp(left, right) == 0 ? 0 : 1;
}

// `<`, `>`, `<=`, `>=` on strings and chars, by byte (code point) order.
QIQ_GUARDED(int32_t, qiq_string_compare, 0, (const char *left, const char *right), (left, right)) {
    if (left == NULL || right == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "cannot order a null string");
    int result = strcmp(left, right);
    return result < 0 ? -1 : result > 0;
}

// len() of a string: the number of characters.
QIQ_GUARDED(int64_t, qiq_len_string, 0, (const char *text), (text)) {
    if (text == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "len() of a null string");
    return (int64_t)qiq_char_count(text);
}

QIQ_GUARDED(char *, qiq_string_char_at, NULL, (const char *text, uint64_t ordinal), (text, ordinal)) {
    if (text == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "cannot iterate a null string");
    const char *start = qiq_char_offset(text, (int64_t)ordinal);
    if (*start == 0) qiq_throw(QIQ_RUNTIME_EXCEPTION, "string position out of range");
    return qiq_copy_bytes(start, qiq_utf8_char_width((const unsigned char *)start));
}

QIQ_GUARDED(char *, qiq_string_concat, NULL, (const char *left, const char *right), (left, right)) {
    qiq_require_string(left);
    qiq_require_string(right);
    size_t left_length = strlen(left);
    size_t right_length = strlen(right);
    char *result = qiq_alloc(left_length + right_length + 1);
    memcpy(result, left, left_length);
    memcpy(result + left_length, right, right_length + 1);
    return result;
}

// ---------------------------------------------------------------------------
// Case mapping. Simple one-to-one mappings for ASCII, Latin-1, Latin
// Extended-A, basic Greek, and basic Cyrillic. Other characters are unchanged.
// ---------------------------------------------------------------------------

// In Latin Extended-A, most pairs are (even upper, odd lower); these two ranges
// are (odd upper, even lower).
static bool qiq_latin_ext_a_odd_upper(uint32_t c) {
    return (c >= 0x139 && c <= 0x148) || (c >= 0x179 && c <= 0x17e);
}

static uint32_t qiq_to_upper(uint32_t c) {
    if (c >= 'a' && c <= 'z') return c - 0x20;
    if (c >= 0xe0 && c <= 0xfe && c != 0xf7) return c - 0x20;
    if (c == 0xff) return 0x178;
    if (c >= 0x100 && c <= 0x17f) {
        if (c == 0x131) return 'I';
        if (c == 0x17f) return 'S';
        if (c == 0x130 || c == 0x138 || c == 0x149 || c == 0x178) return c;
        if (qiq_latin_ext_a_odd_upper(c)) return (c % 2 == 0) ? c - 1 : c;
        return (c % 2 == 1) ? c - 1 : c;
    }
    if (c == 0x3c2) return 0x3a3;
    if (c >= 0x3b1 && c <= 0x3c9) return c - 0x20;
    if (c >= 0x430 && c <= 0x44f) return c - 0x20;
    if (c >= 0x450 && c <= 0x45f) return c - 0x50;
    return c;
}

static uint32_t qiq_to_lower(uint32_t c) {
    if (c >= 'A' && c <= 'Z') return c + 0x20;
    if (c >= 0xc0 && c <= 0xde && c != 0xd7) return c + 0x20;
    if (c == 0x178) return 0xff;
    if (c >= 0x100 && c <= 0x17f) {
        if (c == 0x130) return 'i';
        if (c == 0x131 || c == 0x138 || c == 0x149 || c == 0x17f) return c;
        if (qiq_latin_ext_a_odd_upper(c)) return (c % 2 == 1) ? c + 1 : c;
        return (c % 2 == 0) ? c + 1 : c;
    }
    if (c >= 0x391 && c <= 0x3a9 && c != 0x3a2) return c + 0x20;
    if (c >= 0x410 && c <= 0x42f) return c + 0x20;
    if (c >= 0x400 && c <= 0x40f) return c + 0x50;
    return c;
}

// Eight ASCII bytes with those from `from` to `to` changed by 0x20 (case):
// a byte's high bit is set by adding 0x80 - from when it's >= from, and by
// adding 0x7f - to when it's > to; the bytes with exactly one of the two
// get 0x20 flipped. No byte carries into the next, since all are below 0x80.
static uint64_t qiq_flip_case(uint64_t word, unsigned char from, unsigned char to) {
    uint64_t at_least_from = word + (0x80 - from) * QIQ_ONES;
    uint64_t above_to = word + (0x7f - to) * QIQ_ONES;
    return word ^ (((at_least_from ^ above_to) & QIQ_HIGHS) >> 2);
}

// Maps each character of `text` (or only the first, when `first_only`).
// No mapping makes a character longer in UTF-8 (each maps within ASCII, or
// within two-byte characters, or from two bytes to one), so the result fits
// in the length of `text`.
static char *qiq_map_case(const char *text, uint32_t (*map)(uint32_t), bool first_only) {
    qiq_require_string(text);
    size_t length = strlen(text);
    char *result = qiq_alloc(length + 1);
    const unsigned char *cursor = (const unsigned char *)text;
    char *out = result;
    if (first_only) {
        if (*cursor != 0) {
            size_t width;
            uint32_t code_point = qiq_utf8_decode(cursor, &width);
            out += qiq_utf8_encode(map(code_point), out);
            cursor += width;
        }
        // The rest is copied after validating it, as mapping it would.
        qiq_char_count((const char *)cursor);
        size_t rest = length - (size_t)(cursor - (const unsigned char *)text);
        memcpy(out, cursor, rest + 1);
        return result;
    }
    unsigned char from = map('a') == 'A' ? 'a' : 'A';
    unsigned char to = from + 25;
    while (*cursor != 0) {
        // Whole words of ASCII at a time (read unaligned; the string's
        // length says there are 8 more bytes).
        while (length - (size_t)(cursor - (const unsigned char *)text) >= sizeof(uint64_t)) {
            uint64_t word;
            memcpy(&word, cursor, sizeof word);
            if (word & QIQ_HIGHS) break;
            word = qiq_flip_case(word, from, to);
            memcpy(out, &word, sizeof word);
            out += sizeof word;
            cursor += sizeof word;
        }
        if (*cursor == 0) break;
        if (*cursor < 0x80) {
            *out++ = (char)(*cursor >= from && *cursor <= to ? *cursor ^ 0x20 : *cursor);
            cursor++;
            continue;
        }
        size_t width;
        uint32_t code_point = qiq_utf8_decode(cursor, &width);
        out += qiq_utf8_encode(map(code_point), out);
        cursor += width;
    }
    *out = 0;
    return result;
}

// ---------------------------------------------------------------------------
// Standard library functions (declared with `extern function` in stdlib/string.qiq)
// ---------------------------------------------------------------------------

// PHP semantics, in characters: a negative start counts from the end; a
// negative length leaves that many characters off the end.
QIQ_GUARDED(char *, qiq_stdlib_substr, NULL, (const char *text, int64_t start, int64_t length), (text, start, length)) {
    qiq_require_string(text);
    int64_t count = qiq_char_count(text);
    int64_t from = start;
    if (from < 0) from = count + from < 0 ? 0 : count + from;
    if (from > count) from = count;
    // from + length can only overflow when length is huge, which means "to the end".
    int64_t to = length < 0 ? count + length : (length > count ? count : from + length);
    if (to > count) to = count;
    if (to <= from) return qiq_copy_bytes("", 0);
    const char *begin = qiq_char_offset(text, from);
    const char *end = qiq_char_offset(begin, to - from);
    return qiq_copy_bytes(begin, (size_t)(end - begin));
}

// Character index of the first occurrence of `needle` at or after `offset`,
// or -1 when absent. A negative offset counts from the end (PHP semantics).
QIQ_GUARDED(int64_t, qiq_stdlib_strpos, 0, (const char *haystack, const char *needle, int64_t offset), (haystack, needle, offset)) {
    qiq_require_string(haystack);
    qiq_require_string(needle);
    qiq_char_count(needle);
    int64_t count = qiq_char_count(haystack);
    int64_t from = offset < 0 ? count + offset : offset;
    if (from < 0 || from > count) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "strpos offset is outside the string");
    const char *start = qiq_char_offset(haystack, from);
    const char *found = strstr(start, needle);
    if (found == NULL) return -1;
    // Characters before the match: those before `start`, then the rest.
    return from + qiq_char_count_to(start, found);
}

QIQ_GUARDED(bool, qiq_stdlib_str_contains, false, (const char *haystack, const char *needle), (haystack, needle)) {
    return strstr(qiq_require_string(haystack), qiq_require_string(needle)) != NULL;
}

QIQ_GUARDED(bool, qiq_stdlib_str_ends_with, false, (const char *haystack, const char *needle), (haystack, needle)) {
    size_t haystack_length = strlen(qiq_require_string(haystack));
    size_t needle_length = strlen(qiq_require_string(needle));
    return needle_length <= haystack_length
        && memcmp(haystack + haystack_length - needle_length, needle, needle_length) == 0;
}

// Replaces every occurrence of `search` in `subject` (PHP argument order).
// Matching whole UTF-8 sequences byte-for-byte never splits a character.
QIQ_GUARDED(char *, qiq_stdlib_str_replace, NULL, (const char *search, const char *replace, const char *subject), (search, replace, subject)) {
    qiq_require_string(search);
    qiq_require_string(replace);
    qiq_require_string(subject);
    size_t search_length = strlen(search);
    if (search_length == 0) return qiq_copy_bytes(subject, strlen(subject));
    size_t replace_length = strlen(replace);
    size_t matches = 0;
    for (const char *at = strstr(subject, search); at != NULL; at = strstr(at + search_length, search)) {
        matches++;
    }
    size_t result_length = strlen(subject) + matches * replace_length - matches * search_length;
    char *result = qiq_alloc(result_length + 1);
    char *out = result;
    const char *rest = subject;
    for (const char *at = strstr(rest, search); at != NULL; at = strstr(rest, search)) {
        memcpy(out, rest, (size_t)(at - rest));
        out += at - rest;
        memcpy(out, replace, replace_length);
        out += replace_length;
        rest = at + search_length;
    }
    strcpy(out, rest);
    return result;
}

QIQ_GUARDED(char *, qiq_stdlib_str_repeat, NULL, (const char *text, int64_t times), (text, times)) {
    qiq_require_string(text);
    if (times < 0) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "str_repeat count must not be negative");
    size_t length = strlen(text);
    char *result = qiq_alloc(length * (size_t)times + 1);
    for (int64_t index = 0; index < times; index++) {
        memcpy(result + length * (size_t)index, text, length);
    }
    result[length * (size_t)times] = 0;
    return result;
}

QIQ_GUARDED(char *, qiq_stdlib_strtoupper, NULL, (const char *text), (text)) {
    return qiq_map_case(text, qiq_to_upper, false);
}

QIQ_GUARDED(char *, qiq_stdlib_strtolower, NULL, (const char *text), (text)) {
    return qiq_map_case(text, qiq_to_lower, false);
}

QIQ_GUARDED(char *, qiq_stdlib_ucfirst, NULL, (const char *text), (text)) {
    return qiq_map_case(text, qiq_to_upper, true);
}

// Reverses characters (not bytes), so multi-byte characters stay intact.
QIQ_GUARDED(char *, qiq_stdlib_strrev, NULL, (const char *text), (text)) {
    qiq_require_string(text);
    size_t length = strlen(text);
    char *result = qiq_alloc(length + 1);
    const unsigned char *cursor = (const unsigned char *)text;
    char *out = result + length;
    while (*cursor != 0) {
        size_t width = qiq_utf8_char_width(cursor);
        out -= width;
        memcpy(out, cursor, width);
        cursor += width;
    }
    result[length] = 0;
    return result;
}

// Whether the UTF-8 character `character` (`width` bytes) occurs in `set`.
static bool qiq_char_in_set(const char *character, size_t width, const char *set) {
    const unsigned char *cursor = (const unsigned char *)set;
    while (*cursor != 0) {
        size_t set_width = qiq_utf8_char_width(cursor);
        if (set_width == width && memcmp(cursor, character, width) == 0) return true;
        cursor += set_width;
    }
    return false;
}

// Strips leading and trailing characters that occur in `characters` (whole
// UTF-8 characters, so "é" strips é and nothing else).
QIQ_GUARDED(char *, qiq_stdlib_trim, NULL, (const char *text, const char *characters), (text, characters)) {
    qiq_require_string(text);
    qiq_require_string(characters);
    qiq_char_count(text);
    qiq_char_count(characters);
    const char *start = text;
    while (*start != 0) {
        size_t width = qiq_utf8_char_width((const unsigned char *)start);
        if (!qiq_char_in_set(start, width, characters)) break;
        start += width;
    }
    const char *end = start + strlen(start);
    while (end > start) {
        // Step back to the first byte of the last character.
        const char *last = end - 1;
        while (last > start && ((unsigned char)*last & 0xc0) == 0x80) last--;
        if (!qiq_char_in_set(last, (size_t)(end - last), characters)) break;
        end = last;
    }
    return qiq_copy_bytes(start, (size_t)(end - start));
}

// Splits `text` at each `separator` (PHP semantics). A positive `limit`
// returns at most that many pieces, the last holding the rest of the text; a
// negative `limit` drops that many pieces from the end; 0 counts as 1.
QIQ_GUARDED(void *, qiq_stdlib_explode, NULL, (const char *separator, const char *text, int64_t limit), (separator, text, limit)) {
    qiq_require_string(separator);
    qiq_require_string(text);
    qiq_char_count(separator);
    qiq_char_count(text);
    size_t separator_length = strlen(separator);
    if (separator_length == 0) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "explode separator must not be empty");
    int64_t total = 1;
    for (const char *at = strstr(text, separator); at != NULL; at = strstr(at + separator_length, separator)) {
        total++;
    }
    int64_t count = total;
    if (limit == 0) limit = 1;
    if (limit > 0 && limit < total) count = limit;
    if (limit < 0) count = total + limit > 0 ? total + limit : 0;
    void *pieces = qiq_array_create(count, sizeof(char *), 1);
    char **data = QIQ_ARRAY_DATA(pieces);
    const char *start = text;
    for (int64_t index = 0; index < count; index++) {
        // With a positive limit, the last piece keeps the rest of the text.
        bool rest = limit > 0 && index == count - 1;
        const char *end = rest ? NULL : strstr(start, separator);
        size_t length = end == NULL ? strlen(start) : (size_t)(end - start);
        data[index] = qiq_copy_bytes(start, length);
        start = end == NULL ? start + length : end + separator_length;
    }
    return pieces;
}

// Joins the strings in `pieces` with `separator` between them.
QIQ_GUARDED(char *, qiq_stdlib_implode, NULL, (const char *separator, void *pieces), (separator, pieces)) {
    qiq_require_string(separator);
    if (pieces == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "array is null (declared without a value)");
    int64_t count = QIQ_ARRAY_LENGTH(pieces);
    char **data = QIQ_ARRAY_DATA(pieces);
    size_t separator_length = strlen(separator);
    size_t length = 0;
    for (int64_t index = 0; index < count; index++) {
        if (data[index] == NULL) qiq_throwf(QIQ_RUNTIME_EXCEPTION, "implode: element %lld is null", (long long)index);
        length += strlen(data[index]) + (index > 0 ? separator_length : 0);
    }
    char *result = qiq_alloc(length + 1);
    char *out = result;
    for (int64_t index = 0; index < count; index++) {
        if (index > 0) {
            memcpy(out, separator, separator_length);
            out += separator_length;
        }
        size_t piece_length = strlen(data[index]);
        memcpy(out, data[index], piece_length);
        out += piece_length;
    }
    *out = 0;
    return result;
}

// The string's UTF-8 encoding.
QIQ_GUARDED(void *, qiq_stdlib_string_to_bytes, NULL, (const char *text), (text)) {
    qiq_require_string(text);
    size_t length = strlen(text);
    void *bytes = qiq_array_create((int64_t)length, 1, 0);
    memcpy(QIQ_ARRAY_DATA(bytes), text, length);
    return bytes;
}

// Decodes UTF-8 bytes into a string; throws InvalidUtf8Exception when they
// aren't valid UTF-8 or contain a NUL byte.
QIQ_GUARDED(char *, qiq_stdlib_bytes_to_string, NULL, (void *bytes), (bytes)) {
    if (bytes == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "array is null (declared without a value)");
    size_t length = (size_t)QIQ_ARRAY_LENGTH(bytes);
    const char *data = QIQ_ARRAY_DATA(bytes);
    if (memchr(data, 0, length) != NULL) {
        qiq_throw(QIQ_INVALID_UTF8_EXCEPTION, "bytes contain a NUL byte, which qiq strings cannot hold");
    }
    char *text = qiq_copy_bytes(data, length);
    qiq_char_count(text);
    return text;
}

// ---------------------------------------------------------------------------
// sprintf. A port of PHP's formatted printing: %[argnum$][flags][width]
// [.precision]specifier, with flags '-' (left-justify), '+' (always show a
// sign), '0' or ' ' (padding), and '<char> (any padding character). Widths
// and string precisions count characters, like everything in qiq; numbers
// are formatted exactly as PHP formats them.
//
// The compiler passes the arguments as 64-bit payloads in `values`, with one
// character per argument in `types`: i int, d double, b bool, y byte,
// s string, c char.
// ---------------------------------------------------------------------------

typedef struct {
    char *data;
    size_t length;
    size_t capacity;
} QiqText;

static void qiq_text_reserve(QiqText *text, size_t extra) {
    if (text->length + extra + 1 <= text->capacity) return;
    size_t capacity = text->capacity ? text->capacity : 64;
    while (text->length + extra + 1 > capacity) capacity *= 2;
    char *grown = realloc(text->data, capacity);
    if (grown == NULL) qiq_fatal("out of memory");
    text->data = grown;
    text->capacity = capacity;
}

static void qiq_text_add(QiqText *text, const char *bytes, size_t length) {
    qiq_text_reserve(text, length);
    memcpy(text->data + text->length, bytes, length);
    text->length += length;
}

typedef struct {
    const char *types;
    const uint64_t *values;
    int64_t count;
    QiqText *output;
} QiqFormatArguments;

_Noreturn static void qiq_format_error(QiqText *output, const char *message) {
    free(output->data);
    qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, message);
}

static const char *qiq_argument_text(const QiqFormatArguments *arguments, int64_t index) {
    const char *text = (const char *)(uintptr_t)arguments->values[index];
    if (text == NULL) {
        free(arguments->output->data);
        qiq_throwf(QIQ_RUNTIME_EXCEPTION, "sprintf: argument %lld is null", (long long)index + 2);
    }
    return text;
}

// A string as a number, as PHP reads one: leading whitespace, then the
// longest numeric prefix ("12abc" is 12, "abc" is 0).
static double qiq_text_to_double(const char *text) {
    while (*text == ' ' || *text == '\t' || *text == '\n' || *text == '\r' || *text == '\v' || *text == '\f') text++;
    char *end;
    double value = strtod(text, &end);
    return end == text ? 0.0 : value;
}

// A double as an int, as PHP converts one: truncated, and wrapped modulo
// 2^64 when out of range; NaN and infinities give 0.
static int64_t qiq_double_to_long(double value) {
    if (!isfinite(value)) return 0;
    if (value >= -9223372036854775808.0 && value < 9223372036854775808.0) return (int64_t)value;
    double wrapped = fmod(trunc(value), 18446744073709551616.0);
    if (wrapped < 0) wrapped += 18446744073709551616.0;
    return (int64_t)(uint64_t)wrapped;
}

static int64_t qiq_argument_long(const QiqFormatArguments *arguments, int64_t index) {
    uint64_t bits = arguments->values[index];
    switch (arguments->types[index]) {
    case 'i': return (int64_t)bits;
    case 'y': return (int64_t)(bits & 0xff);
    case 'b': return bits != 0;
    case 'd': {
        double value;
        memcpy(&value, &bits, sizeof value);
        return qiq_double_to_long(value);
    }
    default: {
        const char *text = qiq_argument_text(arguments, index);
        const char *cursor = text;
        while (*cursor == ' ' || *cursor == '\t' || *cursor == '\n' || *cursor == '\r' || *cursor == '\v' || *cursor == '\f') cursor++;
        const char *digits = cursor + (*cursor == '+' || *cursor == '-');
        while (*digits >= '0' && *digits <= '9') digits++;
        if (*digits == '.' || *digits == 'e' || *digits == 'E') return qiq_double_to_long(qiq_text_to_double(text));
        return (int64_t)strtoll(cursor, NULL, 10);
    }
    }
}

static double qiq_argument_double(const QiqFormatArguments *arguments, int64_t index) {
    uint64_t bits = arguments->values[index];
    switch (arguments->types[index]) {
    case 'd': {
        double value;
        memcpy(&value, &bits, sizeof value);
        return value;
    }
    case 's':
    case 'c':
        return qiq_text_to_double(qiq_argument_text(arguments, index));
    default:
        return (double)qiq_argument_long(arguments, index);
    }
}

// The argument as text, as qiq prints it: bools as 1 and 0, doubles like
// to_string. `buffer` holds converted numbers.
static const char *qiq_argument_string(const QiqFormatArguments *arguments, int64_t index, char *buffer, size_t size) {
    uint64_t bits = arguments->values[index];
    switch (arguments->types[index]) {
    case 's':
    case 'c':
        return qiq_argument_text(arguments, index);
    case 'd': {
        double value;
        memcpy(&value, &bits, sizeof value);
        snprintf(buffer, size, "%g", value);
        return buffer;
    }
    default:
        // Every caller's buffer holds at least QIQ_INT_TEXT_SIZE bytes.
        qiq_format_int(buffer, qiq_argument_long(arguments, index));
        return buffer;
    }
}

// PHP's php_sprintf_appendstring: `text` padded to `min_width` characters,
// cut to `max_chars` characters when `max_chars` >= 0. With '0' padding on
// the right, a sign goes before the zeros.
static void qiq_append_padded(QiqText *output, const char *text, int64_t min_width, int64_t max_chars,
                              const char *padding, size_t padding_length, bool left, bool sign_first) {
    size_t byte_length = strlen(text);
    int64_t chars = qiq_char_count(text);
    if (max_chars >= 0 && max_chars < chars) {
        byte_length = (size_t)(qiq_char_offset(text, max_chars) - text);
        chars = max_chars;
    }
    int64_t pad = min_width > chars ? min_width - chars : 0;
    qiq_text_reserve(output, byte_length + (size_t)pad * padding_length);
    if (!left) {
        if (sign_first && padding_length == 1 && padding[0] == '0' && byte_length > 0) {
            qiq_text_add(output, text, 1);
            text++;
            byte_length--;
        }
        for (int64_t index = 0; index < pad; index++) qiq_text_add(output, padding, padding_length);
    }
    qiq_text_add(output, text, byte_length);
    if (left) {
        for (int64_t index = 0; index < pad; index++) qiq_text_add(output, padding, padding_length);
    }
}

// Removes leading zeros from the exponent of C's %e output ("e+03" -> "e+3"),
// as PHP writes it.
static void qiq_trim_exponent(char *number) {
    char *exponent = strpbrk(number, "eE");
    if (exponent == NULL) return;
    char *digits = exponent + 2;
    char *first = digits;
    while (first[0] == '0' && first[1] != '\0') first++;
    memmove(digits, first, strlen(first) + 1);
}

// PHP's php_gcvt: `precision` significant digits, in plain notation unless
// the exponent is below -4 or at least `precision`.
static void qiq_format_general(char *out, size_t size, double value, int precision, char exponent_char) {
    char scientific[96];
    snprintf(scientific, sizeof scientific, "%.*e", precision - 1, fabs(value));
    char digits[96];
    size_t count = 0;
    char *cursor = scientific;
    for (; *cursor != 'e'; cursor++) {
        if (*cursor >= '0' && *cursor <= '9') digits[count++] = *cursor;
    }
    while (count > 1 && digits[count - 1] == '0') count--;
    digits[count] = '\0';
    int decimal_point = atoi(cursor + 1) + 1;
    if (value == 0.0) decimal_point = 1;
    char *dst = out;
    char *end = out + size - 1;
    if (signbit(value)) *dst++ = '-';
    if (decimal_point < 0 ? decimal_point < -3 : decimal_point > precision) {
        int exponent = decimal_point - 1;
        *dst++ = digits[0];
        *dst++ = '.';
        if (count == 1) {
            *dst++ = '0';
        } else {
            for (size_t index = 1; index < count && dst < end; index++) *dst++ = digits[index];
        }
        snprintf(dst, (size_t)(end - dst) + 1, "%c%c%d", exponent_char, exponent < 0 ? '-' : '+', exponent < 0 ? -exponent : exponent);
        return;
    }
    if (decimal_point <= 0) {
        *dst++ = '0';
        *dst++ = '.';
        for (int index = decimal_point; index < 0 && dst < end; index++) *dst++ = '0';
        for (size_t index = 0; index < count && dst < end; index++) *dst++ = digits[index];
        *dst = '\0';
        return;
    }
    for (int index = 0; index < decimal_point && dst < end; index++) {
        *dst++ = (size_t)index < count ? digits[index] : '0';
    }
    if ((size_t)decimal_point < count) {
        *dst++ = '.';
        for (size_t index = (size_t)decimal_point; index < count && dst < end; index++) *dst++ = digits[index];
    }
    *dst = '\0';
}

// Reads a width, precision, or argument number.
static int64_t qiq_format_number(const char **cursor, QiqText *output, const char *what) {
    int64_t value = 0;
    while (**cursor >= '0' && **cursor <= '9') {
        value = value * 10 + (**cursor - '0');
        if (value >= 2147483647) {
            char message[96];
            snprintf(message, sizeof message, "%s must be greater than or equal to zero and less than 2147483647", what);
            qiq_format_error(output, message);
        }
        (*cursor)++;
    }
    return value;
}

QIQ_GUARDED(char *, qiq_sprintf, NULL, (const char *format, const char *types, const uint64_t *values, int64_t count),
            (format, types, values, count)) {
    qiq_require_string(format);
    QiqText output = {0};
    QiqFormatArguments arguments = {types, values, count, &output};
    int64_t next_argument = 0;
    int64_t required = 0;
    const char *cursor = format;
    while (*cursor != '\0') {
        if (*cursor != '%') {
            const char *start = cursor;
            while (*cursor != '\0' && *cursor != '%') cursor++;
            qiq_text_add(&output, start, (size_t)(cursor - start));
            continue;
        }
        cursor++;
        if (*cursor == '%') {
            qiq_text_add(&output, "%", 1);
            cursor++;
            continue;
        }
        if (*cursor == '\0') qiq_format_error(&output, "Missing format specifier at end of string");

        bool left = false;
        bool always_sign = false;
        char padding_buffer[4] = " ";
        const char *padding = padding_buffer;
        size_t padding_length = 1;
        int64_t width = 0;
        int64_t precision = -1;
        int64_t argument;
        bool letter = (*cursor >= 'a' && *cursor <= 'z') || (*cursor >= 'A' && *cursor <= 'Z');
        if (!letter) {
            const char *digits = cursor;
            while (*digits >= '0' && *digits <= '9') digits++;
            if (*digits == '$' && digits > cursor) {
                argument = qiq_format_number(&cursor, &output, "Argument number specifier");
                if (argument <= 0) {
                    qiq_format_error(&output, "Argument number specifier must be greater than zero and less than 2147483647");
                }
                argument--;
                cursor++;
            } else if (*digits == '$') {
                qiq_format_error(&output, "Argument number specifier must be greater than zero and less than 2147483647");
            } else {
                argument = next_argument++;
            }
            for (;; cursor++) {
                if (*cursor == ' ' || *cursor == '0') {
                    padding_buffer[0] = *cursor;
                    padding_buffer[1] = '\0';
                    padding = padding_buffer;
                    padding_length = 1;
                } else if (*cursor == '-') {
                    left = true;
                } else if (*cursor == '+') {
                    always_sign = true;
                } else if (*cursor == '\'') {
                    if (cursor[1] == '\0') qiq_format_error(&output, "Missing padding character");
                    padding = cursor + 1;
                    padding_length = qiq_utf8_char_width((const unsigned char *)padding);
                    cursor += padding_length;
                } else {
                    break;
                }
            }
            if (*cursor >= '0' && *cursor <= '9') width = qiq_format_number(&cursor, &output, "Width");
            if (*cursor == '.') {
                cursor++;
                precision = *cursor >= '0' && *cursor <= '9' ? qiq_format_number(&cursor, &output, "Precision") : 0;
            }
        } else {
            argument = next_argument++;
        }
        if (*cursor == 'l') cursor++;
        char specifier = *cursor;
        if (specifier == '\0') qiq_format_error(&output, "Missing format specifier at end of string");
        if (strchr("bcdeEfFgGosuxX", specifier) == NULL) {
            size_t width_of = qiq_utf8_char_width((const unsigned char *)cursor);
            char message[64];
            snprintf(message, sizeof message, "Unknown format specifier \"%.*s\"", (int)width_of, cursor);
            qiq_format_error(&output, message);
        }
        cursor++;
        if (argument + 1 > required) required = argument + 1;
        if (argument >= count) {
            // Finish reading the format first, to report every missing argument.
            continue;
        }

        char number[512];
        // As in PHP 8, left-justified integers pad with spaces even with the
        // '0' flag (floating-point formats still pad with zeros).
        if (left && padding_length == 1 && padding[0] == '0' && strchr("du", specifier) != NULL) {
            padding = " ";
        }
        switch (specifier) {
        case 's': {
            char converted[64];
            const char *text = qiq_argument_string(&arguments, argument, converted, sizeof converted);
            qiq_append_padded(&output, text, width, precision, padding, padding_length, left, false);
            break;
        }
        case 'd': {
            int64_t value = qiq_argument_long(&arguments, argument);
            int sign = value >= 0 && always_sign;
            number[0] = '+';
            qiq_format_int(number + sign, value);
            qiq_append_padded(&output, number, width, -1, padding, padding_length, left, value < 0 || always_sign);
            break;
        }
        case 'u':
            snprintf(number, sizeof number, "%llu", (unsigned long long)qiq_argument_long(&arguments, argument));
            qiq_append_padded(&output, number, width, -1, padding, padding_length, left, false);
            break;
        case 'x':
        case 'X':
        case 'o':
        case 'b': {
            uint64_t value = (uint64_t)qiq_argument_long(&arguments, argument);
            if (specifier == 'b') {
                char bits[65];
                int length = 0;
                do {
                    bits[64 - ++length] = (char)('0' + (value & 1));
                    value >>= 1;
                } while (value != 0);
                bits[64] = '\0';
                snprintf(number, sizeof number, "%s", bits + 64 - length);
            } else {
                snprintf(number, sizeof number, specifier == 'x' ? "%llx" : specifier == 'X' ? "%llX" : "%llo",
                         (unsigned long long)value);
            }
            qiq_append_padded(&output, number, width, -1, padding, padding_length, left, false);
            break;
        }
        case 'c': {
            // A Unicode code point, written as UTF-8. Width and padding don't apply.
            int64_t code = qiq_argument_long(&arguments, argument);
            if (code <= 0 || code > 0x10ffff || (code >= 0xd800 && code <= 0xdfff)) {
                free(output.data);
                qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "sprintf: %lld is not a Unicode code point", (long long)code);
            }
            char encoded[4];
            size_t length = code < 0x80 ? 1 : code < 0x800 ? 2 : code < 0x10000 ? 3 : 4;
            if (length == 1) {
                encoded[0] = (char)code;
            } else {
                for (size_t index = length - 1; index > 0; index--) {
                    encoded[index] = (char)(0x80 | (code & 0x3f));
                    code >>= 6;
                }
                encoded[0] = (char)((length == 2 ? 0xc0 : length == 3 ? 0xe0 : 0xf0) | code);
            }
            qiq_text_add(&output, encoded, length);
            break;
        }
        default: {
            // e E f F g G
            double value = qiq_argument_double(&arguments, argument);
            int digits = precision < 0 ? 6 : precision > 53 ? 53 : (int)precision;
            if (isnan(value)) {
                qiq_append_padded(&output, "NaN", 3, -1, padding, padding_length, left, false);
                break;
            }
            if (isinf(value)) {
                const char *infinity = value < 0 ? "-Inf" : always_sign ? "+Inf" : "Inf";
                qiq_append_padded(&output, infinity, width, -1, padding, padding_length, left, value < 0 || always_sign);
                break;
            }
            char *body = number + 1;
            size_t body_size = sizeof number - 1;
            // PHP writes negative zero as zero in fixed and exponent notation.
            if (value == 0.0 && specifier != 'g' && specifier != 'G') value = 0.0;
            if (specifier == 'f' || specifier == 'F') {
                snprintf(body, body_size, "%.*f", digits, value);
            } else if (specifier == 'e' || specifier == 'E') {
                snprintf(body, body_size, specifier == 'e' ? "%.*e" : "%.*E", digits, value);
                qiq_trim_exponent(body);
            } else {
                qiq_format_general(body, body_size, value, digits == 0 ? 1 : digits, specifier == 'G' ? 'E' : 'e');
            }
            bool negative = body[0] == '-';
            if (!negative && always_sign) {
                number[0] = '+';
                body = number;
            }
            qiq_append_padded(&output, body, width, -1, padding, padding_length, left, negative || always_sign);
            break;
        }
        }
    }
    if (required > count) {
        free(output.data);
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "%lld arguments are required, %lld given",
                   (long long)required + 1, (long long)count + 1);
    }
    char *result = qiq_string_from(output.data != NULL ? output.data : "", output.length);
    free(output.data);
    return result;
}
