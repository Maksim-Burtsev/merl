use std::collections::HashMap;
use std::ops::ControlFlow;
use std::path::Path;
#[cfg(not(target_family = "wasm"))]
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub use filetype::for_syntax;
use pattern::{lua_pattern, lua_replacement, vim_pattern};
use ratatui::style::Style;
use syntect::highlighting::Highlighter;
use syntect::parsing::Scope;
use tree_sitter::{InputEdit, Language, ParseOptions, ParseState, Parser, Point, Query, Tree};

use crate::buffer::{Buffer, Spans};
use crate::theme::Theme;
#[cfg(target_family = "wasm")]
use crate::web::WasmOnce as OnceLock;

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
        "cmake",
        include_str!("../queries/cmake/highlights.scm"),
        include_str!("../queries/cmake/injections.scm"),
    ),
    (
        "hcl",
        include_str!("../queries/hcl/highlights.scm"),
        include_str!("../queries/hcl/injections.scm"),
    ),
    (
        "terraform",
        include_str!("../queries/terraform/highlights.scm"),
        include_str!("../queries/terraform/injections.scm"),
    ),
    (
        "ini",
        include_str!("../queries/ini/highlights.scm"),
        include_str!("../queries/ini/injections.scm"),
    ),
    (
        "graphql",
        include_str!("../queries/graphql/highlights.scm"),
        include_str!("../queries/graphql/injections.scm"),
    ),
    (
        "nginx",
        include_str!("../queries/nginx/highlights.scm"),
        include_str!("../queries/nginx/injections.scm"),
    ),
    (
        "gdscript",
        include_str!("../queries/gdscript/highlights.scm"),
        include_str!("../queries/gdscript/injections.scm"),
    ),
    (
        "starlark",
        include_str!("../queries/starlark/highlights.scm"),
        include_str!("../queries/starlark/injections.scm"),
    ),
    (
        "solidity",
        include_str!("../queries/solidity/highlights.scm"),
        include_str!("../queries/solidity/injections.scm"),
    ),
    (
        "proto",
        include_str!("../queries/proto/highlights.scm"),
        include_str!("../queries/proto/injections.scm"),
    ),
    (
        "sql",
        include_str!("../queries/sql/highlights.scm"),
        include_str!("../queries/sql/injections.scm"),
    ),
    (
        "dockerfile",
        include_str!("../queries/dockerfile/highlights.scm"),
        include_str!("../queries/dockerfile/injections.scm"),
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
        "elixir",
        include_str!("../queries/elixir/highlights.scm"),
        include_str!("../queries/elixir/injections.scm"),
    ),
    (
        "php",
        include_str!("../queries/php/highlights.scm"),
        include_str!("../queries/php/injections.scm"),
    ),
    (
        "php_only",
        include_str!("../queries/php_only/highlights.scm"),
        include_str!("../queries/php_only/injections.scm"),
    ),
    (
        "ruby",
        include_str!("../queries/ruby/highlights.scm"),
        include_str!("../queries/ruby/injections.scm"),
    ),
    (
        "scala",
        include_str!("../queries/scala/highlights.scm"),
        include_str!("../queries/scala/injections.scm"),
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
    (
        "zig",
        include_str!("../queries/zig/highlights.scm"),
        include_str!("../queries/zig/injections.scm"),
    ),
    (
        "nix",
        include_str!("../queries/nix/highlights.scm"),
        include_str!("../queries/nix/injections.scm"),
    ),
    (
        "scheme",
        include_str!("../queries/scheme/highlights.scm"),
        include_str!("../queries/scheme/injections.scm"),
    ),
    (
        "r",
        include_str!("../queries/r/highlights.scm"),
        include_str!("../queries/r/injections.scm"),
    ),
    (
        "erlang",
        include_str!("../queries/erlang/highlights.scm"),
        include_str!("../queries/erlang/injections.scm"),
    ),
    (
        "racket",
        include_str!("../queries/racket/highlights.scm"),
        include_str!("../queries/racket/injections.scm"),
    ),
    (
        "commonlisp",
        include_str!("../queries/commonlisp/highlights.scm"),
        include_str!("../queries/commonlisp/injections.scm"),
    ),
    (
        "vue",
        include_str!("../queries/vue/highlights.scm"),
        include_str!("../queries/vue/injections.scm"),
    ),
    (
        "svelte",
        include_str!("../queries/svelte/highlights.scm"),
        include_str!("../queries/svelte/injections.scm"),
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
    ("terraform_vars", "terraform"),
    ("confini", "ini"),
    ("dosini", "ini"),
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
    AnyOf(Vec<String>),
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
            Test::AnyOf(words) => nodes.any(|n| {
                let text = &bytes[n.byte_range()];
                words.iter().any(|w| w.as_bytes() == text)
            }),
            Test::HasParent(kinds) => nodes.any(|n| {
                n.parent()
                    .is_some_and(|p| kinds.iter().any(|k| k == p.kind()))
            }),
            Test::HasAncestor(kinds) => nodes.any(|n| {
                std::iter::successors(n.parent(), tree_sitter::Node::parent)
                    .any(|p| kinds.iter().any(|k| k == p.kind()))
            }),
        };
        hit == self.positive
    }
}

struct Patterns {
    query: Query,
    split: Option<heads::Split>,
    checks: Vec<Vec<Check>>,
    offsets: Vec<Vec<Offset>>,
    rewrites: Vec<Vec<Rewrite>>,
    settings: Vec<Vec<(String, Option<String>)>>,
}

impl Patterns {
    #[expect(
        clippy::option_option,
        reason = "a key absent, a key set bare and a key with a value are three answers, as `#set!` has them"
    )]
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
    combined: Option<Patterns>,
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
        "dockerfile" => tree_sitter_dockerfile_updated::language(),
        "cmake" => tree_sitter_cmake::LANGUAGE.into(),
        "proto" => tree_sitter_proto::LANGUAGE.into(),
        "graphql" => arborium_graphql::language().into(),
        "nginx" => tree_sitter_nginx::LANGUAGE.into(),
        "gdscript" => tree_sitter_gdscript::LANGUAGE.into(),
        "starlark" => tree_sitter_starlark::LANGUAGE.into(),
        "solidity" => tree_sitter_solidity::LANGUAGE.into(),
        "hcl" | "terraform" => tree_sitter_hcl::LANGUAGE.into(),
        "ini" => tree_sitter_ini::LANGUAGE.into(),
        "sql" => tree_sitter_sequel::LANGUAGE.into(),
        "scala" => tree_sitter_scala::LANGUAGE.into(),
        "swift" => tree_sitter_swift::LANGUAGE.into(),
        "ruby" => tree_sitter_ruby::LANGUAGE.into(),
        "elixir" => tree_sitter_elixir::LANGUAGE.into(),
        "php" => tree_sitter_php::LANGUAGE_PHP.into(),
        "php_only" => tree_sitter_php::LANGUAGE_PHP_ONLY.into(),
        "jsdoc" => tree_sitter_jsdoc::LANGUAGE.into(),
        "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "tsx" => tree_sitter_typescript::LANGUAGE_TSX.into(),
        "lua" => tree_sitter_lua::LANGUAGE.into(),
        "luadoc" => tree_sitter_luadoc::LANGUAGE.into(),
        "zig" => tree_sitter_zig::LANGUAGE.into(),
        "nix" => tree_sitter_nix::LANGUAGE.into(),
        "scheme" => tree_sitter_scheme::LANGUAGE.into(),
        "r" => tree_sitter_r::LANGUAGE.into(),
        "erlang" => tree_sitter_erlang::LANGUAGE.into(),
        "racket" => tree_sitter_racket::LANGUAGE.into(),
        "commonlisp" => tree_sitter_commonlisp::LANGUAGE_COMMONLISP.into(),
        "vue" => tree_sitter_vue_next::LANGUAGE.into(),
        "svelte" => tree_sitter_svelte_ng::LANGUAGE.into(),
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
        let syntax = crate::buffer::syntax_for_info(&alias, false)?;
        for_syntax(&syntax.name, Some(Path::new(&format!("a.{alias}"))), &[])
    })
}

fn source(
    name: &str,
    pick: fn(&(&str, &'static str, &'static str)) -> &'static str,
    theme: Option<&str>,
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
                out.push_str(&source(
                    base.trim_matches(|c| c == '(' || c == ')'),
                    pick,
                    theme,
                )?);
                out.push('\n');
            }
        }
    }
    out.push_str(text);
    if let Some(extension) = extends::extension(theme, name) {
        out.push('\n');
        out.push_str(extension);
    }
    Ok(out)
}

fn compile(name: &'static str, language: Language) -> Result<Lang, String> {
    let mut lang = compile_text(name, language, &source(name, |r| r.1, None)?)?;
    let injections = source(name, |r| r.2, None)?;
    if !injections.trim().is_empty() {
        let all = patterns(name, &lang.language, &injections, INJECTION_KEYS)?;
        let joins = |p: usize| all.setting(p, "injection.combined").is_some();
        if (0..all.query.pattern_count()).any(joins) {
            let mut only = patterns(name, &lang.language, &injections, INJECTION_KEYS)?;
            for p in (0..all.query.pattern_count()).filter(|&p| !joins(p)) {
                only.query.disable_pattern(p);
                if let Some(split) = &mut only.split {
                    split.disable(p);
                }
            }
            lang.combined = Some(only);
        }
        lang.injections = Some(all);
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
        combined: None,
    })
}

fn as_neovim_reads(text: &str) -> String {
    let dropped = regex::Regex::new(r#"\(#set! @[\w.]+ url @[\w.]+\)|\(#any-of\? "[^)]*\)"#)
        .expect("a valid regex");
    let text = dropped.replace_all(text, "");
    let vim = regex::Regex::new(r"\(#(not-)?match\?").expect("a valid regex");
    let text = vim.replace_all(&text, "(#${1}vim-match?");
    let any_of = regex::Regex::new(r"\(#(not-)?any-of\?").expect("a valid regex");
    any_of.replace_all(&text, "(#${1}nvim-any-of?").into_owned()
}

fn patterns(
    name: &str,
    language: &Language,
    text: &str,
    keys: &[&str],
) -> Result<Patterns, String> {
    let used = regex::Regex::new(r"\(#([a-z-]+[?!])").expect("a valid regex");
    if let Some(op) = text
        .lines()
        .filter(|l| !l.trim_start().starts_with(';'))
        .flat_map(|l| used.captures_iter(l))
        .map(|c| c[1].to_string())
        .find(|op| !PREDICATES.contains(&op.as_str()))
    {
        return Err(format!("{name}: #{op} is not implemented"));
    }
    let text = as_neovim_reads(text);
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
                        prop.value.as_deref().map(str::to_owned),
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
                            Capture(_) => Err(format!("{name}: #{} takes strings", pred.operator)),
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                ),
                _ => return Err(format!("{name}: #{} takes a capture first", pred.operator)),
            };
            let test = match op {
                "lua-match?" if args.len() == 1 => Test::Match(lua_pattern(&args[0])?),
                "vim-match?" if args.len() == 1 => Test::Match(vim_pattern(&args[0])?),
                "contains?" => Test::Contains(args),
                "nvim-any-of?" => Test::AnyOf(args),
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
        split: heads::split(name, language, &query, &text)?,
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
    let mut out = Found::new();
    heads::each_match(patterns, tree.root_node(), &range, bytes, false, |m| {
        if !patterns.matches(m, bytes) {
            return;
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
    });
    out
}

fn first_row(old: &Found, new: &Found) -> usize {
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
    first_row(&found(old), &found(new))
}

fn changed_text_row(patterns: &Patterns, before: &(Tree, String), new: &Tree, text: &str) -> usize {
    let (old, new_bytes) = (before.1.as_bytes(), text.as_bytes());
    let (prefix, suffix) = common_ends(old, new_bytes);
    let around = |len: usize| prefix.saturating_sub(1)..(len - suffix + 1).min(len);
    first_row(
        &found(patterns, &before.0, around(old.len()), old, prefix),
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
        if x.id() != y.id() {
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

#[cfg(test)]
pub type Shape = Vec<(u16, std::ops::Range<usize>, bool, bool)>;

#[cfg(test)]
#[derive(Debug)]
pub struct GroupShape {
    pub key: String,
    pub ranges: Vec<std::ops::Range<usize>>,
    pub shape: Shape,
    pub sexp: String,
    pub stale: bool,
    pub own: Shape,
    pub own_sexp: String,
}

#[cfg(test)]
fn sexp_of(tree: &Tree) -> String {
    tree.root_node().to_sexp()
}

#[cfg(test)]
fn shape_of(tree: &Tree) -> Shape {
    let mut out = Vec::new();
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

pub struct Parsed {
    pub lang: &'static Lang,
    themed: &'static Lang,
    parser: Parser,
    inner: Parser,
    tree: Option<Tree>,
    text: String,
    starts: Vec<usize>,
    repaint_from: Option<usize>,
    halted: bool,
    before: Option<(Tree, String)>,
    grouper: Parser,
    groups: Option<Vec<combined::Group>>,
    regroup: Option<combined::Regroup>,
    pending: Option<std::ops::Range<usize>>,
    #[cfg(test)]
    steps: Option<Vec<combined::Step>>,
}

#[cfg(test)]
thread_local! {
    pub static RECORD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
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
            themed: lang,
            parser,
            inner: Parser::new(),
            tree: None,
            starts,
            text,
            repaint_from: Some(0),
            halted: false,
            before: None,
            grouper: Parser::new(),
            groups: None,
            regroup: None,
            pending: None,
            #[cfg(test)]
            steps: RECORD.get().then(Vec::new),
        })
    }

    pub fn parsing(&self) -> bool {
        self.repaint_from.is_some() || self.regroup.is_some()
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
            #[cfg(test)]
            if let Some(steps) = &mut self.steps {
                steps.push(combined::Step::Text(text.as_bytes().to_vec()));
            }
        }
        let (dropped, row) = self.drop_regroup().unzip();
        for g in self.groups.iter_mut().flatten() {
            g.edit(&edit);
        }
        let shift = |s: std::ops::Range<usize>| combined::shift_span(&s, &edit);
        self.pending = combined::union(self.pending.take().map(shift), dropped.map(shift));
        if std::mem::take(&mut self.halted) {
            self.parser.reset();
        }
        let row = edit.start_position.row.min(row.unwrap_or(usize::MAX));
        self.repaint_from = Some(self.repaint_from.map_or(row, |r| r.min(row)));
        (self.text, self.starts) = (text, starts);
    }

    pub fn parse(&mut self, budget: Option<Duration>) -> Option<usize> {
        let deadline = budget.map(|b| Instant::now() + b);
        let mut from = usize::MAX;
        if let Some(row) = self.repaint_from {
            from = self.parse_top(deadline, row)?;
            self.repaint_from = None;
        }
        let late = deadline.is_some_and(|d| Instant::now() >= d);
        if self.regroup.is_some() && (from == usize::MAX || !late) {
            from = from.min(self.step_regroup(deadline).unwrap_or(usize::MAX));
        }
        let tree = self.tree.as_ref().filter(|_| from < usize::MAX)?;
        let bytes = self.text.as_bytes();
        let from = from.min(self.starts.len() - 1);
        let near = self.starts[from]..self.starts.get(from + 1).copied().unwrap_or(bytes.len());
        let doc = layers::Doc::new(bytes, &self.starts, near.start, near.end);
        let grouped = self.lang.combined.is_some() && self.groups.is_some();
        let mut injected = layers::injections(self.lang, tree, None, &doc, grouped, false);
        for g in self
            .groups
            .iter()
            .flatten()
            .filter(|g| g.touches(near.start, near.end))
        {
            if let Some(t) = &g.tree {
                injected.extend(layers::injections(
                    g.lang,
                    t,
                    Some(&g.ranges),
                    &doc,
                    false,
                    false,
                ));
            }
        }
        Some(layers::region_start(&injected, from))
    }

    fn parse_top(&mut self, deadline: Option<Instant>, from: usize) -> Option<usize> {
        let mut on_time = |_: &ParseState| match deadline {
            Some(d) if Instant::now() >= d => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        };
        let bytes = self.text.as_bytes();
        let resumed = self.halted;
        let Some(tree) = self.parser.parse_with_options(
            &mut |i, _| &bytes[i.min(bytes.len())..],
            self.tree.as_ref(),
            Some(ParseOptions::new().progress_callback(&mut on_time)),
        ) else {
            self.halted = true;
            return None;
        };
        let tree = if resumed && tree.root_node().end_byte() < bytes.len() {
            let input = &mut |i: usize, _| &bytes[i.min(bytes.len())..];
            self.parser
                .parse_with_options(input, self.tree.as_ref(), None)
                .unwrap_or(tree)
        } else {
            tree
        };
        let changed: Vec<tree_sitter::Range> = self
            .tree
            .as_ref()
            .map(|old| old.changed_ranges(&tree).collect())
            .unwrap_or_default();
        let from = match &self.tree {
            Some(old) => changed
                .iter()
                .map(|r| {
                    let range = r.start_byte..r.end_byte;
                    let row = |p: &Patterns| changed_match_row(p, old, &tree, range.clone(), bytes);
                    let injected = self.lang.injections.as_ref().map_or(usize::MAX, row);
                    r.start_point
                        .row
                        .min(row(&self.themed.highlights))
                        .min(injected)
                })
                .fold(from, usize::min),
            None => 0,
        };
        let from = match &self.tree {
            Some(old) => from.min(recovered_row(old, &tree, from)),
            None => from,
        };
        let dirty = self.tree.as_ref().map(|_| {
            let text = self.before.as_ref().map(|(_, t)| {
                let (prefix, suffix) = common_ends(t.as_bytes(), bytes);
                prefix..bytes.len() - suffix
            });
            changed
                .iter()
                .map(|r| r.start_byte..r.end_byte)
                .chain(text)
                .reduce(|a, b| a.start.min(b.start)..a.end.max(b.end))
        });
        let from = match self.before.take() {
            Some(before) => {
                let row = |p: &Patterns| changed_text_row(p, &before, &tree, &self.text);
                let injected = self.lang.injections.as_ref().map_or(usize::MAX, row);
                from.min(row(&self.themed.highlights)).min(injected)
            }
            None => from,
        };
        #[cfg(test)]
        if let Some(steps) = &mut self.steps {
            steps.push(combined::Step::Parse(Vec::new(), bytes.to_vec()));
        }
        (self.tree, self.halted) = (Some(tree), false);
        if self.lang.combined.is_some() {
            let span = match dirty {
                Some(d) => combined::union(d, self.pending.take()),
                None => Some(0..self.text.len()),
            };
            if let Some(span) = span {
                self.start_regroup(span);
            }
        }
        Some(from)
    }

    pub fn spans(
        &mut self,
        lines: &[String],
        from: usize,
        to: usize,
        styles: &mut Styles,
    ) -> Vec<Spans> {
        self.themed = styles.themed(self.lang);
        let Some(tree) = &self.tree else {
            return Vec::new();
        };
        let start = self.starts[from];
        let end = self.starts.get(to + 1).copied().unwrap_or(self.text.len());
        let doc = layers::Doc::new(self.text.as_bytes(), &self.starts, start, end);
        let mut painted = Vec::new();
        layers::top(
            self.lang,
            tree,
            self.groups.as_deref(),
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
            .map(|(row, line)| {
                if Buffer::clips(line) {
                    Vec::new()
                } else {
                    runs(&row)
                }
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
        self.tree.as_ref().map(shape_of).unwrap_or_default()
    }

    #[cfg(test)]
    pub fn own(&self) -> (Shape, String) {
        let steps = self.steps.as_deref().unwrap_or_default();
        let current = matches!(steps.last(),
            Some(combined::Step::Parse(_, b)) if *b == self.text.as_bytes());
        let own = combined::as_tree_sitter(self.lang, steps).filter(|_| current);
        (
            own.as_ref().map(shape_of).unwrap_or_default(),
            own.as_ref().map(sexp_of).unwrap_or_default(),
        )
    }

    #[cfg(test)]
    pub fn groups(&self) -> Vec<GroupShape> {
        let bytes = self.text.as_bytes();
        self.groups
            .iter()
            .flatten()
            .map(|g| {
                let own = g.as_tree_sitter();
                GroupShape {
                    key: format!("{} pattern {}", g.lang.name, g.pattern),
                    ranges: g.ranges.iter().map(|r| r.start_byte..r.end_byte).collect(),
                    shape: g.tree.as_ref().map(shape_of).unwrap_or_default(),
                    sexp: g.tree.as_ref().map(sexp_of).unwrap_or_default(),
                    stale: g.tree.is_some()
                        && g.parsed() != Some(combined::text_of(&g.ranges, bytes)),
                    own: own.as_ref().map(shape_of).unwrap_or_default(),
                    own_sexp: own.as_ref().map(sexp_of).unwrap_or_default(),
                }
            })
            .collect()
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

    fn themed(&self, lang: &'static Lang) -> &'static Lang {
        extends::themed(lang, self.theme.builtin)
    }

    fn of(&mut self, lang: &Lang) -> &[Option<Style>] {
        let i = if let Some(i) = self.by_lang.iter().position(|(n, _)| *n == lang.name) {
            i
        } else {
            let styles = lang
                .highlights
                .query
                .capture_names()
                .iter()
                .map(|name| self.capture(lang.name, name))
                .collect();
            self.by_lang.push((lang.name, styles));
            self.by_lang.len() - 1
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
        Some(Style {
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
        name = name.rsplit_once('.')?.0;
    };
    Some(crate::theme::style(highlighter.style_for_stack(&[scope])))
}

mod combined;
mod extends;
mod filetype;
mod heads;
mod layers;
mod pattern;

#[cfg(test)]
mod tests;
