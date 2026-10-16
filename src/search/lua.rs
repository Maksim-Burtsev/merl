use std::ops::Range;

use regex::Regex;

use super::{Binding, Kind, Value, bindings, continued, indent, literal_lines, uncommented};

pub(super) fn lua_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    static LOCAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^local\s+(?:function\s+(\w+)|([\w\s,<>]+?)\s*(?:=|$))")
            .expect("the regex is a literal")
    });
    static PARAMS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bfunction\b[^(]*\(([^)]*)\)").expect("the regex is a literal")
    });
    static FOR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^for\s+([\w\s,]+?)\s*(?:\bin\b|=)").expect("the regex is a literal")
    });
    let lists = |list: &str| {
        list.split(',')
            .any(|n| n.split('<').next().is_some_and(|n| n.trim() == name))
    };
    let literal = literal_lines(Kind::Lua, &lines.join("\n"));
    let found = |line: usize| {
        vec![Binding {
            line1: line,
            value: Value::Unknown,
        }]
    };
    let mut depth = indent(lines[at]);
    let own = uncommented(Kind::Lua, lines[at]);
    if own
        .trim_start()
        .strip_prefix("until")
        .is_some_and(|r| !r.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
        && let Some(body) = (0..at).rev().find(|&i| !lines[i].trim().is_empty())
    {
        depth = depth.max(indent(lines[body]));
    }
    for i in (0..at).rev() {
        let code = uncommented(Kind::Lua, lines[i]);
        let t = code.trim();
        let ind = indent(lines[i]);
        if depth == 0 {
            break;
        }
        if t.is_empty() || literal[i] || ind > depth {
            continue;
        }
        if ind == depth {
            if let Some(c) = LOCAL.captures(t)
                && (c.get(1).is_some_and(|f| f.as_str() == name)
                    || c.get(2).is_some_and(|l| lists(l.as_str())))
            {
                return found(i + 1);
            }
            continue;
        }
        depth = ind;
        let own = LOCAL
            .captures(t)
            .and_then(|c| c.get(1))
            .is_some_and(|f| f.as_str() == name);
        let params = PARAMS.captures_iter(t).any(|c| lists(&c[1]));
        let vars = FOR.captures(t).is_some_and(|c| lists(&c[1]));
        if own || params || vars {
            return found(i + 1);
        }
    }
    Vec::new()
}
pub fn table_key(text: &str, line: usize, range: &Range<usize>) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(l) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let (Some(after), Some(before)) = (l.get(range.end..), l.get(..range.start)) else {
        return false;
    };
    let (after, before) = (after.trim_start(), before.trim_end());
    after.starts_with('=')
        && !after.starts_with("==")
        && (before.ends_with(['{', ','])
            || (before.is_empty() && continued(Kind::Lua, &lines, line - 1)))
}
pub fn lua_local_value(text: &str, line: usize, name: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let re = Regex::new(&format!(r"^\s*local\s+{}\s*=\s*(.+)", regex::escape(name)))
        .expect("an escaped name keeps the pattern valid");
    let at = if let Some(b) = bindings(Kind::Lua, text, line, name).first() {
        b.line1 - 1
    } else {
        let own = line
            .checked_sub(1)
            .and_then(|i| lines.get(i))
            .map(|l| uncommented(Kind::Lua, l))
            .unwrap_or_default();
        let word = Regex::new(&format!(r"\b{}\b", regex::escape(name)))
            .expect("an escaped name keeps the pattern valid");
        if Regex::new(r"\b(?:for|function)\b")
            .expect("a fixed pattern compiles")
            .find(&own)
            .is_some_and(|m| own.get(m.end()..).is_some_and(|r| word.is_match(r)))
        {
            return None;
        }
        let literal = literal_lines(Kind::Lua, text);
        (0..line.saturating_sub(1).min(lines.len()))
            .rev()
            .find(|&i| !literal[i] && indent(lines[i]) == 0 && re.is_match(lines[i]))?
    };
    let code = uncommented(Kind::Lua, lines[at]);
    Some(re.captures(&code)?[1].trim().to_owned())
}
pub fn lua_required(value: &str) -> Option<&str> {
    static REQUIRE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"^require\s*\(?\s*["']([\w./-]+)["']\s*\)?$"#).expect("the regex is a literal")
    });
    Some(REQUIRE.captures(value)?.get(1)?.as_str())
}
pub fn lua_returned(text: &str) -> Option<String> {
    let literal = literal_lines(Kind::Lua, text);
    text.lines()
        .zip(literal)
        .filter(|(l, inside)| !inside && indent(l) == 0)
        .filter_map(|(l, _)| {
            let t = uncommented(Kind::Lua, l);
            let t = t
                .trim()
                .strip_prefix("return ")?
                .trim()
                .trim_end_matches(';');
            (!t.is_empty() && t.chars().all(|c| c.is_alphanumeric() || c == '_'))
                .then(|| t.to_owned())
        })
        .last()
}
pub fn lua_table(text: &str, owner: Option<&str>, name: &str) -> Option<String> {
    let end = text.lines().count() + 1;
    if lua_local_value(text, end, name).is_some_and(|v| v.starts_with('{')) {
        return Some(name.to_owned());
    }
    let path = format!("{}.{name}", owner?);
    let re = Regex::new(&format!(r"^{}\s*=\s*\{{", regex::escape(&path)))
        .expect("an escaped name keeps the pattern valid");
    let literal = literal_lines(Kind::Lua, text);
    text.lines()
        .zip(literal)
        .any(|(l, inside)| !inside && re.is_match(l))
        .then_some(path)
}
pub fn lua_members(text: &str, table: &str, word: &str) -> Vec<usize> {
    let (t, w) = (regex::escape(table), regex::escape(word));
    let re = Regex::new(&format!(
        r"^\s*(?:function\s+{t}[.:]{w}\s*\(|{t}\.{w}\s*=\s*function\b)"
    ))
    .expect("an escaped name keeps the pattern valid");
    let literal = literal_lines(Kind::Lua, text);
    text.lines()
        .zip(literal)
        .enumerate()
        .filter(|(_, (l, inside))| !inside && re.is_match(l))
        .map(|(i, _)| i + 1)
        .collect()
}
