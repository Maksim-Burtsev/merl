use std::sync::LazyLock;

use regex::Regex;

use super::ruby_locals;

pub enum RubyReturns {
    Itself,
    New(String),
}

fn continues(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with('.') && !t.starts_with("..") || t.starts_with("&.")
}

fn skipped(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with('#')
}

pub fn ruby_joined_before(text: &str, line1: usize, before: &str) -> String {
    if !continues(before) {
        return before.to_owned();
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut joined = before.trim_start().to_owned();
    for line in lines
        .get(..line1.saturating_sub(1))
        .unwrap_or_default()
        .iter()
        .rev()
    {
        if skipped(line) {
            continue;
        }
        joined.insert_str(0, line.trim());
        if !continues(line) {
            break;
        }
    }
    joined
}

pub fn ruby_local_name(expr: &str) -> bool {
    expr.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
        && expr.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !matches!(
            expr,
            "self" | "nil" | "true" | "false" | "end" | "super" | "then" | "do" | "not"
        )
}

pub fn ruby_tapped<'a>(prefix: &'a str, name: &str) -> Option<&'a str> {
    static TAP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(.*)\.tap\s*(?:\{|\bdo)\s*\|\s*(\w+)\s*\|\s*$")
            .expect("the regex is a literal")
    });
    let c = TAP.captures(prefix)?;
    (&c[2] == name).then(|| c.get(1).map_or("", |m| m.as_str()))
}

pub fn ruby_assigned_once(text: &str, line1: usize, name: &str) -> Option<(usize, String)> {
    let earlier: Vec<usize> = ruby_locals(text, line1, name)
        .into_iter()
        .filter(|&l| l < line1)
        .collect();
    let [at] = earlier[..] else {
        return None;
    };
    let lines: Vec<&str> = text.lines().collect();
    let line = lines.get(at.checked_sub(1)?)?;
    let value = line.trim_start().strip_prefix(name)?.trim_start();
    let value = value
        .strip_prefix('=')
        .filter(|v| !v.starts_with(['=', '~', '>']))?;
    let mut value = value.trim().to_owned();
    for next in lines.get(at..line1.saturating_sub(1)).unwrap_or_default() {
        if skipped(next) {
            continue;
        }
        if !continues(next) {
            break;
        }
        value.push_str(next.trim());
    }
    Some((at, value))
}

pub fn ruby_constant_alias(name: &str) -> String {
    format!(r"^\s*{}\s*=\s*(?:::)?[A-Z][\w:]*\s*$", regex::escape(name))
}

pub fn ruby_alias_target<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let value = line
        .trim()
        .strip_prefix(name)?
        .trim_start()
        .strip_prefix('=')?
        .trim();
    value
        .trim_start_matches(':')
        .starts_with(|c: char| c.is_ascii_uppercase())
        .then_some(value)
}

pub fn ruby_last_value(text: &str, def_line1: usize) -> Option<RubyReturns> {
    static NEW: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^((?:::)?[A-Z][\w:]*)\.new(?:\(.*\))?$").expect("the regex is a literal")
    });
    static RETURN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\breturn\b").expect("the regex is a literal"));
    let lines: Vec<&str> = text.lines().collect();
    let def = lines.get(def_line1.checked_sub(1)?)?;
    let t = def.trim_start();
    if !t.starts_with("def ") || t.contains('=') || t.contains(';') {
        return None;
    }
    let indent = def.len() - t.len();
    let body: Vec<&str> = lines
        .get(def_line1..)?
        .iter()
        .take_while(|l| skipped(l) || l.len() - l.trim_start().len() > indent)
        .copied()
        .collect();
    let close = lines.get(def_line1 + body.len())?;
    if !close.trim_start().starts_with("end") || body.iter().any(|l| RETURN.is_match(l)) {
        return None;
    }
    let last = body.iter().rev().find(|l| !skipped(l))?.trim();
    if last == "self" {
        return Some(RubyReturns::Itself);
    }
    NEW.captures(last)
        .map(|c| RubyReturns::New(c[1].to_owned()))
}
