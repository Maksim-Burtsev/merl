use regex::Regex;

use super::{Group, Kind, group, split_top, uncommented};

pub struct KotlinCallChain {
    pub head: String,
    pub constructed: bool,
    pub links: Vec<String>,
}

pub fn kotlin_call_chain(before: &str) -> Option<KotlinCallChain> {
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let upper = |n: &str| n.starts_with(|c: char| c.is_ascii_uppercase());
    let mut s = before.trim_end().strip_suffix('.')?.trim_end();
    let mut links: Vec<String> = Vec::new();
    loop {
        let call = s.ends_with(')');
        let rest = if call {
            let mut depth = 0usize;
            let open = s.char_indices().rev().find_map(|(i, c)| {
                match c {
                    ')' => depth += 1,
                    '(' => depth = depth.checked_sub(1)?,
                    _ => {}
                }
                (depth == 0).then_some(i)
            })?;
            s.get(..open)?.trim_end()
        } else {
            s
        };
        let head = rest.trim_end_matches(ident);
        let name = rest.get(head.len()..)?;
        if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
            return None;
        }
        match head.trim_end().strip_suffix('.') {
            Some(h) if (!call || !upper(name)) && !h.ends_with('.') => {
                links.insert(
                    0,
                    if call {
                        format!("{name}()")
                    } else {
                        name.to_owned()
                    },
                );
                s = h.trim_end();
            }
            _ if call && !upper(name) => return None,
            _ => {
                let mut full = name.to_owned();
                let mut h = head;
                while let Some(q) = h.strip_suffix('.') {
                    let start = q.trim_end_matches(ident);
                    let n = q.get(start.len()..)?;
                    if !upper(n) {
                        return None;
                    }
                    full = format!("{n}.{full}");
                    h = start;
                }
                if h.ends_with(|c: char| ident(c) || ".)]}\"'$".contains(c)) {
                    return None;
                }
                let constructed = call;
                if !constructed && upper(&full) {
                    return None;
                }
                links.iter().any(|l| l.ends_with(')')).then_some(())?;
                return Some(KotlinCallChain {
                    head: full,
                    constructed,
                    links,
                });
            }
        }
    }
}

pub fn kotlin_initializer(text: &str, line1: usize, name: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let re = Regex::new(&format!(
        r"\b(?:val|var)\s+{}\s*=\s*(.+)$",
        regex::escape(name)
    ))
    .ok()?;
    let first = uncommented(Kind::Jvm, lines.get(line1.checked_sub(1)?)?);
    let mut init = re.captures(first.trim_end())?[1].to_owned();
    for l in lines.iter().skip(line1) {
        let code = uncommented(Kind::Jvm, l);
        let t = code.trim();
        if t.is_empty() {
            continue;
        }
        if !(t.starts_with('.') || t.starts_with("?.")) {
            break;
        }
        init.push_str(t);
    }
    Some(init.replace("?.", ".").replace("!!.", "."))
}

pub struct KotlinDestructured {
    pub index: usize,
    pub source: String,
    pub in_loop: bool,
}

pub fn kotlin_destructured(line: &str, name: &str) -> Option<KotlinDestructured> {
    static VAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:val|var)\s*\(([^()]*)\)\s*=\s*([a-z]\w*)\s*$")
            .expect("the regex is a literal")
    });
    static FOR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bfor\s*\(\s*\(([^()]*)\)\s+in\s+([a-z]\w*)\s*\)")
            .expect("the regex is a literal")
    });
    let code = uncommented(Kind::Jvm, line);
    let (c, in_loop) = match VAL.captures(code.trim_end()) {
        Some(c) => (c, false),
        None => (FOR.captures(&code)?, true),
    };
    let index = c[1].split(',').position(|e| e.trim() == name)?;
    Some(KotlinDestructured {
        index,
        source: c[2].to_owned(),
        in_loop,
    })
}

pub fn kotlin_component_type(text: &str, decl_line1: usize, index: usize) -> Option<String> {
    static PROPERTY: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:@\w+\s+)*(?:(?:private|protected|internal|public|override)\s+)*(?:val|var)\s+\w+\s*:\s*([A-Z]\w*(?:\.[A-Z]\w*)*)\??\s*(?:=|$)")
            .expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let k = decl_line1.checked_sub(1)?;
    let header = lines.get(k)?;
    if !Regex::new(r"\bdata\s+class\b").ok()?.is_match(header) {
        return None;
    }
    let open = header.find('(')?;
    let Group {
        inner_uncommented: inner,
        ..
    } = group(Kind::Jvm, &lines, k, open)?;
    let parameters = split_top(Kind::Jvm, &inner, b',');
    let parameter = parameters.get(index)?;
    Some(PROPERTY.captures(parameter)?[1].to_owned())
}
