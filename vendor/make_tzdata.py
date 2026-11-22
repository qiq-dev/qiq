#!/usr/bin/env python3
"""Builds vendor/tzdata.bin, the time zone database qiq programs fall back on
when the system has no zoneinfo files (as in a FROM scratch container), from
a zoneinfo directory compiled from the IANA tz database:

    python3 vendor/make_tzdata.py [/usr/share/zoneinfo]

The file is compact rather than a copy of the TZif files (about 475 KB):
each zone keeps its transitions from 1970 on, with times as variable-length
deltas, and drops the trailing transitions its POSIX rule (the TZif footer)
gives anyway; zones with the same data share one record. stdlib/runtime/tz.c
reads it. Format, all integers unsigned LEB128 unless marked zigzag:

    "QIQTZ1\\n", the tzdata version, "\\n"
    zone count, then for each zone (sorted): name, NUL, record number
    record count, then for each record:
        type count, then per type: offset (zigzag), dst (0 or 1), abbreviation, NUL
        initial type: the one in effect before the first transition (or at 1970)
        transition count, first time (zigzag), the other times as deltas,
            then one type index per transition
        the POSIX rule, NUL (empty when there is none)
"""
import os
import struct
import sys

START = 0  # 1970-01-01: history before it is left to the system's files


def leb(value):
    out = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if value:
            out.append(byte | 0x80)
        else:
            out.append(byte)
            return bytes(out)


def zigzag(value):
    return leb(value * 2 if value >= 0 else -value * 2 - 1)


def read_tzif(data):
    """(transitions [(time, type)], types [(offset, dst, abbr)], footer)."""
    if data[:4] != b"TZif":
        raise ValueError("not TZif")
    version = data[4]

    def counts(at):
        return struct.unpack(">6l", data[at + 20 : at + 44])

    isut, isstd, leap, timecnt, typecnt, charcnt = counts(0)
    at = 44
    if version >= ord("2"):
        at += timecnt * 4 + timecnt + typecnt * 6 + charcnt + leap * 8 + isstd + isut
        isut, isstd, leap, timecnt, typecnt, charcnt = counts(at)
        at += 44
        size = 8
    else:
        size = 4
    times = struct.unpack(">%d%s" % (timecnt, "q" if size == 8 else "l"), data[at : at + timecnt * size])
    at += timecnt * size
    indexes = data[at : at + timecnt]
    at += timecnt
    types = []
    for i in range(typecnt):
        offset, dst, abbr_index = struct.unpack(">lBB", data[at + i * 6 : at + i * 6 + 6])
        types.append([offset, dst, abbr_index])
    at += typecnt * 6
    chars = data[at : at + charcnt]
    at += charcnt + leap * (12 if size == 8 else 8) + isstd + isut
    for t in types:
        end = chars.index(b"\0", t[2])
        t[2] = chars[t[2] : end].decode()
    footer = ""
    if version >= ord("2"):
        rest = data[at:]
        if rest[:1] == b"\n":
            footer = rest[1 : rest.index(b"\n", 1)].decode()
    return list(zip(times, indexes)), [tuple(t) for t in types], footer


# --- POSIX TZ rules, to see which transitions the footer predicts ---------


def parse_rule(rule):
    """(std_abbr, std_offset, dst_abbr, dst_offset, start, end) or None."""
    if not rule:
        return None
    pos = 0

    def name():
        nonlocal pos
        if rule[pos] == "<":
            end = rule.index(">", pos)
            text = rule[pos + 1 : end]
            pos = end + 1
            return text
        start = pos
        while pos < len(rule) and rule[pos].isalpha():
            pos += 1
        return rule[start:pos]

    def offset():
        nonlocal pos
        sign = 1
        if rule[pos] in "+-":
            sign = -1 if rule[pos] == "-" else 1
            pos += 1
        parts = []
        while True:
            start = pos
            while pos < len(rule) and rule[pos].isdigit():
                pos += 1
            parts.append(int(rule[start:pos]))
            if pos < len(rule) and rule[pos] == ":":
                pos += 1
                continue
            break
        while len(parts) < 3:
            parts.append(0)
        return sign * (parts[0] * 3600 + parts[1] * 60 + parts[2])

    std = name()
    std_offset = -offset()  # POSIX offsets are west of UTC
    if pos == len(rule):
        return (std, std_offset, None, None, None, None)
    dst = name()
    dst_offset = std_offset + 3600
    if pos < len(rule) and rule[pos] != ",":
        dst_offset = -offset()

    def date():
        nonlocal pos
        pos += 1  # ,
        if rule[pos] == "M":
            pos += 1
            fields = []
            for _ in range(3):
                start = pos
                while pos < len(rule) and rule[pos].isdigit():
                    pos += 1
                fields.append(int(rule[start:pos]))
                if pos < len(rule) and rule[pos] == ".":
                    pos += 1
            spec = ("M", fields)
        elif rule[pos] == "J":
            pos += 1
            start = pos
            while pos < len(rule) and rule[pos].isdigit():
                pos += 1
            spec = ("J", int(rule[start:pos]))
        else:
            start = pos
            while pos < len(rule) and rule[pos].isdigit():
                pos += 1
            spec = ("N", int(rule[start:pos]))
        time = 7200
        if pos < len(rule) and rule[pos] == "/":
            pos += 1
            time = offset()
        return spec, time

    start = date()
    end = date()
    return (std, std_offset, dst, dst_offset, start, end)


def days_from_civil(y, m, d):
    y -= m <= 2
    era = (y if y >= 0 else y - 399) // 400
    yoe = y - era * 400
    doy = (153 * (m + (-3 if m > 2 else 9)) + 2) // 5 + d - 1
    doe = yoe * 365 + yoe // 4 - yoe // 100 + doy
    return era * 146097 + doe - 719468


def leap(y):
    return y % 4 == 0 and (y % 100 != 0 or y % 400 == 0)


def rule_day(spec, year):
    kind, value = spec
    if kind == "J":
        day = value - 1
        if leap(year) and value >= 60:
            day += 1
        return days_from_civil(year, 1, 1) + day
    if kind == "N":
        return days_from_civil(year, 1, 1) + value
    month, week, weekday = value
    first = days_from_civil(year, month, 1)
    first_weekday = (first + 4) % 7  # 1970-01-01 was a Thursday
    day = first + (weekday - first_weekday) % 7 + (week - 1) * 7
    if week == 5:
        days = [31, 29 if leap(year) else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][month - 1]
        while day >= first + days:
            day -= 7
    return day


def rule_at(rule, when):
    """(offset, dst, abbr) the rule gives at `when`."""
    std, std_offset, dst, dst_offset, start, end = rule
    if dst is None:
        return (std_offset, 0, std)
    # The year by the local standard calendar.
    days = (when + std_offset) // 86400
    y = int(days // 365.2425) + 1970
    while days_from_civil(y, 1, 1) > days:
        y -= 1
    while days_from_civil(y + 1, 1, 1) <= days:
        y += 1
    begin = rule_day(start[0], y) * 86400 + start[1] - std_offset
    finish = rule_day(end[0], y) * 86400 + end[1] - dst_offset
    if begin < finish:
        in_dst = begin <= when < finish
    else:
        in_dst = not (finish <= when < begin)
    return (dst_offset, 1, dst) if in_dst else (std_offset, 0, std)


def rule_transitions(rule, first_year, last_year):
    """The changes a daylight saving rule makes, as [(time, (offset, dst, abbr))]."""
    std, std_offset, dst, dst_offset, start, end = rule
    changes = []
    for year in range(first_year, last_year + 1):
        changes.append((rule_day(start[0], year) * 86400 + start[1] - std_offset, (dst_offset, 1, dst)))
        changes.append((rule_day(end[0], year) * 86400 + end[1] - dst_offset, (std_offset, 0, std)))
    return sorted(changes)


def compact(transitions, types, footer):
    """The zone's data from 1970 on, without what the footer predicts."""
    initial = 0
    for time, index in transitions:
        if time <= START:
            initial = index
    kept = [(t, i) for t, i in transitions if t > START]
    rule = parse_rule(footer)
    # A trailing transition can go when the rule makes exactly that change,
    # and makes no other between it and the transition before it: tz.c then
    # reads every time from the last transition kept on with the rule.
    if rule is not None and rule[2] is not None:
        changes = rule_transitions(rule, 1969, 2200)
        position = {time: n for n, (time, _) in enumerate(changes)}
        while kept:
            time, index = kept[-1]
            previous_time = kept[-2][0] if len(kept) > 1 else START
            previous_type = types[kept[-2][1]] if len(kept) > 1 else types[initial]
            n = position.get(time)
            if n is None or changes[n][1] != types[index]:
                break
            if n > 0 and (changes[n - 1][0] > previous_time or changes[n - 1][1] != previous_type):
                break
            if n == 0:
                break
            kept.pop()
    used = sorted({initial} | {i for _, i in kept})
    renumber = {old: new for new, old in enumerate(used)}
    record = bytearray()
    record += leb(len(used))
    for old in used:
        offset, dst, abbr = types[old]
        record += zigzag(offset) + bytes([dst]) + abbr.encode() + b"\0"
    record += leb(renumber[initial])
    record += leb(len(kept))
    previous = None
    for time, _ in kept:
        record += zigzag(time) if previous is None else leb(time - previous)
        previous = time
    record += bytes(renumber[i] for _, i in kept)
    record += footer.encode() + b"\0"
    return bytes(record)


def main():
    root = sys.argv[1] if len(sys.argv) > 1 else "/usr/share/zoneinfo"
    version = "unknown"
    try:
        with open(os.path.join(root, "tzdata.zi")) as file:
            first = file.readline()
            if first.startswith("# version "):
                version = first.split()[2]
    except OSError:
        pass
    zones = {}
    for directory, _, files in os.walk(root):
        relative = os.path.relpath(directory, root)
        if relative.split(os.sep)[0] in ("posix", "right"):
            continue
        for name in files:
            path = os.path.join(directory, name)
            with open(path, "rb") as file:
                data = file.read()
            if data[:4] != b"TZif":
                continue
            zone = name if relative == "." else os.path.join(relative, name)
            if zone in ("posixrules", "localtime", "Factory"):
                continue
            zones[zone] = compact(*read_tzif(data))
    records = []
    numbers = {}
    for record in sorted(set(zones.values())):
        numbers[record] = len(records)
        records.append(record)
    out = bytearray(b"QIQTZ1\n" + version.encode() + b"\n")
    out += leb(len(zones))
    for zone in sorted(zones):
        out += zone.encode() + b"\0" + leb(numbers[zones[zone]])
    out += leb(len(records))
    for record in records:
        out += record
    target = os.path.join(os.path.dirname(os.path.abspath(__file__)), "tzdata.bin")
    with open(target, "wb") as file:
        file.write(out)
    print(f"{target}: tzdata {version}, {len(zones)} zones, {len(records)} records, {len(out)} bytes")


if __name__ == "__main__":
    main()
