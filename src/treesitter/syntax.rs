use std::cell::RefCell;
use std::collections::HashSet;

use tree_sitter::{Language, Node, Parser, Point, Query, QueryCursor, StreamingIterator, Tree};

use super::{OnceLock, grammar};

const FOLDS_TS: &str = concat!(
    include_str!("../../queries/ecma/folds.scm"),
    include_str!("../../queries/typescript/folds.scm"),
);
const FOLDS_TSX: &str = concat!(
    include_str!("../../queries/ecma/folds.scm"),
    include_str!("../../queries/typescript/folds.scm"),
    include_str!("../../queries/jsx/folds.scm"),
);
const LOCALS: &str = concat!(
    include_str!("../../queries/ecma/locals.scm"),
    include_str!("../../queries/typescript/locals.scm"),
);

pub struct Rules {
    name: &'static str,
    language: Language,
    folds: Query,
    locals: Query,
}

thread_local! {
    pub static FORCE_OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn off() -> bool {
    FORCE_OFF.get() || std::env::var_os("MERL_NO_TREE").is_some()
}

pub fn rules(ext: &str) -> Option<&'static Rules> {
    static TS: OnceLock<Option<Rules>> = OnceLock::new();
    static TSX: OnceLock<Option<Rules>> = OnceLock::new();
    let (cell, name, folds) = match ext {
        "ts" | "mts" | "cts" => (&TS, "typescript", FOLDS_TS),
        "tsx" => (&TSX, "tsx", FOLDS_TSX),
        _ => return None,
    };
    cell.get_or_init(|| {
        let language = grammar(name)?;
        Some(Rules {
            name,
            folds: Query::new(&language, folds).ok()?,
            locals: Query::new(&language, &locals()).ok()?,
            language,
        })
    })
    .as_ref()
}

thread_local! {
    static LAST: RefCell<Option<(&'static str, String, Tree)>> = const { RefCell::new(None) };
}

impl Rules {
    pub fn parser(&self) -> Option<Parser> {
        let mut parser = Parser::new();
        parser.set_language(&self.language).ok()?;
        Some(parser)
    }

    pub fn parse(&self, text: &str) -> Option<Tree> {
        let hit = LAST.with_borrow(|last| {
            last.as_ref()
                .filter(|(n, t, _)| *n == self.name && t == text)
                .map(|(_, _, tree)| tree.clone())
        });
        if hit.is_some() {
            return hit;
        }
        let tree = self.parser()?.parse(text, None)?;
        LAST.set(Some((self.name, text.to_owned(), tree.clone())));
        Some(tree)
    }

    pub fn folds(&self, tree: &Tree, text: &str) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&self.folds, tree.root_node(), text.as_bytes());
        let mut prev = None;
        while let Some(m) = matches.next() {
            let (Some(first), Some(last)) = (m.captures().first(), m.captures().last()) else {
                continue;
            };
            let start = first.node.start_position().row;
            let end = last.node.end_position();
            let stop = if end.column == 0 {
                end.row.saturating_sub(1)
            } else {
                end.row
            };
            if stop > start && prev != Some((start, stop)) {
                out.push((start, stop));
                prev = Some((start, stop));
            }
        }
        out
    }

    pub fn bound_rows(&self, tree: &Tree, text: &str, at: Point, name: &str) -> Vec<(usize, usize)> {
        let bytes = text.as_bytes();
        let mut scopes = HashSet::new();
        let mut defs: Vec<(Node, bool)> = Vec::new();
        let names = self.locals.capture_names();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&self.locals, tree.root_node(), bytes);
        while let Some(m) = matches.next() {
            let parent = self
                .locals
                .property_settings(m.pattern_index)
                .iter()
                .any(|p| &*p.key == "definition.var.scope" && p.value.as_deref() == Some("parent"));
            for c in m.captures() {
                let kind = names[c.index as usize];
                if kind == "local.scope" {
                    scopes.insert(c.node.id());
                } else if kind.starts_with("local.definition")
                    && c.node.utf8_text(bytes).is_ok_and(|t| t == name)
                {
                    defs.push((c.node, parent));
                }
            }
        }
        let root = tree.root_node();
        let mut owned: Vec<(usize, usize, usize)> = Vec::new();
        for (d, parent) in defs {
            let own = scope_or_root(&scopes, root, Some(d));
            owned.push((own.id(), d.start_position().row + 1, own.start_position().row + 1));
            if parent && own.id() != root.id() {
                let up = scope_or_root(&scopes, root, own.parent());
                owned.push((up.id(), d.start_position().row + 1, up.start_position().row + 1));
            }
        }
        let Some(mut node) = root.descendant_for_point_range(at, at) else {
            return Vec::new();
        };
        loop {
            let scope = scope_or_root(&scopes, root, Some(node));
            let mut rows: Vec<(usize, usize)> = (owned.iter())
                .filter(|(s, _, _)| *s == scope.id())
                .map(|&(_, r, from)| (r, from))
                .collect();
            if !rows.is_empty() {
                rows.sort_unstable();
                rows.dedup();
                return rows;
            }
            match scope.parent() {
                Some(p) if scope.id() != root.id() => node = p,
                _ => return Vec::new(),
            }
        }
    }
}

fn scope_or_root<'t>(scopes: &HashSet<usize>, root: Node<'t>, mut n: Option<Node<'t>>) -> Node<'t> {
    while let Some(x) = n {
        if scopes.contains(&x.id()) {
            return x;
        }
        n = x.parent();
    }
    root
}

fn locals() -> String {
    let mut q = LOCALS.to_owned();
    if std::env::var_os("MERL_TREE_PLAIN_LOCALS").is_none() {
        q.push_str(include_str!("../../queries/typescript/locals_merl.scm"));
    }
    q
}
