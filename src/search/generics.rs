use std::sync::LazyLock;

use regex::Regex;

use super::{Kind, indent, literal_lines, uncommented};

pub fn type_param_line(kind: Kind, text: &str, line1: usize, name: &str) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(kind, text);
    let mut k = line1.checked_sub(1).filter(|&k| k < lines.len())?;
    loop {
        if let Some(at) = declares(kind, &lines, k, name) {
            return Some(at + 1);
        }
        let t = uncommented(kind, lines[k]);
        let t = t.trim();
        let depth = indent(lines[k]) + usize::from(continues(t));
        k = (0..k).rev().find(|&i| {
            let t = uncommented(kind, lines[i]);
            let t = t.trim();
            !t.is_empty()
                && literal.get(i) != Some(&true)
                && !continues(t)
                && !t.starts_with(['{', '}', '#'])
                && indent(lines[i]) < depth
        })?;
    }
}

fn continues(t: &str) -> bool {
    t.starts_with([')', ']', '>']) || t.starts_with("where") || t.starts_with("->")
}

fn declares(kind: Kind, lines: &[&str], k: usize, name: &str) -> Option<usize> {
    static RUST: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"\b(?:fn\s+\w+|impl|struct\s+\w+|enum\s+\w+|union\s+\w+|trait\s+\w+|type\s+\w+)\s*<",
        )
        .expect("the regex is a literal")
    });
    static GO: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:func\s+(?:\([^)]*\)\s*)?\w+|type\s+\w+)\s*\[")
            .expect("the regex is a literal")
    });
    static GO_RECEIVER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^func\s*\(\s*\w*\s*\*?\s*\w+\s*\[").expect("the regex is a literal")
    });
    static CPP: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\btemplate\s*<").expect("the regex is a literal"));
    let line = uncommented(kind, lines[k]);
    let t = line.trim();
    let named = |list: &str, last: bool| {
        top_parts(list)
            .iter()
            .any(|p| param_name(p, last) == Some(name))
    };
    match kind {
        Kind::Rust => RUST
            .find_iter(t)
            .filter_map(|m| list_after(t.get(m.end()..)?, b'<', b'>'))
            .any(|l| named(l, false))
            .then_some(k),
        Kind::Go => {
            let list = |re: &Regex| list_after(t.get(re.find(t)?.end()..)?, b'[', b']');
            let typed = list(&GO).is_some_and(|l| {
                let parts = top_parts(l);
                parts
                    .last()
                    .is_some_and(|p| p.split_whitespace().count() > 1)
                    && named(l, false)
            });
            (typed || list(&GO_RECEIVER).is_some_and(|l| named(l, false))).then_some(k)
        }
        Kind::C => (0..=k)
            .rev()
            .take_while(|&i| i == k || bare_template(&uncommented(kind, lines[i])))
            .find(|&i| {
                let l = uncommented(kind, lines[i]);
                CPP.find_iter(&l)
                    .filter_map(|m| list_after(l.get(m.end()..)?, b'<', b'>'))
                    .any(|l| named(l, true))
            }),
        _ => None,
    }
}

fn bare_template(line: &str) -> bool {
    line.trim()
        .strip_prefix("template")
        .and_then(|r| r.trim_start().strip_prefix('<'))
        .and_then(|r| r.get(list_after(r, b'<', b'>')?.len() + 1..))
        .is_some_and(|rest| rest.trim().is_empty())
}

fn list_after(s: &str, open: u8, close: u8) -> Option<&str> {
    let b = s.as_bytes();
    let mut depth = 1usize;
    let angled = open == b'<';
    for (i, &c) in b.iter().enumerate() {
        let arrow = i > 0 && matches!(b[i - 1], b'-' | b'=');
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b'<' if angled => depth += 1,
            b')' | b']' | b'}' => depth = depth.checked_sub(1)?,
            b'>' if angled && !arrow => depth = depth.checked_sub(1)?,
            _ => continue,
        }
        if depth == 0 {
            return (c == close).then(|| s.get(..i)).flatten();
        }
    }
    None
}

fn top_parts(list: &str) -> Vec<&str> {
    let b = list.as_bytes();
    let (mut depth, mut start, mut out) = (0i32, 0, Vec::new());
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b'>' if i > 0 && matches!(b[i - 1], b'-' | b'=') => {}
            b')' | b']' | b'}' | b'>' => depth -= 1,
            b',' if depth == 0 => {
                out.extend(list.get(start..i));
                start = i + 1;
            }
            _ => {}
        }
    }
    out.extend(list.get(start..));
    out
}

fn param_name(part: &str, last: bool) -> Option<&str> {
    static WORD: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[A-Za-z_]\w*").expect("the regex is a literal"));
    let head = part.split('=').next()?.trim();
    let head = head.strip_prefix("const ").unwrap_or(head);
    if head.starts_with('\'') {
        return None;
    }
    if last {
        let head = head.trim_end_matches("...").trim_end();
        return WORD
            .find_iter(head)
            .last()
            .map(|m| m.as_str())
            .filter(|_| head.split_whitespace().count() > 1);
    }
    let head = head.split(':').next()?.trim();
    WORD.find(head)
        .filter(|m| m.start() == 0)
        .map(|m| m.as_str())
}
