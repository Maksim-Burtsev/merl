use regex::Regex;

use super::{Kind, bindings, code, cs_enclosing, cs_type_decl, uncommented};

const NOT_TYPES: &[&str] = &[
    "return", "new", "is", "as", "await", "in", "out", "ref", "yield", "else", "case", "throw",
    "typeof", "not", "and", "or", "using", "goto", "nameof", "sizeof", "default", "params",
];

fn blanked(s: &str) -> String {
    let mut bytes = vec![b' '; s.len()];
    for (i, c) in code(Kind::CSharp, s) {
        bytes[i] = if c.is_ascii() { c } else { b' ' };
    }
    String::from_utf8(bytes).unwrap_or_default()
}

fn closed(rest: &str, local: bool) -> bool {
    let (mut paren, mut brace, mut arrow) = (0i32, 0i32, false);
    let b = rest.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'(' | b'[' => paren += 1,
            b')' | b']' => paren -= 1,
            b'{' => brace += 1,
            b'}' => {
                brace -= 1;
                if brace < 0 || (!local && brace == 0) {
                    return true;
                }
            }
            b'=' if b.get(i + 1) == Some(&b'>') && paren <= 0 && brace == 0 => arrow = true,
            b';' if !local && brace == 0 && paren <= 0 => return true,
            b',' if !local && arrow && brace == 0 && paren <= 0 => return true,
            _ => {}
        }
    }
    false
}

enum Rebound {
    Clear,
    Open,
    Closed,
    Unsure,
}

fn rebound_before(before: &str, name: &str) -> Rebound {
    let n = regex::escape(name);
    let Ok(shape) = Regex::new(&format!(
        r"\b([A-Za-z_][\w.]*(?:<[^;{{}}]*?>)?(?:\[\])?\??)\s+{n}\s*(=>|=|[;,):?]|\bwhen\b|\bin\b|&&|\|\|)|[(,]\s*{n}\s*([,)])|\b{n}\s*(=>)|\}}\s*{n}\s*(?:[):?]|=>|\bwhen\b|&&|\|\|)|\bfrom\s+(?:\w+\s+)?{n}\s+in\b"
    )) else {
        return Rebound::Clear;
    };
    let line = blanked(before);
    let found: Vec<_> = shape
        .captures_iter(&line)
        .filter(|c| c.get(1).is_none_or(|t| !NOT_TYPES.contains(&t.as_str())))
        .collect();
    let Some(last) = found.last() else {
        return Rebound::Clear;
    };
    let whole = last.get(0).map_or(0..0, |m| m.range());
    let out = line
        .get(..whole.start)
        .is_some_and(|l| l.trim_end().ends_with("out"));
    let local = out || last.get(2).is_some_and(|t| matches!(t.as_str(), "=" | ";"));
    let rest = line.get(whole.end..).unwrap_or_default();
    match (found.len(), closed(rest, local)) {
        (1, false) => Rebound::Open,
        (1, true) => Rebound::Closed,
        _ => Rebound::Unsure,
    }
}

fn rebound_above(text: &str, line1: usize, name: &str, bound_at: Option<usize>) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(mut member) = cs_enclosing(&lines, line1 - 1) else {
        return false;
    };
    while let Some(up) = cs_enclosing(&lines, member) {
        if cs_type_decl(lines[up]).is_some() {
            break;
        }
        member = up;
    }
    let n = regex::escape(name);
    let Ok(shape) = Regex::new(&format!(
        r"\bis\s+not\s+[\w.]+(?:<[^>]*>)?\s+{n}\b|!\s*\(.*\bis\s+[\w.]+(?:<[^>]*>)?\s+{n}\b|\bfrom\s+(?:\w+\s+)?{n}\s+in\b"
    )) else {
        return false;
    };
    (member..line1 - 1).any(|i| {
        Some(i + 1) != bound_at && shape.is_match(&blanked(&uncommented(Kind::CSharp, lines[i])))
    })
}

pub fn cs_rebound(text: &str, line1: usize, before: &str, name: &str) -> bool {
    if matches!(name, "this" | "base") {
        return false;
    }
    let bound = bindings(Kind::CSharp, text, line1, name)
        .first()
        .map(|b| b.line1);
    let mine = bound == Some(line1);
    let trusted = match rebound_before(before, name) {
        Rebound::Clear => true,
        Rebound::Open => mine,
        Rebound::Closed => !mine,
        Rebound::Unsure => false,
    };
    !trusted || rebound_above(text, line1, name, bound)
}
