use super::{K, Lexer, Model};

const DECLS: &[&str] = &[
    "func",
    "init",
    "deinit",
    "subscript",
    "class",
    "struct",
    "enum",
    "extension",
    "protocol",
    "actor",
    "var",
    "let",
    "get",
    "set",
    "willSet",
    "didSet",
    "_modify",
    "_read",
];
const STOPS: &[&str] = &[
    "return", "try", "await", "throw", "in", "else", "let", "var", "case", "where", "if", "guard",
    "while", "for", "switch", "is", "as", "some", "any", "inout", "repeat", "do", "catch",
];
const ACCESSORS: &[&str] = &["init", "subscript", "set", "willSet", "didSet"];
const CONTINUES: &[&str] = &[
    ",", ":", ".", "&", "=", "(", "<", "-", "+", "*", "/", "|", "where", "throws", "async",
];

impl Lexer<'_> {
    pub(super) fn swift_string(&mut self) -> bool {
        let s = self.bytes();
        let (l, at) = (self.l, self.i);
        let hashes = s[at..].iter().take_while(|&&b| b == b'#').count();
        if s.get(at + hashes) != Some(&b'"') {
            return false;
        }
        let multi = s[at + hashes..].starts_with(b"\"\"\"");
        let quotes = if multi { 3 } else { 1 };
        let close = [&b"\"".repeat(quotes)[..], &b"#".repeat(hashes)[..]].concat();
        let escape = [&b"\\"[..], &b"#".repeat(hashes)[..]].concat();
        self.i = at + hashes + quotes;
        loop {
            let s = self.bytes();
            if self.i >= s.len() {
                if multi && self.next_line() {
                    continue;
                }
                break;
            }
            if s[self.i..].starts_with(&escape) {
                self.i += escape.len();
                if self.bytes().get(self.i) == Some(&b'(') {
                    self.i += 1;
                    self.swift_hole();
                } else {
                    self.i += 1;
                }
                continue;
            }
            if s[self.i..].starts_with(&close) {
                self.i += close.len();
                break;
            }
            self.i += 1;
        }
        self.emit(l, at, 1, K::Str, self.l);
        true
    }

    fn swift_hole(&mut self) {
        let mut depth = 1;
        while !self.eof() {
            let s = self.bytes();
            if self.i >= s.len() {
                self.next_line();
                continue;
            }
            match s[self.i] {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        self.i += 1;
                        return;
                    }
                }
                b'"' | b'#' if self.swift_string() => {
                    self.out.pop();
                    continue;
                }
                _ => {}
            }
            self.i += 1;
        }
    }
}

impl Model<'_, '_> {
    pub(super) fn swift(&mut self, lines: &[String]) {
        let n = self.toks.len();
        let mut quiet = vec![false; n];
        let mut imports = None;
        for k in 0..n {
            let t = self.toks[k];
            if t.kind != K::Word || self.before(k) == "." {
                continue;
            }
            match t.text {
                "import" => {
                    if let Some(from) = self.import_start(k) {
                        imports = self.join_run(imports, (from, t.line), lines);
                    }
                }
                "if" | "guard" | "for" | "while" | "switch" | "do"
                    if !self.loop_tail(k, "repeat") =>
                {
                    let end = self.sw_stmt(k, &mut quiet);
                    self.add(t.line, end, false);
                }
                "repeat" if self.punct(k + 1, "{") => quiet[k + 1] = true,
                _ => {}
            }
        }
        self.end_run(imports);
        for k in 0..n {
            match self.tx(k) {
                "{" if self.toks[k].kind == K::Punct && !quiet[k] => self.sw_brace(k, &quiet),
                "(" | "[" if self.toks[k].kind == K::Punct => self.sw_bracket(k, &quiet),
                _ => {}
            }
        }
    }

    fn import_start(&self, k: usize) -> Option<usize> {
        let mut j = k;
        while j > 0
            && (self.tx(j - 1).starts_with('@')
                || (self.tx(j - 1) == ")"
                    && self.back[j - 1].is_some_and(|o| self.before(o).starts_with('@'))))
        {
            j = self.back[j - 1].map_or(j - 1, |o| o - 1);
        }
        self.first[j].then(|| self.toks[j].line)
    }

    fn glued(&self, a: usize, b: usize) -> bool {
        let (a, b) = (self.toks[a].text, self.toks[b].text);
        a.as_ptr() as usize + a.len() == b.as_ptr() as usize
    }

    fn declares(&self, k: usize, words: &[&str]) -> bool {
        self.word_at(k) && words.contains(&self.tx(k)) && self.before(k) != "."
    }

    fn sw_body(&self, k: usize) -> Option<usize> {
        let mut j = k + 1;
        while j < self.toks.len() {
            if self.punct(j, "{") {
                return Some(j);
            }
            if self.punct(j, "(") || self.punct(j, "[") {
                j = self.pair[j]? + 1;
                continue;
            }
            if self.closer(j) {
                return None;
            }
            j += 1;
        }
        None
    }

    fn sw_stmt(&mut self, k: usize, quiet: &mut [bool]) -> Option<usize> {
        let mut body = self.sw_body(k)?;
        quiet[body] = true;
        if self.tx(k) == "switch" {
            self.sw_entries(body);
        }
        loop {
            let close = self.pair[body]?;
            let next = match self.tx(close + 1) {
                "else" if self.tx(k) == "if" => close + 1,
                "catch" if self.tx(k) == "do" => close + 1,
                _ => return Some(self.end_of(close)),
            };
            body = self.sw_body(next)?;
            quiet[body] = true;
        }
    }

    fn sw_entries(&mut self, open: usize) {
        let Some(close) = self.pair[open] else {
            return;
        };
        let mut starts = Vec::new();
        let mut j = open + 1;
        while j < close {
            let entry = match self.tx(j) {
                "case" | "default" => self.first[j],
                w => w == "@unknown" && self.first[j] && self.tx(j + 1) == "default",
            };
            if entry {
                starts.push(j);
            }
            if self.opener(j)
                && let Some(c) = self.pair[j]
            {
                j = c;
            }
            j += 1;
        }
        for (i, &s) in starts.iter().enumerate() {
            let next = starts.get(i + 1).copied().unwrap_or(close);
            let end = (self.end_of(next - 1) + 1).min(self.toks[next].line);
            self.add(self.toks[s].line, Some(end), false);
        }
    }

    fn sw_brace(&mut self, k: usize, quiet: &[bool]) {
        let line = self.toks[k].line;
        let decl = self.sw_decl(k);
        self.add(line, self.closing(k), decl);
        if decl || k == 0 || self.first[k] {
            return;
        }
        let p = self.toks[k - 1];
        let callee = match p.text {
            ")" | "]" => return,
            "?" | "!" | ">" => true,
            _ => p.kind == K::Word && !STOPS.contains(&p.text) && !p.text.starts_with('#'),
        };
        if callee && !quiet[k] {
            let start = self.sw_chain_start(k - 1);
            let end = self.trailing_end(k);
            self.add(self.toks[start].line, Some(self.end_of(end)), false);
        }
    }

    fn sw_bracket(&mut self, k: usize, quiet: &[bool]) {
        let Some(c) = self.pair[k] else {
            return;
        };
        let line = self.toks[k].line;
        let callee = k.checked_sub(1).filter(|&p| {
            let t = self.toks[p];
            t.line == line
                && match t.text {
                    ")" | "]" | "}" | "?" | "!" => true,
                    ">" => self.before(p) != "-",
                    w if w.starts_with('@') => self.glued(p, k),
                    _ => t.kind == K::Word && !STOPS.contains(&t.text),
                }
        });
        let Some(mut p) = callee else {
            self.add(line, Some(self.end_of(c)), false);
            return;
        };
        if self.tx(p) == ">" {
            match self.generic_open(p) {
                Some(o) if o > 1 && self.tx(o - 2) == "." => p = o - 1,
                _ => return,
            }
        }
        if self.tx(p).starts_with(['@', '#']) || self.sw_params(p) {
            return;
        }
        let start = self.sw_chain_start(p);
        let mut end = c;
        if self.tx(k) == "(" && self.punct(c + 1, "{") && !quiet[c + 1] && !self.first[c + 1] {
            end = self.trailing_end(c + 1);
        }
        self.add(self.toks[start].line, Some(self.end_of(end)), false);
    }

    fn generic_open(&self, close: usize) -> Option<usize> {
        let mut depth = 0;
        for j in (0..=close).rev() {
            match self.tx(j) {
                ">" => depth += 1,
                "<" if depth == 1 => return Some(j),
                "<" => depth -= 1,
                "(" | "{" | ";" => return None,
                _ => {}
            }
        }
        None
    }

    fn sw_params(&self, mut p: usize) -> bool {
        if matches!(self.tx(p), "?" | "!") && p > 0 {
            p -= 1;
        }
        if self.tx(p) == ">" {
            while p > 0 && self.tx(p) != "<" {
                p -= 1;
            }
            p = p.saturating_sub(1);
        }
        let lead = p.checked_sub(1).map_or("", |b| self.tx(b));
        matches!(lead, "func" | "case") || self.declares(p, ACCESSORS)
    }

    fn trailing_end(&self, open: usize) -> usize {
        let mut c = self.pair[open].unwrap_or(open);
        while self.toks.get(c + 1).is_some_and(|t| t.kind == K::Word)
            && self.punct(c + 2, ":")
            && self.punct(c + 3, "{")
        {
            c = self.pair[c + 3].unwrap_or(c + 3);
        }
        c
    }

    fn sw_chain_start(&self, mut j: usize) -> usize {
        loop {
            if self.closer(j) {
                j = self.back[j].unwrap_or(j);
            }
            let Some(p) = j.checked_sub(1) else {
                return j;
            };
            let (pt, jt) = (self.toks[p], self.toks[j]);
            let operand = |o: usize| {
                let t = self.toks[o];
                t.kind == K::Str
                    || matches!(t.text, ")" | "]" | "}")
                    || (t.kind == K::Word && !STOPS.contains(&t.text))
            };
            let arith = pt.kind == K::Punct
                && matches!(pt.text, "+" | "-" | "*" | "/" | "%")
                && p > 0
                && operand(p - 1)
                && !self.punct(p - 1, pt.text);
            if arith {
                j = p - 1;
                continue;
            }
            if jt.line > pt.end && jt.text != "." && pt.text != "." {
                return j;
            }
            let callee = matches!(pt.text, ")" | "]" | "}" | "?" | "!")
                || pt.kind == K::Str
                || (pt.kind == K::Word && !STOPS.contains(&pt.text));
            let link =
                pt.text == "." || (matches!(jt.text, "." | "?" | "!" | "(" | "[" | "{") && callee);
            if !link {
                return j;
            }
            j = p;
        }
    }

    fn sw_decl(&self, k: usize) -> bool {
        let mut j = k;
        while j > 0 {
            let p = j - 1;
            let (pt, jt) = (self.toks[p], self.toks[j]);
            if jt.line > pt.end && !CONTINUES.contains(&pt.text) && !CONTINUES.contains(&jt.text) {
                return false;
            }
            if self.closer(p) {
                j = self.back[p].unwrap_or(p);
                continue;
            }
            if self.declares(p, DECLS) {
                return true;
            }
            if matches!(
                pt.text,
                "{" | "}" | ";" | "=" | "in" | "return" | "(" | "[" | ","
            ) {
                return false;
            }
            j = p;
        }
        false
    }
}
