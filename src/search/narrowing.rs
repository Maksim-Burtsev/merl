use regex::Regex;

use super::{Kind, indent, literal_lines, uncommented};

pub enum Checked {
    To(String, usize),
    Unreadable,
}

struct Shapes {
    mention: Regex,
    negated: Regex,
    header: Regex,
    negated_header: Regex,
    guard: Regex,
    otherwise: Regex,
    exit: Regex,
    function: Regex,
    class: Regex,
    named: Regex,
}

fn shapes(kind: Kind, name: &str) -> Shapes {
    let n = regex::escape(name);
    let re = |p: &str| {
        Regex::new(&p.replace("{n}", &n)).expect("an escaped name keeps the pattern valid")
    };
    if kind == Kind::Python {
        let not = r"if\s+not\s+isinstance\(\s*{n}\s*,\s*(?P<t>[\w.]+)\s*\)\s*:";
        return Shapes {
            mention: re(r"\bisinstance\(\s*{n}\s*,"),
            negated: re(r"\bnot\s+isinstance\(\s*{n}\s*,"),
            header: re(r"^\s*(?:el)?if\s+isinstance\(\s*{n}\s*,\s*(?P<t>[\w.]+)\s*\)\s*:\s*$"),
            negated_header: re(&format!(r"^\s*(?:el)?{not}\s*$")),
            guard: re(&format!(
                r"^\s*(?:{not}\s*(?:(?:return|raise|continue|break)\b.*|(?P<open>))|assert\s+isinstance\(\s*{{n}}\s*,\s*(?P<a>[\w.]+)\s*\))\s*$"
            )),
            otherwise: re(r"^\s*else\s*:\s*(?:(?:return|raise|continue|break)\b.*|(?P<open>))$"),
            exit: re(r"^\s*(?:return|raise|continue|break)\b"),
            function: re(r"^\s*(?:async\s+)?def\s|\blambda\b"),
            class: re(r"^\s*class\s"),
            named: re(r"\b{n}\b"),
        };
    }
    let not = r"if\s*\(\s*!\s*\(\s*{n}\s+instanceof\s+(?P<t>[\w$.]+)\s*\)\s*\)\s*";
    let exit = r"(?:return|throw|continue|break)\b[^;{}]*;?";
    Shapes {
        mention: re(r"(?:^|[^\w$.]){n}\s+instanceof\b"),
        negated: re(r"!\s*\(\s*{n}\s+instanceof\b"),
        header: re(
            r"^\s*(?:\}\s*else\s+)?if\s*\(\s*{n}\s+instanceof\s+(?P<t>[\w$.]+)\s*\)\s*\{\s*$",
        ),
        negated_header: re(&format!(r"^\s*(?:\}}\s*else\s+)?{not}\{{\s*$")),
        guard: re(&format!(
            r"^\s*{not}(?:{exit}|\{{\s*{exit}\s*\}}|\{{(?P<open>))\s*$"
        )),
        otherwise: re(&format!(
            r"^\s*\}}\s*else\s*(?:{exit}|\{{\s*{exit}\s*\}}|\{{(?P<open>))\s*$"
        )),
        exit: re(&format!(r"^\s*{exit}\s*$")),
        function: re(r"\bfunction\b|=>"),
        class: re(r"^\s*(?:export\s+)?(?:default\s+)?(?:abstract\s+)?class\s"),
        named: re(r"(?:^|[^\w$.]){n}(?:[^\w$]|$)"),
    }
}

pub fn checked_type(kind: Kind, text: &str, line1: usize, name: &str) -> Option<Checked> {
    if !text.contains(if kind == Kind::Python {
        "isinstance"
    } else {
        "instanceof"
    }) {
        return None;
    }
    let s = shapes(kind, name);
    if !s.mention.is_match(text) {
        return None;
    }
    let raw: Vec<&str> = text.lines().collect();
    let literal = literal_lines(kind, text);
    let line = |j: usize| uncommented(kind, raw[j]);
    let filled = |j: &usize| literal.get(*j) != Some(&true) && !raw[*j].trim().is_empty();
    let at = line1.checked_sub(1).filter(|j| *j < raw.len())?;
    let here = line(at);
    if s.mention.is_match(&here) {
        return Some(Checked::Unreadable);
    }
    let mut crossed = s.function.is_match(&here);
    let found = |t: &str, k: usize, crossed: bool| {
        Some(if crossed {
            Checked::Unreadable
        } else {
            Checked::To(t.to_owned(), k + 1)
        })
    };
    let mut within = indent(&here);
    let logical = |end: usize| logical(end, &line, &filled);
    let mut next = at;
    while let Some(end) = (0..next).rev().find(filled) {
        let (k, l) = logical(end);
        next = k;
        let ind = indent(&l);
        if ind > within {
            continue;
        }
        if ind == within {
            if let Some(c) = s.guard.captures(&l) {
                let t = c.name("t").or_else(|| c.name("a"))?.as_str();
                if c.name("open").is_none() || closed_by_exit(kind, &raw, end, &s, &filled) {
                    return found(t, end, crossed);
                }
                return Some(Checked::Unreadable);
            }
            if let Some(c) = s.otherwise.captures(&l) {
                let Some(top) = (0..k)
                    .rev()
                    .filter(filled)
                    .find(|j| indent(&line(*j)) <= ind)
                else {
                    continue;
                };
                let (_, h) = logical(top);
                let exact =
                    c.name("open").is_none() || closed_by_exit(kind, &raw, end, &s, &filled);
                let chained = h.trim_start().starts_with('}') || h.trim_start().starts_with("elif");
                if let Some(t) = s.header.captures(&h).and_then(|c| c.name("t"))
                    && exact
                    && !chained
                    && indent(&h) == ind
                {
                    return found(t.as_str(), top, crossed);
                }
                let exits = (end + 1..raw.len())
                    .filter(filled)
                    .take_while(|j| indent(&line(*j)) > ind)
                    .any(|j| s.exit.is_match(&line(j)));
                if s.mention.is_match(&h) && (exact || exits) {
                    return Some(Checked::Unreadable);
                }
            }
            if s.negated.is_match(&l) {
                return Some(Checked::Unreadable);
            }
            continue;
        }
        within = ind;
        if s.class.is_match(&l) {
            return None;
        }
        if s.function.is_match(&l) {
            if s.named.is_match(&l) {
                return None;
            }
            crossed = true;
        }
        if let Some(c) = s.header.captures(&l) {
            return found(c.name("t")?.as_str(), end, crossed);
        }
        if s.mention.is_match(&l) && !s.negated_header.is_match(&l) {
            return Some(Checked::Unreadable);
        }
    }
    None
}

fn logical(
    end: usize,
    line: &dyn Fn(usize) -> String,
    filled: &dyn Fn(&usize) -> bool,
) -> (usize, String) {
    static QUOTED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#""(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|`[^`]*`"#)
            .expect("the quoted-text pattern is valid")
    });
    let balance = |j: usize| {
        let l = line(j);
        let bare = QUOTED.replace_all(&l, "");
        bare.matches('(').count().cast_signed() - bare.matches(')').count().cast_signed()
    };
    let mut open = balance(end);
    let mut start = end;
    for j in (end.saturating_sub(30)..end).rev().filter(filled) {
        if open >= 0 {
            break;
        }
        open += balance(j);
        if open >= 0 {
            start = j;
        }
    }
    let joined = (start..=end).filter(filled).map(line).collect::<Vec<_>>();
    (start, joined.join(" "))
}

fn closed_by_exit(
    kind: Kind,
    raw: &[&str],
    k: usize,
    s: &Shapes,
    filled: &dyn Fn(&usize) -> bool,
) -> bool {
    let mut body = (k + 1..raw.len()).filter(filled);
    let (Some(exit), Some(after)) = (body.next(), body.next()) else {
        return false;
    };
    let ind = indent(raw[k]);
    let after = uncommented(kind, raw[after]);
    s.exit.is_match(&uncommented(kind, raw[exit]))
        && indent(raw[exit]) > ind
        && if kind == Kind::Python {
            indent(&after) <= ind
        } else {
            indent(&after) == ind && after.trim() == "}"
        }
}
