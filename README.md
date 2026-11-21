# qiq

qiq (pronounced "quick") is a statically typed programming language with a syntax familiar to PHP developers. Its compiler, written in Rust, uses LLVM to turn qiq programs into optimized native executables. The executables are statically linked, so they run on any x86-64 Linux machine without qiq or any libraries installed.

```qiq
class Greeter {
    private string greeting;

    public function __construct(string greeting = "Hello") {
        this->greeting = greeting;
    }

    public function greet(string name): string {
        return sprintf("%s, %s!", this->greeting, name);
    }
}

Greeter greeter = new Greeter();
foreach (["Ada", "Linus"] as name) {
    print(greeter->greet(name));
}
```

```sh
$ qiq run hello.qiq
Hello, Ada!
Hello, Linus!
```

The language has classes with inheritance, interfaces, static members, anonymous functions and exceptions. It also has modules and packages, and memory is managed automatically with reference counting. The standard library covers strings, files, dates, JSON, an HTTP client and an HTTP server.

**Documentation:** [qiq-lang.com](https://qiq-lang.com/) has the manual and the standard library reference.

## Install

Download `qiq` from the [Releases](../../releases) page, make it executable, and put it on your `PATH`:

```sh
chmod +x qiq
sudo mv qiq /usr/local/bin/
qiq version
```

The `qiq` binary runs on x86-64 Linux with glibc and needs nothing else. LLVM, the qiq runtime, musl libc, Mbed TLS and a linker are all built into it. The programs it builds don't even need glibc. Programs that make HTTPS requests read the system's CA certificates (the `ca-certificates` package) when they run.

## Usage

```sh
qiq run hello.qiq [arguments...]     # compile and run, like `go run` or `php hello.php`
qiq build hello.qiq                  # write the executable ./hello
qiq build hello.qiq -o app           # choose the output name
qiq build hello.qiq --emit-llvm      # also write the generated LLVM IR (app.ll)
qiq check hello.qiq                  # report compile errors without building
qiq serve public --port 8080         # serve a directory over HTTP, like `php -S`
qiq mod init github.com/you/app      # make this directory a module (writes qiq.mod)
qiq get github.com/ada/web           # add a dependency (downloaded with git)
qiq mod tidy                         # require exactly the modules that are imported
qiq help
```

## Building from source

You need:

- Rust 1.99 or newer (for example via [rustup](https://rustup.rs))
- the LLVM 18 development libraries (`llvm-config-18`)
- clang, llvm-ar, make, tar and bzip2

```sh
cargo build --release        # builds ./target/release/qiq
```

The first build also compiles musl and Mbed TLS from the tarballs in `vendor/`, which adds about a minute. Later builds reuse them.

## Tests

```sh
cargo test                                      # compiler, qiq.mod and package-fetching unit tests
./target/release/qiq run tests/classes.qiq      # any example program in tests/
```

The programs in `tests/` cover one feature each, and the ones in `tests/stdlib/` cover the standard library. Run them from the repository root, since some read and write files there. `tests/stdlib/http.qiq` needs internet access.

## Repository layout

```
src/
  qiq.rs        the command-line tool: run, build, check, serve, mod, get
  compiler.rs   lexer, parser, type checker, and LLVM code generation
  modfile.rs    qiq.mod parsing and version ordering
  fetch.rs      downloading modules and checking qiq.sum
build.rs        builds the C runtime, musl and Mbed TLS, which get embedded in qiq
stdlib/         the standard library: qiq modules, plus the C runtime in stdlib/runtime/
tests/          example programs, one per feature
vendor/         musl and Mbed TLS source tarballs
```

[CONTRIBUTING.md](CONTRIBUTING.md) explains how the compiler, the runtime and the standard library fit together, and how to cut a release.

## License

The qiq compiler is licensed under either of these, at your option:

- the [MIT license](LICENSE-MIT)
- the [Apache License, Version 2.0](LICENSE-APACHE)

The standard library and runtime in `stdlib/` are compiled into every program qiq builds, so they use the [Zero-Clause BSD license](stdlib/LICENSE). It has no attribution requirement, so your programs carry no license obligations from qiq itself.

Unless you say otherwise, any contribution you submit for inclusion in qiq is licensed the same way: dual MIT and Apache-2.0 for the compiler, and 0BSD for `stdlib/`.

### Third-party code

The `qiq` binary includes:

- [LLVM](https://llvm.org/), under the Apache License 2.0 with LLVM Exceptions
- [Wild](https://github.com/davidlattimore/wild), under MIT or Apache-2.0
- [musl libc](https://musl.libc.org/) 1.2.5, under the MIT license (`vendor/musl-1.2.5.tar.gz`, file `COPYRIGHT`)
- [Mbed TLS](https://github.com/Mbed-TLS/mbedtls) 3.6.8, under Apache-2.0 or GPL-2.0-or-later (`vendor/mbedtls-3.6.8.tar.bz2`, file `LICENSE`)
- the Rust crates listed in `Cargo.lock`, each under its own license

Programs built with qiq are statically linked against musl, and programs that use sockets also include Mbed TLS. Both licenses are permissive. If you distribute such a program, their notices apply to it, just as glibc's do to an ordinary C program.
