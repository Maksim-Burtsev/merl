use super::words::{Blocks, word_byte};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tk {
    Word,
    Value,
    Punct,
}

struct Tok<'a> {
    kind: Tk,
    text: &'a str,
    line: usize,
    start: usize,
}

const FOLDS: &[&str] = &[
    "function",
    "macro",
    "module",
    "baremodule",
    "struct",
    "if",
    "try",
    "for",
    "while",
    "let",
    "quote",
    "do",
    "begin",
];

const KEYWORDS: &[&str] = &[
    "function",
    "macro",
    "module",
    "baremodule",
    "struct",
    "mutable",
    "if",
    "elseif",
    "else",
    "try",
    "catch",
    "finally",
    "for",
    "while",
    "let",
    "quote",
    "do",
    "begin",
    "end",
    "return",
    "in",
    "isa",
    "where",
    "local",
    "global",
    "const",
    "export",
    "import",
    "using",
    "outer",
    "abstract",
    "primitive",
];

fn ident(c: u8) -> bool {
    word_byte(c) || c == b'!'
}

fn word_end(b: &[u8], mut k: usize) -> usize {
    while k < b.len() && ident(b[k]) {
        k += 1;
    }
    k
}

fn line_end(b: &[u8], k: usize) -> usize {
    k + b[k..]
        .iter()
        .position(|&c| c == b'\n')
        .unwrap_or(b.len() - k)
}

fn comment(b: &[u8], mut k: usize) -> usize {
    let mut depth = 0usize;
    while k < b.len() {
        if b[k..].starts_with(b"#=") {
            depth += 1;
            k += 2;
        } else if b[k..].starts_with(b"=#") {
            depth -= 1;
            k += 2;
            if depth == 0 {
                return k;
            }
        } else {
            k += 1;
        }
    }
    b.len()
}

fn string(b: &[u8], mut k: usize, q: u8, triple: bool, interp: bool) -> usize {
    while k < b.len() {
        match b[k] {
            b'\\' => k += 2,
            c if c == q && (!triple || b[k..].starts_with(&[q, q, q])) => {
                return k + if triple { 3 } else { 1 };
            }
            b'$' if interp && b.get(k + 1) == Some(&b'(') => k = code(b, k + 2),
            _ => k += 1,
        }
    }
    b.len()
}

fn code(b: &[u8], mut k: usize) -> usize {
    let mut depth = 1;
    while k < b.len() {
        match b[k] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return k + 1;
                }
            }
            q @ (b'"' | b'`') => {
                let triple = b[k..].starts_with(&[q, q, q]);
                k = string(b, k + if triple { 3 } else { 1 }, q, triple, true);
                continue;
            }
            _ => {}
        }
        k += 1;
    }
    b.len()
}

fn tokens(src: &str) -> Vec<Tok<'_>> {
    let b = src.as_bytes();
    let at = |k: usize| b.get(k).copied().unwrap_or(0);
    let (mut out, mut line, mut i) = (Vec::new(), 0, 0);
    while i < b.len() {
        let c = b[i];
        let start = i;
        let before = if i > 0 { b[i - 1] } else { b' ' };
        let kind = match c {
            b'\n' => {
                line += 1;
                i += 1;
                continue;
            }
            b'#' if at(i + 1) == b'=' => {
                let end = comment(b, i);
                line += b[i..end].iter().filter(|&&c| c == b'\n').count();
                i = end;
                continue;
            }
            b'#' => {
                i = line_end(b, i);
                continue;
            }
            b'"' | b'`' => {
                let triple = at(i + 1) == c && at(i + 2) == c;
                let raw = ident(before) && !KEYWORDS.contains(&last_word(src, i));
                i = string(b, i + if triple { 3 } else { 1 }, c, triple, !raw);
                Tk::Value
            }
            b'\'' if !(ident(before) || matches!(before, b')' | b']' | b'}' | b'\'' | b'.')) => {
                let end = line_end(b, i);
                let mut k = i + 1;
                while k < end && b[k] != b'\'' {
                    k += if b[k] == b'\\' { 2 } else { 1 };
                }
                i = if k < end { k + 1 } else { i + 1 };
                Tk::Value
            }
            b'0'..=b'9' => {
                i = word_end(b, i);
                Tk::Value
            }
            b'@' => {
                i = word_end(b, i + 1).max(i + 1);
                Tk::Value
            }
            b':' if ident(at(i + 1))
                && !at(i + 1).is_ascii_digit()
                && !(ident(before) || matches!(before, b')' | b']' | b'}' | b'\'' | b'"')) =>
            {
                i = word_end(b, i + 1);
                Tk::Value
            }
            c if ident(c) => {
                i = word_end(b, i);
                if before == b'.' { Tk::Value } else { Tk::Word }
            }
            c if c.is_ascii_whitespace() => {
                i += 1;
                continue;
            }
            _ => {
                i += 1;
                Tk::Punct
            }
        };
        let text = src.get(start..i.min(b.len())).unwrap_or("");
        out.push(Tok {
            kind,
            text,
            line,
            start,
        });
        line += text.bytes().filter(|&c| c == b'\n').count();
    }
    out
}

fn last_word(src: &str, i: usize) -> &str {
    let b = src.as_bytes();
    let mut k = i;
    while k > 0 && ident(b[k - 1]) {
        k -= 1;
    }
    src.get(k..i).unwrap_or("")
}

const BRACKETS: &[&str] = &["(", "[", "{", "x["];

fn ends_a_term(t: &Tok) -> bool {
    match t.kind {
        Tk::Value => true,
        Tk::Word => !KEYWORDS.contains(&t.text) || t.text == "end",
        Tk::Punct => matches!(t.text, ")" | "]" | "}" | "'"),
    }
}

pub(crate) fn julia(lines: &[String]) -> Blocks {
    let src = lines.join("\n");
    let toks = tokens(&src);
    let (mut nodes, mut funcs) = (Vec::new(), Vec::new());
    let mut stack: Vec<(usize, &str)> = Vec::new();
    for (k, t) in toks.iter().enumerate() {
        let line = t.line;
        let inner = stack.last().map_or("", |f| f.1);
        let bracketed = BRACKETS.contains(&inner);
        match (t.kind, t.text) {
            (Tk::Punct, "[")
                if k > 0
                    && ends_a_term(&toks[k - 1])
                    && toks[k - 1].start + toks[k - 1].text.len() == t.start =>
            {
                stack.push((line, "x["));
            }
            (Tk::Punct, "(" | "[" | "{") => stack.push((line, t.text)),
            (Tk::Punct, ")" | "]" | "}") => {
                if let Some(at) = stack.iter().rposition(|f| BRACKETS.contains(&f.1)) {
                    stack.truncate(at);
                }
            }
            (Tk::Word, "end") if !bracketed => {
                if let Some(&(h, w)) = stack.last() {
                    if FOLDS.contains(&w) {
                        nodes.push((h, line));
                    }
                    if matches!(w, "function" | "macro") {
                        funcs.push((h, line));
                    }
                    stack.pop();
                }
            }
            (Tk::Word, "abstract" | "primitive")
                if toks.get(k + 1).is_some_and(|n| n.text == "type") =>
            {
                stack.push((line, "type"));
            }
            (Tk::Word, w) if FOLDS.contains(&w) => {
                let index = inner == "x[" && w == "begin";
                let comprehension =
                    bracketed && matches!(w, "for" | "if") && k > 0 && ends_a_term(&toks[k - 1]);
                if !index && !comprehension {
                    stack.push((line, w));
                }
            }
            _ => {}
        }
    }
    Blocks::from_nodes(nodes, funcs, lines.len())
}
