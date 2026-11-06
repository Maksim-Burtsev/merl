use regex::Regex;

use super::jvm::{opens_type, scope_of};
use super::jvm_rebound::blanked;
use super::{Kind, indent, literal_lines};

#[derive(Clone, Copy, PartialEq)]
enum Scope {
    Arm,
    Generator,
    Block,
}

fn open_after(rest: &str, scope: Scope) -> bool {
    let bytes = rest.as_bytes();
    let mut floor = 0i32;
    let mut depth = 0i32;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => {
                depth -= 1;
                if depth < floor {
                    if scope == Scope::Generator && floor == 0 {
                        floor = -1;
                    } else {
                        return false;
                    }
                }
            }
            b';' if scope != Scope::Block && depth == floor => return false,
            b'c' if scope != Scope::Block
                && depth == floor
                && rest.get(i..).is_some_and(|r| r.starts_with("case"))
                && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric())
                && !rest
                    .get(i + 4..)
                    .unwrap_or_default()
                    .starts_with(|c: char| c.is_alphanumeric() || c == '_') =>
            {
                return false;
            }
            _ => {}
        }
    }
    true
}

fn binders(code: &str, name: &str) -> Vec<(usize, Scope)> {
    static CASE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bcase\s+([^;]*?)=>").expect("the regex is a literal")
    });
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let arm = rule(format!(
        r"(?:^|[^\w.])(?:{n}\s*(?::[^=;(){{}}]*)?|\([^()]*[(,\s]{n}\b[^()]*\)|\({n}\b[^()]*\))\s*=>|\bdef\s+\w+[^=;]*?[(,]\s*(?:implicit\s+|using\s+)?{n}\s*:[^;]*?="
    ));
    let generator = rule(format!(r"(?:^|[^\w.]){n}\s*(?::[^<;]*)?<-"));
    let block = rule(format!(
        r"\b(?:val|var|given)\s+(?:[^=;]*?[^\w.])?{n}\b[^=;]*=|\bdef\s+{n}\b[^;(]*="
    ));
    let pattern = rule(format!(r"(?:^|[^\w.]){n}\b\s*(?:[^\w\s.(]|$)"));
    let mut out: Vec<(usize, Scope)> = arm.find_iter(code).map(|m| (m.end(), Scope::Arm)).collect();
    out.extend(
        generator
            .find_iter(code)
            .map(|m| (m.end(), Scope::Generator)),
    );
    out.extend(block.find_iter(code).map(|m| (m.end(), Scope::Block)));
    for c in CASE.captures_iter(code) {
        let pat = c[1].split(" if ").next().unwrap_or_default();
        if pattern.is_match(pat) {
            out.push((c.get(0).map_or(0, |m| m.end()), Scope::Arm));
        }
    }
    out
}

fn enumerator_head(lines: &[&str], at: usize) -> Option<usize> {
    let for_ = Regex::new(r"\bfor\b").expect("the regex is a literal");
    (at.saturating_sub(8)..=at)
        .rev()
        .take_while(|&k| indent(lines[k]) <= indent(lines[at]))
        .find(|&k| for_.is_match(&blanked(lines[k])))
}

fn continues(line: &str) -> bool {
    let t = line.trim_start();
    ["do", "yield", "}", ")"].iter().any(|p| {
        t.strip_prefix(p).is_some_and(|r| match *p {
            "}" | ")" => {
                let r = r.trim_start();
                r.starts_with("yield") || r.starts_with("do")
            }
            _ => !r.starts_with(|c: char| c.is_alphanumeric() || c == '_'),
        })
    })
}

fn covers(
    lines: &[&str],
    literal: &[bool],
    from: usize,
    at: usize,
    inside: impl Fn(&str) -> bool,
) -> bool {
    (from + 1..=at)
        .filter(|&k| !literal[k] && !blanked(lines[k]).trim().is_empty())
        .all(|k| inside(lines[k]))
}

fn unread_above(lines: &[&str], at: usize, name: &str) -> Option<usize> {
    let literal = literal_lines(Kind::Jvm, &lines.join("\n"));
    let n = regex::escape(name);
    let enumerator = Regex::new(&format!(r"^\s*{n}\s*=(?:[^=>]|$)")).ok()?;
    let ends_in_arrow = Regex::new(r"=>\s*$").ok()?;
    let open_for = Regex::new(r"\bfor\s*[({]?\s*$").ok()?;
    let depth = indent(lines[at]);
    for k in (at.saturating_sub(400)..at).rev() {
        if literal[k] {
            continue;
        }
        let code = blanked(lines[k]);
        let ind = indent(lines[k]);
        if ind == 0 && !code.trim().is_empty() {
            return None;
        }
        let generator = binders(&code, name)
            .iter()
            .any(|&(_, s)| s == Scope::Generator);
        let head = enumerator_head(lines, k)
            .filter(|&h| generator || (h < k && open_for.is_match(&blanked(lines[h]))));
        if (generator || enumerator.is_match(&code))
            && let Some(head) = head
        {
            let s = indent(lines[head]);
            if covers(lines, &literal, k, at, |l| indent(l) > s || continues(l)) {
                return Some(k + 1);
            }
        }
        for (end, scope) in binders(&code, name) {
            let local = scope_of(lines, k).is_some_and(|s| !opens_type(lines[s]));
            let found = match scope {
                Scope::Block => {
                    local
                        && open_after(code.get(end..).unwrap_or_default(), Scope::Block)
                        && covers(lines, &literal, k, at, |l| indent(l) >= ind)
                }
                Scope::Arm => {
                    ends_in_arrow.is_match(&code)
                        && open_after(code.get(end..).unwrap_or_default(), Scope::Arm)
                        && covers(lines, &literal, k, at, |l| indent(l) > ind)
                }
                Scope::Generator => false,
            };
            if found {
                return Some(k + 1);
            }
        }
        if opens_type(lines[k]) && ind < depth {
            return None;
        }
    }
    None
}

pub fn scala_rebound(text: &str, line1: usize, before: &str, name: &str, bound: &[usize]) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let code = blanked(before);
    let open: Vec<bool> = binders(&code, name)
        .into_iter()
        .map(|(end, scope)| open_after(code.get(end..).unwrap_or_default(), scope))
        .collect();
    if bound.contains(&line1) {
        return open.is_empty() || open.contains(&false);
    }
    if open.contains(&true) {
        return true;
    }
    unread_above(&lines, at, name).is_some_and(|l| bound.iter().all(|&b| b < l))
}
