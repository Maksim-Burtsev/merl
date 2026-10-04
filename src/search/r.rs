use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::*;

macro_rules! r_name {
    () => {
        r"[A-Za-z.][A-Za-z0-9_.]*"
    };
}

macro_rules! r_assigned {
    () => {
        concat!(r"(?:", r_name!(), r"\s*(?:<<?-|=)\s*)?")
    };
}

fn name_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'.'
}

pub fn r_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    let name = format!("(?:{w}|`{w}`)");
    vec![
        format!(r"^\s*{name}\s*<<?-"),
        format!(r"^{name}\s*=(?:[^=]|$)"),
        format!(r"(?:^|[(,])\s*{name}\s*=\s*(?:function\b|\\\()"),
        format!(
            r#"^\s*{}(?:methods::)?set(?:Class|Generic|RefClass|Method)\s*\(\s*(?:[A-Za-z]+\s*=\s*)?["']{w}["']"#,
            r_assigned!()
        ),
    ]
}

pub(super) const R_FUNCTION_SYMBOL: &str = concat!(
    r"^\s*(?P<name>",
    r_name!(),
    r"|`[^`]+`)\s*(?:<<?-|=)\s*",
    r"(?:function\b|\\\()"
);
pub(super) const R_CLASS_SYMBOL: &str = concat!(
    r"^\s*",
    r_assigned!(),
    r"(?:methods::)?set(?:Class|Generic|RefClass)\s*\(\s*(?:[A-Za-z]+\s*=\s*)?",
    r#"["'](?P<name>[^"']+)["']"#
);
pub(super) const R6_CLASS_SYMBOL: &str = concat!(
    r"^\s*(?P<name>",
    r_name!(),
    r")\s*(?:<<?-|=)\s*(?:R6::)?R6Class\s*\("
);

fn comment_at(line: &str) -> usize {
    let b = line.as_bytes();
    let (mut quote, mut i) = (None, 0);
    while i < b.len() {
        match (quote, b[i]) {
            (Some(_), b'\\') => i += 1,
            (Some(q), c) if c == q => quote = None,
            (None, b'#') => return i,
            (None, c @ (b'"' | b'\'' | b'`')) => quote = Some(c),
            _ => {}
        }
        i += 1;
    }
    b.len()
}

pub fn r_quoted_name(line: &str, col: usize) -> Option<Range<usize>> {
    let code = &line[..comment_at(line)];
    if col >= code.len() {
        return None;
    }
    let ticks: Vec<usize> = code.match_indices('`').map(|(i, _)| i).collect();
    if let Some(&[a, b]) = ticks
        .chunks(2)
        .find(|p| p.len() == 2 && p[0] <= col && col <= p[1])
    {
        return (b > a + 1).then_some(a + 1..b);
    }
    static OPERATOR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"%[^%\s`]+%").unwrap());
    OPERATOR
        .find_iter(code)
        .find(|m| m.start() <= col && col < m.end())
        .map(|m| m.range())
}

pub fn r_foreign_package(line: &str, start: usize, root: &Path) -> bool {
    let before = &line[..start];
    let Some(q) = before
        .strip_suffix(":::")
        .or_else(|| before.strip_suffix("::"))
    else {
        return false;
    };
    let from = q
        .rfind(|c: char| !(c.is_ascii() && name_byte(c as u8)))
        .map_or(0, |i| i + 1);
    let package = std::fs::read_to_string(root.join("DESCRIPTION")).ok();
    let own = package.as_deref().and_then(|d| {
        d.lines()
            .find_map(|l| l.strip_prefix("Package:"))
            .map(str::trim)
    });
    from < q.len() && own != Some(&q[from..])
}

pub fn r_source(line: &str, col: usize) -> Option<String> {
    static SOURCE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"\b(?:sys\.)?source\s*\(\s*(?:file\s*=\s*)?["']([^"']+)["']"#).unwrap()
    });
    SOURCE
        .captures_iter(line)
        .filter_map(|c| c.get(1))
        .find(|m| m.start() <= col + 1 && col <= m.end())
        .map(|m| m.as_str().to_owned())
}

pub fn r_files(dir: &Path, path: &str, files: &[PathBuf]) -> Vec<PathBuf> {
    [Path::new(""), dir]
        .iter()
        .filter_map(|d| lexical(&d.join(path)))
        .find(|f| files.contains(f))
        .into_iter()
        .collect()
}

pub(super) fn r_literal_lines(text: &str) -> Vec<bool> {
    let b = text.as_bytes();
    let mut out = vec![false];
    let mut end: Option<Vec<u8>> = None;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            out.push(end.is_some());
            i += 1;
            continue;
        }
        if let Some(close) = &end {
            if close.len() == 1 && c == b'\\' && b.get(i + 1) != Some(&b'\n') {
                i += 2;
            } else if b[i..].starts_with(close) {
                i += close.len();
                end = None;
            } else {
                i += 1;
            }
            continue;
        }
        let line_end = b[i..]
            .iter()
            .position(|&c| c == b'\n')
            .map_or(b.len(), |n| i + n);
        match c {
            b'#' => i = line_end,
            b'`' => {
                i = b[i + 1..line_end]
                    .iter()
                    .position(|&c| c == b'`')
                    .map_or(line_end, |n| i + n + 2)
            }
            b'"' | b'\'' => {
                end = Some(vec![c]);
                i += 1;
            }
            b'r' | b'R'
                if (i == 0 || !name_byte(b[i - 1]))
                    && matches!(b.get(i + 1), Some(b'"' | b'\'')) =>
            {
                let quote = b[i + 1];
                let dashes = b[i + 2..].iter().take_while(|&&c| c == b'-').count();
                let open = i + 2 + dashes;
                let bracket = match b.get(open) {
                    Some(b'(') => Some(b')'),
                    Some(b'[') => Some(b']'),
                    Some(b'{') => Some(b'}'),
                    _ => None,
                };
                match bracket {
                    Some(close) => {
                        let mut closer = vec![close];
                        closer.resize(dashes + 1, b'-');
                        closer.push(quote);
                        end = Some(closer);
                        i = open + 1;
                    }
                    None => i += 1,
                }
            }
            _ => i += 1,
        }
    }
    out
}
