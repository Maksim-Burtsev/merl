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

pub fn cs_anonymous_member(text: &str, bound1: usize, var: &str, member: &str) -> Option<usize> {
    let made = Regex::new(&format!(r"\b{}\s*=\s*new\s*(?:\{{|$)", regex::escape(var))).ok()?;
    let named = Regex::new(&format!(
        r"^\s*(?:[\w.]+\.)?{}\s*(?:=[^=>]|[,}}]|$)",
        regex::escape(member)
    ))
    .ok()?;
    let lines: Vec<String> = text.lines().map(|l| uncommented(Kind::CSharp, l)).collect();
    let mut from = bound1.checked_sub(1)?;
    let mut start = made.find(lines.get(from)?)?.end();
    if !lines[from].get(..start)?.ends_with('{') {
        from += 1;
        let next = lines.get(from)?;
        start = next.len() - next.trim_start().len() + 1;
        next.trim_start().starts_with('{').then_some(())?;
    }
    let (mut depth, mut fresh) = (1i32, true);
    for (i, line) in lines.iter().enumerate().skip(from) {
        let mut field = (depth == 1 && fresh).then_some(start);
        for (at, c) in code(Kind::CSharp, line).filter(|&(at, _)| at >= start) {
            match c {
                b'{' | b'(' | b'[' => depth += 1,
                b'}' | b')' | b']' => depth -= 1,
                _ => {}
            }
            if !c.is_ascii_whitespace() {
                fresh = c == b',' && depth == 1;
            }
            if matches!((c, depth), (b',', 1) | (b'}', 0)) {
                if let Some(f) = field
                    && named.is_match(line.get(f..=at)?)
                {
                    return Some(i + 1);
                }
                field = Some(at + 1);
            }
            if depth == 0 {
                return None;
            }
        }
        if let Some(f) = field.filter(|_| depth == 1)
            && named.is_match(line.get(f..)?)
        {
            return Some(i + 1);
        }
        start = 0;
    }
    None
}
