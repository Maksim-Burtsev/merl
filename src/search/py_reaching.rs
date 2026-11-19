use regex::Regex;

use super::{Kind, code, indent, literal_lines, python_in_function_body, uncommented};

pub fn python_reaching(text: &str, line1: usize, name: &str, locals: &[usize]) -> Option<usize> {
    let from = locals.iter().copied().filter(|&n| n < line1).max()?;
    if locals.contains(&line1) || !python_in_function_body(text, line1) {
        return None;
    }
    let n = regex::escape(name);
    let outside = Regex::new(&format!(r"\b(?:global|nonlocal)\b[^\n]*\b{n}\b")).ok()?;
    if outside.is_match(text) {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(Kind::Python, text);
    let starts = statement_starts(&lines, &literal);
    let (a, at) = (from - 1, line1 - 1);
    let block = indent(lines.get(at)?);
    let end = (at + 1..lines.len())
        .find(|&k| starts[k])
        .unwrap_or(lines.len());
    let assigned = Regex::new(&format!(r"^{n}\s*(?::[^=]*)?=(?:[^=]|$)")).ok()?;
    let same_block = starts[a]
        && starts[at]
        && indent(lines[a]) == block
        && (a + 1..at).all(|k| !starts[k] || indent(lines[k]) >= block);
    let plain = assigned.is_match(uncommented(Kind::Python, lines[a]).trim());
    let deferred = Regex::new(r"\b(?:def|class|lambda|for|while)\b").ok()?;
    let runs_now = !lines[at..end]
        .iter()
        .any(|l| deferred.is_match(&uncommented(Kind::Python, l)));
    let next = (a + 1..at).find(|&k| starts[k]).unwrap_or(at);
    let wrapped = (next..at).any(|k| {
        !starts[k]
            && (literal.get(k).copied().unwrap_or(true)
                || !uncommented(Kind::Python, lines[k]).trim().is_empty())
    });
    let chained = lines[a..end]
        .iter()
        .any(|l| code(Kind::Python, &uncommented(Kind::Python, l)).any(|(_, c)| c == b';'));
    (same_block
        && plain
        && runs_now
        && !wrapped
        && !chained
        && locals.iter().all(|&l| l <= from || l > end)
        && !lines[a + 1..end].iter().any(|l| binds(l, name)))
    .then_some(from)
}

fn binds(line: &str, name: &str) -> bool {
    let n = regex::escape(name);
    let t = uncommented(Kind::Python, line);
    let Ok(rebinding) = Regex::new(&format!(
        r"(?:^|[^\w.]){n}\s*(?:[-+*/%&|^@:]|//|\*\*|<<|>>)?=(?:[^=]|$)|(?:^|[^\w.]){n}\b[^=\n]*[^=!<>]=(?:[^=]|$)|\bas\s+{n}\b|\b(?:del|import|for|lambda)\b[^\n]*\b{n}\b"
    )) else {
        return true;
    };
    rebinding.is_match(&t)
}

fn statement_starts(lines: &[&str], literal: &[bool]) -> Vec<bool> {
    let (mut depth, mut joined) = (0i32, false);
    lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let t = uncommented(Kind::Python, l);
            if literal.get(i).copied().unwrap_or(true) {
                joined = false;
                return false;
            }
            if t.trim().is_empty() {
                return false;
            }
            let start = depth == 0 && !joined;
            for (_, c) in code(Kind::Python, &t) {
                match c {
                    b'(' | b'[' | b'{' => depth += 1,
                    b')' | b']' | b'}' => depth -= 1,
                    _ => {}
                }
            }
            joined = t.trim_end().ends_with('\\');
            start
        })
        .collect()
}
