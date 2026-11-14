use regex::Regex;

use super::jvm::{opens_type, scope_of};
use super::jvm_rebound::blanked;
use super::scala_mods;
use super::{Kind, indent, path_literal_lines};
use std::path::Path;

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

fn param_list_end(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    (open..bytes.len()).find(|&j| {
        match bytes.get(j) {
            Some(b'(' | b'[' | b'{') => depth += 1,
            Some(b')' | b']' | b'}') => depth -= 1,
            _ => {}
        }
        depth == 0
    })
}

fn def_bodies(code: &str, name: &str) -> Vec<usize> {
    static DEF: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bdef\s+\w+\s*(?:\[[^\]]*\]\s*)?\(").expect("the regex is a literal")
    });
    let param = Regex::new(&format!(
        r"[(,]\s*(?:implicit\s+|using\s+)?{}\s*:",
        regex::escape(name)
    ))
    .expect("an escaped name keeps the pattern valid");
    let bytes = code.as_bytes();
    let mut out = Vec::new();
    for m in DEF.find_iter(code) {
        let mut i = m.end() - 1;
        let mut binds = false;
        while bytes.get(i) == Some(&b'(') {
            let Some(close) = param_list_end(bytes, i) else {
                break;
            };
            binds |= param.is_match(code.get(i..=close).unwrap_or_default());
            i = close + 1;
            while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
                i += 1;
            }
        }
        let rest = code.get(i..).unwrap_or_default();
        let rest = rest.split(';').next().unwrap_or_default();
        let body = if rest.starts_with('{') {
            Some(i + 1)
        } else {
            rest.match_indices('=')
                .find(|&(k, _)| !rest.get(k + 1..).is_some_and(|r| r.starts_with(['>', '='])))
                .map(|(k, _)| i + k + 1)
        };
        out.extend(body.filter(|_| binds));
    }
    out
}

fn binders(code: &str, name: &str) -> Vec<(usize, Scope)> {
    static CASE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bcase\s+([^;]*?)=>").expect("the regex is a literal")
    });
    static GUARD: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\sif\b").expect("the regex is a literal"));
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let arm = rule(format!(
        r"(?:^|[^\w.])(?:{n}\s*(?::[^=;(){{}},]*)?|\([^()]*[(,\s]{n}\b[^()]*\)|\({n}\b[^()]*\))\s*=>"
    ));
    let generator = rule(format!(
        r"(?:(?:^|[^\w.]){n}\s*(?::[^<;,()]*)?|\((?:[^()<;=]*[,\s])?{n}\b[^()<;=]*\))\s*<-"
    ));
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
    out.extend(def_bodies(code, name).into_iter().map(|e| (e, Scope::Arm)));
    for c in CASE.captures_iter(code) {
        let pat = GUARD.split(&c[1]).next().unwrap_or_default();
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

fn enumerators_open(lines: &[&str], head: usize, at: usize) -> bool {
    let mut depth = 0i32;
    for (k, line) in lines.iter().enumerate().take(at).skip(head) {
        let code = blanked(line);
        let from = if k == head {
            code.find("for").unwrap_or_default()
        } else {
            0
        };
        let t = code.trim_start();
        if k > head && depth == 0 && (t.starts_with("do") || t.starts_with("yield")) {
            return false;
        }
        for b in code.bytes().skip(from) {
            match b {
                b'(' | b'{' | b'[' => depth += 1,
                b')' | b'}' | b']' => {
                    depth -= 1;
                    if depth == 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
    }
    true
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

fn body_after_close(line: &str) -> bool {
    let code = blanked(line);
    code.trim_start()
        .strip_prefix(['}', ')'])
        .map(str::trim)
        .is_some_and(|r| !r.is_empty() && !r.starts_with([';', ',', '.', ')', '}']))
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

pub(super) fn scala_generator_line1(
    path: &Path,
    lines: &[&str],
    at: usize,
    col: usize,
    name: &str,
) -> Option<usize> {
    let code = blanked(lines.get(at)?);
    let before = code.get(..col)?;
    let on_line = binders(before, name)
        .into_iter()
        .chain(value_enumerators(before, name))
        .any(|(end, scope)| {
            scope == Scope::Generator
                && open_after(before.get(end..).unwrap_or_default(), Scope::Generator)
        });
    if on_line {
        return Some(at + 1);
    }
    unread_above(path, lines, at, name, true)
}

fn value_enumerators(code: &str, name: &str) -> Vec<(usize, Scope)> {
    Regex::new(&format!(
        r"\bfor\b[^;]*<-[^;]*(?:;[^;]*)*;\s*{}\s*=(?:[^=>]|$)",
        regex::escape(name)
    ))
    .map(|re| {
        re.find_iter(code)
            .map(|m| (m.end().saturating_sub(1), Scope::Generator))
            .collect()
    })
    .unwrap_or_default()
}

fn unread_above(
    path: &Path,
    lines: &[&str],
    at: usize,
    name: &str,
    generators: bool,
) -> Option<usize> {
    let literal = path_literal_lines(Kind::Jvm, path, &lines.join("\n"));
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
        let head = enumerator_head(lines, k).filter(|&h| {
            generator
                || (h < k && open_for.is_match(&blanked(lines[h])) && enumerators_open(lines, h, k))
        });
        if (generator || enumerator.is_match(&code))
            && let Some(head) = head
        {
            let s = indent(lines[head]);
            if covers(lines, &literal, k, at, |l| {
                indent(l) > s || continues(l) || (indent(l) == s && body_after_close(l))
            }) {
                return Some(k + 1);
            }
        }
        for (end, scope) in binders(&code, name).into_iter().filter(|_| !generators) {
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

pub fn scala_rebound(
    path: &Path,
    text: &str,
    line1: usize,
    before: &str,
    name: &str,
    bound: &[usize],
) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let code = blanked(before);
    let mut found = binders(&code, name);
    found.sort_by_key(|&(end, _)| end);
    found.dedup_by_key(|&mut (end, _)| end);
    let open: Vec<bool> = found
        .into_iter()
        .map(|(end, scope)| open_after(code.get(end..).unwrap_or_default(), scope))
        .collect();
    if bound.contains(&line1) {
        return open.iter().rposition(|&o| o) != Some(0);
    }
    if open.contains(&true) {
        return true;
    }
    unread_above(path, &lines, at, name, false).is_some_and(|l| bound.iter().all(|&b| b < l))
}

pub fn scala_alias_owner(text: &str, line1: usize, alias: &str, bound: &[usize]) -> Option<String> {
    let declares = Regex::new(&format!(
        r"^\s*{}\s*(?::[^=]*)?=>\s*(?://.*)?$",
        regex::escape(alias)
    ))
    .ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let mut at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    loop {
        at = scope_of(&lines, at)?;
        let opened = lines[at].rsplit_once('{').map(|(_, r)| r);
        let declared1 = if opened.is_some_and(|r| declares.is_match(r)) {
            Some(at + 1)
        } else {
            lines
                .get(at + 1)
                .is_some_and(|l| declares.is_match(l))
                .then_some(at + 2)
        };
        if let Some(declared1) = declared1 {
            bound.iter().all(|&b| b <= declared1).then_some(())?;
            let name = super::jvm_type_name(lines[at])?;
            return Some(super::qualified(Kind::Jvm, text, at + 1, &name).unwrap_or(name));
        }
    }
}

pub fn scala_implicit_receiver(text: &str, line: usize, name: &str) -> Option<String> {
    static CLASS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:private|protected)(?:\[\w+\])?\s+|final\s+)*implicit\s+(?:final\s+)?class\s+\w+\s*(?:\[([\w\s,+-]*)\]\s*)?\(\s*(?:val\s+)?\w+\s*:\s*([\w.]+)\s*(?:\[([\w\s,]*)\]\s*)?\)(?:\s+extends\s+AnyVal)?\s*\{\s*(?://.*)?$")
            .expect("the regex is a literal")
    });
    let names = |list: Option<regex::Match>| -> Vec<String> {
        list.map_or_else(Vec::new, |m| {
            m.as_str()
                .split(',')
                .map(|p| p.trim().trim_start_matches(['+', '-']).to_owned())
                .collect()
        })
    };
    let lines: Vec<&str> = text.lines().collect();
    let at = line.checked_sub(1).filter(|&i| i < lines.len())?;
    let def = Regex::new(&format!(
        r"{}def\s+`?{}`?(?:[^\w`]|$)",
        scala_mods!(),
        regex::escape(name)
    ))
    .ok()?;
    def.is_match(lines[at]).then_some(())?;
    let c = CLASS.captures(lines[scope_of(&lines, at)?])?;
    (names(c.get(1)) == names(c.get(3))).then(|| c[2].to_owned())
}

pub fn scala_generic_declared_type(line: &str, name: &str) -> Option<String> {
    let code = super::syntax::uncommented(Kind::Jvm, line);
    Regex::new(&format!(
        r"(?:^|[^\w.]){}\s*:\s*([A-Z]\w*(?:\.[A-Z]\w*)*)\[[^\[\]]*(?:\[[^\[\]]*\][^\[\]]*)*\]\s*(?:[=,)]|$|\{{)",
        regex::escape(name)
    ))
    .ok()?
    .captures(&code)
    .map(|c| c[1].to_owned())
}
