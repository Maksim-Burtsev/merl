use super::{Kind, groovy, kotlin, scala, swift_in_string};
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lang {
    Python,
    Ts,
    Java,
    Kotlin,
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
    Brace,
    Dollar,
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
}

enum Change {
    Keep,
    Pop,
    Push(Frame),
}

pub fn in_string_text(kind: Kind, path: &Path, text: &str, at: usize) -> bool {
    let lang = match kind {
        Kind::Swift => return swift_in_string(text, at),
        Kind::Python => Lang::Python,
        Kind::TsJs => Lang::Ts,
        Kind::Jvm if scala(path) => Lang::Scala,
        Kind::Jvm if kotlin(path) || groovy(path) => Lang::Kotlin,
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
            _ => in_code(lang, b, i),
        };
        match change {
            Change::Keep => {}
            Change::Pop => {
                stack.pop();
            }
            Change::Push(f) => stack.push(f),
        }
        i += n.max(1);
    }
    matches!(stack.last(), Some(Frame::Text(_)))
}

fn in_text(t: &Text, rest: &[u8]) -> (usize, Change) {
    if t.escape == Escape::Backslash
        && rest[0] == b'\\'
        && !(t.hole == Hole::Brace && rest.get(1) == Some(&b'{'))
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
        Hole::Brace if rest.starts_with(b"{{") => return (2, Change::Keep),
        Hole::Brace => rest.starts_with(b"{").then_some(1),
        Hole::Dollar => rest.starts_with(b"${").then_some(2),
        Hole::No => None,
    };
    if let Some(n) = hole {
        return (n, Change::Push(Frame::Code));
    }
    if rest[0] == b'\n' && !t.multi {
        return (1, Change::Pop);
    }
    (1, Change::Keep)
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
            quoted(if f { Hole::Brace } else { Hole::No }, Escape::Backslash)
        }
        (Lang::Dart, b'"' | b'\'') => match prefix {
            b"r" => quoted(Hole::No, Escape::No),
            _ => quoted(Hole::Dollar, Escape::Backslash),
        },
        (Lang::Kotlin, b'"') if triple => quoted(Hole::Dollar, Escape::No),
        (Lang::Kotlin, b'"') => quoted(Hole::Dollar, Escape::Backslash),
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
            let hole = if marks.contains(&b'$') {
                Hole::Brace
            } else {
                Hole::No
            };
            if triple {
                quoted(hole, Escape::No)
            } else if marks.contains(&b'@') {
                Some((1, text(b"\"", Escape::Doubled, hole, true)))
            } else {
                quoted(hole, Escape::Backslash)
            }
        }
        (Lang::Ts, b'`') => Some((1, text(b"`", Escape::Backslash, Hole::Dollar, true))),
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
