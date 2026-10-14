use tree_sitter::Tree;

use super::{Lang, Parsed, changed_match_row, layers};

fn near(r: &tree_sitter::Range, span: &std::ops::Range<usize>) -> bool {
    r.start_byte <= span.end && r.end_byte >= span.start
}

impl Parsed {
    fn regroup_span(
        &mut self,
        tree: &Tree,
        dirty: Option<std::ops::Range<usize>>,
    ) -> std::ops::Range<usize> {
        let bytes = self.text.as_bytes();
        let Some(d) = dirty.filter(|_| !std::mem::take(&mut self.rescan)) else {
            return 0..bytes.len();
        };
        let mut span = layers::touched(
            self.lang,
            tree,
            &layers::Doc::new(bytes, &self.starts, d.start, d.end),
        );
        let ranges: Vec<_> = self
            .groups
            .iter()
            .flat_map(|g| g.tree.included_ranges())
            .collect();
        while let Some(r) = ranges
            .iter()
            .find(|r| near(r, &span) && (r.start_byte < span.start || r.end_byte > span.end))
        {
            span = span.start.min(r.start_byte)..span.end.max(r.end_byte);
        }
        span
    }

    pub(super) fn regroup(&mut self, tree: &Tree, dirty: Option<std::ops::Range<usize>>) -> usize {
        if self.lang.combined.is_none() {
            return usize::MAX;
        }
        let span = self.regroup_span(tree, dirty);
        let bytes = self.text.as_bytes();
        let doc = layers::Doc::new(bytes, &self.starts, span.start, span.end);
        let found = layers::combined(self.lang, tree, &doc);
        let mut langs: Vec<&'static Lang> = self.groups.iter().map(|g| g.lang).collect();
        for (child, _) in &found {
            if !langs.iter().any(|l| std::ptr::eq(*l, *child)) {
                langs.push(child);
            }
        }
        let mut old = std::mem::take(&mut self.groups);
        let mut from = usize::MAX;
        for child in langs {
            let before = old
                .iter()
                .position(|g| std::ptr::eq(g.lang, child))
                .map(|k| old.remove(k).tree);
            let was = before
                .as_ref()
                .map(Tree::included_ranges)
                .unwrap_or_default();
            let mut members: Vec<_> = was.iter().filter(|r| !near(r, &span)).copied().collect();
            members.extend(
                found
                    .iter()
                    .filter(|(l, _)| std::ptr::eq(*l, child))
                    .flat_map(|(_, r)| r.iter().copied()),
            );
            members.sort_by_key(|r| (r.start_byte, r.end_byte));
            members.dedup_by_key(|r| (r.start_byte, r.end_byte));
            if let Some(b) = before
                .as_ref()
                .filter(|_| members == was && !was.iter().any(|r| near(r, &span)))
            {
                self.groups.push(layers::Group {
                    lang: child,
                    tree: b.clone(),
                });
                continue;
            }
            let first = members.first().into_iter().chain(was.first());
            let first = first
                .map(|r| r.start_point.row)
                .fold(usize::MAX, usize::min);
            if members.is_empty() {
                from = from.min(first);
                continue;
            }
            let Some(new) = layers::parse(&mut self.inner, child, &members, bytes, before.as_ref())
            else {
                (from, self.rescan) = (from.min(first), true);
                continue;
            };
            match &before {
                None => from = from.min(first),
                Some(b) => {
                    from = b
                        .changed_ranges(&new)
                        .map(|r| {
                            let range = r.start_byte..r.end_byte;
                            let row = changed_match_row(&child.highlights, b, &new, range, bytes);
                            r.start_point.row.min(row)
                        })
                        .fold(from, usize::min);
                }
            }
            self.groups.push(layers::Group {
                lang: child,
                tree: new,
            });
        }
        from
    }
}
