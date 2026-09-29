//! Java and Kotlin scopes, read off the indentation: which block a declaration is seen in.

use regex::Regex;

use super::*;

/// Whether `line` opens a type's body: a class, an interface, an enum, a record, an annotation
/// type, a named `object` or a `companion object`, a Kotlin primary constructor `class X(`
/// included. An anonymous `object :` or `new X() {` is no type here: what it holds is local.
fn opens_type(line: &str) -> bool {
    static TYPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            jvm_mods!(),
            r"(?:(?:fun\s+)?interface|class|enum|record|@interface|object\s+[A-Za-z_]|companion\s+object)\b"
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
/// `List` for `fun <T> List<T>.second()` (#362). `None` for any other line.
pub fn jvm_receiver(line: &str, name: &str) -> Option<String> {
    static EXTENSION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            jvm_mods!(),
            r"(?:fun|val|var)\s+(?:<[^>]*>\s*)?([\w.]+)(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?\??\.([A-Za-z_]\w*)"
        ))
        .unwrap()
    });
    EXTENSION
        .captures(line)
        .filter(|c| &c[2] == name)
        .map(|c| c[1].to_owned())
}

/// The name `this` stands for at 1-based `line` of `text`, as [`qualified`] names a class:
/// the innermost type whose body holds the line (#362). `None` in column 0, and inside an
/// anonymous `object :` or `new X() {`, whose `this` has no name to look up.
pub fn jvm_this_owner(text: &str, line: usize) -> Option<String> {
    static ANONYMOUS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bobject\s*:|\bnew\s+[\w.]+\s*(?:<.*>)?\s*\(.*\)\s*\{\s*$").unwrap()
    });
    static NAME: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:class|interface|enum|record|@interface|object)\s+([A-Za-z_]\w*)").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let mut at = line.checked_sub(1).filter(|&i| i < lines.len())?;
    loop {
        at = scope_of(&lines, at)?;
        if opens_type(lines[at]) {
            let name = NAME.captures(lines[at])?[1].to_owned();
            return Some(qualified(Kind::Jvm, text, at + 1, &name).unwrap_or(name));
        }
        if ANONYMOUS.is_match(lines[at]) {
            return None;
        }
    }
}
