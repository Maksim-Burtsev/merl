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
    C,
    Rust,
    Php,
    Julia,
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
    Php,
    Paren,
    No,
}

struct Text {
    close: Vec<u8>,
    escape: Escape,
    hole: Hole,
    multi: bool,
    label: bool,
}

enum Frame {
    Code(i32),
    Text(Text),
    Path,
    Tag,
    Children,
    Html,
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
        Kind::C => Lang::C,
        _ => return false,
    };
    lex(lang, text.as_bytes(), at, &mut Vec::new()).is_some()
}

pub fn in_string(kind: Kind, text: &str, at: usize) -> bool {
    match kind {
        Kind::Rust => lex(Lang::Rust, text.as_bytes(), at, &mut Vec::new()).is_some(),
        Kind::Php => match lex(Lang::Php, text.as_bytes(), at, &mut Vec::new()) {
            Some((t, from)) if from > at || t.close == b"`" => false,
            Some((t, _)) if t.label => true,
            Some((_, from)) => php_prose(text, from - 1) && !php_qualified_name(text, at),
            None => false,
        },
        _ => false,
    }
}

fn php_prose(text: &str, from: usize) -> bool {
    let b = text.as_bytes();
    let Some(&q) = b.get(from) else {
        return false;
    };
    let mut i = from + 1;
    while let Some(&c) = b.get(i) {
        if c == q || c == b'\n' {
            return false;
        }
        if c == b' ' || c == b'\t' {
            return true;
        }
        i += if c == b'\\' { 2 } else { 1 };
    }
    false
}

fn php_qualified_name(text: &str, at: usize) -> bool {
    let b = text.as_bytes();
    let name = |c: &u8| ident(*c) || *c == b'\\';
    let (head, tail) = (b.get(..at).unwrap_or(b), b.get(at..).unwrap_or_default());
    let head = head.iter().rev().take_while(|c| name(c));
    let tail = tail.iter().take_while(|c| name(c));
    head.chain(tail).any(|&c| c == b'\\')
}

pub(super) fn holes(kind: Kind, line: &[u8]) -> Vec<(usize, usize)> {
    let lang = match kind {
        Kind::Julia => Lang::Julia,
        _ => Lang::CSharp,
    };
    let mut holes = Vec::new();
    lex(lang, line, line.len(), &mut holes);
    holes
}

fn lex(lang: Lang, b: &[u8], at: usize, holes: &mut Vec<(usize, usize)>) -> Option<(Text, usize)> {
    let mut stack: Vec<(Frame, usize)> = Vec::new();
    if lang == Lang::Php {
        stack.push((Frame::Html, 0));
    }
    let mut i = 0;
    while i < at.min(b.len()) {
        let (n, change) = match stack.last() {
            Some((Frame::Text(t), _)) => in_text(t, &b[i..]),
            Some((Frame::Path, _)) => in_path(lang, &b[i..]),
            Some((Frame::Tag, _)) => in_tag(&b[i..]),
            Some((Frame::Children, _)) => in_children(&b[i..]),
            Some((Frame::Html, _)) if b[i..].starts_with(b"<?") => (2, Change::Pop),
            Some((Frame::Html, _)) => (1, Change::Keep),
            Some((Frame::Code(depth), _))
                if lang == Lang::CSharp
                    && stack.len() > 1
                    && matches!(stack[stack.len() - 2], (Frame::Text(_), _))
                    && *depth == 0
                    && format_clause(b, i) =>
            {
                let n = b[i..].iter().position(|&c| matches!(c, b'}' | b'\n'));
                let n = n.unwrap_or(b.len() - i);
                if i + n > at {
                    stack.pop();
                    break;
                }
                (n, Change::Keep)
            }
            Some((Frame::Code(0), _))
                if lang == Lang::Julia
                    && b[i] == b')'
                    && stack.len() > 1
                    && matches!(stack[stack.len() - 2], (Frame::Text(_), _)) =>
            {
                (1, Change::Pop)
            }
            Some((Frame::Code(_), _)) | None => in_code(lang, b, i),
        };
        if let (Change::Keep, Some((Frame::Code(depth), _))) = (&change, stack.last_mut()) {
            match b[i] {
                b'(' | b'[' => *depth += 1,
                b')' | b']' => *depth -= 1,
                _ => {}
            }
        }
        match change {
            Change::Keep => {}
            Change::Pop => {
                if let Some((Frame::Code(_), from)) = stack.pop()
                    && matches!(stack.last(), Some((Frame::Text(_), _)))
                {
                    holes.push((from, i));
                }
            }
            Change::Push(f) => stack.push((f, i + n)),
            Change::Swap(f) => {
                stack.pop();
                stack.push((f, i + n));
            }
        }
        i += n;
    }
    match stack.pop() {
        Some((Frame::Text(t), from)) => Some((t, from)),
        _ => None,
    }
}

fn in_text(t: &Text, rest: &[u8]) -> (usize, Change) {
    if t.escape == Escape::Backslash
        && rest[0] == b'\\'
        && !(matches!(t.hole, Hole::Brace(_)) && rest.get(1) == Some(&b'{'))
        && !(t.label && rest.get(1) == Some(&b'\n'))
    {
        return (2, Change::Keep);
    }
    if t.label {
        if rest[0] == b'\n' {
            let indent = rest[1..].iter().take_while(|&&c| c == b' ' || c == b'\t');
            let n = 1 + indent.count();
            if rest[n..].starts_with(&t.close)
                && rest.get(n + t.close.len()).is_none_or(|&c| !ident(c))
            {
                return (n + t.close.len(), Change::Pop);
            }
        }
    } else if rest.starts_with(&t.close) {
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
        Hole::Php if rest.starts_with(b"{$") => Some(1),
        Hole::Dollar | Hole::Path | Hole::Php if rest.starts_with(b"${") => Some(2),
        Hole::Paren if rest.starts_with(b"$(") => Some(2),
        Hole::Path | Hole::Php | Hole::Paren
            if rest[0] == b'$' && rest.get(1).is_some_and(|&c| name_start(c)) =>
        {
            return (1, Change::Push(Frame::Path));
        }
        Hole::Dollar | Hole::Path | Hole::Php | Hole::Paren | Hole::No => None,
    };
    if let Some(n) = hole {
        return (n, Change::Push(Frame::Code(0)));
    }
    if rest[0] == b'\n' && !t.multi {
        return (1, Change::Pop);
    }
    (1, Change::Keep)
}

fn in_path(lang: Lang, rest: &[u8]) -> (usize, Change) {
    let dot: &[u8] = match rest {
        [b'?', b'-', b'>', ..] if lang == Lang::Php => b"?->",
        _ if lang == Lang::Php => b"->",
        _ => b".",
    };
    if ident(rest[0]) {
        (1, Change::Keep)
    } else if lang != Lang::Julia
        && rest.starts_with(dot)
        && rest.get(dot.len()).is_some_and(|&c| name_start(c))
    {
        (dot.len(), Change::Keep)
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
                label: false,
            })),
        ),
        b'{' => (1, Change::Push(Frame::Code(0))),
        b'>' => (1, Change::Swap(Frame::Children)),
        b'/' if rest.get(1) == Some(&b'>') => (2, Change::Pop),
        _ => (1, Change::Keep),
    }
}

fn in_children(rest: &[u8]) -> (usize, Change) {
    match rest {
        [b'{', ..] => (1, Change::Push(Frame::Code(0))),
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
    let line_comment = match lang {
        Lang::Python => rest[0] == b'#',
        Lang::Julia => rest[0] == b'#' && rest.get(1) != Some(&b'='),
        Lang::Php => rest.starts_with(b"//") || (rest[0] == b'#' && rest.get(1) != Some(&b'[')),
        _ => rest.starts_with(b"//"),
    };
    if line_comment {
        return (line_end(lang, rest), Change::Keep);
    }
    if lang == Lang::Julia && rest.starts_with(b"#=") {
        return (block_comment(rest, (b"#=", b"=#"), true), Change::Keep);
    }
    if !matches!(lang, Lang::Python | Lang::Julia) && rest.starts_with(b"/*") {
        let nests = matches!(lang, Lang::Kotlin | Lang::Scala | Lang::Dart | Lang::Rust);
        return (block_comment(rest, (b"/*", b"*/"), nests), Change::Keep);
    }
    if lang == Lang::Php && rest.starts_with(b"?>") {
        return (2, Change::Push(Frame::Html));
    }
    if let Some((n, t)) = (lang == Lang::Php).then(|| heredoc(rest)).flatten() {
        return (n, Change::Push(Frame::Text(t)));
    }
    if lang == Lang::Rust && rest[0] == b'\'' {
        return (char_literal(rest), Change::Keep);
    }
    if lang == Lang::Julia && rest[0] == b'\'' {
        let adjoint =
            i > 0 && (ident(b[i - 1]) || b"!)]}'.".contains(&b[i - 1]) || b[i - 1] >= 0x80);
        return (if adjoint { 1 } else { char_literal(rest) }, Change::Keep);
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
        b'{' => return (1, Change::Push(Frame::Code(0))),
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
        label: false,
    };
    let quoted = |hole, escape| {
        let n = if triple { 3 } else { 1 };
        Some((
            n,
            text(&rest[..n], escape, hole, triple || lang == Lang::Julia),
        ))
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
        (Lang::C, b'"') if matches!(prefix, b"R" | b"LR" | b"uR" | b"UR" | b"u8R") => {
            let open = rest.iter().take(18).position(|&c| c == b'(')?;
            let close = [b")", &rest[1..open], b"\""].concat();
            Some((open + 1, text(&close, Escape::No, Hole::No, true)))
        }
        (Lang::C, b'\'') if in_number(&b[..i]) => None,
        (Lang::Rust, b'"') => {
            let hashes = b[..i].iter().rev().take_while(|&&c| c == b'#').count();
            let head = &b[..i - hashes];
            let raw = &head[head.iter().rposition(|&c| !ident(c)).map_or(0, |p| p + 1)..];
            if matches!(raw, b"r" | b"br" | b"cr") {
                let close = [b"\"", &b[i - hashes..i]].concat();
                Some((1, text(&close, Escape::No, Hole::No, true)))
            } else {
                Some((1, text(b"\"", Escape::Backslash, Hole::No, true)))
            }
        }
        (Lang::Php, b'"' | b'`') => Some((1, text(&rest[..1], Escape::Backslash, Hole::Php, true))),
        (Lang::Php, b'\'') => Some((1, text(b"'", Escape::Backslash, Hole::No, true))),
        (Lang::Julia, b'"') if prefix.is_empty() => quoted(Hole::Paren, Escape::Backslash),
        (Lang::Julia, b'"') => quoted(Hole::No, Escape::Backslash),
        (Lang::Julia, _) => None,
        (Lang::Java | Lang::Kotlin, _) if triple => quoted(Hole::No, Escape::Backslash),
        (_, b'"' | b'\'') => Some((1, text(&rest[..1], Escape::Backslash, Hole::No, false))),
        _ => None,
    }
}

fn format_clause(b: &[u8], i: usize) -> bool {
    b[i] == b':' && b.get(i + 1) != Some(&b':') && b[i - 1] != b':'
}

fn line_end(lang: Lang, rest: &[u8]) -> usize {
    (0..rest.len())
        .find(|&n| rest[n] == b'\n' || (lang == Lang::Php && rest[n..].starts_with(b"?>")))
        .unwrap_or(rest.len())
}

fn heredoc(rest: &[u8]) -> Option<(usize, Text)> {
    let head = rest.strip_prefix(b"<<<")?;
    let quote = head.first().copied().filter(|&c| c == b'"' || c == b'\'');
    let from = 3 + usize::from(quote.is_some());
    let len = rest[from..].iter().take_while(|&&c| ident(c)).count();
    if !rest.get(from).is_some_and(|&c| name_start(c)) {
        return None;
    }
    let nowdoc = quote == Some(b'\'');
    let text = Text {
        close: rest[from..from + len].to_vec(),
        escape: if nowdoc {
            Escape::No
        } else {
            Escape::Backslash
        },
        hole: if nowdoc { Hole::No } else { Hole::Php },
        multi: true,
        label: true,
    };
    Some((from + len + usize::from(quote.is_some()), text))
}

fn char_literal(rest: &[u8]) -> usize {
    let end = match rest.get(1) {
        Some(b'\\') => (rest.get(3..).unwrap_or_default().iter().take(9))
            .position(|&c| c == b'\'')
            .map(|n| 3 + n),
        Some(&c) => Some(1 + (c.leading_ones() as usize).max(1)),
        None => None,
    };
    end.filter(|&e| rest.get(e) == Some(&b'\''))
        .map_or(1, |e| e + 1)
}

fn block_comment(b: &[u8], (open, close): (&[u8], &[u8]), nests: bool) -> usize {
    let mut depth = 0;
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(open) && (nests || depth == 0) {
            depth += 1;
            i += 2;
        } else if b[i..].starts_with(close) {
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

fn in_number(before: &[u8]) -> bool {
    let start = before
        .iter()
        .rposition(|&c| !ident(c) && c != b'.' && c != b'\'')
        .map_or(0, |p| p + 1);
    matches!(before[start..], [b'.', c, ..] | [c, ..] if c.is_ascii_digit())
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

pub fn python_path_string(lead: &str, tail: &str) -> bool {
    let part = |c: char| c == '_' || c == '.' || c.is_alphanumeric();
    let open = lead.trim_end_matches(part);
    let close = tail.trim_start_matches(part);
    let named = |s: &str| s.starts_with(|c: char| c == '_' || c.is_alphabetic());
    let after = tail.strip_suffix(close).unwrap_or(tail);
    let before = lead.strip_prefix(open).and_then(|b| b.strip_suffix('.'));
    open.chars()
        .last()
        .is_some_and(|q| (q == '"' || q == '\'') && close.starts_with(q))
        && before.is_some_and(|b| b.split('.').all(named))
        && (after.is_empty()
            || after
                .strip_prefix('.')
                .is_some_and(|a| a.split('.').all(named)))
}
