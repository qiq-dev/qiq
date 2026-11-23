// Regular expressions for std/regex (stdlib/regex.qiq), with PCRE2: the
// library PHP's preg_* functions use, so patterns behave the same.
//
// Each pattern is also compiled to machine code by PCRE2's JIT, as PHP
// does, when the system allows it (some forbid memory that is both
// written and run); a pattern the JIT can't compile is matched by the
// interpreter instead, with the same results.
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
// The JIT's stack for backtracking: 32 KB, growing to 1 MB, beyond which a
// match fails as with the match limit.
static pcre2_jit_stack *qiq_regex_jit_stack = NULL;

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
        qiq_regex_jit_stack = pcre2_jit_stack_create(32 * 1024, 1024 * 1024, NULL);
        if (qiq_regex_jit_stack != NULL) pcre2_jit_stack_assign(qiq_regex_context, NULL, qiq_regex_jit_stack);
    }
    // Failing leaves the pattern to the interpreter.
    (void)pcre2_jit_compile(code, PCRE2_JIT_COMPLETE);
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

// Throws the RegexException for a failed pcre2_match.
_Noreturn static void qiq_regex_fail(QiqRegex *regex, int result) {
    if (result == PCRE2_ERROR_MATCHLIMIT || result == PCRE2_ERROR_DEPTHLIMIT || result == PCRE2_ERROR_HEAPLIMIT || result == PCRE2_ERROR_JIT_STACKLIMIT) {
        qiq_throwf(QIQ_REGEX_EXCEPTION, "%s needs too much backtracking on this text (the match limit was reached); make its repetitions less ambiguous", regex->pattern);
    }
    PCRE2_UCHAR message[256];
    pcre2_get_error_message(result, message, sizeof message);
    qiq_throwf(QIQ_REGEX_EXCEPTION, "matching %s failed: %s", regex->pattern, (const char *)message);
}

// Finds the next match from *start into the pattern's match data, moving
// on as Perl and PHP do: after an empty match, it looks for a non-empty
// one at the same place, and then one a character later. Returns
// pcre2_match's result (above 0), or 0 when there are no more matches.
// For an error, calls `cleanup` (if any) and throws.
static int qiq_regex_next(QiqRegex *regex, const char *subject, size_t length, size_t *start, uint32_t *extra, void (*cleanup)(void)) {
    while (*start <= length) {
        int result = pcre2_match(regex->code, (PCRE2_SPTR)subject, length, *start, *extra | PCRE2_NO_UTF_CHECK, regex->data, qiq_regex_context);
        if (result == PCRE2_ERROR_NOMATCH) {
            if (*extra == 0) return 0;
            // No non-empty match here after an empty one: move on a character.
            *extra = 0;
            (*start)++;
            while (*start < length && ((unsigned char)subject[*start] & 0xc0) == 0x80) (*start)++;
            continue;
        }
        if (result < 0) {
            if (cleanup != NULL) cleanup();
            qiq_regex_fail(regex, result);
        }
        PCRE2_SIZE *vector = pcre2_get_ovector_pointer(regex->data);
        size_t match_start = vector[0] > vector[1] ? vector[1] : vector[0];
        *start = vector[1];
        *extra = vector[1] == match_start ? PCRE2_NOTEMPTY_ATSTART | PCRE2_ANCHORED : 0;
        return result;
    }
    return 0;
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
    PCRE2_SIZE *vector = pcre2_get_ovector_pointer(regex->data);
    QiqPosition position = {subject, 0, 0};
    size_t previous_end = 0;  // where the text since the previous match starts
    int64_t found = 0;
    uint32_t extra = 0;
    int result;
    while ((limit < 0 || found < limit) && (result = qiq_regex_next(regex, subject, length, &start, &extra, qiq_regex_clear)) > 0) {
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
    }
    if (want & QIQ_REGEX_GAPS) qiq_regex_add_text(qiq_string_from(subject + previous_end, length - previous_end));
    return found;
}

// ---------------------------------------------------------------------------
// preg_replace: the replacement is read once, and the result built in one
// buffer, with no qiq values made per match.

// A piece of a replacement: text, or (when `group` isn't -1) a group's text.
typedef struct {
    const char *text;
    size_t length;
    int64_t group;
} QiqPiece;

static QiqPiece *qiq_regex_pieces = NULL;
static int64_t qiq_regex_piece_count = 0, qiq_regex_piece_capacity = 0;
static char *qiq_regex_output = NULL;
static int64_t qiq_regex_output_length = 0, qiq_regex_output_capacity = 0;

static void qiq_regex_add_piece(const char *text, size_t length, int64_t group) {
    // Text right after more text joins it.
    if (group < 0 && qiq_regex_piece_count > 0) {
        QiqPiece *last = &qiq_regex_pieces[qiq_regex_piece_count - 1];
        if (last->group < 0 && last->text + last->length == text) {
            last->length += length;
            return;
        }
    }
    qiq_regex_pieces = qiq_grow(qiq_regex_pieces, &qiq_regex_piece_capacity, qiq_regex_piece_count + 1, sizeof(QiqPiece));
    qiq_regex_pieces[qiq_regex_piece_count++] = (QiqPiece){text, length, group};
}

static void qiq_regex_append(const char *text, size_t length) {
    qiq_regex_output = qiq_grow(qiq_regex_output, &qiq_regex_output_capacity, qiq_regex_output_length + (int64_t)length, 1);
    memcpy(qiq_regex_output + qiq_regex_output_length, text, length);
    qiq_regex_output_length += (int64_t)length;
}

// The lowest-numbered group called `name` (`length` bytes), or -1.
static int64_t qiq_regex_group_named(QiqRegex *regex, const char *name, size_t length) {
    uint32_t count = 0, entry_size = 0;
    PCRE2_SPTR table = NULL;
    pcre2_pattern_info(regex->code, PCRE2_INFO_NAMECOUNT, &count);
    pcre2_pattern_info(regex->code, PCRE2_INFO_NAMEENTRYSIZE, &entry_size);
    pcre2_pattern_info(regex->code, PCRE2_INFO_NAMETABLE, &table);
    int64_t found = -1;
    for (uint32_t index = 0; index < count; index++) {
        PCRE2_SPTR entry = table + (size_t)index * entry_size;
        const char *entry_name = (const char *)entry + 2;
        if (strlen(entry_name) == length && memcmp(entry_name, name, length) == 0) {
            int64_t group = ((int64_t)entry[0] << 8) | entry[1];
            if (found < 0 || group < found) found = group;
        }
    }
    return found;
}

// Reads a replacement, in which $1 (or ${1}, or \1; one or two digits) is a
// group's text, ${name} a named group's, and $$ a dollar sign. Same rules
// and messages as the compiler's check (src/regex_check.rs).
static void qiq_regex_read_replacement(QiqRegex *regex, const char *replacement) {
    qiq_regex_piece_count = 0;
    size_t length = strlen(replacement);
    size_t index = 0;
    while (index < length) {
        char c = replacement[index];
        char next = index + 1 < length ? replacement[index + 1] : '\0';
        int64_t group;
        if (c == '$' && next == '$') {
            qiq_regex_add_piece(replacement + index, 1, -1);
            index += 2;
            continue;
        }
        if ((c == '$' || c == '\\') && next >= '0' && next <= '9') {
            group = next - '0';
            index += 2;
            if (index < length && replacement[index] >= '0' && replacement[index] <= '9') {
                group = group * 10 + (replacement[index] - '0');
                index++;
            }
        } else if (c == '$' && next == '{') {
            const char *name = replacement + index + 2;
            const char *close = strchr(name, '}');
            if (close == NULL) qiq_throw(QIQ_REGEX_EXCEPTION, "the replacement has a '${' without a closing '}'");
            size_t name_length = (size_t)(close - name);
            bool digits = name_length > 0;
            for (size_t at = 0; at < name_length; at++) digits = digits && name[at] >= '0' && name[at] <= '9';
            if (digits) {
                group = 0;
                for (size_t at = 0; at < name_length && group <= 100000; at++) group = group * 10 + (name[at] - '0');
            } else {
                group = name_length > 0 ? qiq_regex_group_named(regex, name, name_length) : -1;
                if (group < 0) {
                    qiq_throwf(QIQ_REGEX_EXCEPTION, "the replacement refers to a group named '%.*s', which the pattern doesn't have", (int)name_length, name);
                }
            }
            index = (size_t)(close - replacement) + 1;
        } else {
            qiq_regex_add_piece(replacement + index, 1, -1);
            index++;
            continue;
        }
        if (group > (int64_t)regex->groups) {
            qiq_throwf(QIQ_REGEX_EXCEPTION, "the replacement refers to group %lld, but the pattern has %u group(s)", (long long)group, regex->groups);
        }
        qiq_regex_add_piece(NULL, 0, group);
    }
}

// __regex_replace(handle, subject, replacement, limit): the subject with its
// matches (the first `limit`, or all if negative) replaced.
QIQ_GUARDED(char *, qiq_stdlib___regex_replace, NULL, (int64_t handle, const char *subject, const char *replacement, int64_t limit),
            (handle, subject, replacement, limit)) {
    QiqRegex *regex = qiq_regex(handle);
    if (subject == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "the subject is null");
    if (replacement == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "the replacement is null");
    qiq_regex_read_replacement(regex, replacement);
    qiq_regex_output_length = 0;
    size_t length = strlen(subject);
    PCRE2_SIZE *vector = pcre2_get_ovector_pointer(regex->data);
    size_t start = 0, previous_end = 0;
    uint32_t extra = 0;
    int64_t found = 0;
    int result;
    while ((limit < 0 || found < limit) && (result = qiq_regex_next(regex, subject, length, &start, &extra, NULL)) > 0) {
        size_t match_start = vector[0], match_end = vector[1];
        if (match_start > match_end) match_start = match_end;
        size_t gap_end = match_start < previous_end ? previous_end : match_start;
        qiq_regex_append(subject + previous_end, gap_end - previous_end);
        for (int64_t piece = 0; piece < qiq_regex_piece_count; piece++) {
            QiqPiece *p = &qiq_regex_pieces[piece];
            if (p->group < 0) {
                qiq_regex_append(p->text, p->length);
            } else if (p->group < result && vector[2 * p->group] != PCRE2_UNSET) {
                size_t group_start = p->group == 0 ? match_start : vector[2 * p->group];
                qiq_regex_append(subject + group_start, vector[2 * p->group + 1] - group_start);
            }
        }
        found++;
        previous_end = match_end > gap_end ? match_end : gap_end;
    }
    qiq_regex_append(subject + previous_end, length - previous_end);
    return qiq_string_from(qiq_regex_output, (size_t)qiq_regex_output_length);
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
