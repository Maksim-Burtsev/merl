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
            r"^\s*(?:val|var)\s+\(([\w\s,]+)\)\s*=\s*(?:([A-Za-z]\w*)\.)?([a-z]\w*)\s*\((?:[^()]|\([^()]*\))*\)\s*;?\s*$",
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

fn built_item(items: &str, at: usize, count: usize) -> Option<String> {
    static NEW: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^new\s+([A-Z]\w*)\s*(?:\(.*\))?$").expect("the regex is a literal")
    });
    let items = split_top(Kind::Jvm, items, b',');
    (items.len() == count).then_some(())?;
    Some(NEW.captures(items.get(at)?.trim())?[1].to_owned())
}

pub fn scala_tuple_literal(line: &str, name: &str) -> Option<String> {
    static PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:val|var)\s+\(([\w\s,]+)\)\s*=\s*\((.*)\)\s*;?\s*$")
            .expect("the regex is a literal")
    });
    let code = uncommented(Kind::Jvm, line);
    let c = PATTERN.captures(code.trim_end())?;
    let names: Vec<&str> = c[1].split(',').map(str::trim).collect();
    let at = names.iter().position(|n| *n == name)?;
    built_item(&c[2], at, names.len())
}

pub fn scala_tuple_body(line: &str, method: &str, at: usize, count: usize) -> Option<String> {
    let def = Regex::new(&format!(
        r"^\s*def\s+{}\s*(?:\(\s*\))?\s*=\s*\((.*)\)\s*;?\s*$",
        regex::escape(method)
    ))
    .ok()?;
    let code = uncommented(Kind::Jvm, line);
    built_item(&def.captures(code.trim_end())?[1], at, count)
}

pub fn scala_tuple_arm(line: &str, name: &str) -> Option<(usize, usize)> {
    static PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bcase\s*\(([\w\s,]+)\)\s*=>").expect("the regex is a literal")
    });
    let code = uncommented(Kind::Jvm, line);
    let c = PATTERN.captures(&code)?;
    alone_in_arm(&code, c.get(0)?.end(), name).then_some(())?;
    let names: Vec<&str> = c[1].split(',').map(str::trim).collect();
    Some((names.iter().position(|n| *n == name)?, names.len()))
}

fn alone_in_arm(code: &str, from: usize, name: &str) -> bool {
    static CASE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\bcase\b").expect("the regex is a literal"));
    let rest = code.get(from..).unwrap_or_default();
    CASE.find(rest).is_none_or(|m| {
        Regex::new(&format!(r"\b{}\b", regex::escape(name)))
            .is_ok_and(|re| !re.is_match(rest.get(m.end()..).unwrap_or_default()))
    })
}

pub fn scala_arm_subject(text: &str, line1: usize) -> Option<String> {
    static SAME: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^\s*|[^=!<>]=\s*)(\w+(?:\(\s*\))?)\s+match\s*\{\s*case\b")
            .expect("the regex is a literal")
    });
    static OPENS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(\w+(?:\(\s*\))?)\s+match\s*\{?\s*$").expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    let code = uncommented(Kind::Jvm, lines[at]);
    if code.contains("match") {
        let c = SAME.captures(&code)?;
        return (code.split("match").count() == 2).then(|| c[1].to_owned());
    }
    code.trim_start().starts_with("case").then_some(())?;
    let opener = uncommented(Kind::Jvm, lines[super::jvm::scope_of(&lines, at)?]);
    Some(OPENS.captures(&opener)?[1].to_owned())
}

pub fn scala_option_of(line: &str, name: &str) -> Option<String> {
    let code = uncommented(Kind::Jvm, line);
    Regex::new(&format!(
        r"(?:^|[^\w.]){}\s*:\s*Option\[([A-Z]\w*)\]",
        regex::escape(name)
    ))
    .ok()?
    .captures(&code)
    .map(|c| c[1].to_owned())
}

pub fn scala_extracted(line: &str, name: &str) -> Option<(String, usize, usize)> {
    static PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:val|var)\s+([A-Z]\w*)\s*\(([\w\s,]+)\)\s*=|case\s+([A-Z]\w*)\s*\(([\w\s,]+)\)\s*(?:=>|if\b))")
            .expect("the regex is a literal")
    });
    static INLINE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bmatch\s*\{\s*case\s+([A-Z]\w*)\s*\(([\w\s,]+)\)\s*(?:=>|if\b)")
            .expect("the regex is a literal")
    });
    let code = uncommented(Kind::Jvm, line);
    let (class, names) = if let Some(c) = PATTERN.captures(&code) {
        c.get(1).zip(c.get(2)).or_else(|| c.get(3).zip(c.get(4)))?
    } else {
        let c = INLINE.captures(&code)?;
        (code.split("match").count() == 2 && alone_in_arm(&code, c.get(0)?.end(), name))
            .then_some(())?;
        c.get(1).zip(c.get(2))?
    };
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
