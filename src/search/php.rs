use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::symbols::php_mods;
use super::syntax::{close_of, literal_lines};
use super::words::steps_over;
use super::{Kind, indent};

pub(super) fn php_method(w: &str) -> String {
    format!(r"{}function\s+&?\s*{w}\s*\(", php_mods!())
}

pub(super) fn php_properties(w: &str) -> [String; 2] {
    [
        format!(r"{}(?:\??[\w\\|]+\s+)?\${w}\b", php_mods!("+")),
        format!(
            r"function\s+__construct\s*\(.*\b(?:public|private|protected|readonly)\s+(?:\??[\w\\|]+\s+)?\${w}\b"
        ),
    ]
}

pub(super) fn php_constants(w: &str) -> [String; 2] {
    [
        format!(r"{}const\s+(?:[\w\\|&?()]+\s+)?{w}\b", php_mods!()),
        format!(r"^\s*case\s+{w}\s*(?:=[^=]|;|$)"),
    ]
}

pub(super) fn php_tags(w: &str) -> [String; 2] {
    [
        format!(r"^\s*\*\s*@property(?:-read|-write)?\s+(?:[^$]*\s)?\${w}\b"),
        format!(r"^\s*\*\s*@method\s+(?:[^(]*\s)?{w}\s*\("),
    ]
}

pub fn php_member_patterns(word: &str, call: bool) -> Vec<String> {
    let w = regex::escape(word);
    let [property, promoted] = php_properties(&w);
    let [property_tag, method_tag] = php_tags(&w);
    match call {
        true => vec![php_method(&w).replacen(r"^\s*", r"^\s+", 1), method_tag],
        false => vec![property, promoted, property_tag],
    }
}

const NAMESPACE_HEAD: &str = r"^\s*(?:<\?php\s+)?namespace\s+";

pub(super) fn php_namespace_line(w: &str) -> String {
    format!(r"{NAMESPACE_HEAD}(?:[\w\\]+\\)?{w}\s*[;{{]")
}

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

pub fn php_namespace_patterns(
    patterns: &mut Vec<String>,
    text: &str,
    line: &str,
    range: Range<usize>,
) {
    static ON_NAMESPACE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(&format!(r"{NAMESPACE_HEAD}[\w\\]*$")).unwrap());
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
        .trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '\\')
        .len();
    let spelled = &line[start..range.end];
    let head = before.trim_start();
    let absolute = spelled.starts_with('\\')
        || head.starts_with("use ")
        || head
            .trim_start_matches("<?php")
            .trim_start()
            .starts_with("namespace ");
    let mut name = spelled.trim_start_matches('\\').to_owned();
    if !absolute && let Some(own) = php_namespace(text) {
        name = format!("{own}\\{name}");
    }
    *patterns = vec![format!(r"{NAMESPACE_HEAD}{}\s*[;{{]", regex::escape(&name))];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhpAccess {
    Call,
    Property,
    Constant,
}

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

pub struct PhpClassHeader<'a> {
    pub keyword: &'a str,
    pub name: &'a str,
    pub extends: Option<&'a str>,
}

pub fn php_class_header(line: &str) -> Option<PhpClassHeader<'_>> {
    static HEADER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:abstract|final|readonly)\s+)*(class|trait|interface|enum)\s+(\w+)(?:\s*:\s*\w+)?(?:\s+extends\s+([\w\\]+))?").unwrap()
    });
    let c = HEADER.captures(line)?;
    Some(PhpClassHeader {
        keyword: c.get(1)?.as_str(),
        name: c.get(2)?.as_str(),
        extends: c.get(3).map(|m| m.as_str()),
    })
}

fn php_outer_line<S: AsRef<str>>(lines: &[S], at: usize) -> Option<usize> {
    let depth = indent(lines.get(at)?.as_ref());
    (0..at).rev().find(|&i| {
        let l = lines[i].as_ref();
        !steps_over(Some(Kind::Php), l.trim_start()) && indent(l) < depth
    })
}

pub fn php_enclosing_class<S: AsRef<str>>(lines: &[S], at: usize) -> Option<usize> {
    static ANONYMOUS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bnew\s+class\b").unwrap());
    let mut at = at;
    loop {
        at = php_outer_line(lines, at)?;
        let l = lines[at].as_ref();
        if ANONYMOUS.is_match(l) {
            return None;
        }
        if php_class_header(l).is_some() {
            return Some(at);
        }
    }
}

pub fn php_class_members<S: AsRef<str>>(lines: &[S], class: usize, re: &Regex) -> Vec<usize> {
    static CONSTRUCTOR: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\bfunction\s+__construct\s*\(").unwrap());
    (0..lines.len())
        .filter(|&i| re.is_match(lines[i].as_ref()))
        .filter(|&i| match php_outer_line(lines, i) {
            Some(o) if o == class => true,
            Some(o) if CONSTRUCTOR.is_match(lines[o].as_ref()) => {
                php_outer_line(lines, o) == Some(class)
            }
            _ => php_tag_class(lines, i) == Some(class),
        })
        .collect()
}

pub fn php_class_traits<S: AsRef<str>>(lines: &[S], class: usize) -> Vec<String> {
    static USE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s+use\s+([\w\\]+(?:\s*,\s*[\w\\]+)*)\s*[;{]").unwrap());
    (class + 1..lines.len())
        .filter(|&i| php_outer_line(lines, i) == Some(class))
        .filter_map(|i| USE.captures(lines[i].as_ref()))
        .flat_map(|c| {
            c[1].split(',')
                .map(|t| t.trim().to_owned())
                .collect::<Vec<_>>()
        })
        .collect()
}

static NAMESPACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"(?m){NAMESPACE_HEAD}([\w\\]+)\s*[;{{]")).unwrap());

pub fn php_namespace(text: &str) -> Option<&str> {
    NAMESPACE
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
}

pub fn php_block(text: &str, line: usize) -> &str {
    let mut lines = vec![0];
    lines.extend(text.match_indices('\n').map(|(i, _)| i + 1));
    let declared: Vec<usize> = NAMESPACE
        .captures_iter(text)
        .map(|c| lines.partition_point(|&o| o <= c.get(1).map_or(0, |m| m.start())) - 1)
        .collect();
    if declared.len() < 2 {
        return text;
    }
    let literal = super::syntax::literal_lines(Kind::Php, text);
    let declared: Vec<usize> = declared
        .into_iter()
        .filter(|&l| literal.get(l) != Some(&true))
        .collect();
    if declared.len() < 2 {
        return text;
    }
    let i = declared.iter().rposition(|&l| l <= line).unwrap_or(0);
    let end = declared.get(i + 1).map_or(text.len(), |&l| lines[l]);
    &text[lines[declared[i]]..end]
}

#[derive(Debug, PartialEq, Eq)]
pub struct PhpClassAt {
    pub written: String,
    pub member: bool,
}

pub fn php_class_at(line: &str, range: Range<usize>) -> Option<PhpClassAt> {
    static AFTER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:\b(?:new|extends|implements|instanceof|insteadof)|\bcatch\s*\(|\)\s*:\s*\??|\bimplements\s.*,)\s*$").unwrap()
    });
    static TYPED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*(?:&|\.\.\.)?\s*\$\w").unwrap());
    let name_back = |s: &str| {
        let n = s.trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '\\');
        (n.len(), s[n.len()..].to_owned())
    };
    let (before, word, after) = (
        &line[..range.start],
        &line[range.clone()],
        &line[range.end..],
    );
    let is_class = |at: usize, written: &str| {
        !written.trim_start_matches('\\').is_empty()
            && !written.starts_with(|c: char| c.is_ascii_digit())
            && !matches!(written, "self" | "static" | "parent")
            && !line[..at].ends_with(['$', '>'])
            && !line[..at].ends_with("::")
    };
    if let Some(b) = before
        .strip_suffix("::$")
        .or_else(|| before.strip_suffix("::"))
    {
        let (at, written) = name_back(b);
        return (word != "class" && is_class(at, &written)).then_some(PhpClassAt {
            written,
            member: true,
        });
    }
    if after.starts_with(|c: char| c == '\\' || c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let (at, head) = name_back(before);
    let written = format!("{head}{word}");
    let pre = line[..at].trim_end();
    let used = line.starts_with("use ")
        && !line.contains('{')
        && !["use function ", "use const "]
            .iter()
            .any(|u| line.starts_with(u));
    let typed = TYPED.is_match(after)
        && (pre.is_empty()
            || pre.ends_with(['(', ',', '?', '|'])
            || ["public", "protected", "private", "readonly"]
                .iter()
                .any(|m| pre.ends_with(m)));
    (is_class(at, &written)
        && (used || after.trim_start().starts_with("::") || AFTER.is_match(pre) || typed))
        .then(|| match used && !written.starts_with('\\') {
            true => format!("\\{written}"),
            false => written,
        })
        .map(|written| PhpClassAt {
            written,
            member: false,
        })
}

#[derive(Debug, PartialEq, Eq)]
pub struct PhpResolved {
    pub full: String,
    pub imported: bool,
}

pub fn php_resolve(text: &str, written: &str) -> Option<PhpResolved> {
    static USE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?m)^use\s+([\w\\\s,]+?)\s*;").unwrap());
    static GROUP: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?m)^use\s+[^;{]*\{([^}]*)\}").unwrap());
    if let Some(full) = written.strip_prefix('\\') {
        return Some(PhpResolved {
            full: full.to_owned(),
            imported: false,
        });
    }
    let (first, rest) = match written.split_once('\\') {
        Some((first, rest)) => (first, Some(rest)),
        None => (written, None),
    };
    let used = USE
        .captures_iter(text)
        .flat_map(|c| {
            c[1].split(',')
                .map(|item| item.trim().to_owned())
                .collect::<Vec<_>>()
        })
        .filter(|item| !item.starts_with("function ") && !item.starts_with("const "))
        .find_map(|item| {
            let (path, alias) = match item.split_once(" as ") {
                Some((path, alias)) => (path.trim().to_owned(), alias.trim().to_owned()),
                None => (item.clone(), item.rsplit('\\').next()?.to_owned()),
            };
            (alias == first).then(|| path.trim_start_matches('\\').to_owned())
        });
    let grouped = || {
        GROUP.captures_iter(text).any(|c| {
            c[1].split(',').any(|item| {
                let item = item.trim();
                let alias = item.split_once(" as ").map_or(item, |(_, a)| a.trim());
                alias.rsplit('\\').next() == Some(first)
            })
        })
    };
    let tail = rest.map_or(String::new(), |r| format!("\\{r}"));
    let (full, imported) = match used {
        Some(path) => (format!("{path}{tail}"), true),
        None if grouped() => return None,
        None => match php_namespace(text) {
            Some(ns) => (format!("{ns}\\{written}"), false),
            None => (written.to_owned(), false),
        },
    };
    Some(PhpResolved { full, imported })
}

pub fn php_psr4(root: &Path, dir: &Path) -> Vec<(String, Vec<PathBuf>)> {
    static MAP: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#""psr-4"\s*:\s*\{([^}]*)\}"#).unwrap());
    static ENTRY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#""((?:[^"\\]|\\.)*)"\s*:\s*(?:"((?:[^"\\]|\\.)*)"|\[([^\]]*)\])"#).unwrap()
    });
    static STRING: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#""((?:[^"\\]|\\.)*)""#).unwrap());
    let unescape = |s: &str| s.replace("\\\\", "\\").replace("\\/", "/");
    let mut map = Vec::new();
    for at in dir.ancestors() {
        let Ok(text) = std::fs::read_to_string(root.join(at).join("composer.json")) else {
            continue;
        };
        let dir_of = |d: &str| {
            let d = unescape(d);
            let d = d.trim_start_matches("./").trim_end_matches('/');
            match d {
                "" | "." => at.to_path_buf(),
                d => at.join(d),
            }
        };
        for m in MAP.captures_iter(&text) {
            for c in ENTRY.captures_iter(&m[1]) {
                let dirs = match (c.get(2), c.get(3)) {
                    (Some(one), _) => vec![dir_of(one.as_str())],
                    (_, Some(list)) => STRING
                        .captures_iter(list.as_str())
                        .map(|s| dir_of(&s[1]))
                        .collect(),
                    _ => Vec::new(),
                };
                map.push((unescape(&c[1]).trim_start_matches('\\').to_owned(), dirs));
            }
        }
    }
    map.sort_by_key(|(prefix, _)| std::cmp::Reverse(prefix.len()));
    map
}

#[derive(Debug, PartialEq, Eq)]
pub enum Psr4File {
    Found(PathBuf),
    Missing,
    OutsideProject,
}

pub fn php_psr4_file(
    map: &[(String, Vec<PathBuf>)],
    full: &str,
    has: impl Fn(&Path) -> bool,
) -> Psr4File {
    let mut covered = false;
    for (prefix, dirs) in map {
        let Some(rest) = full.strip_prefix(prefix.as_str()) else {
            continue;
        };
        covered |= !prefix.is_empty();
        if let Some(file) = dirs
            .iter()
            .map(|d| d.join(format!("{}.php", rest.replace('\\', "/"))))
            .find(|f| has(f))
        {
            return Psr4File::Found(file);
        }
    }
    match covered {
        true => Psr4File::Missing,
        false => Psr4File::OutsideProject,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhpBinding {
    Type(String),
    New(String),
    StaticCall { class: String, method: String },
    ThisCall(String),
    FunctionCall(String),
}

const NO_CLASS: &[&str] = &[
    "array",
    "bool",
    "callable",
    "false",
    "float",
    "int",
    "iterable",
    "mixed",
    "never",
    "null",
    "object",
    "parent",
    "resource",
    "static",
    "string",
    "true",
    "void",
    "public",
    "private",
    "protected",
    "readonly",
    "var",
    "global",
    "return",
    "echo",
    "print",
    "yield",
    "clone",
    "new",
    "as",
    "case",
    "throw",
    "else",
    "and",
    "or",
    "xor",
    "instanceof",
    "include",
    "require",
    "fn",
    "function",
    "use",
    "list",
    "isset",
    "unset",
    "empty",
    "match",
];

pub fn php_written_type(line: &str, name: &str) -> Option<String> {
    let re = Regex::new(&format!(r"(\??[\w\\]+)\s+&?\${}\b", regex::escape(name))).ok()?;
    let c = re.captures(line)?;
    let m = c.get(1)?;
    let before = line[..m.start()].trim_end();
    if before.ends_with(['|', '&', '.', '$', ':', '>', '-', '=']) {
        return None;
    }
    let written = m.as_str().trim_start_matches('?');
    let short = written.rsplit('\\').next().unwrap_or(written);
    (!short.is_empty() && !NO_CLASS.contains(&short.to_ascii_lowercase().as_str()))
        .then(|| written.to_owned())
}

pub fn php_binding(statement: &str, name: &str) -> Option<PhpBinding> {
    static NEW: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^new\s+([\w\\]+)\s*").unwrap());
    static STATIC: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^([\w\\]+)::(\w+)\s*\(").unwrap());
    static THIS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\$this->(\w+)\s*\(").unwrap());
    static FUNCTION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([\w\\]+)\s*\(").unwrap());
    let first = statement.lines().next().unwrap_or_default();
    let n = regex::escape(name);
    let assign = Regex::new(&format!(r"(?:^|[^\w$:>])\${n}\s*=([^=>]|$)")).ok()?;
    if let Some(t) = php_written_type(first, name) {
        return Some(PhpBinding::Type(t));
    }
    let at = assign.captures(first)?.get(1)?.start();
    let value = statement[at..].trim_start();
    let call_ends_statement = |open: usize| {
        let close = close_of(Kind::Php, value, open)?;
        value[close..].trim_start().starts_with(';').then_some(())
    };
    if let Some(c) = NEW.captures(value) {
        let class = c[1].to_owned();
        let rest = c.get(0)?.end();
        match value[rest..].starts_with('(') {
            true => call_ends_statement(rest)?,
            false => value[rest..].starts_with(';').then_some(())?,
        }
        let lower = class.to_ascii_lowercase();
        return (lower != "static" && lower != "class" && lower != "parent")
            .then_some(PhpBinding::New(class));
    }
    let open = value.find('(')?;
    call_ends_statement(open)?;
    if let Some(c) = STATIC.captures(value) {
        let lower = c[1].to_ascii_lowercase();
        return (lower != "static" && lower != "parent").then(|| PhpBinding::StaticCall {
            class: c[1].to_owned(),
            method: c[2].to_owned(),
        });
    }
    if let Some(c) = THIS.captures(value) {
        return Some(PhpBinding::ThisCall(c[1].to_owned()));
    }
    let c = FUNCTION.captures(value)?;
    let short = c[1]
        .rsplit('\\')
        .next()
        .unwrap_or(&c[1])
        .to_ascii_lowercase();
    (!NO_CLASS.contains(&short.as_str())).then(|| PhpBinding::FunctionCall(c[1].to_owned()))
}

pub fn php_return_type(header: &str) -> Option<String> {
    static RETURNS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*:\s*\??([\w\\]+)\s*(?:\{|;|$)").unwrap());
    let open = header.find("function")? + header[header.find("function")?..].find('(')?;
    let close = close_of(Kind::Php, header, open)?;
    let written = RETURNS.captures(&header[close..])?.get(1)?.as_str();
    let short = written.rsplit('\\').next().unwrap_or(written);
    (!NO_CLASS.contains(&short.to_ascii_lowercase().as_str())).then(|| written.to_owned())
}

pub(super) fn php_code(l: &str) -> String {
    let mut out = vec![b' '; l.len()];
    for (i, c) in super::syntax::code(Kind::Php, l).take_while(|&(_, c)| c != 0) {
        out[i] = c;
    }
    String::from_utf8_lossy(&out).into_owned()
}
pub fn php_variable(text: &str, line: usize, name: &str) -> Option<Vec<usize>> {
    let raw: Vec<&str> = text.lines().collect();
    let at = line.checked_sub(1).filter(|&i| i < raw.len())?;
    let literal = literal_lines(Kind::Php, text);
    let code_lines: Vec<String> = raw.iter().map(|l| php_code(l)).collect();
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let var = rule(format!(r"(?:^|[^\w$:>])\${n}\b"));
    if let Some(bound) = php_parameter_of_statement_ending_at(&code_lines, &literal, at, &var) {
        let promoted = rule(format!(
            r"(?:(?:public|private|protected|readonly)\s+)+(?:\??[\w\\|]+\s+)?\${n}\b"
        ));
        let property_under_cursor = bound == at && promoted.is_match(&code_lines[at]);
        return (!property_under_cursor).then(|| vec![bound + 1]);
    }
    let binds = [
        format!(r"(?:^|[^\w$:>])\${n}\s*=(?:[^=>]|$)"),
        format!(r"\bforeach\s*\(.*\bas\b.*\${n}\b"),
        format!(r"\bcatch\s*\([^)]*\${n}\b"),
        format!(r"^\s*(?:global|static)\s+[^;(]*\${n}\b"),
        format!(r"(?:^|[^\w$])(?:list\s*\(|\[)[^;=]*\${n}\b[^;=]*[\])]\s*=(?:[^=>]|$)"),
    ]
    .map(rule);
    let scope = php_scope(&code_lines, &literal, at);
    let mut out = Vec::new();
    let from = match scope {
        PhpScope::Class => return None,
        PhpScope::TopOrNamespace => 0,
        PhpScope::Function {
            header_line,
            body_line,
            brace_byte,
        } => {
            let header = &code_lines[header_line..=body_line];
            out.extend(
                php_parameter_lines(header, brace_byte, &var)
                    .into_iter()
                    .map(|i| header_line + i + 1),
            );
            body_line + 1
        }
    };
    out.extend(
        (from..=at)
            .filter(|&i| !literal[i] && binds.iter().any(|b| b.is_match(&code_lines[i])))
            .filter(|&i| i == at || php_scope(&code_lines, &literal, i) == scope)
            .map(|i| i + 1),
    );
    Some(out)
}
fn php_parameter_of_statement_ending_at(
    code_lines: &[String],
    literal: &[bool],
    at: usize,
    var: &Regex,
) -> Option<usize> {
    static HEAD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w$:>])(?:function|fn)\b([^{]*?)(?:\{|=>|$)").unwrap()
    });
    let mut top = at;
    while top > 0 && at - top < 50 && !literal[top - 1] && {
        let t = code_lines[top - 1].trim();
        !t.is_empty() && !t.ends_with([';', '{', '}'])
    } {
        top -= 1;
    }
    let statement = code_lines[top..=at].join("\n");
    let at_var = HEAD
        .captures_iter(&statement)
        .filter_map(|c| var.find(&c[1]).map(|m| c.get(1).unwrap().start() + m.end()))
        .last()?;
    Some(top + statement[..at_var].matches('\n').count())
}
fn php_parameter_lines(header: &[String], brace_byte: usize, var: &Regex) -> Vec<usize> {
    let last = header.len() - 1;
    let mut after_function = false;
    let mut out = Vec::new();
    for (i, l) in header.iter().enumerate() {
        let l = if i == last { &l[..brace_byte] } else { &l[..] };
        let l = match FUNCTION.find_iter(l).last() {
            Some(m) => {
                after_function = true;
                &l[m.end()..]
            }
            None => l,
        };
        if after_function && var.is_match(l) {
            out.push(i);
        }
    }
    out
}
static FUNCTION: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"(?:^|[^\w$:>])function\b").unwrap());
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhpScope {
    TopOrNamespace,
    Class,
    Function {
        header_line: usize,
        body_line: usize,
        brace_byte: usize,
    },
}
fn php_scope(lines: &[String], literal: &[bool], at: usize) -> PhpScope {
    static CLASS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w$:>])(?:class|interface|trait|enum)\b").unwrap()
    });
    let mut depth = 0usize;
    for i in (0..at).rev().filter(|&i| !literal[i]) {
        for (pos, c) in lines[i]
            .bytes()
            .enumerate()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            match c {
                b'}' => depth += 1,
                b'{' if depth > 0 => depth -= 1,
                b'{' => {
                    let own = &lines[i][..pos];
                    let cut = own.rfind([';', '{', '}']);
                    let mut start = i;
                    let mut header = own[cut.map_or(0, |c| c + 1)..].to_owned();
                    while cut.is_none() && start > 0 && i - start < 50 {
                        let t = lines[start - 1].trim();
                        if literal[start - 1] || t.is_empty() || t.ends_with([';', '{', '}']) {
                            break;
                        }
                        start -= 1;
                        header = format!("{t}\n{header}");
                    }
                    if FUNCTION.is_match(&header) {
                        return PhpScope::Function {
                            header_line: start,
                            body_line: i,
                            brace_byte: pos,
                        };
                    }
                    if CLASS.is_match(&header) {
                        return PhpScope::Class;
                    }
                    if header.trim_start().starts_with("namespace") {
                        return PhpScope::TopOrNamespace;
                    }
                }
                _ => {}
            }
        }
    }
    PhpScope::TopOrNamespace
}
