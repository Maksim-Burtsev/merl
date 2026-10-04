//! Python's own rules for `d`: what a name is bound to in a module, a class or a function,
//! the statements of a module's top level, and the builtins, which have no source.

use regex::Regex;

use super::*;

/// Python's builtins (#336), `dir(builtins)` of 3.13 without the module's own dunders and
/// `super`, which `d` reads as the class above: compiled into the interpreter, with no source on
/// the machine to land on.
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
/// The Python builtin types whose members `d` knows have no source (#336), as a type is written:
/// `list[int]` is `list`, and `typing`'s `List` is not one of them.
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
pub(super) fn python_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    static DEF: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(?:async\s+)?def\s+(\w+)\s*\(").unwrap());
    static SCOPE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(?:(?:async\s+)?def|class)\s+(\w+)").unwrap());
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let annotated = rule(format!(r"^{n}\s*:\s*([^=]+?)\s*(?:=.*)?$"));
    let assigned = rule(format!(r"^{n}\s*=\s*([^=].*)$"));
    // An import binds the names it imports, not the modules on their path: `from .guild import
    // Guild` leaves a parameter `guild` alone.
    let unknown = rule(format!(
        r"^(?:async\s+)?for\s+[^=]*\b{n}\b.*\sin\s|\bas\s+{n}\b|\b{n}\s*:=|^(?:global|nonlocal)\s.*\b{n}\b|^from\s+\S+\s+import\s.*\b{n}\b|^import\s(?:.*[\s,])?{n}\b"
    ));
    let inline = rule(format!(
        r"\bfor\s+[^=]*?\b{n}\b[^=]*?\s+in\b|\blambda\b[^:]*\b{n}\b"
    ));
    // A plain loop over a plain name; `async for`, a tuple target and a call are unknown.
    let element = rule(format!(r"^for\s+{n}\s+in\s+([A-Za-z_]\w*)\s*:"));
    // `first = repo = …`: the value is behind the last `=`, which the rules do not look for.
    let chained = rule(format!(r"^(?:[\w.\[\]]+\s*=\s*)+{n}\s*=[^=]"));
    // `a, repo = …` or `(a, repo) = …`, not the keyword argument of a call.
    let tuple = Regex::new(r"^(\(?[\w\s,.*\[\]]+\)?)\s*=[^=]").unwrap();

    // The functions around the cursor, innermost first; `None` is the module.
    let mut scopes = Vec::new();
    let mut depth = indent(lines[at]);
    for i in (0..at).rev() {
        let t = lines[i].trim_start();
        if depth == 0 {
            break;
        }
        // A closer at a lower indent ends a multi-line signature; its `def` is further up.
        if t.is_empty() || t.starts_with(['#', ')', ']']) || indent(lines[i]) >= depth {
            continue;
        }
        depth = indent(lines[i]);
        if DEF.is_match(t) {
            scopes.push(Some(i));
        }
    }
    scopes.push(None);

    let literal = literal_lines(Kind::Python, &lines.join("\n"));
    let mut out = Vec::new();
    // `super()` is the class of the method it is written in, and nothing in a function inside
    // that method, where the call has no arguments to find.
    if name == "super" {
        if let Some(line) = scopes[0].and_then(|d| python_class_of(lines, d)) {
            let value = Value::Class(line);
            out.push(Binding { line, value });
        }
        return out;
    }
    for scope in scopes {
        let (start, base) = match scope {
            Some(d) => {
                let open = lines[d].find('(').expect("a def has a parameter list");
                let Some((params, end, _)) = group(Kind::Python, lines, d, open) else {
                    continue;
                };
                python_params(lines, d, &params, name, &mut out);
                (end + 1, Some(indent(lines[d])))
            }
            None => (0, None),
        };
        // A nested function or class is skipped down to the lines back at its indent.
        let mut skip: Option<usize> = None;
        for (i, l) in lines.iter().enumerate().skip(start) {
            let code = uncommented(Kind::Python, l);
            let t = code.trim();
            // A docstring's example binds nothing.
            if t.is_empty() || literal[i] {
                continue;
            }
            let ind = indent(l);
            if base.is_some_and(|b| ind <= b) {
                break;
            }
            if let Some(k) = skip {
                if ind > k || t.starts_with([')', ']']) {
                    continue;
                }
                skip = None;
            }
            if let Some(c) = SCOPE.captures(t) {
                if &c[1] == name {
                    out.push(Binding {
                        line: i + 1,
                        value: Value::Unknown,
                    });
                }
                skip = Some(ind);
                continue;
            }
            let value = if let Some(c) = element.captures(t) {
                Some(Value::Element(c[1].to_owned()))
            } else if unknown.is_match(t) || (i == at && inline.is_match(t)) {
                Some(Value::Unknown)
            } else {
                // A binding need not start its line (#131): `if x: ledger = A()`, `a = 1; b = 2`.
                for s in python_statements(t, continued(Kind::Python, lines, i)) {
                    let value = if unknown.is_match(s) || chained.is_match(s) {
                        Some(Value::Unknown)
                    } else if let Some(c) = annotated.captures(s) {
                        Some(Value::Type(c[1].to_owned()))
                    } else if let Some(c) = assigned.captures(s) {
                        Some(value_of(Kind::Python, &c[1]))
                    } else {
                        tuple
                            .captures(s)
                            .filter(|c| c[1].contains(',') && names(&c[1], name))
                            .map(|_| Value::Unknown)
                    };
                    if let Some(value) = value {
                        out.push(Binding { line: i + 1, value });
                    }
                }
                None
            };
            if let Some(value) = value {
                out.push(Binding { line: i + 1, value });
            }
        }
        // The innermost function that binds the name is the one the cursor reads.
        if !out.is_empty() {
            break;
        }
    }
    out
}
/// The simple statements the trimmed Python line `t` holds: what follows the `:` of a compound
/// header written on the same line (`if x: a = 1`, `else: a = 2`, `for … : a = 3`), cut at each
/// `;`. A line with neither is its one statement. A `:` inside brackets or a string, and the one
/// of `:=`, end no header. A line that `continues` the one above it holds a statement only behind
/// the end of a header wrapped over several lines, `    flag): a = 1`: a bracket closed that the
/// line did not open, then the `:`.
pub(super) fn python_statements(t: &str, continues: bool) -> Vec<&str> {
    static HEADER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"^(?:(?:if|elif|else|try|except|finally|while|with|for|async\s+with|async\s+for)\b|case\s)",
        )
        .unwrap()
    });
    // The `:` right behind a closer the line did not open ends a wrapped header; one further
    // on is a lambda's or a slice's behind the end of a call's arguments.
    let (mut depth, header, mut closed) = (0i32, HEADER.is_match(t), None);
    let colon = code(Kind::Python, t).find(|&(i, c)| {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' if depth == 0 => closed = Some(i + 1),
            b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
        let ends = header || closed.is_some_and(|k| k <= i && t[k..i].trim().is_empty());
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
/// `text` without the bodies of its functions and classes and without the lines inside its
/// docstrings and strings: the lines a Python module runs itself, where an import binds a name
/// of the module.
pub fn python_module_level(text: &str) -> String {
    let literal = literal_lines(Kind::Python, text);
    let mut skip: Option<usize> = None;
    let mut out = String::new();
    for (i, l) in text.lines().enumerate() {
        let t = l.trim_start();
        // A line of a docstring or of a string is no code, and ends no body however it is
        // indented. With none left, the result is read by [`imports_as_written`].
        if literal[i] {
            continue;
        }
        if let Some(k) = skip {
            // A closer at the header's indent ends a signature wrapped over several lines, and
            // a comment at the margin ends no body.
            if t.is_empty() || indent(l) > k || t.starts_with([')', ']', '#']) {
                continue;
            }
            skip = None;
        }
        if ["def ", "async def ", "class "]
            .iter()
            .any(|k| t.starts_with(k))
        {
            skip = Some(indent(l));
            continue;
        }
        out.push_str(l);
        out.push('\n');
    }
    out
}
/// The binding of `name` among the parameters of the `def` on line `d`, on the line the parameter
/// is written on: its annotation, the class for the first parameter of a method, else unknown.
fn python_params(lines: &[&str], d: usize, params: &str, name: &str, out: &mut Vec<Binding>) {
    for (i, p) in split_top(Kind::Python, params, b',')
        .into_iter()
        .enumerate()
    {
        // The line the name is written on: a wrapped signature puts one on each (#338).
        let at = p.as_ptr() as usize - params.as_ptr() as usize + p.len() - p.trim_start().len();
        let line = d + 1 + params[..at].matches('\n').count();
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
            None if i == 0 => python_class_of(lines, d).map_or(Value::Unknown, Value::Class),
            None => Value::Unknown,
        };
        out.push(Binding { line, value });
    }
}
/// Whether the body of the Python class that 1-based `line` sits in binds `name`: a `def`, a
/// class or an assignment of the body's own, which a name read in that body sees before the
/// module's and the builtins. `false` in a method, which does not see them, and outside a class.
pub fn python_class_binds(text: &str, line: usize, name: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let mut depth = indent(lines[at]);
    for i in (0..at).rev() {
        let t = lines[i].trim_start();
        if depth == 0 {
            break;
        }
        if t.is_empty() || t.starts_with(['#', ')', ']']) || indent(lines[i]) >= depth {
            continue;
        }
        depth = indent(lines[i]);
        if t.starts_with("def ") || t.starts_with("async def ") {
            return false;
        }
        if t.starts_with("class ") {
            // The body runs down to the first code line back at the class's indent.
            let end = (i + 1..lines.len())
                .find(|&j| {
                    let t = lines[j].trim_start();
                    !t.is_empty() && !t.starts_with(['#', ')', ']']) && indent(lines[j]) <= depth
                })
                .unwrap_or(lines.len());
            return !python_bindings(&lines[i + 1..end], at - i - 1, name).is_empty();
        }
    }
    false
}
/// Whether the Python `def` on 1-based `line` of `text` is a method: the nearest code line above
/// it indented less opens a class (#522). A bare name never calls one.
pub fn python_method(text: &str, line: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
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
/// Whether 1-based `line` of the Python `text` is in the body of a function: walking out through
/// the blocks around it, a `def` comes before any `class` (#338). A `def` there is a local of that
/// function, no member, while a method of a class nested in a function is one.
pub fn python_in_function(text: &str, line: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
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
/// Whether the Python `def` on 1-based `line` of `text` carries `@overload` or `@typing.overload`
/// among its decorators (#338).
pub fn python_overload(text: &str, line: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(d) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
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
/// The 1-based line of the class the `def` on line `d` is a method of, unless it is a
/// `@staticmethod`.
fn python_class_of(lines: &[&str], d: usize) -> Option<usize> {
    let ind = indent(lines[d]);
    let mut decorators = true;
    for i in (0..d).rev() {
        let t = lines[i].trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if indent(lines[i]) < ind && !t.starts_with([')', ']']) {
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
/// Whether the word at `range` of 1-based `line` names a keyword argument of a Python call:
/// `recipe_yield=…` behind a `(` or a `,`, or at the start of a line that continues a call. It
/// names a parameter of whatever is called, and no variable of that spelling.
pub fn keyword_argument(text: &str, line: usize, range: &std::ops::Range<usize>) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(l) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let after = l[range.end..].trim_start();
    let before = l[..range.start].trim_end();
    after.starts_with('=')
        && !after.starts_with("==")
        && (before.ends_with(['(', ','])
            || (before.is_empty() && continued(Kind::Python, &lines, line - 1)))
}
