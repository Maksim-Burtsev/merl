use std::collections::HashMap;
use std::sync::OnceLock;

use ratatui::style::Style;
use syntect::highlighting::Highlighter;
use syntect::parsing::Scope;
use tree_sitter::{
    InputEdit, Language, Parser, Point, Query, QueryCursor, StreamingIterator, Tree,
};

use crate::buffer::{Buffer, Spans};
use crate::theme::Theme;

const NEOVIM_TREESITTER_PRIORITY: u32 = 100;

const QUERIES: &[(&str, &str)] = &[("python", include_str!("../queries/python/highlights.scm"))];

const PREDICATES: &[&str] = &[
    "eq?",
    "not-eq?",
    "any-of?",
    "not-any-of?",
    "lua-match?",
    "not-lua-match?",
    "set!",
];

pub struct Lang {
    pub name: &'static str,
    language: Language,
    query: Query,
    priority: Vec<u32>,
    lua_match: Vec<Vec<(u32, regex::bytes::Regex, bool)>>,
}

pub fn for_syntax(syntect_name: &str) -> Option<&'static Lang> {
    match syntect_name {
        "Python" => lang("python"),
        _ => None,
    }
}

pub fn lang(name: &str) -> Option<&'static Lang> {
    static PYTHON: OnceLock<Option<Lang>> = OnceLock::new();
    match name {
        "python" => PYTHON
            .get_or_init(|| compile("python", tree_sitter_python::LANGUAGE.into()).ok())
            .as_ref(),
        _ => None,
    }
}

fn source(name: &str) -> Result<String, String> {
    let (_, text) = QUERIES
        .iter()
        .find(|(n, _)| *n == name)
        .ok_or_else(|| format!("no queries for {name}"))?;
    let mut out = String::new();
    for line in text.lines() {
        if let Some(bases) = line.strip_prefix("; inherits:") {
            for base in bases.split(',').map(str::trim).filter(|b| !b.is_empty()) {
                out.push_str(&source(base.trim_matches(|c| c == '(' || c == ')'))?);
                out.push('\n');
            }
        }
    }
    out.push_str(text);
    Ok(out)
}

fn compile(name: &'static str, language: Language) -> Result<Lang, String> {
    compile_text(name, language, &source(name)?)
}

fn compile_text(name: &'static str, language: Language, text: &str) -> Result<Lang, String> {
    let used = regex::Regex::new(r"\(#([a-z-]+[?!])").expect("a valid regex");
    if let Some(op) = used
        .captures_iter(text)
        .map(|c| c[1].to_string())
        .find(|op| !PREDICATES.contains(&op.as_str()))
    {
        return Err(format!("{name}: #{op} is not implemented"));
    }
    let query = Query::new(&language, text).map_err(|e| format!("{name}: {e}"))?;
    let mut priority = Vec::new();
    let mut lua_match = Vec::new();
    for p in 0..query.pattern_count() {
        let mut prio = NEOVIM_TREESITTER_PRIORITY;
        for prop in query.property_settings(p) {
            match (&*prop.key, prop.value.as_deref(), prop.capture_id) {
                ("priority", Some(v), None) => {
                    prio = v.parse().map_err(|_| format!("{name}: priority {v}"))?;
                }
                (key, _, _) => return Err(format!("{name}: #set! {key} is not implemented")),
            }
        }
        priority.push(prio);
        let mut preds = Vec::new();
        for pred in query.general_predicates(p) {
            let positive = match &*pred.operator {
                "lua-match?" => true,
                "not-lua-match?" => false,
                op => return Err(format!("{name}: #{op} is not implemented")),
            };
            match &*pred.args {
                [
                    tree_sitter::QueryPredicateArg::Capture(c),
                    tree_sitter::QueryPredicateArg::String(pattern),
                ] => preds.push((*c, lua_pattern(pattern)?, positive)),
                _ => {
                    return Err(format!(
                        "{name}: #{} takes a capture and a pattern",
                        pred.operator
                    ));
                }
            }
        }
        lua_match.push(preds);
    }
    Ok(Lang {
        name,
        language,
        query,
        priority,
        lua_match,
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
    tree: Tree,
    text: String,
    starts: Vec<usize>,
}

fn joined(lines: &[String]) -> String {
    let mut text = String::with_capacity(lines.iter().map(|l| l.len() + 1).sum());
    for l in lines {
        text.push_str(l);
        text.push('\n');
    }
    text
}

fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect()
}

fn point(starts: &[usize], byte: usize) -> Point {
    let row = starts.partition_point(|&s| s <= byte) - 1;
    Point::new(row, byte - starts[row])
}

impl Parsed {
    pub fn new(lang: &'static Lang, lines: &[String]) -> Option<Self> {
        let mut parser = Parser::new();
        parser.set_language(&lang.language).ok()?;
        let text = joined(lines);
        let tree = parser.parse(&text, None)?;
        Some(Self {
            lang,
            parser,
            tree,
            starts: line_starts(&text),
            text,
        })
    }

    pub fn update(&mut self, lines: &[String]) -> usize {
        let text = joined(lines);
        let (old, new) = (self.text.as_bytes(), text.as_bytes());
        let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
        let room = old.len().min(new.len()) - prefix;
        let suffix = old
            .iter()
            .rev()
            .zip(new.iter().rev())
            .take(room)
            .take_while(|(a, b)| a == b)
            .count();
        if prefix == old.len() && old.len() == new.len() {
            return usize::MAX;
        }
        let starts = line_starts(&text);
        let edit = InputEdit {
            start_byte: prefix,
            old_end_byte: old.len() - suffix,
            new_end_byte: new.len() - suffix,
            start_position: point(&self.starts, prefix),
            old_end_position: point(&self.starts, old.len() - suffix),
            new_end_position: point(&starts, new.len() - suffix),
        };
        self.tree.edit(&edit);
        let first = edit.start_position.row;
        let Some(tree) = self.parser.parse(&text, Some(&self.tree)) else {
            return first;
        };
        let first = self
            .tree
            .changed_ranges(&tree)
            .map(|r| r.start_point.row)
            .fold(first, usize::min);
        (self.tree, self.text, self.starts) = (tree, text, starts);
        first
    }

    pub fn spans(&self, lines: &[String], from: usize, to: usize, styles: &Styles) -> Vec<Spans> {
        let lang = self.lang;
        let start = self.starts[from];
        let end = self.starts.get(to + 1).copied().unwrap_or(self.text.len());
        let mut cursor = QueryCursor::new();
        cursor.set_byte_range(start..self.text.len());
        let bytes = self.text.as_bytes();
        let mut painted: Vec<(u32, usize, usize, Style)> = Vec::new();
        let mut checked: Option<(u32, bool)> = None;
        let mut captures = cursor.captures(&lang.query, self.tree.root_node(), bytes);
        while let Some((m, i)) = captures.next() {
            let node = m.captures()[*i].node;
            if node.start_byte() >= end {
                break;
            }
            let ok = match checked {
                Some((id, ok)) if id == m.id() => ok,
                _ => {
                    let ok = lang.lua_match[m.pattern_index]
                        .iter()
                        .all(|(c, re, positive)| {
                            m.nodes_for_capture_index(*c)
                                .all(|n| re.is_match(&bytes[n.byte_range()]) == *positive)
                        });
                    checked = Some((m.id(), ok));
                    ok
                }
            };
            if !ok {
                continue;
            }
            let c = m.captures()[*i];
            if let Some(style) = styles.by_capture[c.index as usize] {
                let prio = lang.priority[m.pattern_index];
                painted.push((prio, node.start_byte(), node.end_byte(), style));
            }
        }
        painted.sort_by_key(|p| p.0);
        let mut cells: Vec<Vec<Style>> = (from..=to)
            .map(|l| vec![styles.normal; lines[l].len()])
            .collect();
        for (_, a, b, style) in painted {
            let (a, b) = (a.max(start), b.min(end));
            if a >= b {
                continue;
            }
            let (first, last) = (point(&self.starts, a).row, point(&self.starts, b - 1).row);
            for l in first..=last.min(to) {
                let row = &mut cells[l - from];
                let s = a.saturating_sub(self.starts[l]).min(row.len());
                let e = (b - self.starts[l]).min(row.len());
                row[s..e].iter_mut().for_each(|c| *c = c.patch(style));
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
        self.tree.root_node().to_sexp()
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

pub struct Styles {
    normal: Style,
    by_capture: Vec<Option<Style>>,
}

impl Styles {
    pub fn new(lang: &Lang, theme: &Theme) -> Self {
        let highlighter = Highlighter::new(&theme.syntect);
        let normal_fg = theme.group("Normal").and_then(|s| s.fg).unwrap_or(theme.fg);
        let by_capture = lang
            .query
            .capture_names()
            .iter()
            .map(|name| {
                if name.starts_with('_') {
                    return None;
                }
                if theme.groups.is_empty() {
                    return textmate(name, &highlighter);
                }
                let s = theme.group(&format!("@{name}.{}", lang.name))?;
                Some(Style {
                    bg: None,
                    underline_color: None,
                    ..s
                })
            })
            .collect();
        Self {
            normal: Style::new().fg(normal_fg),
            by_capture,
        }
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

#[cfg(test)]
mod tests;
