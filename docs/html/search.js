const SEARCH_INDEX = [{"title": "Introduction", "url": "introduction.html", "kind": "Getting started"}, {"title": "Installing and running", "url": "installation.html", "kind": "Getting started"}, {"title": "Basic syntax", "url": "basic-syntax.html", "kind": "Language reference"}, {"title": "Types", "url": "types.html", "kind": "Language reference"}, {"title": "Variables", "url": "variables.html", "kind": "Language reference"}, {"title": "Operators", "url": "operators.html", "kind": "Language reference"}, {"title": "Control structures", "url": "control-structures.html", "kind": "Language reference"}, {"title": "Functions", "url": "functions.html", "kind": "Language reference"}, {"title": "Anonymous functions", "url": "anonymous-functions.html", "kind": "Language reference"}, {"title": "Strings and chars", "url": "strings.html", "kind": "Language reference"}, {"title": "Arrays", "url": "arrays.html", "kind": "Language reference"}, {"title": "Maps", "url": "maps.html", "kind": "Language reference"}, {"title": "Classes and objects", "url": "classes.html", "kind": "Language reference"}, {"title": "Static members", "url": "static.html", "kind": "Language reference"}, {"title": "Interfaces", "url": "interfaces.html", "kind": "Language reference"}, {"title": "Exceptions", "url": "exceptions.html", "kind": "Language reference"}, {"title": "Modules", "url": "modules.html", "kind": "Language reference"}, {"title": "Packages", "url": "packages.html", "kind": "Language reference"}, {"title": "Memory management", "url": "memory.html", "kind": "Language reference"}, {"title": "Printing", "url": "printing.html", "kind": "Language reference"}, {"title": "Standard library", "url": "stdlib.html", "kind": "Standard library"}, {"title": "General", "url": "ref.general.html", "kind": "Standard library"}, {"title": "len", "url": "function.len.html", "kind": "The length of a string, array, or map"}, {"title": "gc_collect_cycles", "url": "function.gc-collect-cycles.html", "kind": "Free garbage reference cycles now"}, {"title": "Strings", "url": "ref.strings.html", "kind": "Standard library"}, {"title": "substr", "url": "function.substr.html", "kind": "Return part of a string"}, {"title": "strpos", "url": "function.strpos.html", "kind": "Find the position of the first occurrence of a substring"}, {"title": "str_contains", "url": "function.str-contains.html", "kind": "Determine if a string contains a substring"}, {"title": "str_starts_with", "url": "function.str-starts-with.html", "kind": "Check if a string starts with a given substring"}, {"title": "str_ends_with", "url": "function.str-ends-with.html", "kind": "Check if a string ends with a given substring"}, {"title": "str_replace", "url": "function.str-replace.html", "kind": "Replace all occurrences of a substring"}, {"title": "htmlspecialchars", "url": "function.htmlspecialchars.html", "kind": "Escape text for HTML"}, {"title": "sprintf", "url": "function.sprintf.html", "kind": "Format values into a string"}, {"title": "str_repeat", "url": "function.str-repeat.html", "kind": "Repeat a string"}, {"title": "strtoupper", "url": "function.strtoupper.html", "kind": "Make a string uppercase"}, {"title": "strtolower", "url": "function.strtolower.html", "kind": "Make a string lowercase"}, {"title": "ucfirst", "url": "function.ucfirst.html", "kind": "Make a string's first character uppercase"}, {"title": "strrev", "url": "function.strrev.html", "kind": "Reverse a string"}, {"title": "trim", "url": "function.trim.html", "kind": "Strip characters from the beginning and end of a string"}, {"title": "explode", "url": "function.explode.html", "kind": "Split a string by a separator"}, {"title": "implode", "url": "function.implode.html", "kind": "Join array elements with a string"}, {"title": "string_to_bytes", "url": "function.string-to-bytes.html", "kind": "Get a string's UTF-8 bytes"}, {"title": "bytes_to_string", "url": "function.bytes-to-string.html", "kind": "Decode UTF-8 bytes into a string"}, {"title": "Arrays", "url": "ref.arrays.html", "kind": "Standard library"}, {"title": "range", "url": "function.range.html", "kind": "An array of numbers or characters from start to end"}, {"title": "array_push", "url": "function.array-push.html", "kind": "Add values to the end of an array"}, {"title": "array_pop", "url": "function.array-pop.html", "kind": "Remove and return the last element"}, {"title": "array_shift", "url": "function.array-shift.html", "kind": "Remove and return the first element"}, {"title": "array_unshift", "url": "function.array-unshift.html", "kind": "Add values to the front of an array"}, {"title": "array_splice", "url": "function.array-splice.html", "kind": "Remove or replace a range of elements"}, {"title": "Maps", "url": "ref.maps.html", "kind": "Standard library"}, {"title": "map_key_exists", "url": "function.map-key-exists.html", "kind": "Check whether a map has a key"}, {"title": "map_keys", "url": "function.map-keys.html", "kind": "The keys of a map, as an array"}, {"title": "map_values", "url": "function.map-values.html", "kind": "The values of a map, as an array"}, {"title": "in_map", "url": "function.in-map.html", "kind": "Check whether a map has a value"}, {"title": "unset", "url": "function.unset.html", "kind": "Remove an entry from a map"}, {"title": "Files", "url": "ref.files.html", "kind": "Standard library"}, {"title": "file_get_contents", "url": "function.file-get-contents.html", "kind": "Read an entire file into a string"}, {"title": "file_put_contents", "url": "function.file-put-contents.html", "kind": "Write a string to a file"}, {"title": "file", "url": "function.file.html", "kind": "Read a file into an array of lines"}, {"title": "file_exists", "url": "function.file-exists.html", "kind": "Check whether a file or directory exists"}, {"title": "is_file", "url": "function.is-file.html", "kind": "Check whether a path is a regular file"}, {"title": "is_dir", "url": "function.is-dir.html", "kind": "Check whether a path is a directory"}, {"title": "scandir", "url": "function.scandir.html", "kind": "List the names in a directory"}, {"title": "file_get_bytes", "url": "function.file-get-bytes.html", "kind": "Read an entire file as raw bytes"}, {"title": "file_put_bytes", "url": "function.file-put-bytes.html", "kind": "Write raw bytes to a file"}, {"title": "Date and time", "url": "ref.datetime.html", "kind": "Standard library"}, {"title": "time", "url": "function.time.html", "kind": "The current timestamp"}, {"title": "microtime", "url": "function.microtime.html", "kind": "The current time with microseconds"}, {"title": "hrtime", "url": "function.hrtime.html", "kind": "A monotonic clock in nanoseconds"}, {"title": "date", "url": "function.date.html", "kind": "Format a timestamp in the current timezone"}, {"title": "gmdate", "url": "function.gmdate.html", "kind": "Format a timestamp in UTC"}, {"title": "mktime", "url": "function.mktime.html", "kind": "Build a timestamp from a local date and time"}, {"title": "gmmktime", "url": "function.gmmktime.html", "kind": "Build a timestamp from a UTC date and time"}, {"title": "checkdate", "url": "function.checkdate.html", "kind": "Check whether a date exists"}, {"title": "strtotime", "url": "function.strtotime.html", "kind": "Parse an ISO 8601 date"}, {"title": "date_default_timezone_set", "url": "function.date-default-timezone-set.html", "kind": "Set the timezone for local times"}, {"title": "date_default_timezone_get", "url": "function.date-default-timezone-get.html", "kind": "The current timezone's name"}, {"title": "sleep", "url": "function.sleep.html", "kind": "Pause for whole seconds"}, {"title": "usleep", "url": "function.usleep.html", "kind": "Pause for microseconds"}, {"title": "JSON", "url": "ref.json.html", "kind": "Standard library"}, {"title": "json_encode", "url": "function.json-encode.html", "kind": "Convert a value to JSON"}, {"title": "json_decode", "url": "function.json-decode.html", "kind": "Convert JSON to a typed value"}, {"title": "HTTP", "url": "ref.http.html", "kind": "Standard library"}, {"title": "http_request", "url": "function.http-request.html", "kind": "Send an HTTP request"}, {"title": "http_get", "url": "function.http-get.html", "kind": "Send a GET request"}, {"title": "http_post", "url": "function.http-post.html", "kind": "Send a POST request"}, {"title": "urlencode", "url": "function.urlencode.html", "kind": "Percent-encode text for a query string"}, {"title": "rawurlencode", "url": "function.rawurlencode.html", "kind": "Percent-encode text for a URL path"}, {"title": "urldecode", "url": "function.urldecode.html", "kind": "Decode percent-encoded text"}, {"title": "rawurldecode", "url": "function.rawurldecode.html", "kind": "Decode a percent-encoded URL path"}, {"title": "http_build_query", "url": "function.http-build-query.html", "kind": "Build a query string from a map"}, {"title": "HTTP server", "url": "ref.http-server.html", "kind": "Standard library"}, {"title": "http_serve", "url": "function.http-serve.html", "kind": "Run an HTTP server"}, {"title": "http_file_server", "url": "function.http-file-server.html", "kind": "A handler that serves files from a directory"}, {"title": "http_strip_prefix", "url": "function.http-strip-prefix.html", "kind": "Remove a prefix from the path before calling a handler"}, {"title": "http_log", "url": "function.http-log.html", "kind": "Log each request to stderr"}, {"title": "http_content_type", "url": "function.http-content-type.html", "kind": "The Content-Type for a file name"}, {"title": "Program", "url": "ref.program.html", "kind": "Standard library"}, {"title": "argv", "url": "function.argv.html", "kind": "The command-line arguments"}, {"title": "exit", "url": "function.exit.html", "kind": "End the program with an exit status"}, {"title": "getenv", "url": "function.getenv.html", "kind": "Read an environment variable"}, {"title": "Conversions", "url": "ref.conversions.html", "kind": "Standard library"}, {"title": "to_int", "url": "function.to-int.html", "kind": "Convert a value to an int"}, {"title": "to_double", "url": "function.to-double.html", "kind": "Convert a value to a double"}, {"title": "to_byte", "url": "function.to-byte.html", "kind": "Convert an int to a byte"}, {"title": "to_string", "url": "function.to-string.html", "kind": "Convert a value to a string"}, {"title": "Output", "url": "ref.output.html", "kind": "Standard library"}, {"title": "print", "url": "function.print.html", "kind": "Write values to standard output"}, {"title": "The File class", "url": "class.file.html", "kind": "Standard library"}, {"title": "The Response class", "url": "class.response.html", "kind": "Standard library"}, {"title": "The Request class", "url": "class.request.html", "kind": "Standard library"}, {"title": "The ResponseWriter class", "url": "class.responsewriter.html", "kind": "Standard library"}, {"title": "The Router class", "url": "class.router.html", "kind": "Standard library"}, {"title": "The Socket class", "url": "class.socket.html", "kind": "Standard library"}, {"title": "Exception classes", "url": "exception-classes.html", "kind": "Standard library"}];
// Search box: filters SEARCH_INDEX (pages and functions, written by
// generate.py above this script) as you type. Arrow keys move, Enter opens.
(function () {
  const input = document.getElementById("search");
  const list = document.getElementById("search-results");
  if (!input || !list) return;
  let selected = -1;

  function score(entry, query) {
    const title = entry.title.toLowerCase();
    if (title === query) return 0;
    if (title.startsWith(query)) return 1;
    if (title.includes(query)) return 2;
    if (entry.kind.toLowerCase().includes(query)) return 3;
    return -1;
  }

  function render() {
    const query = input.value.trim().toLowerCase();
    list.innerHTML = "";
    selected = -1;
    if (!query) {
      list.hidden = true;
      return;
    }
    const matches = SEARCH_INDEX.map((entry) => [score(entry, query), entry])
      .filter(([rank]) => rank >= 0)
      .sort((a, b) => a[0] - b[0] || a[1].title.localeCompare(b[1].title))
      .slice(0, 12);
    for (const [, entry] of matches) {
      const item = document.createElement("li");
      const link = document.createElement("a");
      link.href = entry.url;
      link.textContent = entry.title;
      const kind = document.createElement("span");
      kind.className = "kind";
      kind.textContent = entry.kind;
      link.appendChild(kind);
      item.appendChild(link);
      list.appendChild(item);
    }
    if (!matches.length) {
      const item = document.createElement("li");
      item.innerHTML = '<a>No matches</a>';
      list.appendChild(item);
    }
    list.hidden = false;
  }

  function move(step) {
    const links = list.querySelectorAll("a[href]");
    if (!links.length) return;
    if (selected >= 0) links[selected].classList.remove("selected");
    selected = (selected + step + links.length) % links.length;
    links[selected].classList.add("selected");
  }

  input.addEventListener("input", render);
  input.addEventListener("keydown", (event) => {
    if (event.key === "ArrowDown") { move(1); event.preventDefault(); }
    else if (event.key === "ArrowUp") { move(-1); event.preventDefault(); }
    else if (event.key === "Enter") {
      const links = list.querySelectorAll("a[href]");
      const target = links[selected >= 0 ? selected : 0];
      if (target) window.location = target.href;
    } else if (event.key === "Escape") {
      input.value = "";
      render();
    }
  });
  document.addEventListener("click", (event) => {
    if (!event.target.closest(".search")) list.hidden = true;
  });
  // "/" focuses the search box, as on many documentation sites.
  document.addEventListener("keydown", (event) => {
    if (event.key === "/" && document.activeElement !== input) {
      event.preventDefault();
      input.focus();
    }
  });
})();
