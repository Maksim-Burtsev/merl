use ratatui::style::Style;
use syntect::highlighting::{HighlightIterator, HighlightState};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference};
use tree_sitter::{Node, Parser, Point, QueryCursor, Range, StreamingIterator, Tree};

use super::{Lang, NEOVIM_TREESITTER_PRIORITY, Styles, point, resolve};

const MAX_DEPTH: usize = 8;

pub enum Target {
    Tree(&'static Lang),
    Syntect(&'static SyntaxReference),
}

impl Target {
    fn of(name: &str, syntect: bool) -> Option<Self> {
        resolve(name).map(Target::Tree).or_else(|| {
            syntect
                .then(|| crate::buffer::syntax_for_info(name))
                .flatten()
                .map(Target::Syntect)
        })
    }

    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Target::Tree(a), Target::Tree(b)) => std::ptr::eq(*a, *b),
            (Target::Syntect(a), Target::Syntect(b)) => std::ptr::eq(*a, *b),
            _ => false,
        }
    }
}

pub struct Injected {
    target: Target,
    regions: Vec<Vec<Range>>,
}

pub struct Doc<'a> {
    bytes: &'a [u8],
    starts: &'a [usize],
    start: usize,
    end: usize,
}

pub struct Paint {
    pub prio: u32,
    pub sub: u32,
    pub a: usize,
    pub b: usize,
    pub style: Style,
}

pub struct Ctx<'a, 'b> {
    pub parser: &'a mut Parser,
    pub sub: u32,
    pub depth: usize,
    pub styles: &'a mut Styles<'b>,
    pub out: &'a mut Vec<Paint>,
}

impl<'a> Doc<'a> {
    pub fn new(bytes: &'a [u8], starts: &'a [usize], start: usize, end: usize) -> Self {
        Self {
            bytes,
            starts,
            start,
            end,
        }
    }

    fn byte(&self, p: Point) -> usize {
        let row = p.row.min(self.starts.len() - 1);
        (self.starts[row] + p.column).min(self.bytes.len())
    }

    fn range(&self, node: Node, offset: Option<[i64; 4]>) -> Range {
        let r = node.range();
        let Some([sr, sc, er, ec]) = offset else {
            return r;
        };
        let at = |p: Point, dr: i64, dc: i64| {
            Point::new(
                (p.row as i64 + dr).max(0) as usize,
                (p.column as i64 + dc).max(0) as usize,
            )
        };
        let (s, e) = (at(r.start_point, sr, sc), at(r.end_point, er, ec));
        if (s.row, s.column) > (e.row, e.column) {
            return r;
        }
        Range {
            start_byte: self.byte(s),
            end_byte: self.byte(e),
            start_point: s,
            end_point: e,
        }
    }

    fn node_ranges(&self, node: Node, offset: Option<[i64; 4]>, children: bool) -> Vec<Range> {
        let range = self.range(node, offset);
        if children || node.named_child_count() == 0 {
            return vec![range];
        }
        let mut out = Vec::new();
        let (mut sp, mut sb) = (range.start_point, range.start_byte);
        let mut piece = |from: Point, from_byte: usize, to: Point, to_byte: usize| {
            if from_byte < to_byte {
                out.push(Range {
                    start_byte: from_byte,
                    end_byte: to_byte,
                    start_point: from,
                    end_point: to,
                });
            }
        };
        let mut walk = node.walk();
        for child in node.named_children(&mut walk) {
            let cs = child.start_position();
            if cs.row > sp.row || cs.column > sp.column {
                piece(sp, sb, cs, child.start_byte());
            }
            (sp, sb) = (child.end_position(), child.end_byte());
        }
        let ep = range.end_point;
        if ep.row > sp.row || ep.column > sp.column {
            piece(sp, sb, ep, range.end_byte);
        }
        out
    }

    fn touches(&self, region: &[Range]) -> bool {
        region
            .iter()
            .any(|r| r.start_byte < self.end && r.end_byte > self.start)
    }
}

fn clip(ranges: Vec<Range>, parent: Option<&[Range]>) -> Vec<Range> {
    let Some(parent) = parent else {
        return ranges;
    };
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < ranges.len() && j < parent.len() {
        let (r1, r2) = (&ranges[i], &parent[j]);
        let (a, b) = (
            r1.start_byte.max(r2.start_byte),
            r1.end_byte.min(r2.end_byte),
        );
        if a < b {
            let start = if r1.start_byte >= r2.start_byte {
                r1
            } else {
                r2
            };
            let end = if r1.end_byte <= r2.end_byte { r1 } else { r2 };
            out.push(Range {
                start_byte: a,
                end_byte: b,
                start_point: start.start_point,
                end_point: end.end_point,
            });
        }
        if r1.end_byte <= r2.end_byte {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

fn merge(into: &mut Vec<Injected>, from: Vec<Injected>) {
    for inj in from {
        match into.iter_mut().find(|i| i.target.same(&inj.target)) {
            Some(i) => i.regions.extend(inj.regions),
            None => into.push(inj),
        }
    }
}

pub fn injections(
    lang: &'static Lang,
    tree: &Tree,
    parent: Option<&[Range]>,
    doc: &Doc,
) -> Vec<Injected> {
    let Some(inj) = &lang.injections else {
        return Vec::new();
    };
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(doc.start..doc.end);
    let names = inj.query.capture_names();
    let mut out: Vec<Injected> = Vec::new();
    let mut combined: Vec<(usize, usize, usize)> = Vec::new();
    let syntect = lang.name.starts_with("markdown");
    let mut matches = cursor.matches(&inj.query, tree.root_node(), doc.bytes);
    while let Some(m) = matches.next() {
        if !inj.matches(m, doc.bytes) {
            continue;
        }
        let p = m.pattern_index;
        let mut target = match inj.setting(p, "injection.self") {
            Some(_) => Some(Target::Tree(lang)),
            None => inj
                .setting(p, "injection.language")
                .flatten()
                .and_then(|n| Target::of(n, syntect)),
        };
        let children = inj.setting(p, "injection.include-children").is_some();
        let mut ranges = Vec::new();
        for c in m.captures() {
            let offset = inj.offset(p, c.index);
            match names[c.index as usize] {
                "injection.language" => {
                    let r = doc.range(c.node, offset);
                    let text = inj.text(p, c.index, &doc.bytes[r.start_byte..r.end_byte]);
                    target = std::str::from_utf8(&text)
                        .ok()
                        .and_then(|n| Target::of(n, syntect));
                }
                "injection.content" => ranges.extend(doc.node_ranges(c.node, offset, children)),
                _ => {}
            }
        }
        let Some(target) = target else { continue };
        let ranges = clip(ranges, parent);
        if ranges.is_empty() {
            continue;
        }
        let k = match out.iter().position(|i| i.target.same(&target)) {
            Some(k) => k,
            None => {
                out.push(Injected {
                    target,
                    regions: Vec::new(),
                });
                out.len() - 1
            }
        };
        if inj.setting(p, "injection.combined").is_none() {
            out[k].regions.push(ranges);
            continue;
        }
        match combined.iter().find(|(cp, ck, _)| *cp == p && *ck == k) {
            Some(&(_, _, r)) => out[k].regions[r].extend(ranges),
            None => {
                combined.push((p, k, out[k].regions.len()));
                out[k].regions.push(ranges);
            }
        }
    }
    out
}

pub fn region_start(injected: &[Injected], row: usize) -> usize {
    let spans = injected.iter().flat_map(|i| {
        i.regions.iter().flat_map(move |r| match i.target {
            Target::Tree(_) => match (r.first(), r.last()) {
                (Some(a), Some(b)) => vec![(a.start_point.row, b.end_point.row)],
                _ => Vec::new(),
            },
            Target::Syntect(_) => r
                .iter()
                .map(|x| (x.start_point.row, x.end_point.row))
                .collect(),
        })
    });
    spans
        .filter(|&(first, last)| first <= row && row <= last)
        .map(|(first, _)| first)
        .fold(row, usize::min)
}

fn parse(parser: &mut Parser, lang: &Lang, region: &[Range], bytes: &[u8]) -> Option<Tree> {
    parser.set_language(&lang.language).ok()?;
    parser.set_included_ranges(region).ok()?;
    parser.parse_with_options(&mut |i, _| &bytes[i.min(bytes.len())..], None, None)
}

pub fn layer(lang: &'static Lang, trees: &[Tree], parent: bool, ctx: &mut Ctx, doc: &Doc) {
    for tree in trees {
        ctx.sub += 1;
        paint(lang, tree, ctx, doc);
    }
    if ctx.depth >= MAX_DEPTH {
        return;
    }
    let mut children = Vec::new();
    for tree in trees {
        let ranges = tree.included_ranges();
        let clip_to = parent.then_some(&ranges[..]);
        merge(&mut children, injections(lang, tree, clip_to, doc));
    }
    for inj in &children {
        let regions = inj.regions.iter().filter(|r| doc.touches(r));
        match inj.target {
            Target::Tree(child) => {
                let trees: Vec<Tree> = regions
                    .filter_map(|r| parse(ctx.parser, child, r, doc.bytes))
                    .collect();
                ctx.depth += 1;
                layer(child, &trees, true, ctx, doc);
                ctx.depth -= 1;
            }
            Target::Syntect(syntax) => {
                for region in regions {
                    ctx.sub += 1;
                    syntect(syntax, region, ctx, doc);
                }
            }
        }
    }
}

fn paint(lang: &Lang, tree: &Tree, ctx: &mut Ctx, doc: &Doc) {
    let included = tree.included_ranges();
    let q = &lang.highlights;
    let styles = ctx.styles.of(lang);
    let names = q.query.capture_names();
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(doc.start..doc.bytes.len());
    let mut checked: Option<(u32, bool)> = None;
    let mut captures = cursor.captures(&q.query, tree.root_node(), doc.bytes);
    while let Some((m, i)) = captures.next() {
        let c = m.captures()[*i];
        if c.node.start_byte() >= doc.end {
            break;
        }
        let ok = match checked {
            Some((id, ok)) if id == m.id() => ok,
            _ => {
                let ok = q.matches(m, doc.bytes);
                checked = Some((m.id(), ok));
                ok
            }
        };
        let Some(style) = styles[c.index as usize].filter(|_| ok) else {
            continue;
        };
        let nospell = u32::from(names[c.index as usize] == "nospell");
        let prio = lang.priority[m.pattern_index] + nospell;
        let r = doc.range(c.node, q.offset(m.pattern_index, c.index));
        for inc in &included {
            let (a, b) = (
                r.start_byte.max(inc.start_byte),
                r.end_byte.min(inc.end_byte),
            );
            if a < b {
                ctx.out.push(Paint {
                    prio,
                    sub: ctx.sub,
                    a,
                    b,
                    style,
                });
            }
        }
    }
}

fn syntect(syntax: &'static SyntaxReference, region: &[Range], ctx: &mut Ctx, doc: &Doc) {
    let highlighter = &ctx.styles.highlighter;
    for r in region {
        let mut parse = ParseState::new(syntax);
        let mut state = HighlightState::new(highlighter, ScopeStack::new());
        let mut a = r.start_byte;
        while a < r.end_byte && a < doc.end {
            let row = point(doc.starts, a).row;
            let eol = doc.starts.get(row + 1).map_or(doc.bytes.len(), |s| s - 1);
            let b = eol.min(r.end_byte);
            let Ok(text) = std::str::from_utf8(&doc.bytes[a..b]) else {
                break;
            };
            let line = format!("{text}\n");
            let ops = parse
                .parse_line(&line, crate::buffer::syntaxes())
                .unwrap_or_default();
            let mut at = a;
            for (style, piece) in HighlightIterator::new(&mut state, &ops, &line, highlighter) {
                let s = at;
                at += piece.len();
                let e = at.min(b);
                if s < e && e > doc.start {
                    ctx.out.push(Paint {
                        prio: NEOVIM_TREESITTER_PRIORITY,
                        sub: ctx.sub,
                        a: s,
                        b: e,
                        style: crate::theme::style(style),
                    });
                }
            }
            a = eol + 1;
        }
    }
}
