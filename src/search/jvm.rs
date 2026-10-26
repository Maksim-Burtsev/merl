use regex::Regex;

use super::{
    Binding, Group, Kind, Value, code, comment, def_patterns, group, indent, jvm_mods,
    literal_lines, qualified, scala_given_tail, scala_mods, split_top, steps_over, uncommented,
};

pub(super) fn opens_type(line: &str) -> bool {
    static TYPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            scala_mods!(),
            r"(?:(?:fun\s+)?interface|class|enum|record|@interface|trait\s+[A-Za-z_]\w*|(?:package\s+)?object\s+[A-Za-z_]\w*|companion\s+object",
            r"|extension\s*(?:\[[^\]]*\]\s*)?\(\s*\w+\s*:\s*\w+)\b"
        ))
        .expect("the regex is a literal")
    });
    TYPE.is_match(line)
}

pub(super) fn scope_of(lines: &[&str], at: usize) -> Option<usize> {
    let depth = indent(lines[at]);
    (depth > 0).then_some(())?;
    (0..at).rev().find(|&i| {
        let t = lines[i].trim_start();
        !(steps_over(Some(Kind::Jvm), t)
            || t.starts_with(['@', ')', ']', '>', '{'])
            || indent(lines[i]) >= depth)
    })
}

pub fn jvm_local_block(text: &str, line1: usize) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    let scope = scope_of(&lines, at)?;
    if opens_type(lines[scope]) {
        return None;
    }
    let depth = indent(lines[at]);
    let last_line1_of_block = (at + 1..lines.len())
        .find(|&i| {
            let t = lines[i].trim_start();
            !steps_over(Some(Kind::Jvm), t) && indent(lines[i]) < depth
        })
        .unwrap_or(lines.len());
    Some(last_line1_of_block)
}

pub fn jvm_private(line: &str) -> bool {
    static MODS: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(jvm_mods!()).expect("the regex is a literal"));
    MODS.find(line)
        .is_some_and(|m| m.as_str().split_whitespace().any(|w| w == "private"))
}

pub fn jvm_receiver(line: &str, name: &str) -> Option<String> {
    static EXTENSION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            jvm_mods!(),
            r"(?:fun|val|var)\s+(?:<[^>]*>\s*)?([\w.]+)(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?\??\.([A-Za-z_]\w*)",
            r"|^\s*extension\s*(?:\[[^\]]*\]\s*)?\(\s*\w+\s*:\s*([\w.]+)[^)]*\).*?\bdef\s+`?([A-Za-z_]\w*)"
        ))
        .expect("the regex is a literal")
    });
    let c = EXTENSION.captures(line)?;
    let (receiver, declared) = match c.get(1) {
        Some(r) => (r, &c[2]),
        None => (c.get(3)?, &c[4]),
    };
    (declared == name).then(|| receiver.as_str().to_owned())
}

pub fn jvm_receiver_at(text: &str, line: usize, name: &str) -> Option<String> {
    static BLOCK: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*extension\s*(?:\[[^\]]*\]\s*)?\(\s*\w+\s*:\s*([\w.]+)[^)]*\)\s*(?:\(\s*using\b[^)]*\)\s*)?(?://.*)?$").expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let at = line.checked_sub(1).filter(|&i| i < lines.len())?;
    if let Some(receiver) = jvm_receiver(lines[at], name) {
        return Some(receiver);
    }
    let def = Regex::new(&format!(
        r"{}def\s+`?{}`?(?:[^\w`]|$)",
        scala_mods!(),
        regex::escape(name)
    ))
    .ok()?;
    if !def.is_match(lines[at]) {
        return None;
    }
    BLOCK
        .captures(lines[scope_of(&lines, at)?])
        .map(|c| c[1].to_owned())
}

pub fn jvm_this_owner(text: &str, line1: usize) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    loop {
        at = scope_of(&lines, at)?;
        if anonymous(lines[at]) {
            return None;
        }
        if opens_type(lines[at]) {
            let name = jvm_type_name(lines[at])?;
            return Some(qualified(Kind::Jvm, text, at + 1, &name).unwrap_or(name));
        }
    }
}

pub(super) fn scala_patterns(word: &str) -> Vec<String> {
    const RESERVED: &[&str] = &[
        "class",
        "object",
        "trait",
        "type",
        "def",
        "val",
        "var",
        "given",
        "case",
        "enum",
        "extends",
        "with",
        "new",
        "import",
        "package",
        "extension",
        "this",
        "super",
    ];
    let w = regex::escape(word);
    let n = if RESERVED.contains(&word) {
        format!("`{w}`")
    } else {
        format!("`?{w}`?")
    };
    let mods = scala_mods!();
    let other_name_with_params = r"`?\w+`?(?:\([^)]*\))?";
    let ann = r"(?:@[\w.]+(?:\([^)]*\))?\s+)*";
    vec![
        format!(r"{mods}(?:class|trait|object|enum|type|package\s+object)\s+{n}(?:[^\w`]|$)"),
        format!(r"(?:{mods}|^\s*extension\b.*?\b)def\s+{n}(?:[^\w`]|$)"),
        format!(r"{mods}(?:val|var)\s+{n}(?:[^\w.`]|$)"),
        format!(r"{mods}given\s+{n}{}", scala_given_tail!()),
        format!(
            r"^\s*{ann}case\s+(?:{other_name_with_params}\s*,\s*)*{n}(?:\([^)]*\))?(?:\s*,\s*{other_name_with_params})*(?:\s+extends\s+[\w.\[\]]+(?:\([^)]*\))?)?\s*(?://.*)?$"
        ),
        format!(
            r"{mods}class\s+`?\w+`?[^(]*\((?:.*[(,])?\s*{ann}(?:(?:private|protected|override|final|implicit)(?:\[\w+\])?\s+)*(?:val|var)\s+{n}\s*:"
        ),
        format!(
            r"^\s*{ann}(?:(?:final|sealed|private|protected)(?:\[\w+\])?\s+)*case\s+class\s+`?\w+`?[^(]*\((?:.*[(,])?\s*{ann}(?:(?:private|protected|override|final|implicit)(?:\[\w+\])?\s+)*(?:(?:val|var)\s+)?{n}\s*:"
        ),
    ]
}

pub(super) fn param_names(entry: &str, groovy: bool) -> Vec<String> {
    static KOTLIN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:@[\w.]+(?:\([^)]*\))?\s+)*(?:(?:vararg|noinline|crossinline|private|public|protected|internal|override|open|val|var|using|implicit)\s+)*([A-Za-z_]\w*)\s*:").expect("the regex is a literal")
    });
    static JAVA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:@[\w.]+(?:\([^)]*\))?\s+)*(?:final\s+)?[\w.<>\[\]?,\s]*?[\w>\]]\s*(?:\.\.\.)?\s+([A-Za-z_]\w*)$").expect("the regex is a literal")
    });
    let e = entry.split_whitespace().collect::<Vec<_>>().join(" ");
    let e = e.trim_matches(|c: char| c == '(' || c == ')' || c.is_whitespace());
    let typed = if groovy {
        e.split_once('=').map_or(e, |(p, _)| p).trim_end()
    } else {
        e
    };
    let name = KOTLIN
        .captures(e)
        .or_else(|| {
            (!typed.contains(['=', ':']))
                .then(|| JAVA.captures(typed))
                .flatten()
        })
        .map(|c| c[1].to_owned())
        .or_else(|| (groovy && ident(typed)).then(|| typed.to_owned()));
    name.into_iter().filter(|n| n != "_").collect()
}

fn lambda_names(line: &str) -> Vec<String> {
    static SCALA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[(,{=]|=>)\s*(?:\(([^()]*)\)|([A-Za-z_]\w*))\s*=>")
            .expect("the regex is a literal")
    });
    static CASE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bcase\s+(.+?)\s*(?:\bif\b.*?)?=>").expect("the regex is a literal")
    });
    static BINDER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"(?:^|[^\w.`"'])([a-z_]\w*)\s*([(.]?)"#).expect("the regex is a literal")
    });
    static KOTLIN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\{\s*([^{}]*?)\s*->").expect("the regex is a literal")
    });
    static JAVA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[(,=]|->)\s*(?:\(([^()]*)\)|([A-Za-z_]\w*))\s*->")
            .expect("the regex is a literal")
    });
    let mut out = Vec::new();
    for c in KOTLIN.captures_iter(line) {
        for entry in c[1].split(',') {
            let e = entry.trim().trim_matches(['(', ')']).trim();
            let name = e.split(':').next().unwrap_or("").trim();
            if ident(name) {
                out.push(name.to_owned());
            }
        }
    }
    for c in CASE.captures_iter(line) {
        for b in BINDER.captures_iter(&c[1]).filter(|b| b[2].is_empty()) {
            out.push(b[1].to_owned());
        }
    }
    for c in JAVA.captures_iter(line).chain(SCALA.captures_iter(line)) {
        match (c.get(1), c.get(2)) {
            (Some(list), _) => {
                for entry in list.as_str().split(',') {
                    let e = entry.trim();
                    out.extend(if e.contains(char::is_whitespace) {
                        param_names(e, false)
                    } else {
                        ident(e).then(|| e.to_owned()).into_iter().collect()
                    });
                }
            }
            (None, Some(name)) => out.push(name.as_str().to_owned()),
            _ => {}
        }
    }
    out.retain(|n| n != "_");
    out
}
fn ident(s: &str) -> bool {
    s.starts_with(|c: char| c.is_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn params_open_byte(line: &str) -> Option<usize> {
    static FUN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            r"\bfun\s+(?:<[^>]*>\s*)?(?:[\w.]+(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?\??\.)?[A-Za-z_]\w*\s*\(",
            r"|\bconstructor\s*\(",
            r"|\bdef\s+`?[A-Za-z_]\w*`?\s*(?:\[[^\]]*\]\s*)?\("
        ))
        .expect("the regex is a literal")
    });
    static METHOD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            jvm_mods!(),
            r"(?:<[^>]*>\s*)?",
            super::jvm_return_type!(),
            r"\s+[A-Za-z_]\w*\s*\(",
            r"|",
            jvm_mods!(),
            r"[A-Z]\w*\s*\("
        ))
        .expect("the regex is a literal")
    });
    if opens_type(line) {
        return None;
    }
    FUN.find(line)
        .or_else(|| METHOD.find(line))
        .map(|m| m.end() - 1)
}

pub fn jvm_function(line: &str) -> Option<String> {
    let head = line.get(..params_open_byte(line)?)?.trim_end();
    let unnamed = head.trim_end_matches(|c: char| c.is_alphanumeric() || c == '_');
    Some(head.get(unnamed.len()..)?.to_owned())
}

fn param_lists_line1_binding(lines: &[&str], from: usize, name: &str) -> Option<usize> {
    static DEF: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bdef\s+[A-Za-z_]\w*\s*\(").expect("the regex is a literal")
    });
    let word = Regex::new(&format!(
        r"(?:^|[^\w$.]){}(?:[^\w$]|$)",
        regex::escape(name)
    ))
    .expect("an escaped name keeps the pattern valid");
    let groovy = DEF.is_match(lines[from]);
    let mut open = params_open_byte(lines[from])?;
    let mut at = from;
    while let Some(Group {
        inner_uncommented: inner,
        close_line: last,
        after_close: rest,
    }) = group(Kind::Jvm, lines, at, open)
    {
        if split_top(Kind::Jvm, &inner, b',')
            .iter()
            .any(|e| param_names(e, groovy).iter().any(|p| p == name))
        {
            return (from..=last)
                .find(|&i| word.is_match(&uncommented(Kind::Jvm, lines[i])))
                .map(|i| i + 1);
        }
        let scala_next_list = rest.trim_start();
        if !scala_next_list.starts_with('(') {
            return None;
        }
        (at, open) = (last, lines[last].len() - scala_next_list.len());
    }
    None
}

fn header_lines1_binding(lines: &[&str], from: usize, to: usize, name: &str) -> Vec<usize> {
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let ret = super::jvm_return_type!();
    let clause = rule(format!(
        r"\bfor\s*\(\s*(?:(?:final\s+)?(?:var|val|{ret})\s+)?{n}\s*(?:[:=]|\s+in\b)|\bfor\s*\(\s*\([^)]*\b{n}\b[^)]*\)\s+in\b|\bcatch\s*\(\s*(?:final\s+)?[\w.|\s]+?\s+{n}\s*\)|\bcatch\s*\(\s*{n}\s*:|\btry\s*\(\s*(?:final\s+)?(?:var|{ret})\s+{n}\s*=|\binstanceof\s+(?:final\s+)?[\w.]+(?:<[^>]*>)?\s+{n}\b|\b(?:instanceof|case)\s+[\w.]+(?:<[^>]*>)?\s*\(.*\b(?:var|{ret})\s+{n}\s*[,)]"
    ));
    let mut out: Vec<usize> = param_lists_line1_binding(lines, from, name)
        .into_iter()
        .collect();
    for (i, l) in lines.iter().enumerate().take(to + 1).skip(from) {
        let code = uncommented(Kind::Jvm, l);
        if clause.is_match(&code) || lambda_names(&code).iter().any(|p| p == name) {
            out.push(i + 1);
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn declares_local(t: &str, name: &str) -> bool {
    let n = regex::escape(name);
    let ret = super::jvm_return_type!();
    Regex::new(&format!(
        r"^(?:(?:final|lateinit|const)\s+)*(?:(?:val|var|def|fun)\s+{n}(?:[^\w.]|$)|(?:val|var)\s*\([^)]*\b{n}\b[^)]*\)\s*=|(?:@[\w.]+\s+)*(?:var|{ret})(?:\.\.\.)?\s+{n}\s*[=;,:])"
    ))
    .is_ok_and(|re| re.is_match(t))
}

pub(super) fn jvm_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let binding = |line| Binding {
        line1: line,
        value: Value::Unknown,
    };
    if matches!(name, "this" | "super" | "it") {
        return Vec::new();
    }
    let text = lines.join("\n");
    let literal = literal_lines(Kind::Jvm, &text);
    let bound_on_cursor_line = header_lines1_binding(lines, at, at, name);
    let mut out: Vec<Binding> = bound_on_cursor_line.into_iter().map(binding).collect();
    let mut depth = indent(lines[at]);
    let mut i = at;
    while out.is_empty() && depth > 0 && i > 0 {
        i -= 1;
        let t = uncommented(Kind::Jvm, lines[i]);
        let t = t.trim();
        let ind = indent(lines[i]);
        if t.is_empty() || comment(Kind::Jvm, t) || literal[i] || ind > depth {
            continue;
        }
        if ind == depth {
            let member_or_top_level = scope_of(lines, i).is_none_or(|s| opens_type(lines[s]));
            if declares_local(t, name) && !member_or_top_level {
                out.push(binding(i + 1));
            }
            continue;
        }
        let end = i;
        let closes_wrapped_header = t.starts_with([')', ']', '>']);
        if closes_wrapped_header {
            i = (0..i)
                .rev()
                .find(|&j| !lines[j].trim().is_empty() && indent(lines[j]) <= ind)
                .unwrap_or(i);
        }
        depth = indent(lines[i]);
        if opens_type(lines[i]) {
            let member_or_top_level = scope_of(lines, i).is_none_or(|s| opens_type(lines[s]));
            if member_or_top_level
                || header_may_bind(lines, i, end)
                || !jvm_members_of(&text, i + 1, name).is_empty()
            {
                break;
            }
            continue;
        }
        out.extend(
            header_lines1_binding(lines, i, end, name)
                .into_iter()
                .map(binding),
        );
    }
    out
}

fn header_may_bind(lines: &[&str], from: usize, to: usize) -> bool {
    static PARAMETERS_OR_SUPERTYPES: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"[(:]|\b(?:extends|implements)\b").expect("the regex is a literal")
    });
    let header = (from..=to)
        .map(|k| uncommented(Kind::Jvm, lines[k]))
        .collect::<Vec<_>>()
        .join(" ");
    let header = header.split('{').next().unwrap_or_default();
    PARAMETERS_OR_SUPERTYPES.is_match(header)
}

pub fn jvm_enclosing_types(text: &str, line1: usize) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(mut at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    while let Some(s) = scope_of(&lines, at) {
        if opens_type(lines[s]) || anonymous(lines[s]) {
            out.push(s + 1);
        }
        at = s;
    }
    out
}

pub(super) fn anonymous(line: &str) -> bool {
    static ANONYMOUS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bobject\s*:|\bnew\s+[\w.]+\s*(?:<.*>)?\s*\(.*\)\s*\{\s*$")
            .expect("the regex is a literal")
    });
    ANONYMOUS.is_match(line)
}

pub fn jvm_type_name(line: &str) -> Option<String> {
    static NAME: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:class|interface|enum|record|object|trait)\s+(?:class\s+)?([A-Za-z_]\w*)")
            .expect("the regex is a literal")
    });
    (opens_type(line) && !anonymous(line))
        .then(|| NAME.captures(line).map(|c| c[1].to_owned()))
        .flatten()
}

pub fn jvm_members_of(text: &str, decl_line1: usize, name: &str) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = decl_line1.checked_sub(1).filter(|&k| k < lines.len()) else {
        return Vec::new();
    };
    let Ok(re) = Regex::new(&def_patterns(Kind::Jvm, name).join("|")) else {
        return Vec::new();
    };
    let literal = literal_lines(Kind::Jvm, text);
    members_of(&lines, &literal, k, name, &re)
}

fn primary_constructor_property_on_header(lines: &[&str], k: usize, name: &str) -> bool {
    static PROPERTY: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:val|var)\s+([A-Za-z_]\w*)\s*:").expect("the regex is a literal")
    });
    let Some(open) = lines[k].find('(') else {
        return false;
    };
    !anonymous(lines[k])
        && group(Kind::Jvm, lines, k, open).is_some_and(
            |Group {
                 inner_uncommented: inner,
                 ..
             }| {
                split_top(Kind::Jvm, &inner, b',')
                    .iter()
                    .any(|e| PROPERTY.captures(e).is_some_and(|c| &c[1] == name))
            },
        )
        && PROPERTY.captures_iter(lines[k]).any(|c| &c[1] == name)
}

fn members_of(lines: &[&str], literal: &[bool], k: usize, name: &str, re: &Regex) -> Vec<usize> {
    static COMPANION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(jvm_mods!(), r"companion\s+object\b")).expect("the regex is a literal")
    });
    static RECORD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(jvm_mods!(), r"record\s+\w+")).expect("the regex is a literal")
    });
    if jvm_type_name(lines[k]).as_deref() == Some(name) {
        return Vec::new();
    }
    let base = indent(lines[k]);
    let code = |i: usize| {
        let t = lines[i].trim_start();
        !literal.get(i).copied().unwrap_or(false) && !steps_over(Some(Kind::Jvm), t)
    };
    let wrapped_header_tail = |i: usize| lines[i].trim_start().starts_with([')', '{']);
    let body_end = (k + 1..lines.len())
        .find(|&i| code(i) && indent(lines[i]) <= base && !wrapped_header_tail(i))
        .unwrap_or(lines.len());
    let member_indent = (k + 1..body_end)
        .filter(|&i| code(i) && indent(lines[i]) > base)
        .map(|i| indent(lines[i]))
        .min();
    let mut out = Vec::new();
    if primary_constructor_property_on_header(lines, k, name) {
        out.push(k + 1);
    }
    if RECORD.is_match(lines[k]) {
        let record_header = lines.iter().enumerate().skip(k).take(40);
        for (i, l) in record_header {
            if re.is_match(l) {
                out.push(i + 1);
            }
            if l.contains('{') {
                break;
            }
        }
    }
    let Some(member_indent) = member_indent else {
        return out;
    };
    for i in (k + 1..body_end).filter(|&i| code(i) && indent(lines[i]) == member_indent) {
        if re.is_match(lines[i]) {
            out.push(i + 1);
        } else if COMPANION.is_match(lines[i]) {
            out.extend(members_of(lines, literal, i, name, re));
        }
    }
    out
}

pub fn jvm_bases(text: &str, decl_line1: usize, scala: bool) -> Vec<String> {
    static JAVA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bextends\s+(.+?)\s*(?:\bimplements\b|\{|$)").expect("the regex is a literal")
    });
    if scala {
        return scala_bases(text, decl_line1);
    }
    let Some(outside_brackets) = header_outside_brackets(text, decl_line1) else {
        return Vec::new();
    };
    let list = match JAVA.captures(&outside_brackets) {
        Some(c) => c[1].to_owned(),
        None => match outside_brackets.split_once(':') {
            Some((_, rest)) => rest.split(" where ").next().unwrap_or("").to_owned(),
            None => return Vec::new(),
        },
    };
    base_names(&list)
}

pub fn java_implements(text: &str, decl_line1: usize) -> Vec<String> {
    static IMPLEMENTS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bimplements\s+(.+?)\s*(?:\{|$)").expect("the regex is a literal")
    });
    header_outside_brackets(text, decl_line1)
        .and_then(|h| IMPLEMENTS.captures(&h).map(|c| base_names(&c[1])))
        .unwrap_or_default()
}

fn header_outside_brackets(text: &str, decl_line1: usize) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let k = decl_line1.checked_sub(1).filter(|&k| k < lines.len())?;
    let mut header = String::new();
    for l in lines[k..].iter().take(20) {
        let code = uncommented(Kind::Jvm, l);
        header.push_str(code.split('{').next().unwrap_or(""));
        header.push(' ');
        if code.contains('{') {
            break;
        }
    }
    let mut outside_brackets = String::new();
    let mut depth = 0i32;
    for c in header.chars() {
        match c {
            '<' | '(' => depth += 1,
            '>' | ')' => depth -= 1,
            _ if depth == 0 => outside_brackets.push(c),
            _ => {}
        }
    }
    Some(outside_brackets)
}

fn base_names(list: &str) -> Vec<String> {
    list.split(',')
        .filter_map(|b| b.split_whitespace().next())
        .filter_map(|b| b.rsplit('.').next())
        .filter(|b| b.starts_with(|c: char| c.is_ascii_alphabetic()))
        .map(str::to_owned)
        .collect()
}

fn scala_bases(text: &str, decl_line1: usize) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = decl_line1.checked_sub(1).filter(|&k| k < lines.len()) else {
        return Vec::new();
    };
    let mut outside_brackets = String::new();
    let mut depth = 0i32;
    for l in lines[k..].iter().take(20) {
        let code = uncommented(Kind::Jvm, l);
        for c in code.split('{').next().unwrap_or("").chars() {
            match c {
                '[' | '(' => depth += 1,
                ']' | ')' => depth -= 1,
                _ if depth == 0 => outside_brackets.push(c),
                _ => {}
            }
        }
        outside_brackets.push(' ');
        if depth <= 0 || code.contains('{') {
            break;
        }
    }
    let Some((_, list)) = outside_brackets.split_once(" extends ") else {
        return Vec::new();
    };
    let list = list.split(" derives ").next().unwrap_or("");
    list.split(',')
        .flat_map(|b| b.split(" with "))
        .filter_map(|b| b.split_whitespace().next())
        .map(|b| b.trim_end_matches([':', '=']))
        .filter_map(|b| b.rsplit('.').next())
        .filter(|b| b.starts_with(|c: char| c.is_ascii_alphabetic()))
        .map(str::to_owned)
        .collect()
}

pub struct JvmImport {
    pub bound_name: String,
    pub path: Vec<String>,
}
pub fn jvm_imports(text: &str) -> Vec<JvmImport> {
    static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*import\s+(?:static\s+)?([\w.]*\w)(\.\*)?(?:\s+as\s+([A-Za-z_]\w*))?\s*;?\s*(?://.*)?$").expect("the regex is a literal")
    });
    text.lines()
        .filter_map(|l| IMPORT.captures(l))
        .map(|c| {
            let mut path: Vec<String> = c[1].split('.').map(str::to_owned).collect();
            let name = match (c.get(2), c.get(3)) {
                (Some(_), _) => "*".to_owned(),
                (None, Some(alias)) => alias.as_str().to_owned(),
                (None, None) => path.last().cloned().unwrap_or_default(),
            };
            if name == "*" {
                path.push(name.clone());
            }
            JvmImport {
                bound_name: name,
                path,
            }
        })
        .collect()
}

pub fn jvm_package(text: &str) -> Option<String> {
    static PACKAGE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?m)^[ \t]*package\s+([\w.]*\w)").expect("the regex is a literal")
    });
    PACKAGE
        .captures_iter(text)
        .map(|c| c[1].to_owned())
        .find(|p| p != "object")
}

const CAST_OPERAND: &str = r"(?:[\w.!?]|\([^()]*\))+";
const GENERIC: &str = r"(?:<(?:[^<>]|<(?:[^<>]|<[^<>]*>)*>)*>)?";

pub fn jvm_declared_type(line: &str, name: &str, kotlin: bool) -> Option<String> {
    let n = regex::escape(name);
    let code = uncommented(Kind::Jvm, line);
    let ty = r"([A-Z]\w*(?:\.[A-Z]\w*)*)";
    let rules: Vec<String> = if kotlin {
        if Regex::new(&format!(r"\b{n}\b[^=]*\bby\b")).is_ok_and(|re| re.is_match(&code)) {
            return None;
        }
        vec![
            format!(r"(?:^|[^\w.]){n}\s*:\s*{ty}{GENERIC}\??\s*(?:[=,)]|$|\{{)"),
            format!(
                r"\b(?:val|var)\s+{n}\s*=\s*{CAST_OPERAND}\s+as\??\s+{ty}{GENERIC}\??\s*;?\s*$"
            ),
            format!(r"\b(?:val|var)\s+{n}\s*=\s*([A-Z]\w*){GENERIC}\s*\("),
        ]
    } else {
        vec![
            format!(
                r"(?:^|[\s(,<])((?:[a-z]\w*\.)*[A-Z]\w*(?:\.[A-Z]\w*)*){GENERIC}(?:\s*\.\.\.)?\s+{n}\s*(?:[=;,:)]|&&|$)"
            ),
            format!(r"\bvar\s+{n}\s*=\s*new\s+{ty}{GENERIC}\s*\("),
            format!(r"\bvar\s+{n}\s*=\s*\(\s*{ty}{GENERIC}\s*\)\s*[\w(]"),
        ]
    };
    rules.iter().find_map(|r| {
        let re = Regex::new(r).ok()?;
        let c = re.captures(&code)?;
        let at = c.get(1)?.start();
        (!code.get(..at)?.ends_with('.')).then(|| c[1].to_owned())
    })
}

pub fn scala_type_parameter(text: &str, name: &str) -> bool {
    static LIST: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:\b(?:def|class|trait|type|given)\s+`?\w+`?\s*|\bextension\s*)\[([^\[\]]*(?:\[[^\[\]]*\][^\[\]]*)*)\]").expect("the regex is a literal")
    });
    LIST.captures_iter(text).any(|c| {
        split_top(Kind::Jvm, &c[1], b',').iter().any(|e| {
            let e = e.trim().trim_start_matches(['+', '-']);
            e.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .next()
                .is_some_and(|word| word == name)
        })
    })
}

pub fn jvm_type_parameter(text: &str, name: &str) -> bool {
    static LIST: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:\b(?:class|interface|record)\s+[A-Za-z_]\w*\s*|\bfun\s*|(?:^|[\s(,])\s*)<([^<>]*(?:<[^<>]*>[^<>]*)*)>").expect("the regex is a literal")
    });
    LIST.captures_iter(text).any(|c| {
        split_top(Kind::Jvm, &c[1], b',').iter().any(|e| {
            e.split_whitespace()
                .find(|w| !matches!(*w, "in" | "out" | "reified"))
                .is_some_and(|w| w.trim_end_matches(':') == name)
        })
    })
}

pub fn jvm_accessor(word: &str) -> Option<JvmAccessor> {
    [("get", false), ("is", false), ("set", true)]
        .into_iter()
        .find_map(|(prefix, setter)| {
            let rest = word.strip_prefix(prefix)?;
            let mut chars = rest.chars();
            let first = chars.next().filter(char::is_ascii_uppercase)?;
            let mut names = vec![format!("{}{}", first.to_ascii_lowercase(), chars.as_str())];
            if prefix == "is" {
                names.push(word.to_owned());
            }
            Some(JvmAccessor {
                fields_it_may_read: names,
                setter,
            })
        })
}
pub struct JvmAccessor {
    pub fields_it_may_read: Vec<String>,
    pub setter: bool,
}

pub fn jvm_lombok_fields(text: &str, decl_line1: usize, word: &str) -> Vec<usize> {
    static ANNOTATIONS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:@[\w.]+(?:\([^)]*\))?\s*)*").expect("the regex is a literal")
    });
    let Some(JvmAccessor {
        fields_it_may_read: names,
        setter,
    }) = jvm_accessor(word)
    else {
        return Vec::new();
    };
    if !text
        .lines()
        .any(|l| l.trim_start().starts_with("import lombok."))
    {
        return Vec::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    let with_annotations_above = |at: usize| {
        let mut out = String::from(lines[at]);
        for l in lines[..at].iter().rev() {
            if l.trim_start().starts_with('@') {
                out.push_str(l);
            } else {
                break;
            }
        }
        out
    };
    let (own, class) = if setter {
        ("Setter", "Data|Setter")
    } else {
        ("Getter", "Data|Value|Getter")
    };
    let own_re = Regex::new(&format!(r"@(?:lombok\.)?{own}\b")).expect("a fixed pattern");
    let class_re = Regex::new(&format!(r"@(?:lombok\.)?(?:{class})\b")).expect("a fixed pattern");
    let none_re =
        Regex::new(&format!(r"@(?:lombok\.)?{own}\s*\([^)]*\bNONE\b")).expect("a fixed pattern");
    let type_annotated = decl_line1
        .checked_sub(1)
        .is_some_and(|k| k < lines.len() && class_re.is_match(&with_annotations_above(k)));
    let mut out: Vec<usize> = names
        .iter()
        .flat_map(|n| jvm_members_of(text, decl_line1, n))
        .filter(|&l| {
            let code = uncommented(Kind::Jvm, lines[l - 1]);
            let code = ANNOTATIONS.replace(&code, "");
            let head = code.split('=').next().unwrap_or("");
            let field = !head.contains('(')
                && code.contains(';')
                && !head.split_whitespace().any(|w| w == "static");
            let own = with_annotations_above(l - 1);
            field && !none_re.is_match(&own) && (own_re.is_match(&own) || type_annotated)
        })
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

pub struct JvmAssignedCall {
    pub receiver: Option<String>,
    pub method: String,
}
pub fn jvm_assigned_call(line: &str, name: &str) -> Option<JvmAssignedCall> {
    let re = Regex::new(&format!(
        r"\b(?:var|val)\s+{}\s*=\s*(?:([a-z]\w*)\.)?([a-z]\w*)\s*\([^()]*\)\s*;?\s*$",
        regex::escape(name)
    ))
    .ok()?;
    let code = uncommented(Kind::Jvm, line);
    let c = re.captures(code.trim_end())?;
    Some(JvmAssignedCall {
        receiver: c.get(1).map(|m| m.as_str().to_owned()),
        method: c[2].to_owned(),
    })
}

pub fn java_declares_method(line: &str, name: &str) -> bool {
    let code = uncommented(Kind::Jvm, line);
    Regex::new(&format!(r"\b{}\b\s*(.)", regex::escape(name)))
        .ok()
        .and_then(|re| re.captures(&code).map(|c| &c[1] == "("))
        .unwrap_or(false)
}

pub fn jvm_literal_type(line: &str, name: &str, java: bool) -> Option<&'static str> {
    let re = Regex::new(&format!(
        r"\b(?:var|val)\s+{}\s*=\s*(.+?)\s*;?\s*$",
        regex::escape(name)
    ))
    .ok()?;
    let code = uncommented(Kind::Jvm, line);
    let c = re.captures(code.trim_end())?;
    let value = &c[1];
    let string = Regex::new(r#"^"(?:[^"\\]|\\.)*"$"#).ok()?;
    if string.is_match(value) {
        return Some("String");
    }
    if java {
        return None;
    }
    if Regex::new(r"^-?(?:0[xX][0-9a-fA-F_]+|[0-9][0-9_]*)$").is_ok_and(|re| re.is_match(value)) {
        return kotlin_int_type(value);
    }
    [
        (r"^'(?:[^'\\]|\\.[^']*)'$", "Char"),
        (r"^(?:true|false)$", "Boolean"),
        (r"^-?(?:0[xX][0-9a-fA-F_]+|[0-9][0-9_]*)L$", "Long"),
        (r"^-?[0-9][0-9_]*\.[0-9][0-9_]*$", "Double"),
        (r"^-?[0-9][0-9_]*(?:\.[0-9][0-9_]*)?[fF]$", "Float"),
    ]
    .into_iter()
    .find(|(r, _)| Regex::new(r).is_ok_and(|re| re.is_match(value)))
    .map(|(_, ty)| ty)
}

fn kotlin_int_type(value: &str) -> Option<&'static str> {
    let digits: String = value.trim_start_matches('-').replace('_', "");
    let n = match digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        Some(hex) => u64::from_str_radix(hex, 16),
        None => digits.parse::<u64>(),
    }
    .ok()?;
    let int = u64::from(i32::MAX.unsigned_abs());
    match n {
        n if n <= int => Some("Int"),
        n if n > int + 1 && n <= i64::MAX.unsigned_abs() => Some("Long"),
        _ => None,
    }
}

pub fn jvm_return_type(line: &str, name: &str, kotlin: bool) -> Option<String> {
    let n = regex::escape(name);
    let rule = if kotlin {
        format!(r"\bfun\s+(?:<[^>]*>\s*)?{n}\s*\([^)]*\)\s*:\s*([A-Z]\w*){GENERIC}\??")
    } else {
        format!(r"(?:^|\s)([A-Z]\w*){GENERIC}\s+{n}\s*\(")
    };
    let code = uncommented(Kind::Jvm, line);
    Regex::new(&rule)
        .ok()?
        .captures(&code)
        .map(|c| c[1].to_owned())
}

static DEFAULT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"[^=!<>]=(?:[^=~]|$)").expect("the regex is a literal")
});

pub fn jvm_parameters(text: &str, line: usize, word: &str, groovy: bool) -> Option<(usize, usize)> {
    let rows: Vec<&str> = text.lines().collect();
    let at = line.checked_sub(1)?;
    let head = Regex::new(&format!(r"\b{}\s*\(", regex::escape(word))).ok()?;
    let open = head.find(rows.get(at)?)?.end() - 1;
    let Group {
        inner_uncommented: inner,
        ..
    } = group(Kind::Jvm, &rows, at, open)?;
    if inner.trim().is_empty() {
        return Some((0, 0));
    }
    let (mut depth, mut count, mut defaults) = (0i32, 1, 0);
    let mut top = String::new();
    let mut close = |top: &mut String| {
        defaults += usize::from(groovy && DEFAULT.is_match(top));
        top.clear();
    };
    let b = inner.as_bytes();
    let mut angles = Vec::new();
    let generic_opens = |i: usize, top: &str| {
        let name = b[..i].iter().rev();
        let name = name.take_while(|c| c.is_ascii_alphanumeric() || **c == b'_');
        !groovy
            || !DEFAULT.is_match(top)
            || (name.last().is_some_and(u8::is_ascii_uppercase)
                && !matches!(b.get(i + 1), Some(b'<' | b'=')))
    };
    for (i, c) in code(Kind::Jvm, &inner) {
        match c {
            b'<' if usize::try_from(depth) == Ok(angles.len()) && generic_opens(i, &top) => {
                depth += 1;
                angles.push(depth);
            }
            b'>' if angles.last() == Some(&depth) => {
                angles.pop();
                depth -= 1;
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => {
                count += 1;
                close(&mut top);
            }
            _ if depth == 0 => top.push(char::from(c)),
            _ => {}
        }
    }
    close(&mut top);
    Some(if inner.contains("...") {
        (count - 1, usize::MAX)
    } else {
        (count - defaults, defaults)
    })
}

pub fn jvm_smart_cast(text: &str, line: usize, before: &str, name: &str) -> Option<String> {
    let n = regex::escape(name);
    let ty = format!(r"([A-Z]\w*){GENERIC}\??");
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let test = rule(format!(
        r"\bif\s*\(\s*{n}\s+is\s+{ty}\s*(?:\)|&&)|^\s*{n}\s+is\s+{ty}\s*->"
    ));
    let branch = rule(format!(r"^\s*is\s+{ty}\s*->"));
    let subject = rule(format!(
        r"\bwhen\s*\(\s*(?:{n}\s*|val\s+{n}\s*(?::[^=]*)?=.*)\)\s*\{{\s*$"
    ));
    let lines: Vec<&str> = text.lines().collect();
    let at = line.checked_sub(1).filter(|&i| i < lines.len())?;
    let header_of = |i: usize| {
        let depth = indent(lines[i]);
        (0..i).rev().find(|&j| {
            let t = lines[j].trim_start();
            !(t.is_empty() || comment(Kind::Jvm, t)) && indent(lines[j]) < depth
        })
    };
    let narrowed = |head: &str, at: usize| -> Option<String> {
        let code = uncommented(Kind::Jvm, head);
        if let Some(c) = test.captures(&code) {
            let rest = code.get(c.get_match().end()..)?;
            return (!rest.contains("else") && !rest.contains(';') && !rest.contains("||"))
                .then(|| c.get(1).or_else(|| c.get(2)).map(|m| m.as_str().to_owned()))
                .flatten();
        }
        let c = branch.captures(&code)?;
        let when = header_of(at)?;
        subject
            .is_match(&uncommented(Kind::Jvm, lines[when]))
            .then(|| c[1].to_owned())
    };
    if let Some(t) = narrowed(before, at) {
        return Some(t);
    }
    let mut i = at;
    while let Some(h) = header_of(i) {
        let head = lines[h];
        if opens_type(head) || params_open_byte(head).is_some() {
            return None;
        }
        let code = uncommented(Kind::Jvm, head);
        let opens = code.trim_end().ends_with('{') || code.trim_end().ends_with("->");
        if opens && let Some(t) = narrowed(head, h) {
            return Some(t);
        }
        i = h;
    }
    None
}

pub fn kotlin_lambda_collection(
    text: &str,
    line1: usize,
    before: &str,
    name: &str,
) -> Option<String> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            r"(?:^|[^\w.])([a-z]\w*|[A-Z]\w*\s*\.\s*entries)\s*\.\s*(?:forEach|map|mapNotNull|flatMap|filter|filterNot|sumOf",
            r"|any|all|none|count|first|firstOrNull|last|lastOrNull|find|findLast|single|singleOrNull",
            r"|onEach|sortedBy|sortedByDescending|maxBy|minBy|maxByOrNull|minByOrNull|maxOf|minOf",
            r"|groupBy|associateBy|associateWith|partition|takeWhile|dropWhile|distinctBy)\s*\{$"
        ))
        .expect("the regex is a literal")
    });
    let source = |head: &str| Some(CALL.captures(head)?[1].replace(char::is_whitespace, ""));
    if let Some((head, bound)) = kotlin_lambda_head(text, line1, before)
        && bound == name
    {
        return source(&head);
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    let nearest: Vec<usize> = jvm_bindings(&lines, at, name)
        .iter()
        .map(|b| b.line1)
        .collect();
    for _ in 0..6 {
        let s = scope_of(&lines, at)?;
        let code = uncommented(Kind::Jvm, lines[s]);
        if opens_type(&code)
            || code
                .split(|c: char| !c.is_alphanumeric())
                .any(|w| w == "fun")
        {
            return None;
        }
        let opens = code.trim_end().ends_with('{') || code.trim_end().ends_with("->");
        if opens
            && let Some((head, bound)) = kotlin_lambda_head(text, s + 1, &code)
            && bound == name
        {
            return (name == "it" || nearest == [s + 1])
                .then(|| source(&head))
                .flatten();
        }
        at = s;
    }
    None
}

pub fn kotlin_lambda_subject(text: &str, line1: usize, before: &str, name: &str) -> Option<String> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.])([a-z]\w*)\s*\.\s*(?:let|also)\s*\{$")
            .expect("the regex is a literal")
    });
    let (head, bound) = kotlin_lambda_head(text, line1, before)?;
    (bound == name).then_some(())?;
    Some(CALL.captures(&head)?[1].to_owned())
}

pub fn kotlin_lambda_receiver(text: &str, line1: usize, before: &str) -> Option<String> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"(?:(?:^|[^\w.])([a-z]\w*)\s*\.\s*(?:run|apply)|\bwith\s*\(\s*([a-z]\w*)\s*\))\s*\{$",
        )
        .expect("the regex is a literal")
    });
    let (head, bound) = kotlin_lambda_head(text, line1, before)?;
    (bound == "it").then_some(())?;
    let c = CALL.captures(&head)?;
    Some(c.get(1).or_else(|| c.get(2))?.as_str().to_owned())
}

pub fn kotlin_scope_returned(before: &str) -> Option<String> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\.\s*(?:apply|also)\s*$").expect("the regex is a literal")
    });
    let close = before.rfind("}.")?;
    let tail = before.get(close + 1..)?;
    if !tail
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        return None;
    }
    let body = before.get(..close)?;
    let mut depth = 1usize;
    let open = body.char_indices().rev().find_map(|(i, c)| {
        match c {
            '}' => depth += 1,
            '{' => depth -= 1,
            _ => {}
        }
        (depth == 0).then_some(i)
    })?;
    let head = body.get(..open)?;
    let m = CALL.find(head)?;
    Some(format!("{}{tail}", head.get(..m.start())?))
}

fn kotlin_lambda_head(text: &str, line1: usize, before: &str) -> Option<(String, String)> {
    static PARAMETER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*([A-Za-z_]\w*)\s*->").expect("the regex is a literal")
    });
    let innermost = |code: &str| {
        let mut open = Vec::new();
        for (i, c) in code.char_indices() {
            match c {
                '{' => open.push(i),
                '}' => {
                    open.pop();
                }
                _ => {}
            }
        }
        open.pop()
    };
    let mut code = uncommented(Kind::Jvm, before).replace("?.", ".");
    if innermost(&code).is_none() {
        if code.contains('}') {
            return None;
        }
        let lines: Vec<&str> = text.lines().collect();
        let at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
        code = uncommented(Kind::Jvm, lines[scope_of(&lines, at)?]).replace("?.", ".");
        let tail = code.get(innermost(&code)? + 1..)?;
        if !(tail.trim().is_empty()
            || PARAMETER
                .find(tail)
                .is_some_and(|m| m.end() == tail.trim_end().len()))
        {
            return None;
        }
    }
    let brace = innermost(&code)?;
    let inside = code.get(brace + 1..)?;
    let bound = match PARAMETER.captures(inside) {
        Some(c) => c[1].to_owned(),
        None if inside.contains("->") => return None,
        None => "it".to_owned(),
    };
    Some((code.get(..=brace)?.to_owned(), bound))
}

pub fn java_element_access(before: &str) -> Option<String> {
    static ACCESS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            r"(?:^|[^\w.])([a-z]\w*)\s*\.\s*(?:get|getFirst|getLast|peek|peekFirst|peekLast",
            r"|poll|pollFirst|pollLast|pop|element|orElseThrow|orElse)\s*\([^()]*\)\s*\.$"
        ))
        .expect("the regex is a literal")
    });
    Some(ACCESS.captures(before)?[1].to_owned())
}

pub fn java_element_type(line: &str, name: &str) -> Option<String> {
    let re = Regex::new(&format!(
        r"(?:^|[\s(,])(?:List|ArrayList|LinkedList|Deque|ArrayDeque|Queue|Stack|Vector|Optional|PriorityQueue|BlockingQueue)<\s*(?:\?\s+extends\s+)?([A-Z]\w*(?:\.[A-Z]\w*)*)\s*>\s+{}\s*(?:[=;,)]|$)",
        regex::escape(name)
    ))
    .ok()?;
    Some(re.captures(&uncommented(Kind::Jvm, line))?[1].to_owned())
}

pub fn kotlin_element_type(line: &str, name: &str) -> Option<String> {
    let re = Regex::new(&format!(
        r"(?:^|[^\w.]){}\s*:\s*(?:List|MutableList|ArrayList|Set|MutableSet|HashSet|Collection|MutableCollection|Iterable|Sequence|Array)<\s*(?:out\s+)?([A-Z]\w*(?:\.[A-Z]\w*)*)\??\s*>",
        regex::escape(name)
    ))
    .ok()?;
    Some(re.captures(&uncommented(Kind::Jvm, line))?[1].to_owned())
}

pub struct KotlinExtension {
    pub name: String,
    pub receiver: String,
}

pub fn kotlin_extension_at(text: &str, line1: usize, before: &str) -> Option<KotlinExtension> {
    static BODY: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?::[^={]*)?[={]").expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    let head = |k: usize| {
        let g = group(Kind::Jvm, &lines, k, params_open_byte(lines[k])?)?;
        let rest = uncommented(Kind::Jvm, g.after_close);
        let body = BODY.find(&rest)?.end();
        Some((
            g.close_line,
            rest.len() - body,
            rest.trim_end().len() == body,
        ))
    };
    let (k, inside) = match head(at) {
        Some((close, after_body, _)) if close == at => {
            let code = uncommented(Kind::Jvm, lines[at]);
            (at, before.get(code.len() - after_body..)?.to_owned())
        }
        _ => {
            let k = scope_of(&lines, at)?;
            let (_, _, opens_body) = head(k)?;
            if !opens_body {
                return None;
            }
            (k, before.to_owned())
        }
    };
    let inside = uncommented(Kind::Jvm, &inside);
    if scope_of(&lines, k).is_some() || inside.matches('{').count() != inside.matches('}').count() {
        return None;
    }
    let name = jvm_function(lines[k])?;
    let receiver = jvm_receiver(lines[k], &name)?;
    Some(KotlinExtension { name, receiver })
}

pub fn kotlin_initializer_parameter(
    text: &str,
    line1: usize,
    before: &str,
    name: &str,
) -> Option<usize> {
    static PROPERTY: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"^\s*(?:[a-z]+\s+)*(?:val|var)\s+[A-Za-z_]\w*\s*(?::\s*[\w.<>?, ]*)?=(?:[^=>]|$)",
        )
        .expect("the regex is a literal")
    });
    static CLASS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bclass\s+[A-Za-z_]\w*\s*(?:<[^()]*>)?\s*(?:[\w@\s]*\bconstructor\s*)?\(")
            .expect("the regex is a literal")
    });
    if !PROPERTY.is_match(&uncommented(Kind::Jvm, before)) {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();
    let at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    let k = scope_of(&lines, at)?;
    let open = CLASS.find(lines[k])?.end() - 1;
    let Group {
        inner_uncommented: inner,
        close_line,
        ..
    } = group(Kind::Jvm, &lines, k, open)?;
    if !split_top(Kind::Jvm, &inner, b',')
        .iter()
        .any(|e| param_names(e, false).iter().any(|p| p == name))
    {
        return None;
    }
    let word = Regex::new(&format!(
        r"(?:^|[^\w$.]){}(?:[^\w$]|$)",
        regex::escape(name)
    ))
    .ok()?;
    (k..=close_line)
        .find(|&i| word.is_match(&uncommented(Kind::Jvm, lines[i])))
        .map(|i| i + 1)
}

pub fn jvm_when_else(text: &str, line1: usize, name: &str) -> Option<usize> {
    static ELSE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*else\s*->").expect("the regex is a literal"));
    let subject = Regex::new(&format!(
        r"\bwhen\s*\(\s*{}\s*\)\s*\{{\s*$",
        regex::escape(name)
    ))
    .ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    if !ELSE.is_match(lines[at]) {
        return None;
    }
    let depth = indent(lines[at]);
    let head = (0..at).rev().find(|&j| {
        let t = lines[j].trim_start();
        !(t.is_empty() || comment(Kind::Jvm, t)) && indent(lines[j]) < depth
    })?;
    subject
        .is_match(&uncommented(Kind::Jvm, lines[head]))
        .then_some(head + 1)
}

pub fn jvm_constructed_receiver(before: &str, kotlin: bool) -> Option<String> {
    static JAVA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"(?:^|[^\w.])new\s+([A-Z]\w*(?:\.[A-Z]\w*)*){GENERIC}\s*\([^()]*\)\.$"
        ))
        .expect("the regex is a literal")
    });
    static KOTLIN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"(?:^|[^\w.])([A-Z]\w*(?:\.[A-Z]\w*)*){GENERIC}\s*\([^()]*\)\.$"
        ))
        .expect("the regex is a literal")
    });
    let re = if kotlin { &*KOTLIN } else { &*JAVA };
    re.captures(before.trim_end()).map(|c| c[1].to_owned())
}

pub fn jvm_cast_receiver(before: &str, kotlin: bool) -> Option<String> {
    static JAVA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"(?:^|[^\w.])\(\s*\(\s*([A-Z]\w*){GENERIC}\s*\)\s*[a-z]{CAST_OPERAND}\s*\)\.$"
        ))
        .expect("the regex is a literal")
    });
    static KOTLIN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"(?:^|[^\w.])\(\s*{CAST_OPERAND}\s+as\??\s+([A-Z]\w*){GENERIC}\??\s*\)(?:\?|!!)?\.$"
        ))
        .expect("the regex is a literal")
    });
    let re = if kotlin { &*KOTLIN } else { &*JAVA };
    re.captures(before.trim_end()).map(|c| c[1].to_owned())
}

pub fn jvm_getter_chain(before: &str) -> Vec<String> {
    let mut rest = before;
    let mut chain = Vec::new();
    while let Some(link) = rest.strip_suffix('.') {
        let (link, call) = link.strip_suffix("()").map_or((link, false), |l| (l, true));
        let head = link.trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_');
        let name = link.get(head.len()..).unwrap_or_default();
        if name.is_empty() || (call && jvm_accessor(name).is_none_or(|a| a.setter)) {
            return Vec::new();
        }
        chain.insert(
            0,
            if call {
                format!("{name}()")
            } else {
                name.to_owned()
            },
        );
        rest = head;
    }
    let led_by_a_name = chain.first().is_some_and(|f| !f.ends_with(')'));
    let calls = chain.iter().any(|l| l.ends_with(')'));
    if !led_by_a_name || !calls || rest.ends_with(['$', ')', ']', '>', ':', '"']) {
        return Vec::new();
    }
    chain
}

pub struct JvmCall {
    pub arguments: Vec<String>,
    pub trailing_lambda: bool,
}

pub fn jvm_call<S: AsRef<str>>(lines: &[S], line: usize, open: usize) -> Option<JvmCall> {
    let rows: Vec<&str> = lines.iter().map(AsRef::as_ref).collect();
    let Group {
        inner_uncommented: inner,
        after_close,
        ..
    } = group(Kind::Jvm, &rows, line, open)?;
    let arguments = if inner.trim().is_empty() {
        Vec::new()
    } else {
        split_top(Kind::Jvm, &inner, b',')
            .into_iter()
            .map(|a| a.trim().to_owned())
            .collect()
    };
    Some(JvmCall {
        arguments,
        trailing_lambda: after_close.trim_start().starts_with('{'),
    })
}

pub fn jvm_parameter_parts(text: &str, line1: usize, word: &str) -> Option<Vec<String>> {
    let rows: Vec<&str> = text.lines().collect();
    let at = line1.checked_sub(1)?;
    let head = Regex::new(&format!(r"\b{}\s*\(", regex::escape(word))).ok()?;
    let open = head.find(rows.get(at)?)?.end() - 1;
    let Group {
        inner_uncommented: inner,
        ..
    } = group(Kind::Jvm, &rows, at, open)?;
    if inner.trim().is_empty() {
        return Some(Vec::new());
    }
    let b = inner.as_bytes();
    let (mut depth, mut start, mut out) = (0i32, 0, Vec::new());
    for (i, c) in code(Kind::Jvm, &inner) {
        match c {
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b'>' if i > 0 && b[i - 1] == b'-' => {}
            b')' | b']' | b'}' | b'>' => depth -= 1,
            b',' if depth == 0 => {
                out.push(inner.get(start..i)?.trim().to_owned());
                start = i + 1;
            }
            _ => {}
        }
        if depth < 0 {
            return None;
        }
    }
    out.push(inner.get(start..)?.trim().to_owned());
    (depth == 0).then_some(out)
}

pub fn jvm_takes(parameters: &[String], count: usize) -> bool {
    let vararg = parameters
        .iter()
        .any(|p| p.starts_with("vararg ") || p.contains("..."));
    let required = parameters.iter().filter(|p| !DEFAULT.is_match(p)).count();
    if vararg {
        count + 1 >= required
    } else {
        (required..=parameters.len()).contains(&count)
    }
}

pub fn jvm_takes_call(parameters: &[String], call: &JvmCall, kotlin: bool) -> bool {
    static NAMED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(\w+)\s*=(?:[^=]|$)").expect("the regex is a literal")
    });
    let named = |a: &String| NAMED.captures(a).map(|c| c[1].to_owned());
    let declares = |name: String| {
        parameters.iter().any(|p| {
            let head = p.split([':', '=']).next().unwrap_or_default();
            head.split_whitespace().last() == Some(name.as_str())
        })
    };
    let count = call.arguments.len() + usize::from(call.trailing_lambda);
    jvm_takes(parameters, count)
        && (!kotlin || call.arguments.iter().filter_map(named).all(declares))
}

pub fn jvm_literal_parameter(parameter: &str) -> Option<&'static str> {
    static JAVA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:final\s+)?(int|String)\s+\w+$").expect("the regex is a literal")
    });
    static KOTLIN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\w+\s*:\s*(Int|String)$").expect("the regex is a literal")
    });
    let c = JAVA
        .captures(parameter)
        .or_else(|| KOTLIN.captures(parameter))?;
    Some(if c[1].eq_ignore_ascii_case("int") {
        "int"
    } else {
        "string"
    })
}

pub fn jvm_literal_argument(argument: &str) -> Option<&'static str> {
    static STRING: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"^"(?:[^"\\$\n]|\\.)*"$"#).expect("the regex is a literal")
    });
    static INT: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\d{1,9}$").expect("the regex is a literal"));
    if STRING.is_match(argument) {
        Some("string")
    } else if INT.is_match(argument) {
        Some("int")
    } else {
        None
    }
}
