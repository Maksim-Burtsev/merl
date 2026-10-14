use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct Blocks {
    spans: HashMap<usize, usize>,
    funcs: Vec<(usize, usize)>,
}

impl Blocks {
    pub(crate) fn from_nodes(
        nodes: Vec<(usize, usize)>,
        funcs: Vec<(usize, usize)>,
        len: usize,
    ) -> Self {
        let spans = super::vim_folds(len, nodes.into_iter());
        Self { spans, funcs }
    }

    pub(crate) fn region(&self, h: usize) -> Option<usize> {
        self.spans.get(&h).copied()
    }

    pub(crate) fn target(&self, l: usize) -> Option<(usize, usize)> {
        if let Some(e) = self.region(l) {
            return Some((l, e));
        }
        let (h, _) = (self.funcs.iter().copied())
            .filter(|&(h, e)| h < l && l <= e)
            .max_by_key(|&(h, _)| h)?;
        Some((h, self.region(h)?))
    }
}

struct Open {
    line: usize,
    word: &'static str,
    fold: bool,
    func: bool,
    armed: bool,
    clause: Option<usize>,
}

struct Build<'a> {
    lines: &'a [String],
    stack: Vec<Open>,
    blocks: Blocks,
}

impl<'a> Build<'a> {
    fn new(lines: &'a [String]) -> Self {
        Build {
            lines,
            stack: Vec::new(),
            blocks: Blocks::default(),
        }
    }

    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "the three flags are independent facts about the opener, stored as they come"
    )]
    fn open(&mut self, line: usize, word: &'static str, fold: bool, func: bool, armed: bool) {
        self.stack.push(Open {
            line,
            word,
            fold,
            func,
            armed,
            clause: None,
        });
    }

    fn span(&mut self, h: usize, e: usize) {
        if e > h {
            let end = self.blocks.spans.entry(h).or_insert(e);
            *end = (*end).max(e);
        }
    }

    fn end_clause(&mut self, line: usize) {
        let Some(c) = self.stack.last_mut().and_then(|o| o.clause.take()) else {
            return;
        };
        let last = (c..line).rev().find(|&i| !self.lines[i].trim().is_empty());
        self.span(c, last.unwrap_or(c));
    }

    fn clause(&mut self, line: usize) {
        self.end_clause(line);
        if let Some(top) = self.stack.last_mut() {
            top.clause = Some(line);
        }
    }

    fn close(&mut self, line: usize, words: &[&str]) {
        let Some(at) = self.stack.iter().rposition(|o| words.contains(&o.word)) else {
            return;
        };
        self.stack.truncate(at + 1);
        self.end_clause(line);
        let o = self.stack.pop().expect("truncated to at + 1");
        if o.fold {
            self.span(o.line, line);
        }
        if o.func && line > o.line {
            self.blocks.funcs.push((o.line, line));
        }
    }

    fn top(&self) -> Option<&'static str> {
        self.stack.last().map(|o| o.word)
    }
}

pub(super) fn word_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80
}

fn word_at(b: &[u8], i: usize) -> usize {
    let mut j = i;
    while j < b.len() && word_byte(b[j]) {
        j += 1;
    }
    j
}

fn text(s: &[u8], i: usize, j: usize) -> &str {
    std::str::from_utf8(&s[i..j]).unwrap_or("")
}

fn brace_end(b: &[u8], mut i: usize) -> usize {
    let mut depth = 0;
    while i < b.len() {
        match b[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    i
}

fn closing(open: u8) -> u8 {
    match open {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        b'<' => b'>',
        c => c,
    }
}

#[derive(Clone, Copy)]
struct Quote {
    open: u8,
    close: u8,
    depth: usize,
    interp: bool,
}

impl Quote {
    fn new(open: u8, interp: bool) -> Self {
        Self {
            open,
            close: closing(open),
            depth: 0,
            interp,
        }
    }

    fn scan(&mut self, b: &[u8], mut i: usize) -> Option<usize> {
        while i < b.len() {
            let c = b[i];
            if c == b'\\' {
                i += 2;
                continue;
            }
            if self.interp && c == b'#' && b.get(i + 1) == Some(&b'{') {
                i = brace_end(b, i + 1) + 1;
                continue;
            }
            if c == self.close {
                if self.depth == 0 {
                    return Some(i + 1);
                }
                self.depth -= 1;
            } else if c == self.open && self.open != self.close {
                self.depth += 1;
            }
            i += 1;
        }
        None
    }
}

struct Ruby<'a> {
    b: Build<'a>,
    quote: Option<Quote>,
    heredocs: Vec<(String, bool)>,
    start: bool,
    lambda: bool,
    value_word: bool,
}

pub(crate) fn ruby(lines: &[String]) -> Blocks {
    let mut r = Ruby {
        b: Build::new(lines),
        quote: None,
        heredocs: Vec::new(),
        start: true,
        lambda: false,
        value_word: false,
    };
    let mut heredoc: Option<(String, bool)> = None;
    let mut comment = false;
    for (n, line) in lines.iter().enumerate() {
        if let Some((id, indented)) = &heredoc {
            let t = if *indented {
                line.trim()
            } else {
                line.trim_end()
            };
            if t == id {
                heredoc = (!r.heredocs.is_empty()).then(|| r.heredocs.remove(0));
            }
            continue;
        }
        if comment {
            comment = !line.starts_with("=end");
            continue;
        }
        if line.starts_with("=begin") {
            comment = true;
            continue;
        }
        if line.trim_end() == "__END__" {
            break;
        }
        r.line(n, line);
        if r.quote.is_none() && heredoc.is_none() && !r.heredocs.is_empty() {
            heredoc = Some(r.heredocs.remove(0));
        }
    }
    r.b.blocks
}

impl Ruby<'_> {
    fn line(&mut self, n: usize, line: &str) {
        let s = line.as_bytes();
        let mut i = 0;
        if let Some(q) = self.quote.as_mut() {
            match q.scan(s, 0) {
                Some(j) => {
                    self.quote = None;
                    self.start = false;
                    i = j;
                }
                None => return,
            }
        }
        self.lambda = false;
        self.value_word = false;
        let mut last = 0;
        while i < s.len() {
            let c = s[i];
            if c == b' ' || c == b'\t' {
                i += 1;
                continue;
            }
            if c != b'#' {
                last = c;
            }
            i = self.token(n, s, i);
        }
        if let Some(o) = (self.b.stack.last_mut()).filter(|_| !b"|&\\,(+-*/<>=.".contains(&last)) {
            o.armed = false;
        }
        self.start = self.quote.is_none() && !line.trim_end().ends_with('\\') || self.start;
    }

    fn open_quote(&mut self, s: &[u8], mut q: Quote, from: usize) -> usize {
        self.start = false;
        q.scan(s, from).unwrap_or_else(|| {
            self.quote = Some(q);
            s.len()
        })
    }

    fn token(&mut self, n: usize, s: &[u8], mut i: usize) -> usize {
        let c = s[i];
        let next = s.get(i + 1).copied().unwrap_or(0);
        let was_word = std::mem::take(&mut self.value_word);
        let spaced = i > 0 && matches!(s[i - 1], b' ' | b'\t');
        let free = self.start || spaced;
        match c {
            b'#' => i = s.len(),
            b'"' | b'`' | b'\'' => i = self.open_quote(s, Quote::new(c, c != b'\''), i + 1),
            b'%' if free
                && ((b"qQwWiIrsx".contains(&next)
                    && s.get(i + 2).is_some_and(u8::is_ascii_punctuation))
                    || b"([{<|!/".contains(&next)) =>
            {
                let (d, from) = if next.is_ascii_alphabetic() {
                    (s[i + 2], i + 3)
                } else {
                    (next, i + 2)
                };
                i = self.open_quote(s, Quote::new(d, !b"qwis".contains(&next)), from);
            }
            b'/' if self.start || (was_word && spaced && !matches!(next, b' ' | b'=')) => {
                i = self.open_quote(s, Quote::new(b'/', true), i + 1);
            }
            b'?' if self.start
                && next != 0
                && next != b' '
                && !word_byte(*s.get(i + 2).unwrap_or(&0)) =>
            {
                i += 2;
                self.start = false;
            }
            b':' => i = self.colon(s, i, free),
            b'<' if next == b'<' && heredoc_id(s, i + 2).is_some() => {
                let (id, indented, j) = heredoc_id(s, i + 2).expect("the guard saw a heredoc");
                self.heredocs.push((id, indented));
                i = j;
                self.start = false;
            }
            b'\\' => i += 1,
            b'-' if next == b'>' => {
                self.lambda = true;
                i += 2;
                self.start = true;
            }
            b'{' => {
                self.b
                    .open(n, "{", std::mem::take(&mut self.lambda), false, false);
                i += 1;
                self.start = true;
            }
            b'}' => {
                self.b.close(n, &["{"]);
                i += 1;
                self.start = false;
            }
            b')' | b']' => {
                i += 1;
                self.start = false;
            }
            b'$' | b'@' => {
                i += 1;
                while i < s.len() && s[i] == b'@' {
                    i += 1;
                }
                i = word_at(s, i).max(i + 1);
                self.start = false;
            }
            b'0'..=b'9' => {
                i = word_at(s, i);
                self.start = false;
            }
            c if word_byte(c) => i = self.word(n, s, i),
            _ => {
                i += 1;
                self.start = !matches!(c, b'.');
            }
        }
        i
    }

    fn colon(&mut self, s: &[u8], mut i: usize, free: bool) -> usize {
        let next = s.get(i + 1).copied().unwrap_or(0);
        if next == b':' {
            self.start = true;
            return i + 2;
        }
        if next == b'"' {
            return self.open_quote(s, Quote::new(b'"', true), i + 2);
        }
        if word_byte(next) && free {
            i = word_at(s, i + 1);
            while i < s.len() && matches!(s[i], b'?' | b'!' | b'=') {
                i += 1;
            }
        } else if b"/+-*%<>=!&|^~[`".contains(&next) && free {
            i += 1;
            while i < s.len() && b"/+-*%<>=!&|^~[]@`".contains(&s[i]) {
                i += 1;
            }
        } else {
            self.start = true;
            return i + 1;
        }
        self.start = false;
        i
    }

    fn word(&mut self, n: usize, s: &[u8], i: usize) -> usize {
        let mut j = word_at(s, i);
        if j < s.len() && matches!(s[j], b'?' | b'!') && s.get(j + 1) != Some(&b'=') {
            j += 1;
        }
        let w = text(s, i, j);
        let dot = s[..i].iter().rev().find(|c| !matches!(c, b' ' | b'\t')) == Some(&b'.');
        let key = s.get(j) == Some(&b':') && s.get(j + 1) != Some(&b':');
        let at = self.start;
        self.start = false;
        self.value_word = true;
        if dot || key {
            self.start = key;
            return j;
        }
        self.keyword(n, s, w, at, j)
    }

    fn keyword(&mut self, n: usize, s: &[u8], w: &str, at: bool, mut i: usize) -> usize {
        match w {
            "def" => {
                let rest = text(s, i, s.len());
                let name = rest.trim_start();
                let skip = rest.len() - name.len()
                    + name
                        .find(|c: char| c.is_whitespace() || c == '(' || c == ';')
                        .unwrap_or(name.len());
                if !endless(text(s, i + skip, s.len())) {
                    self.b.open(n, "def", true, true, false);
                }
                i += skip;
                self.value_word = false;
            }
            "class" => {
                self.b.open(n, "class", true, true, false);
                let rest = text(s, i, s.len()).trim_start();
                if rest.starts_with("<<") {
                    i = s.len() - rest.len() + 2;
                }
                self.start = true;
            }
            "module" => self.b.open(n, "module", true, true, false),
            "if" | "unless" | "while" | "until" if at => {
                let w = if w == "if" || w == "unless" {
                    "if"
                } else {
                    "while"
                };
                self.b.open(n, w, true, false, w == "while");
                self.start = true;
            }
            "if" | "unless" | "while" | "until" => self.start = true,
            "case" | "begin" | "for" => {
                let w = if w == "for" { "while" } else { "case" };
                self.b.open(n, w, true, false, w == "while");
                self.start = true;
            }
            "do" => {
                match self.b.stack.last_mut() {
                    Some(o) if o.armed => o.armed = false,
                    _ => self.b.open(n, "do", true, false, false),
                }
                self.lambda = false;
                self.start = true;
            }
            "end" => {
                self.b
                    .close(n, &["def", "class", "module", "if", "while", "case", "do"]);
            }
            "else" | "elsif" | "when" | "ensure" | "rescue" | "in" if at => {
                if w != "in" || self.b.top() == Some("case") {
                    self.b.clause(n);
                }
                self.start = true;
            }
            "then" | "and" | "or" | "not" | "else" | "elsif" | "when" | "in" | "rescue" => {
                self.start = true;
            }
            _ => {}
        }
        i
    }
}

fn heredoc_id(s: &[u8], mut i: usize) -> Option<(String, bool, usize)> {
    let indented = matches!(s.get(i), Some(b'~' | b'-'));
    if indented {
        i += 1;
    }
    let (from, to, end) = match s.get(i) {
        Some(&q @ (b'\'' | b'"' | b'`')) => {
            let close = s[i + 1..].iter().position(|&c| c == q)? + i + 1;
            (i + 1, close, close + 1)
        }
        Some(c)
            if c.is_ascii_uppercase() || (indented && (c.is_ascii_alphabetic() || *c == b'_')) =>
        {
            let e = word_at(s, i);
            (i, e, e)
        }
        _ => return None,
    };
    let id = String::from_utf8_lossy(&s[from..to]).into_owned();
    Some((id, indented, end))
}

fn endless(rest: &str) -> bool {
    let mut r = rest;
    if r.starts_with('(') {
        let mut depth = 0;
        let Some(close) = r.find(|c| {
            depth += match c {
                '(' => 1,
                ')' => -1,
                _ => 0,
            };
            depth == 0
        }) else {
            return false;
        };
        let Some(after) = r.get(close + 1..) else {
            return false;
        };
        r = after;
    }
    let r = r.trim_start();
    r.starts_with('=') && !r.starts_with("==") && !r.starts_with("=~") && !r.starts_with("=>")
}

pub(crate) fn rbs(lines: &[String]) -> Blocks {
    let mut b = Build::new(lines);
    for (n, line) in lines.iter().enumerate() {
        let mut t = line.trim_start();
        while let Some(rest) = t.strip_prefix("%a") {
            let Some(open) = rest.bytes().next().filter(u8::is_ascii) else {
                break;
            };
            let close = closing(open) as char;
            let Some((_, after)) = rest.get(1..).and_then(|r| r.split_once(close)) else {
                break;
            };
            t = after.trim_start();
        }
        let Some((w, rest)) = t.split_at_checked(word_at(t.as_bytes(), 0)) else {
            continue;
        };
        if rest.starts_with(':') {
            continue;
        }
        match w {
            "class" | "module" | "interface" => {
                let name = rest.trim_start();
                let after =
                    name.trim_start_matches(|c: char| c.is_alphanumeric() || "_:".contains(c));
                if !after.trim_start().starts_with('=') {
                    b.open(n, "class", true, true, false);
                }
            }
            "end" => b.close(n, &["class"]),
            _ => {}
        }
    }
    b.blocks
}

fn lua_operand(w: &str) -> bool {
    matches!(
        w,
        "and"
            | "or"
            | "not"
            | "return"
            | "local"
            | "in"
            | "if"
            | "then"
            | "else"
            | "elseif"
            | "while"
            | "do"
            | "until"
            | "repeat"
            | "function"
    )
}

fn lua_word(b: &mut Build, n: usize, w: &str, params: &mut bool) {
    match w {
        "function" => {
            b.open(n, "function", true, true, false);
            *params = true;
        }
        "if" => b.open(n, "if", true, false, false),
        "elseif" | "else" => b.clause(n),
        "while" | "for" => b.open(n, "while", true, false, true),
        "do" => match b.stack.last_mut() {
            Some(o) if o.armed => o.armed = false,
            _ => b.open(n, "do", true, false, false),
        },
        "repeat" => b.open(n, "repeat", true, false, false),
        "until" => b.close(n, &["repeat"]),
        "end" => b.close(n, &["function", "if", "while", "do"]),
        _ => {}
    }
}

pub(crate) fn lua(lines: &[String]) -> Blocks {
    let mut b = Build::new(lines);
    let mut long: Option<usize> = None;
    let mut value = false;
    let mut params = false;
    for (n, line) in lines.iter().enumerate() {
        let s = line.as_bytes();
        let mut i = 0;
        if let Some(level) = long {
            match long_end(line, 0, level) {
                Some(j) => {
                    long = None;
                    i = j;
                }
                None => continue,
            }
        }
        while i < s.len() {
            let c = s[i];
            let next = s.get(i + 1).copied().unwrap_or(0);
            if c == b'-' && next == b'-' {
                if let Some((level, from)) = long_open(s, i + 2) {
                    match long_end(line, from, level) {
                        Some(j) => {
                            i = j;
                            continue;
                        }
                        None => long = Some(level),
                    }
                }
                break;
            }
            if c == b'['
                && let Some((level, from)) = long_open(s, i)
            {
                if let Some(j) = long_end(line, from, level) {
                    i = j;
                } else {
                    long = Some(level);
                    break;
                }
                value = true;
                continue;
            }
            match c {
                b'\'' | b'"' => {
                    i = super::skip_quoted(s, i + 1, c);
                    value = true;
                }
                b'(' => {
                    b.open(n, "(", value || params, false, false);
                    params = false;
                    value = false;
                    i += 1;
                }
                b')' => {
                    b.close(n, &["("]);
                    value = true;
                    i += 1;
                }
                b'{' => {
                    b.open(n, "{", true, false, false);
                    value = false;
                    i += 1;
                }
                b'}' => {
                    b.close(n, &["{"]);
                    value = true;
                    i += 1;
                }
                b']' => {
                    value = true;
                    i += 1;
                }
                b' ' | b'\t' => i += 1,
                c if word_byte(c) => {
                    let j = word_at(s, i);
                    let w = text(s, i, j);
                    i = j;
                    value = !lua_operand(w);
                    lua_word(&mut b, n, w, &mut params);
                }
                _ => {
                    value = false;
                    i += 1;
                }
            }
        }
    }
    b.blocks
}

fn long_open(s: &[u8], i: usize) -> Option<(usize, usize)> {
    if s.get(i) != Some(&b'[') {
        return None;
    }
    let level = s[i + 1..].iter().take_while(|&&c| c == b'=').count();
    (s.get(i + 1 + level) == Some(&b'[')).then_some((level, i + 2 + level))
}

fn long_end(line: &str, from: usize, level: usize) -> Option<usize> {
    let close = format!("]{}]", "=".repeat(level));
    line.get(from..)?
        .find(&close)
        .map(|j| from + j + close.len())
}

struct Shell<'a> {
    b: Build<'a>,
    closed: bool,
    quote: Option<(u8, bool)>,
    heredocs: Vec<(String, bool, usize)>,
    cmd: bool,
    pattern: bool,
    func: Option<usize>,
}

pub(crate) fn shell(lines: &[String], zsh: bool) -> Blocks {
    let mut sh = Shell {
        b: Build::new(lines),
        closed: false,
        quote: None,
        heredocs: Vec::new(),
        cmd: true,
        pattern: false,
        func: None,
    };
    let mut heredoc: Option<(String, bool, usize)> = None;
    for (n, line) in lines.iter().enumerate() {
        if let Some((id, tabs, head)) = &heredoc {
            let t = if *tabs {
                line.trim_start_matches('\t')
            } else {
                line.as_str()
            };
            if t == id {
                sh.b.span(*head, n);
                heredoc = (!sh.heredocs.is_empty()).then(|| sh.heredocs.remove(0));
            }
            continue;
        }
        sh.line(n, line, zsh);
        if sh.quote.is_none() && heredoc.is_none() && !sh.heredocs.is_empty() {
            heredoc = Some(sh.heredocs.remove(0));
        }
    }
    sh.b.blocks
}

impl Shell<'_> {
    fn line(&mut self, n: usize, line: &str, zsh: bool) {
        let s = line.as_bytes();
        let mut i = 0;
        if let Some((q, escapes)) = self.quote {
            match shell_quote(s, 0, q, escapes) {
                Some(j) => {
                    self.quote = None;
                    i = j;
                }
                None => return,
            }
        }
        while i < s.len() {
            i = self.token(n, s, i, zsh);
        }
        if !line.ends_with('\\') {
            self.cmd = true;
        }
    }

    fn token(&mut self, n: usize, s: &[u8], mut i: usize, zsh: bool) -> usize {
        let c = s[i];
        let next = s.get(i + 1).copied().unwrap_or(0);
        let boundary = i == 0 || matches!(s[i - 1], b' ' | b'\t' | b';' | b'(' | b'&' | b'|');
        match c {
            b' ' | b'\t' => i += 1,
            b'#' if boundary => i = s.len(),
            b'\'' | b'"' | b'`' => {
                let escapes = c != b'\'' || (i > 0 && s[i - 1] == b'$');
                if let Some(j) = shell_quote(s, i + 1, c, escapes) {
                    i = j;
                } else {
                    self.quote = Some((c, escapes));
                    i = s.len();
                }
                self.cmd = false;
            }
            b'\\' => i += 2,
            b')' if self.pattern => {
                self.pattern = false;
                self.cmd = true;
                i += 1;
            }
            _ if self.pattern => {
                let j = shell_word(s, i).max(i + 1);
                if text(s, i, j) == "esac" {
                    self.pattern = false;
                    self.b.close(n, &["case"]);
                }
                i = j;
            }
            b';' => {
                if next == b';' || next == b'&' {
                    self.pattern = self.b.top() == Some("case");
                    i += 1;
                }
                self.cmd = true;
                i += 1;
            }
            b'&' | b'|' => {
                self.cmd = !(next == b'>' || (i > 0 && matches!(s[i - 1], b'>' | b'<')));
                i += 1;
            }
            b'(' if zsh && self.cmd && next == b')' => {
                self.func = Some(n);
                i += 2;
            }
            b'(' => {
                self.cmd = true;
                i += 1;
            }
            b')' => {
                self.cmd = false;
                i += 1;
            }
            b'<' if next == b'<' && s.get(i + 2) == Some(&b'<') => i += 3,
            b'<' if next == b'<' => i = self.heredoc(n, s, i),
            b'$' if next == b'{' => {
                i = brace_end(s, i) + 1;
                self.cmd = false;
            }
            b'$' if next == b'(' => {
                self.cmd = true;
                i += 2;
            }
            _ => i = self.word(n, s, i, zsh),
        }
        i
    }

    fn heredoc(&mut self, n: usize, s: &[u8], i: usize) -> usize {
        let tabs = s.get(i + 2) == Some(&b'-');
        let mut j = i + 2 + usize::from(tabs);
        while j < s.len() && s[j] == b' ' {
            j += 1;
        }
        let k = match s.get(j) {
            Some(&q @ (b'\'' | b'"')) => shell_quote(s, j + 1, q, false).unwrap_or(s.len()),
            _ => shell_word(s, j),
        };
        let id: String = text(s, j, k)
            .chars()
            .filter(|c| !matches!(c, '\'' | '"' | '\\'))
            .collect();
        if !id.is_empty() {
            self.heredocs.push((id, tabs, n));
        }
        k.max(j)
    }

    fn word(&mut self, n: usize, s: &[u8], i: usize, zsh: bool) -> usize {
        let j = shell_word(s, i).max(i + 1);
        let w = text(s, i, j);
        let was = self.cmd;
        self.cmd = false;
        let after_brace = std::mem::replace(&mut self.closed, w == "}");
        if zsh && w == "always" && after_brace {
            self.cmd = true;
            return j;
        }
        if w == "in"
            && let Some(o) = (self.b.stack.last_mut()).filter(|o| o.word == "case" && o.armed)
        {
            o.armed = false;
            self.pattern = true;
            return j;
        }
        if !(was || w == "{" && self.func.is_some()) {
            return j;
        }
        self.keyword(n, s, w, j)
    }

    fn keyword(&mut self, n: usize, s: &[u8], w: &str, mut i: usize) -> usize {
        match w {
            "if" | "while" | "until" => {
                self.b
                    .open(n, if w == "if" { "if" } else { "loop" }, true, false, false);
                self.cmd = true;
            }
            "for" | "select" => self.b.open(n, "loop", true, false, false),
            "case" => self.b.open(n, "case", true, false, true),
            "then" | "do" | "!" | "time" => self.cmd = true,
            "elif" | "else" => {
                self.b.clause(n);
                self.cmd = true;
            }
            "fi" => self.b.close(n, &["if"]),
            "done" => self.b.close(n, &["loop"]),
            "esac" => self.b.close(n, &["case"]),
            "function" => self.func = Some(n),
            "{" => {
                match self.func.take() {
                    Some(h) => {
                        self.b.open(h, "{", true, true, false);
                    }
                    None => self.b.open(n, "{", false, false, false),
                }
                self.cmd = true;
            }
            "}" => self.b.close(n, &["{"]),
            _ => {
                let rest = text(s, i, s.len()).trim_start();
                if rest.starts_with("()") {
                    self.func = Some(n);
                    i = s.len() - rest.len() + 2;
                    self.cmd = true;
                } else if self.func.is_some() {
                    self.cmd = true;
                }
            }
        }
        i
    }
}

fn shell_word(s: &[u8], mut i: usize) -> usize {
    while i < s.len()
        && !matches!(
            s[i],
            b' ' | b'\t' | b';' | b'&' | b'|' | b'(' | b')' | b'<' | b'>' | b'"' | b'\'' | b'`'
        )
    {
        i += if s[i] == b'\\' { 2 } else { 1 };
    }
    i.min(s.len())
}

fn shell_quote(s: &[u8], mut i: usize, q: u8, escapes: bool) -> Option<usize> {
    while i < s.len() {
        match s[i] {
            b'\\' if escapes => i += 2,
            c if c == q => return Some(i + 1),
            b'`' if q == b'"' => i = shell_quote(s, i + 1, b'`', true)?,
            b'$' if q == b'"' && s.get(i + 1) == Some(&b'(') => i = shell_sub(s, i + 2)?,
            _ => i += 1,
        }
    }
    None
}

fn shell_sub(s: &[u8], mut i: usize) -> Option<usize> {
    let mut depth = 1;
    while i < s.len() {
        match s[i] {
            b'\\' => i += 2,
            b'\'' => i = shell_quote(s, i + 1, b'\'', false)?,
            q @ (b'"' | b'`') => i = shell_quote(s, i + 1, q, true)?,
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => i += 1,
        }
    }
    None
}
