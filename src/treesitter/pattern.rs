use std::fmt::Write as _;

#[expect(
    clippy::match_same_arms,
    reason = "the rows group Vim's specials by what they mean"
)]
pub(super) fn vim_pattern(p: &str) -> Result<regex::bytes::Regex, String> {
    let bad = || Err(format!("vim pattern {p:?} is not supported"));
    if ["\\m", "\\M", "\\V"].iter().any(|m| p.starts_with(m)) {
        return bad();
    }
    let body = p.strip_prefix("\\v").unwrap_or(p);
    let mut out = String::new();
    let mut chars = body.chars().peekable();
    let mut class = None;
    let mut case = "";
    while let Some(c) = chars.next() {
        match (c, class) {
            ('<' | '>' | '=' | '@' | '%' | '~' | '{' | '&', _) => return bad(),
            ('\\' | '[', Some(_)) => return bad(),
            ('-', Some(_)) if chars.peek() == Some(&'-') => return bad(),
            (']', Some(false)) => class = None,
            ('^', Some(true)) => {}
            (_, Some(_)) => class = Some(false),
            ('[', None) => class = Some(true),
            ('\\', None) => match chars.next() {
                Some(n @ ('d' | 'w' | 'D' | 'W')) => {
                    out.push('\\');
                    out.push(n);
                    continue;
                }
                Some('c') => {
                    case = "i";
                    continue;
                }
                Some('C') => continue,
                Some(n) if !n.is_ascii_alphanumeric() && n != '_' => {
                    out.push_str(&regex::escape(&n.to_string()));
                    continue;
                }
                _ => return bad(),
            },
            _ => {}
        }
        out.push(c);
    }
    regex::bytes::Regex::new(&format!("(?s{case}-u){out}"))
        .map_err(|e| format!("vim pattern {p:?}: {e}"))
}

pub(super) fn lua_replacement(with: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut chars = with.chars();
    while let Some(c) = chars.next() {
        match c {
            '%' => match chars.next() {
                Some(d @ '0'..='9') => out.extend(format!("${{{d}}}").bytes()),
                Some('%') => out.push(b'%'),
                _ => return Err(format!("lua replacement {with:?}: invalid use of '%'")),
            },
            '$' => out.extend(b"$$"),
            c => out.extend(c.to_string().bytes()),
        }
    }
    Ok(out)
}

pub(super) fn lua_pattern(p: &str) -> Result<regex::bytes::Regex, String> {
    let class = |c: char| -> Option<&'static str> {
        Some(match c {
            'a' => "A-Za-z",
            'c' => "\\x00-\\x1f\\x7f",
            'd' => "0-9",
            'g' => "!-~",
            'l' => "a-z",
            'p' => "!-/:-@\\[-`{-~",
            's' => "\\t\\n\\x0b\\x0c\\r ",
            'u' => "A-Z",
            'w' => "A-Za-z0-9",
            'x' => "0-9A-Fa-f",
            _ => return None,
        })
    };
    let bad = || Err(format!("lua pattern {p:?} is not supported"));
    let chars: Vec<char> = p.chars().collect();
    let mut out = String::from("(?s-u)");
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '^' if i == 0 => out.push('^'),
            '$' if i == chars.len() - 1 => out.push('$'),
            '.' => out.push('.'),
            '*' | '+' | '?' | '(' | ')' => out.push(c),
            '-' => out.push_str("*?"),
            '%' => {
                i += 1;
                let Some(&n) = chars.get(i) else { return bad() };
                if let Some(set) = class(n.to_ascii_lowercase()) {
                    let neg = if n.is_ascii_uppercase() { "^" } else { "" };
                    _ = write!(out, "[{neg}{set}]");
                } else if n.is_ascii_alphanumeric() {
                    return bad();
                } else {
                    out.push_str(&regex::escape(&n.to_string()));
                }
            }
            '[' => {
                out.push('[');
                i += 1;
                if chars.get(i) == Some(&'^') {
                    out.push('^');
                    i += 1;
                }
                let mut first = true;
                loop {
                    let Some(&m) = chars.get(i) else { return bad() };
                    if m == ']' && !first {
                        break;
                    }
                    first = false;
                    if m == '%' {
                        i += 1;
                        let Some(&n) = chars.get(i) else { return bad() };
                        match class(n) {
                            Some(set) => out.push_str(set),
                            None if n.is_ascii_alphanumeric() => return bad(),
                            None => _ = write!(out, "\\x{:02x}", n as u32),
                        }
                    } else if m == '-' {
                        out.push('-');
                    } else if m.is_ascii_alphanumeric() {
                        out.push(m);
                    } else {
                        _ = write!(out, "\\x{:02x}", m as u32);
                    }
                    i += 1;
                }
                out.push(']');
            }
            _ => out.push_str(&regex::escape(&c.to_string())),
        }
        i += 1;
    }
    regex::bytes::Regex::new(&out).map_err(|e| format!("lua pattern {p:?}: {e}"))
}
