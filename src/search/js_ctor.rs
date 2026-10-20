use regex::Regex;

use super::indent;

pub fn js_constructor(line: &str) -> Option<&str> {
    static FUNCTION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:export\s+(?:default\s+)?)?function\s+([A-Za-z_$][\w$]*)\s*\(")
            .expect("the regex is a literal")
    });
    Some(FUNCTION.captures(line)?.get(1)?.as_str())
}
pub fn js_exported_class(line: &str) -> bool {
    static CLASS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:module\.)?exports\.[\w$]+|module\.exports)\s*=\s*class\b")
            .expect("the regex is a literal")
    });
    CLASS.is_match(line)
}
pub fn prototype_owner(header: &str) -> Option<&str> {
    static METHOD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^([A-Za-z_$][\w$]*)\.prototype\.[\w$]+\s*=\s*(?:async\s+)?function\b")
            .expect("the regex is a literal")
    });
    Some(METHOD.captures(header)?.get(1)?.as_str())
}
pub fn prototype_members(text: &str, name: &str, word: &str) -> Vec<usize> {
    let assigned = Regex::new(&format!(
        r"^\s*{}\.prototype\.{}\s*=[^=]",
        regex::escape(name),
        regex::escape(word)
    ))
    .expect("an escaped name keeps the pattern valid");
    (1..)
        .zip(text.lines())
        .filter(|(_, l)| assigned.is_match(l))
        .map(|(n, _)| n)
        .collect()
}
pub fn prototype_mixed(text: &str, line1: usize, name: &str) -> bool {
    let mixin = Regex::new(&format!(
        r"^\s*Object\.assign\(\s*{}\.prototype\s*,\s*\{{\s*$",
        regex::escape(name)
    ))
    .expect("an escaped name keeps the pattern valid");
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let depth = indent(lines[at]);
    lines[..at]
        .iter()
        .rev()
        .find(|l| !l.trim().is_empty() && indent(l) < depth)
        .is_some_and(|l| mixin.is_match(l))
}
pub fn constructor_field(text: &str, decl: usize, word: &str) -> Option<usize> {
    let assigned = Regex::new(&format!(r"^this\.{}\s*=[^=]", regex::escape(word)))
        .expect("an escaped name keeps the pattern valid");
    let lines: Vec<&str> = text.lines().collect();
    let head = indent(lines.get(decl.checked_sub(1)?)?);
    let body = lines.get(decl..)?;
    let end = body
        .iter()
        .position(|l| !l.trim().is_empty() && indent(l) <= head)?;
    let level = body[..end]
        .iter()
        .find(|l| !l.trim().is_empty())
        .map(|l| indent(l))?;
    (0..end)
        .find(|&i| indent(body[i]) == level && assigned.is_match(body[i].trim_start()))
        .map(|i| decl + i + 1)
}
pub fn ts_assigned_class(line: &str) -> Option<&str> {
    static ASSIGNED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*(?::[^=]*)?=\s*class\b",
        )
        .expect("the regex is a literal")
    });
    Some(ASSIGNED.captures(line)?.get(1)?.as_str())
}
