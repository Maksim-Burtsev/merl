use regex::Regex;
use std::sync::LazyLock;

use super::c::{
    Head, c_code, c_head, c_head_start, c_innermost_open_brace, c_struct_head, c_words_on_line,
};

pub fn cpp_scope_path(code: &str, pos: usize) -> Vec<String> {
    static NAMESPACE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[\n;])\s*(?:inline\s+)?namespace\b\s*((?:\w+\s*::\s*)*\w+)?\s*$")
            .expect("the regex is a literal")
    });
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
    let mut path = cpp_scope_path(&code, at);
    (!path.is_empty()).then(|| {
        path.push(name.to_owned());
        path.join("::")
    })
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
    static TOKEN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\w+|::|&&|\.\.\.|\S").expect("the regex is a literal"));
    let tokens: Vec<&str> = TOKEN.find_iter(head).map(|m| m.as_str()).collect();
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
                out.push(parameter_type(&inner[from..i]));
                from = i + 1;
            }
            _ => {}
        }
    }
    if !inner.is_empty() && inner != ["void"] {
        out.push(parameter_type(&inner[from..]));
    }
    let quals = tokens[close + 1..]
        .iter()
        .filter(|t| matches!(**t, "const" | "volatile" | "&" | "&&"));
    out.push(quals.copied().collect::<Vec<_>>().join(" "));
    Some(out)
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
}
