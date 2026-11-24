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
    "<tr><td><code>std/time</code></td><td><a href=\"ref.datetime.html\">Date and time</a>, and the <a href=\"class.datetime.html\">DateTime</a> and <a href=\"class.duration.html\">Duration</a> classes</td></tr>"
    "<tr><td><code>std/os</code></td><td>The <a href=\"ref.program.html\">program</a>: arguments, exit status, environment, and running other commands</td></tr>"
    "<tr><td><code>std/memory</code></td><td><a href=\"function.gc-collect-cycles.html\">gc_collect_cycles</a></td></tr>"
    "<tr><td><code>std/net</code></td><td>The <a href=\"class.socket.html\">Socket</a> and <a href=\"class.url.html\">Url</a> classes</td></tr>"
    "<tr><td><code>std/net/http</code></td><td>The <a href=\"ref.http.html\">HTTP client</a> and URL encoding</td></tr>"
    "<tr><td><code>std/net/http/server</code></td><td>The <a href=\"ref.http-server.html\">HTTP server</a></td></tr>"
    "<tr><td><code>std/crypto</code></td><td><a href=\"ref.crypto.html\">Hashing and encoding</a>: MD5, SHA-1, SHA-2, HMAC, hex, base64, and password hashing</td></tr>"
    "<tr><td><code>std/csv</code></td><td><a href=\"ref.csv.html\">CSV</a>: reading and writing CSV as rows, by column name, as objects, or a row at a time</td></tr>"
    "<tr><td><code>std/testing</code></td><td>The <code>T</code> class that <a href=\"testing.html\">tests</a> take, for <code>qiq test</code></td></tr>"
    "<tr><td><code>std/math</code></td><td><a href=\"ref.math.html\">Math</a>: rounding, roots, logarithms, trigonometry, <code>number_format</code>, and secure random numbers</td></tr>"
    "<tr><td><code>std/regex</code></td><td><a href=\"ref.regex.html\">Regular expressions</a>, with PHP's <code>preg_*</code> functions, and the <a href=\"class.regexmatch.html\">RegexMatch</a> class</td></tr>"
    "<tr><td><code>std/sql</code></td><td>SQL databases (SQLite, PostgreSQL, MySQL, and MariaDB): the <a href=\"class.database.html\">Database</a>, <a href=\"class.row.html\">Row</a>, and <a href=\"class.cursor.html\">Cursor</a> classes</td></tr>"
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
                    "output": "found\nfalse",
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
                    "output": "secure\nfalse",
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
                    "output": "=-=-=-=-=-\ntrue",
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
                    "output": "true false false",
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
            "examples": [{"title": "Basic use", "code": 'print(is_dir("."), is_dir("missing"));', "output": "true false"}],
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
            "notes": ["<code>to_string(true)</code> is <code>\"true\"</code>, as <code>print(true)</code> prints it."],
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
                    "output": "age: 36 2.5 true\n[1, 2, 3] [\"a\", \"b,c\"]\n{\"ada\" => [true, false]}\nSquare\n\ndone",
                }
            ],
            "notes": [
                "Booleans print as <code>true</code> and <code>false</code>. Inside arrays and maps, strings and chars are quoted, so the output reads like a qiq literal.",
            ],
            "see_also": ["println", "to_string"],
        },
        {
            "name": "println",
            "purpose": "Write values to standard output, one per line",
            "params": [param("values", "any...", "Zero or more values of any type except <code>void</code>.")],
            "returns": "void",
            "description": "Writes each of its arguments as <a href=\"function.print.html\">print</a> writes it, each on its own line. With one value or none, it's the same as <code>print</code>. Like <code>print</code>, it's a statement built into the language.",
            "return_description": "Nothing; <code>println</code> is a statement.",
            "errors": [],
            "examples": [
                {
                    "title": "One value per line",
                    "code": 'string name = "Ada";\nint age = 36;\nprintln(name, age, [1, 2], true);\nprint(name, age);',
                    "output": "Ada\n36\n[1, 2]\ntrue\nAda 36",
                }
            ],
            "see_also": ["print"],
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
        "zoneinfo files when the program runs, so compiled programs stay static. "
        "For dates you calculate with, compare, or keep in several time zones, the <a href=\"class.datetime.html\">DateTime</a> class is easier."
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
                    "output": "true",
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
                    "output": "true",
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
                    "output": "true false false false",
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
                    "output": "true",
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
                    "output": "36\nfalse",
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
                    "output": "true false",
                }
            ],
            "notes": ["This checks every entry, so it takes time proportional to the size of the map. PHP's equivalent is <code>in_array</code>."],
            "see_also": ["map_key_exists", "map_values"],
        },
        {
            "name": "map_merge",
            "purpose": "Combine maps",
            "params": [param("maps", "map[K, V]...", "Two or more maps of the same type.")],
            "returns": "map[K, V]",
            "description": "Returns a new map with every entry of every map. For a key in more than one, the value from the last map wins, in the place the key first appeared, as with PHP's <code>array_merge</code> for string keys (and unlike it for int keys, which keep their values rather than being renumbered). The maps aren't changed.",
            "return_description": "The combined map.",
            "errors": ["<code>RuntimeException</code> if a map is <code>null</code>. Maps of different types are a compile error."],
            "examples": [{"title": "Settings over defaults", "code": 'map[string, int] defaults = {"port" => 80, "workers" => 4};\nmap[string, int] config = map_merge(defaults, {"port" => 8080});\nprint(config, defaults);',
                          "output": '{"port" => 8080, "workers" => 4} {"port" => 80, "workers" => 4}'}],
            "see_also": ["map_diff", "map_intersect_keys"],
        },
        {
            "name": "map_diff_keys",
            "purpose": "The entries whose keys another map doesn't have",
            "params": [param("map", "map[K, V]", "The map to take entries from."), param("other", "map[K, W]", "The map whose keys to leave out; its values can be of any type.")],
            "returns": "map[K, V]",
            "description": "Returns a new map with the entries of <code>map</code> whose key isn't in <code>other</code>, in <code>map</code>'s order, like PHP's <code>array_diff_key</code>. Only keys are compared, so the two maps need the same key type but not the same value type.",
            "return_description": "The entries left.",
            "errors": ["<code>RuntimeException</code> if a map is <code>null</code>. Maps with different key types are a compile error."],
            "examples": [{"title": "Removing fields", "code": 'map[string, string] user = {"name" => "Ada", "password" => "secret", "email" => "ada@example.com"};\nmap[string, bool] hidden = {"password" => true};\nprint(map_diff_keys(user, hidden));',
                          "output": '{"name" => "Ada", "email" => "ada@example.com"}'}],
            "see_also": ["map_intersect_keys", "map_diff"],
        },
        {
            "name": "map_intersect_keys",
            "purpose": "The entries whose keys another map has",
            "params": [param("map", "map[K, V]", "The map to take entries from."), param("other", "map[K, W]", "The map whose keys to keep; its values can be of any type.")],
            "returns": "map[K, V]",
            "description": "Returns a new map with the entries of <code>map</code> whose key is also in <code>other</code>, in <code>map</code>'s order, with <code>map</code>'s values, like PHP's <code>array_intersect_key</code>. Only keys are compared, so the two maps need the same key type but not the same value type.",
            "return_description": "The entries kept.",
            "errors": ["<code>RuntimeException</code> if a map is <code>null</code>. Maps with different key types are a compile error."],
            "examples": [{"title": "Only the fields allowed", "code": 'map[string, string] form = {"name" => "Ada", "email" => "ada@example.com", "admin" => "1"};\nmap[string, bool] allowed = {"name" => true, "email" => true};\nprint(map_intersect_keys(form, allowed));',
                          "output": '{"name" => "Ada", "email" => "ada@example.com"}'}],
            "see_also": ["map_diff_keys", "map_intersect"],
        },
        {
            "name": "map_diff",
            "purpose": "The entries another map doesn't have",
            "params": [param("map", "map[K, V]", "The map to take entries from."), param("other", "map[K, V]", "A map of the same type.")],
            "returns": "map[K, V]",
            "description": "Returns a new map with the entries of <code>map</code> that <code>other</code> doesn't have: whose key isn't in <code>other</code>, or is with a different value. Like PHP's <code>array_diff_assoc</code>. Values compare as <code>==</code> compares them, as in <a href=\"function.in-map.html\">in_map</a>.",
            "return_description": "The entries that differ.",
            "errors": ["<code>RuntimeException</code> if a map is <code>null</code>. Maps of different types are a compile error."],
            "examples": [{"title": "What changed", "code": 'map[string, int] before = {"tea" => 3, "cake" => 1, "jam" => 2};\nmap[string, int] after = {"tea" => 3, "cake" => 5, "bread" => 1};\nprint(map_diff(after, before));',
                          "output": '{"cake" => 5, "bread" => 1}'}],
            "see_also": ["map_intersect", "map_diff_keys"],
        },
        {
            "name": "map_intersect",
            "purpose": "The entries another map has too",
            "params": [param("map", "map[K, V]", "The map to take entries from."), param("other", "map[K, V]", "A map of the same type.")],
            "returns": "map[K, V]",
            "description": "Returns a new map with the entries of <code>map</code> that <code>other</code> also has, with the same key and an equal value. Like PHP's <code>array_intersect_assoc</code>. Values compare as <code>==</code> compares them.",
            "return_description": "The entries in both.",
            "errors": ["<code>RuntimeException</code> if a map is <code>null</code>. Maps of different types are a compile error."],
            "examples": [{"title": "What stayed the same", "code": 'map[string, int] before = {"tea" => 3, "cake" => 1};\nmap[string, int] after = {"tea" => 3, "cake" => 5};\nprint(map_intersect(before, after));',
                          "output": '{"tea" => 3}'}],
            "see_also": ["map_diff", "map_intersect_keys"],
        },
        {
            "name": "map_flip",
            "purpose": "Swap keys and values",
            "params": [param("map", "map[K, V]", "A map whose values can be keys: ints, doubles, bools, strings, or chars.")],
            "returns": "map[V, K]",
            "description": "Returns a new map whose keys are <code>map</code>'s values and whose values are its keys, like PHP's <code>array_flip</code>. A value that appears more than once becomes one key, with the last key it had.",
            "return_description": "The flipped map.",
            "errors": ["<code>RuntimeException</code> if the map is <code>null</code> or has a null string value. A value type that can't be a key, such as an array, is a compile error."],
            "examples": [{"title": "A reverse lookup", "code": 'map[string, int] statuses = {"OK" => 200, "Not Found" => 404};\nmap[int, string] names = map_flip(statuses);\nprint(names[404], map_flip({"a" => 1, "b" => 1}));',
                          "output": 'Not Found {1 => "b"}'}],
            "see_also": ["map_keys", "map_values"],
        },
        {
            "name": "map_filter",
            "purpose": "The entries a callback accepts",
            "params": [param("map", "map[K, V]", "The map."), param("callback", "function(K, V): bool", "Called with each key and value; returns whether to keep the entry.")],
            "returns": "map[K, V]",
            "description": "Returns a new map of the entries for which <code>callback</code> returns <code>true</code>, in the map's order, like PHP's <code>array_filter</code> with <code>ARRAY_FILTER_USE_BOTH</code>. The callback can be an anonymous function, whose parameters can leave out their types (they come from the map), a named function, or a function value.",
            "return_description": "The entries kept.",
            "errors": ["Whatever <code>callback</code> throws."],
            "examples": [{"title": "In stock", "code": 'map[string, int] stock = {"tea" => 12, "cake" => 0, "jam" => 3};\nprint(map_filter(stock, fn(item, count) => count > 0));',
                          "output": '{"tea" => 12, "jam" => 3}'}],
            "see_also": ["map_map", "map_intersect_keys", "array_filter"],
        },
        {
            "name": "map_map",
            "purpose": "Transform each value, keeping the keys",
            "params": [param("map", "map[K, V]", "The map."), param("callback", "function(V): W", "Called with each value; returns its replacement, of any type.")],
            "returns": "map[K, W]",
            "description": "Returns a new map with the same keys, in the same order, and what <code>callback</code> returns for each value, like PHP's <code>array_map</code> on an associative array. The new values' type is the callback's return type. The callback can be an anonymous function, whose parameters can leave out their types (they come from the map), a named function, or a function value.",
            "return_description": "The map with its new values.",
            "errors": ["Whatever <code>callback</code> throws. A callback that returns nothing is a compile error."],
            "examples": [{"title": "Prices with tax", "code": 'map[string, double] prices = {"tea" => 3.5, "cake" => 12.0};\nmap[string, string] shown = map_map(prices, fn(p) => number_format(p * 1.2, 2));\nprint(shown);',
                          "output": '{"tea" => "4.20", "cake" => "14.40"}'}],
            "see_also": ["map_filter", "array_map"],
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
        {
            "name": "array_filter",
            "purpose": "The elements a callback accepts",
            "params": [param("array", "T[]", "The array."), param("callback", "function(T): bool", "Called with each element; returns whether to keep it.")],
            "returns": "T[]",
            "description": "Returns a new array of the elements for which <code>callback</code> returns <code>true</code>, in order. Unlike PHP's <code>array_filter</code>, the result's indexes are 0, 1, 2, ..., as an array's always are. The callback can be an anonymous function, whose parameters can leave out their types (they come from the array's elements), a named function, or a function value.",
            "return_description": "The elements kept.",
            "errors": ["Whatever <code>callback</code> throws. A callback of the wrong type is a compile error."],
            "examples": [{"title": "Keeping some", "code": 'class Product {\n    function __construct(public string name, public int stock) {}\n}\nProduct[] products = [new Product("Tea", 12), new Product("Cake", 0), new Product("Jam", 3)];\nProduct[] inStock = array_filter(products, fn(p) => p->stock > 0);\nprint(len(inStock), array_filter([5, -2, 8, -7], fn(n) => n > 0));',
                          "output": "2 [5, 8]"}],
            "see_also": ["array_map", "array_find", "map_filter"],
        },
        {
            "name": "array_map",
            "purpose": "Transform each element",
            "params": [param("array", "T[]", "The array."), param("callback", "function(T): U", "Called with each element; returns its replacement, of any type.")],
            "returns": "U[]",
            "description": "Returns a new array of what <code>callback</code> returns for each element, in order. The result's element type is the callback's return type, so it can differ from the array's. The callback can be an anonymous function, whose parameters can leave out their types (they come from the array's elements), a named function, or a function value.",
            "return_description": "The new elements.",
            "errors": ["Whatever <code>callback</code> throws. A callback that returns nothing is a compile error."],
            "examples": [{"title": "Changing the type", "code": 'string[] labels = array_map([1, 2, 3], fn(n) => f"#{n}");\nint[] squares = array_map([1, 2, 3], fn(n) => n * n);\nprint(labels, squares);',
                          "output": '["#1", "#2", "#3"] [1, 4, 9]'}],
            "see_also": ["array_filter", "array_reduce", "map_map"],
        },
        {
            "name": "array_reduce",
            "purpose": "Combine the elements into one value",
            "params": [param("array", "T[]", "The array."), param("callback", "function(U, T): U", "Called with the value so far and each element; returns the new value so far."),
                       param("initial", "U", "The value to start from, which is also the result for an empty array.")],
            "returns": "U",
            "description": "Calls <code>callback</code> with <code>initial</code> and the first element, then with its result and the second element, and so on, and returns the last result. The result's type is <code>initial</code>'s; an empty <code>[]</code> or <code>{}</code> takes the type the result is used as.",
            "return_description": "The combined value.",
            "errors": ["Whatever <code>callback</code> throws. An initial value of <code>null</code>, whose type can't be told, is a compile error."],
            "examples": [{"title": "Totals and counts", "code": 'print(array_reduce([5, -2, 8], fn(total, n) => total + n, 0));\nmap[string, int] counts = array_reduce(["a", "b", "a"], fn(seen, word) => map_merge(seen, {word => (seen[word] ?? 0) + 1}), {});\nprint(counts);',
                          "output": '11\n{"a" => 2, "b" => 1}'}],
            "see_also": ["array_map", "array_filter"],
        },
        {
            "name": "array_find",
            "purpose": "The first element a callback accepts",
            "params": [param("array", "T[]", "An array of strings, arrays, maps, or objects."), param("callback", "function(T): bool", "Called with each element until it returns true.")],
            "returns": "T",
            "description": "Returns the first element for which <code>callback</code> returns <code>true</code>, or <code>null</code> if there is none, like PHP 8.4's <code>array_find</code>. The rest of the elements aren't looked at. Ints, doubles, bools, bytes, and chars can't be <code>null</code>, so for arrays of them, use <a href=\"function.array-find-index.html\">array_find_index</a>.",
            "return_description": "The element, or <code>null</code>.",
            "errors": ["Whatever <code>callback</code> throws. An array of ints or other values that can't be <code>null</code> is a compile error."],
            "examples": [{"title": "The first match", "code": 'string[] words = ["tea", "jam", "cake"];\nprint(array_find(words, fn(w) => len(w) > 3), array_find(words, fn(w) => len(w) > 9) ?? "none");',
                          "output": "cake none"}],
            "see_also": ["array_find_index", "array_any"],
        },
        {
            "name": "array_find_index",
            "purpose": "The index of the first element a callback accepts",
            "params": [param("array", "T[]", "The array."), param("callback", "function(T): bool", "Called with each element until it returns true.")],
            "returns": "int",
            "description": "Returns the index of the first element for which <code>callback</code> returns <code>true</code>, or -1 if there is none, like PHP 8.4's <code>array_find_key</code>. It works for arrays of any type.",
            "return_description": "The index, or -1.",
            "errors": ["Whatever <code>callback</code> throws."],
            "examples": [{"title": "Where it is", "code": 'int[] readings = [3, 8, -1, 4];\nprint(array_find_index(readings, fn(r) => r < 0), array_find_index(readings, fn(r) => r > 100));',
                          "output": "2 -1"}],
            "see_also": ["array_find"],
        },
        {
            "name": "array_any",
            "purpose": "Whether a callback accepts any element",
            "params": [param("array", "T[]", "The array."), param("callback", "function(T): bool", "Called with each element until it returns true.")],
            "returns": "bool",
            "description": "Returns whether <code>callback</code> returns <code>true</code> for at least one element, like PHP 8.4's <code>array_any</code>, stopping at the first. For an empty array, it's <code>false</code>.",
            "return_description": "Whether any element matched.",
            "errors": ["Whatever <code>callback</code> throws."],
            "examples": [{"title": "Any negative?", "code": 'print(array_any([3, -1, 4], fn(n) => n < 0), array_any([3, 4], fn(n) => n < 0));',
                          "output": "true false"}],
            "see_also": ["array_all", "array_find"],
        },
        {
            "name": "array_all",
            "purpose": "Whether a callback accepts every element",
            "params": [param("array", "T[]", "The array."), param("callback", "function(T): bool", "Called with each element until it returns false.")],
            "returns": "bool",
            "description": "Returns whether <code>callback</code> returns <code>true</code> for every element, like PHP 8.4's <code>array_all</code>, stopping at the first that fails. For an empty array, it's <code>true</code>.",
            "return_description": "Whether every element matched.",
            "errors": ["Whatever <code>callback</code> throws."],
            "examples": [{"title": "All valid?", "code": 'string[] emails = ["ada@example.com", "lin@example.com"];\nprint(array_all(emails, fn(e) => str_contains(e, "@")), array_all(["x"], fn(e) => str_contains(e, "@")));',
                          "output": "true false"}],
            "see_also": ["array_any"],
        },
    ],
}

PROGRAM = {
    "slug": "program",
    "title": "Program",
    "intro": "The running program: its command-line arguments, its exit status, and its environment variables, and running other programs with <code>exec</code> and <code>shell</code>. Import them from <code>std/os</code>.",
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
                    "output": "info true",
                }
            ],
            "notes": ["PHP's <code>getenv</code> returns <code>false</code> for a variable that isn't set; qiq returns <code>null</code>, so <code>??</code> gives a default."],
            "see_also": ["setenv", "argv"],
        },
        {
            "name": "setenv",
            "purpose": "Set an environment variable",
            "params": [
                param("name", "string", "The variable's name."),
                param("value", "string", "Its new value."),
            ],
            "returns": "void",
            "description": "Sets the environment variable <code>name</code> to <code>value</code> for the rest of the program, replacing any value it had. Like PHP's <code>putenv(\"NAME=value\")</code>, with the name and value apart. It affects what <code>getenv</code> returns and what the standard library reads from the environment: <code>TZ</code> sets the default time zone for <code>date</code> and <a href=\"class.datetime.html\">DateTime</a> (unless <code>date_default_timezone_set</code> chose one), <code>PGPASSWORD</code> and <code>MYSQL_PWD</code> are read when a <a href=\"class.database.html\">Database</a> opens without a password in its URL, and <code>SSL_CERT_FILE</code> and <code>SSL_CERT_DIR</code> when the first TLS connection is made.",
            "return_description": "Nothing.",
            "errors": ["<code>InvalidArgumentException</code> if <code>name</code> is empty or contains <code>=</code>, and <code>RuntimeException</code> if either argument is <code>null</code> (use <code>unsetenv</code> to remove a variable)."],
            "examples": [
                {
                    "title": "Setting and removing a variable",
                    "code": 'setenv("QIQ_EXAMPLE_MODE", "test");\nprint(getenv("QIQ_EXAMPLE_MODE"));\nunsetenv("QIQ_EXAMPLE_MODE");\nprint(getenv("QIQ_EXAMPLE_MODE") ?? "unset");',
                    "output": "test\nunset",
                }
            ],
            "notes": ["The SSL variables are read once, so set them before the program's first HTTPS request or TLS database connection."],
            "see_also": ["unsetenv", "getenv"],
        },
        {
            "name": "unsetenv",
            "purpose": "Remove an environment variable",
            "params": [param("name", "string", "The variable's name.")],
            "returns": "void",
            "description": "Removes the environment variable <code>name</code>, so <code>getenv</code> returns <code>null</code> for it. Removing one that isn't set does nothing.",
            "return_description": "Nothing.",
            "errors": ["<code>InvalidArgumentException</code> if <code>name</code> is empty or contains <code>=</code>, and <code>RuntimeException</code> if it's <code>null</code>."],
            "examples": [
                {
                    "title": "Basic use",
                    "code": 'setenv("QIQ_EXAMPLE_TOKEN", "secret");\nunsetenv("QIQ_EXAMPLE_TOKEN");\nprint(getenv("QIQ_EXAMPLE_TOKEN") == null);',
                    "output": "true",
                }
            ],
            "see_also": ["setenv", "getenv"],
        },
        {
            "name": "getpid",
            "purpose": "Get the program's process ID",
            "params": [],
            "returns": "int",
            "description": "Returns the process ID of the running program, like PHP's <code>getmypid</code>. Each worker process of the <a href=\"function.http-serve.html\">HTTP server</a> has its own, which makes it useful in log lines; it's also a simple way to make a name unique on the machine, such as a temporary file's.",
            "return_description": "The process ID, a positive int.",
            "errors": [],
            "examples": [
                {
                    "title": "In a file name",
                    "code": 'string path = "/tmp/report-" + to_string(getpid()) + ".txt";\nprint(getpid() > 0, str_starts_with(path, "/tmp/report-"));',
                    "output": "true true",
                }
            ],
            "see_also": ["argv"],
        },
        {
            "name": "exec",
            "purpose": "Run a program and wait for it",
            "params": [
                param("command", "string[]", "The program followed by its arguments, such as <code>[\"git\", \"status\"]</code>. The program is looked up on <code>PATH</code> unless it contains a <code>/</code>."),
                param("input", "string", "Text written to the command's standard input. Without it, the command's standard input is empty.", "null"),
                param("cwd", "string", "The directory the command runs in; the program's own by default.", "null"),
                param("env", "map[string, string]", "Environment variables to add or replace for the command. It also gets the program's other variables.", "null"),
                param("timeout", "int", "Seconds the command may run before it's killed; 0 for no limit.", "0"),
            ],
            "returns": "ProcessResult",
            "description": "Runs <code>command</code>, waits for it to finish, and returns a <a href=\"class.processresult.html\">ProcessResult</a> with its exit status and everything it wrote to standard output and standard error. No shell is involved: each element of <code>command</code> reaches the program as one argument, exactly as given, so spaces, quotes, <code>$</code>, and <code>;</code> need no escaping and can't be used to inject another command. Use <a href=\"function.shell.html\">shell</a> for pipes and redirects.",
            "return_description": "The exit status and output. A command that exits with an error status returns normally; check <code>exitCode</code> or <code>ok()</code>, or call <code>check()</code> to throw instead.",
            "errors": [
                "<code>ProcessException</code> if the command can't be run (the program isn't found or isn't executable, or <code>cwd</code> doesn't exist), or runs longer than <code>timeout</code>.",
                "<code>InvalidUtf8Exception</code> if its output isn't UTF-8 text.",
                "<code>InvalidArgumentException</code> if <code>command</code> is empty, an <code>env</code> name is empty or contains <code>=</code>, or <code>timeout</code> is negative.",
            ],
            "examples": [
                {
                    "title": "Reading a command's output",
                    "code": 'ProcessResult result = exec(["echo", "hello", "world"]);\nprint(result->exitCode, trim(result->stdout));',
                    "output": "0 hello world",
                },
                {
                    "title": "Arguments are passed as they are",
                    "code": 'string name = "it\'s; rm -rf ~";\nprint(exec(["printf", "[%s]", name])->stdout);',
                    "output": "[it's; rm -rf ~]",
                },
                {
                    "title": "Input, a directory, and a variable",
                    "code": 'print(trim(exec(["tr", "a-z", "A-Z"], input: "quiet")->stdout));\nprint(trim(exec(["pwd"], cwd: "/")->stdout));\nprint(trim(exec(["printenv", "GREETING"], env: {"GREETING" => "hi"})->stdout));',
                    "output": "QUIET\n/\nhi",
                },
                {
                    "title": "Failure",
                    "code": 'ProcessResult result = exec(["ls", "/no/such/dir"]);\nprint(result->ok(), result->exitCode > 0);\ntry {\n    exec(["no-such-program"]);\n} catch (ProcessException e) {\n    print(e->getMessage());\n}',
                    "output": "false true\ncommand not found: no-such-program",
                },
            ],
            "notes": [
                "PHP's <code>exec</code>, <code>shell_exec</code>, and <code>system</code> pass a string to the shell, so every value in it needs <code>escapeshellarg</code>. qiq's <code>exec</code> takes the arguments as an array, like Python's <code>subprocess.run</code> and Go's <code>exec.Command</code>.",
                "The command doesn't inherit the program's open files, sockets, or database connections.",
                "When the timeout is reached the command is killed with <code>SIGKILL</code>. A command ended by a signal has exit status 128 plus the signal's number, as in the shell.",
            ],
            "see_also": ["shell", "getenv"],
        },
        {
            "name": "shell",
            "purpose": "Run a command with the system shell",
            "params": [
                param("command", "string", "A shell command line, such as <code>ls *.qiq | wc -l</code>."),
                param("input", "string", "Text written to the command's standard input.", "null"),
                param("cwd", "string", "The directory the command runs in.", "null"),
                param("env", "map[string, string]", "Environment variables to add or replace.", "null"),
                param("timeout", "int", "Seconds before the command is killed; 0 for no limit.", "0"),
            ],
            "returns": "ProcessResult",
            "description": "Runs <code>command</code> with <code>/bin/sh -c</code>, for pipes, redirects, wildcards, <code>&amp;&amp;</code>, and variables, and returns a <a href=\"class.processresult.html\">ProcessResult</a>. It's <code>exec([\"/bin/sh\", \"-c\", command], ...)</code>, with the same settings and errors.",
            "return_description": "The exit status and output of the shell, which is the exit status of the last command it ran.",
            "errors": ["As for <a href=\"function.exec.html\">exec</a>."],
            "examples": [
                {
                    "title": "A pipeline",
                    "code": 'print(trim(shell("printf \'pear\\napple\\nfig\\n\' | sort | head -n 2")->stdout));',
                    "output": "apple\nfig",
                },
                {
                    "title": "Must succeed",
                    "code": 'try {\n    shell("echo starting; echo disk full >&2; exit 1")->check();\n} catch (ProcessException e) {\n    print(e->getMessage(), e->getCode());\n}',
                    "output": "command failed with exit status 1: disk full 1",
                },
            ],
            "notes": ["The shell interprets everything in <code>command</code>, so never build it from untrusted text, such as a request's form fields: a value like <code>x; rm -rf ~</code> would run a second command. Pass such values to <a href=\"function.exec.html\">exec</a> as arguments, or to the shell through <code>env</code> (<code>shell(\"grep -- \\\"$PATTERN\\\" log.txt\", env: {\"PATTERN\" => pattern})</code>)."],
            "see_also": ["exec"],
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

REQUEST_CLASS = '<h1>The Request class</h1>\n<p>A request received by the <a href="ref.http-server.html">HTTP server</a>, passed to every handler, like Go\'s <code>*http.Request</code>.</p>\n<h2>Class synopsis</h2>\n<div class="synopsis"><code><span class="kw">class</span> <span class="cls">Request</span> {<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> method;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> path;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> rawPath;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> rawQuery;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> protocol;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> scheme;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">map[string, string]</span> headers;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">byte[]</span> bytes;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">string</span> remoteAddress;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public</span> <span class="type">map[string, string]</span> params;<br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">text</span>(): <span class="type">string</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">header</span>(<span class="type">string</span> name): <span class="type">string</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">query</span>(<span class="type">string</span> name): <span class="type">string</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">form</span>(<span class="type">string</span> name): <span class="type">string</span><br>\n&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">pathValue</span>(<span class="type">string</span> name): <span class="type">string</span><br>\n}</code></div>\n<table>\n<thead><tr><th>Member</th><th>Description</th></tr></thead>\n<tbody>\n<tr><td><code>method</code></td><td>The method in upper case: <code>GET</code>, <code>POST</code>, ...</td></tr>\n<tr><td><code>path</code></td><td>The URL path, percent-decoded: <code>/files/a b.txt</code></td></tr>\n<tr><td><code>rawPath</code></td><td>The path as the client sent it: <code>/files/a%20b.txt</code></td></tr>\n<tr><td><code>rawQuery</code></td><td>What follows <code>?</code> in the URL, or an empty string</td></tr>\n<tr><td><code>protocol</code></td><td><code>HTTP/1.1</code> or <code>HTTP/1.0</code></td></tr>\n<tr><td><code>scheme</code></td><td><code>http</code>, or <code>https</code> when a trusted proxy reports that the client used HTTPS (see <code>trustProxy</code> in <a href="function.http-serve.html">http_serve</a>)</td></tr>\n<tr><td><code>headers</code></td><td>The request headers, with lowercase names; a header sent more than once has its values joined with <code>, </code></td></tr>\n<tr><td><code>bytes</code></td><td>The body, empty when there is none</td></tr>\n<tr><td><code>remoteAddress</code></td><td>The client\'s address and port, such as <code>127.0.0.1:51234</code> or <code>[::1]:51234</code>; <code>unix</code> on a Unix socket. With <code>trustProxy</code>, the client address the proxy forwarded, such as <code>203.0.113.9</code></td></tr>\n<tr><td><code>params</code></td><td>The values of the wildcards in the <a href="class.router.html">Router</a> pattern that matched</td></tr>\n<tr><td><code>text()</code></td><td>The body as a string; throws <code>InvalidUtf8Exception</code> if it isn\'t UTF-8</td></tr>\n<tr><td><code>header(name)</code></td><td>One header, by name in any case, or <code>null</code></td></tr>\n<tr><td><code>query(name)</code></td><td>A field of the query string (<code>?page=2</code>), decoded, or <code>null</code>; the first one if it\'s repeated</td></tr>\n<tr><td><code>form(name)</code></td><td>A field of a form posted as <code>application/x-www-form-urlencoded</code>, or else of the query string, or <code>null</code></td></tr>\n<tr><td><code>pathValue(name)</code></td><td>A wildcard\'s value, such as <code>id</code> in <code>/items/{id}</code>, or <code>null</code></td></tr>\n<tr><td><code>url()</code></td><td>The full <a href="class.url.html">Url</a> the client asked for, from the scheme, the <code>Host</code> header, the path, and the query, such as <code>https://shop.example.com/items/7?page=2</code></td></tr>\n</tbody>\n</table>\n<p>For a JSON body, decode the text into the type you expect: <code>Item item = json_decode(r-&gt;text());</code>.</p>\n<h2>Example</h2>\n<pre class="code">router-&gt;handle("POST /login", function(Request r, ResponseWriter w): void {\n    string user = r-&gt;form("user");\n    if (user == null) {\n        w-&gt;error(400, "missing user");\n        return;\n    }\n    w-&gt;write("Welcome, " + user + " (from " + r-&gt;remoteAddress + ")\\n");\n});</pre>\n'

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
                    "output": "60 $2y$12$\ntrue false",
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
                    "output": "true false false",
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

REGEX = {
    "slug": "regex",
    "title": "Regular expressions",
    "intro": (
        "Pattern matching with PHP's <code>preg_*</code> functions, from <code>std/regex</code>. Matching is done by PCRE2, the library PHP uses, "
        "so a pattern that works in PHP works the same here. The results are returned rather than written into a <code>$matches</code> argument: "
        "<code>preg_match</code> returns a <a href=\"class.regexmatch.html\">RegexMatch</a>, or <code>null</code>."
    ),
    "details": r"""<h2>Writing patterns</h2>
<p>A pattern is written as in PHP: between delimiters, usually <code>/</code>, followed by any flags. Write patterns as <a href="strings.html">raw strings</a>, between backticks, so their backslashes need no escaping: <code>`/\d+\.\d+/`</code> rather than <code>"/\\d+\\.\\d+/"</code>.</p>
<p>The delimiter can be any ASCII punctuation character. <code>#</code> saves escaping slashes in paths and URLs (<code>`#^/users/(\d+)$#`</code>), and the bracket pairs <code>()</code>, <code>[]</code>, <code>{}</code>, and <code>&lt;&gt;</code> work too.</p>
<table>
<thead><tr><th>Flag</th><th>Effect</th></tr></thead>
<tbody>
<tr><td><code>i</code></td><td>Case-insensitive</td></tr>
<tr><td><code>m</code></td><td><code>^</code> and <code>$</code> match at the start and end of each line</td></tr>
<tr><td><code>s</code></td><td><code>.</code> also matches a newline</td></tr>
<tr><td><code>x</code></td><td>Whitespace in the pattern is ignored, and <code>#</code> starts a comment</td></tr>
<tr><td><code>U</code></td><td>Quantifiers are lazy by default, and <code>?</code> makes them greedy</td></tr>
<tr><td><code>D</code></td><td><code>$</code> matches only at the very end, not before a final newline</td></tr>
<tr><td><code>A</code></td><td>The match must start where the search starts</td></tr>
<tr><td><code>n</code></td><td>Plain <code>(...)</code> doesn't capture; only named groups do</td></tr>
<tr><td><code>J</code></td><td>Several groups may have the same name</td></tr>
<tr><td><code>u</code></td><td>Accepted for PHP compatibility; it's always on</td></tr>
</tbody>
</table>
<p>The pattern syntax is PCRE2's, which is Perl's: classes such as <code>\d</code>, <code>\w</code>, <code>\s</code>, and <code>\b</code>, lazy quantifiers such as <code>*?</code>, groups <code>(...)</code>, named groups <code>(?&lt;name&gt;...)</code>, non-capturing groups <code>(?:...)</code>, lookahead and lookbehind (<code>(?=...)</code>, <code>(?&lt;=...)</code>), backreferences (<code>\1</code>), and Unicode properties such as <code>\p{L}</code> for any letter.</p>
<h2>Differences from PHP</h2>
<ul>
<li>Every pattern works on characters, as with PHP's <code>u</code> flag: <code>.</code> matches one character, and <code>\w</code>, <code>\d</code>, and <code>\b</code> know letters and digits in every script.</li>
<li>Positions count characters, as everywhere in qiq, rather than bytes, so they work with <code>substr</code>.</li>
<li>A pattern written as a literal is checked when the program compiles: <code>preg_match(`/(\d+/`, text)</code> is a compile error (<code>invalid regular expression /(\d+/: missing closing parenthesis at offset 4</code>), even on a line that rarely runs, and so is a <code>preg_replace</code> replacement that refers to a group the pattern doesn't have. In VS Code, the qiq extension underlines it as you type.</li>
<li>A malformed pattern built while the program runs throws <code>RegexException</code> instead of returning <code>false</code> with a warning. So does a pattern that needs too much backtracking on its text, such as <code>`/(a+)+$/`</code> on a long run of <code>a</code>s, where PHP returns <code>false</code> and sets <code>preg_last_error()</code>.</li>
<li>A group that took no part in a match is <code>null</code>, not an empty string.</li>
<li><code>preg_match</code> and <code>preg_match_all</code> return matches; <code>preg_test</code> answers whether there is one.</li>
</ul>
<p>Compiled patterns are cached, as in PHP, so a pattern used in a loop is compiled only once. As in PHP, each is also compiled to machine code by PCRE2's JIT, which makes matching several times faster; on a system that forbids memory that is both written and run, patterns are matched by PCRE2's interpreter instead, with the same results.</p>""",
    "functions": [
        {
            "name": "preg_match",
            "purpose": "Find the first match of a pattern",
            "params": [
                param("pattern", "string", "The pattern, between delimiters: <code>`/\\d+/`</code>."),
                param("subject", "string", "The text to search."),
                param("offset", "int", "The character to start searching from.", "0"),
            ],
            "returns": "RegexMatch",
            "description": "Searches <code>subject</code> for <code>pattern</code> and returns the first match as a <a href=\"class.regexmatch.html\">RegexMatch</a>, with its text, position, and groups. Like PHP's <code>preg_match($pattern, $subject, $matches)</code>, but the match is returned rather than written into <code>$matches</code>.",
            "return_description": "The first match, or <code>null</code> if there is none.",
            "errors": [
                "<code>RegexException</code> if the pattern is malformed or needs too much backtracking.",
                "<code>InvalidArgumentException</code> if <code>offset</code> is negative or past the end of <code>subject</code>.",
            ],
            "examples": [
                {
                    "title": "Groups",
                    "code": r"""RegexMatch m = preg_match(`/(\d{4})-(\d{2})-(\d{2})/`, "Released on 2026-11-23.");
if (m != null) {
    print(m->text, m->group(1), m->group(2), m->start);
}""",
                    "output": "2026-11-23 2026 11 12",
                },
                {
                    "title": "Named groups",
                    "code": r"""RegexMatch m = preg_match(`/(?<user>[\w.]+)@(?<host>[\w.]+)/`, "mail ada@example.com");
print(m->named("user"), m->named("host"));
print(m->namedGroups());""",
                    "output": "ada example.com\n{\"user\" => \"ada\", \"host\" => \"example.com\"}",
                },
                {
                    "title": "No match",
                    "code": r"""string version = preg_match(`/v(\d+)/`, "no version here")?->group(1) ?? "unknown";
print(version);""",
                    "output": "unknown",
                },
            ],
            "see_also": ["preg_match_all", "preg_test"],
        },
        {
            "name": "preg_match_all",
            "purpose": "Find every match of a pattern",
            "params": [
                param("pattern", "string", "The pattern."),
                param("subject", "string", "The text to search."),
                param("offset", "int", "The character to start searching from.", "0"),
            ],
            "returns": "RegexMatch[]",
            "description": "Returns every match of <code>pattern</code> in <code>subject</code>, in order, as <a href=\"class.regexmatch.html\">RegexMatch</a> objects. Matches don't overlap: each search starts where the previous match ended. This is PHP's <code>preg_match_all</code> with <code>PREG_SET_ORDER</code>: one object per match.",
            "return_description": "The matches; an empty array if there are none.",
            "errors": ["<code>RegexException</code> if the pattern is malformed or needs too much backtracking."],
            "examples": [
                {
                    "title": "Every match",
                    "code": r"""foreach (preg_match_all(`/#(\w+)/`, "Tagged #qiq and #regex.") as tag) {
    print(tag->group(1), tag->start);
}""",
                    "output": "qiq 7\nregex 16",
                },
                {
                    "title": "Counting",
                    "code": r"""print(len(preg_match_all(`/\b\w+\b/`, "one two three")));""",
                    "output": "3",
                },
            ],
            "see_also": ["preg_match"],
        },
        {
            "name": "preg_test",
            "purpose": "Check whether a pattern matches",
            "params": [param("pattern", "string", "The pattern."), param("subject", "string", "The text to search.")],
            "returns": "bool",
            "description": "Returns whether <code>pattern</code> matches anywhere in <code>subject</code>. Use it where PHP code writes <code>if (preg_match(...))</code>.",
            "return_description": "<code>true</code> if there is a match.",
            "errors": ["<code>RegexException</code> if the pattern is malformed or needs too much backtracking."],
            "examples": [
                {
                    "title": "Validating input",
                    "code": r"""foreach (["ada_99", "no spaces!", "ok"] as name) {
    print(name, preg_test(`/^\w{3,16}$/`, name));
}""",
                    "output": "ada_99 true\nno spaces! false\nok false",
                }
            ],
            "see_also": ["preg_match"],
        },
        {
            "name": "preg_replace",
            "purpose": "Replace matches of a pattern",
            "params": [
                param("pattern", "string", "The pattern."),
                param("replacement", "string", "The text for each match, in which <code>$1</code> stands for the first group."),
                param("subject", "string", "The text to change."),
                param("limit", "int", "The most matches to replace; -1 for all.", "-1"),
            ],
            "returns": "string",
            "description": "Returns <code>subject</code> with each match of <code>pattern</code> replaced by <code>replacement</code>. In the replacement, <code>$0</code> stands for the whole match, <code>$1</code> to <code>$99</code> for a group (<code>\\1</code> and <code>${1}</code> work too), <code>${name}</code> for a named group, and <code>$$</code> for a dollar sign. A group that took no part in the match gives an empty string.",
            "return_description": "The new string.",
            "errors": ["<code>RegexException</code> if the pattern is malformed or needs too much backtracking, or the replacement refers to a group the pattern doesn't have."],
            "examples": [
                {
                    "title": "Collapsing whitespace",
                    "code": r"""print(preg_replace(`/\s+/`, " ", "too    many\n\nspaces"));""",
                    "output": "too many spaces",
                },
                {
                    "title": "Rearranging with groups",
                    "code": r"""print(preg_replace(`/(\d{4})-(\d{2})-(\d{2})/`, "$3/$2/$1", "Due 2026-11-23."));
print(preg_replace(`/(?<first>\w+) (?<last>\w+)/`, "${last}, ${first}", "Ada Lovelace"));""",
                    "output": "Due 23/11/2026.\nLovelace, Ada",
                },
                {
                    "title": "Only the first",
                    "code": r"""print(preg_replace(`/o/`, "0", "foo boo", 1));""",
                    "output": "f0o boo",
                },
            ],
            "notes": ["PHP's <code>preg_replace</code> also accepts arrays of patterns and replacements. In qiq, call it once for each."],
            "see_also": ["preg_replace_callback", "str_replace"],
        },
        {
            "name": "preg_replace_callback",
            "purpose": "Replace matches with what a function returns",
            "params": [
                param("pattern", "string", "The pattern."),
                param("callback", "function(RegexMatch): string", "Called with each match; returns its replacement."),
                param("subject", "string", "The text to change."),
                param("limit", "int", "The most matches to replace; -1 for all.", "-1"),
            ],
            "returns": "string",
            "description": "Returns <code>subject</code> with each match of <code>pattern</code> replaced by what <code>callback</code> returns for it, for replacements that need computing.",
            "return_description": "The new string.",
            "errors": ["<code>RegexException</code> if the pattern is malformed or needs too much backtracking, and whatever <code>callback</code> throws."],
            "examples": [
                {
                    "title": "Computed replacements",
                    "code": r"""print(preg_replace_callback(`/\d+/`, fn(RegexMatch m) => to_string(to_int(m->text) * 2), "3 apples and 10 pears"));
print(preg_replace_callback(`/\b\w/`, fn(RegexMatch m) => strtoupper(m->text), "the quick brown fox"));""",
                    "output": "6 apples and 20 pears\nThe Quick Brown Fox",
                }
            ],
            "see_also": ["preg_replace"],
        },
        {
            "name": "preg_split",
            "purpose": "Split a string at matches of a pattern",
            "params": [
                param("pattern", "string", "The pattern that separates the pieces."),
                param("subject", "string", "The text to split."),
                param("limit", "int", "Above 0, the most pieces to return, the last holding the rest; -1 for no limit.", "-1"),
                param("skipEmpty", "bool", "Leave out empty pieces, as PHP's <code>PREG_SPLIT_NO_EMPTY</code>.", "false"),
            ],
            "returns": "string[]",
            "description": "Splits <code>subject</code> at each match of <code>pattern</code> and returns the pieces between them. Like <code>explode</code>, but the separator is a pattern.",
            "return_description": "The pieces.",
            "errors": ["<code>RegexException</code> if the pattern is malformed or needs too much backtracking."],
            "examples": [
                {
                    "title": "Several separators",
                    "code": r"""print(preg_split(`/[\s,;]+/`, "red, green;blue  yellow"));
print(preg_split(`/,/`, "a,,b,", skipEmpty: true));
print(preg_split(`/=/`, "key=value=more", 2));""",
                    "output": "[\"red\", \"green\", \"blue\", \"yellow\"]\n[\"a\", \"b\"]\n[\"key\", \"value=more\"]",
                }
            ],
            "see_also": ["explode"],
        },
        {
            "name": "preg_quote",
            "purpose": "Escape text for use in a pattern",
            "params": [
                param("text", "string", "The text to match literally."),
                param("delimiter", "string", "The pattern's delimiter, if it isn't one of the characters escaped anyway.", "null"),
            ],
            "returns": "string",
            "description": "Returns <code>text</code> with a backslash before each character that has a meaning in a pattern (<code>. \\ + * ? [ ^ ] $ ( ) { } = ! &lt; &gt; | : - #</code>) and before <code>delimiter</code>, so text from a user or a file can go inside a pattern and match only itself.",
            "return_description": "The escaped text.",
            "errors": [],
            "examples": [
                {
                    "title": "Searching for user input",
                    "code": r"""string search = "$9.99 (sale)";
print(preg_quote(search));
print(preg_test("/" + preg_quote(search, "/") + "/i", "Now $9.99 (SALE)!"));""",
                    "output": "\\$9\\.99 \\(sale\\)\ntrue",
                }
            ],
            "see_also": ["preg_match"],
        },
    ],
}

REGEXMATCH_CLASS = r"""<h1>The RegexMatch class</h1>
<p>A match of a regular expression, from <a href="function.preg-match.html">preg_match</a>, <a href="function.preg-match-all.html">preg_match_all</a>, or <a href="function.preg-replace-callback.html">preg_replace_callback</a>: the text it matched, where, and its groups. Import it from <code>std/regex</code> to name the type.</p>
<h2>Class synopsis</h2>
<div class="synopsis"><code><span class="kw">class</span> <span class="cls">RegexMatch</span> {<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public readonly</span> <span class="type">string</span> text;<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public readonly</span> <span class="type">int</span> start;<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public readonly</span> <span class="type">int</span> end;<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public readonly</span> <span class="type">string[]</span> groups;<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">group</span>(<span class="type">int</span> number): <span class="type">string</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">named</span>(<span class="type">string</span> name): <span class="type">string</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">namedGroups</span>(): <span class="type">map[string, string]</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">groupStart</span>(<span class="type">int</span> number): <span class="type">int</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">groupEnd</span>(<span class="type">int</span> number): <span class="type">int</span><br>
}</code></div>
<table>
<thead><tr><th>Member</th><th>Description</th></tr></thead>
<tbody>
<tr><td><code>text</code></td><td>The whole match</td></tr>
<tr><td><code>start</code>, <code>end</code></td><td>The position of its first character, and of the character after its last, so <code>end - start</code> is its length and <code>substr(subject, start, end - start)</code> is <code>text</code></td></tr>
<tr><td><code>groups</code></td><td>The whole match, then each group in the order its opening parenthesis appears; <code>null</code> for a group that took no part</td></tr>
<tr><td><code>group(number)</code></td><td>One group: 0 for the whole match, 1 for the first group. Throws <code>OutOfRangeException</code> if the pattern has no such group.</td></tr>
<tr><td><code>named(name)</code></td><td>A group written <code>(?&lt;name&gt;...)</code>. Throws <code>InvalidArgumentException</code> if the pattern has no group with that name.</td></tr>
<tr><td><code>namedGroups()</code></td><td>Every named group, by name</td></tr>
<tr><td><code>groupStart(number)</code>, <code>groupEnd(number)</code></td><td>Where a group starts and ends, or -1 if it took no part</td></tr>
</tbody>
</table>
<p>Positions count characters, not bytes, as everywhere in qiq.</p>
<h2>Example</h2>
<pre class="code">RegexMatch m = preg_match(`/(\w+)=(\d+)?/`, "width= height=40");
print(m-&gt;text, m-&gt;group(1), m-&gt;group(2) ?? "none", m-&gt;start, m-&gt;end);   // width= width none 0 6</pre>
"""

def _simple(name, purpose, params, returns, description, code, output, see_also, errors=None, notes=None):
    entry = {"name": name, "purpose": purpose, "params": params, "returns": returns, "description": description,
             "return_description": "", "errors": errors or [], "examples": [{"title": "Example", "code": code, "output": output}],
             "see_also": see_also}
    if notes: entry["notes"] = notes
    return entry

_X = [param("x", "double", "The value.")]
_GENERIC = "It works on ints and on doubles and returns the same type: the compiler handles it for the types it's given."

MATH = {
    "slug": "math",
    "title": "Math",
    "intro": (
        "Rounding, roots, logarithms, trigonometry, number formatting, and secure random numbers, from <code>std/math</code>, "
        "with PHP's names and results. <code>abs</code>, <code>min</code>, <code>max</code>, and <code>clamp</code> work on ints and doubles alike; "
        "the rest take doubles. Use the <code>**</code> operator for powers, and <code>/</code> and <code>%</code> for integer division and remainders."
    ),
    "details": r"""<h2>Ints and doubles</h2>
<p>qiq doesn't mix ints and doubles in arithmetic, and the same goes for these functions: <code>sqrt(16)</code> is a compile error, and <code>sqrt(16.0)</code> or <code>sqrt(to_double(n))</code> is what to write. <code>floor</code>, <code>ceil</code>, and <code>round</code> return doubles, as in PHP; <code>to_int(round(x))</code> makes an int of the result. <code>number_format</code> takes either.</p>
<h2 id="rounding-as-in-php">Rounding as in PHP</h2>
<p><code>round</code> rounds halves away from zero, so <code>round(2.5)</code> is 3 and <code>round(-2.5)</code> is -3. It computes exactly as PHP 8.3 does: when the requested place is within the 15 significant digits a double holds reliably, the value is first rounded to those 15 digits. So <code>round(1.005, 2)</code> is 1.01, as written, although the double nearest 1.005 is slightly below it, and a result of arithmetic such as <code>2.8499999999999996</code>, which is 2.85 with a little error, rounds as 2.85 does. Checked against PHP 8.3 on 285,000 values, the results are the same, with one exception PHP itself fixed in 8.4: values just below a half, such as <code>0.49999999999999994</code>, round down, where PHP 8.3 rounds them up.</p>""",
    "functions": [
        {
            "name": "abs", "purpose": "The absolute value", "params": [param("x", "int|double", "The value.")], "returns": "int|double",
            "description": "Returns <code>x</code> without its sign. " + _GENERIC,
            "return_description": "The value, never negative (except for the smallest int, which has no positive counterpart and stays as it is, as <code>-</code> wraps).",
            "errors": [],
            "examples": [{"title": "Ints and doubles", "code": "print(abs(-7), abs(2.5), abs(-0.25));", "output": "7 2.5 0.25"}],
            "see_also": ["min", "max"],
        },
        {
            "name": "min", "purpose": "The smallest of several values",
            "params": [param("values", "int...|double...|int[]|double[]", "Two or more ints, or two or more doubles, or one array of them.")],
            "returns": "int|double",
            "description": "Returns the smallest of its arguments, or of the elements of an array. " + _GENERIC,
            "return_description": "The smallest value.",
            "errors": ["<code>InvalidArgumentException</code> for an empty array."],
            "examples": [{"title": "Values and arrays", "code": "int[] scores = [72, 95, 64];\nprint(min(3, 1, 2), min(2.5, -1.5), min(scores));", "output": "1 -1.5 64"}],
            "see_also": ["max", "clamp"],
        },
        {
            "name": "max", "purpose": "The largest of several values",
            "params": [param("values", "int...|double...|int[]|double[]", "Two or more ints, or two or more doubles, or one array of them.")],
            "returns": "int|double",
            "description": "Returns the largest of its arguments, or of the elements of an array. " + _GENERIC,
            "return_description": "The largest value.",
            "errors": ["<code>InvalidArgumentException</code> for an empty array."],
            "examples": [{"title": "Values and arrays", "code": "double[] prices = [4.5, 12.0, 7.25];\nprint(max(3, 1, 2), max(prices));", "output": "3 12"}],
            "see_also": ["min", "clamp"],
        },
        {
            "name": "clamp", "purpose": "Limit a value to a range",
            "params": [param("x", "int|double", "The value."), param("low", "int|double", "The smallest result."), param("high", "int|double", "The largest result.")],
            "returns": "int|double",
            "description": "Returns <code>x</code>, or <code>low</code> if <code>x</code> is below it, or <code>high</code> if it's above: <code>max(low, min(x, high))</code> in one call. " + _GENERIC + " PHP has no <code>clamp</code>.",
            "return_description": "A value from <code>low</code> to <code>high</code>.",
            "errors": ["<code>InvalidArgumentException</code> if <code>low</code> is greater than <code>high</code>."],
            "examples": [{"title": "Keeping a value in range", "code": "print(clamp(15, 0, 10), clamp(-3, 0, 10), clamp(0.5, 0.0, 1.0));", "output": "10 0 0.5"}],
            "see_also": ["min", "max"],
        },
        _simple("floor", "Round down", _X, "double", "Returns the largest whole number not above <code>x</code>, as a double.", "print(floor(2.7), floor(-2.7), to_int(floor(9.99)));", "2 -3 9", ["ceil", "round"]),
        _simple("ceil", "Round up", _X, "double", "Returns the smallest whole number not below <code>x</code>, as a double.", "int items = 23;\nint perPage = 10;\nprint(ceil(2.1), ceil(-2.1), to_int(ceil(to_double(items) / to_double(perPage))));", "3 -2 3", ["floor", "round"]),
        {
            "name": "round", "purpose": "Round to a number of decimal places",
            "params": [param("x", "double", "The value."), param("precision", "int", "Decimal places; negative rounds to tens, hundreds, and so on.", "0")],
            "returns": "double",
            "description": "Returns <code>x</code> rounded to <code>precision</code> decimal places, with halves rounded away from zero, as PHP's <code>round</code> (see <a href=\"#rounding-as-in-php\">above</a>).",
            "return_description": "The rounded value, as a double.",
            "errors": [],
            "examples": [{"title": "Places and halves", "code": "print(round(2.5), round(-2.5), round(3.14159, 2), round(1.005, 2), round(1234.5, -2));", "output": "3 -3 3.14 1.01 1200"}],
            "notes": ["To print a rounded value with a fixed number of decimals, such as <code>3.10</code>, use <a href=\"function.number-format.html\">number_format</a> or an f-string spec like <code>{x:.2f}</code>."],
            "see_also": ["floor", "ceil", "number_format"],
        },
        {
            "name": "number_format", "purpose": "Format a number with grouped thousands",
            "params": [param("x", "int|double", "The number."), param("decimals", "int", "Decimal places; negative rounds before the point.", "0"),
                       param("point", "string", "The decimal point.", "\".\""), param("thousands", "string", "Put between each group of three digits.", "\",\"")],
            "returns": "string",
            "description": "Returns <code>x</code> as text, rounded to <code>decimals</code> places as <code>round</code> rounds, with <code>thousands</code> between each group of three digits, as PHP's <code>number_format</code>. An int is written exactly, however large. A value that rounds to zero has no minus sign.",
            "return_description": "The formatted number.",
            "errors": [],
            "examples": [{"title": "Prices and counts", "code": "print(number_format(1234567.891, 2));\nprint(number_format(1234567.891, 2, \",\", \".\"));\nprint(number_format(9876543), number_format(0.5), number_format(-0.4));", "output": "1,234,567.89\n1.234.567,89\n9,876,543 1 0"}],
            "see_also": ["round", "sprintf"],
        },
        _simple("sqrt", "Square root", _X, "double", "Returns the square root of <code>x</code>, or <code>NAN</code> for a negative number.", "print(sqrt(16.0), sqrt(2.0), is_nan(sqrt(-1.0)));", "4 1.41421 true", ["hypot"]),
        _simple("exp", "e to a power", _X, "double", "Returns <i>e</i> raised to the power <code>x</code>.", "print(exp(0.0), exp(1.0));", "1 2.71828", ["log"]),
        {
            "name": "log", "purpose": "Logarithm",
            "params": [param("x", "double", "The value."), param("base", "double", "The base; <i>e</i> (a natural logarithm) by default.", "e")],
            "returns": "double",
            "description": "Returns the logarithm of <code>x</code> to <code>base</code>. For bases 10 and 2, <code>log10</code> and <code>log2</code> are exact for powers of the base.",
            "return_description": "The logarithm; <code>-INF</code> for 0 and <code>NAN</code> for a negative number.",
            "errors": ["<code>InvalidArgumentException</code> if <code>base</code> is 0 or less, or 1."],
            "examples": [{"title": "Bases", "code": "print(log(exp(2.0)), log(8.0, 2.0), log10(1000.0), log2(1024.0));", "output": "2 3 3 10"}],
            "see_also": ["exp", "log10", "log2"],
        },
        _simple("log10", "Base-10 logarithm", _X, "double", "Returns the base-10 logarithm of <code>x</code>.", "print(log10(1000.0), log10(0.01));", "3 -2", ["log"]),
        _simple("log2", "Base-2 logarithm", _X, "double", "Returns the base-2 logarithm of <code>x</code>.", "print(log2(8.0), log2(1.0));", "3 0", ["log"]),
        _simple("hypot", "Length of a hypotenuse", [param("x", "double", "One side."), param("y", "double", "The other side.")], "double",
                "Returns <code>sqrt(x * x + y * y)</code>, without overflowing for large values: the distance of the point (x, y) from the origin.", "print(hypot(3.0, 4.0));", "5", ["sqrt"]),
        _simple("sin", "Sine", [param("x", "double", "An angle in radians.")], "double", "Returns the sine of <code>x</code>. <code>cos</code> and <code>tan</code> work the same way.", "print(sin(pi() / 2.0), cos(0.0), tan(pi() / 4.0));", "1 1 1", ["cos", "tan", "deg2rad"]),
        _simple("cos", "Cosine", [param("x", "double", "An angle in radians.")], "double", "Returns the cosine of <code>x</code>.", "print(cos(pi()));", "-1", ["sin", "tan"]),
        _simple("tan", "Tangent", [param("x", "double", "An angle in radians.")], "double", "Returns the tangent of <code>x</code>.", "print(tan(0.0));", "0", ["sin", "cos"]),
        _simple("asin", "Arc sine", _X, "double", "Returns the angle, in radians from -pi/2 to pi/2, whose sine is <code>x</code>; <code>NAN</code> outside -1 to 1.", "print(asin(1.0), rad2deg(asin(0.5)));", "1.5708 30", ["acos", "atan"]),
        _simple("acos", "Arc cosine", _X, "double", "Returns the angle, in radians from 0 to pi, whose cosine is <code>x</code>; <code>NAN</code> outside -1 to 1.", "print(acos(1.0), acos(-1.0));", "0 3.14159", ["asin", "atan"]),
        _simple("atan", "Arc tangent", _X, "double", "Returns the angle, in radians from -pi/2 to pi/2, whose tangent is <code>x</code>.", "print(atan(1.0) * 4.0);", "3.14159", ["atan2"]),
        _simple("atan2", "Angle of a point", [param("y", "double", "The point's y."), param("x", "double", "The point's x.")], "double",
                "Returns the angle of the point (x, y) from the positive x axis, in radians from -pi to pi, using the signs of both to find the quadrant.", "print(rad2deg(atan2(1.0, 1.0)), rad2deg(atan2(1.0, -1.0)));", "45 135", ["atan"]),
        _simple("deg2rad", "Degrees to radians", [param("degrees", "double", "An angle in degrees.")], "double", "Returns the angle in radians.", "print(deg2rad(180.0));", "3.14159", ["rad2deg"]),
        _simple("rad2deg", "Radians to degrees", [param("radians", "double", "An angle in radians.")], "double", "Returns the angle in degrees.", "print(rad2deg(pi() / 2.0));", "90", ["deg2rad"]),
        _simple("pi", "The number pi", [], "double", "Returns pi, 3.141592653589793, like PHP's <code>pi()</code> and <code>M_PI</code>.", "print(pi(), number_format(pi(), 5));", "3.14159 3.14159", ["deg2rad"]),
        _simple("is_nan", "Check for not-a-number", _X, "bool", "Returns whether <code>x</code> is <code>NAN</code>, the result of operations such as <code>0.0 / 0.0</code> or <code>sqrt(-1.0)</code>. <code>NAN</code> isn't equal to anything, itself included, so <code>x == x</code> can't tell.", "double nan = 0.0 / 0.0;\nprint(is_nan(nan), nan == nan, is_nan(1.0));", "true false false", ["is_finite", "is_infinite"]),
        _simple("is_finite", "Check for an ordinary number", _X, "bool", "Returns whether <code>x</code> is neither infinite nor <code>NAN</code>.", "print(is_finite(1.5), is_finite(1.0 / 0.0));", "true false", ["is_nan", "is_infinite"]),
        _simple("is_infinite", "Check for infinity", _X, "bool", "Returns whether <code>x</code> is <code>INF</code> or <code>-INF</code>, the result of dividing a nonzero double by zero, for example.", "print(is_infinite(-1.0 / 0.0), is_infinite(1e308));", "true false", ["is_finite"]),
        {
            "name": "random_int", "purpose": "A secure random int in a range",
            "params": [param("min", "int", "The smallest result."), param("max", "int", "The largest result.")],
            "returns": "int",
            "description": "Returns a random int from <code>min</code> to <code>max</code>, both included, each equally likely, from the operating system's secure random source, as PHP's <code>random_int</code>. It's suitable for tokens, codes, and passwords.",
            "return_description": "The random int.",
            "errors": ["<code>InvalidArgumentException</code> if <code>min</code> is greater than <code>max</code>."],
            "examples": [{"title": "Dice", "code": "int roll = random_int(1, 6);\nprint(roll >= 1 && roll <= 6);", "output": "true"}],
            "notes": ["Each call asks the kernel; nothing is kept between calls, so the processes of the <a href=\"function.http-serve.html\">HTTP server</a> never share numbers."],
            "see_also": ["random_double"],
        },
        {
            "name": "random_double", "purpose": "A secure random double from 0 to 1",
            "params": [], "returns": "double",
            "description": "Returns a random double from 0 up to, but not including, 1, from the operating system's secure random source, with every multiple of 2<sup>-53</sup> in that range equally likely.",
            "return_description": "The random double.",
            "errors": [],
            "examples": [{"title": "A probability", "code": "bool heads = random_double() < 0.5;\nprint(heads || !heads);", "output": "true"}],
            "see_also": ["random_int"],
        },
    ],
}

_DELIMITER = param("delimiter", "string", "The character between fields: <code>\",\"</code>, <code>\";\"</code> for files from Excel in many European locales, or <code>\"\\t\"</code> for tab-separated values.", "\",\"")
_LINE_ENDING = param("lineEnding", "string", "What ends each row: <code>\"\\n\"</code> or <code>\"\\r\\n\"</code>.", "\"\\n\"")
_CSV_ERROR = "<code>CsvException</code>, naming the line, for a quoted field that's never closed or a field that isn't UTF-8 text; <code>InvalidArgumentException</code> for a delimiter that isn't one character, or is a quote or a line break."

CSV = {
    "slug": "csv",
    "title": "CSV",
    "intro": (
        "Reading and writing CSV, the comma-separated values that spreadsheets and databases exchange, from <code>std/csv</code>: "
        "as rows of strings, by column name, as objects of a class, or a record at a time for files too large to read at once."
    ),
    "details": r"""<h2>The format</h2>
<p>qiq reads and writes CSV as RFC 4180 defines it, the format Excel and every CSV library use. A field can be quoted, and must be when it holds the delimiter, a quote, or a line break; inside quotes, <code>""</code> is a quote. When reading:</p>
<ul>
<li>Lines can end with <code>\n</code> or <code>\r\n</code>, and the last line needs no line break.</li>
<li>Blank lines are skipped.</li>
<li>A UTF-8 byte order mark at the start of the text, which Excel writes, is ignored.</li>
<li>Text after a field's closing quote is kept, so a slightly malformed file still reads, but a quoted field that's never closed throws <code>CsvException</code>.</li>
</ul>
<p>When writing, a field is quoted only when it must be, and rows end with <code>\n</code> unless <code>lineEnding</code> says <code>\r\n</code>; Excel reads both.</p>
<h2>Four ways to read</h2>
<table>
<thead><tr><th>To get</th><th>From text</th><th>From a file</th></tr></thead>
<tbody>
<tr><td>Rows of strings: <code>string[][]</code></td><td><a href="function.csv-parse.html">csv_parse</a></td><td><a href="function.csv-read.html">csv_read</a></td></tr>
<tr><td>Rows by column name: <code>map[string, string][]</code></td><td><a href="function.csv-parse-maps.html">csv_parse_maps</a></td><td><a href="function.csv-read-maps.html">csv_read_maps</a></td></tr>
<tr><td>Objects of a class, typed</td><td><a href="function.csv-decode.html">csv_decode</a></td><td><code>csv_decode(file_get_contents(path))</code></td></tr>
<tr><td>A record at a time</td><td></td><td><a href="class.csvreader.html">CsvReader</a></td></tr>
</tbody>
</table>
<p>And to write: <a href="function.csv-format.html">csv_format</a> makes text from rows, <a href="function.csv-write.html">csv_write</a> writes them to a file, <a href="function.csv-encode.html">csv_encode</a> writes objects, and <a href="class.csvwriter.html">CsvWriter</a> writes a row at a time.</p>""",
    "functions": [
        {
            "name": "csv_parse", "purpose": "Read CSV text as rows",
            "params": [param("text", "string", "The CSV."), _DELIMITER],
            "returns": "string[][]",
            "description": "Reads every record of <code>text</code>, each as an array of its fields, like PHP's <code>str_getcsv</code> for a whole text, or Python's <code>csv.reader</code>.",
            "return_description": "The rows; an empty array for text without records.",
            "errors": [_CSV_ERROR],
            "examples": [{"title": "Quoted fields", "code": 'string text = "name,note\\nTea,\\"strong, black\\"\\n\\"Cake \\"\\"deluxe\\"\\"\\",\\"two\\nlines\\"\\n";\nstring[][] rows = csv_parse(text);\nprint(len(rows), rows[1]);\nprint(rows[2][0], rows[2][1] == "two\\nlines");',
                          "output": '3 ["Tea", "strong, black"]\nCake "deluxe" true'}],
            "see_also": ["csv_read", "csv_parse_maps", "csv_format"],
        },
        {
            "name": "csv_parse_maps", "purpose": "Read CSV text by column name",
            "params": [param("text", "string", "The CSV, with a header row."), _DELIMITER],
            "returns": "map[string, string][]",
            "description": "Reads the first record as a header of column names, and each record after it as a map from those names to its fields.",
            "return_description": "A map for each record after the header.",
            "errors": [_CSV_ERROR, "<code>CsvException</code> if a record has more or fewer fields than the header, or the header names a column twice."],
            "examples": [{"title": "By name", "code": 'map[string, string][] people = csv_parse_maps("name,email\\nAda,ada@example.com\\nLin,lin@example.com\\n");\nforeach (people as person) {\n    print(person["name"], person["email"]);\n}',
                          "output": "Ada ada@example.com\nLin lin@example.com"}],
            "see_also": ["csv_read_maps", "csv_decode"],
        },
        {
            "name": "csv_decode", "purpose": "Read CSV text into objects",
            "params": [param("text", "string", "The CSV, with a header row."), _DELIMITER],
            "returns": "T[]",
            "description": "Reads each record after the header into a new object of the class the result is used as, as <a href=\"function.json-decode.html\">json_decode</a> does for JSON: each public property is set from the column with its name, converted to the property's type. Columns without a property are ignored, and the columns can be in any order. Around ints, doubles, and bools, spaces are ignored; bools are <code>true</code>, <code>false</code>, <code>1</code>, <code>0</code>, <code>yes</code>, or <code>no</code>, in any case.",
            "return_description": "The objects, one for each record after the header.",
            "errors": ["<code>CsvException</code> if a column a property needs is missing, a record has more or fewer fields than the header, or a field doesn't fit its property, such as <code>abc</code> for an int; the message names the line and column.",
                       "A compile error if the result isn't used as an array of objects, or the class has a public property that isn't an int, double, bool, or string."],
            "examples": [{"title": "Typed rows", "code": 'class Product {\n    public string name;\n    public double price;\n    public int stock;\n}\n\nstring text = "name,stock,price\\nTea,12,3.50\\n\\"Cake, large\\",0,12\\n";\nProduct[] products = csv_decode(text);\nforeach (products as product) {\n    print(product->name, product->price * 2.0, product->stock + 1);\n}',
                          "output": "Tea 7 13\nCake, large 24 1"}],
            "see_also": ["csv_encode", "json_decode"],
        },
        {
            "name": "csv_encode", "purpose": "Write objects as CSV text",
            "params": [param("objects", "T[]", "The objects, of a class whose public properties are ints, doubles, bools, or strings."), _DELIMITER, _LINE_ENDING],
            "returns": "string",
            "description": "Writes a header of the class's public property names, then a row for each object, with each property as <code>print</code> would write it (bools as <code>true</code> and <code>false</code>, and a null string as an empty field). What it writes, <a href=\"function.csv-decode.html\">csv_decode</a> reads back.",
            "return_description": "The CSV text.",
            "errors": ["<code>RuntimeException</code> if an element is null.", "A compile error if the class has a public property that isn't an int, double, bool, or string."],
            "examples": [{"title": "A report", "code": 'class Line {\n    function __construct(public string item, public int quantity, public bool paid) {}\n}\n\nLine[] lines = [new Line("Tea", 2, true), new Line("Cake, large", 1, false)];\nprint(csv_encode(lines));',
                          "output": 'item,quantity,paid\nTea,2,true\n"Cake, large",1,false\n'}],
            "see_also": ["csv_decode", "csv_format"],
        },
        {
            "name": "csv_format", "purpose": "Write rows as CSV text",
            "params": [param("rows", "string[][]", "The rows."), _DELIMITER, _LINE_ENDING],
            "returns": "string",
            "description": "Writes each row as a line of CSV, quoting a field only when it holds the delimiter, a quote, or a line break.",
            "return_description": "The CSV text, each row ending with <code>lineEnding</code>.",
            "errors": ["<code>InvalidArgumentException</code> for a delimiter that isn't one character, or a line ending other than <code>\"\\n\"</code> and <code>\"\\r\\n\"</code>."],
            "examples": [{"title": "Quoting what must be", "code": 'print(csv_format([["name", "note"], ["Tea", "strong, black"], ["Cake", "say \\"hi\\""]]));\nprint(csv_format([["a", "b"]], ";"));',
                          "output": 'name,note\nTea,"strong, black"\nCake,"say ""hi"""\n\na;b\n'}],
            "see_also": ["csv_write", "csv_parse"],
        },
        {
            "name": "csv_read", "purpose": "Read a CSV file as rows",
            "params": [param("path", "string", "The file."), _DELIMITER],
            "returns": "string[][]",
            "description": "Reads the whole file, as <a href=\"function.csv-parse.html\">csv_parse</a> reads text. For files too large to read at once, use <a href=\"class.csvreader.html\">CsvReader</a>.",
            "return_description": "The rows.",
            "errors": [_CSV_ERROR, "<code>FileNotFoundException</code>, <code>PermissionDeniedException</code>, or <code>IOException</code> if the file can't be read."],
            "examples": [{"title": "Write, then read", "code": 'csv_write("cities.csv", [["id", "city"], ["1", "Berlin"], ["2", "São Paulo"]]);\nprint(csv_read("cities.csv"));',
                          "output": '[["id", "city"], ["1", "Berlin"], ["2", "São Paulo"]]'}],
            "see_also": ["csv_read_maps", "csv_write", "CsvReader"],
        },
        {
            "name": "csv_read_maps", "purpose": "Read a CSV file by column name",
            "params": [param("path", "string", "The file, with a header row."), _DELIMITER],
            "returns": "map[string, string][]",
            "description": "Reads the whole file, as <a href=\"function.csv-parse-maps.html\">csv_parse_maps</a> reads text.",
            "return_description": "A map for each record after the header.",
            "errors": ["As for <a href=\"function.csv-parse-maps.html\">csv_parse_maps</a>, and the file errors of <a href=\"function.csv-read.html\">csv_read</a>."],
            "examples": [{"title": "By name", "code": 'csv_write("stock.csv", [["item", "count"], ["tea", "12"]]);\nprint(csv_read_maps("stock.csv")[0]["count"]);',
                          "output": "12"}],
            "see_also": ["csv_read", "csv_decode"],
        },
        {
            "name": "csv_write", "purpose": "Write rows to a CSV file",
            "params": [param("path", "string", "The file."), param("rows", "string[][]", "The rows."), _DELIMITER, _LINE_ENDING,
                       param("append", "bool", "Add to the end of the file instead of replacing it.", "false")],
            "returns": "void",
            "description": "Writes <code>rows</code> to the file as <a href=\"function.csv-format.html\">csv_format</a> formats them, creating the file if it doesn't exist. For a row at a time, use <a href=\"class.csvwriter.html\">CsvWriter</a>.",
            "return_description": "Nothing.",
            "errors": ["As for <a href=\"function.csv-format.html\">csv_format</a>, and <code>PermissionDeniedException</code> or <code>IOException</code> if the file can't be written."],
            "examples": [{"title": "Adding rows", "code": 'csv_write("log.csv", [["when", "what"]]);\ncsv_write("log.csv", [["today", "started, finally"]], append: true);\nprint(file_get_contents("log.csv"));',
                          "output": 'when,what\ntoday,"started, finally"\n'}],
            "see_also": ["csv_read", "CsvWriter"],
        },
    ],
}

CSVREADER_CLASS = r"""<h1>The CsvReader class</h1>
<p>Reads a CSV file a record at a time, so a file of any size takes little memory. Import it from <code>std/csv</code>. It reads as <a href="function.csv-parse.html">csv_parse</a> does.</p>
<h2>Class synopsis</h2>
<div class="synopsis"><code><span class="kw">final class</span> <span class="cls">CsvReader</span> {<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">static function</span> <span class="fn">open</span>(<span class="type">string</span> path, <span class="type">string</span> delimiter = <span class="default">","</span>): <span class="cls">CsvReader</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">next</span>(): <span class="type">string[]</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">line</span>(): <span class="type">int</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">close</span>(): <span class="type">void</span><br>
}</code></div>
<table>
<thead><tr><th>Method</th><th>Description</th></tr></thead>
<tbody>
<tr><td><code>CsvReader::open(path, delimiter)</code></td><td>Opens the file. Throws <code>FileNotFoundException</code>, <code>PermissionDeniedException</code>, or <code>IOException</code> if it can't.</td></tr>
<tr><td><code>next()</code></td><td>The next record's fields, or <code>null</code> at the end of the file. Throws <code>CsvException</code> for malformed CSV.</td></tr>
<tr><td><code>line()</code></td><td>The line the last record starts on, for messages about it. A record with a quoted line break spans several lines.</td></tr>
<tr><td><code>close()</code></td><td>Closes the file; closing twice does nothing.</td></tr>
</tbody>
</table>
<h2>Example</h2>
<pre class="code">CsvReader reader = CsvReader::open("orders.csv");
string[] header = reader-&gt;next();
string[] order = reader-&gt;next();
while (order != null) {
    if (len(order) != len(header)) {
        print(f"line {reader-&gt;line()}: expected {len(header)} fields");
    }
    order = reader-&gt;next();
}
reader-&gt;close();</pre>
"""

CSVWRITER_CLASS = r"""<h1>The CsvWriter class</h1>
<p>Writes a CSV file a row at a time. Import it from <code>std/csv</code>. It writes as <a href="function.csv-format.html">csv_format</a> does.</p>
<h2>Class synopsis</h2>
<div class="synopsis"><code><span class="kw">final class</span> <span class="cls">CsvWriter</span> {<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">static function</span> <span class="fn">open</span>(<span class="type">string</span> path, <span class="type">string</span> delimiter = <span class="default">","</span>, <span class="type">string</span> lineEnding = <span class="default">"\n"</span>, <span class="type">bool</span> append = <span class="default">false</span>): <span class="cls">CsvWriter</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">write</span>(<span class="type">string[]</span> row): <span class="type">void</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">close</span>(): <span class="type">void</span><br>
}</code></div>
<table>
<thead><tr><th>Method</th><th>Description</th></tr></thead>
<tbody>
<tr><td><code>CsvWriter::open(path, delimiter, lineEnding, append)</code></td><td>Creates the file, or replaces it, or adds to it with <code>append</code>. Throws <code>PermissionDeniedException</code> or <code>IOException</code> if it can't, and <code>InvalidArgumentException</code> for a bad delimiter or line ending.</td></tr>
<tr><td><code>write(row)</code></td><td>Writes one row.</td></tr>
<tr><td><code>close()</code></td><td>Writes what's left and closes the file.</td></tr>
</tbody>
</table>
<h2>Example</h2>
<pre class="code">CsvWriter writer = CsvWriter::open("report.csv");
writer-&gt;write(["day", "visits"]);
foreach (range(1, 7) as day) {
    writer-&gt;write([to_string(day), to_string(random_int(100, 999))]);
}
writer-&gt;close();</pre>
"""

CATEGORIES = [GENERAL, STRINGS, REGEX, MATH, ARRAYS, MAPS, FILES, CSV, DATES, JSON, HTTP, HTTP_SERVER, CRYPTO, PROGRAM, CONVERSIONS, OUTPUT]

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
│   ├── DatabaseException
│   │   └── ConstraintViolationException
│   │       └── UniqueViolationException
│   ├── ProcessException
│   ├── KeyNotFoundException
│   ├── OutOfRangeException
│   └── InvalidUtf8Exception
├── ArithmeticException
│   └── DivisionByZeroException
├── InvalidArgumentException
├── JsonException
├── RegexException
├── CsvException
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
<tr><td><code>DatabaseException</code></td><td>A <a href="class.database.html">database</a> couldn't be opened or rejected a statement (an SQL error, a broken constraint, a missing table, a lock held for more than 5 seconds), the wrong number of values was given for a statement's placeholders, or a <a href="class.row.html">Row</a> getter was asked for a column that doesn't exist or holds another type. <code>getSqlState()</code> gives the SQLSTATE and <code>getCode()</code> the database's error number; see <a href="class.database.html">Errors</a>.</td></tr>
<tr><td><code>ConstraintViolationException</code></td><td>A statement broke a constraint: <code>NOT NULL</code>, <code>CHECK</code>, a foreign key, or a unique key. A subclass of <code>DatabaseException</code>.</td></tr>
<tr><td><code>UniqueViolationException</code></td><td>A row would duplicate another's value in a unique key or primary key. A subclass of <code>ConstraintViolationException</code>.</td></tr>
<tr><td><code>ProcessException</code></td><td>A command given to <a href="function.exec.html">exec</a> or <a href="function.shell.html">shell</a> couldn't be run (it wasn't found or isn't executable, or the working directory doesn't exist), or ran past its timeout. A command that exits with an error status doesn't throw; check <code>exitCode</code>.</td></tr>
<tr><td><code>KeyNotFoundException</code></td><td>A map lookup used a key that isn't in the map.</td></tr>
<tr><td><code>OutOfRangeException</code></td><td>An array index is below 0 or not less than the array's length.</td></tr>
<tr><td><code>InvalidUtf8Exception</code></td><td>Bytes or file contents that should be text aren't valid UTF-8.</td></tr>
<tr><td><code>ArithmeticException</code></td><td>An int raised to a negative power, a negative shift amount, or a double that can't be converted to an int.</td></tr>
<tr><td><code>DivisionByZeroException</code></td><td>Int <code>/</code> or <code>%</code> with a zero right-hand side.</td></tr>
<tr><td><code>InvalidArgumentException</code></td><td>A standard library function was given an argument it can't accept, such as a negative <code>str_repeat</code> count.</td></tr>
<tr><td><code>RegexException</code></td><td>A <a href="ref.regex.html">regular expression</a> is malformed, its replacement refers to a group it doesn't have, or it needs too much backtracking on its text.</td></tr>
<tr><td><code>CsvException</code></td><td><a href="ref.csv.html">CSV</a> is malformed (a quoted field that's never closed), or doesn't fit what it's read into: a record with the wrong number of fields, a missing column, or a value that isn't a number where one is needed. The message names the line.</td></tr>
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


DATABASE_CLASS = """<h1>The Database class</h1>
<p>A connection to an SQL database, like PHP's <code>PDO</code>: SQLite, which is built into every program that uses it, so there's no server or library to install and the database is a single file; or a <a href="#postgres">PostgreSQL</a>, <a href="#mysql">MySQL, or MariaDB</a> server, whose protocols qiq speaks itself, so there's nothing to install for those either. The same methods work with all of them.</p>
<h2>Class synopsis</h2>
<div class="synopsis"><code><span class="kw">final class</span> <span class="cls">Database</span> {<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">static function</span> <span class="fn">open</span>(<span class="type">string</span> dsn): <span class="cls">Database</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">execute</span>(<span class="type">string</span> sql, ...values): <span class="type">int</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">query</span>(<span class="type">string</span> sql, ...values): <span class="cls">Row</span>[]<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">queryOne</span>(<span class="type">string</span> sql, ...values): <span class="cls">Row</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">queryAs</span>(<span class="type">string</span> sql, ...values): <span class="cls">T</span>[]<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">queryOneAs</span>(<span class="type">string</span> sql, ...values): <span class="cls">T</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">cursor</span>(<span class="type">string</span> sql, ...values): <span class="cls">Cursor</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">executeScript</span>(<span class="type">string</span> sql): <span class="type">void</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">lastInsertId</span>(): <span class="type">int</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">begin</span>(): <span class="type">void</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">commit</span>(): <span class="type">void</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">rollback</span>(): <span class="type">void</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">inTransaction</span>(): <span class="type">bool</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">transaction</span>(<span class="kw">function</span>(): <span class="type">void</span> body): <span class="type">void</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">close</span>(): <span class="type">void</span><br>
}</code></div>

<h2>Example</h2>
```qiq
Database db = Database::open("sqlite::memory:");
db->execute("create table users (id integer primary key, name text not null, age integer)");
db->execute("insert into users (name, age) values (?, ?)", "Ada", 36);
db->execute("insert into users (name, age) values (?, ?)", "Lin", 41);
print("new id:", db->lastInsertId());

foreach (db->query("select id, name, age from users where age > ? order by name", 30) as row) {
    print(row->getInt("id"), row->getString("name"), row->getInt("age"));
}
Row ada = db->queryOne("select age from users where name = ?", "Ada");
if (ada != null) {
    print("Ada is", ada->getInt("age"));
}
print(db->execute("update users set age = age + 1"), "rows changed");
```
```output
new id: 2
1 Ada 36
2 Lin 41
Ada is 36
2 rows changed
```

<h2>Values for placeholders</h2>
<p>Write a <code>?</code> in the SQL for each value, and give the values after the SQL, in order. Each can be an <code>int</code>, <code>double</code>, <code>bool</code> (stored as 1 or 0), <code>byte</code>, <code>string</code>, <code>char</code>, <code>byte[]</code> (stored as a blob), <a href="class.datetime.html">DateTime</a> (stored as text: see <a href="class.datetime.html">In databases</a>), or <code>null</code>; a <code>null</code> string or array is stored as NULL. The compiler checks the types, as it does for <a href="function.sprintf.html">sprintf</a>.</p>
<p>The values are sent to the database separately from the SQL, never pasted into it, so a value can't change what the statement does: this is what prevents SQL injection. Build the SQL itself only from text in your program, and give everything else as a value.</p>
<pre class="code">Database db = Database::open("sqlite:app.db");
string name = "Robert'); drop table students; --";
db->execute("insert into students (name) values (?)", name);   // stores the name as it is</pre>
<p>The number of values must match the number of placeholders, or the call throws <code>DatabaseException</code>. Named placeholders such as <code>:name</code> aren't supported.</p>

<h2>Methods</h2>
<table>
<thead><tr><th>Method</th><th>Description</th></tr></thead>
<tbody>
<tr><td><code>Database::open(dsn)</code></td><td>Opens a database. <code>"sqlite:path/to/file.db"</code> opens a file, relative to the directory the program runs in, creating it if it doesn't exist; <code>"sqlite::memory:"</code> makes a database in memory that lasts until it's closed. SQLite's URI file names work too, such as <code>"sqlite:file:data.db?mode=ro"</code> to open a file read-only. <code>"postgres://user:password@host:5432/database"</code> connects to PostgreSQL, and <code>"mysql://user:password@host:3306/database"</code> to MySQL or MariaDB (see <a href="#postgres">below</a>). Throws <code>DatabaseException</code> when the database can't be opened or the login fails, and <code>NetworkException</code> when the server can't be reached.</td></tr>
<tr><td><code>execute(sql, ...values)</code></td><td>Runs one statement, such as <code>INSERT</code>, <code>UPDATE</code>, <code>DELETE</code>, or <code>CREATE TABLE</code>. Returns the number of rows it inserted, changed, or deleted.</td></tr>
<tr><td><code>query(sql, ...values)</code></td><td>Runs a query and returns all its rows as an array of <a href="class.row.html">Row</a> objects, empty when nothing matches.</td></tr>
<tr><td><code>queryOne(sql, ...values)</code></td><td>The first row of a query, or <code>null</code> when there is none. It stops after that row, so it's the quick way to look up one record.</td></tr>
<tr><td><code>queryAs(sql, ...values)</code>, <code>queryOneAs(sql, ...values)</code></td><td>Like <code>query</code> and <code>queryOne</code>, with each row read into an object of the class the result is used as: <code>User[] users = db-&gt;queryAs(...)</code>. See <a href="#objects">Reading rows into objects</a>.</td></tr>
<tr><td><code>cursor(sql, ...values)</code></td><td>Runs a query and returns a <a href="class.cursor.html">Cursor</a>, which reads its rows a batch at a time: for results too big to hold in memory at once.</td></tr>
<tr><td><code>executeScript(sql)</code></td><td>Runs several statements separated by semicolons, such as a schema. They can't have placeholders. <code>execute</code>, <code>query</code>, and <code>queryOne</code> run exactly one statement and throw if the SQL has more, so a value can never smuggle in a second one.</td></tr>
<tr><td><code>lastInsertId()</code></td><td>The id (the <code>INTEGER PRIMARY KEY</code>, or SQLite's rowid) of the last row inserted. With PostgreSQL, the value this connection's last sequence gave (such as a <code>serial</code> or identity column's), as PDO does; <code>insert ... returning id</code> with <code>queryOne</code> says exactly which column you mean. With MySQL, the last <code>AUTO_INCREMENT</code> value this connection generated, as <code>LAST_INSERT_ID()</code> gives.</td></tr>
<tr><td><code>begin()</code>, <code>commit()</code>, <code>rollback()</code></td><td>Start a transaction, save its changes, or undo them. Without a transaction, each statement is saved on its own.</td></tr>
<tr><td><code>inTransaction()</code></td><td>Whether a transaction is open.</td></tr>
<tr><td><code>transaction(body)</code></td><td>Runs <code>body</code> in a transaction: commits when it returns, or rolls back and rethrows when it throws. Inside another transaction, it uses a savepoint, so only <code>body</code>'s own changes roll back (see <a href="#nested">Nested transactions</a>).</td></tr>
<tr><td><code>close()</code></td><td>Closes the connection. Closing twice does nothing; using the database afterwards throws <code>DatabaseException</code>. A program's open databases are closed when it ends.</td></tr>
</tbody>
</table>

<h2 id="objects">Reading rows into objects</h2>
<p><code>queryAs</code> reads each row into an object, and <code>queryOneAs</code> reads the first row into one (or gives <code>null</code>). Like <a href="function.json-decode.html">json_decode</a>, they take the class from where the result goes, so use them where the type is known: in a declaration, an assignment, an argument, or a <code>return</code>.</p>
```qiq
class User {
    function __construct(public int id, public string name, public string email = "", public bool admin = false) {}
}

Database db = Database::open("sqlite::memory:");
db->execute("create table users (id integer primary key, name text, email text, admin integer)");
db->execute("insert into users (name, email, admin) values (?, ?, ?)", "Ada", "ada@example.com", true);
db->execute("insert into users (name, email) values (?, ?)", "Lin", null);

User[] users = db->queryAs("select * from users order by name");
foreach (users as user) {
    print(user->id, user->name, user->email, user->admin);
}
User lin = db->queryOneAs("select id, name from users where name = ?", "Lin");
print(lin->name, lin->email == "");
```
```output
1 Ada ada@example.com true
2 Lin  false
Lin true
```
<ul>
<li>When the class has a constructor, it's called with each parameter set from the column of the same name, read with the getter for the parameter's type (<code>getInt</code> for an <code>int</code>, and so on), so constructor promotion makes a short record class. A parameter with a default gets it when its column is NULL or missing; one without a default needs the column, or the query throws <code>DatabaseException</code>, as does a value of another type.</li>
<li>When the class has no constructor, its public properties are set the same way, by name, after the object is made. Properties with defaults keep them when their column is missing or NULL.</li>
<li>Parameters and properties can be <code>int</code>, <code>double</code>, <code>bool</code>, <code>string</code>, <code>byte[]</code>, or <code>DateTime</code> (read with <code>getDateTime</code>); others are a compile error. Name columns to match, with <code>AS</code> where they differ: <code>select created_at as createdAt</code>. Columns that nothing reads are ignored.</li>
<li>The constructor must be public, and the class concrete: not an interface or abstract.</li>
<li>For results too big to hold at once, a <a href="class.cursor.html">Cursor</a>'s <code>nextAs</code> and <code>fetchAs</code> read rows into objects the same way, a batch at a time.</li>
</ul>

<h2>Transactions</h2>
<p>Changes in a transaction are saved together or not at all. They're also much faster: outside a transaction, SQLite makes each statement durable on its own, waiting for the disk each time.</p>
```qiq
Database db = Database::open("sqlite::memory:");
db->execute("create table accounts (name text primary key, balance integer check (balance >= 0))");
db->execute("insert into accounts values (?, ?), (?, ?)", "ada", 100, "lin", 20);

function transfer(Database db, string from, string to, int amount): void {
    db->transaction(function(): void {
        db->execute("update accounts set balance = balance + ? where name = ?", amount, to);
        db->execute("update accounts set balance = balance - ? where name = ?", amount, from);
    });
}
transfer(db, "ada", "lin", 30);
try {
    transfer(db, "lin", "ada", 500);
} catch (DatabaseException e) {
    print("failed:", e->getMessage());
}
foreach (db->query("select * from accounts order by name") as row) {
    print(row->getString("name"), row->getInt("balance"));
}
```
```output
failed: CHECK constraint failed: balance >= 0
ada 70
lin 50
```
<p>The failed transfer had already added 500 to Ada's balance when the second update failed; rolling back undid it.</p>

<h3 id="nested">Nested transactions</h3>
<p>A <code>transaction</code> inside another (or inside a <code>begin()</code> you called) doesn't start a new transaction: it sets a savepoint. When its body throws, the changes it made are rolled back to the savepoint and the exception goes on; the outer transaction can catch it and carry on, or let it go and roll back everything. When its body returns, its changes become part of the outer transaction, saved when that commits. So a function can use <code>transaction</code> whether or not its caller already has one.</p>
```qiq
Database db = Database::open("sqlite::memory:");
db->execute("create table orders (id integer primary key, item text unique)");

function addOrder(Database db, string item): bool {
    try {
        db->transaction(function(): void {
            db->execute("insert into orders (item) values (?)", item);
        });
        return true;
    } catch (UniqueViolationException e) {
        return false;
    }
}
db->transaction(function(): void {
    print(addOrder(db, "pen"), addOrder(db, "ink"), addOrder(db, "pen"));
});
print(db->queryOne("select count(*) as n from orders")->getInt("n"));
```
```output
true true false
2
```
<p>The savepoints work the same way in SQLite, PostgreSQL, and MySQL. In PostgreSQL, rolling back to a savepoint is also how a transaction recovers from a failed statement, which otherwise blocks every statement after it until <code>rollback()</code>.</p>

<h2>Errors</h2>
<p>Everything that goes wrong throws <code>DatabaseException</code>, with the database's message: an SQL syntax error, a missing table or column, a broken <code>UNIQUE</code>, <code>NOT NULL</code>, <code>CHECK</code>, or foreign key constraint, or the wrong number of values for the placeholders. Two subclasses say which kind of constraint failed, the same way for every database:</p>
<ul>
<li><code>UniqueViolationException</code>: the row would duplicate another's value in a unique key or primary key, as when an email address is already registered.</li>
<li><code>ConstraintViolationException</code>, which <code>UniqueViolationException</code> extends: any broken constraint (<code>NOT NULL</code>, <code>CHECK</code>, a foreign key, or a unique key).</li>
</ul>
<p><code>getSqlState()</code> gives the standard five-character SQLSTATE, such as <code>23505</code> for a duplicate key in PostgreSQL or <code>42P01</code> for a missing table (<code>HY000</code>, a general error, when there's no more specific one), and <code>getCode()</code> the database's own error number: SQLite's extended result code, or MySQL's error number, such as 1062 for a duplicate entry (PostgreSQL has none, so it's 0). SQLSTATEs differ between databases (MySQL reports every constraint as <code>23000</code>), so catch the subclasses to handle a kind of error portably.</p>
```qiq
Database db = Database::open("sqlite::memory:");
db->execute("create table users (email text unique not null)");
db->execute("insert into users values (?)", "ada@example.com");

function register(Database db, string email): string {
    try {
        db->execute("insert into users values (?)", email);
        return "welcome";
    } catch (UniqueViolationException e) {
        return "that email is already registered";
    } catch (ConstraintViolationException e) {
        return "invalid: " + e->getMessage() + " (" + e->getSqlState() + ")";
    }
}
print(register(db, "lin@example.com"));
print(register(db, "ada@example.com"));
string none = null;
print(register(db, none));
```
```output
welcome
that email is already registered
invalid: NOT NULL constraint failed: users.email (23502)
```
```qiq
Database db = Database::open("sqlite::memory:");
db->execute("create table users (email text unique)");
db->execute("insert into users values (?)", "ada@example.com");
try {
    db->execute("insert into users values (?)", "ada@example.com");
} catch (DatabaseException e) {
    print(e->getMessage());
}
try {
    db->query("select * from users where email = ?");
} catch (DatabaseException e) {
    print(e->getMessage());
}
```
```output
UNIQUE constraint failed: users.email
query: the SQL has 1 placeholder (?), but 0 values were given
```

<h2 id="postgres">PostgreSQL</h2>
<pre class="code">Database db = Database::open("postgres://app:secret@db.example.com:5432/shop");
Row order = db->queryOne("insert into orders (customer, total) values (?, ?) returning id", customer, total);
print("order", order->getInt("id"));</pre>
<p>The URL is <code>postgres://user:password@host:port/database</code> (or <code>postgresql://</code>), as libpq writes it. Everything after <code>postgres://</code> is optional: the user defaults to <code>postgres</code>, the host to <code>localhost</code>, the port to 5432, and the database to the user's name. Percent-encode characters such as <code>@</code>, <code>:</code>, and <code>/</code> in the user and password (<code>p%40ss</code> for <code>p@ss</code>). Without a password in the URL, the <code>PGPASSWORD</code> environment variable is used, which keeps it out of your code. Logins use SCRAM-SHA-256 (PostgreSQL's default), MD5, or a plain password.</p>
<table>
<thead><tr><th>Option</th><th>Meaning</th></tr></thead>
<tbody>
<tr><td><code>sslmode=require</code></td><td>Encrypt the connection with TLS, and check that the server's certificate is valid for its host name, against the system's CA certificates or the ones in <code>SSL_CERT_FILE</code> (libpq calls this <code>verify-full</code>, which is accepted too). This is the default for every host except the local machine.</td></tr>
<tr><td><code>sslrootcert=/path/ca.pem</code></td><td>Check the server's certificate against the CA certificates in this file instead of the system's, as for a managed database whose provider has its own CA (such as Amazon RDS's bundle). It turns TLS on, even for the local machine.</td></tr>
<tr><td><code>host=/var/run/postgresql</code></td><td>Connect through the Unix socket in this directory (to <code>.s.PGSQL.5432</code>, or the port given) instead of TCP, as libpq does; a path to the socket file works too, and so does the path as the URL's host, percent-encoded: <code>postgres://app@%2Fvar%2Frun%2Fpostgresql/shop</code>. It's the usual way to reach a server on the same machine, and PostgreSQL's <code>peer</code> login can then use the program's system user instead of a password. TLS isn't used over a socket.</td></tr>
<tr><td><code>sslmode=disable</code></td><td>Don't encrypt. The default for <code>localhost</code>, <code>127.0.0.1</code>, and <code>::1</code>; use it for other hosts only on a network you trust, such as between containers on one machine.</td></tr>
<tr><td><code>connect_timeout=10</code></td><td>Seconds to wait for the server when connecting; 10 by default. Statements have no time limit; set PostgreSQL's <code>statement_timeout</code> for one.</td></tr>
</tbody>
</table>
<p>Options go after <code>?</code>, separated by <code>&amp;</code>: <code>postgres://app@10.0.0.5/shop?sslmode=disable&amp;connect_timeout=3</code>. Each TLS connection checks the certificate's host name, so connect by the name the certificate was issued for. Unlike libpq, there's no <code>prefer</code> mode that falls back to an unencrypted connection when the server doesn't offer TLS: without <code>sslmode=disable</code>, such a server is an error that says so.</p>
<ul>
<li>Placeholders are <code>?</code>, as with SQLite; qiq turns them into PostgreSQL's <code>$1</code>, <code>$2</code>, ..., leaving alone any <code>?</code> inside strings, quoted names, comments, and <code>$$</code>-quoted text. For PostgreSQL's own <code>?</code> operators (on <code>jsonb</code>), write <code>??</code>: <code>where tags ?? 'new'</code>.</li>
<li>Values are sent as text that PostgreSQL reads as the type the column or function needs, as PDO does; a byte array is sent as <code>bytea</code>. Where nothing gives a type, as in <code>select ? as n</code>, the value is text: cast it, <code>select ?::int as n</code>.</li>
<li>Columns are read by type: <code>smallint</code>, <code>integer</code>, and <code>bigint</code> with <code>getInt</code>, <code>real</code> and <code>double precision</code> with <code>getDouble</code>, <code>boolean</code> with <code>getBool</code>, <code>bytea</code> with <code>getBytes</code>, and <code>numeric</code> with <code>getString</code> (its exact digits) or <code>getDouble</code>, or <code>getInt</code> when it has no fraction (as <code>sum</code> of <code>bigint</code> gives). Everything else, such as <code>text</code>, dates, times, <code>uuid</code>, and <code>json</code>, is read with <code>getString</code>, as PostgreSQL writes it.</li>
<li>Errors carry PostgreSQL's message and detail: <code>duplicate key value violates unique constraint "users_email_key": Key (email)=(ada@example.com) already exists.</code> After an error in a transaction, PostgreSQL refuses further statements until <code>rollback()</code>, which <code>transaction(body)</code> does for you.</li>
<li>Statements are prepared by name, and each connection keeps its 16 most recently used ones, so running one again skips PostgreSQL's parsing and planning: a point query took 20 µs rather than 30, and a query with a join or grouping about 40% less. When a table a cached statement reads changes its columns, or something drops the server's prepared statements (<code>DEALLOCATE</code>, <code>DISCARD</code>), the statement is prepared again and run again; inside a transaction it can't be run again, so that statement throws once. A connection pooler in front of PostgreSQL must support prepared statements: PgBouncer does from version 1.21, with <code>max_prepared_statements</code> set.</li>
<li>A connection that is lost, because the server restarted or closed it, connects again for the next statement. If a transaction was open, that statement throws instead, saying so, since the server rolled the transaction back.</li>
</ul>

<h2 id="mysql">MySQL and MariaDB</h2>
<pre class="code">Database db = Database::open("mysql://app:secret@db.example.com:3306/shop");
db->execute("insert into orders (customer, total) values (?, ?)", customer, total);
print("order", db->lastInsertId());</pre>
<p>The URL is <code>mysql://user:password@host:port/database</code>, or <code>mariadb://</code>, which is the same. The user defaults to <code>root</code>, the host to <code>localhost</code>, and the port to 3306; without a database, statements name their tables' databases (<code>shop.orders</code>) or start with <code>use shop</code>. Without a password in the URL, the <code>MYSQL_PWD</code> environment variable is used. The options and the TLS rules are the same as for <a href="#postgres">PostgreSQL</a>: TLS with the certificate checked, except for the local machine, <code>sslmode=disable</code> to turn it off, <code>sslrootcert</code> for a CA of your own, and <code>connect_timeout</code>. For a Unix socket, give its path: <code>mysql://app:secret@localhost/shop?socket=/var/run/mysqld/mysqld.sock</code> (or the directory, for a socket named <code>mysqld.sock</code>). Logins use <code>caching_sha2_password</code> (MySQL 8's default; on an unencrypted connection, the password is sent encrypted with the server's RSA key) or <code>mysql_native_password</code> (MariaDB's). qiq needs MySQL or MariaDB 5.5 or later.</p>
<ul>
<li>Statements are prepared on the server, and their values sent separately, never pasted into the SQL. Each connection keeps its 16 most recently used statements prepared, so running one again skips that step. MySQL's placeholders are <code>?</code> already.</li>
<li>Columns are read by type: the integer types (including <code>BOOLEAN</code>, which is <code>TINYINT(1)</code>) with <code>getInt</code> or <code>getBool</code>, <code>FLOAT</code> and <code>DOUBLE</code> with <code>getDouble</code>, <code>DECIMAL</code> with <code>getString</code> (its exact digits) or <code>getDouble</code>, <code>BINARY</code>, <code>VARBINARY</code>, <code>BLOB</code>, and <code>BIT</code> with <code>getBytes</code>, and everything else (text, <code>DATE</code>, <code>DATETIME</code>, <code>TIME</code>, <code>JSON</code>, ...) with <code>getString</code>, as MySQL writes it: <code>2026-11-22 10:30:00.125</code>. A <code>BIGINT UNSIGNED</code> too big for an int reads like a <code>DECIMAL</code>.</li>
<li><code>execute</code> counts the rows an <code>UPDATE</code> matched, as SQLite and PostgreSQL do, rather than only those whose values changed.</li>
<li>A few statements can't be prepared (some administrative ones); they run as plain queries, which works when they have no values.</li>
<li>Statements that change tables, such as <code>CREATE TABLE</code>, commit an open transaction first: MySQL can't roll them back.</li>
</ul>

<h2>Program size</h2>
<p>SQLite adds about 1.6 MB to a program, and the PostgreSQL and MySQL drivers (with TLS) about 0.7 MB more. The drivers are only included when a <code>Database::open</code> could need them: a program whose URLs are all literals such as <code>"sqlite:app.db"</code> leaves them out, while one that builds its URL at run time, as from an environment variable, includes both.</p>

<h2>Using a database in a web server</h2>
<p>A database can be opened before calling <a href="function.http-serve.html">http_serve</a> and used in the handler. Each worker process gets its own connection: a connection opened before the workers start is opened again in each worker the first time it's used there, and the settings it made are made again, since they belong to the connection: SQLite's <code>PRAGMA</code> statements, PostgreSQL's <code>SET</code> statements (except <code>SET LOCAL</code>), and MySQL's <code>SET</code> statements (except <code>SET GLOBAL</code>, <code>SET PERSIST</code>, and <code>SET TRANSACTION</code>), including user variables such as <code>@user_id</code>. A <code>sqlite::memory:</code> database isn't shared between workers. With PostgreSQL and MySQL, each worker holds one connection to the server, so 4 workers use 4 of its connections.</p>
<p>With SQLite, the workers can read at the same time, but only one can write at a time. A statement waits up to 5 seconds for another process's write to finish before throwing <code>DatabaseException</code> (<code>database is locked</code>). For a server, switch the database to write-ahead logging, which lets reads go on while a write happens, and stop waiting for the disk after every commit:</p>
<pre class="code">Database db = Database::open("sqlite:app.db");
db->executeScript("pragma journal_mode = wal; pragma synchronous = normal;");
http_serve(":8080", function(Request r, ResponseWriter w): void {
    db->execute("insert into visits (path) values (?)", r->path);
    w->write("hello\n");
});</pre>
<p>With these two settings, a handler that inserts a row and reads one served about 28,000 requests a second with 4 workers on a laptop; without them, about 600, because each commit waited for the disk. <code>synchronous = normal</code> still never corrupts the database, but a power cut can lose the last commits.</p>
"""

ROW_CLASS = """<h1>The Row class</h1>
<p>One row of a query's result, from <a href="class.database.html">Database</a>'s <code>query</code> and <code>queryOne</code> (or read straight into your own class with <a href="class.database.html#objects">queryAs</a>). Read each column by name with the getter for its type. A getter throws <code>DatabaseException</code> when the column doesn't exist or holds another type, rather than converting the value, so a mistake shows up where it is.</p>
<h2>Class synopsis</h2>
<div class="synopsis"><code><span class="kw">final class</span> <span class="cls">Row</span> {<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">getInt</span>(<span class="type">string</span> column): <span class="type">int</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">getDouble</span>(<span class="type">string</span> column): <span class="type">double</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">getBool</span>(<span class="type">string</span> column): <span class="type">bool</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">getString</span>(<span class="type">string</span> column): <span class="type">string</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">getBytes</span>(<span class="type">string</span> column): <span class="type">byte[]</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">getDateTime</span>(<span class="type">string</span> column, <span class="type">string</span> zone = <span class="default">null</span>): <span class="cls">DateTime</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">getIntOr</span>(<span class="type">string</span> column, <span class="type">int</span> fallback): <span class="type">int</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">getDoubleOr</span>(<span class="type">string</span> column, <span class="type">double</span> fallback): <span class="type">double</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">getBoolOr</span>(<span class="type">string</span> column, <span class="type">bool</span> fallback): <span class="type">bool</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">isNull</span>(<span class="type">string</span> column): <span class="type">bool</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">has</span>(<span class="type">string</span> column): <span class="type">bool</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">columns</span>(): <span class="type">string[]</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">toMap</span>(): <span class="type">map[string, string]</span><br>
}</code></div>

<h2>Methods</h2>
<table>
<thead><tr><th>Method</th><th>Reads</th><th>When the column is NULL</th></tr></thead>
<tbody>
<tr><td><code>getInt(column)</code></td><td>An <code>INTEGER</code></td><td>Throws</td></tr>
<tr><td><code>getDouble(column)</code></td><td>A <code>REAL</code>, or an <code>INTEGER</code> as a double (<code>SUM</code> and <code>AVG</code> can give either), or a PostgreSQL <code>numeric</code> or MySQL <code>DECIMAL</code></td><td>Throws</td></tr>
<tr><td><code>getBool(column)</code></td><td>An <code>INTEGER</code> that is 0 or 1, as SQLite stores booleans</td><td>Throws</td></tr>
<tr><td><code>getString(column)</code></td><td><code>TEXT</code>, or a PostgreSQL <code>numeric</code> or MySQL <code>DECIMAL</code> as its exact digits</td><td>Returns <code>null</code>, so <code>?? ""</code> gives a default</td></tr>
<tr><td><code>getBytes(column)</code></td><td>A <code>BLOB</code>, or text that isn't valid UTF-8</td><td>Returns <code>null</code></td></tr>
<tr><td><code>getDateTime(column, zone)</code></td><td>A <a href="class.datetime.html">DateTime</a>: text such as a PostgreSQL <code>timestamptz</code> or a MySQL <code>DATETIME</code>, read as <code>DateTime::parse</code> reads it (a time without an offset is in <code>zone</code>, or the default zone), or an <code>INTEGER</code> Unix timestamp</td><td>Returns <code>null</code></td></tr>
<tr><td><code>getIntOr</code>, <code>getDoubleOr</code>, <code>getBoolOr(column, fallback)</code></td><td>The same as the getter</td><td>Returns <code>fallback</code></td></tr>
<tr><td><code>isNull(column)</code></td><td colspan="2">Whether the column is NULL</td></tr>
<tr><td><code>has(column)</code></td><td colspan="2">Whether the result has a column with that name</td></tr>
<tr><td><code>columns()</code></td><td colspan="2">The column names, in the order the query gives them</td></tr>
<tr><td><code>toMap()</code></td><td colspan="2">The row as text, for printing and debugging: each value written as <code>print</code> writes it, NULL as <code>null</code>, and a blob as its size</td></tr>
</tbody>
</table>
<p>Column names are what the query calls them: a table's column name, or the name given with <code>AS</code>. Name computed columns, as in <code>select count(*) as n</code>; without <code>AS</code>, the name is the expression's text, <code>count(*)</code>. When two columns have the same name, as <code>id</code> often does in a join, the getters read the first; rename the others with <code>AS</code>.</p>
<p>SQLite stores each value with its own type, whatever the column was declared as, so the getter follows the value: an <code>INTEGER</code> column that was given <code>"12"</code> holds text, which <code>getString</code> reads. PostgreSQL's and MySQL's columns each have one type; see <a href="class.database.html#postgres">PostgreSQL</a> and <a href="class.database.html#mysql">MySQL</a> for which getter reads which.</p>

<h2>Example</h2>
```qiq
Database db = Database::open("sqlite::memory:");
db->execute("create table items (name text, price real, stock integer, note text, active integer)");
db->execute("insert into items values (?, ?, ?, ?, ?)", "pen", 1.5, 10, null, true);
db->execute("insert into items values (?, ?, ?, ?, ?)", "ink", 7.25, null, "refill", false);

foreach (db->query("select * from items order by name") as row) {
    print(row->getString("name"), row->getDouble("price"), row->getIntOr("stock", 0), row->getString("note") ?? "-", row->getBool("active"));
}
Row totals = db->queryOne("select count(*) as n, sum(price) as total from items");
print(totals->getInt("n"), totals->getDouble("total"), totals->toMap());

Row ink = db->queryOne("select * from items where name = ?", "ink");
try {
    print(ink->getInt("stock"));
} catch (DatabaseException e) {
    print(e->getMessage());
}
```
```output
ink 7.25 0 refill false
pen 1.5 10 - true
2 8.75 {"n" => "2", "total" => "8.75"}
column 'stock' is NULL; check with isNull first, or read it with a fallback, as getIntOr("stock", fallback)
```
"""

CURSOR_CLASS = """<h1>The Cursor class</h1>
<p>The rows of a query, read a batch at a time rather than all at once, from <a href="class.database.html">Database</a>'s <code>cursor</code>. Use it for results too big to hold in memory: reading a million rows with <code>query</code> took 190 MB, and with a cursor under 1 MB.</p>
<h2>Class synopsis</h2>
<div class="synopsis"><code><span class="kw">final class</span> <span class="cls">Cursor</span> {<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">next</span>(): <span class="cls">Row</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">fetch</span>(<span class="type">int</span> count): <span class="cls">Row</span>[]<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">nextAs</span>(): <span class="cls">T</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">fetchAs</span>(<span class="type">int</span> count): <span class="cls">T</span>[]<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">close</span>(): <span class="type">void</span><br>
}</code></div>
<table>
<thead><tr><th>Method</th><th>Description</th></tr></thead>
<tbody>
<tr><td><code>next()</code></td><td>The next <a href="class.row.html">Row</a>, or <code>null</code> after the last. It reads ahead 256 rows at a time, so each call is quick.</td></tr>
<tr><td><code>fetch(count)</code></td><td>Up to <code>count</code> more rows: fewer only at the end, and an empty array after it. Throws <code>InvalidArgumentException</code> for a count below 1.</td></tr>
<tr><td><code>nextAs()</code>, <code>fetchAs(count)</code></td><td>Like <code>next</code> and <code>fetch</code>, with each row read into an object of the class the result is used as, as <a href="class.database.html#objects">queryAs</a> does: <code>Event event = rows-&gt;nextAs();</code>, <code>Event[] batch = rows-&gt;fetchAs(500);</code>.</td></tr>
<tr><td><code>close()</code></td><td>Stops reading; the rest of the rows are skipped. A cursor closes itself after its last row, and closing twice does nothing.</td></tr>
</tbody>
</table>
<h2>Example</h2>
```qiq
Database db = Database::open("sqlite::memory:");
db->execute("create table events (id integer primary key, kind text)");
db->transaction(function(): void {
    foreach (range(1000) as i) {
        string kind = "view";
        if (i % 3 == 0) {
            kind = "click";
        }
        db->execute("insert into events (kind) values (?)", kind);
    }
});

Cursor rows = db->cursor("select id, kind from events where kind = ? order by id", "click");
int clicks = 0;
Row row = rows->next();
while (row != null) {
    clicks++;
    row = rows->next();
}
print("clicks:", clicks);

// In batches, stopping early.
Cursor batches = db->cursor("select id from events order by id");
Row[] batch = batches->fetch(400);
while (len(batch) > 0) {
    print("batch of", len(batch), "starting at", batch[0]->getInt("id"));
    if (batch[0]->getInt("id") > 400) {
        batches->close();
    }
    batch = batches->fetch(400);
}
```
```output
clicks: 334
batch of 400 starting at 1
batch of 400 starting at 401
```
<h2>Reading rows into objects</h2>
<p><code>nextAs</code> and <code>fetchAs</code> read rows into your own class, by constructor parameter or public property name, as <a href="class.database.html#objects">queryAs</a> does, so a large result can be streamed as objects. Like <code>queryAs</code>, they take the class from where the result goes, so use them in a declaration, an assignment, an argument, or a <code>return</code>.</p>
```qiq
class Event {
    function __construct(public int id, public string kind, public string note = "-") {}
}

Database db = Database::open("sqlite::memory:");
db->execute("create table events (id integer primary key, kind text, note text)");
db->execute("insert into events (kind, note) values (?, ?), (?, ?), (?, ?)", "click", "home", "view", null, "click", null);

Cursor rows = db->cursor("select * from events where kind = ? order by id", "click");
Event event = rows->nextAs();
while (event != null) {
    print(event->id, event->kind, event->note);
    event = rows->nextAs();
}
```
```output
1 click home
3 click -
```

<h2>Cursors and the connection</h2>
<p>With SQLite, a cursor has its own statement, so the database can run other statements, and other cursors, while it's open. With PostgreSQL and MySQL, the rows come from the connection as they're read, as with libpq's single-row mode and unbuffered MySQL queries, so that database can't run anything else until the cursor has read its last row or been closed: another statement throws <code>DatabaseException</code>. Open a second <code>Database</code> for statements to run while reading.</p>
<p>Closing a network cursor early stops its statement on the server, so leaving a huge result after a few rows is quick: PostgreSQL gets a cancel request on a connection of its own (as libpq sends one), and MySQL a <code>KILL QUERY</code> from a second login, after a few thousand more rows, since a short rest is quicker to read than to stop. Inside a PostgreSQL transaction, cancelling would abort the transaction, so there the rest of the rows are read instead; ask for fewer with <code>LIMIT</code> when that matters.</p>
<p>A cursor that's no longer used isn't closed until the program ends, or its database is closed, so close one you stop reading early, in a <code>finally</code> block when the reading can throw.</p>
"""

DATETIME_CLASS = """<h1>The DateTime class</h1>
<p>An instant in time, shown in a time zone of its own, precise to the microsecond. It works like PHP's <code>DateTimeImmutable</code>: a <code>DateTime</code> never changes, and the methods that change something give a new one, so passing one around can't change it under you.</p>
```qiq
DateTime meeting = DateTime::of(2026, 12, 3, 14, 30, zone: "Europe/London");
print(meeting);
print(meeting->format("l j F Y, g:i a T"));
print(meeting->withZone("America/New_York")->format("H:i T"), meeting->withZone("Asia/Kolkata")->format("H:i T"));
DateTime reminder = meeting->addDays(-1)->withTime(9, 0);
print(reminder, reminder->until(meeting));
```
```output
2026-12-03T14:30:00+00:00
Thursday 3 December 2026, 2:30 pm GMT
09:30 EST 20:00 IST
2026-12-02T09:00:00+00:00 1d 5h 30m
```
<h2>Class synopsis</h2>
<div class="synopsis"><code><span class="kw">final class</span> <span class="cls">DateTime</span> {<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">static function</span> <span class="fn">now</span>(<span class="type">string</span> zone = <span class="default">null</span>): <span class="cls">DateTime</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">static function</span> <span class="fn">of</span>(<span class="type">int</span> year, <span class="type">int</span> month, <span class="type">int</span> day, <span class="type">int</span> hour = <span class="default">0</span>, <span class="type">int</span> minute = <span class="default">0</span>, <span class="type">int</span> second = <span class="default">0</span>, <span class="type">int</span> microsecond = <span class="default">0</span>, <span class="type">string</span> zone = <span class="default">null</span>): <span class="cls">DateTime</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">static function</span> <span class="fn">parse</span>(<span class="type">string</span> text, <span class="type">string</span> format = <span class="default">null</span>, <span class="type">string</span> zone = <span class="default">null</span>): <span class="cls">DateTime</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">static function</span> <span class="fn">fromTimestamp</span>(<span class="type">int</span> seconds, <span class="type">string</span> zone = <span class="default">null</span>): <span class="cls">DateTime</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">static function</span> <span class="fn">fromTimestampMicros</span>(<span class="type">int</span> microseconds, <span class="type">string</span> zone = <span class="default">null</span>): <span class="cls">DateTime</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">year</span>(), <span class="fn">month</span>(), <span class="fn">day</span>(), <span class="fn">hour</span>(), <span class="fn">minute</span>(), <span class="fn">second</span>(), <span class="fn">microsecond</span>(): <span class="type">int</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">dayOfWeek</span>(), <span class="fn">dayOfYear</span>(), <span class="fn">daysInMonth</span>(), <span class="fn">timestamp</span>(), <span class="fn">timestampMicros</span>(), <span class="fn">offset</span>(): <span class="type">int</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">zone</span>(): <span class="type">string</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">withZone</span>(<span class="type">string</span> zone), <span class="fn">withDate</span>(<span class="type">int</span> year, <span class="type">int</span> month, <span class="type">int</span> day), <span class="fn">withTime</span>(<span class="type">int</span> hour, <span class="type">int</span> minute, <span class="type">int</span> second = <span class="default">0</span>, <span class="type">int</span> microsecond = <span class="default">0</span>): <span class="cls">DateTime</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">addYears</span>(<span class="type">int</span> n), <span class="fn">addMonths</span>, <span class="fn">addWeeks</span>, <span class="fn">addDays</span>, <span class="fn">addHours</span>, <span class="fn">addMinutes</span>, <span class="fn">addSeconds</span>: <span class="cls">DateTime</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">add</span>(<span class="cls">Duration</span> d), <span class="fn">sub</span>(<span class="cls">Duration</span> d): <span class="cls">DateTime</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">startOfDay</span>(), <span class="fn">endOfDay</span>(), <span class="fn">startOfMonth</span>(), <span class="fn">endOfMonth</span>(): <span class="cls">DateTime</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">isBefore</span>(<span class="cls">DateTime</span> other), <span class="fn">isAfter</span>(<span class="cls">DateTime</span> other), <span class="fn">equals</span>(<span class="cls">DateTime</span> other): <span class="type">bool</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">compareTo</span>(<span class="cls">DateTime</span> other): <span class="type">int</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">until</span>(<span class="cls">DateTime</span> other), <span class="fn">since</span>(<span class="cls">DateTime</span> other): <span class="cls">Duration</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">format</span>(<span class="type">string</span> format): <span class="type">string</span><br>
}</code></div>

<h2>Making one</h2>
<table>
<thead><tr><th>Method</th><th>Description</th></tr></thead>
<tbody>
<tr><td><code>DateTime::now(zone)</code></td><td>The current time.</td></tr>
<tr><td><code>DateTime::of(year, month, day, hour, minute, second, microsecond, zone)</code></td><td>A date and time on the zone's clock. Fields that don't make one, such as April 31 or 24:00, throw <code>InvalidArgumentException</code>; unlike PHP, they don't roll over into the next month or day.</td></tr>
<tr><td><code>DateTime::parse(text, format, zone)</code></td><td>Reads a date and time; see <a href="#parsing">Parsing</a>.</td></tr>
<tr><td><code>DateTime::fromTimestamp(seconds, zone)</code>, <code>fromTimestampMicros(microseconds, zone)</code></td><td>The instant that many seconds (or microseconds) after 1970-01-01 00:00:00 UTC: from <code>time()</code>, a database's Unix timestamps, or an API.</td></tr>
</tbody>
</table>
<p>Every <code>zone</code> is a name from the tz database (<code>"Europe/London"</code>, <code>"America/New_York"</code>), <code>"UTC"</code>, or a fixed offset (<code>"+05:30"</code>). A null zone means the default one: <code>date_default_timezone_set</code>'s, or the <code>TZ</code> environment variable's, or the system's (<code>/etc/localtime</code>), or UTC. An unknown name throws <code>InvalidArgumentException</code>.</p>

<h2>Reading it</h2>
<p><code>year()</code>, <code>month()</code> (1–12), <code>day()</code>, <code>hour()</code> (0–23), <code>minute()</code>, <code>second()</code>, and <code>microsecond()</code> are the time on the zone's clock. <code>dayOfWeek()</code> is 1 for Monday to 7 for Sunday, as ISO 8601 numbers them; <code>dayOfYear()</code> is 1 for January 1. <code>timestamp()</code> and <code>timestampMicros()</code> count from 1970 UTC, whatever the zone. <code>zone()</code> is the zone's name and <code>offset()</code> its offset from UTC at this instant, in seconds (3600 for <code>+01:00</code>, more in summer where clocks go forward).</p>
<p><code>format(format)</code> takes <a href="function.date.html">date</a>'s letters, plus <code>u</code> for the microseconds, <code>v</code> for the milliseconds, and <code>e</code> for the zone's name. Printing a <code>DateTime</code>, or adding one to a string, gives ISO 8601: <code>2026-12-03T14:30:00+00:00</code>, with the microseconds when there are any. <code>json_encode</code> writes it the same way, as a string.</p>

<h2>Changing it</h2>
<p>Each of these gives a new <code>DateTime</code> in the same zone. Negative numbers go back.</p>
<ul>
<li><code>addDays</code>, <code>addWeeks</code>, <code>addMonths</code>, and <code>addYears</code> step through the calendar and keep the time on the clock, even across a daylight saving change: a day after 12:00 is 12:00 the next day, though that may be 23 or 25 hours later. A month or year later on a day the month doesn't have is its last day: January 31 plus a month is February 28 (or 29), where PHP would give March 3.</li>
<li><code>addHours</code>, <code>addMinutes</code>, <code>addSeconds</code>, <code>add(duration)</code>, and <code>sub(duration)</code> add elapsed time: exactly that long later, whatever the clock then says.</li>
<li><code>withZone(zone)</code> shows the same instant on another zone's clock. <code>withDate</code> and <code>withTime</code> change the date or the time of day and keep the other.</li>
<li><code>startOfDay()</code> and <code>endOfDay()</code> are 00:00:00 and 23:59:59.999999 on the same date; <code>startOfMonth()</code> and <code>endOfMonth()</code> the same for the month.</li>
</ul>
```qiq
DateTime saturday = DateTime::of(2026, 3, 28, 12, 0, zone: "Europe/London");
print(saturday->addDays(1), saturday->addHours(24));
print(DateTime::of(2026, 1, 31)->addMonths(1)->format("Y-m-d"), DateTime::of(2024, 2, 29)->addYears(1)->format("Y-m-d"));
```
```output
2026-03-29T12:00:00+01:00 2026-03-29T13:00:00+01:00
2026-02-28 2025-02-28
```
<p>When the clocks go forward, a time they skip, such as 01:30 on that Sunday in London, moves forward with them, to 02:30. When they go back, a time that happens twice is the second of the two (in standard time), as PHP chooses; a calendar step from a time with one of the two offsets keeps that offset.</p>

<h2>Comparing</h2>
<p><code>isBefore</code>, <code>isAfter</code>, <code>equals</code>, and <code>compareTo</code> (-1, 0, or 1) compare instants, whatever the zones: 10:00 in London is 11:00 in Paris on the same winter day. Use them rather than <code>==</code>, which for objects asks whether they're the same object. <code>a-&gt;until(b)</code> is the <a href="class.duration.html">Duration</a> from <code>a</code> to <code>b</code>, negative when <code>b</code> is earlier, and <code>a-&gt;since(b)</code> the other way round.</p>

<h2 id="parsing">Parsing</h2>
<p>Without a format, <code>parse</code> reads ISO 8601 and what databases write: <code>2026-11-22</code>, <code>2026-11-22 10:30</code>, <code>2026-11-22T10:30:00.125Z</code>, <code>2026-11-22 10:30:00+01</code> (PostgreSQL), and <code>2026-11-22T10:30:00+01:00</code>. With a format, it takes PHP's <code>DateTime::createFromFormat</code> letters: <code>d</code> <code>j</code> <code>m</code> <code>n</code> <code>M</code> <code>F</code> <code>Y</code> <code>y</code> <code>D</code> <code>l</code> <code>H</code> <code>G</code> <code>h</code> <code>g</code> <code>A</code> <code>a</code> <code>i</code> <code>s</code> <code>u</code> <code>v</code> <code>U</code> <code>O</code> <code>P</code> <code>p</code> <code>e</code> <code>T</code>, and <code>\\</code> before a letter to match it literally. Fields the format doesn't give are those of 1970-01-01 00:00:00, so <code>"Y-m-d"</code> gives midnight. A time with an offset or zone of its own is in it; others are in the <code>zone</code> argument, or the default zone. Text that doesn't fit throws <code>InvalidArgumentException</code>.</p>
```qiq
print(DateTime::parse("2026-11-22T10:30:00+01:00"));
print(DateTime::parse("22/11/2026 3:05 pm", "d/m/Y g:i a", "Europe/Paris"));
print(DateTime::parse("Sun, 22 Nov 2026 10:30:00 +0000", "D, d M Y H:i:s O")->zone());
```
```output
2026-11-22T10:30:00+01:00
2026-11-22T15:05:00+01:00
UTC
```

<h2>Time zone data</h2>
<p>Zones come from the system's zoneinfo files, which the system keeps up to date with each government's changes. Where there are none, as in a <code>FROM scratch</code> or plain Alpine container, qiq uses the copy of the tz database built into it (version 2026c, covering 1970 on), which adds about 90 KB to programs that use <code>DateTime</code>.</p>

<h2>In databases</h2>
<p>A <code>DateTime</code> can be a query's value, and <a href="class.row.html">Row</a>'s <code>getDateTime</code> reads one back; <code>queryAs</code> fills <code>DateTime</code> parameters and properties the same way. A value is sent as its time in the default zone, with the offset for PostgreSQL and SQLite, and without it for MySQL, whose <code>DATETIME</code> has no zone. Read it back in the same default zone and it's the same instant; PostgreSQL's <code>timestamptz</code> keeps the instant in any case.</p>
"""

DURATION_CLASS = """<h1>The Duration class</h1>
<p>A length of time, precise to the microsecond, such as how long until a deadline or how long a request took: what <a href="class.datetime.html">DateTime</a>'s <code>until</code> and <code>since</code> give, and what its <code>add</code> and <code>sub</code> take. A day here is always 24 hours; for calendar days, use <code>DateTime</code>'s <code>addDays</code>.</p>
<h2>Class synopsis</h2>
<div class="synopsis"><code><span class="kw">final class</span> <span class="cls">Duration</span> {<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">static function</span> <span class="fn">ofDays</span>(<span class="type">int</span> n), <span class="fn">ofHours</span>, <span class="fn">ofMinutes</span>, <span class="fn">ofSeconds</span>, <span class="fn">ofMilliseconds</span>, <span class="fn">ofMicroseconds</span>: <span class="cls">Duration</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">totalDays</span>(), <span class="fn">totalHours</span>(), <span class="fn">totalMinutes</span>(), <span class="fn">totalSeconds</span>(), <span class="fn">totalMilliseconds</span>(), <span class="fn">totalMicroseconds</span>(): <span class="type">int</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">plus</span>(<span class="cls">Duration</span> other), <span class="fn">minus</span>(<span class="cls">Duration</span> other), <span class="fn">multipliedBy</span>(<span class="type">int</span> times), <span class="fn">negated</span>(), <span class="fn">abs</span>(): <span class="cls">Duration</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">isNegative</span>(), <span class="fn">isZero</span>(), <span class="fn">equals</span>(<span class="cls">Duration</span> other): <span class="type">bool</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">compareTo</span>(<span class="cls">Duration</span> other): <span class="type">int</span><br>
}</code></div>
<p>The <code>total</code> methods give the length in whole units, rounded toward zero: 90 minutes is 1 hour. Printing a <code>Duration</code> gives its units, as in <code>1d 2h 30m 5.5s</code>; <code>json_encode</code> writes ISO 8601's form, as in <code>"P1DT2H30M5.5S"</code>.</p>
<h2>Example</h2>
```qiq
DateTime start = DateTime::of(2026, 11, 22, 9, 0, zone: "UTC");
DateTime end = DateTime::of(2026, 11, 23, 17, 45, 30, zone: "UTC");
Duration worked = start->until(end);
print(worked, worked->totalHours(), worked->totalMinutes());
Duration timeout = Duration::ofSeconds(30)->multipliedBy(4);
print(timeout, timeout->compareTo(Duration::ofMinutes(2)), start->add(timeout));
print(json_encode([worked, Duration::ofMilliseconds(-1500)]));
```
```output
1d 8h 45m 30s 32 1965
2m 0 2026-11-22T09:02:00+00:00
["P1DT8H45M30S","-PT1.5S"]
```
"""

URL_CLASS = """<h1>The Url class</h1>
<p>A URL, as RFC 3986 defines it: read one apart, build one up without gluing strings together, and resolve links against it as a browser does. It's like Go's <code>net/url</code> <code>URL</code>, but immutable, as <a href="class.datetime.html">DateTime</a> is: the <code>with</code> methods give a new <code>Url</code>, so a base URL shared around can't be changed under you.</p>
```qiq
Url api = Url::parse("https://api.example.com/v1/");
Url search = api->resolve("search")->withQuery("q", "café & crème")->withQuery("page", "2");
print(search);

Url link = Url::parse("https://ada@[2001:db8::1]:8443/files/a%20b.txt?tag=x&tag=y#top");
print(link->scheme(), link->user(), link->host(), link->port(), link->path(), link->fragment());
print(link->query("tag"), link->queryAll("tag"), link->resolve("../img/logo.png"));
```
```output
https://api.example.com/v1/search?q=caf%C3%A9+%26+cr%C3%A8me&page=2
https ada 2001:db8::1 8443 /files/a b.txt top
x ["x", "y"] https://ada@[2001:db8::1]:8443/img/logo.png
```
<h2>Class synopsis</h2>
<div class="synopsis"><code><span class="kw">final class</span> <span class="cls">Url</span> {<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">static function</span> <span class="fn">parse</span>(<span class="type">string</span> text): <span class="cls">Url</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">scheme</span>(), <span class="fn">user</span>(), <span class="fn">password</span>(), <span class="fn">host</span>(), <span class="fn">path</span>(), <span class="fn">rawPath</span>(), <span class="fn">rawQuery</span>(), <span class="fn">fragment</span>(): <span class="type">string</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">port</span>(): <span class="type">int</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">isAbsolute</span>(): <span class="type">bool</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">query</span>(<span class="type">string</span> name): <span class="type">string</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">queryAll</span>(<span class="type">string</span> name), <span class="fn">queryNames</span>(): <span class="type">string[]</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">withQuery</span>(<span class="type">string</span> name, <span class="type">string</span> value), <span class="fn">withAddedQuery</span>(<span class="type">string</span> name, <span class="type">string</span> value), <span class="fn">withoutQuery</span>(<span class="type">string</span> name), <span class="fn">withRawQuery</span>(<span class="type">string</span> raw): <span class="cls">Url</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">withScheme</span>(<span class="type">string</span> scheme), <span class="fn">withUser</span>(<span class="type">string</span> user, <span class="type">string</span> password = <span class="default">null</span>), <span class="fn">withHost</span>(<span class="type">string</span> host), <span class="fn">withPort</span>(<span class="type">int</span> port), <span class="fn">withPath</span>(<span class="type">string</span> path), <span class="fn">withFragment</span>(<span class="type">string</span> fragment): <span class="cls">Url</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">resolve</span>(<span class="type">string</span> reference): <span class="cls">Url</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">equals</span>(<span class="cls">Url</span> other): <span class="type">bool</span><br>
}</code></div>

<h2>Reading it</h2>
<ul>
<li><code>Url::parse(text)</code> reads an absolute URL (<code>https://example.com/a</code>) or a relative one (<code>../a</code>, <code>?page=2</code>). Characters that should have been escaped, such as a space or <code>é</code>, are accepted and escaped. A bad <code>%</code> escape, a port that isn't a number from 0 to 65535, or a host with characters a host can't have throws <code>InvalidArgumentException</code>.</li>
<li>The parts come back unescaped: <code>path()</code> is <code>/a b</code> for <code>/a%20b</code>. <code>rawPath()</code> and <code>rawQuery()</code> are as the URL writes them, escaped.</li>
<li>A part the URL doesn't have is <code>null</code>, and the port is 0. <code>host()</code> is <code>""</code> for an empty host, as in <code>file:///tmp/a</code>, and <code>null</code> for a URL without one, as in <code>mailto:ada@example.com</code>, whose path is <code>ada@example.com</code>. An IPv6 host comes without its brackets.</li>
<li><code>scheme()</code> is in lower case. The host is kept as written, and so is a port that's the scheme's default.</li>
</ul>

<h2>The query</h2>
<p><code>query(name)</code> is a parameter's first value, unescaped, with <code>+</code> read as a space as forms send it, or <code>null</code>; a parameter without <code>=</code>, as in <code>?debug</code>, is <code>""</code>. <code>queryAll(name)</code> gives every value, in order, for repeated parameters such as <code>?tag=a&amp;tag=b</code>, and <code>queryNames()</code> each name once. <code>withQuery(name, value)</code> sets a parameter, in the place of its first value and without its others, or at the end when it's new; <code>withAddedQuery</code> adds another value at the end, and <code>withoutQuery</code> removes all of them. Values are escaped for you, and other parameters are kept as they're written. Values are strings: write <code>withQuery("page", to_string(page))</code> for a number.</p>

<h2>Building one</h2>
<p><code>withScheme</code>, <code>withUser</code>, <code>withHost</code> (an IPv6 address without brackets), <code>withPort</code> (0 for none), <code>withPath</code>, and <code>withFragment</code> each change one part, taking it unescaped. <code>null</code> removes a part. Printing a <code>Url</code>, <code>to_string</code>, and adding it to a string give it in full and correctly escaped, and <code>json_encode</code> writes it as that string. <code>equals</code> compares how two are written; <code>==</code>, on objects, asks whether they're the same object.</p>
```qiq
Url page = Url::parse("http://localhost/")
    ->withScheme("https")
    ->withHost("shop.example.com")
    ->withPath("/search results/é")
    ->withQuery("q", "1 + 1")
    ->withFragment("top");
print(page, page->withoutQuery("q")->withPort(8443));
```
```output
https://shop.example.com/search%20results/%C3%A9?q=1+%2B+1#top https://shop.example.com:8443/search%20results/%C3%A9#top
```

<h2>Resolving links</h2>
<p><code>resolve(reference)</code> is the URL a link to <code>reference</code> on this page leads to, as RFC 3986 (section 5.2) defines it and browsers do: <code>"/b"</code> keeps the scheme and host, <code>"c"</code> replaces the last segment of the path, <code>"../c"</code> goes up a directory, <code>"?x=1"</code> keeps the path, <code>"//cdn.example.com/a"</code> keeps the scheme, and an absolute URL is itself. <code>.</code> and <code>..</code> segments are removed. The HTTP client follows redirects with it.</p>
```qiq
Url base = Url::parse("https://example.com/docs/guide/intro.html?v=2");
foreach (["setup.html", "../api/", "/about", "?v=3", "#install", "//cdn.example.com/x.js", "mailto:help@example.com"] as link) {
    print(base->resolve(link));
}
```
```output
https://example.com/docs/guide/setup.html
https://example.com/docs/api/
https://example.com/about
https://example.com/docs/guide/intro.html?v=3
https://example.com/docs/guide/intro.html?v=2#install
https://cdn.example.com/x.js
mailto:help@example.com
```
<p>Not supported yet: internationalized domain names written in their own script (<code>bücher.example</code>), which aren't converted to their <code>xn--</code> form.</p>
"""

PROCESSRESULT_CLASS = """<h1>The ProcessResult class</h1>
<p>What a command run by <a href="function.exec.html">exec</a> or <a href="function.shell.html">shell</a> did: its exit status and its output. Import it from <code>std/os</code> to name the type.</p>
<h2>Class synopsis</h2>
<div class="synopsis"><code><span class="kw">class</span> <span class="cls">ProcessResult</span> {<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public readonly</span> <span class="type">int</span> exitCode;<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public readonly</span> <span class="type">string</span> stdout;<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">public readonly</span> <span class="type">string</span> stderr;<br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">ok</span>(): <span class="type">bool</span><br>
&nbsp;&nbsp;&nbsp;&nbsp;<span class="kw">function</span> <span class="fn">check</span>(): <span class="cls">ProcessResult</span><br>
}</code></div>
<table>
<thead><tr><th>Member</th><th>Description</th></tr></thead>
<tbody>
<tr><td><code>exitCode</code></td><td>The exit status: 0 for success, 1 to 255 for failure, and 128 plus the signal's number if a signal ended the command</td></tr>
<tr><td><code>stdout</code></td><td>Everything the command wrote to standard output, including the final newline; <code>trim</code> removes it</td></tr>
<tr><td><code>stderr</code></td><td>Everything it wrote to standard error</td></tr>
<tr><td><code>ok()</code></td><td>Whether <code>exitCode</code> is 0</td></tr>
<tr><td><code>check()</code></td><td>Returns the result if the command succeeded, and otherwise throws <code>ProcessException</code>, with the exit status as its code and the error output in its message</td></tr>
</tbody>
</table>
<h2>Example</h2>
<pre class="code">import exec, ProcessResult from "std/os";

// When failing is an expected answer: grep exits with 1 when nothing matches.
ProcessResult found = exec(["grep", "-c", "TODO", "main.qiq"]);
if (found-&gt;exitCode &lt;= 1) {
    print("TODOs:", trim(found-&gt;stdout));
}

// When the command must succeed.
string commit = trim(exec(["git", "rev-parse", "HEAD"])-&gt;check()-&gt;stdout);</pre>
"""

# Pages for standard library classes: (href, title, summary, html).
CLASS_PAGES = [
    ("class.file.html", "The File class", "Reading and writing files a line or a chunk at a time, including standard input and output.", FILE_CLASS),
    ("class.response.html", "The Response class", "The result of an HTTP request: status, headers, and body.", RESPONSE_CLASS),
    ("class.request.html", "The Request class", "A request received by the HTTP server: method, path, headers, and body.", REQUEST_CLASS),
    ("class.responsewriter.html", "The ResponseWriter class", "Builds the HTTP server's response to a request.", RESPONSEWRITER_CLASS),
    ("class.router.html", "The Router class", "Sends each HTTP request to a handler by method and path.", ROUTER_CLASS),
    ("class.socket.html", "The Socket class", "TCP connections, optionally encrypted with TLS, for talking to network services.", SOCKET_CLASS),
    ("class.url.html", "The Url class", "URLs: reading them apart, building them with their parts escaped, and resolving links.", URL_CLASS),
    ("class.datetime.html", "The DateTime class", "An instant in a time zone: calendar arithmetic, zones and daylight saving, parsing, and formatting.", DATETIME_CLASS),
    ("class.duration.html", "The Duration class", "A length of time, from DateTime's until and since, for its add and sub.", DURATION_CLASS),
    ("class.database.html", "The Database class", "SQL databases (SQLite, PostgreSQL, MySQL, and MariaDB): running statements with values for their placeholders, queries, and transactions.", DATABASE_CLASS),
    ("class.row.html", "The Row class", "One row of a query's result, read by column name with typed getters.", ROW_CLASS),
    ("class.cursor.html", "The Cursor class", "A query's rows, read a batch at a time, for results too big to hold in memory.", CURSOR_CLASS),
    ("class.csvreader.html", "The CsvReader class", "Reads a CSV file a record at a time.", CSVREADER_CLASS),
    ("class.csvwriter.html", "The CsvWriter class", "Writes a CSV file a row at a time.", CSVWRITER_CLASS),
    ("class.regexmatch.html", "The RegexMatch class", "A match of a regular expression: its text, position, and groups.", REGEXMATCH_CLASS),
    ("class.processresult.html", "The ProcessResult class", "The exit status and output of a command run by exec or shell.", PROCESSRESULT_CLASS),
    ("exception-classes.html", "Exception classes", "The exception classes thrown by the language and the standard library.", EXCEPTION_CLASSES),
]
