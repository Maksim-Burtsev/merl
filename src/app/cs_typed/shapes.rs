use crate::app::{Hit, Regex, search};

pub(super) fn plain_of<'h>(plain: bool, many: &[&'h Hit]) -> Vec<&'h Hit> {
    (many.iter().copied())
        .filter(|h| plain && search::cs_type_decl(&h.text).is_some_and(|d| !d.generic))
        .collect()
}

pub(super) fn partial_shape(name: &str) -> Regex {
    Regex::new(&format!(
        r"\bpartial\b.*?\b{}\s*(<[^>]*>)?\s*(\()?",
        regex::escape(name)
    ))
    .expect("an escaped name keeps the pattern valid")
}

pub(super) fn part_shape(shape: &Regex, header: &str) -> Option<(String, bool)> {
    let c = shape.captures(header)?;
    let generics = c.get(1).map_or("", |m| m.as_str());
    Some((
        generics.replace(char::is_whitespace, ""),
        c.get(2).is_some(),
    ))
}

pub(super) fn one_namespace(text: &str, line1: Option<usize>) -> bool {
    let mut namespaces = text.lines().filter(|l| search::cs_namespace_line(l));
    match (namespaces.next(), namespaces.next()) {
        (None, _) => true,
        (Some(only), None) => {
            only.trim_end().ends_with(';')
                || line1.is_none_or(|n| {
                    (text.lines().nth(n - 1)).is_some_and(|l| l.starts_with([' ', '\t']))
                })
        }
        _ => false,
    }
}
