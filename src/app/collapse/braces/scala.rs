use super::super::indent;
use super::{K, Lexer, Model};

const MODIFIERS: &[&str] = &[
    "private",
    "protected",
    "override",
    "implicit",
    "final",
    "sealed",
    "abstract",
    "lazy",
    "case",
    "inline",
    "transparent",
    "opaque",
    "open",
    "infix",
];
const KEYWORDS: &[&str] = &[
    "if",
    "else",
    "while",
    "for",
    "do",
    "try",
    "catch",
    "finally",
    "match",
    "case",
    "yield",
    "return",
    "throw",
    "new",
    "extends",
    "with",
    "then",
    "def",
    "val",
    "var",
    "class",
    "object",
    "trait",
    "import",
    "package",
    "type",
    "given",
    "using",
    "implicit",
    "lazy",
    "override",
    "private",
    "protected",
    "final",
    "sealed",
    "abstract",
];
const CONT_END: &[&str] = &[
    ",", "(", "[", "{", "=", ".", "+", "-", "*", "/", "%", "&", "|", "^", "<", ">", ":", "!", "=>",
    "if", "else", "while", "for", "do", "try", "catch", "finally", "match", "yield", "then",
    "return", "throw", "new", "with", "extends",
];
const INDENT: &[&str] = &[
    "=", "=>", "else", "then", "do", "yield", "try", "finally", "return", "match",
];
const CONT_START: &[&str] = &[
    ".", "else", "catch", "finally", "match", "with", "extends", "yield", "then", "do", "=>", "|",
    "&", "=", ":",
];

impl Lexer<'_> {
    pub(super) fn scala_quote(&mut self) -> bool {
        let s = self.bytes();
        let (l, at) = (self.l, self.i);
        match s[at] {
            b'\'' => {
                let len = match s.get(at + 1) {
                    Some(b'\\') => s[at + 2..].iter().position(|&b| b == b'\'').map(|p| p + 3),
                    Some(_) => {
                        let ch = self.lines[l][at + 1..]
                            .chars()
                            .next()
                            .map_or(1, char::len_utf8);
                        (s.get(at + 1 + ch) == Some(&b'\'')).then_some(ch + 2)
                    }
                    None => None,
                };
                self.i += len.unwrap_or(1);
                if len.is_some() {
                    self.emit(l, at, 1, K::Str, l);
                }
                true
            }
            b'"' => {
                let interpolated = at > 0 && super::word(s[at - 1]);
                self.scala_string(interpolated);
                self.emit(l, at, 1, K::Str, self.l);
                true
            }
            _ => false,
        }
    }

    fn scala_string(&mut self, interpolated: bool) {
        let s = self.bytes();
        let triple = s[self.i..].starts_with(b"\"\"\"");
        self.i += if triple { 3 } else { 1 };
        loop {
            let s = self.bytes();
            if self.i >= s.len() {
                if triple && self.next_line() {
                    continue;
                }
                return;
            }
            if interpolated && s[self.i..].starts_with(b"${") {
                self.i += 2;
                self.scala_hole();
                continue;
            }
            if interpolated && s[self.i..].starts_with(b"$$") {
                self.i += 2;
                continue;
            }
            if !triple && s[self.i] == b'\\' {
                self.i += 2;
                continue;
            }
            if triple && s[self.i..].starts_with(b"\"\"\"") {
                self.i += 3;
                while self.bytes().get(self.i) == Some(&b'"') {
                    self.i += 1;
                }
                return;
            }
            if !triple && s[self.i] == b'"' {
                self.i += 1;
                return;
            }
            self.i += 1;
        }
    }

    fn scala_hole(&mut self) {
        let mut depth = 1;
        while !self.eof() {
            let s = self.bytes();
            if self.i >= s.len() {
                self.next_line();
                continue;
            }
            match s[self.i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        self.i += 1;
                        return;
                    }
                }
                b'"' => {
                    let interpolated = self.i > 0 && super::word(s[self.i - 1]);
                    self.scala_string(interpolated);
                    continue;
                }
                _ => {}
            }
            self.i += 1;
        }
    }
}

impl Model<'_, '_> {
    pub(super) fn scala(&mut self, lines: &[String]) {
        let n = self.toks.len();
        let mut parent: Vec<Option<usize>> = vec![None; n];
        let mut stack = Vec::new();
        for (k, up) in parent.iter_mut().enumerate() {
            if self.closer(k) {
                stack.pop();
            }
            *up = stack.last().copied();
            if self.opener(k) {
                stack.push(k);
            }
        }
        for (k, up) in parent.into_iter().enumerate() {
            let t = self.toks[k];
            if t.kind == K::Punct {
                if t.text == "{" && self.sc_call_block(k) {
                    self.add(t.line, self.closing(k), false);
                }
                continue;
            }
            if t.kind != K::Word || self.before(k) == "." {
                continue;
            }
            match t.text {
                "class" | "trait" | "object" => {
                    let start = self.toks[self.sc_start(k)].line;
                    let end = self.sc_template_end(k, lines);
                    self.add(start, Some(end), true);
                }
                "def" => {
                    let start = self.toks[self.sc_start(k)].line;
                    if let Some(end) = self.sc_def_end(k, lines) {
                        self.add(start, Some(end), true);
                    }
                }
                "val" if up.is_none_or(|p| self.tx(p) == "{") => {
                    let start = self.toks[self.sc_start(k)].line;
                    if let Some(end) = self.sc_def_end(k, lines) {
                        self.add(start, Some(end), false);
                    }
                }
                "import" if self.first[k] => {
                    let end = self.sc_expr_end(k + 1, lines);
                    self.add(t.line, Some(end), false);
                }
                "while" | "for" | "try" if !self.sc_loop_tail(k) => {
                    let end = self.sc_expr_end(k, lines);
                    self.add(t.line, Some(end), false);
                }
                "do" if self.sc_starts_expr(k) => {
                    let end = self.sc_expr_end(k, lines);
                    self.add(t.line, Some(end), false);
                }
                "match" if self.punct(k + 1, "{") => {
                    let start = self.toks[self.sc_scrutinee(k)].line;
                    self.add(start, self.closing(k + 1), false);
                }
                "match" if self.sc_braceless(k, lines) => {
                    let start = self.toks[self.sc_scrutinee(k)].line;
                    let end = self.sc_indented_end(k + 1, indent(&lines[t.line]), lines);
                    self.add(start, Some(end), false);
                }
                _ => {}
            }
        }
    }

    fn sc_loop_tail(&self, k: usize) -> bool {
        self.tx(k) == "while"
            && (k.checked_sub(1)).is_some_and(|p| {
                let mut j = p;
                while j > 0 && !self.first[j] {
                    j -= 1;
                }
                (j..=p).any(|i| self.tx(i) == "do")
                    || (self.punct(p, "}") && self.back[p].is_some_and(|o| self.before(o) == "do"))
            })
    }

    fn sc_starts_expr(&self, k: usize) -> bool {
        k == 0 || self.first[k] || matches!(self.before(k), "=" | "{" | "(" | ";" | "=>" | ",")
    }

    fn sc_start(&self, k: usize) -> usize {
        let mut s = k;
        while s > 0 {
            let p = self.tx(s - 1);
            if MODIFIERS.contains(&p) {
                s -= 1;
            } else if p == "]"
                && let Some(o) = self.back[s - 1]
                && matches!(self.before(o), "private" | "protected")
            {
                s = o - 1;
            } else if let Some(a) = self.sc_annotation(s - 1) {
                s = a;
            } else {
                break;
            }
        }
        s
    }

    fn sc_annotation(&self, mut j: usize) -> Option<usize> {
        while matches!(self.tx(j), ")" | "]") {
            j = self.back[j]?.checked_sub(1)?;
        }
        loop {
            if self.tx(j).starts_with('@') {
                return Some(j);
            }
            if !self.word_at(j) || j < 2 || self.tx(j - 1) != "." {
                return None;
            }
            j -= 2;
        }
    }

    fn sc_template_end(&self, k: usize, lines: &[String]) -> usize {
        let (mut j, mut last) = (k + 1, k);
        let head = indent(&lines[self.toks[k].line]);
        while j < self.toks.len() {
            if self.punct(j, "{") {
                return self.end_of(self.pair[j].unwrap_or(j));
            }
            if j > k + 1 && self.toks[j].line > self.toks[last].end {
                if self.tx(last) == ":" && indent(&lines[self.toks[j].line]) > head {
                    return self.sc_indented_end(j, head, lines);
                }
                let cont = CONT_START.contains(&self.tx(j)) || CONT_END.contains(&self.tx(last));
                if !cont {
                    return self.end_of(last);
                }
            }
            if self.opener(j)
                && let Some(c) = self.pair[j]
            {
                (j, last) = (c + 1, c);
                continue;
            }
            if self.closer(j) || self.punct(j, ";") {
                return self.end_of(last);
            }
            (last, j) = (j, j + 1);
        }
        self.end_of(last)
    }

    fn sc_indented_end(&self, mut j: usize, head: usize, lines: &[String]) -> usize {
        while j < self.toks.len() {
            if self.closer(j) || (self.first[j] && indent(&lines[self.toks[j].line]) <= head) {
                return self.sc_outdent(j, head, lines);
            }
            j = match self.pair[j] {
                Some(c) if self.opener(j) => c + 1,
                _ => j + 1,
            };
        }
        lines.len() - 1
    }

    fn sc_outdent(&self, j: usize, head: usize, lines: &[String]) -> usize {
        let (line, at) = (self.toks[j].line, indent(&lines[self.toks[j].line]));
        if self.sc_marker(j) && at == head {
            return line;
        }
        line - usize::from(at == 0 && self.first[j])
    }

    fn sc_braceless(&self, k: usize, lines: &[String]) -> bool {
        let line = self.toks[k].line;
        self.toks
            .get(k + 1)
            .is_some_and(|t| t.line > line && indent(&lines[t.line]) > indent(&lines[line]))
    }

    fn sc_marker(&self, j: usize) -> bool {
        let line = self.toks[j].line;
        self.tx(j) == "end"
            && self.first[j]
            && self
                .toks
                .get(j + 1)
                .is_some_and(|t| t.kind == K::Word && t.line == line)
            && self.toks.get(j + 2).is_none_or(|t| t.line > line)
    }

    fn sc_def_end(&self, k: usize, lines: &[String]) -> Option<usize> {
        let mut j = k + 1;
        while j < self.toks.len() {
            if self.punct(j, "=") {
                return Some(self.sc_expr_end(j, lines));
            }
            if self.punct(j, "{") && self.tx(k) == "def" {
                return self.closing(j);
            }
            if self.opener(j) {
                j = self.pair[j]? + 1;
                continue;
            }
            if self.closer(j) || self.punct(j, ";") {
                return None;
            }
            if self.toks[j].line > self.toks[j - 1].end
                && !CONT_END.contains(&self.tx(j - 1))
                && !CONT_START.contains(&self.tx(j))
            {
                return None;
            }
            j += 1;
        }
        None
    }

    fn sc_expr_end(&self, from: usize, lines: &[String]) -> usize {
        let n = self.toks.len();
        let col = |j: usize| indent(&lines[self.toks[j].line]);
        let head = col(from.min(n - 1));
        let (mut j, mut last) = (from, from.min(n - 1));
        let mut blocks: Vec<usize> = Vec::new();
        let outdent = |j: usize| self.toks[j].line - usize::from(col(j) == 0);
        while j < n {
            if j > from && self.toks[j].line > self.toks[last].end {
                let marker = last > 0 && self.sc_marker(last - 1);
                let cond = self.tx(last) == ")"
                    && self.back[last]
                        .is_some_and(|o| matches!(self.before(o), "if" | "while" | "for"));
                let opens = !marker && (cond || INDENT.contains(&self.tx(last)));
                let cont = !marker
                    && (opens
                        || CONT_END.contains(&self.tx(last))
                        || CONT_START.contains(&self.tx(j))
                        || (self.tx(last) == "}"
                            && self.back[last].is_some_and(|o| self.before(o) == "for")));
                let at = col(j);
                if cont {
                    blocks.retain(|&c| c <= at);
                    if opens && blocks.last().is_none_or(|&c| c < at) {
                        blocks.push(at);
                    }
                } else {
                    let had = !blocks.is_empty();
                    blocks.retain(|&c| c <= at);
                    if blocks.is_empty() {
                        return match had {
                            true => self.sc_outdent(j, head, lines),
                            false => self.toks[last].end,
                        };
                    }
                }
            }
            if self.opener(j) {
                match self.pair[j] {
                    Some(c) => {
                        (last, j) = (c, c + 1);
                        continue;
                    }
                    None => break,
                }
            }
            if self.closer(j) || self.punct(j, ";") {
                if !blocks.is_empty() && self.first[j] {
                    return outdent(j);
                }
                break;
            }
            (last, j) = (j, j + 1);
        }
        match j >= n && !blocks.is_empty() {
            true => lines.len() - 1,
            false => self.toks[last].end,
        }
    }

    fn sc_scrutinee(&self, k: usize) -> usize {
        let mut s = k;
        while s > 0 {
            let p = s - 1;
            let t = self.toks[p];
            if self.first[s] && self.tx(s) != "." {
                break;
            }
            if self.closer(p) {
                s = self.back[p].unwrap_or(p);
            } else if (t.kind == K::Word && !KEYWORDS.contains(&t.text))
                || t.kind == K::Str
                || t.text == "."
            {
                s = p;
            } else {
                break;
            }
        }
        s
    }

    fn sc_call_block(&self, k: usize) -> bool {
        let Some(p) = k.checked_sub(1) else {
            return false;
        };
        if self.tx(k + 1) == "case" {
            return false;
        }
        let pt = self.toks[p];
        if matches!(pt.text, ")" | "]") {
            let Some(o) = self.back[p] else {
                return false;
            };
            let head = o.checked_sub(1).map(|h| self.toks[h]);
            return head.is_some_and(|h| {
                (h.kind == K::Word && !KEYWORDS.contains(&h.text) && self.sc_callee_word(o - 1))
                    || matches!(h.text, ")" | "]")
            }) && !self.sc_in_header(o);
        }
        pt.kind == K::Word && !KEYWORDS.contains(&pt.text) && self.sc_callee_word(p)
    }

    fn sc_callee_word(&self, w: usize) -> bool {
        if self.first[w] || self.before(w) == "." || KEYWORDS.contains(&self.before(w)) {
            return true;
        }
        let b = self.toks[w - 1];
        b.kind == K::Punct && !matches!(b.text, ")" | "]" | "}" | "@" | ":")
    }

    fn sc_in_header(&self, o: usize) -> bool {
        let mut j = o;
        while j > 0 {
            j -= 1;
            if self.closer(j) && self.tx(j) != "}" {
                j = self.back[j].unwrap_or(j);
                continue;
            }
            match self.tx(j) {
                "def" | "class" | "object" | "trait" | "new" | "extends" | "with" => return true,
                "=" | "{" | "}" | ";" | "(" | "," | "=>" => return false,
                _ => {}
            }
            if self.first[j] {
                return false;
            }
        }
        false
    }
}
