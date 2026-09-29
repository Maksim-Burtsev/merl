//! C and C++ read as code: the struct a field sits in (#359), the function a parameter or a local
//! belongs to (#378). Everything here scans [`c_code`], where comments, literals and
//! preprocessor lines are blanks, so a brace in a string or a `#define` body opens nothing.

use regex::Regex;
use std::sync::LazyLock;

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
                    None => quoted(b, i, b'"'),
                }
            }
            // `1'000` is a digit separator, no character literal.
            b'\'' if !(i > 0 && b[i - 1].is_ascii_digit()) => quoted(b, i, b'\''),
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
/// The end of the literal that opens with `q` at `i`: past its closing quote, or at the end of
/// its line when it has none.
fn quoted(b: &[u8], i: usize, q: u8) -> usize {
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 1,
            b'\n' => return j,
            c if c == q => return j + 1,
            _ => {}
        }
        j += 1;
    }
    b.len()
}
/// The byte of the `{` that `at` stands inside, the innermost one not closed before it; `None`
/// at file scope.
fn c_opener(code: &[u8], at: usize) -> Option<usize> {
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
/// What stands before byte `at` back to the previous `;`, `{` or `}`, and that byte (`0` at the
/// start of the file): the head of a body when `at` is its `{`, a statement's start otherwise.
fn c_back_to_stop(code: &str, at: usize) -> (&str, u8) {
    let b = code.as_bytes();
    match b[..at]
        .iter()
        .rposition(|c| matches!(c, b';' | b'{' | b'}'))
    {
        Some(s) => (&code[s + 1..at], b[s]),
        None => (&code[..at], 0),
    }
}
/// `s` with each `<…>` of a template, innermost first, taken out.
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
    let head = ATTR.replace_all(head.trim(), "");
    let head = untemplated(head.trim());
    HEAD.captures(head.trim_start())
        .map(|c| c.get(1).map_or(String::new(), |m| m.as_str().to_owned()))
}
/// The struct, union or class whose body byte `at` of `code` stands directly in, and its name
/// (the nearest named one out, for a nested anonymous `union { … } u;`, `""` for none).
fn c_struct_around(code: &str, at: usize) -> Option<String> {
    let mut open = c_opener(code.as_bytes(), at)?;
    let mut name = c_struct_head(c_back_to_stop(code, open).0)?;
    // An anonymous body's own name is its outer one's.
    while name.is_empty() {
        let Some(outer) = c_opener(code.as_bytes(), open) else {
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
/// function, a nested type, `typedef`, `using`, `friend` and `static_assert` are none. Gives the
/// name of the type it is a member of.
fn c_field_at(code: &str, at: usize, word: &str) -> Option<String> {
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
        // `} name;` closes a nested body; a bare `name;` is an expression.
        stop == b'}' && AFTER.is_match(after) && !after.trim_start().starts_with(['{', '='])
    } else if let Some(m) = FN_POINTER.find(pre) {
        // `T (*name)(…);`
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
/// Of the 1-based `hits` of a C or C++ `text`, the lines that declare `word` as a data member
/// ([`c_field_at`]), each with the name of the type it belongs to.
pub fn c_field_rows(text: &str, hits: &[usize], word: &str) -> Vec<(usize, String)> {
    let code = c_code(text);
    let starts: Vec<usize> = std::iter::once(0)
        .chain(code.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let mut rows = Vec::new();
    for &line in hits {
        let Some(&from) = line.checked_sub(1).and_then(|k| starts.get(k)) else {
            continue;
        };
        let to = starts.get(line).map_or(code.len(), |&s| s - 1);
        let found = code[from..to].match_indices(word).find_map(|(i, _)| {
            let at = from + i;
            let whole = !code[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_')
                && !code[at + word.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_');
            whole.then(|| c_field_at(&code, at, word)).flatten()
        });
        if let Some(owner) = found {
            rows.push((line, owner));
        }
    }
    rows
}
/// Whether the word at byte `start` of 1-based `line` of a C++ `text`, followed by `(` or `{`,
/// names a member in a constructor's initializer list: `X::X(…) : a_(x), b_{y} {`, `X(…) : a_(x)
/// {` in X's body, and its continuation lines. Gives the constructor's class, `X`.
pub fn c_initialized_member(text: &str, line: usize, start: usize) -> Option<String> {
    static CTOR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^\s*(?:(?:public|private|protected)\s*:\s*)*(?:[\w~]+\s+)*(?:\w+::)*?(?:(\w+)::)?(\w+)\s*\(\)\s*(?:(?:const|noexcept|override)\b\s*(?:\(\))?\s*)*:\s*(?:(?:\w+::)*\w+\s*(?:\(\)|\{\})\s*,\s*)*$",
        )
        .unwrap()
    });
    let code = c_code(text);
    let at = code
        .split('\n')
        .take(line.checked_sub(1)?)
        .map(|l| l.len() + 1)
        .sum::<usize>()
        + start;
    let b = code.as_bytes();
    // Back over the list to the constructor's name, each balanced `(…)` and `{…}` read as empty.
    let mut s = String::new();
    let (mut depth, mut i) = (0usize, at);
    while i > 0 {
        i -= 1;
        let c = b[i];
        match c {
            // A body closed above ends the list; `b_{y},` is an item of it.
            b'}' if depth == 0 && !code[i + 1..].trim_start().starts_with(',') => break,
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
    if !code[at..]
        .trim_start_matches(|c: char| c.is_alphanumeric() || c == '_')
        .trim_start()
        .starts_with(['(', '{'])
    {
        return None;
    }
    let s = untemplated(&s);
    let caps = CTOR.captures(&s)?;
    let name = caps.get(2)?.as_str();
    // `X::X(`: the class is its own name; an in-class `X(` is named as the class it sits in.
    match caps.get(1) {
        Some(owner) if owner.as_str() != name => None,
        _ => Some(name.to_owned()),
    }
}
