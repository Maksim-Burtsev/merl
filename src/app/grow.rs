use tree_sitter::{Node, Point};

use super::{App, TextLine, is_word, next_char, prev_char, shown_str};

type Pos = (usize, usize);
type Span = (Pos, Pos);
pub(super) type Place = (TextLine, usize);
pub(super) type Grown = (Option<Place>, Place, Option<(Place, Place)>);

const PAIRS: [(&str, &str); 3] = [("(", ")"), ("[", "]"), ("{", "}")];
const QUOTES: [&str; 5] = ["\"\"\"", "'''", "\"", "'", "`"];

impl App {
    pub(super) fn grow(&mut self) {
        let before = (self.anchor, (self.at(), self.col));
        let sel = self.selection();
        if self.grown.last().is_some_and(|g| g.2 != sel) {
            self.grown.clear();
        }
        self.buf.parse_more(None);
        let cur = match self.file_selection() {
            Some(s) => Some(s),
            None if self.on_deleted() => None,
            None => match self.at() {
                TextLine::File(l) => Some(((l, self.col), (l, self.col))),
                TextLine::Deleted(..) => None,
            },
        };
        let step = cur
            .zip(self.buf.ts_tree())
            .map(|(cur, tree)| next_step(&self.buf.lines, tree.root_node(), cur));
        match step {
            Some(Some((a, b))) => {
                self.anchor = Some((TextLine::File(a.0), a.1));
                self.set_at(TextLine::File(b.0));
                self.col = b.1;
                self.sync_want_x();
            }
            Some(None) => {}
            None => self.grow_selection(),
        }
        let after = self.selection();
        if after != sel {
            self.grown.push((before.0, before.1, after));
        }
    }

    pub(super) fn shrink(&mut self) {
        let Some((anchor, at, after)) = self.grown.pop() else {
            return;
        };
        if after != self.selection() {
            self.grown.clear();
            return;
        }
        self.anchor = anchor;
        self.set_at(at.0);
        self.col = at.1;
        self.sync_want_x();
    }
}

fn next_step(lines: &[String], root: Node, cur: Span) -> Option<Span> {
    let line = |r: usize| shown_str(&lines[r]);
    let len = |r: usize| line(r).len();
    let lead = |r: usize| line(r).len() - line(r).trim_start().len();
    let tail = |r: usize| line(r).trim_end().len();
    let point = |p: Pos| Point::new(p.0, p.1);
    let last = lines.len() - 1;
    let mut steps = vec![((0, 0), (last, len(last)))];
    let (l, col) = cur.1;
    let s = line(l);
    let (mut from, mut to) = (col.min(s.len()), col.min(s.len()));
    while from > 0
        && s.get(prev_char(s, from)..)
            .and_then(|t| t.chars().next())
            .is_some_and(is_word)
    {
        from = prev_char(s, from);
    }
    while s
        .get(to..)
        .and_then(|t| t.chars().next())
        .is_some_and(is_word)
    {
        to = next_char(s, to);
    }
    if from < to {
        steps.push(((l, from), (l, to)));
    }
    let (a, b) = (cur.0.0, cur.1.0);
    if a != b || !line(a).trim().is_empty() {
        steps.push(((a, 0), (b, len(b))));
    }
    let core = if cur.0 == cur.1 {
        cur
    } else {
        (
            (a, cur.0.1.max(lead(a).min(tail(a)))),
            (b, cur.1.1.min(tail(b)).max(lead(b))),
        )
    };
    let end_of = |p: Point| {
        if p.row > last {
            (last, len(last))
        } else {
            (p.row, p.column)
        }
    };
    let mut node = root.descendant_for_point_range(point(core.0), point(core.1));
    while let Some(n) = node {
        for (open, close) in pairs(lines, n) {
            let (os, oe) = (open.start_position(), open.end_position());
            let (cs, ce) = (close.start_position(), close.end_position());
            let mut inner = ((oe.row, oe.column), (cs.row, cs.column));
            if oe.row < cs.row {
                if line(oe.row)
                    .get(oe.column..)
                    .is_some_and(|t| t.trim().is_empty())
                {
                    inner.0 = (oe.row + 1, 0);
                }
                if line(cs.row)
                    .get(..cs.column)
                    .is_some_and(|t| t.trim().is_empty())
                {
                    inner.1 = (cs.row - 1, len(cs.row - 1));
                }
            }
            if inner.0 < inner.1 {
                steps.push(inner);
            }
            steps.push(((os.row, os.column), end_of(ce)));
        }
        node = n.parent();
    }
    let mut node = root.named_descendant_for_point_range(point(core.0), point(core.1));
    while let Some(n) = node {
        let x = n.start_position();
        let mut y = end_of(n.end_position());
        if y.1 == 0 && y.0 > x.row {
            y = (y.0 - 1, len(y.0 - 1));
        }
        if x.row <= last
            && x.column == lead(x.row)
            && line(y.0).get(y.1..).is_some_and(|t| t.trim().is_empty())
        {
            steps.push(((x.row, 0), (y.0, len(y.0))));
        }
        node = n.parent();
    }
    let mut starts = Vec::with_capacity(lines.len());
    let mut acc = 0;
    for s in lines {
        starts.push(acc);
        acc += s.len() + 1;
    }
    let off = |p: Pos| starts[p.0] + p.1;
    steps
        .into_iter()
        .filter(|&(a, b)| a <= cur.0 && cur.1 <= b && (a, b) != cur)
        .min_by_key(|&(a, b)| (off(b) - off(a), std::cmp::Reverse(off(a))))
}

fn leaf<'a>(lines: &'a [String], n: Node) -> &'a str {
    let (x, y) = (n.start_position(), n.end_position());
    if n.child_count() != 0 || x.row != y.row {
        return "";
    }
    lines
        .get(x.row)
        .and_then(|s| shown_str(s).get(x.column..y.column))
        .unwrap_or("")
}

fn pairs<'t>(lines: &[String], n: Node<'t>) -> Vec<(Node<'t>, Node<'t>)> {
    let leaf = |k: Node| leaf(lines, k);
    let mut found = Vec::new();
    let kids = n.child_count();
    if let (Some(f), Some(z)) = (n.child(0), n.child(kids.saturating_sub(1)))
        && kids >= 2
        && QUOTES.iter().any(|q| *q == leaf(z) && leaf(f).ends_with(q))
    {
        found.push((f, z));
    }
    let mut open: Vec<Node> = Vec::new();
    let mut walk = n.walk();
    for k in n.children(&mut walk) {
        let t = leaf(k);
        if let Some(i) = open
            .iter()
            .rposition(|&o| PAIRS.iter().any(|(a, b)| *a == leaf(o) && *b == t))
        {
            found.push((open[i], k));
            open.truncate(i);
        } else if PAIRS.iter().any(|(a, _)| *a == t) {
            open.push(k);
        }
    }
    found
}
