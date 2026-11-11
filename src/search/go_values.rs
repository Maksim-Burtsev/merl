use regex::Regex;

use super::{GO_SHORT, Kind, Value, go_assigned, names, split_top, value_of};

pub(super) fn go_statement_value(t: &str, line: usize, name: &str) -> Option<Value> {
    static GO_VAR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"^var\s+([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)*)(?:\s+([^=]+?))?\s*(?:=\s*(.*))?$",
        )
        .expect("the regex is a literal")
    });
    let list = |s: &str| -> Vec<String> { s.split(',').map(|p| p.trim().to_owned()).collect() };
    let anonymous = |e: &str| {
        e.trim_start()
            .trim_start_matches('&')
            .strip_prefix("struct")
            .is_some_and(|r| r.trim_start().starts_with('{'))
    };
    let value = |e: &str| {
        if anonymous(e) {
            Value::Struct(line)
        } else {
            value_of(Kind::Go, e)
        }
    };
    let assigned = |names: &str, values: &str, at: usize| match go_assigned(names, values, at) {
        Some(v) => value(v),
        None if split_top(Kind::Go, values, b',').len() == 1 => Value::Position {
            of: Box::new(value_of(Kind::Go, values)),
            at,
            count: names.split(',').count(),
        },
        None => Value::Unknown,
    };
    if let Some(c) = GO_SHORT.captures(t) {
        let at = list(&c[1]).iter().position(|p| p == name)?;
        Some(assigned(&c[1], &c[2], at))
    } else if let Some(c) = GO_VAR.captures(t) {
        let at = list(&c[1]).iter().position(|p| p == name)?;
        Some(match (c.get(2), c.get(3)) {
            (Some(ty), _) if anonymous(ty.as_str()) => Value::Struct(line),
            (Some(ty), _) => Value::Type(ty.as_str().to_owned()),
            (None, Some(v)) => assigned(&c[1], v.as_str(), at),
            (None, None) => Value::Unknown,
        })
    } else {
        t.strip_prefix("const ")
            .is_some_and(|rest| names(rest.split('=').next().unwrap_or(rest), name))
            .then_some(Value::Unknown)
    }
}
