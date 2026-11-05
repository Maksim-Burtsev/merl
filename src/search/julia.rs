use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::{Binding, Import, Value, indent};

macro_rules! julia_macros {
    () => {
        r"(?:(?:[\w.]+\.)?@[\w.]+\s+)*"
    };
}

macro_rules! julia_method_tail {
    () => {
        r"\s*(?:::\s*[^=\s][^=]*?)?\s*(?:where\b[^=]*)?=(?:[^=>]|$)"
    };
}

macro_rules! julia_name_end {
    () => {
        r"(?:[^\w!.]|$)"
    };
}

const MACROS: &str = julia_macros!();
const END: &str = julia_name_end!();
const QUALIFIER: &str = r"(?:[\w.]+\.)?";
const TYPE_PARAMS: &str = r"(?:\{\s*[^}\s][^}]*\}\s*)?";

pub fn julia_patterns(word: &str) -> Vec<String> {
    let (w, e) = (regex::escape(word), END);
    vec![
        format!(r"^\s*{MACROS}function\s+{QUALIFIER}{w}{e}"),
        format!(r"^\s*{MACROS}{QUALIFIER}{w}\s*{TYPE_PARAMS}\("),
        format!(r"^\s*{MACROS}(?:mutable\s+)?struct\s+{w}{e}"),
        format!(r"^\s*(?:abstract|primitive)\s+type\s+{w}{e}"),
        julia_macro(word),
        format!(r"^\s*(?:bare)?module\s+{w}{e}"),
        format!(r"^\s*(?:global\s+)?const\s+{w}\s*{TYPE_PARAMS}(?:::[^=]*)?=(?:[^=]|$)"),
        format!(r"^{w}\s*(?:::[^=]*)?=(?:[^=>]|$)"),
        julia_field(word),
        format!(r"^\s*@enum\s+(?:{w}{e}|[\w.]+(?:::[\w.]+)?\s+(?:[\w\s=]*\s)?{w}{e})"),
    ]
}

pub fn julia_narrow(
    patterns: &mut Vec<String>,
    text: &str,
    row: usize,
    line: &str,
    r: std::ops::Range<usize>,
) {
    let (Some(word), Some(before), Some(after)) =
        (line.get(r.clone()), line.get(..r.start), line.get(r.end..))
    else {
        return;
    };
    let matches = |p: &str| Regex::new(p).is_ok_and(|re| re.is_match(line));
    let (field, mac) = (julia_field(word), julia_macro(word));
    if matches(&mac) {
        return;
    }
    if before.ends_with('@') {
        patterns.retain(|p| *p == mac);
        return;
    }
    patterns.retain(|p| *p != mac);
    if on_value(text, row, before) {
        patterns.retain(|p| *p == field);
        return;
    }
    let called = after.starts_with(['(', '{']) || after.starts_with(".(");
    if called || (!before.ends_with('.') && !matches(&field)) {
        patterns.retain(|p| *p != field);
    }
}

fn on_value(text: &str, row: usize, before: &str) -> bool {
    let Some(receiver) = before.strip_suffix('.') else {
        return false;
    };
    if receiver.ends_with([')', ']']) {
        return true;
    }
    let name = |c: char| c.is_alphanumeric() || c == '_' || c == '!' || c == '.';
    let chain: String = receiver.chars().rev().take_while(|&c| name(c)).collect();
    let chain: String = chain.chars().rev().collect();
    let Some(head) = chain.split('.').next().filter(|h| !h.is_empty()) else {
        return false;
    };
    let lines: Vec<&str> = text.lines().collect();
    head.starts_with(|c: char| c.is_lowercase() || c == '_')
        && row < lines.len()
        && !julia_bindings(&lines, row, head).is_empty()
}

fn julia_field(word: &str) -> String {
    format!(
        r"^\s+{}\s*(?:::[^=]*)?(?:=(?:[^=>].*)?)?$",
        regex::escape(word)
    )
}

fn julia_macro(word: &str) -> String {
    format!(r"^\s*macro\s+{}{END}", regex::escape(word))
}

pub(super) const JULIA_FUNCTION_SYMBOL: &str = concat!(
    r"^\s*",
    julia_macros!(),
    r"function\s+(?:[\w.]+\.)?(?P<name>[A-Za-z_][\w!]*)",
    julia_name_end!()
);
pub(super) const JULIA_METHOD_SYMBOL: &str = concat!(
    r"^(?:[\w.]+\.)?(?P<name>[A-Za-z_][\w!]*)(?:\{[^}]*\})?",
    r"\((?:[^()]|\((?:[^()]|\([^()]*\))*\))*\)",
    julia_method_tail!()
);
pub(super) const JULIA_TYPE_SYMBOL: &str = concat!(
    r"^\s*",
    julia_macros!(),
    r"(?:(?:mutable\s+)?struct|(?:abstract|primitive)\s+type|macro|(?:bare)?module)",
    r"\s+(?P<name>[A-Za-z_][\w!]*)"
);

static STRUCT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"^\s*{MACROS}(?:mutable\s+)?struct\s")).expect("the regex is a literal")
});
static KEYWORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^\s*(?:{MACROS}(?:function|(?:mutable\s+)?struct)|abstract\s+type|primitive\s+type|macro|(?:bare)?module|(?:global\s+)?const|@enum)\s"
    ))
    .expect("the regex is a literal")
});
static HEAD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^\s*{MACROS}{QUALIFIER}([A-Za-z_][\w!]*)\s*{TYPE_PARAMS}\("
    ))
    .expect("the regex is a literal")
});

pub fn julia_declares<S: AsRef<str>>(lines: &[S], line: usize, word: &str) -> bool {
    let Some(text) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let text = text.as_ref();
    if KEYWORD.is_match(text) {
        return true;
    }
    if let Some(c) = HEAD.captures(text).filter(|c| &c[1] == word) {
        let Some(head) = text.get(c.get_match().end() - 1..) else {
            return false;
        };
        if closing(head).is_some() || head.contains(['"', '\'', '#']) {
            return defines(head);
        }
        let text: Vec<&str> = lines.iter().map(AsRef::as_ref).collect();
        return method_def(&text, line - 1);
    }
    if !text.starts_with([' ', '\t']) {
        return true;
    }
    let depth = indent(text);
    let owner = |code: &[bool]| {
        (0..line - 1)
            .rev()
            .map(|i| (i, lines[i].as_ref()))
            .find(|&(i, l)| {
                !code.get(i).copied().unwrap_or(false)
                    && !l.trim().is_empty()
                    && !l.trim_start().starts_with('#')
                    && indent(l) < depth
            })
            .is_some_and(|(_, l)| STRUCT.is_match(l))
    };
    let text: Vec<&str> = lines.iter().map(AsRef::as_ref).collect();
    owner(&[])
        || (text
            .iter()
            .any(|l| l.contains("\"\"\"") || l.contains("#=") || l.contains('`'))
            && owner(&julia_literal_lines(&text.join("\n"))))
}

fn defines(from_paren: &str) -> bool {
    static REST: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(concat!("^", julia_method_tail!())).expect("the regex is a literal")
    });
    closing(from_paren).is_some_and(|close| {
        from_paren
            .get(close + 1..)
            .is_some_and(|rest| REST.is_match(rest))
    })
}

fn closing(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

static INCLUDE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\binclude\s*\(\s*"([^"$]+)"\s*\)"#).expect("the regex is a literal")
});

pub fn julia_includes(from: &Path, line: &str, file: &Path) -> bool {
    let dir = from.parent().map(Path::to_path_buf).unwrap_or_default();
    INCLUDE
        .captures_iter(line.split('#').next().unwrap_or_default())
        .any(|c| super::imports::lexical(&dir.join(&c[1])).as_deref() == Some(file))
}

pub fn julia_include(line: &str, col: usize) -> Option<String> {
    INCLUDE
        .captures_iter(line)
        .find(|c| c.get(0).is_some_and(|m| m.start() <= col && col < m.end()))
        .map(|c| c[1].to_owned())
}

pub(super) fn julia_imports(text: &str, out: &mut Vec<Import>, macros: bool) {
    static IMPORT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^\s*(?:using|import)\s+([^\n#]+)").expect("the regex is a literal")
    });
    let path = |m: &str| -> Vec<String> {
        let m = m.trim();
        let unprefixed = m.trim_start_matches('.');
        let dots = m.len() - unprefixed.len();
        let mut parts: Vec<String> = unprefixed
            .split('.')
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect();
        if dots > 0 {
            parts.insert(0, ".".repeat(dots));
        }
        parts
    };
    let named = |item: &str| -> Option<(String, String)> {
        let mut it = item.split_whitespace();
        let name = it.next()?.trim_start_matches('@').to_owned();
        let alias = match (it.next(), it.next()) {
            (Some("as"), Some(a)) => a.trim_start_matches('@').to_owned(),
            _ => name.clone(),
        };
        (!name.is_empty()).then_some((alias, name))
    };
    for c in IMPORT.captures_iter(text) {
        let mut clause = c[1].trim().to_owned();
        let mut below = c
            .get(0)
            .and_then(|m| text.get(m.end()..))
            .unwrap_or_default()
            .lines()
            .skip(1);
        while clause.ends_with([',', ':'])
            && let Some(line) = below.next()
        {
            clause.push_str(line.split('#').next().unwrap_or_default().trim());
        }
        let clause = clause.trim_end_matches(';');
        match clause.split_once(':').filter(|(_, r)| !r.starts_with(':')) {
            Some((module, items)) => {
                let base = path(module);
                for (alias, name) in items
                    .split(',')
                    .map(str::trim)
                    .filter(|i| macros || !i.starts_with('@'))
                    .filter_map(named)
                {
                    let mut p = base.clone();
                    p.push(name);
                    out.push(Import {
                        name: alias,
                        path: p,
                    });
                }
            }
            None => {
                for item in clause.split(',') {
                    let mut it = item.split_whitespace();
                    let Some(module) = it.next() else { continue };
                    let p = path(module);
                    let alias = match (it.next(), it.next()) {
                        (Some("as"), Some(a)) => a.to_owned(),
                        _ => match p.last() {
                            Some(last) if !last.starts_with('.') => last.clone(),
                            _ => continue,
                        },
                    };
                    out.push(Import {
                        name: alias,
                        path: p,
                    });
                }
            }
        }
    }
}

pub(super) fn julia_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let binding = |line: usize| {
        vec![Binding {
            line1: line + 1,
            value: Value::Unknown,
        }]
    };
    let w = regex::escape(name);
    let generator = Regex::new(&format!(
        r"\bfor\s+\(?(?:[\w!]+\s*,\s*)*{w}(?:\s*,\s*[\w!]+)*\)?\s*(?:in\b|=|∈)"
    ))
    .expect("an escaped name keeps the pattern valid");
    let lambda = Regex::new(&format!(
        r"(?:^|[^\w!])(?:{w}|\((?:[^()]*[,\s])?{w}(?:[,\s:=][^()]*)?\))\s*->"
    ))
    .expect("an escaped name keeps the pattern valid");
    let catch = Regex::new(&format!(r"^\s*catch\s+{w}\s*(?:#.*)?$"))
        .expect("an escaped name keeps the pattern valid");
    let shadow = Regex::new(&format!(r"^\s*let\b(?:.*[^\w!])?{w}(?:[^\w!]|$)"))
        .expect("an escaped name keeps the pattern valid");
    let mut let_seen = false;
    if generator.is_match(lines[at]) || lambda.is_match(lines[at]) {
        return binding(at);
    }
    let literal = julia_literal_lines(&lines.join("\n"));
    let assigns = assignment(name);
    let code = |i: usize| !literal.get(i).copied().unwrap_or(false);
    let mut depth = indent(lines[at]);
    let mut body_end = at;
    let mut outer = None;
    for k in (0..=at).rev() {
        let l = lines[k];
        if !code(k) || l.trim().is_empty() || l.trim_start().starts_with('#') {
            continue;
        }
        let header = HEADER.is_match(l) || method_def(lines, k);
        if k != at && indent(l) >= depth {
            continue;
        }
        if k == at && !header {
            continue;
        }
        let body = std::mem::replace(&mut depth, indent(l));
        let_seen |= k != at && shadow.is_match(l);
        if k != at && (generator.is_match(l) || (catch.is_match(l) && !let_seen)) {
            let assigned = (k + 1..body_end)
                .rev()
                .find(|&i| code(i) && assigns(lines[i]));
            return binding(assigned.unwrap_or(k));
        }
        if !header && k != at {
            outer = (k + 1..body_end)
                .find(|&i| code(i) && indent(lines[i]) == body && assigns(lines[i]))
                .or(outer);
            if outer.is_some() && l.trim_start().starts_with("@testset") {
                break;
            }
        }
        if header {
            if let Some(i) = (k + 1..body_end)
                .rev()
                .find(|&i| code(i) && assigns(lines[i]))
            {
                return binding(i);
            }
            let head = lines[k..lines.len().min(k + 20)].join("\n");
            if params(&head).iter().any(|p| p == name) {
                return binding(k);
            }
            body_end = k;
        }
        if depth == 0 {
            break;
        }
    }
    outer.map(binding).unwrap_or_default()
}

fn assignment(name: &str) -> impl Fn(&str) -> bool {
    let w = regex::escape(name);
    let re = Regex::new(&format!(
        r"^\s*(?:local\s+)?\(?(?:[\w!]+\s*,\s*)*{w}\s*(?:::[^=,]*)?(?:,\s*[\w!]+\s*)*\)?\s*=(?:[^=>]|$)|^\s*for\s+\(?(?:[\w!]+\s*,\s*)*{w}(?:\s*,\s*[\w!]+)*\)?\s*(?:in\b|=|∈)"
    ))
    .expect("an escaped name keeps the pattern valid");
    move |l: &str| {
        let code = l.split_once('#').map_or(l, |(c, _)| c).trim_end();
        re.is_match(l)
            && !code.ends_with(',')
            && code.matches(['(', '[']).count() >= code.matches([')', ']']).count()
    }
}

pub fn julia_assigns_here(line: &str, start: usize, word: &str) -> bool {
    assignment(word)(line)
        && line
            .match_indices('=')
            .find(|&(i, _)| {
                line.split_at_checked(i).is_some_and(|(before, rest)| {
                    !rest.get(1..).is_some_and(|r| r.starts_with(['=', '>']))
                        && !before.ends_with(['=', '!', '<', '>'])
                })
            })
            .is_some_and(|(i, _)| start < i)
}

static HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^\s*{MACROS}(?:function|macro)\s|\bdo(?:\s+[^#]*)?$"
    ))
    .expect("the regex is a literal")
});

fn method_def(lines: &[&str], k: usize) -> bool {
    HEAD.find(lines[k]).is_some_and(|m| {
        lines[k..lines.len().min(k + 20)]
            .join("\n")
            .get(m.end() - 1..)
            .is_some_and(defines)
    })
}

fn params(head: &str) -> Vec<String> {
    static NAME: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[A-Za-z_][\w!]*").expect("the regex is a literal"));
    static DO: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\bdo(?:\s+([^#]*))?\s*(?:#.*)?$").expect("the regex is a literal")
    });
    let first = head.lines().next().unwrap_or_default();
    if let Some(c) = DO.captures(first) {
        return c
            .get(1)
            .map_or("", |m| m.as_str())
            .split(',')
            .filter_map(|a| NAME.find(a.trim().trim_start_matches('(')))
            .map(|m| m.as_str().to_owned())
            .collect();
    }
    let mut out = Vec::new();
    let Some(mut rest) = head.find('(').and_then(|i| head.get(i..)) else {
        return out;
    };
    while rest.starts_with('(') {
        let Some(close) = closing(rest) else { break };
        let mut depth = 0usize;
        let mut start = 1;
        let (Some(inner), Some(after)) = (rest.get(..close), rest.get(close + 1..)) else {
            break;
        };
        for (i, c) in inner.char_indices() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                ',' | ';' if depth == 1 => {
                    out.extend(
                        (inner.get(start..i))
                            .and_then(|s| NAME.find(s.trim()))
                            .map(|m| m.as_str().to_owned()),
                    );
                    start = i + 1;
                }
                _ => {}
            }
        }
        out.extend(
            (inner.get(start..))
                .and_then(|s| NAME.find(s.trim()))
                .map(|m| m.as_str().to_owned()),
        );
        rest = after.trim_start();
    }
    out
}

pub(super) fn julia_literal_lines(text: &str) -> Vec<bool> {
    #[derive(PartialEq)]
    enum At {
        Code,
        Str { quote: u8, triple: bool, line: bool },
        Comment(usize),
    }
    let b = text.as_bytes();
    let mut out = vec![false];
    let mut at = At::Code;
    let mut i = 0;
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'!';
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            if let At::Str { line: true, .. } = at {
                at = At::Code;
            }
            out.push(at != At::Code);
            i += 1;
            continue;
        }
        match at {
            At::Code => match c {
                b'#' if b.get(i + 1) == Some(&b'=') => {
                    at = At::Comment(1);
                    i += 2;
                    continue;
                }
                b'#' => {
                    while i < b.len() && b[i] != b'\n' {
                        i += 1;
                    }
                    continue;
                }
                b'"' | b'`' => {
                    let triple = b.get(i + 1) == Some(&c) && b.get(i + 2) == Some(&c);
                    let prefixed = i > 0 && word(b[i - 1]);
                    at = At::Str {
                        quote: c,
                        triple,
                        line: prefixed && !triple,
                    };
                    i += if triple { 3 } else { 1 };
                    continue;
                }
                b'\'' => {
                    let adjoint = i > 0 && (word(b[i - 1]) || b")]}'.".contains(&b[i - 1]));
                    if !adjoint {
                        let mut j = i + 1;
                        while j < b.len() && b[j] != b'\n' {
                            match b[j] {
                                b'\\' => j += 2,
                                b'\'' => break,
                                _ => j += 1,
                            }
                        }
                        if b.get(j) == Some(&b'\'') {
                            i = j + 1;
                            continue;
                        }
                    }
                }
                _ => {}
            },
            At::Str { quote, triple, .. } => {
                if c == b'\\' {
                    i += 1;
                    if b.get(i) == Some(&b'\n') {
                        continue;
                    }
                } else if c == quote
                    && (!triple || (b.get(i + 1) == Some(&quote) && b.get(i + 2) == Some(&quote)))
                {
                    at = At::Code;
                    i += if triple { 3 } else { 1 };
                    continue;
                }
            }
            At::Comment(depth) => {
                if c == b'#' && b.get(i + 1) == Some(&b'=') {
                    at = At::Comment(depth + 1);
                    i += 2;
                    continue;
                }
                if c == b'=' && b.get(i + 1) == Some(&b'#') {
                    at = match depth {
                        1 => At::Code,
                        d => At::Comment(d - 1),
                    };
                    i += 2;
                    continue;
                }
            }
        }
        i += 1;
    }
    out.truncate(text.lines().count().max(1));
    out
}

pub fn julia_name(line: &str, r: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let rest = &line.as_bytes()[r.end..];
    if rest.first() == Some(&b'!') && rest.get(1) != Some(&b'=') {
        r.start..r.end + 1
    } else {
        r
    }
}

pub fn julia_extends(line: &str, word: &str) -> bool {
    Regex::new(&format!(
        r"^\s*{MACROS}(?:function\s+)?[\w.]+\.{}\s*{TYPE_PARAMS}\(",
        regex::escape(word)
    ))
    .is_ok_and(|re| re.is_match(line))
}

pub fn julia_type_param<S: AsRef<str>>(lines: &[S], word: &str) -> bool {
    static HEAD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\bwhere\s*|\b(?:struct|type)\s+\w+\s*").expect("the regex is a literal")
    });
    let Ok(item) = Regex::new(&format!(r"^\s*{}\b", regex::escape(word))) else {
        return false;
    };
    let text = lines
        .iter()
        .map(AsRef::as_ref)
        .collect::<Vec<_>>()
        .join("\n");
    HEAD.find_iter(&text).any(|m| {
        let rest = text.get(m.end()..).unwrap_or_default();
        let names = |r: std::ops::Range<usize>| rest.get(r).is_some_and(|s| item.is_match(s));
        let Some(list) = closing(rest).filter(|_| rest.starts_with('{')) else {
            return item.is_match(rest);
        };
        let mut depth = 0usize;
        let mut start = 1;
        rest.char_indices()
            .take_while(|&(i, _)| i < list)
            .any(|(i, c)| {
                match c {
                    '(' | '[' | '{' => depth += 1,
                    ')' | ']' | '}' => depth = depth.saturating_sub(1),
                    ',' if depth == 1 => {
                        let hit = names(start..i);
                        start = i + 1;
                        return hit;
                    }
                    _ => {}
                }
                false
            })
            || names(start..list)
    })
}

pub fn julia_whole(line: &str, word: &str) -> bool {
    word.ends_with('!') || julia_col(line, word).is_some()
}

pub fn julia_col(line: &str, word: &str) -> Option<usize> {
    let b = line.as_bytes();
    let part = |i: usize| {
        b.get(i)
            .is_some_and(|&c| c.is_ascii_alphanumeric() || c == b'_')
    };
    line.match_indices(word).map(|(i, _)| i).find(|&i| {
        let end = i + word.len();
        !(part(end) || i > 0 && part(i - 1))
            && (word.ends_with('!') || b.get(end) != Some(&b'!') || b.get(end + 1) == Some(&b'='))
    })
}

pub fn julia_slug(uuid: &str, tree: &str) -> Option<String> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let uuid = u128::from_str_radix(&uuid.replace('-', ""), 16).ok()?;
    let mut bytes = uuid.to_le_bytes().to_vec();
    if tree.len() != 40 {
        return None;
    }
    for i in (0..40).step_by(2) {
        bytes.push(u8::from_str_radix(tree.get(i..i + 2)?, 16).ok()?);
    }
    let mut crc = !0u32;
    for &byte in &bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0x82F6_3B78 & (crc & 1).wrapping_neg());
        }
    }
    let mut crc = !crc;
    Some(
        (0..5)
            .map(|_| {
                let c = CHARS[(crc % 62) as usize] as char;
                crc /= 62;
                c
            })
            .collect(),
    )
}

#[expect(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "extensions match as written, as kind_of does; ignoring case would change which files merl recognises (#873)"
)]
pub fn julia_roots(root: &Path, share: Option<PathBuf>, depot: &Path) -> Vec<PathBuf> {
    let dirs_in = |d: &Path| -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = std::fs::read_dir(d)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        found.sort();
        found
    };
    let mut dirs = Vec::new();
    if let Some(share) = share {
        dirs.push(share.join("base"));
        for version in dirs_in(&share.join("stdlib")) {
            dirs.extend(dirs_in(&version).into_iter().map(|p| p.join("src")));
        }
    }
    let read = |names: &[String]| {
        names
            .iter()
            .find_map(|n| std::fs::read_to_string(root.join(n)).ok())
            .unwrap_or_default()
    };
    let mut versioned: Vec<String> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("Manifest-v") && n.ends_with(".toml"))
        .collect();
    versioned.sort();
    let manifests: Vec<String> = std::iter::once("JuliaManifest.toml".to_owned())
        .chain(versioned.into_iter().rev())
        .chain(["Manifest.toml".to_owned()])
        .collect();
    let own = read(&["JuliaProject.toml".to_owned(), "Project.toml".to_owned()])
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == "name").then(|| v.trim().trim_matches('"').to_owned())
        });
    let mut pinned = std::collections::HashMap::new();
    let mut name: Option<String> = None;
    let (mut uuid, mut tree) = (None, None);
    let manifest = read(&manifests);
    for line in manifest.lines().chain(["[["]) {
        let line = line.trim();
        if line.starts_with("[[") {
            if let (Some(n), Some(u), Some(t)) = (name.take(), uuid.take(), tree.take())
                && let Some(slug) = julia_slug(u, t)
            {
                pinned.insert(n, slug);
            }
            (uuid, tree) = (None, None);
            name = line
                .trim_start_matches("[[")
                .trim_end_matches("]]")
                .trim_start_matches("deps.")
                .trim_matches('"')
                .to_owned()
                .into();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim().trim_matches('"');
        match k.trim() {
            "uuid" => uuid = Some(v),
            "git-tree-sha1" => tree = Some(v),
            _ => {}
        }
    }
    for package in dirs_in(&depot.join("packages")) {
        let Some(n) = package.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if own.as_deref() == Some(n) {
            continue;
        }
        match pinned.get(n) {
            Some(slug) => dirs.push(package.join(slug).join("src")),
            None => dirs.extend(dirs_in(&package).into_iter().map(|p| p.join("src"))),
        }
    }
    dirs
}

pub fn julia_share(bindir: &str) -> Option<PathBuf> {
    Some(Path::new(bindir.trim()).parent()?.join("share/julia"))
}

pub fn julia_depot(var: Option<std::ffi::OsString>, home: &Path) -> PathBuf {
    var.and_then(|v| std::env::split_paths(&v).next())
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| home.join(".julia"))
}

static FUNCTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"^\s*{MACROS}function\s")).expect("the regex is a literal")
});
static PARAM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z_][\w!]*").expect("the regex is a literal"));

fn signature(lines: &[&str], k: usize) -> Option<(String, String)> {
    let head = lines.get(k..lines.len().min(k + 20))?.join("\n");
    let from = head.get(head.find('(')?..)?;
    let close = closing(from)?;
    let tail = from.get(close + 1..)?.lines().next().unwrap_or_default();
    Some((from.get(1..close)?.to_owned(), tail.to_owned()))
}

fn top_level(inner: &str) -> Vec<&str> {
    let (mut out, mut depth, mut start) = (Vec::new(), 0usize, 0);
    for (i, c) in inner.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' | ';' if depth == 0 => {
                out.extend(inner.get(start..i));
                start = i + 1;
            }
            _ => {}
        }
    }
    out.extend(inner.get(start..));
    out
}

fn names_any(text: &str, names: &[String]) -> bool {
    names.iter().any(|n| {
        Regex::new(&format!(r"(?:^|[^\w!.]){}(?:[^\w!]|$)", regex::escape(n)))
            .is_ok_and(|re| re.is_match(text))
    })
}

fn rebinds(lines: &[&str], k: usize, name: &str) -> bool {
    let literal = julia_literal_lines(&lines.join("\n"));
    let depth = indent(lines[k]);
    let end = (k + 1..lines.len())
        .find(|&i| {
            let l = lines[i].trim();
            !literal.get(i).copied().unwrap_or(false)
                && !l.is_empty()
                && !l.starts_with('#')
                && indent(lines[i]) <= depth
        })
        .unwrap_or(lines.len());
    let method = lines[k..end].join("\n");
    let Some(body) = method
        .find('(')
        .and_then(|i| Some(i + closing(method.get(i..)?)? + 1))
        .and_then(|i| method.get(i..))
    else {
        return true;
    };
    let (w, e) = (regex::escape(name), r"(?:[^\w!.]|$)");
    Regex::new(&format!(
        r"(?m)(?:^|[^\w!.]){w}\s*->|\((?:[^()]*[,\s])?{w}(?:[,\s:=][^()]*)?\)\s*->|\bfor\b.*?(?:^|[^\w!.]){w}(?:[^\w!].*?)?(?:\s+in\b|∈|=)|\b(?:let|function|do|catch|local|global)\b.*?[^\w!.]{w}{e}|(?:^|[^\w!.]){w}\s*(?:::[^=\n]*)?(?:\s*,\s*[\w!]+)*\s*\)?\s*\.?(?:[-+*/\\^÷%&|⊻]|//|<<|>>>?)?=(?:[^=>]|$)"
    ))
    .map_or(true, |re| re.is_match(body))
}

pub fn julia_param_type(text: &str, row: usize, name: &str) -> Option<String> {
    static TYPED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^([A-Za-z_][\w!]*)\s*::\s*([A-Za-z_]\w*)\s*(?:\{[^{}]*\})?\s*(?:=[^=].*)?$")
            .expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let [b] = &julia_bindings(&lines, row, name)[..] else {
        return None;
    };
    let k = b.line1 - 1;
    if !lines.get(k).is_some_and(|l| FUNCTION.is_match(l)) && !method_def(&lines, k) {
        return None;
    }
    let (inner, tail) = signature(&lines, k)?;
    if rebinds(&lines, k, name) {
        return None;
    }
    let ty = top_level(&inner)
        .into_iter()
        .filter_map(|p| TYPED.captures(p.trim()))
        .find(|c| &c[1] == name)
        .map(|c| c[2].to_owned())?;
    (!(tail.contains("where") && names_any(&tail, std::slice::from_ref(&ty)))).then_some(ty)
}

pub fn julia_type_heads(name: &str) -> String {
    format!(
        r"^\s*{MACROS}(?:(?:mutable\s+)?struct|abstract\s+type)\s+{}{END}",
        regex::escape(name)
    )
}

pub fn julia_declares_type(line: &str, name: &str) -> bool {
    let w = regex::escape(name);
    Regex::new(&format!(
        r"^\s*(?:{MACROS}(?:mutable\s+)?struct|(?:abstract|primitive)\s+type)\s+{w}{END}|^\s*(?:global\s+)?const\s+{w}\s*{TYPE_PARAMS}=(?:[^=]|$)"
    ))
    .is_ok_and(|re| re.is_match(line))
}

pub fn julia_declares_method(line: &str, name: &str) -> bool {
    let w = regex::escape(name);
    Regex::new(&format!(
        r"^\s*{MACROS}(?:function\s+{QUALIFIER}{w}{END}|{QUALIFIER}{w}\s*{TYPE_PARAMS}\()"
    ))
    .is_ok_and(|re| re.is_match(line))
}

pub fn julia_names_a_type_here(lines: &[&str], row: usize, start: usize, name: &str) -> bool {
    let code_line = |k: usize, literal: &[bool]| {
        let l = lines[k].trim_start();
        !literal.get(k).copied().unwrap_or(false) && !l.is_empty() && !l.starts_with('#')
    };
    let Some(line) = lines.get(row) else {
        return false;
    };
    let (Some(before), Some(after)) = (line.get(..start), line.get(start + name.len()..)) else {
        return false;
    };
    if !in_type_position(before) || called(after) || julia_type_param(lines, name) {
        return false;
    }
    let literal = julia_literal_lines(&lines.join("\n"));
    let first = (0..=row)
        .rev()
        .find(|&k| code_line(k, &literal) && indent(lines[k]) == 0)
        .unwrap_or(0);
    let last = (row + 1..lines.len())
        .find(|&k| code_line(k, &literal) && indent(lines[k]) == 0)
        .map_or(lines.len(), |k| {
            k + usize::from(lines[k].trim_start().starts_with("end"))
        });
    (first..last)
        .filter(|&k| !literal.get(k).copied().unwrap_or(false))
        .all(|k| {
            let l = lines[k];
            let l = if l.contains(['"', '\'', '`']) {
                l
            } else {
                l.split('#').next().unwrap_or_default()
            };
            let defined = julia_declares_method(l, name);
            julia_col_all(l, name).all(|i| {
                let (Some(b), Some(a)) = (l.get(..i), l.get(i + name.len()..)) else {
                    return false;
                };
                in_type_position(b) || (called(a) && !defined)
            })
        })
}

fn julia_col_all<'a>(line: &'a str, word: &'a str) -> impl Iterator<Item = usize> + 'a {
    let b = line.as_bytes();
    let part = move |i: usize| {
        b.get(i)
            .is_some_and(|&c| c.is_ascii_alphanumeric() || c == b'_' || c == b'!')
    };
    line.match_indices(word)
        .map(|(i, _)| i)
        .filter(move |&i| !(part(i + word.len()) || i > 0 && part(i - 1)))
}

fn called(after: &str) -> bool {
    let after = match closing(after).filter(|_| after.starts_with('{')) {
        Some(close) => after.get(close + 1..).unwrap_or_default(),
        None => after,
    };
    after.starts_with('(') || after.starts_with(".(")
}

fn in_type_position(before: &str) -> bool {
    let b = before.trim_end();
    let word_ends = |w: &str| {
        b.strip_suffix(w)
            .is_some_and(|r| !r.ends_with(|c: char| c.is_alphanumeric() || c == '_' || c == '!'))
    };
    if b.ends_with("::") || b.ends_with("<:") || b.ends_with(">:") || b.ends_with("=>") {
        return true;
    }
    if word_ends("isa") || b.ends_with("@test_throws") {
        return before.len() > b.len();
    }
    let mut open = Vec::new();
    for (i, c) in b.char_indices() {
        match c {
            '(' | '[' | '{' => open.push((i, c)),
            ')' | ']' | '}' => {
                open.pop();
            }
            _ => {}
        }
    }
    open.last().is_some_and(|&(i, c)| {
        c == '{'
            && (b.ends_with('{') || b.ends_with(','))
            && b.get(..i)
                .is_some_and(|h| h.ends_with(|c: char| c.is_alphanumeric() || c == '_'))
    })
}

pub fn julia_is_struct(line: &str) -> bool {
    STRUCT.is_match(line)
}

pub fn julia_supertype(line: &str) -> Option<String> {
    static SUPER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(
            r"^\s*{MACROS}(?:(?:mutable\s+)?struct|abstract\s+type)\s+[A-Za-z_]\w*\s*(?:\{{[^{{}}]*(?:\{{[^{{}}]*\}}[^{{}}]*)*\}})?\s*(?:<:\s*([A-Za-z_][\w.]*)\s*(?:\{{[^{{}}]*\}})?)?\s*(?:\bend\b)?\s*(?:#.*)?$"
        ))
        .expect("the regex is a literal")
    });
    SUPER
        .captures(line)
        .map(|c| c.get(1).map_or("Any", |m| m.as_str()).to_owned())
}

pub fn julia_alias_pattern(names: &[String]) -> String {
    let names: Vec<String> = names.iter().map(|n| regex::escape(n)).collect();
    format!(
        r"^(?:const\s+)?[A-Za-z_]\w*\s*(?:\{{[^}}]*\}})?\s*=\s*(?:Union\{{[^}}]*)?\b(?:{})\b",
        names.join("|")
    )
}

pub fn julia_alias(line: &str) -> Option<String> {
    PARAM
        .find(line.trim_start().trim_start_matches("const").trim_start())
        .map(|m| m.as_str().to_owned())
}

pub fn julia_struct_of<S: AsRef<str>>(lines: &[S], at: usize) -> Option<usize> {
    let text: Vec<&str> = lines.iter().map(AsRef::as_ref).collect();
    let depth = indent(text.get(at)?);
    let literal = julia_literal_lines(&text.join("\n"));
    let k = (0..at).rev().find(|&i| {
        !literal.get(i).copied().unwrap_or(false)
            && !text[i].trim().is_empty()
            && !text[i].trim_start().starts_with('#')
            && indent(text[i]) < depth
    })?;
    STRUCT.is_match(text[k]).then_some(k)
}

static READABLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:[A-Za-z_][\w.]*\s*(?:\{[^$]*\})?)?$").expect("the regex is a literal")
});

pub fn julia_property_overload<S: AsRef<str>>(
    lines: &[S],
    k: usize,
    types: &[String],
    word: &str,
) -> bool {
    let text: Vec<&str> = lines.iter().map(AsRef::as_ref).collect();
    let Some(&line) = text.get(k) else {
        return false;
    };
    let function = FUNCTION.is_match(line);
    if !function && !method_def(&text, k) {
        return false;
    }
    let Some((inner, tail)) = signature(&text, k) else {
        return true;
    };
    let params = top_level(&inner);
    let (recv, ty) = params
        .first()
        .map(|p| p.split_once("::").unwrap_or((p, "")))
        .unwrap_or_default();
    let ty = ty.trim();
    if !READABLE.is_match(ty) {
        return true;
    }
    if !(ty.is_empty() || ty == "Any" || tail.contains("where") || names_any(ty, types)) {
        return false;
    }
    let (Some(recv), Some(sym), true) = (
        PARAM.find(recv.trim()),
        params.get(1).and_then(|p| PARAM.find(p.trim())),
        function,
    ) else {
        return true;
    };
    let depth = indent(line);
    let body: Vec<&str> = text
        .iter()
        .skip(k + 1)
        .take_while(|l| !(indent(l) == depth && l.trim_start().starts_with("end")))
        .copied()
        .collect();
    let fallback = Regex::new(&format!(
        r"\bgetfield\(\s*{}\s*,\s*{}\s*\)",
        regex::escape(recv.as_str()),
        regex::escape(sym.as_str())
    ));
    let named = Regex::new(&format!(r":{}(?:[^\w!]|$)", regex::escape(word)));
    let s = regex::escape(sym.as_str());
    let other = Regex::new(&format!(
        r"{s}\s*(?:===?|!==?|≡)\s*:[\w!]+|:[\w!]+\s*(?:===?|!==?|≡)\s*{s}(?:[^\w!]|$)|{s}\s+(?:in|∈)\s*[(\[]\s*:[\w!]+(?:\s*,\s*:[\w!]+)*\s*,?\s*[)\]]"
    ));
    let used = Regex::new(&format!(r"(?:^|[^\w!.:]){s}(?:[^\w!]|$)"));
    let (Ok(fallback), Ok(named), Ok(other), Ok(used)) = (fallback, named, other, used) else {
        return true;
    };
    !body.iter().any(|l| fallback.is_match(l))
        || body.iter().any(|l| {
            named.is_match(l) || used.is_match(&other.replace_all(&fallback.replace_all(l, ""), ""))
        })
}
