//! Prepares everything qiq embeds so that compiling qiq programs needs no
//! other tools:
//!
//! - musl libc, built from `vendor/musl-1.2.5.tar.gz` with qiq's memcpy
//!   (`vendor/musl-x86_64-memcpy.c`), which compiled programs link against
//!   statically;
//! - Mbed TLS, built from `vendor/mbedtls-3.6.8.tar.bz2` (sha256
//!   bc54b818e87f49c0b3281dd85d24adaede0e63b4e822e3fbd49c005a38b9595b) as a
//!   static library that programs using HTTPS link against;
//! - SQLite, built from `vendor/sqlite-autoconf-3540100.tar.gz` (SHA3-256
//!   b6ced585c750916b1052667239a12c25a7ff34e7f06e1e6e01ecb0427657d9c4, as
//!   sqlite.org lists it) as a static library for programs using std/sql;
//! - PCRE2, built from `vendor/pcre2-10.49.tar.bz2` (sha256
//!   53c156e1ba416a20da8e65395daa132da0d80e76910424caca3fcdae7831d384, signed
//!   by the release key in PCRE2's SECURITY.md) as a static library for
//!   programs using std/regex;
//! - the C runtime (`stdlib/runtime/*.c`), precompiled to LLVM bitcode so it
//!   is optimized together with each program;
//! - LLVM 18, linked into qiq statically (only the components qiq uses).
//!
//! Building qiq needs LLVM 18 development libraries (`llvm-config-18`),
//! clang, llvm-ar, make, tar, and bzip2. Override the tools with the
//! LLVM_CONFIG, CLANG, and LLVM_AR environment variables.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const MUSL: &str = "musl-1.2.5";
const MBEDTLS: &str = "mbedtls-3.6.8";
const SQLITE: &str = "sqlite-autoconf-3540100";
const PCRE2: &str = "pcre2-10.49";
/// PCRE2's options for the programs qiq builds: UTF-8 with Unicode
/// properties, and the JIT compiler, which regex.c uses when the system
/// allows executable memory.
const PCRE2_OPTIONS: &[&str] = &["-DHAVE_CONFIG_H", "-DPCRE2_CODE_UNIT_WIDTH=8", "-DPCRE2_STATIC", "-DSUPPORT_UNICODE", "-DSUPPORT_JIT"];
/// PCRE2's files for 8-bit (UTF-8) strings, from its NON-AUTOTOOLS-BUILD.
const PCRE2_SOURCES: &[&str] = &[
    "pcre2_auto_possess.c",
    "pcre2_chkdint.c",
    "pcre2_chartables.c",
    "pcre2_compile.c",
    "pcre2_compile_cgroup.c",
    "pcre2_compile_class.c",
    "pcre2_config.c",
    "pcre2_context.c",
    "pcre2_convert.c",
    "pcre2_dfa_match.c",
    "pcre2_error.c",
    "pcre2_extuni.c",
    "pcre2_find_bracket.c",
    "pcre2_jit_compile.c",
    "pcre2_maketables.c",
    "pcre2_match.c",
    "pcre2_match_data.c",
    "pcre2_match_next.c",
    "pcre2_newline.c",
    "pcre2_ord2utf.c",
    "pcre2_pattern_info.c",
    "pcre2_script_run.c",
    "pcre2_serialize.c",
    "pcre2_string_utils.c",
    "pcre2_study.c",
    "pcre2_substitute.c",
    "pcre2_substring.c",
    "pcre2_tables.c",
    "pcre2_ucd.c",
    "pcre2_valid_utf.c",
    "pcre2_xclass.c",
];
/// SQLite's compile-time options: no threads (each server worker is a
/// process) or loadable extensions (programs are static), sub-second sleeps
/// while waiting for locks, double-quoted strings only as identifiers, and
/// the math functions and full-text search.
const SQLITE_OPTIONS: &[&str] = &[
    "-DSQLITE_THREADSAFE=0",
    "-DSQLITE_OMIT_LOAD_EXTENSION",
    "-DSQLITE_DEFAULT_MEMSTATUS=0",
    "-DSQLITE_DQS=0",
    "-DSQLITE_OMIT_DEPRECATED",
    "-DSQLITE_OMIT_SHARED_CACHE",
    "-DSQLITE_LIKE_DOESNT_MATCH_BLOBS",
    "-DSQLITE_ENABLE_MATH_FUNCTIONS",
    "-DSQLITE_ENABLE_FTS5",
    "-DHAVE_USLEEP=1",
];
/// Replaces musl's x86-64 assembly memcpy, which is slow for short copies.
const MUSL_MEMCPY: &str = "vendor/musl-x86_64-memcpy.c";
/// Patches musl's DNS resolver (src/network/res_msend.c) to wait for answers
/// through a hook the runtime sets when tasks start (std/async, see fiber.c),
/// so a lookup lets other tasks run instead of blocking them all: its one
/// poll() call goes through `__qiq_poll_hook` when that's set.
const MUSL_RESOLVER_POLL: &str = "if (poll(pfd, nqueries+1, t1+retry_interval-t2) <= 0) continue;";
const MUSL_RESOLVER_HOOKED: &str = "if ((__qiq_poll_hook ? __qiq_poll_hook : poll)(pfd, nqueries+1, t1+retry_interval-t2) <= 0) continue;";
const MUSL_RESOLVER_HOOK: &str = "\n/* qiq: set by the runtime while tasks run (see build.rs). */\nint (*__qiq_poll_hook)(struct pollfd *, nfds_t, int) = 0;\n";
const RUNTIME_SOURCES: &[&str] = &[
    "core.c",
    "fiber.c",
    "memory.c",
    "map.c",
    "strings.c",
    "arithmetic.c",
    "file.c",
    "array.c",
    "print.c",
    "conversions.c",
    "time.c",
    "json.c",
    "program.c",
    "net.c",
    "crypto.c",
    "sql.c",
    "postgres.c",
    "mysql.c",
    "tz.c",
    "url.c",
    "process.c",
    "regex.c",
    "math.c",
    "csv.c",
];
/// LLVM components qiq uses: reading IR, linking modules, optimizing, and
/// emitting x86-64 object code (plus what inkwell references).
const LLVM_COMPONENTS: &[&str] = &[
    "core",
    "irreader",
    "linker",
    "passes",
    "target",
    "bitwriter",
    "executionengine",
    "x86codegen",
    "x86asmparser",
    "x86desc",
    "x86info",
    "x86disassembler",
];

fn tool(variable: &str, default: &str) -> String {
    println!("cargo:rerun-if-env-changed={variable}");
    env::var(variable).unwrap_or_else(|_| default.to_string())
}

fn run(command: &mut Command) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("could not run {command:?}: {error}"));
    assert!(status.success(), "{command:?} failed with {status}");
}

fn output(command: &mut Command) -> String {
    let result = command
        .output()
        .unwrap_or_else(|error| panic!("could not run {command:?}: {error}"));
    assert!(result.status.success(), "{command:?} failed with {}", result.status);
    String::from_utf8(result.stdout).unwrap().trim().to_string()
}

/// Builds musl into `out/musl` once; later builds reuse it until
/// MUSL_MEMCPY changes. The build replaces musl's x86-64 assembly memcpy
/// with MUSL_MEMCPY and drops its assembly memmove (which is built on that
/// memcpy and copies overlapping bytes backwards one at a time), so musl
/// uses its generic C memmove.
fn build_musl(out: &Path, clang: &str) -> PathBuf {
    let prefix = out.join("musl");
    let memcpy = std::fs::read_to_string(MUSL_MEMCPY).unwrap_or_else(|error| panic!("could not read {MUSL_MEMCPY}: {error}"));
    // Records which memcpy and resolver patch the installed musl was built with.
    let stamp = prefix.join("qiq-memcpy.c");
    let stamped = format!("{memcpy}\n/* {MUSL_RESOLVER_HOOKED} */\n");
    if prefix.join("lib/libc.a").exists() && std::fs::read_to_string(&stamp).ok().as_deref() == Some(stamped.as_str()) {
        return prefix;
    }
    let source = out.join(MUSL);
    for stale in [&source, &prefix] {
        if stale.exists() {
            std::fs::remove_dir_all(stale).unwrap_or_else(|error| panic!("could not remove {}: {error}", stale.display()));
        }
    }
    let tarball = Path::new("vendor").join(format!("{MUSL}.tar.gz"));
    run(Command::new("tar").arg("xzf").arg(&tarball).arg("-C").arg(out));
    let string = source.join("src/string/x86_64");
    for assembly in ["memcpy.s", "memmove.s"] {
        std::fs::remove_file(string.join(assembly)).unwrap_or_else(|error| panic!("could not remove musl's {assembly}: {error}"));
    }
    std::fs::write(string.join("memcpy.c"), &memcpy).unwrap_or_else(|error| panic!("could not write memcpy.c: {error}"));
    let resolver = source.join("src/network/res_msend.c");
    let text = std::fs::read_to_string(&resolver).unwrap_or_else(|error| panic!("could not read musl's res_msend.c: {error}"));
    assert!(text.contains(MUSL_RESOLVER_POLL), "musl's res_msend.c has changed: its poll() call isn't where build.rs patches it");
    let text = text.replacen("#include \"lookup.h\"\n", &format!("#include \"lookup.h\"\n{MUSL_RESOLVER_HOOK}"), 1).replacen(MUSL_RESOLVER_POLL, MUSL_RESOLVER_HOOKED, 1);
    std::fs::write(&resolver, text).unwrap_or_else(|error| panic!("could not write musl's res_msend.c: {error}"));
    run(Command::new("./configure")
        .current_dir(&source)
        .arg(format!("--prefix={}", prefix.display()))
        .arg("--disable-shared")
        .env("CC", clang));
    let jobs = env::var("NUM_JOBS").unwrap_or_else(|_| "4".to_string());
    run(Command::new("make").current_dir(&source).arg(format!("-j{jobs}")));
    run(Command::new("make").current_dir(&source).arg("install"));
    std::fs::write(&stamp, &stamped).unwrap_or_else(|error| panic!("could not write {}: {error}", stamp.display()));
    prefix
}

/// Builds Mbed TLS into `out/libmbedtls.a` once (all three of its libraries:
/// TLS, X.509, and crypto), compiled for x86-64 Linux with musl. Returns the
/// archive and the directory of its headers.
fn build_mbedtls(out: &Path, clang: &str, musl: &Path) -> (PathBuf, PathBuf) {
    let source = out.join(MBEDTLS);
    let include = source.join("include");
    let archive = out.join("libmbedtls.a");
    if archive.exists() {
        return (archive, include);
    }
    let tarball = Path::new("vendor").join(format!("{MBEDTLS}.tar.bz2"));
    run(Command::new("tar").arg("xjf").arg(&tarball).arg("-C").arg(out));
    let objects = out.join("mbedtls-objects");
    std::fs::create_dir_all(&objects).unwrap();
    let library = source.join("library");
    let mut sources: Vec<PathBuf> = std::fs::read_dir(&library)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "c"))
        .collect();
    sources.sort();
    // Compiled in parallel: about 100 files.
    let jobs: usize = env::var("NUM_JOBS").ok().and_then(|jobs| jobs.parse().ok()).unwrap_or(4).max(1);
    let chunk = sources.len().div_ceil(jobs).max(1);
    std::thread::scope(|scope| {
        for files in sources.chunks(chunk) {
            let (objects, library, include) = (&objects, &library, &include);
            scope.spawn(move || {
                for file in files {
                    let object = objects.join(file.file_name().unwrap()).with_extension("o");
                    // -nostdlibinc keeps clang's own headers (for AES-NI intrinsics)
                    // while musl's replace the system's.
                    // No 128-bit division: it needs a compiler runtime helper
                    // (__udivti3) that musl doesn't provide.
                    run(Command::new(clang)
                        .args(["--target=x86_64-unknown-linux-musl", "-O2", "-ffunction-sections", "-fdata-sections"])
                        .arg("-DMBEDTLS_NO_UDBL_DIVISION")
                        .args(["-fno-stack-protector", "-nostdlibinc", "-isystem"])
                        .arg(musl.join("include"))
                        .arg("-I")
                        .arg(include)
                        .arg("-I")
                        .arg(library)
                        .arg("-c")
                        .arg(file)
                        .arg("-o")
                        .arg(&object));
                }
            });
        }
    });
    let llvm_ar = tool("LLVM_AR", "llvm-ar-18");
    let mut command = Command::new(&llvm_ar);
    command.arg("rcs").arg(&archive);
    for file in &sources {
        command.arg(objects.join(file.file_name().unwrap()).with_extension("o"));
    }
    run(&mut command);
    (archive, include)
}

/// Builds SQLite into `out/libsqlite3.a` once, compiled for x86-64 Linux
/// with musl. Returns the archive and the directory of sqlite3.h.
fn build_sqlite(out: &Path, clang: &str, musl: &Path) -> (PathBuf, PathBuf) {
    let source = out.join(SQLITE);
    let archive = out.join("libsqlite3.a");
    let stamp = out.join("sqlite-options.txt");
    let options = SQLITE_OPTIONS.join(" ");
    if archive.exists() && std::fs::read_to_string(&stamp).ok().as_deref() == Some(options.as_str()) {
        return (archive, source);
    }
    let tarball = Path::new("vendor").join(format!("{SQLITE}.tar.gz"));
    run(Command::new("tar").arg("xzf").arg(&tarball).arg("-C").arg(out));
    let object = out.join("sqlite3.o");
    run(Command::new(clang)
        .args(["--target=x86_64-unknown-linux-musl", "-O2", "-ffunction-sections", "-fdata-sections"])
        .args(SQLITE_OPTIONS)
        .args(["-fno-stack-protector", "-nostdlibinc", "-isystem"])
        .arg(musl.join("include"))
        .arg("-c")
        .arg(source.join("sqlite3.c"))
        .arg("-o")
        .arg(&object));
    if archive.exists() {
        std::fs::remove_file(&archive).unwrap();
    }
    run(Command::new(tool("LLVM_AR", "llvm-ar-18")).arg("rcs").arg(&archive).arg(&object));
    std::fs::write(&stamp, options).unwrap();
    (archive, source)
}

/// Builds PCRE2 into `out/libpcre2.a` once (again when PCRE2_OPTIONS
/// change), compiled for x86-64 Linux with musl. Returns the archive and the
/// directory of pcre2.h.
fn build_pcre2(out: &Path, clang: &str, musl: &Path) -> (PathBuf, PathBuf) {
    let source = out.join(PCRE2).join("src");
    let archive = out.join("libpcre2.a");
    let stamp = out.join("pcre2-options.txt");
    let options = PCRE2_OPTIONS.join(" ");
    if archive.exists() && std::fs::read_to_string(&stamp).ok().as_deref() == Some(options.as_str()) {
        return (archive, source);
    }
    if !source.join("pcre2.h").exists() {
        let tarball = Path::new("vendor").join(format!("{PCRE2}.tar.bz2"));
        run(Command::new("tar").arg("xjf").arg(&tarball).arg("-C").arg(out));
        for (from, to) in [("config.h.generic", "config.h"), ("pcre2.h.generic", "pcre2.h"), ("pcre2_chartables.c.dist", "pcre2_chartables.c")] {
            std::fs::copy(source.join(from), source.join(to)).unwrap();
        }
    }
    let objects = out.join("pcre2-objects");
    std::fs::create_dir_all(&objects).unwrap();
    let jobs: usize = env::var("NUM_JOBS").ok().and_then(|jobs| jobs.parse().ok()).unwrap_or(4).max(1);
    let chunk = PCRE2_SOURCES.len().div_ceil(jobs).max(1);
    std::thread::scope(|scope| {
        for files in PCRE2_SOURCES.chunks(chunk) {
            let (objects, source) = (&objects, &source);
            scope.spawn(move || {
                for file in files {
                    run(Command::new(clang)
                        .args(["--target=x86_64-unknown-linux-musl", "-O2", "-ffunction-sections", "-fdata-sections"])
                        .args(PCRE2_OPTIONS)
                        .args(["-fno-stack-protector", "-nostdlibinc", "-isystem"])
                        .arg(musl.join("include"))
                        .arg("-I")
                        .arg(source)
                        .arg("-c")
                        .arg(source.join(file))
                        .arg("-o")
                        .arg(objects.join(file).with_extension("o")));
                }
            });
        }
    });
    if archive.exists() {
        std::fs::remove_file(&archive).unwrap();
    }
    let mut command = Command::new(tool("LLVM_AR", "llvm-ar-18"));
    command.arg("rcs").arg(&archive);
    for file in PCRE2_SOURCES {
        command.arg(objects.join(file).with_extension("o"));
    }
    run(&mut command);
    std::fs::write(&stamp, options).unwrap();
    (archive, source)
}

/// Builds PCRE2 again into `out/libpcre2host.a`, for the machine qiq runs
/// on rather than the programs it builds: the compiler uses it to check
/// pattern literals (src/regex_check.rs).
fn build_pcre2_host(out: &Path, clang: &str) {
    let source = out.join(PCRE2).join("src");
    let archive = out.join("libpcre2host.a");
    if !archive.exists() {
        if !source.join("pcre2.h").exists() {
            let tarball = Path::new("vendor").join(format!("{PCRE2}.tar.bz2"));
            run(Command::new("tar").arg("xjf").arg(&tarball).arg("-C").arg(out));
            for (from, to) in [("config.h.generic", "config.h"), ("pcre2.h.generic", "pcre2.h"), ("pcre2_chartables.c.dist", "pcre2_chartables.c")] {
                std::fs::copy(source.join(from), source.join(to)).unwrap();
            }
        }
        let objects = out.join("pcre2-host-objects");
        std::fs::create_dir_all(&objects).unwrap();
        let jobs: usize = env::var("NUM_JOBS").ok().and_then(|jobs| jobs.parse().ok()).unwrap_or(4).max(1);
        let chunk = PCRE2_SOURCES.len().div_ceil(jobs).max(1);
        std::thread::scope(|scope| {
            for files in PCRE2_SOURCES.chunks(chunk) {
                let (objects, source) = (&objects, &source);
                scope.spawn(move || {
                    for file in files {
                        run(Command::new(clang)
                            .args(["-O2", "-fPIC", "-ffunction-sections", "-fdata-sections"])
                            .args(["-DHAVE_CONFIG_H", "-DPCRE2_CODE_UNIT_WIDTH=8", "-DPCRE2_STATIC", "-DSUPPORT_UNICODE"])
                            .arg("-I")
                            .arg(source)
                            .arg("-c")
                            .arg(source.join(file))
                            .arg("-o")
                            .arg(objects.join(file).with_extension("o")));
                    }
                });
            }
        });
        let mut command = Command::new(tool("LLVM_AR", "llvm-ar-18"));
        command.arg("rcs").arg(&archive);
        for file in PCRE2_SOURCES {
            command.arg(objects.join(file).with_extension("o"));
        }
        run(&mut command);
    }
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=pcre2host");
}

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let clang = tool("CLANG", "clang");
    let llvm_config = tool("LLVM_CONFIG", "llvm-config-18");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=vendor/{MUSL}.tar.gz");
    println!("cargo:rerun-if-changed={MUSL_MEMCPY}");
    println!("cargo:rerun-if-changed=vendor/{MBEDTLS}.tar.bz2");
    println!("cargo:rerun-if-changed=vendor/{SQLITE}.tar.gz");
    println!("cargo:rerun-if-changed=vendor/{PCRE2}.tar.bz2");
    println!("cargo:rerun-if-changed=vendor/tzdata.bin");
    println!("cargo:rerun-if-changed=stdlib/runtime");

    // musl libc for the programs qiq compiles.
    let musl = build_musl(&out, &clang);
    // Mbed TLS for HTTPS.
    let (mbedtls, mbedtls_include) = build_mbedtls(&out, &clang, &musl);
    // SQLite for std/sql.
    let (sqlite, sqlite_include) = build_sqlite(&out, &clang, &musl);
    // PCRE2 for std/regex.
    let (pcre2, pcre2_include) = build_pcre2(&out, &clang, &musl);
    build_pcre2_host(&out, &clang);

    // The C runtime, as bitcode for x86-64 Linux with musl.
    let runtime_dir = out.join("runtime");
    std::fs::create_dir_all(&runtime_dir).unwrap();
    // The embedded time zone database (vendor/make_tzdata.py), as a C array for tz.c.
    let tzdata = std::fs::read("vendor/tzdata.bin").expect("could not read vendor/tzdata.bin");
    let mut header = String::from("// Generated by build.rs from vendor/tzdata.bin.\nstatic const unsigned char qiq_tzdata[] = {");
    for (index, byte) in tzdata.iter().enumerate() {
        if index % 32 == 0 {
            header.push('\n');
        }
        header.push_str(&byte.to_string());
        header.push(',');
    }
    header.push_str("\n};\n");
    std::fs::write(runtime_dir.join("tzdata.h"), header).unwrap();
    let mut embedded = String::from(
        "/// Generated by build.rs: the precompiled runtime and musl.\n\
         pub const RUNTIME_BITCODE: &[(&str, &[u8])] = &[\n",
    );
    for source in RUNTIME_SOURCES {
        let input = Path::new("stdlib/runtime").join(source);
        let bitcode = runtime_dir.join(source.replace(".c", ".bc"));
        run(Command::new(&clang)
            .args(["--target=x86_64-unknown-linux-musl", "-O2", "-emit-llvm", "-c"])
            .args(["-nostdinc", "-fno-stack-protector", "-isystem"])
            .arg(musl.join("include"))
            .arg("-Istdlib/runtime")
            .arg("-I")
            .arg(&mbedtls_include)
            .arg("-I")
            .arg(&sqlite_include)
            .arg("-I")
            .arg(&pcre2_include)
            .args(["-DPCRE2_CODE_UNIT_WIDTH=8", "-DPCRE2_STATIC"])
            .arg("-I")
            .arg(&runtime_dir)
            .arg(&input)
            .arg("-o")
            .arg(&bitcode));
        embedded.push_str(&format!(
            "    ({source:?}, include_bytes!({:?})),\n",
            bitcode.display().to_string()
        ));
    }
    embedded.push_str("];\n\n/// musl files a static executable links, in link order around the program.\n");
    for (name, file) in [("CRT1", "crt1.o"), ("CRTI", "crti.o"), ("CRTN", "crtn.o"), ("LIBC", "libc.a")] {
        embedded.push_str(&format!(
            "pub const MUSL_{name}: &[u8] = include_bytes!({:?});\n",
            musl.join("lib").join(file).display().to_string()
        ));
    }
    embedded.push_str(&format!(
        "\n/// Mbed TLS, linked before libc so programs that use HTTPS get it.\npub const MBEDTLS: &[u8] = include_bytes!({:?});\n",
        mbedtls.display().to_string()
    ));
    embedded.push_str(&format!(
        "\n/// SQLite, for programs that use std/sql.\npub const SQLITE: &[u8] = include_bytes!({:?});\n",
        sqlite.display().to_string()
    ));
    embedded.push_str(&format!(
        "\n/// PCRE2, for programs that use std/regex.\npub const PCRE2: &[u8] = include_bytes!({:?});\n",
        pcre2.display().to_string()
    ));
    std::fs::write(out.join("embedded.rs"), embedded).unwrap();

    // LLVM, statically: only the components qiq uses.
    println!("cargo:rustc-link-search=native={}", output(Command::new(&llvm_config).arg("--libdir")));
    let libraries = output(Command::new(&llvm_config).args(["--link-static", "--libs"]).args(LLVM_COMPONENTS));
    for library in libraries.split_whitespace() {
        println!("cargo:rustc-link-lib=static={}", library.trim_start_matches("-l"));
    }
    // LLVM's own dependencies; zlib and terminfo statically, so qiq only needs glibc.
    for library in ["z", "tinfo"] {
        println!("cargo:rustc-link-lib=static={library}");
    }
    let libstdcxx = output(Command::new("cc").arg("-print-file-name=libstdc++.a"));
    if let Some(directory) = Path::new(&libstdcxx).parent() {
        println!("cargo:rustc-link-search=native={}", directory.display());
    }
    println!("cargo:rustc-link-lib=static=stdc++");
}
