use regex::Regex;

use super::{Kind, indent, path_literal_lines, steps_over};
use std::path::Path;

pub struct JvmImport {
    pub bound_name: String,
    pub path: Vec<String>,
    pub selected: bool,
}
pub fn jvm_imports(path: &Path, text: &str) -> Vec<JvmImport> {
    imports_by_line(path, text)
        .into_iter()
        .map(|(_, i)| i)
        .collect()
}

pub fn jvm_imports_at(path: &Path, text: &str, line1: usize) -> Vec<JvmImport> {
    let lines: Vec<&str> = text.lines().collect();
    let literal = path_literal_lines(Kind::Jvm, path, text);
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
    let (mut inner, outer): (Vec<_>, Vec<_>) = imports_by_line(path, text)
        .into_iter()
        .partition(|&(i, _)| indent(lines[i]) > 0);
    inner.retain(|&(i, _)| in_block(i));
    inner.reverse();
    inner.into_iter().chain(outer).map(|(_, i)| i).collect()
}

fn imports_by_line(path: &Path, text: &str) -> Vec<(usize, JvmImport)> {
    static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*import\s+(?:static\s+)?([\w.]*\w)(\.\*)?(?:\s+as\s+([A-Za-z_]\w*))?\s*;?\s*(?://.*)?$").expect("the regex is a literal")
    });
    let literal = path_literal_lines(Kind::Jvm, path, text);
    let lines = || {
        text.lines()
            .enumerate()
            .filter(|&(i, _)| !literal.get(i).copied().unwrap_or_default())
    };
    let mut imports: Vec<(usize, JvmImport)> = lines()
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
                    selected: false,
                },
            )
        })
        .collect();
    if super::scala(path) {
        imports
            .extend(lines().flat_map(|(i, l)| scala_renames(l).into_iter().map(move |s| (i, s))));
        imports.sort_by_key(|&(i, _)| i);
    }
    imports
}

fn scala_renames(line: &str) -> Vec<JvmImport> {
    static SELECTORS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*import\s+([\w.]*\w)\.\{([^}]*)\}\s*;?\s*(?://.*)?$")
            .expect("the regex is a literal")
    });
    static RENAME: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(\w+)\s*(?:(?:=>|\bas\b)\s*([A-Za-z]\w*)\s*)?$")
            .expect("the regex is a literal")
    });
    let Some(c) = SELECTORS.captures(line) else {
        return Vec::new();
    };
    let base: Vec<String> = c[1].split('.').map(str::to_owned).collect();
    c[2].split(',')
        .filter_map(|s| RENAME.captures(s))
        .filter(|r| r.get(2).is_some() || !["_", "given"].contains(&&r[1]))
        .map(|r| {
            let mut path = base.clone();
            path.push(r[1].to_owned());
            let bound = r.get(2).map_or(&r[1], |m| m.as_str());
            JvmImport {
                bound_name: bound.to_owned(),
                path,
                selected: r.get(2).is_none(),
            }
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

const JAVA_LANG_ANNOTATIONS: [&str; 5] = [
    "Override",
    "Deprecated",
    "SuppressWarnings",
    "FunctionalInterface",
    "SafeVarargs",
];

const KOTLIN_ANNOTATIONS: [&str; 32] = [
    "Deprecated",
    "ReplaceWith",
    "DeprecatedSinceKotlin",
    "Suppress",
    "PublishedApi",
    "DslMarker",
    "OptIn",
    "RequiresOptIn",
    "SinceKotlin",
    "UnsafeVariance",
    "ExtensionFunctionType",
    "ParameterName",
    "BuilderInference",
    "Target",
    "Retention",
    "Repeatable",
    "MustBeDocumented",
    "JvmStatic",
    "JvmField",
    "JvmOverloads",
    "JvmName",
    "JvmMultifileClass",
    "JvmSuppressWildcards",
    "JvmWildcard",
    "JvmSynthetic",
    "JvmInline",
    "JvmRecord",
    "Throws",
    "Synchronized",
    "Volatile",
    "Transient",
    "Strictfp",
];

pub fn jvm_builtin_annotation(path: &Path, before: &str, word: &str) -> bool {
    static USE_SITE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"@(?:file|property|field|get|set|receiver|param|setparam|delegate)\s*:$")
            .expect("the regex is a literal")
    });
    match path.extension().and_then(|e| e.to_str()) {
        Some("java") => before.ends_with('@') && JAVA_LANG_ANNOTATIONS.contains(&word),
        Some("kt" | "kts") => {
            (before.ends_with('@') || USE_SITE.is_match(before))
                && (KOTLIN_ANNOTATIONS.contains(&word) || JAVA_LANG_ANNOTATIONS.contains(&word))
        }
        _ => false,
    }
}

pub struct JvmEnumImport {
    pub owner: String,
    pub single: bool,
}

pub fn jvm_enum_imports(path: &Path, text: &str, word: &str) -> Vec<JvmEnumImport> {
    let java = path.extension().is_some_and(|e| e == "java");
    let kotlin = path.extension().is_some_and(|e| e == "kt" || e == "kts");
    let scala = super::scala(path);
    if !java && !kotlin && !scala {
        return Vec::new();
    }
    let keyword = if java { r"static\s+" } else { "" };
    let end = if java { r"\s*;" } else { r"\s*;?" };
    let (lead, wildcard) = if scala {
        ("", r"\*|_")
    } else {
        (r"\s*", r"\*")
    };
    let Ok(import) = Regex::new(&format!(
        r"^{lead}import\s+{keyword}([\w.]+)\.({wildcard}|{})(?:{end})\s*(?://.*)?$",
        regex::escape(word)
    )) else {
        return Vec::new();
    };
    let literal = path_literal_lines(Kind::Jvm, path, text);
    text.lines()
        .enumerate()
        .filter(|&(i, _)| !literal.get(i).copied().unwrap_or_default())
        .filter_map(|(_, l)| import.captures(l))
        .filter(|c| {
            c[1].rsplit('.')
                .next()
                .is_some_and(|o| o.starts_with(|c: char| c.is_ascii_uppercase()))
        })
        .map(|c| JvmEnumImport {
            owner: c[1].to_owned(),
            single: &c[2] == word,
        })
        .collect()
}

pub fn java_case_label(before: &str) -> bool {
    static CASE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*case\s+(?:\w+\s*,\s*)*$").expect("the regex is a literal")
    });
    CASE.is_match(before)
}

pub fn java_switch_selector(text: &str, line1: usize) -> Option<String> {
    static SWITCH: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bswitch\s*\(\s*([A-Za-z_]\w*)\s*\)\s*\{\s*$").expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    let switch = super::jvm::scope_of(&lines, at)?;
    Some(SWITCH.captures(lines[switch])?[1].to_owned())
}
