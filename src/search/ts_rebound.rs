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
