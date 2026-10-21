use std::ops::Range;
use std::path::PathBuf;

use regex::Regex;

use super::{Binding, Kind, Value, bindings, continued, indent, literal_lines, uncommented};

pub(super) fn lua_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    lua_walk(lines, at, name).map_or_else(
        |_| Vec::new(),
        |line1| {
            vec![Binding {
                line1,
                value: Value::Unknown,
            }]
        },
    )
}
pub fn lua_value_of_its_own(text: &str, line: usize, name: &str) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let at = line.checked_sub(1).filter(|&i| i < lines.len())?;
    lua_walk(&lines, at, name).err().flatten()
}
fn lua_walk(lines: &[&str], at: usize, name: &str) -> Result<usize, Option<usize>> {
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
    let mut unseen = None;
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
    let binds_here = lua_binds_on_its_line(lines[at], name);
    for i in (0..at).rev() {
        let code = uncommented(Kind::Lua, lines[i]);
        let t = code.trim();
        let ind = indent(lines[i]);
        if depth == 0 && binds_here {
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
                return Ok(i + 1);
            }
            continue;
        }
        depth = ind;
        let local = LOCAL.captures(t);
        let own = local
            .as_ref()
            .and_then(|c| c.get(1))
            .is_some_and(|f| f.as_str() == name);
        let params = PARAMS.captures_iter(t).any(|c| lists(&c[1]));
        let vars = FOR.captures(t).is_some_and(|c| lists(&c[1]));
        if own || params || vars {
            return Ok(i + 1);
        }
        if local.and_then(|c| c.get(2)).is_some_and(|l| {
            lists(l.as_str())
                && t.get(l.end()..)
                    .is_some_and(|r| r.trim_start().starts_with('='))
        }) {
            unseen = unseen.or(Some(i + 1));
        }
    }
    Err(unseen)
}
fn lua_binds_on_its_line(line: &str, name: &str) -> bool {
    static HEAD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:for|function)\b").expect("the regex is a literal")
    });
    let own = uncommented(Kind::Lua, line);
    let word = Regex::new(&format!(r"\b{}\b", regex::escape(name)))
        .expect("an escaped name keeps the pattern valid");
    HEAD.find(&own)
        .is_some_and(|m| own.get(m.end()..).is_some_and(|r| word.is_match(r)))
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
    lua_local_at(text, line, name).map(|(_, v)| v)
}
pub fn lua_table_value(value: &str) -> bool {
    static MADE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:\{|setmetatable\s*\(\s*\{)").expect("the regex is a literal")
    });
    MADE.is_match(value)
}
pub fn lua_local_at(text: &str, line: usize, name: &str) -> Option<(usize, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let re = Regex::new(&format!(r"^\s*local\s+{}\s*=\s*(.+)", regex::escape(name)))
        .expect("an escaped name keeps the pattern valid");
    let at = if let Some(b) = bindings(Kind::Lua, text, line, name).first() {
        b.line1 - 1
    } else {
        if line
            .checked_sub(1)
            .and_then(|i| lines.get(i))
            .is_some_and(|l| lua_binds_on_its_line(l, name))
        {
            return None;
        }
        let literal = literal_lines(Kind::Lua, text);
        (0..line.saturating_sub(1).min(lines.len()))
            .rev()
            .find(|&i| !literal[i] && indent(lines[i]) == 0 && re.is_match(lines[i]))?
    };
    let code = uncommented(Kind::Lua, lines[at]);
    Some((at + 1, re.captures(&code)?[1].trim().to_owned()))
}
pub fn lua_std_receiver(text: &str, line: usize, before: &str) -> Option<&'static str> {
    static LITERAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"\(\s*(?:"[^"]*"|'[^']*')\s*\)\s*:$"#).expect("the regex is a literal")
    });
    static LIB: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.:])(\w+)\s*[.:]$").expect("the regex is a literal")
    });
    const STRINGS: [(&str, &str); 11] = [
        ("", "tostring"),
        ("", "type"),
        ("string", "format"),
        ("string", "rep"),
        ("string", "sub"),
        ("string", "upper"),
        ("string", "lower"),
        ("string", "reverse"),
        ("string", "char"),
        ("string", "gsub"),
        ("table", "concat"),
    ];
    let unbound = |name: &str| {
        bindings(Kind::Lua, text, line, name).is_empty()
            && lua_local_value(text, line, name).is_none()
    };
    let before = before.trim_end();
    if LITERAL.is_match(before) {
        return Some("string");
    }
    if let Some(call) = before
        .strip_suffix(':')
        .map(str::trim_end)
        .filter(|c| c.ends_with(')'))
    {
        let head = call.get(..lua_open_paren(call)?)?.trim_end();
        let owner = head.trim_end_matches(|c: char| c.is_alphanumeric() || c == '_');
        let callee = head.get(owner.len()..)?;
        let lib = if owner.ends_with(['.', ':']) {
            lua_std_receiver(text, line, owner)?
        } else if unbound(callee) {
            ""
        } else {
            return None;
        };
        return STRINGS.contains(&(lib, callee)).then_some("string");
    }
    let written = LIB.captures(before)?.get(1)?.as_str();
    let lib = [
        "string",
        "table",
        "math",
        "os",
        "io",
        "coroutine",
        "debug",
        "utf8",
    ]
    .into_iter()
    .find(|&l| l == written)?;
    unbound(lib).then_some(lib)
}
fn lua_open_paren(call: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in call.char_indices().rev() {
        match c {
            ')' => depth += 1,
            '(' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}
pub fn lua_required(value: &str) -> Option<&str> {
    static REQUIRE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"^require\s*\(?\s*["']([\w./-]+)["']\s*\)?$"#).expect("the regex is a literal")
    });
    Some(REQUIRE.captures(value)?.get(1)?.as_str())
}
pub fn lua_require_path(line: &str, col: usize) -> Option<String> {
    static REQUIRE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"(?:^|[^\w.:])require\s*\(?\s*(["'])([\w./-]+)["']"#)
            .expect("the regex is a literal")
    });
    REQUIRE.captures_iter(line).find_map(|c| {
        let path = c.get(2)?;
        (c.get(1)?.start() <= col && col <= path.end()).then(|| path.as_str().to_owned())
    })
}
pub fn lua_module_file(module: &str, files: &[PathBuf]) -> Option<PathBuf> {
    let dir = module.replace('.', "/");
    ["", "lua/"]
        .iter()
        .flat_map(|root| [format!("{root}{dir}.lua"), format!("{root}{dir}/init.lua")])
        .map(PathBuf::from)
        .find(|p| files.contains(p))
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
    if lua_local_value(text, end, name).is_some_and(|v| lua_table_value(&v)) {
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
        r"^\s*(?:function\s+{t}[.:]{w}\s*\(|{t}\.{w}\s*=\s*(?:function\b|([A-Za-z_]\w*)\s*;?\s*$))"
    ))
    .expect("an escaped name keeps the pattern valid");
    let literal = literal_lines(Kind::Lua, text);
    text.lines()
        .zip(literal)
        .enumerate()
        .filter(|(_, (l, inside))| {
            !inside
                && re
                    .captures(l.split("--").next().unwrap_or(l))
                    .is_some_and(|c| {
                        c.get(1).is_none_or(|v| {
                            !matches!(v.as_str(), "nil" | "true" | "false") && v.as_str() != word
                        })
                    })
        })
        .map(|(i, _)| i + 1)
        .collect()
}
pub fn lua_handed_on(text: &str, table: &str, word: &str) -> Option<usize> {
    let (t, w) = (regex::escape(table), regex::escape(word));
    let re = Regex::new(&format!(r"^{t}\.{w}\s*=\s*{w}\s*;?$"))
        .expect("an escaped name keeps the pattern valid");
    let literal = literal_lines(Kind::Lua, text);
    let lines = text.lines().count();
    text.lines()
        .zip(literal)
        .any(|(l, inside)| !inside && re.is_match(uncommented(Kind::Lua, l).trim_end()))
        .then(|| {
            bindings(Kind::Lua, text, lines, word)
                .first()
                .map(|b| b.line1)
        })
        .flatten()
}
pub fn lua_self_table(text: &str, line: usize) -> Option<String> {
    static METHOD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^function\s+([A-Za-z_][\w.]*)(?::\w+\s*\(|\.\w+\s*\(\s*self\s*[,)])")
            .expect("the regex is a literal")
    });
    static PARAMS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bfunction\b[^(]*\(([^)]*)\)").expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let at = line.checked_sub(1).filter(|&i| i < lines.len())?;
    let literal = literal_lines(Kind::Lua, text);
    let mut depth = indent(lines[at]);
    for i in (0..at).rev() {
        let code = uncommented(Kind::Lua, lines[i]);
        let t = code.trim();
        if depth == 0 {
            break;
        }
        if t.is_empty() || literal[i] || indent(lines[i]) >= depth {
            continue;
        }
        depth = indent(lines[i]);
        if let Some(c) = METHOD.captures(t) {
            return Some(c[1].to_owned());
        }
        if PARAMS
            .captures_iter(t)
            .any(|c| c[1].split(',').any(|p| p.trim() == "self"))
        {
            return None;
        }
    }
    None
}
pub fn lua_makes_instance(text: &str, line: usize, table: &str) -> bool {
    let t = regex::escape(table);
    let set = Regex::new(&format!(r"\bsetmetatable\s*\(.*,\s*{t}\s*\)"))
        .expect("an escaped name keeps the pattern valid");
    let index = Regex::new(&format!(r"^{t}\.__index\s*=\s*{t}\s*;?$"))
        .expect("an escaped name keeps the pattern valid");
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(Kind::Lua, text);
    let code = |i: usize| (!literal[i]).then(|| uncommented(Kind::Lua, lines[i]));
    let Some(head) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let body = (head + 1..lines.len())
        .take_while(|&i| lines[i].trim().is_empty() || indent(lines[i]) > indent(lines[head]));
    let sets = std::iter::once(head)
        .chain(body)
        .any(|i| code(i).is_some_and(|c| set.is_match(&c)));
    sets && (0..lines.len()).any(|i| code(i).is_some_and(|c| index.is_match(c.trim_end())))
}
pub fn lua_goto_label(text: &str, line: usize, before: &str, name: &str) -> Option<Vec<usize>> {
    static GOTO: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.:])goto\s+$").expect("the regex is a literal")
    });
    static FUNCTION: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\bfunction\b").expect("the regex is a literal"));
    if !GOTO.is_match(&uncommented(Kind::Lua, before)) {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();
    let at = line.checked_sub(1).filter(|&i| i < lines.len())?;
    let literal = literal_lines(Kind::Lua, text);
    let code: Vec<String> = lines
        .iter()
        .zip(&literal)
        .map(|(l, inside)| {
            if *inside {
                String::new()
            } else {
                uncommented(Kind::Lua, l).trim_end().to_owned()
            }
        })
        .collect();
    let mut low = indent(lines[at]);
    let mut head = None;
    for i in (0..at).rev().filter(|&i| !code[i].trim().is_empty()) {
        if indent(lines[i]) < low {
            low = indent(lines[i]);
            if FUNCTION.is_match(&code[i]) {
                head = Some(i);
                break;
            }
        }
    }
    let start = head.map_or(0, |h| h + 1);
    let end = head.map_or(lines.len(), |h| {
        (h + 1..lines.len())
            .find(|&i| !code[i].trim().is_empty() && indent(lines[i]) <= indent(lines[h]))
            .unwrap_or(lines.len())
    });
    let label = format!("::{name}::");
    let visible = |j: usize| {
        let depth = indent(lines[j]);
        let (from, to) = if j < at { (j + 1, at) } else { (at + 1, j) };
        depth <= indent(lines[at])
            && (from..to).all(|i| code[i].trim().is_empty() || indent(lines[i]) >= depth)
    };
    Some(
        (start..end)
            .filter(|&j| j != at && code[j].trim() == label && visible(j))
            .map(|j| j + 1)
            .collect(),
    )
}
pub fn lua_returns_self(text: &str, line: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(Kind::Lua, text);
    let Some(head) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    (head + 1..lines.len())
        .take_while(|&i| lines[i].trim().is_empty() || indent(lines[i]) > indent(lines[head]))
        .filter(|&i| !literal[i])
        .map(|i| uncommented(Kind::Lua, lines[i]))
        .filter(|c| !c.trim().is_empty())
        .last()
        .is_some_and(|c| c.trim().trim_end_matches(';').trim_end() == "return self")
}
fn lua_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
pub fn lua_name_path(value: &str) -> Option<Vec<String>> {
    let v = value.trim().trim_end_matches(';').trim_end();
    (v.starts_with(|c: char| c.is_alphabetic() || c == '_')
        && v.split('.')
            .all(|p| !p.is_empty() && p.chars().all(lua_name_char)))
    .then(|| v.split('.').map(str::to_owned).collect())
}
pub fn lua_method_step(value: &str) -> Option<(&str, &str)> {
    let v = value.trim().trim_end_matches(';').trim_end();
    if !v.ends_with(')') {
        return None;
    }
    let head = v.get(..lua_open_paren(v)?)?.trim_end();
    let receiver = head.trim_end_matches(lua_name_char);
    let method = head.get(receiver.len()..)?;
    let receiver = receiver.strip_suffix(':')?.trim_end();
    (!method.is_empty() && !receiver.is_empty() && !receiver.ends_with(':'))
        .then_some((receiver, method))
}
pub fn lua_called_receiver(before: &str) -> Option<&str> {
    let path_start = |s: &str| {
        let p = s
            .trim_end_matches(|c: char| lua_name_char(c) || c == '.')
            .len();
        p + s.get(p..).and_then(|r| r.rfind("..")).map_or(0, |i| i + 2)
    };
    let call = before.trim_end().strip_suffix(':')?.trim_end();
    let mut end = call.len();
    loop {
        let part = call.get(..end)?;
        if !part.ends_with(')') {
            return None;
        }
        let head = part.get(..lua_open_paren(part)?)?.trim_end();
        let mut start = path_start(head);
        let named = head.get(start..)?;
        if named.is_empty() || LUA_KEYWORDS.contains(&named) {
            return None;
        }
        match head.get(..start)?.strip_suffix(':').map(str::trim_end) {
            Some(r) if r.ends_with(')') => {
                end = r.len();
                continue;
            }
            Some(r) => start = path_start(r),
            None => {}
        }
        let expr = call.get(start..)?;
        return (!call.get(..start)?.ends_with([')', ']', ':', '"', '\''])
            && expr.starts_with(|c: char| c.is_alphabetic() || c == '_'))
        .then_some(expr);
    }
}
const LUA_KEYWORDS: [&str; 13] = [
    "return", "and", "or", "not", "if", "then", "elseif", "else", "while", "do", "until", "in",
    "local",
];
pub fn lua_call(value: &str) -> Option<(Vec<String>, String)> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^([A-Za-z_][\w.]*)[.:](\w+)\s*\([^()]*\)$").expect("the regex is a literal")
    });
    let c = CALL.captures(value.trim().trim_end_matches(';'))?;
    let chain = c[1].split('.').map(str::to_owned).collect();
    Some((chain, c[2].to_owned()))
}
