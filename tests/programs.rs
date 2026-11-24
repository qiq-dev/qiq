//! Runs the qiq programs in tests/ and checks what they print.
//!
//! Every `.qiq` file in `tests/` and `tests/stdlib/`, and every `main.qiq`
//! in a directory below them (a program made of several files), is a test.
//! It's run with `qiq run` from the repository root, and must print exactly
//! what its expected files say:
//!
//!   - `name.out`: standard output.
//!   - `name.err`: standard error, if it writes any (an uncaught exception).
//!
//! Comments at the top of the program change how it's run:
//!
//!   // test: args Ada "Grace Hopper"   command-line arguments
//!   // test: exit 2                    the exit status it must end with (0 otherwise)
//!   // test: qiq-test                  run it with `qiq test` instead of `qiq run`
//!                                      (its timings are masked: "0.00s")
//!   // test: needs QIQ_TEST_MYSQL      skipped unless this environment variable is
//!                                      set; its output isn't compared (it depends
//!                                      on a server), only its exit status
//!
//! To record or update the expected output after a change, check that the
//! program does the right thing, then run:
//!
//!   QIQ_UPDATE_TESTS=1 cargo test --release --test programs
//!
//! and review the changes to the .out and .err files with git diff.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// What a test's header comments ask for.
#[derive(Default)]
struct Directives {
    args: Vec<String>,
    exit: i32,
    needs: Option<String>,
    qiq_test: bool,
}

fn directives(source: &str) -> Result<Directives, String> {
    let mut found = Directives::default();
    for line in source.lines() {
        let line = line.trim();
        if !line.starts_with("//") {
            break;
        }
        let Some(rest) = line.trim_start_matches('/').trim().strip_prefix("test:") else { continue };
        let rest = rest.trim();
        let (name, value) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        match name {
            "args" => found.args = split_words(value)?,
            "exit" => found.exit = value.trim().parse().map_err(|_| format!("'test: exit' needs a number, found '{value}'"))?,
            "needs" => found.needs = Some(value.trim().to_string()),
            "qiq-test" => found.qiq_test = true,
            _ => return Err(format!("unknown directive 'test: {name}'; the directives are args, exit, needs, and qiq-test")),
        }
    }
    Ok(found)
}

/// Splits arguments at spaces, keeping "quoted text" together.
fn split_words(text: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut started = false;
    for c in text.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    words.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if quoted {
        return Err("'test: args' has an unclosed quote".into());
    }
    if started {
        words.push(current);
    }
    Ok(words)
}

/// The test programs, relative to the repository root, in order.
fn programs(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for directory in ["tests", "tests/stdlib"] {
        let Ok(entries) = std::fs::read_dir(root.join(directory)) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if path.extension().is_some_and(|extension| extension == "qiq") {
                found.push(relative);
            } else if path.is_dir() && path.join("main.qiq").is_file() {
                found.push(relative.join("main.qiq"));
            }
        }
    }
    found.sort();
    found
}

/// `qiq test` output with its timings, which vary, written as 0.00s: in
/// "--- FAIL: test_x (0.01s)" and in "ok  \tfile.qiq\t0.85s".
fn mask_timings(output: &str) -> String {
    let mut masked = String::new();
    for line in output.lines() {
        let mut line = line.to_string();
        if line.starts_with("--- ") && line.ends_with("s)") {
            if let Some(open) = line.rfind('(') {
                line = format!("{}(0.00s)", &line[..open]);
            }
        } else if (line.starts_with("ok  \t") || line.starts_with("FAIL\t")) && line.ends_with('s') {
            if let Some(tab) = line.rfind('\t') {
                if line[tab + 1..line.len() - 1].parse::<f64>().is_ok() {
                    line = format!("{}\t0.00s", &line[..tab]);
                }
            }
        }
        masked.push_str(&line);
        masked.push('\n');
    }
    masked
}

/// The first line where `expected` and `actual` differ, with a little
/// context, for a failure message.
fn difference(what: &str, expected: &str, actual: &str) -> String {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();
    let line = (0..expected_lines.len().max(actual_lines.len()))
        .find(|&index| expected_lines.get(index) != actual_lines.get(index))
        .unwrap_or(expected_lines.len().min(actual_lines.len()));
    let show = |lines: &[&str]| -> String {
        let from = line.saturating_sub(2);
        let to = (line + 3).min(lines.len());
        if from >= to {
            return "      (nothing)\n".into();
        }
        (from..to).map(|index| format!("  {:>4} {}\n", index + 1, lines[index])).collect()
    };
    let mut message = format!("  {what} differs at line {}:\n  expected:\n{}  actual:\n{}", line + 1, show(&expected_lines), show(&actual_lines));
    if expected_lines == actual_lines {
        message = format!("  {what} differs only in its final newline\n");
    }
    message
}

/// Runs one test; returns a description of what went wrong, if anything.
fn check(root: &Path, qiq: &Path, program: &Path, update: bool) -> Result<(), String> {
    let source = std::fs::read_to_string(root.join(program)).map_err(|error| format!("could not read it: {error}"))?;
    let directives = directives(&source)?;
    if let Some(variable) = &directives.needs {
        if std::env::var_os(variable).is_none() {
            return Ok(());
        }
    }
    let output = Command::new(qiq)
        .arg(if directives.qiq_test { "test" } else { "run" })
        .arg(program)
        .args(&directives.args)
        .current_dir(root)
        // Tests read these; keep the developer's own settings out.
        .env_remove("VERBOSE")
        .env_remove("QIQ_UNSET_VAR")
        .output()
        .map_err(|error| format!("could not run {}: {error}", qiq.display()))?;
    let mut stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if directives.qiq_test {
        stdout = mask_timings(&stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let status = output.status.code().unwrap_or(-1);
    let mut problems = String::new();
    if status != directives.exit {
        problems.push_str(&format!("  exit status {status}, expected {}\n", directives.exit));
        if directives.exit == 0 && !stderr.is_empty() {
            problems.push_str(&format!("  standard error:\n{}", stderr.lines().map(|line| format!("    {line}\n")).collect::<String>()));
        }
    }
    if directives.needs.is_some() {
        return if problems.is_empty() { Ok(()) } else { Err(problems) };
    }
    let out_file = root.join(program).with_extension("out");
    let err_file = root.join(program).with_extension("err");
    if update {
        if !problems.is_empty() {
            return Err(format!("{problems}  (not recorded: fix the exit status, or add '// test: exit {status}')\n"));
        }
        std::fs::write(&out_file, &stdout).map_err(|error| error.to_string())?;
        if stderr.is_empty() {
            let _ = std::fs::remove_file(&err_file);
        } else {
            std::fs::write(&err_file, &stderr).map_err(|error| error.to_string())?;
        }
        return Ok(());
    }
    match std::fs::read_to_string(&out_file) {
        Ok(expected) if expected != stdout => problems.push_str(&difference("standard output", &expected, &stdout)),
        Ok(_) => {}
        Err(_) => problems.push_str(&format!("  no {} with its expected output; record it with QIQ_UPDATE_TESTS=1\n", out_file.strip_prefix(root).unwrap().display())),
    }
    let expected_err = std::fs::read_to_string(&err_file).unwrap_or_default();
    if expected_err != stderr {
        problems.push_str(&difference("standard error", &expected_err, &stderr));
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}

#[test]
fn programs_print_what_they_should() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let qiq = PathBuf::from(env!("CARGO_BIN_EXE_qiq"));
    let update = std::env::var_os("QIQ_UPDATE_TESTS").is_some();
    let programs = programs(&root);
    assert!(!programs.is_empty(), "no test programs found in tests/");

    // Each program is compiled and run on its own, so run them in parallel.
    let next = Mutex::new(0usize);
    let failures = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map(|count| count.get()).unwrap_or(4).min(16);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let index = {
                    let mut next = next.lock().unwrap();
                    let index = *next;
                    *next += 1;
                    index
                };
                let Some(program) = programs.get(index) else { break };
                if let Err(problem) = check(&root, &qiq, program, update) {
                    failures.lock().unwrap().push(format!("{}:\n{problem}", program.display()));
                }
            });
        }
    });

    let mut failures = failures.into_inner().unwrap();
    failures.sort();
    if !failures.is_empty() {
        panic!(
            "{} of {} test programs failed:\n\n{}\nIf the new output is right, record it with QIQ_UPDATE_TESTS=1 cargo test --release --test programs",
            failures.len(),
            programs.len(),
            failures.join("\n")
        );
    }
}
