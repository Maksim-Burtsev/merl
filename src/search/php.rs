//! PHP's own rules for `d`: the tags of a class's docblock and the segments of a namespace (#344),
//! the members `->` reaches (#348), and the class `$this`, `self`, `static` and `parent` name
//! (#356).

use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use super::symbols::php_mods;
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

/// The line pattern of a `namespace` line whose last part is the escaped word `w`.
pub(super) fn php_namespace_line(w: &str) -> String {
    format!(r"^\s*namespace\s+(?:[\w\\]+\\)?{w}\s*[;{{]")
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

/// The `namespace` rule of `patterns`, [`def_patterns`](super::def_patterns) of PHP for the word
/// at `range` of `line` in the file `text`, as the word asks (#344). `namespace X\Y;` is written
/// in every file of `X\Y` and names none of the classes that share its last part, so:
/// - a word a `\` follows is a segment of a qualified name, no class or function, and only a
///   line declaring the namespace written up to it answers: `Illuminate\Support` for `Support` in
///   `use Illuminate\Support\Facades\Route;`. A leading `\` is dropped, and outside a `use` or
///   a `namespace` line a name that does not start with one is relative to the file's own
///   namespace;
/// - the last part of a `namespace` line keeps the rule as it is: the other files of the
///   namespace are its namesakes;
/// - any other word is no namespace, and the rule goes.
pub fn php_namespace_patterns(
    patterns: &mut Vec<String>,
    text: &str,
    line: &str,
    range: Range<usize>,
) {
    static ON_NAMESPACE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*namespace\s+[\w\\]*$").unwrap());
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
        .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '\\'))
        .map_or(0, |i| i + 1);
    let spelled = &line[start..range.end];
    let head = before.trim_start();
    let absolute =
        spelled.starts_with('\\') || head.starts_with("use ") || head.starts_with("namespace ");
    let mut name = spelled.trim_start_matches('\\').to_owned();
    if !absolute && let Some(own) = php_namespace(text) {
        name = format!("{own}\\{name}");
    }
    *patterns = vec![format!(r"^\s*namespace\s+{}\s*[;{{]", regex::escape(&name))];
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

/// The namespace a PHP file declares, if any.
pub fn php_namespace(text: &str) -> Option<&str> {
    static NAMESPACE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?m)^\s*namespace\s+([\w\\]+)\s*[;{]").unwrap());
    NAMESPACE
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
}
