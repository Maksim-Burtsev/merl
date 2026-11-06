use regex::Regex;

use super::{
    Kind, close_of, code,
    dart::{ANY_TYPE, TYPE_BODY, dart_header_binds, dart_local},
    indent, literal_lines, uncommented,
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

pub(super) fn dart_binds_at(line: &str, col: usize, name: &str) -> bool {
    static KEYWORD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w$])(if|for|catch)\s*$").expect("the regex is a literal")
    });
    let b = dart_masked(line);
    let line = String::from_utf8_lossy(&b).into_owned();
    let w = regex::escape(name);
    let arm =
        Regex::new(&format!(r"^{ANY_TYPE}\s+{w}\s*(?:when\b[^;]*?)?=>")).expect("an escaped name");
    (0..col.min(b.len())).any(|s| {
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
            return dart_header_binds(&head, name) && dart_reaches(&b, s, col, false);
        }
        if b[s] == b' ' || !(before.is_empty() || before.ends_with(['{', ';', ','])) {
            return false;
        }
        let t = line.get(s..).unwrap_or_default().trim_end();
        if (t.starts_with("case") && dart_header_binds(t, name)) || dart_local(t, name) {
            return dart_reaches(&b, s, col, true);
        }
        arm.is_match(t) && dart_reaches(&b, s, col, false)
    })
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
    let is_not = Regex::new(&format!(r"(?:^|[^\w$.]){w}\s+is!")).expect("an escaped name");
    let b = dart_masked(lines[at]);
    let own = String::from_utf8_lossy(&b).into_owned();
    let here = own.get(..col.min(own.len())).unwrap_or_default();
    if is.find_iter(here).any(|m| {
        let mut depth = 0i32;
        let open = (0..m.start()).rev().find(|&k| {
            match b[k] {
                b')' | b']' | b'}' => depth += 1,
                b'(' | b'[' | b'{' => depth -= 1,
                _ => {}
            }
            depth < 0
        });
        let from = open
            .filter(|&o| b[o] == b'(' && CONTROL.is_match(own.get(..o).unwrap_or_default()))
            .unwrap_or_else(|| m.start());
        dart_reaches(&b, from, col, false)
    }) {
        return true;
    }
    let literal = literal_lines(Kind::Dart, text);
    let mut depth = indent(lines[at]);
    for i in (0..at).rev() {
        let t = uncommented(Kind::Dart, lines[i]);
        let t = t.trim();
        let ind = indent(lines[i]);
        if literal[i] || t.is_empty() || ind > depth {
            continue;
        }
        if ind == depth {
            if t.starts_with("if") && is_not.is_match(t) {
                return true;
            }
            continue;
        }
        if is.is_match(t) {
            return true;
        }
        depth = ind;
        if depth == 0 || TYPE_BODY.is_match(lines[i]) {
            return false;
        }
    }
    false
}
