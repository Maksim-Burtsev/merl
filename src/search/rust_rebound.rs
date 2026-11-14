use std::sync::LazyLock;

use regex::Regex;

use super::rust::{
    binds, rust_line_binds, rust_rebinds, top_split, top_stop, type_at, without_type,
};
use super::{Kind, code};

pub fn rust_closure_holds<'a>(line: &'a str, name: &str, at: usize) -> Option<&'a str> {
    static CLOSURE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[(,={]|\bmove|\breturn|=>)\s*\|([^|]*)\|")
            .expect("the regex is a literal")
    });
    let holds = CLOSURE.captures_iter(line).find(|c| {
        let Some(body) = c.get(0).and_then(|m| line.get(m.end()..at)) else {
            return false;
        };
        let mut depth = 0i32;
        let inside = code(Kind::Rust, body).all(|(_, b)| {
            match b {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                _ => {}
            }
            depth > 0 || (depth == 0 && !matches!(b, b',' | b';'))
        });
        inside
            && !rust_rebinds(body, name)
            && top_split(&c[1])
                .iter()
                .any(|p| binds(without_type(p), name))
    });
    holds.and_then(|c| c.get(1)).map(|m| m.as_str())
}
pub fn rust_closure_param_type(header: &str, name: &str) -> Option<String> {
    top_split(header).into_iter().find_map(|p| {
        let colon = top_stop(p, b":")?;
        let pat = p.get(..colon)?.trim();
        let pat = pat.strip_prefix("mut ").unwrap_or(pat).trim();
        (pat == name).then(|| type_at(p.get(colon + 1..).unwrap_or_default()).to_owned())
    })
}
pub(super) fn macro_binds(call: &str, name: &str) -> bool {
    static CALL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(\w+)!\s*[(\[{]\s*(?:mut\s+)?(\w+)\s*[,)=;\]}]")
            .expect("the regex is a literal")
    });
    const STD: &[&str] = &[
        "assert",
        "assert_eq",
        "assert_ne",
        "debug_assert",
        "debug_assert_eq",
        "debug_assert_ne",
        "dbg",
        "eprint",
        "eprintln",
        "format",
        "matches",
        "panic",
        "print",
        "println",
        "todo",
        "unimplemented",
        "unreachable",
        "vec",
        "write",
        "writeln",
    ];
    CALL.captures(call)
        .is_some_and(|c| &c[2] == name && !STD.contains(&&c[1]))
}
fn without_block_comments(line: &str) -> String {
    let b = line.as_bytes();
    let mut out = b.to_vec();
    let mut past = 0;
    for (i, c) in code(Kind::Rust, line) {
        if i < past || c != b'/' || b.get(i + 1) != Some(&b'*') {
            continue;
        }
        past = line
            .get(i + 2..)
            .and_then(|r| r.find("*/"))
            .map_or(b.len(), |e| i + e + 4);
        out[i..past].fill(b' ');
    }
    String::from_utf8(out).unwrap_or_else(|_| line.to_owned())
}
pub fn rust_rebound_before(line: &str, name: &str, at: usize) -> bool {
    static OPEN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\blet\s|=>|\b\w+!").expect("the regex is a literal"));
    let line = without_block_comments(line);
    let Some(head) = line.get(..at) else {
        return false;
    };
    let code: Vec<(usize, u8)> = code(Kind::Rust, head).collect();
    OPEN.find_iter(head).any(|m| {
        if !code.iter().any(|&(i, _)| i == m.start()) {
            return false;
        }
        let (mut depth, mut ended, mut arm_open) = (0i32, false, true);
        for &(_, b) in code.iter().filter(|&&(i, _)| i >= m.start()) {
            match b {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                b';' if depth == 0 => (ended, arm_open) = (true, false),
                b',' if depth == 0 => arm_open = false,
                _ => {}
            }
            if depth < 0 {
                return false;
            }
        }
        let before = head.get(..m.start()).unwrap_or_default();
        let after = head.get(m.end()..).unwrap_or_default();
        match m.as_str() {
            "=>" => {
                let mut depth = 0i32;
                let Some(&(from, _)) =
                    code.iter()
                        .rev()
                        .filter(|&&(i, _)| i < m.start())
                        .find(|&&(_, b)| {
                            match b {
                                b')' | b']' | b'}' => depth += 1,
                                b'(' | b'[' | b'{' => depth -= 1,
                                _ => {}
                            }
                            depth < 0 || (depth == 0 && b == b',')
                        })
                else {
                    return false;
                };
                let pat = before.get(from + 1..).unwrap_or_default();
                let pat = pat.split(" if ").next().unwrap_or(pat);
                arm_open && binds(pat, name)
            }
            s if s.ends_with('!') => {
                ended && macro_binds(line.get(m.start()..).unwrap_or_default(), name)
            }
            _ => {
                let header = ["if", "while", "&&"]
                    .iter()
                    .any(|k| before.trim_end().ends_with(k));
                let pat = top_stop(after, b"=:;")
                    .and_then(|i| after.get(..i))
                    .unwrap_or(after);
                ended && !header && binds(pat, name)
            }
        }
    })
}
pub enum IfLetScope<'a> {
    Then(&'a str),
    Outside,
    Hidden,
}
fn may_rebind(s: &str, name: &str) -> bool {
    rust_line_binds(s, name) || rust_rebound_before(s, name, s.len())
}
pub fn rust_if_let_branch<'a>(line: &'a str, name: &str, at: usize) -> Option<IfLetScope<'a>> {
    let code: Vec<(usize, u8)> = code(Kind::Rust, line).collect();
    let start = line
        .get(..at)?
        .rmatch_indices("if let ")
        .map(|(i, _)| i)
        .find(|i| code.iter().any(|&(j, _)| j == *i))?;
    let (mut depth, mut open, mut blocks) = (0i32, 0, Vec::new());
    for &(i, b) in code.iter().filter(|&&(i, _)| i >= start) {
        match b {
            b'{' if depth == 0 => (depth, open) = (1, i),
            b'(' | b'[' | b'{' => depth += 1,
            b'}' if depth == 1 => {
                depth = 0;
                blocks.push((open, i));
            }
            b')' | b']' | b'}' => depth -= 1,
            b';' | b',' if depth == 0 => break,
            _ => {}
        }
        if depth < 0 || blocks.len() == 2 {
            break;
        }
    }
    let &(then_open, then_close) = blocks.first()?;
    let header = line.get(start..=then_open)?;
    let pat = header.get(7..)?.split(" = ").next()?;
    if !binds(pat, name) {
        return None;
    }
    let from = if then_open < at && at < then_close {
        then_open
    } else if then_close < at {
        then_close
    } else {
        return None;
    };
    Some(if may_rebind(line.get(from..at)?, name) {
        IfLetScope::Hidden
    } else if from == then_open {
        IfLetScope::Then(header)
    } else {
        IfLetScope::Outside
    })
}
