use super::words::{Blocks, word_byte};

const KEYWORDS: &[&str] = &[
    "and", "or", "not", "in", "if", "else", "for", "return", "lambda", "yield", "assert", "del",
];

struct Stmt {
    first: usize,
    last: usize,
    indent: usize,
    word: String,
}

pub(crate) fn starlark(lines: &[String]) -> Blocks {
    let mut nodes = Vec::new();
    let mut stmts: Vec<Stmt> = Vec::new();
    let mut stack: Vec<(usize, bool)> = Vec::new();
    let mut string: Option<(usize, &[u8])> = None;
    let mut open = false;
    let mut last = 0u8;
    let mut word = "";
    for (n, line) in lines.iter().enumerate() {
        let b = line.as_bytes();
        let t = line.trim_start();
        let code = !t.is_empty() && !t.starts_with('#');
        let part = open || code;
        if !open && !t.is_empty() {
            let w = t.split(|c: char| !c.is_alphanumeric() && c != '_').next();
            let w = if code { w } else { Some("#") };
            stmts.push(Stmt {
                first: n,
                last: n,
                indent: line.len() - t.len(),
                word: w.unwrap_or("").into(),
            });
            last = 0;
        }
        let mut i = 0;
        while i < b.len() {
            if let Some((start, delim)) = string {
                let (step, closed) = super::string_step(&b[i..], delim);
                if closed {
                    nodes.push((start, n));
                    string = None;
                    last = b'"';
                }
                i += step;
                continue;
            }
            let c = b[i];
            match c {
                b'#' => break,
                b'"' | b'\'' => {
                    let triple: &[u8] = if c == b'"' { b"\"\"\"" } else { b"'''" };
                    if b[i..].starts_with(triple) {
                        string = Some((n, triple));
                        i += 3;
                        continue;
                    }
                    i = super::skip_quoted(b, i + 1, c);
                    last = b'"';
                    continue;
                }
                b'(' | b'{' => stack.push((n, true)),
                b'[' => {
                    let index = (word_byte(last) && !KEYWORDS.contains(&word))
                        || matches!(last, b')' | b']' | b'}' | b'"');
                    stack.push((n, !index));
                }
                b')' | b']' | b'}' => {
                    if let Some((h, true)) = stack.pop() {
                        nodes.push((h, n));
                    }
                }
                _ => {}
            }
            if word_byte(c) {
                let j = (i..b.len()).find(|&j| !word_byte(b[j])).unwrap_or(b.len());
                word = std::str::from_utf8(&b[i..j]).unwrap_or("");
                last = b[j - 1];
                i = j;
                continue;
            }
            if c != b' ' && c != b'\t' {
                last = c;
            }
            i += 1;
        }
        open = string.is_some() || !stack.is_empty() || line.trim_end().ends_with('\\');
        if let Some(s) = stmts.last_mut().filter(|_| part) {
            s.last = n;
        }
        if !open {
            word = "";
        }
    }
    let mut funcs = Vec::new();
    for (k, s) in stmts.iter().enumerate() {
        if !["def", "if", "for", "while", "with"].contains(&s.word.as_str()) {
            continue;
        }
        let mut end = s.last;
        for next in &stmts[k + 1..] {
            if next.word == "#" {
                if next.indent > s.indent {
                    end = next.last;
                }
                continue;
            }
            let clause = match s.word.as_str() {
                "if" => ["elif", "else"].contains(&next.word.as_str()),
                "for" | "while" => next.word == "else",
                _ => false,
            };
            if next.indent < s.indent || (next.indent == s.indent && !clause) {
                break;
            }
            end = next.last;
        }
        nodes.push((s.first, end));
        if s.word == "def" {
            funcs.push((s.first, end));
        }
    }
    Blocks::from_nodes(nodes, funcs, lines.len())
}
