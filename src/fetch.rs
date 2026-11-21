//! Downloading modules and checking them: `qiq get`, `qiq mod tidy`, and the
//! downloads `qiq build` and `qiq run` make for modules that qiq.mod pins but
//! the cache doesn't have yet.
//!
//! A module's path is where its git repository lives: github.com/ada/web is
//! cloned from https://github.com/ada/web, and its versions are the tags
//! named like v1.4.2. Git's own settings apply, so a private repository can
//! be reached over SSH with
//!     git config --global url."git@github.com:".insteadOf "https://github.com/"
//!
//! Each version is cloned once into the module cache, without .git, with its
//! files made read-only. qiq.sum records a hash of each module version's
//! files; a download that doesn't match is refused, which catches a tag
//! that was moved and code changed on the way.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::modfile::{self, ModFile, Version, MOD_FILE};

pub const SUM_FILE: &str = "qiq.sum";

/// Checksums from qiq.sum: (module path, version) -> "h1:...".
pub type Sums = BTreeMap<(String, String), String>;

pub fn parse_sums(text: &str, name: &str) -> Result<Sums, String> {
    let mut sums = Sums::new();
    for (index, line) in text.lines().enumerate() {
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.is_empty() {
            continue;
        }
        if words.len() != 3 || !words[2].starts_with("h1:") {
            return Err(format!("{name}:{}: expected '<module path> <version> h1:<hash>'", index + 1));
        }
        sums.insert((words[0].to_string(), words[1].to_string()), words[2].to_string());
    }
    Ok(sums)
}

pub fn format_sums(sums: &Sums) -> String {
    sums.iter().map(|((path, version), hash)| format!("{path} {version} {hash}\n")).collect()
}

/// The module's directory in the cache.
pub fn cache_path(module: &str, version: &Version) -> PathBuf {
    modfile::cache_directory().join(format!("{module}@{version}"))
}

fn git_url(module: &str) -> String {
    format!("https://{module}")
}

/// Runs git, returning its output; errors include what git printed.
fn git(arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(arguments)
        // Fail instead of waiting for a password that nobody will type.
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                "downloading modules needs git, which isn't installed".to_string()
            } else {
                format!("could not run git: {error}")
            }
        })?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(message);
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// The module's versions: its tags named like v1.2.3, oldest first.
pub fn versions(module: &str) -> Result<Vec<Version>, String> {
    let url = git_url(module);
    let listing = git(&["ls-remote", "--tags", "--refs", &url])
        .map_err(|error| format!("could not list the versions of {module} ({url}): {error}"))?;
    let mut versions: Vec<Version> = listing
        .lines()
        .filter_map(|line| line.split('\t').nth(1))
        .filter_map(|reference| reference.strip_prefix("refs/tags/"))
        .filter_map(Version::parse)
        .collect();
    versions.sort();
    versions.dedup();
    Ok(versions)
}

/// The newest release, or the newest prerelease if there are only those.
pub fn latest(module: &str) -> Result<Version, String> {
    let versions = versions(module)?;
    versions
        .iter()
        .filter(|version| !version.is_prerelease())
        .max()
        .or_else(|| versions.iter().max())
        .cloned()
        .ok_or_else(|| format!("{module} has no version tags; a module's versions are git tags such as v0.1.0"))
}

/// Makes sure the module version is in the cache, downloading it if needed,
/// and checks it against `expected` (its qiq.sum hash) when there is one.
/// Returns its directory and hash.
pub fn download(module: &str, version: &Version, expected: Option<&str>) -> Result<(PathBuf, String), String> {
    let target = cache_path(module, version);
    let fresh = !target.is_dir();
    if fresh {
        let parent = target.parent().expect("cache paths have a parent");
        std::fs::create_dir_all(parent).map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        let temporary = parent.join(format!(
            ".{}.download-{}",
            target.file_name().unwrap().to_string_lossy(),
            std::process::id()
        ));
        let _ = remove_tree(&temporary);
        let url = git_url(module);
        let tag = version.to_string();
        eprintln!("qiq: downloading {module} {version}");
        let cloned = git(&[
            "-c",
            "advice.detachedHead=false",
            "clone",
            "--quiet",
            "--depth",
            "1",
            "--branch",
            &tag,
            &url,
            &temporary.to_string_lossy(),
        ]);
        if let Err(error) = cloned {
            let _ = remove_tree(&temporary);
            return Err(format!("could not download {module} {version} from {url}: {error}"));
        }
        prepare_download(&temporary).map_err(|error| {
            let _ = remove_tree(&temporary);
            error
        })?;
        if std::fs::rename(&temporary, &target).is_err() {
            // Another qiq downloaded it at the same time.
            let _ = remove_tree(&temporary);
        }
    }
    let hash = hash_directory(&target)?;
    if let Some(expected) = expected {
        if expected != hash {
            if fresh {
                let _ = remove_tree(&target);
            }
            return Err(format!(
                "checksum mismatch for {module} {version}:\n    {SUM_FILE}: {expected}\n    downloaded: {hash}\n\
                 The code behind the tag isn't what was recorded: the tag may have been moved, or the download tampered with. \
                 Don't use it until you know why."
            ));
        }
    }
    Ok((target, hash))
}

/// Removes .git and symbolic links (which could point outside the module),
/// and makes the files read-only.
fn prepare_download(directory: &Path) -> Result<(), String> {
    remove_tree(&directory.join(".git"))?;
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = std::fs::read_dir(&current).map_err(|error| format!("could not read {}: {error}", current.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            if kind.is_symlink() {
                std::fs::remove_file(&path).map_err(|error| format!("could not remove {}: {error}", path.display()))?;
            } else if kind.is_dir() {
                pending.push(path);
            } else {
                let mut permissions = std::fs::metadata(&path).map_err(|error| error.to_string())?.permissions();
                permissions.set_readonly(true);
                let _ = std::fs::set_permissions(&path, permissions);
            }
        }
    }
    Ok(())
}

/// Removes a directory tree, including read-only files in it.
fn remove_tree(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(path).map_err(|error| format!("could not remove {}: {error}", path.display()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::new();
    for chunk in bytes.chunks(3) {
        let value = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        for index in 0..4 {
            if index <= chunk.len() {
                text.push(ALPHABET[(value >> (18 - 6 * index) & 63) as usize] as char);
            } else {
                text.push('=');
            }
        }
    }
    text
}

/// The hash of a directory's files, as in Go's go.sum: the SHA-256 of a
/// sorted list of "<sha256 of file>  <path>" lines, in base64, after "h1:".
pub fn hash_directory(directory: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = std::fs::read_dir(&current).map_err(|error| format!("could not read {}: {error}", current.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            if kind.is_dir() {
                if entry.file_name() != ".git" {
                    pending.push(path);
                }
            } else if kind.is_file() {
                let relative = path.strip_prefix(directory).expect("inside the directory");
                let name: Vec<String> = relative.components().map(|part| part.as_os_str().to_string_lossy().to_string()).collect();
                files.push((name.join("/"), path));
            }
        }
    }
    files.sort();
    let mut summary = String::new();
    for (name, path) in files {
        let contents = std::fs::read(&path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
        summary.push_str(&format!("{}  {name}\n", hex(&Sha256::digest(&contents))));
    }
    Ok(format!("h1:{}", base64(&Sha256::digest(summary.as_bytes()))))
}

/// The main module: its qiq.mod's directory and contents, found in the
/// current directory or the nearest one above it.
fn main_module() -> Result<(PathBuf, ModFile, Sums), String> {
    let mut directory = std::env::current_dir().map_err(|error| format!("could not find the current directory: {error}"))?;
    loop {
        let file = directory.join(MOD_FILE);
        if file.is_file() {
            let text = std::fs::read_to_string(&file).map_err(|error| format!("could not read {}: {error}", file.display()))?;
            let parsed = modfile::parse(&text, MOD_FILE)?;
            let sum_file = directory.join(SUM_FILE);
            let sums = match std::fs::read_to_string(&sum_file) {
                Ok(text) => parse_sums(&text, SUM_FILE)?,
                Err(_) => Sums::new(),
            };
            return Ok((directory, parsed, sums));
        }
        if !directory.pop() {
            return Err(format!(
                "no {MOD_FILE} in this directory or above it; create one with 'qiq mod init <module path>'"
            ));
        }
    }
}

/// The modules a build uses, by minimal version selection: starting from
/// the main module's requirements, each module's own requirements raise the
/// version chosen for a module to the newest any of them asks for.
/// Downloads (and checks against `sums`) every module it reads, and fills in
/// `hashes`. Returns module path -> (version, which module asked for it).
fn select(
    root: &Path,
    main: &ModFile,
    sums: &Sums,
    hashes: &mut Sums,
) -> Result<BTreeMap<String, (Version, String)>, String> {
    let replaced: HashMap<&str, &str> = main.replaces.iter().map(|(path, target)| (path.as_str(), target.as_str())).collect();
    let mut selected: BTreeMap<String, (Version, String)> = main
        .requires
        .iter()
        .map(|(path, version)| (path.clone(), (version.clone(), main.module.clone())))
        .collect();
    let mut done: HashSet<(String, Version)> = HashSet::new();
    loop {
        let pending: Vec<(String, Version)> = selected
            .iter()
            .map(|(path, (version, _))| (path.clone(), version.clone()))
            .filter(|key| !done.contains(key))
            .collect();
        if pending.is_empty() {
            return Ok(selected);
        }
        for (path, version) in pending {
            done.insert((path.clone(), version.clone()));
            let directory = match replaced.get(path.as_str()) {
                Some(target) => root.join(target),
                None => {
                    let key = (path.clone(), version.to_string());
                    let (directory, hash) = download(&path, &version, sums.get(&key).map(String::as_str))?;
                    hashes.insert(key, hash);
                    directory
                }
            };
            let file = directory.join(MOD_FILE);
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;  // a module without a qiq.mod requires nothing
            };
            let parsed = modfile::parse(&text, &format!("{path}@{version}/{MOD_FILE}"))?;
            if parsed.module != path {
                return Err(format!("{path} {version}'s {MOD_FILE} says it's module {}, so it can't be required as {path}", parsed.module));
            }
            for (needed, needed_version) in parsed.requires {
                if needed == main.module {
                    continue;
                }
                let newer = selected.get(&needed).map_or(true, |(chosen, _)| needed_version > *chosen);
                if newer {
                    selected.insert(needed, (needed_version, format!("{path} {version}")));
                }
            }
        }
    }
}

/// Rewrites qiq.mod with `requires` (direct ones first), and qiq.sum.
fn save(root: &Path, main: &ModFile, requires: &BTreeMap<String, (Version, bool)>, sums: &Sums) -> Result<(), String> {
    let mut text = format!("module {}\n", main.module);
    if let Some((major, minor)) = main.qiq {
        text.push_str(&format!("\nqiq {major}.{minor}\n"));
    }
    let mut lines: Vec<String> = requires
        .iter()
        .filter(|(_, (_, indirect))| !indirect)
        .map(|(path, (version, _))| format!("{path} {version}"))
        .collect();
    lines.extend(
        requires
            .iter()
            .filter(|(_, (_, indirect))| *indirect)
            .map(|(path, (version, _))| format!("{path} {version} // indirect")),
    );
    match lines.len() {
        0 => {}
        1 => text.push_str(&format!("\nrequire {}\n", lines[0])),
        _ => text.push_str(&format!("\nrequire (\n{})\n", lines.iter().map(|line| format!("    {line}\n")).collect::<String>())),
    }
    let replaces: Vec<&(String, String)> = main.replaces.iter().filter(|(path, _)| requires.contains_key(path)).collect();
    if !replaces.is_empty() {
        text.push('\n');
        for (path, target) in replaces {
            text.push_str(&format!("replace {path} => {target}\n"));
        }
    }
    let write = |name: &str, contents: &str| {
        let path = root.join(name);
        std::fs::write(&path, contents).map_err(|error| format!("could not write {}: {error}", path.display()))
    };
    write(MOD_FILE, &text)?;
    if sums.is_empty() {
        let _ = std::fs::remove_file(root.join(SUM_FILE));
        Ok(())
    } else {
        write(SUM_FILE, &format_sums(sums))
    }
}

/// Resolves the build list from `main` (whose requirements may have been
/// changed), then saves qiq.mod and qiq.sum. `direct` lists the modules the
/// main module imports itself; the others are marked `// indirect`.
fn resolve_and_save(root: &Path, main: &ModFile, sums: &Sums, direct: &HashSet<String>) -> Result<BTreeMap<String, (Version, String)>, String> {
    let mut hashes = Sums::new();
    let selected = select(root, main, sums, &mut hashes)?;
    let requires: BTreeMap<String, (Version, bool)> = selected
        .iter()
        .map(|(path, (version, _))| (path.clone(), (version.clone(), !direct.contains(path))))
        .collect();
    save(root, main, &requires, &hashes)?;
    Ok(selected)
}

const GET_USAGE: &str = "usage: qiq get [-u] [module[@version]]...
  qiq get github.com/ada/web          add or update a module to its latest version
  qiq get github.com/ada/web@v1.4.2   use a specific version (a git tag)
  qiq get -u                          update every module the main module requires directly
  qiq get                             download everything qiq.mod requires";

/// `qiq get [-u] [module[@version]]...`
pub fn command_get(args: impl Iterator<Item = String>) -> Result<(), String> {
    let mut update = false;
    let mut requested: Vec<(String, Option<String>)> = Vec::new();
    for arg in args {
        if arg == "-u" {
            update = true;
            continue;
        }
        if arg.starts_with('-') {
            return Err(format!("unknown option '{arg}'\n{GET_USAGE}"));
        }
        let (path, version) = match arg.split_once('@') {
            Some((path, version)) => (path.to_string(), Some(version.to_string())),
            None => (arg.clone(), None),
        };
        if !modfile::valid_module_path(&path, true) {
            return Err(format!(
                "invalid module path '{path}': a module path starts with a domain name, such as github.com/ada/web"
            ));
        }
        requested.push((path, version));
    }
    let (root, mut main, sums) = main_module()?;
    let mut direct: HashSet<String> = main.requires.iter().map(|(path, _)| path.clone()).filter(|path| !main.indirect.contains(path)).collect();
    if update && requested.is_empty() {
        requested = direct.iter().map(|path| (path.clone(), None)).collect();
        requested.sort();
    }
    let mut wanted: Vec<(String, Version)> = Vec::new();
    for (path, version) in &requested {
        if *path == main.module {
            return Err(format!("{path} is this module; it can't require itself"));
        }
        let version = match version.as_deref() {
            None | Some("latest") => latest(path)?,
            Some(text) => {
                let version = Version::parse(text)
                    .ok_or_else(|| format!("invalid version '{text}': expected a version tag such as v1.2.3, or latest"))?;
                let available = versions(path)?;
                if !available.contains(&version) {
                    let shown: Vec<String> = available.iter().rev().take(5).map(Version::to_string).collect();
                    return Err(if shown.is_empty() {
                        format!("{path} has no version tags; a module's versions are git tags such as v0.1.0")
                    } else {
                        format!("{path} has no version {version}; the newest are {}", shown.join(", "))
                    });
                }
                version
            }
        };
        match main.requires.iter_mut().find(|(required, _)| required == path) {
            Some(entry) => entry.1 = version.clone(),
            None => main.requires.push((path.clone(), version.clone())),
        }
        direct.insert(path.clone());
        wanted.push((path.clone(), version));
    }
    let before: HashMap<String, Version> = main.requires.iter().cloned().collect();
    let selected = resolve_and_save(&root, &main, &sums, &direct)?;
    for (path, (version, reason)) in &selected {
        match before.get(path) {
            None => eprintln!("qiq: added {path} {version} (required by {reason})"),
            Some(previous) if previous != version => eprintln!("qiq: {path} {previous} => {version} (required by {reason})"),
            _ => {}
        }
    }
    for (path, version) in wanted {
        let chosen = &selected[&path].0;
        if *chosen != version {
            eprintln!("qiq: using {path} {chosen}, not {version}, because {} requires it", selected[&path].1);
        } else {
            eprintln!("qiq: using {path} {version}");
        }
    }
    Ok(())
}

/// The import paths in a qiq file: the string after `from` in each import.
fn imports_of(source: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if !line.starts_with("import ") {
            continue;
        }
        if let Some(position) = line.rfind(" from ") {
            let rest = line[position + 6..].trim();
            if let Some(quoted) = rest.strip_prefix('"') {
                if let Some(end) = quoted.find('"') {
                    imports.push(quoted[..end].to_string());
                }
            }
        }
    }
    imports
}

/// The .qiq files of the module rooted at `root`, skipping hidden
/// directories and nested modules (directories with their own qiq.mod).
fn module_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = std::fs::read_dir(&directory).map_err(|error| format!("could not read {}: {error}", directory.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            if kind.is_dir() {
                if !name.starts_with('.') && !path.join(MOD_FILE).is_file() {
                    pending.push(path);
                }
            } else if kind.is_file() && name.ends_with(".qiq") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// `qiq mod tidy`: makes qiq.mod require exactly the modules the main
/// module's files import (and what those need), and prunes qiq.sum.
pub fn command_tidy() -> Result<(), String> {
    let (root, mut main, sums) = main_module()?;
    let required: Vec<String> = main.requires.iter().map(|(path, _)| path.clone()).collect();
    let mut direct = HashSet::new();
    let mut missing = Vec::new();
    for file in module_files(&root)? {
        let source = std::fs::read_to_string(&file).map_err(|error| format!("could not read {}: {error}", file.display()))?;
        for import in imports_of(&source) {
            let within = |module: &str| import == module || import.starts_with(&format!("{module}/"));
            if import.starts_with("std/") || import.starts_with('.') || within(&main.module) {
                continue;
            }
            match required.iter().filter(|module| within(module)).max_by_key(|module| module.len()) {
                Some(module) => {
                    direct.insert(module.clone());
                }
                None => {
                    let first = import.split('/').next().unwrap_or("");
                    let relative = file.parent().unwrap_or(&root).join(&import);
                    let exists = relative.is_file() || relative.with_extension("qiq").is_file();
                    if first.contains('.') && import.contains('/') && !exists {
                        let shown = file.strip_prefix(&root).unwrap_or(&file).display().to_string();
                        missing.push(format!("{shown} imports \"{import}\", which no required module provides"));
                    }
                }
            }
        }
    }
    if !missing.is_empty() {
        missing.sort();
        missing.dedup();
        return Err(format!(
            "{}\nAdd the module with 'qiq get <module path>' and run 'qiq mod tidy' again.",
            missing.join("\n")
        ));
    }
    let before: Vec<(String, Version)> = main.requires.clone();
    main.requires.retain(|(path, _)| direct.contains(path));
    let selected = resolve_and_save(&root, &main, &sums, &direct)?;
    for (path, version) in before {
        if !selected.contains_key(&path) {
            eprintln!("qiq: removed {path} {version}, which nothing imports");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_standard_encoding() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn sums_round_trip_and_reject_bad_lines() {
        let text = "github.com/a/b v1.0.0 h1:abc=\nexample.com/c v0.2.0 h1:def=\n";
        let sums = parse_sums(text, "qiq.sum").unwrap();
        assert_eq!(sums.len(), 2);
        assert_eq!(format_sums(&sums), "example.com/c v0.2.0 h1:def=\ngithub.com/a/b v1.0.0 h1:abc=\n");
        assert_eq!(parse_sums("x v1\n", "qiq.sum").unwrap_err(), "qiq.sum:1: expected '<module path> <version> h1:<hash>'");
    }

    #[test]
    fn finds_import_paths() {
        let source = "// x\nimport A, B as C from \"github.com/a/b/c\";\nimport D from \"std/file\";\nprint(1);";
        assert_eq!(imports_of(source), vec!["github.com/a/b/c", "std/file"]);
    }

    #[test]
    fn hashes_depend_on_names_and_contents() {
        let base = std::env::temp_dir().join(format!("qiq-hash-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("sub")).unwrap();
        std::fs::write(base.join("a.qiq"), "a").unwrap();
        std::fs::write(base.join("sub/b.qiq"), "b").unwrap();
        let first = hash_directory(&base).unwrap();
        assert!(first.starts_with("h1:") && first.len() == 47, "{first}");
        assert_eq!(hash_directory(&base).unwrap(), first);
        std::fs::write(base.join("sub/b.qiq"), "c").unwrap();
        let changed = hash_directory(&base).unwrap();
        assert_ne!(changed, first);
        std::fs::rename(base.join("sub/b.qiq"), base.join("sub/d.qiq")).unwrap();
        assert_ne!(hash_directory(&base).unwrap(), changed);
        std::fs::remove_dir_all(&base).unwrap();
    }
}
