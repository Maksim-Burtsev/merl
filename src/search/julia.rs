use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::*;

macro_rules! julia_macros {
    () => {
        r"(?:(?:[\w.]+\.)?@[\w.]+\s+)*"
    };
}

macro_rules! julia_method_tail {
    () => {
        r"\s*(?:::\s*[^=\s][^=]*?)?\s*(?:where\b[^=]*)?=(?:[^=>]|$)"
    };
}

macro_rules! julia_name_end {
    () => {
        r"(?:[^\w!.]|$)"
    };
}

const MACROS: &str = julia_macros!();
const END: &str = julia_name_end!();
const QUALIFIER: &str = r"(?:[\w.]+\.)?";
const TYPE_PARAMS: &str = r"(?:\{[^}]*\}\s*)?";

pub fn julia_patterns(word: &str) -> Vec<String> {
    let (w, e) = (regex::escape(word), END);
    vec![
        format!(r"^\s*{MACROS}function\s+{QUALIFIER}{w}{e}"),
        format!(r"^\s*{MACROS}{QUALIFIER}{w}\s*{TYPE_PARAMS}\("),
        format!(r"^\s*{MACROS}(?:mutable\s+)?struct\s+{w}{e}"),
        format!(r"^\s*(?:abstract|primitive)\s+type\s+{w}{e}"),
        julia_macro(word),
        format!(r"^\s*(?:bare)?module\s+{w}{e}"),
        format!(r"^\s*(?:global\s+)?const\s+{w}\s*(?:::[^=]*)?=(?:[^=]|$)"),
        format!(r"^{w}\s*(?:::[^=]*)?=(?:[^=>]|$)"),
        julia_field(word),
        format!(r"^\s*@enum\s+(?:{w}{e}|[\w.]+(?:::[\w.]+)?\s+(?:[\w\s=]*\s)?{w}{e})"),
    ]
}

pub fn julia_narrow(patterns: &mut Vec<String>, line: &str, r: std::ops::Range<usize>) {
    let (word, before, after) = (&line[r.clone()], &line[..r.start], &line[r.end..]);
    let matches = |p: &str| Regex::new(p).is_ok_and(|re| re.is_match(line));
    let (field, mac) = (julia_field(word), julia_macro(word));
    if matches(&mac) {
        return;
    }
    if before.ends_with('@') {
        patterns.retain(|p| *p == mac);
        return;
    }
    patterns.retain(|p| *p != mac);
    let called = after.starts_with(['(', '{']) || after.starts_with(".(");
    if called || (!before.ends_with('.') && !matches(&field)) {
        patterns.retain(|p| *p != field);
    }
}

fn julia_field(word: &str) -> String {
    format!(
        r"^\s+{}\s*(?:::[^=]*)?(?:=(?:[^=>].*)?)?$",
        regex::escape(word)
    )
}

fn julia_macro(word: &str) -> String {
    format!(r"^\s*macro\s+{}{END}", regex::escape(word))
}

pub(super) const JULIA_FUNCTION_SYMBOL: &str = concat!(
    r"^\s*",
    julia_macros!(),
    r"function\s+(?:[\w.]+\.)?(?P<name>[A-Za-z_][\w!]*)",
    julia_name_end!()
);
pub(super) const JULIA_METHOD_SYMBOL: &str = concat!(
    r"^(?:[\w.]+\.)?(?P<name>[A-Za-z_][\w!]*)(?:\{[^}]*\})?",
    r"\((?:[^()]|\((?:[^()]|\([^()]*\))*\))*\)",
    julia_method_tail!()
);
pub(super) const JULIA_TYPE_SYMBOL: &str = concat!(
    r"^\s*",
    julia_macros!(),
    r"(?:(?:mutable\s+)?struct|(?:abstract|primitive)\s+type|macro|(?:bare)?module)",
    r"\s+(?P<name>[A-Za-z_][\w!]*)"
);

static STRUCT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^\s*{MACROS}(?:mutable\s+)?struct\s")).unwrap());
static KEYWORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^\s*(?:{MACROS}(?:function|(?:mutable\s+)?struct)|abstract\s+type|primitive\s+type|macro|(?:bare)?module|(?:global\s+)?const|@enum)\s"
    ))
    .unwrap()
});
static HEAD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^\s*{MACROS}{QUALIFIER}([A-Za-z_][\w!]*)\s*{TYPE_PARAMS}\("
    ))
    .unwrap()
});

pub fn julia_declares<S: AsRef<str>>(lines: &[S], line: usize, word: &str) -> bool {
    let Some(text) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let text = text.as_ref();
    if KEYWORD.is_match(text) {
        return true;
    }
    if let Some(c) = HEAD.captures(text).filter(|c| &c[1] == word) {
        return defines(&text[c.get(0).map_or(0, |m| m.end() - 1)..]);
    }
    if !text.starts_with([' ', '\t']) {
        return true;
    }
    let depth = indent(text);
    let owner = |code: &[bool]| {
        (0..line - 1)
            .rev()
            .map(|i| (i, lines[i].as_ref()))
            .find(|&(i, l)| {
                !code.get(i).copied().unwrap_or(false)
                    && !l.trim().is_empty()
                    && !l.trim_start().starts_with('#')
                    && indent(l) < depth
            })
            .is_some_and(|(_, l)| STRUCT.is_match(l))
    };
    let text: Vec<&str> = lines.iter().map(AsRef::as_ref).collect();
    owner(&[])
        || (text
            .iter()
            .any(|l| l.contains("\"\"\"") || l.contains("#=") || l.contains('`'))
            && owner(&julia_literal_lines(&text.join("\n"))))
}

fn defines(from_paren: &str) -> bool {
    static REST: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(concat!("^", julia_method_tail!())).unwrap());
    closing(from_paren).is_some_and(|close| REST.is_match(&from_paren[close + 1..]))
}

fn closing(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

pub fn julia_include(line: &str, col: usize) -> Option<String> {
    static INCLUDE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"\binclude\s*\(\s*"([^"$]+)"\s*\)"#).unwrap());
    INCLUDE
        .captures_iter(line)
        .find(|c| c.get(0).is_some_and(|m| m.start() <= col && col < m.end()))
        .map(|c| c[1].to_owned())
}

pub(super) fn julia_imports(text: &str, out: &mut Vec<(String, Vec<String>)>) {
    static IMPORT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?m)^\s*(?:using|import)\s+([^\n#]+)").unwrap());
    let path = |m: &str| -> Vec<String> {
        let m = m.trim();
        let dots = m.len() - m.trim_start_matches('.').len();
        let mut parts: Vec<String> = m[dots..]
            .split('.')
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect();
        if dots > 0 {
            parts.insert(0, ".".repeat(dots));
        }
        parts
    };
    let named = |item: &str| -> Option<(String, String)> {
        let mut it = item.split_whitespace();
        let name = it.next()?.trim_start_matches('@').to_owned();
        let alias = match (it.next(), it.next()) {
            (Some("as"), Some(a)) => a.trim_start_matches('@').to_owned(),
            _ => name.clone(),
        };
        (!name.is_empty()).then_some((alias, name))
    };
    for c in IMPORT.captures_iter(text) {
        let clause = c[1].trim().trim_end_matches(';');
        match clause.split_once(':').filter(|(_, r)| !r.starts_with(':')) {
            Some((module, items)) => {
                let base = path(module);
                for (alias, name) in items.split(',').filter_map(|i| named(i.trim())) {
                    let mut p = base.clone();
                    p.push(name);
                    out.push((alias, p));
                }
            }
            None => {
                for item in clause.split(',') {
                    let mut it = item.split_whitespace();
                    let Some(module) = it.next() else { continue };
                    let p = path(module);
                    let alias = match (it.next(), it.next()) {
                        (Some("as"), Some(a)) => a.to_owned(),
                        _ => match p.last() {
                            Some(last) if !last.starts_with('.') => last.clone(),
                            _ => continue,
                        },
                    };
                    out.push((alias, p));
                }
            }
        }
    }
}

pub(super) fn julia_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let binding = |line: usize| {
        vec![Binding {
            line: line + 1,
            value: Value::Unknown,
        }]
    };
    let w = regex::escape(name);
    let generator = Regex::new(&format!(
        r"\bfor\s+\(?(?:[\w!]+\s*,\s*)*{w}(?:\s*,\s*[\w!]+)*\)?\s*(?:in\b|=|∈)"
    ))
    .expect("an escaped name keeps the pattern valid");
    let lambda = Regex::new(&format!(
        r"(?:^|[^\w!])(?:{w}|\((?:[^()]*[,\s])?{w}(?:[,\s:=][^()]*)?\))\s*->"
    ))
    .expect("an escaped name keeps the pattern valid");
    if generator.is_match(lines[at])
        || lambda.is_match(lines[at])
        || method_head(lines[at]).is_some_and(|h| params(h).iter().any(|p| p == name))
    {
        return binding(at);
    }
    let literal = julia_literal_lines(&lines.join("\n"));
    let assigns = assignment(name);
    let code = |i: usize| !literal.get(i).copied().unwrap_or(false);
    let mut depth = indent(lines[at]);
    let mut body_end = at;
    let mut outer = None;
    for k in (0..=at).rev() {
        let l = lines[k];
        if !code(k) || l.trim().is_empty() || l.trim_start().starts_with('#') {
            continue;
        }
        let header = HEADER.is_match(l) || method_def(lines, k);
        if k != at && indent(l) >= depth {
            continue;
        }
        if k == at && !header {
            continue;
        }
        let body = std::mem::replace(&mut depth, indent(l));
        if k != at && generator.is_match(l) {
            let assigned = (k + 1..body_end)
                .rev()
                .find(|&i| code(i) && assigns(lines[i]));
            return binding(assigned.unwrap_or(k));
        }
        if !header && k != at {
            outer = (k + 1..body_end)
                .find(|&i| code(i) && indent(lines[i]) == body && assigns(lines[i]))
                .or(outer);
            if outer.is_some() && l.trim_start().starts_with('@') {
                break;
            }
        }
        if header {
            if let Some(i) = (k + 1..body_end)
                .rev()
                .find(|&i| code(i) && assigns(lines[i]))
            {
                return binding(i);
            }
            let head = lines[k..lines.len().min(k + 20)].join("\n");
            if params(&head).iter().any(|p| p == name) {
                return binding(k);
            }
            body_end = k;
        }
        if depth == 0 {
            break;
        }
    }
    outer.map(binding).unwrap_or_default()
}

fn assignment(name: &str) -> impl Fn(&str) -> bool {
    let w = regex::escape(name);
    let re = Regex::new(&format!(
        r"^\s*(?:local\s+)?\(?(?:[\w!]+\s*,\s*)*{w}\s*(?:::[^=,]*)?(?:,\s*[\w!]+\s*)*\)?\s*=(?:[^=>]|$)|^\s*for\s+\(?(?:[\w!]+\s*,\s*)*{w}(?:\s*,\s*[\w!]+)*\)?\s*(?:in\b|=|∈)"
    ))
    .expect("an escaped name keeps the pattern valid");
    move |l: &str| {
        let code = l.split_once('#').map_or(l, |(c, _)| c).trim_end();
        re.is_match(l)
            && !code.ends_with(',')
            && code.matches(['(', '[']).count() >= code.matches([')', ']']).count()
    }
}

pub fn julia_assigns_here(line: &str, start: usize, word: &str) -> bool {
    assignment(word)(line)
        && line
            .match_indices('=')
            .find(|&(i, _)| {
                !line[i + 1..].starts_with(['=', '>']) && !line[..i].ends_with(['=', '!', '<', '>'])
            })
            .is_some_and(|(i, _)| start < i)
}

static HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^\s*{MACROS}(?:function|macro)\s|\bdo(?:\s+[^#]*)?$"
    ))
    .unwrap()
});

fn method_def(lines: &[&str], k: usize) -> bool {
    HEAD.find(lines[k])
        .is_some_and(|m| defines(&lines[k..lines.len().min(k + 20)].join("\n")[m.end() - 1..]))
}

fn method_head(line: &str) -> Option<&str> {
    let m = HEAD.find(line)?;
    defines(&line[m.end() - 1..]).then_some(line)
}

fn params(head: &str) -> Vec<String> {
    static NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z_][\w!]*").unwrap());
    static DO: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\bdo(?:\s+([^#]*))?\s*(?:#.*)?$").unwrap());
    let first = head.lines().next().unwrap_or_default();
    if let Some(c) = DO.captures(first) {
        return c
            .get(1)
            .map_or("", |m| m.as_str())
            .split(',')
            .filter_map(|a| NAME.find(a.trim().trim_start_matches('(')))
            .map(|m| m.as_str().to_owned())
            .collect();
    }
    let mut out = Vec::new();
    let mut rest = match head.find('(') {
        Some(i) => &head[i..],
        None => return out,
    };
    while rest.starts_with('(') {
        let Some(close) = closing(rest) else { break };
        let mut depth = 0usize;
        let mut start = 1;
        let inner = &rest[..close];
        for (i, c) in inner.char_indices() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                ',' | ';' if depth == 1 => {
                    out.extend(
                        NAME.find(inner[start..i].trim())
                            .map(|m| m.as_str().to_owned()),
                    );
                    start = i + 1;
                }
                _ => {}
            }
        }
        out.extend(
            NAME.find(inner[start..].trim())
                .map(|m| m.as_str().to_owned()),
        );
        rest = rest[close + 1..].trim_start();
    }
    out
}

pub(super) fn julia_literal_lines(text: &str) -> Vec<bool> {
    #[derive(PartialEq)]
    enum At {
        Code,
        Str { quote: u8, triple: bool, line: bool },
        Comment(usize),
    }
    let b = text.as_bytes();
    let mut out = vec![false];
    let mut at = At::Code;
    let mut i = 0;
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'!';
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            if let At::Str { line: true, .. } = at {
                at = At::Code;
            }
            out.push(at != At::Code);
            i += 1;
            continue;
        }
        match at {
            At::Code => match c {
                b'#' if b.get(i + 1) == Some(&b'=') => {
                    at = At::Comment(1);
                    i += 2;
                    continue;
                }
                b'#' => {
                    while i < b.len() && b[i] != b'\n' {
                        i += 1;
                    }
                    continue;
                }
                b'"' | b'`' => {
                    let triple = b.get(i + 1) == Some(&c) && b.get(i + 2) == Some(&c);
                    let prefixed = i > 0 && word(b[i - 1]);
                    at = At::Str {
                        quote: c,
                        triple,
                        line: prefixed && !triple,
                    };
                    i += if triple { 3 } else { 1 };
                    continue;
                }
                b'\'' => {
                    let adjoint = i > 0 && (word(b[i - 1]) || b")]}'.".contains(&b[i - 1]));
                    if !adjoint {
                        let mut j = i + 1;
                        while j < b.len() && b[j] != b'\n' {
                            match b[j] {
                                b'\\' => j += 2,
                                b'\'' => break,
                                _ => j += 1,
                            }
                        }
                        if b.get(j) == Some(&b'\'') {
                            i = j + 1;
                            continue;
                        }
                    }
                }
                _ => {}
            },
            At::Str { quote, triple, .. } => {
                if c == b'\\' {
                    i += 1;
                    if b.get(i) == Some(&b'\n') {
                        continue;
                    }
                } else if c == quote
                    && (!triple || (b.get(i + 1) == Some(&quote) && b.get(i + 2) == Some(&quote)))
                {
                    at = At::Code;
                    i += if triple { 3 } else { 1 };
                    continue;
                }
            }
            At::Comment(depth) => {
                if c == b'#' && b.get(i + 1) == Some(&b'=') {
                    at = At::Comment(depth + 1);
                    i += 2;
                    continue;
                }
                if c == b'=' && b.get(i + 1) == Some(&b'#') {
                    at = match depth {
                        1 => At::Code,
                        d => At::Comment(d - 1),
                    };
                    i += 2;
                    continue;
                }
            }
        }
        i += 1;
    }
    out.truncate(text.lines().count().max(1));
    out
}

pub fn julia_name(line: &str, r: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let rest = &line.as_bytes()[r.end..];
    match rest.first() == Some(&b'!') && rest.get(1) != Some(&b'=') {
        true => r.start..r.end + 1,
        false => r,
    }
}

pub fn julia_whole(line: &str, word: &str) -> bool {
    word.ends_with('!') || julia_col(line, word).is_some()
}

pub fn julia_col(line: &str, word: &str) -> Option<usize> {
    let b = line.as_bytes();
    let part = |i: usize| {
        b.get(i)
            .is_some_and(|&c| c.is_ascii_alphanumeric() || c == b'_')
    };
    line.match_indices(word).map(|(i, _)| i).find(|&i| {
        let end = i + word.len();
        !(i > 0 && part(i - 1))
            && !part(end)
            && (word.ends_with('!') || b.get(end) != Some(&b'!') || b.get(end + 1) == Some(&b'='))
    })
}

pub fn julia_slug(uuid: &str, tree: &str) -> Option<String> {
    let uuid = u128::from_str_radix(&uuid.replace('-', ""), 16).ok()?;
    let mut bytes = uuid.to_le_bytes().to_vec();
    if tree.len() != 40 {
        return None;
    }
    for i in (0..40).step_by(2) {
        bytes.push(u8::from_str_radix(tree.get(i..i + 2)?, 16).ok()?);
    }
    let mut crc = !0u32;
    for &byte in &bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0x82F6_3B78 & (crc & 1).wrapping_neg());
        }
    }
    let mut crc = !crc;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    Some(
        (0..5)
            .map(|_| {
                let c = CHARS[(crc % 62) as usize] as char;
                crc /= 62;
                c
            })
            .collect(),
    )
}

pub fn julia_roots(root: &Path, share: Option<PathBuf>, depot: &Path) -> Vec<PathBuf> {
    let dirs_in = |d: &Path| -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = std::fs::read_dir(d)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        found.sort();
        found
    };
    let mut dirs = Vec::new();
    if let Some(share) = share {
        dirs.push(share.join("base"));
        for version in dirs_in(&share.join("stdlib")) {
            dirs.extend(dirs_in(&version).into_iter().map(|p| p.join("src")));
        }
    }
    let read = |names: &[String]| {
        names
            .iter()
            .find_map(|n| std::fs::read_to_string(root.join(n)).ok())
            .unwrap_or_default()
    };
    let mut versioned: Vec<String> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("Manifest-v") && n.ends_with(".toml"))
        .collect();
    versioned.sort();
    let manifests: Vec<String> = ["JuliaManifest.toml".to_owned()]
        .into_iter()
        .chain(versioned.into_iter().rev())
        .chain(["Manifest.toml".to_owned()])
        .collect();
    let own = read(&["JuliaProject.toml".to_owned(), "Project.toml".to_owned()])
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == "name").then(|| v.trim().trim_matches('"').to_owned())
        });
    let mut pinned = std::collections::HashMap::new();
    let mut name: Option<String> = None;
    let (mut uuid, mut tree) = (None, None);
    let manifest = read(&manifests);
    for line in manifest.lines().chain(["[["]) {
        let line = line.trim();
        if line.starts_with("[[") {
            if let (Some(n), Some(u), Some(t)) = (name.take(), uuid.take(), tree.take())
                && let Some(slug) = julia_slug(u, t)
            {
                pinned.insert(n, slug);
            }
            (uuid, tree) = (None, None);
            name = line
                .trim_start_matches("[[")
                .trim_end_matches("]]")
                .trim_start_matches("deps.")
                .trim_matches('"')
                .to_owned()
                .into();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim().trim_matches('"');
        match k.trim() {
            "uuid" => uuid = Some(v),
            "git-tree-sha1" => tree = Some(v),
            _ => {}
        }
    }
    for package in dirs_in(&depot.join("packages")) {
        let Some(n) = package.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if own.as_deref() == Some(n) {
            continue;
        }
        match pinned.get(n) {
            Some(slug) => dirs.push(package.join(slug).join("src")),
            None => dirs.extend(dirs_in(&package).into_iter().map(|p| p.join("src"))),
        }
    }
    dirs
}

pub fn julia_share(bindir: &str) -> Option<PathBuf> {
    Some(Path::new(bindir.trim()).parent()?.join("share/julia"))
}

pub fn julia_depot(var: Option<std::ffi::OsString>, home: &Path) -> PathBuf {
    var.and_then(|v| std::env::split_paths(&v).next())
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| home.join(".julia"))
}
