use super::{K, Lexer, Model};

const NOT_CALLEE: &[&str] = &[
    "if", "for", "while", "switch", "catch", "assert", "return", "await", "yield", "throw", "in",
    "is", "as", "case", "else", "do", "async", "sync", "Function",
];
const LIST_AFTER: &[&str] = &[
    "return", "const", "in", "yield", "await", "case", "else", "final", "var",
];
const MODIFIERS: &[&str] = &[
    "abstract",
    "sealed",
    "base",
    "final",
    "interface",
    "mixin",
    "macro",
    "augment",
];

impl Lexer<'_> {
    pub(super) fn dart_string(&mut self) -> bool {
        let s = self.bytes();
        let (l, at) = (self.l, self.i);
        let raw = s[at] == b'r';
        let q = match s.get(at + usize::from(raw)) {
            Some(&q @ (b'"' | b'\'')) => q,
            _ => return false,
        };
        if raw && at > 0 && super::word(s[at - 1]) {
            return false;
        }
        let from = at + usize::from(raw);
        let triple = s[from..].starts_with(&[q, q, q]);
        let close: &[u8] = if triple {
            &s[from..from + 3]
        } else {
            &s[from..from + 1]
        };
        let close = close.to_vec();
        self.i = from + close.len();
        loop {
            let s = self.bytes();
            if self.i >= s.len() {
                if triple && self.next_line() {
                    continue;
                }
                break;
            }
            if s[self.i] == b'\\' && !raw {
                self.i += 2;
                continue;
            }
            if !raw && s[self.i..].starts_with(b"${") {
                self.i += 2;
                self.dart_hole();
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

    fn dart_hole(&mut self) {
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
                b'"' | b'\'' | b'r' if self.dart_string() => {
                    self.out.pop();
                    continue;
                }
                _ => {}
            }
            self.i += 1;
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Scope {
    Code,
    Class,
    Enum,
    Params,
}

impl Model<'_, '_> {
    pub(super) fn dart(&mut self, lines: &[String]) {
        let n = self.toks.len();
        self.dart_imports(lines);
        let mut scope = vec![Scope::Code; n];
        let mut stack: Vec<usize> = Vec::new();
        let mut enum_items = Vec::new();
        for k in 0..n {
            let t = self.toks[k];
            if self.closer(k) {
                stack.pop();
                continue;
            }
            let parent = stack.last().map(|&p| scope[p]);
            let member = parent.is_none_or(|s| matches!(s, Scope::Class | Scope::Enum));
            match (t.kind, t.text) {
                (K::Str, _) if k == 0 || self.toks[k - 1].kind != K::Str => {
                    let mut e = k;
                    while self.toks.get(e + 1).is_some_and(|x| x.kind == K::Str) {
                        e += 1;
                    }
                    self.add(t.line, Some(self.end_of(e)), false);
                }
                (K::Word, "class" | "enum" | "extension") if self.before(k) != "." => {
                    self.dart_decl(k)
                }
                (K::Punct, "=>") if self.dart_body_arrow(k, &scope) => {
                    let end = self.dart_stmt_end(k);
                    self.add(t.line, Some(self.end_of(end)), true);
                }
                (K::Punct, "(") => {
                    let in_enum = parent == Some(Scope::Enum)
                        && !enum_items.contains(&stack[stack.len() - 1]);
                    if self.dart_params(k, member && !in_enum) {
                        scope[k] = Scope::Params;
                    } else if self.dart_callee(k) {
                        self.add(t.line, self.closing(k), false);
                    }
                }
                (K::Punct, "[") => {
                    let list = k == 0 || {
                        let p = self.toks[k - 1];
                        match p.kind {
                            K::Punct => !matches!(p.text, ")" | "]" | "}" | "!"),
                            K::Word => LIST_AFTER.contains(&p.text),
                            _ => false,
                        }
                    };
                    if list {
                        self.add(t.line, self.closing(k), false);
                    }
                }
                (K::Punct, "{") => {
                    scope[k] = self.dart_brace(k, parent);
                    let def = k > 0
                        && match self.tx(k - 1) {
                            ")" => self.back[k - 1].is_some_and(|o| scope[o] == Scope::Params),
                            "async" | "*" => true,
                            _ => k > 1 && self.tx(k - 2) == "get",
                        };
                    if scope[k] == Scope::Code {
                        self.add(t.line, self.closing(k), def);
                    }
                }
                (K::Punct, ";") if parent == Some(Scope::Enum) => {
                    enum_items.push(stack[stack.len() - 1]);
                }
                _ => {}
            }
            if self.opener(k) {
                stack.push(k);
            }
        }
    }

    fn dart_imports(&mut self, lines: &[String]) {
        let n = self.toks.len();
        let mut k = 0;
        let mut run = None;
        while k < n {
            if !(self.first[k] && matches!(self.tx(k), "import" | "export")) {
                k += 1;
                continue;
            }
            let end = (k..n).find(|&j| self.punct(j, ";")).unwrap_or(n - 1);
            let last = self.toks[end].end;
            run = self.join_run(run, (self.toks[k].line, last), lines);
            if !lines[last].trim_end().ends_with(';') {
                self.end_run(run.take());
            }
            k = end + 1;
        }
        self.end_run(run);
    }

    fn dart_decl(&mut self, k: usize) {
        if self.tx(k) == "extension" && self.tx(k + 1) == "type" {
            return;
        }
        let mut s = k;
        while s > 0 {
            if MODIFIERS.contains(&self.tx(s - 1)) {
                s -= 1;
            } else if let Some(a) = self.dart_annotation(s - 1) {
                s = a;
            } else {
                break;
            }
        }
        let mut j = k + 1;
        while j < self.toks.len() {
            match self.tx(j) {
                "{" => {
                    let end = self.closing(j);
                    self.add(self.toks[s].line, end, true);
                    return;
                }
                ";" => {
                    self.add(self.toks[s].line, Some(self.end_of(j)), true);
                    return;
                }
                "(" | "[" => j = self.pair[j].unwrap_or(j),
                ")" | "]" | "}" => return,
                _ => {}
            }
            j += 1;
        }
    }

    fn dart_brace(&self, k: usize, parent: Option<Scope>) -> Scope {
        if parent == Some(Scope::Params) {
            return Scope::Params;
        }
        let mut j = k;
        while j > 0 {
            j -= 1;
            if self.closer(j) && self.tx(j) != "}" {
                j = self.back[j].unwrap_or(j);
                continue;
            }
            if self.toks[j].kind != K::Word {
                if matches!(self.tx(j), "<" | ">" | "," | "." | "?") {
                    continue;
                }
                return Scope::Code;
            }
            match self.tx(j) {
                "class" | "extension" | "mixin" if self.before(j) != "." => return Scope::Class,
                "enum" if self.before(j) != "." => return Scope::Enum,
                "switch" => {
                    let statement = j == 0 || matches!(self.before(j), ";" | "{" | "}" | "else");
                    return if statement {
                        Scope::Code
                    } else {
                        Scope::Params
                    };
                }
                _ => {}
            }
            if self.first[j] && j > 0 && matches!(self.tx(j - 1), ";" | "{" | "}") {
                return Scope::Code;
            }
        }
        Scope::Code
    }

    fn dart_callee(&self, k: usize) -> bool {
        let Some(p) = k.checked_sub(1).map(|p| self.toks[p]) else {
            return false;
        };
        match p.kind {
            K::Word => !NOT_CALLEE.contains(&p.text) || self.before(k - 1) == ".",
            K::Punct => matches!(p.text, ")" | "]" | ">" | "!"),
            _ => false,
        }
    }

    fn dart_annotation(&self, mut j: usize) -> Option<usize> {
        if self.tx(j) == ")" {
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

    fn dart_params(&self, k: usize, member: bool) -> bool {
        let Some(c) = self.pair[k] else {
            return false;
        };
        let head = matches!(self.tx(c + 1), "{" | "=>" | "async" | "sync");
        if !self.dart_callee(k) {
            return head && !(k > 0 && self.word_at(k - 1));
        }
        if self.dart_annotation(k - 1).is_some() {
            return false;
        }
        let mut j = k - 1;
        while j > 0 {
            j -= 1;
            if self.closer(j) && self.tx(j) != "}" {
                let o = self.back[j].unwrap_or(j);
                if self.tx(j) == ")" && self.dart_callee(o) && self.dart_annotation(j).is_none() {
                    return false;
                }
                j = o;
                continue;
            }
            match self.tx(j) {
                ";" | "{" | "}" => break,
                "=" | "=>" | ":" => return false,
                _ => {}
            }
        }
        head || member
    }

    fn dart_body_arrow(&self, k: usize, scope: &[Scope]) -> bool {
        if k < 2 {
            return false;
        }
        if self.word_at(k - 1) && self.tx(k - 2) == "get" {
            return true;
        }
        let mut close = k - 1;
        if self.tx(close) == "*" {
            close -= 1;
        }
        if matches!(self.tx(close), "async" | "sync") {
            close -= 1;
        }
        let Some(o) = (self.tx(close) == ")").then(|| self.back[close]).flatten() else {
            return false;
        };
        let operator = (o.saturating_sub(4)..o).any(|j| self.tx(j) == "operator");
        operator || (scope[o] == Scope::Params && self.dart_callee(o))
    }

    fn dart_stmt_end(&self, k: usize) -> usize {
        let (mut j, mut last) = (k + 1, k);
        while j < self.toks.len() {
            if self.opener(j)
                && let Some(c) = self.pair[j]
            {
                (j, last) = (c + 1, c);
                continue;
            }
            if self.punct(j, ";") {
                return j;
            }
            if self.closer(j) || self.punct(j, ",") {
                return last;
            }
            (last, j) = (j, j + 1);
        }
        last
    }
}
