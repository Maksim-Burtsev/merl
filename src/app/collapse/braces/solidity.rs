use super::{K, Model};

const HEADERS: &[&str] = &[
    "function",
    "modifier",
    "constructor",
    "fallback",
    "receive",
    "contract",
    "interface",
    "library",
    "event",
    "error",
    "import",
    "using",
    "pragma",
];
const NOT_CALLEE: &[&str] = &[
    "if", "for", "while", "function", "return", "returns", "revert", "emit", "mapping", "type",
    "payable", "catch", "assembly", "delete", "address", "bool", "string", "bytes", "byte", "int",
    "uint", "fixed", "ufixed",
];
const SOL_STOPS: &[&str] = &[
    "return",
    "emit",
    "revert",
    "if",
    "else",
    "while",
    "for",
    "do",
    "delete",
    "new",
    "returns",
    "try",
    "unchecked",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Code,
    Header,
    Yul,
}

impl Model<'_, '_> {
    pub(super) fn solidity(&mut self) {
        let n = self.toks.len();
        let mut role = vec![Role::Code; n];
        let mut body = vec![false; n];
        for k in 0..n {
            if role[k] != Role::Code || self.toks[k].kind != K::Word {
                continue;
            }
            let w = self.tx(k);
            if w == "assembly" {
                let Some(b) = (k + 1..n).find(|&j| self.punct(j, "{")) else {
                    continue;
                };
                let c = self.pair[b].unwrap_or(n - 1);
                body[b] = true;
                role[k..=c].fill(Role::Yul);
                self.add(self.toks[k].line, Some(self.end_of(c)), false);
                continue;
            }
            let declares = HEADERS.contains(&w)
                && !matches!(self.before(k), "(" | "," | ".")
                && match w {
                    "event" | "error" => self.word_at(k + 1) && self.punct(k + 2, "("),
                    "constructor" | "fallback" | "receive" => self.punct(k + 1, "("),
                    "import" | "using" | "pragma" => self.first[k],
                    _ => self.word_at(k + 1),
                };
            if !declares {
                continue;
            }
            let end = self.sol_header_end(k);
            role[k..end].fill(Role::Header);
            if matches!(w, "event" | "error") {
                self.add(self.toks[k].line, Some(self.end_of(end.min(n - 1))), false);
            } else if self.punct(end, "{") {
                body[end] = true;
            }
        }
        for k in 0..n {
            let t = self.toks[k];
            match (role[k], t.kind, t.text) {
                (Role::Yul, K::Punct, "{") if !body[k] => self.add(t.line, self.closing(k), false),
                (Role::Code, K::Punct, "{") => self.sol_brace(k, body[k]),
                (Role::Code, K::Punct, "(") => self.sol_paren(k),
                (Role::Code, K::Punct, "[") if self.sol_array(k) => {
                    self.add(t.line, self.closing(k), false);
                }
                (Role::Code, K::Word, "if" | "for" | "while") if self.punct(k + 1, "(") => {
                    let end = self.sol_stmt(k);
                    self.add(t.line, end.map(|e| self.end_of(e)), false);
                }
                (Role::Code, K::Word, "emit") => {
                    let end = self.sol_stmt(k);
                    self.add(t.line, end.map(|e| self.end_of(e)), false);
                }
                _ => {}
            }
        }
    }

    fn sol_header_end(&self, k: usize) -> usize {
        let mut j = k + 1;
        while j < self.toks.len() {
            if self.punct(j, "(") || self.punct(j, "[") {
                j = self.pair[j].map_or(j + 1, |c| c + 1);
                continue;
            }
            if self.punct(j, "{") && matches!(self.tx(k), "import" | "using") {
                j = self.pair[j].map_or(j + 1, |c| c + 1);
                continue;
            }
            if self.punct(j, "{") || self.punct(j, ";") || self.closer(j) {
                return j;
            }
            j += 1;
        }
        j
    }

    fn sol_brace(&mut self, k: usize, def: bool) {
        let p = self.before(k);
        if matches!(p, "(" | ",") {
            return;
        }
        let options = k > 0 && self.sol_options(k);
        let head = match k.checked_sub(2) {
            Some(h) if matches!(self.tx(h), "struct" | "enum") => h,
            _ if p == "unchecked" => k - 1,
            _ if options => self.sol_chain(k - 1),
            _ => k,
        };
        self.add(self.toks[head].line, self.closing(k), def);
    }

    fn sol_options(&self, k: usize) -> bool {
        self.punct(k, "{")
            && self.word_at(k + 1)
            && self.punct(k + 2, ":")
            && self.pair[k].is_some_and(|c| self.punct(c + 1, "("))
    }

    fn sol_paren(&mut self, k: usize) {
        let Some(c) = self.pair[k] else {
            return;
        };
        let line = self.toks[k].line;
        let Some(p) = k.checked_sub(1) else {
            return;
        };
        let t = self.toks[p];
        let callee = match t.kind {
            K::Word => {
                self.before(p) == "." || !(NOT_CALLEE.contains(&t.text) || typed_name(t.text))
            }
            K::Punct => {
                matches!(t.text, ")" | "]") || self.back[p].is_some_and(|o| self.sol_options(o))
            }
            _ => false,
        };
        if callee {
            let start = self.sol_chain(p);
            if matches!(self.before(start), "emit" | "revert" | "catch") {
                return;
            }
            let member = (start..p).any(|j| self.punct(j, ".") || self.punct(j, "["));
            let start = if member { self.sol_reach(start) } else { start };
            let start = if self.before(start) == "new" {
                start - 1
            } else {
                start
            };
            self.add(self.toks[start].line, Some(self.end_of(c)), false);
            return;
        }
        if t.kind == K::Word && NOT_CALLEE.contains(&t.text) && t.text != "return" {
            return;
        }
        if self.sol_tuple(k, c) {
            self.add(line, Some(self.end_of(c)), false);
        }
    }

    fn sol_tuple(&self, k: usize, c: usize) -> bool {
        if c == k + 1 {
            return true;
        }
        let (mut j, mut comma) = (k + 1, false);
        while j < c {
            if self.opener(j) {
                j = self.pair[j].unwrap_or(j) + 1;
                continue;
            }
            if self.punct(j, ",") {
                comma = true;
            }
            let decl = self.word_at(j)
                && (self.word_at(j + 1) || self.punct(j + 1, "["))
                && self.before(j) != "new"
                && !matches!(self.tx(j), "new" | "delete" | "payable" | "type");
            if decl {
                return false;
            }
            j += 1;
        }
        comma
    }

    fn sol_array(&self, k: usize) -> bool {
        let Some(p) = k.checked_sub(1) else {
            return true;
        };
        let t = self.toks[p];
        match t.kind {
            K::Punct => !matches!(t.text, ")" | "]" | "}"),
            K::Word => t.text == "return",
            _ => false,
        }
    }

    fn sol_chain(&self, mut j: usize) -> usize {
        loop {
            if self.closer(j) {
                let Some(o) = self.back[j] else {
                    return j;
                };
                j = o;
                match j.checked_sub(1) {
                    Some(p) if self.word_at(p) || self.closer(p) => j = p,
                    _ => return j,
                }
                continue;
            }
            if self.before(j) != "." {
                return j;
            }
            match j.checked_sub(2) {
                Some(p) if self.word_at(p) || self.closer(p) => j = p,
                _ => return j - 1,
            }
        }
    }

    fn sol_reach(&self, start: usize) -> usize {
        let (mut j, mut from) = (start, start);
        while let Some(p) = j.checked_sub(1) {
            let t = self.toks[p];
            j = match (t.kind, t.text) {
                (K::Punct, ")" | "]") => match self.back[p] {
                    Some(o) => o,
                    None => return from,
                },
                (K::Punct, "(" | "[" | "{" | "}" | "," | ";" | "?" | "=>") => return from,
                (K::Punct, "=") if !self.sol_compares(p) => return from,
                (K::Punct, ":") => match self.sol_question(p) {
                    Some(q) => q,
                    None => return from,
                },
                (K::Word, w) if SOL_STOPS.contains(&w) => return from,
                _ => p,
            };
            from = j;
        }
        from
    }

    fn sol_compares(&self, p: usize) -> bool {
        let near = |a: usize, b: usize| self.toks[a].line == self.toks[b].line;
        let before = p.checked_sub(1).filter(|&q| near(q, p));
        let after = Some(p + 1).filter(|&q| q < self.toks.len() && near(q, p));
        before.is_some_and(|q| matches!(self.tx(q), "=" | "!" | "<" | ">"))
            || after.is_some_and(|q| self.tx(q) == "=")
    }

    fn sol_question(&self, p: usize) -> Option<usize> {
        let mut j = p;
        while let Some(q) = j.checked_sub(1) {
            let t = self.toks[q];
            j = match (t.kind, t.text) {
                (K::Punct, ")" | "]") => self.back[q]?,
                (K::Punct, "?") => return Some(q),
                (K::Punct, "(" | "[" | "{" | "}" | "," | ";" | ":") => return None,
                (K::Word, w) if SOL_STOPS.contains(&w) => return None,
                _ => q,
            };
        }
        None
    }

    fn sol_stmt(&self, k: usize) -> Option<usize> {
        let mut j = k;
        if matches!(self.tx(k), "if" | "for" | "while") {
            let c = self.pair.get(k + 1).copied().flatten()?;
            let end = self.sol_stmt(c + 1)?;
            if self.tx(k) == "if" && self.tx(end + 1) == "else" {
                return self.sol_stmt(end + 2);
            }
            return Some(end);
        }
        if self.tx(j) == "unchecked" {
            j += 1;
        }
        if self.punct(j, "{") {
            return self.pair[j];
        }
        if self.tx(j) == "do" {
            let b = self.sol_stmt(j + 1)?;
            return self.sol_stmt(b + 1);
        }
        while j < self.toks.len() {
            if self.opener(j) {
                j = self.pair[j]? + 1;
                continue;
            }
            if self.punct(j, ";") {
                return Some(j);
            }
            if self.closer(j) {
                return None;
            }
            j += 1;
        }
        None
    }
}

fn typed_name(w: &str) -> bool {
    let digits = ["uint", "int", "bytes"]
        .iter()
        .find_map(|t| w.strip_prefix(t));
    digits.is_some_and(|d| d.bytes().all(|b| b.is_ascii_digit()))
}
