use crate::search::{Kind, lisp_code};

fn delimiter(c: u8, clojure: bool) -> bool {
    c.is_ascii_whitespace() || b"()[]{}\";".contains(&c) || (clojure && c == b',')
}

fn skip_to(
    code: &[u8],
    mut i: usize,
    line: &mut usize,
    done: impl Fn(u8, &mut usize) -> bool,
) -> Option<usize> {
    let mut depth = 0;
    while i < code.len() {
        if code[i] == b'\n' {
            *line += 1;
        } else if done(code[i], &mut depth) {
            return Some(i + 1);
        }
        i += 1;
    }
    None
}

pub(super) fn lisp(lines: &[String], kind: Kind) -> Vec<(usize, usize)> {
    let code = lisp_code(kind, &lines.join("\n"));
    let clojure = kind == Kind::Clojure;
    let opens: &[u8] = if kind == Kind::Scheme { b"([{" } else { b"(" };
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let (mut i, mut line) = (0, 0);
    let (mut meta, mut skip) = (None, 0usize);
    while i < code.len() {
        let c = code[i];
        if c == b'\n' {
            line += 1;
        }
        if c.is_ascii_whitespace() || (clojure && c == b',') || b")]};".contains(&c) {
            i += 1;
            continue;
        }
        let start = line;
        let from = i;
        while i < code.len() && !delimiter(code[i], clojure) {
            i += 1;
        }
        let prefix = &code[from..i];
        let open = code.get(i).copied().filter(|c| b"([{".contains(c));
        if open.is_some() {
            let Some(end) = skip_to(&code, i, &mut line, |c, depth| {
                match c {
                    b'(' | b'[' | b'{' => *depth += 1,
                    b')' | b']' | b'}' => *depth -= 1,
                    _ => {}
                }
                *depth == 0
            }) else {
                break;
            };
            i = end;
        } else if code.get(i) == Some(&b'"') {
            let Some(end) = skip_to(&code, i + 1, &mut line, |c, _| c == b'"') else {
                break;
            };
            i = end;
        }
        if clojure && (prefix.starts_with(b"^") || prefix.starts_with(b"#^")) {
            meta.get_or_insert(start);
            continue;
        }
        let head = meta.take().unwrap_or(start);
        if skip == 0 && prefix.is_empty() && open.is_some_and(|o| opens.contains(&o)) && line > head
        {
            spans.push((head, line));
        }
        skip = skip.saturating_sub(1);
        if kind == Kind::CommonLisp && (prefix.starts_with(b"#+") || prefix.starts_with(b"#-")) {
            skip += 1;
        }
    }
    let starts: Vec<usize> = spans.iter().map(|s| s.0).collect();
    spans.retain(|s| !starts.contains(&s.1));
    spans
}
