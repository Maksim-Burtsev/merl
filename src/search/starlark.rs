use std::ops::Range;
use std::path::{Path, PathBuf};

use regex::Regex;

use super::*;

pub fn starlark_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    vec![
        format!(r"^\s*def\s+{w}\s*\("),
        format!(r"^(?:[A-Za-z_]\w*\s*,\s*)*{w}\s*(?:,\s*[A-Za-z_]\w*\s*)*,?\s*=(?:[^=]|$)"),
    ]
}

pub(super) const STARLARK_RULE_SYMBOL: &str =
    r"^(?P<name>[A-Za-z_]\w*)\s*=\s*(?:rule|provider|aspect|repository_rule|module_extension)\s*\(";
pub(super) const STARLARK_TARGET_SYMBOL: &str =
    r#"^(?:\s+|[A-Za-z_][\w.]*\s*\(.*\b)name\s*=\s*["'](?P<name>[^"']+)["']"#;

#[derive(Debug, PartialEq, Eq)]
pub enum StarlarkLabel {
    Repo(String),
    Target {
        repo: Option<String>,
        package: Option<String>,
        name: String,
    },
}

pub fn starlark_label(s: &str) -> Option<StarlarkLabel> {
    if s.is_empty()
        || s.contains(|c: char| c.is_whitespace() || "$*%{}<>()[]\\\"',;|&?!#".contains(c))
    {
        return None;
    }
    let (repo, rest) = match s.strip_prefix('@') {
        Some(r) => {
            let r = r.strip_prefix('@').unwrap_or(r);
            match r.split_once("//") {
                Some((repo, rest)) => (Some(repo.to_owned()), Some(rest)),
                None if !r.is_empty() && !r.contains([':', '/']) => {
                    return Some(StarlarkLabel::Repo(r.to_owned()));
                }
                None => return None,
            }
        }
        None => (None, s.strip_prefix("//")),
    };
    let (package, name) = match rest {
        Some(abs) => match abs.split_once(':') {
            Some((p, n)) => (Some(p.to_owned()), n.to_owned()),
            None => (
                Some(abs.to_owned()),
                abs.rsplit('/').next().unwrap_or(abs).to_owned(),
            ),
        },
        None => (None, s.strip_prefix(':').unwrap_or(s).to_owned()),
    };
    let name = match (name.is_empty(), &repo) {
        (true, Some(r)) => r.clone(),
        _ => name,
    };
    let bad = |p: &str| p.starts_with('/') || p.split('/').any(|c| c == "..") || p.contains(':');
    (!name.is_empty() && !bad(&name) && !package.as_deref().is_some_and(bad)).then_some(
        StarlarkLabel::Target {
            repo,
            package,
            name,
        },
    )
}

pub fn starlark_string_at(line: &str, col: usize) -> Option<(Range<usize>, bool)> {
    let b = line.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let q = b[i];
        if q == b'#' {
            return None;
        }
        if q != b'"' && q != b'\'' {
            i += 1;
            continue;
        }
        let triple = b[i..].starts_with(&[q, q, q]);
        let open = if triple { 3 } else { 1 };
        let start = i + open;
        let mut j = start;
        while j < b.len() {
            match b[j] {
                b'\\' => j += 2,
                c if c == q && (!triple || b[j..].starts_with(&[q, q, q])) => break,
                _ => j += 1,
            }
        }
        if j >= b.len() {
            return (col >= i).then_some((start.min(b.len())..b.len(), false));
        }
        let close = j + open;
        if (i..close).contains(&col) {
            return Some((start..j, !triple));
        }
        i = close;
    }
    None
}

fn code_lines(text: &str, re: &Regex) -> impl Iterator<Item = usize> {
    let literal = literal_lines(Kind::Starlark, text);
    text.lines()
        .enumerate()
        .filter(move |(i, l)| !literal.get(*i).copied().unwrap_or(false) && re.is_match(l))
        .map(|(i, _)| i + 1)
        .collect::<Vec<_>>()
        .into_iter()
}

pub fn starlark_declarations(text: &str, name: &str) -> Vec<usize> {
    let re = Regex::new(&starlark_patterns(name).join("|")).expect("an escaped name");
    code_lines(text, &re).collect()
}

pub fn starlark_target_line(text: &str, name: &str) -> Option<usize> {
    let n = regex::escape(name);
    let re = Regex::new(&format!(
        r#"^(?:\s+|[A-Za-z_][\w.]*\s*\(.*\b)name\s*=\s*["']{n}["']"#
    ))
    .expect("an escaped name");
    code_lines(text, &re).next()
}

pub fn starlark_repo_line(text: &str, name: &str) -> Option<usize> {
    let n = regex::escape(name);
    let re = Regex::new(&format!(r#"^[^#]*\b(?:repo_)?name\s*=\s*["']{n}["']"#))
        .expect("an escaped name");
    code_lines(text, &re).next()
}

pub fn starlark_project_names(text: &str) -> Vec<String> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?m)^(?:module|workspace)\s*\(([^)]*)\)").unwrap()
    });
    static NAME: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"\b(?:repo_)?name\s*=\s*["']([^"']+)["']"#).unwrap()
    });
    CALL.captures_iter(text)
        .flat_map(|c| {
            NAME.captures_iter(c.get(1).map_or("", |m| m.as_str()))
                .map(|n| n[1].to_owned())
                .collect::<Vec<_>>()
        })
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
pub struct StarlarkLoad {
    pub label: String,
    pub names: Vec<(String, String)>,
    pub name_strings: Vec<(usize, usize)>,
}

enum Tok {
    Str(Range<usize>),
    Name(Range<usize>),
    Eq,
    Comma,
}

fn load_tokens(text: &str, mut i: usize) -> Vec<Tok> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    while i < b.len() {
        match b[i] {
            b'#' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            q @ (b'"' | b'\'') => {
                let mut j = i + 1;
                while j < b.len() && b[j] != q && b[j] != b'\n' {
                    j += if b[j] == b'\\' { 2 } else { 1 };
                }
                if b.get(j) != Some(&q) {
                    break;
                }
                out.push(Tok::Str(i + 1..j));
                i = j;
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let s = i;
                while b
                    .get(i + 1)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
                {
                    i += 1;
                }
                out.push(Tok::Name(s..i + 1));
            }
            b'=' => out.push(Tok::Eq),
            b',' => out.push(Tok::Comma),
            c if c.is_ascii_whitespace() => {}
            _ => break,
        }
        i += 1;
    }
    out
}

pub fn starlark_loads(text: &str) -> Vec<StarlarkLoad> {
    static LOAD: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^load\s*\(").unwrap());
    let literal = literal_lines(Kind::Starlark, text);
    let mut starts = Vec::new();
    let mut at = 0;
    for l in text.split('\n') {
        starts.push(at);
        at += l.len() + 1;
    }
    let place = |o: usize| {
        let line = starts.partition_point(|&s| s <= o) - 1;
        (line, o - starts[line])
    };
    let mut out = Vec::new();
    for (i, &s) in starts.iter().enumerate() {
        let line = &text[s..text[s..].find('\n').map_or(text.len(), |e| s + e)];
        let Some(m) = LOAD
            .find(line)
            .filter(|_| !literal.get(i).copied().unwrap_or(false))
        else {
            continue;
        };
        let toks = load_tokens(text, s + m.end());
        let mut toks = toks.iter();
        let Some(Tok::Str(label)) = toks.next() else {
            continue;
        };
        let mut load = StarlarkLoad {
            label: text[label.clone()].to_owned(),
            names: Vec::new(),
            name_strings: Vec::new(),
        };
        let mut local = None;
        for t in toks {
            match t {
                Tok::Name(r) => local = Some(text[r.clone()].to_owned()),
                Tok::Str(r) => {
                    let name = text[r.clone()].to_owned();
                    load.names
                        .push((local.take().unwrap_or_else(|| name.clone()), name));
                    load.name_strings.push(place(r.start));
                }
                Tok::Eq | Tok::Comma => {}
            }
        }
        out.push(load);
    }
    out
}

pub fn starlark_external(root: &Path) -> Option<PathBuf> {
    let name = root.file_name()?.to_str()?;
    let main = std::fs::canonicalize(root.join(format!("bazel-{name}"))).ok()?;
    let external = main.parent()?.parent()?.join("external");
    external.is_dir().then_some(external)
}

pub fn starlark_repo_dir(external: &Path, repo: &str) -> Option<PathBuf> {
    let exact = external.join(repo);
    if exact.is_dir() {
        return Some(exact);
    }
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(external)
        .ok()?
        .flatten()
        .filter(|e| {
            e.file_name().to_str().is_some_and(|n| {
                n.strip_prefix(repo)
                    .is_some_and(|r| r.starts_with(['+', '~']))
            })
        })
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs.into_iter().next()
}

pub fn starlark_keyword_argument(text: &str, at: usize, end: usize) -> bool {
    let assigns = text[end..].trim_start().strip_prefix('=');
    if assigns.is_none_or(|rest| rest.starts_with('=')) {
        return false;
    }
    let mut open = Vec::new();
    for (i, c) in code(Kind::Starlark, &text[..at]) {
        match c {
            b'(' | b'[' | b'{' => open.push((i, c)),
            b')' | b']' | b'}' => {
                open.pop();
            }
            _ => {}
        }
    }
    static DEF: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"(?m)(?:\bdef\s+\w+|^load)\s*\z").unwrap());
    matches!(open.last(), Some(&(i, b'('))  if !DEF.is_match(&text[..i]))
}
