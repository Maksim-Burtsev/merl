use std::ops::Range;

use regex::Regex;

use super::*;

pub fn qualified(kind: Kind, text: &str, line1: usize, name: &str) -> Option<String> {
    static RECEIVER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^func\s+\(\s*(?:\w+\s+)?\*?\s*([A-Za-z_]\w*)").unwrap()
    });
    static FUNC: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^func\s+(?:\([^)]*\)\s*)?([A-Za-z_]\w*)").unwrap()
    });
    static GO_TYPE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^type\s+([A-Za-z_]\w*)").unwrap());
    static GO_STRUCT_WRITTEN_IN_PLACE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| {
            Regex::new(r"(?:\]|=|\(|,)\s*struct\s*\{\s*(?://.*)?$").unwrap()
        });
    static RUST_FN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"^\s*(?:(?:pub(?:\([^)]*\))?|async|unsafe|const|extern(?:\s+"[^"]*")?|default)\s+)*fn\s+([A-Za-z_]\w*)"#).unwrap()
    });
    static PY_DEF: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*(?:async\s+)?def\s+([A-Za-z_]\w*)").unwrap());
    static IMPL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:unsafe\s+)?impl\b(?:\s*<[^{]*?>)?\s+(?:[\w:]+(?:<[^{]*?>)?\s+for\s+)?&?(?:\w+::)*([A-Za-z_]\w*)").unwrap()
    });
    static EX_MODULE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*def(?:module|protocol)\s+([A-Z](?:[\w.]*\w)?)").unwrap()
    });
    static EX_DEF: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*def(?:p|macrop?|guardp?|delegate)?\s+([\w?!]+)").unwrap()
    });
    static RB_DEF: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*def\s+(?:(?:self\??|[A-Z]\w*)\.)?([A-Za-z_]\w*[?!=]?)").unwrap()
    });
    static COMPANION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(jvm_mods!(), r"companion\s+object\s*(?:[:{]|$)")).unwrap()
    });
    static COLUMN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"^\s*t\.\w+\s*\(?\s*(?:"(\w+)"|:(\w+))"#).unwrap()
    });
    static TABLE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"^\s*create_table\s*\(?\s*(?:"(\w+)"|:(\w+))"#).unwrap()
    });
    if !nests(Some(kind)) {
        return None;
    }
    let sep = separator(kind);
    let lines: Vec<&str> = text.lines().collect();
    let target = *lines.get(line1.checked_sub(1)?)?;
    if kind == Kind::TsJs
        && let Some(owner) = jsdoc_owner_name(&lines, line1 - 1)
    {
        return Some(format!("{owner}{sep}{name}"));
    }
    if kind == Kind::Go
        && let Some(c) = FUNC.captures(target).filter(|c| &c[1] != name)
    {
        return Some(format!("{}{sep}{name}", &c[1]));
    }
    if let Some(c) = RECEIVER.captures(target).filter(|_| kind == Kind::Go) {
        return Some(format!("{}{sep}{name}", &c[1]));
    }
    if kind == Kind::Jvm
        && let Some(receiver) = jvm_receiver_at(text, line1, name)
    {
        return Some(format!("{receiver}{sep}{name}"));
    }
    if kind == Kind::Jvm
        && let Some(f) = jvm_function(target).filter(|f| f != name && target.contains(name))
    {
        let owner = qualified(kind, text, line1, &f).unwrap_or(f);
        return Some(format!("{owner}{sep}{name}"));
    }
    if kind == Kind::Go && go_one_line_field(target, name) {
        let owner = GO_TYPE
            .captures(target)
            .map_or_else(|| "struct{\u{2026}}".to_owned(), |c| c[1].to_owned());
        return (owner != name).then(|| format!("{owner}{sep}{name}"));
    }
    if field_like(kind, target.trim_start(), name)
        && let Some(decl) = enclosing_type(kind, &lines, line1 - 1)
        && field_bindings(kind, text, decl, name)
            .iter()
            .any(|b| b.line1 == line1)
        && let Some(ty) = type_name(kind, lines[decl - 1])
    {
        let owner = qualified(kind, text, decl, &ty).unwrap_or(ty);
        return Some(format!("{owner}{sep}{name}"));
    }
    if kind == Kind::Php
        && let Some(class) = php_tag_class(&lines, line1 - 1)
        && let Some(owner) = declared_name(Some(kind), lines[class])
    {
        let owner = qualified(kind, text, class + 1, &owner).unwrap_or(owner);
        return Some(format!("{owner}{sep}{name}"));
    }
    if kind == Kind::Rust
        && let Some(c) = RUST_FN.captures(target).filter(|c| &c[1] != name)
    {
        let owner = qualified(kind, text, line1, &c[1]).unwrap_or_else(|| c[1].to_owned());
        return Some(format!("{owner}{sep}{name}"));
    }
    if kind == Kind::CSharp
        && let Some(m) = cs_parameter_of(target, name)
    {
        let owner = qualified(kind, text, line1, &m).unwrap_or(m);
        return Some(format!("{owner}{sep}{name}"));
    }
    if kind == Kind::Ruby
        && COLUMN
            .captures(target)
            .is_some_and(|c| c.get(1).or(c.get(2)).is_some_and(|m| m.as_str() == name))
        && let Some(table) = owner_line(&lines, line1 - 1).and_then(|o| TABLE.captures(lines[o]))
    {
        let table = table.get(1).or(table.get(2)).map_or("", |m| m.as_str());
        return Some(format!("{table}{sep}{name}"));
    }
    let def = match kind {
        Kind::Python => PY_DEF.captures(target),
        Kind::Elixir => EX_DEF.captures(target),
        Kind::Ruby => RB_DEF.captures(target),
        _ => None,
    };
    if let Some(c) = def.filter(|c| &c[1] != name) {
        let owner = qualified(kind, text, line1, &c[1]).unwrap_or_else(|| c[1].to_owned());
        return Some(format!("{owner}{sep}{name}"));
    }
    let indent = |s: &str| s.len() - s.trim_start().len();
    let mut depth = indent(target);
    let mut names = vec![name.to_owned()];
    let ruby = kind == Kind::Ruby;
    let mut def_owner = None;
    if ruby {
        let spelled = ruby_namespace(target);
        if target.trim_start().starts_with("def") {
            def_owner = spelled.first().cloned();
        }
        names.extend(spelled);
    }
    let cpp = kind == Kind::C;
    if cpp {
        names.extend(cpp_namespace(target, name));
    }
    for (j, l) in lines[..line1 - 1].iter().enumerate().rev() {
        if depth == 0 {
            break;
        }
        let tail = kind == Kind::Jvm && l.trim_start().starts_with(')');
        if steps_over(Some(kind), l.trim_start()) || tail || indent(l) >= depth {
            continue;
        }
        depth = indent(l);
        if kind == Kind::Jvm && COMPANION.is_match(l) {
            continue;
        }
        if kind == Kind::TsJs
            && ts_declarators(&lines, j).is_some_and(|d| d.iter().any(|d| d.line0 == line1 - 1))
        {
            return qualified(kind, text, j + 1, name);
        }
        if ruby && l.trim_start().starts_with("class << self") {
            continue;
        }
        let named = IMPL
            .captures(l)
            .filter(|_| kind == Kind::Rust)
            .map(|c| c[1].to_owned())
            .or_else(|| {
                EX_MODULE
                    .captures(l)
                    .filter(|_| kind == Kind::Elixir)
                    .map(|c| c[1].to_owned())
            })
            .or_else(|| declared_name(Some(kind), l));
        if named.is_none() && kind == Kind::Go && GO_STRUCT_WRITTEN_IN_PLACE.is_match(l) {
            names.push("struct{\u{2026}}".to_owned());
            break;
        }
        match named {
            Some(n) if def_owner.take().is_some_and(|o| o == n) => {}
            Some(n) => {
                if cpp {
                    names.push(n.clone());
                    names.extend(cpp_namespace(l, &n));
                } else {
                    names.push(n);
                }
            }
            None => break,
        }
        if ruby {
            names.extend(ruby_namespace(l));
        }
    }
    names.reverse();
    (names.len() > 1).then(|| names.join(sep))
}
/// ponytail: a `{…}` group wrapped over several lines is not read.
pub fn elixir_unalias(text: &str, line: usize, mut chain: Vec<String>) -> Vec<String> {
    static ALIAS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*alias\s+([A-Z](?:[\w.]*\w)?)(?:\.\{([^}]*)\}|\s*,\s*as:\s*([A-Z]\w*))?")
            .unwrap()
    });
    let Some(first) = chain.first().cloned() else {
        return chain;
    };
    let last = |s: &str| s.rsplit('.').next() == Some(first.as_str());
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(Kind::Elixir, text);
    let mut least_indent_since_cursor = lines.get(line).map_or(usize::MAX, |l| indent(l));
    let full = (0..line.min(lines.len()))
        .rev()
        .filter(|&i| !literal[i] && !lines[i].trim().is_empty())
        .find_map(|i| {
            let open = indent(lines[i]) <= least_indent_since_cursor;
            least_indent_since_cursor = least_indent_since_cursor.min(indent(lines[i]));
            let c = ALIAS.captures(lines[i]).filter(|_| open)?;
            let module = &c[1];
            match (c.get(2), c.get(3)) {
                (Some(group), _) => group
                    .as_str()
                    .split(',')
                    .map(str::trim)
                    .find(|n| last(n))
                    .map(|n| format!("{module}.{n}")),
                (None, Some(named)) => (named.as_str() == first).then(|| module.to_owned()),
                (None, None) => last(module).then(|| module.to_owned()),
            }
        });
    if let Some(full) = full {
        chain.splice(..1, full.split('.').map(str::to_owned));
    }
    chain
}
fn ruby_namespace(line: &str) -> Vec<String> {
    static SPELLED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:class|module)\s+(?:::)?((?:\w+::)+)\w|def\s+([A-Z]\w*)\.\w)")
            .unwrap()
    });
    let Some(c) = SPELLED.captures(line) else {
        return Vec::new();
    };
    let spelled = c.get(1).or(c.get(2)).map_or("", |m| m.as_str());
    let mut names: Vec<String> = spelled
        .split("::")
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    names.reverse();
    names
}
fn cpp_namespace(line: &str, name: &str) -> Vec<String> {
    let spelled = Regex::new(&format!(
        r"((?:\w+(?:<[^<>]*>)?::)*)\b{}\b",
        regex::escape(name)
    ))
    .expect("an escaped name keeps the pattern valid");
    let Some(c) = spelled.captures(line) else {
        return Vec::new();
    };
    let rest = &line[c.get(0).map_or(0, |m| m.end())..];
    let rest = rest.split('(').next().unwrap_or(rest);
    let again = Regex::new(&format!(r"\b{}\b", regex::escape(name)))
        .expect("an escaped name keeps the pattern valid");
    let qualified_one_is_a_type = again.is_match(rest);
    if qualified_one_is_a_type {
        return Vec::new();
    }
    let mut names: Vec<String> = c[1]
        .split("::")
        .filter(|s| !s.is_empty())
        .map(|s| s.split('<').next().unwrap_or(s).to_owned())
        .collect();
    names.reverse();
    names
}
fn declared_name(kind: Option<Kind>, line: &str) -> Option<String> {
    static ROWS: std::sync::LazyLock<Vec<(Option<Kind>, Regex)>> = std::sync::LazyLock::new(|| {
        SYMBOLS
            .iter()
            .map(|(k, p)| {
                (
                    *k,
                    Regex::new(p).expect("built-in symbol patterns are valid"),
                )
            })
            .collect()
    });
    ROWS.iter()
        .filter(|(k, _)| *k == kind || (k.is_none() && shared_symbols(kind)))
        .find_map(|(k, re)| {
            symbol_name(re, line)
                .filter(|n| *k != Some(Kind::Haskell) || !HASKELL_RESERVED.contains(&n.as_str()))
        })
}
fn nests(kind: Option<Kind>) -> bool {
    !matches!(kind, Some(Kind::Yaml | Kind::Markdown))
}
fn aside(t: &str) -> bool {
    t.is_empty()
        || ["#", "//", "/*", "*", "--"]
            .iter()
            .any(|c| t.starts_with(c))
}
pub(super) fn steps_over(kind: Option<Kind>, t: &str) -> bool {
    let opens_a_wrapped_body = t == "{";
    let cpp_access_specifier =
        kind == Some(Kind::C) && matches!(t, "public:" | "private:" | "protected:");
    let closes_ts_type_parameters = kind == Some(Kind::TsJs) && t.starts_with('>');
    let closes_a_wrapped_python_header = kind == Some(Kind::Python) && t.starts_with([')', ']']);
    aside(t)
        || opens_a_wrapped_body
        || closes_ts_type_parameters
        || cpp_access_specifier
        || closes_a_wrapped_python_header
}
pub fn enclosing_declarations(kind: Option<Kind>, lines: &[String], line: usize) -> Vec<usize> {
    if !nests(kind) {
        return Vec::new();
    }
    let indent = |s: &str| s.len() - s.trim_start().len();
    let wrapped_header_tail = |t: &str| t.starts_with([')', ']', '>', '{']) || t == "where";
    let code_or_tail = |l: &&String| {
        let t = l.trim_start();
        wrapped_header_tail(t) || !steps_over(kind, t)
    };
    let Some(first) = lines
        .get(line..)
        .and_then(|rest| rest.iter().find(code_or_tail))
    else {
        return Vec::new();
    };
    let mut depth = indent(first) + usize::from(wrapped_header_tail(first.trim_start()));
    let mut out = Vec::new();
    for (i, l) in lines[..line].iter().enumerate().rev() {
        if depth == 0 {
            break;
        }
        let t = l.trim_start();
        if steps_over(kind, t) || wrapped_header_tail(t) || indent(l) >= depth {
            continue;
        }
        depth = indent(l);
        if declared_name(kind, l).is_some() {
            out.push(i);
        }
    }
    out.reverse();
    out
}
fn field_like(kind: Kind, t: &str, name: &str) -> bool {
    static MODIFIED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:@[\w$.]+(?:\([^)]*\))?\s*)*(?:(?:public|private|protected|readonly|override)\s+)+([\w$]+)").unwrap()
    });
    match kind {
        Kind::Python => in_method(t, name),
        Kind::TsJs => in_method(t, name) || MODIFIED.captures(t).is_some_and(|c| &c[1] == name),
        _ => false,
    }
}
pub(super) fn in_method(t: &str, name: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    ["self.", "this."].iter().any(|p| {
        let at = format!("{p}{name}");
        t.match_indices(&at)
            .any(|(i, m)| !t[..i].ends_with(ident) && !t[i + m.len()..].starts_with(ident))
    })
}
fn name_start(s: &str, is_name: impl Fn(char) -> bool) -> usize {
    s.char_indices()
        .rev()
        .find(|&(_, c)| !is_name(c))
        .map_or(0, |(i, c)| i + c.len_utf8())
}
pub fn qualifier(line: &str, word_start: usize) -> Vec<String> {
    let mut before = &line[..word_start];
    let mut chain = Vec::new();
    while let Some(rest) = before
        .strip_suffix("::")
        .or_else(|| before.strip_suffix('.'))
    {
        if let Some(head) = rest.strip_suffix("super()")
            && !head.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        {
            chain.insert(0, "super".to_owned());
            before = head;
            break;
        }
        let mut start = name_start(rest, |c| c.is_ascii_alphanumeric() || c == '_');
        if start == rest.len() {
            break;
        }
        if rest[..start].ends_with(".#") {
            start -= 1;
        }
        chain.insert(0, rest[start..].to_owned());
        before = &rest[..start];
    }
    let hangs_off_an_expression = before.ends_with('.') && !before.ends_with("..");
    if hangs_off_an_expression {
        return Vec::new();
    }
    chain
}
pub fn unbroken(kind: Kind, lines: &[String], at: usize, word_start: usize) -> Option<LineAsRead> {
    let led = |l: &str| {
        let t = l.trim_start();
        match kind {
            Kind::TsJs => t.starts_with('.') && !t.starts_with(".."),
            Kind::Php => t.starts_with("->") || t.starts_with("?->"),
            _ => false,
        }
    };
    if !led(&lines[at]) {
        return None;
    }
    let mut joined = lines[at].trim_start().to_owned();
    let mut start = word_start - indent(&lines[at]);
    // ponytail: forty lines of one expression.
    for above in lines[at.saturating_sub(40)..at].iter().rev() {
        let code = uncommented(kind, above);
        let code = match led(&code) {
            true => code.trim(),
            false => code.trim_end(),
        };
        if code.is_empty() {
            continue;
        }
        start += code.len();
        joined.insert_str(0, code);
        if !led(code) {
            return Some(LineAsRead {
                line: joined,
                word_start: start,
            });
        }
    }
    None
}
#[derive(Debug, PartialEq)]
pub struct LineAsRead {
    pub line: String,
    pub word_start: usize,
}
pub fn plain_access(kind: Kind, line: &str, start: usize) -> LineAsRead {
    let before = match kind {
        Kind::TsJs | Kind::Swift => line[..start].replace("?.", ".").replace("!.", "."),
        Kind::Php => line[..start]
            .replace('.', " ")
            .replace("?->", ".")
            .replace("->", "."),
        _ => {
            return LineAsRead {
                line: line.to_owned(),
                word_start: start,
            };
        }
    };
    LineAsRead {
        line: format!("{before}{}", &line[start..]),
        word_start: before.len(),
    }
}
pub fn call_head(kind: Kind, line: &str, word_start: usize) -> Option<CallHead> {
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    let mut before = line[..word_start].strip_suffix('.')?;
    let mut fields = Vec::new();
    while !before.ends_with(')') {
        let start = name_start(before, is_name);
        if start == before.len() {
            return None;
        }
        fields.insert(0, before[start..].to_owned());
        before = before[..start].strip_suffix('.')?;
    }
    let paren_this_closes = code(kind, before)
        .filter(|(_, c)| *c == b'(')
        .map(|(i, _)| i)
        .find(|&i| close_of(kind, before, i) == Some(before.len()))?;
    let mut start = name_start(&before[..paren_this_closes], |c| is_name(c) || c == '.');
    let callee = &before[start..paren_this_closes];
    if kind == Kind::TsJs
        && let Some(rest) = before[..start].trim_end().strip_suffix("new")
        && !rest.ends_with(is_name)
    {
        start = rest.len();
    }
    let written = &before[start..];
    let without_cast_brackets = match callee.is_empty() {
        true => &written[1..written.len() - 1],
        false => written,
    };
    let (call_without_arguments, value) = match value_of(kind, without_cast_brackets) {
        Value::Call(name) => (format!("{name}()"), Value::Call(name)),
        Value::New(name) => (format!("new {name}()"), Value::New(name)),
        value @ (Value::Type(_) | Value::Cast { .. }) => {
            (without_cast_brackets.trim().to_owned(), value)
        }
        _ => return None,
    };
    Some(CallHead {
        call_without_arguments,
        value,
        fields_to_word: fields,
    })
}
#[derive(Debug, PartialEq)]
pub struct CallHead {
    pub call_without_arguments: String,
    pub value: Value,
    pub fields_to_word: Vec<String>,
}
pub fn definition_word(kind: Option<Kind>, line: &str, col: usize) -> Option<(Range<usize>, &str)> {
    let extra = word_chars(kind, true);
    if kind == Some(Kind::R)
        && let Some(r) = r_quoted_name(line, col)
    {
        return Some((r.clone(), &line[r]));
    }
    let sigil_of_the_word_behind = |i: usize| match kind {
        Some(Kind::TsJs) => line.as_bytes().get(i) == Some(&b'#'),
        Some(Kind::Dart) => line.as_bytes().get(i) == Some(&b'$'),
        _ => false,
    };
    let past_sigil = col + usize::from(sigil_of_the_word_behind(col));
    let (range, _) = word_at(line, past_sigil, extra)?;
    let stands_where_a_private_name_can = |i: usize| {
        let before = line[..i].trim_end();
        before.ends_with('.')
            || before.split_whitespace().all(|w| {
                matches!(
                    w,
                    "static" | "readonly" | "async" | "get" | "set" | "accessor" | "declare"
                )
            })
    };
    let start = match range
        .start
        .checked_sub(1)
        .filter(|&i| sigil_of_the_word_behind(i) && stands_where_a_private_name_can(i))
    {
        Some(i) => i,
        None => range.start,
    };
    if kind == Some(Kind::Dart) {
        let r = dart_name(line, range);
        return Some((r.clone(), &line[r]));
    }
    if kind == Some(Kind::Cmake) {
        let r = cmake_name(line, range);
        return Some((r.clone(), &line[r]));
    }
    if kind == Some(Kind::Nix) {
        let r = nix_name(line, range);
        return Some((r.clone(), &line[r]));
    }
    if kind == Some(Kind::Haskell) {
        let r = haskell_name(line, range);
        return Some((r.clone(), &line[r]));
    }
    if matches!(kind, Some(Kind::Ocaml | Kind::Fsharp)) {
        let r = ml_name(line, range, col);
        return Some((r.clone(), &line[r]));
    }
    if kind == Some(Kind::Julia) {
        let r = julia_name(line, start..range.end);
        return Some((r.clone(), &line[r]));
    }
    if kind == Some(Kind::R) {
        let start = line[..range.start].trim_end_matches('.').len();
        return Some((start..range.end, &line[start..range.end]));
    }
    if let Some(k) = kind.filter(|&k| lisp(k)) {
        let r = lisp_name(k, line, range);
        return Some((r.clone(), &line[r]));
    }
    let mut end = range.end;
    let rest = &line.as_bytes()[end..];
    let suffixed = match kind {
        Some(Kind::Ruby) => {
            !matches!(rest.get(1), Some(b'=' | b'~')) && !line[..start].ends_with(['@', '$'])
        }
        Some(Kind::Elixir) => rest.get(1) != Some(&b'='),
        _ => false,
    };
    if suffixed && matches!(rest.first(), Some(b'?' | b'!')) {
        end += 1;
    }
    Some((start..end, &line[start..end]))
}
pub fn dart_name(line: &str, r: Range<usize>) -> Range<usize> {
    let b = line.as_bytes();
    let code: std::collections::HashSet<usize> = code(Kind::Dart, line).map(|(i, _)| i).collect();
    let part = |i: usize| {
        b[i].is_ascii_alphanumeric() || b[i] == b'_' || (b[i] == b'$' && code.contains(&i))
    };
    let (mut start, mut end) = (r.start, r.end);
    while start > 0 && part(start - 1) {
        start -= 1;
    }
    while end < b.len() && part(end) {
        end += 1;
    }
    while start < end && b[end - 1] == b'$' {
        end -= 1;
    }
    start..end
}
fn owner_line(lines: &[&str], line0: usize) -> Option<usize> {
    let depth = indent(lines.get(line0)?);
    (0..line0)
        .rev()
        .find(|&i| !aside(lines[i].trim_start()) && indent(lines[i]) < depth)
}
pub fn ruby_singleton(text: &str, line1: usize) -> bool {
    static ON: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"^\s*(?:def\s+(?:self|[A-Z]\w*)\.|scope\s*\(?\s*:|(?:[mc]attr_\w+|config_accessor)\s)",
        )
        .unwrap()
    });
    static MODULE_WIDE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:extend\s+self|module_function)\s*(?:#|$)").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(t) = line1.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    if ON.is_match(t) {
        return true;
    }
    let Some(owner) = owner_line(&lines, line1 - 1) else {
        return false;
    };
    let head = lines[owner].trim_start();
    // ponytail: a bare `module_function` anywhere in the module's body counts for every `def`
    // of it, the ones above it too; its list form, `module_function :m`, is not read.
    head.starts_with("class << self")
        || (head.starts_with("module ")
            && lines[owner + 1..]
                .iter()
                .take_while(|l| l.trim().is_empty() || indent(l) > indent(lines[owner]))
                .any(|l| MODULE_WIDE.is_match(l)))
}
pub fn ruby_concern_class_method(text: &str, line1: usize, module: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(owner) = line1.checked_sub(1).and_then(|i| owner_line(&lines, i)) else {
        return false;
    };
    let head = lines[owner].trim_start();
    let inside = |block: &str| {
        qualified(Kind::Ruby, text, owner + 1, block).is_some_and(|q| {
            q == format!("{module}.{block}") || q.ends_with(&format!(".{module}.{block}"))
        })
    };
    let scope = ["scope ", "scope("]
        .iter()
        .any(|s| lines[line1 - 1].trim_start().starts_with(s));
    (head.starts_with("class_methods do") && inside("class_methods"))
        || (head.starts_with("module ClassMethods") && inside("ClassMethods"))
        || (scope && head.starts_with("included do") && inside("included"))
}
pub fn word_at<'a>(line: &'a str, byte_col: usize, extra: &str) -> Option<(Range<usize>, &'a str)> {
    let b = line.as_bytes();
    let word =
        |i: usize| b[i].is_ascii_alphanumeric() || b[i] == b'_' || extra.contains(b[i] as char);
    let mut i = byte_col.min(b.len());
    if i == b.len() || !word(i) {
        if i > 0 && word(i - 1) {
            i -= 1;
        } else {
            return None;
        }
    }
    let mut start = i;
    while start > 0 && word(start - 1) {
        start -= 1;
    }
    let mut end = i + 1;
    while end < b.len() && word(end) {
        end += 1;
    }
    while start < end && extra.contains(b[start] as char) {
        start += 1;
    }
    while end > start && extra.contains(b[end - 1] as char) {
        end -= 1;
    }
    (start < end).then(|| (start..end, &line[start..end]))
}
pub fn binds_at(kind: Kind, line: &str, start: usize, word: &str) -> bool {
    match kind {
        Kind::Rust => rust_let_declares(line, start, word),
        Kind::Julia => julia_assigns_here(line, start, word),
        _ => false,
    }
}
