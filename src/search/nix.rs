use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::*;

const NAME: &str = r"[A-Za-z_][\w'-]*";

pub fn nix_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    let part = format!(r#"(?:{NAME}|"[^"]*")"#);
    vec![
        format!(r#"(?:^|[{{;]|\blet\b)\s*(?:{part}\s*\.\s*)*(?:{w}|"{w}")\s*=(?:[^=]|$)"#),
        format!(r"\binherit\b(?:\s*\([^)]*\))?(?:\s+[\w'-]+)*?\s+{w}(?:\s+[\w'-]+)*\s*(?:;|$)"),
    ]
}

pub(super) const NIX_FUNCTION_SYMBOL: &str = concat!(
    r"^\s*(?:let\s+)?(?P<name>[A-Za-z_][\w'-]*)\s*=\s*",
    r"(?:[A-Za-z_][\w'-]*\s*:(?:\s|$)",
    r"|(?:[A-Za-z_][\w'-]*\s*@\s*)?\{(?:[^{}]|\{[^{}]*\})*\}\s*(?:@\s*[A-Za-z_][\w'-]*\s*)?:(?:\s|$))"
);

pub fn nix_name(line: &str, r: Range<usize>) -> Range<usize> {
    let primes = line.as_bytes()[r.end..]
        .iter()
        .take_while(|&&c| c == b'\'')
        .count();
    r.start..r.end + primes
}

pub fn nix_path(line: &str, col: usize) -> Option<String> {
    static PATH: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[\s(\[{=;])((?:\.{1,2}|[\w.+-]*)(?:/[\w.+-]+)+/?)").unwrap()
    });
    let m = PATH
        .captures_iter(line)
        .filter_map(|c| c.get(1))
        .find(|m| m.start() <= col && col < m.end())?;
    let before = &line[..m.start()];
    let quotes = before.matches('"').count() - before.matches("\\\"").count();
    quotes
        .is_multiple_of(2)
        .then(|| m.as_str().trim_end_matches('/').to_owned())
}

pub fn nix_files(dir: &Path, path: &str, files: &[PathBuf]) -> Vec<PathBuf> {
    let Some(at) = lexical(&dir.join(path)) else {
        return Vec::new();
    };
    [at.clone(), at.join("default.nix")]
        .into_iter()
        .find(|f| files.contains(f))
        .into_iter()
        .collect()
}

enum Ctx {
    Code(usize),
    Quoted(usize),
    Indented,
}

struct Lexed {
    code: Vec<u8>,
    literal: Vec<bool>,
}

fn name_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn name_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || b"_'-".contains(&c)
}

fn lex(text: &str) -> Lexed {
    let b = text.as_bytes();
    let mut code = b.to_vec();
    let mut literal = vec![false];
    let mut stack = vec![Ctx::Code(0)];
    let blank = |code: &mut Vec<u8>, r: Range<usize>| {
        for c in &mut code[r] {
            if *c != b'\n' {
                *c = b' ';
            }
        }
    };
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            literal.push(!matches!(stack.last(), Some(Ctx::Code(_))));
            i += 1;
            continue;
        }
        let at = |s: &[u8]| b[i..].starts_with(s);
        let nested = stack.len() > 1;
        match stack.last_mut() {
            Some(Ctx::Code(depth)) => {
                if name_start(c) && (i == 0 || !name_byte(b[i - 1])) {
                    i += b[i..].iter().take_while(|&&c| name_byte(c)).count();
                    continue;
                }
                if c == b'#' {
                    let end = b[i..]
                        .iter()
                        .position(|&c| c == b'\n')
                        .map_or(b.len(), |n| i + n);
                    blank(&mut code, i..end);
                    i = end;
                    continue;
                }
                if at(b"/*") {
                    let end = text[i + 2..].find("*/").map_or(b.len(), |n| i + n + 4);
                    blank(&mut code, i..end);
                    literal.extend(b[i..end].iter().filter(|&&c| c == b'\n').map(|_| true));
                    i = end;
                    continue;
                }
                match c {
                    b'"' => stack.push(Ctx::Quoted(i + 1)),
                    b'\'' if at(b"''") => {
                        stack.push(Ctx::Indented);
                        i += 1;
                    }
                    b'{' => *depth += 1,
                    b'}' if *depth == 0 && nested => {
                        stack.pop();
                    }
                    b'}' => *depth = depth.saturating_sub(1),
                    _ => {}
                }
                i += 1;
            }
            Some(Ctx::Quoted(from)) => {
                let from = *from;
                if c == b'\\' {
                    let end = (i + 2).min(b.len()) - usize::from(b.get(i + 1) == Some(&b'\n'));
                    blank(&mut code, i..end);
                    i = end;
                } else if at(b"${") {
                    stack.push(Ctx::Code(0));
                    i += 2;
                } else if c == b'"' {
                    stack.pop();
                    let inner = &b[from..i];
                    let named = inner.first().is_some_and(|&c| name_start(c))
                        && inner.iter().all(|&c| name_byte(c));
                    if named {
                        code[from..i].copy_from_slice(inner);
                    }
                    i += 1;
                } else {
                    blank(&mut code, i..i + 1);
                    i += 1;
                }
            }
            Some(Ctx::Indented) => {
                if at(b"'''") || at(b"''$") {
                    blank(&mut code, i..i + 3);
                    i += 3;
                } else if at(b"''\\") {
                    let end = (i + 4).min(b.len()) - usize::from(b.get(i + 3) == Some(&b'\n'));
                    blank(&mut code, i..end);
                    i = end;
                } else if at(b"''") {
                    stack.pop();
                    i += 2;
                } else if at(b"${") {
                    stack.push(Ctx::Code(0));
                    i += 2;
                } else {
                    blank(&mut code, i..i + 1);
                    i += 1;
                }
            }
            None => break,
        }
    }
    Lexed { code, literal }
}

pub(super) fn nix_literal_lines(text: &str) -> Vec<bool> {
    lex(text).literal
}

#[derive(Clone, Copy, PartialEq)]
enum Tok<'a> {
    Word(&'a str),
    Punct(u8),
    Eq,
}

fn tokens(code: &str) -> Vec<(Tok<'_>, usize)> {
    let b = code.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if name_start(c) {
            let n = b[i..].iter().take_while(|&&c| name_byte(c)).count();
            out.push((Tok::Word(&code[i..i + n]), i));
            i += n;
            continue;
        }
        let near = |j: usize| b.get(j).is_some_and(|c| b"=!<>".contains(c));
        match c {
            b'=' if !near(i + 1) && !(i > 0 && near(i - 1)) => out.push((Tok::Eq, i)),
            b'$' if b.get(i + 1) == Some(&b'{') => {
                out.push((Tok::Punct(b'{'), i));
                i += 1;
            }
            b'{' | b'}' | b'[' | b']' | b'(' | b')' | b';' | b':' | b',' | b'@' | b'.' | b'?'
            | b'/' | b'"' => out.push((Tok::Punct(c), i)),
            _ => {}
        }
        i += 1;
    }
    out
}

struct Scope<'a> {
    from: usize,
    to: usize,
    binds: Vec<(&'a str, usize)>,
    is_let: bool,
}

struct Parse<'a> {
    toks: Vec<(Tok<'a>, usize)>,
    matched: Vec<Option<usize>>,
    end: usize,
}

impl<'a> Parse<'a> {
    fn new(code: &'a str) -> Self {
        let toks = tokens(code);
        let mut matched = vec![None; toks.len()];
        let mut open: Vec<usize> = Vec::new();
        for (k, (t, _)) in toks.iter().enumerate() {
            match t {
                Tok::Punct(b'{' | b'[' | b'(') => open.push(k),
                Tok::Punct(b'}' | b']' | b')') => {
                    if let Some(o) = open.pop() {
                        (matched[o], matched[k]) = (Some(k), Some(o));
                    }
                }
                _ => {}
            }
        }
        Parse {
            toks,
            matched,
            end: code.len(),
        }
    }

    fn keyword(&self, k: usize, w: &str) -> bool {
        self.toks[k].0 == Tok::Word(w)
            && !(k > 0 && matches!(self.toks[k - 1].0, Tok::Punct(b'.' | b'/' | b'"')))
    }

    fn body_end(&self, mut k: usize) -> usize {
        let (mut lets, mut ifs, mut semis) = (0usize, 0usize, 0usize);
        while k < self.toks.len() {
            let at = self.toks[k].1;
            match self.toks[k].0 {
                Tok::Punct(b'{' | b'[' | b'(') => match self.matched[k] {
                    Some(m) => k = m,
                    None => return self.end,
                },
                Tok::Punct(b'}' | b']' | b')') => return at,
                Tok::Punct(b';') if semis > 0 => semis -= 1,
                Tok::Punct(b';') if lets == 0 => return at,
                _ if self.keyword(k, "with") || self.keyword(k, "assert") => semis += 1,
                _ if self.keyword(k, "let") => lets += 1,
                _ if self.keyword(k, "in") && lets > 0 => lets -= 1,
                _ if self.keyword(k, "in") => return at,
                _ if self.keyword(k, "if") => ifs += 1,
                _ if self.keyword(k, "then") && ifs == 0 => return at,
                _ if self.keyword(k, "else") && ifs == 0 => return at,
                _ if self.keyword(k, "else") => ifs -= 1,
                _ => {}
            }
            k += 1;
        }
        self.end
    }

    fn let_scope(&self, at: usize) -> Option<Scope<'a>> {
        let (mut k, mut lets, mut stmt, mut binds) = (at + 1, 0usize, true, Vec::new());
        while k < self.toks.len() {
            let (t, pos) = self.toks[k];
            if self.keyword(k, "in") && lets == 0 {
                return Some(Scope {
                    from: self.toks[at].1,
                    to: self.body_end(k + 1),
                    binds,
                    is_let: true,
                });
            }
            match t {
                Tok::Punct(b'{' | b'[' | b'(') => {
                    k = self.matched[k]?;
                    stmt = false;
                }
                Tok::Punct(b';') => stmt = lets == 0,
                _ if self.keyword(k, "let") => (lets, stmt) = (lets + 1, false),
                _ if self.keyword(k, "in") => lets -= 1,
                _ if stmt && self.keyword(k, "inherit") => {
                    stmt = false;
                    k += 1;
                    if self.toks.get(k).is_some_and(|t| t.0 == Tok::Punct(b'(')) {
                        k = self.matched[k]? + 1;
                    }
                    while let Some(&(Tok::Word(w), p)) = self.toks.get(k) {
                        binds.push((w, p));
                        k += 1;
                    }
                    continue;
                }
                Tok::Word(w) if stmt => {
                    stmt = false;
                    let next = self.toks.get(k + 1).map(|t| t.0);
                    if matches!(next, Some(Tok::Eq | Tok::Punct(b'.'))) {
                        binds.push((w, pos));
                    }
                }
                Tok::Punct(b'"') if stmt => {
                    if let (Some(&(Tok::Word(w), p)), Some((Tok::Punct(b'"'), _))) =
                        (self.toks.get(k + 1), self.toks.get(k + 2))
                    {
                        binds.push((w, p));
                    }
                    stmt = false;
                }
                _ => {}
            }
            k += 1;
        }
        None
    }

    fn lambda(&self, colon: usize, code: &[u8]) -> Option<Scope<'a>> {
        let after = code.get(self.toks[colon].1 + 1);
        if !after.is_none_or(|c| c.is_ascii_whitespace() || b"([{\"'".contains(c)) {
            return None;
        }
        let word = |k: usize| match self.toks.get(k) {
            Some(&(Tok::Word(w), p)) => Some((w, p)),
            _ => None,
        };
        let punct = |k: usize, c: u8| self.toks.get(k).is_some_and(|t| t.0 == Tok::Punct(c));
        let mut binds = Vec::new();
        let mut close = colon.checked_sub(1)?;
        if let Some(alias) = word(close) {
            if close >= 2 && punct(close - 1, b'@') && punct(close - 2, b'}') {
                binds.push(alias);
                close -= 2;
            } else {
                return Some(Scope {
                    from: alias.1,
                    to: self.body_end(colon + 1),
                    binds: vec![alias],
                    is_let: false,
                });
            }
        }
        if !punct(close, b'}') {
            return None;
        }
        let open = self.matched[close]?;
        let mut from = self.toks[open].1;
        if open >= 2 && punct(open - 1, b'@') {
            let alias = word(open - 2)?;
            binds.push(alias);
            from = alias.1;
        }
        let (mut k, mut item) = (open + 1, true);
        while k < close {
            match self.toks[k].0 {
                Tok::Punct(b'{' | b'[' | b'(') => k = self.matched[k].unwrap_or(close),
                Tok::Punct(b',') => item = true,
                Tok::Word(w) if item => {
                    binds.push((w, self.toks[k].1));
                    item = false;
                }
                _ => item = false,
            }
            k += 1;
        }
        Some(Scope {
            from,
            to: self.body_end(colon + 1),
            binds,
            is_let: false,
        })
    }

    fn scopes(&self, code: &[u8]) -> Vec<Scope<'a>> {
        (0..self.toks.len())
            .filter_map(|k| match self.toks[k].0 {
                Tok::Punct(b':') => self.lambda(k, code),
                _ if self.keyword(k, "let") => self.let_scope(k),
                _ => None,
            })
            .collect()
    }
}

fn line_of(text: &str, pos: usize) -> usize {
    text.as_bytes()[..pos]
        .iter()
        .filter(|&&c| c == b'\n')
        .count()
        + 1
}

pub(super) fn nix_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let text = lines.join("\n");
    let lexed = lex(&text);
    let code = String::from_utf8_lossy(&lexed.code);
    let parse = Parse::new(&code);
    let start: usize = lines[..at].iter().map(|l| l.len() + 1).sum();
    let end = start + lines[at].len();
    let scopes = parse.scopes(&lexed.code);
    scopes
        .iter()
        .filter(|s| s.from <= end && s.to >= start)
        .filter_map(|s| Some((s.from, s.binds.iter().find(|b| b.0 == name)?.1)))
        .max_by_key(|&(from, _)| from)
        .map(|(_, pos)| Binding {
            line: line_of(&text, pos),
            value: Value::Unknown,
        })
        .into_iter()
        .collect()
}

pub fn nix_declares<S: AsRef<str>>(lines: &[S], line: usize, word: &str, in_let: bool) -> bool {
    let text = lines
        .iter()
        .map(AsRef::as_ref)
        .collect::<Vec<_>>()
        .join("\n");
    let Some(here) = text.split('\n').nth(line.wrapping_sub(1)) else {
        return false;
    };
    let start = here.as_ptr() as usize - text.as_ptr() as usize;
    let lexed = lex(&text);
    let code = String::from_utf8_lossy(&lexed.code);
    let shape = Regex::new(&nix_patterns(word).join("|"))
        .is_ok_and(|re| re.is_match(&code[start..start + here.len()]));
    if !shape || in_let {
        return shape;
    }
    let parse = Parse::new(&code);
    !parse.scopes(&lexed.code).iter().any(|s| {
        s.is_let
            && (s.binds.iter())
                .any(|&(w, p)| w == word && (start..=start + here.len()).contains(&p))
    })
}
