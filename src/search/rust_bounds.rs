use std::sync::LazyLock;

use regex::Regex;

use super::rust::{past_generics, top_split};
use super::{Kind, code, indent};

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
    let found: Vec<Vec<String>> = (trait0 + 1..block_end(lines, trait0))
        .filter_map(|i| re.captures(lines[i]).map(|c| bound_names(&c[1])))
        .collect();
    match <[Vec<String>; 1]>::try_from(found) {
        Ok([one]) => one,
        Err(_) => Vec::new(),
    }
}
fn inside_parens(s: &str) -> Option<&str> {
    let mut depth = 0i32;
    for (i, c) in code(Kind::Rust, s) {
        match c {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return s.get(1..i);
                }
            }
            _ => {}
        }
    }
    None
}
pub fn rust_closure_input(sig: &str, arg: usize, input: usize) -> Option<String> {
    static FN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\bfn\s+\w+\s*").expect("the regex is a literal"));
    static CLOSURE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:\w+::)*Fn(?:Mut|Once)?\s*\(").expect("the regex is a literal")
    });
    let open = past_generics(sig, FN.find(sig)?.end())?;
    let params: Vec<&str> = top_split(inside_parens(sig.get(open..)?)?)
        .into_iter()
        .map(str::trim)
        .filter(|p| !p.is_empty() && !p.trim_start_matches(['&', ' ']).ends_with("self"))
        .collect();
    let written = params.get(arg)?.split_once(':')?.1.trim();
    let bound = if let Some(b) = written.strip_prefix("impl ") {
        b.trim_start()
    } else {
        let re = Regex::new(&format!(
            r"(?:<|,|\bwhere)\s*{}\s*:\s*",
            regex::escape(written)
        ))
        .ok()?;
        let [m] = <[_; 1]>::try_from(re.find_iter(sig).collect::<Vec<_>>()).ok()?;
        sig.get(m.end()..)?
    };
    let at = CLOSURE.find(bound)?.end() - 1;
    let inputs = top_split(inside_parens(bound.get(at..)?)?);
    inputs.get(input).map(|t| t.trim().to_owned())
}
pub fn rust_assoc_type(lines: &[&str], impl0: usize, assoc: &str) -> Option<String> {
    let re = Regex::new(&format!(
        r"^\s*type\s+{}\s*=\s*([^;]+);",
        regex::escape(assoc)
    ))
    .ok()?;
    let found: Vec<String> = (impl0 + 1..block_end(lines, impl0))
        .filter_map(|i| re.captures(lines[i]).map(|c| c[1].trim().to_owned()))
        .collect();
    <[String; 1]>::try_from(found).ok().map(|[one]| one)
}
fn block_end(lines: &[&str], open0: usize) -> usize {
    (open0 + 1..lines.len())
        .find(|&i| {
            lines[i].trim_start().starts_with('}') && indent(lines[i]) <= indent(lines[open0])
        })
        .unwrap_or(lines.len())
}
