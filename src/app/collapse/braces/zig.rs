use super::{K, Lexer, Model};

const KEYWORDS: &[&str] = &[
    "if",
    "while",
    "for",
    "switch",
    "fn",
    "struct",
    "enum",
    "union",
    "opaque",
    "error",
    "align",
    "callconv",
    "addrspace",
    "linksection",
    "asm",
    "volatile",
    "catch",
    "orelse",
    "return",
    "try",
    "and",
    "or",
    "comptime",
    "nosuspend",
    "await",
    "resume",
    "suspend",
    "defer",
    "errdefer",
    "break",
    "continue",
    "else",
    "inline",
    "noinline",
    "export",
    "extern",
    "pub",
    "const",
    "var",
    "test",
    "usingnamespace",
    "threadlocal",
    "packed",
    "noalias",
];
const CONTAINERS: &[&str] = &["struct", "enum", "union", "opaque", "error"];

impl Lexer<'_> {
    pub(super) fn zig_lines(&mut self) -> bool {
        let (l, at) = (self.l, self.i);
        if !self.bytes()[at..].starts_with(b"\\\\") {
            return false;
        }
        let gap = |n: &String| n.trim().is_empty() || n.trim_start().starts_with("//");
        let mut j = self.l + 1;
        while let Some(n) = self.lines.get(j) {
            if n.trim_start().starts_with("\\\\") {
                self.l = j;
            } else if !gap(n) {
                break;
            }
            j += 1;
        }
        self.i = self.bytes().len();
        self.emit(l, at, 1, K::Str, self.l);
        true
    }
}

impl Model<'_, '_> {
    pub(super) fn zig(&mut self) {
        let mut stmt = vec![false; self.toks.len()];
        for k in 0..self.toks.len() {
            let t = self.toks[k];
            match (t.kind, t.text) {
                (K::Str, _) => self.add(t.line, Some(t.end), false),
                (K::Punct, "{") => self.zig_brace(k),
                (K::Punct, "(") => self.zig_call(k),
                (K::Word, "if" | "while" | "for") => self.zig_cond(k, &mut stmt),
                (K::Word, "switch") if self.punct(k + 1, "(") => {
                    let body = self.pair[k + 1].map(|c| c + 1);
                    let end = body.filter(|&b| self.punct(b, "{"));
                    self.add(t.line, end.and_then(|b| self.closing(b)), false);
                }
                (K::Word, "asm") => {
                    let j = k + 1 + usize::from(self.tx(k + 1) == "volatile");
                    if self.punct(j, "(") {
                        self.add(t.line, self.closing(j), false);
                    }
                }
                (K::Word, "fn") => self.zig_fn(k),
                (K::Word, "test") if self.first[k] => {
                    let j = (k + 1..k + 3).find(|&j| self.punct(j, "{"));
                    if let Some(b) = j {
                        self.add(self.toks[b].line, self.closing(b), true);
                    }
                }
                _ => {}
            }
        }
    }

    fn zig_brace(&mut self, k: usize) {
        let p = k.checked_sub(1);
        let opener = p.filter(|&p| self.punct(p, ")")).and_then(|p| self.back[p]);
        let head = opener.and_then(|o| o.checked_sub(1));
        if head.is_some_and(|h| self.tx(h) == "switch") {
            return;
        }
        let keyword = [p, head]
            .into_iter()
            .flatten()
            .find(|&j| CONTAINERS.contains(&self.tx(j)));
        let start = self.toks[keyword.unwrap_or(k)].line;
        self.add(start, self.closing(k), false);
    }

    fn zig_call(&mut self, k: usize) {
        let (Some(c), Some(p)) = (self.pair[k], k.checked_sub(1)) else {
            return;
        };
        let t = self.toks[p];
        let callee = match t.kind {
            K::Word => !t.text.starts_with('@') && self.zig_operand(p) && self.before(p) != "fn",
            _ => self.zig_operand(p),
        };
        if callee {
            let start = self.zig_chain(p);
            self.add(self.toks[start].line, Some(self.end_of(c)), false);
        }
    }

    fn zig_operand(&self, p: usize) -> bool {
        let t = self.toks[p];
        match (t.kind, t.text) {
            (K::Word, w) => !KEYWORDS.contains(&w),
            (K::Punct, ")") => (self.back[p].and_then(|o| o.checked_sub(1)))
                .is_some_and(|q| self.word_at(q) && self.zig_operand(q)),
            (K::Punct, "]") => true,
            (K::Punct, "*" | "?") => self.before(p) == ".",
            _ => false,
        }
    }

    fn zig_chain(&self, mut j: usize) -> usize {
        loop {
            if self.closer(j) {
                let Some(o) = self.back[j] else {
                    return j;
                };
                j = o;
                match j.checked_sub(1) {
                    Some(p) if self.zig_operand(p) => j = p,
                    _ => return j,
                }
                continue;
            }
            if self.before(j) != "." {
                return j;
            }
            match j.checked_sub(2) {
                Some(p) if self.zig_operand(p) => j = p,
                _ => return j - 1,
            }
        }
    }

    fn zig_fn(&mut self, k: usize) {
        let quoted =
            self.tx(k + 1) == "@" && self.toks.get(k + 2).is_some_and(|t| t.kind == K::Str);
        let open = k + 2 + usize::from(quoted);
        let named = self.word_at(k + 1) && self.punct(open, "(");
        let open = if named { open } else { k + 1 };
        let Some(c) = self.punct(open, "(").then(|| self.pair[open]).flatten() else {
            return;
        };
        self.add(self.toks[open].line, Some(self.end_of(c)), false);
        let (last, body) = self.zig_return(c + 1);
        if named {
            if let Some(b) = body {
                self.add(self.toks[b].line, self.closing(b), true);
            }
        } else {
            self.add(self.toks[k].line, last.map(|l| self.end_of(l)), false);
        }
    }

    fn zig_return(&self, mut j: usize) -> (Option<usize>, Option<usize>) {
        let mut last = None;
        while j < self.toks.len() {
            let t = self.toks[j];
            if t.kind == K::Punct && matches!(t.text, "," | ";" | "=" | ")" | "]" | "}") {
                return (last, None);
            }
            if t.kind == K::Punct && t.text == "{" && !CONTAINERS.contains(&self.before(j)) {
                return (last, Some(j));
            }
            let close = if self.opener(j) {
                self.pair[j]
            } else {
                Some(j)
            };
            let Some(close) = close else {
                return (last, None);
            };
            last = Some(close);
            j = close + 1;
        }
        (last, None)
    }

    fn zig_cond(&mut self, k: usize, stmt: &mut [bool]) {
        let p = k.checked_sub(1).map_or("{", |p| self.tx(p));
        let statement = stmt[k] || matches!(p, "{" | "}" | ";" | ":" | "inline");
        let Some((end, alt)) = self.zig_cond_end(k) else {
            return;
        };
        let line = self.toks[k].line;
        self.add(line, Some(self.end_of(end)), false);
        if statement && let Some(e) = alt {
            self.add(self.toks[e].line, Some(self.end_of(end)), false);
            let next = self.zig_payload(e + 1);
            if matches!(self.tx(next), "if" | "while" | "for") {
                stmt[next] = true;
            }
        }
    }

    fn zig_payload(&self, j: usize) -> usize {
        if self.tx(j) != "|" {
            return j;
        }
        (j + 1..self.toks.len())
            .find(|&i| self.tx(i) == "|")
            .map_or(j, |i| i + 1)
    }

    fn zig_cond_end(&self, k: usize) -> Option<(usize, Option<usize>)> {
        if !self.punct(k + 1, "(") {
            return None;
        }
        let mut j = self.zig_payload(self.pair[k + 1]? + 1);
        if self.tx(k) == "while" && self.punct(j, ":") && self.punct(j + 1, "(") {
            j = self.pair[j + 1]? + 1;
        }
        let end = self.zig_body(j)?;
        if self.tx(end + 1) != "else" {
            return Some((end, None));
        }
        let body = self.zig_payload(end + 2);
        Some((self.zig_body(body)?, Some(end + 1)))
    }

    fn zig_body(&self, j: usize) -> Option<usize> {
        let labeled = self.word_at(j) && self.tx(j + 1) == ":";
        let first = if labeled { j + 2 } else { j };
        match self.tx(first) {
            "{" => return self.pair[first],
            "if" | "while" | "for" => return Some(self.zig_cond_end(first)?.0),
            "switch" => {
                let c = (*self.pair.get(first + 1)?)?;
                return *self.pair.get(c + 1)?;
            }
            _ => {}
        }
        let mut last = None;
        let mut i = j;
        while i < self.toks.len() {
            let t = self.toks[i];
            if self.closer(i) || (t.kind == K::Punct && matches!(t.text, ";" | ",")) {
                break;
            }
            if t.kind == K::Word && t.text == "else" {
                break;
            }
            let end = match t.text {
                _ if self.opener(i) => self.pair[i]?,
                "if" | "while" | "for" if t.kind == K::Word => self.zig_cond_end(i)?.0,
                _ => i,
            };
            last = Some(end);
            i = end + 1;
        }
        last
    }
}
