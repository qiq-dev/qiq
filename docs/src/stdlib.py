"""The standard library reference, read by generate.py.

Each function: name, purpose (one line), params (name, type, optional
default, description), returns (type), description, return_description,
errors, examples (title, code, output: what the program prints, checked by
`generate.py --check`), notes, see_also. Text fields are HTML.
"""


def param(name, type_, description, default=None):
    return {"name": name, "type": type_, "description": description, "default": default}


NULL_STRING = "<code>RuntimeException</code> if a string argument is <code>null</code> (for example an unset element of a <code>new string[n]</code> array)."

OVERVIEW = (
    "The everyday parts of the standard library are available in every file without an <code>import</code>, "
    "like PHP's built-in functions: the general functions (<code>len</code>, <code>print</code>, the conversions, "
    "and the array, map, and JSON functions), the string functions, and the exception classes. They can't be "
    "redeclared: declaring your own <code>substr</code> is a compile error. The rest is in modules that you "
    "import by name, like Go's packages:</p>\n"
    "<table><thead><tr><th>Import path</th><th>Contents</th></tr></thead><tbody>"
    "<tr><td><code>std/file</code></td><td><a href=\"ref.files.html\">Files</a> and the <a href=\"class.file.html\">File</a> class</td></tr>"
    "<tr><td><code>std/time</code></td><td><a href=\"ref.datetime.html\">Date and time</a></td></tr>"
    "<tr><td><code>std/os</code></td><td>The <a href=\"ref.program.html\">program</a>: arguments, exit status, environment</td></tr>"
    "<tr><td><code>std/memory</code></td><td><a href=\"function.gc-collect-cycles.html\">gc_collect_cycles</a></td></tr>"
    "<tr><td><code>std/net</code></td><td>The <a href=\"class.socket.html\">Socket</a> class</td></tr>"
    "<tr><td><code>std/http</code></td><td>The <a href=\"ref.http.html\">HTTP client</a> and URL encoding</td></tr>"
    "<tr><td><code>std/http/server</code></td><td>The <a href=\"ref.http-server.html\">HTTP server</a></td></tr>"
    "<tr><td><code>std/crypto</code></td><td><a href=\"ref.crypto.html\">Hashing and encoding</a>: MD5, SHA-1, SHA-2, HMAC, hex, base64, and password hashing</td></tr>"
    "</tbody></table>\n"
    "<pre class=\"code\">import File, file_get_contents from \"std/file\";\nimport date from \"std/time\";</pre>\n"
    "<p>Names from these modules are free for your own code when you don't import them: a program can declare its "
    "own <code>File</code> class or <code>time</code> function. Each function's page shows its import line. "
    "Strings are UTF-8, and every position and length counts characters, not bytes."
)

STRINGS = {
    "slug": "strings",
    "title": "Strings",
    "intro": (
        "Functions for working with <code>string</code> values. Strings are UTF-8, and positions and lengths "
        "count characters (Unicode scalar values), so <code>len(\"héllo\")</code> is 5 (see <a href=\"function.len.html\">len</a>). Concatenate with "
        "<code>+</code>; see <a href=\"strings.html\">Strings and chars</a>."
    ),
    "functions": [
        {
            "name": "substr",
            "purpose": "Return part of a string",
            "params": [
                param("text", "string", "The input string."),
                param("start", "int", "The character position to start at, counting from <code>0</code>. A negative <code>start</code> counts from the end: <code>-1</code> is the last character."),
                param("length", "int", "The maximum number of characters to return. A negative <code>length</code> leaves that many characters off the end. By default, everything to the end of the string.", "9223372036854775807"),
            ],
            "returns": "string",
            "description": "Returns the characters of <code>text</code> starting at <code>start</code>, at most <code>length</code> of them. Positions and lengths count characters, not bytes, following PHP's rules for negative values.",
            "return_description": "The extracted part, or an empty string when <code>start</code> is past the end or the range is empty.",
            "errors": [NULL_STRING],
            "examples": [
                {
                    "title": "Positive and negative positions",
                    "code": 'string text = "héllo wörld";\nprint(substr(text, 6));\nprint(substr(text, 1, 4));\nprint(substr(text, -5, 3));\nprint(substr(text, 0, -6));',
                    "output": "wörld\néllo\nwör\nhéllo",
                }
            ],
            "see_also": ["strpos", "len", "explode"],
        },
        {
            "name": "strpos",
            "purpose": "Find the position of the first occurrence of a substring",
            "params": [
                param("haystack", "string", "The string to search in."),
                param("needle", "string", "The string to search for."),
                param("offset", "int", "The character position to start searching from. A negative offset counts from the end.", "0"),
            ],
            "returns": "int",
            "description": "Returns the character position of the first occurrence of <code>needle</code> in <code>haystack</code>, at or after <code>offset</code>.",
            "return_description": "The character position, counting from the start of <code>haystack</code> (not from <code>offset</code>), or <code>-1</code> if <code>needle</code> isn't found. An empty <code>needle</code> is found at <code>offset</code>.",
            "errors": [
                "<code>InvalidArgumentException</code> if <code>offset</code> is outside the string.",
                NULL_STRING,
            ],
            "examples": [
                {
                    "title": "Searching with and without an offset",
                    "code": 'print(strpos("héllo wörld", "wörld"));\nprint(strpos("ab-ab", "ab", 1));\nif (strpos("hello", "z") == -1) {\n    print("not found");\n}',
                    "output": "6\n3\nnot found",
                }
            ],
            "notes": ["Unlike PHP, which returns <code>false</code>, qiq returns <code>-1</code> when nothing is found. To only test for a substring, use <code>str_contains</code>."],
            "see_also": ["str_contains", "substr"],
        },
        {
            "name": "str_contains",
            "purpose": "Determine if a string contains a substring",
            "params": [param("haystack", "string", "The string to search in."), param("needle", "string", "The substring to search for.")],
            "returns": "bool",
            "description": "Checks whether <code>needle</code> occurs anywhere in <code>haystack</code>. The comparison is case-sensitive.",
            "return_description": "<code>true</code> if <code>needle</code> occurs in <code>haystack</code>, otherwise <code>false</code>. An empty <code>needle</code> is always found.",
            "errors": [NULL_STRING],
            "examples": [
                {
                    "title": "Basic use",
                    "code": 'string text = "héllo wörld";\nif (str_contains(text, "wö")) {\n    print("found");\n}\nprint(str_contains(text, "W"));',
                    "output": "found\n0",
                }
            ],
            "see_also": ["str_starts_with", "str_ends_with", "strpos"],
        },
        {
            "name": "str_starts_with",
            "purpose": "Check if a string starts with a given substring",
            "params": [param("haystack", "string", "The string to check."), param("needle", "string", "The prefix to look for.")],
            "returns": "bool",
            "description": "Checks whether <code>haystack</code> begins with <code>needle</code>. The comparison is case-sensitive.",
            "return_description": "<code>true</code> if <code>haystack</code> starts with <code>needle</code>, otherwise <code>false</code>. Every string starts with the empty string.",
            "errors": [NULL_STRING],
            "examples": [
                {
                    "title": "Checking a prefix",
                    "code": 'string url = "https://example.com";\nif (str_starts_with(url, "https://")) {\n    print("secure");\n}\nprint(str_starts_with(url, "http:"));',
                    "output": "secure\n0",
                }
            ],
            "notes": ["This function is written in qiq on top of <code>substr</code> and <code>len</code>; see <code>stdlib/string.qiq</code>."],
            "see_also": ["str_ends_with", "str_contains"],
        },
        {
            "name": "str_ends_with",
            "purpose": "Check if a string ends with a given substring",
            "params": [param("haystack", "string", "The string to check."), param("needle", "string", "The suffix to look for.")],
            "returns": "bool",
            "description": "Checks whether <code>haystack</code> ends with <code>needle</code>. The comparison is case-sensitive.",
            "return_description": "<code>true</code> if <code>haystack</code> ends with <code>needle</code>, otherwise <code>false</code>. Every string ends with the empty string.",
            "errors": [NULL_STRING],
            "examples": [
                {
                    "title": "Checking a file extension",
                    "code": 'string name = "report.qiq";\nif (str_ends_with(name, ".qiq")) {\n    print("a qiq source file");\n}',
                    "output": "a qiq source file",
                }
            ],
            "see_also": ["str_starts_with", "str_contains"],
        },
        {
            "name": "str_replace",
            "purpose": "Replace all occurrences of a substring",
            "params": [
                param("search", "string", "The value to look for."),
                param("replace", "string", "The replacement for each occurrence of <code>search</code>."),
                param("subject", "string", "The string to search and replace in."),
            ],
            "returns": "string",
            "description": "Returns <code>subject</code> with every occurrence of <code>search</code> replaced by <code>replace</code>, scanning from left to right. The argument order is PHP's: what to find, what to put, where.",
            "return_description": "The resulting string. If <code>search</code> is empty, <code>subject</code> is returned unchanged.",
            "errors": [NULL_STRING],
            "examples": [
                {
                    "title": "Replacing text",
                    "code": 'print(str_replace("ö", "oe", "héllo wörld"));\nprint(str_replace("a", "", "banana"));',
                    "output": "héllo woerld\nbnn",
                }
            ],
            "see_also": ["str_contains", "explode", "implode"],
        },
        {
            "name": "htmlspecialchars",
            "purpose": "Escape text for HTML",
            "params": [param("text", "string", "The text to escape.")],
            "returns": "string",
            "description": "Replaces <code>&amp;</code>, <code>&lt;</code>, <code>&gt;</code>, <code>&quot;</code>, and <code>'</code> with HTML entities, as PHP's <code>htmlspecialchars</code> does by default, so text from users can be put in an HTML page or attribute without being read as markup.",
            "return_description": "The escaped text.",
            "errors": [],
            "examples": [{"title": "Escaping user input", "code": 'string name = "<b>Tom</b> & \'Jerry\'";\nprint("<p>Hello, " + htmlspecialchars(name) + "</p>");', "output": "<p>Hello, &lt;b&gt;Tom&lt;/b&gt; &amp; &#039;Jerry&#039;</p>"}],
            "see_also": ["str_replace", "rawurlencode"],
        },
        {
            "name": "sprintf",
            "purpose": "Format values into a string",
            "params": [
                param("format", "string", "Text with placeholders, described below."),
                param("values", "int|double|bool|byte|string|char...", "The values the placeholders use, in order."),
            ],
            "returns": "string",
            "description": """<p>Returns <code>format</code> with each placeholder replaced by a formatted value, as PHP's <code>sprintf</code> does. A placeholder is written <code>%[argnum$][flags][width][.precision]specifier</code>; <code>%%</code> is a literal percent sign.</p>
<table>
<thead><tr><th>Specifier</th><th>Formats the value as</th><th>Example</th></tr></thead>
<tbody>
<tr><td><code>s</code></td><td>A string; <code>.precision</code> is the maximum number of characters</td><td><code>sprintf("%.3s", "abcdef")</code> is <code>abc</code></td></tr>
<tr><td><code>d</code></td><td>An int</td><td><code>-42</code></td></tr>
<tr><td><code>u</code></td><td>An int as unsigned</td><td><code>sprintf("%u", -1)</code> is <code>18446744073709551615</code></td></tr>
<tr><td><code>f</code>, <code>F</code></td><td>A double with <code>.precision</code> decimals (default 6)</td><td><code>sprintf("%.2f", 3.14159)</code> is <code>3.14</code></td></tr>
<tr><td><code>e</code>, <code>E</code></td><td>Scientific notation with <code>.precision</code> decimals</td><td><code>sprintf("%.2e", 1234.5)</code> is <code>1.23e+3</code></td></tr>
<tr><td><code>g</code>, <code>G</code></td><td>Plain or scientific notation, whichever is shorter, with <code>.precision</code> significant digits</td><td><code>sprintf("%g", 0.00001234)</code> is <code>1.234e-5</code></td></tr>
<tr><td><code>x</code>, <code>X</code></td><td>An int in hexadecimal</td><td><code>sprintf("%x", 255)</code> is <code>ff</code></td></tr>
<tr><td><code>o</code></td><td>An int in octal</td><td><code>sprintf("%o", 8)</code> is <code>10</code></td></tr>
<tr><td><code>b</code></td><td>An int in binary</td><td><code>sprintf("%b", 5)</code> is <code>101</code></td></tr>
<tr><td><code>c</code></td><td>The character with that Unicode code point; width doesn't apply</td><td><code>sprintf("%c", 233)</code> is <code>é</code></td></tr>
</tbody>
</table>
<table>
<thead><tr><th>Before the width</th><th>Effect</th></tr></thead>
<tbody>
<tr><td><code>-</code></td><td>Left-justify within the width (the default is right)</td></tr>
<tr><td><code>+</code></td><td>Show a <code>+</code> sign on positive numbers</td></tr>
<tr><td><code>0</code></td><td>Pad with zeros instead of spaces; a sign stays in front</td></tr>
<tr><td><code>'</code><var>char</var></td><td>Pad with any character, such as <code>'*</code> or <code>'·</code></td></tr>
</tbody>
</table>
<p>The width is the minimum number of characters. <code>argnum$</code> picks a value by position, from 1, so <code>%2$s</code> uses the second value and a value can be used more than once.</p>
<p>Values are converted as PHP converts them: <code>%d</code> truncates a double and reads the number at the start of a string (<code>"12abc"</code> is 12); <code>%f</code> reads a double from a string. With <code>%s</code>, bools are <code>1</code> and <code>0</code>, and doubles are written like <a href="function.to-string.html">to_string</a> (<code>%g</code>, so <code>1e+20</code>), where PHP would write <code>1.0E+20</code>.</p>""",
            "return_description": "The formatted string.",
            "errors": [
                "When <code>format</code> is a string literal, the compiler checks it: an unknown specifier, or fewer values than the placeholders use, is a compile error. Values must be ints, doubles, bools, bytes, strings, or chars.",
                "<code>InvalidArgumentException</code> for the same mistakes in a format that isn't a literal, and for <code>%c</code> with a number that isn't a Unicode code point.",
                "<code>RuntimeException</code> if a string value or the format is <code>null</code>.",
            ],
            "examples": [
                {
                    "title": "Padding and precision",
                    "code": 'print(sprintf("%-8s|%6.2f|%05d|%+d", "price", 3.14159, 42, 7));\nprint(sprintf("%x %X %o %b", 255, 255, 8, 5));\nprint(sprintf("%\'*10s|%-10s|", "right", "left"));',
                    "output": "price   |  3.14|00042|+7\nff FF 10 101\n*****right|left      |",
                },
                {
                    "title": "A table of rows",
                    "code": 'string[] items = ["Coffee", "Croissant"];\ndouble[] prices = [3.5, 2.25];\nint i = 0;\nwhile (i < len(items)) {\n    print(sprintf("%-10s %6.2f", items[i], prices[i]));\n    i = i + 1;\n}',
                    "output": "Coffee       3.50\nCroissant    2.25",
                },
                {
                    "title": "Characters, not bytes",
                    "code": 'print(sprintf("[%-7s] [%.2s] [%2$s again]", "héllo", "日本語"));',
                    "output": "[héllo  ] [日本] [日本語 again]",
                },
            ],
            "notes": [
                "Widths and string precisions count characters, like every length in qiq, where PHP counts bytes. Everything else matches PHP: integers and floating-point numbers are formatted exactly as PHP formats them.",
                "To print the result, use <code>print(sprintf(...))</code>; there is no separate <code>printf</code>.",
            ],
            "see_also": ["to_string", "str_repeat", "implode"],
        },
        {
            "name": "str_repeat",
            "purpose": "Repeat a string",
            "params": [param("text", "string", "The string to repeat."), param("times", "int", "How many times to repeat it. <code>0</code> gives an empty string.")],
            "returns": "string",
            "description": "Returns <code>text</code> repeated <code>times</code> times.",
            "return_description": "The repeated string.",
            "errors": ["<code>InvalidArgumentException</code> if <code>times</code> is negative.", NULL_STRING],
            "examples": [
                {
                    "title": "Drawing a line",
                    "code": 'print(str_repeat("=-", 5));\nprint(str_repeat("ab", 0) == "");',
                    "output": "=-=-=-=-=-\n1",
                }
            ],
            "see_also": ["implode"],
        },
        {
            "name": "strtoupper",
            "purpose": "Make a string uppercase",
            "params": [param("text", "string", "The input string.")],
            "returns": "string",
            "description": "Returns <code>text</code> with its letters converted to uppercase. Latin (ASCII, Latin-1, and Latin Extended-A), Greek, and Cyrillic letters are converted; other characters are unchanged.",
            "return_description": "The uppercased string.",
            "errors": [NULL_STRING],
            "examples": [
                {
                    "title": "Several scripts",
                    "code": 'print(strtoupper("héllo wörld"));\nprint(strtoupper("αβγ абв"));',
                    "output": "HÉLLO WÖRLD\nΑΒΓ АБВ",
                }
            ],
            "notes": ["Each character maps to exactly one character, so special cases that change length, such as German <code>ß</code> becoming <code>SS</code>, are left unchanged."],
            "see_also": ["strtolower", "ucfirst"],
        },
        {
            "name": "strtolower",
            "purpose": "Make a string lowercase",
            "params": [param("text", "string", "The input string.")],
            "returns": "string",
            "description": "Returns <code>text</code> with its letters converted to lowercase. Latin, Greek, and Cyrillic letters are converted; other characters are unchanged.",
            "return_description": "The lowercased string.",
            "errors": [NULL_STRING],
            "examples": [
                {
                    "title": "Basic use",
                    "code": 'print(strtolower("ÀÉÎÕÜ ΑΒΓ АБВ"));',
                    "output": "àéîõü αβγ абв",
                }
            ],
            "see_also": ["strtoupper", "ucfirst"],
        },
        {
            "name": "ucfirst",
            "purpose": "Make a string's first character uppercase",
            "params": [param("text", "string", "The input string.")],
            "returns": "string",
            "description": "Returns <code>text</code> with its first character converted to uppercase, using the same rules as <code>strtoupper</code>.",
            "return_description": "The resulting string.",
            "errors": [NULL_STRING],
            "examples": [{"title": "Basic use", "code": 'print(ucfirst("élan"));\nprint(ucfirst("hello world"));', "output": "Élan\nHello world"}],
            "see_also": ["strtoupper", "strtolower"],
        },
        {
            "name": "strrev",
            "purpose": "Reverse a string",
            "params": [param("text", "string", "The string to reverse.")],
            "returns": "string",
            "description": "Returns the characters of <code>text</code> in reverse order. Unlike PHP's byte-wise <code>strrev</code>, multi-byte UTF-8 characters stay intact.",
            "return_description": "The reversed string.",
            "errors": [NULL_STRING],
            "examples": [{"title": "Basic use", "code": 'print(strrev("añb界"));', "output": "界bña"}],
            "see_also": ["substr"],
        },
        {
            "name": "trim",
            "purpose": "Strip characters from the beginning and end of a string",
            "params": [
                param("text", "string", "The string to trim."),
                param("characters", "string", "The characters to strip. Each character in this string, including multi-byte ones, is stripped wherever it appears at either end. By default: space, tab, newline, and carriage return.", '" \\t\\n\\r"'),
            ],
            "returns": "string",
            "description": "Returns <code>text</code> with leading and trailing characters from <code>characters</code> removed.",
            "return_description": "The trimmed string.",
            "errors": [NULL_STRING],
            "examples": [
                {
                    "title": "Whitespace and other characters",
                    "code": 'print("[" + trim("  padded \\n") + "]");\nprint(trim("--==héllo==--", "-="));',
                    "output": "[padded]\nhéllo",
                }
            ],
            "notes": ["PHP's <code>..</code> ranges (such as <code>\"a..z\"</code>) aren't supported; list the characters instead."],
            "see_also": ["str_replace"],
        },
        {
            "name": "explode",
            "purpose": "Split a string by a separator",
            "params": [
                param("separator", "string", "The string to split at. It must not be empty."),
                param("text", "string", "The string to split."),
                param("limit", "int", "With a positive <code>limit</code>, at most that many pieces are returned, the last holding the rest of the string. With a negative <code>limit</code>, all pieces except the last <code>-limit</code> are returned. <code>0</code> counts as <code>1</code>.", "9223372036854775807"),
            ],
            "returns": "string[]",
            "description": "Splits <code>text</code> at each occurrence of <code>separator</code>, following PHP's rules.",
            "return_description": "The pieces. A <code>text</code> without the separator gives a one-element array.",
            "errors": ["<code>InvalidArgumentException</code> if <code>separator</code> is empty.", NULL_STRING],
            "examples": [
                {
                    "title": "Splitting and limiting",
                    "code": 'print(explode(",", "a,b,c,d"));\nprint(explode(",", "a,b,c,d", 2));\nprint(explode(",", "a,b,c,d", -1));\nprint(explode(",", "no separator"));',
                    "output": '["a", "b", "c", "d"]\n["a", "b,c,d"]\n["a", "b", "c"]\n["no separator"]',
                }
            ],
            "see_also": ["implode", "file", "str_replace"],
        },
        {
            "name": "implode",
            "purpose": "Join array elements with a string",
            "params": [param("separator", "string", "The string to put between elements."), param("pieces", "string[]", "The strings to join.")],
            "returns": "string",
            "description": "Joins the strings in <code>pieces</code> with <code>separator</code> between each pair.",
            "return_description": "The joined string; an empty string for an empty array.",
            "errors": [
                "<code>RuntimeException</code> if <code>pieces</code> or one of its elements is <code>null</code>.",
            ],
            "examples": [
                {
                    "title": "Joining words",
                    "code": 'string[] words = ["red", "green", "blue"];\nprint(implode(", ", words));\nprint(implode("-", explode(" ", "a b c")));',
                    "output": "red, green, blue\na-b-c",
                }
            ],
            "notes": ["The argument order is PHP's preferred one: separator first."],
            "see_also": ["explode"],
        },
        {
            "name": "string_to_bytes",
            "purpose": "Get a string's UTF-8 bytes",
            "params": [param("text", "string", "The string to encode.")],
            "returns": "byte[]",
            "description": "Returns the UTF-8 encoding of <code>text</code>, one element per byte.",
            "return_description": "The bytes. A character outside ASCII takes two to four bytes.",
            "errors": [NULL_STRING],
            "examples": [{"title": "Encoding a string", "code": 'print(string_to_bytes("héllo"));', "output": "[104, 195, 169, 108, 108, 111]"}],
            "see_also": ["bytes_to_string", "file_put_bytes"],
        },
        {
            "name": "bytes_to_string",
            "purpose": "Decode UTF-8 bytes into a string",
            "params": [param("bytes", "byte[]", "The UTF-8 bytes to decode.")],
            "returns": "string",
            "description": "Decodes <code>bytes</code> as UTF-8 text.",
            "return_description": "The decoded string.",
            "errors": [
                "<code>InvalidUtf8Exception</code> if the bytes aren't valid UTF-8 or contain a <code>0</code> byte (qiq strings can't hold one).",
                "<code>RuntimeException</code> if <code>bytes</code> is <code>null</code>.",
            ],
            "examples": [
                {
                    "title": "Decoding, and rejecting invalid UTF-8",
                    "code": 'print(bytes_to_string([104, 195, 169]));\ntry {\n    print(bytes_to_string([104, 255]));\n} catch (InvalidUtf8Exception e) {\n    print("not text: " + e->getMessage());\n}',
                    "output": "hé\nnot text: invalid UTF-8 string",
                }
            ],
            "see_also": ["string_to_bytes", "file_get_bytes"],
        },
    ],
}

FILE_ERRORS = [
    "<code>FileNotFoundException</code> if the path (or a directory in it) doesn't exist.",
    "<code>PermissionDeniedException</code> if access is denied.",
    "<code>IOException</code> for other failures; the two exceptions above extend it, so catching <code>IOException</code> catches all three.",
]

FILES = {
    "slug": "files",
    "title": "Files",
    "intro": (
        "Functions for reading and writing files and directories. Paths are relative to the directory the "
        "program runs in, not to the source file. Failures throw exceptions; see "
        "<a href=\"exception-classes.html\">Exception classes</a>."
    ),
    "details": """<p>Text functions such as <code>file_get_contents</code> require UTF-8 content, because every qiq string is UTF-8. To read or write anything else, such as images or text in another encoding, use <code>file_get_bytes</code> and <code>file_put_bytes</code>.</p>
```qiq
try {
    string config = file_get_contents("config.txt");
    print(config);
} catch (FileNotFoundException e) {
    print("no config file, using defaults");
}
```
```output
no config file, using defaults
```""",
    "functions": [
        {
            "name": "file_get_contents",
            "purpose": "Read an entire file into a string",
            "params": [param("path", "string", "The file to read.")],
            "returns": "string",
            "description": "Reads the whole file at <code>path</code> as text.",
            "return_description": "The file's contents.",
            "errors": FILE_ERRORS + ["<code>IOException</code> if <code>path</code> is a directory.", "<code>InvalidUtf8Exception</code> if the contents aren't valid UTF-8 or contain a NUL byte; read such files with <code>file_get_bytes</code>."],
            "examples": [
                {
                    "title": "Writing and reading back",
                    "code": 'int written = file_put_contents("notes.txt", "first line\\nsecond line");\nstring text = file_get_contents("notes.txt");\nprint(len(text));\nprint(text);',
                    "output": "22\nfirst line\nsecond line",
                }
            ],
            "see_also": ["file", "file_put_contents", "file_get_bytes"],
        },
        {
            "name": "file_put_contents",
            "purpose": "Write a string to a file",
            "params": [
                param("path", "string", "The file to write. It's created if it doesn't exist."),
                param("data", "string", "The text to write."),
                param("append", "bool", "When <code>true</code>, add to the end of the file instead of replacing its contents.", "false"),
            ],
            "returns": "int",
            "description": "Writes <code>data</code> to the file at <code>path</code>.",
            "return_description": "The number of <em>bytes</em> written (UTF-8 characters can take more than one).",
            "errors": FILE_ERRORS,
            "examples": [
                {
                    "title": "Replacing and appending",
                    "code": 'int a = file_put_contents("log.txt", "started\\n");\nint b = file_put_contents("log.txt", "héllo\\n", append: true);\nprint(a, b);\nprint(file("log.txt"));',
                    "output": '8 7\n["started", "héllo"]',
                }
            ],
            "see_also": ["file_get_contents", "file_put_bytes"],
        },
        {
            "name": "file",
            "purpose": "Read a file into an array of lines",
            "params": [param("path", "string", "The file to read.")],
            "returns": "string[]",
            "description": "Reads the file at <code>path</code> and splits it into lines. Line endings (<code>\\n</code> or <code>\\r\\n</code>) are removed, and a final line ending doesn't add an empty line.",
            "return_description": "The lines; an empty array for an empty file.",
            "errors": FILE_ERRORS + ["<code>InvalidUtf8Exception</code> if the contents aren't valid UTF-8."],
            "examples": [
                {
                    "title": "Processing a CSV file line by line",
                    "code": 'int written = file_put_contents("people.csv", "name,age\\nada,36\\nlin,29\\n");\nforeach (file("people.csv") as number => line) {\n    string[] fields = explode(",", line);\n    print(number, fields[0], fields[1]);\n}',
                    "output": "0 name age\n1 ada 36\n2 lin 29",
                }
            ],
            "notes": ["Unlike PHP's <code>file()</code>, line endings are removed, as if <code>FILE_IGNORE_NEW_LINES</code> were given."],
            "see_also": ["file_get_contents", "explode"],
        },
        {
            "name": "file_exists",
            "purpose": "Check whether a file or directory exists",
            "params": [param("path", "string", "The path to check.")],
            "returns": "bool",
            "description": "Checks whether anything exists at <code>path</code>.",
            "return_description": "<code>true</code> if a file or directory exists at <code>path</code>, otherwise <code>false</code>. It never throws for a missing or inaccessible path.",
            "errors": ["<code>RuntimeException</code> if <code>path</code> is <code>null</code>."],
            "examples": [
                {
                    "title": "Guarding a read",
                    "code": 'if (file_exists("settings.txt")) {\n    print(file_get_contents("settings.txt"));\n} else {\n    print("no settings");\n}',
                    "output": "no settings",
                }
            ],
            "see_also": ["is_file", "is_dir"],
        },
        {
            "name": "is_file",
            "purpose": "Check whether a path is a regular file",
            "params": [param("path", "string", "The path to check.")],
            "returns": "bool",
            "description": "Checks whether <code>path</code> exists and is a regular file.",
            "return_description": "<code>true</code> for a regular file; <code>false</code> for a directory or a missing path.",
            "errors": ["<code>RuntimeException</code> if <code>path</code> is <code>null</code>."],
            "examples": [
                {
                    "title": "Files and directories",
                    "code": 'int written = file_put_contents("data.txt", "x");\nprint(is_file("data.txt"), is_file("."), is_file("missing.txt"));',
                    "output": "1 0 0",
                }
            ],
            "see_also": ["is_dir", "file_exists"],
        },
        {
            "name": "is_dir",
            "purpose": "Check whether a path is a directory",
            "params": [param("path", "string", "The path to check.")],
            "returns": "bool",
            "description": "Checks whether <code>path</code> exists and is a directory.",
            "return_description": "<code>true</code> for a directory; <code>false</code> for a file or a missing path.",
            "errors": ["<code>RuntimeException</code> if <code>path</code> is <code>null</code>."],
            "examples": [{"title": "Basic use", "code": 'print(is_dir("."), is_dir("missing"));', "output": "1 0"}],
            "see_also": ["is_file", "scandir"],
        },
        {
            "name": "scandir",
            "purpose": "List the names in a directory",
            "params": [param("path", "string", "The directory to list.")],
            "returns": "string[]",
            "description": "Lists the files and directories inside <code>path</code>.",
            "return_description": "The names, sorted by byte order, without <code>.</code> and <code>..</code>.",
            "errors": FILE_ERRORS + ["<code>IOException</code> if <code>path</code> isn't a directory.", "<code>InvalidUtf8Exception</code> if a name isn't valid UTF-8."],
            "examples": [
                {
                    "title": "Listing a directory",
                    "code": 'int a = file_put_contents("b.txt", "");\nint b = file_put_contents("a.txt", "");\nforeach (scandir(".") as name) {\n    if (str_ends_with(name, ".txt")) {\n        print(name);\n    }\n}',
                    "output": "a.txt\nb.txt",
                }
            ],
            "notes": ["Unlike PHP's <code>scandir</code>, the <code>.</code> and <code>..</code> entries are left out."],
            "see_also": ["is_dir", "file_exists"],
        },
        {
            "name": "file_get_bytes",
            "purpose": "Read an entire file as raw bytes",
            "params": [param("path", "string", "The file to read.")],
            "returns": "byte[]",
            "description": "Reads the file at <code>path</code> without interpreting it, so it works for any file, including images, archives, and text that isn't UTF-8.",
            "return_description": "The file's bytes.",
            "errors": FILE_ERRORS + ["<code>IOException</code> if <code>path</code> is a directory."],
            "examples": [
                {
                    "title": "Checking a file signature",
                    "code": 'int written = file_put_bytes("image.png", [137, 80, 78, 71, 13, 10, 26, 10]);\nbyte[] data = file_get_bytes("image.png");\nif (data[0] == 137 && data[1] == 80) {\n    print("looks like a PNG");\n}',
                    "output": "looks like a PNG",
                }
            ],
            "see_also": ["file_put_bytes", "bytes_to_string"],
        },
        {
            "name": "file_put_bytes",
            "purpose": "Write raw bytes to a file",
            "params": [
                param("path", "string", "The file to write. It's created if it doesn't exist."),
                param("data", "byte[]", "The bytes to write."),
                param("append", "bool", "When <code>true</code>, add to the end of the file instead of replacing it.", "false"),
            ],
            "returns": "int",
            "description": "Writes <code>data</code> to the file at <code>path</code> exactly as given.",
            "return_description": "The number of bytes written.",
            "errors": FILE_ERRORS + ["<code>RuntimeException</code> if <code>data</code> is <code>null</code>."],
            "examples": [
                {
                    "title": "Copying a file byte for byte",
                    "code": 'int a = file_put_bytes("original.bin", [0, 255, 7]);\nint b = file_put_bytes("copy.bin", file_get_bytes("original.bin"));\nprint(b, file_get_bytes("copy.bin"));',
                    "output": "3 [0, 255, 7]",
                }
            ],
            "see_also": ["file_get_bytes", "string_to_bytes"],
        },
    ],
}

CONVERSIONS = {
    "slug": "conversions",
    "title": "Conversions",
    "intro": (
        "qiq never converts between types implicitly: <code>1 + 1.5</code> and <code>\"n\" + 1</code> are "
        "compile errors. These built-in functions convert explicitly. They are part of the compiler rather "
        "than declared in <code>stdlib/</code>, because each accepts several types, and like other built-ins "
        "they can't be redeclared. Each also accepts its own result type and returns the value unchanged."
    ),
    "functions": [
        {
            "name": "to_int",
            "purpose": "Convert a value to an int",
            "params": [param("value", "byte|bool|double|string", "The value to convert.")],
            "returns": "int",
            "description": "Converts <code>value</code> to an <code>int</code>: a <code>byte</code> widens; <code>true</code> is <code>1</code> and <code>false</code> is <code>0</code>; a <code>double</code> is truncated toward zero; a <code>string</code> must be a whole int such as <code>\"-42\"</code> (an optional sign, then digits, with no spaces).",
            "return_description": "The converted value.",
            "errors": [
                "<code>InvalidArgumentException</code> if a string isn't an int, or is outside the int range.",
                "<code>ArithmeticException</code> if a double is NaN, infinite, or outside the int range.",
            ],
            "examples": [
                {
                    "title": "Each source type",
                    "code": 'byte b = 200;\nprint(to_int(b) + 1, to_int(true), to_int(-2.9), to_int("8080"));\ntry {\n    print(to_int("12a"));\n} catch (InvalidArgumentException e) {\n    print(e->getMessage());\n}',
                    "output": '201 1 -2 8080\n"12a" is not an int',
                }
            ],
            "see_also": ["to_double", "to_byte", "to_string"],
        },
        {
            "name": "to_double",
            "purpose": "Convert a value to a double",
            "params": [param("value", "int|byte|string", "The value to convert.")],
            "returns": "double",
            "description": "Converts <code>value</code> to a <code>double</code>. A <code>string</code> must be a whole number such as <code>\"2.5\"</code>, <code>\"-3\"</code>, or <code>\"1e5\"</code>, with no leading spaces.",
            "return_description": "The converted value. Ints beyond 2<sup>53</sup> are rounded to the nearest double.",
            "errors": ["<code>InvalidArgumentException</code> if a string isn't a number."],
            "examples": [
                {
                    "title": "Mixing ints and doubles",
                    "code": 'int count = 7;\nprint(to_double(count) / 2.0);\nprint(to_double("2.5") * 2.0);',
                    "output": "3.5\n5",
                }
            ],
            "see_also": ["to_int", "to_string"],
        },
        {
            "name": "to_byte",
            "purpose": "Convert an int to a byte",
            "params": [param("value", "int", "The int to convert; it must be between 0 and 255.")],
            "returns": "byte",
            "description": "Converts <code>value</code> to a <code>byte</code>.",
            "return_description": "The value as a byte.",
            "errors": ["<code>InvalidArgumentException</code> if <code>value</code> is below 0 or above 255."],
            "examples": [
                {
                    "title": "Doing arithmetic on bytes",
                    "code": 'byte[] data = [10, 20];\nbyte sum = to_byte(to_int(data[0]) + to_int(data[1]));\nprint(sum);',
                    "output": "30",
                }
            ],
            "notes": ["Int literals don't need <code>to_byte</code> where a byte is expected: <code>byte b = 30;</code> works, and out-of-range literals are compile errors."],
            "see_also": ["to_int"],
        },
        {
            "name": "to_string",
            "purpose": "Convert a value to a string",
            "params": [param("value", "int|double|byte|bool|char", "The value to convert.")],
            "returns": "string",
            "description": "Converts <code>value</code> to text: ints and bytes in decimal, doubles formatted like <code>print</code> (<code>%g</code>), booleans as <code>true</code> or <code>false</code>, and a <code>char</code> as a one-character string.",
            "return_description": "The text.",
            "errors": [],
            "examples": [
                {
                    "title": "Building a message",
                    "code": 'print("count: " + to_string(42) + ", ratio: " + to_string(0.5) + ", ok: " + to_string(true));',
                    "output": "count: 42, ratio: 0.5, ok: true",
                }
            ],
            "notes": ["<code>to_string(true)</code> is <code>\"true\"</code>, while <code>print(true)</code> prints <code>1</code>."],
            "see_also": ["to_int", "to_double", "print"],
        },
    ],
}

OUTPUT = {
    "slug": "output",
    "title": "Output",
    "intro": "Writing to standard output. See also <a href=\"printing.html\">Printing</a> in the language reference.",
    "functions": [
        {
            "name": "print",
            "purpose": "Write values to standard output",
            "params": [param("values", "any...", "Zero or more values of any type except <code>void</code>.")],
            "returns": "void",
            "description": "Writes its arguments on one line, separated by single spaces, followed by a newline. <code>print</code> is a statement built into the language, so it can't be passed around or redeclared.",
            "return_description": "Nothing; <code>print</code> is a statement.",
            "errors": [],
            "examples": [
                {
                    "title": "Scalars, containers, and objects",
                    "code": 'class Square {}\nprint("age:", 36, 2.5, true);\nprint([1, 2, 3], ["a", "b,c"]);\nprint({"ada" => [true, false]});\nprint(new Square());\nprint();\nprint("done");',
                    "output": 'age: 36 2.5 1\n[1, 2, 3] ["a", "b,c"]\n{"ada" => [true, false]}\nSquare\n\ndone',
                }
            ],
            "notes": [
                "At the top level, booleans print as <code>1</code> and <code>0</code>; inside arrays and maps they print as <code>true</code> and <code>false</code>, and strings and chars are quoted, so the output reads like a qiq literal.",
            ],
            "see_also": ["to_string"],
        }
    ],
}

GENERAL = {
    "slug": "general",
    "title": "General",
    "intro": "<code>len</code> works on several types, so like <code>to_int</code> and the other conversions it's part of the compiler rather than declared in <code>stdlib/</code>. <code>gc_collect_cycles</code> controls memory management.",
    "functions": [
        {
            "name": "len",
            "purpose": "The length of a string, array, or map",
            "params": [param("value", "string|array|map", "The value to measure.")],
            "returns": "int",
            "description": "Returns the number of characters in a string, the number of elements in an array, or the number of entries in a map, like Python's <code>len</code>. For a string, characters are Unicode characters, not bytes, so <code>len(\"héllo\")</code> is <code>5</code>.",
            "return_description": "The length.",
            "errors": ["<code>RuntimeException</code> if the value is <code>null</code>. Passing any other type, such as an <code>int</code>, is a compile error."],
            "examples": [
                {
                    "title": "Strings, arrays, and maps",
                    "code": 'print(len("héllo"), len([10, 20, 30]), len({"ada" => 36, "lin" => 29}));\nint[][] grid = [[1, 2, 3], [4]];\nprint(len(grid), len(grid[0]));\nprint(len(string_to_bytes("héllo")));',
                    "output": "5 3 2\n2 3\n6",
                },
                {
                    "title": "Looping by index",
                    "code": 'string[] names = ["Ada", "Lin", "Grace"];\nint i = 0;\nwhile (i < len(names)) {\n    print(to_string(i + 1) + ". " + names[i]);\n    i = i + 1;\n}',
                    "output": "1. Ada\n2. Lin\n3. Grace",
                },
            ],
            "notes": ["Unlike PHP, which has <code>strlen</code> for strings and <code>count</code> for arrays, qiq has one function for all three. To count a string's bytes instead of its characters, use <code>len(string_to_bytes(text))</code>."],
            "see_also": ["substr", "string_to_bytes"],
        },
        {
            "name": "gc_collect_cycles",
            "purpose": "Free garbage reference cycles now",
            "params": [],
            "returns": "int",
            "description": "Runs the cycle collector immediately. Values are normally freed as soon as nothing refers to them, but objects that refer to each other in a cycle are only freed by the collector, which also runs automatically as possible cycles accumulate. See <a href=\"memory.html\">Memory management</a>.",
            "return_description": "How many values were freed.",
            "errors": [],
            "examples": [
                {
                    "title": "Freeing a cycle",
                    "code": 'class Node {\n    public Node next;\n}\n\nfunction makeRing(): void {\n    Node a = new Node();\n    Node b = new Node();\n    a->next = b;\n    b->next = a;\n}\n\nmakeRing();\nprint(gc_collect_cycles());\nprint(gc_collect_cycles());',
                    "output": "2\n0",
                }
            ],
            "notes": ["There's rarely a need to call this: the collector runs on its own. It can help before measuring memory use, or after building and discarding a large structure full of cycles."],
            "see_also": ["len"],
        },
    ],
}

NOW = "-9223372036854775808"
NOW_NOTE = "Defaults to the current time. The default value, the smallest int, stands for \"now\", because qiq defaults must be literals."
TIMESTAMP_TERMS = "A <em>timestamp</em> is an <code>int</code> counting seconds since 1970-01-01 00:00:00 UTC."
FIELD_PARAMS = lambda zone: [
    param("hour", "int", "The hour, 0 to 23."),
    param("minute", "int", "The minute.", "0"),
    param("second", "int", "The second.", "0"),
    param("month", "int", "The month, 1 to 12.", "1"),
    param("day", "int", "The day of the month.", "1"),
    param("year", "int", "The year, such as <code>2026</code>.", "1970"),
]

DATES = {
    "slug": "datetime",
    "title": "Date and time",
    "intro": (
        TIMESTAMP_TERMS + " Timestamps work like PHP's: get the current one with <code>time</code>, "
        "format one with <code>date</code>, and build one with <code>mktime</code> or <code>strtotime</code>. "
        "Local time follows the system (the <code>TZ</code> environment variable, or <code>/etc/localtime</code>) "
        "until the program calls <code>date_default_timezone_set</code>. Zone data is read from the system's "
        "zoneinfo files when the program runs, so compiled programs stay static."
    ),
    "details": """<h2>Format characters</h2>
<p><code>date</code> and <code>gmdate</code> use PHP's format letters. Any other character is copied as is; put <code>\\\\</code> before a letter to print it literally (in a qiq string, <code>"\\\\T"</code> is the two characters <code>\\T</code>).</p>
<table>
<thead><tr><th>Letter</th><th>Meaning</th><th>Example</th></tr></thead>
<tbody>
<tr><td colspan="3"><strong>Day</strong></td></tr>
<tr><td><code>d</code></td><td>Day of the month, two digits</td><td><code>01</code> to <code>31</code></td></tr>
<tr><td><code>j</code></td><td>Day of the month</td><td><code>1</code> to <code>31</code></td></tr>
<tr><td><code>D</code></td><td>Day name, three letters</td><td><code>Mon</code> to <code>Sun</code></td></tr>
<tr><td><code>l</code></td><td>Day name</td><td><code>Monday</code></td></tr>
<tr><td><code>N</code></td><td>ISO 8601 day of the week</td><td><code>1</code> (Monday) to <code>7</code> (Sunday)</td></tr>
<tr><td><code>w</code></td><td>Day of the week</td><td><code>0</code> (Sunday) to <code>6</code></td></tr>
<tr><td><code>S</code></td><td>English suffix for the day of the month</td><td><code>st</code>, <code>nd</code>, <code>rd</code>, <code>th</code></td></tr>
<tr><td><code>z</code></td><td>Day of the year, from 0</td><td><code>0</code> to <code>365</code></td></tr>
<tr><td colspan="3"><strong>Week</strong></td></tr>
<tr><td><code>W</code></td><td>ISO 8601 week number</td><td><code>01</code> to <code>53</code></td></tr>
<tr><td colspan="3"><strong>Month</strong></td></tr>
<tr><td><code>F</code></td><td>Month name</td><td><code>January</code></td></tr>
<tr><td><code>M</code></td><td>Month name, three letters</td><td><code>Jan</code></td></tr>
<tr><td><code>m</code></td><td>Month, two digits</td><td><code>01</code> to <code>12</code></td></tr>
<tr><td><code>n</code></td><td>Month</td><td><code>1</code> to <code>12</code></td></tr>
<tr><td><code>t</code></td><td>Days in the month</td><td><code>28</code> to <code>31</code></td></tr>
<tr><td colspan="3"><strong>Year</strong></td></tr>
<tr><td><code>Y</code></td><td>Year, at least four digits</td><td><code>2026</code></td></tr>
<tr><td><code>y</code></td><td>Year, two digits</td><td><code>26</code></td></tr>
<tr><td><code>L</code></td><td>Leap year</td><td><code>1</code> or <code>0</code></td></tr>
<tr><td><code>o</code></td><td>ISO 8601 week-numbering year</td><td><code>2026</code></td></tr>
<tr><td colspan="3"><strong>Time</strong></td></tr>
<tr><td><code>H</code></td><td>Hour, 24-hour, two digits</td><td><code>00</code> to <code>23</code></td></tr>
<tr><td><code>G</code></td><td>Hour, 24-hour</td><td><code>0</code> to <code>23</code></td></tr>
<tr><td><code>h</code></td><td>Hour, 12-hour, two digits</td><td><code>01</code> to <code>12</code></td></tr>
<tr><td><code>g</code></td><td>Hour, 12-hour</td><td><code>1</code> to <code>12</code></td></tr>
<tr><td><code>i</code></td><td>Minutes</td><td><code>00</code> to <code>59</code></td></tr>
<tr><td><code>s</code></td><td>Seconds</td><td><code>00</code> to <code>59</code></td></tr>
<tr><td><code>a</code>, <code>A</code></td><td>Morning or afternoon</td><td><code>am</code>/<code>pm</code>, <code>AM</code>/<code>PM</code></td></tr>
<tr><td><code>u</code>, <code>v</code></td><td>Microseconds and milliseconds (always zero, since timestamps are whole seconds)</td><td><code>000000</code>, <code>000</code></td></tr>
<tr><td><code>B</code></td><td>Swatch Internet time</td><td><code>000</code> to <code>999</code></td></tr>
<tr><td colspan="3"><strong>Timezone</strong></td></tr>
<tr><td><code>e</code></td><td>Zone name</td><td><code>Europe/London</code></td></tr>
<tr><td><code>T</code></td><td>Zone abbreviation</td><td><code>GMT</code>, <code>EST</code></td></tr>
<tr><td><code>I</code></td><td>Daylight saving time in effect</td><td><code>1</code> or <code>0</code></td></tr>
<tr><td><code>O</code></td><td>Offset from UTC</td><td><code>+0200</code></td></tr>
<tr><td><code>P</code></td><td>Offset from UTC with a colon</td><td><code>+02:00</code></td></tr>
<tr><td><code>p</code></td><td>Like <code>P</code>, but <code>Z</code> for UTC</td><td><code>Z</code></td></tr>
<tr><td><code>Z</code></td><td>Offset from UTC in seconds</td><td><code>-18000</code></td></tr>
<tr><td colspan="3"><strong>Full date and time</strong></td></tr>
<tr><td><code>c</code></td><td>ISO 8601</td><td><code>2026-11-19T14:30:00+00:00</code></td></tr>
<tr><td><code>r</code></td><td>RFC 2822</td><td><code>Thu, 19 Nov 2026 14:30:00 +0000</code></td></tr>
<tr><td><code>U</code></td><td>The timestamp</td><td><code>1795098600</code></td></tr>
</tbody>
</table>

<h2>Daylight saving time</h2>
<p>Local times are resolved the way PHP resolves them. A time skipped when clocks go forward is read with the offset from before the change, so 02:30 on a night when clocks jump from 02:00 to 03:00 becomes 03:30. A time that occurs twice, when clocks go back, resolves the same way PHP's <code>strtotime</code> does. To avoid the question entirely, work in UTC with <code>gmdate</code> and <code>gmmktime</code>, and convert to local time only for display.</p>
""",
    "functions": [
        {
            "name": "time",
            "purpose": "The current timestamp",
            "params": [],
            "returns": "int",
            "description": "Returns the current time as a timestamp. " + TIMESTAMP_TERMS,
            "return_description": "Seconds since 1970-01-01 00:00:00 UTC.",
            "errors": [],
            "examples": [
                {
                    "title": "A deadline one week from now",
                    "code": 'int now = time();\nint deadline = now + 7 * 24 * 60 * 60;\nprint(deadline - now);',
                    "output": "604800",
                }
            ],
            "see_also": ["microtime", "hrtime", "date"],
        },
        {
            "name": "microtime",
            "purpose": "The current time with microseconds",
            "params": [],
            "returns": "double",
            "description": "Returns the current time in seconds since the Unix epoch, with microseconds, like PHP's <code>microtime(true)</code>.",
            "return_description": "Seconds as a <code>double</code>.",
            "errors": [],
            "examples": [
                {
                    "title": "Comparing with time",
                    "code": 'double now = microtime();\nprint(to_int(now) >= time() - 1);',
                    "output": "1",
                }
            ],
            "notes": ["To measure how long something takes, use <code>hrtime</code>, which isn't affected by changes to the system clock."],
            "see_also": ["time", "hrtime"],
        },
        {
            "name": "hrtime",
            "purpose": "A monotonic clock in nanoseconds",
            "params": [],
            "returns": "int",
            "description": "Returns nanoseconds from a monotonic clock. The value has no fixed starting point, so only the difference between two readings is meaningful, and it never jumps when the system clock is changed.",
            "return_description": "Nanoseconds.",
            "errors": [],
            "examples": [
                {
                    "title": "Timing code",
                    "code": 'int start = hrtime();\nusleep(20000);\nint elapsed = hrtime() - start;\nprint(elapsed >= 20000000);',
                    "output": "1",
                }
            ],
            "notes": ["PHP's <code>hrtime(true)</code> returns the same thing."],
            "see_also": ["microtime", "usleep"],
        },
        {
            "name": "date",
            "purpose": "Format a timestamp in the current timezone",
            "params": [
                param("format", "string", "The format, made of the <a href=\"ref.datetime.html\">format characters</a>."),
                param("timestamp", "int", NOW_NOTE, NOW),
            ],
            "returns": "string",
            "description": "Formats <code>timestamp</code> as local time in the current timezone, using PHP's format letters. Characters that aren't format letters are copied unchanged.",
            "return_description": "The formatted date.",
            "errors": ["<code>InvalidArgumentException</code> if the timestamp is outside the range the calendar can represent.", NULL_STRING],
            "examples": [
                {
                    "title": "Common formats",
                    "code": 'date_default_timezone_set("Europe/Paris");\nint t = 1795098600;\nprint(date("Y-m-d H:i:s", t));\nprint(date("l, F jS Y", t));\nprint(date("c", t));\nprint(date("D, d M Y H:i:s O", t));',
                    "output": "2026-11-19 15:30:00\nThursday, November 19th 2026\n2026-11-19T15:30:00+01:00\nThu, 19 Nov 2026 15:30:00 +0100",
                },
                {
                    "title": "Escaping letters",
                    "code": 'print(gmdate("\\\\D\\\\a\\\\y z \\\\o\\\\f Y", 1795098600));',
                    "output": "Day 322 of 2026",
                },
            ],
            "see_also": ["gmdate", "date_default_timezone_set", "strtotime"],
        },
        {
            "name": "gmdate",
            "purpose": "Format a timestamp in UTC",
            "params": [
                param("format", "string", "The format, made of the <a href=\"ref.datetime.html\">format characters</a>."),
                param("timestamp", "int", NOW_NOTE, NOW),
            ],
            "returns": "string",
            "description": "Like <code>date</code>, but always formats the time in UTC: <code>e</code> prints <code>UTC</code>, <code>T</code> prints <code>GMT</code>, and offsets are zero.",
            "return_description": "The formatted date.",
            "errors": ["<code>InvalidArgumentException</code> if the timestamp is outside the range the calendar can represent.", NULL_STRING],
            "examples": [
                {
                    "title": "Timestamps before and after 1970",
                    "code": 'print(gmdate("Y-m-d H:i:s", 0));\nprint(gmdate("Y-m-d H:i:s", -14182940));\nprint(gmdate("c", 1795098600));',
                    "output": "1970-01-01 00:00:00\n1969-07-20 20:17:40\n2026-11-19T14:30:00+00:00",
                }
            ],
            "see_also": ["date", "gmmktime"],
        },
        {
            "name": "mktime",
            "purpose": "Build a timestamp from a local date and time",
            "params": FIELD_PARAMS("local"),
            "returns": "int",
            "description": "Returns the timestamp of a date and time in the current timezone. Values outside their usual range roll over into the next field, as in PHP: month 13 is January of the next year, and day 0 is the last day of the previous month. The argument order is PHP's; named arguments make calls clearer.",
            "return_description": "The timestamp.",
            "errors": ["<code>InvalidArgumentException</code> if the date is outside the range the calendar can represent."],
            "examples": [
                {
                    "title": "Building dates",
                    "code": 'date_default_timezone_set("America/New_York");\nint t = mktime(9, 30, year: 2026, month: 11, day: 19);\nprint(date("Y-m-d H:i T", t));\nint lastOfFebruary = mktime(0, month: 3, day: 0, year: 2028);\nprint(date("D j M Y", lastOfFebruary));',
                    "output": "2026-11-19 09:30 EST\nTue 29 Feb 2028",
                }
            ],
            "notes": [
                "Unlike PHP, every omitted argument has a fixed default rather than the current time's value.",
                "For a time that occurs twice when clocks go back, PHP's <code>mktime</code> answer depends on whether daylight saving time is in effect when the script runs. qiq always gives the same answer as <code>strtotime</code>.",
            ],
            "see_also": ["gmmktime", "checkdate", "date"],
        },
        {
            "name": "gmmktime",
            "purpose": "Build a timestamp from a UTC date and time",
            "params": FIELD_PARAMS("UTC"),
            "returns": "int",
            "description": "Like <code>mktime</code>, but the date and time are in UTC. Out-of-range values roll over the same way.",
            "return_description": "The timestamp.",
            "errors": ["<code>InvalidArgumentException</code> if the date is outside the range the calendar can represent."],
            "examples": [
                {
                    "title": "Date arithmetic",
                    "code": 'int start = gmmktime(0, year: 2026, month: 1, day: 31);\nint nextMonth = gmmktime(0, year: 2026, month: 2, day: 31);\nprint(gmdate("Y-m-d", start), gmdate("Y-m-d", nextMonth));\nprint((gmmktime(0, year: 2027) - gmmktime(0, year: 2026)) / 86400);',
                    "output": "2026-01-31 2026-03-03\n365",
                }
            ],
            "see_also": ["mktime", "gmdate"],
        },
        {
            "name": "checkdate",
            "purpose": "Check whether a date exists",
            "params": [
                param("month", "int", "The month, 1 to 12."),
                param("day", "int", "The day of the month."),
                param("year", "int", "The year, 1 to 32767."),
            ],
            "returns": "bool",
            "description": "Checks that the date exists in the Gregorian calendar, taking leap years into account.",
            "return_description": "<code>true</code> if the date is valid.",
            "errors": [],
            "examples": [
                {
                    "title": "Leap years",
                    "code": 'print(checkdate(2, 29, 2028), checkdate(2, 29, 2026), checkdate(2, 29, 2100), checkdate(4, 31, 2026));',
                    "output": "1 0 0 0",
                }
            ],
            "see_also": ["mktime"],
        },
        {
            "name": "strtotime",
            "purpose": "Parse an ISO 8601 date",
            "params": [param("text", "string", "An ISO 8601 date, or date and time.")],
            "returns": "int",
            "description": """Parses a date in one of these ISO 8601 forms:
<ul>
<li>a date: <code>2026-11-19</code> (midnight)</li>
<li>a date and time, separated by <code>T</code> or a space: <code>2026-11-19 14:30</code>, <code>2026-11-19T14:30:00</code>; fractions of a second (<code>14:30:00.250</code>) are accepted and dropped</li>
<li>either of those followed by <code>Z</code> or an offset: <code>2026-11-19T14:30:00Z</code>, <code>+02:00</code>, <code>-0500</code>, <code>+05</code></li>
</ul>
<p>Without <code>Z</code> or an offset, the time is local, in the current timezone.</p>""",
            "return_description": "The timestamp.",
            "errors": ["<code>InvalidArgumentException</code> if the text isn't one of the forms above, or names a date or time that doesn't exist (such as February 30 or 25:00).", NULL_STRING],
            "examples": [
                {
                    "title": "Dates with and without offsets",
                    "code": 'date_default_timezone_set("UTC");\nprint(strtotime("2026-11-19T14:30:00Z"));\nprint(strtotime("2026-11-19T16:30:00+02:00"));\nprint(gmdate("c", strtotime("2026-11-19 09:30-05:00")));\ntry {\n    int t = strtotime("next monday");\n} catch (InvalidArgumentException e) {\n    print(e->getMessage());\n}',
                    "output": '1795098600\n1795098600\n2026-11-19T14:30:00+00:00\n"next monday" is not an ISO 8601 date',
                }
            ],
            "notes": ["PHP's <code>strtotime</code> also understands phrases such as <code>\"next monday\"</code> and <code>\"+1 week\"</code>. qiq accepts only ISO 8601, and throws rather than returning <code>false</code>."],
            "see_also": ["mktime", "date"],
        },
        {
            "name": "date_default_timezone_set",
            "purpose": "Set the timezone for local times",
            "params": [param("zone", "string", "A zone name from the tz database, such as <code>\"Europe/London\"</code>, <code>\"America/New_York\"</code>, or <code>\"UTC\"</code>.")],
            "returns": "void",
            "description": "Sets the timezone used by <code>date</code>, <code>mktime</code>, and <code>strtotime</code> for the rest of the program. Without a call, the system timezone is used.",
            "return_description": "Nothing.",
            "errors": ["<code>InvalidArgumentException</code> if the zone isn't in the system's zoneinfo files.", NULL_STRING],
            "examples": [
                {
                    "title": "The same moment in three zones",
                    "code": 'int t = 1795098600;\nstring[] zones = ["UTC", "Asia/Tokyo", "America/Los_Angeles"];\nforeach (zones as zone) {\n    date_default_timezone_set(zone);\n    print(date("H:i T", t), date_default_timezone_get());\n}\ntry {\n    date_default_timezone_set("Mars/Olympus");\n} catch (InvalidArgumentException e) {\n    print(e->getMessage());\n}',
                    "output": '14:30 UTC UTC\n23:30 JST Asia/Tokyo\n06:30 PST America/Los_Angeles\nunknown timezone "Mars/Olympus"',
                }
            ],
            "notes": ["Zone data comes from the system running the program (<code>/usr/share/zoneinfo</code> on most Linux distributions), so a minimal container image may need its tzdata package installed. <code>\"UTC\"</code> always works."],
            "see_also": ["date_default_timezone_get", "date"],
        },
        {
            "name": "date_default_timezone_get",
            "purpose": "The current timezone's name",
            "params": [],
            "returns": "string",
            "description": "Returns the zone set with <code>date_default_timezone_set</code>. Otherwise it returns the system's zone: the <code>TZ</code> environment variable if it's set, else the zone <code>/etc/localtime</code> links to, else <code>\"UTC\"</code>.",
            "return_description": "A zone name such as <code>\"Europe/London\"</code>.",
            "errors": [],
            "examples": [
                {
                    "title": "Reading the zone back",
                    "code": 'date_default_timezone_set("Australia/Sydney");\nprint(date_default_timezone_get());',
                    "output": "Australia/Sydney",
                }
            ],
            "see_also": ["date_default_timezone_set"],
        },
        {
            "name": "sleep",
            "purpose": "Pause for whole seconds",
            "params": [param("seconds", "int", "How long to pause.")],
            "returns": "void",
            "description": "Pauses the program for <code>seconds</code> seconds.",
            "return_description": "Nothing. Unlike PHP's, it returns no value.",
            "errors": ["<code>InvalidArgumentException</code> if <code>seconds</code> is negative."],
            "examples": [
                {
                    "title": "Retrying after a pause",
                    "code": 'int attempt = 1;\nwhile (attempt <= 2) {\n    print("attempt", attempt);\n    sleep(0);\n    attempt = attempt + 1;\n}',
                    "output": "attempt 1\nattempt 2",
                }
            ],
            "see_also": ["usleep"],
        },
        {
            "name": "usleep",
            "purpose": "Pause for microseconds",
            "params": [param("microseconds", "int", "How long to pause; one second is <code>1000000</code>.")],
            "returns": "void",
            "description": "Pauses the program for <code>microseconds</code> microseconds.",
            "return_description": "Nothing.",
            "errors": ["<code>InvalidArgumentException</code> if <code>microseconds</code> is negative."],
            "examples": [
                {
                    "title": "A short pause",
                    "code": 'int start = hrtime();\nusleep(50000);\nprint((hrtime() - start) / 1000000 >= 50);',
                    "output": "1",
                }
            ],
            "see_also": ["sleep", "hrtime"],
        },
    ],
}

JSON = {
    "slug": "json",
    "title": "JSON",
    "intro": (
        "Converting values to and from JSON. Because qiq is statically typed, <code>json_decode</code> decodes into the type "
        "its result is used as (a variable, property, argument, or return value), and checks the JSON against it, so the "
        "rest of the program works with ordinary typed values. Both are built into the compiler and can't be redeclared."
    ),
    "details": """<h2>How qiq types map to JSON</h2>
<table>
<thead><tr><th>qiq type</th><th>JSON</th></tr></thead>
<tbody>
<tr><td><code>int</code>, <code>byte</code>, <code>double</code></td><td>A number. <code>json_decode</code> accepts only whole numbers for <code>int</code> and <code>byte</code> (0 to 255), and any number for <code>double</code>.</td></tr>
<tr><td><code>bool</code></td><td><code>true</code> or <code>false</code></td></tr>
<tr><td><code>string</code>, <code>char</code></td><td>A string; a char is a string of one character</td></tr>
<tr><td>Arrays</td><td>An array</td></tr>
<tr><td>Maps</td><td>An object, in insertion order. Keys are strings in JSON, so int, double, bool, and char keys are written as text and read back from it.</td></tr>
<tr><td>Objects</td><td>An object with the class's public properties</td></tr>
<tr><td><code>null</code> (a string, array, map, or object without a value)</td><td><code>null</code></td></tr>
</tbody>
</table>""",
    "functions": [
        {
            "name": "json_encode",
            "purpose": "Convert a value to JSON",
            "params": [
                param("value", "any", "The value to encode: any type."),
                param("pretty", "bool", "Whether to indent the output (4 spaces per level, one value per line), like PHP's <code>JSON_PRETTY_PRINT</code>.", "false"),
            ],
            "returns": "string",
            "description": "Returns the JSON text for <code>value</code>. Objects are written with their public properties, in the order they're declared (inherited properties first); private and protected properties are left out. Maps are written in insertion order, as PHP writes arrays.",
            "return_description": "The JSON text.",
            "errors": [
                "<code>JsonException</code> if a double is <code>INF</code> or <code>NAN</code>, which JSON can't represent, or if values are nested more than 512 levels deep, as happens when objects refer to each other in a cycle.",
            ],
            "examples": [
                {
                    "title": "Values and objects",
                    "code": 'class Point {\n    public int x;\n    public int y;\n    private string label;\n}\n\nPoint p = new Point();\np->x = 3;\np->y = 4;\nprint(json_encode(p));\nprint(json_encode([1.5, 0.1]), json_encode("say \\"hi\\""), json_encode({"b" => true, "a" => false}));',
                    "output": '{"x":3,"y":4}\n[1.5,0.1] "say \\"hi\\"" {"b":true,"a":false}',
                },
                {
                    "title": "Pretty printing",
                    "code": 'map[string, int[]] data = {"primes" => [2, 3], "empty" => []};\nprint(json_encode(data, true));',
                    "output": '{\n    "primes": [\n        2,\n        3\n    ],\n    "empty": []\n}',
                },
            ],
            "notes": [
                "Numbers are written exactly as PHP writes them: the shortest form that reads back as the same value, so <code>0.1</code> is <code>0.1</code> and <code>1.0</code> is <code>1</code>. Unlike PHP's defaults, non-ASCII characters and <code>/</code> are written as they are, not escaped, as with PHP's <code>JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES</code>.",
            ],
            "see_also": ["json_decode"],
        },
        {
            "name": "json_decode",
            "purpose": "Convert JSON to a typed value",
            "params": [param("text", "string", "The JSON text.")],
            "returns": "the type it's used as",
            "description": """<p>Parses <code>text</code> and builds a value of the type the result is used as, checking that the JSON matches it:</p>
<ul>
<li>The type comes from where the call is used: <code>User user = json_decode(text);</code>, an assignment, a property, an argument, or a return value. Without one, such as in <code>print(json_decode(text))</code>, it's a compile error.</li>
<li>For a class, the JSON object's keys are matched to public properties by name. Keys the class doesn't have are ignored, and properties the JSON doesn't mention keep their default (zero, <code>false</code>, or <code>null</code>). The constructor isn't called. The class, and every class reachable through its public properties, must be concrete, which the compiler checks.</li>
<li><code>null</code> is accepted wherever a string, char, array, map, or object is expected. If a key appears twice, the last value wins.</li>
</ul>""",
            "return_description": "The decoded value.",
            "errors": [
                "<code>JsonException</code> if the text isn't valid JSON (<code>syntax error at line 1, column 14: expected a string key</code>), or doesn't match the type, with the path to the value: <code>$.users[2].age: expected int, found a string</code>.",
            ],
            "examples": [
                {
                    "title": "Into a class",
                    "code": 'class Address {\n    public string city;\n}\nclass User {\n    public string name;\n    public string[] tags;\n    public Address address;\n}\n\nUser user = json_decode("{\\"name\\": \\"Ada\\", \\"tags\\": [\\"math\\"], \\"address\\": {\\"city\\": \\"London\\"}, \\"extra\\": 1}");\nprint(user->name, user->tags, user->address->city);',
                    "output": 'Ada ["math"] London',
                },
                {
                    "title": "Into arrays and maps",
                    "code": 'int[] ids = json_decode("[3, 1, 2]");\nmap[string, double[]] series = json_decode("{\\"a\\": [1, 2.5], \\"b\\": []}");\nprint(ids, series["a"], len(series["b"]));',
                    "output": "[3, 1, 2] [1, 2.5] 0",
                },
                {
                    "title": "Handling bad input",
                    "code": 'class Order {\n    public int quantity;\n}\n\nstring[] inputs = ["{\\"quantity\\": 2}", "{\\"quantity\\": \\"two\\"}", "{\\"quantity\\": 2"];\nforeach (inputs as input) {\n    try {\n        Order order = json_decode(input);\n        print("ok:", order->quantity);\n    } catch (JsonException e) {\n        print("rejected:", e->getMessage());\n    }\n}',
                    "output": "ok: 2\nrejected: $.quantity: expected int, found a string\nrejected: syntax error at line 1, column 15: unexpected end of JSON, expected ',' or '}'",
                },
            ],
            "notes": [
                "PHP's <code>json_decode</code> returns whatever the JSON contains; qiq's builds exactly the declared type, like JSON decoding in Go or Rust. To accept several shapes, decode into a class with all the properties any of them might have.",
                "JSON is checked in full: text that is both malformed and the wrong shape is reported as a syntax error.",
            ],
            "see_also": ["json_encode"],
        },
    ],
}

MAPS = {
    "slug": "maps",
    "title": "Maps",
    "intro": (
        "Functions for <a href=\"maps.html\">maps</a>. They're built into the compiler, because they work with any key "
        "and value types. Maps keep insertion order, so <code>map_keys</code> and <code>map_values</code> return entries "
        "in the order they were added. To read a key with a default, use <code>map[key] ?? default</code>."
    ),
    "functions": [
        {
            "name": "map_key_exists",
            "purpose": "Check whether a map has a key",
            "params": [param("key", "K", "The key to look for, of the map's key type."), param("map", "map[K, V]", "The map.")],
            "returns": "bool",
            "description": "Returns whether <code>map</code> has an entry for <code>key</code>. An entry whose value is <code>null</code> still counts.",
            "return_description": "<code>true</code> if the key is present.",
            "errors": ["<code>RuntimeException</code> if the map is <code>null</code>. A key of the wrong type is a compile error."],
            "examples": [
                {
                    "title": "Before reading a key",
                    "code": 'map[string, int] ages = {"ada" => 36};\nif (map_key_exists("ada", ages)) {\n    print(ages["ada"]);\n}\nprint(map_key_exists("bob", ages));',
                    "output": "36\n0",
                }
            ],
            "notes": ["To read a key or fall back to a default in one step, use <code>ages[\"bob\"] ?? 0</code>. PHP calls this function <code>array_key_exists</code>; the argument order is the same."],
            "see_also": ["in_map", "unset", "map_keys"],
        },
        {
            "name": "map_keys",
            "purpose": "The keys of a map, as an array",
            "params": [param("map", "map[K, V]", "The map.")],
            "returns": "K[]",
            "description": "Returns a new array of the map's keys, in the order they were added.",
            "return_description": "The keys.",
            "errors": ["<code>RuntimeException</code> if the map is <code>null</code>."],
            "examples": [
                {
                    "title": "Keys in insertion order",
                    "code": 'map[string, int] stock = {"pears" => 2, "apples" => 5};\nstock["kiwis"] = 1;\nprint(map_keys(stock));',
                    "output": '["pears", "apples", "kiwis"]',
                }
            ],
            "see_also": ["map_values", "map_key_exists"],
        },
        {
            "name": "map_values",
            "purpose": "The values of a map, as an array",
            "params": [param("map", "map[K, V]", "The map.")],
            "returns": "V[]",
            "description": "Returns a new array of the map's values, in the order their keys were added. The array refers to the same arrays, maps, and objects as the map; it doesn't copy them.",
            "return_description": "The values.",
            "errors": ["<code>RuntimeException</code> if the map is <code>null</code>."],
            "examples": [
                {
                    "title": "Summing values",
                    "code": 'map[string, int] stock = {"pears" => 2, "apples" => 5};\nint total = 0;\nforeach (map_values(stock) as count) {\n    total = total + count;\n}\nprint(map_values(stock), total);',
                    "output": "[2, 5] 7",
                }
            ],
            "see_also": ["map_keys", "in_map"],
        },
        {
            "name": "in_map",
            "purpose": "Check whether a map has a value",
            "params": [param("value", "V", "The value to look for, of the map's value type."), param("map", "map[K, V]", "The map.")],
            "returns": "bool",
            "description": "Returns whether any entry of <code>map</code> has <code>value</code>. Values compare as <code>==</code> compares them: numbers and bools by value, strings and chars by content, and arrays and maps by contents (the same elements in order, or the same keys and values in any order). Objects and functions compare by identity: the same instance, not an equal one.",
            "return_description": "<code>true</code> if the value is present.",
            "errors": ["<code>RuntimeException</code> if the map is <code>null</code>. A value of the wrong type is a compile error."],
            "examples": [
                {
                    "title": "Looking for a value",
                    "code": 'map[string, string] capitals = {"France" => "Paris", "Japan" => "Tokyo"};\nprint(in_map("Tokyo", capitals), in_map("Rome", capitals));',
                    "output": "1 0",
                }
            ],
            "notes": ["This checks every entry, so it takes time proportional to the size of the map. PHP's equivalent is <code>in_array</code>."],
            "see_also": ["map_key_exists", "map_values"],
        },
        {
            "name": "unset",
            "purpose": "Remove an entry from a map",
            "params": [param("entry", "map[key]", "The entry to remove, written as an index: <code>unset(ages[\"ada\"])</code>.")],
            "returns": "void",
            "description": "Removes the entry for the key, if there is one; removing a key that isn't there does nothing, as in PHP. The other entries keep their order. For arrays, use <a href=\"function.array-splice.html\">array_splice</a>, which moves the later elements down.",
            "return_description": "Nothing.",
            "errors": ["<code>RuntimeException</code> if the map is <code>null</code>. Using <code>unset</code> on an array element, or on anything other than a map entry, is a compile error."],
            "examples": [
                {
                    "title": "Removing entries",
                    "code": 'map[string, int] ages = {"ada" => 36, "lin" => 29, "bob" => 50};\nunset(ages["lin"]);\nunset(ages["nobody"]);\nprint(ages, len(ages));',
                    "output": '{"ada" => 36, "bob" => 50} 2',
                },
                {
                    "title": "While iterating",
                    "code": 'map[string, int] scores = {"ada" => 90, "lin" => 40, "bob" => 75};\nforeach (scores as name => score) {\n    if (score < 50) {\n        unset(scores[name]);\n    }\n}\nprint(scores);',
                    "output": '{"ada" => 90, "bob" => 75}',
                },
            ],
            "see_also": ["map_key_exists"],
        },
    ],
}

ARRAYS = {
    "slug": "arrays",
    "title": "Arrays",
    "intro": (
        "<code>range</code> builds arrays of numbers or characters; the other functions grow and shrink <a href=\"arrays.html\">arrays</a> in place, like Python's list methods. They're "
        "built into the compiler, because they work with any element type. Indexes always run from 0 to "
        "<code>len(array) - 1</code>: removing an element moves the later ones down. Every variable that refers to the "
        "array sees the change."
    ),
    "functions": [
        {
            "name": "range",
            "purpose": "A count from 0, or numbers or characters from start to end",
            "params": [
                param("start", "int|double|char", "The first value. Given alone, as <code>range(count)</code>, it's a count instead: see below."),
                param("end", "int|double|char", "The last value, which is included if the steps land on it."),
                param("step", "int|double", "The distance between values: positive, and for chars an int.", "1"),
            ],
            "returns": "int[]|double[]|char[]",
            "description": """<p><code>range(count)</code>, with one int, returns <code>0</code> up to <code>count - 1</code>: <code>range(3)</code> is <code>[0, 1, 2]</code>, and <code>range(0)</code> and a negative count give <code>[]</code>. It's the loop <code>for (i = 0; i &lt; count; i++)</code> of other languages: <code>foreach (range(len(items)) as i)</code> goes through the indexes of <code>items</code>, and does nothing when it's empty. (PHP's <code>range</code> has no one-argument form.)</p>
<p>With two or three arguments, it returns the values from <code>start</code> to <code>end</code>, as PHP's <code>range</code> does:</p>
<ul>
<li>The result is <code>int[]</code> for ints, <code>double[]</code> if any argument is a double, and <code>char[]</code> for two chars, which count by Unicode code point (<code>range('a', 'e')</code>, <code>range('α', 'ε')</code>).</li>
<li>The end is included: <code>range(1, 3)</code> is <code>[1, 2, 3]</code>. When <code>end</code> is less than <code>start</code>, the values count down: <code>range(3, 1)</code> is <code>[3, 2, 1]</code>. So <code>range(0, n - 1)</code> isn't empty when <code>n</code> is 0: it's <code>[0, -1]</code>. Use <code>range(n)</code>.</li>
<li>For doubles, value <var>i</var> is <code>start + i * step</code>, stopping at <code>end</code>, so rounding can leave the end out: <code>range(0.0, 0.3, 0.1)</code> has three values.</li>
</ul>
<p>In <code>foreach (range(...) as i)</code> over ints, no array is built: the loop just counts.</p>""",
            "return_description": "The values, as a new array.",
            "errors": [
                "<code>InvalidArgumentException</code> if the step is 0, negative for an increasing range, or larger than the distance from <code>start</code> to <code>end</code> (unless they're equal), or if a range used as an array would have more than 2<sup>31</sup> elements.",
                "Arguments of other types, a char mixed with a number, or a count that isn't an int, are compile errors.",
            ],
            "examples": [
                {
                    "title": "Counting from 0",
                    "code": 'string[] queue = ["build", "test"];\nforeach (range(len(queue)) as i) {\n    print(i, queue[i]);\n}\nstring[] empty = [];\nprint(range(3), range(len(empty)));',
                    "output": "0 build\n1 test\n[0, 1, 2] []",
                },
                {
                    "title": "Ints, doubles, and chars",
                    "code": "print(range(1, 5), range(10, 0, 4));\nprint(range(0, 1, 0.25), range('a', 'e', 2));",
                    "output": "[1, 2, 3, 4, 5] [10, 6, 2]\n[0, 0.25, 0.5, 0.75, 1] ['a', 'c', 'e']",
                },
                {
                    "title": "Counting in a loop",
                    "code": 'int total = 0;\nforeach (range(1, 100) as n) {\n    total = total + n;\n}\nprint(total);',
                    "output": "5050",
                },
            ],
            "notes": ["qiq has no C-style <code>for</code> loop; <code>foreach</code> over <code>range(n)</code> or <code>range(start, end)</code> covers counting loops."],
            "see_also": ["array_push", "len"],
        },
        {
            "name": "array_push",
            "purpose": "Add values to the end of an array",
            "params": [param("array", "T[]", "The array to change."), param("values", "T...", "One or more values to add, in order.")],
            "returns": "void",
            "description": "Appends the values to the end of the array. Appending is fast: the array keeps spare room, doubling it when it runs out.",
            "return_description": "Nothing. (PHP's returns the new length; use <code>len(array)</code>.)",
            "errors": ["<code>RuntimeException</code> if the array is <code>null</code>. A value of the wrong type is a compile error."],
            "examples": [
                {
                    "title": "Building a list",
                    "code": 'string[] names;\narray_push(names, "Ada");\narray_push(names, "Lin", "Grace");\nprint(names, len(names));',
                    "output": '["Ada", "Lin", "Grace"] 3',
                }
            ],
            "see_also": ["array_pop", "array_unshift", "array_splice"],
        },
        {
            "name": "array_pop",
            "purpose": "Remove and return the last element",
            "params": [param("array", "T[]", "The array to change.")],
            "returns": "T",
            "description": "Removes the last element and returns it.",
            "return_description": "The element that was last.",
            "errors": ["<code>OutOfRangeException</code> if the array is empty, and <code>RuntimeException</code> if it's <code>null</code>."],
            "examples": [
                {
                    "title": "A stack",
                    "code": 'int[] stack = [1, 2];\narray_push(stack, 3);\nprint(array_pop(stack), array_pop(stack), stack);',
                    "output": "3 2 [1]",
                }
            ],
            "notes": ["PHP's <code>array_pop</code> returns <code>null</code> for an empty array; qiq throws, as Python's <code>pop</code> does. Check <code>len(array) &gt; 0</code> first."],
            "see_also": ["array_push", "array_shift"],
        },
        {
            "name": "array_shift",
            "purpose": "Remove and return the first element",
            "params": [param("array", "T[]", "The array to change.")],
            "returns": "T",
            "description": "Removes the first element and returns it. The other elements move down one place, so the old element 1 becomes element 0. This takes time proportional to the array's length.",
            "return_description": "The element that was first.",
            "errors": ["<code>OutOfRangeException</code> if the array is empty, and <code>RuntimeException</code> if it's <code>null</code>."],
            "examples": [
                {
                    "title": "A queue",
                    "code": 'string[] queue = ["first", "second", "third"];\nwhile (len(queue) > 0) {\n    print(array_shift(queue), len(queue));\n}',
                    "output": "first 2\nsecond 1\nthird 0",
                }
            ],
            "see_also": ["array_unshift", "array_pop"],
        },
        {
            "name": "array_unshift",
            "purpose": "Add values to the front of an array",
            "params": [param("array", "T[]", "The array to change."), param("values", "T...", "One or more values; they end up in the order given.")],
            "returns": "void",
            "description": "Inserts the values at the front of the array, moving the existing elements up. This takes time proportional to the array's length.",
            "return_description": "Nothing. (PHP's returns the new length; use <code>len(array)</code>.)",
            "errors": ["<code>RuntimeException</code> if the array is <code>null</code>. A value of the wrong type is a compile error."],
            "examples": [
                {
                    "title": "Adding to the front",
                    "code": 'int[] numbers = [3, 4];\narray_unshift(numbers, 1, 2);\nprint(numbers);',
                    "output": "[1, 2, 3, 4]",
                }
            ],
            "see_also": ["array_shift", "array_push"],
        },
        {
            "name": "array_splice",
            "purpose": "Remove or replace a range of elements",
            "params": [
                param("array", "T[]", "The array to change."),
                param("offset", "int", "Where the range starts. A negative offset counts from the end."),
                param("length", "int", "How many elements to remove; by default, all from <code>offset</code> to the end. A negative length leaves that many elements at the end."),
                param("replacement", "T[]", "Elements to put in place of the removed ones; by default, none."),
            ],
            "returns": "T[]",
            "description": "Removes <code>length</code> elements starting at <code>offset</code>, inserts the elements of <code>replacement</code> in their place, and moves the rest of the array to close or open the gap, as PHP's <code>array_splice</code> does. Offsets and lengths beyond the array's ends are clipped rather than errors. With a length of <code>0</code>, it only inserts.",
            "return_description": "The removed elements, as a new array.",
            "errors": ["<code>RuntimeException</code> if the array is <code>null</code>."],
            "examples": [
                {
                    "title": "Removing, replacing, and inserting",
                    "code": 'int[] a = [1, 2, 3, 4, 5];\nprint(array_splice(a, 1, 2), a);\nprint(array_splice(a, -1, 1, [8, 9]), a);\nint[] none = array_splice(a, 1, 0, [7]);\nprint(a);',
                    "output": "[2, 3] [1, 4, 5]\n[5] [1, 4, 8, 9]\n[1, 7, 4, 8, 9]",
                },
                {
                    "title": "Removing by index",
                    "code": 'string[] names = ["Ada", "Lin", "Grace"];\narray_splice(names, 1, 1);\nprint(names, names[1]);',
                    "output": '["Ada", "Grace"] Grace',
                },
            ],
            "see_also": ["array_shift", "array_push"],
        },
    ],
}

PROGRAM = {
    "slug": "program",
    "title": "Program",
    "intro": "The running program: its command-line arguments, its exit status, and its environment variables.",
    "details": """<h2>A command-line program</h2>
<pre class="code"><span class="type">string</span>[] args = <span class="fn">argv</span>();
<span class="kw">if</span> (<span class="fn">len</span>(args) &lt; <span class="lit">2</span>) {
    <span class="kw">print</span>(<span class="lit">"usage: "</span> + args[<span class="lit">0</span>] + <span class="lit">" &lt;name&gt;"</span>);
    <span class="fn">exit</span>(<span class="lit">2</span>);
}
<span class="kw">print</span>(<span class="lit">"hello, "</span> + args[<span class="lit">1</span>]);</pre>
<pre class="code">$ qiq run greet.qiq
usage: greet.qiq &lt;name&gt;
$ echo $?
2
$ qiq build greet.qiq &amp;&amp; ./greet Ada
hello, Ada</pre>""",
    "functions": [
        {
            "name": "argv",
            "purpose": "The command-line arguments",
            "params": [],
            "returns": "string[]",
            "description": "Returns the command line as a new array: element 0 is the program, and the arguments follow, as with PHP's <code>$argv</code>, Python's <code>sys.argv</code>, and Go's <code>os.Args</code>. With <code>qiq run app.qiq a b</code>, the program is <code>app.qiq</code>; with an executable built by <code>qiq build</code>, it's the path the executable was started with.",
            "return_description": "At least one element: the program, then each argument.",
            "errors": ["<code>InvalidUtf8Exception</code> if an argument isn't valid UTF-8."],
            "examples": [
                {
                    "title": "Counting the arguments",
                    "code": 'string[] args = argv();\nprint("arguments:", len(args) - 1);\nforeach (args as index => arg) {\n    if (index > 0) {\n        print(index, arg);\n    }\n}',
                    "output": "arguments: 0",
                }
            ],
            "notes": ["qiq has no global variables, so the arguments are read with a function rather than PHP's <code>$argv</code>; it works the same inside functions."],
            "see_also": ["exit", "getenv"],
        },
        {
            "name": "exit",
            "purpose": "End the program with an exit status",
            "params": [param("status", "int", "The exit status, from 0 (success) to 255.", "0")],
            "returns": "void",
            "description": "Flushes output and ends the program immediately with <code>status</code>, which the shell sees as <code>$?</code>. Without <code>exit</code>, a program exits with 0, or with 1 after an uncaught exception.",
            "return_description": "It doesn't return.",
            "errors": ["<code>InvalidArgumentException</code> if <code>status</code> is outside 0 to 255."],
            "examples": [
                {
                    "title": "Stopping early",
                    "code": 'print("checking input");\nstring[] args = argv();\nif (len(args) < 2) {\n    print("no input given");\n    exit(1);\n}\nprint("never printed without an argument");',
                    "output": "checking input\nno input given",
                }
            ],
            "notes": ["<code>finally</code> blocks don't run, as with PHP's <code>exit</code>. To run them, throw an exception and let it reach the top of the program, which exits with status 1."],
            "see_also": ["argv"],
        },
        {
            "name": "getenv",
            "purpose": "Read an environment variable",
            "params": [param("name", "string", "The variable's name, such as <code>HOME</code>.")],
            "returns": "string",
            "description": "Returns the value of the environment variable <code>name</code>, or <code>null</code> if it isn't set. An empty value is an empty string, not <code>null</code>.",
            "return_description": "The value, or <code>null</code>.",
            "errors": ["<code>InvalidUtf8Exception</code> if the value isn't valid UTF-8, and <code>RuntimeException</code> if <code>name</code> is <code>null</code>."],
            "examples": [
                {
                    "title": "With a default",
                    "code": 'string level = getenv("QIQ_EXAMPLE_LOG_LEVEL") ?? "info";\nbool hasPath = getenv("PATH") != null;\nprint(level, hasPath);',
                    "output": "info 1",
                }
            ],
            "notes": ["PHP's <code>getenv</code> returns <code>false</code> for a variable that isn't set; qiq returns <code>null</code>, so <code>??</code> gives a default."],
            "see_also": ["argv"],
        },
    ],
}

HTTP = {
    "slug": "http",
    "title": "HTTP",
    "intro": (
        "HTTP and HTTPS requests in the style of Python's <code>requests</code> module, plus URL encoding. "
        "The client is written in qiq (<code>stdlib/http.qiq</code>) on top of the <a href=\"class.socket.html\">Socket</a> "
        "class, and returns a <a href=\"class.response.html\">Response</a>. HTTPS uses Mbed TLS, built into qiq, and always "
        "checks the server's certificate against the system's CA certificates. Network failures throw "
        "<code>NetworkException</code>; error statuses such as 404 don't, until you call "
        "<code>raiseForStatus()</code>."
    ),
    "details": """<h2>Example</h2>
<pre class="code"><span class="kw">class</span> <span class="cls">Repository</span> {
    <span class="kw">public</span> <span class="type">string</span> full_name;
    <span class="kw">public</span> <span class="type">int</span> stargazers_count;
}

<span class="cls">Response</span> r = <span class="fn">http_get</span>(<span class="lit">"https://api.github.com/repos/torvalds/linux"</span>,
                      headers: {<span class="lit">"Accept"</span> =&gt; <span class="lit">"application/vnd.github+json"</span>});
r-&gt;<span class="fn">raiseForStatus</span>();
<span class="cls">Repository</span> repo = <span class="fn">json_decode</span>(r-&gt;<span class="fn">text</span>());
<span class="kw">print</span>(repo-&gt;full_name, repo-&gt;stargazers_count);</pre>
<h2>What the client does</h2>
<ul>
<li>HTTP/1.1, one connection per request. Request bodies are sent with a <code>Content-Length</code>; responses can use <code>Content-Length</code>, chunked transfer encoding, or end when the server closes the connection.</li>
<li>Redirects (301, 302, 303, 307, 308) are followed, up to 10. A 303, or a 301 or 302 after a POST, becomes a GET without a body, as in browsers and <code>requests</code>. An <code>Authorization</code> header isn't sent on to a different host.</li>
<li>Default headers: <code>User-Agent: qiq-http/1.0</code>, <code>Accept: */*</code>, <code>Accept-Encoding: identity</code> (responses aren't compressed), and <code>Connection: close</code>. Headers you pass replace defaults with the same name, in any case.</li>
<li>The timeout limits each connect, read, and write, in seconds, as in <code>requests</code>.</li>
<li>Certificates come from the system's CA bundle (as Go finds it), or from the file in <code>SSL_CERT_FILE</code> or the directory in <code>SSL_CERT_DIR</code>. TLS 1.2 and 1.3 are supported.</li>
<li>Not supported yet: sessions and cookies, multipart uploads, streaming bodies, compression, proxies, and credentials in URLs.</li>
</ul>""",
    "functions": [
        {
            "name": "http_request",
            "purpose": "Send an HTTP request",
            "params": [
                param("method", "string", "The method, such as <code>GET</code>, <code>POST</code>, <code>PUT</code>, <code>PATCH</code>, <code>DELETE</code>, or <code>HEAD</code>; any case."),
                param("url", "string", "An <code>http://</code> or <code>https://</code> URL."),
                param("body", "string", "The request body, sent as is; <code>null</code> for none.", "null"),
                param("headers", "map[string, string]", "Headers to send, added to the defaults.", "null"),
                param("timeout", "int", "Seconds to wait for each connect, read, and write.", "30"),
            ],
            "returns": "Response",
            "description": "Sends the request, follows redirects, and returns the final <a href=\"class.response.html\">Response</a>. Set a <code>Content-Type</code> header to describe a body, such as <code>application/json</code>.",
            "return_description": "The response, whatever its status.",
            "errors": [
                "<code>NetworkException</code> if the host can't be found, refuses the connection, times out, closes the connection early, or fails the TLS certificate check.",
                "<code>HttpException</code> after more than 10 redirects.",
                "<code>InvalidArgumentException</code> for a URL that isn't <code>http</code> or <code>https</code>, contains credentials, or has an invalid port, and for header names or values with line breaks.",
            ],
            "examples": [
                {
                    "title": "Updating a resource",
                    "code": 'Response r = http_request("PUT", "https://api.example.com/items/7",\n                          json_encode({"name" => "Lamp"}),\n                          {"Content-Type" => "application/json", "Authorization" => "Bearer " + token});\nif (!r->ok()) {\n    print("failed:", r->status, r->text());\n}',
                }
            ],
            "see_also": ["http_get", "http_post"],
        },
        {
            "name": "http_get",
            "purpose": "Send a GET request",
            "params": [
                param("url", "string", "An <code>http://</code> or <code>https://</code> URL."),
                param("headers", "map[string, string]", "Headers to send.", "null"),
                param("timeout", "int", "Seconds to wait for each connect, read, and write.", "30"),
            ],
            "returns": "Response",
            "description": "Same as <code>http_request(&quot;GET&quot;, url, null, headers, timeout)</code>.",
            "return_description": "The response.",
            "errors": ["As for <a href=\"function.http-request.html\">http_request</a>."],
            "examples": [
                {
                    "title": "A query string",
                    "code": 'Response r = http_get("https://api.example.com/search?" + http_build_query({"q" => "qiq lang", "page" => "2"}));\nprint(r->status, r->header("content-type"));',
                },
                {
                    "title": "Downloading a file",
                    "code": 'Response r = http_get("https://example.com/logo.png", timeout: 60);\nr->raiseForStatus();\nint written = file_put_bytes("logo.png", r->bytes);',
                },
            ],
            "see_also": ["http_request", "http_build_query"],
        },
        {
            "name": "http_post",
            "purpose": "Send a POST request",
            "params": [
                param("url", "string", "An <code>http://</code> or <code>https://</code> URL."),
                param("body", "string", "The request body.", '""'),
                param("headers", "map[string, string]", "Headers to send.", "null"),
                param("timeout", "int", "Seconds to wait for each connect, read, and write.", "30"),
            ],
            "returns": "Response",
            "description": "Same as <code>http_request(&quot;POST&quot;, url, body, headers, timeout)</code>.",
            "return_description": "The response.",
            "errors": ["As for <a href=\"function.http-request.html\">http_request</a>."],
            "examples": [
                {
                    "title": "Sending JSON",
                    "code": 'Response r = http_post("https://api.example.com/items",\n                       json_encode({"name" => "Lamp", "price" => 25}),\n                       {"Content-Type" => "application/json"});\nr->raiseForStatus();',
                },
                {
                    "title": "Sending a form",
                    "code": 'Response r = http_post("https://example.com/login",\n                       http_build_query({"user" => "ada", "password" => secret}),\n                       {"Content-Type" => "application/x-www-form-urlencoded"});',
                },
            ],
            "see_also": ["http_request", "json_encode"],
        },
        {
            "name": "urlencode",
            "purpose": "Percent-encode text for a query string",
            "params": [param("text", "string", "The text to encode.")],
            "returns": "string",
            "description": "Encodes the text's UTF-8 bytes as <code>%XX</code>, except letters, digits, and <code>-_.</code>, and writes spaces as <code>+</code>, as PHP's <code>urlencode</code> does. Use it for query strings and form bodies; for path segments, use <a href=\"function.rawurlencode.html\">rawurlencode</a>.",
            "return_description": "The encoded text.",
            "errors": [],
            "examples": [{"title": "Encoding a query value", "code": 'print(urlencode("fish & chips/é"));', "output": "fish+%26+chips%2F%C3%A9"}],
            "see_also": ["rawurlencode", "urldecode", "http_build_query"],
        },
        {
            "name": "rawurlencode",
            "purpose": "Percent-encode text for a URL path",
            "params": [param("text", "string", "The text to encode.")],
            "returns": "string",
            "description": "Encodes the text's UTF-8 bytes as <code>%XX</code>, except letters, digits, and <code>-_.~</code> (RFC 3986), as PHP's <code>rawurlencode</code> does. Spaces become <code>%20</code>.",
            "return_description": "The encoded text.",
            "errors": [],
            "examples": [{"title": "A path segment", "code": 'print("/files/" + rawurlencode("my report~v2.pdf"));', "output": "/files/my%20report~v2.pdf"}],
            "see_also": ["urlencode"],
        },
        {
            "name": "urldecode",
            "purpose": "Decode percent-encoded text",
            "params": [param("text", "string", "The encoded text.")],
            "returns": "string",
            "description": "Decodes <code>%XX</code> sequences, and <code>+</code> as a space, as PHP's <code>urldecode</code> does. A <code>%</code> that isn't followed by two hex digits is kept as is.",
            "return_description": "The decoded text.",
            "errors": ["<code>InvalidUtf8Exception</code> if the decoded bytes aren't valid UTF-8."],
            "examples": [{"title": "Round trip", "code": 'string encoded = urlencode("a b&c=é");\nprint(encoded, urldecode(encoded));', "output": "a+b%26c%3D%C3%A9 a b&c=é"}],
            "see_also": ["urlencode", "rawurlencode", "rawurldecode"],
        },
        {
            "name": "rawurldecode",
            "purpose": "Decode a percent-encoded URL path",
            "params": [param("text", "string", "The encoded text.")],
            "returns": "string",
            "description": "Decodes <code>%XX</code> sequences like <a href=\"function.urldecode.html\">urldecode</a>, but leaves <code>+</code> as is, as PHP's <code>rawurldecode</code> does. Use it for URL paths, where <code>+</code> is an ordinary character.",
            "return_description": "The decoded text.",
            "errors": ["<code>InvalidUtf8Exception</code> if the decoded bytes aren't valid UTF-8."],
            "examples": [{"title": "Paths keep their plus signs", "code": 'print(rawurldecode("/files/C%2B%2B+notes.txt"));\nprint(urldecode("/files/C%2B%2B+notes.txt"));', "output": "/files/C+++notes.txt\n/files/C++ notes.txt"}],
            "see_also": ["rawurlencode", "urldecode"],
        },
        {
            "name": "http_build_query",
            "purpose": "Build a query string from a map",
            "params": [param("fields", "map[string, string]", "The names and values.")],
            "returns": "string",
            "description": "Returns <code>name=value</code> pairs joined with <code>&amp;</code>, with names and values encoded by <a href=\"function.urlencode.html\">urlencode</a>, in the map's order, as PHP's <code>http_build_query</code> does for a flat array.",
            "return_description": "The query string, without a leading <code>?</code>.",
            "errors": [],
            "examples": [{"title": "Query parameters", "code": 'print(http_build_query({"q" => "qiq lang", "page" => "2", "tag" => "a&b"}));', "output": "q=qiq+lang&page=2&tag=a%26b"}],
            "see_also": ["urlencode", "http_get"],
        },
    ],
}

HTTP_SERVER = {
    "slug": "http-server",
    "title": "HTTP server",
    "intro": (
        "An HTTP server in the style of Go's <code>net/http</code>. A <em>handler</em> is any "
        "<code>function(Request, ResponseWriter): void</code>, usually an <a href=\"anonymous-functions.html\">anonymous function</a>: "
        "it reads the <a href=\"class.request.html\">Request</a> and writes the response with the "
        "<a href=\"class.responsewriter.html\">ResponseWriter</a>. A <a href=\"class.router.html\">Router</a> sends each request "
        "to a handler by method and path, and <a href=\"function.http-serve.html\">http_serve</a> runs the server. "
        "To serve a directory of files without writing any code, run <code>qiq serve</code>."
    ),
    "details": """<h2>Example</h2>
<pre class="code">class Item {
    public string name;
    public int price;
}

Router router = new Router();
router-&gt;handle("GET /{$}", fn(Request r, ResponseWriter w) =&gt; w-&gt;write("Welcome!\\n"));

router-&gt;handle("GET /hello/{name}", function(Request r, ResponseWriter w): void {
    w-&gt;write("Hello, " + r-&gt;pathValue("name") + "!\\n");
});

router-&gt;handle("POST /items", function(Request r, ResponseWriter w): void {
    Item item = json_decode(r-&gt;text());
    w-&gt;status(201);
    w-&gt;header("Content-Type", "application/json");
    w-&gt;write(json_encode(item));
});

router-&gt;handle("/static/", http_strip_prefix("/static", http_file_server("public")));

http_serve(":8080", http_log(router-&gt;handler()));</pre>
<pre class="code">$ qiq run app.qiq
$ curl localhost:8080/hello/Ada
Hello, Ada!</pre>
<h2>How the server works</h2>
<ul>
<li><strong>Pre-forked workers.</strong> <code>http_serve</code> opens the listening socket, then forks worker processes (one per CPU by default) that each accept and serve one connection at a time, like Apache's prefork or PHP-FPM. The first process supervises them and starts a new worker if one exits or crashes, so a handler that crashes takes down one worker, never the server.</li>
<li><strong>No shared memory between requests in different workers.</strong> Each worker is a separate process with its own copy of the program's variables, so a counter incremented by a handler counts per worker. Keep shared state in a file or a database.</li>
<li><strong>Stopping.</strong> Ctrl-C (SIGINT) or SIGTERM stops the server gracefully: workers finish the request in progress and exit, and <code>http_serve</code> returns in the first process.</li>
<li><strong>Errors.</strong> An exception thrown by a handler becomes a <code>500 Internal Server Error</code> response, logged to stderr as <code>qiq: 500 GET /path: message</code>; the worker carries on.</li>
<li><strong>Connections.</strong> HTTP/1.1 with keep-alive (and HTTP/1.0). A worker closes an idle connection after 5 seconds, or sooner when another client is waiting for a worker. Request bodies can be sent with <code>Content-Length</code> or chunked, up to 64 MB; <code>Expect: 100-continue</code> is answered.</li>
<li><strong>Responses</strong> are collected in memory and sent with a <code>Content-Length</code> when the handler returns, with a <code>Date</code> header and, unless the handler sets one, <code>Content-Type: text/plain; charset=utf-8</code>. Responses to <code>HEAD</code> requests have no body.</li>
<li>Not supported yet: HTTPS (put a proxy such as Caddy or nginx in front), streaming responses, WebSockets, HTTP/2, and compression.</li>
</ul>
<h2>Behind nginx</h2>
<p>In production, put the server behind nginx (or Caddy), as Go and Node apps are deployed: the proxy handles HTTPS, HTTP/2, compression, and static files, and it buffers slow clients, so a worker is busy only while your code runs. The program needs no special mode: listen where only the proxy can connect, and set <code>trustProxy</code> so requests show the visitor's address rather than the proxy's.</p>
<pre class="code">http_serve("unix:/run/app/app.sock", http_log(router-&gt;handler()), trustProxy: true);
// or: http_serve("127.0.0.1:8080", http_log(router-&gt;handler()), trustProxy: true);</pre>
<pre class="code">server {
    listen 443 ssl;
    server_name example.com;

    location /static/ { root /srv/app; }    # files served by nginx itself

    location / {
        proxy_pass http://unix:/run/app/app.sock;    # or http://127.0.0.1:8080
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}</pre>
<ul>
<li>Any user can connect to a Unix socket file, as to a port on 127.0.0.1; to restrict it, put it in a directory only nginx and the server can enter. The server replaces a socket file left by a crash, refuses to start if another server is listening on it, and removes it when it stops.</li>
<li>With <code>trustProxy</code>, <code>r-&gt;remoteAddress</code> is the client's IP address without a port, and <code>r-&gt;scheme</code> is <code>https</code> for visitors who used HTTPS, for building absolute URLs. Relative redirects such as <code>w-&gt;redirect("/login")</code> need neither.</li>
<li>Leave nginx's upstream <code>keepalive</code> off (the default): each idle connection nginx kept open would occupy a worker.</li>
<li>Run the program as a service, for example with systemd and <code>Restart=always</code>. systemd stops it with SIGTERM, which lets requests in progress finish.</li>
</ul>
<h2>qiq serve</h2>
<p><code>qiq serve</code> serves a directory with <a href=\"function.http-file-server.html\">http_file_server</a>, logging each request, like <code>php -S</code> or <code>python -m http.server</code>:</p>
<pre class="code">$ qiq serve public --port 8080
Serving public at http://127.0.0.1:8080/ (press Ctrl-C to stop)
[Fri Nov 20 10:15:02 2026] 127.0.0.1:51234 [200]: GET / (0.2 ms)</pre>
<p>The directory defaults to the current one, <code>--port</code> to 8000, and <code>--host</code> to 127.0.0.1, which only this machine can reach; <code>--host 0.0.0.0</code> listens on every network interface. <code>--workers</code> sets the number of worker processes.</p>""",
    "functions": [
        {
            "name": "http_serve",
            "purpose": "Run an HTTP server",
            "params": [
                param("address", "string", "Where to listen: <code>\":8080\"</code> for every network interface (IPv4 and IPv6), <code>\"127.0.0.1:8080\"</code> for this machine only, <code>\"[::1]:8080\"</code> for an IPv6 address, or <code>\"unix:/run/app.sock\"</code> for a Unix socket file, to sit behind a proxy such as nginx."),
                param("handler", "function(Request, ResponseWriter): void", "Called for each request; usually <code>router-&gt;handler()</code>."),
                param("workers", "int", "The number of worker processes; <code>0</code> means one per CPU. With <code>1</code>, the server runs in this process without forking.", "0"),
                param("trustProxy", "bool", "Whether the server is behind a proxy such as nginx. If so, <code>Request::remoteAddress</code> is the client address the proxy forwarded (the last entry of <code>X-Forwarded-For</code>, or <code>X-Real-IP</code>), and <code>Request::scheme</code> comes from <code>X-Forwarded-Proto</code>. Only set it when clients can't reach the server except through the proxy, because anyone else could send those headers.", "false"),
            ],
            "returns": "void",
            "description": "Listens on the address, starts the workers, and serves requests until the program gets SIGINT (Ctrl-C) or SIGTERM. Each worker serves one connection at a time; see <a href=\"ref.http-server.html\">How the server works</a>.",
            "return_description": "Returns in the first process once the workers have finished their requests and stopped; worker processes exit without returning.",
            "errors": [
                "<code>NetworkException</code> if the address can't be used, for example because another program is listening on the port (<code>could not listen on :8080: Address in use</code>) or on the socket file.",
                "<code>InvalidArgumentException</code> for an address without a port.",
            ],
            "examples": [
                {"title": "Hello, world", "code": 'http_serve(":8080", fn(Request r, ResponseWriter w) => w->write("Hello, world!\\n"));'},
                {"title": "Behind nginx, on a Unix socket", "code": 'http_serve("unix:/run/app/app.sock", http_log(router->handler()), trustProxy: true);'},
                {"title": "Two workers on localhost", "code": 'Router router = new Router();\nrouter->handle("GET /time", fn(Request r, ResponseWriter w) => w->write(date("H:i:s") + "\\n"));\nhttp_serve("127.0.0.1:3000", http_log(router->handler()), workers: 2);\nprint("stopped");'},
            ],
            "see_also": ["http_file_server", "http_log"],
        },
        {
            "name": "http_file_server",
            "purpose": "A handler that serves files from a directory",
            "params": [param("directory", "string", "The directory to serve.")],
            "returns": "function(Request, ResponseWriter): void",
            "description": "Returns a handler that serves the file at the request's path inside <code>directory</code>, with a <code>Content-Type</code> from its extension (see <a href=\"function.http-content-type.html\">http_content_type</a>). For a directory, it serves its <code>index.html</code>, or a listing of its files; a directory path without a trailing <code>/</code> is redirected to one. Only <code>GET</code> and <code>HEAD</code> are allowed (405 otherwise), and a path containing a <code>..</code> segment gets 404, so files outside the directory can't be reached. Files are read whole into memory.",
            "return_description": "The handler.",
            "errors": [],
            "examples": [
                {"title": "A static site", "code": 'http_serve(":8080", http_file_server("public"));'},
                {"title": "Files under a prefix, next to other routes", "code": 'Router router = new Router();\nrouter->handle("/assets/", http_strip_prefix("/assets", http_file_server("assets")));\nrouter->handle("GET /{$}", fn(Request r, ResponseWriter w) => w->write("home"));\nhttp_serve(":8080", router->handler());'},
            ],
            "see_also": ["http_strip_prefix", "http_serve"],
        },
        {
            "name": "http_strip_prefix",
            "purpose": "Remove a prefix from the path before calling a handler",
            "params": [
                param("prefix", "string", "The start of the path to remove, such as <code>\"/static\"</code>."),
                param("handler", "function(Request, ResponseWriter): void", "The handler to call with the shorter path."),
            ],
            "returns": "function(Request, ResponseWriter): void",
            "description": "Returns a handler that removes <code>prefix</code> from the request's <code>path</code> and <code>rawPath</code> and calls <code>handler</code>, like Go's <code>http.StripPrefix</code>. A request whose path doesn't start with the prefix gets 404.",
            "return_description": "The handler.",
            "errors": [],
            "examples": [{"title": "Mounting a file server", "code": 'router->handle("/static/", http_strip_prefix("/static", http_file_server("public")));\n// GET /static/css/site.css serves public/css/site.css'}],
            "see_also": ["http_file_server"],
        },
        {
            "name": "http_log",
            "purpose": "Log each request to stderr",
            "params": [param("handler", "function(Request, ResponseWriter): void", "The handler to wrap.")],
            "returns": "function(Request, ResponseWriter): void",
            "description": "Returns a handler that calls <code>handler</code> and then writes a line to stderr in the style of <code>php -S</code>: the time, the client's address, the status, the method, the path, and how long the handler took. A handler of this shape that wraps another is often called <em>middleware</em>; write your own the same way.",
            "return_description": "The handler.",
            "errors": [],
            "examples": [
                {"title": "Logging", "code": 'http_serve(":8080", http_log(router->handler()));\n// [Fri Nov 20 10:15:02 2026] 127.0.0.1:51234 [200]: GET /hello/Ada (0.1 ms)'},
                {"title": "Your own middleware", "code": 'function require_token(string token, function(Request, ResponseWriter): void next): function(Request, ResponseWriter): void {\n    return function(Request r, ResponseWriter w): void {\n        if (r->header("Authorization") != "Bearer " + token) {\n            w->error(401);\n            return;\n        }\n        next(r, w);\n    };\n}\nhttp_serve(":8080", http_log(require_token(getenv("API_TOKEN"), router->handler())));'},
            ],
            "see_also": ["http_serve"],
        },
        {
            "name": "http_content_type",
            "purpose": "The Content-Type for a file name",
            "params": [param("name", "string", "A file name or path.")],
            "returns": "string",
            "description": "Looks up the file's extension, in any case, among common web types: HTML, CSS, JavaScript, JSON, text, images, fonts, audio, video, PDF, archives, and WebAssembly. Text types include <code>; charset=utf-8</code>.",
            "return_description": "The type, or <code>application/octet-stream</code> for an unknown or missing extension.",
            "errors": [],
            "examples": [{"title": "Types by extension", "code": 'print(http_content_type("index.HTML"));\nprint(http_content_type("photos/cat.jpg"));\nprint(http_content_type("README"));', "output": "text/html; charset=utf-8\nimage/jpeg\napplication/octet-stream"}],
            "see_also": ["http_file_server"],
        },
    ],
}

RESPONSE_CLASS = """<h1>The Response class</h1>
<p>The result of <a href="function.http-request.html">http_request</a>, <a href="function.http-get.html">http_get</a>, and <a href="function.http-post.html">http_post</a>, like a <code>Response</code> in Python's <code>requests</code>.</p>
<h2>Class synopsis</h2>
<div class="synopsis"><code><span class="kw">class</span> <span class="cls">Response</span> {<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">int</span> status;<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> reason;<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">map[string, string]</span> headers;<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">byte[]</span> bytes;<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> url;<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">text</span>(): <span class="type">string</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">header</span>(<span class="type">string</span> name): <span class="type">string</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">ok</span>(): <span class="type">bool</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">raiseForStatus</span>(): <span class="type">void</span><br>
}</code></div>
<table>
<thead><tr><th>Member</th><th>Description</th><th>In requests</th></tr></thead>
<tbody>
<tr><td><code>status</code>, <code>reason</code></td><td>The status code and phrase, such as <code>200</code> and <code>OK</code></td><td><code>status_code</code>, <code>reason</code></td></tr>
<tr><td><code>headers</code></td><td>The response headers, with lowercase names; a header sent more than once has its values joined with <code>, </code></td><td><code>headers</code></td></tr>
<tr><td><code>bytes</code></td><td>The body as bytes, for any content, including images and archives</td><td><code>content</code></td></tr>
<tr><td><code>text()</code></td><td>The body as a string; throws <code>InvalidUtf8Exception</code> if it isn't UTF-8</td><td><code>text</code></td></tr>
<tr><td><code>header(name)</code></td><td>One header, by name in any case, or <code>null</code></td><td><code>headers[name]</code></td></tr>
<tr><td><code>url</code></td><td>The URL that answered, after redirects</td><td><code>url</code></td></tr>
<tr><td><code>ok()</code></td><td>Whether the status is below 400</td><td><code>ok</code></td></tr>
<tr><td><code>raiseForStatus()</code></td><td>Throws <code>HttpException</code> for a 4xx or 5xx status, with the status as its code</td><td><code>raise_for_status()</code></td></tr>
</tbody>
</table>
<p>For JSON, decode the text into the type you expect: <code>User user = json_decode(r-&gt;text());</code>. There's no <code>r-&gt;json()</code>, because <a href="function.json-decode.html">json_decode</a> takes its type from where it's used.</p>
<h2>Example</h2>
<pre class="code"><span class="cls">Response</span> r = <span class="fn">http_get</span>(<span class="lit">"https://example.com/missing"</span>);
<span class="kw">print</span>(r-&gt;status, r-&gt;<span class="fn">ok</span>(), r-&gt;<span class="fn">header</span>(<span class="lit">"Content-Type"</span>));
<span class="kw">try</span> {
    r-&gt;<span class="fn">raiseForStatus</span>();
} <span class="kw">catch</span> (<span class="cls">HttpException</span> e) {
    <span class="kw">print</span>(e-&gt;<span class="fn">getCode</span>(), e-&gt;<span class="fn">getMessage</span>());   <span class="cmt">// 404 404 Client Error: Not Found for https://example.com/missing</span>
}</pre>
"""

SOCKET_CLASS = """<h1>The Socket class</h1>
<p>A <code>Socket</code> is a TCP connection to a server, optionally encrypted with TLS, for talking to services other than HTTP. It works like a <a href="class.file.html">File</a>: write text or bytes, and read lines or chunks. The <a href="ref.http.html">HTTP client</a> is written in qiq on top of it.</p>
<h2>Class synopsis</h2>
<div class="synopsis"><code><span class="kw">final class</span> <span class="cls">Socket</span> {<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">static function</span> <span class="fn">connect</span>(<span class="type">string</span> host, <span class="type">int</span> port, <span class="type">bool</span> tls = <span class="default">false</span>, <span class="type">int</span> timeout = <span class="default">30</span>): <span class="cls">Socket</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">write</span>(<span class="type">string</span> text): <span class="type">void</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">writeBytes</span>(<span class="type">byte[]</span> data): <span class="type">void</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">readLine</span>(): <span class="type">string</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">read</span>(<span class="type">int</span> length): <span class="type">byte[]</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">readExactly</span>(<span class="type">int</span> length): <span class="type">byte[]</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">readAll</span>(): <span class="type">byte[]</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">close</span>(): <span class="type">void</span><br>
}</code></div>
<table>
<thead><tr><th>Method</th><th>Description</th></tr></thead>
<tbody>
<tr><td><code>Socket::connect(host, port, tls, timeout)</code></td><td>Connects to a host name or IP address (IPv4 or IPv6). With <code>tls</code>, encrypts the connection and checks the server's certificate and host name against the system's CA certificates (or <code>SSL_CERT_FILE</code> / <code>SSL_CERT_DIR</code>); TLS 1.2 and 1.3 are supported. Every connect, read, and write waits at most <code>timeout</code> seconds.</td></tr>
<tr><td><code>write(text)</code>, <code>writeBytes(data)</code></td><td>Sends a string (as UTF-8) or bytes, all of it.</td></tr>
<tr><td><code>readLine()</code></td><td>The next line without <code>\\n</code> or <code>\\r\\n</code>, or <code>null</code> once the server has closed the connection. Bytes that aren't valid UTF-8 become U+FFFD, since network data can't be trusted to be well formed.</td></tr>
<tr><td><code>read(length)</code></td><td>Up to <code>length</code> bytes, as soon as any arrive; an empty array once the connection is closed.</td></tr>
<tr><td><code>readExactly(length)</code></td><td>Exactly <code>length</code> bytes; throws <code>NetworkException</code> if the connection closes first.</td></tr>
<tr><td><code>readAll()</code></td><td>Everything until the server closes the connection.</td></tr>
<tr><td><code>close()</code></td><td>Closes the connection (ending a TLS session properly). Closing twice does nothing. Freeing a <code>Socket</code> doesn't close it.</td></tr>
</tbody>
</table>
<p>Failures throw <code>NetworkException</code>, a subclass of <code>IOException</code>, with messages such as <code>could not find host example.invalid</code>, <code>connection refused by localhost:9</code>, <code>timed out reading from api.example.com:443 after 30 seconds</code>, or <code>TLS certificate of expired.badssl.com:443 is not trusted: The certificate validity has expired</code>.</p>
<h2>Example</h2>
<pre class="code"><span class="cls">Socket</span> s = <span class="cls">Socket</span>::<span class="fn">connect</span>(<span class="lit">"example.com"</span>, <span class="lit">443</span>, tls: <span class="lit">true</span>);
<span class="kw">try</span> {
    s-&gt;<span class="fn">write</span>(<span class="lit">"HEAD / HTTP/1.1\\r\\nHost: example.com\\r\\nConnection: close\\r\\n\\r\\n"</span>);
    <span class="kw">print</span>(s-&gt;<span class="fn">readLine</span>());      <span class="cmt">// HTTP/1.1 200 OK</span>
} <span class="kw">finally</span> {
    s-&gt;<span class="fn">close</span>();
}</pre>
<div class="note"><strong>Note:</strong> A program that uses <code>Socket</code> includes the TLS library (Mbed TLS), so its executable is about 750 KB rather than about 10 KB. Programs that don't use the network aren't affected.</div>
"""

REQUEST_CLASS = '<h1>The Request class</h1>\n<p>A request received by the <a href="ref.http-server.html">HTTP server</a>, passed to every handler, like Go\'s <code>*http.Request</code>.</p>\n<h2>Class synopsis</h2>\n<div class="synopsis"><code><span class="kw">class</span> <span class="cls">Request</span> {<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> method;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> path;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> rawPath;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> rawQuery;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> protocol;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> scheme;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">map[string, string]</span> headers;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">byte[]</span> bytes;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> remoteAddress;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">map[string, string]</span> params;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">text</span>(): <span class="type">string</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">header</span>(<span class="type">string</span> name): <span class="type">string</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">query</span>(<span class="type">string</span> name): <span class="type">string</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">form</span>(<span class="type">string</span> name): <span class="type">string</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">pathValue</span>(<span class="type">string</span> name): <span class="type">string</span><br>\n}</code></div>\n<table>\n<thead><tr><th>Member</th><th>Description</th></tr></thead>\n<tbody>\n<tr><td><code>method</code></td><td>The method in upper case: <code>GET</code>, <code>POST</code>, ...</td></tr>\n<tr><td><code>path</code></td><td>The URL path, percent-decoded: <code>/files/a b.txt</code></td></tr>\n<tr><td><code>rawPath</code></td><td>The path as the client sent it: <code>/files/a%20b.txt</code></td></tr>\n<tr><td><code>rawQuery</code></td><td>What follows <code>?</code> in the URL, or an empty string</td></tr>\n<tr><td><code>protocol</code></td><td><code>HTTP/1.1</code> or <code>HTTP/1.0</code></td></tr>\n<tr><td><code>scheme</code></td><td><code>http</code>, or <code>https</code> when a trusted proxy reports that the client used HTTPS (see <code>trustProxy</code> in <a href="function.http-serve.html">http_serve</a>)</td></tr>\n<tr><td><code>headers</code></td><td>The request headers, with lowercase names; a header sent more than once has its values joined with <code>, </code></td></tr>\n<tr><td><code>bytes</code></td><td>The body, empty when there is none</td></tr>\n<tr><td><code>remoteAddress</code></td><td>The client\'s address and port, such as <code>127.0.0.1:51234</code> or <code>[::1]:51234</code>; <code>unix</code> on a Unix socket. With <code>trustProxy</code>, the client address the proxy forwarded, such as <code>203.0.113.9</code></td></tr>\n<tr><td><code>params</code></td><td>The values of the wildcards in the <a href="class.router.html">Router</a> pattern that matched</td></tr>\n<tr><td><code>text()</code></td><td>The body as a string; throws <code>InvalidUtf8Exception</code> if it isn\'t UTF-8</td></tr>\n<tr><td><code>header(name)</code></td><td>One header, by name in any case, or <code>null</code></td></tr>\n<tr><td><code>query(name)</code></td><td>A field of the query string (<code>?page=2</code>), decoded, or <code>null</code>; the first one if it\'s repeated</td></tr>\n<tr><td><code>form(name)</code></td><td>A field of a form posted as <code>application/x-www-form-urlencoded</code>, or else of the query string, or <code>null</code></td></tr>\n<tr><td><code>pathValue(name)</code></td><td>A wildcard\'s value, such as <code>id</code> in <code>/items/{id}</code>, or <code>null</code></td></tr>\n</tbody>\n</table>\n<p>For a JSON body, decode the text into the type you expect: <code>Item item = json_decode(r-&gt;text());</code>.</p>\n<h2>Example</h2>\n<pre class="code">router-&gt;handle("POST /login", function(Request r, ResponseWriter w): void {\n    string user = r-&gt;form("user");\n    if (user == null) {\n        w-&gt;error(400, "missing user");\n        return;\n    }\n    w-&gt;write("Welcome, " + user + " (from " + r-&gt;remoteAddress + ")\\n");\n});</pre>\n'

RESPONSEWRITER_CLASS = '<h1>The ResponseWriter class</h1>\n<p>Builds the response to a request, like Go\'s <code>http.ResponseWriter</code>. The server creates one for each request and sends the response when the handler returns, so the status and headers can be changed at any point until then.</p>\n<h2>Class synopsis</h2>\n<div class="synopsis"><code><span class="kw">final class</span> <span class="cls">ResponseWriter</span> {<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">status</span>(<span class="type">int</span> code): <span class="type">void</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">statusCode</span>(): <span class="type">int</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">header</span>(<span class="type">string</span> name, <span class="type">string</span> value): <span class="type">void</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">getHeader</span>(<span class="type">string</span> name): <span class="type">string</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">write</span>(<span class="type">string</span> text): <span class="type">void</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">writeBytes</span>(<span class="type">byte[]</span> data): <span class="type">void</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">redirect</span>(<span class="type">string</span> url, <span class="type">int</span> code = <span class="default">302</span>): <span class="type">void</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">error</span>(<span class="type">int</span> code, <span class="type">string</span> message = <span class="default">null</span>): <span class="type">void</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">bytes</span>(): <span class="type">byte[]</span><br>\n}</code></div>\n<table>\n<thead><tr><th>Method</th><th>Description</th></tr></thead>\n<tbody>\n<tr><td><code>status(code)</code></td><td>Sets the status code; 200 by default. Throws <code>InvalidArgumentException</code> for a code outside 100–999.</td></tr>\n<tr><td><code>statusCode()</code></td><td>The status code set so far</td></tr>\n<tr><td><code>header(name, value)</code></td><td>Sets a header, replacing one with the same name in any case. <code>Content-Length</code>, <code>Connection</code>, and <code>Transfer-Encoding</code> are set by the server. Throws <code>InvalidArgumentException</code> if the name or value contains a line break.</td></tr>\n<tr><td><code>getHeader(name)</code></td><td>A header set so far, or <code>null</code></td></tr>\n<tr><td><code>write(text)</code>, <code>writeBytes(data)</code></td><td>Appends to the body</td></tr>\n<tr><td><code>redirect(url, code)</code></td><td>Replies with a redirect: sets the status (302 Found by default; 301, 303, 307, and 308 are the others), the <code>Location</code> header, and a short HTML body</td></tr>\n<tr><td><code>error(code, message)</code></td><td>Replaces the body with a plain-text error, by default the status and its reason (<code>404 Not Found</code>), like Go\'s <code>http.Error</code></td></tr>\n<tr><td><code>bytes()</code></td><td>The body written so far, for middleware and tests</td></tr>\n</tbody>\n</table>\n<p>Unless the handler sets a <code>Content-Type</code>, the response is sent as <code>text/plain; charset=utf-8</code>.</p>\n<h2>Example</h2>\n<pre class="code">router-&gt;handle("GET /items/{id}", function(Request r, ResponseWriter w): void {\n    Item item = find_item(r-&gt;pathValue("id"));\n    if (item == null) {\n        w-&gt;error(404);\n        return;\n    }\n    w-&gt;header("Content-Type", "application/json");\n    w-&gt;write(json_encode(item));\n});</pre>\n'

ROUTER_CLASS = '<h1>The Router class</h1>\n<p>Sends each request to a handler chosen by its method and path, like Go\'s <code>http.ServeMux</code> (with the patterns of Go 1.22).</p>\n<h2>Class synopsis</h2>\n<div class="synopsis"><code><span class="kw">final class</span> <span class="cls">Router</span> {<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">handle</span>(<span class="type">string</span> pattern, <span class="kw">function</span>(<span class="cls">Request</span>, <span class="cls">ResponseWriter</span>): <span class="type">void</span> handler): <span class="type">void</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">notFound</span>(<span class="kw">function</span>(<span class="cls">Request</span>, <span class="cls">ResponseWriter</span>): <span class="type">void</span> handler): <span class="type">void</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">serve</span>(<span class="cls">Request</span> r, <span class="cls">ResponseWriter</span> w): <span class="type">void</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">handler</span>(): <span class="kw">function</span>(<span class="cls">Request</span>, <span class="cls">ResponseWriter</span>): <span class="type">void</span><br>\n}</code></div>\n<h2>Patterns</h2>\n<p>A pattern is an optional method and a path: <code>[METHOD ]/path</code>.</p>\n<table>\n<thead><tr><th>Pattern</th><th>Matches</th></tr></thead>\n<tbody>\n<tr><td><code>/about</code></td><td>Only <code>/about</code>, with any method</td></tr>\n<tr><td><code>GET /about</code></td><td>Only <code>/about</code>, with <code>GET</code> or <code>HEAD</code></td></tr>\n<tr><td><code>/static/</code></td><td><code>/static/</code> and every path below it; <code>/static</code> is redirected to <code>/static/</code></td></tr>\n<tr><td><code>/</code></td><td>Every path: what no other pattern matches</td></tr>\n<tr><td><code>/{$}</code></td><td>Only <code>/</code> itself</td></tr>\n<tr><td><code>/items/{id}</code></td><td><code>/items/7</code>, with <code>r-&gt;pathValue("id")</code> = <code>7</code>; a wildcard matches one non-empty segment, decoded</td></tr>\n<tr><td><code>/files/{path...}</code></td><td><code>/files/a/b.txt</code>, with <code>path</code> = <code>a/b.txt</code>; only as the last segment</td></tr>\n</tbody>\n</table>\n<p>When several patterns match, the most specific wins: at the first segment where they differ, a fixed name beats <code>{name}</code>, which beats <code>{name...}</code> or a trailing <code>/</code>; then the longer pattern wins, then the one with a method. So <code>/items/new</code> beats <code>/items/{id}</code>, whatever the order they were added in. A path that only matches patterns for other methods gets <code>405 Method Not Allowed</code> with an <code>Allow</code> header; one that matches nothing gets <code>404 Not Found</code>, or the <code>notFound</code> handler.</p>\n<p><code>handle</code> throws <code>InvalidArgumentException</code> for a malformed pattern or one that duplicates an earlier pattern.</p>\n<table>\n<thead><tr><th>Method</th><th>Description</th></tr></thead>\n<tbody>\n<tr><td><code>handle(pattern, handler)</code></td><td>Adds a route</td></tr>\n<tr><td><code>notFound(handler)</code></td><td>Sets the handler for requests that match no pattern</td></tr>\n<tr><td><code>serve(r, w)</code></td><td>Handles a request: finds the route, sets <code>r-&gt;params</code>, and calls its handler. Useful in tests, with a <code>Request</code> built by hand.</td></tr>\n<tr><td><code>handler()</code></td><td>The router as a handler, for <a href="function.http-serve.html">http_serve</a> and middleware such as <a href="function.http-log.html">http_log</a></td></tr>\n</tbody>\n</table>\n<h2>Example</h2>\n<pre class="code">Router router = new Router();\nrouter-&gt;handle("GET /{$}", fn(Request r, ResponseWriter w) =&gt; w-&gt;write("home\\n"));\nrouter-&gt;handle("GET /items/{id}", fn(Request r, ResponseWriter w) =&gt; w-&gt;write("item " + r-&gt;pathValue("id") + "\\n"));\nrouter-&gt;handle("GET /items/new", fn(Request r, ResponseWriter w) =&gt; w-&gt;write("new item form\\n"));\nrouter-&gt;handle("DELETE /items/{id}", function(Request r, ResponseWriter w): void {\n    w-&gt;status(204);\n});\n\n// Without a server: build a request and call serve().\nRequest request = new Request("GET", "/items/42", "", "HTTP/1.1", {"host" =&gt; "example.com"}, [], "127.0.0.1:5000");\nResponseWriter writer = new ResponseWriter();\nrouter-&gt;serve(request, writer);\nprint(writer-&gt;statusCode(), bytes_to_string(writer-&gt;bytes()));   // 200 item 42\n\nhttp_serve(":8080", router-&gt;handler());</pre>\n'

ALGORITHMS = "<code>\"md5\"</code>, <code>\"sha1\"</code>, <code>\"sha224\"</code>, <code>\"sha256\"</code>, <code>\"sha384\"</code>, or <code>\"sha512\"</code>, in any case"
UNKNOWN_ALGORITHM = "<code>InvalidArgumentException</code> for an unknown algorithm."

CRYPTO = {
    "slug": "crypto",
    "title": "Hashing and encoding",
    "intro": (
        "Message digests, HMACs, hex, base64, and password hashing, from <code>std/crypto</code>, with PHP's names and results: <code>sha1(\"abc\")</code> "
        "and <code>base64_encode(\"abc\")</code> give what they give in PHP. Text is hashed and encoded as its UTF-8 bytes. "
        "Because a qiq string must be UTF-8 text, binary data, such as a file read with <code>file_get_bytes</code>, goes "
        "through the <code>_bytes</code> functions, which take or return a <code>byte[]</code>. "
        "The hashing is done by Mbed TLS, which every qiq program already includes for HTTPS."
    ),
    "details": """<h2>Choosing an algorithm</h2>
<table>
<thead><tr><th>Algorithm</th><th>Digest</th><th>Use</th></tr></thead>
<tbody>
<tr><td><code>md5</code></td><td>16 bytes, 32 hex digits</td><td>Checksums, cache keys, and formats that require it. Not for security: collisions can be made on purpose.</td></tr>
<tr><td><code>sha1</code></td><td>20 bytes, 40 hex digits</td><td>The same, and Git object names. Not for security either.</td></tr>
<tr><td><code>sha224</code>, <code>sha256</code></td><td>28 and 32 bytes</td><td>The usual choice for integrity checks and signatures (with <code>hash_hmac</code>).</td></tr>
<tr><td><code>sha384</code>, <code>sha512</code></td><td>48 and 64 bytes</td><td>The same, with longer digests.</td></tr>
</tbody>
</table>
<div class="note"><strong>Note:</strong> None of these is a way to store passwords: they're fast, which makes guessing quick. Use <a href="function.bcrypt-hash.html">bcrypt_hash</a> and <a href="function.bcrypt-verify.html">bcrypt_verify</a>, which are slow on purpose.</div>""",
    "functions": [
        {
            "name": "md5",
            "purpose": "Compute a string's MD5 digest",
            "params": [param("text", "string", "The text to hash, as its UTF-8 bytes.")],
            "returns": "string",
            "description": "Returns the MD5 digest of <code>text</code> as 32 lowercase hex digits, like PHP's <code>md5</code>. The same as <code>hash(\"md5\", text)</code>.",
            "return_description": "The digest, in hex.",
            "errors": [NULL_STRING],
            "examples": [{"title": "Basic use", "code": 'print(md5("abc"));', "output": "900150983cd24fb0d6963f7d28e17f72"}],
            "notes": ["MD5 isn't safe where someone could try to make two inputs with the same digest; use <code>hash(\"sha256\", text)</code> there."],
            "see_also": ["sha1", "hash", "hash_bytes"],
        },
        {
            "name": "sha1",
            "purpose": "Compute a string's SHA-1 digest",
            "params": [param("text", "string", "The text to hash, as its UTF-8 bytes.")],
            "returns": "string",
            "description": "Returns the SHA-1 digest of <code>text</code> as 40 lowercase hex digits, like PHP's <code>sha1</code>. The same as <code>hash(\"sha1\", text)</code>.",
            "return_description": "The digest, in hex.",
            "errors": [NULL_STRING],
            "examples": [{"title": "Basic use", "code": 'print(sha1("abc"));', "output": "a9993e364706816aba3e25717850c26c9cd0d89d"}],
            "notes": ["Like MD5, SHA-1 is broken for security uses; prefer SHA-256 for anything new."],
            "see_also": ["md5", "hash", "hash_hmac"],
        },
        {
            "name": "hash",
            "purpose": "Compute a string's digest with a chosen algorithm",
            "params": [
                param("algorithm", "string", "The algorithm: " + ALGORITHMS + "."),
                param("text", "string", "The text to hash, as its UTF-8 bytes."),
            ],
            "returns": "string",
            "description": "Returns the digest of <code>text</code> as lowercase hex, like PHP's <code>hash</code>.",
            "return_description": "The digest, in hex: twice as many digits as the digest has bytes.",
            "errors": [UNKNOWN_ALGORITHM, NULL_STRING],
            "examples": [
                {
                    "title": "SHA-256",
                    "code": 'print(hash("sha256", "abc"));\nprint(len(hash("sha512", "abc")));',
                    "output": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n128",
                }
            ],
            "notes": ["To hash binary data, use <a href=\"function.hash-bytes.html\">hash_bytes</a>."],
            "see_also": ["hash_hmac", "hash_bytes", "md5", "sha1"],
        },
        {
            "name": "hash_hmac",
            "purpose": "Compute an HMAC",
            "params": [
                param("algorithm", "string", "The algorithm: " + ALGORITHMS + "."),
                param("text", "string", "The message."),
                param("key", "string", "The secret key."),
            ],
            "returns": "string",
            "description": "Returns the HMAC of <code>text</code> with <code>key</code> as lowercase hex, like PHP's <code>hash_hmac</code>, with the arguments in the same order. An HMAC shows that a message came from someone who knows the key and wasn't changed on the way, which is how webhooks and many APIs sign requests.",
            "return_description": "The HMAC, in hex.",
            "errors": [UNKNOWN_ALGORITHM, NULL_STRING],
            "examples": [
                {
                    "title": "Signing a message",
                    "code": 'print(hash_hmac("sha256", "what do ya want for nothing?", "Jefe"));',
                    "output": "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
                }
            ],
            "notes": ["To check a signature, compute it again and compare it with <code>==</code>."],
            "see_also": ["hash", "sha1"],
        },
        {
            "name": "hash_bytes",
            "purpose": "Compute the raw digest of bytes",
            "params": [
                param("algorithm", "string", "The algorithm: " + ALGORITHMS + "."),
                param("data", "byte[]", "The bytes to hash."),
            ],
            "returns": "byte[]",
            "description": "Returns the digest of <code>data</code> as bytes, like PHP's <code>hash</code> with <code>binary</code> set. Use it for data that isn't text, such as a file read with <code>file_get_bytes</code>, and where a raw digest is needed; <a href=\"function.bin2hex.html\">bin2hex</a> turns it into hex.",
            "return_description": "The digest: 16 bytes for MD5, 20 for SHA-1, 28, 32, 48, or 64 for SHA-2.",
            "errors": [UNKNOWN_ALGORITHM, "<code>RuntimeException</code> if <code>data</code> is <code>null</code>."],
            "examples": [
                {
                    "title": "Hashing bytes",
                    "code": 'byte[] digest = hash_bytes("md5", string_to_bytes("abc"));\nprint(len(digest), bin2hex(digest));',
                    "output": "16 900150983cd24fb0d6963f7d28e17f72",
                }
            ],
            "notes": ["A file's checksum: <code>bin2hex(hash_bytes(\"sha256\", file_get_bytes(path)))</code>."],
            "see_also": ["hash", "bin2hex"],
        },
        {
            "name": "bin2hex",
            "purpose": "Write bytes as hex",
            "params": [param("data", "byte[]", "The bytes.")],
            "returns": "string",
            "description": "Returns <code>data</code> as lowercase hex, two digits per byte, like PHP's <code>bin2hex</code>.",
            "return_description": "The hex text; <code>\"\"</code> for no bytes.",
            "errors": ["<code>RuntimeException</code> if <code>data</code> is <code>null</code>."],
            "examples": [{"title": "Basic use", "code": 'print(bin2hex([1, 171, 255]));', "output": "01abff"}],
            "see_also": ["hash_bytes", "string_to_bytes", "base64_encode_bytes"],
        },
        {
            "name": "base64_encode",
            "purpose": "Encode text as base64",
            "params": [param("text", "string", "The text, encoded as its UTF-8 bytes.")],
            "returns": "string",
            "description": "Returns <code>text</code> in base64 (RFC 4648), with the standard alphabet and <code>=</code> padding, like PHP's <code>base64_encode</code>.",
            "return_description": "The base64 text: 4 characters for every 3 bytes, rounded up.",
            "errors": [NULL_STRING],
            "examples": [{"title": "Basic use", "code": 'print(base64_encode("user:secret"));\nprint(base64_encode("héllo"));', "output": "dXNlcjpzZWNyZXQ=\naMOpbGxv"}],
            "notes": ["For an HTTP Basic <code>Authorization</code> header: <code>\"Basic \" + base64_encode(user + \":\" + password)</code>."],
            "see_also": ["base64_decode", "base64_encode_bytes"],
        },
        {
            "name": "base64_encode_bytes",
            "purpose": "Encode bytes as base64",
            "params": [param("data", "byte[]", "The bytes.")],
            "returns": "string",
            "description": "Returns <code>data</code> in base64, as <code>base64_encode</code> does for text. Use it for binary data such as images, keys, and digests.",
            "return_description": "The base64 text.",
            "errors": ["<code>RuntimeException</code> if <code>data</code> is <code>null</code>."],
            "examples": [{"title": "Encoding bytes", "code": 'print(base64_encode_bytes([0, 255, 16]));\nprint(base64_encode_bytes(hash_bytes("md5", string_to_bytes("abc"))));', "output": "AP8Q\nkAFQmDzST7DWlj99KOF/cg=="}],
            "see_also": ["base64_decode_bytes", "base64_encode", "hash_bytes"],
        },
        {
            "name": "base64_decode",
            "purpose": "Decode base64 into text",
            "params": [param("text", "string", "The base64 text.")],
            "returns": "string",
            "description": "Returns the text that <code>text</code> encodes. The standard and the URL-safe alphabets are both accepted (<code>+</code> or <code>-</code>, <code>/</code> or <code>_</code>), whitespace and line breaks are skipped, and the <code>=</code> padding may be left out. The bytes it decodes to must be UTF-8 text.",
            "return_description": "The decoded text.",
            "errors": [
                "<code>InvalidArgumentException</code> for anything else: a character that isn't base64, <code>=</code> in the wrong place, or text that ends partway through a byte. The message gives the position.",
                "<code>InvalidUtf8Exception</code> if the decoded bytes aren't UTF-8 text (or contain a NUL byte); use <a href=\"function.base64-decode-bytes.html\">base64_decode_bytes</a> for binary data.",
                NULL_STRING,
            ],
            "examples": [
                {
                    "title": "Decoding, and rejecting what isn't base64",
                    "code": 'print(base64_decode("dXNlcjpzZWNyZXQ="), base64_decode("aGk"));\ntry {\n    print(base64_decode("aGk!"));\n} catch (InvalidArgumentException e) {\n    print(e->getMessage());\n}',
                    "output": "user:secret hi\nbase64_decode: a character that isn't base64 at position 3",
                }
            ],
            "notes": ["PHP's <code>base64_decode</code> drops characters it can't read unless <code>strict</code> is set; qiq throws instead, so corrupted data is noticed."],
            "see_also": ["base64_encode", "base64_decode_bytes"],
        },
        {
            "name": "base64_decode_bytes",
            "purpose": "Decode base64 into bytes",
            "params": [param("text", "string", "The base64 text.")],
            "returns": "byte[]",
            "description": "Returns the bytes that <code>text</code> encodes, accepting what <code>base64_decode</code> accepts. Use it for binary data, and for the parts of a JWT, which use the URL-safe alphabet without padding.",
            "return_description": "The decoded bytes.",
            "errors": ["<code>InvalidArgumentException</code> as for <code>base64_decode</code>.", NULL_STRING],
            "examples": [{"title": "Decoding bytes", "code": 'print(base64_decode_bytes("AP8Q"), base64_decode_bytes("-_8"));', "output": "[0, 255, 16] [251, 255]"}],
            "see_also": ["base64_encode_bytes", "base64_decode", "bin2hex"],
        },
        {
            "name": "bcrypt_hash",
            "purpose": "Hash a password for storing, with bcrypt",
            "params": [
                param("password", "string", "The password, as its UTF-8 bytes: at most 72 bytes."),
                param("cost", "int", "How much work hashing takes, as a power of two: 4 to 31. Each step doubles the time; 12 takes about a third of a second.", "12"),
            ],
            "returns": "string",
            "description": "Returns a bcrypt hash of <code>password</code> with a new random salt, in the form PHP's <code>password_hash</code> makes: 60 characters starting <code>$2y$</code> and the cost, as in <code>$2y$12$</code>. The salt and cost are part of the hash, so store the hash and nothing else, and check a password against it with <a href=\"function.bcrypt-verify.html\">bcrypt_verify</a>. Hashing the same password twice gives different hashes.",
            "return_description": "The hash, 60 characters.",
            "errors": [
                "<code>InvalidArgumentException</code> if the cost isn't 4 to 31, or the password is longer than 72 bytes. bcrypt only uses the first 72 bytes; PHP ignores the rest without saying so, which makes passwords that differ only after 72 bytes match.",
                NULL_STRING,
            ],
            "examples": [
                {
                    "title": "Storing and checking a password",
                    "code": 'string stored = bcrypt_hash("correct horse battery staple");\nprint(len(stored), substr(stored, 0, 7));\nprint(bcrypt_verify("correct horse battery staple", stored), bcrypt_verify("Correct horse battery staple", stored));',
                    "output": "60 $2y$12$\n1 0",
                }
            ],
            "notes": [
                "This is PHP's <code>password_hash</code> with <code>PASSWORD_BCRYPT</code> (or <code>PASSWORD_DEFAULT</code>, which is bcrypt). qiq names the algorithm in the function, and has no other password hashing yet.",
                "The default cost is 12, as in PHP 8.4; PHP 8.3 uses 10. Choose the highest cost your server can afford per login.",
                "Hashes from PHP's <code>password_hash</code> and <code>crypt</code> work with <code>bcrypt_verify</code>, and the other way round.",
            ],
            "see_also": ["bcrypt_verify", "hash_hmac"],
        },
        {
            "name": "bcrypt_verify",
            "purpose": "Check a password against a bcrypt hash",
            "params": [
                param("password", "string", "The password to check."),
                param("hash", "string", "A hash from <code>bcrypt_hash</code>: a bcrypt hash starting <code>$2y$</code>, <code>$2b$</code>, or <code>$2a$</code>."),
            ],
            "returns": "bool",
            "description": "Returns whether <code>password</code> is the password <code>hash</code> was made from, like PHP's <code>password_verify</code>. It hashes <code>password</code> again with the hash's salt and cost, so it takes as long as <code>bcrypt_hash</code> did, and compares the results in a way that takes the same time however much of them matches.",
            "return_description": "<code>true</code> if the password matches; <code>false</code> if it doesn't, or if <code>hash</code> isn't a bcrypt hash.",
            "errors": [NULL_STRING],
            "examples": [
                {
                    "title": "Checking a hash made by PHP",
                    "code": 'string hash = "$2y$05$maMq.WEGV.pFV8IwOFggee/pF6YMCR8YqltautITTO9oL9/6v3xhu";\nprint(bcrypt_verify("password", hash), bcrypt_verify("passw0rd", hash), bcrypt_verify("password", "not a hash"));',
                    "output": "1 0 0",
                }
            ],
            "notes": [
                "This is PHP's <code>password_verify</code>, for bcrypt hashes.",
                "As bcrypt does, only the first 72 bytes of <code>password</code> count, so hashes PHP made from longer passwords still verify.",
            ],
            "see_also": ["bcrypt_hash"],
        },
    ],
}

CATEGORIES = [GENERAL, STRINGS, ARRAYS, MAPS, FILES, DATES, JSON, HTTP, HTTP_SERVER, CRYPTO, PROGRAM, CONVERSIONS, OUTPUT]

FILE_CLASS = """<h1>The File class</h1>
<p>A <code>File</code> reads or writes a file a piece at a time: a line, or a chunk of bytes. Use it for files too large to load at once, for files you build up gradually, and for standard input and output. To read or write a whole file in one call, the <a href="ref.files.html">file functions</a> are simpler.</p>
<h2>Class synopsis</h2>
<div class="synopsis"><code><span class="kw">final class</span> <span class="cls">File</span> {<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">static function</span> <span class="fn">open</span>(<span class="type">string</span> path, <span class="type">string</span> mode = <span class="default">"r"</span>): <span class="cls">File</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">static function</span> <span class="fn">stdin</span>(): <span class="cls">File</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">static function</span> <span class="fn">stdout</span>(): <span class="cls">File</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">static function</span> <span class="fn">stderr</span>(): <span class="cls">File</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">readLine</span>(): <span class="type">string</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">read</span>(<span class="type">int</span> length): <span class="type">byte[]</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">write</span>(<span class="type">string</span> text): <span class="type">void</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">writeBytes</span>(<span class="type">byte[]</span> data): <span class="type">void</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">eof</span>(): <span class="type">bool</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">flush</span>(): <span class="type">void</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">close</span>(): <span class="type">void</span><br>
}</code></div>
<p><code>File</code> is final and its constructor is private: get a <code>File</code> from <code>open</code>, <code>stdin</code>, <code>stdout</code>, or <code>stderr</code>.</p>

<h2>Methods</h2>
<table>
<thead><tr><th>Method</th><th>Description</th></tr></thead>
<tbody>
<tr><td><code>File::open(path, mode)</code></td><td>Opens <code>path</code>, relative to the directory the program runs in. See the modes below.</td></tr>
<tr><td><code>File::stdin()</code>, <code>File::stdout()</code>, <code>File::stderr()</code></td><td>The program's standard streams. Output through <code>File::stdout()</code> and <code>print</code> goes to the same place, in order.</td></tr>
<tr><td><code>readLine()</code></td><td>The next line without its line ending (<code>\\n</code> or <code>\\r\\n</code>), or <code>null</code> at the end of the file. A last line without a line ending is still returned. Throws <code>InvalidUtf8Exception</code> if the line isn't UTF-8.</td></tr>
<tr><td><code>read(length)</code></td><td>Up to <code>length</code> bytes: fewer only at the end of the file, and an empty array after it. Works for any file, including binary ones.</td></tr>
<tr><td><code>write(text)</code></td><td>Writes a string as UTF-8.</td></tr>
<tr><td><code>writeBytes(data)</code></td><td>Writes raw bytes.</td></tr>
<tr><td><code>eof()</code></td><td>Whether there's nothing left to read. It looks ahead, so it's <code>true</code> before a read would come back empty. On standard input it waits for input.</td></tr>
<tr><td><code>flush()</code></td><td>Writes buffered output now.</td></tr>
<tr><td><code>close()</code></td><td>Flushes and closes the file. Closing twice does nothing, so it's safe in <code>finally</code>. Closing a standard stream only flushes it.</td></tr>
</tbody>
</table>

<h2>Modes</h2>
<table>
<thead><tr><th>Mode</th><th>Opens for</th><th>If the file exists</th><th>If it doesn't</th></tr></thead>
<tbody>
<tr><td><code>"r"</code></td><td>Reading</td><td>Reads from the start</td><td><code>FileNotFoundException</code></td></tr>
<tr><td><code>"w"</code></td><td>Writing</td><td>Empties it</td><td>Creates it</td></tr>
<tr><td><code>"a"</code></td><td>Appending</td><td>Writes go to the end</td><td>Creates it</td></tr>
<tr><td><code>"x"</code></td><td>Writing a new file</td><td><code>IOException</code></td><td>Creates it</td></tr>
</tbody>
</table>
<p>Add <code>+</code> (<code>"r+"</code>, <code>"w+"</code>, <code>"a+"</code>, <code>"x+"</code>) to allow both reading and writing. Files are always opened in binary mode: nothing translates line endings.</p>

<h2>Errors</h2>
<p>Methods throw the same exceptions as the <a href="ref.files.html">file functions</a>: <code>FileNotFoundException</code>, <code>PermissionDeniedException</code>, or <code>IOException</code>. Using a file after <code>close()</code> throws <code>IOException</code>, an unknown mode or a negative <code>read</code> length throws <code>InvalidArgumentException</code>, and writing a <code>null</code> string or array throws <code>RuntimeException</code>.</p>

<h2>Examples</h2>
<h3>Reading line by line</h3>
```qiq
int written = file_put_contents("app.log", "INFO started\\nERROR disk full\\r\\nINFO done\\nERROR timeout");

File log = File::open("app.log");
try {
    string line = log->readLine();
    while (line != null) {
        if (str_starts_with(line, "ERROR")) {
            print(line);
        }
        line = log->readLine();
    }
} finally {
    log->close();
}
```
```output
ERROR disk full
ERROR timeout
```

<h3>Writing a file gradually</h3>
```qiq
File report = File::open("squares.csv", "w");
report->write("n,square\\n");
int n = 1;
while (n <= 3) {
    report->write(to_string(n) + "," + to_string(n * n) + "\\n");
    n = n + 1;
}
report->close();
print(file("squares.csv"));
```
```output
["n,square", "1,1", "2,4", "3,9"]
```

<h3>Reading bytes in chunks</h3>
```qiq
int written = file_put_bytes("data.bin", [1, 2, 3, 4, 5, 6, 7]);
File data = File::open("data.bin");
byte[] chunk = data->read(3);
while (len(chunk) > 0) {
    print(chunk);
    chunk = data->read(3);
}
data->close();
```
```output
[1, 2, 3]
[4, 5, 6]
[7]
```

<h3>Reading standard input</h3>
<p>A program that reads <code>File::stdin()</code> works in a shell pipeline:</p>
```qiq
File input = File::stdin();
int count = 0;
while (!input->eof()) {
    string line = input->readLine();
    count = count + 1;
}
print(count, "lines");
```
```sh
cat app.log | qiq run count_lines.qiq
```

<div class="note"><strong>Note:</strong> A line or chunk is freed as soon as nothing refers to it, so reading a file a piece at a time uses the same small amount of memory however large the file is. Freeing a <code>File</code> doesn't close it; call <code>close()</code>. See <a href="memory.html">Memory management</a>.</div>
"""

EXCEPTION_CLASSES = """<h1>Exception classes</h1>
<p>These classes are available in every file without an import, wherever they're defined. The runtime and the standard library throw them, and your own exceptions extend them. See <a href="exceptions.html">Exceptions</a> for <code>throw</code>, <code>try</code>, <code>catch</code>, and <code>finally</code>.</p>
<h2>Hierarchy</h2>
<pre class="tree">Exception
├── RuntimeException
│   ├── IOException
│   │   ├── FileNotFoundException
│   │   ├── PermissionDeniedException
│   │   └── NetworkException
│   ├── KeyNotFoundException
│   ├── OutOfRangeException
│   └── InvalidUtf8Exception
├── ArithmeticException
│   └── DivisionByZeroException
├── InvalidArgumentException
├── JsonException
└── HttpException</pre>
<h2>The Exception class</h2>
<div class="synopsis"><code><span class="kw">class</span> <span class="cls">Exception</span> {<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">protected</span> <span class="type">string</span> message;<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">protected</span> <span class="type">int</span> code;<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">__construct</span>(<span class="type">string</span> message = <span class="default">""</span>, <span class="type">int</span> code = <span class="default">0</span>)<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">getMessage</span>(): <span class="type">string</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">getCode</span>(): <span class="type">int</span><br>
}</code></div>
<p>Every subclass inherits the constructor and both methods. A subclass that declares no constructor uses this one, so <code>class ConfigException extends Exception {}</code> is enough to define a new exception.</p>
<h2>When each is thrown</h2>
<table>
<thead><tr><th>Class</th><th>Thrown when</th></tr></thead>
<tbody>
<tr><td><code>RuntimeException</code></td><td>An error that can only be detected while the program runs, such as using a null array or string, or <code>new static</code> on an abstract class.</td></tr>
<tr><td><code>IOException</code></td><td>Reading or writing a file failed for a reason other than the two below, such as reading a directory.</td></tr>
<tr><td><code>FileNotFoundException</code></td><td>A path given to a file function doesn't exist.</td></tr>
<tr><td><code>PermissionDeniedException</code></td><td>The operating system denied access to a file.</td></tr>
<tr><td><code>KeyNotFoundException</code></td><td>A map lookup used a key that isn't in the map.</td></tr>
<tr><td><code>OutOfRangeException</code></td><td>An array index is below 0 or not less than the array's length.</td></tr>
<tr><td><code>InvalidUtf8Exception</code></td><td>Bytes or file contents that should be text aren't valid UTF-8.</td></tr>
<tr><td><code>ArithmeticException</code></td><td>An int raised to a negative power, a negative shift amount, or a double that can't be converted to an int.</td></tr>
<tr><td><code>DivisionByZeroException</code></td><td>Int <code>/</code> or <code>%</code> with a zero right-hand side.</td></tr>
<tr><td><code>InvalidArgumentException</code></td><td>A standard library function was given an argument it can't accept, such as a negative <code>str_repeat</code> count.</td></tr>
<tr><td><code>JsonException</code></td><td><code>json_decode</code> was given text that isn't JSON or doesn't match the type it decodes into, or <code>json_encode</code> was given a value JSON can't represent.</td></tr>
<tr><td><code>NetworkException</code></td><td>A network connection failed: the host couldn't be found, refused the connection, timed out, closed it early, or failed the TLS certificate check. A subclass of <code>IOException</code>.</td></tr>
<tr><td><code>HttpException</code></td><td><code>Response::raiseForStatus()</code> found a 4xx or 5xx status (the status is the exception's code), or a request was redirected more than 10 times.</td></tr>
</tbody>
</table>
<h2>Example</h2>
```qiq
class ConfigException extends Exception {}

try {
    throw new ConfigException("port is empty", 7);
} catch (Exception e) {
    print(e->getMessage(), e->getCode());
}

map[string, int] ages = {"ada" => 36};
try {
    print(ages["bob"]);
} catch (KeyNotFoundException e) {
    print("KeyNotFoundException:", e->getMessage());
}
```
```output
port is empty 7
KeyNotFoundException: map key not found
```
"""


# Pages for standard library classes: (href, title, summary, html).
CLASS_PAGES = [
    ("class.file.html", "The File class", "Reading and writing files a line or a chunk at a time, including standard input and output.", FILE_CLASS),
    ("class.response.html", "The Response class", "The result of an HTTP request: status, headers, and body.", RESPONSE_CLASS),
    ("class.request.html", "The Request class", "A request received by the HTTP server: method, path, headers, and body.", REQUEST_CLASS),
    ("class.responsewriter.html", "The ResponseWriter class", "Builds the HTTP server's response to a request.", RESPONSEWRITER_CLASS),
    ("class.router.html", "The Router class", "Sends each HTTP request to a handler by method and path.", ROUTER_CLASS),
    ("class.socket.html", "The Socket class", "TCP connections, optionally encrypted with TLS, for talking to network services.", SOCKET_CLASS),
    ("exception-classes.html", "Exception classes", "The exception classes thrown by the language and the standard library.", EXCEPTION_CLASSES),
]
