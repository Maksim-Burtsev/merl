use std::collections::HashMap;
use std::ops::ControlFlow;
use std::path::Path;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use ratatui::style::Style;
use syntect::highlighting::Highlighter;
use syntect::parsing::Scope;
use tree_sitter::{
    InputEdit, Language, ParseOptions, ParseState, Parser, Point, Query, QueryCursor,
    StreamingIterator, Tree,
};

use crate::buffer::{Buffer, Spans};
use crate::theme::Theme;

const NEOVIM_TREESITTER_PRIORITY: u32 = 100;

const QUERIES: &[(&str, &str, &str)] = &[
    (
        "bash",
        include_str!("../queries/bash/highlights.scm"),
        include_str!("../queries/bash/injections.scm"),
    ),
    (
        "c",
        include_str!("../queries/c/highlights.scm"),
        include_str!("../queries/c/injections.scm"),
    ),
    (
        "c_sharp",
        include_str!("../queries/c_sharp/highlights.scm"),
        include_str!("../queries/c_sharp/injections.scm"),
    ),
    (
        "cpp",
        include_str!("../queries/cpp/highlights.scm"),
        include_str!("../queries/cpp/injections.scm"),
    ),
    (
        "css",
        include_str!("../queries/css/highlights.scm"),
        include_str!("../queries/css/injections.scm"),
    ),
    (
        "diff",
        include_str!("../queries/diff/highlights.scm"),
        include_str!("../queries/diff/injections.scm"),
    ),
    (
        "ecma",
        include_str!("../queries/ecma/highlights.scm"),
        include_str!("../queries/ecma/injections.scm"),
    ),
    (
        "go",
        include_str!("../queries/go/highlights.scm"),
        include_str!("../queries/go/injections.scm"),
    ),
    (
        "json",
        include_str!("../queries/json/highlights.scm"),
        include_str!("../queries/json/injections.scm"),
    ),
    (
        "html",
        include_str!("../queries/html/highlights.scm"),
        include_str!("../queries/html/injections.scm"),
    ),
    (
        "html_tags",
        include_str!("../queries/html_tags/highlights.scm"),
        include_str!("../queries/html_tags/injections.scm"),
    ),
    (
        "java",
        include_str!("../queries/java/highlights.scm"),
        include_str!("../queries/java/injections.scm"),
    ),
    (
        "kotlin",
        include_str!("../queries/kotlin/highlights.scm"),
        include_str!("../queries/kotlin/injections.scm"),
    ),
    (
        "javascript",
        include_str!("../queries/javascript/highlights.scm"),
        include_str!("../queries/javascript/injections.scm"),
    ),
    ("jsdoc", include_str!("../queries/jsdoc/highlights.scm"), ""),
    (
        "jsx",
        include_str!("../queries/jsx/highlights.scm"),
        include_str!("../queries/jsx/injections.scm"),
    ),
    (
        "rust",
        include_str!("../queries/rust/highlights.scm"),
        include_str!("../queries/rust/injections.scm"),
    ),
    (
        "python",
        include_str!("../queries/python/highlights.scm"),
        include_str!("../queries/python/injections.scm"),
    ),
    (
        "markdown",
        include_str!("../queries/markdown/highlights.scm"),
        include_str!("../queries/markdown/injections.scm"),
    ),
    (
        "markdown_inline",
        include_str!("../queries/markdown_inline/highlights.scm"),
        include_str!("../queries/markdown_inline/injections.scm"),
    ),
    ("regex", include_str!("../queries/regex/highlights.scm"), ""),
    (
        "printf",
        include_str!("../queries/printf/highlights.scm"),
        "",
    ),
    (
        "query",
        include_str!("../queries/query/highlights.scm"),
        include_str!("../queries/query/injections.scm"),
    ),
    (
        "toml",
        include_str!("../queries/toml/highlights.scm"),
        include_str!("../queries/toml/injections.scm"),
    ),
    (
        "tsx",
        include_str!("../queries/tsx/highlights.scm"),
        include_str!("../queries/tsx/injections.scm"),
    ),
    (
        "swift",
        include_str!("../queries/swift/highlights.scm"),
        include_str!("../queries/swift/injections.scm"),
    ),
    (
        "typescript",
        include_str!("../queries/typescript/highlights.scm"),
        include_str!("../queries/typescript/injections.scm"),
    ),
    (
        "xml",
        include_str!("../queries/xml/highlights.scm"),
        include_str!("../queries/xml/injections.scm"),
    ),
    (
        "yaml",
        include_str!("../queries/yaml/highlights.scm"),
        include_str!("../queries/yaml/injections.scm"),
    ),
    (
        "lua",
        include_str!("../queries/lua/highlights.scm"),
        include_str!("../queries/lua/injections.scm"),
    ),
    (
        "luadoc",
        include_str!("../queries/luadoc/highlights.scm"),
        "",
    ),
];

const ALIASES: &[(&str, &str)] = &[
    ("sh", "bash"),
    ("jsonc", "json"),
    ("csharp", "c_sharp"),
    ("py", "python"),
    ("gyp", "python"),
    ("pandoc", "markdown"),
    ("javascriptreact", "javascript"),
    ("ecma", "javascript"),
    ("ecmascript", "javascript"),
    ("jsx", "javascript"),
    ("js", "javascript"),
    ("typescriptreact", "tsx"),
    ("ts", "typescript"),
    ("xsd", "xml"),
    ("xslt", "xml"),
    ("svg", "xml"),
];

const PREDICATES: &[&str] = &[
    "eq?",
    "not-eq?",
    "any-of?",
    "not-any-of?",
    "lua-match?",
    "not-lua-match?",
    "match?",
    "not-match?",
    "vim-match?",
    "not-vim-match?",
    "contains?",
    "not-contains?",
    "has-parent?",
    "not-has-parent?",
    "has-ancestor?",
    "not-has-ancestor?",
    "set!",
    "offset!",
    "gsub!",
];

const HIGHLIGHT_KEYS: &[&str] = &["priority", "conceal", "conceal_lines"];
const INJECTION_KEYS: &[&str] = &[
    "injection.language",
    "injection.combined",
    "injection.include-children",
    "injection.self",
];
const CAPTURE_KEYS: &[&str] = &["url", "conceal", "bo.commentstring"];

type Offset = (u32, [i64; 4]);
type Rewrite = (u32, regex::bytes::Regex, Vec<u8>);

struct Check {
    capture: u32,
    test: Test,
    positive: bool,
}

enum Test {
    Match(regex::bytes::Regex),
    Contains(Vec<String>),
    HasParent(Vec<String>),
    HasAncestor(Vec<String>),
}

impl Check {
    fn passes(&self, m: &tree_sitter::QueryMatch, bytes: &[u8]) -> bool {
        let mut nodes = m.nodes_for_capture_index(self.capture).peekable();
        if nodes.peek().is_none() {
            return self.positive;
        }
        let hit = match &self.test {
            Test::Match(re) => nodes.all(|n| re.is_match(&bytes[n.byte_range()])),
            Test::Contains(words) => nodes.all(|n| {
                let text = &bytes[n.byte_range()];
                words
                    .iter()
                    .all(|w| text.windows(w.len()).any(|x| x == w.as_bytes()))
            }),
            Test::HasParent(kinds) => nodes.any(|n| {
                n.parent()
                    .is_some_and(|p| kinds.iter().any(|k| k == p.kind()))
            }),
            Test::HasAncestor(kinds) => nodes.any(|n| {
                std::iter::successors(n.parent(), |p| p.parent())
                    .any(|p| kinds.iter().any(|k| k == p.kind()))
            }),
        };
        hit == self.positive
    }
}

struct Patterns {
    query: Query,
    checks: Vec<Vec<Check>>,
    offsets: Vec<Vec<Offset>>,
    rewrites: Vec<Vec<Rewrite>>,
    settings: Vec<Vec<(String, Option<String>)>>,
}

impl Patterns {
    fn setting(&self, pattern: usize, key: &str) -> Option<Option<&str>> {
        self.settings[pattern]
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_deref())
    }

    fn matches(&self, m: &tree_sitter::QueryMatch, bytes: &[u8]) -> bool {
        self.checks[m.pattern_index]
            .iter()
            .all(|c| c.passes(m, bytes))
    }

    fn offset(&self, pattern: usize, capture: u32) -> Option<[i64; 4]> {
        self.offsets[pattern]
            .iter()
            .find(|(c, _)| *c == capture)
            .map(|(_, o)| *o)
    }

    fn text<'a>(&self, pattern: usize, capture: u32, text: &'a [u8]) -> std::borrow::Cow<'a, [u8]> {
        match self.rewrites[pattern]
            .iter()
            .find(|(c, _, _)| *c == capture)
        {
            Some((_, re, with)) => re.replace_all(text, with.as_slice()),
            None => text.into(),
        }
    }
}

pub struct Lang {
    pub name: &'static str,
    language: Language,
    highlights: Patterns,
    priority: Vec<u32>,
    injections: Option<Patterns>,
}

pub fn for_syntax(
    syntect_name: &str,
    path: Option<&Path>,
    lines: &[String],
) -> Option<&'static Lang> {
    let astro = path.is_some_and(|p| p.extension().is_some_and(|e| e == "astro"));
    match syntect_name {
        "Bourne Again Shell (bash)" if !path.is_some_and(zsh) => lang("bash"),
        "C" => lang("c"),
        "C#" => lang("c_sharp"),
        "C++" if cpp(path, lines) => lang("cpp"),
        "CSS" => lang("css"),
        "Diff" if diff(path, lines) => lang("diff"),
        "Lisp" if path.is_some_and(query) => lang("query"),
        "HTML" | "XML" => html_or_xml(path?, lines),
        "Go" => lang("go"),
        "Java" if !path.is_some_and(|p| p.extension().is_some_and(|e| e == "bsh")) => lang("java"),
        "JSON" => lang("json"),
        "Kotlin" => lang("kotlin"),
        "TOML" => lang("toml"),
        "YAML" => lang("yaml"),
        "JavaScript (Babel)" => lang("javascript"),
        "Python" => lang("python"),
        "Rust" => lang("rust"),
        "Swift" => lang("swift"),
        "Markdown" => lang("markdown"),
        "TypeScript" => lang("typescript"),
        "TypeScriptReact" if !astro => lang("tsx"),
        "Lua" => lang("lua"),
        _ => None,
    }
}

fn html_variant(lines: &[String]) -> bool {
    static VARIANT: OnceLock<regex::Regex> = OnceLock::new();
    let re = VARIANT.get_or_init(|| {
        regex::Regex::new(concat!(
            r"@(if|for|defer|switch)|\*(ngIf|ngFor|ngSwitch|ngTemplateOutlet)|\bng-(template|content)",
            r"|\bDTD[ \t]+XHTML[ \t]",
            r"|(?i:\{%[ \t]*(autoescape|block|comment|csrf_token|cycle|debug|extends|filter|firstof|for|if",
            r"|ifchanged|include|load|lorem|now|query_string|regroup|resetcycle|spaceless|templatetag|url",
            r"|verbatim|widthratio|with)\b)|\{#[ \t]+|<extend|<super>",
        ))
        .expect("a valid regex")
    });
    lines.iter().take(40).any(|l| re.is_match(l))
}

fn cpp(path: Option<&Path>, lines: &[String]) -> bool {
    match path.and_then(|p| p.extension()).and_then(|e| e.to_str()) {
        Some("cp") => false,
        Some("h") => !lines.iter().take(200).any(|l| {
            let lower = l.to_lowercase();
            ["@interface", "@end", "@class"]
                .iter()
                .any(|m| lower.starts_with(m))
        }),
        _ => true,
    }
}

const XML_EXTENSIONS: &[&str] = &[
    "svg", "xsd", "xsl", "xslt", "psc1", "tpm", "xliff", "atom", "xul", "cdxml", "mpd", "rss",
    "fsproj", "ui", "vbproj", "xlf", "wsdl", "csproj", "wpl", "xmi", "xpr", "xpfm", "spfm", "bxml",
    "mmi", "xcu", "xlb", "xlc", "xba", "slnx",
];
const XML_NAMES: &[&str] = &[
    "fglrxrc",
    "fonts.conf",
    "Directory.Packages.props",
    "Directory.Build.props",
    "Directory.Build.targets",
];
const XML_PATH_ENDS: &[&str] = &[
    "/etc/blkid.tab",
    "/etc/blkid.tab.old",
    ".csproj.user",
    ".fsproj.user",
    ".vbproj.user",
];

fn html_or_xml(path: &Path, lines: &[String]) -> Option<&'static Lang> {
    static SMIL: OnceLock<regex::Regex> = OnceLock::new();
    let smil = SMIL.get_or_init(|| regex::Regex::new(r"<\?\s*xml.*\?>").expect("a valid regex"));
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let full = path.to_string_lossy();
    let first = lines.first().map_or("", String::as_str);
    let xml = match ext {
        "cpt" | "dtml" | "htm" | "html" | "pt" | "shtml" | "stm" => {
            return if html_variant(lines) {
                None
            } else {
                lang("html")
            };
        }
        "xml" => !docbook_or_xbl(lines),
        "lpr" | "grc" | "ts" => first.contains("<?xml"),
        "smil" => smil.is_match(first),
        _ => {
            XML_EXTENSIONS.contains(&ext)
                || XML_NAMES.contains(&name)
                || XML_PATH_ENDS.iter().any(|end| full.ends_with(end))
                || (full.contains("/etc/xdg/menus/") && ext == "menu")
        }
    };
    if xml { lang("xml") } else { None }
}

fn docbook_or_xbl(lines: &[String]) -> bool {
    lines.iter().take(100).any(|l| {
        let lower = l.to_lowercase();
        l.find("<!DOCTYPE")
            .is_some_and(|i| l[i..].contains("DocBook"))
            || lower.contains(r#" xmlns="http://docbook.org/ns/docbook""#)
            || lower.contains(r#"xmlns:xbl="http://www.mozilla.org/xbl""#)
    })
}

fn query(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "scm")
        && path
            .parent()
            .is_some_and(|d| d.components().any(|c| c.as_os_str() == "queries"))
}

fn diff(path: Option<&Path>, lines: &[String]) -> bool {
    static SEND_EMAIL: OnceLock<regex::Regex> = OnceLock::new();
    let send_email = SEND_EMAIL.get_or_init(|| {
        regex::Regex::new(r"^From [0-9a-fA-F]{40,} Mon Sep 17 00:00:00 2001$")
            .expect("a valid regex")
    });
    let Some(path) = path else { return true };
    let patch = path.extension().is_some_and(|e| e == "patch");
    if patch && lines.first().is_some_and(|l| send_email.is_match(l)) {
        return false;
    }
    let dep3 = path.to_string_lossy().contains("/debian/patches/")
        && path.file_name().is_some_and(|n| n != "series")
        && lines
            .iter()
            .take(100)
            .take_while(|l| !l.starts_with("---"))
            .any(|l| DEP3_HEADERS.iter().any(|h| l.starts_with(h)));
    !dep3
}

const DEP3_HEADERS: &[&str] = &[
    "Description:",
    "Subject:",
    "Origin:",
    "Bug:",
    "Forwarded:",
    "Author:",
    "From:",
    "Reviewed-by:",
    "Acked-by:",
    "Last-Updated:",
    "Applied-Upstream:",
];

fn zsh(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let bare = name.strip_prefix('.').unwrap_or(name);
    let ext = name.rsplit_once('.').map_or("", |(_, e)| e);
    matches!(ext, "zsh" | "zunit" | "zsh-theme")
        || bare.starts_with("zsh")
        || bare.starts_with("zlog")
        || name.starts_with(".zcompdump")
        || matches!(name, ".zprofile" | ".zfbfmarks")
        || path == Path::new("/etc/zprofile")
}

fn grammar(name: &str) -> Option<Language> {
    Some(match name {
        "bash" => tree_sitter_bash::LANGUAGE.into(),
        "c" => tree_sitter_c::LANGUAGE.into(),
        "cpp" => tree_sitter_cpp::LANGUAGE.into(),
        "c_sharp" => tree_sitter_c_sharp::LANGUAGE.into(),
        "css" => tree_sitter_css::LANGUAGE.into(),
        "diff" => tree_sitter_diff::LANGUAGE.into(),
        "query" => tree_sitter_tsquery::LANGUAGE.into(),
        "html" => tree_sitter_html::LANGUAGE.into(),
        "xml" => tree_sitter_xml::LANGUAGE_XML.into(),
        "json" => tree_sitter_json::LANGUAGE.into(),
        "toml" => tree_sitter_toml_ng::LANGUAGE.into(),
        "yaml" => tree_sitter_yaml::LANGUAGE.into(),
        "go" => tree_sitter_go::LANGUAGE.into(),
        "python" => tree_sitter_python::LANGUAGE.into(),
        "rust" => tree_sitter_rust::LANGUAGE.into(),
        "markdown" => tree_sitter_md::LANGUAGE.into(),
        "markdown_inline" => tree_sitter_md::INLINE_LANGUAGE.into(),
        "regex" => tree_sitter_regex::LANGUAGE.into(),
        "printf" => tree_sitter_printf::LANGUAGE.into(),
        "java" => tree_sitter_java::LANGUAGE.into(),
        "javascript" => tree_sitter_javascript::LANGUAGE.into(),
        "kotlin" => tree_sitter_kotlin::LANGUAGE.into(),
        "swift" => tree_sitter_swift::LANGUAGE.into(),
        "jsdoc" => tree_sitter_jsdoc::LANGUAGE.into(),
        "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "tsx" => tree_sitter_typescript::LANGUAGE_TSX.into(),
        "lua" => tree_sitter_lua::LANGUAGE.into(),
        "luadoc" => tree_sitter_luadoc::LANGUAGE.into(),
        _ => return None,
    })
}

pub fn lang(name: &str) -> Option<&'static Lang> {
    static LANGS: [OnceLock<Option<Lang>>; QUERIES.len()] =
        [const { OnceLock::new() }; QUERIES.len()];
    let i = QUERIES.iter().position(|(n, _, _)| *n == name)?;
    LANGS[i]
        .get_or_init(|| compile(QUERIES[i].0, grammar(QUERIES[i].0)?).ok())
        .as_ref()
}

pub fn resolve(alias: &str) -> Option<&'static Lang> {
    let alias: String = alias
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            '-' => '_',
            c => c.to_ascii_lowercase(),
        })
        .collect();
    if alias.is_empty()
        || !alias
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return None;
    }
    let name = ALIASES
        .iter()
        .find(|(a, _)| *a == alias)
        .map_or(alias.as_str(), |(_, n)| n);
    lang(name).or_else(|| {
        let syntax = crate::buffer::syntax_for_info(&alias)?;
        for_syntax(&syntax.name, Some(Path::new(&format!("a.{alias}"))), &[])
    })
}

fn source(
    name: &str,
    pick: fn(&(&str, &'static str, &'static str)) -> &'static str,
) -> Result<String, String> {
    let row = QUERIES
        .iter()
        .find(|(n, _, _)| *n == name)
        .ok_or_else(|| format!("no queries for {name}"))?;
    let text = pick(row);
    let mut out = String::new();
    for line in text.lines() {
        if let Some(bases) = line.strip_prefix("; inherits:") {
            for base in bases.split(',').map(str::trim).filter(|b| !b.is_empty()) {
                out.push_str(&source(base.trim_matches(|c| c == '(' || c == ')'), pick)?);
                out.push('\n');
            }
        }
    }
    out.push_str(text);
    Ok(out)
}

fn compile(name: &'static str, language: Language) -> Result<Lang, String> {
    let mut lang = compile_text(name, language, &source(name, |r| r.1)?)?;
    let injections = source(name, |r| r.2)?;
    if !injections.trim().is_empty() {
        lang.injections = Some(patterns(name, &lang.language, &injections, INJECTION_KEYS)?);
    }
    Ok(lang)
}

fn compile_text(name: &'static str, language: Language, text: &str) -> Result<Lang, String> {
    let highlights = patterns(name, &language, text, HIGHLIGHT_KEYS)?;
    let priority = highlights
        .settings
        .iter()
        .map(|s| match s.iter().find(|(k, _)| k == "priority") {
            Some((_, Some(v))) => v.parse().map_err(|_| format!("{name}: priority {v}")),
            _ => Ok(NEOVIM_TREESITTER_PRIORITY),
        })
        .collect::<Result<_, _>>()?;
    Ok(Lang {
        name,
        language,
        highlights,
        priority,
        injections: None,
    })
}

fn patterns(
    name: &str,
    language: &Language,
    text: &str,
    keys: &[&str],
) -> Result<Patterns, String> {
    let used = regex::Regex::new(r"\(#([a-z-]+[?!])").expect("a valid regex");
    if let Some(op) = used
        .captures_iter(text)
        .map(|c| c[1].to_string())
        .find(|op| !PREDICATES.contains(&op.as_str()))
    {
        return Err(format!("{name}: #{op} is not implemented"));
    }
    let url = regex::Regex::new(r"\(#set! @[\w.]+ url @[\w.]+\)").expect("a valid regex");
    let text = url.replace_all(text, "");
    let vim = regex::Regex::new(r"\(#(not-)?match\?").expect("a valid regex");
    let text = vim.replace_all(&text, "(#${1}vim-match?");
    let query = Query::new(language, &text).map_err(|e| format!("{name}: {e}"))?;
    let mut checks = Vec::new();
    let mut offsets = Vec::new();
    let mut rewrites = Vec::new();
    let mut settings = Vec::new();
    for p in 0..query.pattern_count() {
        let mut set = Vec::new();
        for prop in query.property_settings(p) {
            match prop.capture_id {
                None if keys.contains(&&*prop.key) => {
                    set.push((
                        prop.key.to_string(),
                        prop.value.as_deref().map(str::to_string),
                    ));
                }
                Some(_) if CAPTURE_KEYS.contains(&&*prop.key) => {}
                _ => return Err(format!("{name}: #set! {} is not implemented", prop.key)),
            }
        }
        settings.push(set);
        let mut preds = Vec::new();
        let mut offs = Vec::new();
        let mut subs = Vec::new();
        for pred in query.general_predicates(p) {
            use tree_sitter::QueryPredicateArg::{Capture, String as Text};
            if &*pred.operator == "gsub!" {
                let [Capture(c), Text(pattern), Text(with)] = &*pred.args else {
                    return Err(format!("{name}: #gsub! takes a capture and two strings"));
                };
                subs.push((*c, lua_pattern(pattern)?, lua_replacement(with)?));
                continue;
            }
            if &*pred.operator == "offset!" {
                let [Capture(c), Text(a), Text(b), Text(d), Text(e)] = &*pred.args else {
                    return Err(format!("{name}: #offset! takes a capture and four numbers"));
                };
                let n = |t: &str| {
                    t.parse::<i64>()
                        .map_err(|_| format!("{name}: #offset! {t}"))
                };
                offs.push((*c, [n(a)?, n(b)?, n(d)?, n(e)?]));
                continue;
            }
            let op = &*pred.operator;
            let (positive, op) = match op.strip_prefix("not-") {
                Some(rest) => (false, rest),
                None => (true, op),
            };
            let (capture, args) = match &*pred.args {
                [Capture(c), rest @ ..] if !rest.is_empty() => (
                    *c,
                    rest.iter()
                        .map(|a| match a {
                            Text(s) => Ok(s.to_string()),
                            _ => Err(format!("{name}: #{} takes strings", pred.operator)),
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                ),
                _ => return Err(format!("{name}: #{} takes a capture first", pred.operator)),
            };
            let test = match op {
                "lua-match?" if args.len() == 1 => Test::Match(lua_pattern(&args[0])?),
                "vim-match?" if args.len() == 1 => Test::Match(vim_pattern(&args[0])?),
                "contains?" => Test::Contains(args),
                "has-parent?" => Test::HasParent(args),
                "has-ancestor?" => Test::HasAncestor(args),
                _ => return Err(format!("{name}: #{} is not implemented", pred.operator)),
            };
            preds.push(Check {
                capture,
                test,
                positive,
            });
        }
        checks.push(preds);
        offsets.push(offs);
        rewrites.push(subs);
    }
    Ok(Patterns {
        query,
        checks,
        offsets,
        rewrites,
        settings,
    })
}

type Found = std::collections::HashSet<(usize, Vec<(u32, usize, usize, usize)>)>;

fn found(
    patterns: &Patterns,
    tree: &Tree,
    range: std::ops::Range<usize>,
    bytes: &[u8],
    clip: usize,
) -> Found {
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(range);
    let mut out = Found::new();
    let mut matches = cursor.matches(&patterns.query, tree.root_node(), bytes);
    while let Some(m) = matches.next() {
        if !patterns.matches(m, bytes) {
            continue;
        }
        let captures: Vec<_> = m
            .captures()
            .iter()
            .filter(|c| c.node.start_byte() < clip)
            .map(|c| {
                let r = c.node.byte_range();
                (
                    c.index,
                    r.start,
                    r.end.min(clip),
                    c.node.start_position().row,
                )
            })
            .collect();
        if !captures.is_empty() {
            out.insert((m.pattern_index, captures));
        }
    }
    out
}

fn first_row(old: Found, new: &Found) -> usize {
    old.symmetric_difference(new)
        .flat_map(|(_, captures)| captures.iter().map(|c| c.3))
        .min()
        .unwrap_or(usize::MAX)
}

fn changed_match_row(
    patterns: &Patterns,
    old: &Tree,
    new: &Tree,
    range: std::ops::Range<usize>,
    bytes: &[u8],
) -> usize {
    let found = |tree| found(patterns, tree, range.clone(), bytes, usize::MAX);
    first_row(found(old), &found(new))
}

fn changed_text_row(patterns: &Patterns, before: &(Tree, String), new: &Tree, text: &str) -> usize {
    let (old, new_bytes) = (before.1.as_bytes(), text.as_bytes());
    let (prefix, suffix) = common_ends(old, new_bytes);
    let around = |len: usize| prefix.saturating_sub(1)..(len - suffix + 1).min(len);
    first_row(
        found(patterns, &before.0, around(old.len()), old, prefix),
        &found(patterns, new, around(new_bytes.len()), new_bytes, prefix),
    )
}

fn recovered_row(old: &Tree, new: &Tree, below: usize) -> usize {
    if !old.root_node().has_error() && !new.root_node().has_error() {
        return usize::MAX;
    }
    let (mut a, mut b) = (old.walk(), new.walk());
    loop {
        let (x, y) = (a.node(), b.node());
        let row = x.start_position().row.min(y.start_position().row);
        if row >= below {
            return usize::MAX;
        }
        if x.kind_id() != y.kind_id()
            || x.byte_range() != y.byte_range()
            || x.is_missing() != y.is_missing()
            || a.field_id() != b.field_id()
        {
            return row;
        }
        if x.has_error() || y.has_error() {
            match (a.goto_first_child(), b.goto_first_child()) {
                (true, true) => continue,
                (false, false) => {}
                _ => return row,
            }
        }
        loop {
            match (a.goto_next_sibling(), b.goto_next_sibling()) {
                (true, true) => break,
                (false, false) => {}
                (true, false) => return a.node().start_position().row,
                (false, true) => return b.node().start_position().row,
            }
            if !a.goto_parent() || !b.goto_parent() {
                return usize::MAX;
            }
        }
    }
}

fn vim_pattern(p: &str) -> Result<regex::bytes::Regex, String> {
    let bad = || Err(format!("vim pattern {p:?} is not supported"));
    let body = match p.get(..2) {
        Some("\\v") => &p[2..],
        Some("\\m" | "\\M" | "\\V") => return bad(),
        _ => p,
    };
    let mut out = String::from("(?s-u)");
    let mut chars = body.chars().peekable();
    let mut class = None;
    while let Some(c) = chars.next() {
        match (c, class) {
            ('<' | '>' | '=' | '@' | '%' | '~' | '{' | '&', _) => return bad(),
            ('\\' | '[', Some(_)) => return bad(),
            ('-', Some(_)) if chars.peek() == Some(&'-') => return bad(),
            (']', Some(false)) => class = None,
            ('^', Some(true)) => {}
            (_, Some(_)) => class = Some(false),
            ('[', None) => class = Some(true),
            ('\\', None) => match chars.next() {
                Some(n @ ('d' | 'w' | 'D' | 'W')) => {
                    out.push('\\');
                    out.push(n);
                    continue;
                }
                Some(n) if !n.is_ascii_alphanumeric() && n != '_' => {
                    out.push_str(&regex::escape(&n.to_string()));
                    continue;
                }
                _ => return bad(),
            },
            _ => {}
        }
        out.push(c);
    }
    regex::bytes::Regex::new(&out).map_err(|e| format!("vim pattern {p:?}: {e}"))
}

fn lua_replacement(with: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut chars = with.chars();
    while let Some(c) = chars.next() {
        match c {
            '%' => match chars.next() {
                Some(d @ '0'..='9') => out.extend(format!("${{{d}}}").bytes()),
                Some('%') => out.push(b'%'),
                _ => return Err(format!("lua replacement {with:?}: invalid use of '%'")),
            },
            '$' => out.extend(b"$$"),
            c => out.extend(c.to_string().bytes()),
        }
    }
    Ok(out)
}

fn lua_pattern(p: &str) -> Result<regex::bytes::Regex, String> {
    let class = |c: char| -> Option<&'static str> {
        Some(match c {
            'a' => "A-Za-z",
            'c' => "\\x00-\\x1f\\x7f",
            'd' => "0-9",
            'g' => "!-~",
            'l' => "a-z",
            'p' => "!-/:-@\\[-`{-~",
            's' => "\\t\\n\\x0b\\x0c\\r ",
            'u' => "A-Z",
            'w' => "A-Za-z0-9",
            'x' => "0-9A-Fa-f",
            _ => return None,
        })
    };
    let bad = || Err(format!("lua pattern {p:?} is not supported"));
    let chars: Vec<char> = p.chars().collect();
    let mut out = String::from("(?s-u)");
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '^' if i == 0 => out.push('^'),
            '$' if i == chars.len() - 1 => out.push('$'),
            '.' => out.push('.'),
            '*' | '+' | '?' => out.push(c),
            '-' => out.push_str("*?"),
            '(' | ')' => out.push(c),
            '%' => {
                i += 1;
                let Some(&n) = chars.get(i) else { return bad() };
                if let Some(set) = class(n.to_ascii_lowercase()) {
                    let neg = if n.is_ascii_uppercase() { "^" } else { "" };
                    out.push_str(&format!("[{neg}{set}]"));
                } else if n.is_ascii_alphanumeric() {
                    return bad();
                } else {
                    out.push_str(&regex::escape(&n.to_string()));
                }
            }
            '[' => {
                out.push('[');
                i += 1;
                if chars.get(i) == Some(&'^') {
                    out.push('^');
                    i += 1;
                }
                let mut first = true;
                loop {
                    let Some(&m) = chars.get(i) else { return bad() };
                    if m == ']' && !first {
                        break;
                    }
                    first = false;
                    if m == '%' {
                        i += 1;
                        let Some(&n) = chars.get(i) else { return bad() };
                        match class(n) {
                            Some(set) => out.push_str(set),
                            None if n.is_ascii_alphanumeric() => return bad(),
                            None => out.push_str(&format!("\\x{:02x}", n as u32)),
                        }
                    } else if m == '-' {
                        out.push('-');
                    } else if m.is_ascii_alphanumeric() {
                        out.push(m);
                    } else {
                        out.push_str(&format!("\\x{:02x}", m as u32));
                    }
                    i += 1;
                }
                out.push(']');
            }
            _ => out.push_str(&regex::escape(&c.to_string())),
        }
        i += 1;
    }
    regex::bytes::Regex::new(&out).map_err(|e| format!("lua pattern {p:?}: {e}"))
}

#[cfg(test)]
pub type Shape = Vec<(u16, std::ops::Range<usize>, bool, bool)>;

pub struct Parsed {
    pub lang: &'static Lang,
    parser: Parser,
    inner: Parser,
    tree: Option<Tree>,
    text: String,
    starts: Vec<usize>,
    repaint_from: Option<usize>,
    halted: bool,
    before: Option<(Tree, String)>,
}

fn joined(lines: &[String]) -> (String, Vec<usize>) {
    let mut text = String::with_capacity(lines.iter().map(|l| l.len() + 1).sum());
    let mut starts = Vec::with_capacity(lines.len() + 1);
    starts.push(0);
    for l in lines {
        text.push_str(l);
        text.push('\n');
        starts.push(text.len());
    }
    (text, starts)
}

fn common_ends(old: &[u8], new: &[u8]) -> (usize, usize) {
    let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let room = old.len().min(new.len()) - prefix;
    let suffix = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(room)
        .take_while(|(a, b)| a == b)
        .count();
    (prefix, suffix)
}

fn point(starts: &[usize], byte: usize) -> Point {
    let row = starts.partition_point(|&s| s <= byte) - 1;
    Point::new(row, byte - starts[row])
}

impl Parsed {
    pub fn new(lang: &'static Lang, lines: &[String]) -> Option<Self> {
        let mut parser = Parser::new();
        parser.set_language(&lang.language).ok()?;
        let (text, starts) = joined(lines);
        Some(Self {
            lang,
            parser,
            inner: Parser::new(),
            tree: None,
            starts,
            text,
            repaint_from: Some(0),
            halted: false,
            before: None,
        })
    }

    pub fn parsing(&self) -> bool {
        self.repaint_from.is_some()
    }

    pub fn has_tree(&self) -> bool {
        self.tree.is_some()
    }

    pub fn edit(&mut self, lines: &[String]) {
        let (text, starts) = joined(lines);
        let (old, new) = (self.text.as_bytes(), text.as_bytes());
        if old == new {
            return;
        }
        let (prefix, suffix) = common_ends(old, new);
        let edit = InputEdit {
            start_byte: prefix,
            old_end_byte: old.len() - suffix,
            new_end_byte: new.len() - suffix,
            start_position: point(&self.starts, prefix),
            old_end_position: point(&self.starts, old.len() - suffix),
            new_end_position: point(&starts, new.len() - suffix),
        };
        if let Some(tree) = &mut self.tree {
            if self.before.is_none() {
                self.before = Some((tree.clone(), std::mem::take(&mut self.text)));
            }
            tree.edit(&edit);
        }
        if std::mem::take(&mut self.halted) {
            self.parser.reset();
        }
        let row = edit.start_position.row;
        self.repaint_from = Some(self.repaint_from.map_or(row, |r| r.min(row)));
        (self.text, self.starts) = (text, starts);
    }

    pub fn parse(&mut self, budget: Option<Duration>) -> Option<usize> {
        let from = self.repaint_from?;
        let deadline = budget.map(|b| Instant::now() + b);
        let mut on_time = |_: &ParseState| match deadline {
            Some(d) if Instant::now() >= d => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        };
        let bytes = self.text.as_bytes();
        let Some(tree) = self.parser.parse_with_options(
            &mut |i, _| &bytes[i.min(bytes.len())..],
            self.tree.as_ref(),
            Some(ParseOptions::new().progress_callback(&mut on_time)),
        ) else {
            self.halted = true;
            return None;
        };
        let from = match &self.tree {
            Some(old) => old
                .changed_ranges(&tree)
                .map(|r| {
                    let range = r.start_byte..r.end_byte;
                    let row = |p: &Patterns| changed_match_row(p, old, &tree, range.clone(), bytes);
                    let injected = self.lang.injections.as_ref().map_or(usize::MAX, row);
                    r.start_point
                        .row
                        .min(row(&self.lang.highlights))
                        .min(injected)
                })
                .fold(from, usize::min),
            None => 0,
        };
        let from = match &self.tree {
            Some(old) => from.min(recovered_row(old, &tree, from)),
            None => from,
        };
        let from = match self.before.take() {
            Some(before) => {
                let row = |p: &Patterns| changed_text_row(p, &before, &tree, &self.text);
                let injected = self.lang.injections.as_ref().map_or(usize::MAX, row);
                from.min(row(&self.lang.highlights)).min(injected)
            }
            None => from,
        };
        let near = self.starts[from]..self.starts.get(from + 1).copied().unwrap_or(bytes.len());
        let doc = layers::Doc::new(bytes, &self.starts, near.start, near.end);
        let from = layers::region_start(&layers::injections(self.lang, &tree, None, &doc), from);
        (self.tree, self.repaint_from, self.halted) = (Some(tree), None, false);
        Some(from)
    }

    pub fn spans(
        &mut self,
        lines: &[String],
        from: usize,
        to: usize,
        styles: &mut Styles,
    ) -> Vec<Spans> {
        let Some(tree) = &self.tree else {
            return Vec::new();
        };
        let start = self.starts[from];
        let end = self.starts.get(to + 1).copied().unwrap_or(self.text.len());
        let doc = layers::Doc::new(self.text.as_bytes(), &self.starts, start, end);
        let mut painted = Vec::new();
        layers::layer(
            self.lang,
            std::slice::from_ref(tree),
            false,
            &mut layers::Ctx {
                parser: &mut self.inner,
                sub: 0,
                depth: 0,
                styles,
                out: &mut painted,
            },
            &doc,
        );
        painted.sort_by_key(|p| (p.prio, p.sub));
        let mut cells: Vec<Vec<Style>> = (from..=to)
            .map(|l| vec![styles.normal; lines[l].len()])
            .collect();
        for p in painted {
            let (a, b) = (p.a.max(start), p.b.min(end));
            if a >= b {
                continue;
            }
            let (first, last) = (point(&self.starts, a).row, point(&self.starts, b - 1).row);
            for l in first..=last.min(to) {
                let row = &mut cells[l - from];
                let s = a.saturating_sub(self.starts[l]).min(row.len());
                let e = (b - self.starts[l]).min(row.len());
                row[s..e].iter_mut().for_each(|c| *c = c.patch(p.style));
            }
        }
        cells
            .into_iter()
            .zip(&lines[from..=to])
            .map(|(row, line)| match Buffer::clips(line) {
                true => Vec::new(),
                false => runs(&row),
            })
            .collect()
    }
}

impl Parsed {
    #[cfg(test)]
    pub fn sexp(&self) -> String {
        self.tree
            .as_ref()
            .map(|t| t.root_node().to_sexp())
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub fn shape(&self) -> Shape {
        let mut out = Vec::new();
        let Some(tree) = &self.tree else { return out };
        let mut c = tree.walk();
        loop {
            let n = c.node();
            out.push((
                n.kind_id(),
                n.byte_range(),
                n.is_missing(),
                n.child_count() == 0,
            ));
            if c.goto_first_child() || c.goto_next_sibling() {
                continue;
            }
            loop {
                if !c.goto_parent() {
                    return out;
                }
                if c.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    #[cfg(test)]
    pub fn root_bytes(&self) -> Option<std::ops::Range<usize>> {
        self.tree.as_ref().map(|t| t.root_node().byte_range())
    }
}

fn runs(row: &[Style]) -> Spans {
    let mut out: Spans = Vec::new();
    for (i, s) in row.iter().enumerate() {
        match out.last_mut() {
            Some((last, r)) if last == s => r.end = i + 1,
            _ => out.push((*s, i..i + 1)),
        }
    }
    out
}

pub struct Styles<'a> {
    theme: &'a Theme,
    highlighter: Highlighter<'a>,
    normal: Style,
    by_lang: Vec<(&'static str, Vec<Option<Style>>)>,
}

impl<'a> Styles<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        let normal_fg = theme.group("Normal").and_then(|s| s.fg).unwrap_or(theme.fg);
        Self {
            theme,
            highlighter: Highlighter::new(&theme.syntect),
            normal: Style::new().fg(normal_fg),
            by_lang: Vec::new(),
        }
    }

    fn of(&mut self, lang: &Lang) -> &[Option<Style>] {
        let i = match self.by_lang.iter().position(|(n, _)| *n == lang.name) {
            Some(i) => i,
            None => {
                let styles = lang
                    .highlights
                    .query
                    .capture_names()
                    .iter()
                    .map(|name| self.capture(lang.name, name))
                    .collect();
                self.by_lang.push((lang.name, styles));
                self.by_lang.len() - 1
            }
        };
        &self.by_lang[i].1
    }

    fn capture(&self, lang: &str, name: &str) -> Option<Style> {
        if name.starts_with('_') {
            return None;
        }
        if self.theme.groups.is_empty() {
            return textmate(name, &self.highlighter);
        }
        let s = self.theme.group(&format!("@{name}.{lang}"))?;
        let reversed = s.add_modifier.contains(ratatui::style::Modifier::REVERSED);
        Some(Style {
            bg: s.bg.filter(|_| reversed),
            underline_color: None,
            ..s
        })
    }
}

const TEXTMATE: &[(&str, &str)] = &[
    ("attribute", "entity.name.function.decorator"),
    ("boolean", "constant.language"),
    ("character", "constant.character"),
    ("comment", "comment"),
    ("constant", "constant"),
    ("constant.builtin", "constant.language"),
    ("constructor", "entity.name.class"),
    ("function", "entity.name.function"),
    ("function.builtin", "support.function"),
    ("function.call", "variable.function"),
    ("function.method.call", "variable.function"),
    ("keyword", "keyword"),
    ("keyword.function", "storage.type.function"),
    ("keyword.operator", "keyword.operator"),
    ("keyword.type", "storage.type"),
    ("label", "entity.name.label"),
    ("markup", "markup"),
    ("markup.heading", "markup.heading"),
    ("markup.italic", "markup.italic"),
    ("markup.link", "markup.underline.link"),
    ("markup.link.label", "string.other.link"),
    ("markup.list", "markup.list"),
    ("markup.quote", "markup.quote"),
    ("markup.raw", "markup.inline.raw"),
    ("markup.raw.block", "markup.fenced_code"),
    ("markup.strikethrough", "markup.strikethrough"),
    ("markup.strong", "markup.bold"),
    ("markup.underline", "markup.underline"),
    ("module", "entity.name.namespace"),
    ("number", "constant.numeric"),
    ("operator", "keyword.operator"),
    ("property", "variable.other.property"),
    ("punctuation", "punctuation"),
    ("string", "string"),
    ("string.documentation", "comment.block.documentation"),
    ("string.escape", "constant.character.escape"),
    ("string.regexp", "string.regexp"),
    ("tag", "entity.name.tag"),
    ("type", "entity.name.type"),
    ("type.builtin", "support.type"),
    ("variable", "variable"),
    ("variable.builtin", "variable.language"),
    ("variable.member", "variable.other.member"),
    ("variable.parameter", "variable.parameter"),
];

fn textmate(capture: &str, highlighter: &Highlighter) -> Option<Style> {
    static SCOPES: OnceLock<HashMap<&str, Scope>> = OnceLock::new();
    let scopes = SCOPES.get_or_init(|| {
        TEXTMATE
            .iter()
            .map(|(c, s)| (*c, Scope::new(s).expect("a valid scope")))
            .collect()
    });
    let mut name = capture;
    let scope = loop {
        if let Some(s) = scopes.get(name) {
            break *s;
        }
        name = &name[..name.rfind('.')?];
    };
    Some(crate::theme::style(highlighter.style_for_stack(&[scope])))
}

mod layers;

#[cfg(test)]
mod tests;
