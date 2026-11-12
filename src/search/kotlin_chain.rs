use regex::Regex;

use super::jvm::{
    anonymous, jvm_declared_type, opens_type, param_names, params_open_byte, scope_of,
};
use super::{Group, Kind, group, indent, split_top, uncommented};

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
                if !constructed
                    && upper(&full)
                    && (h.ends_with('@') || !links.first().is_some_and(|l| l.ends_with(')')))
                {
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
    if c[1].split(',').any(|e| e.trim() == &c[2]) {
        return None;
    }
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

pub fn kotlin_constructor_parameter(text: &str, line1: usize, name: &str) -> Option<usize> {
    static CLASS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bclass\s+\w+[^(:{]*\(").expect("the regex is a literal")
    });
    static FUNCTION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.])(?:get|set)\s*\(|\b(?:fun|constructor)\b")
            .expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let in_function = |l: &str| FUNCTION.is_match(&uncommented(Kind::Jvm, l));
    let mut at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    if in_function(lines[at]) {
        return None;
    }
    let decl = loop {
        let s = scope_of(&lines, at)?;
        if opens_type(lines[s]) {
            break s;
        }
        if anonymous(lines[s]) || in_function(lines[s]) {
            return None;
        }
        at = s;
    };
    let open = CLASS.find(lines[decl])?.end() - 1;
    let g = group(Kind::Jvm, &lines, decl, open)?;
    let entry = split_top(Kind::Jvm, &g.inner_uncommented, b',')
        .into_iter()
        .find(|e| param_names(e, false).iter().any(|p| p == name))?;
    let declared = jvm_declared_type(entry, name, true)?;
    (decl..=g.close_line)
        .find(|&i| jvm_declared_type(lines[i], name, true).as_ref() == Some(&declared))
        .map(|i| i + 1)
}

pub fn kotlin_typealias(line: &str) -> Option<(&str, &str)> {
    static ALIAS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"^(?:(?:public|internal)\s+)*typealias\s+([A-Za-z_]\w*)\s*=\s*([A-Za-z_][\w.]*)\s*$",
        )
        .expect("the regex is a literal")
    });
    let c = ALIAS.captures(line)?;
    Some((c.get(1)?.as_str(), c.get(2)?.as_str()))
}

pub fn kotlin_backing_property(text: &str, line1: usize, col: usize) -> Option<usize> {
    static ACCESSOR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:private|protected|internal|public|override|inline)\s+)*(?:get|set)\s*\(([^)]*)\)")
            .expect("the regex is a literal")
    });
    static PROPERTY: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:private|protected|internal|public|override|open|final|lateinit|const|abstract)\s+)*(?:val|var)\s+[A-Za-z_]\w*\b")
            .expect("the regex is a literal")
    });
    static LOCAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:val|var)\s+field\b|\bfield\s*(?:->|:)|\bfun\b|\bclass\b|\bobject\b")
            .expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let mut at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    let here = uncommented(Kind::Jvm, lines[at]);
    let before = here.get(..col)?;
    if let Some(c) = ACCESSOR.captures(before) {
        if c[1]
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .any(|w| w == "field")
        {
            return None;
        }
        if PROPERTY.is_match(before) {
            return Some(at + 1);
        }
        let s = scope_of(&lines, at)?;
        return PROPERTY.is_match(lines[s]).then_some(s + 1);
    }
    if PROPERTY.is_match(before)
        && Regex::new(r"\b(?:get|set)\s*\(").is_ok_and(|r| r.is_match(before))
    {
        return Some(at + 1);
    }
    if LOCAL.is_match(before) {
        return None;
    }
    loop {
        let s = scope_of(&lines, at)?;
        let line = uncommented(Kind::Jvm, lines[s]);
        if let Some(c) = ACCESSOR.captures(&line) {
            if c[1]
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .any(|w| w == "field")
            {
                return None;
            }
            let p = scope_of(&lines, s)?;
            return PROPERTY.is_match(lines[p]).then_some(p + 1);
        }
        if LOCAL.is_match(&line) || PROPERTY.is_match(&line) {
            return None;
        }
        if (s + 1..at).any(|i| LOCAL.is_match(&uncommented(Kind::Jvm, lines[i]))) {
            return None;
        }
        at = s;
    }
}

pub fn kotlin_map_read(before: &str) -> Option<String> {
    static READ: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            r"(?:^|[^\w.])([a-z]\w*)\s*(?:\[[^\[\]]*\]|\.\s*(?:getValue|get)\s*\([^()]*\))",
            r"\s*(?:!!|\?)?\.$"
        ))
        .expect("the regex is a literal")
    });
    Some(READ.captures(before.trim_end())?[1].to_owned())
}

pub fn kotlin_map_value(line: &str, name: &str) -> Option<(String, String)> {
    let re = Regex::new(&format!(
        concat!(
            r"(?:^|[^\w.]){}\s*:\s*(Map|MutableMap|HashMap|LinkedHashMap|SortedMap|TreeMap)<",
            r"\s*[A-Z][\w.]*\??\s*,\s*(?:out\s+)?([A-Z]\w*(?:\.[A-Z]\w*)*)\??\s*>\??\s*(?:[=,)]|$)"
        ),
        regex::escape(name)
    ))
    .ok()?;
    let code = uncommented(Kind::Jvm, line);
    let c = re.captures(&code)?;
    Some((c[1].to_owned(), c[2].to_owned()))
}

pub fn kotlin_guarded(lines: &[&str], at: usize, name: &str) -> Option<String> {
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let guard = rule(format!(
        r"^\s*if\s*\(\s*{n}\s*!is\s+([A-Z]\w*)(?:<[^>]*>)?\??\s*\)\s*(?:return|throw|continue|break)\b"
    ));
    let touched = rule(format!(r"\b{n}\s*(?:[-+*/%]?=[^=]|!?is\b|as\b)"));
    let depth = indent(lines[at]);
    for j in (0..at).rev() {
        let code = uncommented(Kind::Jvm, lines[j]);
        if code.trim().is_empty() {
            continue;
        }
        if indent(lines[j]) < depth {
            return None;
        }
        if let Some(c) = guard.captures(&code).filter(|_| indent(lines[j]) == depth) {
            return Some(c[1].to_owned());
        }
        if touched.is_match(&code) {
            return None;
        }
    }
    None
}

pub fn kotlin_check_closed(text: &str, line1: usize, mention1: usize, name: &str) -> bool {
    let n = regex::escape(name);
    let Ok(check) = Regex::new(&format!(
        r"^\s*(?:if\s*\(\s*{n}\s+is\s+[A-Z][\w.]*(?:<[^>]*>)?\??\s*\)|when\s*\(\s*{n}\s*\))\s*\{{\s*$"
    )) else {
        return false;
    };
    static EXIT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:return|throw|continue|break)\b").expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(head) = mention1.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    if mention1 >= line1 || !check.is_match(&uncommented(Kind::Jvm, head)) {
        return false;
    }
    let depth = indent(head);
    for line in lines.iter().take(line1 - 1).skip(mention1) {
        let code = uncommented(Kind::Jvm, line);
        if code.trim().is_empty() {
            continue;
        }
        if indent(line) <= depth {
            return code.trim() == "}" && indent(line) == depth;
        }
        if EXIT.is_match(&code) {
            return false;
        }
    }
    false
}

pub fn jvm_function_start(lines: &[&str], line1: usize) -> Option<usize> {
    let mut depth = indent(lines.get(line1.checked_sub(1)?)?);
    let mut start = None;
    for j in (0..line1 - 1).rev() {
        let code = uncommented(Kind::Jvm, lines[j]);
        if code.trim().is_empty() || indent(lines[j]) >= depth {
            continue;
        }
        if opens_type(lines[j]) {
            return start;
        }
        if start.is_some() {
            return None;
        }
        if params_open_byte(&code).is_some() {
            start = Some(j + 1);
        }
        depth = indent(lines[j]);
    }
    start
}
