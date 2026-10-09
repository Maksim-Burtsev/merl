//! What a declaration says about types: what it returns, what it extends, and the type a
//! written name spells.

use regex::Regex;

use super::*;

/// What a call of the function declared on `decl_line1` of `text` gives: its declared return
/// type (Python `-> T`, TypeScript `): T`, Go's result or its first one), or in TypeScript without
/// one, the `new T()` that every `return` of the body, or an arrow's expression, agrees on.
pub fn returns(kind: Kind, text: &str, decl_line1: usize) -> Option<Value> {
    static PY_DEF: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*(?:async\s+)?def\s+\w+\s*\(").unwrap());
    static TS_FUNCTION_CONST_OR_METHOD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(
        || {
            Regex::new(r"^\s*(?:(?:export|default|declare|async)\s+)*(?:function\*?\s*[\w$]*|(?:const|let|var)\s+[\w$]+\s*(?::[^=]+)?=\s*(?:async\s+)?(?:function\*?\s*[\w$]*)?)\s*(?:<[^>]*>)?\s*\(|^\s*(?:(?:public|private|protected|static|override|abstract|async)\s+)*#?[\w$]+\??\s*(?:<[^>]*>)?\s*\(").unwrap()
        },
    );
    static TS_RETURN: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*:\s*(.+?)\s*(?:\{|=>|;|$)").unwrap());
    static GO_FUNC_METHOD_OR_INTERFACE_METHOD: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| {
            Regex::new(r"^(?:func\s*(?:\([^)]*\)\s*)?|\s+)[A-Za-z_]\w*\s*(?:\[[^\]]*\])?\s*\(")
                .unwrap()
        });
    let lines: Vec<&str> = text.lines().collect();
    let k = decl_line1.checked_sub(1).filter(|&i| i < lines.len())?;
    let opener = match kind {
        Kind::Python => &PY_DEF,
        Kind::TsJs => &TS_FUNCTION_CONST_OR_METHOD,
        Kind::Go => &GO_FUNC_METHOD_OR_INTERFACE_METHOD,
        _ => return None,
    };
    let open = opener.find(lines[k])?.end() - 1;
    let Group {
        close_line: end,
        after_close: after,
        ..
    } = group(kind, &lines, k, open)?;
    match kind {
        // `-> T:` ends at the first colon outside strings and brackets; a body may follow it.
        Kind::Python => {
            let parts = split_top(kind, after, b':');
            match parts[0].trim().strip_prefix("->") {
                Some(t) => (parts.len() > 1 && !t.trim().is_empty())
                    .then(|| Value::Type(t.trim().to_owned())),
                None => python_class_every_return_calls(text, &lines, k, end),
            }
        }
        Kind::TsJs => {
            if let Some(c) = TS_RETURN.captures(after) {
                return Some(Value::Type(c[1].to_owned()));
            }
            let expression = after.trim_start().strip_prefix("=>").map(str::trim);
            let base = indent(lines[k]);
            let body = end
                + 1
                + lines[end + 1..]
                    .iter()
                    .take_while(|l| !(indent(l) <= base && l.trim_start().starts_with('}')))
                    .count();
            let block = expression.is_none_or(|e| e.starts_with('{'));
            let returned: Vec<(usize, Value)> = match expression {
                Some(e) if !block => vec![(k, value_of(kind, e))],
                _ => (end + 1..body)
                    .map(|i| (i, uncommented(kind, lines[i])))
                    .filter(|(_, l)| names(l, "return"))
                    .map(|(i, l)| match l.trim().strip_prefix("return ") {
                        Some(e) => (i, value_of(kind, e)),
                        None => (i, Value::Unknown),
                    })
                    .collect(),
            };
            let first = &returned.first()?.1;
            if !returned.iter().all(|(_, v)| v == first) {
                return None;
            }
            match first {
                Value::New(t) => Some(Value::New(t.clone())),
                Value::Name(local) if block => {
                    ts_returned_local(text, &lines, &returned, end + 1..body, local)
                }
                _ => None,
            }
        }
        _ => {
            let result = after.split('{').next().unwrap_or("").trim();
            let first = match result.strip_prefix('(') {
                Some(list) => {
                    let first = split_top(kind, list.strip_suffix(')')?, b',')[0].trim();
                    first
                        .split_once(char::is_whitespace)
                        .map_or(first, |(_result_name, t)| t.trim())
                }
                None => result,
            };
            (!first.is_empty()).then(|| Value::Type(first.to_owned()))
        }
    }
}
fn ts_returned_local(
    text: &str,
    lines: &[&str],
    returned: &[(usize, Value)],
    body: std::ops::Range<usize>,
    local: &str,
) -> Option<Value> {
    let assigned = Regex::new(&format!(
        r"(?:^|[^\w$.]){}(?:\s*(?:\?\?|\|\||&&)?=|(?:[^\w$=;][^=;]*)?[\]}}]\s*=)[^=>]",
        regex::escape(local)
    ))
    .expect("an escaped name keeps the pattern valid");
    let mut constructed: Option<Value> = None;
    for (i, _) in returned {
        let found = bindings(Kind::TsJs, text, i + 1, local);
        if found.is_empty() {
            return None;
        }
        for b in found {
            if !body.contains(&(b.line1 - 1)) || !matches!(b.value, Value::New(_)) {
                return None;
            }
            if constructed.as_ref().is_some_and(|c| *c != b.value) {
                return None;
            }
            constructed = Some(b.value);
        }
    }
    let declared = |l: &str| {
        let t = l.trim_start();
        ["const ", "let ", "var "].iter().any(|k| t.starts_with(k))
    };
    let reassigned = body.clone().any(|i| {
        let l = uncommented(Kind::TsJs, lines[i]);
        !declared(&l) && assigned.is_match(&l)
    });
    match reassigned {
        true => None,
        false => constructed,
    }
}
fn python_decorated(lines: &[&str], def_line: usize) -> bool {
    let base = indent(lines[def_line]);
    let comment_or_closing_bracket = |t: &str| t.starts_with(['#', ')', ']', '}']);
    lines[..def_line]
        .iter()
        .rev()
        .map(|l| (indent(l), l.trim()))
        .find(|(ind, t)| !t.is_empty() && !comment_or_closing_bracket(t) && *ind <= base)
        .is_some_and(|(ind, t)| ind == base && t.starts_with('@'))
}
fn python_class_every_return_calls(
    text: &str,
    lines: &[&str],
    def_line: usize,
    signature_end: usize,
) -> Option<Value> {
    let base = indent(lines[def_line]);
    if python_decorated(lines, def_line) {
        return None;
    }
    let literal = literal_lines(Kind::Python, text);
    let mut constructed: Option<String> = None;
    let mut nested_def_or_class_indent: Option<usize> = None;
    for (i, l) in lines.iter().enumerate().skip(signature_end + 1) {
        let code = uncommented(Kind::Python, l);
        let (t, ind) = (code.trim(), indent(l));
        if t.is_empty() || literal.get(i).copied().unwrap_or(false) {
            continue;
        }
        if ind <= base {
            break;
        }
        if nested_def_or_class_indent.is_some_and(|s| ind > s) {
            continue;
        }
        let inner = ["def ", "async def ", "class "];
        nested_def_or_class_indent = inner.iter().any(|p| t.starts_with(p)).then_some(ind);
        if names(t, "yield") {
            return None;
        }
        let return_behind_condition = names(t, "return") && !t.starts_with("return");
        if return_behind_condition {
            return None;
        }
        if t == "return" || t.starts_with("return ") {
            let Value::Call(name) = value_of(Kind::Python, t.strip_prefix("return")?) else {
                return None;
            };
            if constructed.as_ref().is_some_and(|c| *c != name) {
                return None;
            }
            constructed = Some(name);
        }
    }
    constructed.map(Value::New)
}
pub(super) fn type_list(kind: Kind, s: &str) -> Vec<String> {
    let type_parameter_default = |b: &str| b.contains('=');
    split_top(kind, s, b',')
        .into_iter()
        .map(str::trim)
        .filter(|b| !b.is_empty() && !type_parameter_default(b))
        .map(str::to_owned)
        .collect()
}
/// The types the class, interface or struct declared on `decl_line1` of `text` extends or
/// embeds, as written: Python's bases, TypeScript's `extends`, Go's embedded fields.
pub fn bases(kind: Kind, text: &str, decl_line1: usize) -> Vec<String> {
    static TS_EXTENDS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bextends\s+(.+?)\s*(?:\bimplements\b|\{|$)").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = decl_line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    let list = |s: &str| type_list(kind, s);
    match kind {
        Kind::Python => lines[k]
            .trim_start()
            .starts_with("class ")
            .then(|| lines[k].find('('))
            .flatten()
            .and_then(|open| group(kind, &lines, k, open))
            .map_or_else(Vec::new, |g| list(&g.inner_uncommented)),
        Kind::TsJs => TS_EXTENDS
            .captures(&ts_header(&lines, k).0)
            .map_or_else(Vec::new, |c| list(&c[1])),
        Kind::Go if lines[k].contains("struct") || lines[k].contains("interface") => {
            let body = body_of(kind, &lines, k);
            let base = lines[body.clone()]
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|l| indent(l))
                .min();
            body.filter(|&i| Some(indent(lines[i])) == base)
                .filter_map(|i| {
                    let t = lines[i].split("//").next().unwrap_or("");
                    go_embedded(t.split('`').next().unwrap_or("").trim()).map(str::to_owned)
                })
                .collect()
        }
        _ => Vec::new(),
    }
}
pub fn interfaces(kind: Kind, text: &str, decl_line1: usize) -> Vec<String> {
    static TS_IMPLEMENTS: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\bimplements\s+(.+?)\s*(?:\{|$)").unwrap());
    if kind != Kind::TsJs {
        return Vec::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    decl_line1
        .checked_sub(1)
        .filter(|&k| k < lines.len())
        .and_then(|k| {
            TS_IMPLEMENTS
                .captures(&ts_header(&lines, k).0)
                .map(|c| type_list(kind, &c[1]))
        })
        .unwrap_or_default()
}
/// The 1-based line of the type declaration the member on `member_line1` of `text` is written
/// inside: the nearest line above indented less, when it [`declares_type`]. `None` for a Go
/// method, which stands beside its type at the top level, and for a function nested in another
/// one, whose nearest enclosing line declares no type.
pub fn owner_decl(kind: Kind, text: &str, member_line1: usize) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let depth = indent(lines.get(member_line1.checked_sub(1)?)?);
    let (i, above) = lines[..member_line1 - 1]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, l)| {
            let t = l.trim_start();
            let ends_a_wrapped_header = match kind {
                Kind::Python => t.starts_with([')', ']']),
                Kind::TsJs => t.starts_with('>'),
                _ => false,
            };
            !t.is_empty()
                && t.trim_end() != "{"
                && !comment(kind, t)
                && !ends_a_wrapped_header
                && indent(l) < depth
        })?;
    declares_type(kind, above).then_some(i + 1)
}
/// The name a line declaring a type gives it: `Foo` for `class Foo(Base):`,
/// `export abstract class Foo<T> extends Bar {`, `interface Foo {` and `type Foo struct {`.
pub fn type_name(kind: Kind, line: &str) -> Option<String> {
    static NAME: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:class|interface|type|enum)\s+([A-Za-z_$][\w$]*)").unwrap()
    });
    declares_type(kind, line)
        .then(|| NAME.captures(line))
        .flatten()
        .map(|c| c[1].to_owned())
}
/// The 1-based line the type declaration covering `line1` of `text` starts on: `line1`
/// itself when it [`declares_type`], else the nearest line above it that does, when nothing but
/// the rest of a header stands in between — `export class X` over `    extends Y` over `{`, as
/// prettier wraps a long one. `None` when the lines above end a statement first.
pub fn type_decl_at(kind: Kind, text: &str, line1: usize) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let k = line1.checked_sub(1).filter(|&k| k < lines.len())?;
    // ponytail: eight lines of header, which covers the widest prettier writes.
    for i in (k.saturating_sub(8)..=k).rev() {
        if declares_type(kind, lines[i]) {
            return Some(i + 1);
        }
        let t = lines[i].trim();
        if t.is_empty() || t.ends_with([';', '}']) || comment(kind, t) {
            return None;
        }
    }
    None
}
/// The 1-based line on which the type declared on `decl_line1` of `text` declares the member
/// `word` itself: a method, or a signature with no body. `None` when it does not declare one, so
/// a subclass that inherits the member is no implementation of it.
pub fn member_decl(kind: Kind, text: &str, decl_line1: usize, word: &str) -> Option<usize> {
    let patterns = member_or_signature(kind, word)?;
    let re = Regex::new(&patterns.join("|")).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let k = decl_line1.checked_sub(1).filter(|&k| k < lines.len())?;
    body_of(kind, &lines, k)
        .find(|&i| re.is_match(lines[i]) && owner_decl(kind, text, i + 1) == Some(decl_line1))
        .map(|i| i + 1)
}
/// How many parameters the declaration on `line1` of `text` writes: what stands between
/// its brackets, cut at the commas outside brackets and strings, over the lines they span. Go's
/// `a, b string` counts two, as an implementation of the same interface method writes two of its
/// own, and a Go method's receiver is not a parameter.
pub fn params(kind: Kind, text: &str, line1: usize) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let k = line1.checked_sub(1).filter(|&k| k < lines.len())?;
    let mut opens = code(kind, lines[k])
        .filter(|&(_, c)| c == b'(')
        .map(|(i, _)| i);
    let receiver = kind == Kind::Go && lines[k].trim_start().starts_with("func (");
    let open = if receiver { opens.nth(1) } else { opens.next() }?;
    let Group {
        inner_uncommented: inner,
        ..
    } = group(kind, &lines, k, open)?;
    Some(
        split_top(kind, &inner, b',')
            .iter()
            .filter(|p| !p.trim().is_empty())
            .count(),
    )
}
/// The types as written but for the package in front of a name and the spaces: `a, b string` is
/// two `string`s, `ctx context.Context` is `Context`. `None` when the result names its values.
#[derive(Debug, PartialEq, Eq)]
pub struct GoSignature {
    pub param_types: Vec<String>,
    pub result: String,
}
pub fn go_signature(text: &str, line1: usize) -> Option<GoSignature> {
    static PKG: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\b\w+\.").unwrap());
    let kind = Kind::Go;
    let lines: Vec<&str> = text.lines().collect();
    let k = line1.checked_sub(1).filter(|&k| k < lines.len())?;
    let mut opens = code(kind, lines[k])
        .filter(|&(_, c)| c == b'(')
        .map(|(i, _)| i);
    let receiver = lines[k].trim_start().starts_with("func (");
    let open = if receiver { opens.nth(1) } else { opens.next() }?;
    let Group {
        inner_uncommented: inner,
        after_close: rest,
        ..
    } = group(kind, &lines, k, open)?;
    let plain = |t: &str| {
        PKG.replace_all(t, "")
            .split_whitespace()
            .collect::<String>()
    };
    let parts: Vec<&str> = split_top(kind, &inner, b',')
        .into_iter()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    fn typed(p: &str) -> Option<&str> {
        p.split_once(char::is_whitespace).map(|(_, t)| t.trim())
    }
    let named = parts.iter().any(|p| typed(p).is_some());
    let mut types: Vec<String> = Vec::new();
    // Backwards: a name with no type of its own takes the one that follows it.
    for p in parts.iter().rev() {
        let t = match (named, typed(p)) {
            (true, Some(t)) => plain(t),
            (true, None) => types.last()?.clone(),
            (false, _) => plain(p),
        };
        types.push(t);
    }
    types.reverse();
    let result = rest.split('{').next().unwrap_or_default().trim();
    let names_values = result.starts_with('(')
        && split_top(kind, result.trim_matches(['(', ')']), b',')
            .iter()
            .any(|p| p.trim().contains(char::is_whitespace));
    (!names_values).then(|| GoSignature {
        param_types: types,
        result: plain(result),
    })
}
/// Whether `line` declares a type: a Python class, a TypeScript class, interface, type alias or
/// enum, a Go `type`. An alias goes on as `=` or `<`: `type Notifier,` is an item of a wrapped
/// import or export list.
pub fn declares_type(kind: Kind, line: &str) -> bool {
    static TS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"^\s*(?:(?:export|default|declare|abstract)\s+)*(?:(?:class|interface|enum)\s|type\s+[\w$]+\s*[=<])",
        )
        .unwrap()
    });
    match kind {
        Kind::Python => line.trim_start().starts_with("class "),
        Kind::TsJs => TS.is_match(line),
        Kind::Go => line.starts_with("type "),
        _ => false,
    }
}
/// What the Go line `type X = Y` names, `Y` as written; `None` for a defined type, `type X Y`,
/// and for an alias with type parameters, whose arguments the rules do not carry.
pub fn go_alias(kind: Kind, line: &str) -> Option<&str> {
    static ALIAS: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^type\s+[A-Za-z_]\w*\s*=\s*([^/]+)").unwrap());
    let named = ALIAS.captures(line).filter(|_| kind == Kind::Go)?;
    Some(named.get(1)?.as_str().trim())
}
/// The name a written type comes down to, as its dotted parts: `["UserRepository"]`,
/// `["store", "Session"]`. `T | None`, `Optional[T]`, `Annotated[T, …]`, `T | null | undefined`, a
/// quoted forward reference, a Go pointer and generic arguments read as `T`. A list, a function
/// type, a union of two types or an inline object type has no one name.
pub fn type_path(kind: Kind, written: &str) -> Option<Vec<String>> {
    static NAME: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^[A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*)*$").unwrap()
    });
    let mut t = written.trim().trim_end_matches([';', ',']).trim();
    if kind == Kind::Python {
        t = t.trim_matches(['"', '\'']);
    }
    let left: Vec<&str> = split_top(kind, t, b'|')
        .into_iter()
        .map(str::trim)
        .filter(|p| !matches!(*p, "None" | "null" | "undefined"))
        .collect();
    let [one] = left[..] else { return None };
    t = one.trim_start_matches('*');
    if kind == Kind::Python {
        for wrapper in [
            "Optional[",
            "Annotated[",
            "typing.Optional[",
            "typing.Annotated[",
        ] {
            if let Some(inner) = t.strip_prefix(wrapper).and_then(|s| s.strip_suffix(']')) {
                return type_path(kind, split_top(kind, inner, b',')[0]);
            }
        }
    }
    let (open, close) = if kind == Kind::TsJs {
        ('<', '>')
    } else {
        ('[', ']')
    };
    if let Some(i) = t.find(open).filter(|_| t.ends_with(close)) {
        t = t[..i].trim_end();
    }
    NAME.is_match(t)
        .then(|| t.split('.').map(str::to_owned).collect())
}
/// The written type of an element of the collection written as `written`, where the type says so:
/// Python's `list[T]`, `Sequence[T]`, `set[T]`, `tuple[T, ...]` and the like, TypeScript's `T[]`,
/// `Array<T>`, `Set<T>`, Go's `[]T`, `[4]T` and `map[K]T`. A Python `dict` and a TypeScript `Map`
/// hand out keys or pairs, and anything else is not known to hand out anything.
pub fn element_type(kind: Kind, written: &str) -> Option<String> {
    fn last_head_name_and_args(
        kind: Kind,
        t: &str,
        open: char,
        close: char,
    ) -> Option<(&str, Vec<&str>)> {
        let i = t.find(open).filter(|_| t.ends_with(close))?;
        let head = t[..i].trim_end();
        let args = split_top(kind, &t[i + 1..t.len() - 1], b',');
        Some((head.rsplit('.').next().unwrap_or(head), args))
    }
    let t = written.trim().trim_end_matches([';', ',']).trim();
    let element = match kind {
        Kind::Python => match last_head_name_and_args(kind, t.trim_matches(['"', '\'']), '[', ']')?
        {
            ("tuple" | "Tuple", args) if args.len() == 2 && args[1].trim() == "..." => args[0],
            (
                "list" | "List" | "Sequence" | "MutableSequence" | "Iterable" | "Iterator"
                | "Collection" | "set" | "Set" | "frozenset" | "FrozenSet" | "AbstractSet"
                | "deque",
                args,
            ) => args[0],
            _ => return None,
        },
        Kind::TsJs => {
            let t = t.strip_prefix("readonly ").unwrap_or(t).trim();
            match (
                t.strip_suffix("[]"),
                last_head_name_and_args(kind, t, '<', '>'),
            ) {
                (Some(one), _) => one,
                (
                    None,
                    Some((
                        "Array" | "ReadonlyArray" | "Set" | "ReadonlySet" | "Iterable"
                        | "IterableIterator",
                        args,
                    )),
                ) => args[0],
                _ => return None,
            }
        }
        Kind::Go => {
            let rest = t.strip_prefix("map").unwrap_or(t);
            let close = close_of(kind, rest, 0).filter(|_| rest.starts_with('['))?;
            &rest[close..]
        }
        _ => return None,
    };
    let element = element.trim();
    (!element.is_empty()).then(|| element.to_owned())
}
pub fn enum_constants(text: &str, decl_line1: usize) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    let mut line = 1;
    while line < decl_line1 {
        match chars.next() {
            Some('\n') => line += 1,
            Some(_) => {}
            None => return out,
        }
    }
    let mut body_depth: Option<usize> = None;
    let mut parens_before_body = 0usize;
    let mut expect_constant = false;
    let mut annotation = false;
    while let Some(c) = chars.next() {
        match c {
            '\n' => line += 1,
            '/' if chars.peek() == Some(&'/') => while chars.next_if(|&c| c != '\n').is_some() {},
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut prev = ' ';
                for c in chars.by_ref() {
                    line += usize::from(c == '\n');
                    if prev == '*' && c == '/' {
                        break;
                    }
                    prev = c;
                }
            }
            '"' | '\'' => {
                let mut escaped = false;
                for d in chars.by_ref() {
                    line += usize::from(d == '\n');
                    if d == c && !escaped {
                        break;
                    }
                    escaped = d == '\\' && !escaped;
                }
            }
            '{' if body_depth.is_none() && parens_before_body == 0 => {
                body_depth = Some(0);
                expect_constant = true;
            }
            '(' | '{' | '[' => match body_depth.as_mut() {
                Some(d) => *d += 1,
                None => parens_before_body += 1,
            },
            ')' | '}' | ']' => match body_depth {
                Some(0) => break,
                Some(d) => body_depth = Some(d - 1),
                None => parens_before_body = parens_before_body.saturating_sub(1),
            },
            ';' if body_depth == Some(0) => break,
            ',' if body_depth == Some(0) => expect_constant = true,
            '@' if body_depth == Some(0) => annotation = true,
            c if body_depth == Some(0) && (c.is_alphabetic() || c == '_') => {
                let mut name = c.to_string();
                let dotted_name_allowed = annotation;
                while let Some(c) = chars.next_if(|&c| {
                    c.is_alphanumeric() || c == '_' || (dotted_name_allowed && c == '.')
                }) {
                    name.push(c);
                }
                if annotation {
                    annotation = false;
                } else if expect_constant {
                    out.push((name, line));
                    expect_constant = false;
                }
            }
            _ => {}
        }
    }
    out
}
