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
    if call {
        vec![php_method(&w).replacen(r"^\s*", r"^\s+", 1), method_tag]
    } else {
        vec![property, promoted, property_tag]
    }
}

pub(super) fn php_assignment(w: &str) -> String {
    format!(r"^\s*\${w}\s*(?:\.|\?\?|\+)?=(?:$|[^=>])")
}

pub(super) fn php_bare_word_cut(patterns: &mut Vec<String>, line: &str, range: Range<usize>) {
    let (Some(word), Some(before), Some(after)) = (
        line.get(range.clone()),
        line.get(..range.start),
        line.get(range.end..),
    ) else {
        return;
    };
    if word.starts_with('$') || before.ends_with('$') {
        return;
    }
    let w = regex::escape(word);
    let mut cut = vec![php_assignment(&w)];
    let head = before.trim_end();
    if after.trim_start().starts_with('(') && !head.ends_with("->") && !head.ends_with("::") {
        cut.extend(php_properties(&w));
        cut.push(php_tags(&w)[0].clone());
    }
    patterns.retain(|p| !cut.contains(p));
}

const NAMESPACE_HEAD: &str = r"^\s*(?:<\?php\s+)?namespace\s+";

pub(super) fn php_namespace_line(w: &str) -> String {
    format!(r"{NAMESPACE_HEAD}(?:[\w\\]+\\)?{w}\s*[;{{]")
}

pub fn php_tag_class<S: AsRef<str>>(lines: &[S], at: usize) -> Option<usize> {
    static TAG: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*\*\s*@(?:property(?:-read|-write)?|method)\s")
            .expect("the regex is a literal")
    });
    static CLASS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:abstract|final|readonly)\s+)*(?:class|trait|interface|enum)\s+\w")
            .expect("the regex is a literal")
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
        depth += t.matches('[').count().cast_signed() - t.matches(']').count().cast_signed();
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
    static ON_NAMESPACE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(r"{NAMESPACE_HEAD}[\w\\]*$")).expect("the regex is a literal")
    });
    let (Some(word), Some(before), Some(after)) = (
        line.get(range.clone()),
        line.get(..range.start),
        line.get(range.end..),
    ) else {
        return;
    };
    let generic = php_namespace_line(&regex::escape(word));
    let Some(at) = patterns.iter().position(|p| *p == generic) else {
        return;
    };
    if !after.starts_with('\\') {
        if !(ON_NAMESPACE.is_match(before) && after.trim_start().starts_with([';', '{'])) {
            patterns.remove(at);
        }
        return;
    }
    let start = before
        .trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '\\')
        .len();
    let Some(spelled) = line.get(start..range.end) else {
        return;
    };
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
        Regex::new(r"^\s*(?:(?:abstract|final|readonly)\s+)*(class|trait|interface|enum)\s+(\w+)(?:\s*:\s*\w+)?(?:\s+extends\s+([\w\\]+))?").expect("the regex is a literal")
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
    static ANONYMOUS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\bnew\s+class\b").expect("the regex is a literal"));
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
    static CONSTRUCTOR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\bfunction\s+__construct\s*\(").expect("the regex is a literal")
    });
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
    static USE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s+use\s+([\w\\]+(?:\s*,\s*[\w\\]+)*)\s*[;{]")
            .expect("the regex is a literal")
    });
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

pub struct PhpTraitAlias {
    pub from: String,
    pub method: String,
    pub alias: String,
}

pub fn php_trait_aliases<S: AsRef<str>>(lines: &[S], class: usize) -> Vec<PhpTraitAlias> {
    static USE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s+use\s+[\w\\,\s]+\{").expect("the regex is a literal"));
    static ALIAS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"([\w\\]+)::(\w+)\s+as\s+(?:(?:public|protected|private)\s+)?(\w+)\s*;")
            .expect("the regex is a literal")
    });
    let mut out = Vec::new();
    for i in (class + 1..lines.len()).filter(|&i| php_outer_line(lines, i) == Some(class)) {
        let first = php_code(lines[i].as_ref());
        if !USE.is_match(&first) {
            continue;
        }
        let mut block = String::new();
        for l in lines[i..].iter().map(|l| php_code(l.as_ref())) {
            block.push_str(&l);
            block.push('\n');
            if l.contains('}') {
                break;
            }
        }
        out.extend(
            ALIAS
                .captures_iter(&block)
                .filter(|c| !matches!(&c[3], "public" | "protected" | "private"))
                .map(|c| PhpTraitAlias {
                    from: c[1].to_owned(),
                    method: c[2].to_owned(),
                    alias: c[3].to_owned(),
                }),
        );
    }
    out
}

static NAMESPACE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"(?m){NAMESPACE_HEAD}([\w\\]+)\s*[;{{]")).expect("the regex is a literal")
});

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
    let literal = literal_lines(Kind::Php, text);
    let declared: Vec<usize> = declared
        .into_iter()
        .filter(|&l| literal.get(l) != Some(&true))
        .collect();
    if declared.len() < 2 {
        return text;
    }
    let i = declared.iter().rposition(|&l| l <= line).unwrap_or(0);
    let end = declared.get(i + 1).map_or(text.len(), |&l| lines[l]);
    text.get(lines[declared[i]]..end).unwrap_or(text)
}

#[derive(Debug, PartialEq, Eq)]
pub struct PhpClassAt {
    pub written: String,
    pub member: bool,
}

pub fn php_class_at(line: &str, range: Range<usize>) -> Option<PhpClassAt> {
    static AFTER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:\b(?:new|extends|implements|instanceof|insteadof)|\bcatch\s*\(|\)\s*:\s*\??|\bimplements\s.*,)\s*$").expect("the regex is a literal")
    });
    static TYPED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*(?:&|\.\.\.)?\s*\$\w").expect("the regex is a literal"));
    let name_back = |s: &str| {
        let n = s.trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '\\');
        (n.len(), s.get(n.len()..).unwrap_or_default().to_owned())
    };
    let (before, word, after) = (
        line.get(..range.start)?,
        line.get(range.clone())?,
        line.get(range.end..)?,
    );
    let is_class = |at: usize, written: &str| {
        !written.trim_start_matches('\\').is_empty()
            && !written.starts_with(|c: char| c.is_ascii_digit())
            && !matches!(written, "self" | "static" | "parent")
            && line
                .get(..at)
                .is_some_and(|h| !h.ends_with(['$', '>']) && !h.ends_with("::"))
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
    let pre = line.get(..at)?.trim_end();
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
        .then(|| {
            if used && !written.starts_with('\\') {
                format!("\\{written}")
            } else {
                written
            }
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
    static USE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^use\s+([\w\\\s,]+?)\s*;").expect("the regex is a literal")
    });
    static GROUP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^use\s+([^;{]*)\{([^}]*)\}").expect("the regex is a literal")
    });
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
        GROUP.captures_iter(text).find_map(|c| {
            let head = c[1].trim().trim_start_matches('\\');
            c[2].split(',').find_map(|item| {
                let item = item.trim();
                let (path, alias) = item
                    .split_once(" as ")
                    .map_or((item, item), |(p, a)| (p.trim(), a.trim()));
                (alias.rsplit('\\').next() == Some(first)).then(|| {
                    (!head.starts_with("function ") && !head.starts_with("const "))
                        .then(|| format!("{head}{path}"))
                })
            })
        })
    };
    let tail = rest.map_or(String::new(), |r| format!("\\{r}"));
    let (full, imported) = match used.map(Some).or_else(grouped) {
        Some(None) => return None,
        Some(Some(path)) => (format!("{path}{tail}"), true),
        None => match php_namespace(text) {
            Some(ns) => (format!("{ns}\\{written}"), false),
            None => (written.to_owned(), false),
        },
    };
    Some(PhpResolved { full, imported })
}

pub fn php_new_receiver(before: &str) -> Option<String> {
    static NEW: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w$>:])\(\s*new\s+([\w\\]+)\s*(?:\([^()]*\))?\s*\)\s*(?:\?->|->)\s*$")
            .expect("the regex is a literal")
    });
    let written = NEW.captures(before)?.get(1)?.as_str();
    let short = written.rsplit('\\').next().unwrap_or(written);
    (!NO_CLASS.contains(&short.to_ascii_lowercase().as_str()) && short != "class")
        .then(|| written.to_owned())
}

pub fn php_function_at(line: &str, range: Range<usize>) -> Option<String> {
    let (before, word, after) = (
        line.get(..range.start)?,
        line.get(range.clone())?,
        line.get(range.end..)?,
    );
    if !after.trim_start().starts_with('(') {
        return None;
    }
    let pre = before.trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '\\');
    let written = format!("{}{word}", before.get(pre.len()..)?);
    let pre = pre.trim_end();
    let last = pre
        .trim_end_matches('&')
        .trim_end()
        .rsplit(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let not_a_call = pre.ends_with(['$', '>', ':'])
        || pre.ends_with("#[")
        || matches!(last.as_str(), "function" | "fn" | "new" | "use" | "const");
    (!not_a_call && !written.ends_with('\\')).then_some(written)
}

pub fn php_function_names(text: &str, written: &str) -> Vec<PhpResolved> {
    static USE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^use\s+function\s+([\w\\\s,]+?)\s*;").expect("the regex is a literal")
    });
    static GROUP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^use\s+[^;{]*\{([^}]*)\}").expect("the regex is a literal")
    });
    if let Some(full) = written.strip_prefix('\\') {
        return vec![PhpResolved {
            full: full.to_owned(),
            imported: false,
        }];
    }
    if written.contains('\\') {
        return php_resolve(text, written).into_iter().collect();
    }
    let used = USE
        .captures_iter(text)
        .flat_map(|c| {
            c[1].split(',')
                .map(|item| item.trim().to_owned())
                .collect::<Vec<_>>()
        })
        .find_map(|item| {
            let (path, alias) = match item.split_once(" as ") {
                Some((path, alias)) => (path.trim().to_owned(), alias.trim().to_owned()),
                None => (item.clone(), item.rsplit('\\').next()?.to_owned()),
            };
            (alias == written).then(|| path.trim_start_matches('\\').to_owned())
        });
    if let Some(full) = used {
        return vec![PhpResolved {
            full,
            imported: true,
        }];
    }
    let grouped = GROUP.captures_iter(text).any(|c| {
        c[1].split(',')
            .any(|item| item.trim().rsplit([' ', '\\']).next() == Some(written))
    });
    if grouped {
        return Vec::new();
    }
    let global = PhpResolved {
        full: written.to_owned(),
        imported: false,
    };
    match php_namespace(text) {
        Some(ns) => vec![
            PhpResolved {
                full: format!("{ns}\\{written}"),
                imported: false,
            },
            global,
        ],
        None => vec![global],
    }
}

pub fn php_psr4(root: &Path, dir: &Path) -> Vec<(String, Vec<PathBuf>)> {
    static MAP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#""psr-4"\s*:\s*\{([^}]*)\}"#).expect("the regex is a literal")
    });
    static ENTRY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#""((?:[^"\\]|\\.)*)"\s*:\s*(?:"((?:[^"\\]|\\.)*)"|\[([^\]]*)\])"#)
            .expect("the regex is a literal")
    });
    static STRING: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#""((?:[^"\\]|\\.)*)""#).expect("the regex is a literal"));
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
    if covered {
        Psr4File::Missing
    } else {
        Psr4File::OutsideProject
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhpBinding {
    Type(String),
    New(String),
    StaticCall { class: String, method: String },
    ThisCall(String),
    VarCall { var: String, method: String },
    FunctionCall(String),
    ThisProperty(String),
    Captured,
    Anonymous,
}

pub fn php_captures(line: &str, name: &str) -> bool {
    static USE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\bfunction\b[^{]*\buse\s*\(([^)]*)\)").expect("the regex is a literal")
    });
    let var = format!("${name}");
    let code = php_code(line);
    USE.captures(&code).is_some_and(|c| {
        c[1].split(',').any(|item| item.trim() == var)
            && !code
                .get(..c.get_match().start())
                .is_some_and(|head| head.contains(&var))
    })
}

pub fn php_last_binding(text: &str, from: usize, to: usize, name: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(Kind::Php, text);
    let Some(own) = lines.get(from).map(|l| php_code(l)) else {
        return false;
    };
    let own = own.trim();
    let assigns = own.strip_prefix(&format!("${name}")).is_some_and(|rest| {
        rest.trim_start().starts_with('=') && !rest.trim_start().starts_with("==")
    });
    let opens_statement = (0..from)
        .rev()
        .filter(|&i| literal.get(i) != Some(&true))
        .map(|i| php_code(lines[i]).trim().to_owned())
        .find(|l| !l.is_empty())
        .is_none_or(|l| l.ends_with([';', '{', '}']));
    if !assigns || !opens_statement || !own.ends_with(';') || from >= to {
        return false;
    }
    let mut depth = 0i32;
    for (i, line) in lines.iter().enumerate().take(to).skip(from + 1) {
        if literal.get(i) == Some(&true) {
            continue;
        }
        for c in php_code(line).bytes() {
            match c {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            if depth < 0 {
                return false;
            }
        }
    }
    depth == 0
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
    let before = line.get(..m.start())?.trim_end();
    if before.ends_with(['|', '&', '.', '$', ':', '>', '-', '=']) {
        return None;
    }
    let written = m.as_str().trim_start_matches('?');
    let short = written.rsplit('\\').next().unwrap_or(written);
    (!short.is_empty() && !NO_CLASS.contains(&short.to_ascii_lowercase().as_str()))
        .then(|| written.to_owned())
}

pub fn php_param_tag(lines: &[&str], at: usize, name: &str) -> Option<String> {
    static FUNCTION: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\bfunction\b").expect("the regex is a literal"));
    static PARAM: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"@param\s+(\S+)\s+(?:\.\.\.)?&?\$(\w+)\b").expect("the regex is a literal")
    });
    let header = (at.saturating_sub(10)..=at)
        .rev()
        .find(|&i| lines.get(i).is_some_and(|l| FUNCTION.is_match(l)))?;
    if lines[header..at].iter().any(|l| l.contains(['{', ';'])) {
        return None;
    }
    lines[..header]
        .iter()
        .rev()
        .map(|l| l.trim_start())
        .take_while(|l| l.starts_with('*') || l.starts_with("/**") || l.starts_with("#["))
        .find_map(|l| PARAM.captures(l).filter(|c| &c[2] == name))
        .map(|c| c[1].to_owned())
}

pub fn php_tag_type(line: &str, name: &str) -> Option<String> {
    static TAG: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*\*\s*@property(?:-read|-write)?\s+(\S+)\s+\$(\w+)\b")
            .expect("the regex is a literal")
    });
    php_one_class(&TAG.captures(line).filter(|c| &c[2] == name)?[1])
}

pub fn php_one_class(types: &str) -> Option<String> {
    let mut types = types
        .split('|')
        .map(|t| t.trim_start_matches('?'))
        .filter(|t| !t.eq_ignore_ascii_case("null"));
    let written = types.next().filter(|_| types.next().is_none())?;
    let short = written.rsplit('\\').next().unwrap_or(written);
    (!short.is_empty()
        && written
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '\\')
        && !NO_CLASS.contains(&short.to_ascii_lowercase().as_str()))
    .then(|| written.to_owned())
}

pub fn php_binding(statement: &str, name: &str) -> Option<PhpBinding> {
    static NEW: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^new\s+([\w\\]+)\s*").expect("the regex is a literal"));
    static STATIC: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^([\w\\]+)::(\w+)\s*\(").expect("the regex is a literal"));
    static THIS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\$this->(\w+)\s*\(").expect("the regex is a literal"));
    static VAR_CALL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\$(\w+)->(\w+)\s*\(").expect("the regex is a literal"));
    static PROPERTY: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\$this->(\w+)\s*;").expect("the regex is a literal"));
    static FUNCTION: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^([\w\\]+)\s*\(").expect("the regex is a literal"));
    let first = statement.lines().next().unwrap_or_default();
    let n = regex::escape(name);
    let assign = Regex::new(&format!(r"(?:^|[^\w$:>])\${n}\s*=([^=>]|$)")).ok()?;
    if let Some(t) = php_written_type(first, name) {
        return Some(PhpBinding::Type(t));
    }
    if let Some(made) = php_destructured(first, name) {
        return Some(made);
    }
    let at = assign.captures(first)?.get(1)?.start();
    let value = statement.get(at..)?.trim_start();
    let call_ends_statement = |open: usize| {
        let close = close_of(Kind::Php, value, open)?;
        value
            .get(close..)?
            .trim_start()
            .starts_with(';')
            .then_some(())
    };
    if value.starts_with("new") && php_anonymous_class(first).is_some() {
        return Some(PhpBinding::Anonymous);
    }
    if let Some(c) = NEW.captures(value) {
        let class = c[1].to_owned();
        let end = c.get_match().end();
        let rest = value.get(end..)?;
        if rest.starts_with('(') {
            call_ends_statement(end)?;
        } else {
            rest.starts_with(';').then_some(())?;
        }
        let lower = class.to_ascii_lowercase();
        return (lower != "static" && lower != "class" && lower != "parent")
            .then_some(PhpBinding::New(class));
    }
    if let Some(c) = PROPERTY.captures(value) {
        return Some(PhpBinding::ThisProperty(c[1].to_owned()));
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
    if let Some(c) = VAR_CALL.captures(value)
        && !value
            .get(c.get_match().end()..)
            .is_some_and(|r| r.trim_start().starts_with("..."))
    {
        return Some(PhpBinding::VarCall {
            var: c[1].to_owned(),
            method: c[2].to_owned(),
        });
    }
    let c = FUNCTION.captures(value)?;
    let short = c[1]
        .rsplit('\\')
        .next()
        .unwrap_or(&c[1])
        .to_ascii_lowercase();
    (!NO_CLASS.contains(&short.as_str())).then(|| PhpBinding::FunctionCall(c[1].to_owned()))
}

fn php_destructured(line: &str, name: &str) -> Option<PhpBinding> {
    static LIST: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:\[|list\s*\()([^=\[\]()]*)[\])]\s*=\s*\[(.*)\]\s*;\s*$")
            .expect("the regex is a literal")
    });
    static NEW: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^new\s+([\w\\]+)\s*(?:\([^()]*\))?$").expect("the regex is a literal")
    });
    let c = LIST.captures(line)?;
    let var = format!("${name}");
    let at = c[1].split(',').position(|v| v.trim() == var)?;
    let items = &c[2];
    if c[1].contains("=>") || items.contains(['\'', '"', '#']) || items.contains("//") {
        return None;
    }
    let mut depth = 0i32;
    let mut parts = vec![String::new()];
    for ch in items.chars() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(String::new());
                continue;
            }
            _ => {}
        }
        parts.last_mut()?.push(ch);
    }
    NEW.captures(parts.get(at)?.trim())
        .map(|n| n[1].to_owned())
        .filter(|t| {
            !matches!(
                t.to_ascii_lowercase().as_str(),
                "static" | "class" | "parent"
            )
        })
        .map(PhpBinding::New)
}

pub fn php_anonymous_class(line: &str) -> Option<PhpClassHeader<'_>> {
    static ANONYMOUS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"=\s*new\s+class\b(?:\s*\([^()]*\))?(?:\s+extends\s+([\w\\]+))?(?:\s+implements\s+[\w\\,\s]+)?\s*\{\s*$")
            .expect("the regex is a literal")
    });
    ANONYMOUS.captures(line).map(|c| PhpClassHeader {
        keyword: "class",
        name: "class",
        extends: c.get(1).map(|m| m.as_str()),
    })
}

pub fn php_return_written(header: &str) -> Option<&str> {
    static RETURNS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*:\s*\??([\w\\]+)\s*(?:\{|;|$)").expect("the regex is a literal")
    });
    let function = header.find("function")?;
    let open = function + header.get(function..)?.find('(')?;
    let close = close_of(Kind::Php, header, open)?;
    Some(RETURNS.captures(header.get(close..)?)?.get(1)?.as_str())
}

pub fn php_return_type(header: &str) -> Option<String> {
    let written = php_return_written(header)?;
    let short = written.rsplit('\\').next().unwrap_or(written);
    (!NO_CLASS.contains(&short.to_ascii_lowercase().as_str())).then(|| written.to_owned())
}

pub enum PhpCallHead {
    This,
    Var(String),
    New(String),
    Callable(String),
}

pub fn php_call_chain(before: &str) -> Option<(PhpCallHead, Vec<String>)> {
    static VAR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w$:>])\$(\w+)(\(\))?$").expect("the regex is a literal")
    });
    let mut rest = before.strip_suffix('.')?;
    let mut calls = Vec::new();
    while let Some((head, name)) = php_call_segment(rest) {
        calls.insert(0, name.to_owned());
        rest = head;
    }
    let head = if let Some(written) = php_new_receiver(&format!("{rest}->")) {
        PhpCallHead::New(written)
    } else {
        let c = VAR.captures(rest)?;
        match (&c[1], c.get(2)) {
            ("this", None) => PhpCallHead::This,
            ("this", Some(_)) => return None,
            (v, None) => PhpCallHead::Var(v.to_owned()),
            (v, Some(_)) => PhpCallHead::Callable(v.to_owned()),
        }
    };
    (!calls.is_empty() || matches!(head, PhpCallHead::Callable(_))).then_some((head, calls))
}

pub fn php_first_class_callable(line: &str, name: &str) -> Option<(String, String)> {
    let re = Regex::new(&format!(
        r"^\s*\${}\s*=\s*\$(\w+)->(\w+)\(\s*\.\.\.\s*\)\s*;\s*$",
        regex::escape(name)
    ))
    .ok()?;
    let c = re.captures(line)?;
    Some((c[1].to_owned(), c[2].to_owned()))
}

fn php_call_segment(rest: &str) -> Option<(&str, &str)> {
    let inner = rest.strip_suffix(')')?;
    let mut depth = 1;
    let (open, _) = inner.char_indices().rev().find(|&(_, c)| {
        match c {
            ')' => depth += 1,
            '(' => depth -= 1,
            _ => {}
        }
        depth == 0
    })?;
    let (head, args) = inner.split_at_checked(open)?;
    if args.contains(['\'', '"']) || args.trim() == "(..." {
        return None;
    }
    let name_at = head
        .trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_')
        .len();
    let (head, name) = head.split_at_checked(name_at)?;
    let head = head.strip_suffix('.')?;
    (!name.is_empty() && !name.starts_with(|c: char| c.is_ascii_digit())).then_some((head, name))
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
    static HEAD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w$:>])(?:function|fn)\b([^{]*?)(?:\{|=>|$)")
            .expect("the regex is a literal")
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
        .filter_map(|c| c.get(1))
        .filter_map(|g| var.find(g.as_str()).map(|m| g.start() + m.end()))
        .last()?;
    Some(top + statement.get(..at_var)?.matches('\n').count())
}
fn php_parameter_lines(header: &[String], brace_byte: usize, var: &Regex) -> Vec<usize> {
    let last = header.len() - 1;
    let mut after_function = false;
    let mut out = Vec::new();
    for (i, l) in header.iter().enumerate() {
        let l = if i == last {
            let Some(head) = l.get(..brace_byte) else {
                break;
            };
            head
        } else {
            l.as_str()
        };
        let l = match FUNCTION.find_iter(l).last() {
            Some(m) => {
                after_function = true;
                l.get(m.end()..).unwrap_or_default()
            }
            None => l,
        };
        if after_function && var.is_match(l) {
            out.push(i);
        }
    }
    out
}
static FUNCTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|[^\w$:>])function\b").expect("the regex is a literal"));
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
#[expect(
    clippy::string_slice,
    reason = "pos is the byte of an ASCII brace, and cut the byte of an ASCII ; { or }"
)]
fn php_scope(lines: &[String], literal: &[bool], at: usize) -> PhpScope {
    static CLASS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w$:>])(?:class|interface|trait|enum)\b")
            .expect("the regex is a literal")
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
