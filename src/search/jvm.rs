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

/// When the Java or Kotlin declaration on 1-based `line` of `text` is a local, the last 1-based
/// line of the block it is seen in; `None` for a member, a constructor property or a top-level
/// declaration (#357). A declaration is a local when the scope around it opens no type: a
/// function, a constructor, `init {`, an `if`, a lambda, an anonymous object.
pub fn jvm_local_block(text: &str, line: usize) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let at = line.checked_sub(1).filter(|&i| i < lines.len())?;
    let scope = scope_of(&lines, at)?;
    if opens_type(lines[scope]) {
        return None;
    }
    let depth = indent(lines[at]);
    let end = (at + 1..lines.len())
        .find(|&i| {
            let t = lines[i].trim_start();
            !steps_over(Some(Kind::Jvm), t) && indent(lines[i]) < depth
        })
        .unwrap_or(lines.len());
    Some(end)
}

/// Whether the modifiers in front of a Java or Kotlin declaration line include `private`: it is
/// seen in its own file only, whether Java's class or Kotlin's file or class keeps it (#357).
pub fn jvm_private(line: &str) -> bool {
    static MODS: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(jvm_mods!()).unwrap());
    MODS.find(line)
        .is_some_and(|m| m.as_str().split_whitespace().any(|w| w == "private"))
}

/// The receiver type a Kotlin extension on `line` declares `name` for, as written but for its
/// type arguments and a `?`: `Topic` for `fun Topic.asExternalModel()` and `val Topic.testTag`,
/// `List` for `fun <T> List<T>.second()` (#362), and the type of a Scala 3 extension's parameter
/// for the method written on its line: `String` for `extension (s: String) def slug` (#416).
/// `None` for any other line.
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

/// [`jvm_receiver`] for the declaration on 1-based `line` of `text`, and for a Scala 3
/// extension's method on a line of its own, the type of the `extension (s: T)` above it (#416).
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

/// The name `this` stands for at 1-based `line` of `text`, as [`qualified`] names a class:
/// the innermost type whose body holds the line (#362). `None` in column 0, and inside an
/// anonymous `object :` or `new X() {`, whose `this` has no name to look up.
pub fn jvm_this_owner(text: &str, line: usize) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut at = line.checked_sub(1).filter(|&i| i < lines.len())?;
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

/// Scala's rules of the Jvm kind (#416), behind its modifiers ([`scala_mods`]) and with the name
/// in backticks where a keyword has to be. Each needs a word Java and Kotlin never write at the
/// front of a declaration (`def`, `trait`, `type`, `given`, a Scala-only modifier, `case`), or
/// reads a form whose Kotlin spelling the rules before already find, so a `.java` or a `.kt`
/// line answers as before. Operators (`def +(…)`) have no rule, and a reserved word is a name
/// only in backticks: a constructor's `def this(…)` declares no `this`.
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
    // A name of the case list or the class header that is not the word, parameters and all.
    let other = r"`?\w+`?(?:\([^)]*\))?";
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
            r"^\s*{ann}case\s+(?:{other}\s*,\s*)*{n}(?:\([^)]*\))?(?:\s*,\s*{other})*(?:\s+extends\s+[\w.\[\]]+(?:\([^)]*\))?)?\s*(?://.*)?$"
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

/// The byte of line `at` where the parameter list of a function or a constructor declared there
/// opens: Kotlin's `fun` (past an extension's receiver) and `constructor`, Scala's `def`, a Java method (told by
/// the return type before its name) and a Java constructor. `None` for any other line, a
/// Kotlin class's primary constructor included: what it takes is a property or an argument of
/// its initializers, not a parameter of a body.
fn params_open(line: &str) -> Option<usize> {
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

/// The name of the function or the constructor whose header opens on `line`, when one does.
pub fn jvm_function(line: &str) -> Option<String> {
    let head = line[..params_open(line)?].trim_end();
    let start = head
        .trim_end_matches(|c: char| c.is_alphanumeric() || c == '_')
        .len();
    Some(head[start..].to_owned())
}

/// The 1-based lines where the header of a block, lines `from..=to` of `lines`, binds `name` for
/// the block under it: a function's or a constructor's parameters, a lambda's, a loop variable,
/// a `catch` parameter, a resource of `try (…)`, a pattern of `instanceof`.
fn header_binds(lines: &[&str], from: usize, to: usize, name: &str) -> Vec<usize> {
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let ret = jvm_return_type!();
    let clause = rule(format!(
        r"\bfor\s*\(\s*(?:(?:final\s+)?(?:var|val|{ret})\s+)?{n}\s*(?:[:=]|\s+in\b)|\bfor\s*\(\s*\([^)]*\b{n}\b[^)]*\)\s+in\b|\bcatch\s*\(\s*(?:final\s+)?[\w.|\s]+?\s+{n}\s*\)|\bcatch\s*\(\s*{n}\s*:|\btry\s*\(\s*(?:final\s+)?(?:var|{ret})\s+{n}\s*=|\binstanceof\s+(?:final\s+)?[\w.]+(?:<[^>]*>)?\s+{n}\b"
    ));
    let word = rule(format!(r"(?:^|[^\w$.]){n}(?:[^\w$]|$)"));
    let at_line = |from: usize, to: usize| {
        (from..=to)
            .find(|&i| word.is_match(&uncommented(Kind::Jvm, lines[i])))
            .map(|i| i + 1)
    };
    static DEF: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\bdef\s+[A-Za-z_]\w*\s*\(").unwrap());
    let groovy = DEF.is_match(lines[from]);
    let mut out = Vec::new();
    // The parameters, over the lines they span, and Scala's further lists, `(a: A)(b: B)`.
    if let Some(mut open) = params_open(lines[from]) {
        let mut at = from;
        while let Some((inner, last, rest)) = group(Kind::Jvm, lines, at, open) {
            if split_top(Kind::Jvm, &inner, b',')
                .iter()
                .any(|e| param_names(e, groovy).iter().any(|p| p == name))
            {
                out.extend(at_line(from, last));
                break;
            }
            let next = rest.trim_start();
            if !next.starts_with('(') {
                break;
            }
            (at, open) = (last, lines[last].len() - next.len());
        }
    }
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
        line,
        value: Value::Unknown,
    };
    if matches!(name, "this" | "super" | "it") {
        return Vec::new();
    }
    let literal = literal_lines(Kind::Jvm, &lines.join("\n"));
    // What the cursor's own line binds: `fun f(x: Int) = x`, `xs.map { x -> x }`.
    let mut out: Vec<Binding> = header_binds(lines, at, at, name)
        .into_iter()
        .map(binding)
        .collect();
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
        // A statement of a type's body is a member, no local.
        if ind == depth {
            if declares_local(t, name) && scope_of(lines, i).is_some_and(|s| !opens_type(lines[s]))
            {
                out.push(binding(i + 1));
            }
            continue;
        }
        // The header of the block around: a tail (`) {`, `): Int {`, `) = run {`) ends one opened
        // further up, back at its indent.
        let end = i;
        if t.starts_with([')', ']', '>']) {
            i = (0..i)
                .rev()
                .find(|&j| !lines[j].trim().is_empty() && indent(lines[j]) <= ind)
                .unwrap_or(i);
        }
        depth = indent(lines[i]);
        if opens_type(lines[i]) {
            break;
        }
        out.extend(header_binds(lines, i, end, name).into_iter().map(binding));
    }
    out
}

/// The 1-based lines of the types whose bodies hold 1-based `line` of Java or Kotlin `text`,
/// innermost first: a class, an interface, an object, an enum, a record, and an anonymous
/// `object :` or `new X() {`, which is a class for the lines inside it (#376).
pub fn jvm_enclosing_types(text: &str, line: usize) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(mut at) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
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

/// The 1-based lines on which the type declared on 1-based `decl` of Java or Kotlin `text`
/// declares `name` itself (#376): the lines one level inside its body that a `Kind::Jvm`
/// pattern matches, a property of its primary constructor, and a member of its `companion
/// object`. What a nested type declares is its own.
pub fn jvm_members_of(text: &str, decl: usize, name: &str) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = decl.checked_sub(1).filter(|&k| k < lines.len()) else {
        return Vec::new();
    };
    let Ok(re) = Regex::new(&def_patterns(Kind::Jvm, name).join("|")) else {
        return Vec::new();
    };
    let literal = literal_lines(Kind::Jvm, text);
    members_of(&lines, &literal, k, name, &re)
}

fn members_of(lines: &[&str], literal: &[bool], k: usize, name: &str, re: &Regex) -> Vec<usize> {
    static PROPERTY: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\b(?:val|var)\s+([A-Za-z_]\w*)\s*:").unwrap());
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
    // The body runs to the first line back at the header's indent, past the tails of a header
    // wrapped over several lines (`) : Base() {`).
    let end = (k + 1..lines.len())
        .find(|&i| {
            code(i) && indent(lines[i]) <= base && !lines[i].trim_start().starts_with([')', '{'])
        })
        .unwrap_or(lines.len());
    let level = (k + 1..end)
        .filter(|&i| code(i) && indent(lines[i]) > base)
        .map(|i| indent(lines[i]))
        .min();
    let mut out = Vec::new();
    // `class Topic(val id: String)`: a property on the header's own line.
    if let Some(open) = lines[k].find('(')
        && !anonymous(lines[k])
        && let Some((inner, ..)) = group(Kind::Jvm, lines, k, open)
        && split_top(Kind::Jvm, &inner, b',')
            .iter()
            .any(|e| PROPERTY.captures(e).is_some_and(|c| &c[1] == name))
        && PROPERTY.captures_iter(lines[k]).any(|c| &c[1] == name)
    {
        out.push(k + 1);
    }
    // A Java record's components, on its header's line or on the lines it wraps over (#367).
    if RECORD.is_match(lines[k]) {
        for (i, l) in lines.iter().enumerate().skip(k).take(40) {
            if re.is_match(l) {
                out.push(i + 1);
            }
            if l.contains('{') {
                break;
            }
        }
    }
    let Some(level) = level else {
        return out;
    };
    for i in (k + 1..end).filter(|&i| code(i) && indent(lines[i]) == level) {
        if re.is_match(lines[i]) {
            out.push(i + 1);
        } else if COMPANION.is_match(lines[i]) {
            out.extend(members_of(lines, literal, i, name, re));
        }
    }
    out
}

/// The types the Java or Kotlin type declared on 1-based `decl` of `text` extends, as simple
/// names: Java's `extends A, B`, Kotlin's supertypes after the `:` of its header, `Base` for
/// `: Base(…)` and `: pkg.Base`. In a `scala` file, `extends A with B` (#416).
pub fn jvm_bases(text: &str, decl: usize, scala: bool) -> Vec<String> {
    if scala {
        return scala_bases(text, decl);
    }
    static JAVA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bextends\s+(.+?)\s*(?:\bimplements\b|\{|$)").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = decl.checked_sub(1).filter(|&k| k < lines.len()) else {
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
    // Type arguments and the constructor's parameters say nothing of the supertypes.
    let mut flat = String::new();
    let mut depth = 0i32;
    for c in header.chars() {
        match c {
            '<' | '(' => depth += 1,
            '>' | ')' => depth -= 1,
            _ if depth == 0 => flat.push(c),
            _ => {}
        }
    }
    let list = match JAVA.captures(&flat) {
        Some(c) => c[1].to_owned(),
        None => match flat.split_once(':') {
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
fn scala_bases(text: &str, decl: usize) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = decl.checked_sub(1).filter(|&k| k < lines.len()) else {
        return Vec::new();
    };
    let mut flat = String::new();
    let mut depth = 0i32;
    for l in lines[k..].iter().take(20) {
        let code = uncommented(Kind::Jvm, l);
        // Type arguments and the constructor's parameters say nothing of the supertypes.
        for c in code.split('{').next().unwrap_or("").chars() {
            match c {
                '[' | '(' => depth += 1,
                ']' | ')' => depth -= 1,
                _ if depth == 0 => flat.push(c),
                _ => {}
            }
        }
        flat.push(' ');
        if depth <= 0 || code.contains('{') {
            break;
        }
    }
    let Some((_, list)) = flat.split_once(" extends ") else {
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

/// The package the `package` line of Java or Kotlin `text` declares (#372); `None` without one.
/// Scala's `package object shop` declares an object, no package (#416).
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

/// Whether Scala `text` declares `name` as a type parameter, in the `[…]` after the name of a
/// `def`, a `class`, a `trait`, a `type` or a `given`, or after `extension` (#416): `A` in `def
/// f[A: Ordering](a: A)`, `F` in `trait Repo[F[_]]`, `T` in `class Box[+T]`.
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

/// The fields a Lombok accessor `word` may read, and whether it is a setter (#381): `title` for
/// `getTitle` and `setTitle`, `active` or `isActive` for `isActive`. `None` for any other name.
pub fn jvm_accessor(word: &str) -> Option<(Vec<String>, bool)> {
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
            Some((names, setter))
        })
}

/// The 1-based lines of the fields of the type declared on 1-based `decl` of Java `text` that
/// Lombok writes the accessor `word` for (#381): the file imports `lombok.`, and the field
/// carries `@Getter` (for `get` and `is`) or `@Setter` (for `set`), or the type carries `@Data`,
/// `@Value` or `@Getter`, or `@Data` or `@Setter`. A `static` field, and one marked
/// `AccessLevel.NONE`, gets none.
pub fn jvm_lombok_fields(text: &str, decl: usize, word: &str) -> Vec<usize> {
    let Some((names, setter)) = jvm_accessor(word) else {
        return Vec::new();
    };
    if !text
        .lines()
        .any(|l| l.trim_start().starts_with("import lombok."))
    {
        return Vec::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    // A line and the annotations stacked on the lines above it.
    let annotated = |at: usize| {
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
    let type_annotated = decl
        .checked_sub(1)
        .is_some_and(|k| k < lines.len() && class_re.is_match(&annotated(k)));
    let mut out: Vec<usize> = names
        .iter()
        .flat_map(|n| jvm_members_of(text, decl, n))
        .filter(|&l| {
            let code = uncommented(Kind::Jvm, lines[l - 1]);
            let code = ANNOTATIONS.replace(&code, "");
            let head = code.split('=').next().unwrap_or("");
            let field = !head.contains('(')
                && code.contains(';')
                && !head.split_whitespace().any(|w| w == "static");
            let own = annotated(l - 1);
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

pub fn jvm_parameters(text: &str, line: usize, word: &str) -> Option<(usize, usize)> {
    let rows: Vec<&str> = text.lines().collect();
    let at = line.checked_sub(1)?;
    let head = Regex::new(&format!(r"\b{}\s*\(", regex::escape(word))).ok()?;
    let open = head.find(rows.get(at)?)?.end() - 1;
    let (inner, _, _) = group(Kind::Jvm, &rows, at, open)?;
    if inner.trim().is_empty() {
        return Some((0, 0));
    }
    let (mut depth, mut count) = (0i32, 1);
    for (_, c) in code(Kind::Jvm, &inner) {
        match c {
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' | b'>' => depth -= 1,
            b',' if depth == 0 => count += 1,
            _ => {}
        }
    }
    Some(match inner.contains("...") {
        true => (count - 1, usize::MAX),
        false => (count, 0),
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
        if opens_type(head) || params_open(head).is_some() {
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
