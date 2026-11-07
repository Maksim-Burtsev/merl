use regex::Regex;

use super::{
    Binding, Group, Kind, Value, close_of, code, continued, group, indent, literal_lines, names,
    split_top, uncommented, value_of,
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
pub fn python_comprehension_over(line: &str, name: &str) -> Option<String> {
    let t = uncommented(Kind::Python, line);
    Regex::new(&format!(
        r"\bfor\s+{}\s+in\s+([A-Za-z_]\w*)\s*(?:[\])}}]|\bif\b)",
        regex::escape(name)
    ))
    .ok()?
    .captures(&t)
    .map(|c| c[1].to_owned())
}
pub fn python_called(expr: &str) -> Option<String> {
    match value_of(Kind::Python, expr) {
        Value::Call(callee) => Some(callee),
        _ => None,
    }
}
pub fn python_sum_operands(line: &str, name: &str) -> Option<Vec<String>> {
    let t = uncommented(Kind::Python, line);
    let rest = t
        .trim()
        .strip_prefix(name)?
        .trim_start()
        .strip_prefix('=')?;
    if rest.starts_with('=') {
        return None;
    }
    let operands = split_top(Kind::Python, rest, b'+');
    (operands.len() > 1).then(|| operands.iter().map(|o| o.trim().to_owned()).collect())
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
    Regex::new(r"^([\w\s,.*\[\]()]+)\s*=[^=]").expect("the regex is a literal")
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
                r"^(?:async\s+)?for\s+[^=]*\b{n}\b.*\sin\s|\bas\s+(?:[(\[][\w\s,*]*)?\b{n}\b|\b{n}\s*:=|^from\s+\S+\s+import\s.*\b{n}\b\s*(?:[,);\\]|$)|^import\s(?:.*[\s,])?{n}\b"
            )),
            declared_outside: rule(format!(r"^(global|nonlocal)\s.*\b{n}\b")),
            plain_loop_over_a_name: rule(format!(
                r"^for\s+(?:\w+\s*,\s*{n}\s+in\s+([A-Za-z_]\w*)\.items\(\)|{n}\s+in\s+([A-Za-z_]\w*)(\.values\(\))?)\s*:"
            )),
            chained_assignment: rule(format!(r"^(?:[\w.\[\]]+\s*=\s*)+{n}\s*=[^=]")),
        }
    }
}

fn looped(c: &regex::Captures) -> String {
    match (c.get(1), c.get(2)) {
        (Some(items), _) => format!("{}.values()", items.as_str()),
        (None, Some(over)) => format!("{}{}", over.as_str(), c.get(3).map_or("", |v| v.as_str())),
        (None, None) => String::new(),
    }
}
pub(super) fn python_bindings_at(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let in_class = python_class_body_above(lines, at, name);
    if in_class.is_empty() {
        python_bindings(lines, at, name)
    } else {
        in_class
    }
}
fn python_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let r = Rules::of(name);
    let scopes_innermost_first = python_scopes_innermost_first(lines, at);

    let literal = literal_lines(Kind::Python, &lines.join("\n"));
    if name == "super" {
        return python_super(lines, &scopes_innermost_first[0]);
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
                if &c[1] == name || (i == at && python_header_binds(lines, at, t, &r, name)) {
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
            }
            let value = if let Some(c) = r.plain_loop_over_a_name.captures(t) {
                Some(Value::Element(looped(&c)))
            } else if r.unknown.is_match(t)
                || (i <= at
                    && python_inline(t, name)
                    && (i + 1..=at).all(|k| continued(Kind::Python, lines, k)))
                || python_case_binds(t, name)
            {
                Some(Value::Unknown)
            } else {
                out.extend(
                    python_statements(t, continued(Kind::Python, lines, i))
                        .into_iter()
                        .filter_map(|s| statement_binds(&r, s, name))
                        .map(|value| Binding {
                            line1: i + 1,
                            value,
                        }),
                );
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
            out.retain(|b| b.line1 <= at + 1);
            module_only |= global;
            continue;
        }
        if !out.is_empty() {
            break;
        }
    }
    out
}
fn python_header_binds(lines: &[&str], at: usize, t: &str, r: &Rules, name: &str) -> bool {
    let mut params = Vec::new();
    let body = if DEF.is_match(t) {
        let open = lines[at].find('(').expect("a def has a parameter list");
        let Some(g) = group(Kind::Python, lines, at, open) else {
            return false;
        };
        if g.close_line != at {
            return false;
        }
        python_params(lines, at, &g.inner_uncommented, name, &mut params);
        g.after_close
    } else {
        t
    };
    !params.is_empty()
        || code(Kind::Python, body)
            .find(|&(_, c)| c == b':')
            .and_then(|(k, _)| body.get(k + 1..))
            .is_some_and(|rest| {
                split_top(Kind::Python, rest, b';')
                    .into_iter()
                    .map(str::trim)
                    .any(|s| statement_binds(r, s, name).is_some() || python_inline(s, name))
            })
}
fn statement_binds(r: &Rules, s: &str, name: &str) -> Option<Value> {
    if r.unknown.is_match(s) || r.chained_assignment.is_match(s) {
        Some(Value::Unknown)
    } else if let Some(c) = r.annotated.captures(s) {
        Some(Value::Type(c[1].to_owned()))
    } else if let Some(c) = r.assigned.captures(s) {
        Some(value_of(Kind::Python, &c[1]))
    } else {
        TUPLE_TARGET
            .captures(s)
            .filter(|c| c[1].matches('(').count() == c[1].matches(')').count())
            .map(|c| top_level_targets(&c[1]))
            .filter(|t| t.contains(',') && names(t, name))
            .map(|_| Value::Unknown)
    }
}
fn top_level_targets(target: &str) -> String {
    let mut opened = Vec::new();
    let mut prev = ' ';
    target
        .chars()
        .map(|c| {
            if matches!(c, '[' | '(') {
                opened.push(c == '[' || prev.is_alphanumeric() || matches!(prev, '_' | ')' | ']'));
            }
            let shown = !opened.iter().any(|&hidden| hidden);
            if matches!(c, ']' | ')') {
                opened.pop();
            }
            if !c.is_whitespace() {
                prev = c;
            }
            if shown { c } else { ' ' }
        })
        .collect()
}
static FOR_IN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"\bfor\s+([^=]*?)\s+in\b").expect("the regex is a literal")
});
static LAMBDA: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"\blambda\b([^:]*)").expect("the regex is a literal"));
fn lambda_names(params: &str, name: &str) -> bool {
    params.split(',').any(|p| {
        p.split_once('=')
            .map_or(p, |(param, _)| param)
            .trim()
            .trim_start_matches('*')
            == name
    })
}
fn python_inline(t: &str, name: &str) -> bool {
    FOR_IN.captures_iter(t).any(|c| names(&c[1], name))
        || LAMBDA.captures_iter(t).any(|c| lambda_names(&c[1], name))
}
pub fn python_binds_around(line: &str, name: &str, col: usize) -> bool {
    static CLAUSE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\b(?:for|if)\b").expect("the regex is a literal"));
    let (mut shown, mut open, mut groups) = (vec![b' '; line.len()], Vec::new(), Vec::new());
    for (i, c) in code(Kind::Python, line) {
        if c == 0 {
            break;
        }
        shown[i] = if c.is_ascii() { c } else { b'_' };
        match c {
            b'(' | b'[' | b'{' => open.push(i + 1),
            b')' | b']' | b'}' => groups.push((open.pop().unwrap_or(0), i)),
            _ => {}
        }
    }
    groups.extend(open.into_iter().map(|s| (s, line.len())));
    groups.push((0, line.len()));
    let t = line.trim_start();
    let statement_for = t.starts_with("for ") || t.starts_with("async for ");
    groups.iter().enumerate().any(|(g, &(s, e))| {
        if !(s..e).contains(&col) {
            return false;
        }
        let top: String = (s..e)
            .map(|j| {
                let nested = groups
                    .iter()
                    .enumerate()
                    .any(|(h, &(hs, he))| h != g && s <= hs && he <= e && (hs..he).contains(&j));
                if nested || b"()[]{}".contains(&shown[j]) {
                    ' '
                } else {
                    char::from(shown[j])
                }
            })
            .collect();
        let at = col - s;
        let iterable = FOR_IN.find(&top).map(|m| {
            m.end()
                ..CLAUSE
                    .find_at(&top, m.end())
                    .map_or(top.len(), |k| k.start())
        });
        let comprehension = (g + 1 != groups.len() || !statement_for)
            && !iterable.is_some_and(|r| r.contains(&at))
            && FOR_IN.captures_iter(&top).any(|c| names(&c[1], name));
        let lambda = LAMBDA.captures_iter(&top).any(|c| {
            let params = c.get(1).map_or(0..0, |p| p.range());
            let rest = top.get(params.end..).unwrap_or("");
            let end = rest.find([',', ';']).map_or(top.len(), |k| params.end + k);
            rest.starts_with(':')
                && (params.end + 1..end).contains(&at)
                && lambda_names(&c[1], name)
        });
        comprehension || lambda
    })
}
fn python_def_line_body(line: &str, name: &str) -> Option<(std::ops::Range<usize>, bool)> {
    let lines = [line];
    let t = uncommented(Kind::Python, line);
    let open = line.find('(').filter(|_| DEF.is_match(t.trim_start()))?;
    let g = group(Kind::Python, &lines, 0, open).filter(|g| g.close_line == 0)?;
    let mut params = Vec::new();
    python_params(&lines, 0, &g.inner_uncommented, name, &mut params);
    let (k, _) = code(Kind::Python, g.after_close).find(|&(_, c)| c == b':')?;
    let rest = g.after_close.get(k + 1..)?;
    let r = Rules::of(name);
    let binds = !params.is_empty()
        || split_top(Kind::Python, rest, b';')
            .into_iter()
            .any(|s| statement_binds(&r, s.trim(), name).is_some());
    Some((line.len() - rest.len()..t.len(), binds))
}
pub fn python_def_line_binds(line: &str, name: &str, col: usize) -> bool {
    python_def_line_body(line, name).is_some_and(|(body, binds)| binds && body.contains(&col))
}
pub fn python_binds_only_inline(line: &str, name: &str) -> bool {
    let t = uncommented(Kind::Python, line);
    let t = t.trim();
    let r = Rules::of(name);
    python_inline(t, name)
        && match SCOPE.captures(t) {
            Some(c) => &c[1] != name && python_def_line_body(line, name).is_some_and(|(_, b)| !b),
            None => {
                !r.unknown.is_match(t)
                    && !python_case_binds(t, name)
                    && python_statements(t, false)
                        .into_iter()
                        .all(|s| statement_binds(&r, s, name).is_none())
            }
        }
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
pub fn python_class_defines(text: &str, header1: usize, name: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(i) = header1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let n = regex::escape(name);
    let Ok(defines) = Regex::new(&format!(
        r"^\s*(?:(?:async\s+)?def\s+{n}\b|class\s+{n}\b|{n}\s*(?::[^=]*)?=[^=])"
    )) else {
        return false;
    };
    let depth = indent(lines[i]);
    lines[i + 1..]
        .iter()
        .take_while(|l| {
            let t = l.trim_start();
            t.is_empty() || t.starts_with('#') || closes_a_wrapped_header(t) || indent(l) > depth
        })
        .any(|l| defines.is_match(l))
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
    python_enclosing_def(text, line1).is_some()
}
pub fn python_in_function_body(text: &str, line1: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    python_enclosing_def(text, line1).is_some_and(|d| {
        lines[d].find('(').is_some_and(|open| {
            group(Kind::Python, &lines, d, open).is_some_and(|g| g.close_line + 1 < line1)
        })
    })
}
pub fn python_def_hides_import(text: &str, line1: usize, name: &str) -> bool {
    python_def_binds_last(text, line1, name) == Some(true)
}
pub fn python_import_binds_last(text: &str, line1: usize, name: &str) -> bool {
    python_def_binds_last(text, line1, name) == Some(false)
}
fn python_def_binds_last(text: &str, line1: usize, name: &str) -> Option<bool> {
    let Ok(def) = Regex::new(&format!(
        r"^(?:async\s+)?(?:def|class)\s+{}\b",
        regex::escape(name)
    )) else {
        return None;
    };
    let literal = literal_lines(Kind::Python, text);
    let lines: Vec<&str> = text.lines().collect();
    let imports_it = |i: usize| {
        let t = lines[i].trim_start();
        (t.starts_with("from ") || t.starts_with("import ")) && literal.get(i) != Some(&true) && {
            let end = if t.contains('(') {
                (i..lines.len().min(i + 64))
                    .find(|&j| lines[j].contains(')'))
                    .unwrap_or(i)
            } else {
                i
            };
            super::imports(Kind::Python, &lines[i..=end].join("\n"))
                .iter()
                .any(|m| m.name == name)
        }
    };
    let owner = |i: usize| python_enclosing_def(text, i + 1);
    let body = python_in_function_body(text, line1);
    if body {
        let chain: Vec<usize> = std::iter::successors(python_enclosing_def(text, line1), |&d| {
            python_enclosing_def(text, d + 1)
        })
        .collect();
        if (0..lines.len()).any(|i| imports_it(i) && owner(i).is_some_and(|d| chain.contains(&d))) {
            return Some(false);
        }
    }
    let until = if body {
        lines.len()
    } else {
        line1.saturating_sub(1).min(lines.len())
    };
    (0..until)
        .rev()
        .filter(|&i| literal.get(i) != Some(&true))
        .find_map(|i| {
            if def.is_match(lines[i]) {
                Some(true)
            } else if imports_it(i) && owner(i).is_none() {
                Some(false)
            } else {
                None
            }
        })
}
fn python_enclosing_def(text: &str, line1: usize) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
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
            return Some(i);
        }
        if t.starts_with("class ") {
            return None;
        }
    }
    None
}
pub fn python_rebinds_in_loop(text: &str, line1: usize, name: &str, col: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let line = uncommented(Kind::Python, lines[at]);
    let start = indent(lines[at]);
    let Some(eq) = line
        .get(start..)
        .and_then(|t| t.strip_prefix(name))
        .map(|rest| (rest, rest.trim_start()))
        .filter(|(_, r)| r.starts_with('=') && !r.starts_with("=="))
        .map(|(rest, r)| start + name.len() + rest.len() - r.len())
    else {
        return false;
    };
    if col <= eq {
        return false;
    }
    let literal = literal_lines(Kind::Python, text);
    let mut depth = start;
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
        if t.starts_with("while ") {
            return true;
        }
        if let Some(head) = t
            .strip_prefix("for ")
            .or_else(|| t.strip_prefix("async for "))
        {
            let targets = head.split(" in ").next().unwrap_or(head);
            return !targets
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .any(|w| w == name);
        }
        if ["def ", "async def ", "class "]
            .iter()
            .any(|k| t.starts_with(k))
        {
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
pub fn python_class_alias<'a>(decl: &'a str, name: &str) -> Option<&'a str> {
    let rest = decl.strip_prefix(name)?.trim_start().strip_prefix('=')?;
    let target = rest.split('#').next()?.trim();
    (!rest.starts_with('=')
        && target.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && target
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.'))
    .then_some(target)
}
pub fn python_module_assignments(text: &str, name: &str) -> usize {
    text.lines()
        .filter(|l| {
            l.strip_prefix(name).is_some_and(|rest| {
                let rest = rest.trim_start();
                (rest.starts_with('=') && !rest.starts_with("=="))
                    || rest.starts_with(':')
                    || rest.starts_with(',')
            })
        })
        .count()
}
pub fn python_self_derived(line: &str, name: &str) -> bool {
    let t = uncommented(Kind::Python, line);
    let n = regex::escape(name);
    Regex::new(&format!(r"^{n}\s*=\s*{n}(?:\.\w+)+$")).is_ok_and(|r| r.is_match(t.trim()))
}
pub fn python_assigned_call(text: &str, line1: usize, name: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let first = uncommented(Kind::Python, lines.get(line1.checked_sub(1)?)?);
    let rest = first.trim().strip_prefix(name)?.trim_start();
    let rest = rest.strip_prefix('=').filter(|r| !r.starts_with('='))?;
    let mut expr = rest.trim().to_owned();
    let depth = |e: &str| {
        code(Kind::Python, e).fold(0i32, |d, (_, c)| match c {
            b'(' | b'[' | b'{' => d + 1,
            b')' | b']' | b'}' => d - 1,
            _ => d,
        })
    };
    for l in lines.iter().skip(line1).take(40) {
        if depth(&expr) <= 0 {
            break;
        }
        expr.push_str(uncommented(Kind::Python, l).trim());
    }
    if depth(&expr) != 0 {
        return None;
    }
    let inner = if expr.starts_with('(') && close_of(Kind::Python, &expr, 0) == Some(expr.len()) {
        expr.get(1..expr.len() - 1)?.trim()
    } else {
        expr.as_str()
    };
    inner.ends_with(')').then(|| inner.to_owned())
}
pub fn python_patched_mock(text: &str, line1: usize) -> Option<(String, bool)> {
    static HEAD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:(\w+)\s*=\s*|(?:async\s+)?with\s+)((?:\w+\.)*patch(?:\.object)?)\s*\(")
            .expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let at = line1.checked_sub(1)?;
    let c = HEAD.captures(lines.get(at)?)?;
    let callee = c[2].to_owned();
    let Group {
        inner_uncommented: args,
        after_close,
        ..
    } = group(Kind::Python, &lines, at, c.get(0)?.end() - 1)?;
    let after = uncommented(Kind::Python, after_close);
    let binds = match c.get(1) {
        Some(_) => after.trim().is_empty(),
        None => after
            .trim()
            .strip_prefix("as")
            .map(|r| {
                r.trim_start()
                    .trim_start_matches(|c: char| c.is_alphanumeric() || c == '_')
            })
            .is_some_and(|r| r.trim() == ":"),
    };
    (binds && patch_fill(&callee, &args) == Some(Fill::Mock))
        .then_some((callee, c.get(1).is_none()))
}
#[derive(PartialEq, Eq)]
enum Fill {
    Nothing,
    Mock,
    Other,
}
fn patch_fill(callee: &str, args: &str) -> Option<Fill> {
    static KEYWORD: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(\w+)\s*=[^=]").expect("the regex is a literal"));
    if matches!(callee.rsplit('.').next(), Some("dict" | "multiple")) {
        return Some(Fill::Nothing);
    }
    let args: Vec<&str> = split_top(Kind::Python, args, b',')
        .into_iter()
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .collect();
    if args.iter().any(|a| a.starts_with('*')) {
        return None;
    }
    let keywords: Vec<&str> = args
        .iter()
        .filter_map(|a| KEYWORD.captures(a).and_then(|k| k.get(1)))
        .map(|k| k.as_str())
        .collect();
    let positional = args.len() - keywords.len();
    let wanted = if callee.ends_with(".object") { 2 } else { 1 };
    if positional > wanted || keywords.contains(&"new") {
        Some(Fill::Nothing)
    } else if positional < wanted {
        None
    } else if keywords.contains(&"new_callable") || keywords.contains(&"autospec") {
        Some(Fill::Other)
    } else {
        Some(Fill::Mock)
    }
}
pub fn python_patched_param(text: &str, line1: usize, name: &str) -> Option<Vec<String>> {
    static DECORATOR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*@((?:\w+\.)*\w+)\s*(\()?").expect("the regex is a literal")
    });
    static PATCH: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|\.)patch(?:\.\w+)?$").expect("the regex is a literal")
    });
    static FILLS_NONE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:\w+\.)*override_settings$|^pytest\.mark\.\w+$")
            .expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    let d = (0..=at)
        .rev()
        .find(|&i| DEF.is_match(lines[i].trim_start()))?;
    let Group {
        inner_uncommented: params,
        close_line,
        ..
    } = group(Kind::Python, &lines, d, lines[d].find('(')?)?;
    let mut written = Vec::new();
    python_params(&lines, d, &params, name, &mut written);
    if close_line < at || !written.iter().any(|b| b.line1 == line1) {
        return None;
    }
    let names: Vec<&str> = split_top(Kind::Python, &params, b',')
        .into_iter()
        .map(|p| split_top(Kind::Python, p, b'=')[0].trim())
        .map(|p| p.split_once(':').map_or(p, |(n, _)| n.trim()))
        .collect();
    let k = names.iter().position(|&p| p == name)?;
    let skip = usize::from(python_class_of(&lines, d).is_some());
    if k < skip || names[..k].iter().any(|p| p.starts_with('*') || *p == "/") {
        return None;
    }
    let ind = indent(lines[d]);
    let (mut fills, mut patches) = (Vec::new(), Vec::new());
    for i in (0..d).rev() {
        let t = lines[i].trim();
        if t.is_empty()
            || t.starts_with('#')
            || indent(lines[i]) > ind
            || closes_a_wrapped_header(t)
        {
            continue;
        }
        if indent(lines[i]) < ind || !t.starts_with('@') {
            break;
        }
        let c = DECORATOR.captures(lines[i])?;
        if !PATCH.is_match(&c[1]) {
            if FILLS_NONE.is_match(&c[1]) {
                continue;
            }
            return None;
        }
        patches.push(c[1].to_owned());
        let open = c.get(2)?.start();
        let args = group(Kind::Python, &lines, i, open)?.inner_uncommented;
        match patch_fill(&c[1], &args)? {
            Fill::Nothing => {}
            fill => fills.push(fill),
        }
    }
    (fills.get(k - skip)? == &Fill::Mock).then_some(patches)
}
