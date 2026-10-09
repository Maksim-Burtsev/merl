use std::collections::HashMap;

use regex::Regex;

use super::*;

pub(super) fn body_of(kind: Kind, lines: &[&str], k: usize) -> std::ops::Range<usize> {
    let start = match (kind, lines[k].find('(')) {
        (Kind::Python, Some(open)) => {
            group(kind, lines, k, open).map_or(k + 1, |g| g.close_line + 1)
        }
        (Kind::TsJs, _) => ts_header(lines, k).last_line0 + 1,
        _ => k + 1,
    };
    let base = indent(lines[k]);
    let end = (start..lines.len())
        .find(|&i| {
            let t = lines[i].trim();
            !t.is_empty() && t != "{" && !comment(kind, t) && indent(lines[i]) <= base
        })
        .unwrap_or(lines.len());
    start..end.max(start)
}
pub fn field_bindings(kind: Kind, text: &str, decl: usize, name: &str) -> Vec<Binding> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = decl.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    if kind == Kind::TsJs
        && let Some(fields) = jsdoc_properties(&lines, k, name)
    {
        return fields;
    }
    let go_field = |t: &str| go_field(t, name);
    if kind == Kind::Go
        && let Some(body) = go_one_line(lines[k])
    {
        return body
            .split(';')
            .filter_map(go_field)
            .map(|value| Binding { line1: decl, value })
            .collect();
    }
    let body = body_of(kind, &lines, k);
    let Some(base) = lines[body.clone()]
        .iter()
        .find(|l| !l.trim().is_empty() && !comment(kind, l.trim()))
        .map(|l| indent(l))
    else {
        return Vec::new();
    };
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let literal = literal_lines(kind, text);
    let body = body.filter(|&i| !literal.get(i).copied().unwrap_or(false));
    let mut out = Vec::new();
    let mut push = |i: usize, value: Value| {
        out.push(Binding {
            line1: i + 1,
            value,
        })
    };
    match kind {
        Kind::Python => {
            let annotated = rule(format!(r"^(self\.)?{n}\s*:\s*([^=]+?)\s*(?:=.*)?$"));
            let assigned = rule(format!(r"^(self\.)?{n}\s*=\s*([^=].*)$"));
            let unknown = rule(format!(
                r"^\(?[\w\s,.*\[\]]*,[\w\s,.*\[\]]*\bself\.{n}\b[\w\s,.*\[\]]*\)?\s*=[^=]|^\(?[\w\s.*\[\]]*\bself\.{n}\s*,[\w\s,.*\[\]]*\)?\s*=[^=]|\bas\s+self\.{n}\b|^for\s+.*\bself\.{n}\b.*\sin\s"
            ));
            let def = rule(format!(r"^def\s+{n}\s*\("));
            let property = |i: usize, ind: usize| {
                lines[..i]
                    .iter()
                    .rev()
                    .map(|l| uncommented(kind, l))
                    .filter(|l| !l.trim().is_empty())
                    .take_while(|l| indent(l) == ind && l.trim_start().starts_with('@'))
                    .any(|l| {
                        matches!(
                            l.trim(),
                            "@property" | "@cached_property" | "@functools.cached_property"
                        )
                    })
            };
            let mut skip: Option<usize> = None;
            for i in body {
                let (code, ind) = (uncommented(kind, lines[i]), indent(lines[i]));
                let t = code.trim();
                if t.is_empty() || skip.is_some_and(|k| ind > k) {
                    continue;
                }
                skip = t.starts_with("class ").then_some(ind);
                if skip.is_some() || continued(kind, &lines, i) {
                    continue;
                }
                let right = |c: &regex::Captures| c.get(1).is_some() == (ind > base);
                if ind == base && def.is_match(t) && property(i, ind) {
                    push(i, returns(kind, text, i + 1).unwrap_or(Value::Unknown));
                    continue;
                }
                if unknown.is_match(t) {
                    push(i, Value::Unknown);
                    continue;
                }
                for s in python_statements(t, false) {
                    if let Some(c) = annotated.captures(s).filter(right) {
                        push(i, Value::Type(c[2].to_owned()));
                    } else if let Some(c) = assigned.captures(s).filter(right) {
                        push(i, value_of(kind, &c[2]));
                    }
                }
            }
        }
        Kind::TsJs => {
            let member = rule(format!(
                r"^(?:(?:public|private|protected|readonly|static|declare|override|abstract|accessor)\s+)*{n}\s*[?!]?\s*(?::\s*([^=;]+?))?\s*(?:=\s*(.+?))?\s*;?$"
            ));
            let param = rule(format!(
                r"(?:^|[(,])\s*(?:@[\w$.]+(?:\([^)]*\))?\s*)*(?:(?:public|private|protected|readonly|override)\s+)+{n}\s*\??\s*:\s*([^,)=]+)"
            ));
            let assigned = rule(format!(r"^this\.{n}\s*=\s*([^=].*?)\s*;?$"));
            let getter = rule(format!(
                r"^(?:(?:public|private|protected|static|override|abstract|declare)\s+)*get\s+{n}\s*\(\s*\)\s*(?::\s*(.+?))?\s*(?:\{{.*)?;?$"
            ));
            let in_constructor = |i: usize| {
                let ind = indent(lines[i]);
                lines[i].trim_start().starts_with("constructor")
                    || lines[..i]
                        .iter()
                        .rev()
                        .find(|l| !l.trim().is_empty() && indent(l) < ind)
                        .is_some_and(|l| l.trim_start().starts_with("constructor"))
            };
            let foreign = |i: usize| {
                bindings(kind, text, i + 1, "this")
                    .iter()
                    .any(|b| b.value != Value::Class { decl_line1: decl })
            };
            for i in body {
                let (code, ind) = (uncommented(kind, lines[i]), indent(lines[i]));
                let t = code.trim();
                if let Some(c) = getter.captures(t).filter(|_| ind == base) {
                    push(
                        i,
                        c.get(1)
                            .map_or(Value::Unknown, |ty| Value::Type(ty.as_str().to_owned())),
                    );
                } else if let Some(c) = member.captures(t).filter(|_| ind == base) {
                    push(
                        i,
                        match (c.get(1), c.get(2)) {
                            (Some(ty), _) => Value::Type(ty.as_str().trim().to_owned()),
                            (None, Some(v)) => value_of(kind, v.as_str()),
                            (None, None) => Value::Unknown,
                        },
                    );
                } else if let Some(c) = param.captures(t).filter(|_| in_constructor(i)) {
                    push(i, Value::Type(c[1].trim().to_owned()));
                } else if let Some(c) = assigned.captures(t).filter(|_| ind > base && !foreign(i)) {
                    push(i, value_of(kind, &c[1]));
                }
            }
        }
        Kind::Go if lines[k].contains("struct") => {
            for i in body.filter(|&i| indent(lines[i]) == base) {
                if let Some(value) = go_field(lines[i]) {
                    push(i, value);
                }
            }
        }
        _ => {}
    }
    out
}
fn go_field(t: &str, name: &str) -> Option<Value> {
    static GO_FIELD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)*)\s+(\S.*)$").unwrap()
    });
    let t = t.split("//").next().unwrap_or("");
    let t = t.split('`').next().unwrap_or("").trim();
    if let Some(embedded) = go_embedded(t) {
        return (embedded.rsplit('.').next() == Some(name)).then(|| Value::Type(t.to_owned()));
    }
    GO_FIELD
        .captures(t)
        .filter(|c| c[1].split(',').any(|p| p.trim() == name))
        .map(|c| Value::Type(c[2].to_owned()))
}
fn go_one_line(line: &str) -> Option<&str> {
    let at = line.find("struct")?;
    let open = at + line[at..].find('{')?;
    let close = close_of(Kind::Go, line, open)?;
    Some(&line[open + 1..close - 1])
}
pub(super) fn go_one_line_field(line: &str, name: &str) -> bool {
    go_one_line(line).is_some_and(|body| body.split(';').any(|f| go_field(f, name).is_some()))
}
pub(super) fn go_embedded(t: &str) -> Option<&str> {
    static EMBEDDED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\*?((?:[A-Za-z_]\w*\.)?[A-Za-z_]\w*)(?:\[.*\])?$").unwrap()
    });
    EMBEDDED.captures(t).map(|c| c.get(1).unwrap().as_str())
}
pub fn enclosing_type(kind: Kind, lines: &[&str], k: usize) -> Option<usize> {
    enclosing_at(kind, lines, &levels(kind, lines), k)
}
fn levels(kind: Kind, lines: &[&str]) -> Vec<Option<usize>> {
    (lines.iter())
        .map(|l| {
            let t = l.trim();
            let skip = t.is_empty()
                || t == "{"
                || comment(kind, t)
                || t.starts_with([')', ']'])
                || (kind == Kind::TsJs && (t.starts_with('>') || t.starts_with("}>")));
            (!skip).then(|| indent(l))
        })
        .collect()
}
fn enclosing_at(kind: Kind, lines: &[&str], levels: &[Option<usize>], k: usize) -> Option<usize> {
    let mut depth = indent(lines[k]);
    for i in (0..k).rev() {
        if depth == 0 {
            break;
        }
        let Some(level) = levels[i].filter(|&l| l < depth) else {
            continue;
        };
        depth = level;
        if declares_type(kind, lines[i]) {
            return Some(i + 1);
        }
    }
    None
}
pub struct FieldLine {
    pub line: usize,
    pub assigned_in_method: bool,
}

fn declaring(lines: &[&str], bindings: &[Binding], name: &str) -> Option<FieldLine> {
    let assigned = |b: &&Binding| in_method(lines[b.line1 - 1], name);
    let line = |b: &Binding, assigned_in_method| FieldLine {
        line: b.line1,
        assigned_in_method,
    };
    bindings
        .iter()
        .find(|b| !assigned(b))
        .map(|b| line(b, false))
        .or_else(|| bindings.first().map(|b| line(b, true)))
}
pub fn field_line(kind: Kind, text: &str, decl: usize, name: &str) -> Option<FieldLine> {
    let lines: Vec<&str> = text.lines().collect();
    declaring(&lines, &field_bindings(kind, text, decl, name), name)
}
pub fn field_rows(kind: Kind, text: &str, hits: &[usize], name: &str, jsdoc: bool) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let mut types: HashMap<usize, Vec<Binding>> = HashMap::new();
    let mut out = Vec::new();
    let mut levels_of = None;
    for &line in hits {
        let Some(k) = line.checked_sub(1).filter(|&k| k < lines.len()) else {
            continue;
        };
        let owner = jsdoc
            .then(|| jsdoc_owner(&lines, k))
            .flatten()
            .map(|i| i + 1);
        let Some(decl) = owner.or_else(|| {
            let levels = levels_of.get_or_insert_with(|| levels(kind, &lines));
            enclosing_at(kind, &lines, levels, k)
        }) else {
            continue;
        };
        let bindings = types
            .entry(decl)
            .or_insert_with(|| field_bindings(kind, text, decl, name));
        if bindings.iter().any(|b| b.line1 == line)
            && let Some(FieldLine { line: first, .. }) = declaring(&lines, bindings, name)
            && !out.contains(&first)
        {
            out.push(first);
        }
    }
    out.sort_unstable();
    out
}
pub fn field_decl_at(kind: Kind, text: &str, line: usize, start: usize, name: &str) -> bool {
    let Some(l) = line.checked_sub(1).and_then(|k| text.lines().nth(k)) else {
        return false;
    };
    let ident = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    let first = l
        .match_indices(name)
        .map(|(i, _)| i)
        .find(|&i| !l[..i].ends_with(ident) && !l[i + name.len()..].starts_with(ident));
    let bare = l.split("//").next().unwrap_or("");
    let embedded =
        kind == Kind::Go && go_embedded(bare.split('`').next().unwrap_or("").trim()).is_some();
    first == Some(start) && !embedded && field_rows(kind, text, &[line], name, false) == [line]
}
pub fn enum_member(kind: Kind, text: &str, decl: usize, word: &str) -> Option<(usize, usize)> {
    static ENUM: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^[^(=]*\benum\s+\w").unwrap());
    let lines: Vec<&str> = text.lines().collect();
    let k = decl.checked_sub(1).filter(|&k| k < lines.len())?;
    if !ENUM.is_match(lines[k]) {
        return None;
    }
    // ponytail: an enum body still open 2000 lines on is not read, as `group` reads none.
    let rest = lines[k..lines.len().min(k + 2000)].join("\n");
    let open = code(kind, &rest).find(|&(_, c)| c == b'{')?.0;
    let body = &rest[open + 1..close_of(kind, &rest, open)? - 1];
    let gap = r"(?:\s+|//[^\n]*|/\*.*?\*/)*";
    let member = Regex::new(&format!(
        r"(?s)^(?:{gap}\[[^\]]*\])*{gap}({}){gap}(?:=.*)?$",
        regex::escape(word)
    ))
    .expect("an escaped name keeps the pattern valid");
    split_top(kind, body, b',').into_iter().find_map(|part| {
        let at = member.captures(part)?.get(1)?.start();
        let pos = open + 1 + (part.as_ptr() as usize - body.as_ptr() as usize) + at;
        let line_start = rest[..pos].rfind('\n').map_or(0, |n| n + 1);
        Some((decl + rest[..pos].matches('\n').count(), pos - line_start))
    })
}
pub fn cs_namespace(text: &str, line: usize) -> String {
    static NS: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\u{feff}?\s*namespace\s+([\w.]+)").unwrap());
    text.lines()
        .take(line)
        .filter_map(|l| NS.captures(l))
        .last()
        .map_or_else(String::new, |c| c[1].to_owned())
}
pub fn cs_usings(text: &str) -> Vec<String> {
    static USING: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"^\u{feff}?\s*(?:global\s+)?using\s+([\w.]+)\s*;|<Using\s+Include="([\w.]+)""#)
            .unwrap()
    });
    text.lines()
        .filter_map(|l| USING.captures(l))
        .filter_map(|c| c.get(1).or(c.get(2)).map(|m| m.as_str().to_owned()))
        .collect()
}

#[derive(Debug, PartialEq)]
pub enum GoKey {
    No,
    Of(String),
    Unknown,
    Value,
    Struct(usize),
}
pub fn go_key(text: &str, line: usize, start: usize, end: usize) -> GoKey {
    let lines: Vec<&str> = text.lines().collect();
    let Some(&l) = line.checked_sub(1).and_then(|k| lines.get(k)) else {
        return GoKey::No;
    };
    let (word, t) = (&l[start..end], l.trim_start());
    let after = l[end..].trim_start();
    let key = after.starts_with(':')
        && !after.starts_with(":=")
        && !l[..start].trim_end().ends_with('.')
        && !t.starts_with("case ")
        && !t.starts_with("default")
        && uncommented(Kind::Go, l).trim() != format!("{word}:")
        && code(Kind::Go, l).any(|(i, c)| i == start && c != 0);
    if !key {
        return GoKey::No;
    }
    let literal = literal_lines(Kind::Go, text);
    let Some((j, i, b'{')) = go_open_before(&lines, &literal, line - 1, start) else {
        return GoKey::No;
    };
    match go_literal_type(&lines, &literal, j, i, 0) {
        Some(Ok(t)) if t.starts_with('[') || t.starts_with("map[") => GoKey::Value,
        Some(Ok(t)) if t.starts_with("struct@") => t["struct@".len()..]
            .parse()
            .map_or(GoKey::Unknown, GoKey::Struct),
        Some(Ok(t)) => GoKey::Of(t.trim_start_matches('*').to_owned()),
        Some(Err(())) => GoKey::Unknown,
        None => GoKey::No,
    }
}
fn go_open_before(
    lines: &[&str],
    literal: &[bool],
    k: usize,
    col: usize,
) -> Option<(usize, usize, u8)> {
    let mut depth = 0usize;
    // ponytail: a literal still open 2000 lines up is not read.
    for j in (k.saturating_sub(2000)..=k).rev() {
        if j != k && literal.get(j) == Some(&true) {
            continue;
        }
        let l = if j == k { &lines[j][..col] } else { lines[j] };
        let bytes: Vec<(usize, u8)> = code(Kind::Go, l).collect();
        for &(i, c) in bytes.iter().rev() {
            match c {
                b')' | b']' | b'}' => depth += 1,
                b'(' | b'[' | b'{' if depth == 0 => return Some((j, i, c)),
                b'(' | b'[' | b'{' => depth -= 1,
                _ => {}
            }
        }
    }
    None
}
fn go_literal_type(
    lines: &[&str],
    literal: &[bool],
    j: usize,
    i: usize,
    depth: usize,
) -> Option<Result<String, ()>> {
    static TYPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"(?:map\[[^\]]*\]|\[[^\]]*\]|\*)*[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)?(?:\[[^\]]*\])?$",
        )
        .unwrap()
    });
    static STRUCTS: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"(?:\[[^\]]*\]|map\[[^\]]*\])+$").unwrap());
    const KEYWORDS: [&str; 12] = [
        "else",
        "for",
        "switch",
        "select",
        "struct",
        "interface",
        "func",
        "range",
        "go",
        "defer",
        "if",
        "return",
    ];
    let pre = uncommented(Kind::Go, &lines[j][..i]);
    let pre = pre.trim_end();
    let above = || {
        lines[..j]
            .iter()
            .rev()
            .map(|l| uncommented(Kind::Go, l))
            .find(|l| !l.trim().is_empty())
            .unwrap_or_default()
    };
    let elided = match pre.is_empty() {
        true => above().trim_end().ends_with(['{', ',']),
        false => pre.ends_with(['{', ',', ':']),
    };
    if elided {
        // ponytail: eight elided levels deep.
        let (pj, pi, b'{') = go_open_before(lines, literal, j, i)? else {
            return None;
        };
        if depth == 8 {
            return Some(Err(()));
        }
        return Some(
            go_literal_type(lines, literal, pj, pi, depth + 1)?
                .and_then(|p| element_type(Kind::Go, &p).ok_or(())),
        );
    }
    if pre.ends_with('}') {
        let close = lines[j][..i].rfind('}')?;
        let Some((sj, si, b'{')) = go_open_before(lines, literal, j, close) else {
            return Some(Err(()));
        };
        let head = uncommented(Kind::Go, &lines[sj][..si]);
        let Some(head) = head.trim_end().strip_suffix("struct") else {
            return Some(Err(()));
        };
        let collection = STRUCTS.find(head.trim_end()).map_or("", |m| m.as_str());
        return Some(Ok(format!("{collection}struct@{}", sj + 1)));
    }
    let written = TYPE.find(pre)?.as_str();
    let last = written.rsplit(['.', ']', '*']).next().unwrap_or(written);
    let last = last.split('[').next().unwrap_or(last);
    if KEYWORDS.contains(&last) || matches!(last, "nil" | "true" | "false") {
        return None;
    }
    let before = pre[..pre.len() - written.len()].trim_end();
    let comparison = ["==", "!=", "<=", ">=", "&&"]
        .iter()
        .any(|op| before.ends_with(op));
    let opens = before.is_empty()
        || before.ends_with(['=', '(', ',', '{', ':', '[', '&'])
        || before.ends_with("<-")
        || before
            .strip_suffix("return")
            .is_some_and(|b| !b.ends_with(|c: char| c.is_alphanumeric() || c == '_'));
    (opens && !comparison).then(|| Ok(written.to_owned()))
}
