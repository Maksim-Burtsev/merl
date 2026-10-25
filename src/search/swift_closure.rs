use regex::Regex;

use super::{Kind, close_of, code, split_top, uncommented};

pub fn swift_array_call(e: &str) -> Option<(Vec<String>, bool, String)> {
    static TAIL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(\.compactMap\(\\\.self\))?\.([A-Za-z_]\w*)\(\)$")
            .expect("the regex is a literal")
    });
    let e = e.trim();
    let end = close_of(Kind::Swift, e, 0).filter(|_| e.starts_with('['))?;
    let c = TAIL.captures(e.get(end..)?)?;
    let items = split_top(Kind::Swift, e.get(1..end - 1)?, b',');
    let items: Vec<String> = items.iter().map(|i| i.trim().to_owned()).collect();
    (!items
        .iter()
        .any(|i| i.is_empty() || split_top(Kind::Swift, i, b':').len() > 1))
    .then(|| (items, c.get(1).is_some(), c[2].to_owned()))
}

pub fn swift_bare_closure(line: &str, name: &str) -> Option<(String, usize, usize)> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"(?:^|[^\w.)\]?!])(?:self\.)?([a-z_]\w*)\s*\{\s*(?:\[[^\]]*\]\s*)?([\w\s,`]+?)\s+in\b",
        )
        .expect("the regex is a literal")
    });
    static IN: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\bin\b").expect("the regex is a literal"));
    let code = uncommented(Kind::Swift, line);
    let c = CALL
        .captures(&code)
        .filter(|_| IN.find_iter(&code).count() == 1)?;
    let params: Vec<&str> = c[2]
        .split(',')
        .map(|p| p.trim().trim_matches('`'))
        .collect();
    let at = params.iter().position(|p| *p == name)?;
    Some((c[1].to_owned(), at, params.len()))
}

pub fn swift_params(decl: &str) -> Option<Vec<String>> {
    static FUNC: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bfunc\s+`?\w+`?\s*(?:<[^>]*>)?\s*\(").expect("the regex is a literal")
    });
    let code = uncommented(Kind::Swift, decl);
    let open = FUNC.find(&code)?.end() - 1;
    let inner = code.get(open + 1..close_of(Kind::Swift, &code, open)? - 1)?;
    Some(
        split_top(Kind::Swift, inner, b',')
            .into_iter()
            .map(|p| {
                let ty = split_top(Kind::Swift, p, b'=')[0]
                    .split_once(':')
                    .map(|(_, t)| t.trim());
                ty.unwrap_or_default().to_owned()
            })
            .collect(),
    )
}

pub fn swift_constrained_generic(decl: &str, ty: &str) -> bool {
    static GENERICS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bfunc\s+`?\w+`?\s*<([^>]*)>").expect("the regex is a literal")
    });
    GENERICS.captures(decl).is_some_and(|c| {
        c[1].split(',').any(|g| {
            g.split_once(':').is_some_and(|(n, bound)| {
                n.trim() == ty && !matches!(bound.trim(), "" | "Sendable" | "Any")
            })
        })
    })
}

pub fn swift_function_inputs(ty: &str) -> Option<Vec<String>> {
    let t = ty
        .split_whitespace()
        .skip_while(|w| w.starts_with('@'))
        .collect::<Vec<_>>()
        .join(" ");
    let end = close_of(Kind::Swift, &t, 0).filter(|_| t.starts_with('('))?;
    let rest = t.get(end..)?.trim_start();
    let rest = rest.strip_prefix("async").map_or(rest, str::trim_start);
    let rest = rest.strip_prefix("throws").map_or(rest, str::trim_start);
    if !rest.starts_with("->") {
        return None;
    }
    let inner = t.get(1..end - 1)?.trim();
    if inner.is_empty() {
        return Some(Vec::new());
    }
    Some(
        split_top(Kind::Swift, inner, b',')
            .into_iter()
            .map(|i| {
                let parts = split_top(Kind::Swift, i, b':');
                parts.last().copied().unwrap_or(i).trim().to_owned()
            })
            .collect(),
    )
}

pub fn swift_bound_before(lead: &str, name: &str) -> Option<String> {
    let n = regex::escape(name);
    let bound = Regex::new(&format!(
        r"^(?:(?:\}}\s*)?(?:else\s+)?(?:if|guard|while)\s+)?(?:let|var)\s+`?{n}`?\s*=\s*(.+)$"
    ))
    .expect("an escaped name keeps the pattern valid");
    let used = Regex::new(&format!(r"(?:^|[^\w.])`?{n}\b"))
        .expect("an escaped name keeps the pattern valid");
    let code = uncommented(Kind::Swift, lead);
    let parts = split_top(Kind::Swift, &code, b',');
    let (_, before) = parts.split_last()?;
    let last = before
        .iter()
        .rev()
        .find(|p| super::swift_may_bind(p, name))?;
    let value = bound.captures(last.trim())?[1].trim().to_owned();
    (!used.is_match(&value)).then_some(value)
}

pub fn swift_subscript_receiver(before: &str) -> Option<String> {
    static HEAD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.)\]?!])([a-z_]\w*(?:\.[A-Za-z_]\w*)*)$")
            .expect("the regex is a literal")
    });
    let rest = before.strip_suffix('.')?;
    let open = code(Kind::Swift, rest)
        .filter(|(_, c)| *c == b'[')
        .map(|(i, _)| i)
        .find(|&i| close_of(Kind::Swift, rest, i) == Some(rest.len()))?;
    let args = rest.get(open + 1..rest.len() - 1)?;
    if args.trim().is_empty() || args.contains([':', '\\']) {
        return None;
    }
    Some(HEAD.captures(rest.get(..open)?)?[1].to_owned())
}

pub fn swift_before_else(rest: &str) -> &str {
    let mut depth = 0i32;
    for (i, c) in code(Kind::Swift, rest) {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'e' if depth == 0 => {
                if let (Some(head), Some(tail)) = (rest.get(..i), rest.get(i..))
                    && head.ends_with(char::is_whitespace)
                    && tail
                        .strip_prefix("else")
                        .is_some_and(|t| t.trim_start().starts_with('{'))
                {
                    return head.trim_end();
                }
            }
            _ => {}
        }
    }
    rest
}
