use regex::Regex;

use super::{Kind, split_top, uncommented};

pub struct ScalaTupleCall {
    pub receiver: Option<String>,
    pub method: String,
    pub at: usize,
    pub count: usize,
}

pub fn scala_tuple_call(line: &str, name: &str) -> Option<ScalaTupleCall> {
    static PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"^\s*(?:val|var)\s+\(([\w\s,]+)\)\s*=\s*(?:([a-z]\w*)\.)?([a-z]\w*)\s*\((?:[^()]|\([^()]*\))*\)\s*;?\s*$",
        )
        .expect("the regex is a literal")
    });
    let code = uncommented(Kind::Jvm, line);
    let c = PATTERN.captures(code.trim_end())?;
    let names: Vec<&str> = c[1].split(',').map(str::trim).collect();
    let at = names.iter().position(|n| *n == name)?;
    Some(ScalaTupleCall {
        receiver: c.get(2).map(|m| m.as_str().to_owned()),
        method: c[3].to_owned(),
        at,
        count: names.len(),
    })
}

pub fn scala_tuple_item(line: &str, method: &str, at: usize, count: usize) -> Option<String> {
    let head = Regex::new(&format!(
        r"\bdef\s+{}\s*(?:\([^)]*\))*\s*:\s*\(([^()]*)\)\s*(?:=|\{{|$)",
        regex::escape(method)
    ))
    .ok()?;
    let code = uncommented(Kind::Jvm, line);
    let c = head.captures(code.trim_end())?;
    let items = split_top(Kind::Jvm, c.get(1)?.as_str(), b',');
    (items.len() == count).then_some(())?;
    let item = items.get(at)?.trim();
    (item.starts_with(|c: char| c.is_ascii_uppercase())
        && item.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
    .then(|| item.to_owned())
}

pub fn scala_extracted(line: &str, name: &str) -> Option<(String, usize, usize)> {
    static PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:val|var)\s+([A-Z]\w*)\s*\(([\w\s,]+)\)\s*=|case\s+([A-Z]\w*)\s*\(([\w\s,]+)\)\s*(?:=>|if\b))")
            .expect("the regex is a literal")
    });
    let code = uncommented(Kind::Jvm, line);
    let c = PATTERN.captures(&code)?;
    let (class, names) = c.get(1).zip(c.get(2)).or_else(|| c.get(3).zip(c.get(4)))?;
    let names: Vec<&str> = names.as_str().split(',').map(str::trim).collect();
    let at = names.iter().position(|n| *n == name)?;
    Some((class.as_str().to_owned(), at, names.len()))
}

pub fn scala_case_field(line: &str, class: &str, at: usize, count: usize) -> Option<String> {
    static FIELD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:va[lr]\s+)?\w+\s*:\s*([A-Z]\w*)$").expect("the regex is a literal")
    });
    let head = Regex::new(&format!(
        r"^\s*(?:final\s+)?case\s+class\s+{}\s*\(([^()]*)\)",
        regex::escape(class)
    ))
    .ok()?;
    let code = uncommented(Kind::Jvm, line);
    let params = head.captures(&code)?;
    let fields = split_top(Kind::Jvm, params.get(1)?.as_str(), b',');
    (fields.len() == count).then_some(())?;
    Some(FIELD.captures(fields.get(at)?.trim())?[1].to_owned())
}
