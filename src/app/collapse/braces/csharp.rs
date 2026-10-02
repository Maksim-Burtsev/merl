use super::{K, Lexer, Model};

const HEADS: &[&str] = &[
    "if", "for", "foreach", "while", "switch", "catch", "using", "lock", "fixed", "when",
];
const PATTERN: &[&str] = &["is", "case", "and", "or", "not"];
const LEADS: &[&str] = &[
    "new",
    "namespace",
    "class",
    "struct",
    "interface",
    "record",
    "enum",
    "is",
    "case",
    "and",
    "or",
    "not",
    "return",
];

#[derive(Clone, Copy, PartialEq)]
enum Brace {
    Code,
    Data,
}

impl Lexer<'_> {
    pub(super) fn cs_string(&mut self) -> bool {
        let s = self.bytes();
        let (l, at) = (self.l, self.i);
        let mut j = at;
        let (mut holes, mut verbatim) = (false, false);
        while j < s.len() && matches!(s[j], b'$' | b'@') {
            holes |= s[j] == b'$';
            verbatim |= s[j] == b'@';
            j += 1;
        }
        if s.get(j) != Some(&b'"') {
            return false;
        }
        let quotes = s[j..].iter().take_while(|&&b| b == b'"').count();
        if quotes >= 3 && !verbatim {
            self.skip_past(j + quotes, &vec![b'"'; quotes]);
            self.emit(l, at, 1, K::Str, self.l);
            return true;
        }
        self.i = j + 1;
        let mut depth = 0usize;
        loop {
            let s = self.bytes();
            if self.i >= s.len() {
                if verbatim && self.next_line() {
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
                        self.string(c);
                        self.out.pop();
                        continue;
                    }
                    _ => {}
                }
                self.i += 1;
                continue;
            }
            match c {
                b'\\' if !verbatim => self.i += 2,
                b'"' if verbatim && s.get(self.i + 1) == Some(&b'"') => self.i += 2,
                b'"' => {
                    self.i += 1;
                    break;
                }
                b'{' if holes && s.get(self.i + 1) == Some(&b'{') => self.i += 2,
                b'{' if holes => {
                    depth = 1;
                    self.i += 1;
                }
                _ => self.i += 1,
            }
        }
        self.emit(l, at, 1, K::Str, self.l);
        true
    }
}

impl Model<'_, '_> {
    pub(super) fn csharp(&mut self, lines: &[String]) {
        self.cs_preproc(lines);
        let n = self.toks.len();
        let mut brace = vec![Brace::Code; n];
        let mut stack: Vec<usize> = Vec::new();
        let mut usings: Option<(usize, usize)> = None;
        let mut k = 0;
        while k < n {
            let parent = stack.last().copied();
            if self.using(k, parent) {
                let end = (k..n).find(|&j| self.punct(j, ";")).unwrap_or(n - 1);
                let (s, e) = (self.toks[k].line, self.toks[end].end);
                usings = match usings {
                    Some((rs, re)) if (re + 1..s).all(|i| lines[i].trim().is_empty()) => {
                        Some((rs, e))
                    }
                    run => {
                        self.add_run(run);
                        Some((s, e))
                    }
                };
                k = end + 1;
                continue;
            }
            self.add_run(usings.take());
            if self.closer(k) {
                stack.pop();
            } else if self.opener(k) {
                stack.push(k);
                if self.tx(k) == "{" {
                    let up = parent.map_or(Brace::Code, |p| brace[p]);
                    let (b, fold, def) = self.cs_brace(k, up);
                    brace[k] = b;
                    if fold {
                        self.add(self.toks[k].line, self.closing(k), def);
                    }
                } else {
                    brace[k] = Brace::Data;
                }
            }
            k += 1;
        }
        self.add_run(usings);
    }

    fn add_run(&mut self, run: Option<(usize, usize)>) {
        if let Some((s, e)) = run {
            self.add(s, Some(e), false);
        }
    }

    fn using(&self, k: usize, parent: Option<usize>) -> bool {
        let head = match self.tx(k) {
            "global" => self.tx(k + 1) == "using",
            "using" => k == 0 || self.before(k) != "global",
            _ => false,
        };
        let top = parent.is_none_or(|p| {
            let start = self.chain_start(p - 1);
            self.tx(start) == "namespace"
        });
        head && self.first[k] && top && !matches!(self.tx(k + 1), "(" | "var")
    }

    fn chain_start(&self, mut j: usize) -> usize {
        loop {
            let t = self.toks[j];
            if t.kind == K::Punct && t.text == ">" {
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
            } else if LEADS.contains(&t.text)
                || !(t.kind == K::Word || matches!(t.text, "." | "?" | ":" | "[" | "]"))
            {
                return j;
            }
            if j == 0 {
                return 0;
            }
            j -= 1;
        }
    }

    fn cs_brace(&self, k: usize, up: Brace) -> (Brace, bool, bool) {
        let Some(p) = k.checked_sub(1) else {
            return (Brace::Code, true, false);
        };
        let pt = self.toks[p];
        let data = (Brace::Data, false, false);
        let code = |def| (Brace::Code, true, def);
        match pt.text {
            "=>" => return code(true),
            ";" | "}" => return code(false),
            "{" | ":" if up == Brace::Code => return code(false),
            "switch" | "with" | "new" => return data,
            w if PATTERN.contains(&w) => return data,
            _ => {}
        }
        let callee = match pt.text {
            ")" => match self.back[p] {
                Some(o) if self.before(o) == "new" && self.constraint(o - 1) => return code(true),
                Some(o) if self.before(o) == "new" => return data,
                Some(o) if o > 0 => o - 1,
                _ => return code(false),
            },
            ">" => p,
            _ if pt.kind == K::Word => p,
            _ => return data,
        };
        let before = self.chain_start(callee);
        let lead = self.tx(before);
        let words = |j: usize| self.toks[j].kind == K::Word && self.toks[j + 1].kind == K::Word;
        if lead == "new" && !(before + 1..callee).any(words) {
            return (Brace::Data, true, false);
        }
        if up == Brace::Data || PATTERN.contains(&lead) {
            return data;
        }
        let def = match pt.text {
            ")" => !HEADS.contains(&self.tx(callee)),
            "get" | "set" | "init" | "add" | "remove" => true,
            _ => matches!(lead, "class" | "struct" | "interface" | "record"),
        };
        code(def)
    }

    fn constraint(&self, new: usize) -> bool {
        let mut j = new;
        while j > 0 && !matches!(self.tx(j), ";" | "{" | "}" | "=" | "(" | "where") {
            j -= 1;
        }
        self.tx(j) == "where"
    }

    fn cs_preproc(&mut self, lines: &[String]) {
        let mut stack: Vec<(usize, Vec<usize>)> = Vec::new();
        for (n, line) in lines.iter().enumerate() {
            let Some(d) = line.trim_start().strip_prefix('#') else {
                continue;
            };
            let word = d.trim_start().split(|c: char| !c.is_alphabetic()).next();
            match word {
                Some("if") => stack.push((n, Vec::new())),
                Some("elif" | "else") => {
                    if let Some(top) = stack.last_mut() {
                        top.1.push(n);
                    }
                }
                Some("endif") => {
                    let Some((h, alts)) = stack.pop() else {
                        continue;
                    };
                    self.add(h, Some(n), false);
                    let last = (h..n).rev().find(|&i| !lines[i].trim().is_empty());
                    for a in alts {
                        self.add(a, last, false);
                    }
                }
                _ => {}
            }
        }
    }
}
