use super::c::{
    CPlace, CType, DeclaredIn, Head, bracket_contents_blanked, c_body_head, c_class_around, c_code,
    c_enum_head, c_enumerators, c_innermost_open_brace, c_member_decl, c_type_def, close_or_end,
    declared_at, line_starts, scopes_inside_out_to_type_or_namespace,
};
use regex::Regex;
use std::sync::LazyLock;

pub(super) fn scope_pos(
    code: &str,
    starts: &[usize],
    line1: usize,
    col: Option<usize>,
    name: &str,
) -> Option<usize> {
    let from = *starts.get(line1.checked_sub(1)?)?;
    let to = starts.get(line1).map_or(code.len(), |&s| s - 1);
    let is_name = |c: char| c.is_alphanumeric() || c == '_';
    let found: Vec<usize> = (code.get(from..to)?.match_indices(name))
        .map(|(i, _)| i)
        .filter(|&i| {
            code.get(..from + i).is_some_and(|h| !h.ends_with(is_name))
                && (code.get(from + i + name.len()..)).is_some_and(|r| !r.starts_with(is_name))
        })
        .collect();
    let at = col.and_then(|c| found.iter().rfind(|&&i| i <= c));
    Some(at.or_else(|| found.last()).map_or(to, |&i| from + i))
}

enum Reach {
    Plain,
    Hidden,
    Block(usize),
}
pub fn c_block_scoped<T>(
    text: &str,
    name: &str,
    from1: usize,
    hits: Vec<T>,
    line_here: impl Fn(&T) -> Option<usize>,
) -> Vec<T> {
    let code = c_code(text);
    let starts = line_starts(&code);
    let at = scope_pos(&code, &starts, from1, None, name);
    let reach = |line1: usize| {
        let open = scope_pos(&code, &starts, line1, None, name).and_then(|pos| {
            scopes_inside_out_to_type_or_namespace(&code, pos)
                .into_iter()
                .find(|s| matches!(s.head, Head::Function { .. } | Head::Lambda { .. }))
                .map(|s| s.open)
        });
        let (Some(open), Some(at)) = (open, at) else {
            return Reach::Plain;
        };
        let inside = open < at && at <= close_or_end(&code, open, code.len());
        match line1.cmp(&from1) {
            std::cmp::Ordering::Equal => Reach::Plain,
            std::cmp::Ordering::Less if inside => Reach::Block(open),
            _ => Reach::Hidden,
        }
    };
    let hits: Vec<(T, Reach)> = hits
        .into_iter()
        .map(|h| {
            let r = line_here(&h).map_or(Reach::Plain, reach);
            (h, r)
        })
        .filter(|(_, r)| !matches!(r, Reach::Hidden))
        .collect();
    let inner = (hits.iter())
        .filter_map(|(_, r)| match r {
            Reach::Block(open) => Some(*open),
            _ => None,
        })
        .max();
    hits.into_iter()
        .filter(|(_, r)| inner.is_none() || matches!(r, Reach::Block(o) if Some(*o) == inner))
        .map(|(h, _)| h)
        .collect()
}
pub fn c_heads_a_nested_class(word: &str, line: &str) -> bool {
    Regex::new(&format!(
        r"^[^:]*\b(?:struct|class|union)\s+(?:\w+\s+)*{}\s*<[^<>]*>\s*::\s*\w",
        regex::escape(word)
    ))
    .is_ok_and(|re| re.is_match(line))
}
pub(super) fn control_init(code: &str, from: usize, pos: usize, name: &str) -> Option<usize> {
    static CONTROL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:else\s+)?(?:for|if|while|switch)\s*\(").expect("the regex is a literal")
    });
    let mut at = from;
    let mut found = None;
    while let Some(m) = code.get(at..pos).and_then(|s| CONTROL.find(s)) {
        let open = at + m.end() - 1;
        let close = close_or_end(code, open, pos);
        let parens = code.get(open + 1..close).unwrap_or_default();
        if let Some(k) = control_head_declarations(parens, name).last() {
            found = Some(open + 1 + k);
        }
        if close >= pos {
            break;
        }
        at = close + 1;
    }
    found
}
pub(super) fn control_head_declarations(parens: &str, name: &str) -> Vec<usize> {
    let parts = split_top_level(parens, ';');
    let init = parts.len() > 1;
    (parts.into_iter().enumerate())
        .filter_map(|(i, (at, part))| {
            let statement = parens.get(at..=at + part.len()).filter(|_| i == 0 && init);
            (statement.and_then(|s| declared_at(s, name, DeclaredIn::Statement)))
                .or_else(|| declared_at(part, name, DeclaredIn::ControlHead))
                .or_else(|| super::objc_for_in(part, name))
                .map(|k| at + k)
        })
        .collect()
}
fn split_top_level(s: &str, sep: char) -> Vec<(usize, &str)> {
    let f = bracket_contents_blanked(s);
    let mut from = 0;
    f.split(sep)
        .filter_map(|part| {
            let at = from;
            from += part.len() + 1;
            Some((at, s.get(at..at + part.len())?))
        })
        .collect()
}
static DEFINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?m)^[ \t]*#[ \t]*define[ \t]+([A-Za-z_]\w*)\(([^)\n]*)\)((?:[^\n]*\\\r?\n)*[^\n]*)",
    )
    .expect("the regex is a literal")
});
pub fn c_function_macros(text: &str) -> String {
    (DEFINE.find_iter(text))
        .flat_map(|m| ["\n", m.as_str()])
        .collect()
}
pub(super) type Declarer = (String, Vec<usize>);
pub(super) fn declaring_macros(text: &str) -> Vec<Declarer> {
    const NOT_TYPES: &[&str] = &[
        "return",
        "delete",
        "throw",
        "sizeof",
        "case",
        "else",
        "do",
        "new",
        "co_return",
        "co_yield",
        "co_await",
        "goto",
    ];
    let is_name = |c: char| c.is_alphanumeric() || c == '_';
    let declares = |body: &str, param: &str| {
        body.match_indices(param).any(|(j, _)| {
            let (Some(head), Some(rest)) = (body.get(..j), body.get(j + param.len()..)) else {
                return false;
            };
            let ty = head.trim_end_matches(|c: char| c.is_whitespace() || c == '*' || c == '&');
            let word = ty.rsplit(|c: char| !is_name(c)).next().unwrap_or_default();
            let template = ty.ends_with('>') && ty.matches('<').count() >= ty.matches('>').count();
            let after = rest.trim_start();
            !head.ends_with(is_name)
                && !rest.starts_with(is_name)
                && (template || !word.is_empty())
                && !NOT_TYPES.contains(&word)
                && (after.is_empty() || after.starts_with([';', '=', ',', '[', ')']))
        })
    };
    (DEFINE.captures_iter(text))
        .filter_map(|c| {
            let declared: Vec<usize> = (c[2].split(',').enumerate())
                .filter(|(_, p)| !p.trim().is_empty() && declares(&c[3], p.trim()))
                .map(|(k, _)| k)
                .collect();
            (!declared.is_empty()).then(|| (c[1].to_owned(), declared))
        })
        .collect()
}
pub(super) fn macro_declared_at(stmt: &str, name: &str, macros: &[Declarer]) -> Option<usize> {
    static CALL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*([A-Za-z_]\w*)\s*\(").expect("the regex is a literal"));
    let call = CALL.captures(stmt)?;
    let (_, declared) = macros.iter().find(|(m, _)| *m == call[1])?;
    let open = call.get(0)?.end() - 1;
    let close = close_or_end(stmt, open, stmt.len());
    if !stmt
        .get(close + 1..)?
        .trim()
        .trim_end_matches(';')
        .trim()
        .is_empty()
    {
        return None;
    }
    let args = stmt.get(open + 1..close)?;
    let (k, (at, arg)) = split_top_level(args, ',')
        .into_iter()
        .enumerate()
        .find(|(_, (_, a))| a.trim() == name)?;
    declared
        .contains(&k)
        .then(|| open + 1 + at + arg.len() - arg.trim_start().len())
}
pub fn c_local_typedef(text: &str, at: CPlace, name: &str) -> Option<CType> {
    let code = c_code(text);
    let pos = line_starts(&code).get(at.line1.checked_sub(1)?)? + at.byte_col;
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_';
    code.get(..pos)?
        .match_indices(name)
        .filter(|&(i, _)| {
            code.get(..i)
                .is_some_and(|h| h.ends_with(|c: char| !is_name(c)))
                && code
                    .get(i + name.len()..)
                    .is_some_and(|r| r.trim_start().starts_with(';'))
                && c_innermost_open_brace(code.as_bytes(), i)
                    .is_some_and(|open| close_or_end(&code, open, code.len()) > pos)
        })
        .filter_map(|(i, _)| c_type_def(&code, i))
        .last()
        .filter(|t| matches!(t, CType::Name(_)))
}
pub fn c_declares_where<'a, S: AsRef<str> + 'a>(
    line1: usize,
    text: &str,
    word: &str,
    lines: impl FnOnce() -> &'a [S],
) -> bool {
    static ENUMERATORS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(&c_enumerators(None)).expect("the regex is a literal"));
    static MEMBER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(&c_member_decl(None)).expect("the regex is a literal"));
    if !text.trim_start().starts_with('#') && !c_names_in_code(text, word) {
        return false;
    }
    if ENUMERATORS.is_match(text) {
        return c_body_head(lines(), line1).is_some_and(|h| c_enum_head(&h));
    }
    let t = text.trim_start();
    if MEMBER.is_match(text) && !t.starts_with("typedef") && !t.starts_with("friend") {
        return c_class_around(lines(), line1).is_some();
    }
    true
}
fn c_names_in_code(text: &str, word: &str) -> bool {
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let code = c_code(text);
    code.match_indices(word).any(|(i, _)| {
        !code.get(..i).is_some_and(|h| h.ends_with(is_name))
            && !code
                .get(i + word.len()..)
                .is_some_and(|r| r.starts_with(is_name))
    })
}
