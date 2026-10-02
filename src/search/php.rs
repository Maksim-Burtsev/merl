//! PHP's own rules for `d`: the tags of a class's docblock and the segments of a namespace (#344),
//! the members `->` reaches (#348), and the class `$this`, `self`, `static` and `parent` name
//! (#356).

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::symbols::php_mods;
use super::syntax::close_of;
use super::words::steps_over;
use super::{Kind, indent};

/// A function or a method, behind its attributes and modifiers; `&` returns by reference.
pub(super) fn php_method(w: &str) -> String {
    format!(r"{}function\s+&?\s*{w}\s*\(", php_mods!())
}

/// A property, with the type it can carry between its modifiers and the `$`, and a constructor
/// parameter promoted to one, wherever it sits in the constructor's list.
pub(super) fn php_properties(w: &str) -> [String; 2] {
    [
        format!(r"{}(?:\??[\w\\|]+\s+)?\${w}\b", php_mods!("+")),
        format!(
            r"function\s+__construct\s*\(.*\b(?:public|private|protected|readonly)\s+(?:\??[\w\\|]+\s+)?\${w}\b"
        ),
    ]
}

/// A constant, the `const` of a class or a file with its type or without (#344), and an enum
/// case. A `case X:` of a `switch` matches against a constant, so what follows the name must
/// not be a `:`.
pub(super) fn php_constants(w: &str) -> [String; 2] {
    [
        format!(r"{}const\s+(?:[\w\\|&?()]+\s+)?{w}\b", php_mods!()),
        format!(r"^\s*case\s+{w}\s*(?:=[^=]|;|$)"),
    ]
}

/// The tags of a class's docblock (#344): a property, `-read` and `-write` ones included, and a
/// method, `static` or not. A tag of any other docblock declares nothing: [`php_tag_class`]
/// keeps the ones right above a class.
pub(super) fn php_tags(w: &str) -> [String; 2] {
    [
        format!(r"^\s*\*\s*@property(?:-read|-write)?\s+(?:[^$]*\s)?\${w}\b"),
        format!(r"^\s*\*\s*@method\s+(?:[^(]*\s)?{w}\s*\("),
    ]
}

/// The line patterns of what `$x->word` reaches (#348): a `call`, `->word(`, is a method or an
/// `@method` tag, since PHP calls a closure a property holds as `($x->word)(…)`; anything else a
/// property, promoted or tagged. A method is indented: a `function` in column 0 is a global one.
/// Never a local, a class, a constant or a namespace.
pub fn php_member_patterns(word: &str, call: bool) -> Vec<String> {
    let w = regex::escape(word);
    let [property, promoted] = php_properties(&w);
    let [property_tag, method_tag] = php_tags(&w);
    match call {
        true => vec![php_method(&w).replacen(r"^\s*", r"^\s+", 1), method_tag],
        false => vec![property, promoted, property_tag],
    }
}

const NAMESPACE_HEAD: &str = r"^\s*(?:<\?php\s+)?namespace\s+";

pub(super) fn php_namespace_line(w: &str) -> String {
    format!(r"{NAMESPACE_HEAD}(?:[\w\\]+\\)?{w}\s*[;{{]")
}

/// The 0-based line of the `class`, `trait`, `interface` or `enum` whose docblock holds the
/// `@property`, `@property-read`, `@property-write` or `@method` tag on 0-based line `at` of
/// `lines` (#344): the `/** … */` around the tag ends right above that line, `#[…]` attributes
/// aside. `None` for any other line, and for a tag of a function's docblock or a loose comment.
pub fn php_tag_class<S: AsRef<str>>(lines: &[S], at: usize) -> Option<usize> {
    static TAG: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*\*\s*@(?:property(?:-read|-write)?|method)\s").unwrap());
    static CLASS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:abstract|final|readonly)\s+)*(?:class|trait|interface|enum)\s+\w")
            .unwrap()
    });
    let line = |i: usize| lines[i].as_ref().trim_start();
    if !TAG.is_match(lines.get(at)?.as_ref()) {
        return None;
    }
    let open = (0..at).rev().find(|&i| !line(i).starts_with('*'))?;
    if !line(open).starts_with("/**") || (open..at).any(|i| line(i).contains("*/")) {
        return None;
    }
    let end = (at..lines.len()).find(|&i| line(i).contains("*/"))?;
    // An attribute can be wrapped over lines, `#[Guarded([` over `'id',` over `])]`.
    // ponytail: brackets counted as written, a `[` inside a string of an attribute included.
    let mut depth = 0;
    let class = (end + 1..lines.len()).find(|&i| {
        let t = line(i);
        if depth == 0 && !t.starts_with("#[") {
            return true;
        }
        depth += t.matches('[').count() as isize - t.matches(']').count() as isize;
        false
    })?;
    CLASS.is_match(lines[class].as_ref()).then_some(class)
}

pub fn php_namespace_patterns(
    patterns: &mut Vec<String>,
    text: &str,
    line: &str,
    range: Range<usize>,
) {
    static ON_NAMESPACE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(&format!(r"{NAMESPACE_HEAD}[\w\\]*$")).unwrap());
    let generic = php_namespace_line(&regex::escape(&line[range.clone()]));
    let Some(at) = patterns.iter().position(|p| *p == generic) else {
        return;
    };
    let (before, after) = (&line[..range.start], &line[range.end..]);
    if !after.starts_with('\\') {
        if !(ON_NAMESPACE.is_match(before) && after.trim_start().starts_with([';', '{'])) {
            patterns.remove(at);
        }
        return;
    }
    let start = before
        .trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '\\')
        .len();
    let spelled = &line[start..range.end];
    let head = before.trim_start();
    let absolute = spelled.starts_with('\\')
        || head.starts_with("use ")
        || head
            .trim_start_matches("<?php")
            .trim_start()
            .starts_with("namespace ");
    let mut name = spelled.trim_start_matches('\\').to_owned();
    if !absolute && let Some(own) = php_namespace(text) {
        name = format!("{own}\\{name}");
    }
    *patterns = vec![format!(r"{NAMESPACE_HEAD}{}\s*[;{{]", regex::escape(&name))];
}

/// How a PHP member is reached, which says what can declare it (#356): a call is a method, a
/// `->name` or a `::$name` a property, a `::NAME` a constant or an enum case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhpAccess {
    Call,
    Property,
    Constant,
}

/// The line patterns of what `access` reaches of a class, wherever the line sits:
/// [`php_class_members`] keeps the ones directly in the class.
pub fn php_access_patterns(word: &str, access: PhpAccess) -> Vec<String> {
    let w = regex::escape(word);
    let [property, promoted] = php_properties(&w);
    let [property_tag, method_tag] = php_tags(&w);
    match access {
        PhpAccess::Call => vec![php_method(&w), method_tag],
        PhpAccess::Property => vec![property, promoted, property_tag],
        PhpAccess::Constant => php_constants(&w).into(),
    }
}

/// A `class`, `trait`, `interface` or `enum` line: its keyword, its name and the class it
/// `extends`, as written.
pub fn php_class_header(line: &str) -> Option<(&str, &str, Option<&str>)> {
    static HEADER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:abstract|final|readonly)\s+)*(class|trait|interface|enum)\s+(\w+)(?:\s*:\s*\w+)?(?:\s+extends\s+([\w\\]+))?").unwrap()
    });
    let c = HEADER.captures(line)?;
    Some((
        c.get(1)?.as_str(),
        c.get(2)?.as_str(),
        c.get(3).map(|m| m.as_str()),
    ))
}

/// The line 0-based `at` of `lines` stands directly inside: the nearest one above it indented
/// less, past blank lines, comments, attributes and a lone `{`.
fn php_owner<S: AsRef<str>>(lines: &[S], at: usize) -> Option<usize> {
    let depth = indent(lines.get(at)?.as_ref());
    (0..at).rev().find(|&i| {
        let l = lines[i].as_ref();
        !steps_over(Some(Kind::Php), l.trim_start()) && indent(l) < depth
    })
}

/// The 0-based line of the class, trait, interface or enum the 0-based line `at` of `lines` is
/// in, walking out through the lines each one stands inside (#356). `None` at the top level, and
/// inside an anonymous class, `new class(…) extends X {`, whose `$this` is that class.
pub fn php_enclosing_class<S: AsRef<str>>(lines: &[S], at: usize) -> Option<usize> {
    static ANONYMOUS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bnew\s+class\b").unwrap());
    let mut at = at;
    loop {
        at = php_owner(lines, at)?;
        let l = lines[at].as_ref();
        if ANONYMOUS.is_match(l) {
            return None;
        }
        if php_class_header(l).is_some() {
            return Some(at);
        }
    }
}

/// The 0-based lines of `lines` that `re` matches and that declare a member of the class on
/// 0-based line `class` (#356): directly in its body, a constructor parameter promoted on a line
/// of its own, or a tag of its docblock ([`php_tag_class`]).
pub fn php_class_members<S: AsRef<str>>(lines: &[S], class: usize, re: &Regex) -> Vec<usize> {
    static CONSTRUCTOR: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\bfunction\s+__construct\s*\(").unwrap());
    (0..lines.len())
        .filter(|&i| re.is_match(lines[i].as_ref()))
        .filter(|&i| match php_owner(lines, i) {
            Some(o) if o == class => true,
            Some(o) if CONSTRUCTOR.is_match(lines[o].as_ref()) => {
                php_owner(lines, o) == Some(class)
            }
            _ => php_tag_class(lines, i) == Some(class),
        })
        .collect()
}

/// The traits the body of the class on 0-based line `class` pulls in, as written: an indented
/// `use A;` or `use A, B;` directly inside it, not the imports at the top of the file.
pub fn php_class_traits<S: AsRef<str>>(lines: &[S], class: usize) -> Vec<String> {
    static USE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s+use\s+([\w\\]+(?:\s*,\s*[\w\\]+)*)\s*[;{]").unwrap());
    (class + 1..lines.len())
        .filter(|&i| php_owner(lines, i) == Some(class))
        .filter_map(|i| USE.captures(lines[i].as_ref()))
        .flat_map(|c| {
            c[1].split(',')
                .map(|t| t.trim().to_owned())
                .collect::<Vec<_>>()
        })
        .collect()
}

static NAMESPACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"(?m){NAMESPACE_HEAD}([\w\\]+)\s*[;{{]")).unwrap());

pub fn php_namespace(text: &str) -> Option<&str> {
    NAMESPACE
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
}

pub fn php_block(text: &str, line: usize) -> &str {
    let mut lines = vec![0];
    lines.extend(text.match_indices('\n').map(|(i, _)| i + 1));
    let declared: Vec<usize> = NAMESPACE
        .captures_iter(text)
        .map(|c| lines.partition_point(|&o| o <= c.get(1).map_or(0, |m| m.start())) - 1)
        .collect();
    if declared.len() < 2 {
        return text;
    }
    let literal = super::syntax::literal_lines(Kind::Php, text);
    let declared: Vec<usize> = declared
        .into_iter()
        .filter(|&l| literal.get(l) != Some(&true))
        .collect();
    if declared.len() < 2 {
        return text;
    }
    let i = declared.iter().rposition(|&l| l <= line).unwrap_or(0);
    let end = declared.get(i + 1).map_or(text.len(), |&l| lines[l]);
    &text[lines[declared[i]]..end]
}

/// The class name the word at `range` of `line` belongs to, as written (`\A\B`, `B`), and
/// whether the word is a member of it, `B::word`, rather than the name itself (#351). Only where
/// PHP reads a class name: before `::`, after `new`, `extends`, `implements`, `instanceof`,
/// `insteadof` and `catch (`, a type in front of a `$parameter`, a return type, and the last part
/// of a column-0 `use` line. A function or a constant falls back to the global namespace, so no
/// other position is read as a class.
pub fn php_class_at(line: &str, range: Range<usize>) -> Option<(String, bool)> {
    static AFTER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:\b(?:new|extends|implements|instanceof|insteadof)|\bcatch\s*\(|\)\s*:\s*\??|\bimplements\s.*,)\s*$").unwrap()
    });
    static TYPED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*(?:&|\.\.\.)?\s*\$\w").unwrap());
    let name_back = |s: &str| {
        let n = s.trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '\\');
        (n.len(), s[n.len()..].to_owned())
    };
    let (before, word, after) = (
        &line[..range.start],
        &line[range.clone()],
        &line[range.end..],
    );
    let is_class = |at: usize, written: &str| {
        !written.trim_start_matches('\\').is_empty()
            && !written.starts_with(|c: char| c.is_ascii_digit())
            && !matches!(written, "self" | "static" | "parent")
            && !line[..at].ends_with(['$', '>'])
            && !line[..at].ends_with("::")
    };
    if let Some(b) = before
        .strip_suffix("::$")
        .or_else(|| before.strip_suffix("::"))
    {
        let (at, written) = name_back(b);
        return (word != "class" && is_class(at, &written)).then_some((written, true));
    }
    if after.starts_with(|c: char| c == '\\' || c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let (at, head) = name_back(before);
    let written = format!("{head}{word}");
    let pre = line[..at].trim_end();
    let used = line.starts_with("use ")
        && !line.contains('{')
        && !["use function ", "use const "]
            .iter()
            .any(|u| line.starts_with(u));
    let typed = TYPED.is_match(after)
        && (pre.is_empty()
            || pre.ends_with(['(', ',', '?', '|'])
            || ["public", "protected", "private", "readonly"]
                .iter()
                .any(|m| pre.ends_with(m)));
    (is_class(at, &written)
        && (used || after.trim_start().starts_with("::") || AFTER.is_match(pre) || typed))
        .then(|| match used && !written.starts_with('\\') {
            true => format!("\\{written}"),
            false => written,
        })
        .map(|w| (w, false))
}

/// The fully qualified name, with no leading `\`, of the class `written` names in the file
/// `text`, as PHP resolves it, and whether a `use` bound it (#351): a leading `\` spells it in
/// full, else the file's column-0 `use` that binds its first part (`use A\B\C;`, `use A\B\C as
/// D;`; `use function` and `use const` bind no class), else the file's
/// namespace in front of it. `None` for a name a group `use` binds: one clause binds several
/// names there, and the rules do not read it.
pub fn php_resolve(text: &str, written: &str) -> Option<(String, bool)> {
    static USE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?m)^use\s+([\w\\\s,]+?)\s*;").unwrap());
    static GROUP: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?m)^use\s+[^;{]*\{([^}]*)\}").unwrap());
    if let Some(full) = written.strip_prefix('\\') {
        return Some((full.to_owned(), false));
    }
    let (first, rest) = match written.split_once('\\') {
        Some((first, rest)) => (first, Some(rest)),
        None => (written, None),
    };
    let used = USE
        .captures_iter(text)
        .flat_map(|c| {
            c[1].split(',')
                .map(|item| item.trim().to_owned())
                .collect::<Vec<_>>()
        })
        .filter(|item| !item.starts_with("function ") && !item.starts_with("const "))
        .find_map(|item| {
            let (path, alias) = match item.split_once(" as ") {
                Some((path, alias)) => (path.trim().to_owned(), alias.trim().to_owned()),
                None => (item.clone(), item.rsplit('\\').next()?.to_owned()),
            };
            (alias == first).then(|| path.trim_start_matches('\\').to_owned())
        });
    let grouped = || {
        GROUP.captures_iter(text).any(|c| {
            c[1].split(',').any(|item| {
                let item = item.trim();
                let alias = item.split_once(" as ").map_or(item, |(_, a)| a.trim());
                alias.rsplit('\\').next() == Some(first)
            })
        })
    };
    let tail = rest.map_or(String::new(), |r| format!("\\{r}"));
    Some(match used {
        Some(path) => (format!("{path}{tail}"), true),
        None if grouped() => return None,
        None => match php_namespace(text) {
            Some(ns) => (format!("{ns}\\{written}"), false),
            None => (written.to_owned(), false),
        },
    })
}

pub fn php_psr4(root: &Path, dir: &Path) -> Vec<(String, Vec<PathBuf>)> {
    static MAP: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#""psr-4"\s*:\s*\{([^}]*)\}"#).unwrap());
    static ENTRY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#""((?:[^"\\]|\\.)*)"\s*:\s*(?:"((?:[^"\\]|\\.)*)"|\[([^\]]*)\])"#).unwrap()
    });
    static STRING: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#""((?:[^"\\]|\\.)*)""#).unwrap());
    let unescape = |s: &str| s.replace("\\\\", "\\").replace("\\/", "/");
    let mut map = Vec::new();
    for at in dir.ancestors() {
        let Ok(text) = std::fs::read_to_string(root.join(at).join("composer.json")) else {
            continue;
        };
        let dir_of = |d: &str| {
            let d = unescape(d);
            let d = d.trim_start_matches("./").trim_end_matches('/');
            match d {
                "" | "." => at.to_path_buf(),
                d => at.join(d),
            }
        };
        for m in MAP.captures_iter(&text) {
            for c in ENTRY.captures_iter(&m[1]) {
                let dirs = match (c.get(2), c.get(3)) {
                    (Some(one), _) => vec![dir_of(one.as_str())],
                    (_, Some(list)) => STRING
                        .captures_iter(list.as_str())
                        .map(|s| dir_of(&s[1]))
                        .collect(),
                    _ => Vec::new(),
                };
                map.push((unescape(&c[1]).trim_start_matches('\\').to_owned(), dirs));
            }
        }
    }
    map.sort_by_key(|(prefix, _)| std::cmp::Reverse(prefix.len()));
    map
}

/// Where the PSR-4 `map` puts the class `full` (#351): `Some(Ok(file))` for the first mapped
/// file `has` holds, `Some(Err(()))` for a name a prefix covers whose file is missing, `None` for
/// a name the map does not cover: one outside the project. The empty prefix maps any name, so it
/// covers only a name whose file exists under it.
pub fn php_psr4_file(
    map: &[(String, Vec<PathBuf>)],
    full: &str,
    has: impl Fn(&Path) -> bool,
) -> Option<Result<PathBuf, ()>> {
    let mut covered = false;
    for (prefix, dirs) in map {
        let Some(rest) = full.strip_prefix(prefix.as_str()) else {
            continue;
        };
        covered |= !prefix.is_empty();
        if let Some(file) = dirs
            .iter()
            .map(|d| d.join(format!("{}.php", rest.replace('\\', "/"))))
            .find(|f| has(f))
        {
            return Some(Ok(file));
        }
    }
    covered.then_some(Err(()))
}

/// What a line binding PHP's `$name` says of its type (#361).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhpBinding {
    /// A type written in front of it: a parameter, a promoted one, a property, a `catch`.
    Type(String),
    /// `$name = new T(…)`; `self` is the class around the line.
    New(String),
    /// `$name = T::m(…)`, `self::m(…)` among them.
    Static(String, String),
    /// `$name = $this->m(…)`.
    This(String),
    /// `$name = f(…)`.
    Function(String),
}

/// Names that stand where a class does and are none: the types of no single class, what refers
/// to a class that can be a subclass, and the keywords a `$name` can follow on its line.
const NO_CLASS: &[&str] = &[
    "array",
    "bool",
    "callable",
    "false",
    "float",
    "int",
    "iterable",
    "mixed",
    "never",
    "null",
    "object",
    "parent",
    "resource",
    "static",
    "string",
    "true",
    "void",
    "public",
    "private",
    "protected",
    "readonly",
    "var",
    "global",
    "return",
    "echo",
    "print",
    "yield",
    "clone",
    "new",
    "as",
    "case",
    "throw",
    "else",
    "and",
    "or",
    "xor",
    "instanceof",
    "include",
    "require",
    "fn",
    "function",
    "use",
    "list",
    "isset",
    "unset",
    "empty",
    "match",
];

/// The one class written as the type of `$name` on `line`, `?` dropped: `UserUnsubscribed $event`,
/// `private readonly ?Podcast $podcast`. `None` for no type, a union or an intersection
/// (`A|B`, `A&B`), a variadic `T ...$xs`, and a type that is no single class (`mixed`, `array`,
/// `static`…). `self` is returned as written.
pub fn php_written_type(line: &str, name: &str) -> Option<String> {
    let re = Regex::new(&format!(r"(\??[\w\\]+)\s+&?\${}\b", regex::escape(name))).ok()?;
    let c = re.captures(line)?;
    let m = c.get(1)?;
    let before = line[..m.start()].trim_end();
    if before.ends_with(['|', '&', '.', '$', ':', '>', '-', '=']) {
        return None;
    }
    let written = m.as_str().trim_start_matches('?');
    let short = written.rsplit('\\').next().unwrap_or(written);
    (!short.is_empty() && !NO_CLASS.contains(&short.to_ascii_lowercase().as_str()))
        .then(|| written.to_owned())
}

/// The class `statement` binds `$name` to, the statement starting on the line that binds it and
/// running on over the lines below: a type written in front of it, or an assignment whose whole
/// value is `new T(…)`, `T::m(…)`, `$this->m(…)` or `f(…)`. `None` for anything else: a
/// `foreach` target, a `list(…)`, a call on a call, `new static`, `static::m()`.
pub fn php_binding(statement: &str, name: &str) -> Option<PhpBinding> {
    static NEW: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^new\s+([\w\\]+)\s*").unwrap());
    static STATIC: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^([\w\\]+)::(\w+)\s*\(").unwrap());
    static THIS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\$this->(\w+)\s*\(").unwrap());
    static FUNCTION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([\w\\]+)\s*\(").unwrap());
    let first = statement.lines().next().unwrap_or_default();
    let n = regex::escape(name);
    let assign = Regex::new(&format!(r"(?:^|[^\w$:>])\${n}\s*=([^=>]|$)")).ok()?;
    // A parameter's default, `Foo $x = null`, is no assignment of the variable.
    if let Some(t) = php_written_type(first, name) {
        return Some(PhpBinding::Type(t));
    }
    let at = assign.captures(first)?.get(1)?.start();
    let value = statement[at..].trim_start();
    // The whole value, up to the `;` that ends it: a call whose result is called on, or a
    // construction followed by anything, is some other value.
    let ends = |open: usize| {
        let close = close_of(Kind::Php, value, open)?;
        value[close..].trim_start().starts_with(';').then_some(())
    };
    if let Some(c) = NEW.captures(value) {
        let class = c[1].to_owned();
        let rest = c.get(0)?.end();
        match value[rest..].starts_with('(') {
            true => ends(rest)?,
            false => value[rest..].starts_with(';').then_some(())?,
        }
        let lower = class.to_ascii_lowercase();
        return (lower != "static" && lower != "class" && lower != "parent")
            .then_some(PhpBinding::New(class));
    }
    let open = value.find('(')?;
    ends(open)?;
    if let Some(c) = STATIC.captures(value) {
        let lower = c[1].to_ascii_lowercase();
        return (lower != "static" && lower != "parent")
            .then(|| PhpBinding::Static(c[1].to_owned(), c[2].to_owned()));
    }
    if let Some(c) = THIS.captures(value) {
        return Some(PhpBinding::This(c[1].to_owned()));
    }
    let c = FUNCTION.captures(value)?;
    let short = c[1]
        .rsplit('\\')
        .next()
        .unwrap_or(&c[1])
        .to_ascii_lowercase();
    (!NO_CLASS.contains(&short.as_str())).then(|| PhpBinding::Function(c[1].to_owned()))
}

/// The one class a function or a method declares it returns, `?` dropped, in `header`: its line
/// and the lines below it, which its parameters can wrap over. `self` is returned as written;
/// `static`, a union, an intersection and no return type give `None`.
pub fn php_return_type(header: &str) -> Option<String> {
    static RETURNS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*:\s*\??([\w\\]+)\s*(?:\{|;|$)").unwrap());
    let open = header.find("function")? + header[header.find("function")?..].find('(')?;
    let close = close_of(Kind::Php, header, open)?;
    let written = RETURNS.captures(&header[close..])?.get(1)?.as_str();
    let short = written.rsplit('\\').next().unwrap_or(written);
    (!NO_CLASS.contains(&short.to_ascii_lowercase().as_str())).then(|| written.to_owned())
}
