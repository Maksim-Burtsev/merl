use std::sync::LazyLock;

use regex::Regex;

use super::julia::{julia_code, top_level};

const OPENS: &[&str] = &[
    "function",
    "macro",
    "struct",
    "let",
    "begin",
    "quote",
    "do",
    "if",
    "for",
    "while",
    "try",
    "module",
    "baremodule",
];

fn lhs_binds(lhs: &str, name: &str) -> bool {
    let lhs = lhs.trim();
    let lhs = lhs
        .strip_prefix('(')
        .and_then(|l| l.strip_suffix(')'))
        .unwrap_or(lhs);
    lhs.split(',')
        .any(|v| v.split("::").next().is_some_and(|v| v.trim() == name))
}

pub(super) fn julia_let_binds(list: &str, name: &str) -> bool {
    top_level(list).into_iter().any(|part| {
        let lhs = part
            .match_indices('=')
            .find(|&(i, _)| {
                let rest = part.get(i + 1..).unwrap_or_default();
                !rest.starts_with(['=', '>'])
                    && !part
                        .get(..i)
                        .is_some_and(|b| b.ends_with(['=', '!', '<', '>']))
            })
            .and_then(|(i, _)| part.get(..i))
            .unwrap_or(part);
        lhs_binds(lhs, name)
    })
}

fn depths(code: &str) -> Vec<i32> {
    let mut d = 0;
    code.bytes()
        .map(|c| {
            let at = d;
            match c {
                b'(' | b'[' | b'{' => d += 1,
                b')' | b']' | b'}' => d -= 1,
                _ => {}
            }
            at
        })
        .collect()
}

fn still_open(code: &str, depth: &[i32], from: usize, mut blocks: i32) -> bool {
    static WORD: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[A-Za-z_][\w!]*").expect("the regex is a literal"));
    let base = depth.get(from).copied().unwrap_or(0);
    if depth
        .get(from..)
        .is_some_and(|d| d.iter().any(|&d| d < base))
    {
        return false;
    }
    for m in WORD.find_iter(code.get(from..).unwrap_or_default()) {
        let at = from + m.start();
        let prev = code.get(..at).and_then(|b| b.chars().next_back());
        if depth.get(at) != Some(&base) || prev.is_some_and(|c| matches!(c, '.' | ':' | '@')) {
            continue;
        }
        if OPENS.contains(&m.as_str()) {
            blocks += 1;
        } else if m.as_str() == "end" {
            blocks -= 1;
            if blocks <= 0 {
                return false;
            }
        }
    }
    true
}

pub fn julia_binds_before(before: &str, name: &str) -> bool {
    static LET: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:^|[^\w!.:@])let\b").expect("the regex is a literal"));
    static ASSIGN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:local\s+)?([\w!\s,(){}:]*?)\s*=(?:[^=>]|$)")
            .expect("the regex is a literal")
    });
    let code = julia_code(before);
    let depth = depths(&code);
    let semi_after = |from: usize, d: i32| {
        (from..code.len()).find(|&i| code.as_bytes()[i] == b';' && depth[i] == d)
    };
    let lets = LET.find_iter(&code).any(|m| {
        let at = m.end() - 3;
        semi_after(at, depth[at]).is_some_and(|semi| {
            code.get(m.end()..semi)
                .is_some_and(|list| julia_let_binds(list, name))
                && still_open(&code, &depth, at, 0)
        })
    });
    let starts = std::iter::once(0).chain(
        (0..code.len())
            .filter(|&i| code.as_bytes()[i] == b';' && depth[i] == 0)
            .map(|i| i + 1),
    );
    lets || starts.into_iter().any(|s| {
        semi_after(s, 0).is_some_and(|e| {
            code.get(s..e)
                .and_then(|st| ASSIGN.captures(st))
                .is_some_and(|c| lhs_binds(&c[1], name))
                && still_open(&code, &depth, s, 1)
        })
    })
}
