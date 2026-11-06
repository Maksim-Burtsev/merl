use super::c::{Head, c_code, close_or_end, line_starts, scopes_inside_out_to_type_or_namespace};
use regex::Regex;

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
