//! The fields a type declares, and which of its lines declares the one being looked for.

use std::collections::HashMap;

use regex::Regex;

use super::*;

/// The lines of the body of the class, interface or struct declared on line `k`: past a Python
/// header over several lines, up to the first line back at the declaration's indent.
pub(super) fn body_of(kind: Kind, lines: &[&str], k: usize) -> std::ops::Range<usize> {
    let start = match (kind, lines[k].find('(')) {
        (Kind::Python, Some(open)) => {
            group(kind, lines, k, open).map_or(k + 1, |(_, end, _)| end + 1)
        }
        (Kind::TsJs, _) => ts_header(lines, k).1 + 1,
        _ => k + 1,
    };
    let base = indent(lines[k]);
    let end = (start..lines.len())
        .find(|&i| {
            let t = lines[i].trim();
            // A lone `{` at the declaration's own indentation opens its body, as prettier
            // writes a wrapped class header; it does not end it.
            !t.is_empty() && t != "{" && !comment(kind, t) && indent(lines[i]) <= base
        })
        .unwrap_or(lines.len());
    start..end.max(start)
}
/// The declarations of the field `name` of the class, interface or struct declared on 1-based
/// `decl` of `text`:
/// - Python: `name: T` or `name = …` in the class body, `self.name: T = …` or `self.name = …` in
///   its methods, a `def name` under `@property`, `@cached_property` or
///   `@functools.cached_property`, read as its `-> T` (a setter declares nothing);
/// - TypeScript: a member `name: T` or `name = …` behind any modifiers, a constructor parameter
///   with one (`private name: T`), `this.name = …`, a getter `get name(): T`;
/// - Go: a struct field `name T` or `a, name T`, and an embedded `*Name` under its type's name.
pub fn field_bindings(kind: Kind, text: &str, decl: usize, name: &str) -> Vec<Binding> {
    static GO_FIELD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)*)\s+(\S.*)$").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = decl.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    let body = body_of(kind, &lines, k);
    let Some(base) = lines[body.clone()]
        .iter()
        .find(|l| !l.trim().is_empty() && !comment(kind, l.trim()))
        .map(|l| indent(l))
    else {
        return Vec::new();
    };
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    // A docstring or a block comment declares nothing, whatever its lines look like.
    let literal = literal_lines(kind, text);
    let body = body.filter(|&i| !literal.get(i).copied().unwrap_or(false));
    let mut out = Vec::new();
    let mut push = |i: usize, value: Value| out.push(Binding { line: i + 1, value });
    match kind {
        Kind::Python => {
            let annotated = rule(format!(r"^(self\.)?{n}\s*:\s*([^=]+?)\s*(?:=.*)?$"));
            let assigned = rule(format!(r"^(self\.)?{n}\s*=\s*([^=].*)$"));
            // A tuple target (`self.a, self.repo = …`), not an argument of a call.
            let unknown = rule(format!(
                r"^\(?[\w\s,.*\[\]]*,[\w\s,.*\[\]]*\bself\.{n}\b[\w\s,.*\[\]]*\)?\s*=[^=]|^\(?[\w\s.*\[\]]*\bself\.{n}\s*,[\w\s,.*\[\]]*\)?\s*=[^=]|\bas\s+self\.{n}\b|^for\s+.*\bself\.{n}\b.*\sin\s"
            ));
            let def = rule(format!(r"^def\s+{n}\s*\("));
            // The decorators right above line `i` at its indent include a property's.
            let property = |i: usize, ind: usize| {
                lines[..i]
                    .iter()
                    .rev()
                    .map(|l| uncommented(kind, l))
                    .filter(|l| !l.trim().is_empty())
                    .take_while(|l| indent(l) == ind && l.trim_start().starts_with('@'))
                    .any(|l| {
                        matches!(
                            l.trim(),
                            "@property" | "@cached_property" | "@functools.cached_property"
                        )
                    })
            };
            let mut skip: Option<usize> = None;
            for i in body {
                let (code, ind) = (uncommented(kind, lines[i]), indent(lines[i]));
                let t = code.trim();
                if t.is_empty() || skip.is_some_and(|k| ind > k) {
                    continue;
                }
                skip = t.starts_with("class ").then_some(ind);
                if skip.is_some() || continued(kind, &lines, i) {
                    continue;
                }
                // `x: T` belongs to the class body, `self.x: T` to a method.
                let right = |c: &regex::Captures| c.get(1).is_some() == (ind > base);
                if ind == base && def.is_match(t) && property(i, ind) {
                    push(i, returns(kind, text, i + 1).unwrap_or(Value::Unknown));
                    continue;
                }
                // An assignment need not start its line (#131): `if x: self.repo = A()`.
                // A header binds on its own line: `with open(p) as self.h:`.
                if unknown.is_match(t) {
                    push(i, Value::Unknown);
                    continue;
                }
                for s in python_statements(t, false) {
                    if let Some(c) = annotated.captures(s).filter(right) {
                        push(i, Value::Type(c[2].to_owned()));
                    } else if let Some(c) = assigned.captures(s).filter(right) {
                        push(i, value_of(kind, &c[2]));
                    }
                }
            }
        }
        Kind::TsJs => {
            let member = rule(format!(
                r"^(?:(?:public|private|protected|readonly|static|declare|override|abstract|accessor)\s+)*{n}\s*[?!]?\s*(?::\s*([^=;]+?))?\s*(?:=\s*(.+?))?\s*;?$"
            ));
            let param = rule(format!(
                r"(?:^|[(,])\s*(?:@[\w$.]+(?:\([^)]*\))?\s*)*(?:(?:public|private|protected|readonly|override)\s+)+{n}\s*\??\s*:\s*([^,)=]+)"
            ));
            let assigned = rule(format!(r"^this\.{n}\s*=\s*([^=].*?)\s*;?$"));
            let getter = rule(format!(
                r"^(?:(?:public|private|protected|static|override|abstract|declare)\s+)*get\s+{n}\s*\(\s*\)\s*(?::\s*(.+?))?\s*(?:\{{.*)?;?$"
            ));
            // A parameter behind a modifier is a field only in the constructor's list: in a type
            // literal or another method's parameters it is nothing of the class.
            let in_constructor = |i: usize| {
                let ind = indent(lines[i]);
                lines[i].trim_start().starts_with("constructor")
                    || lines[..i]
                        .iter()
                        .rev()
                        .find(|l| !l.trim().is_empty() && indent(l) < ind)
                        .is_some_and(|l| l.trim_start().starts_with("constructor"))
            };
            // `this` there is another object: a literal, a `function`, a nested class.
            let foreign = |i: usize| {
                bindings(kind, text, i + 1, "this")
                    .iter()
                    .any(|b| b.value != Value::Class(decl))
            };
            for i in body {
                let (code, ind) = (uncommented(kind, lines[i]), indent(lines[i]));
                let t = code.trim();
                if let Some(c) = getter.captures(t).filter(|_| ind == base) {
                    push(
                        i,
                        c.get(1)
                            .map_or(Value::Unknown, |ty| Value::Type(ty.as_str().to_owned())),
                    );
                } else if let Some(c) = member.captures(t).filter(|_| ind == base) {
                    push(
                        i,
                        match (c.get(1), c.get(2)) {
                            (Some(ty), _) => Value::Type(ty.as_str().trim().to_owned()),
                            (None, Some(v)) => value_of(kind, v.as_str()),
                            (None, None) => Value::Unknown,
                        },
                    );
                } else if let Some(c) = param.captures(t).filter(|_| in_constructor(i)) {
                    push(i, Value::Type(c[1].trim().to_owned()));
                } else if let Some(c) = assigned.captures(t).filter(|_| ind > base && !foreign(i)) {
                    push(i, value_of(kind, &c[1]));
                }
            }
        }
        Kind::Go if lines[k].contains("struct") => {
            for i in body.filter(|&i| indent(lines[i]) == base) {
                let t = lines[i].split("//").next().unwrap_or("");
                let t = t.split('`').next().unwrap_or("").trim();
                if let Some(embedded) = go_embedded(t) {
                    if embedded.rsplit('.').next() == Some(name) {
                        push(i, Value::Type(t.to_owned()));
                    }
                } else if let Some(c) = GO_FIELD.captures(t)
                    && c[1].split(',').any(|p| p.trim() == name)
                {
                    push(i, Value::Type(c[2].to_owned()));
                }
            }
        }
        _ => {}
    }
    out
}
/// The type an embedded Go field names, `sync.Mutex` for `*sync.Mutex`: a line of nothing else.
pub(super) fn go_embedded(t: &str) -> Option<&str> {
    static EMBEDDED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\*?((?:[A-Za-z_]\w*\.)?[A-Za-z_]\w*)(?:\[.*\])?$").unwrap()
    });
    EMBEDDED.captures(t).map(|c| c.get(1).unwrap().as_str())
}
/// The 1-based line of the type declaration 0-based line `k` of `lines` sits in, however deep:
/// the enclosing lines, each indented less than the last, until one declares a type. `None` when
/// the walk reaches the top level first.
pub(super) fn enclosing_type(kind: Kind, lines: &[&str], k: usize) -> Option<usize> {
    let mut depth = indent(lines[k]);
    for i in (0..k).rev() {
        if depth == 0 {
            break;
        }
        let t = lines[i].trim();
        // A closer at a lower indent ends a signature or a header wrapped over several lines.
        if t.is_empty()
            || t == "{"
            || comment(kind, t)
            || t.starts_with([')', ']'])
            || (kind == Kind::TsJs && t.starts_with('>'))
            || indent(lines[i]) >= depth
        {
            continue;
        }
        depth = indent(lines[i]);
        if declares_type(kind, lines[i]) {
            return Some(i + 1);
        }
    }
    None
}
/// The line among `bindings`, the [`field_bindings`] of one type over `lines`, that declares the
/// field: the first that is no assignment inside a method, else the first assignment, which the
/// second value says.
fn declaring(lines: &[&str], bindings: &[Binding], name: &str) -> Option<(usize, bool)> {
    let assigned = |b: &&Binding| in_method(lines[b.line - 1], name);
    bindings
        .iter()
        .find(|b| !assigned(b))
        .map(|b| (b.line, false))
        .or_else(|| bindings.first().map(|b| (b.line, true)))
}
/// The line on which the type declared on 1-based `decl` of `text` declares its field `name` —
/// the class-body annotation, a constructor parameter, the struct field, else the first
/// `self.name = …` — and whether that line is an assignment inside a method. `None` when the type
/// has no field of that name.
pub fn field_line(kind: Kind, text: &str, decl: usize, name: &str) -> Option<(usize, bool)> {
    let lines: Vec<&str> = text.lines().collect();
    declaring(&lines, &field_bindings(kind, text, decl, name), name)
}
/// The fields `name` the 1-based `hits` of `text` stand for: a hit that is one of the
/// [`field_bindings`] of the type around it gives that type's [`field_line`], once, in line
/// order. What [`field_patterns`] greps is more than the fields; this keeps the fields.
pub fn field_rows(kind: Kind, text: &str, hits: &[usize], name: &str) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let mut types: HashMap<usize, Vec<Binding>> = HashMap::new();
    let mut out = Vec::new();
    for &line in hits {
        let Some(k) = line.checked_sub(1).filter(|&k| k < lines.len()) else {
            continue;
        };
        let Some(decl) = enclosing_type(kind, &lines, k) else {
            continue;
        };
        let bindings = types
            .entry(decl)
            .or_insert_with(|| field_bindings(kind, text, decl, name));
        if bindings.iter().any(|b| b.line == line)
            && let Some((first, _)) = declaring(&lines, bindings, name)
            && !out.contains(&first)
        {
            out.push(first);
        }
    }
    out.sort_unstable();
    out
}
/// Whether the word `name` at byte `start` of 1-based `line` of `text` is the name a field's
/// declaration gives it ([`field_rows`]): the first time the line spells it, as `self.repo = repo`
/// and `this.f = this.f.bind(this)` spell it twice, and never a Go embedded struct, whose name
/// is its type's.
pub fn field_decl_at(kind: Kind, text: &str, line: usize, start: usize, name: &str) -> bool {
    let Some(l) = line.checked_sub(1).and_then(|k| text.lines().nth(k)) else {
        return false;
    };
    let ident = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    let first = l
        .match_indices(name)
        .map(|(i, _)| i)
        .find(|&i| !l[..i].ends_with(ident) && !l[i + name.len()..].starts_with(ident));
    let bare = l.split("//").next().unwrap_or("");
    let embedded =
        kind == Kind::Go && go_embedded(bare.split('`').next().unwrap_or("").trim()).is_some();
    first == Some(start) && !embedded && field_rows(kind, text, &[line], name) == [line]
}
