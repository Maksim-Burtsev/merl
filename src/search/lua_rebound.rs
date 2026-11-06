use regex::Regex;

use super::{Kind, code, uncommented};

fn lua_code(s: &str) -> String {
    let mut out = vec![b' '; s.len()];
    for (i, c) in code(Kind::Lua, s) {
        if c == 0 {
            break;
        }
        out[i] = c;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn still_open(line: &str, words: &[(usize, &str)], from: usize) -> bool {
    let mut depth = 1usize;
    for &(at, w) in words.iter().filter(|(at, _)| *at >= from) {
        match w {
            "function" | "do" | "then" | "repeat" => depth += 1,
            "end" | "until" | "elseif" | "else" => {
                depth -= 1;
                if depth == 0 {
                    return w == "until" && line.get(at..).is_some_and(|r| !r.contains(';'));
                }
                if w == "else" {
                    depth += 1;
                }
            }
            _ => {}
        }
    }
    true
}

static WORD: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"[A-Za-z_]\w*").expect("the regex is a literal"));

pub fn lua_binds_through(line: &str, name: &str, assigned: bool) -> bool {
    if !line.contains(name) {
        return false;
    }
    let line = uncommented(Kind::Lua, line);
    let code = lua_code(&line);
    let mut depth = 0usize;
    for w in WORD.find_iter(&code) {
        match w.as_str() {
            "function" | "do" | "then" | "repeat" => depth += 1,
            "end" | "until" | "elseif" => match depth.checked_sub(1) {
                Some(d) => depth = d,
                None => return false,
            },
            _ => {}
        }
    }
    depth == 0
        && !code.trim_end().ends_with([',', '{', '(', '['])
        && lua_binds_before(&format!("{line};"), name, assigned)
}

pub fn lua_binds_before(before: &str, name: &str, assigned: bool) -> bool {
    static LOCAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\blocal\s+(?:function\s+(\w+)|([\w\s,<>]+?)\s*(=|;|$))")
            .expect("the regex is a literal")
    });
    static PARAMS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bfunction\b\s*([\w.]*(:)?\w*)\s*\(([^)]*)\)").expect("the regex is a literal")
    });
    static FOR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bfor\s+([\w\s,]+?)\s*(?:\bin\b|=)").expect("the regex is a literal")
    });
    static ASSIGN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|;|\b(?:do|then|else|repeat)\b)\s*([\w\s,]+?)\s*=[^=]")
            .expect("the regex is a literal")
    });
    let line = lua_code(before);
    let words: Vec<(usize, &str)> = WORD
        .find_iter(&line)
        .map(|m| (m.start(), m.as_str()))
        .collect();
    let lists = |list: &str| {
        list.split(',')
            .any(|n| n.split('<').next().is_some_and(|n| n.trim() == name))
    };
    let after_statement = |from: usize| {
        let mut depth = 0usize;
        for (i, c) in line.bytes().enumerate().skip(from) {
            match c {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth = depth.saturating_sub(1),
                b';' if depth == 0 => return Some(i + 1),
                _ => {}
            }
        }
        words
            .iter()
            .find(|(at, w)| *at >= from && *w == "until")
            .map(|(at, _)| at + 5)
    };
    let local = LOCAL.captures_iter(&line).any(|c| {
        let Some(start) = c.get(0).map(|m| m.start()) else {
            return false;
        };
        let visible = if let Some(f) = c.get(1) {
            (f.as_str() == name).then_some(f.end())
        } else {
            c.get(2).filter(|l| lists(l.as_str())).and_then(|l| {
                match c.get(3).map(|e| e.as_str()) {
                    Some("=") => after_statement(l.end()),
                    _ => Some(l.end()),
                }
            })
        };
        visible.is_some() && still_open(&line, &words, start + 1)
    });
    let params = PARAMS.captures_iter(&line).any(|c| {
        let method_self = name == "self" && c.get(2).is_some();
        (method_self || lists(&c[3]))
            && c.get(0).is_some_and(|m| still_open(&line, &words, m.end()))
    });
    let vars = FOR.captures_iter(&line).any(|c| {
        let Some(head) = c.get(0) else {
            return false;
        };
        lists(&c[1])
            && words
                .iter()
                .find(|(at, w)| *at >= head.end() && *w == "do")
                .is_some_and(|(at, _)| still_open(&line, &words, at + 2))
    });
    let reassigned = assigned
        && ASSIGN.captures_iter(&line).any(|c| {
            c.get(1).is_some_and(|l| {
                !l.as_str().trim_start().starts_with("local")
                    && lists(l.as_str())
                    && after_statement(l.end()).is_some()
                    && still_open(&line, &words, l.start())
            })
        });
    local || params || vars || reassigned
}
