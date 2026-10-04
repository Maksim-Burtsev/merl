use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::*;

const LEAD: &str = r"^\s*(?:@\w+(?:\([^)]*\))?\s+)*";

pub fn gdscript_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    vec![
        format!(r"{LEAD}(?:static\s+)?func\s+{w}\s*\("),
        format!(r"{LEAD}(?:class_name|class|signal|enum)\s+{w}\b"),
        format!(r"{LEAD}(?:static\s+)?(?:var|const)\s+{w}\b"),
        format!(r"^\s*enum\b[^{{#]*\{{[^}}#]*\b{w}\b"),
        format!(r"^\s+(?:[A-Za-z_]\w*\s*(?:=\s*[^,#]+)?,\s*)*{w}\s*(?:=[^=]|,|\}}|#|$)"),
    ]
}

pub(super) const GDSCRIPT_SYMBOL: &str = concat!(
    r"^\s*(?:@\w+(?:\([^)]*\))?\s+)*(?:static\s+)?",
    r"(?:func|class_name|class|signal|enum)\s+(?P<name>[A-Za-z_]\w*)"
);

static MEMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"{LEAD}(?:static\s+)?(?:var|const|signal)\s")).unwrap());
static KEYWORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"{LEAD}(?:static\s+)?(?:func|class_name|class|enum)\b"
    ))
    .unwrap()
});
static FUNC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"{LEAD}(?:static\s+)?func\b\s*[A-Za-z_]?\w*\s*\(")).unwrap()
});
static LAMBDA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bfunc\b\s*[A-Za-z_]?\w*\s*\(").unwrap());
static LOCAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:var|const)\s+([A-Za-z_]\w*)").unwrap());
static FOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*for\s+([A-Za-z_]\w*)\s*(?::[^:]*)?\s+in\b").unwrap());

fn opener<S: AsRef<str>>(lines: &[S], line: usize) -> Option<&str> {
    let depth = indent(lines.get(line - 1)?.as_ref());
    lines[..line - 1].iter().map(AsRef::as_ref).rev().find(|l| {
        let t = l.trim_start();
        !t.is_empty() && !t.starts_with('#') && indent(l) < depth
    })
}

pub fn gdscript_declares<S: AsRef<str>>(lines: &[S], line: usize, line_text: &str) -> bool {
    if KEYWORD.is_match(line_text) {
        return true;
    }
    let class = |l: &str| l.trim_start().starts_with("class ");
    if MEMBER.is_match(line_text) {
        return indent(line_text) == 0 || opener(lines, line).is_some_and(class);
    }
    opener(lines, line).is_some_and(|l| l.trim_start().starts_with("enum"))
}

fn params(text: &str, open: usize) -> Vec<(usize, String)> {
    static NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*([A-Za-z_]\w*)").unwrap());
    let mut out = Vec::new();
    let (mut depth, mut start) = (0usize, open + 1);
    for (i, c) in text.char_indices().skip_while(|&(i, _)| i <= open) {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' if depth > 0 => depth -= 1,
            ',' | ')' if depth == 0 => {
                if let Some(m) = NAME.captures(&text[start..i]).and_then(|c| c.get(1)) {
                    out.push((start + m.start(), m.as_str().to_owned()));
                }
                start = i + 1;
                if c == ')' {
                    break;
                }
            }
            _ => {}
        }
    }
    out
}

fn bound_by_params(lines: &[&str], i: usize, open: usize, name: &str) -> Option<usize> {
    let text = lines[i..lines.len().min(i + 12)].join("\n");
    let (at, _) = params(&text, open).into_iter().find(|(_, n)| n == name)?;
    Some(i + 1 + text[..at].matches('\n').count())
}

fn lambda_binds(lines: &[&str], i: usize, code: &str, name: &str) -> Option<usize> {
    LAMBDA
        .find_iter(code)
        .find_map(|m| bound_by_params(lines, i, m.end() - 1, name))
}

pub(super) fn gdscript_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let literal = literal_lines(Kind::Gdscript, &lines.join("\n"));
    let found = |line: usize| {
        vec![Binding {
            line,
            value: Value::Unknown,
        }]
    };
    let mut limit = indent(lines[at]);
    let mut local = None;
    for i in (0..=at).rev() {
        if literal.get(i) == Some(&true) {
            continue;
        }
        let code = uncommented(Kind::Gdscript, lines[i]);
        let t = code.trim();
        if t.is_empty() {
            continue;
        }
        let depth = indent(&code);
        if i != at && depth > limit {
            continue;
        }
        let opens = i == at || depth < limit;
        if opens && let Some(m) = FUNC.find(&code) {
            let param = bound_by_params(lines, i, m.end() - 1, name);
            return local.or(param).map(found).unwrap_or_default();
        }
        if opens && t.starts_with("class ") {
            return Vec::new();
        }
        if local.is_none() {
            let declared = |c: Option<regex::Captures>| c.is_some_and(|c| &c[1] == name);
            if declared(LOCAL.captures(&code)) || (opens && declared(FOR.captures(&code))) {
                local = Some(i + 1);
            } else if opens {
                local = lambda_binds(lines, i, &code, name);
            }
        }
        if depth == 0 {
            return Vec::new();
        }
        limit = limit.min(depth);
    }
    Vec::new()
}

pub fn gdscript_path(line: &str, col: usize) -> Option<String> {
    static RES: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#""(res://[^"]+)"|'(res://[^']+)'"#).unwrap());
    let m = RES
        .captures_iter(line)
        .find(|c| c.get(0).is_some_and(|m| m.start() <= col && col < m.end()))?;
    m.get(1).or(m.get(2)).map(|p| p.as_str().to_owned())
}

pub fn gdscript_root(here: &Path, files: &[PathBuf]) -> Option<PathBuf> {
    here.parent()?
        .ancestors()
        .find(|d| files.contains(&d.join("project.godot")))
        .map(Path::to_path_buf)
}

pub fn gdscript_files(here: &Path, path: &str, files: &[PathBuf]) -> Vec<PathBuf> {
    let Some(rest) = path.strip_prefix("res://") else {
        return Vec::new();
    };
    let root = gdscript_root(here, files).unwrap_or_default();
    lexical(&root.join(rest))
        .filter(|f| files.contains(f))
        .into_iter()
        .collect()
}

pub fn gdscript_autoloads(project: &str) -> Vec<(String, String)> {
    let mut section = "";
    let mut out = Vec::new();
    for l in project.lines().map(str::trim) {
        if l.starts_with('[') {
            section = l;
        } else if section == "[autoload]"
            && let Some((name, value)) = l.split_once('=')
            && let Some(path) = value.trim().trim_matches('"').strip_prefix('*')
        {
            out.push((name.trim().to_owned(), path.to_owned()));
        }
    }
    out
}

pub fn gdscript_names_nothing(line: &str, start: usize) -> bool {
    let lead = &line[..start];
    let path = lead.trim_end_matches(|c: char| c == '_' || c == '/' || c.is_alphanumeric());
    let mut quote = None;
    let mut escaped = false;
    for c in lead.chars() {
        match quote {
            Some(_) if escaped => escaped = false,
            Some(_) if c == '\\' => escaped = true,
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '#' => return false,
            None if c == '"' || c == '\'' => quote = Some(c),
            None => {}
        }
    }
    let signal = ["emit_signal(\"", "emit_signal(&\""]
        .iter()
        .any(|s| lead.ends_with(s));
    path.ends_with(['$', '%']) || (quote.is_some() && !signal)
}
