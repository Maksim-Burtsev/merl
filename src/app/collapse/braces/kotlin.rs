use super::{K, Lexer, Model};

const MODIFIERS: &[&str] = &[
    "public",
    "private",
    "protected",
    "internal",
    "override",
    "open",
    "abstract",
    "final",
    "inline",
    "suspend",
    "operator",
    "infix",
    "tailrec",
    "external",
    "lateinit",
    "data",
    "sealed",
    "enum",
    "annotation",
    "inner",
    "value",
    "const",
    "expect",
    "actual",
];
const CONT_END: &[&str] = &[
    ",", "(", "[", "{", "=", ".", ">", "+", "-", "*", "/", "%", "&", "|", "?", ":", "!", "<",
];
const CONT_START: &[&str] = &[
    ".", "?", ":", "&", "|", "+", "*", "/", "%", "=", ")", "]", "}", "else", "as", "is", "in",
    "catch", "finally",
];

impl Lexer<'_> {
    pub(super) fn kt_string(&mut self) -> bool {
        let s = self.bytes();
        let (l, at) = (self.l, self.i);
        if s.get(at) != Some(&b'"') {
            return false;
        }
        let raw = s[at..].starts_with(b"\"\"\"");
        self.i = at + if raw { 3 } else { 1 };
        let mut depth = 0usize;
        loop {
            let s = self.bytes();
            if self.i >= s.len() {
                if (raw || depth > 0) && self.next_line() {
                    continue;
                }
                break;
            }
            let c = s[self.i];
            if depth > 0 {
                match c {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    b'"' | b'\'' => {
                        self.kt_inner(c);
                        continue;
                    }
                    _ => {}
                }
                self.i += 1;
                continue;
            }
            if s[self.i..].starts_with(b"${") {
                depth = 1;
                self.i += 2;
                continue;
            }
            match c {
                b'\\' if !raw => self.i += 2,
                b'"' if raw && s[self.i..].starts_with(b"\"\"\"") => {
                    self.i += 3;
                    while self.bytes().get(self.i) == Some(&b'"') {
                        self.i += 1;
                    }
                    break;
                }
                b'"' if !raw => {
                    self.i += 1;
                    break;
                }
                _ => self.i += 1,
            }
        }
        self.emit(l, at, 1, K::Str, self.l);
        true
    }

    fn kt_inner(&mut self, q: u8) {
        if q == b'"' {
            self.kt_string();
            self.out.pop();
        } else {
            self.string(q);
            self.out.pop();
        }
    }
}

impl Model<'_, '_> {
    pub(super) fn kotlin(&mut self) {
        let n = self.toks.len();
        let mut quiet = vec![false; n];
        let mut arrows = Vec::new();
        self.kt_imports();
        for k in 0..n {
            let t = self.toks[k];
            let prev = self.before(k);
            match t.text {
                "when" if t.kind == K::Word => {
                    let open = if self.punct(k + 1, "(") {
                        self.pair[k + 1].map(|c| c + 1)
                    } else {
                        Some(k + 1)
                    };
                    if let Some(o) = open.filter(|&o| self.punct(o, "{")) {
                        quiet[o] = true;
                        self.add(t.line, self.closing(o), false);
                        let mut j = o + 1;
                        while let Some(c) = self.pair[o].filter(|&c| j < c) {
                            if self.opener(j) {
                                j = self.pair[j].unwrap_or(j);
                            } else if self.punct(j, "-") && self.punct(j + 1, ">") {
                                arrows.push(j);
                            }
                            j += 1;
                            let _ = c;
                        }
                    }
                }
                "init" if t.kind == K::Word && self.punct(k + 1, "{") => {
                    quiet[k + 1] = true;
                    self.add(t.line, self.closing(k + 1), false);
                }
                "constructor" if t.kind == K::Word && self.punct(k + 1, "(") => {
                    let start = self.kt_member_start(k);
                    let lead = start.checked_sub(1).map_or("", |s| self.tx(s));
                    let Some(close) = self.pair[k + 1] else {
                        continue;
                    };
                    if !matches!(lead, "" | ";" | "{" | "}") {
                        self.add(self.toks[start].line, Some(self.end_of(close)), false);
                        continue;
                    }
                    let mut j = close + 1;
                    if self.punct(j, ":") && self.punct(j + 2, "(") {
                        j = self.pair[j + 2].map_or(j, |c| c + 1);
                    }
                    let end = if self.punct(j, "{") {
                        quiet[j] = true;
                        self.pair[j]
                    } else {
                        Some(j - 1)
                    };
                    self.add(self.toks[start].line, end.map(|e| self.end_of(e)), true);
                }
                "(" if self.kt_class_params(k) => self.add(t.line, self.closing(k), false),
                "-" if self.punct(k + 1, ">") && arrows.contains(&k) => {
                    let first = k + 2;
                    if first < n && !self.punct(first, "{") {
                        let end = self.kt_expr_end(first, true);
                        self.add(self.toks[first].line, Some(self.end_of(end)), false);
                    }
                }
                "=" if t.kind == K::Punct && self.kt_body(k) => {
                    let end = self.kt_expr_end(k + 1, false);
                    self.add(t.line, Some(self.end_of(end)), true);
                }
                "{" if t.kind == K::Punct && !quiet[k] => {
                    let skip = match prev {
                        "try" | "finally" => true,
                        ")" => self.back[k - 1].is_some_and(|o| self.before(o) == "catch"),
                        _ => false,
                    };
                    if !skip {
                        let def = self.kt_def(k);
                        self.add(t.line, self.closing(k), def);
                    }
                }
                _ => {}
            }
        }
    }

    fn kt_class_params(&self, k: usize) -> bool {
        let Some(mut j) = k.checked_sub(1) else {
            return false;
        };
        if self.tx(j) == ">" {
            while j > 0 && self.tx(j) != "<" {
                j -= 1;
            }
            j = j.saturating_sub(1);
        }
        j >= 1 && self.word_at(j) && self.tx(j - 1) == "class"
    }

    fn word_at(&self, k: usize) -> bool {
        self.toks.get(k).is_some_and(|t| t.kind == K::Word)
    }

    fn kt_imports(&mut self) {
        let n = self.toks.len();
        let mut k = 0;
        while k < n {
            if !(self.tx(k) == "import" && self.first[k]) {
                k += 1;
                continue;
            }
            let start = self.toks[k].line;
            let mut end = start;
            while k < n && self.tx(k) == "import" && self.first[k] {
                end = self.toks[k].line;
                k += 1;
                while k < n && !self.first[k] {
                    k += 1;
                }
            }
            self.add(start, Some(end), false);
        }
    }

    fn kt_member_start(&self, k: usize) -> usize {
        let mut s = k;
        while s > 0 {
            let t = self.toks[s - 1];
            if MODIFIERS.contains(&t.text) || t.text.starts_with('@') {
                s -= 1;
            } else if t.text == ")"
                && let Some(o) = self.back[s - 1]
                && self.before(o).starts_with('@')
            {
                s = o - 1;
            } else {
                break;
            }
        }
        s
    }

    fn kt_body(&self, k: usize) -> bool {
        let Some(mut j) = k.checked_sub(1) else {
            return false;
        };
        if self.tx(j) != ")" {
            let mut angle = 0i32;
            while j > 0
                && (self.word_at(j)
                    || matches!(self.tx(j), "." | "?" | "<" | ">" | "," | "*" | ")"))
            {
                match self.tx(j) {
                    ">" if self.before(j) == "-" => j -= 1,
                    ">" => angle += 1,
                    "<" => angle -= 1,
                    ")" => j = self.back[j].unwrap_or(j),
                    "," if angle == 0 => return false,
                    _ => {}
                }
                j -= 1;
            }
            if self.tx(j) != ":" || j == 0 {
                return false;
            }
            j -= 1;
        }
        let Some(o) = (self.tx(j) == ")").then(|| self.back[j]).flatten() else {
            return false;
        };
        let name = o.saturating_sub(1);
        if matches!(self.tx(name), "get" | "set") {
            return true;
        }
        let mut i = name;
        while i > 0 && name - i < 12 {
            i -= 1;
            match self.tx(i) {
                "fun" => return true,
                "." | "?" | "<" | ">" | "," | "*" => {}
                _ if self.word_at(i) && !self.first[i + 1] => {}
                _ => return false,
            }
        }
        false
    }

    fn kt_expr_end(&self, from: usize, entry: bool) -> usize {
        let n = self.toks.len();
        let mut j = from;
        let mut last = from.min(n - 1);
        while j < n {
            if j > from && self.toks[j].line > self.toks[last].end {
                let starts = CONT_START.contains(&self.tx(j))
                    && !(entry && matches!(self.tx(j), "is" | "in" | "else"));
                let cont = CONT_END.contains(&self.tx(last)) || starts;
                if !cont {
                    break;
                }
            }
            if self.opener(j) {
                match self.pair[j] {
                    Some(c) => j = c,
                    None => break,
                }
            } else if self.closer(j) || self.punct(j, ";") {
                break;
            }
            last = j;
            j += 1;
        }
        last
    }

    fn kt_def(&self, k: usize) -> bool {
        let mut j = k;
        while j > 0 {
            j -= 1;
            if self.closer(j) {
                j = self.back[j].unwrap_or(j);
                continue;
            }
            match self.tx(j) {
                "fun" | "class" | "object" | "interface" | "get" | "set" => return true,
                ";" | "{" | "}" | "=" => return false,
                _ => {}
            }
            if self.first[j] && j > 0 {
                let p = self.tx(j - 1);
                let t = self.tx(j);
                if !(CONT_END.contains(&p) || matches!(t, "." | ":" | "?" | "where")) {
                    return false;
                }
            }
        }
        false
    }
}
