use super::{K, Lexer, Model};

const KEYWORDS: &[&str] = &[
    "if",
    "else",
    "for",
    "while",
    "switch",
    "case",
    "default",
    "catch",
    "synchronized",
    "return",
    "throw",
    "assert",
    "in",
    "instanceof",
    "as",
    "new",
    "yield",
    "def",
    "import",
    "package",
    "class",
    "interface",
    "enum",
    "trait",
    "try",
    "finally",
    "do",
    "break",
    "continue",
    "extends",
    "implements",
    "throws",
];
const TYPED: &[&str] = &[
    "def",
    "void",
    "public",
    "private",
    "protected",
    "static",
    "final",
    "abstract",
    "synchronized",
    "native",
    "transient",
    "boolean",
    "byte",
    "char",
    "short",
    "int",
    "long",
    "float",
    "double",
];

impl Lexer<'_> {
    pub(super) fn groovy_quote(&mut self) -> bool {
        let s = self.bytes();
        let (l, at) = (self.l, self.i);
        match s[at] {
            b'"' => return self.kt_string(),
            b'\'' if s[at..].starts_with(b"'''") => self.skip_past(at + 3, b"'''"),
            b'\'' => {
                self.string(b'\'');
                return true;
            }
            b'$' if s[at..].starts_with(b"$/") => self.skip_past(at + 2, b"/$"),
            b'/' if self.slashy() => {}
            _ => return false,
        }
        self.emit(l, at, 1, K::Str, self.l);
        true
    }

    fn slashy(&mut self) -> bool {
        let operand = match self.out.last() {
            None => true,
            Some(p) if p.kind == K::Punct => !matches!(p.text, ")" | "]" | "}"),
            Some(p) => p.kind == K::Word && KEYWORDS.contains(&p.text),
        };
        let s = self.bytes();
        let mut j = self.i + 1;
        while operand && j < s.len() {
            match s[j] {
                b'\\' => j += 2,
                b'/' => {
                    self.i = j + 1;
                    return true;
                }
                _ => j += 1,
            }
        }
        false
    }
}

impl Model<'_, '_> {
    pub(super) fn groovy(&mut self) {
        let mut glued = Vec::new();
        for k in 0..self.toks.len() {
            if let Some((args, end)) = self.gv_juxt(k) {
                glued.push(args);
                self.add(self.toks[args].line, Some(end), false);
            }
        }
        for k in 0..self.toks.len() {
            let t = self.toks[k];
            if t.kind != K::Punct {
                continue;
            }
            match t.text {
                "{" if !self.gv_switch(k) => {
                    let def = self.gv_def(k);
                    self.add(t.line, self.closing(k), def);
                }
                "(" if glued.contains(&k) => {}
                "(" if self.gv_call(k) => {
                    let c = self.pair[k].map_or(k, |c| c + 1);
                    let end = if self.punct(c, "{") { c } else { k };
                    self.add(t.line, self.closing(end), false);
                }
                "[" if !self.gv_index(k) => self.add(t.line, self.closing(k), false),
                _ => {}
            }
        }
    }

    fn gv_juxt(&self, k: usize) -> Option<(usize, usize)> {
        if !self.word_at(k) || !self.first[k] || KEYWORDS.contains(&self.tx(k)) {
            return None;
        }
        if k > 0 && (self.gv_op(k - 1) || matches!(self.before(k), "," | "(" | "[" | "." | "=")) {
            return None;
        }
        if self.punct(k + 1, ":")
            && self
                .toks
                .get(k + 2)
                .is_some_and(|t| t.line > self.toks[k].line)
        {
            let bare = k > 0
                && ((self.word_at(k - 1) && !KEYWORDS.contains(&self.before(k)))
                    || (self.toks[k - 1].kind == K::Str && self.before(k - 1) == "."));
            return bare.then(|| (k, self.gv_stmt_end(k + 2)));
        }
        let mut j = k;
        while self.punct(j + 1, ".") && self.word_at(j + 2) {
            j += 2;
        }
        let a = j + 1;
        let t = self.toks.get(a).filter(|t| t.line == self.toks[j].line)?;
        if t.kind == K::Str {
            return Some((a, self.gv_stmt_end(a)));
        }
        let c = self.pair[a].filter(|_| self.punct(a, "(") || self.punct(a, "["))?;
        let mut n = c + 1;
        if self.punct(n, ".") && self.word_at(n + 1) {
            n += 2;
        }
        (n < self.toks.len() && self.gv_op(n)).then(|| (a, self.gv_stmt_end(a)))
    }

    fn gv_op(&self, k: usize) -> bool {
        let t = self.toks[k];
        let next = self.tx(k + 1);
        match t.kind {
            K::Word => matches!(t.text, "instanceof" | "in" | "as"),
            K::Punct => match t.text {
                "=" => matches!(next, "=" | "~") || matches!(self.before(k), "=" | "!" | "<" | ">"),
                "~" => self.before(k) == "=",
                "!" => next == "=",
                "?" => !matches!(next, "." | "["),
                "<" | ">" | "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^" => true,
                _ => false,
            },
            _ => false,
        }
    }

    fn gv_stmt_end(&self, from: usize) -> usize {
        let mut j = from;
        let mut end = self.end_of(j);
        while j < self.toks.len() {
            if self.opener(j)
                && let Some(c) = self.pair[j]
            {
                j = c;
            }
            end = end.max(self.end_of(j));
            let n = j + 1;
            if n >= self.toks.len() || self.closer(n) || self.punct(n, ";") {
                break;
            }
            let wrap = self.gv_op(j) || self.punct(j, ",") || self.punct(j, ".");
            let lead = self.punct(n, ".") || (self.punct(n, "?") && self.punct(n + 1, "."));
            let lead = lead || (self.gv_op(n) && !matches!(self.tx(n), "-" | "+" | "*" | "!"));
            if self.toks[n].line > end && !wrap && !lead {
                break;
            }
            j = n;
        }
        end
    }

    fn gv_switch(&self, k: usize) -> bool {
        k > 0
            && self.punct(k - 1, ")")
            && self.back[k - 1].is_some_and(|o| self.before(o) == "switch")
    }

    fn gv_call(&self, k: usize) -> bool {
        let Some(p) = k.checked_sub(1) else {
            return false;
        };
        self.word_at(p) && !KEYWORDS.contains(&self.tx(p)) && !self.gv_params(k)
    }

    fn gv_params(&self, k: usize) -> bool {
        let name = k - 1;
        let before = self.before(name);
        if before == "." {
            return false;
        }
        let capital = |w: &str| w.starts_with(|c: char| c.is_ascii_uppercase());
        let typed = name > 0
            && !self.first[name]
            && (TYPED.contains(&before)
                || matches!(before, ">" | "]")
                || (self.word_at(name - 1) && capital(before)));
        let after = self.pair[k].map_or("", |c| self.tx(c + 1));
        let constructor = (self.first[name] || TYPED.contains(&before))
            && capital(self.tx(name))
            && matches!(after, "{" | "throws");
        typed || constructor
    }

    fn gv_index(&self, k: usize) -> bool {
        let Some(p) = k.checked_sub(1) else {
            return false;
        };
        let t = self.toks[p];
        !self.first[k]
            && match t.kind {
                K::Word => !KEYWORDS.contains(&t.text),
                K::Str => true,
                _ => matches!(t.text, ")" | "]" | "}"),
            }
    }

    fn gv_def(&self, k: usize) -> bool {
        let mut j = k;
        while j > 0 {
            let p = j - 1;
            if self.punct(p, ")") {
                let Some(o) = self.back[p] else {
                    return false;
                };
                if o > 0 && self.word_at(o - 1) && self.gv_params(o) {
                    return true;
                }
                j = o;
                continue;
            }
            if self.closer(p) || self.opener(p) || self.punct(p, ";") || self.punct(p, "=") {
                return false;
            }
            if matches!(self.tx(p), "class" | "interface" | "enum" | "trait")
                && self.before(p) != "."
            {
                return true;
            }
            j = p;
        }
        false
    }
}
