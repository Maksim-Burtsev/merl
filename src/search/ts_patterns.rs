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
        return Some(Value::Field { chain: from, field });
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
pub(super) struct TsDeclarator {
    pub line0: usize,
    pub as_own_statement: String,
}
pub(super) fn ts_declarators(lines: &[&str], k: usize) -> Option<Vec<TsDeclarator>> {
    static KEYWORD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:export\s+)?(?:declare\s+)?(?:const|let|var)\s+").unwrap()
    });
    let first = uncommented(Kind::TsJs, lines.get(k)?);
    if !first.trim_end().ends_with(',') {
        return None;
    }
    let keyword = KEYWORD.find(first.trim_start())?.as_str().to_owned();
    // ponytail: two hundred lines of declarators.
    let text = blank_comments(&lines[k..lines.len().min(k + 200)].join("\n"));
    let ind = indent(lines[k]);
    let (mut depth, mut end) = (0i32, text.len());
    let mut starts = vec![0];
    for (i, c) in code(Kind::TsJs, &text) {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => starts.push(i + 1),
            b';' if depth == 0 => {
                end = i;
                break;
            }
            // Its last line is the one before a line back at its indent.
            b'\n' if depth == 0 => {
                let next = text[i + 1..].lines().find(|l| !l.trim().is_empty());
                if next.is_none_or(|l| indent(l) <= ind) {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for (n, &from) in starts.iter().enumerate() {
        let to = starts.get(n + 1).map_or(end, |&s| s - 1).min(end);
        let Some(piece) = text.get(from..to) else {
            continue;
        };
        let lead = piece.len() - piece.trim_start().len();
        let line = k + text[..from + lead].matches('\n').count();
        let piece = piece.split_whitespace().collect::<Vec<_>>().join(" ");
        if piece.is_empty() {
            continue;
        }
        out.push(TsDeclarator {
            line0: line,
            as_own_statement: match n {
                0 => piece,
                _ => format!("{keyword}{piece}"),
            },
        });
    }
    (out.len() > 1).then_some(out)
}
/// `s` with its TypeScript comments, `//` and `/* */`, blanked out and its lines kept.
fn blank_comments(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = b.to_vec();
    let (mut i, mut quote) = (0, None);
    while i < b.len() {
        match (quote, b[i]) {
            (Some(q), c) => {
                if c == b'\\' {
                    i += 1;
                } else if c == q {
                    quote = None;
                }
            }
            (None, c @ (b'"' | b'\'' | b'`')) => quote = Some(c),
            (None, b'/') if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    out[i] = b' ';
                    i += 1;
                }
                continue;
            }
            (None, b'/') if b.get(i + 1) == Some(&b'*') => {
                let end = s[i + 2..].find("*/").map_or(b.len(), |e| i + 2 + e + 2);
                for o in &mut out[i..end] {
                    if *o != b'\n' {
                        *o = b' ';
                    }
                }
                i = end;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_owned())
}
