use regex::Regex;

use super::*;

pub(super) fn ts_params(params: &str, line: usize, name: &str, out: &mut Vec<Binding>) {
    static MODS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:(?:public|private|protected|readonly|override)\s+)*(?:\.\.\.)?").unwrap()
    });
    for p in split_top(Kind::TsJs, params, b',') {
        let p = MODS.replace(p.trim(), "");
        if p.starts_with(['{', '[']) {
            if names(&p.replace("...", " "), name) {
                let value = typed_pattern(&p, name).unwrap_or(Value::Unknown);
                out.push(Binding { line1: line, value });
            }
            continue;
        }
        let parts = split_top(Kind::TsJs, &p, b'=');
        let (pname, annotation) = match parts[0].split_once(':') {
            Some((pname, t)) => (pname, Some(t.trim())),
            None => (parts[0], None),
        };
        if pname.trim().trim_end_matches('?') != name {
            continue;
        }
        let value = match (annotation, parts.get(1)) {
            (Some(t), _) => Value::Type(t.to_owned()),
            (None, Some(default)) => value_of(Kind::TsJs, default),
            (None, None) => Value::Unknown,
        };
        out.push(Binding { line1: line, value });
    }
}

fn typed_pattern(p: &str, name: &str) -> Option<Value> {
    let close = close_of(Kind::TsJs, p, 0).filter(|_| p.starts_with('{'))?;
    let written = p[close..].trim_start().strip_prefix(':')?;
    let written = split_top(Kind::TsJs, written, b'=')[0].trim();
    let field = split_top(Kind::TsJs, &p[1..close - 1], b',')
        .into_iter()
        .find_map(|item| {
            let item = item.trim();
            let (field, local) = item.split_once(':').unwrap_or((item, item));
            (local.trim() == name).then(|| field.trim())
        })?;
    let plain =
        |s: &str| !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || "_$".contains(c));
    (plain(field) && !written.is_empty())
        .then(|| Value::Member(Box::new(Value::Type(written.to_owned())), field.to_owned()))
}

pub(super) fn ts_destructured(t: &str, name: &str) -> Option<Value> {
    static SHAPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:export\s+)?(?:const|let|var)\s*\{([^{}\[\]]*)\}\s*=\s*(.+?)\s*;?$")
            .unwrap()
    });
    static CHAIN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:this|[A-Za-z_$][\w$]*)(?:\.#?[A-Za-z_$][\w$]*)*$").unwrap()
    });
    let c = SHAPE.captures(t)?;
    let field = c[1].split(',').find_map(|item| {
        let (field, local) = item.split_once(':').unwrap_or((item, item));
        (local.trim() == name).then(|| field.trim().to_owned())
    })?;
    if CHAIN.is_match(&c[2]) {
        let from = c[2].split('.').map(str::to_owned).collect();
        return Some(Value::Field(from, field));
    }
    match value_of(Kind::TsJs, &c[2]) {
        call @ Value::Call(_) => Some(Value::Member(Box::new(call), field)),
        _ => None,
    }
}

pub fn ts_literal_member(
    text: &str,
    line: usize,
    written: &str,
    word: &str,
) -> Option<(usize, String)> {
    if split_top(Kind::TsJs, written, b'|').len() > 1 {
        return None;
    }
    let operands = split_top(Kind::TsJs, written, b'&');
    if operands.len() < 2 {
        return None;
    }
    let member = Regex::new(&format!(
        r"^(?:readonly\s+)?{}\s*\??\s*[:(]",
        regex::escape(word)
    ))
    .expect("an escaped name keeps the pattern valid");
    let declaring: Vec<(&str, usize)> = operands
        .iter()
        .map(|o| o.trim())
        .filter(|o| o.starts_with('{') && close_of(Kind::TsJs, o, 0) == Some(o.len()))
        .filter_map(|o| {
            let inner = &o[1..o.len() - 1];
            let at = split_top(Kind::TsJs, inner, b';')
                .into_iter()
                .flat_map(|s| split_top(Kind::TsJs, s, b','))
                .find(|m| member.is_match(m.trim_start()))?;
            let offset = at.as_ptr() as usize - o.as_ptr() as usize;
            Some((o, offset + at.len() - at.trim_start().len()))
        })
        .collect();
    let [(literal, offset)] = declaring[..] else {
        return None;
    };
    let lines: Vec<&str> = text.lines().collect();
    let from = line.checked_sub(1)?;
    let header: Vec<&str> = lines[from..lines.len().min(from + 40)]
        .iter()
        .map(|l| l.trim())
        .collect();
    let header = uncommented(Kind::TsJs, &header.join("\n"));
    let start = header.find(literal)?;
    let found = from + header[..start + offset].matches('\n').count() + 1;
    Some((
        found,
        literal.split_whitespace().collect::<Vec<_>>().join(" "),
    ))
}
