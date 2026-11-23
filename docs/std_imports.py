"""Adds `import ... from "std/..."` lines to qiq examples that use names from
the standard library modules that need importing, so every example in the
manual is a complete program."""
import os
import re

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "stdlib") + os.sep
MODULES = [
    ("file.qiq", "std/file"),
    ("date.qiq", "std/time"),
    ("memory.qiq", "std/memory"),
    ("program.qiq", "std/os"),
    ("net.qiq", "std/net"),
    ("http.qiq", "std/net/http"),
    ("server.qiq", "std/net/http/server"),
    ("crypto.qiq", "std/crypto"),
    ("sql.qiq", "std/sql"),
    ("regex.qiq", "std/regex"),
]
DECL = re.compile(r"^\s*(?:(?:extern|final|abstract)\s+)*(function|class|interface)\s+([A-Za-z_]\w*)")

NAMES = {}  # name -> (module path, kind)
for file, path in MODULES:
    for line in open(ROOT + file):
        if line.startswith((" ", "\t")):
            continue
        m = DECL.match(line)
        if m and not m.group(2).startswith("__"):
            NAMES[m.group(2)] = (path, m.group(1))
ORDER = [path for _, path in MODULES]


def strip(code):
    """Removes comments and string/char literals."""
    out = []
    i = 0
    while i < len(code):
        c = code[i]
        if code.startswith("//", i):
            j = code.find("\n", i)
            i = len(code) if j < 0 else j
        elif code.startswith("/*", i):
            j = code.find("*/", i)
            i = len(code) if j < 0 else j + 2
        elif c in "\"'":
            j = i + 1
            while j < len(code) and code[j] != c:
                j += 2 if code[j] == "\\" else 1
            out.append(' "" ')
            i = j + 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def used(code):
    bare = strip(code)
    declared = set()
    for m in re.finditer(r"\b(function|class|interface)\s+([A-Za-z_]\w*)", bare):
        declared.add(m.group(2))
    # Already imported names.
    for m in re.finditer(r"\bimport\s+(.*?)\s+from\b", bare):
        for part in m.group(1).split(","):
            words = part.split()
            if words:
                declared.add(words[-1])
                declared.add(words[0])
    found = {}
    for name, (path, kind) in NAMES.items():
        if name in declared:
            continue
        if kind == "function":
            # A call, not a method call (->name), a static call (::name), or a declaration.
            pattern = r"(?<![\w>:$])" + re.escape(name) + r"\s*\("
        else:
            # Not a named argument (name: value), though Class:: is a use.
            pattern = r"(?<![\w>$])" + re.escape(name) + r"\b(?!\s*:(?!:))"
        if re.search(pattern, bare):
            found.setdefault(path, []).append(name)
    return found


def import_lines(found):
    lines = []
    for path in ORDER:
        if path in found:
            lines.append(f'import {", ".join(sorted(found[path], key=str.lower))} from "{path}";')
    return lines


def add_imports(code):
    found = used(code)
    if not found:
        return code
    lines = import_lines(found)
    source = code.split("\n")
    # After leading comment lines; before any existing imports or code.
    index = 0
    while index < len(source) and source[index].strip().startswith("//"):
        index += 1
    block = lines + ([""] if index < len(source) and not source[index].strip().startswith("import ") and source[index].strip() != "" else [])
    return "\n".join(source[:index] + block + source[index:])


def module_of(name):
    """The import path for a standard library name, or None for the prelude."""
    entry = NAMES.get(name)
    return entry[0] if entry else None
