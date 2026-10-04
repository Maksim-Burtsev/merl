use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

const PACKAGE: &str = r"[A-Za-z_]\w*(?:::\w+)*";

pub fn perl_sub_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    vec![
        format!(r"^\s*(?:sub|method)\s+(?:\w+::)*{w}\s*(?:[({{:]|$)"),
        format!(r"^\s*use\s+constant\s+{w}\s*(?:=>|,)"),
        format!(r"^\s*use\s+constant\s*\{{.*\b{w}\s*=>"),
        perl_constant_entry(word),
        format!(r#"^\s*has\s*(?:\(\s*)?(?:['"]\+?{w}['"]|\+?{w}\b|\[[^\]]*\b{w}\b)"#),
    ]
}

fn perl_constant_entry(word: &str) -> String {
    format!(r"^\s*{}\s*=>", regex::escape(word))
}

pub fn perl_roots(root: &Path, inc: &str) -> Vec<PathBuf> {
    std::iter::once(root.join("local/lib/perl5"))
        .chain(inc.lines().map(PathBuf::from).filter(|p| p.is_absolute()))
        .collect()
}

pub fn perl_package_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    vec![format!(
        r"^\s*(?:package|class)\s+{w}(?:\s*[;{{]|\s+v?\d|\s+:|\s*$)"
    )]
}

pub fn perl_variable_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    vec![
        format!(r"^\s*our\s+[$@%]{w}\b"),
        format!(r"^\s*our\s*\([^)]*[$@%]{w}\b"),
        format!(r"^\s*field\s+[$@%]{w}\b"),
    ]
}

pub fn perl_patterns(word: &str) -> Vec<String> {
    let mut all = perl_sub_patterns(word);
    all.extend(perl_package_patterns(word));
    all.extend(perl_variable_patterns(word));
    all
}

pub(super) const PERL_SUB_SYMBOL: &str =
    r"^\s*(?:sub|method)\s+(?P<name>[A-Za-z_]\w*(?:::\w+)*)\s*(?:[({:]|$)";
pub(super) const PERL_PACKAGE_SYMBOL: &str =
    r"^\s*(?:package|class)\s+(?P<name>[A-Za-z_]\w*(?:::\w+)*)(?:\s*[;{]|\s+v?\d|\s+:|\s*$)";

pub fn perl_declares<S: AsRef<str>>(lines: &[S], line: usize, word: &str, text: &str) -> bool {
    static KEY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*\w+\s*=>").unwrap());
    static ENTRY: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*(?:\w+\s*=>.*)?(?:#.*)?$").unwrap());
    static OPENER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*use\s+constant\s*\{").unwrap());
    let entry = perl_constant_entry(word);
    let others = (perl_patterns(word).into_iter())
        .filter(|p| *p != entry)
        .collect::<Vec<_>>()
        .join("|");
    if !KEY.is_match(text) || Regex::new(&others).is_ok_and(|re| re.is_match(text)) {
        return true;
    }
    let mut at = line.saturating_sub(1);
    while at > 0 {
        at -= 1;
        let Some(l) = lines.get(at).map(AsRef::as_ref) else {
            return false;
        };
        if OPENER.is_match(l) {
            return !l.contains('}');
        }
        if !ENTRY.is_match(l) {
            return false;
        }
    }
    false
}

pub fn perl_name(line: &str, col: usize) -> Option<Range<usize>> {
    let b = line.as_bytes();
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut start = col.min(b.len());
    while start > 0 && word(b[start - 1]) {
        start -= 1;
    }
    let mut end = col.min(b.len());
    while end < b.len() && word(b[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    while start >= 3 && &b[start - 2..start] == b"::" && word(b[start - 3]) {
        start -= 2;
        while start > 0 && word(b[start - 1]) {
            start -= 1;
        }
    }
    while b.get(end..end + 2) == Some(b"::") && b.get(end + 2).is_some_and(|&c| word(c)) {
        end += 2;
        while end < b.len() && word(b[end]) {
            end += 1;
        }
    }
    Some(start..end)
}

pub fn perl_used_module(line: &str) -> Option<Range<usize>> {
    static USE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(&format!(r"^\s*(?:use|no|require)\s+({PACKAGE})\b")).unwrap());
    let m = USE.captures(line)?.get(1)?;
    (!m.as_str().starts_with(|c: char| c.is_ascii_digit())).then(|| m.range())
}

pub fn perl_module_file(package: &str) -> String {
    format!("{}.pm", package.replace("::", "/"))
}

pub fn perl_imports(text: &str) -> Vec<(String, String)> {
    static USE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^\s*use\s+([A-Z]\w*(?:::\w+)*)(?:\s+v?[\d._]+)?\s+([^;]*);").unwrap()
    });
    static ITEM: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"\bqw\s*[(\[{</|]([^)\]}>/|]*)|'([^']*)'|"([^"]*)""#).unwrap()
    });
    let literal = perl_literal_lines(text);
    let starts: Vec<usize> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let mut out = Vec::new();
    for c in USE.captures_iter(text) {
        let at = c.get(0).map_or(0, |m| m.start());
        let line = starts.partition_point(|&s| s <= at).saturating_sub(1);
        if literal.get(line) == Some(&true) {
            continue;
        }
        let module = &c[1];
        for item in ITEM.captures_iter(&c[2]) {
            let list = (1..=3).find_map(|g| item.get(g)).map_or("", |m| m.as_str());
            for name in list.split_whitespace() {
                let name = name.trim_start_matches('&');
                if name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                    && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    out.push((name.to_owned(), module.to_owned()));
                }
            }
        }
    }
    out
}

pub fn perl_lexical(text: &str, line: usize, col: usize, name: &str) -> Option<usize> {
    let (literal, code) = perl_lex(text);
    let n = regex::escape(name);
    let decl = Regex::new(&format!(
        r"\b(?:my|state)\s+(?:[A-Z][\w:]*\s+)?[$@%]{n}\b|\b(?:my|state)\s*\([^)]*[$@%]{n}\b|^\s*(?:sub|method)\s+[\w:]*\s*\([^)]*[$@%]{n}\b"
    ))
    .ok()?;
    static HEADER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\b(?:for|foreach)\s+$").unwrap());
    let mut depth = 0usize;
    for i in (0..=line.min(code.len().saturating_sub(1))).rev() {
        if literal[i] && i != line {
            continue;
        }
        let l = &code[i];
        let limit = match i == line {
            true => col.min(l.len()),
            false => l.len(),
        };
        let starts: Vec<(usize, bool)> = (decl.find_iter(l))
            .filter(|m| m.start() < limit)
            .map(|m| {
                let head = m.as_str().trim_start();
                let signature = head.starts_with("sub") || head.starts_with("method");
                (m.start(), signature || HEADER.is_match(&l[..m.start()]))
            })
            .collect();
        let mut closed = false;
        for (p, c) in l[..limit].char_indices().rev() {
            match c {
                '}' => depth += 1,
                '{' if depth > 0 => {
                    depth -= 1;
                    closed |= depth == 0;
                }
                _ => {}
            }
            if depth == 0
                && starts
                    .iter()
                    .any(|&(s, header)| s == p && !(header && closed))
            {
                return Some(i + 1);
            }
        }
    }
    None
}

pub fn perl_literal_lines(text: &str) -> Vec<bool> {
    perl_lex(text).0
}

enum State {
    Code,
    Pod,
    Data,
    Heredoc(String, bool),
    Quote(u8, u8, usize),
}

fn perl_lex(text: &str) -> (Vec<bool>, Vec<String>) {
    let mut literal = Vec::new();
    let mut code = Vec::new();
    let mut state = State::Code;
    let mut pending: std::collections::VecDeque<(String, bool)> = Default::default();
    for line in text.split('\n') {
        match &state {
            State::Data => {
                literal.push(true);
                code.push(String::new());
                continue;
            }
            State::Pod => {
                literal.push(true);
                code.push(String::new());
                if line.starts_with("=cut") {
                    state = State::Code;
                }
                continue;
            }
            State::Heredoc(label, indented) => {
                literal.push(true);
                code.push(String::new());
                let closing = match indented {
                    true => line.trim(),
                    false => line,
                };
                if closing == label {
                    state = match pending.pop_front() {
                        Some((l, i)) => State::Heredoc(l, i),
                        None => State::Code,
                    };
                }
                continue;
            }
            State::Code => {
                if line.starts_with('=') && line[1..].starts_with(|c: char| c.is_ascii_alphabetic())
                {
                    literal.push(true);
                    code.push(String::new());
                    if !line.starts_with("=cut") {
                        state = State::Pod;
                    }
                    continue;
                }
                literal.push(false);
                if matches!(line.trim_end(), "__END__" | "__DATA__") {
                    code.push(String::new());
                    state = State::Data;
                    continue;
                }
            }
            State::Quote(..) => literal.push(true),
        }
        let (blanked, next, heredocs) = scan_line(line, std::mem::replace(&mut state, State::Code));
        code.push(blanked);
        pending.extend(heredocs);
        state = match (next, pending.pop_front()) {
            (State::Code, Some((l, i))) => State::Heredoc(l, i),
            (next, Some(h)) => {
                pending.push_front(h);
                next
            }
            (next, None) => next,
        };
    }
    (literal, code)
}

fn scan_line(line: &str, state: State) -> (String, State, Vec<(String, bool)>) {
    let b = line.as_bytes();
    let mut out = b.to_vec();
    let mut heredocs = Vec::new();
    let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut quote = match state {
        State::Quote(open, close, depth) => Some((open, close, depth)),
        _ => None,
    };
    let mut i = 0;
    while i < b.len() {
        if let Some((open, close, depth)) = quote.as_mut() {
            let c = b[i];
            out[i] = b' ';
            if c == b'\\' {
                if i + 1 < b.len() {
                    out[i + 1] = b' ';
                }
                i += 2;
                continue;
            }
            if c == *close {
                if *depth == 0 {
                    quote = None;
                } else {
                    *depth -= 1;
                }
            } else if c == *open && open != close {
                *depth += 1;
            }
            i += 1;
            continue;
        }
        let c = b[i];
        let prev = i.checked_sub(1).map(|p| b[p]);
        match c {
            b'#' if prev != Some(b'$') => {
                out[i..].fill(b' ');
                break;
            }
            b'\'' | b'"' | b'`' if !(c == b'\'' && prev.is_some_and(ident)) => {
                let end = (i + 1..b.len())
                    .scan(false, |esc, j| {
                        let hit = !*esc && b[j] == c;
                        *esc = !*esc && b[j] == b'\\';
                        Some((j, hit))
                    })
                    .find(|&(_, hit)| hit)
                    .map_or(b.len(), |(j, _)| j + 1);
                out[i..end].fill(b' ');
                i = end;
                continue;
            }
            b'<' if b[i..].starts_with(b"<<") && prev != Some(b'<') => {
                let mut j = i + 2;
                let indented = b.get(j) == Some(&b'~');
                if indented {
                    j += 1;
                }
                let (label, end) = match b.get(j) {
                    Some(&q @ (b'"' | b'\'')) => {
                        let close = b[j + 1..].iter().position(|&x| x == q);
                        match close {
                            Some(n) => (Some(&line[j + 1..j + 1 + n]), j + n + 2),
                            None => (None, j),
                        }
                    }
                    Some(&x) if x.is_ascii_alphabetic() || x == b'_' => {
                        let n = b[j..].iter().take_while(|&&x| ident(x)).count();
                        (Some(&line[j..j + n]), j + n)
                    }
                    _ => (None, j),
                };
                if let Some(label) = label {
                    heredocs.push((label.to_owned(), indented));
                    i = end;
                    continue;
                }
                i += 2;
                continue;
            }
            b'q' if !prev.is_some_and(|p| {
                ident(p) || matches!(p, b'$' | b'@' | b'%' | b'&' | b'>' | b':')
            }) =>
            {
                let n = match b.get(i + 1) {
                    Some(b'q' | b'w') => 2,
                    _ => 1,
                };
                let after = &b[i + n..];
                let gap = after.iter().take_while(|c| c.is_ascii_whitespace()).count();
                let d = after.get(gap).copied();
                let fat = after[gap..].starts_with(b"=>");
                if let Some(d) = d.filter(|&d| {
                    !ident(d)
                        && !d.is_ascii_whitespace()
                        && !matches!(d, b',' | b';' | b')')
                        && !fat
                }) && (gap == 0 || !matches!(d, b'=' | b'#'))
                {
                    let close = match d {
                        b'(' => b')',
                        b'[' => b']',
                        b'{' => b'}',
                        b'<' => b'>',
                        x => x,
                    };
                    let start = i + n + gap;
                    out[i..=start].fill(b' ');
                    quote = Some((d, close, 0));
                    i = start + 1;
                    continue;
                }
                i += n;
                continue;
            }
            _ => {}
        }
        i += if ident(c) {
            b[i..].iter().take_while(|&&x| ident(x)).count()
        } else {
            1
        };
    }
    let next = match quote {
        Some((open, close, depth)) => State::Quote(open, close, depth),
        None => State::Code,
    };
    (String::from_utf8_lossy(&out).into_owned(), next, heredocs)
}
