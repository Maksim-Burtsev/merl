//! Java and Kotlin scopes, read off the indentation: which block a declaration is seen in.

use regex::Regex;

use super::*;

/// Whether `line` opens a type's body: a class, an interface, an enum, a record, an annotation
/// type, a named `object` or a `companion object`, a Kotlin primary constructor `class X(`
/// included, and Scala's `trait`, `case class`, `package object` and the `extension (s: T)` the
/// methods under it are declared in (#416). An anonymous
/// `object :` or `new X() {` is no type here: what it holds is local.
fn opens_type(line: &str) -> bool {
    static TYPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            scala_mods!(),
            r"(?:(?:fun\s+)?interface|class|enum|record|@interface|trait\s+[A-Za-z_]\w*|(?:package\s+)?object\s+[A-Za-z_]\w*|companion\s+object",
            r"|extension\s*(?:\[[^\]]*\]\s*)?\(\s*\w+\s*:\s*\w+)\b"
        ))
        .unwrap()
    });
    TYPE.is_match(line)
}

/// The 0-based line of the scope around 0-based `at` of `lines`: the first line above indented
/// less, over blank lines, comments, annotations and the tails of wrapped headers (`) : Base {`,
/// `) {`), as [`enclosing_declarations`] walks. `None` in column 0.
fn scope_of(lines: &[&str], at: usize) -> Option<usize> {
    let depth = indent(lines[at]);
    (depth > 0).then_some(())?;
    (0..at).rev().find(|&i| {
        let t = lines[i].trim_start();
        !(steps_over(Some(Kind::Jvm), t)
            || t.starts_with(['@', ')', ']', '>', '{'])
            || indent(lines[i]) >= depth)
    })
}

/// `None` for a member, a constructor property or a top-level declaration (#357). A declaration
/// is a local when the scope around it opens no type: a function, a constructor, `init {`, an
/// `if`, a lambda, an anonymous object.
pub fn jvm_local_block(text: &str, line1: usize) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    let scope = scope_of(&lines, at)?;
    if opens_type(lines[scope]) {
        return None;
    }
    let depth = indent(lines[at]);
    let last_line1_of_block = (at + 1..lines.len())
        .find(|&i| {
            let t = lines[i].trim_start();
            !steps_over(Some(Kind::Jvm), t) && indent(lines[i]) < depth
        })
        .unwrap_or(lines.len());
    Some(last_line1_of_block)
}

/// Whether the modifiers in front of a Java or Kotlin declaration line include `private`: it is
/// seen in its own file only, whether Java's class or Kotlin's file or class keeps it (#357).
pub fn jvm_private(line: &str) -> bool {
    static MODS: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(jvm_mods!()).unwrap());
    MODS.find(line)
        .is_some_and(|m| m.as_str().split_whitespace().any(|w| w == "private"))
}

pub fn jvm_receiver(line: &str, name: &str) -> Option<String> {
    static EXTENSION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            jvm_mods!(),
            r"(?:fun|val|var)\s+(?:<[^>]*>\s*)?([\w.]+)(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?\??\.([A-Za-z_]\w*)",
            r"|^\s*extension\s*(?:\[[^\]]*\]\s*)?\(\s*\w+\s*:\s*([\w.]+)[^)]*\).*?\bdef\s+`?([A-Za-z_]\w*)"
        ))
        .unwrap()
    });
    let c = EXTENSION.captures(line)?;
    let (receiver, declared) = match c.get(1) {
        Some(r) => (r, &c[2]),
        None => (c.get(3)?, &c[4]),
    };
    (declared == name).then(|| receiver.as_str().to_owned())
}

pub fn jvm_receiver_at(text: &str, line: usize, name: &str) -> Option<String> {
    static BLOCK: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*extension\s*(?:\[[^\]]*\]\s*)?\(\s*\w+\s*:\s*([\w.]+)[^)]*\)\s*(?:\(\s*using\b[^)]*\)\s*)?(?://.*)?$").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let at = line.checked_sub(1).filter(|&i| i < lines.len())?;
    if let Some(receiver) = jvm_receiver(lines[at], name) {
        return Some(receiver);
    }
    let def = Regex::new(&format!(
        r"{}def\s+`?{}`?(?:[^\w`]|$)",
        scala_mods!(),
        regex::escape(name)
    ))
    .ok()?;
    if !def.is_match(lines[at]) {
        return None;
    }
    BLOCK
        .captures(lines[scope_of(&lines, at)?])
        .map(|c| c[1].to_owned())
}

/// The name `this` stands for, as [`qualified`] names a class: the innermost type whose body holds
/// the line (#362). `None` in column 0, and inside an anonymous `object :` or `new X() {`, whose
/// `this` has no name to look up.
pub fn jvm_this_owner(text: &str, line1: usize) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    loop {
        at = scope_of(&lines, at)?;
        if anonymous(lines[at]) {
            return None;
        }
        if opens_type(lines[at]) {
            let name = jvm_type_name(lines[at])?;
            return Some(qualified(Kind::Jvm, text, at + 1, &name).unwrap_or(name));
        }
    }
}

pub(super) fn scala_patterns(word: &str) -> Vec<String> {
    const RESERVED: &[&str] = &[
        "class",
        "object",
        "trait",
        "type",
        "def",
        "val",
        "var",
        "given",
        "case",
        "enum",
        "extends",
        "with",
        "new",
        "import",
        "package",
        "extension",
        "this",
        "super",
    ];
    let w = regex::escape(word);
    let n = match RESERVED.contains(&word) {
        true => format!("`{w}`"),
        false => format!("`?{w}`?"),
    };
    let mods = scala_mods!();
    let other_name_with_params = r"`?\w+`?(?:\([^)]*\))?";
    let ann = r"(?:@[\w.]+(?:\([^)]*\))?\s+)*";
    vec![
        format!(r"{mods}(?:class|trait|object|enum|type|package\s+object)\s+{n}(?:[^\w`]|$)"),
        // A method, and a Scala 3 extension's method written on the `extension` line.
        format!(r"(?:{mods}|^\s*extension\b.*?\b)def\s+{n}(?:[^\w`]|$)"),
        format!(r"{mods}(?:val|var)\s+{n}(?:[^\w.`]|$)"),
        format!(r"{mods}given\s+{n}{}", scala_given_tail!()),
        // An enum's case, alone or in a list, with its parameters or the parent it extends. What
        // follows the names is nothing, so a match case (`case Red | Green =>`) and Java's
        // `case RED:`, `case RED, GREEN:` and `case RED ->` are no declarations.
        format!(
            r"^\s*{ann}case\s+(?:{other_name_with_params}\s*,\s*)*{n}(?:\([^)]*\))?(?:\s*,\s*{other_name_with_params})*(?:\s+extends\s+[\w.\[\]]+(?:\([^)]*\))?)?\s*(?://.*)?$"
        ),
        // A field on its class's line: a `val` or a `var` among the parameters, and every
        // parameter of a `case class`. One on a line of its own is a method's parameter's shape.
        format!(
            r"{mods}class\s+`?\w+`?[^(]*\((?:.*[(,])?\s*{ann}(?:(?:private|protected|override|final|implicit)(?:\[\w+\])?\s+)*(?:val|var)\s+{n}\s*:"
        ),
        format!(
            r"^\s*{ann}(?:(?:final|sealed|private|protected)(?:\[\w+\])?\s+)*case\s+class\s+`?\w+`?[^(]*\((?:.*[(,])?\s*{ann}(?:(?:private|protected|override|final|implicit)(?:\[\w+\])?\s+)*(?:(?:val|var)\s+)?{n}\s*:"
        ),
    ]
}

/// The names one parameter or one lambda parameter of a Java or Kotlin list binds: `x` for
/// `x: Int`, `vararg x: Int`, `final String x`, `String... x`, `(a, x)`, `x`; none for `_` or
/// what reads as no parameter (a call's `a = b`, a lone type).
fn param_names(entry: &str, groovy: bool) -> Vec<String> {
    static KOTLIN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:@[\w.]+(?:\([^)]*\))?\s+)*(?:(?:vararg|noinline|crossinline|private|public|protected|internal|override|open|val|var|using|implicit)\s+)*([A-Za-z_]\w*)\s*:").unwrap()
    });
    static JAVA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:@[\w.]+(?:\([^)]*\))?\s+)*(?:final\s+)?[\w.<>\[\]?,\s]*?[\w>\]]\s*(?:\.\.\.)?\s+([A-Za-z_]\w*)$").unwrap()
    });
    let e = entry.split_whitespace().collect::<Vec<_>>().join(" ");
    let e = e.trim_matches(|c: char| c == '(' || c == ')' || c.is_whitespace());
    let typed = match groovy {
        true => e.split_once('=').map_or(e, |(p, _)| p).trim_end(),
        false => e,
    };
    let name = KOTLIN
        .captures(e)
        .or_else(|| {
            (!typed.contains(['=', ':']))
                .then(|| JAVA.captures(typed))
                .flatten()
        })
        .map(|c| c[1].to_owned())
        .or_else(|| (groovy && ident(typed)).then(|| typed.to_owned()));
    name.into_iter().filter(|n| n != "_").collect()
}

/// The names a lambda on `line` binds for the body after its `->`: Kotlin's `{ x ->`,
/// `{ a, x ->`, `{ (a, x) ->`, Java's `x ->`, `(a, x) ->`, `(Type x) ->`; Scala's `x =>`,
/// `(a, x: Int) =>`, and the lower-case names a `case` pattern binds, `case Some(x) =>` (#416).
fn lambda_names(line: &str) -> Vec<String> {
    static SCALA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[(,{=]|=>)\s*(?:\(([^()]*)\)|([A-Za-z_]\w*))\s*=>").unwrap()
    });
    static CASE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\bcase\s+(.+?)\s*(?:\bif\b.*?)?=>").unwrap());
    static BINDER: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r#"(?:^|[^\w.`"'])([a-z_]\w*)\s*([(.]?)"#).unwrap());
    static KOTLIN: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\{\s*([^{}]*?)\s*->").unwrap());
    static JAVA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[(,=]|->)\s*(?:\(([^()]*)\)|([A-Za-z_]\w*))\s*->").unwrap()
    });
    let mut out = Vec::new();
    for c in KOTLIN.captures_iter(line) {
        // `(a, x)` destructures; any other entry is a name, with or without its type.
        for entry in c[1].split(',') {
            let e = entry.trim().trim_matches(['(', ')']).trim();
            let name = e.split(':').next().unwrap_or("").trim();
            if ident(name) {
                out.push(name.to_owned());
            }
        }
    }
    for c in CASE.captures_iter(line) {
        for b in BINDER.captures_iter(&c[1]).filter(|b| b[2].is_empty()) {
            out.push(b[1].to_owned());
        }
    }
    for c in JAVA.captures_iter(line).chain(SCALA.captures_iter(line)) {
        match (c.get(1), c.get(2)) {
            (Some(list), _) => {
                for entry in list.as_str().split(',') {
                    let e = entry.trim();
                    out.extend(match e.contains(char::is_whitespace) {
                        true => param_names(e, false),
                        false => ident(e).then(|| e.to_owned()).into_iter().collect(),
                    });
                }
            }
            (None, Some(name)) => out.push(name.as_str().to_owned()),
            _ => {}
        }
    }
    out.retain(|n| n != "_");
    out
}
fn ident(s: &str) -> bool {
    s.starts_with(|c: char| c.is_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Kotlin's `fun` (past an extension's receiver) and `constructor`, Scala's `def`, a Java method
/// (told by the return type before its name) and a Java constructor. `None` for any other line, a
/// Kotlin class's primary constructor included: what it takes is a property or an argument of
/// its initializers, not a parameter of a body.
fn params_open_byte(line: &str) -> Option<usize> {
    static FUN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            r"\bfun\s+(?:<[^>]*>\s*)?(?:[\w.]+(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?\??\.)?[A-Za-z_]\w*\s*\(",
            r"|\bconstructor\s*\(",
            // Scala's method, past its type parameters (#416).
            r"|\bdef\s+`?[A-Za-z_]\w*`?\s*(?:\[[^\]]*\]\s*)?\("
        ))
        .unwrap()
    });
    static METHOD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            jvm_mods!(),
            r"(?:<[^>]*>\s*)?",
            jvm_return_type!(),
            r"\s+[A-Za-z_]\w*\s*\(",
            r"|",
            jvm_mods!(),
            r"[A-Z]\w*\s*\("
        ))
        .unwrap()
    });
    if opens_type(line) {
        return None;
    }
    FUN.find(line)
        .or_else(|| METHOD.find(line))
        .map(|m| m.end() - 1)
}

pub fn jvm_function(line: &str) -> Option<String> {
    let head = line[..params_open_byte(line)?].trim_end();
    let start = head
        .trim_end_matches(|c: char| c.is_alphanumeric() || c == '_')
        .len();
    Some(head[start..].to_owned())
}

fn param_lists_line1_binding(lines: &[&str], from: usize, name: &str) -> Option<usize> {
    static DEF: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\bdef\s+[A-Za-z_]\w*\s*\(").unwrap());
    let word = Regex::new(&format!(
        r"(?:^|[^\w$.]){}(?:[^\w$]|$)",
        regex::escape(name)
    ))
    .expect("an escaped name keeps the pattern valid");
    let groovy = DEF.is_match(lines[from]);
    let mut open = params_open_byte(lines[from])?;
    let mut at = from;
    while let Some(Group {
        inner_uncommented: inner,
        close_line: last,
        after_close: rest,
    }) = group(Kind::Jvm, lines, at, open)
    {
        if split_top(Kind::Jvm, &inner, b',')
            .iter()
            .any(|e| param_names(e, groovy).iter().any(|p| p == name))
        {
            return (from..=last)
                .find(|&i| word.is_match(&uncommented(Kind::Jvm, lines[i])))
                .map(|i| i + 1);
        }
        let scala_next_list = rest.trim_start();
        if !scala_next_list.starts_with('(') {
            return None;
        }
        (at, open) = (last, lines[last].len() - scala_next_list.len());
    }
    None
}

/// A function's or a constructor's parameters, a lambda's, a loop variable, a `catch` parameter, a
/// resource of `try (…)`, a pattern of `instanceof`.
fn header_lines1_binding(lines: &[&str], from: usize, to: usize, name: &str) -> Vec<usize> {
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let ret = jvm_return_type!();
    let clause = rule(format!(
        r"\bfor\s*\(\s*(?:(?:final\s+)?(?:var|val|{ret})\s+)?{n}\s*(?:[:=]|\s+in\b)|\bfor\s*\(\s*\([^)]*\b{n}\b[^)]*\)\s+in\b|\bcatch\s*\(\s*(?:final\s+)?[\w.|\s]+?\s+{n}\s*\)|\bcatch\s*\(\s*{n}\s*:|\btry\s*\(\s*(?:final\s+)?(?:var|{ret})\s+{n}\s*=|\binstanceof\s+(?:final\s+)?[\w.]+(?:<[^>]*>)?\s+{n}\b"
    ));
    let mut out: Vec<usize> = param_lists_line1_binding(lines, from, name)
        .into_iter()
        .collect();
    for (i, l) in lines.iter().enumerate().take(to + 1).skip(from) {
        let code = uncommented(Kind::Jvm, l);
        if clause.is_match(&code) || lambda_names(&code).iter().any(|p| p == name) {
            out.push(i + 1);
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Whether the statement `t`, trimmed, declares the local `name`: Java's `Type x =`, `Type x;`,
/// `var x =`, Kotlin's `val x`, `var x`, `val (a, x) =`.
fn declares_local(t: &str, name: &str) -> bool {
    let n = regex::escape(name);
    let ret = jvm_return_type!();
    Regex::new(&format!(
        r"^(?:(?:final|lateinit|const)\s+)*(?:(?:val|var|def)\s+{n}(?:[^\w.]|$)|(?:val|var)\s*\([^)]*\b{n}\b[^)]*\)\s*=|(?:@[\w.]+\s+)*(?:var|{ret})(?:\.\.\.)?\s+{n}\s*[=;,:])"
    ))
    .is_ok_and(|re| re.is_match(t))
}

/// The declarations of `name` that 0-based line `at` of Java or Kotlin `lines` reads as a local
/// (#376): walking out of the blocks around it, innermost first, the statements of each block
/// above the cursor, then what its header binds: a lambda's parameters, a loop's variable, a
/// function's or a constructor's parameters. The first block that declares the name answers. A
/// block is the lines indented deeper than the line that opens it, as [`enclosing_declarations`]
/// reads them; the walk stops at a type's body, whose members are [`jvm_members_of`]'s.
pub(super) fn jvm_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let binding = |line| Binding {
        line1: line,
        value: Value::Unknown,
    };
    if matches!(name, "this" | "super" | "it") {
        return Vec::new();
    }
    let literal = literal_lines(Kind::Jvm, &lines.join("\n"));
    let bound_on_cursor_line = header_lines1_binding(lines, at, at, name);
    let mut out: Vec<Binding> = bound_on_cursor_line.into_iter().map(binding).collect();
    let mut depth = indent(lines[at]);
    let mut i = at;
    while out.is_empty() && depth > 0 && i > 0 {
        i -= 1;
        let t = uncommented(Kind::Jvm, lines[i]);
        let t = t.trim();
        let ind = indent(lines[i]);
        if t.is_empty() || comment(Kind::Jvm, t) || literal[i] || ind > depth {
            continue;
        }
        if ind == depth {
            let member_or_top_level = scope_of(lines, i).is_none_or(|s| opens_type(lines[s]));
            if declares_local(t, name) && !member_or_top_level {
                out.push(binding(i + 1));
            }
            continue;
        }
        let end = i;
        let closes_wrapped_header = t.starts_with([')', ']', '>']);
        if closes_wrapped_header {
            i = (0..i)
                .rev()
                .find(|&j| !lines[j].trim().is_empty() && indent(lines[j]) <= ind)
                .unwrap_or(i);
        }
        depth = indent(lines[i]);
        if opens_type(lines[i]) {
            break;
        }
        out.extend(
            header_lines1_binding(lines, i, end, name)
                .into_iter()
                .map(binding),
        );
    }
    out
}

/// Innermost first: a class, an interface, an object, an enum, a record, and an anonymous
/// `object :` or `new X() {`, which is a class for the lines inside it (#376).
pub fn jvm_enclosing_types(text: &str, line1: usize) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(mut at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    while let Some(s) = scope_of(&lines, at) {
        if opens_type(lines[s]) || anonymous(lines[s]) {
            out.push(s + 1);
        }
        at = s;
    }
    out
}

/// Whether `line` opens an anonymous class: Kotlin's `object :`, Java's `new X(…) {`.
fn anonymous(line: &str) -> bool {
    static ANONYMOUS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bobject\s*:|\bnew\s+[\w.]+\s*(?:<.*>)?\s*\(.*\)\s*\{\s*$").unwrap()
    });
    ANONYMOUS.is_match(line)
}

/// The name the type declared on `line` gives itself; `None` for an anonymous class and a
/// `companion object` with no name.
pub fn jvm_type_name(line: &str) -> Option<String> {
    static NAME: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:class|interface|enum|record|object|trait)\s+([A-Za-z_]\w*)").unwrap()
    });
    (opens_type(line) && !anonymous(line))
        .then(|| NAME.captures(line).map(|c| c[1].to_owned()))
        .flatten()
}

/// The lines one level inside the type's body that a `Kind::Jvm` pattern matches, a property of
/// its primary constructor, and a member of its `companion object` (#376). What a nested type
/// declares is its own.
pub fn jvm_members_of(text: &str, decl_line1: usize, name: &str) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = decl_line1.checked_sub(1).filter(|&k| k < lines.len()) else {
        return Vec::new();
    };
    let Ok(re) = Regex::new(&def_patterns(Kind::Jvm, name).join("|")) else {
        return Vec::new();
    };
    let literal = literal_lines(Kind::Jvm, text);
    members_of(&lines, &literal, k, name, &re)
}

fn primary_constructor_property_on_header(lines: &[&str], k: usize, name: &str) -> bool {
    static PROPERTY: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\b(?:val|var)\s+([A-Za-z_]\w*)\s*:").unwrap());
    let Some(open) = lines[k].find('(') else {
        return false;
    };
    !anonymous(lines[k])
        && group(Kind::Jvm, lines, k, open).is_some_and(
            |Group {
                 inner_uncommented: inner,
                 ..
             }| {
                split_top(Kind::Jvm, &inner, b',')
                    .iter()
                    .any(|e| PROPERTY.captures(e).is_some_and(|c| &c[1] == name))
            },
        )
        && PROPERTY.captures_iter(lines[k]).any(|c| &c[1] == name)
}

fn members_of(lines: &[&str], literal: &[bool], k: usize, name: &str, re: &Regex) -> Vec<usize> {
    static COMPANION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(jvm_mods!(), r"companion\s+object\b")).unwrap()
    });
    static RECORD: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(concat!(jvm_mods!(), r"record\s+\w+")).unwrap());
    // A constructor is no member called like its class: the name is the type's.
    if jvm_type_name(lines[k]).as_deref() == Some(name) {
        return Vec::new();
    }
    let base = indent(lines[k]);
    let code = |i: usize| {
        let t = lines[i].trim_start();
        !literal.get(i).copied().unwrap_or(false) && !steps_over(Some(Kind::Jvm), t)
    };
    let wrapped_header_tail = |i: usize| lines[i].trim_start().starts_with([')', '{']);
    let body_end = (k + 1..lines.len())
        .find(|&i| code(i) && indent(lines[i]) <= base && !wrapped_header_tail(i))
        .unwrap_or(lines.len());
    let member_indent = (k + 1..body_end)
        .filter(|&i| code(i) && indent(lines[i]) > base)
        .map(|i| indent(lines[i]))
        .min();
    let mut out = Vec::new();
    if primary_constructor_property_on_header(lines, k, name) {
        out.push(k + 1);
    }
    if RECORD.is_match(lines[k]) {
        let record_header = lines.iter().enumerate().skip(k).take(40);
        for (i, l) in record_header {
            if re.is_match(l) {
                out.push(i + 1);
            }
            if l.contains('{') {
                break;
            }
        }
    }
    let Some(member_indent) = member_indent else {
        return out;
    };
    for i in (k + 1..body_end).filter(|&i| code(i) && indent(lines[i]) == member_indent) {
        if re.is_match(lines[i]) {
            out.push(i + 1);
        } else if COMPANION.is_match(lines[i]) {
            out.extend(members_of(lines, literal, i, name, re));
        }
    }
    out
}

/// As simple names: Java's `extends A, B`, Kotlin's supertypes after the `:` of its header,
/// `Base` for `: Base(…)` and `: pkg.Base`. In a `scala` file, `extends A with B` (#416).
pub fn jvm_bases(text: &str, decl_line1: usize, scala: bool) -> Vec<String> {
    if scala {
        return scala_bases(text, decl_line1);
    }
    static JAVA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bextends\s+(.+?)\s*(?:\bimplements\b|\{|$)").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = decl_line1.checked_sub(1).filter(|&k| k < lines.len()) else {
        return Vec::new();
    };
    let mut header = String::new();
    for l in lines[k..].iter().take(20) {
        let code = uncommented(Kind::Jvm, l);
        header.push_str(code.split('{').next().unwrap_or(""));
        header.push(' ');
        if code.contains('{') {
            break;
        }
    }
    let mut outside_brackets = String::new();
    let mut depth = 0i32;
    for c in header.chars() {
        match c {
            '<' | '(' => depth += 1,
            '>' | ')' => depth -= 1,
            _ if depth == 0 => outside_brackets.push(c),
            _ => {}
        }
    }
    let list = match JAVA.captures(&outside_brackets) {
        Some(c) => c[1].to_owned(),
        None => match outside_brackets.split_once(':') {
            Some((_, rest)) => rest.split(" where ").next().unwrap_or("").to_owned(),
            None => return Vec::new(),
        },
    };
    list.split(',')
        .filter_map(|b| b.split_whitespace().next())
        .filter_map(|b| b.rsplit('.').next())
        .filter(|b| b.starts_with(|c: char| c.is_ascii_alphabetic()))
        .map(str::to_owned)
        .collect()
}

/// [`jvm_bases`] in Scala: a body needs no brace, so the header is the declaration's line and
/// the lines its open brackets carry it over, never the body under it (#416).
fn scala_bases(text: &str, decl_line1: usize) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = decl_line1.checked_sub(1).filter(|&k| k < lines.len()) else {
        return Vec::new();
    };
    let mut outside_brackets = String::new();
    let mut depth = 0i32;
    for l in lines[k..].iter().take(20) {
        let code = uncommented(Kind::Jvm, l);
        for c in code.split('{').next().unwrap_or("").chars() {
            match c {
                '[' | '(' => depth += 1,
                ']' | ')' => depth -= 1,
                _ if depth == 0 => outside_brackets.push(c),
                _ => {}
            }
        }
        outside_brackets.push(' ');
        if depth <= 0 || code.contains('{') {
            break;
        }
    }
    let Some((_, list)) = outside_brackets.split_once(" extends ") else {
        return Vec::new();
    };
    let list = list.split(" derives ").next().unwrap_or("");
    list.split(',')
        .flat_map(|b| b.split(" with "))
        .filter_map(|b| b.split_whitespace().next())
        .map(|b| b.trim_end_matches([':', '=']))
        .filter_map(|b| b.rsplit('.').next())
        .filter(|b| b.starts_with(|c: char| c.is_ascii_alphabetic()))
        .map(str::to_owned)
        .collect()
}

/// The names the `import` lines of Java or Kotlin `text` bind, each with the dotted path it
/// names (#372): `import app.a.User;` binds `User` to `[app, a, User]`, `import static a.C.m;`
/// binds `m` to `[a, C, m]`, Kotlin's `import a.C as D` binds `D` to `[a, C]`, and a wildcard,
/// static or not, binds `*` to its path with `*` last.
pub fn jvm_imports(text: &str) -> Vec<(String, Vec<String>)> {
    static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*import\s+(?:static\s+)?([\w.]*\w)(\.\*)?(?:\s+as\s+([A-Za-z_]\w*))?\s*;?\s*(?://.*)?$").unwrap()
    });
    text.lines()
        .filter_map(|l| IMPORT.captures(l))
        .map(|c| {
            let mut path: Vec<String> = c[1].split('.').map(str::to_owned).collect();
            let name = match (c.get(2), c.get(3)) {
                (Some(_), _) => "*".to_owned(),
                (None, Some(alias)) => alias.as_str().to_owned(),
                (None, None) => path.last().cloned().unwrap_or_default(),
            };
            if name == "*" {
                path.push(name.clone());
            }
            (name, path)
        })
        .collect()
}

pub fn jvm_package(text: &str) -> Option<String> {
    static PACKAGE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"(?m)^[ \t]*package\s+([\w.]*\w)").unwrap());
    PACKAGE
        .captures_iter(text)
        .map(|c| c[1].to_owned())
        .find(|p| p != "object")
}

const CAST_OPERAND: &str = r"(?:[\w.!?]|\([^()]*\))+";
const GENERIC: &str = r"(?:<(?:[^<>]|<(?:[^<>]|<[^<>]*>)*>)*>)?";

pub fn jvm_declared_type(line: &str, name: &str, kotlin: bool) -> Option<String> {
    let n = regex::escape(name);
    let code = uncommented(Kind::Jvm, line);
    let ty = r"([A-Z]\w*(?:\.[A-Z]\w*)*)";
    let rules: Vec<String> = match kotlin {
        true => {
            if Regex::new(&format!(r"\b{n}\b[^=]*\bby\b")).is_ok_and(|re| re.is_match(&code)) {
                return None;
            }
            vec![
                format!(r"(?:^|[^\w.]){n}\s*:\s*{ty}{GENERIC}\??\s*(?:[=,)]|$|\{{)"),
                format!(
                    r"\b(?:val|var)\s+{n}\s*=\s*{CAST_OPERAND}\s+as\??\s+{ty}{GENERIC}\??\s*;?\s*$"
                ),
                format!(r"\b(?:val|var)\s+{n}\s*=\s*([A-Z]\w*){GENERIC}\s*\("),
            ]
        }
        false => vec![
            format!(r"(?:^|[\s(,<]){ty}{GENERIC}(?:\s*\.\.\.)?\s+{n}\s*(?:[=;,:)]|&&|$)"),
            format!(r"\bvar\s+{n}\s*=\s*new\s+{ty}{GENERIC}\s*\("),
            format!(r"\bvar\s+{n}\s*=\s*\(\s*{ty}{GENERIC}\s*\)\s*[\w(]"),
        ],
    };
    rules.iter().find_map(|r| {
        let re = Regex::new(r).ok()?;
        let c = re.captures(&code)?;
        let at = c.get(1)?.start();
        (!code[..at].ends_with('.')).then(|| c[1].to_owned())
    })
}

pub fn scala_type_parameter(text: &str, name: &str) -> bool {
    static LIST: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:\b(?:def|class|trait|type|given)\s+`?\w+`?\s*|\bextension\s*)\[([^\[\]]*(?:\[[^\[\]]*\][^\[\]]*)*)\]").unwrap()
    });
    LIST.captures_iter(text).any(|c| {
        split_top(Kind::Jvm, &c[1], b',').iter().any(|e| {
            let e = e.trim().trim_start_matches(['+', '-']);
            let end = e
                .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .unwrap_or(e.len());
            &e[..end] == name
        })
    })
}

/// Whether Java or Kotlin `text` declares `name` as a type parameter: `class Box<T>`, `fun <T>`,
/// `<T extends Base> T pick(`.
pub fn jvm_type_parameter(text: &str, name: &str) -> bool {
    static LIST: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:\b(?:class|interface|record)\s+[A-Za-z_]\w*\s*|\bfun\s*|(?:^|[\s(,])\s*)<([^<>]*(?:<[^<>]*>[^<>]*)*)>").unwrap()
    });
    LIST.captures_iter(text).any(|c| {
        split_top(Kind::Jvm, &c[1], b',').iter().any(|e| {
            e.split_whitespace()
                .find(|w| !matches!(*w, "in" | "out" | "reified"))
                .is_some_and(|w| w.trim_end_matches(':') == name)
        })
    })
}

/// The Lombok accessor `word` (#381): `title` for `getTitle` and `setTitle`, `active` or
/// `isActive` for `isActive`. `None` for any other name.
pub fn jvm_accessor(word: &str) -> Option<JvmAccessor> {
    [("get", false), ("is", false), ("set", true)]
        .into_iter()
        .find_map(|(prefix, setter)| {
            let rest = word.strip_prefix(prefix)?;
            let mut chars = rest.chars();
            let first = chars.next().filter(char::is_ascii_uppercase)?;
            let mut names = vec![format!("{}{}", first.to_ascii_lowercase(), chars.as_str())];
            if prefix == "is" {
                names.push(word.to_owned());
            }
            Some(JvmAccessor {
                fields_it_may_read: names,
                setter,
            })
        })
}
pub struct JvmAccessor {
    pub fields_it_may_read: Vec<String>,
    pub setter: bool,
}

/// Lombok writes the accessor `word` for a field when the file imports `lombok.` (#381), and the
/// field carries `@Getter` (for `get` and `is`) or `@Setter` (for `set`), or the type carries `@Data`,
/// `@Value` or `@Getter`, or `@Data` or `@Setter`. A `static` field, and one marked
/// `AccessLevel.NONE`, gets none.
pub fn jvm_lombok_fields(text: &str, decl_line1: usize, word: &str) -> Vec<usize> {
    let Some(JvmAccessor {
        fields_it_may_read: names,
        setter,
    }) = jvm_accessor(word)
    else {
        return Vec::new();
    };
    if !text
        .lines()
        .any(|l| l.trim_start().starts_with("import lombok."))
    {
        return Vec::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    let with_annotations_above = |at: usize| {
        let mut out = String::from(lines[at]);
        for l in lines[..at].iter().rev() {
            match l.trim_start().starts_with('@') {
                true => out.push_str(l),
                false => break,
            }
        }
        out
    };
    let (own, class) = match setter {
        true => ("Setter", "Data|Setter"),
        false => ("Getter", "Data|Value|Getter"),
    };
    let own_re = Regex::new(&format!(r"@(?:lombok\.)?{own}\b")).expect("a fixed pattern");
    let class_re = Regex::new(&format!(r"@(?:lombok\.)?(?:{class})\b")).expect("a fixed pattern");
    let none_re =
        Regex::new(&format!(r"@(?:lombok\.)?{own}\s*\([^)]*\bNONE\b")).expect("a fixed pattern");
    static ANNOTATIONS: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*(?:@[\w.]+(?:\([^)]*\))?\s*)*").unwrap());
    let type_annotated = decl_line1
        .checked_sub(1)
        .is_some_and(|k| k < lines.len() && class_re.is_match(&with_annotations_above(k)));
    let mut out: Vec<usize> = names
        .iter()
        .flat_map(|n| jvm_members_of(text, decl_line1, n))
        .filter(|&l| {
            let code = uncommented(Kind::Jvm, lines[l - 1]);
            let code = ANNOTATIONS.replace(&code, "");
            let head = code.split('=').next().unwrap_or("");
            let field = !head.contains('(')
                && code.contains(';')
                && !head.split_whitespace().any(|w| w == "static");
            let own = with_annotations_above(l - 1);
            field && !none_re.is_match(&own) && (own_re.is_match(&own) || type_annotated)
        })
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// The call a Java `var` or a Kotlin `val`/`var` named `name` on `line` is assigned from, when
/// the whole value is one call of a lowercase name, `var status = plugin.statusNonNull();`: the
/// receiver in front of it, if any, and the method (#388, #391).
pub fn jvm_assigned_call(line: &str, name: &str) -> Option<(Option<String>, String)> {
    let re = Regex::new(&format!(
        r"\b(?:var|val)\s+{}\s*=\s*(?:([a-z]\w*)\.)?([a-z]\w*)\s*\([^()]*\)\s*;?\s*$",
        regex::escape(name)
    ))
    .ok()?;
    let code = uncommented(Kind::Jvm, line);
    let c = re.captures(code.trim_end())?;
    Some((c.get(1).map(|m| m.as_str().to_owned()), c[2].to_owned()))
}

/// The type the Java or Kotlin method `name` declared on `line` returns, as
/// [`jvm_declared_type`] reads one: `PluginStatus` for `public PluginStatus statusNonNull() {`
/// and `fun status(): Status`.
pub fn jvm_return_type(line: &str, name: &str, kotlin: bool) -> Option<String> {
    let n = regex::escape(name);
    let rule = match kotlin {
        true => format!(r"\bfun\s+(?:<[^>]*>\s*)?{n}\s*\([^)]*\)\s*:\s*([A-Z]\w*){GENERIC}\??"),
        false => format!(r"(?:^|\s)([A-Z]\w*){GENERIC}\s+{n}\s*\("),
    };
    let code = uncommented(Kind::Jvm, line);
    Regex::new(&rule)
        .ok()?
        .captures(&code)
        .map(|c| c[1].to_owned())
}

static DEFAULT: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"[^=!<>]=(?:[^=~]|$)").unwrap());

pub fn jvm_parameters(text: &str, line: usize, word: &str, groovy: bool) -> Option<(usize, usize)> {
    let rows: Vec<&str> = text.lines().collect();
    let at = line.checked_sub(1)?;
    let head = Regex::new(&format!(r"\b{}\s*\(", regex::escape(word))).ok()?;
    let open = head.find(rows.get(at)?)?.end() - 1;
    let Group {
        inner_uncommented: inner,
        ..
    } = group(Kind::Jvm, &rows, at, open)?;
    if inner.trim().is_empty() {
        return Some((0, 0));
    }
    let (mut depth, mut count, mut defaults) = (0i32, 1, 0);
    let mut top = String::new();
    let mut close = |top: &mut String| {
        defaults += usize::from(groovy && DEFAULT.is_match(top));
        top.clear();
    };
    let b = inner.as_bytes();
    let mut angles = Vec::new();
    let generic_opens = |i: usize, top: &str| {
        let name = b[..i].iter().rev();
        let name = name.take_while(|c| c.is_ascii_alphanumeric() || **c == b'_');
        !groovy
            || !DEFAULT.is_match(top)
            || (name.last().is_some_and(u8::is_ascii_uppercase)
                && !matches!(b.get(i + 1), Some(b'<' | b'=')))
    };
    for (i, c) in code(Kind::Jvm, &inner) {
        match c {
            b'<' if depth == angles.len() as i32 && generic_opens(i, &top) => {
                depth += 1;
                angles.push(depth);
            }
            b'>' if angles.last() == Some(&depth) => {
                angles.pop();
                depth -= 1;
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => {
                count += 1;
                close(&mut top);
            }
            _ if depth == 0 => top.push(char::from(c)),
            _ => {}
        }
    }
    close(&mut top);
    Some(match inner.contains("...") {
        true => (count - 1, usize::MAX),
        false => (count - defaults, defaults),
    })
}

pub fn jvm_smart_cast(text: &str, line: usize, before: &str, name: &str) -> Option<String> {
    let n = regex::escape(name);
    let ty = format!(r"([A-Z]\w*){GENERIC}\??");
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let test = rule(format!(
        r"\bif\s*\(\s*{n}\s+is\s+{ty}\s*(?:\)|&&)|^\s*{n}\s+is\s+{ty}\s*->"
    ));
    let branch = rule(format!(r"^\s*is\s+{ty}\s*->"));
    let subject = rule(format!(r"\bwhen\s*\(\s*{n}\s*\)\s*\{{\s*$"));
    let lines: Vec<&str> = text.lines().collect();
    let at = line.checked_sub(1).filter(|&i| i < lines.len())?;
    let header_of = |i: usize| {
        let depth = indent(lines[i]);
        (0..i).rev().find(|&j| {
            let t = lines[j].trim_start();
            !(t.is_empty() || comment(Kind::Jvm, t)) && indent(lines[j]) < depth
        })
    };
    let narrowed = |head: &str, at: usize| -> Option<String> {
        let code = uncommented(Kind::Jvm, head);
        if let Some(c) = test.captures(&code) {
            let rest = &code[c.get(0)?.end()..];
            return (!rest.contains("else") && !rest.contains(';') && !rest.contains("||"))
                .then(|| c.get(1).or(c.get(2)).map(|m| m.as_str().to_owned()))
                .flatten();
        }
        let c = branch.captures(&code)?;
        let when = header_of(at)?;
        subject
            .is_match(&uncommented(Kind::Jvm, lines[when]))
            .then(|| c[1].to_owned())
    };
    if let Some(t) = narrowed(before, at) {
        return Some(t);
    }
    let mut i = at;
    while let Some(h) = header_of(i) {
        let head = lines[h];
        if opens_type(head) || params_open_byte(head).is_some() {
            return None;
        }
        let code = uncommented(Kind::Jvm, head);
        let opens = code.trim_end().ends_with('{') || code.trim_end().ends_with("->");
        if opens && let Some(t) = narrowed(head, h) {
            return Some(t);
        }
        i = h;
    }
    None
}

pub fn jvm_cast_receiver(before: &str, kotlin: bool) -> Option<String> {
    static JAVA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"(?:^|[^\w.])\(\s*\(\s*([A-Z]\w*){GENERIC}\s*\)\s*[a-z]{CAST_OPERAND}\s*\)\.$"
        ))
        .unwrap()
    });
    static KOTLIN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"(?:^|[^\w.])\(\s*{CAST_OPERAND}\s+as\??\s+([A-Z]\w*){GENERIC}\??\s*\)(?:\?|!!)?\.$"
        ))
        .unwrap()
    });
    let re = match kotlin {
        true => &*KOTLIN,
        false => &*JAVA,
    };
    re.captures(before.trim_end()).map(|c| c[1].to_owned())
}
