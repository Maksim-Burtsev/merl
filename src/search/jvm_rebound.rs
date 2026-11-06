use regex::Regex;

use super::{Kind, code, indent, uncommented};

fn blanked(s: &str) -> String {
    let mut bytes = vec![b' '; s.len()];
    for (i, c) in code(Kind::Jvm, s) {
        bytes[i] = if c.is_ascii() { c } else { b' ' };
    }
    String::from_utf8(bytes).unwrap_or_default()
}

pub fn jvm_rebound_before(before: &str, name: &str) -> bool {
    let n = regex::escape(name);
    let ret = super::jvm_return_type!();
    let Ok(declared) = Regex::new(&format!(
        r"[;{{]\s*(?:final\s+)?(?:val|var|{ret})\s+{n}\s*[=;:,]|[(,]\s*(?:final\s+)?{ret}\s+{n}\s*[,)]|[(,]\s*{n}\s*:\s*[A-Z]"
    )) else {
        return false;
    };
    let line = blanked(before);
    declared.find_iter(&line).any(|m| {
        let mut depth = 0i32;
        line.get(m.end()..).is_some_and(|rest| {
            rest.bytes().all(|b| {
                match b {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                depth >= 0
            })
        })
    })
}

pub(super) fn negated_pattern_exits(lines: &[&str], at: usize, name: &str) -> bool {
    static EXIT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:return|throw|continue|break)\b").expect("the regex is a literal")
    });
    let Ok(pattern) = Regex::new(&format!(
        r"!\s*\(.*\binstanceof\s+(?:final\s+)?[\w.]+(?:<[^>]*>)?\s+{}\b",
        regex::escape(name)
    )) else {
        return false;
    };
    if !pattern.is_match(&blanked(&uncommented(Kind::Jvm, lines[at]))) {
        return false;
    }
    let depth = indent(lines[at]);
    std::iter::once(lines[at])
        .chain(
            lines[at + 1..]
                .iter()
                .copied()
                .take_while(|l| l.trim().is_empty() || indent(l) > depth),
        )
        .any(|l| EXIT.is_match(&blanked(&uncommented(Kind::Jvm, l))))
}
