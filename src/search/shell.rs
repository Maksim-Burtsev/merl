use regex::Regex;

use super::*;

pub(super) fn shell_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    shell_function_at(lines, at).map_or_else(Vec::new, |f| {
        (f + 1..=at)
            .filter(|&i| shell_local_of(lines[i]).is_some_and(|names| names.contains(&name)))
            .map(|i| Binding {
                line1: i + 1,
                value: Value::Unknown,
            })
            .collect()
    })
}
pub fn shell_function_at(lines: &[&str], at: usize) -> Option<usize> {
    static HEADER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:function\s+[^\s(){}]+|[\w.:-]+\s*\(\s*\))").unwrap()
    });
    static ONE_LINE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\{.*[;\s]\}\s*(?:#.*)?$|\(\s*$").unwrap());
    (0..at.min(lines.len())).rev().find(|&k| {
        let l = lines[k];
        HEADER.is_match(l)
            && !ONE_LINE.is_match(l)
            && !lines[k + 1..=at]
                .iter()
                .any(|b| b.trim_start().starts_with('}') && indent(b) == indent(l))
    })
}
pub fn shell_local_of(line: &str) -> Option<Vec<&str>> {
    static LOCAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:local|declare|typeset)((?:\s+-\w+)*)\s+(.*)").unwrap()
    });
    let c = LOCAL.captures(line)?;
    if c[1].contains(['g', 'p', 'f', 'F']) {
        return None;
    }
    let rest = c.get(2).map_or("", |m| m.as_str());
    let names = shell_words(rest)
        .into_iter()
        .map(|w| w.split(['=', '+']).next().unwrap_or(""))
        .take_while(|n| {
            n.chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
        .collect();
    Some(names)
}
fn shell_words(s: &str) -> Vec<&str> {
    let (b, mut out, mut i) = (s.as_bytes(), Vec::new(), 0);
    while i < b.len() {
        if b[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let (start, mut depth, mut quote) = (i, 0usize, None);
        while i < b.len() && (quote.is_some() || depth > 0 || !b[i].is_ascii_whitespace()) {
            match (quote, b[i]) {
                (Some(b'"'), b'\\') => i += 1,
                (Some(q), c) if c == q => quote = None,
                (Some(_), _) => {}
                (None, c @ (b'"' | b'\'')) => quote = Some(c),
                (None, b'(') => depth += 1,
                (None, b')') => depth = depth.saturating_sub(1),
                _ => {}
            }
            i += 1;
        }
        out.push(&s[start..i.min(b.len())]);
    }
    out
}
