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
#include <strings.h>
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

static void qiq_append_bytes(QiqBuffer *buffer, const char *text, size_t add) {
    if (buffer->length + add + 1 > buffer->capacity) {
        size_t capacity = buffer->capacity ? buffer->capacity : 64;
        while (buffer->length + add + 1 > capacity) capacity *= 2;
        char *grown = realloc(buffer->data, capacity);
        if (grown == NULL) qiq_fatal("out of memory");
        buffer->data = grown;
        buffer->capacity = capacity;
    }
    memcpy(buffer->data + buffer->length, text, add);
    buffer->length += add;
    buffer->data[buffer->length] = '\0';
}

static void qiq_append(QiqBuffer *buffer, const char *text) {
    qiq_append_bytes(buffer, text, strlen(text));
}

// `value` in decimal, with zeros in front to make at least `digits` digits.
// Written directly rather than with snprintf, which takes about 250 ns a
// call in musl: date("D, d M Y H:i:s") made eleven calls.
static void qiq_append_number(QiqBuffer *buffer, long long value, int digits) {
    char text[QIQ_INT_TEXT_SIZE];
    int length = qiq_format_int(text, value);
    const char *number = text;
    if (value < 0) {
        qiq_append_bytes(buffer, "-", 1);
        number++;
        length--;
    }
    for (int padding = digits - length; padding > 0; padding--) qiq_append_bytes(buffer, "0", 1);
    qiq_append_bytes(buffer, number, (size_t)length);
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
    qiq_append_number(buffer, year, 4);
}

// What a format writes: the time's fields (`tm`, with tm_gmtoff and tm_zone
// set), its timestamp, its microseconds, and its zone's name for 'e' (NULL
// for the default zone's).
static void qiq_format(QiqBuffer *out, const char *format, int64_t timestamp, const struct tm *tm, bool utc, int micro,
                       const char *zone) {
    long long year = (long long)tm->tm_year + 1900;
    int hour12 = tm->tm_hour % 12 == 0 ? 12 : tm->tm_hour % 12;
    long offset = utc ? 0 : tm->tm_gmtoff;
    char zone_buffer[512];
    char text[64];
    for (const char *c = format; *c; c++) {
        switch (*c) {
        // Day
        case 'd': qiq_append_number(out, tm->tm_mday, 2); break;
        case 'D': qiq_append_bytes(out, day_names[tm->tm_wday], 3); break;
        case 'j': qiq_append_number(out, tm->tm_mday, 0); break;
        case 'l': qiq_append(out, day_names[tm->tm_wday]); break;
        case 'N': qiq_append_number(out, tm->tm_wday == 0 ? 7 : tm->tm_wday, 0); break;
        case 'S': {
            int day = tm->tm_mday;
            const char *suffix = "th";
            if (day % 10 == 1 && day != 11) suffix = "st";
            if (day % 10 == 2 && day != 12) suffix = "nd";
            if (day % 10 == 3 && day != 13) suffix = "rd";
            qiq_append(out, suffix);
            break;
        }
        case 'w': qiq_append_number(out, tm->tm_wday, 0); break;
        case 'z': qiq_append_number(out, tm->tm_yday, 0); break;
        // ISO 8601 week and week-numbering year
        case 'W': strftime(text, sizeof text, "%V", tm); qiq_append(out, text); break;
        case 'o': strftime(text, sizeof text, "%G", tm); qiq_append(out, text); break;
        // Month
        case 'F': qiq_append(out, month_names[tm->tm_mon]); break;
        case 'm': qiq_append_number(out, tm->tm_mon + 1, 2); break;
        case 'M': qiq_append_bytes(out, month_names[tm->tm_mon], 3); break;
        case 'n': qiq_append_number(out, tm->tm_mon + 1, 0); break;
        case 't': qiq_append_number(out, qiq_days_in_month(year, tm->tm_mon + 1), 0); break;
        // Year
        case 'L': qiq_append(out, qiq_is_leap(year) ? "1" : "0"); break;
        case 'Y': qiq_append_year(out, year); break;
        case 'y': qiq_append_number(out, (year < 0 ? -year : year) % 100, 2); break;
        // Time
        case 'a': qiq_append(out, tm->tm_hour < 12 ? "am" : "pm"); break;
        case 'A': qiq_append(out, tm->tm_hour < 12 ? "AM" : "PM"); break;
        case 'B': {
            long long seconds = ((timestamp + 3600) % 86400 + 86400) % 86400;
            qiq_append_number(out, seconds * 10 / 864, 3);
            break;
        }
        case 'g': qiq_append_number(out, hour12, 0); break;
        case 'G': qiq_append_number(out, tm->tm_hour, 0); break;
        case 'h': qiq_append_number(out, hour12, 2); break;
        case 'H': qiq_append_number(out, tm->tm_hour, 2); break;
        case 'i': qiq_append_number(out, tm->tm_min, 2); break;
        case 's': qiq_append_number(out, tm->tm_sec, 2); break;
        case 'u': qiq_append_number(out, micro, 6); break;
        case 'v': qiq_append_number(out, micro / 1000, 3); break;
        // Timezone
        case 'e': qiq_append(out, zone != NULL ? zone : utc ? "UTC" : qiq_zone_name(zone_buffer, sizeof zone_buffer)); break;
        case 'I': qiq_append(out, !utc && tm->tm_isdst > 0 ? "1" : "0"); break;
        case 'O': qiq_append_offset(out, offset, false); break;
        case 'P': qiq_append_offset(out, offset, true); break;
        case 'p':
            if (offset == 0) qiq_append(out, "Z");
            else qiq_append_offset(out, offset, true);
            break;
        case 'T': qiq_append(out, utc ? "GMT" : (tm->tm_zone ? tm->tm_zone : "")); break;
        case 'Z': qiq_append_number(out, offset, 0); break;
        // Full date/time
        case 'c': qiq_format(out, "Y-m-d\\TH:i:sP", timestamp, tm, utc, micro, zone); break;
        case 'r': qiq_format(out, "D, d M Y H:i:s O", timestamp, tm, utc, micro, zone); break;
        case 'U': qiq_append_number(out, timestamp, 0); break;
        case '\\':
            if (c[1] != '\0') c++;
            /* fall through */
        default: {
            // Copy the character, or all bytes of a UTF-8 character.
            size_t width = 1;
            while ((c[width] & 0xc0) == 0x80) width++;
            qiq_append_bytes(out, c, width);
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
    qiq_format(&out, format, timestamp, &tm, utc, 0, NULL);
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

// Days since 1970-01-01 of a date (Howard Hinnant's days_from_civil).
static int64_t qiq_days_from_civil_time(int64_t year, int month, int day) {
    year -= month <= 2;
    int64_t era = (year >= 0 ? year : year - 399) / 400;
    int64_t year_of_era = year - era * 400;
    int64_t day_of_year = (153 * (month + (month > 2 ? -3 : 9)) + 2) / 5 + day - 1;
    int64_t day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    return era * 146097 + day_of_era - 719468;
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
// DateTime (stdlib/date.qiq): an instant, as microseconds since 1970 UTC,
// shown in a time zone of its own (tz.c).

const char *qiq_time_default_zone(void) {
    return qiq_zone;
}

static int64_t qiq_floor_div(int64_t a, int64_t b) {
    return a / b - (a % b != 0 && (a < 0) != (b < 0));
}

static const QiqZone *qiq_datetime_zone(const char *name) {
    const QiqZone *zone = qiq_zone_find(name != NULL ? name : qiq_default_zone_name());
    if (zone == NULL) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION,
                   "unknown time zone \"%s\": use a name from the tz database such as \"Europe/London\", \"UTC\", or an offset such as \"+05:30\"",
                   name);
    }
    return zone;
}

// The zone's name as DateTime keeps it ("+0530" is "+05:30"), or the
// default zone's for null.
QIQ_GUARDED(char *, qiq_stdlib___zone_name, NULL, (const char *name), (name)) {
    const char *canonical = qiq_zone_name_of(qiq_datetime_zone(name));
    return qiq_string_from(canonical, strlen(canonical));
}

QIQ_GUARDED(int64_t, qiq_stdlib___zone_offset, 0, (const char *name, int64_t seconds), (name, seconds)) {
    return qiq_zone_offset(qiq_datetime_zone(name), seconds, NULL, NULL);
}

// `prefer` picks between the two instants of a time that happens twice
// (when it's one of their offsets); anything else picks the second.
QIQ_GUARDED(int64_t, qiq_stdlib___zone_local_to_utc, 0, (const char *name, int64_t local, int64_t prefer), (name, local, prefer)) {
    return qiq_zone_local_to_utc(qiq_datetime_zone(name), local, prefer >= -86400 && prefer <= 86400 ? (int32_t)prefer : QIQ_ZONE_LATER);
}

int64_t qiq_stdlib___datetime_now(void) {
    struct timespec now = qiq_clock(CLOCK_REALTIME);
    return (int64_t)now.tv_sec * 1000000 + now.tv_nsec / 1000;
}

QIQ_GUARDED(char *, qiq_stdlib___datetime_format, NULL, (const char *format, int64_t micros, const char *name), (format, micros, name)) {
    if (format == NULL) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "format is null");
    const QiqZone *zone = qiq_datetime_zone(name);
    int64_t seconds = qiq_floor_div(micros, 1000000);
    int micro = (int)(micros - seconds * 1000000);
    bool dst;
    const char *abbreviation;
    int32_t offset = qiq_zone_offset(zone, seconds, &dst, &abbreviation);
    time_t local = (time_t)(seconds + offset);
    struct tm tm;
    if (gmtime_r(&local, &tm) == NULL) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "the date is out of range");
    tm.tm_gmtoff = offset;
    tm.tm_isdst = dst;
    tm.tm_zone = abbreviation;
    QiqBuffer out = {0};
    qiq_append(&out, "");
    qiq_format(&out, format, seconds, &tm, false, micro, qiq_zone_name_of(zone));
    char *text = qiq_string_from(out.data, out.length);
    free(out.data);
    return text;
}

// Parsing DateTime::parse's text: the fields found, then the instant.
typedef struct {
    int64_t year;
    int month, day, hour, minute, second, micro;
    int hour12;               // 0, or 1 for am and 2 for pm with a 12-hour clock
    bool has_offset;
    int32_t offset;
    const QiqZone *zone;      // from 'e' or 'T' in the format
    bool has_timestamp;
    int64_t timestamp;
} QiqParsed;

// The zone a parsed time is in, for DateTime::parse to use.
static char qiq_parsed_zone[256];

char *qiq_stdlib___datetime_parsed_zone(void) {
    return qiq_string_from(qiq_parsed_zone, strlen(qiq_parsed_zone));
}

// Up to `most` digits (at least `least`).
static bool qiq_read_number(const char **cursor, int least, int most, int64_t *value) {
    const char *c = *cursor;
    int count = 0;
    *value = 0;
    while (count < most && *c >= '0' && *c <= '9') {
        *value = *value * 10 + (*c++ - '0');
        count++;
    }
    if (count < least) return false;
    *cursor = c;
    return true;
}

// A fraction of a second, as microseconds: ".5" is 500000; digits past six are dropped.
static bool qiq_read_fraction(const char **cursor, int *micro) {
    const char *c = *cursor;
    int digits = 0, value = 0;
    while (*c >= '0' && *c <= '9') {
        if (digits < 6) value = value * 10 + (*c - '0');
        digits++;
        c++;
    }
    if (digits == 0) return false;
    for (int i = digits; i < 6; i++) value *= 10;
    *micro = value;
    *cursor = c;
    return true;
}

// Z, or [+-]HH[[:]MM]: an offset from UTC.
static bool qiq_read_offset(const char **cursor, int32_t *offset) {
    const char *c = *cursor;
    if (*c == 'Z' || *c == 'z') {
        *offset = 0;
        *cursor = c + 1;
        return true;
    }
    if (*c != '+' && *c != '-') return false;
    int sign = *c++ == '-' ? -1 : 1;
    int64_t hours, minutes = 0;
    if (!qiq_read_number(&c, 2, 2, &hours)) return false;
    if (*c == ':') c++;
    if (*c >= '0' && *c <= '9' && !qiq_read_number(&c, 2, 2, &minutes)) return false;
    if (hours > 18 || minutes > 59) return false;
    *offset = (int32_t)(sign * (hours * 3600 + minutes * 60));
    *cursor = c;
    return true;
}

// ISO 8601 and what databases write: "2026-11-22", "2026-11-22 10:30",
// "2026-11-22T10:30:00.125Z", "2026-11-22 10:30:00+01" (PostgreSQL),
// "2026-11-22T10:30:00+01:00".
static bool qiq_parse_standard(const char *text, QiqParsed *parsed) {
    const char *c = text;
    int64_t year, month, day, hour = 0, minute = 0, second = 0;
    if (*c == '+' || *c == '-') return false;
    if (!qiq_read_number(&c, 4, 4, &year) || *c++ != '-' || !qiq_read_number(&c, 2, 2, &month) || *c++ != '-' ||
        !qiq_read_number(&c, 2, 2, &day)) {
        return false;
    }
    parsed->year = year, parsed->month = (int)month, parsed->day = (int)day;
    if (*c == 'T' || *c == 't' || *c == ' ') {
        c++;
        if (!qiq_read_number(&c, 2, 2, &hour) || *c++ != ':' || !qiq_read_number(&c, 2, 2, &minute)) return false;
        if (*c == ':') {
            c++;
            if (!qiq_read_number(&c, 2, 2, &second)) return false;
            if ((*c == '.' || *c == ',') && (c++, !qiq_read_fraction(&c, &parsed->micro))) return false;
        }
        if (*c == ' ' && (c[1] == '+' || c[1] == '-' || c[1] == 'Z')) c++;
        if (*c != '\0') {
            if (strcmp(c, "UTC") == 0) {
                parsed->has_offset = true;
                parsed->offset = 0;
                c += 3;
            } else {
                if (!qiq_read_offset(&c, &parsed->offset)) return false;
                parsed->has_offset = true;
            }
        }
    }
    parsed->hour = (int)hour, parsed->minute = (int)minute, parsed->second = (int)second;
    return *c == '\0';
}

static int qiq_read_name(const char **cursor, const char *const *names, int count, bool short_form) {
    for (int i = 0; i < count; i++) {
        size_t length = short_form ? 3 : strlen(names[i]);
        if (strncasecmp(*cursor, names[i], length) == 0) {
            *cursor += length;
            return i;
        }
    }
    return -1;
}

// With PHP's DateTime::createFromFormat letters. Fields the format doesn't
// give are those of 1970-01-01 00:00:00.
static bool qiq_parse_format(const char *text, const char *format, QiqParsed *parsed) {
    const char *c = text;
    for (const char *f = format; *f; f++) {
        int64_t value;
        switch (*f) {
        case 'd': case 'j':
            if (!qiq_read_number(&c, *f == 'd' ? 2 : 1, 2, &value)) return false;
            parsed->day = (int)value;
            break;
        case 'm': case 'n':
            if (!qiq_read_number(&c, *f == 'm' ? 2 : 1, 2, &value)) return false;
            parsed->month = (int)value;
            break;
        case 'M': case 'F': {
            int month = qiq_read_name(&c, month_names, 12, *f == 'M');
            if (month < 0) return false;
            parsed->month = month + 1;
            break;
        }
        case 'D': case 'l':
            if (qiq_read_name(&c, day_names, 7, *f == 'D') < 0) return false;
            break;
        case 'Y':
            if (!qiq_read_number(&c, 4, 4, &value)) return false;
            parsed->year = value;
            break;
        case 'y':
            if (!qiq_read_number(&c, 2, 2, &value)) return false;
            parsed->year = value < 70 ? 2000 + value : 1900 + value;
            break;
        case 'H': case 'G': case 'h': case 'g':
            if (!qiq_read_number(&c, *f == 'H' || *f == 'h' ? 2 : 1, 2, &value)) return false;
            parsed->hour = (int)value;
            if (*f == 'h' || *f == 'g') {
                if (value < 1 || value > 12) return false;
                if (parsed->hour12 == 0) parsed->hour12 = -1; // until A or a says which
            }
            break;
        case 'A': case 'a':
            if (strncasecmp(c, "am", 2) == 0) parsed->hour12 = 1;
            else if (strncasecmp(c, "pm", 2) == 0) parsed->hour12 = 2;
            else return false;
            c += 2;
            break;
        case 'i':
            if (!qiq_read_number(&c, 2, 2, &value)) return false;
            parsed->minute = (int)value;
            break;
        case 's':
            if (!qiq_read_number(&c, 2, 2, &value)) return false;
            parsed->second = (int)value;
            break;
        case 'u':
            if (!qiq_read_fraction(&c, &parsed->micro)) return false;
            break;
        case 'v':
            if (!qiq_read_number(&c, 3, 3, &value)) return false;
            parsed->micro = (int)value * 1000;
            break;
        case 'U': {
            int sign = 1;
            if (*c == '-') sign = -1, c++;
            if (!qiq_read_number(&c, 1, 18, &value)) return false;
            parsed->has_timestamp = true;
            parsed->timestamp = sign * value;
            break;
        }
        case 'O': case 'P': case 'p':
            if (!qiq_read_offset(&c, &parsed->offset)) return false;
            parsed->has_offset = true;
            break;
        case 'e': case 'T': {
            // A zone name ("Europe/London", "UTC") or an offset.
            if (*c == '+' || *c == '-' || *c == 'Z') {
                if (!qiq_read_offset(&c, &parsed->offset)) return false;
                parsed->has_offset = true;
                break;
            }
            char name[128];
            size_t length = 0;
            while (length + 1 < sizeof name && ((*c >= 'a' && *c <= 'z') || (*c >= 'A' && *c <= 'Z') || (*c >= '0' && *c <= '9') ||
                                                strchr("/_-+", *c) != NULL) && *c != '\0') {
                name[length++] = *c++;
            }
            name[length] = '\0';
            if (strcmp(name, "GMT") == 0) snprintf(name, sizeof name, "UTC");
            parsed->zone = qiq_zone_find(name);
            if (parsed->zone == NULL) return false;
            break;
        }
        case '\\':
            if (f[1] != '\0') f++;
            /* fall through */
        default:
            if (*c != *f) return false;
            c++;
        }
    }
    if (parsed->hour12 != 0) {
        if (parsed->hour12 == -1) return false; // h or g without am/pm
        parsed->hour = parsed->hour % 12 + (parsed->hour12 == 2 ? 12 : 0);
    }
    return *c == '\0';
}

static const char *const qiq_month_names_for_errors[] = {"January", "February", "March", "April", "May", "June", "July",
                                                          "August", "September", "October", "November", "December"};

// Checks a date and time's fields; `caller` names the function for errors.
static void qiq_check_fields(int64_t year, int64_t month, int64_t day, int64_t hour, int64_t minute, int64_t second, int64_t micro,
                             const char *caller) {
    if (year < -9999 || year > 9999) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "%s: year %lld is out of range (-9999 to 9999)", caller, (long long)year);
    if (month < 1 || month > 12) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "%s: month %lld is not between 1 and 12", caller, (long long)month);
    if (day < 1 || day > qiq_days_in_month(year, (int)month)) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "%s: day %lld doesn't exist in %s %lld", caller, (long long)day,
                   qiq_month_names_for_errors[month - 1], (long long)year);
    }
    if (hour < 0 || hour > 23 || minute < 0 || minute > 59 || second < 0 || second > 59) {
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "%s: %02lld:%02lld:%02lld isn't a time of day", caller, (long long)hour, (long long)minute,
                   (long long)second);
    }
    if (micro < 0 || micro > 999999) qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "%s: microsecond %lld is not between 0 and 999999", caller, (long long)micro);
}

QIQ_GUARDED_VOID(qiq_stdlib___datetime_check, (int64_t year, int64_t month, int64_t day, int64_t hour, int64_t minute, int64_t second, int64_t micro, const char *caller),
                 (year, month, day, hour, minute, second, micro, caller)) {
    qiq_check_fields(year, month, day, hour, minute, second, micro, caller);
}

// Parses `text` (with `format`, or as ISO 8601 when it's null) into
// microseconds since 1970 UTC. A time without an offset or zone of its own
// is in `zone` (or the default zone); the zone it's in is kept for
// __datetime_parsed_zone.
QIQ_GUARDED(int64_t, qiq_stdlib___datetime_parse, 0, (const char *text, const char *format, const char *name), (text, format, name)) {
    if (text == NULL) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "DateTime::parse: the text is null");
    QiqParsed parsed = {1970, 1, 1, 0, 0, 0, 0, 0, false, 0, NULL, false, 0};
    bool ok = format == NULL ? qiq_parse_standard(text, &parsed) : qiq_parse_format(text, format, &parsed);
    if (!ok) {
        if (format == NULL) {
            qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION,
                       "DateTime::parse: \"%s\" isn't an ISO 8601 date and time, such as 2026-11-22, 2026-11-22 10:30:00, or "
                       "2026-11-22T10:30:00+01:00; give the format of others, as in DateTime::parse(text, \"d/m/Y H:i\")",
                       text);
        }
        qiq_throwf(QIQ_INVALID_ARGUMENT_EXCEPTION, "DateTime::parse: \"%s\" doesn't match the format \"%s\"", text, format);
    }
    const QiqZone *zone = qiq_datetime_zone(name);
    if (parsed.has_timestamp) {
        snprintf(qiq_parsed_zone, sizeof qiq_parsed_zone, "%s", parsed.has_offset || parsed.zone != NULL || name != NULL ? qiq_zone_name_of(zone) : "UTC");
        return parsed.timestamp * 1000000 + parsed.micro;
    }
    qiq_check_fields(parsed.year, parsed.month, parsed.day, parsed.hour, parsed.minute, parsed.second, parsed.micro, "DateTime::parse");
    int64_t local = (qiq_days_from_civil_time(parsed.year, parsed.month, parsed.day) * 86400) + parsed.hour * 3600 + parsed.minute * 60 + parsed.second;
    int64_t utc;
    if (parsed.has_offset) {
        // In the zone the offset names: "+01:00", or UTC.
        char offset[16];
        int32_t magnitude = parsed.offset < 0 ? -parsed.offset : parsed.offset;
        if (parsed.offset == 0) snprintf(offset, sizeof offset, "UTC");
        else snprintf(offset, sizeof offset, "%c%02d:%02d", parsed.offset < 0 ? '-' : '+', magnitude / 3600, magnitude / 60 % 60);
        snprintf(qiq_parsed_zone, sizeof qiq_parsed_zone, "%s", qiq_zone_name_of(qiq_datetime_zone(offset)));
        utc = local - parsed.offset;
    } else {
        if (parsed.zone != NULL) zone = parsed.zone;
        snprintf(qiq_parsed_zone, sizeof qiq_parsed_zone, "%s", qiq_zone_name_of(zone));
        utc = qiq_zone_local_to_utc(zone, local, QIQ_ZONE_LATER);
    }
    return utc * 1000000 + parsed.micro;
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
