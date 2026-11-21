// Date and time: the clock, formatting with PHP's date() letters, building
// and parsing timestamps, timezones, and sleeping. Timestamps are seconds
// since 1970-01-01 00:00:00 UTC. Local time follows the TZ environment
// variable or /etc/localtime, until the program calls
// date_default_timezone_set; zone data is read from the system's zoneinfo
// files when the program runs.
#define _DEFAULT_SOURCE
#include <errno.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

#include "qiq_runtime.h"

// The default for "timestamp" parameters: the current time.
#define QIQ_NOW INT64_MIN

static const char *const day_names[] = {"Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"};
static const char *const month_names[] = {"January", "February", "March", "April", "May", "June", "July",
                                          "August", "September", "October", "November", "December"};

static char *qiq_copy_string(const char *text) {
    char *copy = strdup(text);
    if (copy == NULL) qiq_fatal("out of memory");
    return copy;
}

static struct timespec qiq_clock(clockid_t clock) {
    struct timespec now;
    clock_gettime(clock, &now);
    return now;
}

int64_t qiq_stdlib_time(void) {
    return (int64_t)qiq_clock(CLOCK_REALTIME).tv_sec;
}

double qiq_stdlib_microtime(void) {
    struct timespec now = qiq_clock(CLOCK_REALTIME);
    return (double)now.tv_sec + (double)(now.tv_nsec / 1000) / 1e6;
}

int64_t qiq_stdlib_hrtime(void) {
    struct timespec now = qiq_clock(CLOCK_MONOTONIC);
    return (int64_t)now.tv_sec * 1000000000 + now.tv_nsec;
}

// ---------------------------------------------------------------------------
// Timezones

// The zone set by date_default_timezone_set, or NULL for the system default.
static char *qiq_zone = NULL;

static const char *const zoneinfo_directories[] = {"/usr/share/zoneinfo/", "/share/zoneinfo/", "/etc/zoneinfo/"};

// Whether `zone` names a zoneinfo file (a "TZif" file under one of the
// directories musl searches).
static bool qiq_zone_exists(const char *zone) {
    if (zone[0] == '\0' || zone[0] == '/' || strstr(zone, "..") != NULL) return false;
    for (const char *c = zone; *c; c++) {
        bool allowed = (*c >= 'A' && *c <= 'Z') || (*c >= 'a' && *c <= 'z') || (*c >= '0' && *c <= '9')
            || *c == '/' || *c == '_' || *c == '-' || *c == '+';
        if (!allowed) return false;
    }
    for (size_t i = 0; i < sizeof zoneinfo_directories / sizeof *zoneinfo_directories; i++) {
        char path[512];
        if (snprintf(path, sizeof path, "%s%s", zoneinfo_directories[i], zone) >= (int)sizeof path) return false;
        FILE *file = fopen(path, "rb");
        if (file == NULL) continue;
        char magic[4];
        size_t read = fread(magic, 1, 4, file);
        fclose(file);
        if (read == 4 && memcmp(magic, "TZif", 4) == 0) return true;
    }
    return false;
}

QIQ_GUARDED_VOID(qiq_stdlib_date_default_timezone_set, (const char *zone), (zone)) {
    if (zone == NULL) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "timezone is null");
    if (strcmp(zone, "UTC") != 0 && !qiq_zone_exists(zone)) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "unknown timezone \"%s\"", zone);
    }
    char *copy = qiq_copy_string(zone);
    free(qiq_zone);
    qiq_zone = copy;
    setenv("TZ", zone, 1);
    tzset();
}

// The zone name: the one set by the program, else TZ, else the name of the
// zone /etc/localtime links to, else "UTC".
static const char *qiq_zone_name(char *buffer, size_t size) {
    if (qiq_zone != NULL) return qiq_zone;
    const char *tz = getenv("TZ");
    if (tz != NULL && tz[0] != '\0') return tz[0] == ':' ? tz + 1 : tz;
    ssize_t length = readlink("/etc/localtime", buffer, size - 1);
    if (length > 0) {
        buffer[length] = '\0';
        char *name = strstr(buffer, "zoneinfo/");
        if (name != NULL) return name + strlen("zoneinfo/");
    }
    return "UTC";
}

char *qiq_stdlib_date_default_timezone_get(void) {
    char buffer[512];
    const char *zone = qiq_zone_name(buffer, sizeof buffer);
    return qiq_string_from(zone, strlen(zone));
}

// ---------------------------------------------------------------------------
// Formatting

typedef struct {
    char *data;
    size_t length;
    size_t capacity;
} QiqBuffer;

static void qiq_append(QiqBuffer *buffer, const char *text) {
    size_t add = strlen(text);
    if (buffer->length + add + 1 > buffer->capacity) {
        size_t capacity = buffer->capacity ? buffer->capacity : 64;
        while (buffer->length + add + 1 > capacity) capacity *= 2;
        char *grown = realloc(buffer->data, capacity);
        if (grown == NULL) qiq_fatal("out of memory");
        buffer->data = grown;
        buffer->capacity = capacity;
    }
    memcpy(buffer->data + buffer->length, text, add + 1);
    buffer->length += add;
}

static void qiq_appendf(QiqBuffer *buffer, const char *format, long long value) {
    char text[32];
    snprintf(text, sizeof text, format, value);
    qiq_append(buffer, text);
}

static bool qiq_is_leap(long long year) {
    return (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
}

static int qiq_days_in_month(long long year, int month) {
    static const int days[] = {31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31};
    return month == 2 && qiq_is_leap(year) ? 29 : days[month - 1];
}

// "+0200" or, with `colon`, "+02:00".
static void qiq_append_offset(QiqBuffer *buffer, long offset, bool colon) {
    char text[16];
    long magnitude = offset < 0 ? -offset : offset;
    snprintf(text, sizeof text, colon ? "%c%02ld:%02ld" : "%c%02ld%02ld", offset < 0 ? '-' : '+',
             magnitude / 3600, magnitude / 60 % 60);
    qiq_append(buffer, text);
}

static void qiq_append_year(QiqBuffer *buffer, long long year) {
    qiq_appendf(buffer, year < 0 ? "-%04lld" : "%04lld", year < 0 ? -year : year);
}

static void qiq_format(QiqBuffer *out, const char *format, int64_t timestamp, const struct tm *tm, bool utc) {
    long long year = (long long)tm->tm_year + 1900;
    int hour12 = tm->tm_hour % 12 == 0 ? 12 : tm->tm_hour % 12;
    long offset = utc ? 0 : tm->tm_gmtoff;
    char zone_buffer[512];
    char text[64];
    for (const char *c = format; *c; c++) {
        switch (*c) {
        // Day
        case 'd': qiq_appendf(out, "%02lld", tm->tm_mday); break;
        case 'D': snprintf(text, sizeof text, "%.3s", day_names[tm->tm_wday]); qiq_append(out, text); break;
        case 'j': qiq_appendf(out, "%lld", tm->tm_mday); break;
        case 'l': qiq_append(out, day_names[tm->tm_wday]); break;
        case 'N': qiq_appendf(out, "%lld", tm->tm_wday == 0 ? 7 : tm->tm_wday); break;
        case 'S': {
            int day = tm->tm_mday;
            const char *suffix = "th";
            if (day % 10 == 1 && day != 11) suffix = "st";
            if (day % 10 == 2 && day != 12) suffix = "nd";
            if (day % 10 == 3 && day != 13) suffix = "rd";
            qiq_append(out, suffix);
            break;
        }
        case 'w': qiq_appendf(out, "%lld", tm->tm_wday); break;
        case 'z': qiq_appendf(out, "%lld", tm->tm_yday); break;
        // ISO 8601 week and week-numbering year
        case 'W': strftime(text, sizeof text, "%V", tm); qiq_append(out, text); break;
        case 'o': strftime(text, sizeof text, "%G", tm); qiq_append(out, text); break;
        // Month
        case 'F': qiq_append(out, month_names[tm->tm_mon]); break;
        case 'm': qiq_appendf(out, "%02lld", tm->tm_mon + 1); break;
        case 'M': snprintf(text, sizeof text, "%.3s", month_names[tm->tm_mon]); qiq_append(out, text); break;
        case 'n': qiq_appendf(out, "%lld", tm->tm_mon + 1); break;
        case 't': qiq_appendf(out, "%lld", qiq_days_in_month(year, tm->tm_mon + 1)); break;
        // Year
        case 'L': qiq_append(out, qiq_is_leap(year) ? "1" : "0"); break;
        case 'Y': qiq_append_year(out, year); break;
        case 'y': qiq_appendf(out, "%02lld", (year < 0 ? -year : year) % 100); break;
        // Time
        case 'a': qiq_append(out, tm->tm_hour < 12 ? "am" : "pm"); break;
        case 'A': qiq_append(out, tm->tm_hour < 12 ? "AM" : "PM"); break;
        case 'B': {
            long long seconds = ((timestamp + 3600) % 86400 + 86400) % 86400;
            qiq_appendf(out, "%03lld", seconds * 10 / 864);
            break;
        }
        case 'g': qiq_appendf(out, "%lld", hour12); break;
        case 'G': qiq_appendf(out, "%lld", tm->tm_hour); break;
        case 'h': qiq_appendf(out, "%02lld", hour12); break;
        case 'H': qiq_appendf(out, "%02lld", tm->tm_hour); break;
        case 'i': qiq_appendf(out, "%02lld", tm->tm_min); break;
        case 's': qiq_appendf(out, "%02lld", tm->tm_sec); break;
        case 'u': qiq_append(out, "000000"); break;
        case 'v': qiq_append(out, "000"); break;
        // Timezone
        case 'e': qiq_append(out, utc ? "UTC" : qiq_zone_name(zone_buffer, sizeof zone_buffer)); break;
        case 'I': qiq_append(out, !utc && tm->tm_isdst > 0 ? "1" : "0"); break;
        case 'O': qiq_append_offset(out, offset, false); break;
        case 'P': qiq_append_offset(out, offset, true); break;
        case 'p':
            if (offset == 0) qiq_append(out, "Z");
            else qiq_append_offset(out, offset, true);
            break;
        case 'T': qiq_append(out, utc ? "GMT" : (tm->tm_zone ? tm->tm_zone : "")); break;
        case 'Z': qiq_appendf(out, "%lld", offset); break;
        // Full date/time
        case 'c': qiq_format(out, "Y-m-d\\TH:i:sP", timestamp, tm, utc); break;
        case 'r': qiq_format(out, "D, d M Y H:i:s O", timestamp, tm, utc); break;
        case 'U': qiq_appendf(out, "%lld", timestamp); break;
        case '\\':
            if (c[1] != '\0') c++;
            /* fall through */
        default: {
            // Copy the character, or all bytes of a UTF-8 character.
            size_t width = 1;
            while ((c[width] & 0xc0) == 0x80) width++;
            snprintf(text, sizeof text, "%.*s", (int)width, c);
            qiq_append(out, text);
            c += width - 1;
        }
        }
    }
}

static char *qiq_date(const char *format, int64_t timestamp, bool utc) {
    if (format == NULL) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "format is null");
    if (timestamp == QIQ_NOW) timestamp = qiq_stdlib_time();
    time_t seconds = (time_t)timestamp;
    struct tm tm;
    if ((utc ? gmtime_r(&seconds, &tm) : localtime_r(&seconds, &tm)) == NULL) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "timestamp %lld is out of range", (long long)timestamp);
    }
    QiqBuffer out = {0};
    qiq_append(&out, "");
    qiq_format(&out, format, timestamp, &tm, utc);
    char *text = qiq_string_from(out.data, out.length);
    free(out.data);
    return text;
}

QIQ_GUARDED(char *, qiq_stdlib_date, NULL, (const char *format, int64_t timestamp), (format, timestamp)) {
    return qiq_date(format, timestamp, false);
}

QIQ_GUARDED(char *, qiq_stdlib_gmdate, NULL, (const char *format, int64_t timestamp), (format, timestamp)) {
    return qiq_date(format, timestamp, true);
}

// ---------------------------------------------------------------------------
// Building timestamps

static int qiq_tm_field(int64_t value, const char *name) {
    if (value < -1000000000 || value > 1000000000) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "%s %lld is out of range", name, (long long)value);
    }
    return (int)value;
}

// The local UTC offset, in seconds, at `when`.
static long qiq_offset_at(time_t when) {
    struct tm tm;
    if (localtime_r(&when, &tm) == NULL) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "date is out of range");
    return tm.tm_gmtoff;
}

// The timestamp of `naive` (a local time written as if it were UTC), resolved
// near a daylight saving change the way PHP's date library (timelib) does it:
// take the offset at `naive`, then the offset that applies once that one is
// subtracted. A time skipped by the change keeps the offset from before it,
// so 02:30 in a spring-forward gap becomes 03:30.
static int64_t qiq_local_to_utc(int64_t naive) {
    long current = qiq_offset_at((time_t)naive);
    long after = qiq_offset_at((time_t)(naive - current));
    if (current == after) return naive - current;
    // Find the change to `after`: the first second at which it applies.
    int64_t high = naive - current;
    int64_t low = high - 2 * 86400;
    if (qiq_offset_at((time_t)low) == after) return naive - after;
    while (high - low > 1) {
        int64_t middle = low + (high - low) / 2;
        if (qiq_offset_at((time_t)middle) == after) high = middle;
        else low = middle;
    }
    int64_t change = high;
    bool skipped = naive - after >= change + (current - after) && naive - after < change;
    return skipped ? naive - current : naive - after;
}

// Out-of-range fields roll over: month 13 is January of the next year.
static int64_t qiq_make_time(int64_t hour, int64_t minute, int64_t second, int64_t month, int64_t day,
                             int64_t year, bool utc) {
    struct tm tm = {0};
    tm.tm_hour = qiq_tm_field(hour, "hour");
    tm.tm_min = qiq_tm_field(minute, "minute");
    tm.tm_sec = qiq_tm_field(second, "second");
    tm.tm_mon = qiq_tm_field(month, "month") - 1;
    tm.tm_mday = qiq_tm_field(day, "day");
    tm.tm_year = qiq_tm_field(year, "year") - 1900;
    errno = 0;
    time_t naive = timegm(&tm);
    if (naive == (time_t)-1 && errno != 0) {
        qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "date is out of range");
    }
    return utc ? (int64_t)naive : qiq_local_to_utc((int64_t)naive);
}

QIQ_GUARDED(int64_t, qiq_stdlib_mktime, 0,
            (int64_t hour, int64_t minute, int64_t second, int64_t month, int64_t day, int64_t year),
            (hour, minute, second, month, day, year)) {
    return qiq_make_time(hour, minute, second, month, day, year, false);
}

QIQ_GUARDED(int64_t, qiq_stdlib_gmmktime, 0,
            (int64_t hour, int64_t minute, int64_t second, int64_t month, int64_t day, int64_t year),
            (hour, minute, second, month, day, year)) {
    return qiq_make_time(hour, minute, second, month, day, year, true);
}

bool qiq_stdlib_checkdate(int64_t month, int64_t day, int64_t year) {
    return year >= 1 && year <= 32767 && month >= 1 && month <= 12 && day >= 1
        && day <= qiq_days_in_month(year, (int)month);
}

// ---------------------------------------------------------------------------
// Parsing

// Reads exactly `count` digits.
static bool qiq_digits(const char **cursor, int count, int *value) {
    *value = 0;
    for (int i = 0; i < count; i++) {
        char c = (*cursor)[i];
        if (c < '0' || c > '9') return false;
        *value = *value * 10 + (c - '0');
    }
    *cursor += count;
    return true;
}

// Parses an ISO 8601 date, date and time, or date, time, and offset:
// "2026-11-19", "2026-11-19 14:30", "2026-11-19T14:30:00.5Z",
// "2026-11-19T14:30:00+02:00". Without an offset the time is local.
static bool qiq_parse_iso8601(const char *text, int64_t *result) {
    const char *c = text;
    int year, month, day, hour = 0, minute = 0, second = 0;
    if (!qiq_digits(&c, 4, &year) || *c++ != '-' || !qiq_digits(&c, 2, &month) || *c++ != '-'
        || !qiq_digits(&c, 2, &day)) {
        return false;
    }
    if (!qiq_stdlib_checkdate(month, day, year)) return false;
    bool has_offset = false;
    long offset = 0;
    if (*c == 'T' || *c == 't' || *c == ' ') {
        c++;
        if (!qiq_digits(&c, 2, &hour) || *c++ != ':' || !qiq_digits(&c, 2, &minute)) return false;
        if (*c == ':') {
            c++;
            if (!qiq_digits(&c, 2, &second)) return false;
            if (*c == '.' || *c == ',') {
                c++;
                if (*c < '0' || *c > '9') return false;
                while (*c >= '0' && *c <= '9') c++;
            }
        }
        if (hour > 23 || minute > 59 || second > 59) return false;
        if (*c == 'Z' || *c == 'z') {
            c++;
            has_offset = true;
        } else if (*c == '+' || *c == '-') {
            int sign = *c++ == '-' ? -1 : 1;
            int offset_hours, offset_minutes = 0;
            if (!qiq_digits(&c, 2, &offset_hours)) return false;
            if (*c == ':') c++;
            if (*c != '\0' && !qiq_digits(&c, 2, &offset_minutes)) return false;
            if (offset_hours > 23 || offset_minutes > 59) return false;
            offset = sign * (offset_hours * 3600L + offset_minutes * 60L);
            has_offset = true;
        }
    }
    if (*c != '\0') return false;
    if (has_offset) {
        *result = qiq_make_time(hour, minute, second, month, day, year, true) - offset;
    } else {
        *result = qiq_make_time(hour, minute, second, month, day, year, false);
    }
    return true;
}

QIQ_GUARDED(int64_t, qiq_stdlib_strtotime, 0, (const char *text), (text)) {
    if (text == NULL) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "date is null");
    int64_t result;
    if (!qiq_parse_iso8601(text, &result)) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "\"%s\" is not an ISO 8601 date", text);
    }
    return result;
}

// ---------------------------------------------------------------------------
// Sleeping

static void qiq_sleep_for(struct timespec duration) {
    while (nanosleep(&duration, &duration) != 0 && errno == EINTR) {
    }
}

QIQ_GUARDED_VOID(qiq_stdlib_sleep, (int64_t seconds), (seconds)) {
    if (seconds < 0) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "sleep time %lld is negative", (long long)seconds);
    qiq_sleep_for((struct timespec){.tv_sec = (time_t)seconds, .tv_nsec = 0});
}

QIQ_GUARDED_VOID(qiq_stdlib_usleep, (int64_t microseconds), (microseconds)) {
    if (microseconds < 0) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "sleep time %lld is negative", (long long)microseconds);
    }
    qiq_sleep_for((struct timespec){.tv_sec = (time_t)(microseconds / 1000000),
                                    .tv_nsec = (long)(microseconds % 1000000) * 1000});
}
