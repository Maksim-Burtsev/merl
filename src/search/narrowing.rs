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
    for k in (0..at).rev().filter(filled) {
        let l = line(k);
        let ind = indent(&l);
        if ind > within {
            continue;
        }
        if ind == within {
            if let Some(c) = s.guard.captures(&l) {
                let t = c.name("t").or_else(|| c.name("a"))?.as_str();
                if c.name("open").is_none() || closed_by_exit(kind, &raw, k, &s, &filled) {
                    return found(t, k, crossed);
                }
                return Some(Checked::Unreadable);
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
            return found(c.name("t")?.as_str(), k, crossed);
        }
        if s.mention.is_match(&l) && !s.negated_header.is_match(&l) {
            return Some(Checked::Unreadable);
        }
    }
    None
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
