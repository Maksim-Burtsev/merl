use std::sync::LazyLock;

use regex::Regex;

use super::rust::{binds, rust_rebinds, top_split, top_stop, without_type};
use super::{Kind, code};

pub fn rust_closure_holds(line: &str, name: &str, at: usize) -> bool {
    static CLOSURE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[(,={]|\bmove|\breturn|=>)\s*\|([^|]*)\|")
            .expect("the regex is a literal")
    });
    CLOSURE.captures_iter(line).any(|c| {
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
pub fn rust_rebound_before(line: &str, name: &str, at: usize) -> bool {
    static OPEN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\blet\s|=>|\b\w+!").expect("the regex is a literal"));
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
