use super::{K, Lexer, Model};

const MODIFIERS: &[&str] = &[
    "public",
    "private",
    "protected",
    "static",
    "abstract",
    "final",
    "readonly",
];
const STATEMENTS: &[&str] = &[
    "if", "switch", "while", "for", "foreach", "match", "try", "do",
];
const NOT_SUBJECT: &[&str] = &[
    "return", "yield", "echo", "print", "case", "and", "or", "xor", "else", "throw", "new", "in",
    "as", "use", "fn",
];

impl Lexer<'_> {
    pub(super) fn php_quote(&mut self) -> bool {
        let s = self.bytes();
        let (l, at) = (self.l, self.i);
        match s[at] {
            b'#' if s.get(at + 1) != Some(&b'[') => self.i = s.len(),
            b'<' if s[at..].starts_with(b"<<<") => return self.heredoc(),
            q @ (b'\'' | b'"') => {
                self.i = at + 1;
                loop {
                    let s = self.bytes();
                    if self.i >= s.len() {
                        if self.next_line() {
                            continue;
                        }
                        break;
                    }
                    match s[self.i] {
                        b'\\' => self.i += 2,
                        c if c == q => {
                            self.i += 1;
                            break;
                        }
                        _ => self.i += 1,
                    }
                }
                self.emit(l, at, 1, K::Str, self.l);
            }
            _ => return false,
        }
        true
    }

    fn heredoc(&mut self) -> bool {
        let (l, at) = (self.l, self.i);
        let s = self.bytes();
        let id: Vec<u8> = (s[at + 3..].iter())
            .copied()
            .filter(|&b| !matches!(b, b'\'' | b'"' | b' '))
            .take_while(|&b| b.is_ascii_alphanumeric() || b == b'_')
            .collect();
        if id.is_empty() {
            return false;
        }
        while self.next_line() {
            let s = self.bytes().trim_ascii_start();
            let end = s.starts_with(&id)
                && !s
                    .get(id.len())
                    .is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'_');
            if end {
                self.i = self.bytes().len() - s.len() + id.len();
                break;
            }
        }
        self.emit(l, at, 1, K::Str, self.l);
        true
    }
}

impl Model<'_, '_> {
    pub(super) fn php(&mut self, lines: &[String]) {
        let n = self.toks.len();
        let mut uses = None;
        let mut stack: Vec<bool> = Vec::new();
        for k in 0..n {
            let t = self.toks[k];
            if self.closer(k) {
                stack.pop();
            }
            if self.opener(k) {
                let space = self.tx(k) == "{" && self.php_namespace_body(k);
                stack.push(stack.last().copied().unwrap_or(true) && space);
            }
            if t.kind != K::Word || self.php_member(k) {
                self.php_end_uses(&mut uses, k);
                if t.kind == K::Punct {
                    self.php_bracket(k);
                }
                continue;
            }
            match t.text {
                "use" if self.first[k] && stack.last().copied().unwrap_or(true) => {
                    let end = self.php_semicolon(k);
                    uses = self.join_run(uses, (t.line, self.end_of(end)), lines);
                    continue;
                }
                "while" if self.do_while(k) => {}
                w if STATEMENTS.contains(&w) => {
                    let end = self.php_stmt_end(k);
                    self.add(t.line, end.map(|e| self.end_of(e)), false);
                }
                "function" if self.word_next(k) => {
                    let start = self.php_decl_start(k);
                    let end = self.php_decl_end(k);
                    self.add(self.toks[start].line, end.map(|e| self.end_of(e)), true);
                }
                "class" | "interface" | "trait" | "enum"
                    if self.word_next(k) && !matches!(self.before(k), "new" | "::") =>
                {
                    let start = self.php_decl_start(k);
                    let body = (k..n).find(|&j| self.punct(j, "{"));
                    let end = body.and_then(|b| self.pair[b]);
                    self.add(self.toks[start].line, end.map(|e| self.end_of(e)), true);
                }
                "static" if self.first[k] && self.tx(k + 1).starts_with('$') => {
                    let end = self.php_semicolon(k);
                    self.add(t.line, Some(self.end_of(end)), false);
                }
                "array" if self.punct(k + 1, "(") => {
                    self.add(t.line, self.closing(k + 1), false);
                }
                _ => {}
            }
            self.php_end_uses(&mut uses, k);
        }
        self.end_run(uses);
    }

    fn php_member(&self, k: usize) -> bool {
        let pair = |a: &str| k > 1 && self.tx(k - 2) == a;
        match self.before(k) {
            ">" => pair("-"),
            ":" => pair(":"),
            "\\" | "function" => true,
            _ => false,
        }
    }

    fn php_end_uses(&mut self, uses: &mut Option<(usize, usize)>, k: usize) {
        if uses.is_some_and(|(_, e)| self.toks[k].line > e) {
            self.end_run(uses.take());
        }
    }

    fn php_namespace_body(&self, k: usize) -> bool {
        let mut j = k;
        while j > 0 && !matches!(self.tx(j - 1), ";" | "{" | "}") {
            j -= 1;
        }
        self.tx(j) == "namespace"
    }

    fn word_next(&self, k: usize) -> bool {
        self.toks.get(k + 1).is_some_and(|t| t.kind == K::Word)
            || (self.punct(k + 1, "&") && self.toks.get(k + 2).is_some_and(|t| t.kind == K::Word))
    }

    fn php_bracket(&mut self, k: usize) {
        if !self.punct(k, "[") {
            return;
        }
        let Some(p) = k.checked_sub(1) else {
            return;
        };
        let pt = self.toks[p];
        let subscript = match pt.kind {
            K::Word => !NOT_SUBJECT.contains(&pt.text) && pt.text != "#",
            K::Str => true,
            _ => matches!(pt.text, ")" | "]" | "}") && !self.cast(p),
        };
        if !subscript && pt.text != "#" {
            self.add(self.toks[k].line, self.closing(k), false);
        }
    }

    fn cast(&self, close: usize) -> bool {
        let cast = ["int", "bool", "float", "string", "array", "object"];
        close > 1 && self.punct(close - 2, "(") && cast.contains(&self.tx(close - 1))
    }

    fn php_semicolon(&self, k: usize) -> usize {
        let mut j = k;
        while j + 1 < self.toks.len() && !self.punct(j, ";") {
            if self.opener(j)
                && let Some(c) = self.pair[j]
            {
                j = c;
            }
            j += 1;
        }
        j.min(self.toks.len() - 1)
    }

    fn pair_of(&self, k: usize) -> Option<usize> {
        self.pair.get(k).copied().flatten()
    }

    fn php_decl_start(&self, k: usize) -> usize {
        let mut s = k;
        while s > 0 {
            let p = s - 1;
            if MODIFIERS.contains(&self.tx(p)) {
                s = p;
            } else if self.tx(p) == "]"
                && let Some(o) = self.back[p]
                && o > 0
                && self.tx(o - 1) == "#"
            {
                s = o - 1;
            } else {
                break;
            }
        }
        s
    }

    fn php_decl_end(&self, k: usize) -> Option<usize> {
        let mut j = k + 1;
        while j < self.toks.len() {
            match self.tx(j) {
                "{" => return self.pair[j],
                ";" => return Some(j),
                "(" | "[" => j = self.pair[j]?,
                "}" => return None,
                _ => {}
            }
            j += 1;
        }
        None
    }

    fn php_body_end(&self, j: usize, kw: &str) -> Option<usize> {
        match self.tx(j) {
            "{" => self.pair[j],
            ":" => self.php_alt_end(j, kw),
            _ if STATEMENTS.contains(&self.tx(j)) => self.php_stmt_end(j),
            _ => Some(self.php_semicolon(j)),
        }
    }

    fn php_alt_end(&self, colon: usize, kw: &str) -> Option<usize> {
        let close = format!("end{kw}");
        let mut depth = 0usize;
        let mut j = colon + 1;
        while j < self.toks.len() {
            let t = self.tx(j);
            if t == kw && self.punct(j + 1, "(") {
                let c = self.pair[j + 1]?;
                if self.punct(c + 1, ":") {
                    depth += 1;
                }
                j = c + 1;
                continue;
            }
            if t == close {
                if depth == 0 {
                    return Some(self.php_semicolon(j));
                }
                depth -= 1;
            }
            j += 1;
        }
        None
    }

    fn php_stmt_end(&self, k: usize) -> Option<usize> {
        let kw = self.tx(k);
        let mut j = k + 1;
        if self.punct(j, "(") {
            j = self.pair[j]? + 1;
        }
        if kw == "if" && self.punct(j, ":") {
            return self.php_alt_end(j, "if");
        }
        let mut end = self.php_body_end(j, kw)?;
        loop {
            let next = end + 1;
            match (kw, self.tx(next)) {
                ("if", "elseif") => {
                    let c = self.pair_of(next + 1)?;
                    end = self.php_body_end(c + 1, kw)?;
                }
                ("if", "else") => end = self.php_body_end(next + 1, kw)?,
                ("try", "catch") => {
                    let c = self.pair_of(next + 1)?;
                    end = self.pair_of(c + 1)?;
                }
                ("try", "finally") => end = self.pair_of(next + 1)?,
                ("do", "while") => end = self.php_semicolon(next),
                _ => return Some(end),
            }
        }
    }
}
