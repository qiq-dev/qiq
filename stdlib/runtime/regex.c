// Regular expressions for std/regex (stdlib/regex.qiq), with PCRE2: the
// library PHP's preg_* functions use, so patterns behave the same.
//
// Patterns are written as in PHP, between delimiters and followed by flags:
// "/\d+/i". Every pattern is compiled in UTF mode with Unicode properties
// (as PHP's /u flag does), since qiq strings are always UTF-8. Compiled
// patterns are cached, like PHP's, so a pattern used in a loop is compiled
// once.
//
// __regex_scan finds matches and keeps their groups, the text between them,
// and their positions (in characters, as everywhere in qiq) until
// __regex_texts and __regex_offsets take them; preg_match, preg_match_all,
// preg_replace, and preg_split are written in qiq on top of these.
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <pcre2.h>

#include "qiq_runtime.h"

// As PHP's pcre.backtrack_limit and pcre.recursion_limit (roughly): enough
// for any reasonable pattern, and a RegexException instead of minutes of
// work for a catastrophic one such as /(a+)+$/.
#define QIQ_REGEX_MATCH_LIMIT 1000000
#define QIQ_REGEX_DEPTH_LIMIT 100000
// Patterns kept compiled; the cache is emptied when it fills, as PHP's is.
#define QIQ_REGEX_CACHE_SIZE 4096

typedef struct {
    char *pattern;     // as written, with delimiters and flags
    pcre2_code *code;
    uint32_t groups;   // capture groups, not counting the whole match
    // Kept with the pattern and reused by every scan: creating one per call
    // costs more than a short match itself with musl's malloc. A scan always
    // finishes before anything else can use the pattern (callbacks run
    // after it), so one is enough.
    pcre2_match_data *data;
    // The group names (see __regex_names), built when first asked for and
    // kept (retained): every match of the pattern shares them.
    void *names;
} QiqRegex;

static QiqRegex qiq_regex_cache[QIQ_REGEX_CACHE_SIZE];
static int64_t qiq_regex_cached = 0;
static pcre2_match_context *qiq_regex_context = NULL;

// The last scan's results, until taken.
static char **qiq_regex_texts = NULL;
static int64_t qiq_regex_text_count = 0, qiq_regex_text_capacity = 0;
static int64_t *qiq_regex_offsets = NULL;
static int64_t qiq_regex_offset_count = 0, qiq_regex_offset_capacity = 0;

static void *qiq_grow(void *data, int64_t *capacity, int64_t needed, size_t element) {
    if (needed <= *capacity) return data;
    int64_t size = *capacity == 0 ? 64 : *capacity;
    while (size < needed) size *= 2;
    void *grown = realloc(data, (size_t)size * element);
    if (grown == NULL) qiq_fatal("out of memory");
    *capacity = size;
    return grown;
}

static void qiq_regex_add_text(char *text) {
    qiq_regex_texts = qiq_grow(qiq_regex_texts, &qiq_regex_text_capacity, qiq_regex_text_count + 1, sizeof(char *));
    qiq_regex_texts[qiq_regex_text_count++] = text;
}

static void qiq_regex_add_offset(int64_t offset) {
    qiq_regex_offsets = qiq_grow(qiq_regex_offsets, &qiq_regex_offset_capacity, qiq_regex_offset_count + 1, sizeof(int64_t));
    qiq_regex_offsets[qiq_regex_offset_count++] = offset;
}

static void qiq_regex_clear(void) {
    for (int64_t index = 0; index < qiq_regex_text_count; index++) {
        if (qiq_regex_texts[index] != NULL) qiq_release(qiq_regex_texts[index]);
    }
    qiq_regex_text_count = 0;
    qiq_regex_offset_count = 0;
}

// The number of characters in `length` bytes of UTF-8.
static int64_t qiq_characters(const char *text, size_t length) {
    int64_t count = 0;
    for (size_t index = 0; index < length; index++) {
        if (((unsigned char)text[index] & 0xc0) != 0x80) count++;
    }
    return count;
}

// Converts byte positions in a subject to character positions, quickly for
// positions that mostly move forward.
typedef struct {
    const char *subject;
    size_t byte;
    int64_t character;
} QiqPosition;

static int64_t qiq_character_at(QiqPosition *position, size_t byte) {
    if (byte >= position->byte) {
        position->character += qiq_characters(position->subject + position->byte, byte - position->byte);
    } else {
        position->character -= qiq_characters(position->subject + byte, position->byte - byte);
    }
    position->byte = byte;
    return position->character;
}

// Reads a PHP-style pattern: the delimiters, the expression between them,
// and the flags after. Throws RegexException for a malformed one.
static void qiq_regex_split(const char *pattern, size_t *body_start, size_t *body_length, uint32_t *options) {
    size_t length = strlen(pattern);
    size_t start = 0;
    while (start < length && (pattern[start] == ' ' || pattern[start] == '\t' || pattern[start] == '\n' || pattern[start] == '\r')) start++;
    if (start == length) qiq_throw(QIQ_REGEX_EXCEPTION, "empty regular expression; write it between delimiters, as in `/\\d+/`");
    char open = pattern[start];
    if ((open >= 'a' && open <= 'z') || (open >= 'A' && open <= 'Z') || (open >= '0' && open <= '9') || open == '\\') {
        qiq_throwf(QIQ_REGEX_EXCEPTION, "the delimiter can't be a letter, digit, or backslash: write the pattern between delimiters, as in \"/%s/\"", pattern);
    }
    if ((unsigned char)open >= 0x80) qiq_throw(QIQ_REGEX_EXCEPTION, "the delimiter must be an ASCII punctuation character, such as /");
    char close = open == '(' ? ')' : open == '[' ? ']' : open == '{' ? '}' : open == '<' ? '>' : open;
    size_t index = start + 1;
    int depth = 1;
    for (; index < length; index++) {
        char c = pattern[index];
        if (c == '\\' && index + 1 < length) {
            index++;
        } else if (c == close && --depth == 0) {
            break;
        } else if (c == open && open != close) {
            depth++;
        }
    }
    if (index >= length) qiq_throwf(QIQ_REGEX_EXCEPTION, "no ending delimiter '%c' in %s", close, pattern);
    *body_start = start + 1;
    *body_length = index - start - 1;
    uint32_t flags = PCRE2_UTF | PCRE2_UCP | PCRE2_NO_UTF_CHECK;
    for (index++; index < length; index++) {
        switch (pattern[index]) {
            case 'i': flags |= PCRE2_CASELESS; break;
            case 'm': flags |= PCRE2_MULTILINE; break;
            case 's': flags |= PCRE2_DOTALL; break;
            case 'x': flags |= PCRE2_EXTENDED; break;
            case 'u': break;  // always on
            case 'U': flags |= PCRE2_UNGREEDY; break;
            case 'D': flags |= PCRE2_DOLLAR_ENDONLY; break;
            case 'A': flags |= PCRE2_ANCHORED; break;
            case 'n': flags |= PCRE2_NO_AUTO_CAPTURE; break;
            case 'J': flags |= PCRE2_DUPNAMES; break;
            case ' ': case '\n': case '\r': case '\t': break;
            default:
                qiq_throwf(QIQ_REGEX_EXCEPTION, "unknown flag '%c' in %s; the flags are i, m, s, x, u, U, D, A, n, and J", pattern[index], pattern);
        }
    }
    *options = flags;
}

// __regex_compile(pattern): a handle for the compiled pattern.
QIQ_GUARDED(int64_t, qiq_stdlib___regex_compile, 0, (const char *pattern), (pattern)) {
    if (pattern == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "the pattern is null");
    for (int64_t index = 0; index < qiq_regex_cached; index++) {
        if (strcmp(qiq_regex_cache[index].pattern, pattern) == 0) return index;
    }
    size_t body_start, body_length;
    uint32_t options;
    qiq_regex_split(pattern, &body_start, &body_length, &options);
    int error;
    PCRE2_SIZE error_offset;
    pcre2_code *code = pcre2_compile((PCRE2_SPTR)(pattern + body_start), body_length, options, &error, &error_offset, NULL);
    if (code == NULL) {
        PCRE2_UCHAR message[256];
        pcre2_get_error_message(error, message, sizeof message);
        int64_t at = qiq_characters(pattern + body_start, error_offset);
        qiq_throwf(QIQ_REGEX_EXCEPTION, "invalid regular expression %s: %s at offset %lld", pattern, (const char *)message, (long long)at);
    }
    if (qiq_regex_context == NULL) {
        qiq_regex_context = pcre2_match_context_create(NULL);
        if (qiq_regex_context == NULL) qiq_fatal("out of memory");
        pcre2_set_match_limit(qiq_regex_context, QIQ_REGEX_MATCH_LIMIT);
        pcre2_set_depth_limit(qiq_regex_context, QIQ_REGEX_DEPTH_LIMIT);
    }
    if (qiq_regex_cached == QIQ_REGEX_CACHE_SIZE) {
        for (int64_t index = 0; index < qiq_regex_cached; index++) {
            free(qiq_regex_cache[index].pattern);
            pcre2_code_free(qiq_regex_cache[index].code);
            pcre2_match_data_free(qiq_regex_cache[index].data);
            if (qiq_regex_cache[index].names != NULL) qiq_release(qiq_regex_cache[index].names);
        }
        qiq_regex_cached = 0;
    }
    uint32_t groups = 0;
    pcre2_pattern_info(code, PCRE2_INFO_CAPTURECOUNT, &groups);
    char *copy = strdup(pattern);
    if (copy == NULL) qiq_fatal("out of memory");
    pcre2_match_data *data = pcre2_match_data_create_from_pattern(code, NULL);
    if (data == NULL) qiq_fatal("out of memory");
    qiq_regex_cache[qiq_regex_cached] = (QiqRegex){copy, code, groups, data, NULL};
    return qiq_regex_cached++;
}

static QiqRegex *qiq_regex(int64_t handle) {
    if (handle < 0 || handle >= qiq_regex_cached) qiq_throw(QIQ_RUNTIME_EXCEPTION, "invalid regular expression handle");
    return &qiq_regex_cache[handle];
}

// __regex_names(handle): one element per group, the whole match first: the
// group's name, or "" for an unnamed group.
QIQ_GUARDED(void *, qiq_stdlib___regex_names, NULL, (int64_t handle), (handle)) {
    QiqRegex *regex = qiq_regex(handle);
    if (regex->names != NULL) {
        qiq_retain(regex->names);
        return regex->names;
    }
    void *names = qiq_array_create((int64_t)regex->groups + 1, sizeof(char *), QIQ_ELEMENTS_ARE_REFERENCES);
    char **data = QIQ_ARRAY_DATA(names);
    for (uint32_t group = 0; group <= regex->groups; group++) data[group] = qiq_string_from("", 0);
    uint32_t count = 0, entry_size = 0;
    PCRE2_SPTR table = NULL;
    pcre2_pattern_info(regex->code, PCRE2_INFO_NAMECOUNT, &count);
    pcre2_pattern_info(regex->code, PCRE2_INFO_NAMEENTRYSIZE, &entry_size);
    pcre2_pattern_info(regex->code, PCRE2_INFO_NAMETABLE, &table);
    for (uint32_t index = 0; index < count; index++) {
        PCRE2_SPTR entry = table + (size_t)index * entry_size;
        uint32_t group = ((uint32_t)entry[0] << 8) | entry[1];
        const char *name = (const char *)entry + 2;
        if (group <= regex->groups) {
            qiq_release(data[group]);
            data[group] = qiq_string_from(name, strlen(name));
        }
    }
    regex->names = names;
    qiq_retain(names);
    return names;
}

// What a caller of __regex_scan uses; what it doesn't is left out, which
// saves an allocation per match: without QIQ_REGEX_GAPS there are no gap
// texts at all (so one match's texts are exactly its groups), without
// QIQ_REGEX_GROUPS the group texts are null, and without
// QIQ_REGEX_OFFSETS there are no offsets.
#define QIQ_REGEX_GAPS 1     // the text between matches, and after the last
#define QIQ_REGEX_GROUPS 2   // the text of the whole match and each group
#define QIQ_REGEX_OFFSETS 4  // their positions

// __regex_scan(handle, subject, offset, limit, want): finds up to `limit` matches
// (all if negative), starting the search at character `offset`, and returns
// how many it found. For each match, the texts are the text since the
// previous match (or the start of the subject), then the whole match and
// each group (null for a group that took no part); after the last match
// comes the rest of the subject. The offsets are each group's start and end
// characters (-1 for a group that took no part). `want` says which of
// these the caller uses (QIQ_REGEX_*).
QIQ_GUARDED(int64_t, qiq_stdlib___regex_scan, 0, (int64_t handle, const char *subject, int64_t offset, int64_t limit, int64_t want),
            (handle, subject, offset, limit, want)) {
    QiqRegex *regex = qiq_regex(handle);
    if (subject == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "the subject is null");
    qiq_regex_clear();
    size_t length = strlen(subject);
    // The byte at character `offset`.
    if (offset < 0) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "the offset must not be negative, found %lld", (long long)offset);
    size_t start = 0;
    for (int64_t skipped = 0; skipped < offset; skipped++) {
        if (start >= length) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "offset %lld is past the end of the subject", (long long)offset);
        start++;
        while (start < length && ((unsigned char)subject[start] & 0xc0) == 0x80) start++;
    }
    pcre2_match_data *data = regex->data;
    PCRE2_SIZE *vector = pcre2_get_ovector_pointer(data);
    QiqPosition position = {subject, 0, 0};
    size_t previous_end = 0;  // where the text since the previous match starts
    int64_t found = 0;
    uint32_t extra = 0;       // options for retrying after an empty match
    while (limit < 0 || found < limit) {
        if (start > length) break;
        int result = pcre2_match(regex->code, (PCRE2_SPTR)subject, length, start, extra | PCRE2_NO_UTF_CHECK, data, qiq_regex_context);
        if (result == PCRE2_ERROR_NOMATCH) {
            if (extra == 0) break;
            // No non-empty match here after an empty one: move on a character.
            extra = 0;
            start++;
            while (start < length && ((unsigned char)subject[start] & 0xc0) == 0x80) start++;
            continue;
        }
        if (result < 0) {
            qiq_regex_clear();
            if (result == PCRE2_ERROR_MATCHLIMIT || result == PCRE2_ERROR_DEPTHLIMIT || result == PCRE2_ERROR_HEAPLIMIT) {
                qiq_throwf(QIQ_REGEX_EXCEPTION, "%s needs too much backtracking on this text (the match limit was reached); make its repetitions less ambiguous", regex->pattern);
            }
            PCRE2_UCHAR message[256];
            pcre2_get_error_message(result, message, sizeof message);
            qiq_throwf(QIQ_REGEX_EXCEPTION, "matching %s failed: %s", regex->pattern, (const char *)message);
        }
        size_t match_start = vector[0], match_end = vector[1];
        // \K can put a match's start after its end; treat that as empty.
        if (match_start > match_end) match_start = match_end;
        size_t gap_end = match_start < previous_end ? previous_end : match_start;
        if (want & QIQ_REGEX_GAPS) qiq_regex_add_text(qiq_string_from(subject + previous_end, gap_end - previous_end));
        for (uint32_t group = 0; group <= regex->groups; group++) {
            PCRE2_SIZE group_start = vector[2 * group], group_end = vector[2 * group + 1];
            bool unset = (int)group >= result || group_start == PCRE2_UNSET;
            if (group == 0) group_start = match_start;
            qiq_regex_add_text(!unset && (want & QIQ_REGEX_GROUPS) ? qiq_string_from(subject + group_start, group_end - group_start) : NULL);
            if (want & QIQ_REGEX_OFFSETS) {
                qiq_regex_add_offset(unset ? -1 : qiq_character_at(&position, group_start));
                qiq_regex_add_offset(unset ? -1 : qiq_character_at(&position, group_end));
            }
        }
        found++;
        previous_end = match_end > gap_end ? match_end : gap_end;
        start = match_end;
        // After an empty match, look for a non-empty one at the same place
        // before moving on, as Perl and PHP do.
        extra = match_end == match_start ? PCRE2_NOTEMPTY_ATSTART | PCRE2_ANCHORED : 0;
    }
    if (want & QIQ_REGEX_GAPS) qiq_regex_add_text(qiq_string_from(subject + previous_end, length - previous_end));
    return found;
}

// __regex_texts(): the last scan's texts, which are then forgotten.
QIQ_GUARDED(void *, qiq_stdlib___regex_texts, NULL, (void), ()) {
    void *array = qiq_array_create(qiq_regex_text_count, sizeof(char *), QIQ_ELEMENTS_ARE_REFERENCES);
    memcpy(QIQ_ARRAY_DATA(array), qiq_regex_texts, (size_t)qiq_regex_text_count * sizeof(char *));
    qiq_regex_text_count = 0;
    return array;
}

// __regex_offsets(): the last scan's offsets, which are then forgotten.
QIQ_GUARDED(void *, qiq_stdlib___regex_offsets, NULL, (void), ()) {
    void *array = qiq_array_create(qiq_regex_offset_count, sizeof(int64_t), 0);
    memcpy(QIQ_ARRAY_DATA(array), qiq_regex_offsets, (size_t)qiq_regex_offset_count * sizeof(int64_t));
    qiq_regex_offset_count = 0;
    return array;
}
