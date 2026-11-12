use regex::Regex;

use super::bindings::{Binding, Value, this_opener};

pub(super) fn ts_this_on_line(code: &str, col: usize, line1: usize) -> Option<Binding> {
    static FUNCTION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bfunction\b\s*\*?\s*[\w$]*\s*\([^()]*\)\s*(?::[^{}()]*)?\{$")
            .expect("the regex is a literal")
    });
    static METHOD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"(?:^|,)\s*(?:(?:async|get|set)\s+)?\*?\s*[\w$]+\s*\([^()]*\)\s*(?::[^{}()]*)?\{$",
        )
        .expect("the regex is a literal")
    });
    let prefix = code.get(..col)?;
    let mut open = Vec::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (i, c) in prefix.char_indices() {
        match quote {
            Some(_) if escaped => escaped = false,
            Some(_) if c == '\\' => escaped = true,
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                '"' | '\'' | '`' => quote = Some(c),
                '{' => open.push(i),
                '}' => {
                    open.pop();
                }
                _ => {}
            },
        }
    }
    let mut inner: Option<usize> = None;
    for &i in open.iter().rev() {
        let header = prefix.get(..=i)?.trim();
        let value = match this_opener(header, line1) {
            Some(Value::Unknown) if !FUNCTION.is_match(header) => inner
                .and_then(|j| prefix.get(i + 1..=j))
                .is_some_and(|s| METHOD.is_match(s))
                .then_some(Value::Unknown),
            value => value,
        };
        if let Some(value) = value {
            return Some(Binding { line1, value });
        }
        inner = Some(i);
    }
    None
}

pub fn ts_literal_this_key(text: &str, line1: usize, col: usize, name: &str) -> Option<usize> {
    static METHOD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            r"^(?:(?:async|get|set)\s+)?\*?\s*([\w$]+)\s*\([^()]*\)\s*(?::[^{}()]*)?\{$",
            r"|^[\w$]+\s*:\s*(?:async\s+)?function\b\s*\*?\s*[\w$]*\s*\([^()]*\)\s*(?::[^{}()]*)?\{$"
        ))
        .expect("the regex is a literal")
    });
    static THIS_PARAM: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\(\s*this\s*[:,)]").expect("the regex is a literal")
    });
    static LITERAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:(?:export\s+)?(?:const|let|var)\s+[\w$]+(?:\s*:[^=]*)?|[\w$.]+)\s*=\s*\{$")
            .expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let code = |i: usize| super::uncommented(super::Kind::TsJs, lines[i]);
    let scope = |at: usize| {
        let depth = super::indent(lines[at]);
        (0..at)
            .rev()
            .find(|&i| !code(i).trim().is_empty() && super::indent(lines[i]) < depth)
    };
    let own = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    let prefix = lines[own].get(..col)?;
    if ts_this_on_line(lines[own], col, line1).is_some()
        || prefix
            .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
            .any(|w| w == "class")
    {
        return None;
    }
    let mut at = scope(own)?;
    loop {
        let header = code(at);
        let header = header.trim();
        let method = METHOD.captures(header).is_some_and(|c| {
            c.get(1).is_none_or(|m| {
                !matches!(
                    m.as_str(),
                    "if" | "for" | "while" | "switch" | "catch" | "function"
                )
            })
        });
        if method {
            if THIS_PARAM.is_match(header) {
                return None;
            }
            break;
        }
        if this_opener(header, 0).is_some() {
            return None;
        }
        at = scope(at)?;
    }
    let literal = scope(at)?;
    LITERAL.is_match(code(literal).trim()).then_some(())?;
    let key = Regex::new(&format!(
        r"^(?:(?:async|get|set)\s+)?\*?\s*(?:{n}|'{n}'|\x22{n}\x22)\s*(?:[:(,]|$)",
        n = regex::escape(name)
    ))
    .ok()?;
    let body = super::indent(lines[at]);
    (literal + 1..lines.len())
        .take_while(|&i| code(i).trim().is_empty() || super::indent(lines[i]) >= body)
        .find(|&i| super::indent(lines[i]) == body && key.is_match(code(i).trim()))
        .map(|i| i + 1)
}
