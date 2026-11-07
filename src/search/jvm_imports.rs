use regex::Regex;

use super::{Kind, indent, literal_lines, steps_over};

pub struct JvmImport {
    pub bound_name: String,
    pub path: Vec<String>,
}
pub fn jvm_imports(text: &str) -> Vec<JvmImport> {
    imports_by_line(text).into_iter().map(|(_, i)| i).collect()
}

pub fn jvm_imports_at(text: &str, line1: usize) -> Vec<JvmImport> {
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(Kind::Jvm, text);
    let at = line1.saturating_sub(1);
    let in_block = |i: usize| {
        let depth = indent(lines[i]);
        i < at
            && (i + 1..=at).all(|k| {
                lines.get(k).is_none_or(|l| {
                    literal.get(k).copied().unwrap_or_default()
                        || steps_over(Some(Kind::Jvm), l.trim_start())
                        || indent(l) >= depth
                })
            })
    };
    let (mut inner, outer): (Vec<_>, Vec<_>) = imports_by_line(text)
        .into_iter()
        .partition(|&(i, _)| indent(lines[i]) > 0);
    inner.retain(|&(i, _)| in_block(i));
    inner.reverse();
    inner.into_iter().chain(outer).map(|(_, i)| i).collect()
}

fn imports_by_line(text: &str) -> Vec<(usize, JvmImport)> {
    static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*import\s+(?:static\s+)?([\w.]*\w)(\.\*)?(?:\s+as\s+([A-Za-z_]\w*))?\s*;?\s*(?://.*)?$").expect("the regex is a literal")
    });
    let literal = literal_lines(Kind::Jvm, text);
    text.lines()
        .enumerate()
        .filter(|&(i, _)| !literal.get(i).copied().unwrap_or_default())
        .filter_map(|(i, l)| Some((i, IMPORT.captures(l)?)))
        .map(|(i, c)| {
            let mut path: Vec<String> = c[1].split('.').map(str::to_owned).collect();
            let name = match (c.get(2), c.get(3)) {
                (Some(_), _) => "*".to_owned(),
                (None, Some(alias)) => alias.as_str().to_owned(),
                (None, None) => path.last().cloned().unwrap_or_default(),
            };
            if name == "*" {
                path.push(name.clone());
            }
            (
                i,
                JvmImport {
                    bound_name: name,
                    path,
                },
            )
        })
        .collect()
}

pub fn jvm_package(text: &str) -> Option<String> {
    static PACKAGE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?m)^[ \t]*package\s+([\w.]*\w)").expect("the regex is a literal")
    });
    PACKAGE
        .captures_iter(text)
        .map(|c| c[1].to_owned())
        .find(|p| p != "object")
}
