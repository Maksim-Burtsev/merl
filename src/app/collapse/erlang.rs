use super::words::{Blocks, word_byte};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tk {
    Atom,
    Var,
    Value,
    Macro,
    Punct,
}

struct Tok<'a> {
    kind: Tk,
    text: &'a str,
    line: usize,
}

const OPENERS: &[&str] = &["begin", "case", "if", "receive", "try", "maybe"];

const OPERATORS: &[&str] = &[
    "after", "and", "andalso", "band", "begin", "bnot", "bor", "bsl", "bsr", "bxor", "case",
    "catch", "div", "else", "fun", "if", "maybe", "not", "of", "or", "orelse", "receive", "rem",
    "try", "when", "xor",
];

const PUNCTS: &[&str] = &["->", "||", ">>", "<<", "#{"];

fn tokens(src: &str) -> Vec<Tok<'_>> {
    let b = src.as_bytes();
    let at = |k: usize| b.get(k).copied().unwrap_or(0);
    let (mut out, mut line, mut i) = (Vec::new(), 0, 0);
    while i < b.len() {
        let c = b[i];
        let start = i;
        let kind = match c {
            b'\n' => {
                line += 1;
                i += 1;
                continue;
            }
            b'%' => {
                i += b[i..]
                    .iter()
                    .position(|&c| c == b'\n')
                    .unwrap_or(b.len() - i);
                continue;
            }
            b'"' if at(i + 1) == b'"' && at(i + 2) == b'"' => {
                i = triple(b, i + 3);
                Tk::Value
            }
            b'"' | b'\'' => {
                i = quoted(b, i + 1, c);
                Tk::Value
            }
            b'$' => {
                i += match (at(i + 1), at(i + 2)) {
                    (b'\\', b'x') if at(i + 3) == b'{' => {
                        4 + b[i + 3..].iter().position(|&c| c == b'}').unwrap_or(0)
                    }
                    (b'\\', _) => 3,
                    _ => 2,
                };
                Tk::Value
            }
            b'?' => {
                i += 1 + usize::from(at(i + 1) == b'?');
                if at(i) == b'\'' {
                    i = quoted(b, i + 1, b'\'');
                } else {
                    i = word_end(b, i);
                }
                Tk::Macro
            }
            b'0'..=b'9' => {
                i = number(b, i);
                Tk::Value
            }
            c if word_byte(c) => {
                i = word_end(b, i);
                if c.is_ascii_uppercase() || c == b'_' {
                    Tk::Var
                } else {
                    Tk::Atom
                }
            }
            b'.' if matches!(at(i + 1), 0 | b' ' | b'\t' | b'\r' | b'\n' | b'%') => {
                i += 1;
                Tk::Punct
            }
            b'.' => {
                i += b[i..].iter().take_while(|&&c| c == b'.').count();
                continue;
            }
            c if c.is_ascii_whitespace() => {
                i += 1;
                continue;
            }
            _ => {
                let rest = &b[i..];
                i += PUNCTS
                    .iter()
                    .find(|p| rest.starts_with(p.as_bytes()))
                    .map_or(1, |p| p.len());
                Tk::Punct
            }
        };
        let text = src.get(start..i.min(b.len())).unwrap_or("");
        out.push(Tok { kind, text, line });
        line += text.bytes().filter(|&c| c == b'\n').count();
    }
    out
}

fn word_end(b: &[u8], mut k: usize) -> usize {
    while k < b.len() && (word_byte(b[k]) || b[k] == b'@') {
        k += 1;
    }
    k
}

fn number(b: &[u8], mut k: usize) -> usize {
    k = word_end(b, k);
    if b.get(k) == Some(&b'#') {
        k = word_end(b, k + 1);
    }
    if b.get(k) == Some(&b'.') && b.get(k + 1).is_some_and(u8::is_ascii_digit) {
        k = word_end(b, k + 1);
        if b.get(k - 1).is_some_and(|c| matches!(c, b'e' | b'E'))
            && matches!(b.get(k), Some(b'-' | b'+'))
        {
            k = word_end(b, k + 1);
        }
    }
    k
}

fn quoted(b: &[u8], mut k: usize, close: u8) -> usize {
    while k < b.len() {
        match b[k] {
            b'\\' => k += 2,
            c if c == close => return k + 1,
            _ => k += 1,
        }
    }
    b.len()
}

fn triple(b: &[u8], from: usize) -> usize {
    let mut k = from;
    while let Some(nl) = b[k.min(b.len())..].iter().position(|&c| c == b'\n') {
        k += nl + 1;
        let rest = &b[k..];
        let pad = rest
            .iter()
            .take_while(|c| matches!(c, b' ' | b'\t'))
            .count();
        if rest[pad..].starts_with(b"\"\"\"") {
            return k + pad + 3;
        }
    }
    b.len()
}

struct Frame {
    line: usize,
    fold: bool,
    block: bool,
}

fn ends_a_term(t: &Tok) -> bool {
    match t.kind {
        Tk::Var | Tk::Value | Tk::Macro => true,
        Tk::Atom => !OPERATORS.contains(&t.text),
        Tk::Punct => matches!(t.text, ")" | "]" | "}" | ">>"),
    }
}

fn anonymous_fun(toks: &[Tok], k: usize) -> bool {
    let text = |n: usize| toks.get(n).map(|t| (t.kind, t.text));
    match (text(k + 1), text(k + 2)) {
        (Some((Tk::Punct, "(")), Some((Tk::Punct, "("))) => false,
        (Some((Tk::Punct, "(")), Some((Tk::Punct, ")"))) => text(k + 3) == Some((Tk::Punct, "->")),
        (Some((Tk::Punct, "(")), _) | (Some((Tk::Var, _)), Some((Tk::Punct, "("))) => true,
        _ => false,
    }
}

fn close(stack: &mut Vec<Frame>, block: bool, line: usize, nodes: &mut Vec<(usize, usize)>) {
    if let Some(at) = stack.iter().rposition(|f| f.block == block) {
        nodes.extend((stack[at].fold).then_some((stack[at].line, line)));
        stack.truncate(at);
    }
}

pub(crate) fn erlang(lines: &[String]) -> Blocks {
    let src = lines.join("\n");
    let toks = tokens(&src);
    let (mut nodes, mut funcs) = (Vec::new(), Vec::new());
    let mut stack: Vec<Frame> = Vec::new();
    let (mut form, mut clause, mut export, mut guard) = (true, None, None, false);
    for (k, t) in toks.iter().enumerate() {
        let next = toks.get(k + 1).map(|n| n.text);
        if form {
            form = false;
            let named = matches!(t.kind, Tk::Atom | Tk::Macro)
                || (t.kind == Tk::Value && t.text.starts_with('\''));
            if named && next == Some("(") {
                clause = Some(t.line);
            } else if t.text == "-" && matches!(next, Some("export" | "export_type")) {
                export = Some(t.line);
            }
        }
        let line = t.line;
        let ends_clause = match (t.kind, t.text) {
            (Tk::Punct, ".") => {
                nodes.extend(export.take().map(|h| (h, line)));
                stack.clear();
                (form, guard) = (true, false);
                true
            }
            (Tk::Punct, ";") => stack.is_empty() && !guard,
            (Tk::Atom, "when") if stack.is_empty() => {
                guard = true;
                false
            }
            (Tk::Punct, "->") if stack.is_empty() => {
                guard = false;
                false
            }
            (Tk::Punct, "(" | "[" | "{" | "<<" | "#{") => {
                let fold = t.text == "#{" && (k == 0 || !ends_a_term(&toks[k - 1]));
                stack.push(Frame {
                    line,
                    fold,
                    block: false,
                });
                false
            }
            (Tk::Punct, "||") => {
                if let Some(f) = stack.last_mut().filter(|f| !f.block) {
                    f.fold = false;
                }
                false
            }
            (Tk::Punct, ")" | "]" | "}" | ">>") => {
                close(&mut stack, false, line, &mut nodes);
                false
            }
            (Tk::Atom, "end") => {
                close(&mut stack, true, line, &mut nodes);
                false
            }
            (Tk::Atom, w) if OPENERS.contains(&w) || (w == "fun" && anonymous_fun(&toks, k)) => {
                let fold = matches!(w, "case" | "maybe" | "fun");
                stack.push(Frame {
                    line,
                    fold,
                    block: true,
                });
                false
            }
            _ => false,
        };
        if ends_clause && let Some(h) = clause.take() {
            nodes.push((h, line));
            funcs.push((h, line));
            form = true;
        }
    }
    Blocks::from_nodes(nodes, funcs, lines.len())
}
