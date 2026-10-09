use std::collections::HashMap;
use std::path::Path;

use crate::search::{Kind, kind_of};

mod c;
mod csharp;
mod dart;
mod java;
mod kotlin;
mod objc;
mod php;
mod rust;
mod scala;
mod swift;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Lang {
    Js,
    Ts,
    Go,
    CSharp,
    Rust,
    Java,
    Kotlin,
    C,
    Cpp,
    ObjC,
    Swift,
    Php,
    Dart,
    Scala,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Syntax {
    lang: Lang,
    jsx: bool,
}

pub(crate) const OBJC: Syntax = Syntax {
    lang: Lang::ObjC,
    jsx: false,
};

pub(crate) fn syntax_of(path: &Path) -> Option<Syntax> {
    let ext = path.extension()?.to_str()?;
    let (lang, jsx) = match (kind_of(path), ext) {
        (Some(Kind::Swift), _) => (Lang::Swift, false),
        (Some(Kind::Php), _) => (Lang::Php, false),
        (_, "js" | "mjs" | "cjs") => (Lang::Js, false),
        (_, "jsx") => (Lang::Js, true),
        (_, "ts" | "mts" | "cts") => (Lang::Ts, false),
        (_, "tsx") => (Lang::Ts, true),
        (_, "go") => (Lang::Go, false),
        (_, "dart") => (Lang::Dart, false),
        (_, "scala" | "sc") => (Lang::Scala, false),
        (_, "cs") => (Lang::CSharp, false),
        (_, "rs") => (Lang::Rust, false),
        (_, "java") => (Lang::Java, false),
        (_, "kt" | "kts") => (Lang::Kotlin, false),
        (_, "c") => (Lang::C, false),
        (_, "m") => (Lang::ObjC, false),
        (_, "h" | "cc" | "cpp" | "cxx" | "c++" | "hh" | "hpp" | "hxx" | "h++" | "ipp") => {
            (Lang::Cpp, false)
        }
        _ => return None,
    };
    Some(Syntax { lang, jsx })
}

pub(crate) struct Folds {
    pub(crate) starts: HashMap<usize, usize>,
    pub(crate) heads: HashMap<usize, usize>,
    pub(crate) defs: Vec<(usize, usize)>,
}

pub(crate) fn folds(lines: &[String], syntax: Syntax) -> Folds {
    let mut toks = Lexer::run(lines, syntax);
    let comments: Vec<(usize, usize)> = (toks.iter())
        .filter(|t| t.kind == K::Comment)
        .map(|t| (t.line, t.end))
        .collect();
    toks.retain(|t| t.kind != K::Comment);
    let mut model = Model::new(&toks);
    for (s, e) in comments {
        model.add(s, Some(e), false);
    }
    match syntax.lang {
        Lang::Go => model.go(),
        Lang::CSharp => model.csharp(lines),
        Lang::Rust => model.rust(lines),
        Lang::Java => model.java(lines),
        Lang::Kotlin => model.kotlin(),
        Lang::C | Lang::Cpp | Lang::ObjC => model.c_family(syntax.lang, lines),
        Lang::Swift => model.swift(lines),
        Lang::Php => model.php(lines),
        Lang::Dart => model.dart(lines),
        Lang::Scala => model.scala(lines),
        lang => model.ecma(lang == Lang::Ts),
    }
    let levels = levels(lines.len(), &model.nodes);
    let mut starts = HashMap::new();
    for h in 0..lines.len() {
        if let Some(e) = closes_at(&levels, h) {
            starts.insert(h, e);
        }
    }
    let mut defs: Vec<(usize, usize)> = (model.nodes.iter())
        .filter(|n| n.2)
        .filter_map(|n| Some((n.0, *starts.get(&n.0)?)))
        .collect();
    defs.sort_unstable();
    defs.dedup();
    let semicolons = !matches!(
        syntax.lang,
        Lang::Js | Lang::Ts | Lang::Go | Lang::Kotlin | Lang::Swift | Lang::Scala
    );
    let heads = match semicolons {
        true => heads(lines, &toks, &model.back, &starts),
        false => HashMap::new(),
    };
    Folds {
        starts,
        heads,
        defs,
    }
}

fn heads(
    lines: &[String],
    toks: &[Tok],
    back: &[Option<usize>],
    starts: &HashMap<usize, usize>,
) -> HashMap<usize, usize> {
    let mut heads = HashMap::new();
    for (k, t) in toks.iter().enumerate() {
        let alone = lines[t.line].trim_start().starts_with('{');
        if !(alone && t.kind == K::Punct && t.text == "{" && starts.contains_key(&t.line)) {
            continue;
        }
        let header = |p: &Tok| p.kind == K::Word || matches!(p.text, ")" | "]" | ">");
        if !k.checked_sub(1).is_some_and(|p| header(&toks[p])) {
            continue;
        }
        let (mut j, mut from) = (k, k);
        while j > 0 {
            let p = toks[j - 1];
            let ends_line = toks[j].line > p.end;
            if p.kind == K::Punct
                && (matches!(p.text, ";" | "{" | "}" | "(" | "[") || (p.text == "," && ends_line))
            {
                break;
            }
            j = match p.text {
                ")" | "]" => back[j - 1].unwrap_or(j - 1),
                _ => j - 1,
            };
            from = j;
        }
        for l in toks[from].line..t.line {
            if !starts.contains_key(&l) {
                heads.entry(l).or_insert(t.line);
            }
        }
    }
    heads
}

fn levels(n: usize, nodes: &[(usize, usize, bool)]) -> Vec<(usize, bool)> {
    let mut ranges: Vec<(usize, usize)> = nodes
        .iter()
        .filter(|r| r.1 > r.0 && r.1 < n)
        .map(|r| (r.0, r.1))
        .collect();
    ranges.sort_unstable();
    ranges.dedup();
    let (mut enter, mut leave) = (vec![0usize; n], vec![0usize; n]);
    for (a, b) in ranges {
        enter[a] += 1;
        leave[b] += 1;
    }
    let mut out = Vec::with_capacity(n);
    let (mut prev, mut leave_prev) = (0usize, 0usize);
    for l in 0..n {
        let mut gone = leave[l];
        let mut level = (prev + enter[l]).saturating_sub(leave_prev);
        if enter[l] > 0 && gone > 0 {
            level = level.saturating_sub(gone);
            gone = 0;
        }
        out.push((level, enter[l] > 0));
        leave_prev = gone;
        prev = level;
    }
    out
}

fn closes_at(levels: &[(usize, bool)], h: usize) -> Option<usize> {
    let (level, start) = levels[h];
    if !start {
        return None;
    }
    let end = (h + 1..levels.len())
        .take_while(|&l| {
            let (x, st) = levels[l];
            x >= level && !(st && x <= level)
        })
        .last()?;
    Some(end)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum K {
    Word,
    Punct,
    Str,
    JsxOpen,
    JsxSelf,
    JsxClose,
    Comment,
}

#[derive(Clone, Copy, Debug)]
struct Tok<'a> {
    line: usize,
    end: usize,
    kind: K,
    text: &'a str,
}

struct Lexer<'a> {
    lines: &'a [String],
    syntax: Syntax,
    out: Vec<Tok<'a>>,
    l: usize,
    i: usize,
}

const REGEX_AFTER: &[&str] = &[
    "return", "typeof", "case", "in", "of", "delete", "void", "throw", "new", "else", "do",
];
const JSX_AFTER: &[&str] = &[
    "(", ",", "=", "?", ":", "&", "|", "=>", "{", "[", "return", "!", "default",
];

impl<'a> Lexer<'a> {
    fn run(lines: &'a [String], syntax: Syntax) -> Vec<Tok<'a>> {
        let mut lx = Lexer {
            lines,
            syntax,
            out: Vec::new(),
            l: 0,
            i: 0,
        };
        if !lines.is_empty() {
            if syntax.lang == Lang::Php {
                lx.php_html();
            }
            lx.code(false);
        }
        lx.out
    }

    fn bytes(&self) -> &'a [u8] {
        self.lines[self.l].as_bytes()
    }

    fn emit(&mut self, line: usize, at: usize, len: usize, kind: K, end: usize) {
        let text = match kind {
            K::Str => "`",
            K::Comment => "/*",
            K::JsxOpen | K::JsxSelf => "<",
            K::JsxClose => ">",
            K::Word | K::Punct => &self.lines[line][at..at + len],
        };
        self.out.push(Tok {
            line,
            end,
            kind,
            text,
        });
    }

    fn eof(&self) -> bool {
        self.l + 1 >= self.lines.len() && self.i >= self.bytes().len()
    }

    fn next_line(&mut self) -> bool {
        if self.l + 1 >= self.lines.len() {
            self.i = self.bytes().len();
            return false;
        }
        self.l += 1;
        self.i = 0;
        true
    }

    fn c_family(&self) -> bool {
        matches!(self.syntax.lang, Lang::C | Lang::Cpp | Lang::ObjC)
    }

    fn ecma(&self) -> bool {
        matches!(self.syntax.lang, Lang::Js | Lang::Ts)
    }

    fn code(&mut self, stop: bool) {
        let mut depth = 0usize;
        while !self.eof() {
            let s = self.bytes();
            if self.i >= s.len() {
                self.next_line();
                continue;
            }
            let (l, i, c) = (self.l, self.i, s[self.i]);
            if matches!(c, b' ' | b'\t' | b'\r') {
                self.i += 1;
                continue;
            }
            if s[i..].starts_with("\u{feff}".as_bytes()) {
                self.i += 3;
                continue;
            }
            if s[i..].starts_with(b"//") {
                self.i = s.len();
                continue;
            }
            if s[i..].starts_with(b"/*")
                && matches!(
                    self.syntax.lang,
                    Lang::Rust | Lang::Swift | Lang::Dart | Lang::Scala
                )
            {
                self.nested_comment(i + 2);
                continue;
            }
            if s[i..].starts_with(b"/*") {
                self.skip_past(i + 2, b"*/");
                if self.c_family() && self.l > l {
                    self.emit(l, i, 1, K::Comment, self.l);
                }
                continue;
            }
            if self.c_family() && c == b'#' && self.c_directive() {
                continue;
            }
            if self.syntax.lang == Lang::Cpp
                && matches!(c, b'R' | b'u' | b'U' | b'L')
                && self.cpp_raw()
            {
                continue;
            }
            if c == b'`'
                && matches!(self.syntax.lang, Lang::Swift | Lang::Scala)
                && let Some(len) = s[i + 1..].iter().position(|&b| b == b'`')
            {
                self.emit(l, i, len + 2, K::Word, l);
                self.i += len + 2;
                continue;
            }
            if c == b'`' {
                if self.ecma() {
                    self.template();
                } else {
                    self.skip_past(i + 1, b"`");
                    self.emit(l, i, 1, K::Str, self.l);
                }
                continue;
            }
            if self.syntax.lang == Lang::Rust
                && matches!(c, b'\'' | b'"' | b'r' | b'b' | b'c')
                && self.rust_quote()
            {
                continue;
            }
            if self.syntax.lang == Lang::Java && s[i..].starts_with(b"\"\"\"") {
                self.skip_past(i + 3, b"\"\"\"");
                self.emit(l, i, 1, K::Str, self.l);
                continue;
            }
            if self.syntax.lang == Lang::Php
                && matches!(c, b'#' | b'\'' | b'"' | b'<' | b'?')
                && self.php_quote()
            {
                continue;
            }
            if self.syntax.lang == Lang::Swift && matches!(c, b'"' | b'#') && self.swift_string() {
                continue;
            }
            if self.syntax.lang == Lang::Scala && matches!(c, b'"' | b'\'') && self.scala_quote() {
                continue;
            }
            if self.syntax.lang == Lang::Dart
                && matches!(c, b'"' | b'\'' | b'r')
                && self.dart_string()
            {
                continue;
            }
            if self.syntax.lang == Lang::Kotlin && c == b'"' && self.kt_string() {
                continue;
            }
            if self.syntax.lang == Lang::CSharp
                && matches!(c, b'"' | b'$' | b'@')
                && self.cs_string()
            {
                continue;
            }
            if c == b'"' || c == b'\'' {
                self.string(c);
                continue;
            }
            let prev = self.out.last().copied();
            if self.ecma() && c == b'/' {
                let regex = match prev {
                    None => true,
                    Some(p) if p.kind == K::Punct => !matches!(p.text, ")" | "]" | "}"),
                    Some(p) => p.kind == K::Word && REGEX_AFTER.contains(&p.text),
                };
                if regex {
                    self.regex();
                    continue;
                }
            }
            if self.syntax.jsx
                && c == b'<'
                && prev.is_none_or(|p| p.kind != K::Str && JSX_AFTER.contains(&p.text))
                && s.get(i + 1)
                    .is_some_and(|&n| n.is_ascii_alphabetic() || n == b'>')
            {
                self.jsx();
                continue;
            }
            if word(c) {
                let php = self.syntax.lang == Lang::Php;
                let len = 1
                    + (s[i + 1..].iter())
                        .take_while(|&&b| word(b) && !(php && b == b'#'))
                        .count();
                self.emit(l, i, len, K::Word, l);
                self.i += len;
                continue;
            }
            if s[i..].starts_with(b"=>") {
                self.emit(l, i, 2, K::Punct, l);
                self.i += 2;
                continue;
            }
            if c == b'{' {
                depth += 1;
            } else if c == b'}' {
                if depth == 0 && stop {
                    self.emit(l, i, 1, K::Punct, l);
                    self.i += 1;
                    return;
                }
                depth = depth.saturating_sub(1);
            }
            self.emit(l, i, 1, K::Punct, l);
            self.i += 1;
        }
    }

    fn skip_past(&mut self, from: usize, delim: &[u8]) {
        self.i = from;
        loop {
            let s = self.bytes();
            if let Some(j) = find(&s[self.i.min(s.len())..], delim) {
                self.i += j + delim.len();
                return;
            }
            if !self.next_line() {
                return;
            }
        }
    }

    fn string(&mut self, q: u8) {
        let (l, at) = (self.l, self.i);
        let s = self.bytes();
        let mut i = at + 1;
        while i < s.len() {
            if s[i] == b'\\' {
                i += 2;
                continue;
            }
            i += 1;
            if s[i - 1] == q {
                break;
            }
        }
        self.i = i.min(s.len());
        self.emit(l, at, 1, K::Str, l);
    }

    fn regex(&mut self) {
        let (l, at) = (self.l, self.i);
        let s = self.bytes();
        let (mut j, mut class) = (at + 1, false);
        while j < s.len() {
            match s[j] {
                b'\\' => j += 1,
                b'[' => class = true,
                b']' => class = false,
                b'/' if !class => break,
                _ => {}
            }
            j += 1;
        }
        self.emit(l, at, 1, K::Str, l);
        j = (j + 1).min(s.len());
        while j < s.len() && s[j].is_ascii_alphabetic() {
            j += 1;
        }
        self.i = j;
    }

    fn template(&mut self) {
        let (mut l, mut at) = (self.l, self.i);
        self.i += 1;
        while !self.eof() {
            let s = self.bytes();
            if self.i >= s.len() {
                self.next_line();
                continue;
            }
            let c = s[self.i];
            if c == b'\\' {
                self.i += 2;
                continue;
            }
            if s[self.i..].starts_with(b"${") {
                self.emit(l, at, 1, K::Str, self.l);
                self.emit(self.l, self.i + 1, 1, K::Punct, self.l);
                self.i += 2;
                self.code(true);
                (l, at) = (self.l, self.i);
                continue;
            }
            self.i += 1;
            if c == b'`' {
                break;
            }
        }
        self.emit(l, at, 1, K::Str, self.l);
    }

    fn jsx(&mut self) {
        let start = self.out.len();
        self.emit(self.l, self.i, 1, K::JsxOpen, self.l);
        self.i += 1;
        if self.bytes().get(self.i) == Some(&b'>') {
            self.i += 1;
        } else if self.tag() {
            self.out[start].kind = K::JsxSelf;
            self.emit(self.l, self.i - 1, 1, K::JsxClose, self.l);
            return;
        }
        while !self.eof() {
            let s = self.bytes();
            if self.i >= s.len() {
                self.next_line();
                continue;
            }
            match s[self.i] {
                b'{' => {
                    self.emit(self.l, self.i, 1, K::Punct, self.l);
                    self.i += 1;
                    self.code(true);
                }
                b'<' if s.get(self.i + 1) == Some(&b'/') => {
                    while !self.eof() && self.bytes().get(self.i) != Some(&b'>') {
                        if self.i >= self.bytes().len() {
                            self.next_line();
                        } else {
                            self.i += 1;
                        }
                    }
                    self.emit(self.l, self.i, 0, K::JsxClose, self.l);
                    self.i += 1;
                    return;
                }
                b'<' => self.jsx(),
                _ => self.i += 1,
            }
        }
    }

    fn tag(&mut self) -> bool {
        while !self.eof() {
            let s = self.bytes();
            if self.i >= s.len() {
                self.next_line();
                continue;
            }
            match s[self.i] {
                b'{' => {
                    self.emit(self.l, self.i, 1, K::Punct, self.l);
                    self.i += 1;
                    self.code(true);
                }
                q @ (b'"' | b'\'') => {
                    self.string(q);
                    self.out.pop();
                }
                b'/' if s.get(self.i + 1) == Some(&b'>') => {
                    self.i += 2;
                    return true;
                }
                b'>' => {
                    self.i += 1;
                    return false;
                }
                _ => self.i += 1,
            }
        }
        false
    }
}

fn word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'$' | b'#' | b'@') || b >= 0x80
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

const NOT_CALLEE: &[&str] = &[
    "if",
    "for",
    "while",
    "switch",
    "catch",
    "function",
    "with",
    "return",
    "typeof",
    "await",
    "in",
    "of",
    "async",
    "yield",
    "case",
    "new",
    "else",
    "throw",
    "void",
    "delete",
    "instanceof",
    "extends",
    "as",
    "satisfies",
    "keyof",
];
const EXPR_BEFORE: &[&str] = &[
    "{", "=", "(", ",", ":", "[", "?", "|", "&", "!", "=>", "+", "-", "*", "%", "<", ">", "~", "^",
];
const HEADS: &[&str] = &["if", "for", "while", "switch", "catch", "with"];
const GO_LINE_END: &[&str] = &[
    "break",
    "continue",
    "fallthrough",
    "return",
    ")",
    "]",
    "}",
    "++",
    "--",
];

struct Model<'t, 'a> {
    toks: &'t [Tok<'a>],
    pair: Vec<Option<usize>>,
    back: Vec<Option<usize>>,
    first: Vec<bool>,
    nodes: Vec<(usize, usize, bool)>,
}

impl<'t, 'a> Model<'t, 'a> {
    fn new(toks: &'t [Tok<'a>]) -> Self {
        let (mut pair, mut back) = (vec![None; toks.len()], vec![None; toks.len()]);
        let mut stack = Vec::new();
        for (k, t) in toks.iter().enumerate() {
            let open = matches!(t.kind, K::JsxOpen | K::JsxSelf)
                || (t.kind == K::Punct && matches!(t.text, "(" | "[" | "{"));
            let close =
                t.kind == K::JsxClose || (t.kind == K::Punct && matches!(t.text, ")" | "]" | "}"));
            if open {
                stack.push(k);
            } else if close && let Some(o) = stack.pop() {
                pair[o] = Some(k);
                back[k] = Some(o);
            }
        }
        let first = (0..toks.len())
            .map(|k| k == 0 || toks[k - 1].line != toks[k].line)
            .collect();
        Model {
            toks,
            pair,
            back,
            first,
            nodes: Vec::new(),
        }
    }

    fn tx(&self, k: usize) -> &str {
        self.toks.get(k).map_or("", |t| t.text)
    }

    fn before(&self, k: usize) -> &str {
        k.checked_sub(1).map_or("", |p| self.tx(p))
    }

    fn punct(&self, k: usize, text: &str) -> bool {
        self.toks
            .get(k)
            .is_some_and(|t| t.kind == K::Punct && t.text == text)
    }

    fn opener(&self, k: usize) -> bool {
        self.toks[k].kind == K::Punct && matches!(self.toks[k].text, "(" | "[" | "{")
    }

    fn closer(&self, k: usize) -> bool {
        self.toks[k].kind == K::Punct && matches!(self.toks[k].text, ")" | "]" | "}")
    }

    fn end_of(&self, k: usize) -> usize {
        self.toks[k].end
    }

    fn closing(&self, k: usize) -> Option<usize> {
        self.pair[k].map(|c| self.end_of(c))
    }

    fn join_run(
        &mut self,
        run: Option<(usize, usize)>,
        (s, e): (usize, usize),
        lines: &[String],
    ) -> Option<(usize, usize)> {
        match run {
            Some((rs, re)) if (re + 1..s).all(|i| lines[i].trim().is_empty()) => Some((rs, e)),
            run => {
                self.end_run(run);
                Some((s, e))
            }
        }
    }

    fn end_run(&mut self, run: Option<(usize, usize)>) {
        if let Some((s, e)) = run {
            self.add(s, Some(e), false);
        }
    }

    fn if_chain(
        &mut self,
        chain: &mut Vec<(usize, Vec<usize>)>,
        word: &str,
        n: usize,
        lines: &[String],
    ) {
        match word {
            "if" | "ifdef" | "ifndef" => chain.push((n, Vec::new())),
            "elif" | "elifdef" | "elifndef" | "else" => {
                if let Some(top) = chain.last_mut() {
                    top.1.push(n);
                }
            }
            "endif" => {
                if let Some((h, alts)) = chain.pop() {
                    self.add(h, Some(n), false);
                    let last = (h..n).rev().find(|&i| !lines[i].trim().is_empty());
                    for a in alts {
                        self.add(a, last, false);
                    }
                }
            }
            _ => {}
        }
    }

    fn word_at(&self, k: usize) -> bool {
        self.toks.get(k).is_some_and(|t| t.kind == K::Word)
    }

    fn loop_tail(&self, k: usize, head: &str) -> bool {
        self.tx(k) == "while"
            && k > 0
            && self.punct(k - 1, "}")
            && self.back[k - 1].is_some_and(|o| self.before(o) == head)
    }

    fn add(&mut self, start: usize, end: Option<usize>, def: bool) {
        if let Some(end) = end.filter(|&e| e > start) {
            self.nodes.push((start, end, def));
        }
    }

    fn ecma(&mut self, ts: bool) {
        let n = self.toks.len();
        let mut k = 0;
        while k < n {
            if self.import(k) {
                let start = self.toks[k].line;
                let mut end = self.import_end(k);
                while end + 1 < n && self.import(end + 1) {
                    end = self.import_end(end + 1);
                }
                self.add(start, Some(self.end_of(end)), false);
                k = end + 1;
            } else {
                k += 1;
            }
        }
        for k in 0..n {
            self.ecma_node(k, ts);
        }
    }

    fn import(&self, k: usize) -> bool {
        self.tx(k) == "import" && self.first[k] && !matches!(self.tx(k + 1), "(" | ".")
    }

    fn import_end(&self, k: usize) -> usize {
        let (mut j, mut last) = (k + 1, k);
        while j < self.toks.len() {
            if self.opener(j)
                && let Some(c) = self.pair[j]
            {
                (j, last) = (c + 1, c);
                continue;
            }
            if self.tx(j) == ";" {
                return j;
            }
            if self.first[j] && self.toks[j].line > self.toks[last].end {
                return last;
            }
            (last, j) = (j, j + 1);
        }
        last
    }

    fn ecma_node(&mut self, k: usize, ts: bool) {
        let t = self.toks[k];
        let line = t.line;
        let w = t.text;
        match t.kind {
            K::Word
                if matches!(
                    w,
                    "if" | "for" | "while" | "do" | "switch" | "try" | "with" | "catch"
                ) =>
            {
                self.add(line, self.stmt_end(k), false)
            }
            K::Word if matches!(w, "function" | "class") => {
                if self.before(k) != "." {
                    self.add(line, self.stmt_end(k), true);
                }
            }
            K::Word
                if ts
                    && matches!(w, "interface" | "enum" | "namespace" | "module")
                    && self.toks.get(k + 1).is_some_and(|n| n.kind == K::Word) =>
            {
                self.add(line, self.chain(k, &[]), false)
            }
            K::Word
                if ts
                    && w == "type"
                    && self.toks.get(k + 1).is_some_and(|n| n.kind == K::Word)
                    && (self.first[k] || matches!(self.before(k), "export" | "declare")) =>
            {
                self.add(line, Some(self.alias_end(k)), false)
            }
            K::Word if ts && w.starts_with('@') && self.first[k] => {
                let mut j = k;
                while self.tx(j).starts_with('@') {
                    j += 1;
                    while self.tx(j) == "." {
                        j += 2;
                    }
                    if self.punct(j, "(")
                        && let Some(c) = self.pair[j]
                    {
                        j = c + 1;
                    }
                }
                if self.tx(j) == "class" {
                    self.add(line, self.stmt_end(j), true);
                }
            }
            K::Word if matches!(w, "case" | "default") && self.first[k] => {
                self.add(line, Some(self.case_end(k)), false)
            }
            K::JsxOpen => self.add(line, self.closing(k), false),
            K::Punct if w == "=>" && k > 0 && self.toks[k - 1].kind == K::Word => {
                self.add(line, Some(self.arrow_end(k)), true)
            }
            K::Punct if w == "(" => self.paren(k),
            K::Punct if w == "[" => {
                let array = k == 0 || {
                    let p = self.toks[k - 1];
                    (p.kind == K::Punct && !matches!(p.text, ")" | "]" | "}"))
                        || (p.kind == K::Word
                            && matches!(
                                p.text,
                                "return" | "of" | "in" | "await" | "yield" | "case"
                            ))
                };
                if array {
                    self.add(line, self.closing(k), false);
                }
            }
            K::Punct if w == "{" && k > 0 => {
                let p = self.toks[k - 1];
                if matches!(p.text, "(" | ",") && self.in_params(k) {
                    return;
                }
                let object = (p.kind == K::Punct && EXPR_BEFORE.contains(&p.text))
                    || (p.kind == K::Word
                        && matches!(p.text, "return" | "yield" | "await" | "of" | "in" | "case"));
                if object {
                    self.add(line, self.closing(k), false);
                }
            }
            _ => {}
        }
    }

    fn paren(&mut self, k: usize) {
        let Some(c) = self.pair[k] else {
            return;
        };
        let line = self.toks[k].line;
        let prev = k.checked_sub(1).map(|p| self.toks[p]);
        let mut a = Some(c + 1);
        if a.is_some_and(|a| self.punct(a, ":")) {
            a = self.skip_type(c + 2);
        }
        let after = a.map_or("", |a| self.tx(a));
        if matches!(after, "{" | "=>") && !prev.is_some_and(|p| HEADS.contains(&p.text)) {
            let a = a.unwrap_or(c);
            let end = match after {
                "=>" => Some(self.arrow_end(a)),
                _ => self.closing(a),
            };
            self.add(line, end, true);
            return;
        }
        let callee = prev.is_some_and(|p| {
            (p.kind == K::Word && (!NOT_CALLEE.contains(&p.text) || self.before(k - 1) == "."))
                || (p.kind != K::Word && matches!(p.text, ")" | "]" | "." | ">"))
                || (p.kind == K::Str && p.text == "`")
        });
        if callee {
            self.add(line, Some(self.end_of(c)), false);
        }
    }

    fn skip_type(&self, mut j: usize) -> Option<usize> {
        let mut angle = 0i32;
        while j < self.toks.len() {
            let t = self.toks[j];
            if t.kind == K::Punct {
                match t.text {
                    "<" => angle += 1,
                    ">" => angle -= 1,
                    "=>" if angle <= 0 => return Some(j),
                    "{" if angle <= 0 => {
                        if !matches!(self.before(j), ":" | "|" | "&" | "<" | ",") {
                            return Some(j);
                        }
                        j = self.pair[j].unwrap_or(j);
                    }
                    "(" | "[" | "{" => j = self.pair[j].unwrap_or(j),
                    ")" | "]" | "}" | ";" | "," => return None,
                    _ => {}
                }
            }
            j += 1;
        }
        None
    }

    fn chain(&self, k: usize, more: &[&str]) -> Option<usize> {
        let mut j = k;
        while j < self.toks.len() {
            if self.opener(j) {
                j = self.pair[j]?;
                if self.tx(j) == "}" {
                    let next = self.tx(j + 1);
                    if more.contains(&next) || (next == "while" && self.tx(k) == "do") {
                        j += 1;
                        continue;
                    }
                    return Some(self.end_of(j));
                }
                j += 1;
                continue;
            }
            if self.punct(j, ";") {
                return Some(self.end_of(j));
            }
            if self.closer(j) {
                return Some(self.end_of(j.checked_sub(1)?));
            }
            j += 1;
        }
        None
    }

    fn stmt_end(&self, k: usize) -> Option<usize> {
        match self.tx(k) {
            "if" => self.chain(k, &["else"]),
            "try" => self.chain(k, &["catch", "finally"]),
            _ => self.chain(k, &[]),
        }
    }

    fn alias_end(&self, k: usize) -> usize {
        let (mut j, mut last) = (k + 1, k);
        while j < self.toks.len() {
            let t = self.toks[j];
            if self.opener(j)
                && let Some(c) = self.pair[j]
            {
                (j, last) = (c + 1, c);
                continue;
            }
            if t.text == ";" {
                return self.end_of(j);
            }
            if self.closer(j) {
                return self.end_of(last);
            }
            let continues = matches!(t.text, "|" | "&" | "." | "?" | ":" | ">")
                || matches!(
                    self.tx(last),
                    "=" | "|" | "&" | "<" | "," | ":" | "?" | "extends"
                );
            if self.first[j] && t.line > self.toks[last].end && !continues {
                return self.end_of(last);
            }
            (last, j) = (j, j + 1);
        }
        self.end_of(last)
    }

    fn in_params(&self, k: usize) -> bool {
        let mut depth = 0usize;
        for j in (0..k).rev() {
            if self.closer(j) {
                depth += 1;
            } else if self.opener(j) {
                if depth == 0 {
                    return self.tx(j) == "("
                        && self.pair[j]
                            .is_some_and(|c| matches!(self.tx(c + 1), "{" | "=>" | ":"));
                }
                depth -= 1;
            }
        }
        false
    }

    fn arrow_end(&self, k: usize) -> usize {
        let mut j = k + 1;
        if self.punct(j, "{")
            && let Some(c) = self.pair[j]
        {
            return self.end_of(c);
        }
        let mut last = j.min(self.toks.len().saturating_sub(1));
        while j < self.toks.len() {
            if self.opener(j)
                && let Some(c) = self.pair[j]
            {
                (j, last) = (c + 1, c);
                continue;
            }
            if self.closer(j) || self.punct(j, ",") || self.punct(j, ";") {
                break;
            }
            (last, j) = (j, j + 1);
        }
        self.end_of(last)
    }

    fn case_end(&self, k: usize) -> usize {
        let (mut j, mut last) = (k + 1, k);
        while j < self.toks.len() {
            if self.opener(j)
                && let Some(c) = self.pair[j]
            {
                (j, last) = (c + 1, c);
                continue;
            }
            if self.closer(j) || (self.first[j] && matches!(self.tx(j), "case" | "default")) {
                break;
            }
            (last, j) = (j, j + 1);
        }
        self.end_of(last)
    }

    fn go(&mut self) {
        let n = self.toks.len();
        let mut stack = Vec::new();
        let parent: Vec<Option<usize>> = (0..n)
            .map(|k| {
                if self.closer(k) {
                    stack.pop();
                }
                let up = stack.last().copied();
                if self.opener(k) {
                    stack.push(k);
                }
                up
            })
            .collect();
        let mut body: HashMap<usize, &str> = HashMap::new();
        for k in 0..n {
            let t = self.toks[k];
            if t.kind != K::Word {
                continue;
            }
            let start = self.first[k] || matches!(self.before(k), ";" | "else" | "{" | "}" | ":");
            match t.text {
                w @ ("if" | "for" | "switch" | "select") if start => {
                    if let Some(b) = self.go_body(k) {
                        body.insert(b, w);
                    }
                    if w != "select" {
                        self.add(t.line, Some(self.go_stmt_end(k)), false);
                    }
                }
                "func" => {
                    if let Some(b) = self.go_body(k) {
                        body.insert(b, "func");
                        self.add(t.line, self.closing(b), true);
                    }
                }
                "type" | "var" | "const" | "import" if start => {
                    self.add(t.line, Some(self.go_stmt_end(k)), false)
                }
                "else" if self.punct(k + 1, "{") => {
                    body.insert(k + 1, "else");
                }
                _ => {}
            }
        }
        let mut literal = vec![false; n];
        for k in 0..n {
            if !self.punct(k, "{") {
                continue;
            }
            let Some(c) = self.pair[k] else {
                continue;
            };
            let line = self.toks[k].line;
            match body.get(&k) {
                Some(&"switch") => {
                    self.cases(k, c);
                    continue;
                }
                Some(&"select") => continue,
                Some(_) => {
                    self.add(line, Some(self.end_of(c)), false);
                    continue;
                }
                None => {}
            }
            let prev = self.before(k);
            if matches!(prev, "struct" | "interface") {
                continue;
            }
            let inside = parent[k].is_some_and(|p| literal[p]);
            if self.first[k] && matches!(prev, ";" | "{" | "}" | ":" | "") && !inside {
                self.add(line, Some(self.end_of(c)), false);
                continue;
            }
            literal[k] = true;
            let typed = (prev == "}")
                .then(|| self.back[k - 1])
                .flatten()
                .filter(|&o| matches!(self.before(o), "struct" | "interface"));
            let from = typed.map_or(line, |o| self.toks[o - 1].line);
            self.add(from, Some(self.end_of(c)), false);
            self.elements(k, c);
        }
    }

    fn go_stmt_end(&self, k: usize) -> usize {
        let (mut j, mut last) = (k, k);
        while j < self.toks.len() {
            let t = self.toks[j];
            if j > k && t.line > self.toks[last].end {
                let lt = self.toks[last];
                let ends = matches!(lt.kind, K::Word | K::Str) || GO_LINE_END.contains(&lt.text);
                if ends && lt.text != "else" {
                    return self.end_of(last);
                }
            }
            if self.opener(j)
                && let Some(c) = self.pair[j]
            {
                (j, last) = (c + 1, c);
                continue;
            }
            if self.punct(j, ";") || self.closer(j) {
                return self.end_of(last);
            }
            (last, j) = (j, j + 1);
        }
        self.end_of(last)
    }

    fn go_body(&self, k: usize) -> Option<usize> {
        let func = self.tx(k) == "func";
        let mut j = k + 1;
        while j < self.toks.len() {
            let t = self.toks[j];
            let stops = self.closer(j) || matches!(t.text, "," | ";" | "=");
            if func && ((t.kind == K::Punct && stops) || t.line > self.toks[j - 1].end) {
                return None;
            }
            if self.punct(j, "(") || self.punct(j, "[") {
                j = self.pair[j]? + 1;
                continue;
            }
            if self.punct(j, "{") {
                if matches!(self.before(j), "struct" | "interface") {
                    j = self.pair[j]? + 1;
                    if self.punct(j, "{") {
                        j = self.pair[j]? + 1;
                    }
                    continue;
                }
                return Some(j);
            }
            if t.line > self.toks[k].line + 50 {
                return None;
            }
            j += 1;
        }
        None
    }

    fn elements(&mut self, k: usize, c: usize) {
        let (mut j, mut first, mut last) = (k + 1, None::<usize>, k);
        while j < c {
            if self.punct(j, ",") || self.punct(j, ":") {
                if let Some(f) = first.take() {
                    let line = self.toks[f].line;
                    self.add(line, Some(self.end_of(last)), false);
                }
                j += 1;
                continue;
            }
            first.get_or_insert(j);
            if self.opener(j)
                && let Some(p) = self.pair[j]
            {
                j = p;
            }
            last = j;
            j += 1;
        }
        if let Some(f) = first {
            let line = self.toks[f].line;
            self.add(line, Some(self.end_of(last)), false);
        }
    }

    fn cases(&mut self, k: usize, c: usize) {
        let mut starts = Vec::new();
        let mut j = k + 1;
        while j < c {
            if self.first[j] && matches!(self.tx(j), "case" | "default") {
                starts.push(j);
            }
            if self.opener(j)
                && let Some(p) = self.pair[j]
            {
                j = p;
            }
            j += 1;
        }
        for (i, &s) in starts.iter().enumerate() {
            let next = starts
                .get(i + 1)
                .map_or(self.toks[c].line, |&n| self.toks[n].line);
            self.add(self.toks[s].line, Some(next.saturating_sub(1)), false);
        }
    }
}
