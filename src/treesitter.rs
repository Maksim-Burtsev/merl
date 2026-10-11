use std::collections::HashMap;
use std::ops::ControlFlow;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use ratatui::style::Style;
use syntect::highlighting::Highlighter;
use syntect::parsing::Scope;
use tree_sitter::{
    InputEdit, Language, ParseOptions, ParseState, Parser, Point, Query,
    Tree,
};

use crate::buffer::{Buffer, Spans};
use crate::theme::Theme;

const NEOVIM_TREESITTER_PRIORITY: u32 = 100;

const QUERIES: &[(&str, &str, &str)] = &[
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
    ("printf", include_str!("../queries/printf/highlights.scm"), ""),
];

const ALIASES: &[(&str, &str)] = &[("py", "python"), ("gyp", "python"), ("pandoc", "markdown")];

const PREDICATES: &[&str] = &[
    "eq?",
    "not-eq?",
    "any-of?",
    "not-any-of?",
    "lua-match?",
    "not-lua-match?",
    "set!",
    "offset!",
];

const HIGHLIGHT_KEYS: &[&str] = &["priority", "conceal", "conceal_lines"];
const INJECTION_KEYS: &[&str] = &[
    "injection.language",
    "injection.combined",
    "injection.include-children",
];
const CAPTURE_KEYS: &[&str] = &["url", "conceal"];

type LuaMatch = Vec<(u32, regex::bytes::Regex, bool)>;
type Offset = (u32, [i64; 4]);

struct Patterns {
    query: Query,
    lua_match: Vec<LuaMatch>,
    offsets: Vec<Vec<Offset>>,
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
        self.lua_match[m.pattern_index]
            .iter()
            .all(|(c, re, positive)| {
                m.nodes_for_capture_index(*c)
                    .all(|n| re.is_match(&bytes[n.byte_range()]) == *positive)
            })
    }

    fn offset(&self, pattern: usize, capture: u32) -> Option<[i64; 4]> {
        self.offsets[pattern]
            .iter()
            .find(|(c, _)| *c == capture)
            .map(|(_, o)| *o)
    }
}

pub struct Lang {
    pub name: &'static str,
    language: Language,
    highlights: Patterns,
    priority: Vec<u32>,
    injections: Option<Patterns>,
}

pub fn for_syntax(syntect_name: &str) -> Option<&'static Lang> {
    match syntect_name {
        "Python" => lang("python"),
        "Markdown" => lang("markdown"),
        _ => None,
    }
}

fn grammar(name: &str) -> Option<Language> {
    Some(match name {
        "python" => tree_sitter_python::LANGUAGE.into(),
        "markdown" => tree_sitter_md::LANGUAGE.into(),
        "markdown_inline" => tree_sitter_md::INLINE_LANGUAGE.into(),
        "regex" => tree_sitter_regex::LANGUAGE.into(),
        "printf" => tree_sitter_printf::LANGUAGE.into(),
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
    if alias.is_empty() || !alias.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return None;
    }
    let name = ALIASES
        .iter()
        .find(|(a, _)| *a == alias)
        .map_or(alias.as_str(), |(_, n)| n);
    lang(name)
}

fn source(name: &str, pick: fn(&(&str, &'static str, &'static str)) -> &'static str) -> Result<String, String> {
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

fn patterns(name: &str, language: &Language, text: &str, keys: &[&str]) -> Result<Patterns, String> {
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
    let query = Query::new(language, &text).map_err(|e| format!("{name}: {e}"))?;
    let mut lua_match = Vec::new();
    let mut offsets = Vec::new();
    let mut settings = Vec::new();
    for p in 0..query.pattern_count() {
        let mut set = Vec::new();
        for prop in query.property_settings(p) {
            match prop.capture_id {
                None if keys.contains(&&*prop.key) => {
                    set.push((prop.key.to_string(), prop.value.as_deref().map(str::to_string)));
                }
                Some(_) if CAPTURE_KEYS.contains(&&*prop.key) => {}
                _ => return Err(format!("{name}: #set! {} is not implemented", prop.key)),
            }
        }
        settings.push(set);
        let mut preds = Vec::new();
        let mut offs = Vec::new();
        for pred in query.general_predicates(p) {
            use tree_sitter::QueryPredicateArg::{Capture, String as Text};
            match (&*pred.operator, &*pred.args) {
                ("lua-match?" | "not-lua-match?", [Capture(c), Text(pattern)]) => {
                    preds.push((*c, lua_pattern(pattern)?, &*pred.operator == "lua-match?"));
                }
                ("offset!", [Capture(c), Text(a), Text(b), Text(d), Text(e)]) => {
                    let n = |t: &str| t.parse::<i64>().map_err(|_| format!("{name}: #offset! {t}"));
                    offs.push((*c, [n(a)?, n(b)?, n(d)?, n(e)?]));
                }
                ("lua-match?" | "not-lua-match?" | "offset!", _) => {
                    return Err(format!("{name}: #{} takes other arguments", pred.operator));
                }
                (op, _) => return Err(format!("{name}: #{op} is not implemented")),
            }
        }
        lua_match.push(preds);
        offsets.push(offs);
    }
    Ok(Patterns {
        query,
        lua_match,
        offsets,
        settings,
    })
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

pub struct Parsed {
    pub lang: &'static Lang,
    parser: Parser,
    inner: Parser,
    tree: Option<Tree>,
    injected: Vec<layers::Injected>,
    text: String,
    starts: Vec<usize>,
    repaint_from: Option<usize>,
    halted: bool,
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
            injected: Vec::new(),
            starts,
            text,
            repaint_from: Some(0),
            halted: false,
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
        let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
        if prefix == old.len() && old.len() == new.len() {
            return;
        }
        let room = old.len().min(new.len()) - prefix;
        let suffix = old
            .iter()
            .rev()
            .zip(new.iter().rev())
            .take(room)
            .take_while(|(a, b)| a == b)
            .count();
        let edit = InputEdit {
            start_byte: prefix,
            old_end_byte: old.len() - suffix,
            new_end_byte: new.len() - suffix,
            start_position: point(&self.starts, prefix),
            old_end_position: point(&self.starts, old.len() - suffix),
            new_end_position: point(&starts, new.len() - suffix),
        };
        if let Some(tree) = &mut self.tree {
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
                .map(|r| r.start_point.row)
                .fold(from, usize::min),
            None => 0,
        };
        let doc = layers::Doc::new(bytes, &self.starts, 0, bytes.len());
        self.injected = layers::injections(self.lang, &tree, None, &doc, true);
        let from = layers::region_start(&self.injected, from);
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
            Some(&self.injected),
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
        Some(Style {
            bg: None,
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
