struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    last: usize,
    depth: usize,
    nodes: Vec<(usize, usize)>,
}

fn ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn ident_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b'\'' | b'-')
}

fn uri_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || b"%/?:@&=+$,-_.!~*'".contains(&c)
}

impl<'a> Parser<'a> {
    fn peek(&self) -> u8 {
        self.s.get(self.i).copied().unwrap_or(0)
    }

    fn at(&self, lit: &[u8]) -> bool {
        self.s[self.i..].starts_with(lit)
    }

    fn take(&mut self, n: usize) {
        self.i = (self.i + n).min(self.s.len());
        self.last = self.i;
    }

    fn ws(&mut self) {
        while self.i < self.s.len() {
            if self.peek().is_ascii_whitespace() {
                self.i += 1;
            } else if self.peek() == b'#' {
                self.i += self.s[self.i..].iter().take_while(|&&c| c != b'\n').count();
            } else if self.at(b"/*") {
                self.i = (self.s[self.i + 2..].windows(2).position(|w| w == b"*/"))
                    .map_or(self.s.len(), |p| self.i + p + 4);
            } else {
                return;
            }
        }
    }

    fn word(&self) -> &'a [u8] {
        let rest = &self.s[self.i..];
        if !rest.first().is_some_and(|&c| ident_start(c)) {
            return &[];
        }
        let n = rest.iter().take_while(|&&c| ident_char(c)).count();
        &rest[..n]
    }

    fn next_after(&mut self, n: usize) -> u8 {
        let i = self.i;
        self.i += n;
        self.ws();
        let c = self.peek();
        self.i = i;
        c
    }

    fn eat(&mut self, c: u8) {
        self.ws();
        if self.peek() == c {
            self.take(1);
        }
    }

    fn eat_word(&mut self, w: &[u8]) {
        self.ws();
        if self.word() == w {
            self.take(w.len());
        }
    }

    fn node(&mut self, start: usize) {
        self.nodes.push((start, self.last));
    }

    fn expr(&mut self) {
        self.ws();
        let start = self.i;
        if self.depth > 256 {
            self.i = self.s.len();
            return;
        }
        self.depth += 1;
        match self.word() {
            b"let" if self.next_after(3) != b'{' => {
                self.take(3);
                self.bindings(true);
                self.eat_word(b"in");
                self.expr();
                self.node(start);
            }
            b"with" | b"assert" => {
                let with = self.word() == b"with";
                self.take(self.word().len());
                self.expr();
                self.eat(b';');
                self.expr();
                if with {
                    self.node(start);
                }
            }
            b"if" => {
                self.take(2);
                self.expr();
                self.eat_word(b"then");
                self.expr();
                self.eat_word(b"else");
                self.expr();
                self.node(start);
            }
            _ if self.lambda_head() => {
                self.expr();
                self.node(start);
            }
            _ => self.operators(),
        }
        self.depth -= 1;
    }

    fn lambda_head(&mut self) -> bool {
        let w = self.word().len();
        if w > 0 && !matches!(self.word(), b"rec" | b"or") {
            let after = self.next_after(w);
            let colon = after == b':' && {
                let i = self.i;
                self.i += w;
                self.ws();
                let uri = self.s.get(self.i + 1).is_some_and(|&c| uri_char(c));
                self.i = i;
                !uri
            };
            if colon {
                self.take(w);
                self.eat(b':');
                return true;
            }
            if after == b'@' {
                self.take(w);
                self.eat(b'@');
                self.ws();
                self.formals();
                self.eat(b':');
                return true;
            }
            return false;
        }
        if self.peek() != b'{' || !self.formals_ahead() {
            return false;
        }
        self.formals();
        self.ws();
        if self.peek() == b'@' {
            self.take(1);
            self.ws();
            let w = self.word().len();
            self.take(w);
        }
        self.eat(b':');
        true
    }

    fn formals_ahead(&mut self) -> bool {
        let i = self.i;
        self.i += 1;
        self.ws();
        let ahead = match self.peek() {
            b'}' => matches!(self.next_after(1), b':' | b'@'),
            b'.' => self.at(b"..."),
            _ if self.word().is_empty() || self.word() == b"inherit" => false,
            _ => matches!(self.next_after(self.word().len()), b',' | b'?' | b'}'),
        };
        self.i = i;
        ahead
    }

    fn formals(&mut self) {
        self.take(1);
        loop {
            self.ws();
            let before = self.i;
            match self.peek() {
                0 | b'}' => break,
                b'?' => {
                    self.take(1);
                    self.expr();
                }
                b'.' | b',' => self.take(1),
                _ => self.take(self.word().len()),
            }
            if self.i == before {
                break;
            }
        }
        self.eat(b'}');
    }

    fn operators(&mut self) {
        loop {
            self.ws();
            match self.peek() {
                0 | b';' | b')' | b']' | b'}' | b',' => return,
                _ => {}
            }
            match self.word() {
                b"then" | b"else" | b"in" => return,
                b"if" | b"with" | b"assert" => return self.expr(),
                b"let" if self.next_after(3) != b'{' => return self.expr(),
                _ => {}
            }
            if !self.operand() {
                self.take(1);
            }
        }
    }

    fn operand(&mut self) -> bool {
        let start = self.i;
        match self.peek() {
            b'{' => {
                self.attrset();
                self.node(start);
            }
            b'[' => self.list(start),
            b'(' => {
                self.take(1);
                self.expr();
                self.eat(b')');
            }
            b'"' => self.string(),
            b'\'' if self.at(b"''") => {
                self.indented();
                self.node(start);
            }
            b'$' if self.at(b"${") => self.interpolation(),
            _ => return self.word_operand(start),
        }
        true
    }

    fn word_operand(&mut self, start: usize) -> bool {
        let w = self.word().len();
        if w == 0 {
            return false;
        }
        if matches!(self.word(), b"rec" | b"let") && self.next_after(w) == b'{' {
            let rec = self.word() == b"rec";
            self.take(w);
            self.ws();
            self.attrset();
            if rec {
                self.node(start);
            }
            return true;
        }
        self.take(w);
        if self.peek() == b':' && self.s.get(self.i + 1).is_some_and(|&c| uri_char(c)) {
            let n = self.s[self.i..]
                .iter()
                .take_while(|&&c| uri_char(c))
                .count();
            self.take(n);
        }
        true
    }

    fn attrset(&mut self) {
        self.take(1);
        self.bindings(false);
        self.eat(b'}');
    }

    fn list(&mut self, start: usize) {
        self.take(1);
        loop {
            self.ws();
            match self.peek() {
                b']' => {
                    self.take(1);
                    break;
                }
                0 | b')' | b'}' | b';' => break,
                _ => {}
            }
            if !self.operand() {
                self.take(1);
            }
        }
        self.node(start);
    }

    fn interpolation(&mut self) {
        self.take(2);
        self.expr();
        self.eat(b'}');
    }

    fn string(&mut self) {
        self.take(1);
        while self.i < self.s.len() {
            match self.peek() {
                b'\\' => self.take(2),
                b'"' => return self.take(1),
                b'$' if self.at(b"$$") => self.take(2),
                b'$' if self.at(b"${") => self.interpolation(),
                _ => self.take(1),
            }
        }
    }

    fn indented(&mut self) {
        self.take(2);
        while self.i < self.s.len() {
            if self.at(b"'''") || self.at(b"''$") {
                self.take(3);
            } else if self.at(b"''\\") {
                self.take(4);
            } else if self.at(b"''") {
                return self.take(2);
            } else if self.at(b"$$") {
                self.take(2);
            } else if self.at(b"${") {
                self.interpolation();
            } else {
                self.take(1);
            }
        }
    }

    fn bindings(&mut self, in_let: bool) {
        loop {
            self.ws();
            let before = self.i;
            if self.peek() == 0 || self.peek() == b'}' || in_let && self.word() == b"in" {
                return;
            }
            if self.word() == b"inherit" {
                self.inherit();
            } else {
                self.attrpath();
                self.expr();
                self.eat(b';');
            }
            if self.i == before {
                self.take(1);
            }
        }
    }

    fn inherit(&mut self) {
        self.take(7);
        self.ws();
        if self.peek() == b'(' {
            self.take(1);
            self.expr();
            self.eat(b')');
        }
        loop {
            self.ws();
            match self.peek() {
                b';' => return self.take(1),
                0 | b'}' => return,
                b'"' => self.string(),
                _ if self.word().is_empty() => self.take(1),
                _ => self.take(self.word().len()),
            }
        }
    }

    fn attrpath(&mut self) {
        loop {
            self.ws();
            match self.peek() {
                b'=' => return self.take(1),
                0 | b';' | b'}' => return,
                b'"' => self.string(),
                b'$' if self.at(b"${") => self.interpolation(),
                _ if self.word().is_empty() => self.take(1),
                _ => self.take(self.word().len()),
            }
        }
    }
}

pub(super) fn nix(lines: &[String]) -> Vec<(usize, usize)> {
    let text = lines.join("\n");
    let mut p = Parser {
        s: text.as_bytes(),
        i: 0,
        last: 0,
        depth: 0,
        nodes: Vec::new(),
    };
    while p.i < p.s.len() {
        let before = p.i;
        p.expr();
        if p.i == before {
            p.take(1);
        }
    }
    let starts: Vec<usize> = (lines.iter())
        .scan(0, |at, l| {
            let s = *at;
            *at += l.len() + 1;
            Some(s)
        })
        .collect();
    let row = |b: usize| starts.partition_point(|&s| s <= b).saturating_sub(1);
    let mut nodes = p.nodes;
    nodes.sort_unstable_by_key(|&(s, e)| (s, std::cmp::Reverse(e)));
    let mut rows: Vec<(usize, usize)> = (nodes.iter())
        .map(|&(s, e)| (row(s), row(e.saturating_sub(1).max(s))))
        .filter(|(s, e)| e > s)
        .collect();
    rows.dedup();
    vim_folds(&rows, lines.len())
}

fn vim_folds(nodes: &[(usize, usize)], len: usize) -> Vec<(usize, usize)> {
    let (mut enter, mut leave) = (vec![0usize; len + 1], vec![0usize; len + 1]);
    for &(s, e) in nodes {
        enter[s] += 1;
        leave[e] += 1;
    }
    let mut levels = Vec::with_capacity(len);
    let (mut prev, mut leave_prev) = (0usize, 0usize);
    for n in 0..len {
        let mut leave_line = leave[n];
        let mut level = (prev + enter[n]).saturating_sub(leave_prev);
        if enter[n] > 0 && leave_line > 0 {
            level = level.saturating_sub(leave_line);
            leave_line = 0;
        }
        levels.push((level, enter[n] > 0));
        leave_prev = leave_line;
        prev = level;
    }
    let mut folds = Vec::new();
    for (h, &(level, start)) in levels.iter().enumerate() {
        if !start || level == 0 {
            continue;
        }
        let end = (h + 1..len)
            .find(|&m| levels[m].0 < level || levels[m].1 && levels[m].0 <= level)
            .unwrap_or(len);
        if end - 1 > h {
            folds.push((h, end - 1));
        }
    }
    folds
}
