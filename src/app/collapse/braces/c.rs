use super::{K, Lang, Lexer, Model};

const CONTROL: &[&str] = &["if", "for", "while", "switch", "catch", "@synchronized"];
const ACCESS: &[&str] = &["public", "private", "protected"];
const NOT_ELEMENT: &[&str] = &[
    "return",
    "co_return",
    "co_yield",
    "throw",
    "case",
    "else",
    "do",
];

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Ctx {
    Decl,
    Code,
    Switch,
    Init,
}

impl Lexer<'_> {
    pub(super) fn c_directive(&mut self) -> bool {
        let s = self.bytes();
        if s.get(self.i) != Some(&b'#') || !s[..self.i].iter().all(u8::is_ascii_whitespace) {
            return false;
        }
        loop {
            let s = self.bytes();
            let mut j = self.i;
            while let Some(p) = super::find(&s[j..], b"/*") {
                let open = j + p + 2;
                match super::find(&s[open..], b"*/") {
                    Some(q) => j = open + q + 2,
                    None => {
                        let l = self.l;
                        self.skip_past(open, b"*/");
                        self.emit(l, open - 2, 1, K::Comment, self.l);
                        self.i = self.bytes().len();
                        return true;
                    }
                }
            }
            let continued = s.trim_ascii_end().ends_with(b"\\");
            self.i = s.len();
            if !continued || !self.next_line() {
                return true;
            }
        }
    }

    pub(super) fn cpp_raw(&mut self) -> bool {
        let s = self.bytes();
        let at = self.i;
        let mut j = at;
        for prefix in [&b"u8"[..], b"u", b"U", b"L"] {
            if s[j..].starts_with(prefix) {
                j += prefix.len();
                break;
            }
        }
        if !s[j..].starts_with(b"R\"") {
            return false;
        }
        let open = j + 2;
        let Some(paren) = s[open..].iter().position(|&b| b == b'(') else {
            return false;
        };
        let mut delim = vec![b')'];
        delim.extend_from_slice(&s[open..open + paren]);
        delim.push(b'"');
        let l = self.l;
        self.skip_past(open + paren + 1, &delim);
        self.emit(l, at, 1, K::Str, self.l);
        true
    }
}

impl Model<'_, '_> {
    pub(super) fn c_family(&mut self, lang: Lang, lines: &[String]) {
        let (cpp, objc) = (lang != Lang::C, lang == Lang::ObjC);
        self.c_preproc(lines, objc);
        let n = self.toks.len();
        let mut ctx = vec![Ctx::Decl; n];
        let mut marked: Vec<Option<Ctx>> = vec![None; n];
        let mut stack: Vec<usize> = Vec::new();
        let mut container_end = 0;
        for k in 0..n {
            let up = stack.last().map_or(Ctx::Decl, |&p| ctx[p]);
            let t = self.toks[k];
            if self.closer(k) {
                stack.pop();
                continue;
            }
            if objc && up == Ctx::Decl {
                if let Some(end) = self.objc_container(k, &mut marked) {
                    container_end = end;
                } else if k < container_end {
                    self.objc_method(k, &mut marked);
                }
            }
            if t.kind == K::Word && up != Ctx::Init {
                self.c_word(k, up, lang, &mut marked);
            }
            if !self.opener(k) {
                continue;
            }
            stack.push(k);
            if objc && t.text == "[" && self.before(k) == "@" {
                self.add(t.line, self.closing(k), false);
            }
            if t.text == "["
                && cpp
                && up != Ctx::Decl
                && let Some(body) = self.lambda(k)
            {
                marked[body] = Some(Ctx::Code);
                self.add(t.line, self.closing(body), true);
            }
            if t.text != "{" {
                ctx[k] = if up == Ctx::Switch { Ctx::Code } else { up };
                continue;
            }
            if let Some(c) = marked[k] {
                ctx[k] = c;
                continue;
            }
            let (c, fold) = self.c_brace(k, up, lang);
            ctx[k] = c;
            if !fold {
                continue;
            }
            if up == Ctx::Decl && c == Ctx::Code {
                let start = self.function_start(k, lang);
                self.add(self.toks[start].line, self.closing(k), true);
            } else {
                self.add(t.line, self.closing(k), false);
            }
        }
    }

    fn c_word(&mut self, k: usize, up: Ctx, lang: Lang, marked: &mut [Option<Ctx>]) {
        let (cpp, objc) = (lang != Lang::C, lang == Lang::ObjC);
        let t = self.toks[k];
        let code = matches!(up, Ctx::Code | Ctx::Switch);
        match t.text {
            "if" | "for" | "while" | "switch" | "do" | "try" | "@try" if code => {
                if self.loop_tail(k, "do") {
                    return;
                }
                if matches!(t.text, "do" | "try" | "@try" | "else") || self.punct(k + 1, "{") {
                    if let Some(m) = marked.get_mut(k + 1) {
                        *m = Some(Ctx::Code);
                    }
                } else if let Some(c) = self.paren_after(k).and_then(|o| self.pair[o])
                    && self.punct(c + 1, "{")
                {
                    let switch = t.text == "switch";
                    marked[c + 1] = Some(if switch { Ctx::Switch } else { Ctx::Code });
                    if switch {
                        self.c_cases(c + 1);
                    }
                }
                let end = self.c_stmt_end(k);
                self.add(t.line, Some(self.end_of(end)), false);
            }
            "else" if code && self.punct(k + 1, "{") => marked[k + 1] = Some(Ctx::Code),
            "@finally" | "@autoreleasepool" if objc && self.punct(k + 1, "{") => {
                marked[k + 1] = Some(Ctx::Code);
                self.add(t.line, self.closing(k + 1), false);
            }
            "@throw" | "@property" if objc => {
                let end = self.c_stmt_end(k);
                self.add(t.line, Some(self.end_of(end)), false);
            }
            "catch" | "@catch" if cpp => {
                if let Some(c) = self.paren_after(k).and_then(|o| self.pair[o])
                    && self.punct(c + 1, "{")
                {
                    marked[c + 1] = Some(Ctx::Code);
                    self.add(t.line, self.closing(c + 1), false);
                }
            }
            "template" if cpp && self.punct(k + 1, "<") => {
                let end = self.template_end(k + 1);
                self.add(t.line, Some(self.end_of(end)), false);
            }
            "namespace" if cpp => {
                let mut j = k + 1;
                while self
                    .toks
                    .get(j)
                    .is_some_and(|x| x.kind == K::Word || x.text == ":")
                {
                    j += 1;
                }
                if self.punct(j, "{") {
                    marked[j] = Some(Ctx::Decl);
                    self.add(t.line, self.closing(j), false);
                }
            }
            "extern" if self.toks.get(k + 1).is_some_and(|x| x.kind == K::Str) => {
                if self.punct(k + 2, "{") {
                    marked[k + 2] = Some(Ctx::Decl);
                }
            }
            "struct" | "enum" => self.specifier(k, marked),
            "class" if cpp => self.specifier(k, marked),
            "union" => {
                if let Some(body) = self.specifier_body(k) {
                    marked[body] = Some(Ctx::Decl);
                }
            }
            _ => {}
        }
    }

    fn specifier(&mut self, k: usize, marked: &mut [Option<Ctx>]) {
        if let Some(body) = self.specifier_body(k) {
            marked[body] = Some(Ctx::Decl);
            self.add(self.toks[k].line, self.closing(body), true);
        }
    }

    fn specifier_body(&self, k: usize) -> Option<usize> {
        let mut j = k + 1;
        let mut attr = false;
        while j < self.toks.len() {
            let t = self.toks[j];
            match t.text {
                "{" => return Some(j),
                "<" => {
                    let mut angle = 0i32;
                    while j < self.toks.len() {
                        match self.tx(j) {
                            "<" => angle += 1,
                            ">" => angle -= 1,
                            _ => {}
                        }
                        if angle == 0 {
                            break;
                        }
                        j += 1;
                    }
                }
                "(" if attr => j = self.pair[j]?,
                "[" if self.punct(j + 1, "[") => j = self.pair[j]?,
                ":" | "," => {}
                _ if t.kind == K::Word => {
                    attr = matches!(t.text, "__attribute__" | "alignas" | "__declspec");
                }
                _ => return None,
            }
            j += 1;
        }
        None
    }

    fn c_brace(&self, k: usize, up: Ctx, lang: Lang) -> (Ctx, bool) {
        let cpp = lang != Lang::C;
        let Some(p) = k.checked_sub(1) else {
            return (Ctx::Code, up == Ctx::Decl);
        };
        let pt = self.toks[p];
        if lang == Lang::ObjC && pt.text == "@" {
            return (Ctx::Init, true);
        }
        if lang == Lang::ObjC && self.block_literal(p) {
            return (Ctx::Code, true);
        }
        match up {
            Ctx::Init => (Ctx::Init, true),
            Ctx::Decl => match pt.text {
                "=" => (Ctx::Init, true),
                _ if (pt.kind == K::Word || pt.text == ">")
                    && self.pair[k].is_some_and(|c| matches!(self.tx(c + 1), "," | "{" | ";")) =>
                {
                    (Ctx::Init, true)
                }
                _ if pt.kind == K::Str => (Ctx::Decl, false),
                _ => (Ctx::Code, true),
            },
            Ctx::Code | Ctx::Switch => match pt.text {
                ")" => match self.back[p].map(|o| self.before(o)) {
                    Some(w) if CONTROL.contains(&w) => (Ctx::Code, false),
                    _ => (Ctx::Init, true),
                },
                ";" | "{" | "}" => (Ctx::Code, up == Ctx::Code),
                "=" | "," | "(" | "[" | "return" | "?" => (Ctx::Init, true),
                ":" => (Ctx::Code, false),
                ">" if cpp => (Ctx::Init, true),
                _ if pt.kind == K::Word && cpp && !NOT_ELEMENT.contains(&pt.text) => {
                    (Ctx::Init, true)
                }
                _ => (Ctx::Code, false),
            },
        }
    }

    fn paren_after(&self, k: usize) -> Option<usize> {
        let mut j = k + 1;
        while self.toks.get(j).is_some_and(|t| t.kind == K::Word) {
            j += 1;
        }
        self.punct(j, "(").then_some(j)
    }

    fn c_stmt_end(&self, k: usize) -> usize {
        let n = self.toks.len();
        if k >= n {
            return n - 1;
        }
        match self.tx(k) {
            "{" => self.pair[k].unwrap_or(k),
            "if" => {
                let Some(c) = self.paren_after(k).and_then(|o| self.pair[o]) else {
                    return k;
                };
                let mut e = self.c_stmt_end(c + 1);
                if self.tx(e + 1) == "else" {
                    e = self.c_stmt_end(e + 2);
                }
                e
            }
            "for" | "while" | "switch" => match self.paren_after(k).and_then(|o| self.pair[o]) {
                Some(c) => self.c_stmt_end(c + 1),
                None => k,
            },
            "do" => {
                let e = self.c_stmt_end(k + 1);
                if self.tx(e + 1) == "while"
                    && let Some(c) = self.paren_after(e + 1).and_then(|o| self.pair[o])
                {
                    return if self.punct(c + 1, ";") { c + 1 } else { c };
                }
                e
            }
            "try" | "@try" => {
                let mut e = self.c_stmt_end(k + 1);
                loop {
                    match self.tx(e + 1) {
                        "catch" | "@catch" => {
                            match self.paren_after(e + 1).and_then(|o| self.pair[o]) {
                                Some(c) => e = self.c_stmt_end(c + 1),
                                None => break,
                            }
                        }
                        "@finally" => e = self.c_stmt_end(e + 2),
                        _ => break,
                    }
                }
                e
            }
            _ => {
                let mut j = k;
                while j < n {
                    if self.punct(j, ";") {
                        return j;
                    }
                    if self.opener(j) {
                        match self.pair[j] {
                            Some(c) => j = c,
                            None => return j,
                        }
                    } else if self.closer(j) {
                        return j.saturating_sub(1).max(k);
                    }
                    j += 1;
                }
                n - 1
            }
        }
    }

    fn c_cases(&mut self, open: usize) {
        let Some(close) = self.pair[open] else {
            return;
        };
        let mut labels = Vec::new();
        let mut j = open + 1;
        while j < close {
            let label = matches!(self.tx(j), "case" | "default")
                && matches!(self.before(j), "{" | "}" | ";" | ":");
            if label {
                labels.push(j);
            }
            if self.opener(j)
                && let Some(c) = self.pair[j]
            {
                j = c;
            }
            j += 1;
        }
        for (i, &s) in labels.iter().enumerate() {
            let next = labels.get(i + 1).copied().unwrap_or(close);
            self.add(self.toks[s].line, Some(self.end_of(next - 1)), false);
        }
    }

    fn template_end(&self, open: usize) -> usize {
        let mut j = open;
        let mut angle = 0i32;
        while j < self.toks.len() {
            match self.tx(j) {
                "<" => angle += 1,
                ">" => angle -= 1,
                "(" | "[" => j = self.pair[j].unwrap_or(j),
                _ => {}
            }
            if angle == 0 {
                break;
            }
            j += 1;
        }
        j += 1;
        while j < self.toks.len() {
            if self.punct(j, ";") {
                return j;
            }
            if self.punct(j, "{") {
                let c = self.pair[j].unwrap_or(j);
                return if self.punct(c + 1, ";") { c + 1 } else { c };
            }
            if self.opener(j) {
                j = self.pair[j].unwrap_or(j);
            }
            j += 1;
        }
        self.toks.len() - 1
    }

    fn lambda(&self, k: usize) -> Option<usize> {
        if let Some(p) = k.checked_sub(1) {
            let t = self.toks[p];
            let element = match t.kind {
                K::Word => !NOT_ELEMENT.contains(&t.text) && t.text != "return",
                K::Str => true,
                _ => matches!(t.text, ")" | "]" | "["),
            };
            if element || self.punct(k + 1, "[") {
                return None;
            }
        }
        let mut j = self.pair[k]? + 1;
        if self.punct(j, "(") {
            j = self.pair[j]? + 1;
        }
        for _ in 0..12 {
            match self.toks.get(j)? {
                t if t.kind == K::Punct && t.text == "{" => return Some(j),
                t if t.kind == K::Word || matches!(t.text, "-" | ">" | "<" | ":" | "*" | "&") => {
                    j += 1
                }
                _ => return None,
            }
        }
        None
    }

    fn function_start(&self, k: usize, lang: Lang) -> usize {
        let cpp = lang != Lang::C;
        let mut j = k;
        while j > 0 {
            j -= 1;
            let t = self.tx(j);
            if matches!(t, ")" | "]") || (t == "}" && matches!(self.tx(j + 1), "," | "{")) {
                match self.back[j] {
                    Some(o) => j = o,
                    None => return j + 1,
                }
                continue;
            }
            if matches!(t, ";" | "{" | "}") {
                return j + 1;
            }
            if t.len() > 1 && t.starts_with('@') {
                let line = self.toks[j].line;
                return (j + 1..k)
                    .find(|&i| self.toks[i].line > line)
                    .unwrap_or(j + 1);
            }
            if t == ":" && j > 0 && ACCESS.contains(&self.before(j)) {
                return j + 1;
            }
            if cpp && t == ">" && self.template_close(j) {
                return j + 1;
            }
        }
        0
    }

    fn template_close(&self, mut j: usize) -> bool {
        let mut angle = 0i32;
        while j > 0 {
            match self.tx(j) {
                ">" => angle += 1,
                "<" => angle -= 1,
                ";" | "{" | "}" => return false,
                _ => {}
            }
            if angle == 0 {
                return self.before(j) == "template";
            }
            j -= 1;
        }
        false
    }

    fn c_preproc(&mut self, lines: &[String], objc: bool) {
        let mut stack = Vec::new();
        let mut includes: Option<(usize, usize)> = None;
        let mut n = 0;
        while n < lines.len() {
            let line = lines[n].trim();
            let directive = line.strip_prefix('#').map(|d| {
                let d = d.trim_start();
                let w = d.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("");
                (w, d[w.len()..].to_owned())
            });
            let mut last = n;
            while lines[last].trim_end().ends_with('\\') && last + 1 < lines.len() {
                last += 1;
            }
            match directive.as_ref().map(|(w, r)| (*w, r.as_str())) {
                Some((w, _)) if w == "include" || (objc && w == "import") => {
                    includes = Some((includes.map_or(n, |r| r.0), n));
                }
                _ if line.is_empty() => {}
                _ => self.end_run(includes.take()),
            }
            if let Some((w, _)) = &directive {
                self.if_chain(&mut stack, w, n, lines);
            }
            match directive.as_ref().map(|(w, r)| (*w, r.as_str())) {
                Some(("define", rest)) if last > n => {
                    let name = rest.trim_start();
                    let ident = name
                        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .next()
                        .unwrap_or("");
                    if name[ident.len()..].starts_with('(') {
                        self.add(n, Some(last), false);
                    }
                }
                _ => {}
            }
            n = last + 1;
        }
        self.end_run(includes);
    }
}
