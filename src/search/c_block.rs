use super::c::{
    Head, c_code, close_or_end, line_starts, scope_pos, scopes_inside_out_to_type_or_namespace,
};

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
    let at = scope_pos(&code, &starts, from1, name);
    let reach = |line1: usize| {
        let open = scope_pos(&code, &starts, line1, name).and_then(|pos| {
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
