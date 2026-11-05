use regex::Regex;

use super::swift::{swift_captures, swift_closure_params_bind};
use super::{Kind, close_of, code, split_top, swift_func_decl, swift_signature_line, uncommented};

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

static CLOSURE_HEAD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(
        r"^\s*(?:\[([^\]]*)\]\s*)?(?:@\w+\s+)?(\([^()]*\)|[\w\s,]*?)\s*(?:async\s+)?(?:throws\s+)?(?:->\s*[^{}]+?)?\s*\bin\s",
    )
    .expect("the regex is a literal")
});

fn open_braces(line: &str, at: usize) -> Vec<usize> {
    let mut open = Vec::new();
    for (i, c) in code(Kind::Swift, line).take_while(|&(i, _)| i < at) {
        match c {
            b'{' => open.push(i),
            b'}' => {
                open.pop();
            }
            _ => {}
        }
    }
    open
}

pub fn swift_closure_param(line: &str, name: &str, at: usize) -> bool {
    open_braces(line, at).iter().any(|&o| {
        line.get(o + 1..at)
            .and_then(|s| CLOSURE_HEAD.captures(s))
            .is_some_and(|c| {
                swift_closure_params_bind(&c[2], name)
                    || c.get(1).is_some_and(|l| swift_captures(l.as_str(), name))
            })
    })
}

pub fn swift_param_closure(lead: &str, name: &str) -> Option<(String, String)> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.)\]?!])([A-Za-z_][\w.]*)\.([A-Za-z_]\w*)\s*$")
            .expect("the regex is a literal")
    });
    let binds = |o: usize| {
        lead.get(o + 1..)
            .and_then(|s| CLOSURE_HEAD.captures(s))
            .is_some_and(|c| {
                let params = c[2].trim();
                (params.starts_with('(')
                    || params.split(',').all(|p| p.split_whitespace().count() == 1))
                    && swift_closure_params_bind(params, name)
            })
    };
    let o = open_braces(lead, lead.len())
        .into_iter()
        .rev()
        .find(|&o| binds(o))?;
    let before = uncommented(Kind::Swift, lead.get(..o)?);
    let c = CALL.captures(&before)?;
    Some((c[1].to_owned(), c[2].to_owned()))
}

pub fn swift_shadows(line: &str, name: &str) -> bool {
    let n = regex::escape(name);
    super::swift_may_bind(line, name)
        || CLOSURE_HEAD
            .captures(&uncommented(Kind::Swift, line))
            .is_some_and(|c| swift_closure_params_bind(&c[2], name))
        || Regex::new(&format!(
            r"(?:^|[(,])\s*(?:`?\w+`?\s+)?`?{n}`?\s*:\s*[@A-Z(\[]"
        ))
        .expect("an escaped name keeps the pattern valid")
        .is_match(&uncommented(Kind::Swift, line))
}

pub fn swift_generic_params(lines: &[&str], line1: usize) -> Option<Vec<String>> {
    static HEAD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:\bfunc\s+`?[^\s(<]+`?|\binit[?!]?|\bsubscript|\b(?:class|struct|enum|actor|typealias)\s+`?\w+`?)\s*<")
            .expect("the regex is a literal")
    });
    let first = uncommented(Kind::Swift, lines.get(line1.checked_sub(1)?)?);
    let Some(m) = HEAD.find(&first) else {
        return Some(Vec::new());
    };
    let mut decl = first.get(m.end()..)?.to_owned();
    let mut more = lines.iter().skip(line1).take(15);
    loop {
        let mut depth = 1;
        let close = decl.char_indices().find(|&(i, c)| {
            match c {
                '<' => depth += 1,
                '>' if !decl.get(..i).is_some_and(|s| s.ends_with('-')) => depth -= 1,
                _ => {}
            }
            depth == 0
        });
        if let Some((close, _)) = close {
            let params = split_top(Kind::Swift, decl.get(..close)?, b',');
            return Some(
                params
                    .iter()
                    .filter_map(|p| p.split(':').next())
                    .map(|p| p.trim().trim_start_matches("each ").to_owned())
                    .filter(|p| !p.is_empty())
                    .collect(),
            );
        }
        decl.push(' ');
        decl.push_str(&uncommented(Kind::Swift, more.next()?));
    }
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
pub fn swift_wrapped_params(text: &str, line1: usize) -> bool {
    let head = swift_signature_line(text, line1);
    let lines: Vec<&str> = text.lines().collect();
    head < line1
        && lines.get(head - 1).is_some_and(|h| swift_func_decl(h))
        && lines.get(head - 1..line1 - 1).is_some_and(|ls| {
            let head = ls.join("\n");
            let braces = |b| code(Kind::Swift, &head).filter(|&(_, c)| c == b).count();
            braces(b'{') == braces(b'}')
        })
}
