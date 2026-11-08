use regex::Regex;

use super::{
    Kind, comment, indent, jvm_type_parameter, path_literal_lines, scala, scala_type_parameter,
    split_top, uncommented,
};
use std::path::Path;

pub fn generic_param_line(
    kind: Kind,
    path: &Path,
    text: &str,
    line1: usize,
    name: &str,
) -> Option<usize> {
    let declares = |t: &str| match kind {
        Kind::Dart => dart_declares(t, name),
        Kind::Jvm if scala(path) => scala_type_parameter(t, name),
        Kind::Jvm => jvm_type_parameter(t, name),
        _ => false,
    };
    let lines: Vec<&str> = text.lines().collect();
    let literal = path_literal_lines(kind, path, text);
    let mut depth = usize::MAX;
    for i in (0..line1.min(lines.len())).rev() {
        let code = uncommented(kind, lines[i]);
        let t = code.trim();
        let ind = indent(lines[i]);
        if t.is_empty()
            || comment(kind, t)
            || literal.get(i) == Some(&true)
            || (ind >= depth && i + 1 != line1)
        {
            continue;
        }
        depth = ind + usize::from(t.starts_with([')', '>']));
        if declares(t) {
            return Some(i + 1);
        }
        if depth == 0 {
            break;
        }
    }
    None
}

fn dart_declares(t: &str, name: &str) -> bool {
    static HEADER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:(?:abstract|sealed|base|final|interface|mixin)\s+)*(?:class|mixin|extension|typedef)\s+[\w$]+\s*<|^(?:(?:static|external)\s+)*(?P<ret>[\w$.]+)(?:<[^()=]*>)?\??\s+[\w$]+\s*<")
            .expect("the regex is a literal")
    });
    const STATEMENTS: [&str; 13] = [
        "return", "await", "yield", "throw", "new", "const", "final", "var", "late", "if", "else",
        "case", "assert",
    ];
    let Some(c) = HEADER.captures(t) else {
        return false;
    };
    let ret = c.name("ret");
    if ret.is_some_and(|r| STATEMENTS.contains(&r.as_str())) {
        return false;
    }
    let Some(whole) = c.get(0) else {
        return false;
    };
    let rest = t.get(whole.end()..).unwrap_or_default();
    let mut depth = 1;
    let Some((end, _)) = rest.char_indices().find(|&(_, c)| {
        depth += match c {
            '<' => 1,
            '>' => -1,
            _ => 0,
        };
        depth == 0
    }) else {
        return false;
    };
    let after = rest.get(end + 1..).unwrap_or_default().trim_start();
    if ret.is_some() && !after.starts_with('(') {
        return false;
    }
    split_top(Kind::Dart, rest.get(..end).unwrap_or_default(), b',')
        .iter()
        .any(|p| p.split_whitespace().next() == Some(name))
}
