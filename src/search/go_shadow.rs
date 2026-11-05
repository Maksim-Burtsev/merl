use regex::Regex;

use super::{Binding, Kind, close_of, code, go_params, names};

pub(super) fn go_declares_then_uses(line: &str, name: &str) -> bool {
    static SHORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b((?:var\s+)?[A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)*)\s*(:=)?")
            .expect("the regex is a literal")
    });
    static FUNC: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\bfunc\s*\(").expect("the regex is a literal"));
    let Ok(member) = Regex::new(&format!(r"(?:^|[^\w.]){}\s*\.", regex::escape(name))) else {
        return false;
    };
    let mut b = vec![b' '; line.len()];
    for (i, c) in code(Kind::Go, line).take_while(|&(_, c)| c != 0) {
        b[i] = c;
    }
    let line = String::from_utf8_lossy(&b).into_owned();
    let used_after = |from: usize| line.get(from..).is_some_and(|rest| member.is_match(rest));
    let declared = SHORT.captures_iter(&line).any(|c| {
        let (Some(list), Some(end)) = (c.get(1), c.get(0)) else {
            return false;
        };
        let list = match list.as_str().strip_prefix("var") {
            Some(rest) if rest.starts_with(char::is_whitespace) => rest,
            _ if c.get(2).is_some() => list.as_str(),
            _ => return false,
        };
        names(list, name) && used_after(statement_end(&line, end.end()))
    });
    declared
        || FUNC.find_iter(&line).any(|m| {
            let mut found: Vec<Binding> = Vec::new();
            let Some(close) = close_of(Kind::Go, &line, m.end() - 1) else {
                return false;
            };
            let (Some(params), Some(results)) = (line.get(m.end()..close - 1), line.get(close..))
            else {
                return false;
            };
            go_params(params, &[], 0, name, &mut found);
            let results = results.trim_start();
            let after = line.len() - results.len();
            let inner = close_of(Kind::Go, results, 0)
                .filter(|_| results.starts_with('('))
                .and_then(|r| Some((r, results.get(1..r - 1)?)));
            let end = match inner {
                Some((r, inner)) => {
                    go_params(inner, &[], 0, name, &mut found);
                    after + r
                }
                None => close,
            };
            !found.is_empty() && used_after(end)
        })
}

fn statement_end(line: &str, from: usize) -> usize {
    let mut depth = 0i32;
    let mut last = b' ';
    for (i, c) in code(Kind::Go, line).skip_while(|&(i, _)| i < from) {
        match c {
            b'{' if depth == 0 && last == b' ' => return i,
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' | b';' | b':' if depth == 0 => return i,
            b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
        last = c;
    }
    line.len()
}
