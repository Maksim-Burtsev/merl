use super::{K, Lexer, Model};
use std::collections::HashMap;

const KW: &[&str] = &[
    "as", "async", "break", "const", "continue", "dyn", "else", "enum", "extern", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "static", "struct", "trait", "type", "unsafe", "use", "where", "while", "yield", "box",
];
const QUALIFIERS: &[&str] = &[
    "pub", "unsafe", "async", "const", "extern", "default", "auto",
];
const BLOCK_AFTER: &[&str] = &["else", "unsafe", "async", "move", "loop", "const", "try"];

impl Lexer<'_> {
    pub(super) fn rust_quote(&mut self) -> bool {
        let s = self.bytes();
        let (l, at) = (self.l, self.i);
        match s[at] {
            b'\'' => {
                let next = s.get(at + 1).copied();
                let len = match next {
                    Some(b'\\') => {
                        let close = s[(at + 3).min(s.len())..].iter().position(|&b| b == b'\'');
                        close.map(|c| c + 4)
                    }
                    Some(b) => {
                        let w = std::str::from_utf8(&s[at + 1..])
                            .ok()
                            .and_then(|r| r.chars().next())
                            .map_or(1, char::len_utf8);
                        (b != b'\'' && s.get(at + 1 + w) == Some(&b'\'')).then_some(w + 2)
                    }
                    None => None,
                };
                if let Some(len) = len {
                    self.i = at + len;
                    self.emit(l, at, 1, K::Str, l);
                } else {
                    let w = s[at + 1..].iter().take_while(|&&b| super::word(b)).count();
                    self.emit(l, at, w + 1, K::Word, l);
                    self.i = at + 1 + w;
                }
                true
            }
            b'"' => {
                self.rust_string(at + 1);
                self.emit(l, at, 1, K::Str, self.l);
                true
            }
            b'r' | b'b' | b'c' => {
                let mut j = at;
                if matches!(s[j], b'b' | b'c') {
                    j += 1;
                }
                let raw = s.get(j) == Some(&b'r');
                if raw {
                    j += 1;
                }
                let hashes = s[j.min(s.len())..]
                    .iter()
                    .take_while(|&&b| b == b'#')
                    .count();
                if s.get(j + hashes) != Some(&b'"') || (!raw && (hashes > 0 || j == at)) {
                    return false;
                }
                if raw {
                    let mut delim = vec![b'"'];
                    delim.extend(std::iter::repeat_n(b'#', hashes));
                    self.skip_past(j + hashes + 1, &delim);
                } else {
                    self.rust_string(j + 1);
                }
                self.emit(l, at, 1, K::Str, self.l);
                true
            }
            _ => false,
        }
    }

    pub(super) fn nested_comment(&mut self, from: usize) {
        self.i = from;
        let mut depth = 1;
        loop {
            let s = self.bytes();
            if self.i >= s.len() {
                if !self.next_line() {
                    return;
                }
                continue;
            }
            if s[self.i..].starts_with(b"*/") {
                self.i += 2;
                depth -= 1;
                if depth == 0 {
                    return;
                }
            } else if s[self.i..].starts_with(b"/*") {
                self.i += 2;
                depth += 1;
            } else {
                self.i += 1;
            }
        }
    }

    fn rust_string(&mut self, from: usize) {
        self.i = from;
        loop {
            let s = self.bytes();
            if self.i >= s.len() {
                if !self.next_line() {
                    return;
                }
                continue;
            }
            match s[self.i] {
                b'\\' => self.i += 2,
                b'"' => {
                    self.i += 1;
                    return;
                }
                _ => self.i += 1,
            }
        }
    }
}

impl Model<'_, '_> {
    pub(super) fn rust(&mut self, lines: &[String]) {
        let n = self.toks.len();
        let mut opaque = vec![false; n];
        for k in 0..n {
            if opaque[k] {
                continue;
            }
            let (head, open, fold) = if self.tx(k) == "#" && self.punct(k + 1, "[") {
                (k, k + 1, true)
            } else if self.tx(k) == "#" && self.punct(k + 1, "!") && self.punct(k + 2, "[") {
                (k, k + 2, false)
            } else if self.punct(k, "!")
                && self.word(k.wrapping_sub(1))
                && !matches!(self.before(k), "#" | "macro_rules")
                && !KW.contains(&self.before(k))
                && self.opener_at(k + 1)
            {
                (self.path_head(k - 1), k + 1, true)
            } else if self.tx(k) == "macro_rules" && self.punct(k + 1, "!") && self.opener_at(k + 3)
            {
                (k, k + 3, true)
            } else {
                continue;
            };
            let Some(close) = self.pair[open] else {
                continue;
            };
            opaque[open + 1..close].fill(true);
            if fold {
                let end = if self.punct(close + 1, ";") && self.tx(head) == "macro_rules" {
                    close + 1
                } else {
                    close
                };
                self.add(self.toks[head].line, Some(self.end_of(end)), false);
            }
        }
        let mut body = vec![None::<bool>; n];
        let mut owner = HashMap::new();
        let mut uses: Option<(usize, usize)> = None;
        let mut last_use: Option<usize> = None;
        for k in 0..n {
            if opaque[k] {
                continue;
            }
            let t = self.toks[k];
            if t.kind == K::Word {
                if t.text == "use"
                    && let Some(s) = self.item_start(k)
                {
                    let end = self.depth0(k, &[";"], true);
                    if last_use.is_none_or(|e| e + 1 != s) {
                        self.end_run(uses.take());
                    }
                    last_use = Some(end);
                    let span = (self.toks[s].line, self.toks[end].end);
                    uses = self.join_run(uses, span, lines);
                }
                self.rust_word(k, &mut body, &mut owner);
                continue;
            }
            if t.kind != K::Punct {
                continue;
            }
            match t.text {
                "(" if self.call(k) => {
                    let head = self.chain_head(k - 1, &owner);
                    self.add(self.toks[head].line, self.closing(k), false);
                }
                "[" if self.array(k) => self.add(t.line, self.closing(k), false),
                "{" if body[k].unwrap_or_else(|| self.block(k)) => {
                    self.add(t.line, self.closing(k), false)
                }
                _ => {}
            }
        }
        self.end_run(uses);
    }

    fn rust_word(
        &mut self,
        k: usize,
        body: &mut [Option<bool>],
        owner: &mut HashMap<usize, usize>,
    ) {
        let t = self.toks[k];
        let prev = self.before(k);
        match t.text {
            "fn" | "struct" | "enum" | "trait" | "impl" | "mod" | "union" | "extern" => {
                let Some(s) = self.item_start(k) else {
                    return;
                };
                let next = self.tx(k + 1);
                let item = match t.text {
                    "fn" | "mod" | "struct" | "enum" | "trait" => self.word(k + 1),
                    "union" => self.word(k + 1) && !KW.contains(&next),
                    "extern" => !matches!(next, "crate" | "fn") && self.tx(k + 2) != "fn",
                    _ => true,
                };
                if !item {
                    return;
                }
                let end = self.depth0(k, &["{", ";"], false);
                if self.punct(end, "{") {
                    body[end] = Some(t.text == "fn");
                }
                let close = if self.punct(end, "{") {
                    self.pair[end]
                } else {
                    Some(end)
                };
                let def = matches!(t.text, "fn" | "impl" | "trait");
                self.add(self.toks[s].line, close.map(|c| self.end_of(c)), def);
            }
            "type" | "const" => {
                let Some(s) = self.item_start(k) else {
                    return;
                };
                if t.text == "const" && (!self.word(k + 1) || QUALIFIERS.contains(&self.tx(k + 1)))
                {
                    return;
                }
                if t.text == "const" && self.tx(k + 1) == "fn" {
                    return;
                }
                let end = self.depth0(k, &[";"], true);
                self.add(self.toks[s].line, Some(self.end_of(end)), false);
            }
            "let" if !matches!(prev, "if" | "while" | "&" | "|") => {
                let end = self.depth0(k, &[";"], true);
                self.add(t.line, Some(self.end_of(end)), false);
            }
            "if" | "while" | "for" | "loop" | "match" => {
                let guard = t.text == "if"
                    && (matches!(prev, ")" | "]")
                        || (k > 0 && self.word(k - 1) && !KW.contains(&prev)));
                let header = t.text == "for" && (self.word(k.wrapping_sub(1)) || prev == ">");
                if guard || header || (t.text == "for" && self.tx(k + 1) == "<") {
                    return;
                }
                let open = self.depth0(k, &["{"], false);
                if self.punct(open, "{") {
                    body[open] = Some(t.text != "match");
                }
                let more: &[&str] = if t.text == "if" { &["else"] } else { &[] };
                let start = match (prev, k.checked_sub(2)) {
                    (":", Some(p)) if self.tx(p).starts_with('\'') => p,
                    _ => k,
                };
                let mut close = self.punct(open, "{").then(|| self.pair[open]).flatten();
                while let Some(c) = close.filter(|&c| t.text == "if" && self.tx(c + 1) == "else") {
                    let next = match self.tx(c + 2) {
                        "if" => self.depth0(c + 2, &["{"], false),
                        _ => c + 2,
                    };
                    close = self.punct(next, "{").then(|| self.pair[next]).flatten();
                }
                if let Some(c) = close {
                    owner.insert(c, start);
                }
                self.add(self.toks[start].line, self.chain(k, more), false);
            }
            _ => {}
        }
    }

    fn word(&self, k: usize) -> bool {
        self.toks.get(k).is_some_and(|t| t.kind == K::Word)
    }

    fn opener_at(&self, k: usize) -> bool {
        k < self.toks.len() && self.opener(k)
    }

    fn path_head(&self, mut j: usize) -> usize {
        while j >= 3 && self.punct(j - 1, ":") && self.punct(j - 2, ":") && self.word(j - 3) {
            j -= 3;
        }
        j
    }

    fn item_start(&self, k: usize) -> Option<usize> {
        let mut s = k;
        while s > 0 {
            let p = self.toks[s - 1];
            if (p.kind == K::Word && QUALIFIERS.contains(&p.text)) || p.kind == K::Str {
                s -= 1;
            } else if p.text == ")"
                && let Some(o) = self.back[s - 1]
                && self.before(o) == "pub"
            {
                s = o - 1;
            } else {
                break;
            }
        }
        let ok = s == 0 || matches!(self.tx(s - 1), ";" | "{" | "}" | "]");
        ok.then_some(s)
    }

    fn depth0(&self, k: usize, stop: &[&str], braces: bool) -> usize {
        let mut j = k + 1;
        while j < self.toks.len() {
            let t = self.tx(j);
            if self.toks[j].kind == K::Punct && stop.contains(&t) {
                return j;
            }
            if self.opener(j) && (braces || t != "{") {
                match self.pair[j] {
                    Some(c) => j = c,
                    None => return j,
                }
            } else if self.closer(j) {
                return j - 1;
            }
            j += 1;
        }
        self.toks.len() - 1
    }

    fn element_end(&self, k: usize) -> bool {
        let t = self.toks[k];
        match t.kind {
            K::Word => !KW.contains(&t.text),
            K::Str => true,
            _ => matches!(t.text, ")" | "]" | "?"),
        }
    }

    fn call(&self, k: usize) -> bool {
        let Some(p) = k.checked_sub(1) else {
            return false;
        };
        if self.tx(p) == ">" {
            let mut j = p;
            let mut angle = 0i32;
            loop {
                match self.tx(j) {
                    ">" => angle += 1,
                    "<" => angle -= 1,
                    _ => {}
                }
                if angle == 0 || j == 0 {
                    break;
                }
                j -= 1;
            }
            return j >= 2 && self.punct(j - 1, ":") && self.punct(j - 2, ":");
        }
        let decl = p > 0 && matches!(self.before(p), "fn" | "struct" | "union" | "enum");
        self.element_end(p) && self.tx(p) != "?" && !decl
    }

    fn array(&self, k: usize) -> bool {
        let Some(p) = k.checked_sub(1) else {
            return true;
        };
        !(self.element_end(p) || matches!(self.tx(p), "#" | "!" | ">"))
    }

    fn block(&self, k: usize) -> bool {
        let Some(p) = k.checked_sub(1) else {
            return true;
        };
        let t = self.toks[p];
        match t.kind {
            K::Word => {
                BLOCK_AFTER.contains(&t.text)
                    || matches!(t.text, "return" | "break" | "yield")
                    || self.returns(p)
            }
            K::Str => false,
            _ if t.text == ">" => self.returns(p),
            _ => {
                !matches!(t.text, ">" | "?" | "!" | ":" | "]")
                    || (t.text == ":" && self.before(p).starts_with('\''))
                    || (t.text == "]" && self.back[p].is_some_and(|o| self.before(o) == "#"))
            }
        }
    }

    fn returns(&self, mut j: usize) -> bool {
        for _ in 0..40 {
            if j < 2 || matches!(self.tx(j), ";" | "{" | "}" | "=") {
                return false;
            }
            if self.tx(j) == ">" && self.punct(j - 1, "-") {
                return true;
            }
            j -= 1;
        }
        false
    }

    fn chain_head(&self, mut j: usize, owner: &HashMap<usize, usize>) -> usize {
        loop {
            let t = self.toks[j];
            let mut s = j;
            if let Some(&start) = owner.get(&j) {
                s = start;
            } else if self.closer(j) {
                let Some(o) = self.back[j] else {
                    return j;
                };
                s = o;
                if o > 0 {
                    let callee = match self.tx(o) {
                        "{" => self.word(o - 1) && !KW.contains(&self.tx(o - 1)),
                        _ => self.element_end(o - 1) || matches!(self.tx(o - 1), "!" | ">"),
                    };
                    if callee {
                        j = o - 1;
                        continue;
                    }
                }
            } else if t.text == "?" || (t.text == "!" && j > 0 && self.word(j - 1)) {
                j -= 1;
                continue;
            } else if t.text == ">" {
                let mut angle = 0i32;
                while j > 0 {
                    match self.tx(j) {
                        ">" => angle += 1,
                        "<" => angle -= 1,
                        _ => {}
                    }
                    if angle == 0 {
                        break;
                    }
                    j -= 1;
                }
                s = j;
            } else if !self.element_end(j) {
                return (j + 1).min(self.toks.len() - 1);
            }
            if s >= 2 && self.punct(s - 1, ".") && !self.punct(s - 2, ".") {
                j = s - 2;
            } else if s >= 3 && self.punct(s - 1, ":") && self.punct(s - 2, ":") {
                let before = s - 3;
                if self.element_end(before) || self.tx(before) == ">" {
                    j = before;
                } else {
                    return s - 2;
                }
            } else {
                return s;
            }
        }
    }
}
