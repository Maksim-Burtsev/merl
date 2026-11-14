use regex::Regex;

use super::csharp::{cs_after_attributes, cs_enclosing, cs_split, cs_type_decl};
use super::{Kind, code};

pub fn cs_generic_param(text: &str, line: usize, name: &str) -> bool {
    cs_generic_params(text, line, name, false).is_some()
}
pub fn cs_generic_param_line(text: &str, line: usize, name: &str) -> Option<usize> {
    cs_generic_params(text, line, name, true)
}

fn cs_declares_generic(l: &str, at: usize) -> bool {
    static NAMED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"([\w>\]?])\s+@?\w+\s*$").expect("the regex is a literal")
    });
    if cs_type_decl(l).is_some() {
        return true;
    }
    let head = l.get(..at).unwrap_or_default();
    NAMED.captures(head).is_some_and(|c| {
        cs_ends_in_type(
            head.get(..c.get(1).map_or(0, |m| m.end()))
                .unwrap_or_default(),
        )
    })
}

fn cs_ends_in_type(s: &str) -> bool {
    if let Some(rest) = s.strip_suffix('>') {
        return rest.matches('<').count() > rest.matches('>').count();
    }
    if let Some(rest) = s.strip_suffix('?') {
        return rest.ends_with(|c: char| c.is_alphanumeric() || "_>]".contains(c));
    }
    let word = s.rsplit(|c: char| !c.is_alphanumeric() && c != '_').next();
    s.ends_with(']')
        || !matches!(
            word,
            Some(
                "abstract"
                    | "and"
                    | "as"
                    | "ascending"
                    | "async"
                    | "await"
                    | "base"
                    | "by"
                    | "case"
                    | "checked"
                    | "const"
                    | "default"
                    | "descending"
                    | "do"
                    | "else"
                    | "equals"
                    | "event"
                    | "explicit"
                    | "extern"
                    | "file"
                    | "from"
                    | "group"
                    | "implicit"
                    | "in"
                    | "internal"
                    | "into"
                    | "is"
                    | "join"
                    | "let"
                    | "nameof"
                    | "new"
                    | "not"
                    | "on"
                    | "operator"
                    | "or"
                    | "orderby"
                    | "out"
                    | "override"
                    | "params"
                    | "partial"
                    | "private"
                    | "protected"
                    | "public"
                    | "readonly"
                    | "ref"
                    | "required"
                    | "return"
                    | "scoped"
                    | "sealed"
                    | "select"
                    | "sizeof"
                    | "static"
                    | "this"
                    | "throw"
                    | "typeof"
                    | "unchecked"
                    | "unsafe"
                    | "using"
                    | "virtual"
                    | "volatile"
                    | "when"
                    | "where"
                    | "with"
                    | "yield"
            )
        )
}

fn cs_generic_params(text: &str, line: usize, name: &str, strict: bool) -> Option<usize> {
    static PARAMS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\w\s*<([^()<>]*)>\s*(?:\(|:|\{|\bwhere\b|$)").expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let mut k = line.checked_sub(1).filter(|&k| k < lines.len())?;
    let declares = |l: &str| {
        let l = cs_after_attributes(l);
        let b = l.as_bytes();
        let assigned = code(Kind::CSharp, l).find(|&(i, c)| {
            c == b'=' && b.get(i + 1) != Some(&b'=') && !(i > 0 && b"=!<>".contains(&b[i - 1]))
        });
        let l = assigned.and_then(|(i, _)| l.get(..i)).unwrap_or(l);
        let l = match cs_type_decl(l) {
            Some(_) => cs_base_colon(l).and_then(|i| l.get(..i)).unwrap_or(l),
            None => l,
        };
        PARAMS.captures_iter(l).any(|c| {
            let at = c.get(1).map_or(0, |m| m.start() - 1);
            (!strict || cs_declares_generic(l, at))
                && cs_split(&c[1]).iter().any(|p| {
                    let p = p.trim();
                    let p = p
                        .strip_prefix("in ")
                        .or_else(|| p.strip_prefix("out "))
                        .unwrap_or(p);
                    p.trim() == name
                })
        })
    };
    loop {
        if declares(lines[k]) {
            return Some(k + 1);
        }
        k = cs_enclosing(&lines, k)?;
    }
}

fn cs_base_colon(l: &str) -> Option<usize> {
    let b = l.as_bytes();
    code(Kind::CSharp, l)
        .find(|&(i, c)| c == b':' && b.get(i + 1) != Some(&b':') && !(i > 0 && b[i - 1] == b':'))
        .map(|(i, _)| i)
}
