use std::ops::{ControlFlow, Range as Bytes};
use std::time::Instant;

use tree_sitter::{InputEdit, ParseOptions, ParseState, Point, Range, Tree};

use super::{Lang, Parsed, Patterns, changed_match_row, layers};

const SCAN_STEP: usize = 16 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Member {
    extent: (usize, usize),
    piece: Range,
}

pub struct Group {
    pub pattern: usize,
    pub lang: &'static Lang,
    members: Vec<Member>,
    pub ranges: Vec<Range>,
    pub tree: Option<Tree>,
    #[cfg(test)]
    pub parsed: Vec<u8>,
}

struct Built {
    group: Group,
    old: Option<Tree>,
    parse: bool,
    started: bool,
}

pub struct Regroup {
    span: Bytes<usize>,
    at: usize,
    found: Vec<(usize, &'static Lang, Member)>,
    built: Option<Vec<Built>>,
    from: usize,
}

fn near((start, end): (usize, usize), span: &Bytes<usize>) -> bool {
    start <= span.end && end >= span.start
}

fn shift(b: usize, e: &InputEdit) -> usize {
    if b >= e.old_end_byte {
        b - e.old_end_byte + e.new_end_byte
    } else {
        b.min(e.start_byte)
    }
}

fn shift_point(b: usize, p: Point, e: &InputEdit) -> Point {
    if b < e.old_end_byte {
        return if b > e.start_byte {
            e.start_position
        } else {
            p
        };
    }
    let (old, new) = (e.old_end_position, e.new_end_position);
    let row = p.row.saturating_sub(old.row) + new.row;
    if p.row == old.row {
        Point::new(row, p.column.saturating_sub(old.column) + new.column)
    } else {
        Point::new(row, p.column)
    }
}

fn shifted(r: Range, e: &InputEdit) -> Range {
    Range {
        start_byte: shift(r.start_byte, e),
        end_byte: shift(r.end_byte, e),
        start_point: shift_point(r.start_byte, r.start_point, e),
        end_point: shift_point(r.end_byte, r.end_point, e),
    }
}

pub fn shift_span(span: &Bytes<usize>, e: &InputEdit) -> Bytes<usize> {
    shift(span.start, e)..shift(span.end, e)
}

pub fn union(a: Option<Bytes<usize>>, b: Option<Bytes<usize>>) -> Option<Bytes<usize>> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.start.min(b.start)..a.end.max(b.end)),
        (a, b) => a.or(b),
    }
}

impl Regroup {
    pub fn span(&self) -> &Bytes<usize> {
        &self.span
    }
}

fn first_row(ranges: &[Range]) -> usize {
    ranges.first().map_or(usize::MAX, |r| r.start_point.row)
}

fn marked(mut old: Tree, ranges: &[Range]) -> Tree {
    let was = old.included_ranges();
    let (mut i, mut j) = (0, 0);
    let mut differ: Option<(Range, Range)> = None;
    let mut add = |r: Range| {
        differ = Some(differ.map_or((r, r), |(a, b)| {
            (
                if r.start_byte < a.start_byte { r } else { a },
                if r.end_byte > b.end_byte { r } else { b },
            )
        }));
    };
    while i < was.len() || j < ranges.len() {
        match (was.get(i), ranges.get(j)) {
            (Some(a), Some(b)) if a == b => (i, j) = (i + 1, j + 1),
            (Some(a), Some(b)) if (a.start_byte, a.end_byte) > (b.start_byte, b.end_byte) => {
                add(*b);
                j += 1;
            }
            (Some(a), _) => {
                add(*a);
                i += 1;
            }
            (None, Some(b)) => {
                add(*b);
                j += 1;
            }
            (None, None) => break,
        }
    }
    if let Some((a, b)) = differ {
        old.edit(&InputEdit {
            start_byte: a.start_byte,
            old_end_byte: b.end_byte,
            new_end_byte: b.end_byte,
            start_position: a.start_point,
            old_end_position: b.end_point,
            new_end_position: b.end_point,
        });
    }
    old
}

fn changed_row(
    lang: &Lang,
    old: Option<&Tree>,
    new: &Tree,
    ranges: &[Range],
    bytes: &[u8],
) -> usize {
    let Some(old) = old else {
        return first_row(ranges);
    };
    old.changed_ranges(new)
        .map(|r| {
            let range = r.start_byte..r.end_byte;
            let row = |p: &Patterns| changed_match_row(p, old, new, range.clone(), bytes);
            let injected = lang.injections.as_ref().map_or(usize::MAX, row);
            r.start_point.row.min(row(&lang.highlights)).min(injected)
        })
        .fold(usize::MAX, usize::min)
}

impl Group {
    pub fn touches(&self, start: usize, end: usize) -> bool {
        let i = self.ranges.partition_point(|r| r.end_byte <= start);
        self.ranges.get(i).is_some_and(|r| r.start_byte < end)
    }

    pub fn edit(&mut self, e: &InputEdit) {
        if let Some(tree) = &mut self.tree {
            tree.edit(e);
        }
        let erased = |was: &Range, now: &Range| {
            was.start_byte < was.end_byte && now.start_byte == now.end_byte
        };
        self.members.retain_mut(|m| {
            let piece = shifted(m.piece, e);
            m.extent = (shift(m.extent.0, e), shift(m.extent.1, e));
            let kept = !erased(&m.piece, &piece);
            m.piece = piece;
            kept
        });
        self.ranges.retain_mut(|r| {
            let now = shifted(*r, e);
            let kept = !erased(r, &now);
            *r = now;
            kept
        });
    }
}

impl Parsed {
    pub(super) fn start_regroup(&mut self, span: Bytes<usize>) {
        let len = self.text.len();
        let mut span = span.start.saturating_sub(1)..(span.end + 1).min(len);
        let mut grown = true;
        while grown {
            grown = false;
            for m in self.groups.iter().flat_map(|g| &g.members) {
                if near(m.extent, &span) && (m.extent.0 < span.start || m.extent.1 > span.end) {
                    span = span.start.min(m.extent.0)..span.end.max(m.extent.1);
                    grown = true;
                }
            }
        }
        self.regroup = Some(Regroup {
            at: span.start,
            span,
            found: Vec::new(),
            built: None,
            from: usize::MAX,
        });
    }

    pub(super) fn step_regroup(&mut self, deadline: Option<Instant>) -> Option<usize> {
        let Some(mut job) = self.regroup.take() else {
            return Some(usize::MAX);
        };
        let late = || deadline.is_some_and(|d| Instant::now() >= d);
        let bytes = self.text.as_bytes();
        if let Some(tree) = &self.tree {
            while job.at < job.span.end {
                let end = (job.at + SCAN_STEP).min(job.span.end);
                let doc = layers::Doc::new(bytes, &self.starts, job.at, end);
                layers::members(self.lang, tree, &doc, |pattern, lang, extent, pieces| {
                    for piece in pieces {
                        let extent = (extent.start, extent.end);
                        job.found.push((pattern, lang, Member { extent, piece }));
                    }
                });
                job.at = end;
                if job.at < job.span.end && late() {
                    self.regroup = Some(job);
                    return None;
                }
            }
        }
        let mut built = match job.built.take() {
            Some(built) => built,
            None => self.build(&job.span, &job.found, &mut job.from),
        };
        let mut halted = false;
        for b in built.iter_mut().filter(|b| b.parse) {
            if !b.started {
                b.started = true;
                b.old = b.group.tree.take().map(|t| marked(t, &b.group.ranges));
                if self.grouper.set_language(&b.group.lang.language).is_err()
                    || self.grouper.set_included_ranges(&b.group.ranges).is_err()
                {
                    let was = b.old.take().map(|t| first_row(&t.included_ranges()));
                    let row = first_row(&b.group.ranges).min(was.unwrap_or(usize::MAX));
                    (job.from, b.parse) = (job.from.min(row), false);
                    continue;
                }
            }
            let mut on_time = |_: &ParseState| {
                if late() {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            };
            let Some(new) = self.grouper.parse_with_options(
                &mut |i, _| &bytes[i.min(bytes.len())..],
                b.old.as_ref(),
                Some(ParseOptions::new().progress_callback(&mut on_time)),
            ) else {
                halted = true;
                break;
            };
            let row = changed_row(b.group.lang, b.old.as_ref(), &new, &b.group.ranges, bytes);
            job.from = job.from.min(row);
            #[cfg(test)]
            {
                b.group.parsed = text_of(&b.group.ranges, bytes);
            }
            (b.group.tree, b.old, b.parse) = (Some(new), None, false);
        }
        if halted {
            job.built = Some(built);
            self.regroup = Some(job);
            return None;
        }
        self.groups = built.into_iter().map(|b| b.group).collect();
        Some(job.from)
    }

    fn build(
        &self,
        span: &Bytes<usize>,
        found: &[(usize, &'static Lang, Member)],
        from: &mut usize,
    ) -> Vec<Built> {
        let mut keys: Vec<(usize, &'static Lang)> =
            self.groups.iter().map(|g| (g.pattern, g.lang)).collect();
        for &(p, l, _) in found {
            if !keys.iter().any(|&(kp, kl)| kp == p && std::ptr::eq(kl, l)) {
                keys.push((p, l));
            }
        }
        let mut out = Vec::new();
        for (pattern, lang) in keys {
            let old = self
                .groups
                .iter()
                .find(|g| g.pattern == pattern && std::ptr::eq(g.lang, lang));
            let mut members: Vec<Member> = old
                .map(|g| {
                    g.members
                        .iter()
                        .filter(|m| !near(m.extent, span))
                        .copied()
                        .collect()
                })
                .unwrap_or_default();
            members.extend(
                found
                    .iter()
                    .filter(|&&(p, l, _)| p == pattern && std::ptr::eq(l, lang))
                    .map(|f| f.2),
            );
            let key = |m: &Member| (m.piece.start_byte, m.piece.end_byte, m.extent);
            members.sort_by_key(key);
            members.dedup();
            let mut ranges: Vec<Range> = members.iter().map(|m| m.piece).collect();
            ranges.dedup_by_key(|r| (r.start_byte, r.end_byte));
            if ranges.is_empty() {
                *from = (*from).min(old.map_or(usize::MAX, |g| first_row(&g.ranges)));
                continue;
            }
            let parse = old.is_none_or(|g| {
                g.tree
                    .as_ref()
                    .map_or(g.ranges.clone(), Tree::included_ranges)
                    != ranges
                    || g.ranges
                        .iter()
                        .any(|r| near((r.start_byte, r.end_byte), span))
            });
            out.push(Built {
                group: Group {
                    pattern,
                    lang,
                    members,
                    ranges,
                    tree: old.and_then(|g| g.tree.clone()),
                    #[cfg(test)]
                    parsed: old.map(|g| g.parsed.clone()).unwrap_or_default(),
                },
                old: None,
                parse,
                started: false,
            });
        }
        out
    }
}

#[cfg(test)]
pub fn text_of(ranges: &[Range], bytes: &[u8]) -> Vec<u8> {
    ranges
        .iter()
        .flat_map(|r| &bytes[r.start_byte.min(bytes.len())..r.end_byte.min(bytes.len())])
        .copied()
        .collect()
}
