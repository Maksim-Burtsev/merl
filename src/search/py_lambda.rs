use regex::Regex;

use super::{Kind, Value, code, split_top, uncommented};

pub(super) fn python_lambda_value(line: &str, col: usize, name: &str) -> Option<Value> {
    static LAMBDA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\blambda\b([^:]*):").expect("the regex is a literal")
    });
    static CALLABLE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"^\s*[A-Za-z_]\w*\s*:\s*(?:typing\.)?Callable\s*\[\s*\[(.*)\]\s*,[^\[\]]*\]\s*=$",
        )
        .expect("the regex is a literal")
    });
    static OVER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.])(?:(?:sorted|max|min)\s*\(\s*([A-Za-z_][\w.]*)\s*,\s*|([A-Za-z_][\w.]*)\s*\.\s*sort\s*\(\s*)key\s*=$")
            .expect("the regex is a literal")
    });
    static MAPPED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.])(?:map|filter)\s*\($").expect("the regex is a literal")
    });
    static ITERABLE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*,\s*([A-Za-z_][\w.]*)\s*\)").expect("the regex is a literal")
    });
    let line = uncommented(Kind::Python, line);
    let c = LAMBDA
        .captures_iter(&line)
        .filter(|c| c.get(0).is_some_and(|m| m.end() <= col))
        .last()?;
    let (whole, params) = (c.get(0)?, c.get(1)?);
    let params: Vec<&str> = split_top(Kind::Python, params.as_str(), b',')
        .into_iter()
        .map(str::trim)
        .collect();
    let at = params.iter().position(|p| *p == name)?;
    let mut depth = 0i32;
    let end = code(Kind::Python, line.get(whole.end()..)?)
        .find(|&(_, b)| match b {
            b'(' | b'[' | b'{' => {
                depth += 1;
                false
            }
            b')' | b']' | b'}' => {
                depth -= 1;
                depth < 0
            }
            b',' | 0 => depth == 0,
            _ => false,
        })
        .map_or(line.len(), |(i, _)| whole.end() + i);
    if col >= end {
        return None;
    }
    let before = line.get(..whole.start())?.trim_end();
    if let Some(c) = CALLABLE.captures(before) {
        let args = split_top(Kind::Python, &c[1], b',');
        return (args.len() == params.len())
            .then(|| args.get(at).map(|a| Value::Type(a.trim().to_owned())))
            .flatten();
    }
    if params.len() != 1 {
        return None;
    }
    if let Some(c) = OVER.captures(before) {
        return Some(Value::Element(
            c.get(1).or_else(|| c.get(2))?.as_str().to_owned(),
        ));
    }
    let over = ITERABLE.captures(line.get(end..)?)?;
    MAPPED
        .is_match(before)
        .then(|| Value::Element(over[1].to_owned()))
}
