use regex::Regex;

use super::{
    Binding, Group, Kind, Value, code, continued, group, indent, literal_lines, names, split_top,
    uncommented, value_of,
};

pub const PYTHON_BUILTINS: &[&str] = &[
    "ArithmeticError",
    "AssertionError",
    "AttributeError",
    "BaseException",
    "BaseExceptionGroup",
    "BlockingIOError",
    "BrokenPipeError",
    "BufferError",
    "BytesWarning",
    "ChildProcessError",
    "ConnectionAbortedError",
    "ConnectionError",
    "ConnectionRefusedError",
    "ConnectionResetError",
    "DeprecationWarning",
    "EOFError",
    "Ellipsis",
    "EncodingWarning",
    "EnvironmentError",
    "Exception",
    "ExceptionGroup",
    "False",
    "FileExistsError",
    "FileNotFoundError",
    "FloatingPointError",
    "FutureWarning",
    "GeneratorExit",
    "IOError",
    "ImportError",
    "ImportWarning",
    "IndentationError",
    "IndexError",
    "InterruptedError",
    "IsADirectoryError",
    "KeyError",
    "KeyboardInterrupt",
    "LookupError",
    "MemoryError",
    "ModuleNotFoundError",
    "NameError",
    "None",
    "NotADirectoryError",
    "NotImplemented",
    "NotImplementedError",
    "OSError",
    "OverflowError",
    "PendingDeprecationWarning",
    "PermissionError",
    "ProcessLookupError",
    "PythonFinalizationError",
    "RecursionError",
    "ReferenceError",
    "ResourceWarning",
    "RuntimeError",
    "RuntimeWarning",
    "StopAsyncIteration",
    "StopIteration",
    "SyntaxError",
    "SyntaxWarning",
    "SystemError",
    "SystemExit",
    "TabError",
    "TimeoutError",
    "True",
    "TypeError",
    "UnboundLocalError",
    "UnicodeDecodeError",
    "UnicodeEncodeError",
    "UnicodeError",
    "UnicodeTranslateError",
    "UnicodeWarning",
    "UserWarning",
    "ValueError",
    "Warning",
    "ZeroDivisionError",
    "__build_class__",
    "__debug__",
    "__import__",
    "abs",
    "aiter",
    "all",
    "anext",
    "any",
    "ascii",
    "bin",
    "bool",
    "breakpoint",
    "bytearray",
    "bytes",
    "callable",
    "chr",
    "classmethod",
    "compile",
    "complex",
    "copyright",
    "credits",
    "delattr",
    "dict",
    "dir",
    "divmod",
    "enumerate",
    "eval",
    "exec",
    "exit",
    "filter",
    "float",
    "format",
    "frozenset",
    "getattr",
    "globals",
    "hasattr",
    "hash",
    "help",
    "hex",
    "id",
    "input",
    "int",
    "isinstance",
    "issubclass",
    "iter",
    "len",
    "license",
    "list",
    "locals",
    "map",
    "max",
    "memoryview",
    "min",
    "next",
    "object",
    "oct",
    "open",
    "ord",
    "pow",
    "print",
    "property",
    "quit",
    "range",
    "repr",
    "reversed",
    "round",
    "set",
    "setattr",
    "slice",
    "sorted",
    "staticmethod",
    "str",
    "sum",
    "tuple",
    "type",
    "vars",
    "zip",
];
pub const PYTHON_BUILTIN_TYPES: &[&str] = &[
    "str",
    "bytes",
    "bytearray",
    "int",
    "float",
    "complex",
    "bool",
    "list",
    "dict",
    "set",
    "frozenset",
    "tuple",
    "object",
    "type",
    "range",
    "memoryview",
    "slice",
];
pub const PYTHON_SCALAR_TYPES: &[&str] = &[
    "str",
    "bytes",
    "bytearray",
    "int",
    "float",
    "complex",
    "bool",
];
pub const PYTHON_STDLIB_MODULES: &[&str] = &[
    "os",
    "sys",
    "re",
    "json",
    "math",
    "time",
    "shutil",
    "subprocess",
    "string",
    "textwrap",
    "base64",
    "hashlib",
];
pub fn python_str_literal_receiver(before: &str) -> Option<&str> {
    static LITERAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r#"(?:^|[^\w'"])((?i:[rbfu]{0,2})(?:"[^"\\]*"|'[^'\\]*'))(?:\.\w+\([^()'"]*\))*\.$"#,
        )
        .expect("the regex is a literal")
    });
    LITERAL
        .captures(before)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
}
pub fn python_opened(line: &str, name: &str) -> bool {
    let t = uncommented(Kind::Python, line);
    Regex::new(&format!(
        r"^(?:async\s+)?with\s+open\([^()]*\)\s+as\s+{}\s*:",
        regex::escape(name)
    ))
    .is_ok_and(|r| r.is_match(t.trim()))
}
#[derive(Clone, Copy)]
enum Scope {
    Def(usize),
    Module,
}

fn closes_a_wrapped_header(t: &str) -> bool {
    t.starts_with([')', ']'])
}

static DEF: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^(?:async\s+)?def\s+(\w+)\s*\(").expect("the regex is a literal")
});

fn python_scopes_innermost_first(lines: &[&str], at: usize) -> Vec<Scope> {
    let mut scopes_innermost_first = Vec::new();
    let mut depth = indent(lines[at]);
    for i in (0..at).rev() {
        let t = lines[i].trim_start();
        if depth == 0 {
            break;
        }
        if t.is_empty()
            || t.starts_with('#')
            || closes_a_wrapped_header(t)
            || indent(lines[i]) >= depth
        {
            continue;
        }
        depth = indent(lines[i]);
        if DEF.is_match(t) {
            scopes_innermost_first.push(Scope::Def(i));
        }
    }
    scopes_innermost_first.push(Scope::Module);
    scopes_innermost_first
}

fn python_def_body(lines: &[&str], d: usize, name: &str, out: &mut Vec<Binding>) -> Option<usize> {
    let open = lines[d].find('(').expect("a def has a parameter list");
    let Group {
        inner_uncommented: params,
        close_line: end,
        ..
    } = group(Kind::Python, lines, d, open)?;
    python_params(lines, d, &params, name, out);
    Some(end + 1)
}

fn python_super(lines: &[&str], scope: &Scope) -> Vec<Binding> {
    let mut out = Vec::new();
    if let Scope::Def(d) = *scope
        && let Some(line) = python_class_of(lines, d)
    {
        let value = Value::Class { decl_line1: line };
        out.push(Binding { line1: line, value });
    }
    out
}

static SCOPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^(?:(?:async\s+)?def|class)\s+(\w+)").expect("the regex is a literal")
});
static TUPLE_TARGET: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^(\(?[\w\s,.*\[\]]+\)?)\s*=[^=]").expect("the regex is a literal")
});

struct Rules {
    annotated: Regex,
    assigned: Regex,
    unknown: Regex,
    declared_outside: Regex,
    plain_loop_over_a_name: Regex,
    chained_assignment: Regex,
}

impl Rules {
    fn of(name: &str) -> Self {
        let n = regex::escape(name);
        let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
        Self {
            annotated: rule(format!(r"^{n}\s*:\s*([^=]+?)\s*(?:=.*)?$")),
            assigned: rule(format!(r"^{n}\s*=\s*([^=].*)$")),
            unknown: rule(format!(
                r"^(?:async\s+)?for\s+[^=]*\b{n}\b.*\sin\s|\bas\s+{n}\b|\b{n}\s*:=|^from\s+\S+\s+import\s.*\b{n}\b\s*(?:[,);\\]|$)|^import\s(?:.*[\s,])?{n}\b"
            )),
            declared_outside: rule(format!(r"^(global|nonlocal)\s.*\b{n}\b")),
            plain_loop_over_a_name: rule(format!(r"^for\s+{n}\s+in\s+([A-Za-z_]\w*)\s*:")),
            chained_assignment: rule(format!(r"^(?:[\w.\[\]]+\s*=\s*)+{n}\s*=[^=]")),
        }
    }
}

pub(super) fn python_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let r = Rules::of(name);
    let scopes_innermost_first = python_scopes_innermost_first(lines, at);

    let literal = literal_lines(Kind::Python, &lines.join("\n"));
    if name == "super" {
        return python_super(lines, &scopes_innermost_first[0]);
    }
    let in_class = python_class_body_above(lines, at, name);
    if !in_class.is_empty() {
        return in_class;
    }
    let mut out = Vec::new();
    let mut module_only = false;
    for scope in scopes_innermost_first {
        if module_only && matches!(scope, Scope::Def(_)) {
            continue;
        }
        let mut outside = None;
        let (start, base) = match scope {
            Scope::Def(d) => {
                let Some(start) = python_def_body(lines, d, name, &mut out) else {
                    continue;
                };
                (start, Some(indent(lines[d])))
            }
            Scope::Module => (0, None),
        };
        let mut nested_def_or_class_indent: Option<usize> = None;
        for (i, l) in lines.iter().enumerate().skip(start) {
            let code = uncommented(Kind::Python, l);
            let t = code.trim();
            if t.is_empty() || literal[i] {
                continue;
            }
            let ind = indent(l);
            if base.is_some_and(|b| ind <= b) {
                break;
            }
            if let Some(k) = nested_def_or_class_indent {
                if ind > k || closes_a_wrapped_header(t) {
                    continue;
                }
                nested_def_or_class_indent = None;
            }
            if let Some(c) = SCOPE.captures(t) {
                if &c[1] == name {
                    out.push(Binding {
                        line1: i + 1,
                        value: Value::Unknown,
                    });
                }
                nested_def_or_class_indent = Some(ind);
                continue;
            }
            if let Some(c) = r.declared_outside.captures(t) {
                outside = Some(&c[1] == "global");
                continue;
            }
            let value = if let Some(c) = r.plain_loop_over_a_name.captures(t) {
                Some(Value::Element(c[1].to_owned()))
            } else if r.unknown.is_match(t)
                || (i == at && python_inline(t, name))
                || python_case_binds(t, name)
            {
                Some(Value::Unknown)
            } else {
                for s in python_statements(t, continued(Kind::Python, lines, i)) {
                    let value = if r.unknown.is_match(s) || r.chained_assignment.is_match(s) {
                        Some(Value::Unknown)
                    } else if let Some(c) = r.annotated.captures(s) {
                        Some(Value::Type(c[1].to_owned()))
                    } else if let Some(c) = r.assigned.captures(s) {
                        Some(value_of(Kind::Python, &c[1]))
                    } else {
                        TUPLE_TARGET
                            .captures(s)
                            .filter(|c| c[1].contains(',') && names(&c[1], name))
                            .map(|_| Value::Unknown)
                    };
                    if let Some(value) = value {
                        out.push(Binding {
                            line1: i + 1,
                            value,
                        });
                    }
                }
                None
            };
            if let Some(value) = value {
                out.push(Binding {
                    line1: i + 1,
                    value,
                });
            }
        }
        if let Some(global) = outside {
            out.clear();
            module_only |= global;
            continue;
        }
        if !out.is_empty() {
            break;
        }
    }
    out
}
fn python_inline(t: &str, name: &str) -> bool {
    static INLINE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bfor\s+([^=]*?)\s+in\b|\blambda\b([^:]*)").expect("the regex is a literal")
    });
    INLINE.captures_iter(t).any(|c| match (c.get(1), c.get(2)) {
        (Some(targets), _) => names(targets.as_str(), name),
        (_, Some(params)) => params.as_str().split(',').any(|p| {
            p.split_once('=')
                .map_or(p, |(param, _)| param)
                .trim()
                .trim_start_matches('*')
                == name
        }),
        _ => false,
    })
}
pub fn python_inline_binds(line: &str, name: &str) -> bool {
    let t = uncommented(Kind::Python, line);
    let t = t.trim_start();
    !t.starts_with("for ") && !t.starts_with("async for ") && python_inline(t, name)
}
fn python_case_binds(t: &str, name: &str) -> bool {
    let Some(pattern) = t.strip_prefix("case ") else {
        return false;
    };
    let pattern = pattern.split(" if ").next().unwrap_or(pattern);
    let in_code: Vec<usize> = code(Kind::Python, pattern).map(|(i, _)| i).collect();
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    pattern.match_indices(name).any(|(i, _)| {
        let end = i + name.len();
        in_code.contains(&i)
            && !pattern
                .get(..i)
                .and_then(|s| s.bytes().next_back())
                .is_some_and(|c| word(c) || c == b'.')
            && !pattern.as_bytes().get(end).copied().is_some_and(word)
            && !pattern
                .get(end..)
                .is_some_and(|s| s.trim_start().starts_with(['(', '.', '=']))
    })
}
pub(super) fn python_statements(t: &str, continues: bool) -> Vec<&str> {
    static HEADER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"^(?:(?:if|elif|else|try|except|finally|while|with|for|async\s+with|async\s+for)\b|case\s)",
        )
        .expect("the regex is a literal")
    });
    let (mut depth, header, mut end_of_unopened_closer) = (0i32, HEADER.is_match(t), None);
    let colon = code(Kind::Python, t).find(|&(i, c)| {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' if depth == 0 => end_of_unopened_closer = Some(i + 1),
            b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
        let right_behind_unopened_closer = end_of_unopened_closer
            .is_some_and(|k| t.get(k..i).is_some_and(|s| s.trim().is_empty()));
        let ends = header || right_behind_unopened_closer;
        c == b':' && ends && depth == 0 && t.get(i + 1..).is_some_and(|r| !r.starts_with('='))
    });
    let rest = match colon {
        Some((i, _)) => match t.get(i + 1..) {
            Some(rest) => rest,
            None => return Vec::new(),
        },
        None if continues => return Vec::new(),
        None => t,
    };
    split_top(Kind::Python, rest, b';')
        .into_iter()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}
pub fn python_module_level(text: &str) -> String {
    let literal = literal_lines(Kind::Python, text);
    let mut def_or_class_indent: Option<usize> = None;
    let mut out = String::new();
    for (i, l) in text.lines().enumerate() {
        let t = l.trim_start();
        if literal[i] {
            continue;
        }
        if let Some(k) = def_or_class_indent {
            if t.is_empty() || indent(l) > k || closes_a_wrapped_header(t) || t.starts_with('#') {
                continue;
            }
            def_or_class_indent = None;
        }
        if ["def ", "async def ", "class "]
            .iter()
            .any(|k| t.starts_with(k))
        {
            def_or_class_indent = Some(indent(l));
            continue;
        }
        out.push_str(l);
        out.push('\n');
    }
    out
}
fn python_params(
    lines: &[&str],
    def_line0: usize,
    params: &str,
    name: &str,
    out: &mut Vec<Binding>,
) {
    for (i, p) in split_top(Kind::Python, params, b',')
        .into_iter()
        .enumerate()
    {
        let at = p.as_ptr() as usize - params.as_ptr() as usize + p.len() - p.trim_start().len();
        let Some(above) = params.get(..at) else {
            continue;
        };
        let line_written_on = def_line0 + 1 + above.matches('\n').count();
        let head = split_top(Kind::Python, p, b'=')[0]
            .trim()
            .trim_start_matches('*');
        let (pname, annotation) = match head.split_once(':') {
            Some((pname, t)) => (pname.trim(), Some(t.trim())),
            None => (head, None),
        };
        if pname != name {
            continue;
        }
        let value = match annotation {
            Some(t) => Value::Type(t.to_owned()),
            None if i == 0 => python_class_of(lines, def_line0)
                .map_or(Value::Unknown, |decl_line1| Value::Class { decl_line1 }),
            None => Value::Unknown,
        };
        out.push(Binding {
            line1: line_written_on,
            value,
        });
    }
}
fn python_class_header(lines: &[&str], at: usize) -> Option<usize> {
    let mut depth = indent(lines[at]);
    for i in (0..at).rev() {
        let t = lines[i].trim_start();
        if depth == 0 {
            break;
        }
        if t.is_empty()
            || t.starts_with('#')
            || closes_a_wrapped_header(t)
            || indent(lines[i]) >= depth
        {
            continue;
        }
        depth = indent(lines[i]);
        if t.starts_with("def ") || t.starts_with("async def ") {
            return None;
        }
        if t.starts_with("class ") {
            return Some(i);
        }
    }
    None
}
fn python_class_body_above(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    static INNER_SCOPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:for|lambda)\b").expect("the regex is a literal")
    });
    let Some(header) = python_class_header(lines, at) else {
        return Vec::new();
    };
    let t = uncommented(Kind::Python, lines[at]);
    if INNER_SCOPE.is_match(&t) || continued(Kind::Python, lines, at) {
        return Vec::new();
    }
    python_bindings(&lines[header + 1..=at], at - header - 1, name)
        .into_iter()
        .filter(|b| b.line1 + header < at)
        .map(|b| Binding {
            line1: b.line1 + header + 1,
            value: b.value,
        })
        .collect()
}
pub fn python_class_binds(text: &str, line1: usize, name: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let Some(i) = python_class_header(&lines, at) else {
        return false;
    };
    let depth = indent(lines[i]);
    let body_end = (i + 1..lines.len())
        .find(|&j| {
            let t = lines[j].trim_start();
            !t.is_empty()
                && !t.starts_with('#')
                && !closes_a_wrapped_header(t)
                && indent(lines[j]) <= depth
        })
        .unwrap_or(lines.len());
    !python_bindings(&lines[i + 1..body_end], at - i - 1, name).is_empty()
}
pub fn python_method(text: &str, line1: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let literal = literal_lines(Kind::Python, text);
    let depth = indent(lines[at]);
    (0..at)
        .rev()
        .find(|&i| {
            let t = lines[i].trim_start();
            !t.is_empty()
                && !t.starts_with(['#', ')', ']'])
                && !literal[i]
                && indent(lines[i]) < depth
        })
        .is_some_and(|i| lines[i].trim_start().starts_with("class "))
}
pub fn python_in_function(text: &str, line1: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let literal = literal_lines(Kind::Python, text);
    let mut depth = indent(lines[at]);
    for i in (0..at).rev() {
        let t = lines[i].trim_start();
        if depth == 0 {
            break;
        }
        if t.is_empty() || t.starts_with(['#', ')', ']']) || literal[i] || indent(lines[i]) >= depth
        {
            continue;
        }
        depth = indent(lines[i]);
        if t.starts_with("def ") || t.starts_with("async def ") {
            return true;
        }
        if t.starts_with("class ") {
            return false;
        }
    }
    false
}
pub fn python_overload(text: &str, line1: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(d) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    lines[..d]
        .iter()
        .rev()
        .map(|l| l.trim())
        .filter(|t| !t.is_empty() && !t.starts_with('#'))
        .take_while(|t| t.starts_with('@'))
        .any(|t| t == "@overload" || t == "@typing.overload")
}
fn python_class_of(lines: &[&str], def_line0: usize) -> Option<usize> {
    let ind = indent(lines[def_line0]);
    let mut decorators = true;
    for i in (0..def_line0).rev() {
        let t = lines[i].trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if indent(lines[i]) < ind && !closes_a_wrapped_header(t) {
            return t.starts_with("class ").then_some(i + 1);
        }
        if decorators && indent(lines[i]) == ind && t.starts_with('@') {
            if t == "@staticmethod" {
                return None;
            }
            continue;
        }
        decorators = false;
    }
    None
}
pub fn names_a_parameter(text: &str, line1: usize, range: &std::ops::Range<usize>) -> bool {
    keyword_argument(text, line1, range) || def_parameter(text, line1, range)
}
fn def_parameter(text: &str, line1: usize, range: &std::ops::Range<usize>) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let l = lines[at];
    let (Some(name), Some(before), Some(after)) = (
        l.get(range.clone()),
        l.get(..range.start),
        l.get(range.end..),
    ) else {
        return false;
    };
    let (before, after) = (before.trim_end(), after.trim_start());
    if !(before.is_empty() || before.ends_with(['(', ',', '*']))
        || !(after.is_empty() || after.starts_with([',', ')', ':', '=']))
        || after.starts_with("==")
    {
        return false;
    }
    let Some(d) = (0..=at)
        .rev()
        .find(|&i| DEF.is_match(lines[i].trim_start()))
    else {
        return false;
    };
    let Some(Group {
        inner_uncommented: params,
        close_line,
        ..
    }) = lines[d]
        .find('(')
        .and_then(|open| group(Kind::Python, &lines, d, open))
    else {
        return false;
    };
    let mut out = Vec::new();
    python_params(&lines, d, &params, name, &mut out);
    close_line >= at && out.iter().any(|b| b.line1 == line1)
}
pub fn keyword_argument(text: &str, line1: usize, range: &std::ops::Range<usize>) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(l) = line1.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let (Some(before), Some(after)) = (l.get(..range.start), l.get(range.end..)) else {
        return false;
    };
    let (before, after) = (before.trim_end(), after.trim_start());
    after.starts_with('=')
        && !after.starts_with("==")
        && (before.ends_with(['(', ','])
            || (before.is_empty() && continued(Kind::Python, &lines, line1 - 1)))
}
