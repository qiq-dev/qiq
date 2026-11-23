mod compiler;
mod fetch;
mod modfile;
mod regex_check;

use std::env;
use std::path::{Path, PathBuf};
use std::process;

// The precompiled runtime bitcode and musl libc, embedded by build.rs.
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

use inkwell::context::Context;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::module::Linkage;
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetTriple};
use inkwell::OptimizationLevel;

/// Compiled programs are static x86-64 Linux executables linked against musl.
const TARGET_TRIPLE: &str = "x86_64-unknown-linux-musl";

const USAGE: &str = "\
Usage:
  qiq run <file.qiq> [arguments...]          compile and run a program
  qiq build <file.qiq> [-o <output>] [--emit-llvm]
                                             compile a program to an executable
  qiq check <file.qiq> [--stdin]             report compile errors without building
                                             (--stdin: read the file's text from stdin)
  qiq serve [directory] [--host <host>] [--port <port>] [--workers <n>]
                                             serve a directory's files over HTTP
  qiq mod init <module path>                 make this directory a module (qiq.mod)
  qiq get [-u] [module[@version]]...         add, update, or download required modules
  qiq mod tidy                               require exactly the modules that are imported
  qiq help                                   show this help
  qiq version                                show the qiq version

`qiq build main.qiq` writes ./main. The executable is statically linked, so it
runs on any x86-64 Linux machine without qiq or any libraries installed. qiq
needs no other tools: LLVM, the runtime, libc, and the linker are built in.
--emit-llvm also writes the generated LLVM IR next to the output (<output>.ll).

`qiq serve` serves the current directory (or the one given) at
http://127.0.0.1:8000/, logging each request. --host 0.0.0.0 makes it
reachable from other machines; --workers sets how many processes serve
requests (default: one per CPU).";

/// A directory for intermediate files, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new(purpose: &str) -> Result<Self, String> {
        let path = env::temp_dir().join(format!("qiq-{purpose}-{}", process::id()));
        std::fs::create_dir_all(&path)
            .map_err(|error| format!("could not create {}: {error}", path.display()))?;
        Ok(Self(path))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Compiles `input` to a statically linked executable at `output`, optionally
/// keeping the LLVM IR at `emit_llvm`. Everything happens in this process:
/// LLVM optimizes the program together with the runtime and emits an object
/// file, and the embedded linker links it against the embedded musl libc.
fn build(input: &str, output: &Path, emit_llvm: Option<&Path>) -> Result<(), String> {
    // Errors from compile_file already name the file they occurred in.
    let ir = compiler::compile_file(input)?;
    build_ir(&ir, output, emit_llvm)
}

/// Builds an executable from a program's LLVM IR.
fn build_ir(ir: &str, output: &Path, emit_llvm: Option<&Path>) -> Result<(), String> {
    if let Some(path) = emit_llvm {
        std::fs::write(path, &ir).map_err(|error| format!("could not write {}: {error}", path.display()))?;
    }
    let work = TempDir::new("build")?;
    let object = work.0.join("program.o");
    emit_object(ir, &object)?;
    link(&work.0, &object, output)
}

/// Optimizes the program's IR, with the runtime linked in, into an object file.
fn emit_object(ir: &str, object: &Path) -> Result<(), String> {
    Target::initialize_x86(&InitializationConfig::default());
    let triple = TargetTriple::create(TARGET_TRIPLE);
    let target = Target::from_triple(&triple)
        .map_err(|error| format!("LLVM could not set up the {TARGET_TRIPLE} target: {error}"))?;
    let machine = target
        .create_target_machine(&triple, "x86-64", "", OptimizationLevel::Default, RelocMode::Static, CodeModel::Default)
        .ok_or_else(|| format!("LLVM could not create a {TARGET_TRIPLE} target machine"))?;
    let context = Context::create();
    // An error here is a compiler bug: qiq generated IR that LLVM rejects.
    let internal = |stage: &str, error: String| format!("internal compiler error ({stage}): {error}");
    let module = context
        .create_module_from_ir(MemoryBuffer::create_from_memory_range_copy(ir.as_bytes(), "program"))
        .map_err(|error| internal("parsing the generated IR", error.to_string()))?;
    module.set_triple(&triple);
    module.set_data_layout(&machine.get_target_data().get_data_layout());
    // The runtime is optimized together with the program, so small helpers
    // such as checked division can be inlined.
    for (name, bitcode) in embedded::RUNTIME_BITCODE {
        let runtime = context
            .create_module_from_ir(MemoryBuffer::create_from_memory_range_copy(bitcode, name))
            .map_err(|error| internal("loading the runtime", error.to_string()))?;
        module
            .link_in_module(runtime)
            .map_err(|error| internal("linking the runtime", error.to_string()))?;
    }
    // The module is the whole program, so everything but `main` can be
    // internal. The optimizer then drops what nothing uses: runtime helpers
    // and standard library methods the program never calls.
    for function in module.get_functions() {
        if function.count_basic_blocks() > 0 && function.get_name().to_bytes() != b"main" {
            function.set_linkage(Linkage::Internal);
        }
    }
    let mut global = module.get_first_global();
    while let Some(value) = global {
        if value.get_initializer().is_some() {
            value.set_linkage(Linkage::Internal);
        }
        global = value.get_next_global();
    }
    module
        .verify()
        .map_err(|error| internal("verifying the generated IR", error.to_string()))?;
    module
        .run_passes("default<O2>", &machine, PassBuilderOptions::create())
        .map_err(|error| internal("optimizing", error.to_string()))?;
    machine
        .write_to_file(&module, FileType::Object, object)
        .map_err(|error| format!("could not write {}: {error}", object.display()))
}

/// Links the object statically against the embedded musl libc.
fn link(work: &Path, object: &Path, output: &Path) -> Result<(), String> {
    let mut paths = Vec::new();
    for (name, bytes) in [
        ("crt1.o", embedded::MUSL_CRT1),
        ("crti.o", embedded::MUSL_CRTI),
        ("libc.a", embedded::MUSL_LIBC),
        ("crtn.o", embedded::MUSL_CRTN),
        ("libmbedtls.a", embedded::MBEDTLS),
        ("libsqlite3.a", embedded::SQLITE),
        ("libpcre2.a", embedded::PCRE2),
    ] {
        let path = work.join(name);
        std::fs::write(&path, bytes).map_err(|error| format!("could not write {}: {error}", path.display()))?;
        paths.push(path.display().to_string());
    }
    let arguments: Vec<String> = vec![
        "ld".into(),
        "-static".into(),
        "-o".into(),
        output.display().to_string(),
        paths[0].clone(),
        paths[1].clone(),
        object.display().to_string(),
        // Archives contribute only the members a program uses, so programs
        // without HTTPS, SQL, or regular expressions don't grow.
        paths[6].clone(),
        paths[5].clone(),
        paths[4].clone(),
        paths[2].clone(),
        paths[3].clone(),
    ];
    let failed = |error: String| format!("could not link {}: {error}", output.display());
    let mut args = libwild::Args::new(|| arguments.iter()).map_err(|error| failed(error.to_string()))?;
    args.parse(|| arguments.iter()).map_err(|error| failed(error.to_string()))?;
    libwild::run(args).map_err(|error| failed(error.to_string()))
}

/// `qiq build <file> [-o <output>] [--emit-llvm]`
fn command_build(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let mut input = None;
    let mut output = None;
    let mut emit_llvm = false;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "-o" => output = Some(args.next().ok_or("missing output after -o")?),
            "--emit-llvm" => emit_llvm = true,
            _ if argument.starts_with('-') => return Err(format!("unknown option '{argument}'\n\n{USAGE}")),
            _ if input.is_none() => input = Some(argument),
            _ => return Err(format!("unexpected argument '{argument}'\n\n{USAGE}")),
        }
    }
    let input = input.ok_or(format!("missing input file\n\n{USAGE}"))?;
    // Like `go build main.go`: ./main by default.
    let output = match output {
        Some(output) => PathBuf::from(output),
        None => PathBuf::from(
            Path::new(&input)
                .file_stem()
                .ok_or_else(|| format!("cannot name an executable after '{input}'; use -o"))?,
        ),
    };
    let ir_path = emit_llvm.then(|| output.with_extension("ll"));
    build(&input, &output, ir_path.as_deref())
}

/// `qiq check <file> [--stdin]`: compile without optimizing or linking, to
/// report errors quickly. With --stdin the file's text comes from standard
/// input (an editor's unsaved buffer); imports still resolve from `<file>`.
fn command_check(args: impl Iterator<Item = String>) -> Result<(), String> {
    let mut input = None;
    let mut stdin = false;
    for argument in args {
        match argument.as_str() {
            "--stdin" => stdin = true,
            _ if argument.starts_with('-') => return Err(format!("unknown option '{argument}'\n\n{USAGE}")),
            _ if input.is_none() => input = Some(argument),
            _ => return Err(format!("unexpected argument '{argument}'\n\n{USAGE}")),
        }
    }
    let input = input.ok_or(format!("missing input file\n\n{USAGE}"))?;
    if stdin {
        let mut source = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut source)
            .map_err(|error| format!("could not read standard input: {error}"))?;
        compiler::compile_file_with_source(&input, &source)?;
    } else {
        compiler::compile_file(&input)?;
    }
    Ok(())
}

/// `qiq run <file> [arguments...]`: build into a temporary directory, run the
/// program with the arguments, and exit with its status.
fn command_run(mut args: impl Iterator<Item = String>) -> Result<i32, String> {
    let input = args.next().ok_or(format!("missing input file\n\n{USAGE}"))?;
    let work = TempDir::new("run")?;
    let executable = work.0.join("program");
    build(&input, &executable, None)?;
    // The program sees the source file as its name (argv()[0]), not the
    // temporary executable.
    run_executable(&executable, &input, args)
}

/// Runs a built program and returns its exit status. Ctrl-C reaches the
/// program (the terminal sends it to both processes); qiq ignores it, so it
/// waits for the program to finish and then removes the temporary directory.
fn run_executable(executable: &Path, name: &str, args: impl Iterator<Item = String>) -> Result<i32, String> {
    extern "C" {
        fn signal(number: i32, handler: usize) -> usize;
    }
    const SIGINT: i32 = 2;
    const SIG_IGN: usize = 1;
    use std::os::unix::process::CommandExt;
    let mut command = process::Command::new(executable);
    command.arg0(name).args(args);
    // The child starts with the default handler again.
    let previous = unsafe { signal(SIGINT, SIG_IGN) };
    unsafe {
        command.pre_exec(|| {
            signal(SIGINT, 0);
            Ok(())
        });
    }
    let status = command.status();
    unsafe { signal(SIGINT, previous) };
    let status = status.map_err(|error| format!("could not run the program: {error}"))?;
    // A program killed by a signal exits like a shell would report it.
    use std::os::unix::process::ExitStatusExt;
    Ok(status.code().unwrap_or_else(|| 128 + status.signal().unwrap_or(0)))
}

/// The program behind `qiq serve`; its arguments are the directory, host,
/// port, and number of workers.
const SERVE_PROGRAM: &str = r#"
import File, is_dir from "std/file";
import argv, exit from "std/os";
import http_file_server, http_log, http_serve from "std/net/http/server";

string[] args = argv();
string directory = args[1];
if (!is_dir(directory)) {
    File::stderr()->write("qiq: " + directory + " is not a directory\n");
    exit(1);
}
string host = args[2];
if (str_contains(host, ":")) {
    host = "[" + host + "]";
}
string shown = host;
if (shown == "" || shown == "0.0.0.0" || shown == "[::]") {
    shown = "localhost";
}
print("Serving " + directory + " at http://" + shown + ":" + args[3] + "/ (press Ctrl-C to stop)");
http_serve(host + ":" + args[3], http_log(http_file_server(directory)), to_int(args[4]));
"#;

/// `qiq serve [directory] [--host <host>] [--port <port>] [--workers <n>]`:
/// a static file server, like `php -S` or `python -m http.server`.
fn command_serve(mut args: impl Iterator<Item = String>) -> Result<i32, String> {
    let mut directory = None;
    let mut host = "127.0.0.1".to_string();
    let mut port = "8000".to_string();
    let mut workers = "0".to_string();
    while let Some(arg) = args.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((flag, value)) if arg.starts_with("--") => (flag.to_string(), Some(value.to_string())),
            _ => (arg.clone(), None),
        };
        let mut value = |name: &str| {
            inline.clone().or_else(|| args.next()).ok_or(format!("{name} needs a value\n\n{USAGE}"))
        };
        match flag.as_str() {
            "--host" | "-H" => host = value("--host")?,
            "--port" | "-p" => port = value("--port")?,
            "--workers" | "-w" => workers = value("--workers")?,
            _ if flag.starts_with('-') && flag != "-" => return Err(format!("unknown option '{flag}'\n\n{USAGE}")),
            _ if directory.is_none() => directory = Some(arg),
            _ => return Err(format!("unexpected argument '{arg}'\n\n{USAGE}")),
        }
    }
    if port.parse::<u16>().is_err() {
        return Err(format!("invalid port '{port}'"));
    }
    if workers.parse::<u16>().is_err() {
        return Err(format!("invalid number of workers '{workers}'"));
    }
    let directory = directory.unwrap_or_else(|| ".".to_string());
    let ir = compiler::compile_source(SERVE_PROGRAM)?;
    let work = TempDir::new("serve")?;
    let executable = work.0.join("server");
    build_ir(&ir, &executable, None)?;
    run_executable(&executable, "qiq serve", [directory, host, port, workers].into_iter())
}

/// `qiq mod init <module path>`: writes qiq.mod in the current directory.
fn command_mod(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let usage = "usage: qiq mod init <module path>   create qiq.mod, such as: qiq mod init github.com/you/project\n       qiq mod tidy                 require exactly the modules that are imported";
    match args.next().as_deref() {
        Some("init") => {}
        Some("tidy") => {
            if let Some(extra) = args.next() {
                return Err(format!("unexpected argument '{extra}'\n{usage}"));
            }
            return fetch::command_tidy();
        }
        Some(other) => return Err(format!("unknown 'qiq mod' command '{other}'\n{usage}")),
        None => return Err(usage.to_string()),
    }
    let module = args.next().ok_or(usage.to_string())?;
    if let Some(extra) = args.next() {
        return Err(format!("unexpected argument '{extra}'\n{usage}"));
    }
    if !modfile::valid_module_path(&module, false) {
        return Err(format!(
            "invalid module path '{module}': use letters, digits, and . _ ~ -, separated by /, such as github.com/you/project"
        ));
    }
    let path = Path::new(modfile::MOD_FILE);
    if path.exists() {
        return Err(format!("{} already exists", modfile::MOD_FILE));
    }
    std::fs::write(path, modfile::new_file(&module))
        .map_err(|error| format!("could not write {}: {error}", modfile::MOD_FILE))?;
    println!("qiq: created {} for module {module}", modfile::MOD_FILE);
    Ok(())
}

fn run() -> Result<i32, String> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or(USAGE.to_string())?;
    match command.as_str() {
        "run" => command_run(args),
        "build" => command_build(args).map(|()| 0),
        "check" => command_check(args).map(|()| 0),
        "serve" | "server" => command_serve(args),
        "mod" => command_mod(args).map(|()| 0),
        "get" => fetch::command_get(args).map(|()| 0),
        "help" | "-h" | "--help" => {
            println!("{USAGE}");
            Ok(0)
        }
        "version" | "--version" => {
            println!("qiq {}", env!("CARGO_PKG_VERSION"));
            Ok(0)
        }
        // The original form, `qiq <file.qiq> -o <output>`, is `qiq build`.
        _ if command.ends_with(".qiq") => command_build(std::iter::once(command).chain(args)).map(|()| 0),
        _ => Err(format!("unknown command '{command}'\n\n{USAGE}")),
    }
}

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(error) => {
            eprintln!("qiq: {error}");
            process::exit(1);
        }
    }
}
