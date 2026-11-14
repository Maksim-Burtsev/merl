use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

use super::{
    Kind, directly_inside, groovy, indent, jvm_enclosing_types, jvm_type_name, uncommented,
};

static EXT_PROPERTY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:project\.)?ext\.([A-Za-z_]\w*)\s*=[^=]").expect("the regex is a literal")
});
static EXT_MEMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s+([A-Za-z_]\w*)\s*=[^=]").expect("the regex is a literal"));
static TASK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*task\s+([A-Za-z_]\w*)(?:[^\w.=]|$)").expect("the regex is a literal")
});
static CONSTRUCTOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*([A-Za-z_]\w*)\s*\([^;]*\)\s*\{").expect("the regex is a literal")
});
static REGISTERED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\btasks\.(?:register|create)\s*\(?\s*['"]([A-Za-z_]\w*)['"]"#)
        .expect("the regex is a literal")
});

pub(super) fn groovy_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    vec![
        format!(r"^\s*(?:project\.)?ext\.{w}\s*=[^=]"),
        format!(r"^\s+{w}\s*=[^=]"),
        format!(r"^\s*task\s+{w}(?:[^\w.=]|$)"),
        format!(r#"\btasks\.(?:register|create)\s*\(?\s*['"]{w}['"]"#),
        format!(r"^\s*{w}\s*\([^;]*\)\s*\{{"),
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
    if names(&CONSTRUCTOR, line_text, word) {
        return Some(
            groovy(path) && {
                let lines = lines().iter().map(AsRef::as_ref).collect::<Vec<&str>>();
                jvm_enclosing_types(&lines.join("\n"), line)
                    .first()
                    .and_then(|&t| jvm_type_name(lines[t - 1]))
                    .is_some_and(|name| name == word)
            },
        );
    }
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

fn labelled(line: &str, col: usize) -> bool {
    let Some((r, _)) = super::word_at(line, col, "") else {
        return false;
    };
    let (Some(lead), Some(tail)) = (line.get(..r.start), line.get(r.end..)) else {
        return false;
    };
    let (lead, tail) = (lead.trim_end(), tail.trim_start());
    tail.starts_with(':')
        && !tail.starts_with("::")
        && !lead.ends_with('?')
        && !lead.ends_with("case")
}

pub(super) fn script_binding(
    path: &Path,
    lines: &[&str],
    text: &str,
    literal: &[bool],
    at: usize,
    col: Option<usize>,
    name: &str,
) -> Option<usize> {
    if !groovy(path)
        || path.extension().is_some_and(|e| e == "gradle")
        || col.is_some_and(|c| labelled(lines[at], c))
    {
        return None;
    }
    let n = regex::escape(name);
    let ret = super::jvm_return_type!();
    let assigned = Regex::new(&format!(r"^{n}\s*=[^=~]")).ok()?;
    let declared = Regex::new(&format!(
        r"^(?:@[\w.]+(?:\([^)]*\))?\s+)*(?:final\s+)?(?:def|var|{ret})\s+{n}\s*(?:[=;]|$)"
    ))
    .ok()?;
    let top = |k: &usize| !literal.get(*k).copied().unwrap_or(true) && indent(lines[*k]) == 0;
    let script = |k: usize| uncommented(Kind::Jvm, lines[k]);
    let first = (0..=at)
        .filter(top)
        .find(|&k| assigned.is_match(&script(k)))?;
    let declared_too = (0..lines.len())
        .filter(top)
        .any(|k| declared.is_match(&script(k)));
    (!declared_too && jvm_enclosing_types(text, at + 1).is_empty()).then_some(first)
}

pub fn groovy_script_local(path: &Path, text: &str, line1: usize) -> bool {
    static LOCAL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(concat!(
            r"^(?:final\s+)?(?:def|var|",
            super::jvm_return_type!(),
            r")\s+[A-Za-z_$][\w$]*\s*(?:[=;]|$)"
        ))
        .expect("the regex is a literal")
    });
    static FIELD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^@(?:groovy\.transform\.)?Field\s*$").expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let line = lines
        .get(line1.wrapping_sub(1))
        .copied()
        .unwrap_or_default();
    let field = lines[..line1.saturating_sub(1).min(lines.len())]
        .iter()
        .rev()
        .map(|l| uncommented(Kind::Jvm, l))
        .take_while(|l| l.starts_with('@'))
        .any(|l| FIELD.is_match(l.trim_end()));
    groovy(path)
        && path.extension().is_none_or(|e| e != "gradle")
        && !field
        && LOCAL.is_match(&uncommented(Kind::Jvm, line))
        && jvm_enclosing_types(text, line1).is_empty()
}

pub fn groovy_script_sees(path: &Path, text: &str, decl_line1: usize, at_line1: usize) -> bool {
    static METHOD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(concat!(
            r"^(?:@[\w.]+(?:\([^)]*\))?\s+)*(?:(?:public|protected|private|static|final",
            r"|abstract|synchronized|def|void|int|long|short|byte|char|boolean|float|double",
            r"|(?:[a-z_]\w*\.)*[A-Z][\w$]*(?:<[^()]*>)?(?:\[\])*)\s+)+",
            r"[A-Za-z_$][\w$]*\s*\(([^{]*)"
        ))
        .expect("the regex is a literal")
    });
    let header = |code: &str| {
        METHOD.captures(code).is_some_and(|c| {
            let rest = c.get(1).map_or("", |m| m.as_str());
            code.contains('{') || rest.matches('(').count() >= rest.matches(')').count()
        })
    };
    let lines: Vec<&str> = text.lines().collect();
    let literal = super::path_literal_lines(Kind::Jvm, path, text);
    let opener = (0..at_line1.min(lines.len())).rev().find(|&j| {
        let code = uncommented(Kind::Jvm, lines[j]);
        !literal.get(j).copied().unwrap_or(true)
            && indent(lines[j]) == 0
            && !code.trim().is_empty()
            && !code.starts_with(['}', ')', ']'])
    });
    opener.is_some_and(|j| {
        decl_line1 <= j
            && !header(&uncommented(Kind::Jvm, lines[j]))
            && jvm_enclosing_types(text, at_line1).is_empty()
    })
}
