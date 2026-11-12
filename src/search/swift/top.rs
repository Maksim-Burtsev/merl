use regex::Regex;

use super::super::{Binding, Kind, Value, indent};
use super::{Binds, swift_code, swift_statement_binds};

pub fn swift_bindings_or_top(
    text: &str,
    lines: &[&str],
    top: Option<&[bool]>,
    line1: usize,
    name: &str,
) -> Vec<Binding> {
    let bound = super::super::bindings(Kind::Swift, text, line1, name);
    match top {
        Some(literal) if bound.is_empty() => swift_top_level(lines, literal, line1, name)
            .into_iter()
            .collect(),
        _ => bound,
    }
}
fn swift_top_level(lines: &[&str], literal: &[bool], line1: usize, name: &str) -> Option<Binding> {
    if indent(lines.get(line1.checked_sub(1)?)?) > 0 {
        return None;
    }
    let word = Regex::new(&format!(r"(?:^|[^\w.]){}\b", regex::escape(name))).ok()?;
    let mut bound = (0..lines.len())
        .filter(|&i| i + 1 != line1 && indent(lines[i]) == 0 && swift_code(lines, literal, i))
        .filter_map(
            |i| match swift_statement_binds(lines, line1 - 1, i, name, &word) {
                (Binds::Name, line) => Some(Some(line)),
                (Binds::UnreadablePatternMentionsName, _) => Some(None),
                (Binds::Nothing, _) => None,
            },
        );
    let line = bound.next()??;
    (bound.next().is_none() && line < line1).then_some(Binding {
        line1: line,
        value: Value::Unknown,
    })
}
pub fn swift_member_scope_top(
    lines: &[&str],
    literal: &[bool],
    line1: usize,
    at: usize,
    name: &str,
) -> Option<usize> {
    let mut top = match super::swift_scope(lines, literal, line1 + 1).header_line1 {
        h if h == line1 && h > at && line1 < lines.len() => line1,
        _ if line1 > at && super::swift_func_decl(lines[line1 - 1]) => {
            if super::swift_binds_on(lines[line1 - 1], name) {
                return None;
            }
            line1
        }
        _ => super::swift_scope(lines, literal, line1).header_line1,
    };
    while top > at {
        match super::swift_scope(lines, literal, top).header_line1 {
            up if up > at => top = up,
            _ => break,
        }
    }
    Some(top)
}
