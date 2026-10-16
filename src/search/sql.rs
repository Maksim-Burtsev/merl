use regex::Regex;

pub fn sql_cte(word: &str) -> String {
    let w = regex::escape(word);
    format!(r"(?i)^\s*(?:\)\s*)?(?:WITH\s+(?:RECURSIVE\s+)?|,\s*)?{w}(?:\s*\([^()]*\))?\s+AS(?:\s+(?:NOT\s+)?MATERIALIZED)?\s*\(")
}
pub fn sql_cte_sees(text: &str, cte: usize, word: &str, line: usize, col: usize) -> Option<bool> {
    let start = |l: usize| -> usize { text.split_inclusive('\n').take(l - 1).map(str::len).sum() };
    let head = Regex::new(&sql_cte(word)).expect("an escaped name keeps the pattern valid");
    let Some(m) = text.lines().nth(cte - 1).and_then(|l| head.find(l)) else {
        return Some(false);
    };
    let (open, at, b) = (start(cte) + m.end() - 1, start(line) + col, text.as_bytes());
    let (mut first, mut depth, mut close, mut i) = (0, None::<usize>, None, 0);
    let (mut ended, mut after) = (false, 0_usize);
    while i < b.len() {
        let skip = match &b[i..] {
            [b'-', b'-', ..] => "\n",
            [b'/', b'*', ..] => "*/",
            [b'\'', ..] => "'",
            [b'(', ..] if close.is_some() => {
                after += 1;
                ""
            }
            [b')', ..] if close.is_some() => {
                if after == 0 {
                    ended = true;
                    break;
                }
                after -= 1;
                ""
            }
            [b'(', ..] if i >= open => {
                depth = Some(depth.map_or(1, |d| d + 1));
                ""
            }
            [b')', ..] if close.is_none() => {
                depth = depth.map(|d| d - 1);
                if depth == Some(0) {
                    close = Some(i);
                }
                ""
            }
            [b';', ..] if i < open => {
                first = i + 1;
                ""
            }
            [b';', ..] => {
                ended = true;
                break;
            }
            _ => "",
        };
        i += match skip {
            "" => 1,
            s => text
                .get(i + 1..)
                .and_then(|t| t.find(s))
                .map_or(b.len(), |n| n + 1 + s.len()),
        };
    }
    let recursive = Regex::new(r"(?i)\bWITH\s+RECURSIVE\b").expect("a valid pattern");
    let from = if text.get(first..open).is_some_and(|t| recursive.is_match(t)) {
        open
    } else {
        close.unwrap_or(i)
    };
    if at <= from {
        Some(false)
    } else {
        ended.then_some(at <= i)
    }
}

const SQL_IDENT: &str = r#"(?:"[^"]+"|`[^`]+`|[A-Za-z_]\w*)"#;

pub fn sql_local(text: &str, at: usize, word: &str) -> Vec<usize> {
    let (code, bodies) = sql_code(text);
    let line1 = |i: usize| text.bytes().take(i).filter(|&c| c == b'\n').count() + 1;
    sql_parameter(&code, &bodies, at, word)
        .map(line1)
        .into_iter()
        .collect()
}

fn sql_code(text: &str) -> (String, Vec<std::ops::Range<usize>>) {
    let tag = Regex::new(r"^\$(?:[A-Za-z_]\w*)?\$").expect("a valid pattern");
    let (b, mut out) = (text.as_bytes(), text.as_bytes().to_vec());
    let (mut bodies, mut open, mut i) = (Vec::new(), None::<(usize, &str)>, 0);
    while i < b.len() {
        let rest = text.get(i..).unwrap_or_default();
        let skip = match b[i] {
            b'-' if rest.starts_with("--") => rest.find('\n'),
            b'/' if rest.starts_with("/*") => rest.get(2..).and_then(|r| r.find("*/")).map(|n| n + 4),
            b'\'' => rest.get(1..).and_then(|r| r.find('\'')).map(|n| n + 2),
            b'"' => {
                i += rest.get(1..).and_then(|r| r.find('"')).map_or(b.len(), |n| n + 2);
                continue;
            }
            b'$' if i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_') => {
                let Some(d) = tag.find(rest).map(|m| m.as_str()) else {
                    i += 1;
                    continue;
                };
                match open {
                    None => open = Some((i + d.len(), d)),
                    Some((from, t)) if t == d => {
                        bodies.push(from..i);
                        open = None;
                    }
                    Some(_) => {}
                }
                out[i..i + d.len()].fill(b';');
                i += d.len();
                continue;
            }
            _ => {
                i += 1;
                continue;
            }
        };
        let end = skip.map_or(b.len(), |n| (i + n).min(b.len()));
        out[i..end].iter_mut().filter(|c| **c != b'\n').for_each(|c| *c = b' ');
        i = end;
    }
    (String::from_utf8_lossy(&out).into_owned(), bodies)
}

fn sql_parameter(
    code: &str,
    bodies: &[std::ops::Range<usize>],
    at: usize,
    word: &str,
) -> Option<usize> {
    let body = bodies.iter().find(|b| b.contains(&at))?;
    let head_end = code.get(..body.start)?.trim_end_matches(';').len();
    let head_start = code.get(..head_end)?.rfind(';').map_or(0, |n| n + 1);
    let head = code.get(head_start..head_end)?;
    let create = Regex::new(&format!(
        r"(?i)\bCREATE\s+(?:OR\s+REPLACE\s+)?(?:FUNCTION|PROCEDURE)\s+{SQL_IDENT}(?:\s*\.\s*{SQL_IDENT})*\s*\("
    ))
    .expect("a valid pattern");
    let param = Regex::new(&format!(
        r"^\s*(?:(?i:IN|OUT|INOUT|VARIADIC)\s+)?({SQL_IDENT})\s+(\S+)"
    ))
    .expect("a valid pattern");
    let (mut from, mut depth) = (create.find(head)?.end(), 0_usize);
    for (i, c) in head.bytes().enumerate().skip(from) {
        if c == b'(' {
            depth += 1;
        } else if c == b')' && depth > 0 {
            depth -= 1;
        } else if (c == b',' || c == b')') && depth == 0 {
            let named = param.captures(head.get(from..i)?).filter(|p| {
                let (name, ty) = (&p[1], p[2].to_ascii_lowercase());
                !["in", "out", "inout", "variadic"].contains(&name.to_ascii_lowercase().as_str())
                    && !["precision", "varying", "with", "without"].contains(&ty.as_str())
                    && name.trim_matches(['"', '`']).eq_ignore_ascii_case(word)
            });
            if let Some(n) = named.and_then(|p| p.get(1)) {
                return Some(head_start + from + n.start());
            }
            if c == b')' {
                return None;
            }
            from = i + 1;
        }
    }
    None
}
