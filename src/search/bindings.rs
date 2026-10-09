use std::ops::Range;

use regex::Regex;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Type(String),
    New(String),
    Call(String),
    Name(String),
    Class {
        decl_line1: usize,
    },
    Cast {
        callee: String,
        written_type: String,
    },
    Element(String),
    Field {
        chain: Vec<String>,
        field: String,
    },
    Member(Box<Value>, String),
    Struct(usize),
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub line1: usize,
    pub value: Value,
}
pub(super) fn value_of(kind: Kind, expr: &str) -> Value {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:await\s+)?(new\s+)?([A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*)*)\s*(?:<[^()]*>)?\s*\(").unwrap()
    });
    static LITERAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(&)?([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)?)\s*(?:\[[^\]]*\])?\s*\{").unwrap()
    });
    static NAME: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^[A-Za-z_$][\w$]*$").unwrap());
    static GO_STRUCTS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^((?:\[[^\]]*\]|map\[[^\]]*\])+)\s*struct\s*\{").unwrap()
    });
    static ASSERTION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^[A-Za-z_][\w.]*\.\(\s*(\*?[A-Za-z_][\w.]*)\s*\)$").unwrap()
    });
    let e = uncommented(kind, expr);
    let e = e.trim().trim_end_matches(';').trim_end();
    let ends = |open: usize| close_of(kind, e, open).is_none_or(|end| e[end..].trim().is_empty());
    if kind == Kind::TsJs {
        let mut depth = 0i32;
        let mut cast: Option<(usize, usize)> = None;
        for (i, c) in code(kind, e) {
            match c {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                b' ' if depth == 0 && e[i..].starts_with(" as ") => {
                    cast = Some((cast.map_or(i, |(first, _)| first), i + 4));
                }
                _ => {}
            }
        }
        let operand = |head: &str| {
            let head = head.trim_start_matches("await ").trim_start_matches("new ");
            let mut depth = 0i32;
            code(kind, head).all(|(_, c)| {
                match c {
                    b'(' | b'[' | b'{' => depth += 1,
                    b')' | b']' | b'}' => depth -= 1,
                    _ => {}
                }
                c != b' ' || depth > 0
            })
        };
        if let Some((first, i)) = cast {
            return match operand(&e[..first]) {
                true => Value::Type(e[i..].trim().to_owned()),
                false => Value::Unknown,
            };
        }
    }
    if kind == Kind::Go
        && let Some(c) = ASSERTION.captures(e)
    {
        return Value::Type(c[1].to_owned());
    }
    if let Some(c) = CALL.captures(e) {
        let open = c.get(0).unwrap().end() - 1;
        let name = c[2].to_owned();
        let args = close_of(kind, e, open).map(|end| e[open + 1..end - 1].trim());
        return match (ends(open), c.get(1).is_some()) {
            (false, _) => Value::Unknown,
            (true, true) if kind == Kind::TsJs => Value::New(name),
            (true, false) if kind == Kind::Go && name == "new" => match args {
                Some(inner) => Value::New(inner.to_owned()),
                None => Value::Unknown,
            },
            (true, false) if kind == Kind::Python && matches!(&*name, "cast" | "typing.cast") => {
                match args {
                    Some(inner) => Value::Cast {
                        callee: name,
                        written_type: split_top(kind, inner, b',')[0].trim().to_owned(),
                    },
                    None => Value::Unknown,
                }
            }
            (true, false) if kind == Kind::Go && name == "make" => match args {
                Some(inner) => Value::Type(split_top(kind, inner, b',')[0].trim().to_owned()),
                None => Value::Unknown,
            },
            (true, false) => Value::Call(name),
            (true, true) => Value::Unknown,
        };
    }
    if kind == Kind::Go
        && let Some(c) = LITERAL.captures(e)
        && ends(c.get(0).unwrap().end() - 1)
    {
        return Value::New(c[2].to_owned());
    }
    if kind == Kind::Go
        && let Some(c) = GO_STRUCTS.captures(e)
    {
        return Value::Type(format!("{}struct", &c[1]));
    }
    if kind == Kind::Go
        && (e.starts_with('[') || e.starts_with("map["))
        && let Some(end) = e.find('[').and_then(|i| close_of(kind, e, i))
        && let Some(open) = e[end..].find('{').map(|i| end + i)
        && ends(open)
    {
        return Value::Type(e[..open].trim().to_owned());
    }
    if NAME.is_match(e) && !matches!(e, "None" | "null" | "undefined" | "nil" | "this" | "self") {
        return Value::Name(e.to_owned());
    }
    Value::Unknown
}
pub fn bindings(kind: Kind, text: &str, line: usize, name: &str) -> Vec<Binding> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    match kind {
        Kind::Python | Kind::Starlark => python_bindings(&lines, at, name),
        Kind::TsJs | Kind::Go | Kind::CSharp => block_bindings(kind, &lines, at, name),
        Kind::Lua => lua_bindings(&lines, at, name),
        Kind::Shell => shell_bindings(&lines, at, name),
        Kind::PowerShell => powershell_params(&lines, at, name),
        Kind::Zig => zig_bindings(&lines, at, name),
        Kind::C => c_bindings(text, line, name),
        Kind::Jvm => jvm_bindings(&lines, at, name),
        Kind::Swift => swift_bindings(&lines, at, name),
        Kind::Ruby => ruby_bindings(&lines, at, name),
        Kind::Rust => rust_bindings(&lines, at, name),
        Kind::Elixir => elixir_bindings(&lines, at, name),
        Kind::Nix => nix_bindings(&lines, at, name),
        Kind::Haskell => haskell_bindings(&lines, at, name),
        Kind::Julia => julia_bindings(&lines, at, name),
        Kind::Gdscript => gdscript_bindings(&lines, at, name),
        Kind::Solidity => solidity_bindings(&lines, at, name),
        k if lisp(k) => lisp_bindings(k, &lines, at, None, name),
        _ => Vec::new(),
    }
}
fn elixir_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    static HEAD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^def(?:p|macrop?|guardp?|module|impl|protocol)?\s+[\w.?!]+").unwrap()
    });
    let literal = literal_lines(Kind::Elixir, &lines.join("\n"));
    let code = |i: usize| uncommented(Kind::Elixir, lines[i]).trim().to_owned();
    let found = |line: usize| {
        vec![Binding {
            line1: line,
            value: Value::Unknown,
        }]
    };
    let own = code(at);
    if let Some((h, _)) = own.split_once("->")
        && let Some((_, p)) = h.rsplit_once("fn ")
        && elixir_binds(p, name)
    {
        return found(at + 1);
    }
    let mut depth = indent(lines[at]);
    let mut i = at;
    while i > 0 {
        i -= 1;
        let t = code(i);
        if t.is_empty() || literal[i] || indent(lines[i]) > depth {
            continue;
        }
        if indent(lines[i]) == depth {
            let parts = split_top(Kind::Elixir, &t, b'=');
            if parts.len() > 1
                && !t.contains("->")
                && !parts[1].starts_with('~')
                && elixir_binds(parts[0], name)
            {
                return found(i + 1);
            }
            continue;
        }
        depth = indent(lines[i]);
        if t.starts_with(')') {
            while i > 0 && (lines[i - 1].trim().is_empty() || indent(lines[i - 1]) > depth) {
                i -= 1;
            }
            i = i.saturating_sub(1);
        }
        let head = code(i);
        if let Some(m) = HEAD.find(&head) {
            if let Some((_, tail)) = head.split_once("do:")
                && let Some(h) = tail.strip_suffix("->")
                && let Some((_, p)) = h.rsplit_once("fn ")
                && elixir_binds(p, name)
            {
                return found(i + 1);
            }
            let end = (i..at)
                .find(|&j| code(j).ends_with(" do") || code(j).contains("do:"))
                .unwrap_or(i);
            return (i..=end)
                .find(|&j| match j == i {
                    true => elixir_binds(&head[m.end()..], name),
                    false => elixir_binds(&code(j), name),
                })
                .map_or_else(Vec::new, |j| found(j + 1));
        }
        if head.starts_with("for ") || head.starts_with("with ") {
            let end = (i..at)
                .find(|&j| code(j).ends_with(" do") || code(j).contains("do:"))
                .unwrap_or(i);
            let generates = |j: usize| {
                let c = code(j);
                let c = c
                    .strip_prefix("for ")
                    .or_else(|| c.strip_prefix("with "))
                    .unwrap_or(&c);
                split_top(Kind::Elixir, c, b',').iter().any(|g| {
                    g.split_once("<-")
                        .is_some_and(|(p, _)| elixir_binds(p, name))
                })
            };
            if let Some(j) = (i..=end).find(|&j| generates(j)) {
                return found(j + 1);
            }
        }
        if let Some(h) = t.strip_suffix("->") {
            let h = h.rsplit_once("fn ").map_or(h, |(_, p)| p);
            if elixir_binds(h, name) {
                return found(i + 1);
            }
        }
    }
    Vec::new()
}
fn elixir_binds(p: &str, name: &str) -> bool {
    static DEFAULT: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\\\\[^,)]*").unwrap());
    let p = p.split(" when ").next().unwrap_or(p);
    let p = p.split(", do:").next().unwrap_or(p);
    let p = DEFAULT.replace_all(p, "");
    let word = |c: char| c.is_alphanumeric() || c == '_';
    p.match_indices(name).any(|(i, _)| {
        let before = p[..i].chars().next_back();
        let after = p[i + name.len()..].trim_start();
        !before.is_some_and(|c| word(c) || ".@:^&%".contains(c))
            && !p[i + name.len()..].starts_with(|c: char| word(c) || c == '?' || c == '!')
            && !(after.starts_with(':') && !after.starts_with("::"))
            && !after.starts_with(['(', '.'])
    })
}
pub fn elixir_module_lines(text: &str, line: usize, pattern: &Regex) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    static MODULE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*def(?:module|impl|protocol)\s").unwrap());
    let literal = literal_lines(Kind::Elixir, text);
    let code = |i: usize| !literal[i] && !lines[i].trim().is_empty();
    let mut depth = indent(lines[at]);
    let header = (0..at).rev().find(|&i| {
        if !code(i) || indent(lines[i]) >= depth {
            return false;
        }
        depth = indent(lines[i]);
        MODULE.is_match(lines[i])
    });
    let Some(h) = header else {
        return Vec::new();
    };
    let body: Vec<usize> = (h + 1..lines.len())
        .filter(|&i| code(i))
        .take_while(|&i| indent(lines[i]) > indent(lines[h]))
        .collect();
    let Some(child) = body.first().map(|&i| indent(lines[i])) else {
        return Vec::new();
    };
    body.into_iter()
        .filter(|&i| indent(lines[i]) == child && pattern.is_match(lines[i]))
        .map(|i| i + 1)
        .collect()
}
fn lua_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    static LOCAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^local\s+(?:function\s+(\w+)|([\w\s,<>]+?)\s*(?:=|$))").unwrap()
    });
    static PARAMS: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\bfunction\b[^(]*\(([^)]*)\)").unwrap());
    static FOR: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^for\s+([\w\s,]+?)\s*(?:\bin\b|=)").unwrap());
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
pub(super) static ZIG_FN_OR_TEST_HEAD: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"\bfn\b|^(?:pub\s+)?test\b").unwrap());
fn zig_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let n = regex::escape(name);
    let decl = Regex::new(&format!(r"^(?:comptime\s+)?(?:const|var)\s+{n}\b"))
        .expect("an escaped name keeps the pattern valid");
    let param = Regex::new(&format!(r"(?:^|[(,])\s*(?:comptime\s+|noalias\s+)?{n}\s*:"))
        .expect("an escaped name keeps the pattern valid");
    let literal = literal_lines(Kind::Zig, &lines.join("\n"));
    let code = |i: usize| uncommented(Kind::Zig, lines[i]).trim().to_owned();
    let mut depth = indent(lines[at]);
    let mut out = Vec::new();
    let mut i = at;
    while i > 0 {
        i -= 1;
        let t = code(i);
        if t.is_empty() || literal[i] || indent(lines[i]) > depth {
            continue;
        }
        if indent(lines[i]) == depth {
            if decl.is_match(&t) {
                out.push(Binding {
                    line1: i + 1,
                    value: Value::Unknown,
                });
            }
            continue;
        }
        let end = i;
        depth = indent(lines[i]);
        if t.starts_with(')') {
            while i > 0 && (lines[i - 1].trim().is_empty() || indent(lines[i - 1]) > depth) {
                i -= 1;
            }
            i = i.saturating_sub(1);
        }
        if ZIG_FN_OR_TEST_HEAD.is_match(&code(i)) {
            for j in i..=end {
                if param.is_match(&code(j)) {
                    out.push(Binding {
                        line1: j + 1,
                        value: Value::Unknown,
                    });
                }
            }
            return out;
        }
    }
    Vec::new()
}
pub fn table_key(text: &str, line: usize, range: &Range<usize>) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(l) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let after = l[range.end..].trim_start();
    let before = l[..range.start].trim_end();
    after.starts_with('=')
        && !after.starts_with("==")
        && (before.ends_with(['{', ','])
            || (before.is_empty() && continued(Kind::Lua, &lines, line - 1)))
}
pub fn lua_local_value(text: &str, line: usize, name: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let re = Regex::new(&format!(r"^\s*local\s+{}\s*=\s*(.+)", regex::escape(name)))
        .expect("an escaped name keeps the pattern valid");
    let at = match bindings(Kind::Lua, text, line, name).first() {
        Some(b) => b.line1 - 1,
        None => {
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
                .is_some_and(|m| word.is_match(&own[m.end()..]))
            {
                return None;
            }
            let literal = literal_lines(Kind::Lua, text);
            (0..line.saturating_sub(1).min(lines.len()))
                .rev()
                .find(|&i| !literal[i] && indent(lines[i]) == 0 && re.is_match(lines[i]))?
        }
    };
    let code = uncommented(Kind::Lua, lines[at]);
    Some(re.captures(&code)?[1].trim().to_owned())
}
pub fn lua_required(value: &str) -> Option<&str> {
    static REQUIRE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"^require\s*\(?\s*["']([\w./-]+)["']\s*\)?$"#).unwrap()
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
pub(super) fn continued(kind: Kind, lines: &[&str], i: usize) -> bool {
    lines[..i]
        .iter()
        .rev()
        .map(|l| uncommented(kind, l))
        .find(|t| !t.trim().is_empty() && !comment(kind, t.trim()))
        .is_some_and(|t| t.trim_end().ends_with(['(', '[', '{', ',', '\\']))
}
pub fn written_line<S: AsRef<str>>(lines: &[S], line: usize, name: &str) -> usize {
    let has = |l: &str| names(&uncommented(Kind::TsJs, l).replace("...", " "), name);
    let first = lines[line - 1].as_ref();
    if has(first) {
        return line;
    }
    for (k, l) in lines.iter().enumerate().skip(line) {
        let l = l.as_ref();
        if l.trim().is_empty() {
            continue;
        }
        if has(l) {
            return k + 1;
        }
        if indent(l) <= indent(first) {
            break;
        }
    }
    line
}
fn block_bindings(kind: Kind, lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let this = kind == Kind::TsJs && (name == "this" || name == "super");
    let mut out = Vec::new();
    if !this {
        opener_bindings(kind, &uncommented(kind, lines[at]), at + 1, name, &mut out);
    }
    let literal = literal_lines(kind, &lines.join("\n"));
    let mut depth = indent(lines[at]);
    let mut i = at;
    let mut declared_in_block = false;
    let in_cs_type_body = |i: usize| {
        kind == Kind::CSharp
            && cs_enclosing(lines, i).is_some_and(|e| cs_type_decl(lines[e]).is_some())
    };
    let mut among_cs_members = in_cs_type_body(at);
    let mut arm: Option<Vec<String>> = None;
    let mut first_line_read_whole: Option<usize> = None;
    let mut go_block_statement_at: Option<usize> = None;
    let type_switch = Regex::new(&format!(
        r"^switch\s+(?:[^;{{]*;\s*)?{}\s*:=\s*[^;{{]+\.\(type\)\s*\{{$",
        regex::escape(name)
    ))
    .expect("an escaped name keeps the pattern valid");
    while i > 0 {
        i -= 1;
        let code = uncommented(kind, behind_doc(kind, lines[i]));
        let t = code.trim();
        let ind = indent(lines[i]);
        let label = kind == Kind::Go
            && t.strip_suffix(':').is_some_and(|l| {
                l != "default"
                    && !l.is_empty()
                    && l.chars().all(|c| c.is_alphanumeric() || c == '_')
            });
        let cs_directive = kind == Kind::CSharp && t.starts_with('#');
        if t.is_empty() || comment(kind, t) || ind > depth || literal[i] || label || cs_directive {
            continue;
        }
        if ind == depth {
            if !this && !among_cs_members {
                let before = out.len();
                let opener = (kind == Kind::TsJs && t.starts_with(['}', ']']))
                    .then(|| {
                        (0..i).rev().find(|&j| {
                            let l = lines[j].trim();
                            indent(lines[j]) <= ind && !l.is_empty() && !comment(kind, l)
                        })
                    })
                    .flatten();
                match opener {
                    Some(j) => {
                        let whole: Vec<&str> = lines[j..=i].iter().map(|l| l.trim()).collect();
                        let whole = uncommented(kind, &whole.join("\n")).replace('\n', " ");
                        statement_bindings(kind, &whole, j + 1, name, &mut out);
                        first_line_read_whole = Some(j);
                    }
                    None if first_line_read_whole == Some(i) => {}
                    None => match ts_declarators(lines, i).filter(|_| kind == Kind::TsJs) {
                        Some(each) => {
                            for d in each {
                                statement_bindings(
                                    kind,
                                    &d.as_own_statement,
                                    d.line0 + 1,
                                    name,
                                    &mut out,
                                );
                            }
                        }
                        None => statement_bindings(kind, t, i + 1, name, &mut out),
                    },
                }
                if kind == Kind::Go && out.len() > before {
                    if let Some(k) = go_block_statement_at.filter(|&k| k < before) {
                        out.remove(k);
                    }
                    go_block_statement_at = Some(out.len() - 1);
                }
                if let Some(types) = arm.as_deref().filter(|_| type_switch.is_match(t)) {
                    let value = match types {
                        [one] => Value::Type(one.clone()),
                        _ => Value::Unknown,
                    };
                    out.push(Binding {
                        line1: i + 1,
                        value,
                    });
                }
                if t.starts_with("switch ") || t.starts_with("select ") {
                    arm = None;
                }
                declared_in_block |= out.len() > before;
            }
            continue;
        }
        let end = i;
        let line_back_at_indent = (0..i)
            .rev()
            .find(|&j| !lines[j].trim().is_empty() && indent(lines[j]) <= ind);
        let body = t == "{" && line_back_at_indent.is_some_and(|j| declares_type(kind, lines[j]));
        let closes_type_params = kind == Kind::TsJs
            && t.starts_with('>')
            && line_back_at_indent.is_some_and(|j| lines[j].trim_end().ends_with('<'));
        let header_above = closes_type_params
            || (kind == Kind::TsJs && body)
            || (kind == Kind::CSharp && t == "{");
        if t.starts_with([')', '}', ']']) || header_above {
            while i > 0 && (lines[i - 1].trim().is_empty() || indent(lines[i - 1]) > ind) {
                i -= 1;
            }
            i = i.saturating_sub(1);
            while kind == Kind::Go && lines[i].trim_start().starts_with('}') {
                let Some(j) = (0..i)
                    .rev()
                    .find(|&j| !lines[j].trim().is_empty() && indent(lines[j]) <= ind)
                else {
                    break;
                };
                if !uncommented(kind, lines[j]).trim_end().ends_with("struct {") {
                    break;
                }
                i = j;
            }
        }
        let sibling = ["else", "catch", "finally"]
            .iter()
            .any(|k| t.trim_start_matches('}').trim_start().starts_with(k));
        let header: Vec<&str> = match (sibling || closes_type_params) && end > i {
            true => vec![lines[i].trim(), t],
            false => lines[i..=end].iter().map(|l| l.trim()).collect(),
        };
        let header = uncommented(kind, &header.join("\n"));
        depth = ind.min(indent(lines[i]));
        go_block_statement_at = None;
        if !this {
            if kind == Kind::Go {
                let types = header
                    .strip_prefix("case ")
                    .and_then(|h| h.strip_suffix(':'));
                arm = match (types, header.starts_with("default")) {
                    (Some(types), _) => Some(type_list(kind, types)),
                    (None, true) => Some(Vec::new()),
                    (None, false) => arm,
                };
            }
            if opener_bindings(kind, &header, i + 1, name, &mut out) || declared_in_block {
                break;
            }
            if kind == Kind::CSharp {
                if cs_type_decl(lines[i]).is_some() || cs_namespace_line(lines[i]) {
                    break;
                }
                among_cs_members |= in_cs_type_body(i);
            }
        } else if let Some(value) = this_opener(&header, i + 1) {
            out.push(Binding {
                line1: i + 1,
                value,
            });
            break;
        }
    }
    out
}
fn this_opener(header: &str, line: usize) -> Option<Value> {
    static CLASS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:(?:export|default|declare|abstract)\s+)*class\b").unwrap()
    });
    if CLASS.is_match(header) {
        return Some(Value::Class { decl_line1: line });
    }
    let before = header.strip_suffix('{').map(str::trim_end);
    let case = (header.starts_with("case ") || header.starts_with("default"))
        && before.is_some_and(|b| b.ends_with(':'));
    let object = !case
        && before.is_some_and(|b| {
            b.ends_with(['=', '(', '[', ',', '?', ':', '|', '&'])
                || b.ends_with("return")
                || b.ends_with("yield")
                || b.ends_with("default")
        });
    (object || names(header, "function")).then_some(Value::Unknown)
}
pub fn ts_arrow_binds(line: &str, name: &str, at: usize) -> bool {
    ts_arrows(line, name)
        .iter()
        .any(|a| a.param_at < at && at > a.body_start && at < a.body_end)
}
pub fn ts_arrow_param(line: &str, name: &str, at: usize) -> bool {
    let before = line[..at].trim_end();
    let after = line[at + name.len()..].trim_start();
    let returned = before
        .strip_suffix(':')
        .is_some_and(|b| b.trim_end().ends_with(')'));
    let slot = (after.starts_with("=>") && !returned)
        || before.ends_with(['(', ','])
        || before.ends_with("...");
    slot && ts_arrows(line, name).iter().any(|a| a.param_at == at)
}
struct TsArrow {
    param_at: usize,
    body_start: usize,
    body_end: usize,
}
fn ts_arrows(line: &str, name: &str) -> Vec<TsArrow> {
    let Ok(word) = Regex::new(&format!(r"(?:^|[^\w$.]){}\b", regex::escape(name))) else {
        return Vec::new();
    };
    // ponytail: a pair of apostrophes in JSX text still reads as a string; reading JSX text is
    // the way out.
    if code(Kind::TsJs, &format!("{line}\n)")).last() != Some((line.len() + 1, b')')) {
        return Vec::new();
    }
    let b = line.as_bytes();
    let bytes: Vec<(usize, u8)> = code(Kind::TsJs, line)
        .take_while(|&(_, c)| c != 0)
        .collect();
    let last_open_bracket_before = |upto: usize| {
        let mut stack = Vec::new();
        for &(i, c) in bytes.iter().take_while(|&&(i, _)| i < upto) {
            match c {
                b'(' | b'[' | b'{' => stack.push(i),
                b')' | b']' | b'}' => {
                    stack.pop();
                }
                _ => {}
            }
        }
        stack.last().copied()
    };
    let arrow_after_params = |params_end: usize, behind_return_type: bool| {
        let t = line[params_end..].trim_start();
        let t = match t.strip_prefix(':').filter(|_| behind_return_type) {
            Some(ty) => ty
                .find("=>")
                .filter(|&e| !ty[..e].contains([';', '{', '}', '(', ')']))
                .map(|e| &ty[e..])?,
            None => t,
        };
        t.starts_with("=>").then(|| line.len() - t.len())
    };
    word.find_iter(line)
        .map(|m| m.end() - name.len())
        .filter_map(|decl| {
            let start = arrow_after_params(decl + name.len(), false).or_else(|| {
                let open = last_open_bracket_before(decl).filter(|&p| b[p] == b'(')?;
                let before = line[..open].trim_end();
                if return_type(before) || before.ends_with('<') {
                    return None;
                }
                arrow_after_params(close_of(Kind::TsJs, line, open)?, true)
            });
            let start = start?;
            let (mut depth, mut ternaries_awaiting_colon) = (0i32, 0i32);
            let end = bytes
                .iter()
                .skip_while(|&&(i, _)| i < start + 2)
                .find(|&&(i, c)| match c {
                    b'(' | b'[' | b'{' => {
                        depth += 1;
                        false
                    }
                    b')' | b']' | b'}' => {
                        depth -= 1;
                        depth < 0
                    }
                    b',' | b';' => depth == 0,
                    b'?' if depth == 0 => {
                        let chain = b[i - 1] == b'?' || matches!(b.get(i + 1), Some(b'.' | b'?'));
                        ternaries_awaiting_colon += i32::from(!chain);
                        false
                    }
                    b':' if depth == 0 => {
                        ternaries_awaiting_colon -= 1;
                        ternaries_awaiting_colon < 0
                    }
                    b'=' if depth == 0 => {
                        !b"=<>!".contains(&b[i - 1])
                            && !b.get(i + 1).is_some_and(|c| b"=>{\"'".contains(c))
                    }
                    _ => false,
                })
                .map_or(line.len(), |&(i, _)| i);
            Some(TsArrow {
                param_at: decl,
                body_start: start,
                body_end: end,
            })
        })
        .collect()
}
fn opener_bindings(
    kind: Kind,
    header: &str,
    line: usize,
    name: &str,
    out: &mut Vec<Binding>,
) -> bool {
    static TS_LOOP: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bfor\s*\(\s*(?:const|let|var)\s+([^;]+?)\s+(?:of|in)\s|\bfor\s*\(\s*(?:const|let|var)\s+([^;]*);|\bcatch\s*\(([^)]*)\)").unwrap()
    });
    static TS_ARROW: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(?::\s*[^=]+?)?\s*=>").unwrap());
    static TS_FUNCTION: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\bfunction\*?\s*[\w$]*\s*(?:<[^>]*>)?$").unwrap());
    static TS_METHOD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"^(?:(?:public|private|protected|static|readonly|abstract|override|async|get|set)\s+)*\*?(?:#?([\w$]+)|"[^"\n]*"|'[^'\n]*'|\[[^\]\n]*\])\s*(?:<[^>]*>)?$"#).unwrap()
    });
    static TS_BODY: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(?::.*)?\{").unwrap());
    static TS_RETURN_TYPE_ARROW_BRACE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(?::[^(){}]+?)?\s*(?:=>)?\s*\{?\s*$").unwrap());
    static GO_ASSIGN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|\belse\s+)(?:if|for|switch|case)\s+([^;{]*?)\s*:=").unwrap()
    });
    static GO_FUNC: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bfunc\b\s*(?:\(([^)]*)\)\s*)?(?:[A-Za-z_]\w*\s*)?(?:\[[^\]]*\]\s*)?\(")
            .unwrap()
    });
    let unknown = |out: &mut Vec<Binding>| {
        out.push(Binding {
            line1: line,
            value: Value::Unknown,
        })
    };
    let n = regex::escape(name);
    let element = |p: String| {
        let rule = Regex::new(&p).expect("an escaped name keeps the pattern valid");
        let value = Value::Element(rule.captures(header)?[1].to_owned());
        Some(Binding { line1: line, value })
    };
    let mut binds_for_body = false;
    match kind {
        Kind::TsJs => {
            let starts = |l: &str| {
                let l = l.trim_start_matches('}').trim_start();
                l.starts_with("for") || l.starts_with("catch")
            };
            let is_loop = starts(header) || header.rsplit('\n').next().is_some_and(starts);
            let count = out.len();
            if let Some(b) = element(format!(
                r"\bfor\s*\(\s*(?:const|let|var)\s+{n}\s+of\s+((?:this|[A-Za-z_$][\w$]*)(?:\.[A-Za-z_$][\w$]*)*)\s*\)"
            )) {
                out.push(b);
            } else if TS_LOOP
                .captures_iter(header)
                .any(|c| c.iter().skip(1).flatten().any(|m| names(m.as_str(), name)))
            {
                unknown(out);
            }
            binds_for_body |= is_loop && out.len() > count;
            for (open, _) in header.match_indices('(') {
                let Some(close) = close_of(kind, header, open) else {
                    continue;
                };
                let (before, after) = (header[..open].trim_end(), header[close..].trim_start());
                let method = TS_METHOD.captures(before).is_some_and(|c| {
                    !c.get(1).is_some_and(|m| {
                        matches!(
                            m.as_str(),
                            "if" | "for"
                                | "while"
                                | "switch"
                                | "catch"
                                | "with"
                                | "return"
                                | "super"
                        )
                    }) && TS_BODY.is_match(after)
                });
                if TS_ARROW.is_match(after) || TS_FUNCTION.is_match(before) || method {
                    let count = out.len();
                    ts_params(&header[open + 1..close - 1], line, name, out);
                    binds_for_body |=
                        out.len() > count && TS_RETURN_TYPE_ARROW_BRACE.is_match(after);
                }
            }
            let arrow = Regex::new(&format!(
                r"(?:^|[^\w$.]){}\s*=>(\s*\{{?\s*$)?",
                regex::escape(name)
            ))
            .expect("an escaped name keeps the pattern valid");
            if let Some(c) = arrow
                .captures_iter(header)
                .last()
                .filter(|c| !return_type(&header[..c.get(0).map_or(0, |m| m.start())]))
            {
                unknown(out);
                binds_for_body |= c.get(1).is_some();
            }
        }
        Kind::CSharp => {
            let opener = cs_opener(header, name);
            if opener.binds {
                unknown(out);
            }
            binds_for_body = opener.hides_outer_scopes;
        }
        Kind::Go => {
            let count = out.len();
            let structs = Regex::new(&format!(
                r"^for\s+[A-Za-z_]\w*\s*,\s*{n}\s*:=\s*range\s+(?:\[[^\]]*\]|map\[[^\]]*\])+\s*struct\s*\{{"
            ))
            .expect("an escaped name keeps the pattern valid");
            if structs.is_match(header) {
                out.push(Binding {
                    line1: line,
                    value: Value::Struct(line),
                });
            } else if let Some(b) = element(format!(
                r"^for\s+[A-Za-z_]\w*\s*,\s*{n}\s*:=\s*range\s+([A-Za-z_]\w*)\s*\{{"
            )) {
                out.push(b);
            } else if GO_ASSIGN.captures_iter(header).any(|c| names(&c[1], name)) {
                unknown(out);
            }
            binds_for_body |= out.len() > count;
            if let Some(c) = GO_FUNC.captures_iter(header).last() {
                let count = out.len();
                if let Some(receiver) = c.get(1) {
                    go_params(receiver.as_str(), line, name, out);
                }
                let open = c.get(0).unwrap().end() - 1;
                if let Some(close) = close_of(kind, header, open) {
                    go_params(&header[open + 1..close - 1], line, name, out);
                    let results = header[close..].trim_start();
                    if results.starts_with('(')
                        && let Some(end) = close_of(kind, results, 0)
                    {
                        go_params(&results[1..end - 1], line, name, out);
                    }
                    let body = results.trim_end().strip_suffix('{');
                    binds_for_body |=
                        out.len() > count && body.is_some_and(|r| !r.contains(['{', '}']));
                }
            }
        }
        _ => {}
    }
    binds_for_body
}
pub fn go_binds_here(line: &str, name: &str, at: usize) -> bool {
    let (mut open, mut types) = (Vec::new(), Vec::new());
    let ident = |c: &u8| c.is_ascii_alphanumeric() || *c == b'_';
    let mut last = b' ';
    for (i, c) in code(Kind::Go, &line[..at]) {
        match c {
            b'(' | b'[' | b'{' => open.push(i),
            b')' | b']' | b'}' => {
                open.pop();
            }
            b'f' if line[i..].starts_with("func")
                && !(i > 0 && ident(&line.as_bytes()[i - 1]))
                && !line.as_bytes().get(i + 4).is_some_and(ident) =>
            {
                types.push((i, open.last().copied(), matches!(last, b')' | b']')));
            }
            _ => {}
        }
        if !c.is_ascii_whitespace() {
            last = c;
        }
    }
    open.iter()
        .filter(|&&b| line.as_bytes()[b] == b'{')
        .any(|&b| {
            let mut header = line[..=b].to_owned();
            for &(f, inside, result) in types.iter().filter(|t| t.0 < b) {
                if result || inside.is_some_and(|p| !open.contains(&p)) {
                    header.replace_range(f..f + 4, "type");
                }
            }
            opener_bindings(Kind::Go, header.trim(), 0, name, &mut Vec::new())
        })
}
pub fn go_param_type(line: &str, name: &str, at: usize) -> bool {
    let head = format!(
        r"^\s*(?:func\s*(?:\([^()]*\)\s*)?)?{}\s*\(",
        regex::escape(name)
    );
    let Some(open) = Regex::new(&head)
        .ok()
        .and_then(|re| re.find(line))
        .map(|m| m.end())
    else {
        return false;
    };
    if at < open {
        return false;
    }
    let (mut depth, mut item_start, mut list_close) = (0usize, open, None);
    for (i, c) in code(Kind::Go, &line[open..]) {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' if depth == 0 => {
                list_close = Some(open + i);
                break;
            }
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b',' if depth == 0 && open + i < at => item_start = open + i + 1,
            _ => {}
        }
    }
    let Some(close) = list_close.filter(|&c| at < c) else {
        return false;
    };
    let named = split_top(Kind::Go, &line[open..close], b',')
        .iter()
        .any(|i| i.trim().contains(char::is_whitespace));
    !named || !line[item_start..at].trim().is_empty()
}
fn return_type(before: &str) -> bool {
    let b = before.as_bytes();
    let mut depth = 0i32;
    for i in (0..b.len()).rev() {
        match b[i] {
            b')' | b']' | b'}' => depth += 1,
            b'>' if i == 0 || b[i - 1] != b'=' => depth += 1,
            b'(' | b'[' | b'{' | b'<' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            b':' if depth == 0 => return before[..i].trim_end().ends_with(')'),
            b',' | b';' | b'=' if depth == 0 => return false,
            _ => {}
        }
    }
    false
}
fn go_params(params: &str, line: usize, name: &str, out: &mut Vec<Binding>) {
    let items: Vec<&str> = split_top(Kind::Go, params, b',')
        .into_iter()
        .map(str::trim)
        .filter(|i| !i.is_empty())
        .collect();
    if !items.iter().any(|i| i.contains(char::is_whitespace)) {
        return;
    }
    let mut ty: Option<&str> = None;
    for item in items.iter().rev() {
        let pname = match item.split_once(char::is_whitespace) {
            Some((pname, t)) => {
                ty = Some(t.trim());
                pname
            }
            None => item,
        };
        if pname == name {
            let value = match ty {
                Some(t) if !t.starts_with("...") => Value::Type(t.to_owned()),
                _ => Value::Unknown,
            };
            out.push(Binding { line1: line, value });
        }
    }
}
fn statement_bindings(kind: Kind, t: &str, line: usize, name: &str, out: &mut Vec<Binding>) {
    static GO_SHORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)*)\s*:=\s*(.*)$").unwrap()
    });
    static GO_VAR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"^var\s+([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)*)(?:\s+([^=]+?))?\s*(?:=\s*(.*))?$",
        )
        .unwrap()
    });
    static TS_DESTRUCTURE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:export\s+)?(?:const|let|var)\s*[\[{](.*)[\]}]\s*(?::.*)?=").unwrap()
    });
    static KEY: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"[\w$]+\s*:").unwrap());
    let n = regex::escape(name);
    let value = match kind {
        Kind::TsJs => {
            let declared = Regex::new(&format!(
                r"^(?:export\s+)?(?:declare\s+)?(?:const|let|var)\s+{n}\s*!?\s*(?::\s*([^=]+?))?\s*(?:=\s*(.*?))?\s*;?$"
            ))
            .expect("an escaped name keeps the pattern valid");
            let named = Regex::new(&format!(
                r"^(?:export\s+)?(?:default\s+)?(?:declare\s+)?(?:abstract\s+)?(?:async\s+)?(?:function\*?|class|enum|namespace|interface|type)\s+{n}(?:[^\w$]|$)"
            ))
            .expect("an escaped name keeps the pattern valid");
            if let Some(c) = declared.captures(t) {
                match (c.get(1), c.get(2).filter(|v| !v.as_str().is_empty())) {
                    (Some(ty), _) => Value::Type(ty.as_str().to_owned()),
                    (None, Some(v)) => value_of(kind, v.as_str()),
                    (None, None) => Value::Unknown,
                }
            } else if let Some(field) = ts_destructured(t, name) {
                field
            } else if named.is_match(t)
                || TS_DESTRUCTURE
                    .captures(t)
                    .is_some_and(|c| names(&KEY.replace_all(&c[1], "").replace("...", " "), name))
            {
                Value::Unknown
            } else {
                return;
            }
        }
        Kind::Go => {
            let list =
                |s: &str| -> Vec<String> { s.split(',').map(|p| p.trim().to_owned()).collect() };
            if let Some(c) = GO_SHORT.captures(t) {
                match list(&c[1]).iter().position(|p| p == name) {
                    Some(0) => value_of(kind, &c[2]),
                    Some(_) => Value::Unknown,
                    None => return,
                }
            } else if let Some(c) = GO_VAR.captures(t) {
                match (
                    list(&c[1]).iter().position(|p| p == name),
                    c.get(2),
                    c.get(3),
                ) {
                    (None, ..) => return,
                    (Some(_), Some(ty), _) => Value::Type(ty.as_str().to_owned()),
                    (Some(0), None, Some(v)) => value_of(kind, v.as_str()),
                    _ => Value::Unknown,
                }
            } else if t
                .strip_prefix("const ")
                .is_some_and(|rest| names(rest.split('=').next().unwrap_or(rest), name))
            {
                Value::Unknown
            } else {
                return;
            }
        }
        Kind::CSharp if cs_statement(t, name) => Value::Unknown,
        _ => return,
    };
    out.push(Binding { line1: line, value });
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct TsHeader {
    pub one_line_without_type_params: String,
    pub last_line0: usize,
}
pub(super) fn ts_header(lines: &[&str], k: usize) -> TsHeader {
    // ponytail: forty lines of header; hono's widest list of type parameters runs to six.
    let text = uncommented(Kind::TsJs, &lines[k..lines.len().min(k + 40)].join("\n"));
    let b = text.as_bytes();
    let (mut depth, mut angle) = (0i32, 0i32);
    let (mut type_params_from, mut type_params_to) = (None, None);
    let mut open = None;
    for (i, c) in code(Kind::TsJs, &text) {
        match c {
            b'{' if depth == 0 && angle == 0 => {
                open = Some(i);
                break;
            }
            b'\n' if depth == 0 && angle == 0 => {
                let next = text[i + 1..].lines().next().unwrap_or("");
                if indent(next) <= indent(lines[k]) && !next.trim_start().starts_with('{') {
                    break;
                }
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'<' if depth == 0 => {
                if angle == 0 && type_params_from.is_none() && !names(&text[..i], "extends") {
                    type_params_from = Some(i);
                }
                angle += 1;
            }
            b'>' if depth == 0 && angle > 0 && b[i - 1] != b'=' => {
                angle -= 1;
                if angle == 0 && type_params_from.is_some() && type_params_to.is_none() {
                    type_params_to = Some(i + 1);
                }
            }
            _ => {}
        }
    }
    let Some(open) = open else {
        return TsHeader {
            one_line_without_type_params: lines[k].to_owned(),
            last_line0: k,
        };
    };
    let last = k + text[..open].matches('\n').count();
    let mut header = text[..=open].to_owned();
    if let (Some(from), Some(to)) = (type_params_from, type_params_to) {
        header.replace_range(from..to, "");
    }
    TsHeader {
        one_line_without_type_params: header.split_whitespace().collect::<Vec<_>>().join(" "),
        last_line0: last,
    }
}
pub fn go_may_declare(text: &str, line: usize, name: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(Kind::Go, text);
    let at = line.saturating_sub(1).min(lines.len());
    let mentions = |l: &str| {
        let code: String = {
            let mut bytes = vec![b' '; l.len()];
            for (i, c) in code(Kind::Go, l) {
                bytes[i] = if c == 0 { b' ' } else { c };
            }
            String::from_utf8_lossy(&bytes).into_owned()
        };
        code.match_indices(name).any(|(i, m)| {
            let ident = |c: char| c.is_alphanumeric() || c == '_';
            !code[..i].ends_with(ident)
                && !code[i + m.len()..].starts_with(ident)
                && !code[i + m.len()..].trim_start().starts_with(['.', ')'])
        })
    };
    for i in (0..=at.min(lines.len().saturating_sub(1))).rev() {
        let l = lines[i];
        if literal[i] || l.trim().is_empty() {
            continue;
        }
        if mentions(l) {
            return true;
        }
        let ends = l.starts_with("func")
            || ["}", ")"].contains(&l.trim_end())
            || ["type ", "var ", "const ", "import ", "package "]
                .iter()
                .any(|k| l.starts_with(k));
        if i < at && ends {
            return false;
        }
    }
    false
}
pub fn package_bindings(text: &str, name: &str) -> Vec<Binding> {
    let literal = literal_lines(Kind::Go, text);
    let mut out = Vec::new();
    let mut in_var_block = false;
    let mut entry_indent = None;
    for (i, l) in text.lines().enumerate() {
        let code = uncommented(Kind::Go, l);
        let t = code.trim();
        if literal[i] || t.is_empty() {
            continue;
        }
        match (indent(l), in_var_block) {
            (0, _) if t == "var (" => (in_var_block, entry_indent) = (true, None),
            (0, _) => {
                in_var_block = false;
                statement_bindings(Kind::Go, t, i + 1, name, &mut out);
            }
            (n, true) if Some(n) == entry_indent.or(Some(n)) => {
                entry_indent = Some(n);
                let words: Vec<&str> = t.split_whitespace().collect();
                let t = format!("var {}", words.join(" "));
                statement_bindings(Kind::Go, &t, i + 1, name, &mut out);
            }
            _ => {}
        }
    }
    out
}
