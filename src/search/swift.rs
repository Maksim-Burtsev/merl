//! Swift's scopes (#371, #366): the blocks and type bodies around a line, and the names a
//! line binds.

use regex::Regex;

use super::*;

/// A Swift line that opens a function's body, whose `let`s, `var`s and parameters are its own: a
/// `func`, an `init`, a `subscript`, a `deinit`, an accessor, a computed property.
static SWIFT_FUNC: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(&format!(
        r"{}(?:func\s|(?:init|subscript)\s*[?!(<]|deinit\b|(?:get|set|willSet|didSet|_read|_modify)\b|(?:var|let)\s+`?\w+`?\s*:[^=]*\{{\s*$)",
        swift_mods!()
    ))
    .unwrap()
});
/// A Swift line that opens a type's body, whose `let`s and `var`s are members. Read after
/// [`SWIFT_FUNC`], which takes `class func` and `class var x: T {`.
static SWIFT_TYPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(&format!(
        r"{}(?:class|struct|enum|protocol|actor|extension)\s+[`\w]",
        swift_mods!()
    ))
    .unwrap()
});
/// A `let` or a `var` statement of Swift, and what follows the keyword.
static SWIFT_DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(&format!(r"{}(?:let|var)\s+(.*)$", swift_mods!())).unwrap()
});
/// Whether 0-based line `i` of Swift `lines` is something the walk over the blocks reads: code,
/// no comment, no `#if`, no line of a multi-line string.
fn swift_code<S: AsRef<str>>(lines: &[S], literal: &[bool], i: usize) -> bool {
    let t = lines[i].as_ref().trim();
    !t.is_empty()
        && !literal.get(i).copied().unwrap_or(false)
        && !t.starts_with('#')
        && !comment(Kind::Swift, t)
}
/// The first line of the header that 0-based line `i` of Swift `lines` ends: `) -> Int {` and a
/// lone `{` close one wrapped over the lines above, from the line back at their indent, and a line
/// under one ending in `,` goes on a condition or a parameter list.
fn swift_header_start<S: AsRef<str>>(lines: &[S], literal: &[bool], mut i: usize) -> usize {
    loop {
        let ind = indent(lines[i].as_ref());
        let wrapped = lines[i]
            .as_ref()
            .trim_start()
            .starts_with([')', '{', ']', '>']);
        let code = |j: usize| swift_code(lines, literal, j);
        let Some(j) = (0..i)
            .rev()
            .find(|&j| code(j) && indent(lines[j].as_ref()) <= ind)
        else {
            return i;
        };
        let comma = (0..i).rev().find(|&j| code(j)) == Some(j)
            && uncommented(Kind::Swift, lines[j].as_ref())
                .trim_end()
                .ends_with(',');
        if !wrapped && !comma {
            return i;
        }
        i = j;
    }
}
/// Where 1-based `line` of a Swift file sits, told by indentation (#371): the 1-based line of the
/// nearest `func`, `init`, `subscript`, `deinit` or accessor around it, or of a type's header when
/// that comes first, or 0 at the top of the file; and whether a `let` or a `var` there is a local,
/// which is when the block right around it is no type's body and not the file's top level.
/// `literal` is [`literal_lines`] of the file.
pub fn swift_scope<S: AsRef<str>>(lines: &[S], literal: &[bool], line: usize) -> (usize, bool) {
    let Some(target) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return (0, false);
    };
    let (mut depth, mut i, mut local) = (indent(target.as_ref()), line - 1, None);
    while i > 0 && depth > 0 {
        i -= 1;
        if !swift_code(lines, literal, i) || indent(lines[i].as_ref()) >= depth {
            continue;
        }
        depth = indent(lines[i].as_ref());
        i = swift_header_start(lines, literal, i);
        let t = uncommented(Kind::Swift, lines[i].as_ref());
        let func = SWIFT_FUNC.is_match(&t);
        let ty = !func && SWIFT_TYPE.is_match(&t);
        let local = *local.get_or_insert(!ty);
        if func || ty {
            return (i + 1, local);
        }
    }
    (0, local.unwrap_or(false))
}
/// A Swift type's header (#380): its keyword (`class`, `extension`, …), the type it names, the
/// first type its header inherits, and whether a `where` constrains it.
pub fn swift_type_header(line: &str) -> Option<(String, String, Option<String>, bool)> {
    static HEADER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"{}(class|struct|enum|protocol|actor|extension)\s+`?([\w.]+)`?(?:<[^>]*>)?\s*(?::\s*([\w.]+))?",
            swift_mods!()
        ))
        .unwrap()
    });
    let t = uncommented(Kind::Swift, line);
    if SWIFT_FUNC.is_match(&t) {
        return None;
    }
    let c = HEADER.captures(&t)?;
    Some((
        c[1].to_owned(),
        c[2].to_owned(),
        c.get(3).map(|m| m.as_str().to_owned()),
        t.contains(" where "),
    ))
}
/// The 1-based line of the header of the Swift type whose body holds 1-based `line`, through
/// the functions and closures around it; `None` at the top of a file (#380).
pub fn swift_enclosing_type<S: AsRef<str>>(
    lines: &[S],
    literal: &[bool],
    line: usize,
) -> Option<usize> {
    let mut at = line;
    loop {
        at = swift_scope(lines, literal, at).0;
        if at == 0 {
            return None;
        }
        if swift_type_header(lines[at - 1].as_ref()).is_some() {
            return Some(at);
        }
    }
}
/// Whether a Swift line declares a `func`, a `let` or a `var` with no `static` or `class` among
/// its modifiers, a member only a value reaches (#380).
pub fn swift_instance_member(line: &str) -> bool {
    swift_static_member(line) == Some(false)
}
/// Whether a Swift `func`, `let` or `var` line is `static` or `class`; `None` for any other line.
pub fn swift_static_member(line: &str) -> Option<bool> {
    static DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(r"({})(?:func|let|var)\s", swift_mods!())).unwrap()
    });
    let c = DECL.captures(line)?;
    Some(
        c[1].split(|ch: char| !ch.is_alphanumeric())
            .any(|w| w == "static" || w == "class"),
    )
}
/// Whether a Swift line declares an enum case.
pub fn swift_case(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("case ") || t.starts_with("indirect case ")
}
/// Whether a Swift line is an `extension` (#371).
pub fn swift_extension(line: &str) -> bool {
    static EXTENSION: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(&format!(r"{}extension\s", swift_mods!())).unwrap());
    EXTENSION.is_match(line)
}
/// The function whose local the `let` or `var` on 1-based `line` of a Swift file is, as
/// [`swift_scope`] names it; `None` for a member, a global, or no `let` or `var` at all.
pub fn swift_local<S: AsRef<str>>(lines: &[S], literal: &[bool], line: usize) -> Option<usize> {
    let text = lines.get(line.checked_sub(1)?)?.as_ref();
    let (scope, local) = swift_scope(lines, literal, line);
    (local && SWIFT_DECL.is_match(&uncommented(Kind::Swift, text))).then_some(scope)
}
/// Whether a Swift pattern binds `name`: `x`, `(a, b)`, `.cut(n)`, `.x(label: a)`, `x as T`. A
/// `let` or a `var` in front binds every name in it, as a `for` does (`bind_all`); otherwise only
/// the names behind a `let` of their own do, as in `.x(let a, b)`. `None` for a pattern the rules
/// cannot read that mentions `name`: a nested tuple or enum pattern.
fn swift_pattern_binds(pattern: &str, bind_all: bool, name: &str) -> Option<bool> {
    let p = pattern.trim();
    let p = p.split(" where ").next().unwrap_or(p).trim();
    let (p, bind_all) = match p.strip_prefix("let ").or_else(|| p.strip_prefix("var ")) {
        Some(rest) => (rest.trim(), true),
        None => (p, bind_all),
    };
    let p = p.split(" as ").next().unwrap_or(p).trim();
    let bare = |n: &str| n.trim().trim_matches('`') == name;
    let Some(open) = p.find('(') else {
        return Some(bind_all && bare(p));
    };
    let inner = p[open + 1..].trim_end();
    let inner = inner.strip_suffix(')').unwrap_or(inner);
    if inner.contains(['(', '[']) {
        return (!names(inner, name)).then_some(false);
    }
    Some(split_top(Kind::Swift, inner, b',').iter().any(|e| {
        // `label: a`, `label: let a`
        let e = e.rsplit(':').next().unwrap_or(e).trim();
        match e.strip_prefix("let ").or_else(|| e.strip_prefix("var ")) {
            Some(n) => bare(n),
            None => bind_all && bare(e),
        }
    }))
}
/// Whether the clauses of an `if`, a `while` or a `guard` condition, `text` from the keyword on,
/// bind `name`: `let x = …`, `var x: T = …`, `case let .x(a) = …`. The shorthand `let x` rebinds
/// an outer `x` and binds nothing new.
fn swift_condition_binds(text: &str, name: &str) -> Option<bool> {
    static KEYWORD: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\b(?:if|while|guard)\s").unwrap());
    let Some(m) = KEYWORD.find(text) else {
        return Some(false);
    };
    let rest = text[m.end()..].trim_end().trim_end_matches('{');
    let rest = match m.as_str().starts_with("guard") {
        true => rest.rsplit_once(" else").map_or(rest, |(c, _)| c),
        false => rest,
    };
    let mut bound = Some(false);
    for clause in split_top(Kind::Swift, rest, b',') {
        let c = clause.trim();
        let (pattern, bind_all) = match c.strip_prefix("case ") {
            Some(p) => (p, false),
            None if c.starts_with("let ") || c.starts_with("var ") => (c, true),
            None => continue,
        };
        let parts = split_top(Kind::Swift, pattern, b'=');
        if parts.len() < 2 {
            continue;
        }
        match swift_pattern_binds(split_top(Kind::Swift, parts[0], b':')[0], bind_all, name) {
            Some(true) => return Some(true),
            None => bound = None,
            Some(false) => {}
        }
    }
    bound
}
/// Whether a `let` or a `var` statement, `rest` past its keyword, binds `name`: `a = 1`, `b: T`,
/// `a = 1, b = 2`, `(a, b) = t`.
fn swift_decl_binds(rest: &str, name: &str) -> Option<bool> {
    let mut bound = Some(false);
    for part in split_top(Kind::Swift, rest, b',') {
        let pattern = split_top(Kind::Swift, split_top(Kind::Swift, part, b'=')[0], b':')[0];
        match swift_pattern_binds(pattern, true, name) {
            Some(true) => return Some(true),
            None => bound = None,
            Some(false) => {}
        }
    }
    bound
}
/// Swift's locals (#366), walked outward from the cursor over the blocks around it, told by
/// indentation: a `let`, a `var` or a `guard` statement above the cursor in each block; what the
/// block's header binds — an `if let`, a `while let`, a `for`, a `catch` (a bare one binds
/// `error`), a closure's parameters, a `case` of a `switch`, the parameters of a `func`, `init` or
/// `subscript`, and on out past a nested function's header (#564). The innermost wins. A type's body and the top of the
/// file bind no local, and a pattern the rules cannot read that names the word stops the walk:
/// the search by name decides then.
pub(super) fn swift_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    swift_walk(lines, at, name).unwrap_or_default()
}
/// Whether the walk of [`swift_bindings`] from 1-based `line` proves that no local of the
/// enclosing functions names `name`: it reached a type's body and found no binding. A walk that
/// stopped proves nothing (#380).
pub fn swift_no_local(text: &str, line: usize, name: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    line.checked_sub(1)
        .filter(|&at| at < lines.len())
        .is_some_and(|at| swift_walk(&lines, at, name).is_some_and(|b| b.is_empty()))
}
/// The walk of [`swift_bindings`]: `None` where it stopped, where the search by name decides.
fn swift_walk(lines: &[&str], at: usize, name: &str) -> Option<Vec<Binding>> {
    let literal = literal_lines(Kind::Swift, &lines.join("\n"));
    let code = |i: usize| uncommented(Kind::Swift, lines[i]).trim().to_owned();
    let found = |line: usize| {
        Some(vec![Binding {
            line,
            value: Value::Unknown,
        }])
    };
    // The 1-based line of `start..=end` that writes the name, or `start`'s.
    let written = |start: usize, end: usize, re: &Regex| {
        (start..=end)
            .find(|&j| re.is_match(&code(j)))
            .unwrap_or(start)
            + 1
    };
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let (word, param) = (
        rule(format!(r"(?:^|[^\w.]){n}\b")),
        rule(format!(r"\b{n}\s*:")),
    );
    let mut depth = indent(lines[at]);
    // The nearest binding among the statements of the block the walk is in: it counts once the
    // block's header says the block is no type's body.
    let mut pending = None;
    let mut i = at;
    while i > 0 && depth > 0 {
        i -= 1;
        if !swift_code(lines, &literal, i) || indent(lines[i]) > depth {
            continue;
        }
        let t = code(i);
        if indent(lines[i]) == depth {
            if pending.is_some() {
                continue;
            }
            let (binds, line) = if let Some(c) = SWIFT_DECL.captures(&t) {
                (swift_decl_binds(&c[1], name), i + 1)
            } else if t.starts_with("guard ") {
                // A `guard` wrapped over lines goes on to its `else`.
                let end = (i..at).find(|&j| code(j).contains("else")).unwrap_or(i);
                let text: Vec<String> = (i..=end).map(code).collect();
                (
                    swift_condition_binds(&text.join(" "), name),
                    written(i, end, &word),
                )
            } else {
                (Some(false), 0)
            };
            match binds {
                None => return None,
                Some(true) => pending = Some(line),
                Some(false) => {}
            }
            continue;
        }
        // The header of a block around the cursor, over the lines it wraps.
        depth = indent(lines[i]);
        let end = i;
        i = swift_header_start(lines, &literal, i);
        let head = code(i);
        let text = (i..=end).map(code).collect::<Vec<_>>().join(" ");
        if SWIFT_FUNC.is_match(&head) {
            if let Some(line) = pending {
                return found(line);
            }
            // Parameters wrapped over the lines under the header's first go on to its `{`.
            let end = (end..at).find(|&j| code(j).ends_with('{')).unwrap_or(end);
            let text = (i..=end).map(code).collect::<Vec<_>>().join(" ");
            if swift_params_bind(&text, name) {
                return found(written(i, end, &param));
            }
            // A nested function reads the bindings of the function around it above its header,
            // as a closure does (#564): the walk goes on out, and a type's body stops it.
            continue;
        }
        if SWIFT_TYPE.is_match(&head) {
            return Some(Vec::new());
        }
        if let Some(line) = pending {
            return found(line);
        }
        match swift_header_binds(&head, &text, &code(end), name) {
            None => return None,
            // A `case` names its line, the others the line of the header that writes the name.
            Some(true) if head.starts_with("case ") => return found(i + 1),
            Some(true) => return found(written(i, end, &word)),
            Some(false) => {}
        }
    }
    None
}
/// Whether the parameters of the `func`, `init`, `subscript` or `set` header `text` name `name`:
/// the last word in front of a parameter's `:`, so `_ attempt: Int` and `for attempt: Int` bind
/// `attempt`, and an argument label binds nothing.
fn swift_params_bind(text: &str, name: &str) -> bool {
    static PARAMS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(r"{}(?:func|init|subscript|set)\b", swift_mods!())).unwrap()
    });
    let mods = PARAMS.find(text).map_or(text.len(), |m| m.end());
    let params = text[mods..].find('(').and_then(|open| {
        let open = mods + open;
        close_of(Kind::Swift, text, open).map(|close| &text[open + 1..close - 1])
    });
    params.is_some_and(|p| {
        split_top(Kind::Swift, p, b',').iter().any(|p| {
            let names = split_top(Kind::Swift, p, b':')[0];
            names
                .split_whitespace()
                .last()
                .is_some_and(|w| w.trim_matches('`') == name)
        })
    })
}
/// Whether the header of a Swift block binds `name` for the block: `head` its first line, `text`
/// all its lines joined, `last` the line that opens the block. A `catch` (a bare one binds
/// `error`), a `case` of a `switch`, a `for`, an `if let` or a `while let`, a closure's
/// parameters; `None` for a pattern naming the word that the rules cannot read.
fn swift_header_binds(head: &str, text: &str, last: &str, name: &str) -> Option<bool> {
    static CATCH: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(?:\}\s*)?catch\b\s*(.*?)\s*\{$").unwrap());
    static CONDITION: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(?:\}\s*)?(?:else\s+)?(?:if|while)\s").unwrap());
    static FOR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^for\s+(?:try\s+)?(?:await\s+)?(?:case\s+)?(.+?)\s+in\s").unwrap()
    });
    static CLOSURE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"\{\s*(?:\[[^\]]*\]\s*)?(?:@\w+\s+)?(\([^()]*\)|[\w\s,]*?)\s*(?:async\s+)?(?:throws\s+)?(?:->\s*[^{}]+?)?\s*\bin$",
        )
        .unwrap()
    });
    if let Some(c) = CATCH.captures(head) {
        let n = regex::escape(name);
        return match &c[1] {
            "" => Some(name == "error"),
            pattern => Some(
                Regex::new(&format!(r"\b(?:let|var)\s+{n}\b"))
                    .expect("an escaped name keeps the pattern valid")
                    .is_match(pattern),
            ),
        };
    }
    if let Some(pattern) = head.strip_prefix("case ") {
        let pattern = pattern.strip_suffix(':').unwrap_or(pattern);
        let mut bound = Some(false);
        for p in split_top(Kind::Swift, pattern, b',') {
            match swift_pattern_binds(p, false, name) {
                Some(true) => return Some(true),
                None => bound = None,
                Some(false) => {}
            }
        }
        return bound;
    }
    if let Some(c) = FOR.captures(text) {
        swift_pattern_binds(&c[1], true, name)
    } else if CONDITION.is_match(head) {
        swift_condition_binds(text, name)
    } else if let Some(c) = CLOSURE.captures(last) {
        let params = c[1].trim().trim_start_matches('(').trim_end_matches(')');
        Some(params.split(',').any(|p| {
            let p = p.split(':').next().unwrap_or(p);
            p.split_whitespace().last() == Some(name)
        }))
    } else {
        Some(false)
    }
}
/// Whether a Swift line, read on its own, binds `name` for what follows it the way no
/// declaration pattern reads (#525): a `for`, an `if let`, a `guard let`, a closure's parameter,
/// a `catch let`, a `case let` of a `switch`; and a function's parameter in its header (#533).
pub fn swift_binds_on(line: &str, name: &str) -> bool {
    let t = uncommented(Kind::Swift, line).trim().to_owned();
    if SWIFT_FUNC.is_match(&t) && swift_params_bind(&t, name) {
        return true;
    }
    let binds = match t.starts_with("guard ") {
        true => swift_condition_binds(&t, name),
        false => swift_header_binds(&t, &t, &t, name),
    };
    binds == Some(true)
}
// ---- Swift's receiver types (#384) ------------------------------------------------------------
/// What a Swift line that binds a name gives it, as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwiftGiven {
    /// An annotation: `lhs: Instant`, `let encoder: FormEncoder`, `var cache: Cache?`.
    Type(String),
    /// A value: `let printer = FormEncoder()`, `if let s = self.session`.
    Value(String),
    /// An element of the collection written: `for x in xs`.
    Element(String),
}
/// What the Swift line `line` gives `name`, which it binds: a `for` over a collection, else the
/// annotation or the value behind the name's first `:` or `=`. `None` for anything else: a
/// pattern, a closure's parameter with no type, a `catch`.
pub fn swift_given(line: &str, name: &str) -> Option<SwiftGiven> {
    let t = uncommented(Kind::Swift, line);
    let t = t.trim();
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let each = rule(format!(
        r"^for\s+(?:case\s+)?(?:(?:let|var)\s+)?`?{n}`?\s+in\s+(.+?)\s*(?:where\s.*)?\{{?\s*$"
    ));
    if let Some(c) = each.captures(t) {
        return Some(SwiftGiven::Element(c[1].trim().to_owned()));
    }
    let at = rule(format!(r"(?:^|[^\w.])`?{n}`?\s*([:=])"));
    let c = at
        .captures_iter(t)
        .find(|c| !t[c.get(0).unwrap().end()..].starts_with([':', '=']))?;
    let rest = &t[c.get(0).unwrap().end()..];
    if &c[1] == ":" {
        // The annotation runs to a `,`, `)`, `=` or `{` outside its own brackets.
        let mut depth = 0i32;
        let mut end = rest.len();
        for (i, ch) in rest.char_indices() {
            match ch {
                '(' | '[' | '<' => depth += 1,
                ')' | ']' | '>' if depth > 0 => depth -= 1,
                ')' | ',' | '=' | '{' if depth == 0 => {
                    end = i;
                    break;
                }
                _ => {}
            }
        }
        let ty = rest[..end].trim();
        return (!ty.is_empty()).then(|| SwiftGiven::Type(ty.to_owned()));
    }
    // The `{` that opens an `if`'s or a `guard`'s body, not a trailing closure.
    let rest = rest.trim();
    let rest = match ["if ", "guard ", "while ", "} else if "]
        .iter()
        .any(|k| t.starts_with(k))
    {
        true => rest
            .strip_suffix('{')
            .map_or(rest, str::trim_end)
            .trim_end_matches(" else")
            .trim_end(),
        false => rest,
    };
    let value = split_top(Kind::Swift, rest, b',')[0].trim();
    (!value.is_empty()).then(|| SwiftGiven::Value(value.to_owned()))
}
/// The type a Swift annotation names, when it names one type (#384): `Instant`, `Cache?` and
/// `T!` read as `T`, `Box<Int>` as `Box`. `None` for `any P`, `some P`, a tuple, a closure, a
/// collection, a composition, a path, `Self` and `Any`.
pub fn swift_type_name(written: &str) -> Option<String> {
    let mut t = written.trim();
    t = t.strip_prefix("inout ").unwrap_or(t).trim_start();
    while let Some(rest) = t.strip_prefix('@') {
        t = rest.split_once(' ')?.1.trim_start();
    }
    let t = t.trim_end_matches(['?', '!']);
    let t = match t.find('<') {
        Some(open) if t.ends_with('>') => &t[..open],
        Some(_) => return None,
        None => t,
    };
    (t.starts_with(|c: char| c.is_alphabetic() || c == '_')
        && t.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !matches!(t, "Self" | "Any" | "AnyObject" | "any" | "some"))
    .then(|| t.to_owned())
}
/// The element of the Swift collection type written as `written`: `[T]`, `[T]?`, `Array<T>`,
/// `Set<T>`; `None` for a dictionary and anything else.
pub fn swift_element(written: &str) -> Option<String> {
    let t = written.trim().trim_end_matches(['?', '!']);
    let inner = match t.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        Some(inner) => inner,
        None => ["Array<", "Set<", "ContiguousArray<", "ArraySlice<"]
            .iter()
            .find_map(|p| t.strip_prefix(p))?
            .strip_suffix('>')?,
    };
    (split_top(Kind::Swift, inner, b':').len() == 1).then(|| inner.trim().to_owned())
}
/// The generic parameters a Swift header line declares: `T` and `U` of `func f<T, U: P>(`.
pub fn swift_generics(line: &str) -> Vec<String> {
    static GENERICS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"(?:\bfunc\s+`?[^\s(<]+`?|\binit[?!]?|\bsubscript|\b(?:class|struct|enum|actor|typealias)\s+`?\w+`?)\s*<([^>]*)>",
        )
        .unwrap()
    });
    GENERICS
        .captures(line)
        .map(|c| {
            c[1].split(',')
                .filter_map(|p| p.split(':').next())
                .map(|p| p.trim().to_owned())
                .filter(|p| !p.is_empty())
                .collect()
        })
        .unwrap_or_default()
}
/// The return type the Swift function declared on 1-based `line` of `text` writes, `-> T`, over a
/// header wrapped to its `{`.
pub fn swift_returns(text: &str, line: usize) -> Option<String> {
    let mut header = String::new();
    for l in text.lines().skip(line.checked_sub(1)?).take(8) {
        let l = uncommented(Kind::Swift, l);
        match l.find('{') {
            Some(open) => {
                header.push_str(&l[..open]);
                break;
            }
            None => header.push_str(&l),
        }
        header.push(' ');
    }
    // The `->` outside the parameters, which a closure parameter's type may hold too.
    let mut depth = 0i32;
    let arrow = header.char_indices().find_map(|(i, c)| {
        match c {
            '(' | '[' | '<' => depth += 1,
            ')' | ']' => depth -= 1,
            '>' if depth > 0 && !header[..i].ends_with('-') => depth -= 1,
            '-' if depth == 0 && header[i..].starts_with("->") => return Some(i),
            _ => {}
        }
        None
    })?;
    let ret = header[arrow + 2..].trim();
    let ret = ret.split(" where ").next().unwrap_or(ret).trim();
    (!ret.is_empty()).then(|| ret.to_owned())
}
/// A Swift expression as the receiver rules read it (#384).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwiftExpr {
    /// A call, a construction among them, of the callee as written: `FormEncoder`,
    /// `FormEncoder.init`, `session.request`.
    Call(String),
    /// Names alone: `self.session`, `encoder`.
    Chain(String),
    /// A cast's type: `x as! Session`.
    Cast(String),
}
/// What the Swift expression `e` is, when the rules read it: a call with nothing after it but a
/// trailing closure, a chain of names, or a cast; `try`, `try?`, `try!` and `await` in front are
/// read past.
pub fn swift_expr(e: &str) -> Option<SwiftExpr> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)\s*(?:<[^()]*>)?\s*([({])?").unwrap()
    });
    static CAST: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\sas[!?]?\s+([\w<>\[\]?]+)$").unwrap());
    let mut e = e.trim();
    loop {
        let rest = ["try? ", "try! ", "try ", "await "]
            .iter()
            .find_map(|p| e.strip_prefix(p));
        match rest {
            Some(rest) => e = rest.trim_start(),
            None => break,
        }
    }
    if let Some(c) = CAST.captures(e) {
        return Some(SwiftExpr::Cast(c[1].to_owned()));
    }
    let c = CALL.captures(e)?;
    let callee = c[1].to_owned();
    let Some(open) = c.get(2) else {
        return (c.get(0).unwrap().end() == e.len()).then_some(SwiftExpr::Chain(callee));
    };
    // What closes on the line ends it, save a trailing closure; one that wraps is read as is.
    let mut at = open.start();
    while let Some(end) = close_of(Kind::Swift, e, at) {
        let rest = e[end..].trim_start();
        if rest.is_empty() {
            break;
        }
        if !rest.starts_with('{') || e.as_bytes()[at] == b'{' {
            return None;
        }
        at = e.len() - rest.len();
    }
    Some(SwiftExpr::Call(callee))
}
/// Whether a Swift line binds `name` in a way [`swift_given`] or the walk over the scopes may not
/// read: behind a `let` or a `var` before any `=`, as a closure's or a `for`'s name.
pub fn swift_may_bind(line: &str, name: &str) -> bool {
    let n = regex::escape(name);
    Regex::new(&format!(
        r"\b(?:let|var)\s[^=]*\b{n}\b|\{{[^{{}}]*\b{n}\b[^{{}}]*\bin\b|\bfor\b[^{{]*\b{n}\b[^{{]*\bin\b"
    ))
    .expect("an escaped name keeps the pattern valid")
    .is_match(&uncommented(Kind::Swift, line))
}
