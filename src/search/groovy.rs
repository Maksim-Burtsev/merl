use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

use super::*;

static EXT_PROPERTY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:project\.)?ext\.([A-Za-z_]\w*)\s*=[^=]").unwrap());
static EXT_MEMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s+([A-Za-z_]\w*)\s*=[^=]").unwrap());
static TASK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*task\s+([A-Za-z_]\w*)(?:[^\w.=]|$)").unwrap());
static REGISTERED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\btasks\.(?:register|create)\s*\(?\s*['"]([A-Za-z_]\w*)['"]"#).unwrap()
});

pub(super) fn groovy_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    vec![
        format!(r"^\s*(?:project\.)?ext\.{w}\s*=[^=]"),
        format!(r"^\s+{w}\s*=[^=]"),
        format!(r"^\s*task\s+{w}(?:[^\w.=]|$)"),
        format!(r#"\btasks\.(?:register|create)\s*\(?\s*['"]{w}['"]"#),
    ]
}

fn names(re: &Regex, line: &str, word: &str) -> bool {
    re.captures(line).is_some_and(|c| &c[1] == word)
}

pub(super) fn groovy_declares<'a, S: AsRef<str> + 'a>(
    path: &Path,
    word: &str,
    line: usize,
    line_text: &str,
    lines: impl FnOnce() -> &'a [S],
) -> Option<bool> {
    let member = names(&EXT_MEMBER, line_text, word);
    let other = [&*EXT_PROPERTY, &TASK, &REGISTERED]
        .iter()
        .any(|re| names(re, line_text, word));
    if !member && !other {
        return None;
    }
    Some(
        groovy(path)
            && (other || {
                let lines = lines();
                directly_inside(lines, line, "ext {") || directly_inside(lines, line, "ext{")
            }),
    )
}

pub fn gradle_global(line: &str) -> bool {
    [&*EXT_PROPERTY, &EXT_MEMBER, &TASK, &REGISTERED]
        .iter()
        .any(|re| re.is_match(line))
}
