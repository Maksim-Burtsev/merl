use std::path::{Path, PathBuf};

use regex::Regex;

use super::*;

pub const ERLANG_PREDEFINED: &[&str] = &[
    "MODULE",
    "MODULE_STRING",
    "LINE",
    "FILE",
    "FUNCTION_NAME",
    "FUNCTION_ARITY",
    "MACHINE",
    "OTP_RELEASE",
];

pub(super) const ERLANG_ATTRIBUTE_SYMBOL: &str =
    r"^-\s*(?:module|record|define|type|opaque)\s*\(?\s*(?P<name>[A-Za-z_]\w*)";
pub(super) const ERLANG_CLAUSE_SYMBOL: &str = r"^(?P<name>[a-z]\w*)\s*\(.*\)\s*(?:when\s.*)?->";

pub(super) fn erlang_patterns(w: &str) -> [String; 6] {
    [
        format!(r"^{w}\s*\("),
        erlang_module_pattern(w),
        erlang_record_pattern(w),
        format!(r"^-\s*record\s*\(\s*\w+\s*,.*[{{,]\s*{w}\s*(?:=|::|,|\}}|$)"),
        erlang_macro_pattern(w),
        erlang_type_pattern(w),
    ]
}

pub fn erlang_type_pattern(w: &str) -> String {
    format!(r"^-\s*(?:type|opaque)\s+{w}\s*\(")
}

pub fn erlang_module_pattern(w: &str) -> String {
    format!(r"^-\s*module\s*\(\s*{w}\s*\)")
}

pub fn erlang_record_pattern(w: &str) -> String {
    format!(r"^-\s*record\s*\(\s*{w}\s*,")
}

pub fn erlang_macro_pattern(w: &str) -> String {
    format!(r"^-\s*define\s*\(\s*{w}\s*[(,]")
}

pub fn erlang_clause_pattern(w: &str) -> String {
    format!(r"^{w}\s*\(")
}

pub(super) fn erlang_head(line: &str) -> bool {
    static HEAD: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^[a-z]\w*\s*\(").unwrap());
    HEAD.is_match(line)
}

pub(super) fn erlang_clause<S: AsRef<str>>(lines: &[S], line: usize) -> bool {
    let Some(first) = lines.get(line.wrapping_sub(1)).map(AsRef::as_ref) else {
        return false;
    };
    let Some(open) = first.find('(') else {
        return false;
    };
    let mut text = first[open..].to_owned();
    for l in lines.iter().skip(line).take(30) {
        text.push('\n');
        text.push_str(l.as_ref());
    }
    let b = text.as_bytes();
    let (mut depth, mut i) = (0usize, 0);
    while i < b.len() {
        match b[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let rest = skip_blank(&text[i + 1..]);
                    return rest.starts_with("->")
                        || rest.strip_prefix("when").is_some_and(|r| {
                            r.starts_with(|c: char| c.is_whitespace() || c == '(')
                        });
                }
            }
            b'$' => i += if b.get(i + 1) == Some(&b'\\') { 2 } else { 1 },
            b'%' => i += text[i..].find('\n').unwrap_or(text.len() - i),
            q @ (b'"' | b'\'') => i += quoted(&b[i..], q),
            b'.' if b.get(i + 1).is_none_or(|c| c.is_ascii_whitespace()) => return false,
            _ => {}
        }
        i += 1;
    }
    false
}

fn skip_blank(mut s: &str) -> &str {
    loop {
        s = s.trim_start();
        match s.strip_prefix('%') {
            Some(c) => s = c.split_once('\n').map_or("", |(_, r)| r),
            None => return s,
        }
    }
}

fn quoted(b: &[u8], q: u8) -> usize {
    let mut i = 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            c if c == q => return i,
            _ => {}
        }
        i += 1;
    }
    b.len()
}

pub fn file_literal_lines(kind: Kind, path: &Path, text: &str) -> Vec<bool> {
    match erlang(path) {
        true => erlang_literal_lines(text),
        false => literal_lines(kind, text),
    }
}

fn erlang_literal_lines(text: &str) -> Vec<bool> {
    let mut out = Vec::new();
    let mut inside: Option<&str> = None;
    for line in text.lines() {
        out.push(inside.is_some());
        let b = line.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if let Some(close) = inside {
                match line[i..].starts_with(close) {
                    true if close == "\"\"\"" => {
                        inside = None;
                        i += 3;
                        continue;
                    }
                    true => inside = None,
                    false if b[i] == b'\\' && close == "\"" => i += 1,
                    false => {}
                }
                i += 1;
                continue;
            }
            match b[i] {
                b'%' => break,
                b'$' => i += if b.get(i + 1) == Some(&b'\\') { 2 } else { 1 },
                b'\'' => i += quoted(&b[i..], b'\''),
                b'"' if line[i..].starts_with("\"\"\"") && line[i + 3..].trim().is_empty() => {
                    inside = Some("\"\"\"");
                    i += 2;
                }
                b'"' => inside = Some("\""),
                _ => {}
            }
            i += 1;
        }
    }
    out
}

pub fn erlang_imported(text: &str, word: &str) -> Option<String> {
    static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?m)^-\s*import\s*\(\s*([a-z]\w*)\s*,\s*\[([^\]]*)\]").unwrap()
    });
    let listed = Regex::new(&format!(r"(?:^|[\s,]){}\s*/", regex::escape(word))).ok()?;
    IMPORT
        .captures_iter(text)
        .find(|c| listed.is_match(&c[2]))
        .map(|c| c[1].to_owned())
}

pub fn erlang_include(line: &str, col: usize) -> Option<(bool, String)> {
    static INCLUDE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r#"^-\s*include(_lib)?\s*\(\s*"([^"]+)""#).unwrap());
    let c = INCLUDE.captures(line)?;
    let path = c.get(2)?;
    (path.start() <= col && col <= path.end())
        .then(|| (c.get(1).is_some(), path.as_str().to_owned()))
}

pub fn erlang_include_files(
    lib: bool,
    path: &str,
    here: &Path,
    files: &[PathBuf],
    outside: &[PathBuf],
) -> Vec<PathBuf> {
    let spelled = Path::new(path);
    if lib {
        let mut parts = spelled.components();
        let Some(app) = parts.next().and_then(|c| c.as_os_str().to_str()) else {
            return Vec::new();
        };
        let rest = parts.as_path();
        let of_app = |f: &&PathBuf| {
            f.ends_with(rest)
                && f.ancestors()
                    .nth(rest.components().count())
                    .and_then(|d| d.file_name()?.to_str())
                    .is_some_and(|d| {
                        d == app || d.strip_prefix(app).is_some_and(|v| v.starts_with('-'))
                    })
        };
        return files
            .iter()
            .chain(outside)
            .filter(of_app)
            .cloned()
            .collect();
    }
    let dir = here.parent().unwrap_or(Path::new(""));
    if let Some(beside) = lexical(&dir.join(spelled)).filter(|f| files.contains(f)) {
        return vec![beside];
    }
    let under = |f: &&PathBuf| {
        f.ends_with(spelled)
            && f.ancestors()
                .nth(spelled.components().count())
                .is_some_and(|d| d.ends_with("include"))
    };
    let found: Vec<PathBuf> = files.iter().filter(under).cloned().collect();
    match found.is_empty() {
        true => outside.iter().filter(under).cloned().collect(),
        false => found,
    }
}

pub fn erlang_record_field<S: AsRef<str>>(
    lines: &[S],
    record: usize,
    field: &str,
) -> Option<usize> {
    let re = Regex::new(&format!(
        r"(?:^|[{{,])\s*{}\s*(?:=[^=]|::|,|\}}|$)",
        regex::escape(field)
    ))
    .ok()?;
    let first = lines.get(record.checked_sub(1)?)?.as_ref();
    let open = first.find('{').unwrap_or(first.len());
    for (i, l) in lines.iter().enumerate().skip(record - 1) {
        let l = l.as_ref();
        let l = match i + 1 == record {
            true => &l[open..],
            false => l,
        };
        let code = l.split('%').next().unwrap_or("");
        if re.is_match(code) {
            return Some(i + 1);
        }
        if code.trim_end().ends_with(").") {
            return None;
        }
    }
    None
}

pub fn rebar_deps(root: &Path, file: &Path) -> Vec<PathBuf> {
    file.ancestors()
        .take_while(|dir| dir.starts_with(root))
        .flat_map(|dir| {
            [
                (dir.join("rebar.config"), dir.join("_build/default/lib")),
                (dir.join("erlang.mk"), dir.join("deps")),
            ]
        })
        .filter(|(made_by, deps)| made_by.is_file() && deps.is_dir())
        .map(|(_, deps)| deps)
        .collect()
}

pub(super) fn otp_roots(root_dir: Option<String>) -> Vec<PathBuf> {
    root_dir
        .map(|r| Path::new(r.trim()).join("lib"))
        .into_iter()
        .collect()
}
