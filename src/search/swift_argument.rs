use regex::Regex;

use super::{Kind, SwiftExpr, code, group, split_top, swift_expr, swift_mods, uncommented};

#[derive(Debug, PartialEq, Eq)]
pub struct SwiftCall {
    pub labels: Vec<String>,
    pub args: Vec<String>,
    pub trailing_closure: bool,
}
pub fn swift_call_labels(lines: &[&str], line0: usize, after: usize) -> Option<SwiftCall> {
    static LABEL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*`?(\w+)`?\s*:").expect("the regex is a literal")
    });
    let line = lines.get(line0)?;
    let rest = line.get(after..)?;
    if rest.trim_start().starts_with('{') {
        let condition = ["if ", "guard ", "while ", "for ", "switch ", "} else"]
            .iter()
            .any(|k| line.trim_start().starts_with(k));
        return (!condition).then(|| SwiftCall {
            labels: Vec::new(),
            args: Vec::new(),
            trailing_closure: true,
        });
    }
    if !rest.starts_with('(') {
        return None;
    }
    let g = group(Kind::Swift, lines, line0, after)?;
    let trailing_closure = g.after_close.trim_start().starts_with('{');
    let inner = g.inner_uncommented;
    let (labels, args) = if inner.trim().is_empty() {
        (Vec::new(), Vec::new())
    } else {
        split_top(Kind::Swift, &inner, b',')
            .into_iter()
            .map(|a| match LABEL.captures(a) {
                Some(c) => (
                    c[1].to_owned(),
                    a.get(c[0].len()..).unwrap_or("").trim().to_owned(),
                ),
                None => ("_".to_owned(), a.trim().to_owned()),
            })
            .unzip()
    };
    Some(SwiftCall {
        labels,
        args,
        trailing_closure,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub struct SwiftArgument {
    pub owner: String,
    pub callee: String,
    pub open: usize,
    pub label: String,
    pub on_value: bool,
}

fn swift_open_argument(lead: &str) -> Option<(usize, String)> {
    static LABEL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^`?(\w+)`?\s*:$").expect("the regex is a literal")
    });
    let bytes: Vec<(usize, u8)> = code(Kind::Swift, lead).collect();
    let mut depth = 0usize;
    let mut open = None;
    for &(i, c) in bytes.iter().rev() {
        match c {
            b')' | b']' | b'}' => depth += 1,
            b'(' if depth == 0 => {
                open = Some(i);
                break;
            }
            b'[' | b'{' if depth == 0 => return None,
            b'(' | b'[' | b'{' => depth -= 1,
            _ => {}
        }
    }
    let open = open?;
    let args = split_top(Kind::Swift, lead.get(open + 1..)?, b',');
    let last = args.last().map_or("", |a| a.trim());
    let label = match LABEL.captures(last) {
        Some(c) => c[1].to_owned(),
        None if last.is_empty() && args.len() <= 1 => "_".to_owned(),
        None => return None,
    };
    Some((open, label))
}

pub fn swift_free_argument(lead: &str) -> Option<(String, usize, String)> {
    static FREE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.?!`#@])([a-z_]\w*)\s*$").expect("the regex is a literal")
    });
    let (open, label) = swift_open_argument(lead)?;
    let callee = FREE.captures(lead.get(..open)?)?[1].to_owned();
    let keyword = [
        "if", "guard", "while", "switch", "return", "case", "in", "try", "await", "throw", "for",
        "repeat", "where", "is", "as", "let", "var", "func", "init", "self", "super", "some",
        "any", "else",
    ];
    (!keyword.contains(&callee.as_str())).then_some((callee, open, label))
}

pub fn swift_argument(lead: &str) -> Option<SwiftArgument> {
    static CALLEE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.?!`])([A-Z]\w*(?:\.[A-Z]\w*)*)(?:\.([a-z_]\w*))?\s*$")
            .expect("the regex is a literal")
    });
    static METHOD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.?!`])([a-z_]\w*(?:\.[a-z_]\w*)*)\.([a-z_]\w*)\s*$")
            .expect("the regex is a literal")
    });
    let (open, label) = swift_open_argument(lead)?;
    let before = lead.get(..open)?;
    if let Some(c) = CALLEE.captures(before) {
        return Some(SwiftArgument {
            owner: c[1].to_owned(),
            callee: c.get(2).map_or("init", |m| m.as_str()).to_owned(),
            open,
            label,
            on_value: false,
        });
    }
    let c = METHOD.captures(before)?;
    Some(SwiftArgument {
        owner: c[1].to_owned(),
        callee: c[2].to_owned(),
        open,
        label,
        on_value: true,
    })
}

pub fn swift_case_value(value: &str) -> Option<(String, String)> {
    let (SwiftExpr::Call(e) | SwiftExpr::Chain(e)) = swift_expr(value)? else {
        return None;
    };
    let (ty, case) = e.rsplit_once('.')?;
    let typed = ty.split('.').all(|p| p.starts_with(char::is_uppercase));
    (typed && case.starts_with(|c: char| c.is_lowercase() || c == '_'))
        .then(|| (ty.to_owned(), case.to_owned()))
}

pub fn swift_var(line: &str, name: &str) -> bool {
    Regex::new(&format!(r"\bvar\s+`?{}`?\s*=", regex::escape(name))).is_ok_and(|r| r.is_match(line))
}

pub fn swift_declares_func(line: &str) -> Option<bool> {
    static DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(r"{}(func|let|var)\s", swift_mods!())).expect("the regex is a literal")
    });
    DECL.captures(line).map(|c| &c[1] == "func")
}

pub fn swift_call_fields(e: &str) -> Option<(String, Vec<String>)> {
    let e = e.trim();
    let (call, tail) = e.split_at(e.rfind(')')? + 1);
    let fields: Vec<String> = tail
        .strip_prefix('.')?
        .split('.')
        .map(str::to_owned)
        .collect();
    let ident = |f: &String| {
        f.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && f.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    (fields.iter().all(ident) && matches!(swift_expr(call)?, SwiftExpr::Call(_)))
        .then(|| (call.to_owned(), fields))
}

pub fn swift_compared(
    lines: &[&str],
    literal: &[bool],
    line1: usize,
    lead: &str,
) -> Option<String> {
    static EQUALS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.?!)\]`])`?([a-z_]\w*)`?\s*(?:==|!=)\s*$")
            .expect("the regex is a literal")
    });
    static ARM: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*case\s+(?:\.\w+\s*,\s*)*$").expect("the regex is a literal")
    });
    static SUBJECT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:return\s+)?switch\s+`?([a-z_]\w*)`?\s*\{\s*$")
            .expect("the regex is a literal")
    });
    let lead = uncommented(Kind::Swift, lead);
    if let Some(c) = EQUALS.captures(&lead) {
        return Some(c[1].to_owned());
    }
    if !ARM.is_match(&lead) {
        return None;
    }
    let switch = super::swift_switch_of_arm(lines, literal, line1)?;
    let header = uncommented(Kind::Swift, lines.get(switch - 1)?);
    Some(SUBJECT.captures(&header)?[1].to_owned())
}
