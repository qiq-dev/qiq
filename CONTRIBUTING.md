# Contributing to qiq

This guide covers how qiq works inside and how to work on it. To learn the language itself, read the manual at [qiq-lang.com](https://qiq-lang.com/). For build requirements, see the [README](README.md#building-from-source).

## How qiq builds executables

Building happens entirely inside the `qiq` process and never runs another program. The one exception is downloading packages, which uses `git`.

1. The compiler (`src/compiler.rs`) turns the qiq source into LLVM IR.
2. LLVM is linked into qiq. It loads the IR together with the qiq runtime, which `build.rs` precompiled from `stdlib/runtime/*.c` to LLVM bitcode, and optimizes the two as one module. That lets runtime helpers such as checked division be inlined into the program. LLVM then emits an x86-64 object file. Everything except `main` is made internal first, because the module is the whole program. The optimizer can then remove any runtime and standard library code the program doesn't use, so a hello-world executable is about 11 KB.
3. [Wild](https://github.com/davidlattimore/wild), a linker written in Rust that is also linked into qiq, links the object statically against musl libc. `build.rs` built musl from the vendored source and embedded it in qiq.

Programs are compiled with `-O2` for the `x86_64-unknown-linux-musl` target. `qiq build hello.qiq --emit-llvm` writes the generated IR to `hello.ll`, which helps when debugging code generation.

## The standard library and runtime

```
stdlib/
  string.qiq        declarations of the string functions
  exceptions.qiq    the exception classes
  file.qiq          the file functions and the File class
  date.qiq          the date and time functions
  memory.qiq        gc_collect_cycles
  program.qiq       argv, exit, and getenv
  net.qiq           the Socket class
  http.qiq          the HTTP client, Response, and URL encoding (written in qiq)
  server.qiq        the HTTP server and router (written in qiq)
  runtime/          the C runtime linked into every program
    qiq_runtime.h   shared declarations, and the exception kinds the runtime raises
    core.c          raising exceptions and reporting uncaught ones
    memory.c        allocation, reference counting, and the cycle collector
    map.c           maps
    strings.c       UTF-8 string support and the string functions
    arithmetic.c    checked integer division, remainder, power, and shifts
    file.c          the file functions
    array.c         array allocation, length, and bounds checks
    print.c         printing arrays, maps, objects, and several values
    conversions.c   to_int, to_double, to_byte, and to_string
    time.c          the date and time functions
    json.c          json_encode and json_decode
    program.c       argv, exit, and getenv
    net.c           sockets, TLS, and the server's listening and worker processes
```

Each `.qiq` file in `stdlib/` is a module. There are two ways to write a standard library function:

- Write it in qiq, like `str_starts_with`.
- Declare it with `extern function name(params): type;` and implement it in C as `qiq_stdlib_<name>`.

Only the standard library may use `extern function`. Extern functions can take and return only `int`, `double`, `bool`, `byte`, `string` and `char`, and one-dimensional arrays of those.

The standard library and the runtime are embedded in the compiler when it's built, so `qiq` works from any directory. While you work on the `.qiq` files, set `QIQ_STDLIB=stdlib` to load them from disk without rebuilding:

```sh
QIQ_STDLIB=stdlib ./target/release/qiq run tests/stdlib/strings.qiq
```

If you change the C runtime, you still need to run `cargo build`.

Some notes on specific parts of the runtime:

- **Maps** (`map.c`) use the same layout as Python's dict. Entries are kept in an array in insertion order, with an open-addressing index of positions into that array. Removed entries stay in place, marked dead, until the array is rebuilt when it fills up.
- **JSON** (`json.c`) parses in one pass straight into typed values. It's driven by a type descriptor and by a table of each class's public properties, which the compiler generates. It was checked against PHP's `json_encode` and `json_decode` on thousands of random values and corrupted documents.
- **Sockets** (`net.c`) are non-blocking, and every operation waits at most the timeout, using `poll`. TLS 1.2 and 1.3 come from Mbed TLS 3.6, which `build.rs` builds from `vendor/` into a static library that is linked only into programs that use sockets. Certificates are verified against the system's CA certificates, found the way Go finds them. `SSL_CERT_FILE` and `SSL_CERT_DIR` override that.

## Memory management

Strings, arrays, maps and objects are reference counted. Each one has a 16-byte header, holding the reference count and the value's kind, just before the address the program uses. A string is therefore still a plain `char *`, and an array still points at its length. See `stdlib/runtime/memory.c`.

The compiler tracks who owns each value:

- Function results and new values are owned temporaries.
- Values read from properties, elements and maps are retained.
- Storing a value either hands over a temporary or retains the value.
- Parameters that a function doesn't reassign are borrowed, which costs nothing.
- Temporaries are released at the end of each statement, or before an exception jumps to a handler.

Each class gets a generated table that records which of its fields hold references and which of those can be part of a cycle. The runtime uses it to release an object's fields.

Cycles are freed by Bacon and Rajan's synchronous cycle collector:

- When a count drops to a value above zero, the value is recorded as a possible root.
- After 10,000 possible roots, the next allocation runs the collector.
- The collector subtracts the references that come from inside the graph below the roots, then frees whatever has no references left.

Only values that can be part of a cycle are tracked: objects whose class has a field that can hold an object, and arrays and maps that can hold objects. When a collection finds little garbage, the next one waits longer. Graph walks use an explicit stack instead of recursion.

Set `QIQ_DEBUG_MEMORY=1` to check a program for memory bugs. Freed values are kept and marked, so a program that uses one stops with an error. At exit, after a final collection, the program prints how many values were allocated and freed:

```sh
QIQ_DEBUG_MEMORY=1 ./target/release/qiq run tests/memory.qiq
# 75005
# qiq memory: 84996 allocated, 84996 freed, 0 still referenced
```

Run this after any change to code generation or the runtime that touches reference counts. It should end with nothing still referenced.

## Closures

Before the compiler emits a function, it finds the variables that the function's anonymous functions use (`captured_names`). It keeps those variables in heap cells, which are one-field objects, instead of in stack slots. The function and its closures then read and write the same storage, and that storage outlives the call.

A function value is an object. It holds a pointer to the anonymous function's code, emitted as `@qiq.closure.N` with the object as its first argument. It also holds the cells it captured, or `this`. Cells and closures are reference-counted objects, so the cycle collector frees the cycles that closures often form. When no function type is expected, an arrow function's return type comes from emitting its expression once in a scratch context. See `tests/closures.qiq`.

## Modules and packages

- `src/modfile.rs` parses `qiq.mod` and orders versions. Version selection is Go's minimal version selection.
- `src/fetch.rs` downloads modules and checks hashes. `qiq get` lists versions with `git ls-remote` and clones the tag from `https://<module path>`. It records each version's hash in `qiq.sum`, computed like Go's `h1:` hashes.
- The compiler's `Loader` resolves imports.

Downloaded modules are cached in `~/.cache/qiq/mod/` (or `$XDG_CACHE_HOME/qiq/mod`). To test against a clean cache, set `QIQ_MODCACHE` to an empty directory.

## Tests

```sh
cargo test
```

`cargo test` runs the unit tests in `src/compiler.rs`, `src/modfile.rs` and `src/fetch.rs`. Programs in `tests/` exercise one feature each. Run them with `./target/release/qiq run tests/<name>.qiq` from the repository root. When you add a language feature, add a test program for it, and add a unit test where the behaviour can be checked inside the compiler.

## Documentation

The manual at [qiq-lang.com](https://qiq-lang.com/) lives in its own repository. When you change the language or the standard library, update the manual too.

## Releasing

The version comes from `version` in `Cargo.toml`. It's compiled into the binary, so `qiq version` reports it. New `qiq.mod` files get their `qiq X.Y` line from it, and a module that requires a newer qiq than the running compiler is rejected. The Git tag doesn't affect the binary, so the tag and `Cargo.toml` must match.

1. Set `version` in `Cargo.toml` to the new version.
2. Run `cargo build --release`, which also updates `Cargo.lock`. Commit both files.
3. Check that `./target/release/qiq version` prints the new version.
4. Tag the commit and push the tag:
   ```sh
   git tag v0.1.0
   git push origin v0.1.0
   ```
5. Create a GitHub release from the tag and attach `target/release/qiq`.

Versions follow semantic versioning (MAJOR.MINOR.PATCH). Until 1.0:

- Bump the patch number (0.1.0 → 0.1.1) for bug fixes.
- Bump the minor number (0.1 → 0.2) for new language or standard library features, or for breaking changes.

`qiq.mod` checks only the major and minor numbers, so a module that uses a new feature can declare `qiq 0.2`. An older compiler then rejects it with a clear message.
