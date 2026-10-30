use regex::Regex;
use std::ops::Range;
use std::sync::LazyLock;

use super::{Binding, Value};

pub(super) static ACCESS_LABELS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:public|private|protected)\s*:\s*)+").expect("the regex is a literal")
});

pub fn c_includes(text: &str) -> Vec<CInclude> {
    static INCLUDE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?m)^[ \t]*#[ \t]*(?:include|include_next|import)[ \t]*([<"])([^>"\n]+)[>"]"#)
            .expect("the regex is a literal")
    });
    (INCLUDE.captures_iter(text))
        .map(|c| CInclude {
            as_written: c[2].trim().to_owned(),
            quoted_not_angled: &c[1] == "\"",
        })
        .collect()
}
pub struct CInclude {
    pub as_written: String,
    pub quoted_not_angled: bool,
}

pub fn c_code(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = b.to_vec();
    let blank = |out: &mut Vec<u8>, from: usize, to: usize| {
        for c in &mut out[from..to] {
            if *c != b'\n' {
                *c = b' ';
            }
        }
    };
    let (mut i, mut line_start, mut directive) = (0, true, false);
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            directive &= text
                .get(..i)
                .is_some_and(|h| h.trim_end_matches('\r').ends_with('\\'));
            line_start = true;
            i += 1;
            continue;
        }
        if directive {
            let end = if b[i..].starts_with(b"/*") {
                text.get(i + 2..)
                    .and_then(|r| r.find("*/"))
                    .map_or(b.len(), |n| i + n + 4)
            } else {
                i + 1
            };
            blank(&mut out, i, end);
            i = end;
            continue;
        }
        if c == b'#' && line_start {
            directive = true;
            continue;
        }
        if !c.is_ascii_whitespace() {
            line_start = false;
        }
        let digit_separator = i > 0 && b[i - 1].is_ascii_digit();
        let end = match c {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                let mut j = i;
                while j < b.len() && !(b[j] == b'\n' && b[j - 1] != b'\\') {
                    j += 1;
                }
                j
            }
            b'/' if b.get(i + 1) == Some(&b'*') => text
                .get(i + 2..)
                .and_then(|r| r.find("*/"))
                .map_or(b.len(), |n| i + n + 4),
            b'"' => {
                let raw = text
                    .get(..i)
                    .and_then(|h| h.strip_suffix('R'))
                    .is_some_and(|t| {
                        let t = ["u8", "u", "U", "L"]
                            .iter()
                            .find_map(|p| t.strip_suffix(p))
                            .unwrap_or(t);
                        !t.ends_with(|c: char| c.is_alphanumeric() || c == '_')
                    });
                match raw.then(|| text.get(i + 1..)?.split_once('(')).flatten() {
                    Some((delim, body)) => {
                        let close = format!("){delim}\"");
                        body.find(&close)
                            .map_or(b.len(), |n| i + 2 + delim.len() + n + close.len())
                    }
                    None => literal_end(b, i, b'"'),
                }
            }
            b'\'' if !digit_separator => literal_end(b, i, b'\''),
            _ => {
                i += 1;
                continue;
            }
        };
        blank(&mut out, i, end.min(b.len()));
        i = end.max(i + 1);
    }
    String::from_utf8(out).unwrap_or_default()
}
fn literal_end(b: &[u8], open: usize, quote: u8) -> usize {
    let mut j = open + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 1,
            b'\n' => return j,
            c if c == quote => return j + 1,
            _ => {}
        }
        j += 1;
    }
    b.len()
}
pub(super) fn c_innermost_open_brace(code: &[u8], at: usize) -> Option<usize> {
    let mut depth = 0usize;
    for i in (0..at.min(code.len())).rev() {
        match code[i] {
            b'}' => depth += 1,
            b'{' if depth == 0 => return Some(i),
            b'{' => depth -= 1,
            _ => {}
        }
    }
    None
}
fn c_back_to_stop(code: &str, at: usize) -> Option<(&str, Option<u8>)> {
    let b = code.as_bytes();
    match b
        .get(..at)?
        .iter()
        .rposition(|c| matches!(c, b';' | b'{' | b'}'))
    {
        Some(s) => Some((code.get(s + 1..at)?, Some(b[s]))),
        None => Some((code.get(..at)?, None)),
    }
}
fn untemplated(s: &str) -> String {
    static ANGLE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<[^<>]*>").expect("the regex is a literal"));
    let mut s = s.to_owned();
    while ANGLE.is_match(&s) {
        s = ANGLE.replace_all(&s, "").into_owned();
    }
    s
}
pub(super) fn c_struct_head(head: &str) -> Option<String> {
    static ATTR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"__attribute__\s*\(\((?:[^()]|\([^()]*\))*\)\)|\b(?:alignas|__declspec)\s*\([^()]*\)|\[\[[^\]]*\]\]").expect("the regex is a literal")
    });
    static HEAD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^(?:template\s*(?:<.*>)?\s*)?(?:(?:typedef|static|extern|const|volatile)\s+)*(?:struct|union|class)\b(?:\s+(?:[A-Z][A-Z0-9_]*\s+)*(?:\w+::)*(\w+))?(?:\s+final)?\s*(?::(?:[^:]|$)[^;{}()=]*)?$",
        )
        .expect("the regex is a literal")
    });
    let head = ACCESS_LABELS.replace(head, "");
    let head = ATTR.replace_all(head.trim(), "");
    let head = untemplated(head.trim());
    HEAD.captures(head.trim_start())
        .map(|c| c.get(1).map_or(String::new(), |m| m.as_str().to_owned()))
}
fn c_struct_around(code: &str, at: usize) -> Option<String> {
    let mut open = c_innermost_open_brace(code.as_bytes(), at)?;
    let mut name = c_struct_head(c_back_to_stop(code, open)?.0)?;
    while name.is_empty() {
        let Some(outer) = c_innermost_open_brace(code.as_bytes(), open) else {
            break;
        };
        match c_back_to_stop(code, outer).and_then(|(head, _)| c_struct_head(head)) {
            Some(n) => (open, name) = (outer, n),
            None => break,
        }
    }
    Some(name)
}
fn c_field_owner(code: &str, at: usize, word: &str) -> Option<String> {
    static ACCESS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:public|private|protected|signals|slots)\s*:\s*)+")
            .expect("the regex is a literal")
    });
    static REFUSED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:typedef|using|friend|static_assert|return|case|goto|else|delete|new|throw|sizeof|template|operator)\b|^(?:struct|union|class|enum)(?:\s+(?:struct|class))?\s*$").expect("the regex is a literal")
    });
    static AFTER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:\[[^\]]*\]\s*)*(?:[;,{\[]|=[^=]|:[^:]|[A-Z_][A-Z0-9_]*\s*\()")
            .expect("the regex is a literal")
    });
    static FN_POINTER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\(\s*\*+$").expect("the regex is a literal"));
    static CALLED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*\)\s*\(").expect("the regex is a literal"));
    let (before, stop) = c_back_to_stop(code, at)?;
    let after = code.get(at + word.len()..)?;
    let pre = ACCESS.replace(before, "");
    let pre = pre.trim();
    let shaped = if pre.is_empty() {
        let closes_nested_body = stop == Some(b'}');
        closes_nested_body && AFTER.is_match(after) && !after.trim_start().starts_with(['{', '='])
    } else if let Some(m) = FN_POINTER.find(pre) {
        let ty = pre.get(..m.start())?.trim();
        !ty.is_empty()
            && !untemplated(ty).contains(['(', ')', '='])
            && !REFUSED.is_match(ty)
            && CALLED.is_match(after)
    } else {
        let flat = untemplated(pre);
        !flat.contains(['(', ')', '='])
            && !pre.ends_with([':', '.'])
            && !pre.ends_with("->")
            && !REFUSED.is_match(pre)
            && AFTER.is_match(after)
    };
    if !shaped {
        return None;
    }
    c_struct_around(code, at)
}
pub fn c_field_pattern(word: &str) -> String {
    let w = regex::escape(word);
    format!(
        r"(?:[\w\]>]\s+|[*&,}}>]\s*){w}\s*(?:\[[^\]]*\]\s*)*(?:[;,={{\[]|:(?:[^:]|$)|[A-Z_][A-Z0-9_]*\s*\()|\(\s*\*+\s*{w}\s*\)\s*\("
    )
}
pub struct CFieldRow {
    pub line1: usize,
    pub owner_type: String,
}
pub fn c_field_rows(text: &str, hit_lines1: &[usize], word: &str) -> Vec<CFieldRow> {
    let code = c_code(text);
    let starts = line_starts(&code);
    let mut rows = Vec::new();
    for &line in hit_lines1 {
        let Some(&from) = line.checked_sub(1).and_then(|k| starts.get(k)) else {
            continue;
        };
        let to = starts.get(line).map_or(code.len(), |&s| s - 1);
        let Some(row) = code.get(from..to) else {
            continue;
        };
        let found = row.match_indices(word).find_map(|(i, _)| {
            let at = from + i;
            let whole = !code
                .get(..at)?
                .ends_with(|c: char| c.is_alphanumeric() || c == '_')
                && !code
                    .get(at + word.len()..)?
                    .starts_with(|c: char| c.is_alphanumeric() || c == '_');
            whole.then(|| c_field_owner(&code, at, word)).flatten()
        });
        if let Some(owner) = found {
            rows.push(CFieldRow {
                line1: line,
                owner_type: owner,
            });
        }
    }
    rows
}
pub fn c_initialized_member(text: &str, line1: usize, byte_col: usize) -> Option<String> {
    static CTOR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^\s*(?:(?:public|private|protected)\s*:\s*)*(?:[\w~]+\s+)*(?:\w+::)*?(?:(?P<class>\w+)::)?(?P<ctor>\w+)\s*\(\)\s*(?:(?:const|noexcept|override)\b\s*(?:\(\))?\s*)*:\s*(?:(?:\w+::)*\w+\s*(?:\(\)|\{\})\s*,\s*)*$",
        )
        .expect("the regex is a literal")
    });
    let code = c_code(text);
    let at = code
        .split('\n')
        .take(line1.checked_sub(1)?)
        .map(|l| l.len() + 1)
        .sum::<usize>()
        + byte_col;
    let s = ctor_head_with_brackets_emptied(&code, at)?;
    if !code
        .get(at..)?
        .trim_start_matches(|c: char| c.is_alphanumeric() || c == '_')
        .trim_start()
        .starts_with(['(', '{'])
    {
        return None;
    }
    let s = untemplated(&s);
    let caps = CTOR.captures(&s)?;
    let ctor = caps.name("ctor")?.as_str();
    match caps.name("class") {
        Some(class) if class.as_str() != ctor => None,
        _ => Some(ctor.to_owned()),
    }
}
#[expect(
    clippy::match_same_arms,
    reason = "each arm is one kind of byte the scan stops at"
)]
fn ctor_head_with_brackets_emptied(code: &str, at: usize) -> Option<String> {
    let b = code.as_bytes();
    let item_of_list = |close: usize| {
        code.get(close + 1..)
            .is_some_and(|r| r.trim_start().starts_with(','))
    };
    let mut s = String::new();
    let (mut depth, mut i) = (0usize, at);
    while i > 0 {
        i -= 1;
        let c = b[i];
        match c {
            b'}' if depth == 0 && !item_of_list(i) => break,
            b')' | b'}' => {
                if depth == 0 {
                    s.insert(0, c as char);
                }
                depth += 1;
            }
            b'(' | b'{' if depth == 0 => break,
            b'(' | b'{' => {
                depth -= 1;
                if depth == 0 {
                    s.insert(0, c as char);
                }
            }
            b';' if depth == 0 => break,
            _ if depth == 0 => s.insert(0, c as char),
            _ => {}
        }
        if at - i > 4096 {
            return None;
        }
    }
    Some(s)
}

pub(super) fn line_starts(code: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(code.match_indices('\n').map(|(i, _)| i + 1))
        .collect()
}
#[expect(
    clippy::match_same_arms,
    reason = "each arm is one kind of byte the scan stops at"
)]
pub(super) fn c_head_start(code: &str, open: usize) -> usize {
    let b = code.as_bytes();
    let item_of_init_list = |close: usize| {
        code.get(close + 1..)
            .is_some_and(|r| r.trim_start().starts_with([',', '{']))
    };
    let mut depth = 0usize;
    for i in (0..open).rev() {
        match b[i] {
            b'}' if depth == 0 && !item_of_init_list(i) => return i + 1,
            b')' | b']' | b'}' => depth += 1,
            b'(' | b'[' | b'{' if depth == 0 => return i + 1,
            b'(' | b'[' | b'{' => depth -= 1,
            b';' if depth == 0 => return i + 1,
            _ => {}
        }
    }
    0
}
pub(super) fn bracket_contents_blanked(s: &str) -> String {
    let mut depth = 0usize;
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let inside = depth > 0;
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if inside && depth > 0 && c != '\n' {
            out.push_str(&" ".repeat(c.len_utf8()));
        } else {
            out.push(c);
        }
    }
    out
}
pub(super) fn top_level_paren_groups(s: &str) -> Vec<Range<usize>> {
    let (mut depth, mut from, mut out) = (0usize, 0, Vec::new());
    for (i, c) in s.bytes().enumerate() {
        match c {
            b'(' | b'[' | b'{' => {
                if depth == 0 && c == b'(' {
                    from = i;
                }
                depth += 1;
            }
            b')' | b']' | b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 && c == b')' {
                    out.push(from..i + 1);
                }
            }
            _ => {}
        }
    }
    out
}
pub(super) enum Head {
    TypeOrNamespace,
    Function { params: Range<usize>, name: String },
    Lambda { params: Range<usize> },
    Catch { params: Range<usize> },
    Control { parens: Range<usize> },
    Block,
}
pub(super) fn c_head(head: &str) -> Head {
    static NAMESPACE_EXTERN_OR_ENUM: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:inline\s+)?namespace\b|^\s*extern\s*$|^\s*(?:typedef\s+)?enum\b[^(]*$")
            .expect("the regex is a literal")
    });
    static QUALS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)^\s*(?:(?:const|volatile|noexcept|override|final|mutable|throw|&&?|[A-Z_][A-Z0-9_]*)\s*(?:\(\s*\))?\s*)*(?:->.*|:[^:].*)?$").expect("the regex is a literal")
    });
    static NAME: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"((?:\w+\s*::\s*)*(?:operator\s*[^\w\s]+|~?\w+))\s*$")
            .expect("the regex is a literal")
    });
    if c_struct_head(head).is_some() || NAMESPACE_EXTERN_OR_ENUM.is_match(head) {
        return Head::TypeOrNamespace;
    }
    let f = bracket_contents_blanked(head);
    let Some(group) = top_level_paren_groups(head)
        .into_iter()
        .find(|g| f.get(g.end..).is_some_and(|r| QUALS.is_match(r)))
    else {
        return Head::Block;
    };
    let Some(before) = head.get(..group.start).map(str::trim_end) else {
        return Head::Block;
    };
    if before.ends_with(']') {
        return Head::Lambda { params: group };
    }
    let Some(name) = NAME.captures(before).and_then(|c| c.get(1)) else {
        return Head::Block;
    };
    let name: String = name.as_str().split_whitespace().collect();
    match name.as_str() {
        "if" | "for" | "while" | "switch" => Head::Control { parens: group },
        "catch" => Head::Catch { params: group },
        _ => Head::Function {
            params: group,
            name,
        },
    }
}
const STATEMENT_WORDS_NOT_TYPES: &[&str] = &[
    "return",
    "case",
    "goto",
    "else",
    "sizeof",
    "delete",
    "new",
    "throw",
    "typedef",
    "using",
    "if",
    "while",
    "for",
    "switch",
    "do",
    "try",
    "catch",
    "co_return",
    "co_yield",
    "co_await",
    "default",
    "break",
    "continue",
    "namespace",
    "template",
    "friend",
    "public",
    "private",
    "protected",
    "operator",
    "static_assert",
    "asm",
];
#[derive(Clone, Copy, PartialEq)]
enum DeclaredIn {
    Statement,
    ControlHead,
}
fn declared_at(stmt: &str, name: &str, place: DeclaredIn) -> Option<usize> {
    static BOUND: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:const\s+)?auto\s*&{0,2}\s*\[([^\]]*)\]\s*[=:{(]")
            .expect("the regex is a literal")
    });
    static TOKEN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[A-Za-z_]\w*").expect("the regex is a literal"));
    if let Some(m) = BOUND.captures(stmt).and_then(|c| c.get(1)) {
        return TOKEN
            .find_iter(m.as_str())
            .find(|t| t.as_str() == name)
            .map(|t| m.start() + t.start());
    }
    let f = bracket_contents_blanked(stmt);
    let is_name = |c: char| c.is_alphanumeric() || c == '_';
    f.match_indices(name).find_map(|(j, _)| {
        let (head, rest) = (f.get(..j)?, f.get(j + name.len()..)?);
        let whole = !head.ends_with(is_name) && !rest.starts_with(is_name);
        let (pre, after) = (head.trim(), rest.trim_start());
        let first = TOKEN.find(pre)?;
        let followed = match after.as_bytes() {
            [b'=', b'=', ..] | [b':', b':', ..] => false,
            [b'=' | b':' | b'{', ..] => true,
            [b';' | b',' | b'[' | b'(', ..] => place == DeclaredIn::Statement,
            _ => false,
        };
        let pre_flat = untemplated(pre);
        let mut segs: Vec<&str> = pre_flat.split(',').collect();
        let last = segs.len() - 1;
        for seg in &mut segs[..last] {
            *seg = seg.split_once('=').map_or(*seg, |(d, _)| d);
        }
        let tokens = |s: &str| TOKEN.find_iter(s).count();
        let shaped = segs
            .concat()
            .chars()
            .all(|c| is_name(c) || c.is_whitespace() || "*&:[]".contains(c))
            && !pre_flat.ends_with(':')
            && match segs.as_slice() {
                [one] => tokens(one) >= 1,
                [head, middle @ .., last] => {
                    tokens(head) >= 2 && middle.iter().all(|m| tokens(m) >= 1) && tokens(last) == 0
                }
                [] => false,
            };
        let and_expression = pre.ends_with('&') && !after.starts_with(['=', ':']);
        let declares_type = matches!(pre, "struct" | "union" | "class" | "enum");
        (whole
            && !declares_type
            && first.start() == 0
            && !STATEMENT_WORDS_NOT_TYPES.contains(&first.as_str())
            && followed
            && shaped
            && !and_expression)
            .then_some(j)
    })
}
pub(super) fn parameter_at(params: &str, name: &str) -> Option<usize> {
    static POINTER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^[^(]*\(\s*[*&^]+\s*(\w+)\s*(?:\[[^\]]*\]\s*)?\)")
            .expect("the regex is a literal")
    });
    static LAST: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"[\w\s*&:>]*?[\s*&>]([A-Za-z_]\w*)\s*(?:\[[^\]]*\]\s*)*(?:=.*)?$")
            .expect("the regex is a literal")
    });
    const BUILTIN: &[&str] = &[
        "int", "char", "short", "long", "float", "double", "void", "signed", "unsigned", "bool",
        "const", "volatile",
    ];
    let f = bracket_contents_blanked(params);
    let mut from = 0;
    for entry in f.split(',') {
        let at = from;
        from += entry.len() + 1;
        let raw = params.get(at..at + entry.len())?;
        let lead = entry.len() - entry.trim_start().len();
        let found = match POINTER.captures(raw) {
            Some(c) => c.get(1).map(|m| (m.start(), m.as_str())),
            None => LAST
                .captures(entry.trim())
                .and_then(|c| c.get(1))
                .filter(|m| {
                    !entry.trim().get(..m.start()).is_some_and(|b| {
                        b.split_whitespace()
                            .all(|w| ["const", "volatile", "struct", "enum", "union"].contains(&w))
                    })
                })
                .map(|m| (lead + m.start(), m.as_str())),
        };
        if let Some((j, _)) = found.filter(|(_, n)| *n == name && !BUILTIN.contains(n)) {
            return Some(at + j);
        }
    }
    None
}
#[expect(
    clippy::match_same_arms,
    reason = "a guarded arm sits between the two; merging them would move one past it"
)]
fn block_declarations(code: &str, from: usize, pos: usize, name: &str) -> Vec<usize> {
    let b = code.as_bytes();
    let (mut depth, mut start, mut out) = (0usize, from, Vec::new());
    let take = |from: usize, to: usize, out: &mut Vec<usize>| {
        if let Some(j) = code
            .get(from..to)
            .and_then(|s| declared_at(s, name, DeclaredIn::Statement))
        {
            out.push(from + j);
        }
    };
    for (i, &c) in b.iter().enumerate().take(pos).skip(from) {
        match c {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.saturating_sub(1),
            b'{' if depth == 0 => {
                take(start, i + 1, &mut out);
                depth += 1;
            }
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    start = i + 1;
                }
            }
            b';' if depth == 0 => {
                take(start, i + 1, &mut out);
                start = i + 1;
            }
            _ => {}
        }
    }
    if let Some(at) = control_init(code, start, pos.min(code.len()), name) {
        return vec![at];
    }
    if depth == 0 && start < pos {
        take(start, pos.min(code.len()), &mut out);
    }
    out
}
fn control_init(code: &str, from: usize, pos: usize, name: &str) -> Option<usize> {
    static CONTROL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:else\s+)?(?:for|if|while|switch)\s*\(").expect("the regex is a literal")
    });
    let mut at = from;
    let mut found = None;
    while let Some(m) = code.get(at..pos).and_then(|s| CONTROL.find(s)) {
        let open = at + m.end() - 1;
        let close = close_or_end(code, open, pos);
        let parts = code.get(open + 1..close).map(split_top_level_semicolons);
        for (j, part) in parts.unwrap_or_default() {
            if let Some(k) = declared_at(part, name, DeclaredIn::ControlHead) {
                found = Some(open + 1 + j + k);
            }
        }
        if close >= pos {
            break;
        }
        at = close + 1;
    }
    found
}
pub(super) fn close_or_end(code: &str, open: usize, end: usize) -> usize {
    let mut depth = 0usize;
    for (i, c) in code.bytes().enumerate().take(end).skip(open) {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    end
}
pub(super) struct Scope {
    pub(super) open: usize,
    pub(super) head_start: usize,
    pub(super) head: Head,
}
pub(super) fn scopes_inside_out_to_type_or_namespace(code: &str, pos: usize) -> Vec<Scope> {
    let mut out = Vec::new();
    let mut at = pos;
    while let Some(open) = c_innermost_open_brace(code.as_bytes(), at) {
        let head_start = c_head_start(code, open);
        let Some(head) = code.get(head_start..open).map(c_head) else {
            break;
        };
        if matches!(head, Head::TypeOrNamespace) {
            break;
        }
        out.push(Scope {
            open,
            head_start,
            head,
        });
        at = open;
    }
    out
}
pub(super) fn scope_pos(code: &str, starts: &[usize], line1: usize, name: &str) -> Option<usize> {
    let from = *starts.get(line1.checked_sub(1)?)?;
    let to = starts.get(line1).map_or(code.len(), |&s| s - 1);
    let is_name = |c: char| c.is_alphanumeric() || c == '_';
    Some(
        code.get(from..to)?
            .match_indices(name)
            .filter(|(i, _)| {
                code.get(..from + i).is_some_and(|h| !h.ends_with(is_name))
                    && (code.get(from + i + name.len()..)).is_some_and(|r| !r.starts_with(is_name))
            })
            .last()
            .map_or(to, |(i, _)| from + i),
    )
}
pub fn c_bindings(text: &str, line1: usize, col: Option<usize>, name: &str) -> Vec<Binding> {
    c_bindings_at(text, line1, col, name)
        .into_iter()
        .map(|p| Binding {
            line1: p.line1,
            value: Value::Unknown,
        })
        .collect()
}
pub fn c_bindings_at(text: &str, line1: usize, col: Option<usize>, name: &str) -> Vec<CPlace> {
    let code = c_code(text);
    let starts = line_starts(&code);
    let Some(pos) = scope_pos(&code, &starts, line1, name) else {
        return Vec::new();
    };
    let place = |at: usize| {
        let line1 = starts.partition_point(|&s| s <= at);
        CPlace {
            line1,
            byte_col: at - starts[line1 - 1],
        }
    };
    let member = col.is_some_and(|c| {
        (code.get(starts[line1 - 1] + c..))
            .and_then(|r| r.strip_prefix(name))
            .is_some_and(|r| r.trim_start().starts_with(['(', '{']))
    });
    if let Some(at) = initializer_parameter(&code, pos, name).filter(|_| !member) {
        return vec![place(at)];
    }
    for Scope {
        open,
        head_start: start,
        head,
    } in scopes_inside_out_to_type_or_namespace(&code, pos)
    {
        let found = block_declarations(&code, open + 1, pos, name);
        if !found.is_empty() {
            return found.into_iter().map(place).collect();
        }
        if let Some(j) = code
            .get(start..open)
            .and_then(|h| super::objc_parameter(h, name))
        {
            return vec![place(start + j)];
        }
        let found = match head {
            Head::Function { params, .. } | Head::Lambda { params } | Head::Catch { params } => {
                let inner = start + params.start + 1..start + params.end - 1;
                code.get(inner.clone())
                    .and_then(|p| parameter_at(p, name))
                    .map(|j| inner.start + j)
                    .or_else(|| super::cpp::init_capture(&code, start..inner.start - 1, name))
            }
            Head::Control { parens } => {
                let inner = start + parens.start + 1..start + parens.end - 1;
                code.get(inner.clone())
                    .and_then(|parens| {
                        (split_top_level_semicolons(parens).into_iter())
                            .find_map(|(at, stmt)| {
                                declared_at(stmt, name, DeclaredIn::ControlHead).map(|j| at + j)
                            })
                            .or_else(|| super::objc_for_in(parens, name))
                    })
                    .map(|j| inner.start + j)
            }
            Head::Block | Head::TypeOrNamespace => None,
        };
        if let Some(at) = found {
            return vec![place(at)];
        }
    }
    Vec::new()
}
fn initializer_parameter(code: &str, pos: usize, name: &str) -> Option<usize> {
    let open = pos + code.get(pos..)?.find(['{', ';', '}'])?;
    if code.as_bytes()[open] != b'{' {
        return None;
    }
    let start = c_head_start(code, open);
    let Head::Function { params, .. } = c_head(code.get(start..open)?) else {
        return None;
    };
    if start + params.end > pos {
        return None;
    }
    let inner = start + params.start + 1..start + params.end - 1;
    parameter_at(code.get(inner.clone())?, name).map(|j| inner.start + j)
}
fn split_top_level_semicolons(s: &str) -> Vec<(usize, &str)> {
    let f = bracket_contents_blanked(s);
    let mut from = 0;
    f.split(';')
        .filter_map(|part| {
            let at = from;
            from += part.len() + 1;
            Some((at, s.get(at..at + part.len())?))
        })
        .collect()
}
pub fn c_inside_function(text: &str, line1: usize, name: &str) -> bool {
    let code = c_code(text);
    let starts = line_starts(&code);
    scope_pos(&code, &starts, line1, name).is_some_and(|pos| {
        scopes_inside_out_to_type_or_namespace(&code, pos)
            .iter()
            .any(|s| matches!(s.head, Head::Function { .. } | Head::Lambda { .. }))
    })
}
pub fn c_function_local_reach(
    text: &str,
    line1: usize,
    name: &str,
    from1: usize,
) -> Option<Option<usize>> {
    let code = c_code(text);
    let starts = line_starts(&code);
    let open = scope_pos(&code, &starts, line1, name).and_then(|pos| {
        scopes_inside_out_to_type_or_namespace(&code, pos)
            .into_iter()
            .find(|s| matches!(s.head, Head::Function { .. } | Head::Lambda { .. }))
            .map(|s| s.open)
    })?;
    let at = scope_pos(&code, &starts, from1, name)?;
    let inside = open < at && at <= close_or_end(&code, open, code.len());
    match line1.cmp(&from1) {
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Less if inside => Some(Some(open)),
        _ => Some(None),
    }
}
pub fn c_method_class(text: &str, line1: usize, name: &str) -> Option<String> {
    let code = c_code(text);
    let starts = line_starts(&code);
    let pos = scope_pos(&code, &starts, line1, name)?;
    let (open, name) = scopes_inside_out_to_type_or_namespace(&code, pos)
        .into_iter()
        .find_map(|scope| match scope.head {
            Head::Function { name, .. } => Some((scope.open, name)),
            _ => None,
        })?;
    if let Some((owner, _)) = name.rsplit_once("::").and_then(|(o, m)| {
        let owner = o.rsplit("::").next()?;
        Some((owner.to_owned(), m))
    }) {
        return Some(owner);
    }
    let outer = c_innermost_open_brace(code.as_bytes(), c_head_start(&code, open))?;
    c_struct_head(c_back_to_stop(&code, outer)?.0).filter(|n| !n.is_empty())
}

const C_ENUMERATOR: &str = r"[A-Za-z_]\w*\s*(?:=[^,;{}]*)?";
pub fn c_enumerators(word: Option<&str>) -> String {
    let w = word.map_or_else(|| r"[A-Za-z_]\w*".to_owned(), regex::escape);
    format!(
        r"^\s*(?:{C_ENUMERATOR},\s*)*{w}\s*(?:=[^,;{{}}]*)?(?:,\s*{C_ENUMERATOR})*,?\s*(?:\}}[^{{]*)?(?:(?://|/\*).*)?$"
    )
}
pub fn c_enum_line(word: &str) -> String {
    format!(
        r"^\s*(?:typedef\s+)?enum\b[^{{;=()]*\{{\s*(?:{C_ENUMERATOR},\s*)*{}\s*(?:=[^,;{{}}]*)?\s*(?:[,}}]|$)",
        regex::escape(word)
    )
}
pub fn c_member_decl(word: Option<&str>) -> String {
    let w = word.map_or_else(|| r"~?[A-Za-z_]\w*".to_owned(), regex::escape);
    format!(
        r"^\s+[^;(){{}}=#/]*[\w>][\s*&]+{w}\s*\((?:[^(){{}};]|\([^(){{}};]*\))*(?:\)[^{{}};]*;)?\s*(?:(?://|/\*).*)?$"
    )
}
pub fn c_body_head<S: AsRef<str>>(lines: &[S], line1: usize) -> Option<String> {
    c_body_opener(lines, line1).map(|opener| opener.head)
}
struct BodyOpener {
    brace_line1: usize,
    head: String,
}
fn c_body_opener<S: AsRef<str>>(lines: &[S], line1: usize) -> Option<BodyOpener> {
    let mut depth = 0usize;
    let mut open = None;
    'up: for k in (0..line1.checked_sub(1)?.min(lines.len())).rev() {
        let l = c_code(lines[k].as_ref());
        for (i, c) in l.bytes().enumerate().rev() {
            match c {
                b'}' => depth += 1,
                b'{' if depth == 0 => {
                    open = Some((k, i));
                    break 'up;
                }
                b'{' => depth -= 1,
                _ => {}
            }
        }
    }
    let (k, i) = open?;
    let head = c_code(lines[k].as_ref()).get(..i)?.to_owned();
    Some(BodyOpener {
        brace_line1: k + 1,
        head: head_up_past_comments_to_a_blank_line(lines, k, head),
    })
}
fn head_up_past_comments_to_a_blank_line<S: AsRef<str>>(
    lines: &[S],
    brace_line0: usize,
    mut head: String,
) -> String {
    let k = brace_line0;
    for j in (k.saturating_sub(8)..=k).rev() {
        let t = lines[j].as_ref().trim_start();
        if j < k {
            if t.is_empty() {
                break;
            }
            if t.starts_with(['*', '#']) || t.starts_with("/*") || t.starts_with("//") {
                continue;
            }
            head = format!("{} {head}", c_code(lines[j].as_ref()));
        }
        if let Some((_, rest)) = head.rsplit_once([';', '{', '}']) {
            head = rest.to_owned();
            break;
        }
    }
    head.trim().to_owned()
}
fn c_scopes<S: AsRef<str>>(lines: &[S], line1: usize) -> Vec<String> {
    static NAMESPACE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:inline\s+)?namespace\s*((?:\w+\s*::\s*)*\w+)?\s*$")
            .expect("the regex is a literal")
    });
    let mut chain = Vec::new();
    let mut at = line1;
    while let Some(BodyOpener { brace_line1, head }) = c_body_opener(lines, at) {
        match (c_struct_head(&head), NAMESPACE.captures(&head)) {
            (Some(name), _) => chain.push(name),
            (None, Some(c)) if !chain.is_empty() => {
                let path = c.get(1).map_or("", |m| m.as_str());
                chain.extend(path.rsplit("::").map(|p| p.trim().to_owned()));
            }
            _ => break,
        }
        at = brace_line1;
    }
    chain.reverse();
    chain
}
pub fn c_enum_head(head: &str) -> bool {
    static ENUM: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:typedef\s+)?enum\b(?:\s+(?:class|struct)\b)?[^(){}=;]*$")
            .expect("the regex is a literal")
    });
    ENUM.is_match(head)
}
pub fn c_class_around<S: AsRef<str>>(lines: &[S], line1: usize) -> Option<String> {
    c_struct_head(&c_body_head(lines, line1)?)
}
pub fn c_declares_where<'a, S: AsRef<str> + 'a>(
    line1: usize,
    text: &str,
    lines: impl FnOnce() -> &'a [S],
) -> bool {
    static ENUMERATORS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(&c_enumerators(None)).expect("the regex is a literal"));
    static MEMBER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(&c_member_decl(None)).expect("the regex is a literal"));
    if ENUMERATORS.is_match(text) {
        return c_body_head(lines(), line1).is_some_and(|h| c_enum_head(&h));
    }
    let t = text.trim_start();
    if MEMBER.is_match(text) && !t.starts_with("typedef") && !t.starts_with("friend") {
        return c_class_around(lines(), line1).is_some();
    }
    true
}
pub fn c_member_class(text: &str, line1: usize, word: &str) -> Option<Vec<String>> {
    let decl = Regex::new(&c_member_decl(Some(word))).expect("an escaped name keeps it valid");
    let lines: Vec<&str> = text.lines().collect();
    if !decl.is_match(lines.get(line1.checked_sub(1)?)?) {
        return None;
    }
    Some(c_scopes(&lines, line1)).filter(|s| s.last().is_some_and(|c| !c.is_empty()))
}
pub fn c_defines_member(text: &str, scopes: &[String], word: &str) -> bool {
    static TEMPLATE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<[^;()]*>").expect("the regex is a literal"));
    let Ok(re) = Regex::new(&format!(
        r"((?:\w+\s*(?:<[^;()]*>)?\s*::\s*)+)~?{}\s*\(",
        regex::escape(word)
    )) else {
        return false;
    };
    re.captures_iter(text).any(|c| {
        let path = TEMPLATE.replace_all(&c[1], "");
        let written: Vec<&str> = path
            .split("::")
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect();
        let at = c.get(1).map_or(0, |m| m.start());
        let qualifier_starts_a_name = text.get(..at).is_some_and(|h| {
            !h.ends_with(|ch: char| ch == ':' || ch.is_alphanumeric() || ch == '_')
        });
        qualifier_starts_a_name
            && written.len() <= scopes.len()
            && scopes[scopes.len() - written.len()..]
                .iter()
                .zip(&written)
                .all(|(s, w)| s == w)
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CType {
    Name(String),
    Body(usize),
}
pub fn c_receiver(before: &str) -> Option<CReceiver> {
    receiver(before, false)
}
#[derive(Debug, PartialEq, Eq)]
pub struct CReceiver {
    pub head: String,
    pub head_called: bool,
    pub fields: Vec<String>,
    pub calls: Vec<bool>,
    pub braced: bool,
    pub start: usize,
}
pub fn cpp_receiver(before: &str) -> Option<CReceiver> {
    receiver(before, true)
}
fn receiver(before: &str, cpp: bool) -> Option<CReceiver> {
    static QUALIFIER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:\w*\s*::\s*)*$").expect("the regex is a literal"));
    let code = c_code(before);
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut rest = code.trim_end();
    let (mut fields, mut calls) = (Vec::new(), Vec::new());
    loop {
        rest = match rest.strip_suffix("->") {
            Some(r) => r,
            None => rest.strip_suffix('.').filter(|r| !r.ends_with('.'))?,
        }
        .trim_end();
        while rest.ends_with(']') {
            if cpp {
                return None;
            }
            rest = rest.get(..opening_of_last_bracket(rest)?)?.trim_end();
        }
        let called = rest.ends_with(')');
        let braced = cpp && rest.ends_with('}');
        if called || braced {
            rest = rest.get(..opening_of_last_bracket(rest)?)?.trim_end();
        }
        let start = rest.trim_end_matches(is_name).len();
        let (before, name) = (rest.get(..start)?, rest.get(start..)?);
        if name.is_empty()
            || name.starts_with(|c: char| c.is_ascii_digit())
            || (name == "this" && !cpp)
            || before.ends_with(|c: char| !c.is_ascii())
        {
            return None;
        }
        rest = before.trim_end();
        let member = rest.ends_with("->") || rest.ends_with('.') && !rest.ends_with("..");
        match (member, called, braced) {
            (true, _, false) if cpp || !called => {
                fields.insert(0, name.to_owned());
                calls.insert(0, called);
            }
            (false, _, _) => {
                let start = QUALIFIER.find(rest).map_or(rest.len(), |m| m.start());
                let head = if braced {
                    format!("{}{name}", rest.get(start..)?).replace(char::is_whitespace, "")
                } else {
                    name.to_owned()
                };
                return Some(CReceiver {
                    head,
                    head_called: called,
                    fields,
                    calls,
                    braced,
                    start,
                });
            }
            (true, _, _) => return None,
        }
    }
}
fn opening_of_last_bracket(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in s.bytes().enumerate().rev() {
        match c {
            b')' | b']' | b'}' => depth += 1,
            b'(' | b'[' | b'{' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}
const C_TYPE_KEYWORDS: &[&str] = &[
    "const",
    "volatile",
    "struct",
    "union",
    "enum",
    "unsigned",
    "signed",
    "static",
    "extern",
    "register",
    "inline",
    "restrict",
    "__restrict",
    "__restrict__",
    "typedef",
    "__inline",
];
#[expect(
    clippy::match_same_arms,
    reason = "each arm is one kind of byte the scan stops at"
)]
pub fn c_decl_type(code: &str, at: usize) -> Option<CType> {
    let b = code.as_bytes();
    let (mut depth, mut i) = (0usize, at);
    let (mut first_comma, mut last_comma, mut param) = (None, None, false);
    let stop = loop {
        if i == 0 {
            break None;
        }
        i -= 1;
        match b[i] {
            b')' | b']' => depth += 1,
            b'(' | b'[' if depth > 0 => depth -= 1,
            b'(' => {
                param = true;
                break Some(i);
            }
            b'[' => return None,
            b',' if depth == 0 => {
                first_comma = Some(i);
                last_comma = last_comma.or(Some(i));
            }
            b';' | b'{' | b'}' if depth == 0 => break Some(i),
            b';' | b'{' | b'}' => return None,
            _ => {}
        }
    };
    let from = stop.map_or(0, |s| s + 1);
    let (segment, named) = match (param, first_comma) {
        (true, _) => (code.get(last_comma.map_or(from, |c| c + 1)..at)?, false),
        (false, Some(c)) => (code.get(from..c)?, true),
        (false, None) => (code.get(from..at)?, false),
    };
    if let Some(s) = stop.filter(|&s| b[s] == b'}' && !param && first_comma.is_none())
        && segment
            .trim_matches(|c: char| c.is_whitespace() || c == '*')
            .is_empty()
    {
        let open = c_innermost_open_brace(b, s)?;
        let head = c_back_to_stop(code, open)?.0.trim();
        return (!head.starts_with("typedef") && c_struct_head(head).is_some())
            .then_some(CType::Body(open));
    }
    let flat = bracket_contents_blanked(segment);
    let typed = flat.split('=').next().unwrap_or_default();
    if typed.contains(['(', ')', '<', '{', '}']) || typed.contains("::") {
        return None;
    }
    let mut words: Vec<&str> = typed
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty() && !C_TYPE_KEYWORDS.contains(w))
        .collect();
    if named {
        words.pop();
    }
    words
        .last()
        .filter(|w| !w.starts_with(|c: char| c.is_ascii_digit()))
        .map(|w| CType::Name((*w).to_owned()))
}
pub fn c_type_def(code: &str, at: usize) -> Option<CType> {
    let tagged = ends_with_word(code.get(..at)?, &["struct", "union"]);
    let after = at
        + code
            .get(at..)?
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))?;
    if tagged {
        let open = after + code.get(after..)?.find(|c: char| !c.is_whitespace())?;
        return (code.as_bytes()[open] == b'{').then_some(CType::Body(open));
    }
    let (stmt, stop) = c_back_to_stop(code, at)?;
    let s = at - stmt.len();
    if stop == Some(b'}')
        && stmt
            .trim_matches(|c: char| c.is_whitespace() || c == '*')
            .is_empty()
    {
        let open = c_innermost_open_brace(code.as_bytes(), s - 1)?;
        let head = c_back_to_stop(code, open)?.0.trim();
        return (head.starts_with("typedef") && c_struct_head(head).is_some())
            .then_some(CType::Body(open));
    }
    if !stmt.trim_start().starts_with("typedef") {
        return None;
    }
    let written = code.get(at..after)?;
    c_decl_type(code, at).filter(|t| *t != CType::Name(written.to_owned()))
}
fn ends_with_word(s: &str, words: &[&str]) -> bool {
    let s = s.trim_end();
    words.iter().any(|w| {
        s.strip_suffix(w)
            .is_some_and(|r| !r.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_'))
    })
}
pub fn c_body_field(code: &str, open: usize, word: &str) -> Option<usize> {
    let close = close_or_end(code, open, code.len());
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let b = code.as_bytes();
    code.get(open + 1..close)?
        .match_indices(word)
        .find_map(|(i, _)| {
            let at = open + 1 + i;
            let whole = !code.get(..at)?.ends_with(is_name)
                && !code.get(at + word.len()..)?.starts_with(is_name);
            if !whole || c_field_owner(code, at, word).is_none() {
                return None;
            }
            let mut o = c_innermost_open_brace(b, at)?;
            while o != open {
                let end = close_or_end(code, o, close);
                let anonymous = c_struct_head(c_back_to_stop(code, o)?.0)
                    .is_some_and(|n| n.is_empty())
                    && code
                        .get(end + 1..)
                        .is_some_and(|r| r.trim_start().starts_with(';'));
                if !anonymous {
                    return None;
                }
                o = c_innermost_open_brace(b, o)?;
            }
            Some(at)
        })
}
pub fn c_value_type(text: &str, line1: usize, name: &str) -> Result<Option<CType>, ()> {
    let code = c_code(text);
    let starts = line_starts(&code);
    let mut at: Vec<usize> = c_bindings_at(text, line1, None, name)
        .into_iter()
        .map(|p| starts[p.line1 - 1] + p.byte_col)
        .collect();
    if at.is_empty() {
        at = block_declarations(&code, 0, code.len(), name);
        let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_';
        at.extend(code.match_indices(name).map(|(i, _)| i).filter(|&i| {
            let (Some(head), Some(rest)) = (code.get(..i), code.get(i + name.len()..)) else {
                return false;
            };
            !head.ends_with(is_name)
                && !rest.starts_with(is_name)
                && head
                    .trim_end_matches(|c: char| c.is_whitespace() || c == '*')
                    .ends_with('}')
                && c_innermost_open_brace(code.as_bytes(), i).is_none()
                && matches!(c_decl_type(&code, i), Some(CType::Body(_)))
        }));
    }
    let mut types = at
        .into_iter()
        .filter(|&a| {
            code.get(..a)
                .is_some_and(|h| !ends_with_word(h, &["struct", "union", "enum"]))
        })
        .map(|a| c_decl_type(&code, a));
    let Some(first) = types.next() else {
        return Ok(None);
    };
    match first {
        Some(t) if types.all(|o| o.as_ref() == Some(&t)) => Ok(Some(t)),
        _ => Err(()),
    }
}
pub fn c_words_on_line(
    code: &str,
    line1: usize,
    name: &str,
    followed_by: Option<char>,
) -> Vec<usize> {
    let starts = line_starts(code);
    let Some(&from) = line1.checked_sub(1).and_then(|k| starts.get(k)) else {
        return Vec::new();
    };
    let to = starts.get(line1).map_or(code.len(), |&s| s - 1);
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let Some(row) = code.get(from..to) else {
        return Vec::new();
    };
    row.match_indices(name)
        .map(|(i, _)| from + i)
        .filter(|&i| {
            let (Some(head), Some(rest)) = (code.get(..i), code.get(i + name.len()..)) else {
                return false;
            };
            !head.ends_with(is_name)
                && !rest.starts_with(is_name)
                && followed_by.is_none_or(|c| rest.trim_start().starts_with(c))
        })
        .collect()
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CPlace {
    pub line1: usize,
    pub byte_col: usize,
}
pub fn c_place(code: &str, at: usize) -> CPlace {
    let starts = line_starts(code);
    let line1 = starts.partition_point(|&s| s <= at);
    CPlace {
        line1,
        byte_col: at - starts[line1 - 1],
    }
}
pub fn c_body_name(code: &str, open: usize) -> String {
    c_back_to_stop(code, open)
        .and_then(|(head, _)| c_struct_head(head))
        .unwrap_or_default()
}
#[expect(
    clippy::match_same_arms,
    reason = "each arm is one kind of byte the scan stops at"
)]
pub(super) fn cpp_decl_start(code: &str, at: usize) -> Option<usize> {
    let b = code.as_bytes();
    let (mut depth, mut i) = (0usize, at);
    loop {
        if i == 0 {
            return Some(0);
        }
        i -= 1;
        match b[i] {
            b'>' if i > 0 && b[i - 1] == b'-' => return None,
            b')' | b']' | b'>' => depth += 1,
            b'(' | b',' if depth == 0 => return Some(i + 1),
            b'[' | b'<' if depth == 0 => return None,
            b'(' | b'[' | b'<' => depth -= 1,
            b';' | b'{' | b'}' if depth == 0 => return Some(i + 1),
            b';' | b'{' | b'}' => return None,
            _ => {}
        }
    }
}
pub(super) fn cpp_class_path(written: &str) -> Option<Vec<String>> {
    const ARROW: &[&str] = &[
        "optional",
        "expected",
        "auto_ptr",
        "weak_ptr",
        "reverse_iterator",
        "move_iterator",
        "counted_iterator",
        "common_iterator",
        "basic_const_iterator",
        "istream_iterator",
        "istreambuf_iterator",
        "indirect",
        "polymorphic",
    ];
    static DROPPED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\b(?:const|volatile|static|mutable|constexpr|inline|struct|class|typename|extern|thread_local|register|virtual|explicit)\b|\[\[[^\]]*\]\]").expect("the regex is a literal")
    });
    static STD_NAME: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\bstd(?:\s*::\s*\w+)+").expect("the regex is a literal"));
    static WORD: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[A-Za-z_]\w*").expect("the regex is a literal"));
    static SMART: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:::\s*)?(?:std\s*::\s*)?(?:unique_ptr|shared_ptr)\s*<(.*)>$")
            .expect("the regex is a literal")
    });
    const BUILTIN: &[&str] = &[
        "int", "char", "short", "long", "float", "double", "void", "signed", "unsigned", "bool",
        "size_t",
    ];
    let s = DROPPED.replace_all(written, "");
    let s = s.trim_matches(|c: char| c.is_whitespace() || c == '*' || c == '&');
    if let Some(c) = SMART.captures(s) {
        let inner = untemplated(&c[1]);
        return cpp_class_path(inner.split(',').next()?);
    }
    let flat = untemplated(s);
    let path: Vec<String> = flat
        .split("::")
        .map(|p| p.trim().to_owned())
        .skip_while(String::is_empty)
        .collect();
    let named = |p: &String| {
        !p.is_empty()
            && !p.starts_with(|c: char| c.is_ascii_digit())
            && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    let std_only = || {
        let args = STD_NAME.replace_all(s, "");
        WORD.find_iter(&args).all(|w| BUILTIN.contains(&w.as_str()))
            || (path.len() == 2 && !ARROW.contains(&path[1].as_str()))
    };
    (path.iter().all(named)
        && !path.is_empty()
        && !BUILTIN.contains(&path[path.len() - 1].as_str())
        && (path[0] != "std" || std_only()))
    .then_some(path)
}
pub fn cpp_template_param(code: &str, name: &str) -> bool {
    Regex::new(&format!(
        r"\btemplate\s*<[^;{{}}]*\b(?:typename|class)\s+{}\b",
        regex::escape(name)
    ))
    .is_ok_and(|re| re.is_match(code))
}
pub fn cpp_class_body(code: &str, line1: usize, name: &str) -> Option<usize> {
    c_words_on_line(code, line1, name, None)
        .into_iter()
        .find_map(|at| {
            let open = at + code.get(at..)?.find([';', '{', '(', '='])?;
            (code.as_bytes()[open] == b'{'
                && c_struct_head(c_back_to_stop(code, open)?.0).as_deref() == Some(name))
            .then_some(open)
        })
}
pub fn cpp_bases(code: &str, open: usize) -> Vec<Vec<String>> {
    static ACCESS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\b(?:public|private|protected|virtual)\b").expect("the regex is a literal")
    });
    let Some((head, _)) = c_back_to_stop(code, open) else {
        return Vec::new();
    };
    let head = untemplated(head);
    let list = head
        .match_indices(':')
        .find(|&(i, _)| {
            head.get(i + 1..).is_some_and(|r| !r.starts_with(':'))
                && head.get(..i).is_some_and(|h| !h.ends_with(':'))
        })
        .and_then(|(i, _)| head.get(i + 1..))
        .unwrap_or("");
    list.split(',')
        .filter(|b| !b.trim().is_empty())
        .filter_map(|b| cpp_class_path(&ACCESS.replace_all(b, "")))
        .collect()
}
pub fn cpp_body_members(code: &str, open: usize, word: &str) -> Vec<usize> {
    static REFUSED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:typedef|using|friend|static_assert|return|operator|template\s*<[^>]*>\s*friend)\b").expect("the regex is a literal")
    });
    let close = close_or_end(code, open, code.len());
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let b = code.as_bytes();
    let Some(body) = code.get(open + 1..close) else {
        return Vec::new();
    };
    body.match_indices(word)
        .map(|(i, _)| open + 1 + i)
        .filter(|&at| {
            let (Some(head), Some(rest)) = (code.get(..at), code.get(at + word.len()..)) else {
                return false;
            };
            if head.ends_with(is_name)
                || rest.starts_with(is_name)
                || c_innermost_open_brace(b, at) != Some(open)
            {
                return false;
            }
            if c_field_owner(code, at, word).is_some() {
                return true;
            }
            let Some((pre, _)) = c_back_to_stop(code, at) else {
                return false;
            };
            let pre = ACCESS_LABELS.replace(pre, "");
            let pre = untemplated(pre.trim());
            !pre.is_empty()
                && !pre.contains(['(', ')', '=', '.', '~'])
                && !pre.ends_with(':')
                && !pre.ends_with("->")
                && !REFUSED.is_match(&pre)
                && rest.trim_start().starts_with('(')
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_of(text: &str, needle: &str) -> usize {
        text.lines().position(|l| l.contains(needle)).unwrap() + 1
    }

    fn bound_on(text: &str, line1: usize, name: &str) -> Vec<usize> {
        c_bindings_at(text, line1, None, name)
            .into_iter()
            .map(|p| p.line1)
            .collect()
    }

    #[test]
    fn includes_count_under_every_branch_of_an_if() {
        let text = "#if X\n#include \"a.h\"\n#else\n#  include_next <b.h>\n#endif\n#import <c.h>\n";
        let got: Vec<(String, bool)> = (c_includes(text).into_iter())
            .map(|i| (i.as_written, i.quoted_not_angled))
            .collect();
        let want = [("a.h", true), ("b.h", false), ("c.h", false)];
        assert_eq!(got, want.map(|(f, q)| (f.to_owned(), q)));
    }

    #[test]
    fn c_code_blanks_comments_literals_and_directives_in_place() {
        for (text, code) in [
            ("a; // x {\nb;", "a; b;"),
            ("a; // x \\\n { \nb;", "a; b;"),
            ("a; /* { */ b;", "a; b;"),
            ("s = \"{\"; c = '{'; n = 1'000;", "s = ; c = ; n = 1'000;"),
            ("r = R\"x(a \")\" { )x\"; b;", "r = R ; b;"),
            ("r = u8R\"(a { )\"; b;", "r = u8R ; b;"),
            ("#define X {\\\n  }\nb;", "b;"),
            ("#if 1 /* {\n } */\nb;", "b;"),
            ("  # include <a.h> {\nb;", "b;"),
        ] {
            let got = c_code(text);
            assert_eq!(
                got.split_whitespace().collect::<Vec<_>>().join(" "),
                code,
                "{text:?}"
            );
            assert_eq!(got.len(), text.len(), "{text:?}");
            let breaks = |s: &str| s.match_indices('\n').map(|(i, _)| i).collect::<Vec<_>>();
            assert_eq!(breaks(&got), breaks(text), "{text:?}");
        }
    }

    #[test]
    fn a_struct_union_or_class_head_opens_a_body_and_nothing_else_does() {
        for (head, want) in [
            ("struct invoice ", Some("invoice")),
            ("typedef struct ", Some("")),
            ("union value", Some("value")),
            (
                "template <typename T> class LEDGER_API Ledger : public Base<T> ",
                Some("Ledger"),
            ),
            (
                "struct __attribute__ ((__packed__)) sdshdr8 ",
                Some("sdshdr8"),
            ),
            ("enum state ", None),
            ("int main(void) ", None),
            ("static const int xs[] = ", None),
            ("namespace billing ", None),
            ("", None),
        ] {
            assert_eq!(c_struct_head(head).as_deref(), want, "{head:?}");
        }
    }

    #[test]
    fn an_anonymous_body_is_the_named_one_around_it() {
        let code =
            c_code("struct outer {\n  union {\n    int a;\n  } u;\n};\nstruct {\n  int b;\n} x;\n");
        assert_eq!(
            c_struct_around(&code, code.find("a;").unwrap()).as_deref(),
            Some("outer")
        );
        assert_eq!(
            c_struct_around(&code, code.find("b;").unwrap()).as_deref(),
            Some("")
        );
    }

    #[test]
    fn data_members_are_told_from_the_other_lines_of_a_body() {
        let text = "struct S {\n  int a;\n  int *x, *b;\n  int c[4];\n  unsigned d : 4;\n  static int e;\n  int (*f)(int);\n  int g = 1;\n  int h{2};\n  int i GUARDED_BY(mu_);\n  struct {\n    int j;\n  } k;\n  int m(int);\n  struct n;\n  typedef int o;\n  using p = int;\n  friend class q;\n  static_assert(r);\n};\n";
        let lines: Vec<usize> = (1..=text.lines().count()).collect();
        for (word, field) in [
            ("a", true),
            ("b", true),
            ("c", true),
            ("d", true),
            ("e", true),
            ("f", true),
            ("g", true),
            ("h", true),
            ("i", true),
            ("j", true),
            ("k", true),
            ("m", false),
            ("n", false),
            ("o", false),
            ("p", false),
            ("q", false),
            ("r", false),
        ] {
            let rows = c_field_rows(text, &lines, word);
            let owners: Vec<&str> = rows.iter().map(|r| r.owner_type.as_str()).collect();
            assert_eq!(owners, if field { vec!["S"] } else { vec![] }, "{word}");
        }
    }

    #[test]
    fn the_field_pattern_takes_each_declarator_form() {
        let re = Regex::new(&c_field_pattern("f")).unwrap();
        for line in [
            "  int f;",
            "  T *a, *f;",
            "  int f[4];",
            "  int f : 4;",
            "  int f = 1;",
            "  int f{1};",
            "  int f GUARDED_BY(mu_);",
            "  } f;",
            "  int (*f)(int);",
            "  void (**f)(void);",
        ] {
            assert!(re.is_match(line), "{line}");
        }
        assert!(!re.is_match("  x = f(1);"));
    }

    #[test]
    fn a_member_of_a_constructors_initializer_list_names_its_class() {
        let text = "Ledger::Ledger(int n)\n    : Base<T>(n), total_(n),\n      rows_{n} {\n}\nclass Box {\n  Box(int n) : size_(n) {}\n  int f() { return g(1); }\n};\nLedger::Other(int n) : x_(n) {}\n";
        let at = |line1: usize, word: &str| {
            let col = text.lines().nth(line1 - 1).unwrap().find(word).unwrap();
            c_initialized_member(text, line1, col)
        };
        assert_eq!(at(2, "total_").as_deref(), Some("Ledger"));
        assert_eq!(at(3, "rows_").as_deref(), Some("Ledger"));
        assert_eq!(at(6, "size_").as_deref(), Some("Box"));
        assert_eq!(at(2, "Base"), None);
        assert_eq!(at(7, "g"), None);
        assert_eq!(at(9, "x_"), None);
    }

    #[test]
    fn a_same_named_member_of_an_initializer_list_is_not_its_parameter() {
        let text = "Kettle::Kettle(int spout) : spout(spout) {}\n";
        let at = |col: usize| c_bindings_at(text, 1, Some(col), "spout");
        assert_eq!(at(28), vec![]);
        assert_eq!(
            at(34),
            vec![CPlace {
                line1: 1,
                byte_col: 19
            }]
        );
    }

    #[test]
    fn a_local_is_declared_in_each_form_a_statement_takes() {
        for decl in [
            "T x;",
            "T *x = f();",
            "T a, *x;",
            "T a = 1, *x;",
            "T x[N];",
            "struct S x;",
            "const T& x = y;",
            "T x(1, 2);",
            "T x{1};",
            "auto [a, x] = p;",
            "const auto& [x, b] = p;",
        ] {
            let text = format!("void f(int x) {{\n  {decl}\n  use(x);\n}}\n");
            assert_eq!(bound_on(&text, 3, "x"), [2], "{decl}");
        }
        for stmt in [
            "x = y;",
            "p = x;",
            "a & x;",
            "return x;",
            "x == y;",
            "struct x;",
        ] {
            let text = format!("void f(int x) {{\n  {stmt}\n  use(x);\n}}\n");
            assert_eq!(bound_on(&text, 3, "x"), [1], "{stmt}");
        }
    }

    #[test]
    fn a_control_head_binds_only_what_takes_an_initializer() {
        for (head, want) in [
            ("if (T *x = f())", 2),
            ("while (T x = next())", 2),
            ("for (auto& x : xs)", 2),
            ("switch (int x = g(); x)", 2),
            ("for (T x; x < n; ++x)", 1),
            ("if (a * x[0] > 1)", 1),
            ("while (a * x(1))", 1),
        ] {
            let text = format!("void f(int x) {{\n  {head} {{\n    use(x);\n  }}\n}}\n");
            assert_eq!(bound_on(&text, 3, "x"), [want], "{head}");
        }
    }

    #[test]
    fn a_for_without_braces_binds_its_statement_and_the_innermost_head_wins() {
        let text = "void f(int i) {\n  for (int i = 0; i < n; i++)\n    use(i);\n}\n";
        assert_eq!(bound_on(text, 3, "i"), [2]);
        let text = "void f(int i) {\n  for (int i = 0; i < n; i++)\n    for (T i : xs)\n      use(i);\n}\n";
        assert_eq!(bound_on(text, 4, "i"), [3]);
    }

    #[test]
    fn parameters_are_the_last_word_of_each_entry_or_a_function_pointers_name() {
        for params in [
            "int x",
            "T *x",
            "T x[]",
            "T& x",
            "T x = 1",
            "T (*x)(int)",
            "int n, void (^x)(void)",
        ] {
            let text = format!("void f({params}) {{\n  use(x);\n}}\n");
            assert_eq!(bound_on(&text, 2, "x"), [1], "{params}");
        }
        for head in ["bool C::operator()(int x) const", "bool operator==(T x)"] {
            let text = format!("{head} {{\n  use(x);\n}}\n");
            assert_eq!(bound_on(&text, 2, "x"), [1], "{head}");
        }
        let text = "void f(unsigned int, T) {\n  use(int, T);\n}\n";
        assert_eq!(bound_on(text, 2, "int"), Vec::<usize>::new());
        assert_eq!(bound_on(text, 2, "T"), Vec::<usize>::new());
    }

    #[test]
    fn a_block_closed_above_the_cursor_binds_nothing() {
        let text = "void f(int x) {\n  {\n    T x;\n  }\n  use(x);\n}\n";
        assert_eq!(bound_on(text, 5, "x"), [1]);
    }

    #[test]
    fn the_cursor_reads_the_last_use_on_its_line() {
        let text = "void f(int x) {\n  { int x = 1; use(x); }\n}\n";
        assert_eq!(bound_on(text, 2, "x"), [2]);
    }

    #[test]
    fn a_lambda_reads_on_into_its_function_and_a_catch_binds_its_body() {
        let text = "void f(int x) {\n  auto g = [](int y) {\n    use(x, y);\n  };\n}\n";
        assert_eq!(bound_on(text, 3, "x"), [1]);
        assert_eq!(bound_on(text, 3, "y"), [2]);
        let text = "void f(int e) {\n  try {\n  } catch (const E& e) {\n    use(e);\n  }\n}\n";
        assert_eq!(bound_on(text, 4, "e"), [3]);
        let text = "void f(int x) {\n  switch (k) {\n  case 1: if (T *x = g()) {\n    use(x);\n  }\n  }\n}\n";
        assert_eq!(bound_on(text, 4, "x"), [3]);
    }

    #[test]
    fn a_method_stands_in_its_out_of_line_class_or_the_class_around_it() {
        let text = "int Ledger::total() {\n  return n;\n}\nclass Box {\n  int f() {\n    auto g = [](int a) { return n; };\n  }\n};\nint free() {\n  return n;\n}\n";
        assert_eq!(c_method_class(text, 2, "n").as_deref(), Some("Ledger"));
        assert_eq!(c_method_class(text, 6, "n").as_deref(), Some("Box"));
        assert_eq!(c_method_class(text, 10, "n"), None);
    }

    #[test]
    fn enum_constant_lines() {
        let line = |p: String, l: &str| Regex::new(&p).unwrap().is_match(l);
        assert!(line(c_enumerators(Some("B")), "  A, B = 1 << 2,"));
        assert!(line(c_enumerators(Some("LAST")), "    LAST };"));
        assert!(line(c_enumerators(None), "  A = 1, // one"));
        assert!(!line(c_enumerators(None), "  foo(a, b);"));
        assert!(line(c_enum_line("B"), "typedef enum { A, B } X;"));
        assert!(line(c_enum_line("A"), "enum X { A, B };"));
        assert!(!line(c_enum_line("A"), "enum X {"));
    }

    #[test]
    fn a_member_declared_with_no_body() {
        let re = Regex::new(&c_member_decl(None)).unwrap();
        for line in [
            "  virtual void Run() = 0;",
            "  int size() const;",
            "  void f() override;",
            "  virtual ~Box() = default;",
            "  Status Get(const ReadOptions& options,",
            "  Slice key(k, n);",
        ] {
            assert!(re.is_match(line), "{line}");
        }
        for line in [
            "int size() const;",
            "  // Users of Env (including the",
            "  void f() {",
        ] {
            assert!(!re.is_match(line), "{line}");
        }
    }

    #[test]
    fn a_body_head_reads_up_over_a_few_lines() {
        let lines = ["enum Order", "{", "  A,", "};"];
        assert_eq!(c_body_head(&lines, 3).as_deref(), Some("enum Order"));
        let lines = ["class X", ": public Y {", "  int a;"];
        assert_eq!(
            c_body_head(&lines, 3).as_deref(),
            Some("class X : public Y")
        );
        let lines = ["struct S // note {", "{", "  int a;"];
        assert_eq!(c_body_head(&lines, 3).as_deref(), Some("struct S"));
        assert_eq!(c_body_head(&["int a;", "int b;"], 2), None);
    }

    #[test]
    fn a_member_declaration_names_its_classes_up_to_a_function() {
        let text = "namespace leveldb {\nclass Iterator {\n public:\n  virtual bool Valid() const = 0;\n  bool Next() { return true; }\n};\n}\nnamespace a {\n  void g(int);\n}\nnamespace n {\nvoid f() {\n  struct Local {\n    void h();\n  };\n}\n}\nstruct {\n  void k();\n} x;\n";
        let class = |word: &str| c_member_class(text, line_of(text, &format!("{word}(")), word);
        let path = |p: &[&str]| Some(p.iter().map(ToString::to_string).collect::<Vec<_>>());
        assert_eq!(class("Valid"), path(&["leveldb", "Iterator"]));
        assert_eq!(class("Next"), None);
        assert_eq!(class("g"), None);
        assert_eq!(class("h"), path(&["Local"]));
        assert_eq!(class("k"), None);
    }

    #[test]
    fn an_out_of_line_definition_is_qualified_by_the_scopes_around_its_class() {
        let scopes = ["leveldb".to_owned(), "Iterator".to_owned()];
        for (line, defines) in [
            ("bool Iterator::Valid() const {", true),
            ("bool leveldb::Iterator::Valid() const {", true),
            ("Iterator::~Valid() {", true),
            ("bool SkipList<K>::Iterator::Valid() const {", false),
            ("bool Other::Valid() {", false),
            ("bool x::leveldb::Iterator::Valid() {", false),
        ] {
            assert_eq!(c_defines_member(line, &scopes, "Valid"), defines, "{line}");
        }
    }

    #[test]
    fn a_receiver_is_read_back_through_fields_indexes_and_a_call() {
        let parts = |r: Option<CReceiver>| r.map(|r| (r.head, r.head_called, r.fields));
        let r = |head: &str, called: bool, fields: &[&str]| {
            Some((
                head.to_owned(),
                called,
                fields.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ))
        };
        assert_eq!(
            parts(c_receiver("x = a->b.c->")),
            r("a", false, &["b", "c"])
        );
        assert_eq!(parts(c_receiver("c->argv[j]->")), r("c", false, &["argv"]));
        assert_eq!(
            parts(c_receiver("lookupClient(x)->")),
            r("lookupClient", true, &[])
        );
        for before in ["((client *)p)->", "this->", "a->f(x)->", "1.", "A{}."] {
            assert_eq!(c_receiver(before), None, "{before}");
        }
        assert_eq!(cpp_receiver("a[0]."), None);
        assert_eq!(parts(cpp_receiver("this->")), r("this", false, &[]));
        let chain = cpp_receiver("x = ::ns::A{1}.f(2).g->")
            .map(|r| (r.head, r.braced, r.fields, r.calls, r.start));
        let fields = vec!["f".to_owned(), "g".to_owned()];
        let want = ("::ns::A".to_owned(), true, fields, vec![true, false], 3);
        assert_eq!(chain, Some(want));
        let call = cpp_receiver("ns::f(a).g().").map(|r| (r.head, r.head_called, r.start));
        assert_eq!(call, Some(("f".to_owned(), true, 0)));
    }

    #[test]
    fn a_declaration_types_its_name_by_the_last_word_or_its_body() {
        let at = |code: &str, name: &str| {
            let code = c_code(code);
            let at = code.rfind(name).unwrap();
            c_decl_type(&code, at)
        };
        let name = |n: &str| Some(CType::Name(n.to_owned()));
        assert_eq!(at("struct client *c;", "c"), name("client"));
        assert_eq!(
            at("static const struct client *a, *c;", "c"),
            name("client")
        );
        assert_eq!(at("void f(client *c) {", "c"), name("client"));
        assert_eq!(at("int (*c)(int);", "c"), None);
        assert_eq!(at("std::vector<int> c;", "c"), None);
        assert_eq!(at("Box<int> c;", "c"), None);
        assert_eq!(at("struct { int a; } c;", "c"), Some(CType::Body(7)));
        assert_eq!(at("typedef struct { int a; } c;", "c"), None);
    }

    #[test]
    fn a_type_declaration_opens_a_body_or_names_another_type() {
        let at = |code: &str, name: &str| {
            let code = c_code(code);
            let at = code.rfind(name).unwrap();
            c_type_def(&code, at)
        };
        assert_eq!(
            at("typedef struct tag name;", "name"),
            Some(CType::Name("tag".to_owned()))
        );
        assert_eq!(
            at("typedef T name;", "name"),
            Some(CType::Name("T".to_owned()))
        );
        assert_eq!(at("struct name {", "name"), Some(CType::Body(12)));
        assert_eq!(
            at("typedef struct {\n} name;", "name"),
            Some(CType::Body(15))
        );
        assert_eq!(at("struct name;", "name"), None);
        assert_eq!(at("enum name {", "name"), None);
    }

    #[test]
    fn a_field_of_a_body_may_sit_in_a_nested_anonymous_one() {
        let code = c_code(
            "struct S {\n  union {\n    int a;\n  };\n  struct N {\n    int b;\n  } n;\n};\n",
        );
        let open = code.find('{').unwrap();
        assert_eq!(c_body_field(&code, open, "a"), code.find("a;"));
        assert_eq!(c_body_field(&code, open, "b"), None);
        assert_eq!(c_body_field(&code, open, "n"), code.find("n;"));
    }

    #[test]
    fn a_body_cut_off_inside_a_nested_anonymous_one_finds_no_field() {
        let code = c_code("struct S { struct { int x;");
        assert_eq!(c_body_field(&code, code.find('{').unwrap(), "x"), None);
    }

    #[test]
    fn a_value_type_is_its_one_declaration_in_the_file() {
        let text = "static struct name { int a; } name;\nint f() { return name.a; }\n";
        let open = text.find('{').unwrap();
        assert_eq!(c_value_type(text, 2, "name"), Ok(Some(CType::Body(open))));
        let text = "struct a *x;\nstruct b *x;\nint f() { return x->n; }\n";
        assert_eq!(c_value_type(text, 3, "x"), Err(()));
        assert_eq!(c_value_type(text, 3, "y"), Ok(None));
    }
}
