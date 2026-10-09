use super::words::{Blocks, word_byte};

#[derive(Clone, Copy, PartialEq)]
enum Tk {
    Word,
    Keyword,
    Value,
    Key,
    Op,
    Open(u8),
    Close(u8),
    Semi,
}

struct Tok<'a> {
    kind: Tk,
    text: &'a str,
    line: usize,
    end: usize,
    spaced: bool,
}

const KEYWORDS: &[&str] = &[
    "do", "end", "fn", "else", "after", "rescue", "catch", "when", "and", "or", "not", "in",
];

const OPS: &[&str] = &[
    "===", "!==", "<<<", ">>>", "<<~", "~>>", "<~>", "|||", "&&&", "^^^", "...", "<|>", "+++",
    "---", "|>", "++", "--", "<>", "==", "!=", "=~", "<=", ">=", "&&", "||", "->", "<-", "=>",
    "::", "..", "//", "**", "<~", "~>", "\\\\",
];

struct Lexer<'a> {
    src: &'a str,
    b: &'a [u8],
    starts: Vec<usize>,
    i: usize,
}

impl<'a> Lexer<'a> {
    fn line(&self, at: usize) -> usize {
        self.starts.partition_point(|&s| s <= at) - 1
    }

    fn at(&self, k: usize) -> u8 {
        self.b.get(k).copied().unwrap_or(0)
    }

    fn word_end(&self, mut k: usize) -> usize {
        while k < self.b.len() && word_byte(self.b[k]) {
            k += 1;
        }
        k
    }

    fn heredoc(&self, from: usize, q: u8) -> usize {
        let mut k = from;
        loop {
            let Some(nl) = self.b[k..].iter().position(|&c| c == b'\n') else {
                return self.b.len();
            };
            k += nl + 1;
            let rest = &self.b[k..];
            let pad = rest
                .iter()
                .take_while(|c| matches!(c, b' ' | b'\t'))
                .count();
            if rest[pad..].starts_with(&[q, q, q]) {
                return k + pad + 3;
            }
        }
    }

    fn quoted(&self, mut k: usize, close: u8, interp: bool) -> usize {
        while k < self.b.len() {
            match self.b[k] {
                b'\\' => k += 2,
                c if c == close => return k + 1,
                b'#' if interp && self.at(k + 1) == b'{' => k = self.interpolation(k + 2),
                _ => k += 1,
            }
        }
        self.b.len()
    }

    fn interpolation(&self, mut k: usize) -> usize {
        let mut depth = 1;
        while k < self.b.len() {
            match self.b[k] {
                b'\\' => k += 2,
                b'"' => k = self.quoted(k + 1, b'"', true),
                b'{' => {
                    depth += 1;
                    k += 1;
                }
                b'}' => {
                    depth -= 1;
                    k += 1;
                    if depth == 0 {
                        return k;
                    }
                }
                _ => k += 1,
            }
        }
        self.b.len()
    }

    fn string(&self, k: usize, q: u8) -> usize {
        if self.at(k + 1) == q && self.at(k + 2) == q {
            self.heredoc(k + 3, q)
        } else {
            self.quoted(k + 1, q, true)
        }
    }

    fn sigil(&self, k: usize) -> Option<usize> {
        let c = self.at(k + 1);
        let upper = c.is_ascii_uppercase();
        if !upper && !c.is_ascii_lowercase() {
            return None;
        }
        let mut d = k + 2;
        while upper && (self.at(d).is_ascii_uppercase() || self.at(d).is_ascii_digit()) {
            d += 1;
        }
        let open = self.at(d);
        let mut end = match open {
            b'"' | b'\'' if self.at(d + 1) == open && self.at(d + 2) == open => {
                self.heredoc(d + 3, open)
            }
            b'/' | b'|' | b'"' | b'\'' | b'(' | b'[' | b'{' | b'<' => {
                let close = match open {
                    b'(' => b')',
                    b'[' => b']',
                    b'{' => b'}',
                    b'<' => b'>',
                    c => c,
                };
                self.quoted(d + 1, close, !upper)
            }
            _ => return None,
        };
        while self.at(end).is_ascii_alphanumeric() {
            end += 1;
        }
        Some(end)
    }

    fn key_after(&self, k: usize) -> bool {
        self.at(k) == b':'
            && self.at(k + 1) != b':'
            && matches!(self.at(k + 1), b' ' | b'\t' | b'\n' | b'\r' | 0)
    }

    fn next(&mut self) -> Option<(Tk, usize, usize)> {
        let b = self.b;
        loop {
            let c = self.at(self.i);
            match c {
                0 if self.i >= b.len() => return None,
                b' ' | b'\t' | b'\r' | b'\n' => self.i += 1,
                b'\\' if self.at(self.i + 1) == b'\n' => self.i += 2,
                b'#' => {
                    while self.i < b.len() && b[self.i] != b'\n' {
                        self.i += 1;
                    }
                }
                _ => break,
            }
        }
        let s = self.i;
        let c = b[s];
        let next = self.at(s + 1);
        let (kind, e) = match c {
            b'"' | b'\'' => {
                let e = self.string(s, c);
                if self.key_after(e) {
                    (Tk::Key, e + 1)
                } else {
                    (Tk::Value, e)
                }
            }
            b'~' if self.sigil(s).is_some() => (Tk::Value, self.sigil(s).unwrap()),
            b'?' => {
                let mut e = s + 1;
                if self.at(e) == b'\\' {
                    e += 1;
                }
                e += 1;
                while e < b.len() && (b[e] & 0xC0) == 0x80 {
                    e += 1;
                }
                (Tk::Value, e.min(b.len()))
            }
            b':' if next == b'"' || next == b'\'' => (Tk::Value, self.quoted(s + 2, next, true)),
            b':' if next != b':' && word_byte(next) => {
                let mut e = self.word_end(s + 1);
                while matches!(self.at(e), b'?' | b'!' | b'@' | b'.') && word_byte(self.at(e + 1))
                    || matches!(self.at(e), b'?' | b'!')
                {
                    e = self.word_end(e + 1);
                }
                (Tk::Value, e)
            }
            b':' if next != b':' && b"+-*/<>=!&|^~.\\@".contains(&next) => {
                let mut e = s + 1;
                while b"+-*/<>=!&|^~.\\@".contains(&self.at(e)) {
                    e += 1;
                }
                (Tk::Value, e)
            }
            b'0'..=b'9' => {
                let mut e = s;
                while self.at(e).is_ascii_alphanumeric()
                    || self.at(e) == b'_'
                    || (self.at(e) == b'.' && self.at(e + 1).is_ascii_digit())
                    || (matches!(self.at(e), b'-' | b'+') && matches!(b[e - 1], b'e' | b'E'))
                {
                    e += 1;
                }
                (Tk::Value, e)
            }
            c if word_byte(c) => {
                let mut e = self.word_end(s);
                if matches!(self.at(e), b'?' | b'!') && self.at(e + 1) != b'=' {
                    e += 1;
                }
                let w = &self.src[s..e];
                if self.key_after(e) {
                    (Tk::Key, e + 1)
                } else if c.is_ascii_uppercase() || matches!(w, "true" | "false" | "nil") {
                    (Tk::Value, e)
                } else if KEYWORDS.contains(&w) && !self.b[..s].ends_with(b".") {
                    (Tk::Keyword, e)
                } else {
                    (Tk::Word, e)
                }
            }
            b'%' if next == b'{' => (Tk::Open(b'%'), s + 2),
            b'%' if word_byte(next) => {
                let mut e = s + 1;
                while word_byte(self.at(e)) || self.at(e) == b'.' {
                    e += 1;
                }
                if self.at(e) == b'{' {
                    (Tk::Open(b'%'), e + 1)
                } else {
                    (Tk::Op, s + 1)
                }
            }
            b'(' | b'[' | b'{' => (Tk::Open(c), s + 1),
            b')' | b']' | b'}' => (Tk::Close(c), s + 1),
            b';' => (Tk::Semi, s + 1),
            _ => match OPS.iter().find(|op| b[s..].starts_with(op.as_bytes())) {
                Some(op) => (Tk::Op, s + op.len()),
                None if c == b'<' && next == b'<' => (Tk::Open(b'<'), s + 2),
                None if c == b'>' && next == b'>' => (Tk::Close(b'>'), s + 2),
                None => (Tk::Op, s + 1),
            },
        };
        self.i = e.max(s + 1).min(b.len());
        Some((kind, s, self.i))
    }
}

fn tokens(src: &str) -> Vec<Tok<'_>> {
    let mut starts = vec![0];
    starts.extend(src.match_indices('\n').map(|(i, _)| i + 1));
    let mut lx = Lexer {
        src,
        b: src.as_bytes(),
        starts,
        i: 0,
    };
    let mut out = Vec::new();
    while let Some((kind, s, e)) = lx.next() {
        let spaced = s > 0 && matches!(lx.b[s - 1], b' ' | b'\t');
        out.push(Tok {
            kind,
            text: src.get(s..e).unwrap_or(""),
            line: lx.line(s),
            end: lx.line(e.saturating_sub(1).max(s)),
            spaced,
        });
    }
    out
}

#[derive(Clone, Copy, PartialEq)]
enum Fk {
    Root,
    Do,
    Fn,
    Paren,
    Fold(u8),
    Plain(u8),
    Args,
}

struct Frame {
    kind: Fk,
    line: usize,
    func: bool,
    head: usize,
    expr: Option<usize>,
    before: usize,
    clause: Option<usize>,
    when: Option<usize>,
}

impl Frame {
    fn new(kind: Fk, line: usize) -> Self {
        Frame {
            kind,
            line,
            func: false,
            head: 0,
            expr: None,
            before: line,
            clause: None,
            when: None,
        }
    }

    fn clauses(&self) -> bool {
        matches!(self.kind, Fk::Root | Fk::Do | Fk::Fn | Fk::Paren)
    }

    fn end_clause(&mut self, closer: usize, last: usize, nodes: &mut Vec<(usize, usize)>) {
        if let Some(c) = self.clause.take() {
            nodes.push((c, if closer > last { closer - 1 } else { last }));
        }
    }
}

fn continues(t: &Tok) -> bool {
    match t.kind {
        Tk::Op | Tk::Key | Tk::Open(_) => true,
        Tk::Keyword => t.text != "end",
        _ => false,
    }
}

fn leads(t: &Tok, next: Option<&Tok>) -> bool {
    match t.kind {
        Tk::Keyword => matches!(t.text, "when" | "and" | "or" | "in"),
        Tk::Op => match t.text {
            "@" | "&" | "^" | "!" | "..." => false,
            "-" | "+" => next.is_some_and(|n| n.spaced || n.line != t.line),
            _ => true,
        },
        _ => false,
    }
}

fn opens_args(t: &Tok, next: Option<&Tok>, after: Option<&Tok>) -> bool {
    let Some(n) = next else {
        return false;
    };
    if t.kind != Tk::Word || !n.spaced || n.line != t.end {
        return false;
    }
    match n.kind {
        Tk::Word | Tk::Value | Tk::Key => true,
        Tk::Open(c) => c != b'(',
        Tk::Keyword => matches!(n.text, "fn" | "not"),
        Tk::Op => match n.text {
            "@" | "&" | "^" | "!" => true,
            "-" | "+" => after.is_some_and(|a| !a.spaced && a.line == n.line),
            _ => false,
        },
        _ => false,
    }
}

fn close_args(stack: &mut Vec<Frame>, last: usize, nodes: &mut Vec<(usize, usize)>) {
    while stack.last().is_some_and(|f| f.kind == Fk::Args) {
        let f = stack.pop().unwrap();
        nodes.push((f.line, last));
    }
}

fn pop_to(
    stack: &mut Vec<Frame>,
    at: usize,
    last: usize,
    nodes: &mut Vec<(usize, usize)>,
) -> Frame {
    for f in stack.drain(at + 1..).collect::<Vec<_>>() {
        if f.kind == Fk::Args {
            nodes.push((f.line, last));
        }
    }
    stack.pop().unwrap()
}

fn defines(head: &str) -> bool {
    head.starts_with("def") || matches!(head, "test" | "describe")
}

pub(crate) fn elixir(lines: &[String]) -> Blocks {
    let src = lines.join("\n");
    let toks = tokens(&src);
    let mut nodes: Vec<(usize, usize)> = Vec::new();
    let mut funcs = Vec::new();
    let mut stack = vec![Frame::new(Fk::Root, 0)];
    let mut last = 0;
    let mut cont = true;
    let mut semi = false;
    for (k, t) in toks.iter().enumerate() {
        let next = toks.get(k + 1);
        let boundary = semi || (k > 0 && t.line > last && !cont && !leads(t, next));
        if boundary {
            close_args(&mut stack, last, &mut nodes);
        }
        let closes = matches!(t.kind, Tk::Close(_))
            || t.text == "->"
            || (t.kind == Tk::Keyword
                && matches!(t.text, "end" | "else" | "after" | "rescue" | "catch"));
        if let Some(f) = stack.last_mut()
            && f.clauses()
            && !closes
            && (boundary || f.expr.is_none())
        {
            f.expr = Some(t.line);
            f.head = k;
            f.before = last;
            f.when = None;
        }
        match (t.kind, t.text) {
            (Tk::Keyword, "do") => {
                close_args(&mut stack, last, &mut nodes);
                let func =
                    (stack.last()).is_some_and(|f| f.expr.is_some() && defines(toks[f.head].text));
                let mut f = Frame::new(Fk::Do, t.line);
                f.func = func;
                stack.push(f);
            }
            (Tk::Keyword, "fn") => stack.push(Frame::new(Fk::Fn, t.line)),
            (Tk::Keyword, "end") => {
                close_args(&mut stack, last, &mut nodes);
                if let Some(at) = (stack.iter()).rposition(|f| matches!(f.kind, Fk::Do | Fk::Fn)) {
                    let mut f = pop_to(&mut stack, at, last, &mut nodes);
                    f.end_clause(t.line, last, &mut nodes);
                    nodes.push((f.line, t.line));
                    if f.func {
                        funcs.push((f.line, t.line));
                    }
                }
            }
            (Tk::Keyword, "else" | "after" | "rescue" | "catch") => {
                close_args(&mut stack, last, &mut nodes);
                if let Some(f) = stack.last_mut().filter(|f| f.kind == Fk::Do) {
                    f.end_clause(t.line, last, &mut nodes);
                    f.expr = None;
                }
            }
            (Tk::Keyword, "when") => {
                if let Some(f) = stack.last_mut().filter(|f| f.clauses() && f.when.is_none()) {
                    f.when = Some(last);
                }
            }
            (Tk::Op, "->") => {
                close_args(&mut stack, last, &mut nodes);
                if let Some(f) = stack.last_mut().filter(|f| f.clauses()) {
                    if let Some(c) = f.clause {
                        nodes.push((c, f.before));
                    }
                    let start = match f.expr {
                        Some(e) => {
                            nodes.push((e, f.when.unwrap_or(last)));
                            e
                        }
                        _ => t.line,
                    };
                    f.clause = Some(start);
                    f.expr = None;
                    f.when = None;
                }
            }
            (Tk::Open(c), _) => {
                let kind = match c {
                    b'(' => Fk::Paren,
                    b'[' if !t.spaced
                        && k > 0
                        && toks[k - 1].end == t.line
                        && matches!(toks[k - 1].kind, Tk::Word | Tk::Value | Tk::Close(_)) =>
                    {
                        Fk::Plain(b']')
                    }
                    b'[' => Fk::Fold(b']'),
                    b'<' => Fk::Plain(b'>'),
                    _ => Fk::Fold(b'}'),
                };
                stack.push(Frame::new(kind, t.line));
            }
            (Tk::Close(c), _) => {
                close_args(&mut stack, last, &mut nodes);
                let matches = |f: &Frame| match f.kind {
                    Fk::Paren => c == b')',
                    Fk::Fold(e) | Fk::Plain(e) => e == c,
                    _ => false,
                };
                if let Some(at) = stack
                    .iter()
                    .rposition(|f| matches(f) || matches!(f.kind, Fk::Do | Fk::Fn))
                    .filter(|&at| matches(&stack[at]))
                {
                    let mut f = pop_to(&mut stack, at, last, &mut nodes);
                    if f.kind == Fk::Paren {
                        f.end_clause(t.line, last, &mut nodes);
                    }
                    if matches!(f.kind, Fk::Paren | Fk::Fold(_)) {
                        nodes.push((f.line, t.line));
                    }
                }
            }
            _ => {}
        }
        if opens_args(t, next, toks.get(k + 2)) {
            stack.push(Frame::new(Fk::Args, next.unwrap().line));
        }
        last = t.end;
        cont = continues(t);
        semi = t.kind == Tk::Semi;
    }
    Blocks::from_nodes(nodes, funcs, lines.len())
}
