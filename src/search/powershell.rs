use std::path::{Path, PathBuf};

use regex::Regex;

use super::*;

const SCOPE: &str = r"(?:(?:global|script|local|private):)?";
const TYPES: &str = r"(?:\[[^\]]*\]\s*)*";

pub fn powershell_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    let mut out = vec![String::new(); SHAPES];
    out[Shape::FunctionOrFilter as usize] =
        format!(r"(?i)^\s*(?:function|filter)\s+{SCOPE}{w}\s*(?:[({{#]|$)");
    out[Shape::ClassOrEnum as usize] =
        format!(r"(?i)^\s*{TYPES}(?:class|enum)\s+{w}\s*(?:[:{{#]|$)");
    out[Shape::AssignmentOrProperty as usize] = format!(
        r"(?i)^\s*(?:(?:hidden|static)\s+)*{TYPES}\${SCOPE}{w}\s*(?:[-+*/%]?=(?:[^=]|$)|;|#|$)"
    );
    out[Shape::MethodOrConstructor as usize] =
        format!(r"(?i)^\s*(?:(?:hidden|static)\s+)*{TYPES}{w}\s*\(");
    out[Shape::EnumMember as usize] = format!(r"(?i)^\s+{w}\s*(?:=\s*[^=\s]|#|$)");
    out[Shape::Alias as usize] =
        format!(r#"(?i)^\s*(?:Set|New)-Alias\s+(?:-Name\s+)?["']?{w}["']?(?:\s|$)"#);
    out[Shape::StaticProperty as usize] = format!(
        r"(?i)^\s*(?:hidden\s+)*static\s+(?:hidden\s+)*{TYPES}\${w}\s*(?:=(?:[^=]|$)|;|#|$)"
    );
    out[Shape::EnvironmentVariable as usize] = format!(r"(?i)^\s*\$env:{w}\s*[-+]?=(?:[^=]|$)");
    out
}

#[derive(Clone, Copy)]
enum Shape {
    FunctionOrFilter,
    ClassOrEnum,
    AssignmentOrProperty,
    MethodOrConstructor,
    EnumMember,
    Alias,
    StaticProperty,
    EnvironmentVariable,
}

const SHAPES: usize = Shape::EnvironmentVariable as usize + 1;

pub fn powershell_sigil(patterns: &mut Vec<String>, before: &str, after: &str) {
    static SIGIL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?i)(?:[$@]|\$(?:global|script|local|private|using):)$").unwrap()
    });
    use Shape::*;
    if patterns.len() != SHAPES {
        return;
    }
    let keep: &[Shape] = match before {
        b if b.to_ascii_lowercase().ends_with("$env:") => &[EnvironmentVariable],
        b if SIGIL.is_match(b) => &[AssignmentOrProperty],
        b if b.ends_with('.') => &[AssignmentOrProperty, MethodOrConstructor],
        b if b.ends_with("::") => &[
            FunctionOrFilter,
            ClassOrEnum,
            MethodOrConstructor,
            EnumMember,
            Alias,
            StaticProperty,
        ],
        _ if after.starts_with("]::new(") => &[
            FunctionOrFilter,
            ClassOrEnum,
            MethodOrConstructor,
            EnumMember,
            Alias,
        ],
        _ => &[FunctionOrFilter, ClassOrEnum, EnumMember, Alias],
    };
    let all = std::mem::take(patterns);
    patterns.extend(keep.iter().map(|&s| all[s as usize].clone()));
}

pub fn powershell_declares_here(line: &str, word: &str) -> bool {
    let part = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    let Some(at) = line
        .match_indices(word)
        .map(|(i, _)| i)
        .find(|&i| !line[..i].ends_with(part) && !line[i + word.len()..].starts_with(part))
    else {
        return false;
    };
    let mut patterns = powershell_patterns(word);
    powershell_sigil(&mut patterns, &line[..at], &line[at + word.len()..]);
    Regex::new(&patterns.join("|")).is_ok_and(|re| re.is_match(line))
}

pub fn powershell_argument(before: &str) -> bool {
    before
        .strip_suffix('-')
        .is_some_and(|b| b.is_empty() || b.ends_with([' ', '\t', '(']))
}

pub fn powershell_declares<S: AsRef<str>>(lines: &[S], line1: usize, line_text: &str) -> bool {
    static KEYWORD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?i)^\s*(?:(?:function|filter)\s|(?:\[[^\]]*\]\s*)*(?:class|enum)\s|(?:Set|New)-Alias\s)").unwrap()
    });
    if KEYWORD.is_match(line_text) {
        return true;
    }
    if VARIABLE.is_match(line_text) {
        return !in_param_block(lines, line1);
    }
    if METHOD.is_match(line_text) {
        return in_class(lines, line1);
    }
    member_of_an_enum(lines, line1)
}

static VARIABLE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?i)^\s*(?:(?:hidden|static)\s+)*(?:\[[^\]]*\]\s*)*\$").unwrap()
});
static METHOD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?i)^\s*(?:(?:hidden|static)\s+)*(?:\[[^\]]*\]\s*)*[A-Za-z_][\w-]*\s*\(").unwrap()
});

fn opens(l: &str, keyword: &str) -> bool {
    static OPENER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?i)^\s*(?:\[[^\]]*\]\s*)*(class|enum)\s").unwrap()
    });
    OPENER
        .captures(l)
        .is_some_and(|c| c[1].eq_ignore_ascii_case(keyword))
}

fn in_class<S: AsRef<str>>(lines: &[S], line1: usize) -> bool {
    owner(lines, line1).is_some_and(|o| opens(o, "class"))
}

fn member_of_an_enum<S: AsRef<str>>(lines: &[S], line1: usize) -> bool {
    owner(lines, line1).is_some_and(|o| opens(o, "enum"))
}

fn params_at<S: AsRef<str>>(lines: &[S], line0: usize) -> Option<usize> {
    let l = lines[line0].as_ref();
    PARAM
        .find(l)
        .or_else(|| METHOD.find(l).filter(|_| in_class(lines, line0 + 1)))
        .map(|m| m.end() - 1)
}

fn owner<S: AsRef<str>>(lines: &[S], line1: usize) -> Option<&str> {
    let depth = indent(lines.get(line1.checked_sub(1)?)?.as_ref());
    lines[..line1 - 1]
        .iter()
        .map(AsRef::as_ref)
        .rev()
        .find(|l| {
            let t = l.trim_start();
            !t.is_empty() && !t.starts_with('#') && indent(l) < depth
        })
}

fn in_param_block<S: AsRef<str>>(lines: &[S], line1: usize) -> bool {
    let Some(k) = line1.checked_sub(1) else {
        return false;
    };
    let Some(open) = (0..k)
        .rev()
        .take(80)
        .find(|&i| PARAM.is_match(lines[i].as_ref()))
    else {
        return false;
    };
    let mut depth = 0i32;
    for l in &lines[open..k] {
        let l = l.as_ref();
        let from = match PARAM.find(l) {
            Some(m) => m.end() - 1,
            None => 0,
        };
        depth += brackets(&l[from..]);
    }
    depth > 0
}

static PARAM: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?i)\bparam\s*\(|^\s*(?:function|filter)\s+[\w:-]+\s*\(").unwrap()
});

fn brackets(s: &str) -> i32 {
    let (mut depth, mut quote) = (0, None);
    for c in s.bytes() {
        match (quote, c) {
            (Some(q), _) if c == q => quote = None,
            (Some(_), _) => {}
            (None, b'"' | b'\'') => quote = Some(c),
            (None, b'#') => break,
            (None, b'(') => depth += 1,
            (None, b')') => depth -= 1,
            _ => {}
        }
    }
    depth
}

pub fn powershell_params(lines: &[&str], cursor_line0: usize, name: &str) -> Vec<Binding> {
    let var = Regex::new(&format!(r"(?i)\${}(?:[^\w-]|$)", regex::escape(name)))
        .expect("an escaped name keeps the pattern valid");
    let mut depth = indent(lines[cursor_line0]);
    for i in (0..=cursor_line0).rev() {
        let l = lines[i];
        if l.trim().is_empty() || indent(l) > depth {
            continue;
        }
        let open = params_at(lines, i);
        let function_or_method =
            HEADER.is_match(l) || (METHOD.is_match(l) && in_class(lines, i + 1));
        let beside_the_cursors = i != cursor_line0 && indent(l) == depth && function_or_method;
        depth = indent(l);
        let Some(from) = open.filter(|_| !beside_the_cursors) else {
            continue;
        };
        let mut open = 0;
        for (n, b) in lines.iter().enumerate().skip(i).take(80) {
            let from = if n == i { from } else { 0 };
            if var.is_match(&b[from..]) {
                return vec![Binding {
                    line1: n + 1,
                    value: Value::Unknown,
                }];
            }
            open += brackets(&b[from..]);
            if open <= 0 {
                break;
            }
        }
    }
    Vec::new()
}

static HEADER: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"(?i)^\s*(?:function|filter)\s").unwrap());

pub fn powershell_import(line: &str, byte_col: usize) -> Option<String> {
    static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r#"(?i)^\s*(?:\.|Import-Module|using\s+module)\s+(?:-Name\s+)?(["']?)([^"'\s;|]+\.ps[dm]?1)["']?"#,
        )
        .unwrap()
    });
    let c = IMPORT.captures(line)?;
    let path = c.get(2)?;
    if !(c.get(1)?.start() <= byte_col && byte_col <= path.end()) {
        return None;
    }
    let p = path.as_str().replace('\\', "/");
    let p = ["$PSScriptRoot/", "${PSScriptRoot}/"]
        .iter()
        .find_map(|r| {
            (p.len() >= r.len() && p[..r.len()].eq_ignore_ascii_case(r)).then(|| &p[r.len()..])
        })
        .unwrap_or(&p);
    Some(p.trim_start_matches("./").to_owned())
}

pub(super) fn powershell_roots(
    env: Option<std::ffi::OsString>,
    home: &Path,
    pwsh: Option<PathBuf>,
) -> Vec<PathBuf> {
    if let Some(env) = env.filter(|e| !e.is_empty()) {
        return std::env::split_paths(&env).collect();
    }
    let beside = pwsh
        .and_then(|p| std::fs::canonicalize(p).ok())
        .and_then(|p| Some(p.parent()?.join("Modules")));
    [
        home.join(".local/share/powershell/Modules"),
        PathBuf::from("/usr/local/share/powershell/Modules"),
    ]
    .into_iter()
    .chain(beside)
    .collect()
}

pub(super) const POWERSHELL_SYMBOL: &str = r"(?i)^\s*(?:(?:function|filter)\s+(?:(?:global|script|local|private):)?|(?:\[[^\]]*\]\s*)*(?:class|enum)\s+)(?P<name>[A-Za-z_][\w-]*)";
