use regex::Regex;
use std::sync::LazyLock;

use super::c::{
    Head, bracket_contents_blanked, c_code, c_head, c_head_start, c_innermost_open_brace,
    c_struct_head, c_words_on_line, close_or_end, cpp_decl_start,
};

static NAMESPACE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[\n;])\s*(?:inline\s+)?namespace\b\s*((?:\w+\s*::\s*)*\w+)?\s*$")
        .expect("the regex is a literal")
});

pub fn cpp_scope_path(code: &str, pos: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut at = pos;
    while let Some(open) = c_innermost_open_brace(code.as_bytes(), at) {
        let head = code.get(c_head_start(code, open)..open).unwrap_or_default();
        let path = match (c_struct_head(head), NAMESPACE.captures(head)) {
            (Some(name), _) => name,
            (None, Some(c)) => c.get(1).map_or("", |m| m.as_str()).to_owned(),
            (None, None) => match c_head(head) {
                Head::Function { name, .. } => {
                    name.rsplit_once("::").unzip().0.unwrap_or("").to_owned()
                }
                _ => String::new(),
            },
        };
        let parts = path.rsplit("::").map(str::trim).filter(|p| !p.is_empty());
        out.extend(parts.map(str::to_owned));
        at = open;
    }
    out.reverse();
    out
}

pub fn cpp_scoped_name(text: &str, line1: usize, name: &str) -> Option<String> {
    let code = c_code(text);
    let at = *c_words_on_line(&code, line1, name, None).first()?;
    let rest = code.get(at + name.len()..)?.trim_start();
    let body = rest.starts_with('{')
        || rest.starts_with("final")
        || (rest.starts_with(':') && !rest.starts_with("::"));
    if !body || code.get(..at)?.trim_end().ends_with("::") {
        return None;
    }
    let mut path = vec![name.to_owned()];
    let mut pos = at;
    while let Some(open) = c_innermost_open_brace(code.as_bytes(), pos) {
        let head = code.get(c_head_start(&code, open)..open)?;
        let named = match (c_struct_head(head), NAMESPACE.captures(head)) {
            (Some(class), _) if !class.is_empty() && !head.contains("::") => class,
            (None, Some(c)) => c.get(1)?.as_str().to_owned(),
            _ => return None,
        };
        path.extend(named.rsplit("::").map(|p| p.trim().to_owned()));
        pos = open;
    }
    path.reverse();
    (path.len() > 1).then(|| path.join("::"))
}

pub fn cpp_owner(text: &str, line1: usize, name: &str) -> Option<Vec<String>> {
    static PURE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\)[^;=]*=\s*0\s*;").expect("the regex is a literal"));
    let code = c_code(text);
    let at = *c_words_on_line(&code, line1, name, None).first()?;
    let line = code.get(at..)?.lines().next()?;
    if PURE.is_match(line) {
        return None;
    }
    let mut path = cpp_scope_path(&code, at);
    let head = code.get(..at)?.trim_end();
    if let Some(head) = head.strip_suffix("::") {
        let start = head
            .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':'))
            .map_or(0, |i| i + 1);
        let qualifier = head.get(start..)?.split("::").filter(|p| !p.is_empty());
        path.extend(qualifier.map(str::to_owned));
    }
    Some(path)
}

pub fn cpp_alias_in_scope(text: &str, pos: usize, alias: &str) -> Option<Vec<String>> {
    let code = c_code(text);
    let re = Regex::new(&format!(
        r"\bnamespace\s+{}\s*=\s*((?:\w+\s*::\s*)*\w+)\s*;",
        regex::escape(alias)
    ))
    .ok()?;
    let mut open = Vec::new();
    let mut at = pos.min(code.len());
    while let Some(o) = c_innermost_open_brace(code.as_bytes(), at) {
        open.push(o);
        at = o;
    }
    let seen = |start: usize| {
        start < pos
            && c_innermost_open_brace(code.as_bytes(), start).is_none_or(|o| open.contains(&o))
    };
    let c = re
        .captures_iter(&code)
        .filter(|c| c.get(0).is_some_and(|m| seen(m.start())))
        .last()?;
    Some(c[1].split("::").map(|p| p.trim().to_owned()).collect())
}

pub fn cpp_one_namespace(
    name: &str,
    rows: &[super::Hit],
    text_of: impl Fn(&std::path::Path) -> Option<String>,
) -> bool {
    let w = regex::escape(name);
    let (Ok(opens), Ok(uses)) = (
        Regex::new(&format!(
            r"^\s*(?:inline\s+)?namespace\s+((?:\w+\s*::\s*)*){w}\s*(?:\{{|$)"
        )),
        Regex::new(&format!(r"^\s*using\s+namespace\s+(?:\w+\s*::\s*)*{w}\s*;")),
    ) else {
        return false;
    };
    let mut paths = Vec::new();
    for h in rows.iter().filter(|h| !uses.is_match(&h.text)) {
        let (Some(c), Some(text)) = (opens.captures(&h.text), text_of(&h.path)) else {
            return false;
        };
        let code = c_code(&text);
        let start: usize = (text.split('\n').take(h.line1.saturating_sub(1)))
            .map(|l| l.len() + 1)
            .sum();
        let mut path = cpp_scope_path(&code, start.min(code.len()));
        path.extend(
            (c[1].split("::").map(str::trim))
                .filter(|p| !p.is_empty())
                .map(str::to_owned),
        );
        paths.push(path);
    }
    paths.first().is_some_and(|p| paths.iter().all(|q| q == p))
}

pub fn cpp_namespace_alias(code: &str, alias: &str) -> Option<Vec<String>> {
    let re = Regex::new(&format!(
        r"\bnamespace\s+{}\s*=\s*((?:\w+\s*::\s*)*\w+)\s*;",
        regex::escape(alias)
    ))
    .ok()?;
    let c = re.captures(code)?;
    Some(c[1].split("::").map(|p| p.trim().to_owned()).collect())
}

pub fn cpp_using_declares(code: &str, name: &str) -> bool {
    Regex::new(&format!(
        r"\busing\s+(?:\w+\s*::\s*)+{}\s*;",
        regex::escape(name)
    ))
    .is_ok_and(|re| re.is_match(code))
}

pub fn cpp_defined_signature(text: &str, line1: usize, word: &str) -> Option<Vec<String>> {
    let code = c_code(text);
    let at = *c_words_on_line(&code, line1, word, Some('(')).first()?;
    let open = at + code.get(at..)?.find(['{', ';'])?;
    if code.as_bytes()[open] != b'{' {
        return None;
    }
    let start = c_head_start(&code, open);
    let Head::Function { params, name } = c_head(code.get(start..open)?) else {
        return None;
    };
    if start + params.start < at || !name.ends_with(&format!("::{word}")) {
        return None;
    }
    signature(code.get(start + params.start..open)?)
}

pub fn cpp_declared_signature(text: &str, line1: usize, word: &str) -> Option<Vec<String>> {
    let code = c_code(text);
    c_words_on_line(&code, line1, word, Some('('))
        .into_iter()
        .find_map(|at| {
            let open = at + code.get(at..)?.find('(')?;
            let end = open + code.get(open..)?.find(['{', ';'])?;
            signature(code.get(open..end)?)
        })
}

fn signature(head: &str) -> Option<Vec<String>> {
    let tokens = head_tokens(head);
    let (params, close) = split_parameters(&tokens)?;
    let mut out: Vec<String> = params.iter().map(|p| parameter_type(p)).collect();
    let quals = tokens[close + 1..]
        .iter()
        .filter(|t| matches!(**t, "const" | "volatile" | "&" | "&&"));
    out.push(quals.copied().collect::<Vec<_>>().join(" "));
    Some(out)
}

fn head_tokens(head: &str) -> Vec<&str> {
    static TOKEN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\w+|::|&&|\.\.\.|\S").expect("the regex is a literal"));
    TOKEN.find_iter(head).map(|m| m.as_str()).collect()
}

fn split_parameters<'a>(tokens: &'a [&'a str]) -> Option<(Vec<&'a [&'a str]>, usize)> {
    let close = tokens.iter().position(|t| *t == ")")?;
    let inner = tokens.get(1..close)?;
    if tokens.first() != Some(&"(") || inner.contains(&"(") {
        return None;
    }
    let mut out = Vec::new();
    let (mut depth, mut from) = (0usize, 0);
    for (i, t) in inner.iter().enumerate() {
        match *t {
            "<" => depth += 1,
            ">" => depth = depth.checked_sub(1)?,
            "," if depth == 0 => {
                out.push(&inner[from..i]);
                from = i + 1;
            }
            _ => {}
        }
    }
    if !inner.is_empty() && inner != ["void"] {
        out.push(&inner[from..]);
    }
    Some((out, close))
}

fn parameter_type(param: &[&str]) -> String {
    const TYPE_WORDS: &[&str] = &[
        "int", "char", "short", "long", "float", "double", "void", "signed", "unsigned", "bool",
        "auto", "const", "volatile", "struct", "class", "enum", "union", "typename",
    ];
    const BEFORE_TYPE: &[&str] = &[
        "const", "volatile", "struct", "class", "enum", "union", "typename",
    ];
    let param = param
        .iter()
        .position(|t| *t == "=")
        .map_or(param, |eq| &param[..eq]);
    let named = match param {
        [.., before, last] => {
            last.starts_with(|c: char| c.is_alphabetic() || c == '_')
                && !TYPE_WORDS.contains(last)
                && !BEFORE_TYPE.contains(before)
                && (before.starts_with(|c: char| c.is_alphanumeric() || c == '_')
                    || matches!(*before, "*" | "&" | "&&" | ">"))
        }
        _ => false,
    };
    let kept = if named {
        &param[..param.len() - 1]
    } else {
        param
    };
    kept.join(" ")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CppArg {
    NonZeroInt,
    Str,
    Literal,
    Name(String),
    Class(Vec<String>),
    Other,
}

pub fn cpp_call_args(text: &str, open: usize) -> Option<Vec<CppArg>> {
    static INT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[1-9][0-9']*[uUlL]*$").expect("the regex is a literal"));
    static LITERAL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:[0-9][\w'.]*|true|false|nullptr)$").expect("the regex is a literal")
    });
    static NAME: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[A-Za-z_]\w*$").expect("the regex is a literal"));
    let code = c_code(text);
    let b = code.as_bytes();
    if b.get(open) != Some(&b'(') {
        return None;
    }
    let (mut depth, mut from, mut ranges) = (0usize, open + 1, Vec::new());
    let mut i = open + 1;
    loop {
        match *b.get(i)? {
            b')' if depth == 0 => {
                ranges.push(from..i);
                break;
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.checked_sub(1)?,
            b',' if depth == 0 => {
                ranges.push(from..i);
                from = i + 1;
            }
            b'<' | b';' => return None,
            _ => {}
        }
        i += 1;
    }
    if let [only] = ranges.as_slice()
        && text.get(only.clone())?.trim().is_empty()
    {
        return Some(Vec::new());
    }
    ranges
        .into_iter()
        .map(|r| {
            let (raw, c) = (text.get(r.clone())?.trim(), code.get(r)?.trim());
            Some(if raw.is_empty() {
                return None;
            } else if c.is_empty() && raw.starts_with('"') && raw.ends_with('"') {
                CppArg::Str
            } else if c.is_empty() {
                CppArg::Literal
            } else if INT.is_match(c) {
                CppArg::NonZeroInt
            } else if LITERAL.is_match(c) {
                CppArg::Literal
            } else if NAME.is_match(c) && c != "this" {
                CppArg::Name(c.to_owned())
            } else {
                CppArg::Other
            })
        })
        .collect()
}

pub fn cpp_parameters(text: &str, line1: usize, word: &str) -> Option<Vec<Vec<String>>> {
    let code = c_code(text);
    let at = *c_words_on_line(&code, line1, word, Some('(')).first()?;
    let open = at + code.get(at..)?.find('(')?;
    let end = open + code.get(open..)?.find(['{', ';'])?;
    let tokens = head_tokens(code.get(open..end)?);
    let (params, _) = split_parameters(&tokens)?;
    Some(
        (params.into_iter())
            .map(|p| {
                (parameter_type(p).split_whitespace())
                    .filter(|t| !matches!(*t, "const" | "volatile" | "&" | "&&"))
                    .map(str::to_owned)
                    .collect()
            })
            .collect(),
    )
}

pub fn cpp_not_viable(params: &[Vec<String>], args: &[CppArg]) -> bool {
    const ARITHMETIC: &[&str] = &[
        "int", "long", "short", "unsigned", "signed", "char", "float", "double",
    ];
    let fixed = (params.iter())
        .take_while(|ty| !ty.iter().any(|t| t == "..."))
        .count();
    if fixed == params.len() && args.len() > params.len() {
        return true;
    }
    params[..fixed].iter().zip(args).any(|(ty, arg)| {
        let arithmetic = !ty.is_empty() && ty.iter().all(|t| ARITHMETIC.contains(&t.as_str()));
        match arg {
            CppArg::NonZeroInt => ty.iter().any(|t| t == "*"),
            CppArg::Str | CppArg::Class(_) => arithmetic,
            CppArg::Literal | CppArg::Name(_) | CppArg::Other => false,
        }
    })
}

enum Brace {
    Namespace(Vec<String>),
    Class,
    Linkage,
    Code,
    Unknown,
}

fn brace(code: &str, open: usize) -> Brace {
    static MACRO: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[A-Z][A-Z0-9_]*$").expect("the regex is a literal"));
    let head = code.get(c_head_start(code, open)..open).unwrap_or_default();
    if c_struct_head(head).is_some() {
        return Brace::Class;
    }
    if let Some(c) = NAMESPACE.captures(head) {
        return match c.get(1) {
            Some(name) if !c[0].contains("inline") => Brace::Namespace(
                (name.as_str().split("::"))
                    .map(|p| p.trim().to_owned())
                    .collect(),
            ),
            _ => Brace::Unknown,
        };
    }
    match c_head(head) {
        Head::TypeOrNamespace if head.trim() == "extern" => Brace::Linkage,
        Head::TypeOrNamespace => Brace::Unknown,
        Head::Function { name, .. } if name.contains("::") || MACRO.is_match(&name) => {
            Brace::Unknown
        }
        Head::Block if head.contains(['(', ':']) => Brace::Unknown,
        _ => Brace::Code,
    }
}

pub fn cpp_call_namespaces(code: &str, pos: usize) -> Option<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    let mut at = pos;
    while let Some(open) = c_innermost_open_brace(code.as_bytes(), at) {
        match brace(code, open) {
            Brace::Namespace(n) => out = [n, out].concat(),
            Brace::Linkage | Brace::Code => {}
            Brace::Class | Brace::Unknown => return None,
        }
        at = open;
    }
    Some(out)
}

pub fn cpp_function_namespace(
    text: &str,
    line1: usize,
    word: &str,
) -> Result<Option<Vec<String>>, ()> {
    static QUALIFIER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\b((?:\w+\s*::\s*)+)$").expect("the regex is a literal"));
    let code = c_code(text);
    let at = *c_words_on_line(&code, line1, word, Some('('))
        .first()
        .ok_or(())?;
    let mut out: Vec<String> = Vec::new();
    let mut pos = at;
    while let Some(open) = c_innermost_open_brace(code.as_bytes(), pos) {
        match brace(&code, open) {
            Brace::Namespace(n) => out = [n, out].concat(),
            Brace::Linkage => {}
            Brace::Class
                if !text
                    .lines()
                    .nth(line1 - 1)
                    .is_some_and(|l| l.contains("friend")) =>
            {
                return Ok(None);
            }
            Brace::Class | Brace::Code | Brace::Unknown => return Err(()),
        }
        pos = open;
    }
    if let Some(q) = QUALIFIER.captures(code.get(..at).unwrap_or_default()) {
        out.extend(
            q[1].split("::")
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(str::to_owned),
        );
    }
    Ok(Some(out))
}

pub fn cpp_usings(code: &str, word: &str, pos: usize) -> Result<Vec<Vec<String>>, ()> {
    static DIRECTIVE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\busing\s+namespace\b").expect("the regex is a literal"));
    if DIRECTIVE.is_match(code) {
        return Err(());
    }
    let re = Regex::new(&format!(
        r"\busing\s+((?:\w+\s*::\s*)+){}\s*;",
        regex::escape(word)
    ))
    .map_err(|_| ())?;
    let mut out = Vec::new();
    for c in re.captures_iter(code) {
        let start = c.get(0).map_or(0, |m| m.start());
        match (c_innermost_open_brace(code.as_bytes(), start), start < pos) {
            (Some(_), _) => return Err(()),
            (None, true) => out.push(
                (c[1].split("::").map(str::trim))
                    .filter(|p| !p.is_empty())
                    .map(str::to_owned)
                    .collect(),
            ),
            (None, false) => {}
        }
    }
    Ok(out)
}

pub fn cpp_plain_class_namespace(code: &str, open: usize) -> Option<Vec<String>> {
    static BASE_OR_TEMPLATE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\btemplate\b|(?:^|[^:]):(?:[^:]|$)").expect("the regex is a literal")
    });
    static OPERATOR: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\boperator\b").expect("the regex is a literal"));
    let head = code.get(c_head_start(code, open)..open)?;
    let body = code.get(open..close_or_end(code, open, code.len()))?;
    if BASE_OR_TEMPLATE.is_match(head) || OPERATOR.is_match(body) {
        return None;
    }
    let mut out: Vec<String> = Vec::new();
    let mut at = open;
    while let Some(o) = c_innermost_open_brace(code.as_bytes(), at) {
        match brace(code, o) {
            Brace::Namespace(n) => out = [n, out].concat(),
            Brace::Linkage => {}
            Brace::Class | Brace::Code | Brace::Unknown => return None,
        }
        at = o;
    }
    Some(out)
}

pub struct CppWritten {
    pub pointer: bool,
    pub first_arg: Option<String>,
}

pub fn cpp_written(code: &str, at: usize, class: &str) -> Option<CppWritten> {
    let written = code.get(cpp_decl_start(code, at)?..at)?;
    let mut depth = 0usize;
    let outer: String = (written.chars())
        .filter(|&c| match c {
            '<' => {
                depth += 1;
                false
            }
            '>' => {
                depth = depth.saturating_sub(1);
                false
            }
            _ => depth == 0,
        })
        .collect();
    let w = regex::escape(class);
    Regex::new(&format!(r"\b{w}\b"))
        .ok()?
        .is_match(&outer)
        .then_some(())?;
    let opened = Regex::new(&format!(r"\b{w}\s*<")).ok()?.find(written);
    Some(CppWritten {
        pointer: outer.trim_end().ends_with('*'),
        first_arg: opened.and_then(|m| first_arg(written.get(m.end()..)?)),
    })
}

pub fn cpp_std_class_body(code: &str, line1: usize, name: &str) -> Option<(usize, Option<String>)> {
    let w = regex::escape(name);
    let re = Regex::new(&format!(
        r"^\s*(?:struct|class)\s+(?:[A-Z_][A-Z0-9_]*\s+)*{w}\b\s*(<)?"
    ))
    .ok()?;
    c_words_on_line(code, line1, name, None)
        .into_iter()
        .find_map(|at| {
            let start = code.get(..at)?.rfind('\n').map_or(0, |n| n + 1);
            let open = at + code.get(at..)?.find([';', '{', '(', '='])?;
            let head = code.get(start..open)?;
            let special = re.captures(head)?.get(1).map(|lt| {
                (head.get(lt.end()..))
                    .and_then(first_arg)
                    .unwrap_or_else(|| "_".to_owned())
            });
            (code.as_bytes()[open] == b'{' && c_struct_head(head).as_deref() == Some(name))
                .then_some((open, special))
        })
}

fn first_arg(s: &str) -> Option<String> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '<' | '(' => depth += 1,
            '>' | ')' if depth > 0 => depth -= 1,
            ',' | '>' if depth == 0 => {
                return Some(s.get(..i)?.split_whitespace().collect::<Vec<_>>().join(" "));
            }
            _ => {}
        }
    }
    None
}

pub(super) fn init_capture(code: &str, span: std::ops::Range<usize>, name: &str) -> Option<usize> {
    let (base, head) = (span.start, code.get(span)?);
    let close = head.trim_end().strip_suffix(']')?.len();
    let open = bracket_contents_blanked(head).get(..close)?.rfind('[')?;
    let list = head.get(open + 1..close)?;
    let mut from = 0;
    for entry in bracket_contents_blanked(list).split(',') {
        let at = from;
        from += entry.len() + 1;
        let raw = list.get(at..at + entry.len())?;
        let lead = raw.len() - raw.trim_start_matches([' ', '\t', '\n', '&']).len();
        if (raw.get(lead..)?.strip_prefix(name))
            .is_some_and(|r| r.trim_start().starts_with(['=', '{', '(']))
        {
            return Some(base + open + 1 + at + lead);
        }
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_definition_s_parameter_types_are_its_declaration_s() {
        let declared = |line: &str| cpp_declared_signature(line, 1, "f");
        let defined = |line: &str| cpp_defined_signature(line, 1, "f");
        let def = defined("int K::f(const char* cups, int n = 2) const {");
        assert_eq!(def, declared("  int f(const char *, int) const;"));
        assert_ne!(def, declared("  int f(const char* cups, int n);"));
        assert_ne!(def, declared("  int f(int cups, int n) const;"));
        assert_eq!(
            defined("void K::f(std::map<int, long long> m) {"),
            declared("void f(std::map<int, long long>);")
        );
        assert_ne!(
            defined("void K::f(long long x) {"),
            declared("void f(long);")
        );
        assert_eq!(defined("void K::f(void) {"), declared("void f();"));
        assert_eq!(defined("if (K::f(x)) {"), None);
        assert_eq!(defined("void f(int x) {"), None);
        assert_eq!(declared("void f(int (*cb)(int));"), None);
    }

    #[test]
    fn a_call_rules_out_only_what_its_plain_arguments_cannot_reach() {
        let args = |call: &str| cpp_call_args(call, call.find('(').expect("a call")).expect("args");
        let params = |decl: &str| cpp_parameters(decl, 1, "f").expect("params");
        let out = |decl: &str, call: &str| cpp_not_viable(&params(decl), &args(call));
        assert_eq!(
            args(r#"f(1, "a", 0, x, a + b, 'c')"#),
            [
                CppArg::NonZeroInt,
                CppArg::Str,
                CppArg::Literal,
                CppArg::Name("x".into()),
                CppArg::Other,
                CppArg::Literal
            ]
        );
        assert_eq!(cpp_call_args("f(a < b, c)", 1), None);
        assert!(out("int f(const char* s);", "f(1)"));
        assert!(!out("int f(const char* s);", "f(0)"));
        assert!(out("int f(int n);", r#"f("a")"#));
        assert!(!out("int f(bool b);", r#"f("a")"#));
        assert!(out("int f(int n);", "f(1, 2)"));
        assert!(!out("int f(const char* s, int n);", r#"f("a")"#));
        assert!(!out("int f(const char* s, ...);", r#"f("a", 1, 2)"#));
        assert!(!out("int f(Leaf leaf);", "f(1)"));
        assert!(!out("int f(const long& n);", "f(x)"));
    }

    #[test]
    fn an_unqualified_call_sees_its_namespaces_and_the_usings_before_it() {
        let code = "namespace a {\nint g() {\n  return f(1);\n}\n}\nusing b::f;\nstruct S {\nint h() { return f(2); }\n};\n";
        let code = c_code(code);
        assert_eq!(
            cpp_call_namespaces(&code, code.find("f(1)").expect("a call")),
            Some(vec!["a".into()])
        );
        assert_eq!(
            cpp_call_namespaces(&code, code.find("f(2)").expect("a call")),
            None
        );
        let tried = c_code("int S::g() try {\n  return f(3);\n} catch (...) {}");
        assert_eq!(
            cpp_call_namespaces(&tried, tried.find("f(3)").expect("a call")),
            None
        );
        assert_eq!(
            cpp_usings(&code, "f", code.find("f(1)").expect("a call")),
            Ok(vec![])
        );
        assert_eq!(
            cpp_usings(&code, "f", code.len()),
            Ok(vec![vec!["b".into()]])
        );
        assert_eq!(cpp_usings("using namespace b;", "f", 0), Err(()));
        assert_eq!(cpp_usings("void g() { using b::f; }", "f", 0), Err(()));
        let ns = |text: &str, line1| cpp_function_namespace(text, line1, "f");
        assert_eq!(
            ns("namespace a::b {\nint f(int);\n}", 2),
            Ok(Some(vec!["a".into(), "b".into()]))
        );
        assert_eq!(ns("int a::f(int n) {}", 1), Ok(Some(vec!["a".into()])));
        assert_eq!(ns("struct S {\n  int f(int);\n};", 2), Ok(None));
        assert_eq!(ns("struct S {\n  friend int f(S);\n};", 2), Err(()));
        assert_eq!(ns("namespace {\nint f(int);\n}", 2), Err(()));
        assert_eq!(ns("inline namespace v1 {\nint f(int);\n}", 2), Err(()));
        assert_eq!(ns("extern \"C\" {\nint f(int);\n}", 2), Ok(Some(vec![])));
    }
}
