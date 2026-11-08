use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use super::{
    Kind, code, comment, indent, literal_lines, path_literal_lines, scala, scala_type_parameter,
    split_top, uncommented,
};

pub fn in_prose(kind: Kind, path: &Path, text: &str, line1: usize, col: usize) -> bool {
    let Some(line) = line1.checked_sub(1).and_then(|k| text.lines().nth(k)) else {
        return true;
    };
    let lead = line.get(..col).unwrap_or(line);
    comment(kind, line.trim())
        || path_literal_lines(kind, path, text).get(line1 - 1) == Some(&true)
        || lead.rfind("/*") > lead.rfind("*/")
        || uncommented(kind, line).len() <= col
        || (!lead.contains('$') && !code(kind, line).any(|(i, _)| i == col))
}

fn jvm_declares(t: &str, name: &str) -> bool {
    static HEAD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:\b(?:class|interface|record|typealias)\s+[A-Za-z_]\w*\s*|\b(?:fun|val|var)\s*|^(?:@\w+(?:\([^()]*\))?\s+|(?:public|protected|private|static|final|abstract|synchronized|native|default|strictfp)\s+)*)<([^<>]*(?:<[^<>]*>[^<>]*)*)>").expect("the regex is a literal")
    });
    HEAD.captures_iter(t).any(|c| {
        split_top(Kind::Jvm, &c[1], b',').iter().any(|e| {
            e.split_whitespace()
                .find(|w| !matches!(*w, "in" | "out" | "reified"))
                .is_some_and(|w| w.trim_end_matches(':') == name)
        })
    })
}

fn scala_declares(t: &str, name: &str) -> bool {
    static ANONYMOUS_GIVEN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\bgiven\s+[\w.]+\s*\[[^\]]*\]\s*(?:with\b|=|\{|$)")
            .expect("the regex is a literal")
    });
    !ANONYMOUS_GIVEN.is_match(t) && scala_type_parameter(t, name)
}

pub fn generic_param_line(
    kind: Kind,
    path: &Path,
    text: &str,
    line1: usize,
    name: &str,
) -> Option<usize> {
    let declares = |t: &str| match kind {
        Kind::Dart => dart_declares(t, name),
        Kind::Jvm if scala(path) => scala_declares(t, name),
        Kind::Jvm => jvm_declares(t, name),
        _ => false,
    };
    let lines: Vec<&str> = text.lines().collect();
    let literal = path_literal_lines(kind, path, text);
    let mut depth = usize::MAX;
    for i in (0..line1.min(lines.len())).rev() {
        let code = uncommented(kind, lines[i]);
        let t = code.trim();
        let ind = indent(lines[i]);
        if t.is_empty()
            || comment(kind, t)
            || literal.get(i) == Some(&true)
            || (ind >= depth && i + 1 != line1)
        {
            continue;
        }
        depth = ind + usize::from(t.starts_with([')', '>', ']']));
        if declares(t) {
            return Some(i + 1);
        }
        if let Some(at) = wrapped_list(kind, &lines, &literal, i)
            .filter(|&last| declares(&joined(kind, &lines, i, last)))
            .and_then(|last| {
                (i + 1..=last).find(|&j| holds_word(&uncommented(kind, lines[j]), name))
            })
        {
            return Some(at + 1);
        }
        if depth == 0 {
            break;
        }
    }
    None
}

fn wrapped_list(kind: Kind, lines: &[&str], literal: &[bool], i: usize) -> Option<usize> {
    let (open, close) = match uncommented(kind, lines[i]).trim_end().chars().last()? {
        '<' => ('<', '>'),
        '[' => ('[', ']'),
        _ => return None,
    };
    let mut depth = 1i32;
    for (j, line) in lines.iter().enumerate().skip(i + 1).take(15) {
        if literal.get(j) == Some(&true) {
            return None;
        }
        let l = uncommented(kind, line);
        for (k, c) in l.char_indices() {
            let arrow = || l.get(..k).is_some_and(|p| p.ends_with(['-', '=']));
            if c == open {
                depth += 1;
            } else if c == close && !(open == '<' && arrow()) {
                depth -= 1;
            }
        }
        if depth <= 0 {
            return Some(j);
        }
    }
    None
}

fn joined(kind: Kind, lines: &[&str], from: usize, to: usize) -> String {
    (from..=to)
        .map(|j| uncommented(kind, lines[j]).trim().to_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn holds_word(code: &str, name: &str) -> bool {
    code.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .any(|w| w == name)
}

fn dart_declares(t: &str, name: &str) -> bool {
    static HEADER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:(?:abstract|sealed|base|final|interface|mixin)\s+)*(?:class|mixin|extension|typedef)\s+[\w$]+\s*<|^(?:(?:static|external)\s+)*(?P<ret>[\w$.]+)(?:<[^()=]*>)?\??\s+[\w$]+\s*<")
            .expect("the regex is a literal")
    });
    const STATEMENTS: [&str; 13] = [
        "return", "await", "yield", "throw", "new", "const", "final", "var", "late", "if", "else",
        "case", "assert",
    ];
    let Some(c) = HEADER.captures(t) else {
        return false;
    };
    let ret = c.name("ret");
    if ret.is_some_and(|r| STATEMENTS.contains(&r.as_str())) {
        return false;
    }
    let Some(whole) = c.get(0) else {
        return false;
    };
    let rest = t.get(whole.end()..).unwrap_or_default();
    let mut depth = 1;
    let Some((end, _)) = rest.char_indices().find(|&(_, c)| {
        depth += match c {
            '<' => 1,
            '>' => -1,
            _ => 0,
        };
        depth == 0
    }) else {
        return false;
    };
    let after = rest.get(end + 1..).unwrap_or_default().trim_start();
    if ret.is_some() && !after.starts_with('(') {
        return false;
    }
    split_top(Kind::Dart, rest.get(..end).unwrap_or_default(), b',')
        .iter()
        .any(|p| p.split_whitespace().next() == Some(name))
}

pub fn type_param_line(kind: Kind, text: &str, line1: usize, name: &str) -> Option<usize> {
    type_param(kind, text, line1, name).map(|(at, _)| at)
}

pub struct TypeParam {
    pub bound: Option<String>,
}

pub fn type_param_bound(kind: Kind, text: &str, line1: usize, name: &str) -> Option<TypeParam> {
    let (at, bound) = type_param(kind, text, line1, name)?;
    let bound = bound?;
    let clause = match kind {
        Kind::Rust | Kind::Swift => "where",
        _ => return Some(TypeParam { bound }),
    };
    let clauses = text.lines().take(line1).skip(at - 1).any(|l| {
        coded(kind, l)
            .split(|c: char| !c.is_alphanumeric())
            .any(|w| w == clause)
    });
    Some(TypeParam {
        bound: bound.or_else(|| clauses.then(|| clause.to_owned())),
    })
}

type Bound = Option<Option<String>>;

fn type_param(kind: Kind, text: &str, line1: usize, name: &str) -> Option<(usize, Bound)> {
    static RUST_ITEM: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:pub(?:\([^)]*\))?\s+)?(?:(?:const|async|unsafe|extern(?:\s+\S+)?)\s+)*(fn|struct|enum|union|trait|impl|type|mod)\b")
            .expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(kind, text);
    let mut k = line1.checked_sub(1).filter(|&k| k < lines.len())?;
    let mut inside_item = false;
    loop {
        if kind == Kind::Rust
            && let Some(c) = RUST_ITEM.captures(coded(kind, lines[k]).trim())
        {
            if inside_item && &c[1] == "fn" {
                return None;
            }
            inside_item = true;
        }
        if let Some((at, bound)) = declares(kind, &lines, k, name) {
            return Some((at + 1, bound));
        }
        let t = uncommented(kind, lines[k]);
        let t = t.trim();
        let depth = indent(lines[k]) + usize::from(continues(t));
        k = (0..k).rev().find(|&i| {
            let t = uncommented(kind, lines[i]);
            let t = t.trim();
            !t.is_empty()
                && literal.get(i) != Some(&true)
                && !continues(t)
                && !t.starts_with(['{', '}', '#'])
                && indent(lines[i]) < depth
        })?;
    }
}

fn coded(kind: Kind, s: &str) -> String {
    let bytes: Vec<u8> = code(kind, s)
        .map(|(_, c)| c)
        .take_while(|&c| c != 0)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn continues(t: &str) -> bool {
    t.starts_with([')', ']', '>']) || t.starts_with("where") || t.starts_with("->")
}

fn declares(kind: Kind, lines: &[&str], k: usize, name: &str) -> Option<(usize, Bound)> {
    static RUST: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"\b(?:fn\s+\w+|impl|struct\s+\w+|enum\s+\w+|union\s+\w+|trait\s+\w+|type\s+\w+)\s*<",
        )
        .expect("the regex is a literal")
    });
    static GO: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:func\s+(?:\([^)]*\)\s*)?\w+|type\s+\w+)\s*\[")
            .expect("the regex is a literal")
    });
    static GO_RECEIVER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^func\s*\(\s*\w*\s*\*?\s*(\w+)\s*\[").expect("the regex is a literal")
    });
    static CPP: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\btemplate\s*<").expect("the regex is a literal"));
    static SWIFT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\b(?:func\s+\w+|init[?!]?|subscript|(?:struct|class|enum|actor)\s+\w+)\s*<")
            .expect("the regex is a literal")
    });
    let line = coded(kind, lines[k]);
    let t = line.trim();
    let named = |list: &str, last: bool| {
        let parts = top_parts(list);
        let i = parts
            .iter()
            .position(|p| param_name(p, last) == Some(name))?;
        Some(Some(param_bound(kind, &parts, i)))
    };
    let angled = |re: &Regex| {
        re.find_iter(t)
            .filter_map(|m| list_after(t.get(m.end()..)?, b'<', b'>'))
            .find_map(|l| named(l, false))
            .map(|b| (k, b))
    };
    match kind {
        Kind::Rust => angled(&RUST),
        Kind::Swift => angled(&SWIFT),
        Kind::Go => {
            let list = |re: &Regex| list_after(t.get(re.find(t)?.end()..)?, b'[', b']');
            list(&GO)
                .filter(|l| go_constrained(l))
                .and_then(|l| named(l, false))
                .or_else(|| {
                    let ty = GO_RECEIVER.captures(t)?.get(1)?.as_str();
                    let i = top_parts(list(&GO_RECEIVER)?)
                        .iter()
                        .position(|p| param_name(p, false) == Some(name))?;
                    Some(go_type_bound(lines, ty, i))
                })
                .map(|b| (k, b))
        }
        Kind::C => (0..=k)
            .rev()
            .take_while(|&i| i == k || bare_template(&uncommented(kind, lines[i])))
            .find_map(|i| {
                let l = coded(kind, lines[i]);
                CPP.find_iter(&l)
                    .filter_map(|m| list_after(l.get(m.end()..)?, b'<', b'>'))
                    .find_map(|l| named(l, true))
                    .map(|b| (i, b))
            }),
        _ => None,
    }
}

fn go_constrained(list: &str) -> bool {
    top_parts(list)
        .last()
        .and_then(|p| p.split_whitespace().nth(1))
        .is_some_and(|c| c.starts_with(|c: char| c.is_alphabetic() || "_~[(".contains(c)))
}

fn go_type_bound(lines: &[&str], ty: &str, i: usize) -> Bound {
    let head = format!("type {ty}");
    lines.iter().find_map(|l| {
        let l = coded(Kind::Go, l);
        let list = list_after(
            l.strip_prefix(&head)?.trim_start().strip_prefix('[')?,
            b'[',
            b']',
        )?;
        let parts = top_parts(list);
        (go_constrained(list) && i < parts.len()).then(|| param_bound(Kind::Go, &parts, i))
    })
}

fn param_bound(kind: Kind, parts: &[&str], i: usize) -> Option<String> {
    let part = parts[i].split('=').next().unwrap_or_default().trim();
    let bound = match kind {
        Kind::Go => parts[i..]
            .iter()
            .find_map(|p| p.trim().split_once(char::is_whitespace))
            .map(|(_, c)| c.trim())
            .filter(|c| !matches!(*c, "any" | "interface{}")),
        Kind::C => part
            .split_whitespace()
            .next()
            .filter(|w| !matches!(*w, "typename" | "class")),
        _ => part.split_once(':').map(|(_, b)| b.trim()),
    };
    bound.map(str::to_owned)
}

fn bare_template(line: &str) -> bool {
    line.trim()
        .strip_prefix("template")
        .and_then(|r| r.trim_start().strip_prefix('<'))
        .and_then(|r| r.get(list_after(r, b'<', b'>')?.len() + 1..))
        .is_some_and(|rest| rest.trim().is_empty())
}

fn list_after(s: &str, open: u8, close: u8) -> Option<&str> {
    let b = s.as_bytes();
    let mut depth = 1usize;
    let angled = open == b'<';
    for (i, &c) in b.iter().enumerate() {
        let arrow = i > 0 && matches!(b[i - 1], b'-' | b'=');
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b'<' if angled => depth += 1,
            b')' | b']' | b'}' => depth = depth.checked_sub(1)?,
            b'>' if angled && !arrow => depth = depth.checked_sub(1)?,
            _ => continue,
        }
        if depth == 0 {
            return (c == close).then(|| s.get(..i)).flatten();
        }
    }
    None
}

fn top_parts(list: &str) -> Vec<&str> {
    let b = list.as_bytes();
    let (mut depth, mut start, mut out) = (0i32, 0, Vec::new());
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b'>' if i > 0 && matches!(b[i - 1], b'-' | b'=') => {}
            b')' | b']' | b'}' | b'>' => depth -= 1,
            b',' if depth == 0 => {
                out.extend(list.get(start..i));
                start = i + 1;
            }
            _ => {}
        }
    }
    out.extend(list.get(start..));
    out
}

fn param_name(part: &str, last: bool) -> Option<&str> {
    static WORD: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[A-Za-z_]\w*").expect("the regex is a literal"));
    let head = part.split('=').next()?.trim();
    let head = head.strip_prefix("const ").unwrap_or(head);
    if head.starts_with('\'') {
        return None;
    }
    if last {
        let head = head.trim_end_matches("...").trim_end();
        return WORD
            .find_iter(head)
            .last()
            .map(|m| m.as_str())
            .filter(|_| head.split_whitespace().count() > 1);
    }
    let head = head.split(':').next()?.trim();
    WORD.find(head)
        .filter(|m| m.start() == 0)
        .map(|m| m.as_str())
}

pub fn declared_type_on(kind: Kind, line: &str, name: &str) -> Option<String> {
    let n = regex::escape(name);
    let pattern = match kind {
        Kind::Rust | Kind::Swift => format!(
            r"(?:^|[^\w.])(?:_\s+)?{n}\s*:\s*(?:&\s*)?(?:mut\s+|inout\s+)?([A-Za-z_]\w*)[?!]?\s*(?:[,)=;{{]|$)"
        ),
        _ => return None,
    };
    let code = coded(kind, line);
    Regex::new(&pattern)
        .ok()?
        .captures(code.trim())
        .map(|c| c[1].to_owned())
}

pub fn own_field(kind: Kind, text: &str, line1: usize, name: &str) -> Option<super::Binding> {
    static HEAD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^[^(]*\b(?:struct|class|union|actor)\s+[A-Za-z_]")
            .expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(kind, text);
    let mut k = line1.checked_sub(1).filter(|&k| k < lines.len())?;
    let parent = loop {
        let p = (0..k).rev().find(|&i| {
            let t = uncommented(kind, lines[i]);
            let t = t.trim();
            !t.is_empty()
                && literal.get(i) != Some(&true)
                && !t.starts_with(['{', '}', '#'])
                && indent(lines[i]) < indent(lines[k])
        })?;
        if HEAD.is_match(&coded(kind, lines[p])) {
            break p;
        }
        k = p;
    };
    let (outer, base) = (indent(lines[parent]), indent(lines[k]));
    lines
        .iter()
        .enumerate()
        .skip(parent + 1)
        .take_while(|(_, l)| l.trim().is_empty() || indent(l) > outer)
        .filter(|&(i, l)| indent(l) == base && literal.get(i) != Some(&true))
        .find_map(|(i, l)| {
            let t = coded(kind, l);
            let field =
                t.split_whitespace().any(|w| w == "var" || w == "let") && !t.contains("func ");
            Some(super::Binding {
                line1: i + 1,
                value: super::Value::Type(declared_type_on(kind, l, name).filter(|_| field)?),
            })
        })
}

pub fn rust_impl_field(text: &str, line1: usize, name: &str) -> Option<String> {
    fn angled(s: &str) -> Option<Vec<&str>> {
        Some(top_parts(list_after(
            s.get(s.find('<')? + 1..)?,
            b'<',
            b'>',
        )?))
    }
    let lines: Vec<&str> = text.lines().collect();
    let owner = super::rust_self_type(&lines, line1.checked_sub(1)?)?;
    let head = coded(Kind::Rust, lines[owner.impl_line0]);
    let generics = head.trim().strip_prefix("impl")?.trim_start();
    let params = angled(generics).filter(|_| generics.starts_with('<'))?;
    let args = angled(head.get(head.rfind(&format!("{}<", owner.written))?..)?)?;
    let decl = Regex::new(&super::rust_type_decl_pattern(&owner.written)).ok()?;
    let mut decls = (0..lines.len()).filter(|&i| decl.is_match(lines[i]));
    let s = decls.next().filter(|_| decls.next().is_none())?;
    let written = super::rust_struct_field(&lines, s, name)?.written_type;
    let i = angled(&coded(Kind::Rust, lines[s]))?
        .iter()
        .position(|p| param_name(p, false) == Some(&written))?;
    let arg = args.get(i)?.trim();
    params
        .iter()
        .any(|p| param_name(p, false) == Some(arg))
        .then(|| arg.to_owned())
}
