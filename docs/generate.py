#!/usr/bin/env python3
"""Builds the qiq manual (docs/html/) from docs/src/.

    python3 docs/generate.py           # write docs/html/
    python3 docs/generate.py --check   # also compile and run every example that
                                       # shows its output, and compare (needs a
                                       # built ./target/release/qiq)

Sources:
  src/language/*.html  language pages: an HTML fragment whose first line is
                       `<!-- title: ... -->`. Code goes in fenced blocks:
                       ```qiq (or ```sh) ... ```, optionally followed by an
                       ```output block with what the program prints.
  src/stdlib.py        the standard library reference: categories, functions,
                       and exception classes.

No dependencies beyond Python 3.
"""

import html
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from std_imports import add_imports, import_lines, module_of, used  # noqa: E402

ROOT = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(ROOT, "src")
OUT = os.path.join(ROOT, "html")
QIQ = os.path.join(ROOT, "..", "target", "release", "qiq")

sys.path.insert(0, SRC)
import stdlib  # noqa: E402

# Language pages, in menu order: (section, file slug, menu label).
LANGUAGE = [
    ("Getting started", "introduction", "Introduction"),
    ("Getting started", "installation", "Installing and running"),
    ("Language reference", "basic-syntax", "Basic syntax"),
    ("Language reference", "types", "Types"),
    ("Language reference", "variables", "Variables"),
    ("Language reference", "operators", "Operators"),
    ("Language reference", "control-structures", "Control structures"),
    ("Language reference", "functions", "Functions"),
    ("Language reference", "anonymous-functions", "Anonymous functions"),
    ("Language reference", "strings", "Strings and chars"),
    ("Language reference", "arrays", "Arrays"),
    ("Language reference", "maps", "Maps"),
    ("Language reference", "classes", "Classes and objects"),
    ("Language reference", "static", "Static members"),
    ("Language reference", "interfaces", "Interfaces"),
    ("Language reference", "exceptions", "Exceptions"),
    ("Language reference", "modules", "Modules"),
    ("Language reference", "packages", "Packages"),
    ("Language reference", "memory", "Memory management"),
    ("Language reference", "printing", "Printing"),
]

# ---------------------------------------------------------------------------
# Syntax highlighting
# ---------------------------------------------------------------------------

KEYWORDS = set(
    "function return try catch finally throw if else new foreach while switch case default as "
    "break continue class abstract final static public private protected extends interface "
    "implements parent print import from extern self this".split()
)
TYPES = set("int double bool string char byte void map".split())
LITERALS = set("true false null".split())
TOKEN = re.compile(
    r"(?P<comment>//[^\n]*|/\*.*?(?:\*/|$))"
    r"|(?P<string>\"(?:\\.|[^\"\\])*\")"
    r"|(?P<raw>`[^`]*`?)"
    r"|(?P<char>'(?:\\.|[^'\\])')"
    r"|(?P<number>\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b)"
    r"|(?P<word>[A-Za-z_][A-Za-z0-9_]*)"
    r"|(?P<other>.)",
    re.S,
)


def highlight_qiq(code):
    out = []
    for match in TOKEN.finditer(code):
        kind = match.lastgroup
        text = html.escape(match.group())
        if kind == "word":
            word = match.group()
            if word in KEYWORDS:
                kind = "kw"
            elif word in TYPES:
                kind = "type"
            elif word in LITERALS:
                kind = "lit"
            elif word[0].isupper():
                kind = "cls"
            elif code[match.end():match.end() + 1] == "(":
                kind = "fn"
            else:
                out.append(text)
                continue
        elif kind == "raw":
            kind = "string"
        elif kind == "other":
            out.append(text)
            continue
        out.append(f'<span class="{kind}">{text}</span>')
    return "".join(out)


def highlight_sh(code):
    lines = []
    for line in code.split("\n"):
        if "#" in line:
            command, comment = line.split("#", 1)
            lines.append(f'{html.escape(command)}<span class="comment">#{html.escape(comment)}</span>')
        else:
            lines.append(html.escape(line))
    return "\n".join(lines)


def code_block(language, code, label=None):
    body = highlight_qiq(code) if language == "qiq" else highlight_sh(code) if language == "sh" else html.escape(code)
    caption = f'<div class="code-label">{html.escape(label)}</div>' if label else ""
    return f'<div class="code {language}">{caption}<pre><code>{body}</code></pre></div>'


def output_block(text):
    return (
        '<div class="output"><div class="output-label">Output</div>'
        f"<pre>{html.escape(text)}</pre></div>"
    )


FENCE = re.compile(r"```(qiq|sh|output|text)\n(.*?)\n```(?:\n+```output\n(.*?)\n```)?", re.S)


def render_fences(fragment, examples, page):
    """Replaces fenced blocks with highlighted HTML; collects runnable examples."""

    def replace(match):
        language, code, output = match.group(1), match.group(2), match.group(3)
        if language == "qiq":
            code = add_imports(code)
        if language == "qiq" and output is not None:
            examples.append((page, code, output))
        rendered = code_block(language, code)
        if output is not None:
            rendered += output_block(output)
        return rendered

    return FENCE.sub(replace, fragment)


# ---------------------------------------------------------------------------
# Pages
# ---------------------------------------------------------------------------


def slug_of_function(name):
    return f"function.{name.replace('_', '-')}.html"


def category_page(category):
    return f"ref.{category['slug']}.html"


def nav_entries():
    """(section, href, label, children) in menu order."""
    entries = []
    for section, slug, label in LANGUAGE:
        entries.append((section, f"{slug}.html", label, []))
    entries.append(("Standard library", "stdlib.html", "Overview", []))
    for category in stdlib.CATEGORIES:
        children = [(slug_of_function(f["name"]), f["name"]) for f in category["functions"]]
        entries.append(("Standard library", category_page(category), category["title"], children))
    for href, title, _, _ in stdlib.CLASS_PAGES:
        entries.append(("Standard library", href, title, []))
    return entries


def render_nav(current):
    parts = []
    section = None
    for entry_section, href, label, children in nav_entries():
        if entry_section != section:
            if section is not None:
                parts.append("</ul>")
            section = entry_section
            parts.append(f'<h2>{html.escape(section)}</h2><ul>')
        active = href == current or any(child == current for child, _ in children)
        cls = ' class="active"' if href == current else ""
        parts.append(f'<li><a href="{href}"{cls}>{html.escape(label)}</a>')
        if children and active:
            parts.append('<ul class="children">')
            for child_href, child_label in children:
                child_cls = ' class="active"' if child_href == current else ""
                parts.append(f'<li><a href="{child_href}"{child_cls}><code>{html.escape(child_label)}</code></a></li>')
            parts.append("</ul>")
        parts.append("</li>")
    parts.append("</ul>")
    return "\n".join(parts)


def page_order():
    """Every page in reading order, for previous/next links."""
    order = []
    for _, href, label, children in nav_entries():
        order.append((href, label))
        order.extend(children)
    return order


def layout(href, title, body, breadcrumbs):
    order = page_order()
    index = next((i for i, (h, _) in enumerate(order) if h == href), None)
    pager = ""
    if index is not None:
        previous = order[index - 1] if index > 0 else None
        following = order[index + 1] if index + 1 < len(order) else None
        pager = '<nav class="pager">'
        pager += f'<a class="prev" href="{previous[0]}">« {html.escape(previous[1])}</a>' if previous else "<span></span>"
        pager += f'<a class="next" href="{following[0]}">{html.escape(following[1])} »</a>' if following else "<span></span>"
        pager += "</nav>"
    crumbs = " › ".join(
        [f'<a href="{h}">{html.escape(t)}</a>' for h, t in breadcrumbs] + [f"<span>{html.escape(title)}</span>"]
    )
    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{html.escape(title)} - qiq Manual</title>
<link rel="stylesheet" href="style.css">
</head>
<body>
<header class="topbar">
  <button class="menu-toggle" aria-label="Show menu" onclick="document.body.classList.toggle('menu-open')">☰</button>
  <a class="brand" href="introduction.html"><span class="logo">qiq</span><span class="brand-text">Manual</span></a>
  <div class="search">
    <input id="search" type="search" placeholder="Search the manual" autocomplete="off" aria-label="Search the manual">
    <ul id="search-results" hidden></ul>
  </div>
</header>
<div class="layout">
<nav class="sidebar" aria-label="Manual">
{render_nav(href)}
</nav>
<main class="content">
<div class="breadcrumbs">{crumbs}</div>
{body}
{pager}
</main>
</div>
<script src="search.js"></script>
</body>
</html>
"""


def write(name, text):
    with open(os.path.join(OUT, name), "w", encoding="utf-8") as file:
        file.write(text)


def read_language_page(slug):
    with open(os.path.join(SRC, "language", f"{slug}.html"), encoding="utf-8") as file:
        text = file.read()
    match = re.match(r"<!-- title: (.*?) -->\n", text)
    if not match:
        raise SystemExit(f"src/language/{slug}.html must start with <!-- title: ... -->")
    return match.group(1), text[match.end():]


def signature_html(function):
    params = []
    for param in function["params"]:
        default = f' = <span class="default">{html.escape(param["default"])}</span>' if param.get("default") else ""
        params.append(
            f'<span class="type">{html.escape(param["type"])}</span> <span class="param">{html.escape(param["name"])}</span>{default}'
        )
    return (
        f'<div class="synopsis"><code><span class="fn">{function["name"]}</span>('
        + ", ".join(params)
        + f'): <span class="type">{html.escape(function["returns"])}</span></code></div>'
    )


PRE_CODE = re.compile(r'<pre class="code">(.*?)</pre>', re.S)


def add_imports_html(fragment):
    """Adds the standard library imports that hand-written <pre class="code">
    examples need, so they're complete programs like the generated ones."""

    def replace(match):
        text = html.unescape(re.sub(r"<[^>]+>", "", match.group(1)))
        lines = import_lines(used(text))
        if not lines:
            return match.group(0)
        imports = "".join(highlight_qiq(line) + "\n" for line in lines)
        return f'<pre class="code">{imports}\n{match.group(1)}</pre>'

    return PRE_CODE.sub(replace, fragment)


def class_import_html(title):
    """The import line for a class page titled "The File class"."""
    match = re.fullmatch(r"The (\w+) class", title)
    return import_html(match.group(1)) if match else ""


def import_html(name):
    """How to import a standard library name, or nothing for the prelude."""
    module = module_of(name)
    if module is None:
        return ""
    return f'<p class="import">Import it with <code>import {name} from "{module}";</code></p>'


def function_page(category, function, examples):
    href = slug_of_function(function["name"])
    parts = [
        f'<h1 class="refname"><code>{function["name"]}</code></h1>',
        f'<p class="refpurpose"><code>{function["name"]}</code> — {function["purpose"]}</p>',
        "<h2>Description</h2>",
        signature_html(function),
        import_html(function["name"]),
        f'<p>{function["description"]}</p>',
        "<h2>Parameters</h2>",
    ]
    if function["params"]:
        parts.append('<dl class="parameters">')
        for param in function["params"]:
            parts.append(f'<dt><code>{html.escape(param["name"])}</code></dt><dd>{param["description"]}</dd>')
        parts.append("</dl>")
    else:
        parts.append("<p>This function has no parameters.</p>")
    parts.append(f'<h2>Return values</h2><p>{function["return_description"]}</p>')
    if function.get("errors"):
        parts.append("<h2>Errors/Exceptions</h2><ul>")
        parts.extend(f"<li>{error}</li>" for error in function["errors"])
        parts.append("</ul>")
    if function.get("examples"):
        parts.append("<h2>Examples</h2>")
        for number, example in enumerate(function["examples"], 1):
            parts.append(f'<div class="example"><p class="example-title"><strong>Example #{number}</strong> {example["title"]}</p>')
            code = add_imports(example["code"])
            parts.append(code_block("qiq", code))
            if example.get("output") is not None:
                parts.append(output_block(example["output"]))
                examples.append((href, code, example["output"]))
            parts.append("</div>")
    if function.get("notes"):
        parts.append("<h2>Notes</h2>")
        parts.extend(f'<div class="note"><strong>Note:</strong> {note}</div>' for note in function["notes"])
    if function.get("see_also"):
        links = ", ".join(
            f'<a href="{slug_of_function(name)}"><code>{name}</code></a>' for name in function["see_also"]
        )
        parts.append(f"<h2>See also</h2><p>{links}</p>")
    breadcrumbs = [("stdlib.html", "Standard library"), (category_page(category), category["title"])]
    write(href, layout(href, function["name"], "\n".join(parts), breadcrumbs))


def category_listing(category):
    rows = "".join(
        f'<tr><td><a href="{slug_of_function(f["name"])}"><code>{f["name"]}</code></a></td><td>{f["purpose"]}</td></tr>'
        for f in category["functions"]
    )
    return f'<table class="listing"><thead><tr><th>Function</th><th>Description</th></tr></thead><tbody>{rows}</tbody></table>'


def build(examples):
    if os.path.isdir(OUT):
        shutil.rmtree(OUT)
    os.makedirs(OUT)
    shutil.copy(os.path.join(SRC, "style.css"), os.path.join(OUT, "style.css"))
    search_index = []

    for section, slug, label in LANGUAGE:
        title, fragment = read_language_page(slug)
        href = f"{slug}.html"
        body = render_fences(fragment, examples, href)
        write(href, layout(href, title, body, [(f"{LANGUAGE[0][1]}.html", "Manual"), ("#", section)]))
        search_index.append({"title": title, "url": href, "kind": section})
    # The manual opens at the introduction.
    write("index.html", '<!DOCTYPE html><meta charset="utf-8"><meta http-equiv="refresh" content="0; url=introduction.html"><title>qiq Manual</title><a href="introduction.html">qiq Manual</a>\n')

    overview = [
        "<h1>Standard library</h1>",
        f"<p>{stdlib.OVERVIEW}</p>",
    ]
    for category in stdlib.CATEGORIES:
        overview.append(f'<h2><a href="{category_page(category)}">{category["title"]}</a></h2><p>{category["intro"]}</p>')
        overview.append(category_listing(category))
    for href, title, summary, _ in stdlib.CLASS_PAGES:
        overview.append(f'<h2><a href="{href}">{html.escape(title)}</a></h2><p>{summary}</p>')
    write("stdlib.html", layout("stdlib.html", "Standard library", "\n".join(overview), [("introduction.html", "Manual")]))
    search_index.append({"title": "Standard library", "url": "stdlib.html", "kind": "Standard library"})

    for category in stdlib.CATEGORIES:
        href = category_page(category)
        body = f'<h1>{category["title"]}</h1><p>{category["intro"]}</p>'
        body += add_imports_html(render_fences(category.get("details", ""), examples, href))
        body += "<h2>Functions</h2>" + category_listing(category)
        write(href, layout(href, category["title"], body, [("stdlib.html", "Standard library")]))
        search_index.append({"title": category["title"], "url": href, "kind": "Standard library"})
        for function in category["functions"]:
            function_page(category, function, examples)
            search_index.append({"title": function["name"], "url": slug_of_function(function["name"]), "kind": function["purpose"]})

    for href, title, _, source in stdlib.CLASS_PAGES:
        body = add_imports_html(render_fences(source, examples, href))
        # The import line goes after the page's first paragraph.
        first = body.find("</p>")
        if first >= 0:
            body = body[: first + 4] + class_import_html(title) + body[first + 4 :]
        write(href, layout(href, title, body, [("stdlib.html", "Standard library")]))
        search_index.append({"title": title, "url": href, "kind": "Standard library"})

    with open(os.path.join(SRC, "search.js"), encoding="utf-8") as file:
        script = file.read()
    write("search.js", f"const SEARCH_INDEX = {json.dumps(search_index)};\n{script}")
    return search_index


# ---------------------------------------------------------------------------
# --check: run the examples
# ---------------------------------------------------------------------------


def check(examples):
    if not os.path.exists(QIQ):
        raise SystemExit(f"--check needs a built compiler at {QIQ} (cargo build --release)")
    failures = 0
    for page, code, expected in examples:
        with tempfile.TemporaryDirectory() as work:
            source = os.path.join(work, "example.qiq")
            with open(source, "w", encoding="utf-8") as file:
                file.write(code + "\n")
            result = subprocess.run([QIQ, "run", source], cwd=work, capture_output=True, text=True)
            actual = (result.stdout + result.stderr).rstrip("\n")
            if actual != expected.rstrip("\n"):
                failures += 1
                print(f"FAIL {page}\n--- code\n{code}\n--- expected\n{expected}\n--- actual\n{actual}\n")
    print(f"{len(examples) - failures} of {len(examples)} examples print what the manual says")
    return failures == 0


def main():
    examples = []
    pages = build(examples)
    print(f"wrote {len(pages) + 1} pages to {os.path.relpath(OUT)}")
    if "--check" in sys.argv and not check(examples):
        sys.exit(1)


if __name__ == "__main__":
    main()
