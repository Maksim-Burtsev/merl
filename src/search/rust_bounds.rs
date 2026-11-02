use std::sync::LazyLock;

use regex::Regex;

use super::indent;

pub fn rust_signature(lines: &[&str], at: usize) -> Option<String> {
    static FN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\bfn\s+\w+").expect("the regex is a literal"));
    let f = (0..=at).rev().take(20).find(|&i| FN.is_match(lines[i]))?;
    if lines[f..at].iter().any(|l| l.contains(['{', ';'])) {
        return None;
    }
    let mut sig = String::new();
    for l in &lines[f..] {
        sig.push_str(l);
        sig.push(' ');
        if l.contains('{') || l.trim_end().ends_with(';') {
            break;
        }
    }
    Some(sig)
}
pub fn rust_bounds(lines: &[&str], at: usize, param: &str) -> Vec<String> {
    let Some(sig) = rust_signature(lines, at) else {
        return Vec::new();
    };
    let Ok(re) = Regex::new(&format!(
        r"(?:<|,|\bwhere)\s*{}\s*:\s*([^,{{>]+)",
        regex::escape(param)
    )) else {
        return Vec::new();
    };
    re.captures_iter(&sig)
        .flat_map(|c| bound_names(&c[1]))
        .collect()
}
fn bound_names(bounds: &str) -> Vec<String> {
    bounds
        .split('+')
        .filter_map(|b| {
            let name = b.trim().split('<').next()?.rsplit("::").next()?.trim();
            name.starts_with(|c: char| c.is_ascii_uppercase())
                .then(|| name.to_owned())
        })
        .collect()
}
pub fn rust_assoc_bounds(lines: &[&str], trait0: usize, assoc: &str) -> Vec<String> {
    let Ok(re) = Regex::new(&format!(
        r"^\s*type\s+{}\s*:([^;={{]+);",
        regex::escape(assoc)
    )) else {
        return Vec::new();
    };
    let end = (trait0 + 1..lines.len())
        .find(|&i| {
            lines[i].trim_start().starts_with('}') && indent(lines[i]) <= indent(lines[trait0])
        })
        .unwrap_or(lines.len());
    let found: Vec<Vec<String>> = (trait0 + 1..end)
        .filter_map(|i| re.captures(lines[i]).map(|c| bound_names(&c[1])))
        .collect();
    match <[Vec<String>; 1]>::try_from(found) {
        Ok([one]) => one,
        Err(_) => Vec::new(),
    }
}
