use regex::Regex;
use std::sync::LazyLock;

use super::dart::DartType;
use super::{Kind, close_of, code, split_top};

const DECLARED: &str = r"(?:([a-z_$][\w$]*)\.)?(_?[A-Z][\w$]*)\??";

pub enum DartSlot {
    Positional(usize),
    Named(String),
}

pub enum DartShorthand {
    Case(String),
    Returned(DartType),
    Argument(String, DartSlot),
}

pub fn dart_shorthand(text: &str, at: usize, before: &str, after: &str) -> Option<DartShorthand> {
    static ARROW: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(
            r"^\s*(?:static\s+)?{DECLARED}\s+(?:get\s+[a-z_$][\w$]*|[a-z_$][\w$]*\s*\([^()]*\))\s*=>$"
        ))
        .expect("the regex is a literal")
    });
    static RETURN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:^|[\s;{}])return$").expect("the regex is a literal"));
    let head = before.strip_suffix('.')?.trim_end();
    if header(text, at).trim() == "." && after.trim_start().starts_with("=>") {
        return switch_subject(text, at).map(DartShorthand::Case);
    }
    if head.ends_with(|c: char| c.is_alphanumeric() || "_$)]}.?!'\"".contains(c))
        && !head.ends_with("case")
        && !head.ends_with("return")
    {
        return None;
    }
    if head.trim_start() == "case" {
        return switch_subject(text, at).map(DartShorthand::Case);
    }
    if let Some(c) = ARROW.captures(head) {
        return Some(DartShorthand::Returned(declared(&c)));
    }
    if RETURN.is_match(head) {
        return returned(text, at).map(DartShorthand::Returned);
    }
    argument(head).map(|(callee, slot)| DartShorthand::Argument(callee, slot))
}

fn declared(c: &regex::Captures) -> DartType {
    DartType {
        prefix: c.get(1).map(|m| m.as_str().to_owned()),
        class: c[2].to_owned(),
        constructor: None,
    }
}

fn open_braces(text: &str, at: usize) -> Vec<usize> {
    let mut open = Vec::new();
    for (i, c) in code(Kind::Dart, text.get(..at).unwrap_or_default()) {
        match c {
            b'{' => open.push(i),
            b'}' => {
                open.pop();
            }
            _ => {}
        }
    }
    open
}

fn header(text: &str, brace: usize) -> &str {
    let start = text
        .get(..brace)
        .and_then(|t| t.rfind('\n'))
        .map_or(0, |n| n + 1);
    text.get(start..brace).unwrap_or_default()
}

fn switch_subject(text: &str, at: usize) -> Option<String> {
    static SWITCH: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w$.])switch\s*\(\s*([a-z_$][\w$]*)\s*\)\s*$")
            .expect("the regex is a literal")
    });
    let brace = *open_braces(text, at).last()?;
    Some(SWITCH.captures(header(text, brace))?[1].to_owned())
}

fn returned(text: &str, at: usize) -> Option<DartType> {
    static FLOW: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^\s*(?:\}\s*)?(?:(?:else\s+)?if\s*\(.*\)|else|for\s*\(.*\)|while\s*\(.*\)|do|try|finally|switch\s*\(.*\)|(?:on\s+[\w$.<>]+\s*)?catch\s*\(.*\)|on\s+[\w$.<>]+)\s*$",
        )
        .expect("the regex is a literal")
    });
    static FUNCTION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(
            r"^\s*(?:@[\w$.]+\s+)*(?:(?:static|external)\s+)*{DECLARED}\s+(?:get\s+[a-z_$][\w$]*|[a-z_$][\w$]*\s*\([^()]*\))\s*$"
        ))
        .expect("the regex is a literal")
    });
    for brace in open_braces(text, at).into_iter().rev() {
        let head = header(text, brace);
        if let Some(c) = FUNCTION.captures(head) {
            return Some(declared(&c));
        }
        if !FLOW.is_match(head) {
            return None;
        }
    }
    None
}

fn argument(head: &str) -> Option<(String, DartSlot)> {
    static NAMED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*([a-z_$][\w$]*)\s*:\s*$").expect("the regex is a literal")
    });
    static NAMED_ARG: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*[a-z_$][\w$]*\s*:").expect("the regex is a literal"));
    static CALLEE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w$.?!])([a-z_$][\w$]*)\s*$").expect("the regex is a literal")
    });
    let mut open = Vec::new();
    for (i, c) in code(Kind::Dart, head) {
        match c {
            b'(' | b'[' | b'{' => open.push((i, c)),
            b')' | b']' | b'}' => {
                open.pop();
            }
            _ => {}
        }
    }
    let &(paren, b'(') = open.last()? else {
        return None;
    };
    let callee = CALLEE.captures(head.get(..paren)?)?[1].to_owned();
    if matches!(
        callee.as_str(),
        "if" | "for" | "while" | "switch" | "catch" | "return" | "assert"
    ) {
        return None;
    }
    let args = split_top(Kind::Dart, head.get(paren + 1..)?, b',');
    let (last, earlier) = args.split_last()?;
    if let Some(c) = NAMED.captures(last) {
        return Some((callee, DartSlot::Named(c[1].to_owned())));
    }
    if !last.trim().is_empty() {
        return None;
    }
    let index = earlier.iter().filter(|a| !NAMED_ARG.is_match(a)).count();
    Some((callee, DartSlot::Positional(index)))
}

pub fn dart_param_type(line: &str, name: &str, slot: &DartSlot) -> Option<DartType> {
    static PARAM: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(
            r"^\s*(?:(?:required|covariant|final)\s+)*{DECLARED}\s+([a-z_$][\w$]*)\s*(?:=.*)?$"
        ))
        .expect("the regex is a literal")
    });
    let open = Regex::new(&format!(r"\b{}\s*\(", regex::escape(name)))
        .ok()?
        .find(line)?
        .end()
        - 1;
    let close = close_of(Kind::Dart, line, open)?;
    let params = line.get(open + 1..close - 1)?;
    let mut positional = Vec::new();
    let mut named = Vec::new();
    for part in split_top(Kind::Dart, params, b',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match part.strip_prefix('{') {
            Some(group) => named.extend(split_top(Kind::Dart, group.strip_suffix('}')?, b',')),
            None => match part.strip_prefix('[') {
                Some(group) => {
                    positional.extend(split_top(Kind::Dart, group.strip_suffix(']')?, b','));
                }
                None => positional.push(part),
            },
        }
    }
    let c = match slot {
        DartSlot::Positional(i) => PARAM.captures(positional.get(*i)?)?,
        DartSlot::Named(n) => named
            .iter()
            .filter_map(|p| PARAM.captures(p))
            .find(|c| c[3] == *n)?,
    };
    Some(declared(&c))
}
