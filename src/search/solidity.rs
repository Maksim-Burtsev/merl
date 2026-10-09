use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::*;

const NAME: &str = r"[A-Za-z_$][\w$]*";
const TYPE: &str =
    r"(?:mapping\s*\(.*\)|[A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*)*)(?:\s*\[[^\]]*\])*";
const MODS: &str = r"(?:public|private|internal|external|constant|immutable|transient|override(?:\s*\([^)]*\))?|memory|storage|calldata|payable|virtual)";
const STATEMENTS: &[&str] = &[
    "return",
    "emit",
    "revert",
    "delete",
    "else",
    "using",
    "import",
    "pragma",
    "new",
    "throw",
    "case",
    "do",
    "assembly",
    "unchecked",
];

pub fn solidity_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    vec![
        format!(r"^\s*(?:abstract\s+)?(?:contract|interface|library)\s+{w}\b"),
        format!(r"^\s*(?:function|modifier)\s+{w}\b"),
        format!(r"^\s*(?:event|error)\s+{w}\s*\("),
        format!(r"^\s*(?:struct|enum)\s+{w}\b"),
        format!(r"^\s*type\s+{w}\s+is\b"),
        format!(r"^\s*enum\s+{NAME}\s*\{{(?:[^}}]*,)?\s*{w}\s*[,}}]"),
        format!(r"^\s+{w}\s*,?\s*(?://.*)?$"),
        solidity_variable(&w),
    ]
}

fn solidity_variable(w: &str) -> String {
    format!(
        r"^\s*{TYPE}(?:\s+{MODS})*\s+{w}\s*(?:;|=(?:[^=>]|$))|^\s*\((?:[^()]*,)?\s*{NAME}(?:\.{NAME})*(?:\[[^\]]*\])*(?:\s+(?:memory|storage|calldata))?\s+{w}\s*[,)][^;]*=[^=]|^\s*let\s+(?:{NAME}\s*,\s*)*{w}\s*(?:,[^:]*)?:="
    )
}

pub(super) const SOLIDITY_SYMBOL: &str =
    r"^\s*(?:(?:abstract\s+)?contract|library|modifier|event|error)\s+(?P<name>[A-Za-z_$][\w$]*)";

pub fn solidity_declares<S: AsRef<str>>(lines: &[S], line: usize, line_text: &str) -> bool {
    static BARE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s+[A-Za-z_$][\w$]*\s*,?\s*(?://.*)?$").unwrap());
    let first = line_text.split_whitespace().next().unwrap_or_default();
    !STATEMENTS.contains(&first)
        && (!BARE.is_match(line_text) || directly_inside(lines, line, "enum "))
}

pub struct SolidityImport {
    pub bound: String,
    pub name: Option<String>,
    pub path: String,
}

pub fn solidity_imports(text: &str) -> Vec<SolidityImport> {
    static IMPORT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(
            r#"(?m)^\s*import\s*(?:\{{([^}}]*)\}}\s*from\s*|\*\s*as\s+({NAME})\s+from\s*)?["']([^"']+)["'](?:\s*as\s+({NAME}))?\s*;"#
        ))
        .unwrap()
    });
    let mut out = Vec::new();
    for c in IMPORT.captures_iter(text) {
        let path = c[3].to_owned();
        if let Some(q) = c.get(2).or_else(|| c.get(4)) {
            out.push(SolidityImport {
                bound: q.as_str().to_owned(),
                name: None,
                path: path.clone(),
            });
        }
        for item in c.get(1).map_or("", |m| m.as_str()).split(',') {
            let mut it = item.split_whitespace();
            let Some(name) = it.next() else { continue };
            let bound = match (it.next(), it.next()) {
                (Some("as"), Some(alias)) => alias,
                _ => name,
            };
            out.push(SolidityImport {
                bound: bound.to_owned(),
                name: Some(name.to_owned()),
                path: path.clone(),
            });
        }
    }
    out
}

pub fn solidity_import_path(line: &str, col: usize) -> Option<String> {
    static IMPORT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^\s*import\b[^"']*["']([^"']+)["']"#).unwrap());
    let path = IMPORT.captures(line)?.get(1)?;
    (path.start() <= col + 1 && col <= path.end()).then(|| path.as_str().to_owned())
}

pub fn solidity_file(root: &Path, files: &[PathBuf], here: &Path, path: &str) -> Option<PathBuf> {
    let found = |p: PathBuf| -> Option<PathBuf> {
        let p = lexical(&p)?;
        let p = p.strip_prefix(root).map_or(p.clone(), Path::to_path_buf);
        (files.contains(&p) || root.join(&p).is_file()).then_some(p)
    };
    if path.starts_with("./") || path.starts_with("../") {
        return found(here.parent()?.join(path));
    }
    let remapped = solidity_remappings(root)
        .into_iter()
        .filter(|(from, _)| path.starts_with(from.as_str()))
        .max_by_key(|(from, _)| from.len())
        .map(|(from, to)| PathBuf::from(format!("{to}{}", &path[from.len()..])));
    let dir = root.join(here.parent().unwrap_or(Path::new("")));
    remapped
        .into_iter()
        .chain(node_modules(root, &dir).into_iter().map(|d| d.join(path)))
        .chain([Path::new("lib").join(path), PathBuf::from(path)])
        .find_map(found)
}

fn solidity_remappings(root: &Path) -> Vec<(String, String)> {
    static LIST: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)\bremappings\s*=\s*\[(.*?)\]").unwrap());
    static ENTRY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"["']([^"']+)["']"#).unwrap());
    let read = |name: &str| std::fs::read_to_string(root.join(name)).unwrap_or_default();
    let toml = read("foundry.toml");
    let listed: Vec<String> = LIST
        .captures_iter(&toml)
        .flat_map(|c| {
            ENTRY
                .captures_iter(&c[1])
                .map(|e| e[1].to_owned())
                .collect::<Vec<_>>()
        })
        .collect();
    read("remappings.txt")
        .lines()
        .map(str::to_owned)
        .chain(listed)
        .filter_map(|l| {
            let (from, to) = l.trim().split_once('=')?;
            let from = from.rsplit_once(':').map_or(from, |(_, f)| f);
            (!from.is_empty()).then(|| (from.to_owned(), to.trim().to_owned()))
        })
        .collect()
}

const HEADS: &[&str] = &["function", "modifier", "constructor", "fallback", "receive"];

fn solidity_head(t: &str) -> bool {
    HEADS.iter().any(|h| {
        t.strip_prefix(h)
            .is_some_and(|rest| rest.starts_with([' ', '(', '\t']) || rest.is_empty())
    })
}

fn solidity_params(head: &str) -> Vec<std::ops::Range<usize>> {
    let (mut depth, mut open, mut lists) = (0, 0, Vec::new());
    for (i, c) in head.char_indices() {
        match c {
            '(' => {
                if depth == 0 {
                    open = i + 1;
                }
                depth += 1;
            }
            ')' if depth > 0 => {
                depth -= 1;
                let returns = head[..open - 1].trim_end().ends_with("returns");
                if depth == 0 && (lists.is_empty() || returns) {
                    lists.push(open..i);
                }
            }
            '{' | ';' if depth == 0 => break,
            _ => {}
        }
    }
    lists
}

pub(super) fn solidity_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let w = regex::escape(name);
    let local = Regex::new(&solidity_variable(&w)).expect("an escaped name");
    let each = Regex::new(&format!(r"^for\s*\(\s*{TYPE}\s+{w}\s*[=;]")).expect("an escaped name");
    let literal = literal_lines(Kind::Solidity, &lines.join("\n"));
    let found = |line: usize| {
        vec![Binding {
            line1: line + 1,
            value: Value::Unknown,
        }]
    };
    let declares = |i: usize| local.is_match(lines[i]) && solidity_declares(lines, i + 1, lines[i]);
    let params = |i: usize| -> Option<usize> {
        let span: Vec<&str> = (i..lines.len().min(i + 12))
            .take_while(|&j| j == i || !lines[j - 1].contains(['{', ';']))
            .map(|j| lines[j].split("//").next().unwrap_or_default())
            .collect();
        let head = span.join("\n");
        let at = solidity_params(&head).into_iter().find_map(|list| {
            let mut from = list.start;
            head[list].split(',').find_map(|item| {
                let start = from;
                from += item.len() + 1;
                let last = item.trim_end().rsplit(char::is_whitespace).next()?;
                (last == name && item.split_whitespace().count() > 1)
                    .then(|| start + item.trim_end().len() - last.len())
            })
        })?;
        Some(i + head[..at].matches('\n').count())
    };
    let own = lines[at].trim();
    if solidity_head(own) {
        return params(at).map(found).unwrap_or_default();
    }
    let mut hit = (declares(at) || each.is_match(own)).then_some(at);
    let (mut depth, mut wrapped) = (indent(lines[at]), own.starts_with(')'));
    for i in (0..at).rev() {
        let t = lines[i].trim();
        let ind = indent(lines[i]);
        if t.is_empty() || literal[i] || t.starts_with("//") || ind > depth {
            continue;
        }
        if solidity_head(t) && (ind < depth || wrapped) {
            return hit.or_else(|| params(i)).map(found).unwrap_or_default();
        }
        if ind == depth {
            if declares(i) {
                hit.get_or_insert(i);
            }
            continue;
        }
        if each.is_match(t) {
            hit.get_or_insert(i);
        }
        if t.starts_with(['}', ')']) {
            wrapped = t.starts_with(')');
        } else if t == "{" {
            wrapped = true;
        } else if ["contract ", "library ", "interface ", "abstract "]
            .iter()
            .any(|k| t.starts_with(k))
        {
            return Vec::new();
        }
        depth = ind;
    }
    Vec::new()
}
