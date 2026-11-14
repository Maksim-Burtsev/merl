use regex::Regex;

use super::{CS_TYPE, Kind, close_of, split_top, uncommented};

fn var_pattern(code: &str, name: &str) -> Option<(usize, usize, usize)> {
    static VAR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bvar\s*\(([\w@\s,]+)\)\s*=\s*").expect("the regex is a literal")
    });
    let c = VAR.captures(code)?;
    let names: Vec<&str> = c[1].split(',').collect();
    let at = (names.iter())
        .position(|n| n.trim().trim_start_matches('@') == name.trim_start_matches('@'))?;
    Some((c.get_match().end(), at, names.len()))
}

pub(super) fn cs_deconstructed<'a>(code: &'a str, name: &str) -> Option<&'a str> {
    let (open, at, count) = var_pattern(code, name)?;
    code.get(open..)?.starts_with('(').then_some(())?;
    let close = close_of(Kind::CSharp, code, open)?;
    let values = split_top(Kind::CSharp, code.get(open + 1..close - 1)?, b',');
    let ends = code
        .get(close..)?
        .trim()
        .trim_end_matches(';')
        .trim()
        .is_empty();
    (ends && values.len() == count).then_some(values.get(at)?.trim())
}

pub(super) fn cs_deconstructed_from<'a>(
    code: &'a str,
    name: &str,
) -> Option<(&'a str, usize, usize)> {
    let (start, at, count) = var_pattern(code, name)?;
    let value = code.get(start..)?.trim();
    (!value.starts_with('(')).then_some((value, at, count))
}

pub fn cs_tuple_item(written: &str, at: usize, count: usize) -> Option<String> {
    static ITEM: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(r"^({CS_TYPE})(?:\s+@?\w+)?$")).expect("the regex is a literal")
    });
    let inner = written.trim().strip_prefix('(')?.strip_suffix(')')?;
    let items = split_top(Kind::CSharp, inner, b',');
    (items.len() == count).then_some(())?;
    Some(ITEM.captures(items.get(at)?.trim())?[1].to_owned())
}

pub fn cs_out_params(line: &str) -> Vec<String> {
    static OUT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(r"\bout\s+({CS_TYPE})\s+@?\w+")).expect("the regex is a literal")
    });
    let code = uncommented(Kind::CSharp, line);
    OUT.captures_iter(&code).map(|c| c[1].to_owned()).collect()
}
