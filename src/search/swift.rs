//! Swift's scopes (#371, #366): the blocks and type bodies around a line, and the names a
//! line binds.

use std::path::{Path, PathBuf};

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
static SWIFT_LOCAL_DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(&format!(
        r"{}(?:func|class|struct|enum|actor|typealias)\s+`?(\w+)",
        swift_mods!()
    ))
    .unwrap()
});
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
pub fn swift_local_decl(line: &str, name: &str) -> bool {
    SWIFT_LOCAL_DECL
        .captures(&uncommented(Kind::Swift, line))
        .is_some_and(|c| &c[1] == name)
}
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
/// A Swift type's header (#380).
pub fn swift_type_header(line: &str) -> Option<SwiftTypeHeader> {
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
    Some(SwiftTypeHeader {
        keyword: c[1].to_owned(),
        name: c[2].to_owned(),
        first_inherited: c.get(3).map(|m| m.as_str().to_owned()),
        where_constrained: t.contains(" where "),
    })
}
pub struct SwiftTypeHeader {
    pub keyword: String,
    pub name: String,
    pub first_inherited: Option<String>,
    pub where_constrained: bool,
}
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
pub fn swift_case(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("case ") || t.starts_with("indirect case ")
}
pub fn swift_extension(line: &str) -> bool {
    static EXTENSION: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(&format!(r"{}extension\s", swift_mods!())).unwrap());
    EXTENSION.is_match(line)
}
/// The function whose local the `let` or `var` is, as [`swift_scope`] names it; `None` for a
/// member, a global, or no `let` or `var` at all.
pub fn swift_local<S: AsRef<str>>(lines: &[S], literal: &[bool], line1: usize) -> Option<usize> {
    let text = lines.get(line1.checked_sub(1)?)?.as_ref();
    let (scope, local) = swift_scope(lines, literal, line1);
    (local && SWIFT_DECL.is_match(&uncommented(Kind::Swift, text))).then_some(scope)
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Binds {
    Name,
    Nothing,
    UnreadablePatternMentionsName,
}
impl Binds {
    fn name_if(binds: bool) -> Self {
        match binds {
            true => Binds::Name,
            false => Binds::Nothing,
        }
    }
    fn first_of(each: impl IntoIterator<Item = Binds>) -> Self {
        let mut out = Binds::Nothing;
        for b in each {
            match b {
                Binds::Name => return Binds::Name,
                Binds::UnreadablePatternMentionsName => out = b,
                Binds::Nothing => {}
            }
        }
        out
    }
}
/// `x`, `(a, b)`, `.cut(n)`, `.x(label: a)`, `x as T`. A `let` or a `var` in front binds every
/// name in it, as a `for` does (`bind_all`); otherwise only the names behind a `let` of their own
/// do, as in `.x(let a, b)`. A nested tuple or enum pattern is unreadable.
fn swift_pattern_binds(pattern: &str, bind_all: bool, name: &str) -> Binds {
    let p = pattern.trim();
    let p = p.split(" where ").next().unwrap_or(p).trim();
    let (p, bind_all) = match p.strip_prefix("let ").or_else(|| p.strip_prefix("var ")) {
        Some(rest) => (rest.trim(), true),
        None => (p, bind_all),
    };
    let p = p.split(" as ").next().unwrap_or(p).trim();
    let bare = |n: &str| n.trim().trim_matches('`') == name;
    let Some(open) = p.find('(') else {
        return Binds::name_if(bind_all && bare(p));
    };
    let inner = p[open + 1..].trim_end();
    let inner = inner.strip_suffix(')').unwrap_or(inner);
    if inner.contains(['(', '[']) {
        return match names(inner, name) {
            true => Binds::UnreadablePatternMentionsName,
            false => Binds::Nothing,
        };
    }
    Binds::name_if(split_top(Kind::Swift, inner, b',').iter().any(|e| {
        let past_label = e.rsplit(':').next().unwrap_or(e).trim();
        match past_label
            .strip_prefix("let ")
            .or_else(|| past_label.strip_prefix("var "))
        {
            Some(n) => bare(n),
            None => bind_all && bare(past_label),
        }
    }))
}
/// `let x = …`, `var x: T = …`, `case let .x(a) = …`. The shorthand `let x` rebinds an outer `x`
/// and binds nothing new.
fn swift_condition_binds(from_keyword: &str, name: &str) -> Binds {
    static KEYWORD: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\b(?:if|while|guard)\s").unwrap());
    let Some(m) = KEYWORD.find(from_keyword) else {
        return Binds::Nothing;
    };
    let rest = from_keyword[m.end()..].trim_end().trim_end_matches('{');
    let rest = match m.as_str().starts_with("guard") {
        true => rest.rsplit_once(" else").map_or(rest, |(c, _)| c),
        false => rest,
    };
    Binds::first_of(
        split_top(Kind::Swift, rest, b',')
            .into_iter()
            .filter_map(|clause| {
                let c = clause.trim();
                let (pattern, bind_all) = match c.strip_prefix("case ") {
                    Some(p) => (p, false),
                    None if c.starts_with("let ") || c.starts_with("var ") => (c, true),
                    None => return None,
                };
                let parts = split_top(Kind::Swift, pattern, b'=');
                (parts.len() >= 2).then(|| {
                    swift_pattern_binds(split_top(Kind::Swift, parts[0], b':')[0], bind_all, name)
                })
            }),
    )
}
/// `a = 1`, `b: T`, `a = 1, b = 2`, `(a, b) = t`.
fn swift_decl_binds(past_keyword: &str, name: &str) -> Binds {
    Binds::first_of(
        split_top(Kind::Swift, past_keyword, b',')
            .into_iter()
            .map(|part| {
                let pattern =
                    split_top(Kind::Swift, split_top(Kind::Swift, part, b'=')[0], b':')[0];
                swift_pattern_binds(pattern, true, name)
            }),
    )
}
pub(super) fn swift_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    swift_walk(lines, at, name).unwrap_or_default()
}
pub fn swift_no_local(text: &str, line: usize, name: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    line.checked_sub(1)
        .filter(|&at| at < lines.len())
        .is_some_and(|at| swift_walk(&lines, at, name).is_some_and(|b| b.is_empty()))
}
fn swift_walk(lines: &[&str], at: usize, name: &str) -> Option<Vec<Binding>> {
    let literal = literal_lines(Kind::Swift, &lines.join("\n"));
    let code = |i: usize| uncommented(Kind::Swift, lines[i]).trim().to_owned();
    let found = |line: usize| {
        Some(vec![Binding {
            line,
            value: Value::Unknown,
        }])
    };
    let line1_writing_name_or_start = |start: usize, end: usize, re: &Regex| {
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
    let generic = |text: &str| swift_generics(text).iter().any(|g| g == name);
    let decls = |open: usize, depth: usize| {
        let lines: Vec<usize> = (open + 1..lines.len())
            .filter(|&j| swift_code(lines, &literal, j))
            .take_while(|&j| indent(lines[j]) >= depth)
            .filter(|&j| indent(lines[j]) == depth && swift_local_decl(lines[j], name))
            .collect();
        let bindings = lines.iter().map(|&j| Binding {
            line: j + 1,
            value: Value::Unknown,
        });
        Some(bindings.collect::<Vec<_>>()).filter(|b| !b.is_empty() && !lines.contains(&at))
    };
    if generic(&code(at)) {
        return found(at + 1);
    }
    let mut depth = indent(lines[at]);
    let mut bound_in_block_unless_type_body = None;
    let mut only_type_header_left = false;
    let mut unproven = false;
    let mut i = at;
    while i > 0 && depth > 0 {
        i -= 1;
        if !swift_code(lines, &literal, i) || indent(lines[i]) > depth {
            continue;
        }
        let t = code(i);
        if indent(lines[i]) == depth {
            if bound_in_block_unless_type_body.is_some() || only_type_header_left {
                continue;
            }
            let (binds, line) = if let Some(c) = SWIFT_DECL.captures(&t) {
                (swift_decl_binds(&c[1], name), i + 1)
            } else if t.starts_with("guard ") {
                let guard_end = (i..at).find(|&j| code(j).contains("else")).unwrap_or(i);
                let text: Vec<String> = (i..=guard_end).map(code).collect();
                (
                    swift_condition_binds(&text.join(" "), name),
                    line1_writing_name_or_start(i, guard_end, &word),
                )
            } else {
                (Binds::Nothing, 0)
            };
            match binds {
                Binds::UnreadablePatternMentionsName => return None,
                Binds::Name => bound_in_block_unless_type_body = Some(line),
                Binds::Nothing => {}
            }
            continue;
        }
        let body = depth;
        depth = indent(lines[i]);
        let end = i;
        let local = decls(end, body);
        i = swift_header_start(lines, &literal, i);
        let head = code(i);
        let text = (i..=end).map(code).collect::<Vec<_>>().join(" ");
        if only_type_header_left {
            return match generic(&text) {
                true => found(i + 1),
                false => (!unproven).then(Vec::new),
            };
        }
        if SWIFT_FUNC.is_match(&head) {
            if let Some(line) = bound_in_block_unless_type_body {
                return found(line);
            }
            if local.is_some() {
                return local;
            }
            let params_end = (end..at).find(|&j| code(j).ends_with('{')).unwrap_or(end);
            let text = (i..=params_end).map(code).collect::<Vec<_>>().join(" ");
            if swift_params_bind(&text, name) {
                return found(line1_writing_name_or_start(i, params_end, &param));
            }
            if generic(&text) {
                return found(i + 1);
            }
            only_type_header_left = (0..i)
                .rev()
                .find(|&j| swift_code(lines, &literal, j) && indent(lines[j]) < indent(lines[i]))
                .map(|j| code(swift_header_start(lines, &literal, j)))
                .is_some_and(|h| !SWIFT_FUNC.is_match(&h) && SWIFT_TYPE.is_match(&h));
            unproven |= !only_type_header_left;
            continue;
        }
        // A type's generic parameters bind inside it; an outer type's, which a member of this one
        // may shadow, are left to the search by name.
        if SWIFT_TYPE.is_match(&head) {
            return match generic(&text) {
                true => found(i + 1),
                false => (!unproven).then(Vec::new),
            };
        }
        if let Some(line) = bound_in_block_unless_type_body {
            return found(line);
        }
        if local.is_some() && !head.starts_with("where ") {
            return local;
        }
        match swift_header_binds(&head, &text, &code(end), name) {
            Binds::UnreadablePatternMentionsName => return None,
            Binds::Name if head.starts_with("case ") => return found(i + 1),
            Binds::Name => return found(line1_writing_name_or_start(i, end, &word)),
            Binds::Nothing => {}
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
/// A `catch` (a bare one binds `error`), a `case` of a `switch`, a `for`, an `if let` or a
/// `while let`, a closure's parameters.
fn swift_header_binds(first_line: &str, joined: &str, opening_line: &str, name: &str) -> Binds {
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
    if let Some(c) = CATCH.captures(first_line) {
        let n = regex::escape(name);
        return Binds::name_if(match &c[1] {
            "" => name == "error",
            pattern => Regex::new(&format!(r"\b(?:let|var)\s+{n}\b"))
                .expect("an escaped name keeps the pattern valid")
                .is_match(pattern),
        });
    }
    if let Some(pattern) = first_line.strip_prefix("case ") {
        let pattern = pattern.strip_suffix(':').unwrap_or(pattern);
        return Binds::first_of(
            split_top(Kind::Swift, pattern, b',')
                .into_iter()
                .map(|p| swift_pattern_binds(p, false, name)),
        );
    }
    if let Some(c) = FOR.captures(joined) {
        swift_pattern_binds(&c[1], true, name)
    } else if CONDITION.is_match(first_line) {
        swift_condition_binds(joined, name)
    } else if let Some(c) = CLOSURE.captures(opening_line) {
        let params = c[1].trim().trim_start_matches('(').trim_end_matches(')');
        Binds::name_if(params.split(',').any(|p| {
            let p = p.split(':').next().unwrap_or(p);
            p.split_whitespace().last() == Some(name)
        }))
    } else {
        Binds::Nothing
    }
}
pub fn swift_binds_on(line: &str, name: &str) -> bool {
    let t = uncommented(Kind::Swift, line).trim().to_owned();
    if SWIFT_FUNC.is_match(&t) && swift_params_bind(&t, name) {
        return true;
    }
    let binds = match t.starts_with("guard ") {
        true => swift_condition_binds(&t, name),
        false => swift_header_binds(&t, &t, &t, name),
    };
    binds == Binds::Name
}
static SWIFT_TYPE_DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(&format!(
        r"{}(?:class|struct|enum|actor|protocol|typealias)\s+`?\w",
        swift_mods!()
    ))
    .unwrap()
});
/// Whether a Swift declaration line is `private` or `fileprivate`, seen in its own file alone;
/// `private(set)` limits the setter only.
pub fn swift_file_private(line: &str) -> bool {
    static DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"({})(?:func|let|var|class|struct|enum|actor|protocol|typealias|init|subscript)\b",
            swift_mods!()
        ))
        .unwrap()
    });
    DECL.captures(&uncommented(Kind::Swift, line))
        .is_some_and(|c| {
            c[1].split_whitespace()
                .any(|w| w == "private" || w == "fileprivate")
        })
}
/// The directories of the test targets of the Swift project at `root`, relative to it: the
/// `path:` of each `.testTarget(` of its `Package.swift`, else `Tests/<name>`; with no
/// `Package.swift` (an Xcode project), `Tests`. The manifest is read, never run.
pub fn swift_test_dirs(root: &Path) -> Vec<PathBuf> {
    let Ok(manifest) = std::fs::read_to_string(root.join("Package.swift")) else {
        return vec![PathBuf::from("Tests")];
    };
    static FIELD: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r#"\b(name|path)\s*:\s*"([^"]*)""#).unwrap());
    manifest
        .match_indices(".testTarget(")
        .filter_map(|(at, m)| {
            let open = at + m.len() - 1;
            let args = &manifest[open..close_of(Kind::Swift, &manifest, open)?];
            let field = |key: &str| {
                FIELD
                    .captures_iter(args)
                    .find(|c| &c[1] == key)
                    .map(|c| c[2].trim_start_matches("./").to_owned())
            };
            match field("path") {
                Some(path) => Some(PathBuf::from(path)),
                None => field("name").map(|n| Path::new("Tests").join(n)),
            }
        })
        .collect()
}
/// Whether a Swift line declares a type: a `class`, `struct`, `enum`, `actor`, `protocol` or
/// `typealias`, no `extension`.
pub fn swift_type_decl(line: &str) -> bool {
    SWIFT_TYPE_DECL.is_match(&uncommented(Kind::Swift, line))
}
/// Where a Swift type is declared, when that limits who sees it bare.
#[derive(Debug, PartialEq, Eq)]
pub enum SwiftTypePlace {
    /// In the body of the function whose header is on this 1-based line: nothing else sees it.
    Function(usize),
    /// In the body of the type of this name, `B` for `extension A.B`: a nested type.
    Nested(String),
}
/// `None` for a type at the top of the file, for a type in a closure there, which the rules cannot
/// place, and for any line that declares no type.
pub fn swift_type_place(text: &str, line1: usize) -> Option<SwiftTypePlace> {
    let lines: Vec<&str> = text.lines().collect();
    if !swift_type_decl(lines.get(line1.checked_sub(1)?)?) {
        return None;
    }
    let literal = literal_lines(Kind::Swift, text);
    let (scope, local) = swift_scope(&lines, &literal, line1);
    let header = uncommented(Kind::Swift, lines.get(scope.checked_sub(1)?)?);
    match local {
        true => SWIFT_FUNC
            .is_match(&header)
            .then_some(SwiftTypePlace::Function(scope)),
        // Only a class, struct, enum or actor is certainly nested: a protocol's `typealias` is seen
        // by every type that conforms to it.
        false
            if !swift_type_header(lines[line1 - 1]).is_some_and(|h| {
                matches!(h.keyword.as_str(), "class" | "struct" | "enum" | "actor")
            }) =>
        {
            None
        }
        false => swift_type_header(&header).and_then(|h| {
            Some(SwiftTypePlace::Nested(
                h.name.rsplit('.').next()?.to_owned(),
            ))
        }),
    }
}
pub fn swift_within<S: AsRef<str>>(
    lines: &[S],
    literal: &[bool],
    line1: usize,
    header_line1: usize,
) -> bool {
    let mut at = line1;
    loop {
        at = swift_scope(lines, literal, at).0;
        if at == header_line1 {
            return true;
        }
        if at == 0 {
            return false;
        }
    }
}
/// Inside `outer`'s body, an extension of it, or a class whose header names it first, its
/// superclass.
pub fn swift_sees_nested<S: AsRef<str>>(
    lines: &[S],
    literal: &[bool],
    line1: usize,
    outer: &str,
) -> bool {
    let mut at = line1;
    while let Some(header) = swift_enclosing_type(lines, literal, at) {
        if let Some(SwiftTypeHeader {
            keyword,
            name,
            first_inherited: base,
            ..
        }) = swift_type_header(lines[header - 1].as_ref())
            && (name.split('.').any(|n| n == outer)
                || (keyword == "class"
                    && base.is_some_and(|b| b.rsplit('.').next() == Some(outer))))
        {
            return true;
        }
        at = header;
    }
    false
}
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
        let ty = rest[..annotation_end(rest)].trim();
        return (!ty.is_empty()).then(|| SwiftGiven::Type(ty.to_owned()));
    }
    let rest = rest.trim();
    let brace_opens_condition_body = ["if ", "guard ", "while ", "} else if "]
        .iter()
        .any(|k| t.starts_with(k));
    let rest = match brace_opens_condition_body {
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
fn annotation_end(rest: &str) -> usize {
    let mut depth = 0i32;
    for (i, ch) in rest.char_indices() {
        match ch {
            '(' | '[' | '<' => depth += 1,
            ')' | ']' | '>' if depth > 0 => depth -= 1,
            ')' | ',' | '=' | '{' if depth == 0 => return i,
            _ => {}
        }
    }
    rest.len()
}
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
/// `-> T`, over a header wrapped to its `{`.
pub fn swift_returns(text: &str, line1: usize) -> Option<String> {
    let mut header = String::new();
    for l in text.lines().skip(line1.checked_sub(1)?).take(8) {
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
    let mut depth = 0i32;
    let arrow_outside_params = header.char_indices().find_map(|(i, c)| {
        match c {
            '(' | '[' | '<' => depth += 1,
            ')' | ']' => depth -= 1,
            '>' if depth > 0 && !header[..i].ends_with('-') => depth -= 1,
            '-' if depth == 0 && header[i..].starts_with("->") => return Some(i),
            _ => {}
        }
        None
    })?;
    let ret = header[arrow_outside_params + 2..].trim();
    let ret = ret.split(" where ").next().unwrap_or(ret).trim();
    (!ret.is_empty()).then(|| ret.to_owned())
}
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
