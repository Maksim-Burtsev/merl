//! C and C++ read as code: the struct a field sits in (#359), the function a parameter or a local
//! belongs to (#378). Everything here scans [`c_code`], where comments, literals and
//! preprocessor lines are blanks, so a brace in a string or a `#define` body opens nothing.

use regex::Regex;
use std::ops::Range;
use std::sync::LazyLock;

use super::{Binding, Value};

static ACCESS_LABELS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:(?:public|private|protected)\s*:\s*)+").unwrap());

/// The files the `#include` lines of the C or C++ `text` name. A line under `#if` counts as any
/// other: whichever branch a build takes, the file reaches no fewer headers.
pub fn c_includes(text: &str) -> Vec<CInclude> {
    static INCLUDE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?m)^[ \t]*#[ \t]*(?:include|include_next|import)[ \t]*([<"])([^>"\n]+)[>"]"#)
            .unwrap()
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

/// `text` of a C or C++ file as code alone: comments, string and character literals (raw strings
/// included) and preprocessor lines with their continuations turned into spaces, the length and
/// the line breaks kept, so a byte of it is the byte of `text` at the same place.
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
            // A directive runs on over a line that ends in `\`.
            directive &= text[..i].trim_end_matches('\r').ends_with('\\');
            line_start = true;
            i += 1;
            continue;
        }
        if directive {
            // A block comment opened on a directive's line runs on past it.
            let end = match b[i..].starts_with(b"/*") {
                true => text[i + 2..].find("*/").map_or(b.len(), |n| i + n + 4),
                false => i + 1,
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
                // A `//` comment runs on over a line that ends in `\` too.
                let mut j = i;
                while j < b.len() && !(b[j] == b'\n' && b[j - 1] != b'\\') {
                    j += 1;
                }
                j
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                text[i + 2..].find("*/").map_or(b.len(), |n| i + n + 4)
            }
            b'"' => {
                // `R"x(…)x"`, behind `u8`, `u`, `U` or `L` or none.
                let raw = text[..i].strip_suffix('R').is_some_and(|t| {
                    let t = ["u8", "u", "U", "L"]
                        .iter()
                        .find_map(|p| t.strip_suffix(p))
                        .unwrap_or(t);
                    !t.ends_with(|c: char| c.is_alphanumeric() || c == '_')
                });
                match raw.then(|| text[i + 1..].find('(')).flatten() {
                    Some(open) => {
                        let delim = &text[i + 1..i + 1 + open];
                        let close = format!("){delim}\"");
                        text[i + 1 + open..]
                            .find(&close)
                            .map_or(b.len(), |n| i + 1 + open + n + close.len())
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
fn c_innermost_open_brace(code: &[u8], at: usize) -> Option<usize> {
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
fn c_back_to_stop(code: &str, at: usize) -> (&str, Option<u8>) {
    let b = code.as_bytes();
    match b[..at]
        .iter()
        .rposition(|c| matches!(c, b';' | b'{' | b'}'))
    {
        Some(s) => (&code[s + 1..at], Some(b[s])),
        None => (&code[..at], None),
    }
}
fn untemplated(s: &str) -> String {
    static ANGLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^<>]*>").unwrap());
    let mut s = s.to_owned();
    while ANGLE.is_match(&s) {
        s = ANGLE.replace_all(&s, "").into_owned();
    }
    s
}
/// The name a struct, union or class head declares (`Some("")` for an anonymous one), or `None`
/// when `head`, the text before a `{`, opens no such body: an `enum`, a function, an
/// initializer (`= {`), a namespace, a block.
fn c_struct_head(head: &str) -> Option<String> {
    static ATTR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"__attribute__\s*\(\((?:[^()]|\([^()]*\))*\)\)|\b(?:alignas|__declspec)\s*\([^()]*\)|\[\[[^\]]*\]\]").unwrap()
    });
    static HEAD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^(?:template\s*(?:<.*>)?\s*)?(?:(?:typedef|static|extern|const|volatile)\s+)*(?:struct|union|class)\b(?:\s+(?:[A-Z][A-Z0-9_]*\s+)*(?:\w+::)*(\w+))?(?:\s+final)?\s*(?::(?:[^:]|$)[^;{}()=]*)?$",
        )
        .unwrap()
    });
    let head = ACCESS_LABELS.replace(head, "");
    let head = ATTR.replace_all(head.trim(), "");
    let head = untemplated(head.trim());
    HEAD.captures(head.trim_start())
        .map(|c| c.get(1).map_or(String::new(), |m| m.as_str().to_owned()))
}
/// The struct, union or class whose body byte `at` of `code` stands directly in, and its name
/// (the nearest named one out, for a nested anonymous `union { … } u;`, `""` for none).
fn c_struct_around(code: &str, at: usize) -> Option<String> {
    let mut open = c_innermost_open_brace(code.as_bytes(), at)?;
    let mut name = c_struct_head(c_back_to_stop(code, open).0)?;
    // An anonymous body's own name is its outer one's.
    while name.is_empty() {
        let Some(outer) = c_innermost_open_brace(code.as_bytes(), open) else {
            break;
        };
        match c_struct_head(c_back_to_stop(code, outer).0) {
            Some(n) => (open, name) = (outer, n),
            None => break,
        }
    }
    Some(name)
}
/// Whether the `word` at byte `at` of `code` is declared there as a data member: a line directly
/// inside a struct, union or class body (`T name;`, `T *a, *name;`, `T name[N];`, `T name : 4;`,
/// `static T name;`, `T (*name)(…);`, C++'s `T name = init;`, `T name{init};`,
/// `T name GUARDED_BY(mu_);`, and the `} name;` that closes a nested anonymous body). A member
/// function, a nested type, `typedef`, `using`, `friend` and `static_assert` are none.
fn c_field_owner(code: &str, at: usize, word: &str) -> Option<String> {
    static ACCESS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:public|private|protected|signals|slots)\s*:\s*)+").unwrap()
    });
    static REFUSED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:typedef|using|friend|static_assert|return|case|goto|else|delete|new|throw|sizeof|template|operator)\b|^(?:struct|union|class|enum)(?:\s+(?:struct|class))?\s*$").unwrap()
    });
    static AFTER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:\[[^\]]*\]\s*)*(?:[;,{\[]|=[^=]|:[^:]|[A-Z_][A-Z0-9_]*\s*\()").unwrap()
    });
    static FN_POINTER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(\s*\*+$").unwrap());
    static CALLED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*\)\s*\(").unwrap());
    let (before, stop) = c_back_to_stop(code, at);
    let after = &code[at + word.len()..];
    let pre = ACCESS.replace(before, "");
    let pre = pre.trim();
    let shaped = if pre.is_empty() {
        let closes_nested_body = stop == Some(b'}');
        closes_nested_body && AFTER.is_match(after) && !after.trim_start().starts_with(['{', '='])
    } else if let Some(m) = FN_POINTER.find(pre) {
        let ty = pre[..m.start()].trim();
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
/// A pattern for the lines that can declare `word` as a C or C++ data member, [`c_field_rows`]
/// deciding which do: the word behind a type, a `*`, a `&`, a `,` or a `}`, followed by what ends
/// a declarator; and `(*word)(`.
pub fn c_field_pattern(word: &str) -> String {
    let w = regex::escape(word);
    format!(
        r"(?:[\w\]>]\s+|[*&,}}>]\s*){w}\s*(?:\[[^\]]*\]\s*)*(?:[;,={{\[]|:(?:[^:]|$)|[A-Z_][A-Z0-9_]*\s*\()|\(\s*\*+\s*{w}\s*\)\s*\("
    )
}
/// Of the `hit_lines1` of a C or C++ `text`, the lines that declare `word` as a data member
/// ([`c_field_owner`]), each with the name of the type it belongs to.
pub fn c_field_rows(text: &str, hit_lines1: &[usize], word: &str) -> Vec<(usize, String)> {
    let code = c_code(text);
    let starts = line_starts(&code);
    let mut rows = Vec::new();
    for &line in hit_lines1 {
        let Some(&from) = line.checked_sub(1).and_then(|k| starts.get(k)) else {
            continue;
        };
        let to = starts.get(line).map_or(code.len(), |&s| s - 1);
        let found = code[from..to].match_indices(word).find_map(|(i, _)| {
            let at = from + i;
            let whole = !code[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_')
                && !code[at + word.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_');
            whole.then(|| c_field_owner(&code, at, word)).flatten()
        });
        if let Some(owner) = found {
            rows.push((line, owner));
        }
    }
    rows
}
/// Whether the word followed by `(` or `{` names a member in a constructor's initializer list:
/// `X::X(…) : a_(x), b_{y} {`, `X(…) : a_(x) {` in X's body, and its continuation lines. Gives the
/// constructor's class, `X`.
pub fn c_initialized_member(text: &str, line1: usize, byte_col: usize) -> Option<String> {
    static CTOR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^\s*(?:(?:public|private|protected)\s*:\s*)*(?:[\w~]+\s+)*(?:\w+::)*?(?:(?P<class>\w+)::)?(?P<ctor>\w+)\s*\(\)\s*(?:(?:const|noexcept|override)\b\s*(?:\(\))?\s*)*:\s*(?:(?:\w+::)*\w+\s*(?:\(\)|\{\})\s*,\s*)*$",
        )
        .unwrap()
    });
    let code = c_code(text);
    let at = code
        .split('\n')
        .take(line1.checked_sub(1)?)
        .map(|l| l.len() + 1)
        .sum::<usize>()
        + byte_col;
    let s = ctor_head_with_brackets_emptied(&code, at)?;
    if !code[at..]
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
fn ctor_head_with_brackets_emptied(code: &str, at: usize) -> Option<String> {
    let b = code.as_bytes();
    let item_of_list = |close: usize| code[close + 1..].trim_start().starts_with(',');
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
        // ponytail: a constructor's head and list over 4 KB are not read.
        if at - i > 4096 {
            return None;
        }
    }
    Some(s)
}

fn line_starts(code: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(code.match_indices('\n').map(|(i, _)| i + 1))
        .collect()
}
fn c_head_start(code: &str, open: usize) -> usize {
    let b = code.as_bytes();
    let item_of_init_list = |close: usize| code[close + 1..].trim_start().starts_with([',', '{']);
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
fn bracket_contents_blanked(s: &str) -> String {
    let mut depth = 0usize;
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let inside = depth > 0;
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        match inside && depth > 0 && c != '\n' {
            true => out.push_str(&" ".repeat(c.len_utf8())),
            false => out.push(c),
        }
    }
    out
}
fn top_level_paren_groups(s: &str) -> Vec<Range<usize>> {
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
enum Head {
    TypeOrNamespace,
    Function { params: Range<usize>, name: String },
    Lambda { params: Range<usize> },
    Catch { params: Range<usize> },
    Control { parens: Range<usize> },
    Block,
}
fn c_head(head: &str) -> Head {
    static NAMESPACE_EXTERN_OR_ENUM: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:inline\s+)?namespace\b|^\s*extern\s*$|^\s*(?:typedef\s+)?enum\b[^(]*$")
            .unwrap()
    });
    static QUALS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)^\s*(?:(?:const|volatile|noexcept|override|final|mutable|throw|&&?|[A-Z_][A-Z0-9_]*)\s*(?:\(\s*\))?\s*)*(?:->.*|:[^:].*)?$").unwrap()
    });
    static NAME: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"((?:\w+\s*::\s*)*~?\w+)\s*$").unwrap());
    if c_struct_head(head).is_some() || NAMESPACE_EXTERN_OR_ENUM.is_match(head) {
        return Head::TypeOrNamespace;
    }
    let f = bracket_contents_blanked(head);
    let Some(group) = top_level_paren_groups(head)
        .into_iter()
        .find(|g| QUALS.is_match(&f[g.end..]))
    else {
        return Head::Block;
    };
    let before = head[..group.start].trim_end();
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
/// Where in `stmt`, a statement with its `;` or `{`, a declaration names `name`: `T name;`,
/// `T *name = …;`, `T a, *name;`, `T name[N];`, `struct S name;`, `const T& name = …;`, `T name(args);`,
/// `T name{…};`, `auto [a, name] = …;`; in a control head only the forms that take an initializer
/// (a condition's `T *p = f()`, a range-for's `auto& x : xs`).
fn declared_at(stmt: &str, name: &str, place: DeclaredIn) -> Option<usize> {
    static BOUND: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:const\s+)?auto\s*&{0,2}\s*\[([^\]]*)\]\s*[=:{(]").unwrap()
    });
    static TOKEN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[A-Za-z_]\w*").unwrap());
    if let Some(m) = BOUND.captures(stmt).and_then(|c| c.get(1)) {
        return TOKEN
            .find_iter(m.as_str())
            .find(|t| t.as_str() == name)
            .map(|t| m.start() + t.start());
    }
    let f = bracket_contents_blanked(stmt);
    let is_name = |c: char| c.is_alphanumeric() || c == '_';
    f.match_indices(name).find_map(|(j, _)| {
        let whole = !f[..j].ends_with(is_name) && !f[j + name.len()..].starts_with(is_name);
        let (pre, after) = (f[..j].trim(), f[j + name.len()..].trim_start());
        let first = TOKEN.find(pre)?;
        let followed = match after.as_bytes() {
            [b'=', b'=', ..] | [b':', b':', ..] => false,
            [b'=' | b':' | b'{', ..] => true,
            [b';' | b',' | b'[' | b'(', ..] => place == DeclaredIn::Statement,
            _ => false,
        };
        // `T a = x, *name` declares `a` with its value first; `p = name` assigns.
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
/// Where in `params`, the text inside a parameter list, a parameter is called `name`: each entry
/// declares its last word (`T name`, `T *name`, `T name[]`, `T& name`, `T name = x`) or the name
/// of a function pointer, `T (*name)(…)`; `void`, `...` and an unnamed `T` declare nothing.
fn parameter_at(params: &str, name: &str) -> Option<usize> {
    static POINTER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[^(]*\(\s*[*&^]+\s*(\w+)\s*(?:\[[^\]]*\]\s*)?\)").unwrap());
    static LAST: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"[\w\s*&:>]*?[\s*&>]([A-Za-z_]\w*)\s*(?:\[[^\]]*\]\s*)*(?:=.*)?$").unwrap()
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
        let raw = &params[at..at + entry.len()];
        let lead = entry.len() - entry.trim_start().len();
        let found = match POINTER.captures(raw) {
            Some(c) => c.get(1).map(|m| (m.start(), m.as_str())),
            None => LAST
                .captures(entry.trim())
                .and_then(|c| c.get(1))
                .map(|m| (lead + m.start(), m.as_str())),
        };
        if let Some((j, _)) = found.filter(|(_, n)| *n == name && !BUILTIN.contains(n)) {
            return Some(at + j);
        }
    }
    None
}
/// The declarations of `name` in the statements of the block whose code starts at byte `from`
/// (past its `{`, or 0 for file scope), above `pos` and directly in it: a block closed before `pos`
/// is not read.
fn block_declarations(code: &str, from: usize, pos: usize, name: &str) -> Vec<usize> {
    let b = code.as_bytes();
    let (mut depth, mut start, mut out) = (0usize, from, Vec::new());
    let take = |from: usize, to: usize, out: &mut Vec<usize>| {
        if let Some(j) = declared_at(&code[from..to], name, DeclaredIn::Statement) {
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
    // The statement `pos` stands in: a `for (T i = …; …)` with no braces binds `i` in the
    // statement it runs, and closer than anything in the block.
    if let Some(at) = control_init(code, start, pos.min(code.len()), name) {
        return vec![at];
    }
    if depth == 0 && start < pos {
        take(start, pos.min(code.len()), &mut out);
    }
    out
}
/// What the heads of `for`, `if`, `while` and `switch` at the start of `code[from..pos]` bind of
/// `name` before `pos`, the innermost head first.
fn control_init(code: &str, from: usize, pos: usize, name: &str) -> Option<usize> {
    static CONTROL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*(?:else\s+)?(?:for|if|while|switch)\s*\(").unwrap());
    let mut at = from;
    let mut found = None;
    while let Some(m) = CONTROL.find(&code[at..pos]) {
        let open = at + m.end() - 1;
        let close = close_or_end(code, open, pos);
        for (j, part) in split_top_level_semicolons(&code[open + 1..close]) {
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
fn close_or_end(code: &str, open: usize, end: usize) -> usize {
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
struct Scope {
    open: usize,
    head_start: usize,
    head: Head,
}
fn scopes_inside_out_to_type_or_namespace(code: &str, pos: usize) -> Vec<Scope> {
    let mut out = Vec::new();
    let mut at = pos;
    while let Some(open) = c_innermost_open_brace(code.as_bytes(), at) {
        let head_start = c_head_start(code, open);
        let head = c_head(&code[head_start..open]);
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
/// The last `name` on the line, else its end.
fn scope_pos(code: &str, starts: &[usize], line1: usize, name: &str) -> Option<usize> {
    let from = *starts.get(line1.checked_sub(1)?)?;
    let to = starts.get(line1).map_or(code.len(), |&s| s - 1);
    let is_name = |c: char| c.is_alphanumeric() || c == '_';
    Some(
        code[from..to]
            .match_indices(name)
            .filter(|(i, _)| {
                !code[..from + i].ends_with(is_name)
                    && !code[from + i + name.len()..].starts_with(is_name)
            })
            .last()
            .map_or(to, |(i, _)| from + i),
    )
}
/// The parameters and locals that bind `name` where `line1` of a C or C++ `text` reads it (#378): the declarations above it in the innermost block around it that has any, a block
/// closed before it not counted, then what a `for`, `if`, `while` or `switch` head or a `catch`
/// binds for its body, then the parameters of the function or the Objective-C method (#417), a
/// lambda's reading on into the function around it.
pub fn c_bindings(text: &str, line1: usize, name: &str) -> Vec<Binding> {
    c_bindings_at(text, line1, name)
        .into_iter()
        .map(|p| Binding {
            line1: p.line1,
            value: Value::Unknown,
        })
        .collect()
}
/// [`c_bindings`] with columns: on a line of one function the cursor can stand on the parameter
/// or on a use of it.
pub fn c_bindings_at(text: &str, line1: usize, name: &str) -> Vec<CPlace> {
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
        if let Some(j) = super::objc_parameter(&code[start..open], name) {
            return vec![place(start + j)];
        }
        let found = match head {
            Head::Function { params, .. } | Head::Lambda { params } | Head::Catch { params } => {
                let inner = start + params.start + 1..start + params.end - 1;
                parameter_at(&code[inner.clone()], name).map(|j| inner.start + j)
            }
            Head::Control { parens } => {
                let inner = start + parens.start + 1..start + parens.end - 1;
                let init = split_top_level_semicolons(&code[inner.clone()]);
                (init.into_iter())
                    .find_map(|(at, stmt)| {
                        declared_at(stmt, name, DeclaredIn::ControlHead).map(|j| at + j)
                    })
                    .or_else(|| super::objc_for_in(&code[inner.clone()], name))
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
fn split_top_level_semicolons(s: &str) -> Vec<(usize, &str)> {
    let f = bracket_contents_blanked(s);
    let mut from = 0;
    f.split(';')
        .map(|part| {
            let at = from;
            from += part.len() + 1;
            (at, &s[at..at + part.len()])
        })
        .collect()
}
/// The class whose method `name` on `line1` of a C++ `text` stands in: the `X` of an out-of-line
/// `R X::m(…) {`, or the class whose body holds the method. A lambda reads on into the method
/// around it.
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
    c_struct_head(c_back_to_stop(&code, outer).0).filter(|n| !n.is_empty())
}

const C_ENUMERATOR: &str = r"[A-Za-z_]\w*\s*(?:=[^,;{}]*)?";
/// A line of enum constants, `NAME,`, `NAME = expr,`, `A, B, C,` or the last one without a comma,
/// with `word` among them (or any name, for `None`). A `}` may close the body after them.
pub fn c_enumerators(word: Option<&str>) -> String {
    let w = word.map_or_else(|| r"[A-Za-z_]\w*".to_owned(), regex::escape);
    format!(
        r"^\s*(?:{C_ENUMERATOR},\s*)*{w}\s*(?:=[^,;{{}}]*)?(?:,\s*{C_ENUMERATOR})*,?\s*(?:\}}[^{{]*)?(?:(?://|/\*).*)?$"
    )
}
/// `enum X { A, B };` and `typedef enum { A, B } X;`: an enum whose constants open on its own
/// line, `word` among them.
pub fn c_enum_line(word: &str) -> String {
    format!(
        r"^\s*(?:typedef\s+)?enum\b[^{{;=()]*\{{\s*(?:{C_ENUMERATOR},\s*)*{}\s*(?:=[^,;{{}}]*)?\s*(?:[,}}]|$)",
        regex::escape(word)
    )
}
/// A member function declared with no body, `word` its name (or any name, for `None`): `T
/// name(params) const;`, `virtual … = 0;`, `… override;`, `= default;`, or the first line of a
/// parameter list that wraps. Indented, behind at least a return type or a keyword, and no `//`
/// comment reading `Users of Env (including`; in a function body the same line is a local
/// object, `Slice key(k, n);`, which [`c_body_head`] tells apart.
pub fn c_member_decl(word: Option<&str>) -> String {
    let w = word.map_or_else(|| r"~?[A-Za-z_]\w*".to_owned(), regex::escape);
    format!(
        r"^\s+[^;(){{}}=#/]*[\w>][\s*&]+{w}\s*\((?:[^(){{}};]|\([^(){{}};]*\))*(?:\)[^{{}};]*;)?\s*(?:(?://|/\*).*)?$"
    )
}
/// What opens the body `line1` of a C or C++ file stands directly in: the code in front of the
/// innermost `{` not closed above the line, back to the `;`, `{` or `}` before it, over a few
/// lines (`enum Order\n{`, a base list that wraps). `None` at file scope.
pub fn c_body_head<S: AsRef<str>>(lines: &[S], line1: usize) -> Option<String> {
    c_body_opener(lines, line1).map(|opener| opener.head)
}
struct BodyOpener {
    brace_line1: usize,
    head: String,
}
fn line_code(l: &str) -> String {
    let mut s = vec![b' '; l.len()];
    if !l.trim_start().starts_with('#') {
        // A comment's first byte comes as 0, and reads as a blank here.
        for (i, c) in super::code(super::Kind::C, l).filter(|&(_, c)| c != 0) {
            s[i] = c;
        }
    }
    String::from_utf8_lossy(&s).into_owned()
}
/// ponytail: reads line by line, so a brace in a block comment counts; `c_code` over the whole
/// file per hit would cost more than the rare comment is worth.
fn c_body_opener<S: AsRef<str>>(lines: &[S], line1: usize) -> Option<BodyOpener> {
    let mut depth = 0usize;
    let mut open = None;
    'up: for k in (0..line1.checked_sub(1)?.min(lines.len())).rev() {
        let l = line_code(lines[k].as_ref());
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
    let head = line_code(lines[k].as_ref())[..i].to_owned();
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
            head = format!("{} {head}", line_code(lines[j].as_ref()));
        }
        if let Some(s) = head.rfind([';', '{', '}']) {
            head = head[s + 1..].to_owned();
            break;
        }
    }
    head.trim().to_owned()
}
/// The classes and namespaces `line1` of a C++ file stands in, outermost first, when the
/// innermost is a class or struct: `["leveldb", "Iterator"]`. An unnamed one reads `""`; a
/// function or a block on the way ends the walk.
fn c_scopes<S: AsRef<str>>(lines: &[S], line1: usize) -> Vec<String> {
    static NAMESPACE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:inline\s+)?namespace\s*((?:\w+\s*::\s*)*\w+)?\s*$").unwrap()
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
        Regex::new(r"^(?:typedef\s+)?enum\b(?:\s+(?:class|struct)\b)?[^(){}=;]*$").unwrap()
    });
    ENUM.is_match(head)
}
/// The class or struct whose body `line1` stands directly in, by name (`""` for an anonymous
/// one).
pub fn c_class_around<S: AsRef<str>>(lines: &[S], line1: usize) -> Option<String> {
    c_struct_head(&c_body_head(lines, line1)?)
}
pub fn c_declares_where<'a, S: AsRef<str> + 'a>(
    line1: usize,
    text: &str,
    lines: impl FnOnce() -> &'a [S],
) -> bool {
    static ENUMERATORS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(&c_enumerators(None)).unwrap());
    static MEMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(&c_member_decl(None)).unwrap());
    if ENUMERATORS.is_match(text) {
        return c_body_head(lines(), line1).is_some_and(|h| c_enum_head(&h));
    }
    let t = text.trim_start();
    if MEMBER.is_match(text) && !t.starts_with("typedef") && !t.starts_with("friend") {
        return c_class_around(lines(), line1).is_some();
    }
    true
}
/// The classes and namespaces around the member function `word` declared with no body directly
/// in a class body ([`c_member_decl`]) on `line1` of `text`, outermost first, its class last;
/// `None` for any other line.
pub fn c_member_class(text: &str, line1: usize, word: &str) -> Option<Vec<String>> {
    let decl = Regex::new(&c_member_decl(Some(word))).expect("an escaped name keeps it valid");
    let lines: Vec<&str> = text.lines().collect();
    if !decl.is_match(lines.get(line1.checked_sub(1)?)?) {
        return None;
    }
    Some(c_scopes(&lines, line1)).filter(|s| s.last().is_some_and(|c| !c.is_empty()))
}
/// Whether `text` defines out of line the member `word` of the class `scopes` ends with
/// ([`c_member_class`]): `R X::word(`, what qualifies `X` being the scopes around it, as far as it
/// goes (`leveldb::Iterator::Valid(`), so `SkipList<K>::Iterator::Valid(` is another class's.
pub fn c_defines_member(text: &str, scopes: &[String], word: &str) -> bool {
    static TEMPLATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^;()]*>").unwrap());
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
        let qualifier_starts_a_name =
            !text[..at].ends_with(|ch: char| ch == ':' || ch.is_alphanumeric() || ch == '_');
        qualifier_starts_a_name
            && written.len() <= scopes.len()
            && scopes[scopes.len() - written.len()..]
                .iter()
                .zip(&written)
                .all(|(s, w)| s == w)
    })
}

/// What a C value is declared as: a type by the last word of its name (`client` for `struct
/// client *c`), or the struct or union body that a `} name;` closes, by the byte of its `{`.
#[derive(Clone, Debug, PartialEq)]
pub enum CType {
    Name(String),
    Body(usize),
}
/// The receiver in front of a C member, `before` being its line up to the member and ending in
/// `->` or `.`. `a->b.c->` gives `a` with `b`, `c`; an index is read past (`c->argv[j]->`), and
/// `lookupClient(x)->` is a called head. `None` for a receiver that starts with anything else: a
/// cast, a bracketed expression, a call of a call or of a field, `this`.
pub fn c_receiver(before: &str) -> Option<CReceiver> {
    receiver(before, false)
}
#[derive(Debug, PartialEq)]
pub struct CReceiver {
    pub head: String,
    pub head_called: bool,
    pub fields: Vec<String>,
}
pub fn cpp_receiver(before: &str) -> Option<CReceiver> {
    receiver(before, true).filter(|r| !r.head_called)
}
fn receiver(before: &str, cpp: bool) -> Option<CReceiver> {
    let code = c_code(before);
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut rest = code.trim_end();
    let mut fields = Vec::new();
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
            rest = rest[..opening_of_last_bracket(rest)?].trim_end();
        }
        let called = rest.ends_with(')');
        if called {
            rest = rest[..opening_of_last_bracket(rest)?].trim_end();
        }
        let start = rest.trim_end_matches(is_name).len();
        let name = &rest[start..];
        // A non-ASCII character in front goes on with the name (`größe`, `e\u{301}tude`): the
        // tail read is not it.
        if name.is_empty()
            || name.starts_with(|c: char| c.is_ascii_digit())
            || (name == "this" && !cpp)
            || rest[..start].ends_with(|c: char| !c.is_ascii())
        {
            return None;
        }
        rest = rest[..start].trim_end();
        let member = rest.ends_with("->") || rest.ends_with('.') && !rest.ends_with("..");
        match (member, called) {
            (true, false) => fields.insert(0, name.to_owned()),
            (false, _) => {
                return Some(CReceiver {
                    head: name.to_owned(),
                    head_called: called,
                    fields,
                });
            }
            // A call of a field is a function pointer's, whose return type is not read.
            (true, true) => return None,
        }
    }
}
fn opening_of_last_bracket(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in s.bytes().enumerate().rev() {
        match c {
            b')' | b']' => depth += 1,
            b'(' | b'[' => {
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
/// What the name declared at byte `at` of `code`, a [`c_code`], is declared as: the body a `}
/// name;` closes, else the last word of the type in front of it (`client` for `client *c`,
/// `static const struct client *a, *c` and a parameter `client *c`). `None` for a type that is
/// no word (a function pointer, a template), a typedef's `} name;` and no type at all.
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
        (true, _) => (&code[last_comma.map_or(from, |c| c + 1)..at], false),
        (false, Some(c)) => (&code[from..c], true),
        (false, None) => (&code[from..at], false),
    };
    // `} name;` closes a body: a value of its struct, when no typedef opens it.
    if let Some(s) = stop.filter(|&s| b[s] == b'}' && !param && first_comma.is_none())
        && segment
            .trim_matches(|c: char| c.is_whitespace() || c == '*')
            .is_empty()
    {
        let open = c_innermost_open_brace(b, s)?;
        let head = c_back_to_stop(code, open).0.trim();
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
/// What the line of a C type's declaration says of the type `name` at byte `at` of `code`, a
/// [`c_code`]: `struct name {` and `union name {` open its body, and so does the `typedef struct
/// … {` whose `} name;` names it; `typedef struct TAG name;` and `typedef T name;` make it
/// another name's. `None` for anything else: a forward declaration, an enum, a class.
pub fn c_type_def(code: &str, at: usize) -> Option<CType> {
    let tagged = ends_with_word(&code[..at], &["struct", "union"]);
    let after = at + code[at..].find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))?;
    if tagged {
        let open = after + code[after..].find(|c: char| !c.is_whitespace())?;
        return (code.as_bytes()[open] == b'{').then_some(CType::Body(open));
    }
    let (stmt, stop) = c_back_to_stop(code, at);
    let s = at - stmt.len();
    if stop == Some(b'}')
        && stmt
            .trim_matches(|c: char| c.is_whitespace() || c == '*')
            .is_empty()
    {
        let open = c_innermost_open_brace(code.as_bytes(), s - 1)?;
        let head = c_back_to_stop(code, open).0.trim();
        return (head.starts_with("typedef") && c_struct_head(head).is_some())
            .then_some(CType::Body(open));
    }
    if !stmt.trim_start().starts_with("typedef") {
        return None;
    }
    c_decl_type(code, at).filter(|t| *t != CType::Name(code[at..after].to_owned()))
}
fn ends_with_word(s: &str, words: &[&str]) -> bool {
    let s = s.trim_end();
    words.iter().any(|w| {
        s.strip_suffix(w)
            .is_some_and(|r| !r.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_'))
    })
}
/// The byte of the member `word` of the struct or union whose body opens at byte `open` of
/// `code`, a [`c_code`]: a field declared directly in it, or in a nested body that has no name of
/// its own (`union { int a; };`).
pub fn c_body_field(code: &str, open: usize, word: &str) -> Option<usize> {
    let close = close_or_end(code, open, code.len());
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let b = code.as_bytes();
    code[open + 1..close]
        .match_indices(word)
        .find_map(|(i, _)| {
            let at = open + 1 + i;
            let whole =
                !code[..at].ends_with(is_name) && !code[at + word.len()..].starts_with(is_name);
            if !whole || c_field_owner(code, at, word).is_none() {
                return None;
            }
            let mut o = c_innermost_open_brace(b, at)?;
            while o != open {
                let end = close_or_end(code, o, close);
                let anonymous = c_struct_head(c_back_to_stop(code, o).0)
                    .is_some_and(|n| n.is_empty())
                    && code[end + 1..].trim_start().starts_with(';');
                if !anonymous {
                    return None;
                }
                o = c_innermost_open_brace(b, o)?;
            }
            Some(at)
        })
}
/// The type the value `name` read on `line1` of a C `text` is declared with in the file:
/// its parameter or local ([`c_bindings_at`]), else its declaration at file scope, `} name;`
/// included. `Ok(None)` when the file declares no such value; `Err(())` when it does with a type
/// not read, or with two.
pub fn c_value_type(text: &str, line1: usize, name: &str) -> Result<Option<CType>, ()> {
    let code = c_code(text);
    let starts = line_starts(&code);
    let mut at: Vec<usize> = c_bindings_at(text, line1, name)
        .into_iter()
        .map(|p| starts[p.line1 - 1] + p.byte_col)
        .collect();
    if at.is_empty() {
        at = block_declarations(&code, 0, code.len(), name);
        let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_';
        at.extend(code.match_indices(name).map(|(i, _)| i).filter(|&i| {
            !code[..i].ends_with(is_name)
                && !code[i + name.len()..].starts_with(is_name)
                && code[..i]
                    .trim_end_matches(|c: char| c.is_whitespace() || c == '*')
                    .ends_with('}')
                && c_innermost_open_brace(code.as_bytes(), i).is_none()
                && matches!(c_decl_type(&code, i), Some(CType::Body(_)))
        }));
    }
    // `static struct name { … } name;` names the tag first.
    let mut types = at
        .into_iter()
        .filter(|&a| !ends_with_word(&code[..a], &["struct", "union", "enum"]))
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
    code[from..to]
        .match_indices(name)
        .map(|(i, _)| from + i)
        .filter(|&i| {
            let after = code[i + name.len()..].trim_start();
            !code[..i].ends_with(is_name)
                && !code[i + name.len()..].starts_with(is_name)
                && followed_by.is_none_or(|c| after.starts_with(c))
        })
        .collect()
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CPlace {
    pub line1: usize,
    pub byte_col: usize,
}
pub fn c_place(code: &str, at: usize) -> CPlace {
    let line_start = code[..at].rfind('\n').map_or(0, |i| i + 1);
    CPlace {
        line1: code[..at].matches('\n').count() + 1,
        byte_col: at - line_start,
    }
}
/// The name of the struct or union whose body opens at byte `open` of `code`, `""` for an
/// anonymous one.
pub fn c_body_name(code: &str, open: usize) -> String {
    c_struct_head(c_back_to_stop(code, open).0).unwrap_or_default()
}
pub fn cpp_type_at(code: &str, at: usize, len: usize) -> Option<Vec<String>> {
    static AUTO: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*=\s*(?:new\s+)?((?:::)?\s*\w+(?:\s*::\s*\w+)*)\s*(?:<[^;(){}]*>)?\s*[({]")
            .unwrap()
    });
    let b = code.as_bytes();
    let (mut depth, mut i) = (0usize, at);
    let from = loop {
        if i == 0 {
            break 0;
        }
        i -= 1;
        match b[i] {
            b'>' if i > 0 && b[i - 1] == b'-' => return None,
            b')' | b']' | b'>' => depth += 1,
            b'(' | b',' if depth == 0 => break i + 1,
            b'[' | b'<' if depth == 0 => return None,
            b'(' | b'[' | b'<' => depth -= 1,
            b';' | b'{' | b'}' if depth == 0 => break i + 1,
            b';' | b'{' | b'}' => return None,
            _ => {}
        }
    };
    let segment = ACCESS_LABELS.replace(&code[from..at], "");
    let path = cpp_class_path(&segment)?;
    if path != ["auto"] {
        return Some(path);
    }
    let init = AUTO.captures(&code[at + len..])?;
    let m = init.get(0)?;
    let open = at + len + m.end() - 1;
    let close = close_or_end(code, open, code.len());
    let path = cpp_class_path(&init[1])?;
    let last = path.last()?;
    (close < code.len()
        && code[close + 1..].trim_start().starts_with([';', ','])
        && (m.as_str().contains("new") || last.starts_with(|c: char| c.is_ascii_uppercase())))
    .then_some(path)
}
fn cpp_class_path(written: &str) -> Option<Vec<String>> {
    static DROPPED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\b(?:const|volatile|static|mutable|constexpr|inline|struct|class|typename|extern|thread_local|register|virtual|explicit)\b|\[\[[^\]]*\]\]").unwrap()
    });
    static STD_NAME: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\bstd(?:\s*::\s*\w+)+").unwrap());
    static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[A-Za-z_]\w*").unwrap());
    static SMART: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:::\s*)?(?:std\s*::\s*)?(?:unique_ptr|shared_ptr)\s*<(.*)>$").unwrap()
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
        .skip_while(|p| p.is_empty())
        .collect();
    let named = |p: &String| {
        !p.is_empty()
            && !p.starts_with(|c: char| c.is_ascii_digit())
            && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
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
pub fn cpp_value_type(text: &str, line1: usize, name: &str) -> Result<Option<Vec<String>>, ()> {
    let code = c_code(text);
    let starts = line_starts(&code);
    let at: Vec<usize> = c_bindings_at(text, line1, name)
        .into_iter()
        .map(|p| starts[p.line1 - 1] + p.byte_col)
        .collect();
    let mut types = at.iter().map(|&a| cpp_type_at(&code, a, name.len()));
    let Some(first) = types.next() else {
        return Ok(None);
    };
    match first {
        Some(t) if types.all(|o| o.as_ref() == Some(&t)) => Ok(Some(t)),
        _ => Err(()),
    }
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
            let open = at + code[at..].find([';', '{', '(', '='])?;
            (code.as_bytes()[open] == b'{'
                && c_struct_head(c_back_to_stop(code, open).0).as_deref() == Some(name))
            .then_some(open)
        })
}
pub fn cpp_bases(code: &str, open: usize) -> Vec<Vec<String>> {
    static ACCESS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\b(?:public|private|protected|virtual)\b").unwrap());
    let head = untemplated(c_back_to_stop(code, open).0);
    let list = head
        .match_indices(':')
        .find(|&(i, _)| !head[i + 1..].starts_with(':') && !head[..i].ends_with(':'))
        .map_or("", |(i, _)| &head[i + 1..]);
    list.split(',')
        .filter(|b| !b.trim().is_empty())
        .filter_map(|b| cpp_class_path(&ACCESS.replace_all(b, "")))
        .collect()
}
pub fn cpp_body_members(code: &str, open: usize, word: &str) -> Vec<usize> {
    static REFUSED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:typedef|using|friend|static_assert|return|operator|template\s*<[^>]*>\s*friend)\b").unwrap()
    });
    let close = close_or_end(code, open, code.len());
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let b = code.as_bytes();
    code[open + 1..close]
        .match_indices(word)
        .map(|(i, _)| open + 1 + i)
        .filter(|&at| {
            if code[..at].ends_with(is_name)
                || code[at + word.len()..].starts_with(is_name)
                || c_innermost_open_brace(b, at) != Some(open)
            {
                return false;
            }
            if c_field_owner(code, at, word).is_some() {
                return true;
            }
            let pre = ACCESS_LABELS.replace(c_back_to_stop(code, at).0, "");
            let pre = untemplated(pre.trim());
            !pre.is_empty()
                && !pre.contains(['(', ')', '=', '.', '~'])
                && !pre.ends_with(':')
                && !pre.ends_with("->")
                && !REFUSED.is_match(&pre)
                && code[at + word.len()..].trim_start().starts_with('(')
        })
        .collect()
}
