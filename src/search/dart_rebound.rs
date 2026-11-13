use regex::Regex;

use super::{
    Kind, close_of, code,
    dart::{ANY_TYPE, DartType, TYPE_BODY, dart_header_binds, dart_local},
    indent, literal_lines,
};

fn dart_masked(line: &str) -> Vec<u8> {
    let mut b = vec![b' '; line.len()];
    for (i, c) in code(Kind::Dart, line).filter(|&(_, c)| c != 0) {
        b[i] = c;
    }
    b
}

fn dart_reaches(b: &[u8], from: usize, col: usize, local: bool) -> bool {
    let mut depth = 0i32;
    for &c in b.get(from..col).unwrap_or_default() {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' => depth -= 1,
            b'}' => {
                depth -= 1;
                if depth == 0 && !local {
                    return false;
                }
            }
            b';' | b',' if depth == 0 && !local => return false,
            _ => {}
        }
        if depth < 0 {
            return false;
        }
    }
    true
}

fn dart_arm_open(b: &[u8], from: usize, col: usize) -> bool {
    static NEXT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:case|default)\b").expect("the regex is a literal")
    });
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'$';
    let mut depth = 0i32;
    (from + 1..col.min(b.len())).all(|k| {
        match b[k] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
        depth != 0 || word(b[k - 1]) || !NEXT.is_match(&String::from_utf8_lossy(&b[k..]))
    })
}

pub(super) fn dart_binder_at(line: &str, col: usize, name: &str) -> Option<usize> {
    dart_binder(line, col, name, true)
}

fn dart_binder(line: &str, col: usize, name: &str, reach: bool) -> Option<usize> {
    static KEYWORD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w$])(if|for|catch|switch)\s*$").expect("the regex is a literal")
    });
    let b = dart_masked(line);
    let line = String::from_utf8_lossy(&b).into_owned();
    let w = regex::escape(name);
    let arm =
        Regex::new(&format!(r"^{ANY_TYPE}\s+{w}\s*(?:when\b[^;]*?)?=>")).expect("an escaped name");
    (0..col.min(b.len())).rev().find(|&s| {
        let before = line.get(..s).unwrap_or_default().trim_end();
        if b[s] == b'(' {
            let Some(close) = close_of(Kind::Dart, &line, s).filter(|&c| c <= col) else {
                return false;
            };
            let after = line.get(close..).unwrap_or_default().trim_start();
            let params = line.get(s..close).unwrap_or_default();
            let head = match KEYWORD.captures(before) {
                Some(k) => format!("{} {params} {{", &k[1]),
                None if ["=>", "{", "async", "sync"]
                    .iter()
                    .any(|p| after.starts_with(p)) =>
                {
                    format!("{params} {{")
                }
                None => return false,
            };
            return dart_header_binds(&head, name) && (!reach || dart_reaches(&b, s, col, false));
        }
        if b[s] == b' ' || !(before.is_empty() || before.ends_with(['{', ';', ','])) {
            return false;
        }
        let t = line.get(s..).unwrap_or_default().trim_end();
        if t.starts_with("case") && dart_header_binds(t, name) {
            return !reach || (dart_reaches(&b, s, col, true) && dart_arm_open(&b, s, col));
        }
        if dart_local(t, name) {
            return !reach || dart_reaches(&b, s, col, true);
        }
        arm.is_match(t) && (!reach || dart_reaches(&b, s, col, false))
    })
}

pub fn dart_binders(line: &str, name: &str) -> usize {
    let mut n = 0;
    let mut col = line.len();
    while let Some(s) = dart_binder(line, col, name, false) {
        n += 1;
        col = s;
    }
    n
}

fn dart_narrows(cond: &str, w: &str, not: bool) -> bool {
    let (and, or) = if not { ("||", "&&") } else { ("&&", "||") };
    let b = cond.as_bytes();
    let mut depth = 0i32;
    let mut parts = Vec::new();
    let mut from = 0;
    for k in 0..b.len() {
        match b[k] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ if depth == 0 && b[k..].starts_with(or.as_bytes()) => return false,
            _ if depth == 0 && b[k..].starts_with(and.as_bytes()) => {
                parts.push(cond.get(from..k).unwrap_or_default());
                from = k + 2;
            }
            _ => {}
        }
    }
    parts.push(cond.get(from..).unwrap_or_default());
    let is = Regex::new(&format!(
        r"^{w}\s+is{}",
        if not { "!" } else { r"\s+[\w$]" }
    ))
    .expect("an escaped name");
    parts.iter().any(|p| is.is_match(p.trim()))
}

fn dart_condition(t: &str) -> Option<(&str, &str)> {
    static CONTROL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:\}\s*else\s+)?(?:if|while)\s*\(").expect("the regex is a literal")
    });
    let open = CONTROL.find(t)?.end() - 1;
    let close = close_of(Kind::Dart, t, open)?;
    Some((t.get(open + 1..close - 1)?, t.get(close..)?.trim()))
}

fn dart_exits(body: &str) -> bool {
    static EXIT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:return|throw|continue|break)\b").expect("the regex is a literal")
    });
    let body = body.trim().trim_start_matches('{').trim_end_matches('}');
    body.split(';')
        .map(str::trim)
        .rfind(|s| !s.is_empty())
        .is_some_and(|s| EXIT.is_match(s.trim_start_matches('{').trim()))
}

pub fn dart_promoted(text: &str, line1: usize, col: usize, name: &str) -> bool {
    static CONTROL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w$])(?:if|while)\s*$").expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let w = regex::escape(name);
    let is = Regex::new(&format!(r"(?:^|[^\w$.]){w}\s+is\s+[\w$]")).expect("an escaped name");
    let b = dart_masked(lines[at]);
    let own = String::from_utf8_lossy(&b).into_owned();
    let here = own.get(..col.min(own.len())).unwrap_or_default();
    let binder = dart_binder_at(lines[at], col, name);
    if is.find_iter(here).any(|m| {
        let mut depth = 0i32;
        let start = m.start() + m.as_str().find(name).unwrap_or(0);
        let open = (0..start).rev().find(|&k| {
            match b[k] {
                b')' | b']' | b'}' => depth += 1,
                b'(' | b'[' | b'{' => depth -= 1,
                _ => {}
            }
            depth < 0
        });
        let control =
            open.filter(|&o| b[o] == b'(' && CONTROL.is_match(own.get(..o).unwrap_or_default()));
        let from = match control {
            Some(o) => {
                let cond = close_of(Kind::Dart, &own, o).and_then(|c| own.get(o + 1..c - 1));
                if !cond.is_some_and(|c| dart_narrows(c, &w, false)) {
                    return false;
                }
                o
            }
            None => start,
        };
        binder.is_none_or(|s| s < from) && dart_reaches(&b, from, col, false)
    }) {
        return true;
    }
    if binder.is_some() {
        return false;
    }
    let literal = literal_lines(Kind::Dart, text);
    let mut depth = indent(lines[at]);
    for i in (0..at).rev() {
        let masked = dart_masked(lines[i]);
        let t = String::from_utf8_lossy(&masked);
        let t = t.trim();
        let ind = indent(lines[i]);
        if literal[i] || t.is_empty() || ind > depth {
            continue;
        }
        if ind == depth {
            if dart_local(t, name) {
                return false;
            }
            let exits = |rest: &str| {
                if rest != "{" {
                    return dart_exits(rest);
                }
                let last = (i + 1..at)
                    .take_while(|&k| lines[k].trim().is_empty() || indent(lines[k]) > depth)
                    .filter(|&k| !lines[k].trim().is_empty())
                    .last();
                last.is_some_and(|k| dart_exits(lines[k]))
            };
            if dart_condition(t).is_some_and(|(c, rest)| dart_narrows(c, &w, true) && exits(rest)) {
                return true;
            }
            continue;
        }
        if dart_binder_at(lines[i], lines[i].len(), name).is_some() {
            return false;
        }
        if dart_condition(t).is_some_and(|(c, _)| dart_narrows(c, &w, false)) {
            return true;
        }
        depth = ind;
        if depth == 0 || TYPE_BODY.is_match(lines[i]) {
            return false;
        }
    }
    false
}

pub fn dart_cast(before: &str) -> Option<DartType> {
    static CAST: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"(?:^|[^\w$.])\(\s*[\w$.]+\s+as\s+(?:([a-z_$][\w$]*)\.)?(_?[A-Z][\w$]*)\??\s*\)[?!]?\.$",
        )
        .expect("the regex is a literal")
    });
    let c = CAST.captures(before.trim_end())?;
    Some(DartType {
        prefix: c.get(1).map(|m| m.as_str().to_owned()),
        class: c[2].to_owned(),
        constructor: None,
    })
}
