#[derive(PartialEq)]
enum Tok<'a> {
    Word(&'a str),
    Open,
    Close,
    Semi,
    Comma,
    Dollar(&'a str),
    Other,
}

fn tokens(lines: &[String]) -> Vec<(usize, Tok<'_>)> {
    let mut out = Vec::new();
    let mut open: Option<(&str, bool)> = None;
    for (n, line) in lines.iter().enumerate() {
        let b = line.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if let Some((close, escapes)) = open {
                if escapes && b[i] == b'\\' {
                    i += 2;
                } else if b[i..].starts_with(close.as_bytes()) {
                    open = None;
                    i += close.len();
                    match close {
                        "\"" => out.push((n, Tok::Word(close))),
                        "'" => out.push((n, Tok::Other)),
                        _ => {}
                    }
                } else {
                    i += 1;
                }
                continue;
            }
            let rest = &b[i..];
            let word_end = |from: usize| {
                from + rest[from..]
                    .iter()
                    .take_while(|&&c| {
                        c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c >= 0x80
                    })
                    .count()
            };
            let (len, tok) = match rest[0] {
                b'-' if rest.starts_with(b"--") => break,
                b'/' if rest.starts_with(b"/*") => {
                    open = Some(("*/", false));
                    (2, None)
                }
                b'\'' => {
                    let escapes = i > 0
                        && matches!(b[i - 1], b'E' | b'e')
                        && (i < 2 || !b[i - 2].is_ascii_alphanumeric());
                    open = Some(("'", escapes));
                    (1, None)
                }
                b'"' => {
                    open = Some(("\"", false));
                    (1, None)
                }
                b'$' => match rest[1..].iter().position(|&c| c == b'$') {
                    Some(p)
                        if rest[1..=p]
                            .iter()
                            .all(|&c| c.is_ascii_alphabetic() || c == b'_') =>
                    {
                        (p + 2, line.get(i..i + p + 2).map(Tok::Dollar))
                    }
                    _ => (1, Some(Tok::Other)),
                },
                b'(' => (1, Some(Tok::Open)),
                b')' => (1, Some(Tok::Close)),
                b';' => (1, Some(Tok::Semi)),
                b',' => (1, Some(Tok::Comma)),
                c if c.is_ascii_whitespace() => (1, None),
                c if c.is_ascii_alphabetic() || c == b'_' || c >= 0x80 => {
                    let e = word_end(1);
                    (e, line.get(i..i + e).map(Tok::Word))
                }
                c if c.is_ascii_digit() => (word_end(1), Some(Tok::Other)),
                _ => (1, Some(Tok::Other)),
            };
            out.extend(tok.map(|t| (n, t)));
            i += len;
        }
    }
    out
}

fn is(t: Option<&Tok>, word: &str) -> bool {
    matches!(t, Some(Tok::Word(w)) if w.eq_ignore_ascii_case(word))
}

fn cte(before: &[(usize, Tok)]) -> bool {
    let mut back = before.iter().rev().map(|(_, t)| t);
    let mut t = back.next();
    if is(t, "MATERIALIZED") {
        t = back.next();
        if is(t, "NOT") {
            t = back.next();
        }
    }
    if !is(t, "AS") || !matches!(back.next(), Some(Tok::Word(_))) {
        return false;
    }
    let t = back.next();
    t == Some(&Tok::Comma) || is(t, "WITH") || is(t, "RECURSIVE")
}

#[derive(PartialEq)]
enum Frame<'a> {
    Top,
    Cte,
    Paren,
    Body(&'a str),
}

struct Level<'a> {
    frame: Frame<'a>,
    start: Option<usize>,
    last: usize,
}

impl Level<'_> {
    fn touch(&mut self, n: usize) {
        self.start.get_or_insert(n);
        self.last = n;
    }

    fn flush(&mut self, folds: &mut Vec<(usize, usize)>) {
        if let Some(h) = self.start.take()
            && self.last > h
        {
            folds.push((h, self.last));
        }
    }
}

struct Statements<'a> {
    stack: Vec<Level<'a>>,
    folds: Vec<(usize, usize)>,
}

impl<'a> Statements<'a> {
    fn push(&mut self, frame: Frame<'a>) {
        self.stack.push(Level {
            frame,
            start: None,
            last: 0,
        });
    }

    fn touch(&mut self, n: usize) {
        if let Some(l) = (self.stack.iter_mut().rev()).find(|l| l.frame != Frame::Paren) {
            l.touch(n);
        }
    }

    fn pop_to(&mut self, depth: usize) {
        while self.stack.len() > depth {
            if let Some(mut l) = self.stack.pop() {
                l.flush(&mut self.folds);
            }
        }
    }

    fn statement_start(&self) -> bool {
        (self.stack.iter().rev())
            .find(|l| l.frame != Frame::Paren)
            .is_some_and(|l| matches!(l.frame, Frame::Body(_)) && l.start.is_none())
    }
}

pub(super) fn sql(lines: &[String]) -> Vec<(usize, usize)> {
    let toks = tokens(lines);
    let mut s = Statements {
        stack: Vec::new(),
        folds: Vec::new(),
    };
    s.push(Frame::Top);
    for (k, (n, t)) in toks.iter().enumerate() {
        let n = *n;
        let depth = s.stack.len();
        let closing = match t {
            Tok::Dollar(tag) => (s.stack.iter()).rposition(|l| l.frame == Frame::Body(tag)),
            _ => None,
        };
        let statement =
            (s.stack.iter()).rposition(|l| matches!(l.frame, Frame::Top | Frame::Body(_)));
        match t {
            Tok::Semi => {
                let at = statement.unwrap_or(0);
                s.pop_to(at + 1);
                s.stack[at].flush(&mut s.folds);
            }
            Tok::Close if matches!(s.stack[depth - 1].frame, Frame::Cte | Frame::Paren) => {
                s.pop_to(depth - 1);
                s.touch(n);
            }
            Tok::Dollar(_) if closing.is_some() => {
                s.pop_to(closing.unwrap_or(depth));
                s.touch(n);
            }
            Tok::Dollar(tag) => {
                s.touch(n);
                s.push(Frame::Body(tag));
            }
            Tok::Open => {
                s.touch(n);
                s.push(if cte(&toks[..k]) {
                    Frame::Cte
                } else {
                    Frame::Paren
                });
            }
            Tok::Word(w)
                if s.statement_start()
                    && ["BEGIN", "END", "DECLARE"]
                        .iter()
                        .any(|k| w.eq_ignore_ascii_case(k)) => {}
            _ => s.touch(n),
        }
    }
    s.pop_to(0);
    s.folds.sort_unstable();
    s.folds
}
