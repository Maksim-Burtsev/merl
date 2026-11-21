use super::super::{Kind, close_of, split_top};

pub fn swift_tuple_item(written: &str, at: usize) -> Option<String> {
    let inner = written.trim().strip_prefix('(')?.strip_suffix(')')?;
    let items = split_top(Kind::Swift, inner, b',');
    let item = items.get(at).filter(|_| items.len() > 1)?.trim();
    let item = match item.split_once(':') {
        Some((label, t))
            if label
                .trim()
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_') =>
        {
            t
        }
        _ => item,
    };
    Some(item.trim().to_owned())
}
pub fn swift_tuple_field(written: &str, label: &str) -> Option<String> {
    let w = written.trim();
    let inner = w.get(
        1..close_of(Kind::Swift, w, 0).filter(|&end| end == w.len() && w.starts_with('('))? - 1,
    )?;
    let items = split_top(Kind::Swift, inner, b',');
    let [one] = <[&str; 1]>::try_from(
        items
            .iter()
            .filter_map(|i| i.split_once(':'))
            .filter(|(l, _)| l.trim() == label)
            .map(|(_, t)| t.trim())
            .collect::<Vec<_>>(),
    )
    .ok()?;
    (items.len() > 1).then(|| one.to_owned())
}
