use std::cell::RefCell;

use super::App;
use crate::buffer::shown_str;
use crate::git::TextLine;

type Pos = (usize, usize);
type Span = (Pos, Pos);
type Place = (TextLine, usize);
type Sel = Option<(Place, Place)>;

thread_local! {
    static HIST: RefCell<Vec<(Option<Place>, Place, Sel)>> = const { RefCell::new(Vec::new()) };
}

pub fn variant() -> Option<char> {
    match std::env::var("MERL_P924").ok()?.as_str() {
        "a" => Some('a'),
        "b" => Some('b'),
        _ => None,
    }
}

const PAIRS: &[(&str, &str)] = &[("(", ")"), ("[", "]"), ("{", "}")];
const QUOTES: &[&str] = &["\"\"\"", "'''", "\"", "'", "`"];

impl App {
    pub(super) fn p924_grow(&mut self) {
        let Some(v) = variant().filter(|_| self.deleted.is_none()) else {
            self.grow_selection();
            return;
        };
        let before = (self.anchor, (self.at(), self.col));
        let sel = self.selection();
        HIST.with_borrow_mut(|h| {
            if h.last().is_some_and(|t| t.2 != sel) {
                h.clear();
            }
        });
        self.buf.parse_more(None);
        if self.buf.ts_tree().is_none() {
            self.grow_selection();
        } else {
            self.grow_by(v);
        }
        let after = self.selection();
        if after != sel {
            HIST.with_borrow_mut(|h| h.push((before.0, before.1, after)));
        }
    }

    pub(super) fn p924_back(&mut self) {
        let sel = self.selection();
        let Some((anchor, at, after)) = HIST.with_borrow_mut(Vec::pop) else {
            return;
        };
        if after != sel {
            HIST.with_borrow_mut(Vec::clear);
            return;
        }
        self.anchor = anchor;
        self.set_at(at.0);
        self.col = at.1;
        self.sync_want_x();
    }

    fn grow_by(&mut self, v: char) {
        self.buf.parse_more(None);
        let lines = &self.buf.lines;
        let len = |r: usize| shown_str(&lines[r]).len();
        let blank = |r: usize| lines[r].trim().is_empty();
        let l = self.line;
        let cur: Span = match self.selection() {
            Some(((a, ac), (b, bc))) => ((a.key(), ac), (b.key(), bc)),
            None => ((l, self.col), (l, self.col)),
        };
        let mut starts = Vec::with_capacity(lines.len() + 1);
        let mut acc = 0;
        for s in lines {
            starts.push(acc);
            acc += s.len() + 1;
        }
        let off = |p: Pos| starts[p.0] + p.1;
        let tidy = |(a, mut b): Span| {
            if b.0 >= lines.len() {
                b = (lines.len() - 1, len(lines.len() - 1));
            }
            if b.1 == 0 && b.0 > a.0 {
                b = (b.0 - 1, len(b.0 - 1));
            }
            (a, b)
        };
        let mut cands: Vec<Span> = Vec::new();
        let s = shown_str(&lines[l]);
        let (mut from, mut to) = (self.col, self.col);
        let word = |c: char| c.is_alphanumeric() || c == '_';
        while from > 0 && s[..from].chars().next_back().is_some_and(word) {
            from -= s[..from].chars().next_back().map_or(1, char::len_utf8);
        }
        while to < s.len() && s[to..].chars().next().is_some_and(word) {
            to += s[to..].chars().next().map_or(1, char::len_utf8);
        }
        if from < to {
            cands.push(((l, from), (l, to)));
        }
        let point = |p: Pos| tree_sitter::Point::new(p.0, p.1);
        let lead = |r: usize| lines[r].len() - lines[r].trim_start().len();
        let tail = |r: usize| lines[r].trim_end().len();
        let core: Span = if cur.0 == cur.1 {
            cur
        } else {
            (
                (cur.0.0, cur.0.1.max(lead(cur.0.0).min(tail(cur.0.0)))),
                (cur.1.0, cur.1.1.min(tail(cur.1.0)).max(lead(cur.1.0))),
            )
        };
        let pos = |p: tree_sitter::Point| (p.row, p.column);
        let leaf = |n: &tree_sitter::Node| -> &str {
            let (a, b) = (n.start_position(), n.end_position());
            if a.row != b.row {
                return "";
            }
            lines[a.row].get(a.column..b.column).unwrap_or("")
        };
        if let Some(tree) = self.buf.ts_tree() {
            let root = tree.root_node();
            if v == 'a' {
                let mut node = root.named_descendant_for_point_range(point(cur.0), point(cur.1));
                while let Some(n) = node {
                    cands.push(tidy((pos(n.start_position()), pos(n.end_position()))));
                    node = n.parent();
                }
            } else {
                let mut node = root.descendant_for_point_range(point(core.0), point(core.1));
                while let Some(n) = node {
                    let mut found: Vec<(tree_sitter::Node, tree_sitter::Node)> = Vec::new();
                    let n_kids = n.child_count();
                    if let (Some(f), Some(z)) = (n.child(0), n.child(n_kids.saturating_sub(1)))
                        && n_kids >= 2
                        && f.child_count() == 0
                        && z.child_count() == 0
                        && QUOTES.iter().any(|q| *q == leaf(&z) && leaf(&f).ends_with(q))
                    {
                        found.push((f, z));
                    }
                    let mut open: Vec<tree_sitter::Node> = Vec::new();
                    let mut c = n.walk();
                    for k in n.children(&mut c) {
                        if k.child_count() != 0 {
                            continue;
                        }
                        let t = leaf(&k);
                        let closes = |o: &tree_sitter::Node| {
                            PAIRS.iter().any(|(a, b)| *a == leaf(o) && *b == t)
                        };
                        if let Some(i) = open.iter().rposition(closes) {
                            found.push((open[i], k));
                            open.truncate(i);
                        } else if PAIRS.iter().any(|(a, _)| *a == t) {
                            open.push(k);
                        }
                    }
                    for (o, k) in found {
                        let (os, oe) = (pos(o.start_position()), pos(o.end_position()));
                        let (cs, ce) = (pos(k.start_position()), pos(k.end_position()));
                        let mut inner = (oe, cs);
                        if inner.0.0 < inner.1.0 {
                            if lines[oe.0][oe.1..].trim().is_empty() {
                                inner.0 = (oe.0 + 1, 0);
                            }
                            if lines[cs.0][..cs.1].trim().is_empty() {
                                inner.1 = (cs.0 - 1, len(cs.0 - 1));
                            }
                        }
                        if inner.0 < inner.1 {
                            cands.push(inner);
                        }
                        cands.push((os, ce));
                    }
                    node = n.parent();
                }
            }
        }
        if v == 'b' {
            let (a, b) = (cur.0.0, cur.1.0);
            if !(a == b && blank(a)) {
                cands.push(((a, 0), (b, len(b))));
            }
            if let Some(tree) = self.buf.ts_tree() {
                let indent = |r: usize| lines[r].len() - lines[r].trim_start().len();
                let mut node = tree
                    .root_node()
                    .named_descendant_for_point_range(point(core.0), point(core.1));
                while let Some(n) = node {
                    let (x, y) = tidy((pos(n.start_position()), pos(n.end_position())));
                    if x.1 == indent(x.0) && lines[y.0][y.1..].trim().is_empty() {
                        cands.push(((x.0, 0), (y.0, len(y.0))));
                    }
                    node = n.parent();
                }
            }
            let last = lines.len() - 1;
            cands.push(((0, 0), (last, len(last))));
        }
        let wider = |&(a, b): &Span| a <= cur.0 && cur.1 <= b && (a, b) != cur;
        let best = cands
            .into_iter()
            .filter(wider)
            .min_by_key(|&(a, b)| (off(b) - off(a), std::cmp::Reverse(off(a))));
        if let Some((a, b)) = best {
            self.anchor = Some((TextLine::File(a.0), a.1));
            self.set_at(TextLine::File(b.0));
            self.col = b.1;
            self.sync_want_x();
        }
    }
}
