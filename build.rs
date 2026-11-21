//! Prepares everything qiq embeds so that compiling qiq programs needs no
//! other tools:
//!
//! - musl libc, built from `vendor/musl-1.2.5.tar.gz` with qiq's memcpy
//!   (`vendor/musl-x86_64-memcpy.c`), which compiled programs link against
//!   statically;
//! - Mbed TLS, built from `vendor/mbedtls-3.6.8.tar.bz2` (sha256
//!   bc54b818e87f49c0b3281dd85d24adaede0e63b4e822e3fbd49c005a38b9595b) as a
//!   static library that programs using HTTPS link against;
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
/// Replaces musl's x86-64 assembly memcpy, which is slow for short copies.
const MUSL_MEMCPY: &str = "vendor/musl-x86_64-memcpy.c";
const RUNTIME_SOURCES: &[&str] = &[
    "core.c",
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
    // Records which memcpy the installed musl was built with.
    let stamp = prefix.join("qiq-memcpy.c");
    if prefix.join("lib/libc.a").exists() && std::fs::read_to_string(&stamp).ok().as_deref() == Some(memcpy.as_str()) {
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
    run(Command::new("./configure")
        .current_dir(&source)
        .arg(format!("--prefix={}", prefix.display()))
        .arg("--disable-shared")
        .env("CC", clang));
    let jobs = env::var("NUM_JOBS").unwrap_or_else(|_| "4".to_string());
    run(Command::new("make").current_dir(&source).arg(format!("-j{jobs}")));
    run(Command::new("make").current_dir(&source).arg("install"));
    std::fs::write(&stamp, &memcpy).unwrap_or_else(|error| panic!("could not write {}: {error}", stamp.display()));
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

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let clang = tool("CLANG", "clang");
    let llvm_config = tool("LLVM_CONFIG", "llvm-config-18");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=vendor/{MUSL}.tar.gz");
    println!("cargo:rerun-if-changed={MUSL_MEMCPY}");
    println!("cargo:rerun-if-changed=vendor/{MBEDTLS}.tar.bz2");
    println!("cargo:rerun-if-changed=stdlib/runtime");

    // musl libc for the programs qiq compiles.
    let musl = build_musl(&out, &clang);
    // Mbed TLS for HTTPS.
    let (mbedtls, mbedtls_include) = build_mbedtls(&out, &clang, &musl);

    // The C runtime, as bitcode for x86-64 Linux with musl.
    let runtime_dir = out.join("runtime");
    std::fs::create_dir_all(&runtime_dir).unwrap();
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
