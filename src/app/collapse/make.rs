use super::words::Blocks;

enum Frame {
    Cond(usize),
    Rule(usize, usize),
}

fn starts_word(t: &str, w: &str) -> bool {
    t.strip_prefix(w)
        .is_some_and(|r| r.is_empty() || r.starts_with([' ', '\t', '(', '#']))
}

fn continued(line: &str) -> bool {
    line.bytes().rev().take_while(|&b| b == b'\\').count() % 2 == 1
}

fn logical_end(lines: &[String], mut n: usize) -> usize {
    while n + 1 < lines.len() && continued(&lines[n]) {
        n += 1;
    }
    n
}

const DIRECTIVES: &[&str] = &[
    "include", "-include", "sinclude", "vpath", "export", "unexport", "override", "undefine",
    "private",
];

fn rule(head: &str) -> bool {
    if DIRECTIVES.iter().any(|w| starts_word(head, w)) {
        return false;
    }
    let b = head.as_bytes();
    let mut depth = 0usize;
    let mut colon = false;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            b'(' | b'{' => depth += 1,
            b')' | b'}' => depth = depth.saturating_sub(1),
            _ if depth > 0 => {}
            b'#' | b';' => return colon,
            b'=' => return false,
            b':' if !colon => {
                let rest = &b[i + 1..];
                let colons = rest.iter().take_while(|&&c| c == b':').count();
                if rest.get(colons) == Some(&b'=') {
                    return false;
                }
                colon = true;
                i += colons;
            }
            _ => {}
        }
        i += 1;
    }
    colon
}

fn define(t: &str) -> bool {
    let t = ["override ", "export "]
        .iter()
        .find_map(|p| t.strip_prefix(p))
        .map_or(t, str::trim_start);
    starts_word(t, "define")
}

struct Make {
    nodes: Vec<(usize, usize)>,
    funcs: Vec<(usize, usize)>,
    stack: Vec<Frame>,
}

impl Make {
    fn close_rule(&mut self) {
        if let Some(&Frame::Rule(h, e)) = self.stack.last() {
            self.stack.pop();
            self.nodes.push((h, e));
            self.funcs.push((h, e));
        }
    }

    fn reach(&mut self, n: usize) {
        if let Some(Frame::Rule(_, e)) = self.stack.last_mut() {
            *e = n;
        }
    }

    fn thing(&mut self, lines: &[String], n: usize, m: usize) -> usize {
        let t = lines[n].trim();
        if ["ifeq", "ifneq", "ifdef", "ifndef"]
            .iter()
            .any(|w| starts_word(t, w))
        {
            self.stack.push(Frame::Cond(n));
            return m;
        }
        self.close_rule();
        if starts_word(t, "endif") {
            if let Some(&Frame::Cond(h)) = self.stack.last() {
                self.stack.pop();
                self.nodes.push((h, n));
                self.reach(n);
            }
        } else if define(t) {
            let end = (n + 1..lines.len()).find(|&i| lines[i].trim_start().starts_with("endef"));
            let Some(e) = end else {
                return lines.len();
            };
            self.nodes.push((n, e));
            self.funcs.push((n, e));
            return e;
        } else if !starts_word(t, "else") {
            let head: Vec<&str> = (lines[n..=m].iter())
                .map(|l| l.trim().trim_end_matches('\\'))
                .collect();
            if rule(&head.join(" ")) {
                self.stack.push(Frame::Rule(n, m));
            }
        }
        m
    }
}

pub(crate) fn make(lines: &[String]) -> Blocks {
    let mut b = Make {
        nodes: Vec::new(),
        funcs: Vec::new(),
        stack: Vec::new(),
    };
    let mut n = 0;
    while n < lines.len() {
        let line = &lines[n];
        let t = line.trim();
        let m = logical_end(lines, n);
        let mut indent = line.bytes().take_while(|&c| c == b' ' || c == b'\t');
        if indent.any(|c| c == b'\t') && !b.stack.is_empty() {
            b.reach(m);
            n = m + 1;
            continue;
        }
        if t.is_empty() || t.starts_with('#') {
            n += 1;
            continue;
        }
        n = b.thing(lines, n, m) + 1;
    }
    b.close_rule();
    Blocks::from_nodes(b.nodes, b.funcs, lines.len())
}

const OPENERS: &[&str] = &["if", "foreach", "while", "function", "macro", "block"];

fn bracket(b: &[u8], i: usize) -> Option<usize> {
    let eq = (b.get(i + 1..)?.iter()).take_while(|&&c| c == b'=').count();
    (b.get(i) == Some(&b'[') && b.get(i + 1 + eq) == Some(&b'[')).then_some(eq)
}

struct Scan<'a> {
    b: &'a [u8],
    i: usize,
    line: usize,
}

impl Scan<'_> {
    fn step(&mut self) {
        if let Some(&c) = self.b.get(self.i) {
            self.line += usize::from(c == b'\n');
            self.i += 1;
        }
    }

    fn skip_to(&mut self, end: impl Fn(&[u8]) -> bool) {
        while self.i < self.b.len() && !end(&self.b[self.i..]) {
            self.step();
        }
    }

    fn comment(&mut self) {
        match bracket(self.b, self.i + 1) {
            Some(eq) => self.close_bracket(eq),
            None => self.skip_to(|r| r[0] == b'\n'),
        }
    }

    fn close_bracket(&mut self, eq: usize) {
        let close: Vec<u8> = (std::iter::once(b']'))
            .chain(std::iter::repeat_n(b'=', eq))
            .chain(std::iter::once(b']'))
            .collect();
        self.skip_to(|r| r.starts_with(&close));
        for _ in 0..close.len() {
            self.step();
        }
    }

    fn arguments(&mut self) -> usize {
        let mut depth = 0usize;
        while self.i < self.b.len() {
            let c = self.b[self.i];
            let start = self.i == 0 || matches!(self.b[self.i - 1], b' ' | b'\t' | b'\n' | b'(');
            match c {
                b'"' => {
                    self.step();
                    while self.i < self.b.len() && self.b[self.i] != b'"' {
                        if self.b[self.i] == b'\\' {
                            self.step();
                        }
                        self.step();
                    }
                }
                b'[' if start && bracket(self.b, self.i).is_some() => {
                    let eq = bracket(self.b, self.i).unwrap_or(0);
                    self.close_bracket(eq);
                    continue;
                }
                b'#' => {
                    self.comment();
                    continue;
                }
                b'\\' => self.step(),
                b'(' => depth += 1,
                b')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let line = self.line;
                        self.step();
                        return line;
                    }
                }
                _ => {}
            }
            self.step();
        }
        self.line
    }
}

pub(crate) fn cmake(lines: &[String]) -> Blocks {
    let text = lines.join("\n");
    let mut s = Scan {
        b: text.as_bytes(),
        i: 0,
        line: 0,
    };
    let (mut nodes, mut funcs, mut stack) = (Vec::new(), Vec::new(), Vec::<(usize, usize)>::new());
    while s.i < s.b.len() {
        let c = s.b[s.i];
        if c == b'#' {
            s.comment();
            continue;
        }
        if !(c.is_ascii_alphabetic() || c == b'_') {
            s.step();
            continue;
        }
        let (at, line) = (s.i, s.line);
        while s.i < s.b.len() && (s.b[s.i].is_ascii_alphanumeric() || s.b[s.i] == b'_') {
            s.step();
        }
        let name = String::from_utf8_lossy(&s.b[at..s.i]).to_ascii_lowercase();
        while s.i < s.b.len() && matches!(s.b[s.i], b' ' | b'\t') {
            s.step();
        }
        if s.b.get(s.i) != Some(&b'(') {
            continue;
        }
        let end = s.arguments();
        if let Some(k) = OPENERS.iter().position(|&w| w == name) {
            stack.push((k, line));
        } else if let Some(k) =
            (name.strip_prefix("end")).and_then(|w| OPENERS.iter().position(|&o| o == w))
            && let Some(at) = stack.iter().rposition(|&(o, _)| o == k)
        {
            stack.truncate(at + 1);
            if let Some((_, h)) = stack.pop() {
                nodes.push((h, end));
                if matches!(OPENERS[k], "function" | "macro") {
                    funcs.push((h, end));
                }
            }
        }
    }
    Blocks::from_nodes(nodes, funcs, lines.len())
}
