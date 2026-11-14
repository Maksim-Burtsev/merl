use super::{Kind, groovy, kotlin, scala, swift_in_string};
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lang {
    Python,
    Ts { jsx: bool },
    Java,
    Kotlin,
    Groovy,
    Scala,
    CSharp,
    Go,
    Dart,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Escape {
    Backslash,
    Doubled,
    No,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Hole {
    Brace(usize),
    Dollar,
    Path,
    No,
}

struct Text {
    close: Vec<u8>,
    escape: Escape,
    hole: Hole,
    multi: bool,
}

enum Frame {
    Code,
    Text(Text),
    Path,
    Tag,
    Children,
}

enum Change {
    Keep,
    Pop,
    Push(Frame),
    Swap(Frame),
}

pub fn in_string_text(kind: Kind, path: &Path, text: &str, at: usize) -> bool {
    let lang = match kind {
        Kind::Swift => return swift_in_string(text, at),
        Kind::Python => Lang::Python,
        Kind::TsJs => Lang::Ts {
            jsx: !path
                .extension()
                .is_some_and(|e| matches!(e.to_str(), Some("ts" | "mts" | "cts"))),
        },
        Kind::Jvm if scala(path) => Lang::Scala,
        Kind::Jvm if kotlin(path) => Lang::Kotlin,
        Kind::Jvm if groovy(path) => Lang::Groovy,
        Kind::Jvm => Lang::Java,
        Kind::CSharp => Lang::CSharp,
        Kind::Go => Lang::Go,
        Kind::Dart => Lang::Dart,
        _ => return false,
    };
    let b = text.as_bytes();
    let mut stack: Vec<Frame> = Vec::new();
    let mut i = 0;
    while i < at.min(b.len()) {
        let (n, change) = match stack.last() {
            Some(Frame::Text(t)) => in_text(t, &b[i..]),
            Some(Frame::Path) => in_path(&b[i..]),
            Some(Frame::Tag) => in_tag(&b[i..]),
            Some(Frame::Children) => in_children(&b[i..]),
            Some(Frame::Code) | None => in_code(lang, b, i),
        };
        match change {
            Change::Keep => {}
            Change::Pop => {
                stack.pop();
            }
            Change::Push(f) => stack.push(f),
            Change::Swap(f) => {
                stack.pop();
                stack.push(f);
            }
        }
        i += n;
    }
    matches!(stack.last(), Some(Frame::Text(_)))
}

fn in_text(t: &Text, rest: &[u8]) -> (usize, Change) {
    if t.escape == Escape::Backslash
        && rest[0] == b'\\'
        && !(matches!(t.hole, Hole::Brace(_)) && rest.get(1) == Some(&b'{'))
    {
        return (2, Change::Keep);
    }
    if rest.starts_with(&t.close) {
        let n = t.close.len();
        if t.escape == Escape::Doubled && rest[n..].starts_with(&t.close) {
            return (2 * n, Change::Keep);
        }
        return (n, Change::Pop);
    }
    let hole = match t.hole {
        Hole::Brace(n) => {
            let k = rest.iter().take_while(|&&c| c == b'{').count();
            match k {
                0 => None,
                _ if n == 1 && k >= 2 => return (2, Change::Keep),
                _ if k < n => return (k, Change::Keep),
                _ => Some(k),
            }
        }
        Hole::Dollar | Hole::Path if rest.starts_with(b"${") => Some(2),
        Hole::Path if rest[0] == b'$' && rest.get(1).is_some_and(|&c| name_start(c)) => {
            return (1, Change::Push(Frame::Path));
        }
        Hole::Dollar | Hole::Path | Hole::No => None,
    };
    if let Some(n) = hole {
        return (n, Change::Push(Frame::Code));
    }
    if rest[0] == b'\n' && !t.multi {
        return (1, Change::Pop);
    }
    (1, Change::Keep)
}

fn in_path(rest: &[u8]) -> (usize, Change) {
    if ident(rest[0]) || (rest[0] == b'.' && rest.get(1).is_some_and(|&c| name_start(c))) {
        (1, Change::Keep)
    } else {
        (0, Change::Pop)
    }
}

fn in_tag(rest: &[u8]) -> (usize, Change) {
    match rest[0] {
        q @ (b'"' | b'\'') => (
            1,
            Change::Push(Frame::Text(Text {
                close: vec![q],
                escape: Escape::No,
                hole: Hole::No,
                multi: true,
            })),
        ),
        b'{' => (1, Change::Push(Frame::Code)),
        b'>' => (1, Change::Swap(Frame::Children)),
        b'/' if rest.get(1) == Some(&b'>') => (2, Change::Pop),
        _ => (1, Change::Keep),
    }
}

fn in_children(rest: &[u8]) -> (usize, Change) {
    match rest {
        [b'{', ..] => (1, Change::Push(Frame::Code)),
        [b'<', b'/', ..] => (
            rest.iter()
                .position(|&c| c == b'>')
                .map_or(rest.len(), |p| p + 1),
            Change::Pop,
        ),
        [b'<', b'>', ..] => (2, Change::Push(Frame::Children)),
        [b'<', c, ..] if name_start(*c) => (1, Change::Push(Frame::Tag)),
        _ => (1, Change::Keep),
    }
}

fn in_code(lang: Lang, b: &[u8], i: usize) -> (usize, Change) {
    let rest = &b[i..];
    let hash_comments = lang == Lang::Python;
    if rest.starts_with(if hash_comments { b"#" } else { b"//" }) {
        return (line_end(rest), Change::Keep);
    }
    if !hash_comments && rest.starts_with(b"/*") {
        let nests = matches!(lang, Lang::Kotlin | Lang::Scala | Lang::Dart);
        return (block_comment(rest, nests), Change::Keep);
    }
    let slashy = matches!(lang, Lang::Ts { .. } | Lang::Groovy);
    if let Some(n) = slashy.then(|| regex_literal(b, i)).flatten() {
        return (n, Change::Keep);
    }
    if lang == Lang::Groovy && rest.starts_with(b"$/") {
        let end = rest.windows(2).skip(2).position(|w| w == b"/$");
        return (end.map_or(rest.len(), |p| p + 4), Change::Keep);
    }
    if lang == (Lang::Ts { jsx: true })
        && rest[0] == b'<'
        && (i == 0 || b[i - 1] != b'<')
        && after_operator(b, i)
    {
        match rest.get(1) {
            Some(b'>') => return (2, Change::Push(Frame::Children)),
            Some(&c) if name_start(c) => return (1, Change::Push(Frame::Tag)),
            _ => {}
        }
    }
    match rest[0] {
        b'{' => return (1, Change::Push(Frame::Code)),
        b'}' => return (1, Change::Pop),
        _ => {}
    }
    match opens(lang, b, i) {
        Some((n, t)) => (n, Change::Push(Frame::Text(t))),
        None => (1, Change::Keep),
    }
}

fn opens(lang: Lang, b: &[u8], i: usize) -> Option<(usize, Text)> {
    let rest = &b[i..];
    let q = rest[0];
    if !matches!(q, b'"' | b'\'' | b'`') {
        return None;
    }
    let triple = q != b'`' && rest.len() >= 3 && rest[..3].iter().all(|&c| c == q);
    let prefix = &b[..i][b[..i].iter().rposition(|&c| !ident(c)).map_or(0, |p| p + 1)..];
    let text = |close: &[u8], escape, hole, multi| Text {
        close: close.to_vec(),
        escape,
        hole,
        multi,
    };
    let quoted = |hole, escape| {
        let n = if triple { 3 } else { 1 };
        Some((n, text(&rest[..n], escape, hole, triple)))
    };
    match (lang, q) {
        (Lang::Python, b'"' | b'\'') => {
            let f = prefix.len() <= 2
                && prefix.iter().all(|c| b"rRbBuUfFtT".contains(c))
                && prefix.iter().any(|c| b"fFtT".contains(c));
            quoted(if f { Hole::Brace(1) } else { Hole::No }, Escape::Backslash)
        }
        (Lang::Dart, b'"' | b'\'') => match prefix {
            b"r" => quoted(Hole::No, Escape::No),
            _ => quoted(Hole::Dollar, Escape::Backslash),
        },
        (Lang::Kotlin, b'"') if triple => quoted(Hole::Dollar, Escape::No),
        (Lang::Kotlin, b'"') => quoted(Hole::Dollar, Escape::Backslash),
        (Lang::Groovy, b'"') => quoted(Hole::Path, Escape::Backslash),
        (Lang::Groovy, b'\'') => quoted(Hole::No, Escape::Backslash),
        (Lang::Scala, b'"') => {
            let hole = if prefix.is_empty() {
                Hole::No
            } else {
                Hole::Dollar
            };
            quoted(
                hole,
                if triple {
                    Escape::No
                } else {
                    Escape::Backslash
                },
            )
        }
        (Lang::Scala, b'\'') if rest.get(2) != Some(&b'\'') && rest.get(1) != Some(&b'\\') => None,
        (Lang::CSharp, b'"') => {
            let marks = &b[..i][b[..i]
                .iter()
                .rposition(|&c| c != b'$' && c != b'@')
                .map_or(0, |p| p + 1)..];
            let dollars: usize = marks.split(|&c| c == b'@').map(<[u8]>::len).sum();
            let hole = if dollars > 0 {
                Hole::Brace(dollars)
            } else {
                Hole::No
            };
            if marks.contains(&b'@') {
                Some((1, text(b"\"", Escape::Doubled, hole, true)))
            } else if triple {
                let n = rest.iter().take_while(|&&c| c == b'"').count();
                Some((n, text(&rest[..n], Escape::No, hole, true)))
            } else {
                quoted(hole, Escape::Backslash)
            }
        }
        (Lang::Ts { .. }, b'`') => Some((1, text(b"`", Escape::Backslash, Hole::Dollar, true))),
        (Lang::Go, b'`') => Some((1, text(b"`", Escape::No, Hole::No, true))),
        (Lang::Java | Lang::Kotlin, _) if triple => quoted(Hole::No, Escape::Backslash),
        (_, b'"' | b'\'') => Some((1, text(&rest[..1], Escape::Backslash, Hole::No, false))),
        _ => None,
    }
}

fn line_end(rest: &[u8]) -> usize {
    rest.iter().position(|&c| c == b'\n').unwrap_or(rest.len())
}

fn block_comment(b: &[u8], nests: bool) -> usize {
    let mut depth = 0;
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(b"/*") && (nests || depth == 0) {
            depth += 1;
            i += 2;
        } else if b[i..].starts_with(b"*/") {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            i += 1;
        }
    }
    i
}

fn ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn name_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn after_operator(b: &[u8], i: usize) -> bool {
    let Some(end) = b[..i].iter().rposition(|c| !c.is_ascii_whitespace()) else {
        return true;
    };
    let c = b[end];
    if ident(c) {
        let start = b[..end]
            .iter()
            .rposition(|&c| !ident(c))
            .map_or(0, |p| p + 1);
        return matches!(
            &b[start..=end],
            b"return"
                | b"typeof"
                | b"instanceof"
                | b"in"
                | b"of"
                | b"new"
                | b"delete"
                | b"void"
                | b"throw"
                | b"case"
                | b"do"
                | b"else"
                | b"yield"
                | b"await"
        );
    }
    b"(,=:[!&|?{};+-*%<>~^".contains(&c) && !(b"+-".contains(&c) && end > 0 && b[end - 1] == c)
}

fn regex_literal(b: &[u8], i: usize) -> Option<usize> {
    if b[i] != b'/' || !after_operator(b, i) {
        return None;
    }
    let mut class = false;
    let mut j = i + 1;
    while let Some(&c) = b.get(j) {
        match c {
            b'\n' => return None,
            b'\\' => j += 1,
            b'[' => class = true,
            b']' => class = false,
            b'/' if !class => return Some(j + 1 - i),
            _ => {}
        }
        j += 1;
    }
    None
}
