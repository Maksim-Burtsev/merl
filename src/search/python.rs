use regex::Regex;

use super::*;

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
#[derive(Clone, Copy)]
enum Scope {
    Def(usize),
    Module,
}

fn closes_a_wrapped_header(t: &str) -> bool {
    t.starts_with([')', ']'])
}

static DEF: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"^(?:async\s+)?def\s+(\w+)\s*\(").unwrap());

pub(super) fn python_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    static SCOPE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(?:(?:async\s+)?def|class)\s+(\w+)").unwrap());
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let annotated = rule(format!(r"^{n}\s*:\s*([^=]+?)\s*(?:=.*)?$"));
    let assigned = rule(format!(r"^{n}\s*=\s*([^=].*)$"));
    let unknown = rule(format!(
        r"^(?:async\s+)?for\s+[^=]*\b{n}\b.*\sin\s|\bas\s+{n}\b|\b{n}\s*:=|^(?:global|nonlocal)\s.*\b{n}\b|^from\s+\S+\s+import\s.*\b{n}\b|^import\s(?:.*[\s,])?{n}\b"
    ));
    let inline = rule(format!(
        r"\bfor\s+[^=]*?\b{n}\b[^=]*?\s+in\b|\blambda\b[^:]*\b{n}\b"
    ));
    let plain_loop_over_a_name = rule(format!(r"^for\s+{n}\s+in\s+([A-Za-z_]\w*)\s*:"));
    let chained_assignment = rule(format!(r"^(?:[\w.\[\]]+\s*=\s*)+{n}\s*=[^=]"));
    let tuple_target = Regex::new(r"^(\(?[\w\s,.*\[\]]+\)?)\s*=[^=]").unwrap();

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

    let literal = literal_lines(Kind::Python, &lines.join("\n"));
    let mut out = Vec::new();
    if name == "super" {
        if let Scope::Def(d) = scopes_innermost_first[0]
            && let Some(line) = python_class_of(lines, d)
        {
            let value = Value::Class { decl_line1: line };
            out.push(Binding { line1: line, value });
        }
        return out;
    }
    for scope in scopes_innermost_first {
        let (start, base) = match scope {
            Scope::Def(d) => {
                let open = lines[d].find('(').expect("a def has a parameter list");
                let Some(Group {
                    inner_uncommented: params,
                    close_line: end,
                    ..
                }) = group(Kind::Python, lines, d, open)
                else {
                    continue;
                };
                python_params(lines, d, &params, name, &mut out);
                (end + 1, Some(indent(lines[d])))
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
            let value = if let Some(c) = plain_loop_over_a_name.captures(t) {
                Some(Value::Element(c[1].to_owned()))
            } else if unknown.is_match(t) || (i == at && inline.is_match(t)) {
                Some(Value::Unknown)
            } else {
                for s in python_statements(t, continued(Kind::Python, lines, i)) {
                    let value = if unknown.is_match(s) || chained_assignment.is_match(s) {
                        Some(Value::Unknown)
                    } else if let Some(c) = annotated.captures(s) {
                        Some(Value::Type(c[1].to_owned()))
                    } else if let Some(c) = assigned.captures(s) {
                        Some(value_of(Kind::Python, &c[1]))
                    } else {
                        tuple_target
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
        if !out.is_empty() {
            break;
        }
    }
    out
}
pub(super) fn python_statements(t: &str, continues: bool) -> Vec<&str> {
    static HEADER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"^(?:(?:if|elif|else|try|except|finally|while|with|for|async\s+with|async\s+for)\b|case\s)",
        )
        .unwrap()
    });
    let (mut depth, header, mut end_of_unopened_closer) = (0i32, HEADER.is_match(t), None);
    let colon = code(Kind::Python, t).find(|&(i, c)| {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' if depth == 0 => end_of_unopened_closer = Some(i + 1),
            b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
        let right_behind_unopened_closer =
            end_of_unopened_closer.is_some_and(|k| k <= i && t[k..i].trim().is_empty());
        let ends = header || right_behind_unopened_closer;
        c == b':' && ends && depth == 0 && !t[i + 1..].starts_with('=')
    });
    let rest = match colon {
        Some((i, _)) => &t[i + 1..],
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
        let line_written_on = def_line0 + 1 + params[..at].matches('\n').count();
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
pub fn python_class_binds(text: &str, line1: usize, name: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
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
            return false;
        }
        if t.starts_with("class ") {
            let body_end = (i + 1..lines.len())
                .find(|&j| {
                    let t = lines[j].trim_start();
                    !t.is_empty()
                        && !t.starts_with('#')
                        && !closes_a_wrapped_header(t)
                        && indent(lines[j]) <= depth
                })
                .unwrap_or(lines.len());
            return !python_bindings(&lines[i + 1..body_end], at - i - 1, name).is_empty();
        }
    }
    false
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
pub fn def_parameter(text: &str, line1: usize, range: &std::ops::Range<usize>) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let (l, name) = (lines[at], &lines[at][range.clone()]);
    let before = l[..range.start].trim_end();
    let after = l[range.end..].trim_start();
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
    let after = l[range.end..].trim_start();
    let before = l[..range.start].trim_end();
    after.starts_with('=')
        && !after.starts_with("==")
        && (before.ends_with(['(', ','])
            || (before.is_empty() && continued(Kind::Python, &lines, line1 - 1)))
}
