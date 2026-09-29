//! PHP's own rules for `d`: the tags of a class's docblock and the segments of a namespace (#344),
//! and the members `->` reaches (#348).

use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use super::symbols::php_mods;

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
    static NAMESPACE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?m)^\s*namespace\s+([\w\\]+)\s*[;{]").unwrap());
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
    if !absolute && let Some(own) = NAMESPACE.captures(text) {
        name = format!("{}\\{name}", &own[1]);
    }
    *patterns = vec![format!(r"^\s*namespace\s+{}\s*[;{{]", regex::escape(&name))];
}
