use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct Blocks {
    spans: HashMap<usize, usize>,
    funcs: Vec<(usize, usize)>,
}

impl Blocks {
    pub(crate) fn region(&self, h: usize) -> Option<usize> {
        self.spans.get(&h).copied()
    }

    pub(crate) fn target(&self, l: usize) -> Option<(usize, usize)> {
        if let Some(e) = self.region(l) {
            return Some((l, e));
        }
        (self.funcs.iter().copied())
            .filter(|&(h, e)| h < l && l <= e)
            .max_by_key(|&(h, _)| h)
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
        let o = self.stack.pop().unwrap();
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

fn word_byte(c: u8) -> bool {
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
        Quote {
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
                let mut depth = 0;
                i += 1;
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
                i += 1;
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

pub(crate) fn ruby(lines: &[String]) -> Blocks {
    let mut b = Build::new(lines);
    let mut quote: Option<Quote> = None;
    let mut heredocs: Vec<(String, bool)> = Vec::new();
    let mut heredoc: Option<(String, bool)> = None;
    let mut comment = false;
    let mut start = true;
    for (n, line) in lines.iter().enumerate() {
        if let Some((id, indented)) = &heredoc {
            let t = if *indented {
                line.trim()
            } else {
                line.trim_end()
            };
            if t == id {
                heredoc = (!heredocs.is_empty()).then(|| heredocs.remove(0));
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
        let s = line.as_bytes();
        let mut i = 0;
        if let Some(q) = quote.as_mut() {
            match q.scan(s, 0) {
                Some(j) => {
                    quote = None;
                    start = false;
                    i = j;
                }
                None => continue,
            }
        }
        let mut lambda = false;
        let mut value_word = false;
        let mut last = 0;
        while i < s.len() {
            let c = s[i];
            let next = s.get(i + 1).copied().unwrap_or(0);
            if c == b' ' || c == b'\t' {
                i += 1;
                continue;
            }
            if c != b'#' {
                last = c;
            }
            let was_word = std::mem::take(&mut value_word);
            let spaced = i > 0 && matches!(s[i - 1], b' ' | b'\t');
            let mut open_quote = |mut q: Quote, from: usize, i: &mut usize| match q.scan(s, from) {
                Some(j) => *i = j,
                None => {
                    quote = Some(q);
                    *i = s.len();
                }
            };
            match c {
                b'#' => break,
                b'"' | b'`' | b'\'' => {
                    open_quote(Quote::new(c, c != b'\''), i + 1, &mut i);
                    start = false;
                }
                b'%' if (start || spaced)
                    && ((b"qQwWiIrsx".contains(&next)
                        && s.get(i + 2).is_some_and(|d| d.is_ascii_punctuation()))
                        || b"([{<|!/".contains(&next)) =>
                {
                    let (d, from) = match next.is_ascii_alphabetic() {
                        true => (s[i + 2], i + 3),
                        false => (next, i + 2),
                    };
                    open_quote(Quote::new(d, !b"qwis".contains(&next)), from, &mut i);
                    start = false;
                }
                b'/' if start || (was_word && spaced && !matches!(next, b' ' | b'=')) => {
                    open_quote(Quote::new(b'/', true), i + 1, &mut i);
                    start = false;
                }
                b'?' if start
                    && next != 0
                    && next != b' '
                    && !word_byte(*s.get(i + 2).unwrap_or(&0)) =>
                {
                    i += 2;
                    start = false;
                }
                b':' if next == b':' => {
                    i += 2;
                    start = true;
                }
                b':' if next == b'"' => {
                    open_quote(Quote::new(b'"', true), i + 2, &mut i);
                    start = false;
                }
                b':' if word_byte(next) && (start || spaced) => {
                    i = word_at(s, i + 1);
                    while i < s.len() && matches!(s[i], b'?' | b'!' | b'=') {
                        i += 1;
                    }
                    start = false;
                }
                b'<' if next == b'<' && heredoc_id(s, i + 2).is_some() => {
                    let (id, indented, j) = heredoc_id(s, i + 2).unwrap();
                    heredocs.push((id, indented));
                    i = j;
                    start = false;
                }
                b':' if b"/+-*%<>=!&|^~[`".contains(&next) && (start || spaced) => {
                    i += 1;
                    while i < s.len() && b"/+-*%<>=!&|^~[]@`".contains(&s[i]) {
                        i += 1;
                    }
                    start = false;
                }
                b'\\' => i += 1,
                b'-' if next == b'>' => {
                    lambda = true;
                    i += 2;
                    start = true;
                }
                b'{' => {
                    b.open(n, "{", std::mem::take(&mut lambda), false, false);
                    i += 1;
                    start = true;
                }
                b'}' => {
                    b.close(n, &["{"]);
                    i += 1;
                    start = false;
                }
                b')' | b']' => {
                    i += 1;
                    start = false;
                }
                b'$' | b'@' => {
                    i += 1;
                    while i < s.len() && s[i] == b'@' {
                        i += 1;
                    }
                    i = word_at(s, i).max(i + 1);
                    start = false;
                }
                b'0'..=b'9' => {
                    i = word_at(s, i);
                    start = false;
                }
                c if word_byte(c) => {
                    let mut j = word_at(s, i);
                    if j < s.len() && matches!(s[j], b'?' | b'!') && s.get(j + 1) != Some(&b'=') {
                        j += 1;
                    }
                    let w = text(s, i, j);
                    let dot =
                        s[..i].iter().rev().find(|c| !matches!(c, b' ' | b'\t')) == Some(&b'.');
                    let key = s.get(j) == Some(&b':') && s.get(j + 1) != Some(&b':');
                    let at = start;
                    i = j;
                    start = false;
                    value_word = true;
                    if dot || key {
                        start = key;
                        continue;
                    }
                    match w {
                        "def" => {
                            let rest = &line[i..];
                            let name = rest.trim_start();
                            let skip = rest.len() - name.len()
                                + name
                                    .find(|c: char| c.is_whitespace() || c == '(' || c == ';')
                                    .unwrap_or(name.len());
                            if !endless(&rest[skip..]) {
                                b.open(n, "def", true, true, false);
                            }
                            i += skip;
                            value_word = false;
                        }
                        "class" => {
                            b.open(n, "class", true, true, false);
                            let rest = line[i..].trim_start();
                            if rest.starts_with("<<") {
                                i = s.len() - rest.len() + 2;
                            }
                            start = true;
                        }
                        "module" => b.open(n, "module", true, true, false),
                        "if" | "unless" | "while" | "until" if at => {
                            let w = if w == "if" || w == "unless" {
                                "if"
                            } else {
                                "while"
                            };
                            b.open(n, w, true, false, w == "while");
                            start = true;
                        }
                        "if" | "unless" | "while" | "until" => start = true,
                        "case" | "begin" | "for" => {
                            let w = if w == "for" { "while" } else { "case" };
                            b.open(n, w, true, false, w == "while");
                            start = true;
                        }
                        "do" => {
                            match b.stack.last_mut() {
                                Some(o) if o.armed => o.armed = false,
                                _ => b.open(n, "do", true, false, false),
                            }
                            lambda = false;
                            start = true;
                        }
                        "end" => {
                            b.close(n, &["def", "class", "module", "if", "while", "case", "do"])
                        }
                        "else" | "elsif" | "when" | "ensure" | "rescue" | "in" if at => {
                            if w != "in" || b.top() == Some("case") {
                                b.clause(n);
                            }
                            start = true;
                        }
                        "then" | "and" | "or" | "not" | "else" | "elsif" | "when" | "in"
                        | "rescue" => start = true,
                        _ => {}
                    }
                }
                _ => {
                    i += 1;
                    start = !matches!(c, b'.');
                }
            }
        }
        if quote.is_none() && heredoc.is_none() && !heredocs.is_empty() {
            heredoc = Some(heredocs.remove(0));
        }
        if let Some(o) = b
            .stack
            .last_mut()
            .filter(|_| !b"|&\\,(+-*/<>=.".contains(&last))
        {
            o.armed = false;
        }
        start = quote.is_none() && !line.trim_end().ends_with('\\') || start;
    }
    b.blocks
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
        r = &r[close + 1..];
    }
    let r = r.trim_start();
    r.starts_with('=') && !r.starts_with("==") && !r.starts_with("=~") && !r.starts_with("=>")
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
                match long_end(line, from, level) {
                    Some(j) => i = j,
                    None => {
                        long = Some(level);
                        break;
                    }
                }
                value = true;
                continue;
            }
            match c {
                b'\'' | b'"' => {
                    i += 1;
                    while i < s.len() && s[i] != c {
                        i += if s[i] == b'\\' { 2 } else { 1 };
                    }
                    i += 1;
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
                    value = !matches!(
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
                    );
                    match w {
                        "function" => {
                            b.open(n, "function", true, true, false);
                            params = true;
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

pub(crate) fn shell(lines: &[String]) -> Blocks {
    let mut b = Build::new(lines);
    let mut quote: Option<(u8, bool)> = None;
    let mut heredocs: Vec<(String, bool, usize)> = Vec::new();
    let mut heredoc: Option<(String, bool, usize)> = None;
    let mut cmd = true;
    let mut pattern = false;
    let mut func: Option<usize> = None;
    for (n, line) in lines.iter().enumerate() {
        if let Some((id, tabs, head)) = &heredoc {
            let t = if *tabs {
                line.trim_start_matches('\t')
            } else {
                line.as_str()
            };
            if t == id {
                b.span(*head, n);
                heredoc = (!heredocs.is_empty()).then(|| heredocs.remove(0));
            }
            continue;
        }
        let s = line.as_bytes();
        let mut i = 0;
        if let Some((q, escapes)) = quote {
            match shell_quote(s, 0, q, escapes) {
                Some(j) => {
                    quote = None;
                    i = j;
                }
                None => continue,
            }
        }
        while i < s.len() {
            let c = s[i];
            let next = s.get(i + 1).copied().unwrap_or(0);
            let boundary = i == 0 || matches!(s[i - 1], b' ' | b'\t' | b';' | b'(' | b'&' | b'|');
            match c {
                b' ' | b'\t' => i += 1,
                b'#' if boundary => break,
                b'\'' | b'"' | b'`' => {
                    let escapes = c != b'\'' || (i > 0 && s[i - 1] == b'$');
                    match shell_quote(s, i + 1, c, escapes) {
                        Some(j) => i = j,
                        None => {
                            quote = Some((c, escapes));
                            i = s.len();
                        }
                    }
                    cmd = false;
                }
                b'\\' => i += 2,
                b')' if pattern => {
                    pattern = false;
                    cmd = true;
                    i += 1;
                }
                _ if pattern => {
                    let j = shell_word(s, i).max(i + 1);
                    if text(s, i, j) == "esac" {
                        pattern = false;
                        b.close(n, &["case"]);
                    }
                    i = j;
                }
                b';' => {
                    if next == b';' || next == b'&' {
                        pattern = b.top() == Some("case");
                        i += 1;
                    }
                    cmd = true;
                    i += 1;
                }
                b'&' | b'|' => {
                    cmd = !(next == b'>' || (i > 0 && matches!(s[i - 1], b'>' | b'<')));
                    i += 1;
                }
                b'(' => {
                    cmd = true;
                    i += 1;
                }
                b')' => {
                    cmd = false;
                    i += 1;
                }
                b'<' if next == b'<' && s.get(i + 2) == Some(&b'<') => i += 3,
                b'<' if next == b'<' => {
                    let tabs = s.get(i + 2) == Some(&b'-');
                    let mut j = i + 2 + usize::from(tabs);
                    while j < s.len() && s[j] == b' ' {
                        j += 1;
                    }
                    let k = match s.get(j) {
                        Some(&q @ (b'\'' | b'"')) => {
                            shell_quote(s, j + 1, q, false).unwrap_or(s.len())
                        }
                        _ => shell_word(s, j),
                    };
                    let id: String = line[j..k]
                        .chars()
                        .filter(|c| !matches!(c, '\'' | '"' | '\\'))
                        .collect();
                    if !id.is_empty() {
                        heredocs.push((id, tabs, n));
                    }
                    i = k.max(j);
                }
                b'$' if next == b'{' => {
                    let mut depth = 0;
                    while i < s.len() {
                        match s[i] {
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
                    i += 1;
                    cmd = false;
                }
                b'$' if next == b'(' => {
                    cmd = true;
                    i += 2;
                }
                _ => {
                    let j = shell_word(s, i).max(i + 1);
                    let w = text(s, i, j);
                    i = j;
                    let was = cmd;
                    cmd = false;
                    if w == "in" && b.stack.last().is_some_and(|o| o.word == "case" && o.armed) {
                        b.stack.last_mut().unwrap().armed = false;
                        pattern = true;
                        continue;
                    }
                    if !was && !(w == "{" && func.is_some()) {
                        continue;
                    }
                    match w {
                        "if" | "while" | "until" => {
                            b.open(n, if w == "if" { "if" } else { "loop" }, true, false, false);
                            cmd = true;
                        }
                        "for" | "select" => b.open(n, "loop", true, false, false),
                        "case" => b.open(n, "case", true, false, true),
                        "then" | "do" | "!" | "time" => cmd = true,
                        "elif" | "else" => {
                            b.clause(n);
                            cmd = true;
                        }
                        "fi" => b.close(n, &["if"]),
                        "done" => b.close(n, &["loop"]),
                        "esac" => b.close(n, &["case"]),
                        "function" => func = Some(n),
                        "{" => {
                            match func.take() {
                                Some(h) => {
                                    b.open(h, "{", true, true, false);
                                }
                                None => b.open(n, "{", false, false, false),
                            }
                            cmd = true;
                        }
                        "}" => b.close(n, &["{"]),
                        _ => {
                            let rest = line[i..].trim_start();
                            if rest.starts_with("()") {
                                func = Some(n);
                                i = s.len() - rest.len() + 2;
                                cmd = true;
                            } else if func.is_some() {
                                cmd = true;
                            }
                        }
                    }
                }
            }
        }
        if quote.is_none() && heredoc.is_none() && !heredocs.is_empty() {
            heredoc = Some(heredocs.remove(0));
        }
        if !line.ends_with('\\') {
            cmd = true;
        }
    }
    b.blocks
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
