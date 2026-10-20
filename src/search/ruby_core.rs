use std::sync::LazyLock;

use regex::Regex;

use super::ruby_locals;

const STRING: &[&str] = &[
    "strip",
    "lstrip",
    "rstrip",
    "chomp",
    "chop",
    "chars",
    "bytes",
    "lines",
    "upcase",
    "downcase",
    "capitalize",
    "swapcase",
    "size",
    "length",
    "split",
    "gsub",
    "sub",
    "tr",
    "squeeze",
    "start_with?",
    "end_with?",
    "include?",
    "index",
    "rindex",
    "center",
    "ljust",
    "rjust",
    "reverse",
    "to_i",
    "to_f",
    "to_sym",
    "to_str",
    "encoding",
    "force_encoding",
    "encode",
    "unpack",
    "unpack1",
    "scan",
    "match",
    "match?",
    "empty?",
    "each_char",
    "each_line",
    "slice",
    "count",
    "ord",
    "hex",
    "oct",
    "bytesize",
    "partition",
    "rpartition",
    "prepend",
    "concat",
    "unicode_normalize",
    "strip!",
    "chomp!",
    "upcase!",
    "downcase!",
    "gsub!",
    "sub!",
    "squish",
];

const HASH: &[&str] = &[
    "dig",
    "fetch",
    "keys",
    "values",
    "each",
    "each_pair",
    "each_key",
    "each_value",
    "map",
    "select",
    "reject",
    "filter_map",
    "merge",
    "merge!",
    "key?",
    "has_key?",
    "value?",
    "include?",
    "empty?",
    "size",
    "length",
    "to_a",
    "transform_values",
    "transform_keys",
    "group_by",
    "sort_by",
    "min_by",
    "max_by",
    "sum",
    "count",
    "find",
    "any?",
    "all?",
    "none?",
    "store",
    "compact",
    "invert",
    "values_at",
    "fetch_values",
    "except",
    "each_with_object",
    "flat_map",
];

const ARRAY: &[&str] = &[
    "each",
    "map",
    "select",
    "reject",
    "first",
    "last",
    "push",
    "pop",
    "shift",
    "unshift",
    "join",
    "include?",
    "index",
    "size",
    "length",
    "empty?",
    "flatten",
    "compact",
    "uniq",
    "sort",
    "sort_by",
    "reverse",
    "sum",
    "min",
    "max",
    "count",
    "find",
    "any?",
    "all?",
    "none?",
    "each_with_index",
    "each_with_object",
    "inject",
    "reduce",
    "zip",
    "take",
    "drop",
    "dig",
    "fetch",
    "group_by",
    "partition",
    "flat_map",
    "filter_map",
    "tally",
    "concat",
    "to_h",
];

const STRING_TO_STRING: &[&str] = &[
    "strip",
    "lstrip",
    "rstrip",
    "chomp",
    "chop",
    "upcase",
    "downcase",
    "capitalize",
    "swapcase",
    "gsub",
    "sub",
    "tr",
    "squeeze",
    "center",
    "ljust",
    "rjust",
    "reverse",
    "dup",
    "freeze",
    "encode",
    "force_encoding",
    "unicode_normalize",
];

const FILE_TO_STRING: &[&str] = &[
    "basename",
    "dirname",
    "join",
    "extname",
    "read",
    "expand_path",
    "absolute_path",
];

pub fn ruby_core_method(class: &str, word: &str) -> bool {
    match class {
        "String" => STRING.contains(&word),
        "Hash" => HASH.contains(&word),
        "Array" => ARRAY.contains(&word),
        _ => false,
    }
}

pub fn ruby_reopens_core(opener: &str, class: &str) -> bool {
    static OPENS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:class|module|refine)\s+(?:::)?(\w+)\b").expect("the regex is a literal")
    });
    OPENS.captures(opener).is_some_and(|c| {
        matches!(
            &c[1],
            "Object" | "Kernel" | "BasicObject" | "Comparable" | "Enumerable"
        ) || &c[1] == class
    })
}

pub fn ruby_receiver_split(before: &str) -> (&str, &str) {
    let mut depth = 0usize;
    let mut quote = None;
    for (i, c) in before.char_indices().rev() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            ')' | ']' | '}' => depth += 1,
            '(' | '[' | '{' if depth == 0 => return before.split_at(i + 1),
            '(' | '[' | '{' => depth -= 1,
            _ if depth > 0 => {}
            c if c.is_alphanumeric() || matches!(c, '_' | '.' | ':' | '?' | '!' | '&') => {}
            _ => return before.split_at(i + c.len_utf8()),
        }
    }
    ("", before)
}

pub fn ruby_last_call(expr: &str) -> Option<(&str, &str)> {
    let callee = match expr.strip_suffix(')') {
        Some(inner) => {
            let mut depth = 0usize;
            let open = inner.char_indices().rev().find_map(|(i, c)| match c {
                ')' => {
                    depth += 1;
                    None
                }
                '(' if depth == 0 => Some(i),
                '(' => {
                    depth -= 1;
                    None
                }
                _ => None,
            })?;
            inner.get(..open)?.trim_end()
        }
        None => expr,
    };
    let (receiver, method) = callee.rsplit_once('.')?;
    let receiver = receiver.strip_suffix('&').unwrap_or(receiver);
    let word = |s: &str| {
        !s.is_empty()
            && s.trim_end_matches(['?', '!'])
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_')
    };
    (word(method) && !receiver.is_empty()).then_some((receiver, method))
}

fn literal(prefix: &str, expr: &str) -> Option<&'static str> {
    let opens = !prefix.trim_end().ends_with(|c: char| {
        c.is_alphanumeric() || matches!(c, '_' | ')' | ']' | '}' | '?' | '!' | '"' | '\'')
    });
    let (first, last) = (expr.chars().next()?, expr.chars().last()?);
    match (first, last) {
        ('"', '"') | ('\'', '\'') if expr.len() > 1 => Some("String"),
        ('{', '}') if opens => Some("Hash"),
        ('[', ']') if opens => Some("Array"),
        _ => None,
    }
}

fn core_value(
    text: &str,
    line1: usize,
    prefix: &str,
    expr: &str,
    constant: &dyn Fn(&str) -> bool,
    depth: usize,
) -> Option<&'static str> {
    if depth > 4 {
        return None;
    }
    let expr = expr.trim();
    if let Some((receiver, method)) = ruby_last_call(expr) {
        return match method {
            "to_s" | "inspect" => Some("String"),
            "to_a" => Some("Array"),
            "to_h" => Some("Hash"),
            m if FILE_TO_STRING.contains(&m)
                && matches!(receiver, "File" | "::File")
                && !constant("File") =>
            {
                Some("String")
            }
            m => {
                let class = core_value(text, line1, prefix, receiver, constant, depth + 1)?;
                match class {
                    "String" if STRING_TO_STRING.contains(&m) => Some("String"),
                    "String" if matches!(m, "split" | "chars" | "lines" | "bytes") => Some("Array"),
                    "Array" if m == "join" => Some("String"),
                    "Hash" if matches!(m, "keys" | "values") => Some("Array"),
                    _ => None,
                }
            }
        };
    }
    if let Some(class) = literal(prefix, expr) {
        return Some(class);
    }
    if !expr.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
        || !expr.chars().all(|c| c.is_alphanumeric() || c == '_')
    {
        return None;
    }
    let [at] = ruby_locals(text, line1, expr)[..] else {
        return None;
    };
    let line = text.lines().nth(at.checked_sub(1)?)?;
    if at >= line1 {
        return None;
    }
    let value = line.trim_start().strip_prefix(expr)?.trim_start();
    let value = value
        .strip_prefix('=')
        .filter(|v| !v.starts_with(['=', '~', '>']))?;
    let value = value.trim();
    let (head, whole) = ruby_receiver_split(value);
    if !head.trim().is_empty() {
        return None;
    }
    core_value(text, at, "", whole, constant, depth + 1)
}

pub fn ruby_core_receiver(
    text: &str,
    line1: usize,
    before: &str,
    constant: &dyn Fn(&str) -> bool,
) -> Option<&'static str> {
    let receiver = before.strip_suffix('.')?;
    let receiver = receiver.strip_suffix('&').unwrap_or(receiver);
    let (prefix, expr) = ruby_receiver_split(receiver);
    core_value(text, line1, prefix, expr, constant, 0)
}
