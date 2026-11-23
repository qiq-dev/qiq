//! Checks regular expressions written as literals at compile time, so a
//! malformed pattern passed to a std/regex function is a compile error, not
//! a RegexException when the line finally runs.
//!
//! It uses the same PCRE2 as the runtime (built for the compiler's machine
//! by build.rs), reads delimiters and flags the same way as
//! stdlib/runtime/regex.c, and reads replacements the same way as
//! __regex_template in stdlib/regex.qiq, with the same messages, so an
//! error reads the same whether the compiler or the program finds it.

use std::ffi::c_void;
use std::os::raw::c_int;

const PCRE2_ANCHORED: u32 = 0x8000_0000;
const PCRE2_CASELESS: u32 = 0x0000_0008;
const PCRE2_DOLLAR_ENDONLY: u32 = 0x0000_0010;
const PCRE2_DOTALL: u32 = 0x0000_0020;
const PCRE2_DUPNAMES: u32 = 0x0000_0040;
const PCRE2_EXTENDED: u32 = 0x0000_0080;
const PCRE2_MULTILINE: u32 = 0x0000_0400;
const PCRE2_NO_AUTO_CAPTURE: u32 = 0x0000_2000;
const PCRE2_UCP: u32 = 0x0002_0000;
const PCRE2_UNGREEDY: u32 = 0x0004_0000;
const PCRE2_UTF: u32 = 0x0008_0000;
const PCRE2_INFO_CAPTURECOUNT: u32 = 4;
const PCRE2_INFO_NAMECOUNT: u32 = 17;
const PCRE2_INFO_NAMEENTRYSIZE: u32 = 18;
const PCRE2_INFO_NAMETABLE: u32 = 19;

extern "C" {
    fn pcre2_compile_8(
        pattern: *const u8,
        length: usize,
        options: u32,
        error_code: *mut c_int,
        error_offset: *mut usize,
        context: *mut c_void,
    ) -> *mut c_void;
    fn pcre2_code_free_8(code: *mut c_void);
    fn pcre2_get_error_message_8(error_code: c_int, buffer: *mut u8, length: usize) -> c_int;
    fn pcre2_pattern_info_8(code: *const c_void, what: u32, result: *mut c_void) -> c_int;
}

/// What the compiler learns from a valid pattern: the name of each group,
/// the whole match first ("" for a group without a name).
pub struct Pattern {
    pub names: Vec<String>,
}

/// Reads a PHP-style pattern as regex.c's qiq_regex_split does: the
/// expression between the delimiters, and the PCRE2 options for its flags.
fn split(pattern: &str) -> Result<(&str, u32), String> {
    let bytes = pattern.as_bytes();
    let mut start = 0;
    while start < bytes.len() && matches!(bytes[start], b' ' | b'\t' | b'\n' | b'\r') {
        start += 1;
    }
    if start == bytes.len() {
        return Err("empty regular expression; write it between delimiters, as in `/\\d+/`".into());
    }
    let open = bytes[start];
    if open.is_ascii_alphanumeric() || open == b'\\' {
        return Err(format!(
            "the delimiter can't be a letter, digit, or backslash: write the pattern between delimiters, as in \"/{pattern}/\""
        ));
    }
    if open >= 0x80 {
        return Err("the delimiter must be an ASCII punctuation character, such as /".into());
    }
    let close = match open {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        b'<' => b'>',
        other => other,
    };
    let mut index = start + 1;
    let mut depth = 1;
    while index < bytes.len() {
        let c = bytes[index];
        if c == b'\\' && index + 1 < bytes.len() {
            index += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                break;
            }
        } else if c == open && open != close {
            depth += 1;
        }
        index += 1;
    }
    if index >= bytes.len() {
        return Err(format!("no ending delimiter '{}' in {pattern}", close as char));
    }
    let body = &pattern[start + 1..index];
    let mut options = PCRE2_UTF | PCRE2_UCP;
    for &flag in &bytes[index + 1..] {
        options |= match flag {
            b'i' => PCRE2_CASELESS,
            b'm' => PCRE2_MULTILINE,
            b's' => PCRE2_DOTALL,
            b'x' => PCRE2_EXTENDED,
            b'U' => PCRE2_UNGREEDY,
            b'D' => PCRE2_DOLLAR_ENDONLY,
            b'A' => PCRE2_ANCHORED,
            b'n' => PCRE2_NO_AUTO_CAPTURE,
            b'J' => PCRE2_DUPNAMES,
            b'u' | b' ' | b'\n' | b'\r' | b'\t' => 0,
            other => {
                // The flag as a character, which may take several bytes.
                let at = bytes[index + 1..].iter().position(|&b| b == other).unwrap() + index + 1;
                let shown = pattern[at..].chars().next().unwrap_or('?');
                return Err(format!(
                    "unknown flag '{shown}' in {pattern}; the flags are i, m, s, x, u, U, D, A, n, and J"
                ));
            }
        };
    }
    Ok((body, options))
}

/// Compiles `pattern` as the runtime would, and returns its groups, or the
/// error the runtime would throw.
pub fn check_pattern(pattern: &str) -> Result<Pattern, String> {
    let (body, options) = split(pattern)?;
    let mut error: c_int = 0;
    let mut offset: usize = 0;
    // SAFETY: the body is valid UTF-8 for its length; PCRE2 copies what it
    // needs, and the code is freed below.
    let code = unsafe { pcre2_compile_8(body.as_ptr(), body.len(), options, &mut error, &mut offset, std::ptr::null_mut()) };
    if code.is_null() {
        let mut buffer = [0u8; 256];
        // SAFETY: PCRE2 writes a NUL-terminated message of at most the
        // buffer's length.
        let length = unsafe { pcre2_get_error_message_8(error, buffer.as_mut_ptr(), buffer.len()) };
        let message = String::from_utf8_lossy(&buffer[..length.max(0) as usize]).into_owned();
        let at = body.get(..offset).map(|text| text.chars().count()).unwrap_or(offset);
        return Err(format!("invalid regular expression {pattern}: {message} at offset {at}"));
    }
    let mut groups: u32 = 0;
    let mut count: u32 = 0;
    let mut entry_size: u32 = 0;
    let mut table: *const u8 = std::ptr::null();
    // SAFETY: each call writes one value of the type PCRE2 documents for it.
    unsafe {
        pcre2_pattern_info_8(code, PCRE2_INFO_CAPTURECOUNT, &mut groups as *mut u32 as *mut c_void);
        pcre2_pattern_info_8(code, PCRE2_INFO_NAMECOUNT, &mut count as *mut u32 as *mut c_void);
        pcre2_pattern_info_8(code, PCRE2_INFO_NAMEENTRYSIZE, &mut entry_size as *mut u32 as *mut c_void);
        pcre2_pattern_info_8(code, PCRE2_INFO_NAMETABLE, &mut table as *mut *const u8 as *mut c_void);
    }
    let mut names = vec![String::new(); groups as usize + 1];
    for index in 0..count as usize {
        // SAFETY: the table has `count` entries of `entry_size` bytes: a
        // two-byte group number, then the NUL-terminated name.
        let entry = unsafe { std::slice::from_raw_parts(table.add(index * entry_size as usize), entry_size as usize) };
        let group = ((entry[0] as usize) << 8) | entry[1] as usize;
        let name = &entry[2..];
        let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
        if group < names.len() {
            names[group] = String::from_utf8_lossy(&name[..end]).into_owned();
        }
    }
    // SAFETY: `code` came from pcre2_compile and isn't used again.
    unsafe { pcre2_code_free_8(code) };
    Ok(Pattern { names })
}

/// Checks a preg_replace replacement against the pattern's groups, as
/// __regex_template does at run time.
pub fn check_replacement(replacement: &str, pattern: &Pattern) -> Result<(), String> {
    let chars: Vec<char> = replacement.chars().collect();
    let groups = pattern.names.len();
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        let next = chars.get(index + 1).copied();
        if c == '$' && next == Some('$') {
            index += 2;
            continue;
        }
        let group;
        if (c == '$' || c == '\\') && next.is_some_and(|d| d.is_ascii_digit()) {
            let mut number = next.unwrap().to_digit(10).unwrap() as usize;
            index += 2;
            if let Some(d) = chars.get(index).and_then(|d| d.to_digit(10)) {
                number = number * 10 + d as usize;
                index += 1;
            }
            group = number;
        } else if c == '$' && next == Some('{') {
            let close = chars[index + 2..].iter().position(|&d| d == '}').map(|at| at + index + 2);
            let Some(close) = close else {
                return Err("the replacement has a '${' without a closing '}'".into());
            };
            let reference: String = chars[index + 2..close].iter().collect();
            index = close + 1;
            group = if !reference.is_empty() && reference.chars().all(|d| d.is_ascii_digit()) {
                reference.parse().unwrap_or(usize::MAX)
            } else {
                match pattern.names.iter().position(|name| !reference.is_empty() && *name == reference) {
                    Some(number) => number,
                    None => {
                        return Err(format!(
                            "the replacement refers to a group named '{reference}', which the pattern doesn't have"
                        ))
                    }
                }
            };
        } else {
            index += 1;
            continue;
        }
        if group >= groups {
            return Err(format!(
                "the replacement refers to group {group}, but the pattern has {} group(s)",
                groups - 1
            ));
        }
    }
    Ok(())
}
