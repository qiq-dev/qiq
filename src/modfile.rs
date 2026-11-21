//! qiq.mod files, which make a directory a module (a package of qiq files
//! that other modules can import):
//!
//!     module github.com/asim/myapp
//!
//!     qiq 0.1
//!
//!     require github.com/ada/web v1.4.2
//!     require (
//!         github.com/lin/slug v0.3.0
//!     )
//!
//!     replace github.com/ada/web => ../web
//!
//! `module` names the module: its import path. `qiq` is the oldest compiler
//! version the module works with. `require` lists the modules it imports
//! and the minimum version of each. `replace` uses a local directory instead
//! of a downloaded module (only in the main module's qiq.mod).

use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;

pub const MOD_FILE: &str = "qiq.mod";

/// A semantic version: v1.4.2, or v2.0.0-beta.1.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    /// The prerelease part after "-", or empty for a release.
    prerelease: String,
}

impl Version {
    pub fn is_prerelease(&self) -> bool {
        !self.prerelease.is_empty()
    }

    pub fn parse(text: &str) -> Option<Version> {
        let rest = text.strip_prefix('v')?;
        let (core, prerelease) = match rest.split_once('-') {
            Some((core, prerelease)) => (core, prerelease),
            None => (rest, ""),
        };
        let numbers: Vec<&str> = core.split('.').collect();
        if numbers.len() != 3 {
            return None;
        }
        let number = |text: &str| {
            let valid = !text.is_empty()
                && text.bytes().all(|b| b.is_ascii_digit())
                && (text == "0" || !text.starts_with('0'));
            if valid { text.parse::<u64>().ok() } else { None }
        };
        let prerelease_valid = !rest.contains('-')
            || (!prerelease.is_empty()
                && prerelease
                    .split('.')
                    .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')));
        if !prerelease_valid {
            return None;
        }
        Some(Version {
            major: number(numbers[0])?,
            minor: number(numbers[1])?,
            patch: number(numbers[2])?,
            prerelease: prerelease.to_string(),
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.prerelease.is_empty() {
            write!(f, "-{}", self.prerelease)?;
        }
        Ok(())
    }
}

impl Ord for Version {
    /// Semantic version precedence: a prerelease comes before its release,
    /// and prerelease parts compare numerically when both are numbers.
    fn cmp(&self, other: &Self) -> Ordering {
        let core = (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch));
        if core != Ordering::Equal {
            return core;
        }
        match (self.prerelease.is_empty(), other.prerelease.is_empty()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            (false, false) => {}
        }
        let mut left = self.prerelease.split('.');
        let mut right = other.prerelease.split('.');
        loop {
            match (left.next(), right.next()) {
                (None, None) => return Ordering::Equal,
                (None, Some(_)) => return Ordering::Less,
                (Some(_), None) => return Ordering::Greater,
                (Some(a), Some(b)) => {
                    let order = match (a.parse::<u64>(), b.parse::<u64>()) {
                        (Ok(a), Ok(b)) => a.cmp(&b),
                        (Ok(_), Err(_)) => Ordering::Less,
                        (Err(_), Ok(_)) => Ordering::Greater,
                        (Err(_), Err(_)) => a.cmp(b),
                    };
                    if order != Ordering::Equal {
                        return order;
                    }
                }
            }
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Default)]
pub struct ModFile {
    pub module: String,
    /// The `qiq` line: the oldest compiler version (major.minor) it needs.
    pub qiq: Option<(u64, u64)>,
    pub requires: Vec<(String, Version)>,
    /// Module path -> local directory (relative to the qiq.mod's directory).
    pub replaces: Vec<(String, String)>,
    /// Requirements marked `// indirect`: needed by other required modules,
    /// not imported by this one.
    pub indirect: HashSet<String>,
}

/// Whether `path` is a valid module path. Modules that can be required must
/// start with a domain name (`github.com/ada/web`), because that's where
/// they're downloaded from; a main module can be named anything (`myapp`).
pub fn valid_module_path(path: &str, needs_domain: bool) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    let segments_valid = segments.iter().all(|segment| {
        !segment.is_empty()
            && *segment != "."
            && *segment != ".."
            && segment.bytes().all(|b| b.is_ascii_alphanumeric() || b"._~-".contains(&b))
    });
    let domain = segments[0].contains('.') && !segments[0].starts_with('.') && !segments[0].ends_with('.');
    segments_valid && (!needs_domain || domain)
}

/// The compiler's own version as (major, minor).
pub fn compiler_version() -> (u64, u64) {
    let mut parts = env!("CARGO_PKG_VERSION").split('.').map(|part| part.parse::<u64>().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0))
}

/// Parses a qiq.mod file; errors are prefixed with `name:line: `.
pub fn parse(text: &str, name: &str) -> Result<ModFile, String> {
    let mut file = ModFile::default();
    // Inside `require (` or `replace (`: which one.
    let mut block: Option<&str> = None;
    for (index, raw) in text.lines().enumerate() {
        let line_number = index + 1;
        let error = |message: String| format!("{name}:{line_number}: {message}");
        let (line, comment) = match raw.find("//") {
            Some(position) => (&raw[..position], raw[position + 2..].trim()),
            None => (raw, ""),
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (directive, arguments) = match block {
            Some(directive) => {
                if line == ")" {
                    block = None;
                    continue;
                }
                (directive, line)
            }
            None => {
                let (directive, arguments) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
                let arguments = arguments.trim();
                if arguments == "(" && (directive == "require" || directive == "replace") {
                    block = Some(if directive == "require" { "require" } else { "replace" });
                    continue;
                }
                (directive, arguments)
            }
        };
        let words: Vec<&str> = arguments.split_whitespace().collect();
        match directive {
            "module" => {
                if !file.module.is_empty() {
                    return Err(error("'module' is repeated".to_string()));
                }
                if words.len() != 1 || !valid_module_path(words[0], false) {
                    return Err(error(format!(
                        "invalid module path '{arguments}': use letters, digits, and . _ ~ -, separated by /, such as github.com/you/project"
                    )));
                }
                file.module = words[0].to_string();
            }
            "qiq" => {
                let version = words.first().and_then(|word| {
                    let (major, minor) = word.split_once('.')?;
                    Some((major.parse::<u64>().ok()?, minor.parse::<u64>().ok()?))
                });
                match version {
                    Some(version) if words.len() == 1 && file.qiq.is_none() => file.qiq = Some(version),
                    _ => return Err(error(format!("invalid qiq version '{arguments}': expected a version such as 0.1"))),
                }
            }
            "require" => {
                if words.len() != 2 {
                    return Err(error("expected 'require <module path> <version>'".to_string()));
                }
                if !valid_module_path(words[0], true) {
                    return Err(error(format!(
                        "invalid module path '{}': a required module's path starts with a domain name, such as github.com/ada/web",
                        words[0]
                    )));
                }
                let version = Version::parse(words[1]).ok_or_else(|| {
                    error(format!("invalid version '{}': expected a version such as v1.2.3", words[1]))
                })?;
                if file.requires.iter().any(|(path, _)| path == words[0]) {
                    return Err(error(format!("{} is required more than once", words[0])));
                }
                if comment == "indirect" {
                    file.indirect.insert(words[0].to_string());
                }
                file.requires.push((words[0].to_string(), version));
            }
            "replace" => {
                if words.len() != 3 || words[1] != "=>" {
                    return Err(error("expected 'replace <module path> => <directory>'".to_string()));
                }
                let target = words[2];
                if !(target.starts_with("./") || target.starts_with("../") || target.starts_with('/') || target == "." || target == "..") {
                    return Err(error(format!(
                        "replace target '{target}' must be a local directory starting with ./, ../, or /"
                    )));
                }
                if file.replaces.iter().any(|(path, _)| path == words[0]) {
                    return Err(error(format!("{} is replaced more than once", words[0])));
                }
                file.replaces.push((words[0].to_string(), target.to_string()));
            }
            _ => return Err(error(format!("unknown directive '{directive}': expected module, qiq, require, or replace"))),
        }
    }
    if block.is_some() {
        return Err(format!("{name}: missing ')' to end a block"));
    }
    if file.module.is_empty() {
        return Err(format!("{name}: missing 'module' line naming the module"));
    }
    if let Some((major, minor)) = file.qiq {
        let (have_major, have_minor) = compiler_version();
        if (major, minor) > (have_major, have_minor) {
            return Err(format!(
                "{name}: the module needs qiq {major}.{minor} or newer, but this is qiq {}",
                env!("CARGO_PKG_VERSION")
            ));
        }
    }
    for (path, _) in &file.replaces {
        if !file.requires.iter().any(|(required, _)| required == path) {
            return Err(format!("{name}: {path} is replaced but not required; add 'require {path} <version>'"));
        }
    }
    Ok(file)
}

/// The text of a new qiq.mod.
pub fn new_file(module: &str) -> String {
    let (major, minor) = compiler_version();
    format!("module {module}\n\nqiq {major}.{minor}\n")
}

/// Where downloaded modules are kept: $QIQ_MODCACHE, or qiq/mod in the
/// user's cache directory.
pub fn cache_directory() -> PathBuf {
    if let Some(directory) = std::env::var_os("QIQ_MODCACHE").filter(|value| !value.is_empty()) {
        return PathBuf::from(directory);
    }
    if let Some(directory) = std::env::var_os("XDG_CACHE_HOME").filter(|value| !value.is_empty()) {
        return PathBuf::from(directory).join("qiq").join("mod");
    }
    let home = std::env::var_os("HOME").unwrap_or_default();
    PathBuf::from(home).join(".cache").join("qiq").join("mod")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_order_like_semver() {
        let order = ["v0.1.0", "v0.9.0", "v0.10.0", "v1.0.0-alpha", "v1.0.0-alpha.2", "v1.0.0-alpha.10", "v1.0.0-beta", "v1.0.0", "v1.0.1", "v2.0.0"];
        let versions: Vec<Version> = order.iter().map(|text| Version::parse(text).unwrap()).collect();
        for pair in versions.windows(2) {
            assert!(pair[0] < pair[1], "{} < {}", pair[0], pair[1]);
        }
        for bad in ["1.0.0", "v1.0", "v1.0.0.0", "v01.0.0", "v1.0.0-", "v1.0.0-a..b", "vx.y.z"] {
            assert!(Version::parse(bad).is_none(), "{bad}");
        }
        assert_eq!(Version::parse("v1.2.3-rc.1").unwrap().to_string(), "v1.2.3-rc.1");
    }

    #[test]
    fn parses_mod_files() {
        let file = parse(
            "// my app\nmodule github.com/asim/myapp\n\nqiq 0.1\n\nrequire github.com/ada/web v1.4.2\nrequire (\n    github.com/lin/slug v0.3.0 // slugs\n)\nreplace github.com/ada/web => ../web\n",
            "qiq.mod",
        )
        .unwrap();
        assert_eq!(file.module, "github.com/asim/myapp");
        assert_eq!(file.qiq, Some((0, 1)));
        assert_eq!(file.requires.len(), 2);
        assert_eq!(file.requires[1].0, "github.com/lin/slug");
        assert_eq!(file.replaces, vec![("github.com/ada/web".to_string(), "../web".to_string())]);
        assert!(file.indirect.is_empty());
        let file = parse("module app\nrequire (\n    x.com/a v1.0.0\n    x.com/b v1.0.0 // indirect\n)\n", "qiq.mod").unwrap();
        assert_eq!(file.indirect.iter().collect::<Vec<_>>(), vec!["x.com/b"]);

        let errors = [
            ("qiq 0.1\n", "qiq.mod: missing 'module' line naming the module"),
            ("module a b\n", "qiq.mod:1: invalid module path 'a b'"),
            ("module app\nrequire web v1.0.0\n", "qiq.mod:2: invalid module path 'web'"),
            ("module app\nrequire x.com/web 1.0\n", "qiq.mod:2: invalid version '1.0'"),
            ("module app\nrequire x.com/a v1.0.0\nrequire x.com/a v1.1.0\n", "qiq.mod:3: x.com/a is required more than once"),
            ("module app\nreplace x.com/a => ../a\n", "qiq.mod: x.com/a is replaced but not required"),
            ("module app\nrequire x.com/a v1.0.0\nreplace x.com/a => elsewhere\n", "qiq.mod:3: replace target 'elsewhere'"),
            ("module app\nrequire (\nx.com/a v1.0.0\n", "qiq.mod: missing ')'"),
            ("module app\nqiq 99.0\n", "qiq.mod: the module needs qiq 99.0 or newer"),
            ("module app\nversion 1\n", "qiq.mod:2: unknown directive 'version'"),
        ];
        for (text, expected) in errors {
            let error = parse(text, "qiq.mod").unwrap_err();
            assert!(error.starts_with(expected), "{text:?}: {error}");
        }
    }
}
