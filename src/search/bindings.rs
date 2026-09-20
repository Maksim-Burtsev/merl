//! What a name is bound to where it is written: the miniature per-language reader
//! behind the receiver types `d` proves.

use std::ops::Range;

use regex::Regex;

use super::*;

// ---- the type of a receiver (#68, steps 2 and 3) --------------------------------------------
/// What a declaration gives a name, as far as the declaration itself tells. The caller resolves a
/// type in the file that wrote it, a call through the declaration of what it calls, and another
/// name at the declaration's line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A type as written: `UserRepository`, `*Repo`, `Optional[Repo]`.
    Type(String),
    /// A construction of the named type: `new Repo()`, `Repo{}`, `&Repo{}`, `new(Repo)`.
    New(String),
    /// A call of the named function, or of a Python class: `make_repo`, `store.Open`.
    Call(String),
    /// Another name, as `self.repo = repo` hands on a parameter.
    Name(String),
    /// The class declared on this 1-based line: Python's `self` and `cls`, TypeScript's `this`.
    Class(usize),
    /// Python's `cast(T, x)` as written: the callee and `T`. It writes the type when the callee
    /// is `typing`'s, which only the caller can tell: a project may declare a `cast` of its own.
    Cast(String, String),
    /// An element of the named collection, as a loop hands it out: `for r in repos`,
    /// `for (const r of repos)`, `for _, r := range repos`. [`element_type`] reads it off the
    /// collection's written type.
    Element(String),
    /// A field of a chain of names, as a TypeScript destructuring hands it on: `this` and `repo`
    /// for `const { repo } = this`, `this.uow` and `users` for `const { users: u } = this.uow`.
    Field(Vec<String>, String),
    /// A declaration whose type the rules cannot read: `for repo in`, a tuple, a parameter with
    /// no annotation.
    Unknown,
}
/// A declaration of a name on 1-based `line`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub line: usize,
    pub value: Value,
}
/// The value an expression gives a name: a call, a construction, another name, or unknown.
pub(super) fn value_of(kind: Kind, expr: &str) -> Value {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:await\s+)?(new\s+)?([A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*)*)\s*(?:<[^()]*>)?\s*\(").unwrap()
    });
    static LITERAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(&)?([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)?)\s*(?:\[[^\]]*\])?\s*\{").unwrap()
    });
    static NAME: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^[A-Za-z_$][\w$]*$").unwrap());
    static ASSERTION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^[A-Za-z_][\w.]*\.\(\s*(\*?[A-Za-z_][\w.]*)\s*\)$").unwrap()
    });
    let e = uncommented(kind, expr);
    let e = e.trim().trim_end_matches(';').trim_end();
    // What comes after the bracket that opens at `open` must be nothing, or the next lines.
    let ends = |open: usize| close_of(kind, e, open).is_none_or(|end| e[end..].trim().is_empty());
    // A cast writes the type (#100): `x as T` (the last one of `x as unknown as T`), Go's
    // `i.(T)`, and below Python's `cast(T, x)`.
    if kind == Kind::TsJs {
        let mut depth = 0i32;
        // The first and the last ` as ` outside brackets.
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
        // `as` binds tighter than `?:`, `||`, `??` and `=>`: only one operand in front of it, a
        // name, a call or a literal, is what the cast types.
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
        return match (ends(open), c.get(1).is_some()) {
            (false, _) => Value::Unknown,
            (true, true) if kind == Kind::TsJs => Value::New(name),
            (true, false) if kind == Kind::Go && name == "new" => {
                let inner = &e[open + 1..e.len().saturating_sub(1)];
                Value::New(inner.trim().to_owned())
            }
            (true, false) if kind == Kind::Python && matches!(&*name, "cast" | "typing.cast") => {
                let inner = &e[open + 1..e.len().saturating_sub(1)];
                Value::Cast(name, split_top(kind, inner, b',')[0].trim().to_owned())
            }
            // `make([]*Repo, 0, n)` writes the type of what it makes.
            (true, false) if kind == Kind::Go && name == "make" => {
                let inner = &e[open + 1..e.len().saturating_sub(1)];
                Value::Type(split_top(kind, inner, b',')[0].trim().to_owned())
            }
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
    // A slice, array or map literal writes its type in front of its `{`: `[]Repo{…}`.
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
/// The declarations of `name` that 1-based `line` of `text`, a file of `kind`, reads, with what
/// each gives it: those of the innermost scope that declares it, which hides the scopes around
/// it (#100). Every one of that scope counts, and the caller trusts them only when they agree.
///
/// - Python: a name belongs to its function, so its parameters and every binding in the body
///   count, before the cursor or after it; a function that binds none reads the enclosing
///   function's, then the module's. Nested functions and classes are scopes of their own. A comprehension or a `lambda` counts
///   on the cursor line only. `self` / `cls` is the class a method sits in, and so is `super`
///   (what [`qualifier`] makes of `super()`): the caller starts above that class.
/// - TypeScript and Go: `const`, `let` and `:=` belong to their block, so the declarations above
///   the cursor count, in the nearest block around it that has any, told by indentation: the
///   statements at the block's level and what its header binds (a function's parameters, a Go
///   receiver). Only what a header binds for the block under it hides: another function on its
///   lines, or on the cursor's own line, binds without hiding. `this` is the class around it, unless a
///   `function` or an object literal comes first, and `super` reads as `this` does.
pub fn bindings(kind: Kind, text: &str, line: usize, name: &str) -> Vec<Binding> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    match kind {
        Kind::Python => python_bindings(&lines, at, name),
        Kind::TsJs | Kind::Go => block_bindings(kind, &lines, at, name),
        _ => Vec::new(),
    }
}
/// Whether the word at `range` of 1-based `line` names a keyword argument of a Python call:
/// `recipe_yield=…` behind a `(` or a `,`, or at the start of a line that continues a call. It
/// names a parameter of whatever is called, and no variable of that spelling.
pub fn keyword_argument(text: &str, line: usize, range: &Range<usize>) -> bool {
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
/// Whether line `i` continues the statement above it: that line ends in an open bracket, a comma
/// or a backslash.
pub(super) fn continued(kind: Kind, lines: &[&str], i: usize) -> bool {
    lines[..i]
        .iter()
        .rev()
        .map(|l| uncommented(kind, l))
        .find(|t| !t.trim().is_empty() && !comment(kind, t.trim()))
        .is_some_and(|t| t.trim_end().ends_with(['(', '[', '{', ',', '\\']))
}
fn python_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
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
/// The binding of `name` among the parameters of the `def` on line `d`: its annotation, the class
/// for the first parameter of a method, else unknown.
fn python_params(lines: &[&str], d: usize, params: &str, name: &str, out: &mut Vec<Binding>) {
    for (i, p) in split_top(Kind::Python, params, b',')
        .into_iter()
        .enumerate()
    {
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
        out.push(Binding { line: d + 1, value });
    }
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
/// Walks up from line `at` through the blocks around it: a line at the cursor's block level is a
/// statement, a line indented less opens the block the walk is in, and deeper lines belong to
/// blocks already closed.
fn block_bindings(kind: Kind, lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let this = kind == Kind::TsJs && (name == "this" || name == "super");
    let mut out = Vec::new();
    if !this {
        opener_bindings(kind, &uncommented(kind, lines[at]), at + 1, name, &mut out);
    }
    // A raw string, a template or a block comment over several lines declares nothing.
    let literal = literal_lines(kind, &lines.join("\n"));
    let mut depth = indent(lines[at]);
    let mut i = at;
    // Whether a declaration of the block the walk is in, or of its header, has been found.
    let mut scoped = false;
    // The types of the innermost Go `case` around the cursor, for the `switch v := x.(type)` it
    // may belong to (#100): gofmt writes the two at one indent, so the `switch` is met as a
    // statement right after its `case`.
    let mut arm: Option<Vec<String>> = None;
    // The first line of a statement already read whole, from the line that closes it.
    let mut read: Option<usize> = None;
    let type_switch = Regex::new(&format!(
        r"^switch\s+(?:[^;{{]*;\s*)?{}\s*:=\s*[^;{{]+\.\(type\)\s*\{{$",
        regex::escape(name)
    ))
    .expect("an escaped name keeps the pattern valid");
    while i > 0 {
        i -= 1;
        let code = uncommented(kind, lines[i]);
        let t = code.trim();
        let ind = indent(lines[i]);
        if t.is_empty() || comment(kind, t) || ind > depth || literal[i] {
            continue;
        }
        if ind == depth {
            if !this {
                let before = out.len();
                // `} = deps;` closes a destructuring prettier wrapped (#131): its names are on
                // the lines above, down from the `const {` that is back at this indent. Any
                // statement closed so is read whole.
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
                        // Its first line alone would read `const repo = {` again, as less.
                        read = Some(j);
                    }
                    None if read == Some(i) => {}
                    None => statement_bindings(kind, t, i + 1, name, &mut out),
                }
                // In `case *Repo:` the variable of a type switch is a `*Repo`; under several
                // types or `default` it is whatever came in. A `switch` met with no `case` on
                // the way up is one the cursor is not in.
                if let Some(types) = arm.as_deref().filter(|_| type_switch.is_match(t)) {
                    let value = match types {
                        [one] => Value::Type(one.clone()),
                        _ => Value::Unknown,
                    };
                    out.push(Binding { line: i + 1, value });
                }
                if t.starts_with("switch ") || t.starts_with("select ") {
                    arm = None;
                }
                scoped |= out.len() > before;
            }
            continue;
        }
        // A header over several lines ends in a closer (`) {`, `} else {`) and starts at the
        // line above that is back at its indent.
        // TypeScript's `> extends Base<K> {` closes a wrapped list of type parameters, and a lone
        // `{` is the body of the clauses wrapped above it (#100), when what they are wrapped under
        // declares a type: under a statement it is a block of its own.
        let end = i;
        // The line the closer is back at the indent of.
        let opener = (0..i)
            .rev()
            .find(|&j| !lines[j].trim().is_empty() && indent(lines[j]) <= ind);
        let body = t == "{" && opener.is_some_and(|j| declares_type(kind, lines[j]));
        // A `>` closes type parameters, a line ending in `<`; the `>` of a JSX tag closes no header.
        let params = kind == Kind::TsJs
            && t.starts_with('>')
            && opener.is_some_and(|j| lines[j].trim_end().ends_with('<'));
        let wrapped = params || (kind == Kind::TsJs && body);
        if t.starts_with([')', '}', ']']) || wrapped {
            while i > 0 && (lines[i - 1].trim().is_empty() || indent(lines[i - 1]) > ind) {
                i -= 1;
            }
            i = i.saturating_sub(1);
        }
        // Between an `if` and its `} else {` lies a block the cursor is not in: only the two
        // lines are the header, where a signature closed by `) {` or `}: Deps) {` is all of
        // its lines.
        let sibling = ["else", "catch", "finally"]
            .iter()
            .any(|k| t.trim_start_matches('}').trim_start().starts_with(k));
        // So do the type parameters between `route<` and `>(repo: Repo) {`: the parameter of an
        // arrow among them, `H extends (repo: Log) => void,`, is none of the function's.
        let header: Vec<&str> = match (sibling || params) && end > i {
            true => vec![lines[i].trim(), t],
            false => lines[i..=end].iter().map(|l| l.trim()).collect(),
        };
        let header = uncommented(kind, &header.join("\n"));
        depth = ind.min(indent(lines[i]));
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
            // The innermost block that declares the name hides the ones around it: a statement
            // of the block the walk leaves here, or what the header binds for that block. A
            // callback elsewhere on the header's lines (`if (xs.some((repo: Repo) => …)) {`,
            // `register((repo: Repo) => repo, {`) is a function the cursor is not in, as one on
            // the cursor's own line may be: its parameters count, and hide nothing.
            if opener_bindings(kind, &header, i + 1, name, &mut out) || scoped {
                break;
            }
        } else if let Some(value) = this_opener(&header, i + 1) {
            out.push(Binding { line: i + 1, value });
            break;
        }
    }
    out
}
/// What `this` is inside the block a header opens: the class it declares, unknown under a
/// `function` or an object literal, `None` for a block `this` passes through (a method, an arrow
/// function, an `if`).
fn this_opener(header: &str, line: usize) -> Option<Value> {
    static CLASS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:(?:export|default|declare|abstract)\s+)*class\b").unwrap()
    });
    if CLASS.is_match(header) {
        return Some(Value::Class(line));
    }
    let before = header.strip_suffix('{').map(str::trim_end);
    // `case X: {` and `default: {` open statements, whatever the colon suggests; the `{` of a
    // `case X: return {` on the same line is a literal's.
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
/// The bindings of `name` a block's header makes for the lines inside it: the parameters of a
/// function, a method or an arrow, a Go receiver and named results, the variables of a loop, a
/// `catch` or a Go `if x := …;`. Returns whether one of them is made for the block under the
/// header, and so hides the scopes around it.
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
        Regex::new(r"^(?:(?:public|private|protected|static|readonly|abstract|override|async|get|set)\s+)*\*?#?([\w$]+)\s*(?:<[^>]*>)?$").unwrap()
    });
    static TS_BODY: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(?::.*)?\{").unwrap());
    // What follows the parameters of the function whose body ends the header: a return type,
    // an arrow, the brace.
    static TS_OPENS: std::sync::LazyLock<Regex> =
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
            line,
            value: Value::Unknown,
        })
    };
    let n = regex::escape(name);
    let element = |p: String| {
        let rule = Regex::new(&p).expect("an escaped name keeps the pattern valid");
        let value = Value::Element(rule.captures(header)?[1].to_owned());
        Some(Binding { line, value })
    };
    // Whether a binding is made for the block under the header: by the loop, the `catch` or the
    // Go statement the header is, or by the function whose body ends it. A callback elsewhere on
    // these lines, `if (xs.some((repo: Repo) => …)) {`, binds for a body the cursor is not in.
    let mut own = false;
    match kind {
        Kind::TsJs => {
            let starts = |l: &str| {
                let l = l.trim_start_matches('}').trim_start();
                l.starts_with("for") || l.starts_with("catch")
            };
            let is_loop = starts(header) || header.rsplit('\n').next().is_some_and(starts);
            let count = out.len();
            // `for (const r of repos)` over a plain name hands out elements; `in` hands out keys.
            if let Some(b) = element(format!(
                r"\bfor\s*\(\s*(?:const|let|var)\s+{n}\s+of\s+([A-Za-z_$][\w$]*)\s*\)"
            )) {
                out.push(b);
            } else if TS_LOOP
                .captures_iter(header)
                .any(|c| c.iter().skip(1).flatten().any(|m| names(m.as_str(), name)))
            {
                unknown(out);
            }
            own |= is_loop && out.len() > count;
            for (open, _) in header.match_indices('(') {
                let Some(close) = close_of(kind, header, open) else {
                    continue;
                };
                let (before, after) = (header[..open].trim_end(), header[close..].trim_start());
                let method = TS_METHOD.captures(before).is_some_and(|c| {
                    !matches!(
                        &c[1],
                        "if" | "for" | "while" | "switch" | "catch" | "with" | "return" | "super"
                    ) && TS_BODY.is_match(after)
                });
                if TS_ARROW.is_match(after) || TS_FUNCTION.is_match(before) || method {
                    let count = out.len();
                    ts_params(&header[open + 1..close - 1], line, name, out);
                    own |= out.len() > count && TS_OPENS.is_match(after);
                }
            }
            let arrow = Regex::new(&format!(
                r"(?:^|[^\w$.]){}\s*=>(\s*\{{?\s*$)?",
                regex::escape(name)
            ))
            .expect("an escaped name keeps the pattern valid");
            if let Some(c) = arrow.captures_iter(header).last() {
                unknown(out);
                own |= c.get(1).is_some();
            }
        }
        Kind::Go => {
            let count = out.len();
            // The second variable of a `range` over a plain name is an element of a slice, an
            // array or a map; the first is an index or a key.
            if let Some(b) = element(format!(
                r"^for\s+[A-Za-z_]\w*\s*,\s*{n}\s*:=\s*range\s+([A-Za-z_]\w*)\s*\{{"
            )) {
                out.push(b);
            } else if GO_ASSIGN.captures_iter(header).any(|c| names(&c[1], name)) {
                unknown(out);
            }
            own |= out.len() > count;
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
                    // Between the parameters and the `{` that ends the header stand results
                    // only: another brace there closes this function or opens a literal.
                    let body = results.trim_end().strip_suffix('{');
                    own |= out.len() > count && body.is_some_and(|r| !r.contains(['{', '}']));
                }
            }
        }
        _ => {}
    }
    own
}
/// The binding of `name` among TypeScript parameters: its annotation, else its default value.
fn ts_params(params: &str, line: usize, name: &str, out: &mut Vec<Binding>) {
    static MODS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:(?:public|private|protected|readonly|override)\s+)*(?:\.\.\.)?").unwrap()
    });
    for p in split_top(Kind::TsJs, params, b',') {
        let p = MODS.replace(p.trim(), "");
        if p.starts_with(['{', '[']) {
            if names(&p, name) {
                out.push(Binding {
                    line,
                    value: Value::Unknown,
                });
            }
            continue;
        }
        let parts = split_top(Kind::TsJs, &p, b'=');
        let (pname, annotation) = match parts[0].split_once(':') {
            Some((pname, t)) => (pname, Some(t.trim())),
            None => (parts[0], None),
        };
        if pname.trim().trim_end_matches('?') != name {
            continue;
        }
        let value = match (annotation, parts.get(1)) {
            (Some(t), _) => Value::Type(t.to_owned()),
            (None, Some(default)) => value_of(Kind::TsJs, default),
            (None, None) => Value::Unknown,
        };
        out.push(Binding { line, value });
    }
}
/// The binding of `name` among Go parameters, a receiver or named results: `a, b *T` gives both
/// names the type written after the last of them. A list of bare types names nothing.
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
            out.push(Binding { line, value });
        }
    }
}
/// What `const { repo, audit: name } = this.deps;` gives `name` (#100): the field it is taken
/// from and the chain of names it is taken out of. `None` when the statement does not bind the
/// name so: a default (`{ name = … }`) and a rest are no item that reads as the name, and a
/// nested pattern, an array's or a right side that is more than names is not read.
pub(super) fn ts_destructured(t: &str, name: &str) -> Option<Value> {
    static SHAPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:export\s+)?(?:const|let|var)\s*\{([^{}\[\]]*)\}\s*=\s*((?:this|[A-Za-z_$][\w$]*)(?:\.#?[A-Za-z_$][\w$]*)*)\s*;?$").unwrap()
    });
    let c = SHAPE.captures(t)?;
    let field = c[1].split(',').find_map(|item| {
        let (field, local) = item.split_once(':').unwrap_or((item, item));
        (local.trim() == name).then(|| field.trim().to_owned())
    })?;
    let from = c[2].split('.').map(str::to_owned).collect();
    Some(Value::Field(from, field))
}
/// The binding of `name` a statement at a block's level makes: a TypeScript `const` / `let` /
/// `var`, a Go `:=` or `var`. A destructuring, a second name of a Go `:=` or a declaration the
/// rules cannot type is unknown.
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
                    .is_some_and(|c| names(&c[1], name))
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
                // The names, not the value: `const csp = "… http://…"` declares no `http`.
                .is_some_and(|rest| names(rest.split('=').next().unwrap_or(rest), name))
            {
                Value::Unknown
            } else {
                return;
            }
        }
        _ => return,
    };
    out.push(Binding { line, value });
}
/// The header of the TypeScript declaration on line `k` as one line, up to the `{` of its body,
/// and the index of the line that `{` stands on. prettier wraps a long one (#100): the clauses on
/// lines of their own over a lone `{`, or the type parameters, which are left out here, since
/// their `extends` are constraints: `class Hono<⏎  E extends Env,⏎> extends Base<E> {`.
pub(super) fn ts_header(lines: &[&str], k: usize) -> (String, usize) {
    // ponytail: forty lines of header; hono's widest list of type parameters runs to six.
    let text = uncommented(Kind::TsJs, &lines[k..lines.len().min(k + 40)].join("\n"));
    let b = text.as_bytes();
    let (mut depth, mut angle) = (0i32, 0i32);
    // The type parameters: the first `<…>` outside brackets, when no clause comes before it.
    let (mut from, mut to) = (None, None);
    let mut open = None;
    for (i, c) in code(Kind::TsJs, &text) {
        match c {
            b'{' if depth == 0 && angle == 0 => {
                open = Some(i);
                break;
            }
            // The declaration ended with no body, `type Loose = any;`: the line under it is back
            // at its indent and is no `{`. The next `{` is another declaration's.
            b'\n' if depth == 0 && angle == 0 => {
                let next = text[i + 1..].lines().next().unwrap_or("");
                if indent(next) <= indent(lines[k]) && !next.trim_start().starts_with('{') {
                    break;
                }
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'<' if depth == 0 => {
                if angle == 0 && from.is_none() && !names(&text[..i], "extends") {
                    from = Some(i);
                }
                angle += 1;
            }
            // The `>` of a `=>` closes nothing.
            b'>' if depth == 0 && angle > 0 && b[i - 1] != b'=' => {
                angle -= 1;
                if angle == 0 && from.is_some() && to.is_none() {
                    to = Some(i + 1);
                }
            }
            _ => {}
        }
    }
    let Some(open) = open else {
        return (lines[k].to_owned(), k);
    };
    let last = k + text[..open].matches('\n').count();
    let mut header = text[..=open].to_owned();
    if let (Some(from), Some(to)) = (from, to) {
        header.replace_range(from..to, "");
    }
    (
        header.split_whitespace().collect::<Vec<_>>().join(" "),
        last,
    )
}
/// Whether the Go function around 1-based `line` of `text` may declare `name` (#100): the name
/// stands as a whole word, outside strings and comments, somewhere from the function's `func`
/// line down to `line` itself, other than in front of a `.` or a `)`. A declaration never
/// writes its name there, whatever its form (`f(repo)` hands it on, and a parameter list of bare
/// names is a list of types), so `false` proves there is no local of the
/// name, which an empty [`bindings`] does not: its walk misses a header with a function-typed
/// parameter, a `var (` block inside a function, the lines above a label. Uses of the name as
/// an argument say `true` as well, and the caller then proves nothing.
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
    // The line itself counts: `if repo := get(); repo.Do() {`, a function written on one line.
    for i in (0..=at.min(lines.len().saturating_sub(1))).rev() {
        let l = lines[i];
        if literal[i] || l.trim().is_empty() {
            continue;
        }
        if mentions(l) {
            return true;
        }
        // The function starts at the `func` in column 0, and what ends the declaration above
        // it, or starts another, is the package's, as the cursor then is. Any other line in
        // column 0 still belongs to the function: a header's closer (`) error {`, `}) {`), a
        // label, whatever else: reading on can only find more mentions.
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
/// What the Go file `text` declares `name` as at the level of its package: `var name T`,
/// `var name = …`, alone or as a line of a `var (` block, anywhere in the file (#100). The scope
/// walk of [`bindings`] reads the open file upwards only, so another file of the package and the
/// lines below the cursor are read through this.
pub fn package_bindings(text: &str, name: &str) -> Vec<Binding> {
    let literal = literal_lines(Kind::Go, text);
    let mut out = Vec::new();
    let mut block = false;
    // The indent of the open block's entries: that of its first line.
    let mut level = None;
    for (i, l) in text.lines().enumerate() {
        let code = uncommented(Kind::Go, l);
        let t = code.trim();
        if literal[i] || t.is_empty() {
            continue;
        }
        match (indent(l), block) {
            (0, _) if t == "var (" => (block, level) = (true, None),
            (0, _) => {
                block = false;
                statement_bindings(Kind::Go, t, i + 1, name, &mut out);
            }
            // gofmt aligns the `=` of a block with spaces, which one `var` line never has. A
            // deeper line belongs to an entry's value or type, `cfg struct {` / `repo *Repo`.
            (n, true) if Some(n) == level.or(Some(n)) => {
                level = Some(n);
                let words: Vec<&str> = t.split_whitespace().collect();
                let t = format!("var {}", words.join(" "));
                statement_bindings(Kind::Go, &t, i + 1, name, &mut out);
            }
            _ => {}
        }
    }
    out
}
