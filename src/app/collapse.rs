use super::*;

impl App {
    pub(super) fn toggle_collapse(&mut self) {
        if self.deleted.is_some() || self.previewing() {
            return;
        }
        let l = self.line;
        if let Some(i) = self.collapsed.iter().position(|&(h, _)| h == l) {
            self.collapsed.remove(i);
            return;
        }
        let (kind, lines) = (self.kind(), &self.buf.lines);
        let l = (0..=l)
            .rev()
            .find(|&i| !lines[i].trim().is_empty())
            .unwrap_or(l);
        let covers = |h: usize| block_end(lines, h).is_some_and(|e| e >= l);
        let declares = || search::enclosing_declarations(kind, lines, l + 1).last() == Some(&l);
        let opens = covers(l) && (lines[l].trim_end().ends_with(['{', ':']) || declares());
        let target = opens
            .then_some(l)
            .or_else(|| {
                let decls = search::enclosing_declarations(kind, lines, l);
                decls.into_iter().rev().find(|&d| covers(d))
            })
            .or_else(|| {
                let deep = depth(lines, l);
                (0..l)
                    .rev()
                    .find(|&h| !lines[h].trim().is_empty() && indent(&lines[h]) < deep && covers(h))
            })
            .or_else(|| covers(l).then_some(l));
        let Some(h) = target else {
            self.message = "nothing to fold".into();
            return;
        };
        let end = block_end(lines, h).unwrap();
        self.collapsed.push((h, end));
        self.nest_collapsed();
        self.anchor = None;
        self.set_at(TextLine::File(h));
        self.apply_want_x(0);
    }

    pub fn hidden(&self, l: usize) -> bool {
        let at = self.at().key();
        let inside = |l: usize, (h, e): (usize, usize)| h < l && l <= e;
        (self.collapsed.iter()).any(|&f| inside(l, f) && !inside(at, f))
    }

    pub fn collapsed_tail(&self, l: usize) -> Option<&str> {
        let e = self.collapsed_at(l)?;
        let (head, end) = (&self.buf.lines[l], &self.buf.lines[e]);
        let level = indent(head) == indent(end) && closes(end.trim());
        Some(if level { end.trim() } else { "" })
    }

    pub fn collapsed_at(&self, l: usize) -> Option<usize> {
        self.collapsed
            .iter()
            .find(|&&(h, _)| h == l)
            .map(|&(_, e)| e)
    }

    pub(super) fn reveal_cursor(&mut self) {
        self.unfold_over(self.at().key());
    }

    pub(super) fn unfold_over(&mut self, l: usize) {
        self.collapsed.retain(|&(h, e)| !(h < l && l <= e));
    }

    pub(super) fn shift_collapsed(&mut self, at: usize, old: usize, new: usize) {
        let lines = &self.buf.lines;
        let moved = |(h, e): (usize, usize)| match () {
            _ if (at..at + old).contains(&h) => None,
            _ if e < at => Some((h, e)),
            _ if h >= at + old => Some((h + new - old, e + new - old)),
            _ => Some((h, block_end(lines, h)?)),
        };
        self.collapsed = self.collapsed.iter().filter_map(|&f| moved(f)).collect();
        self.nest_collapsed();
    }

    pub(super) fn carry_collapsed(&mut self, old: &[String]) {
        let heads: Vec<usize> = (self.collapsed.iter())
            .map(|&(h, _)| (h, super::open::carried(old, &self.buf.lines, h)))
            .filter(|&(h, n)| self.buf.lines.get(n) == old.get(h))
            .map(|(_, n)| n)
            .collect();
        self.measure_collapsed(heads);
    }

    pub(super) fn swap_collapsed(&mut self, old: &Buffer, path: &Path) {
        let left = std::mem::take(&mut self.collapsed);
        if let Some(p) = old.path.clone()
            && !left.is_empty()
        {
            let heads = left.iter().map(|&(h, _)| (h, old.lines[h].clone()));
            self.collapsed_stash.insert(p, heads.collect());
        }
        let heads = (self.collapsed_stash.remove(path).into_iter().flatten())
            .filter(|(h, text)| self.buf.lines.get(*h) == Some(text))
            .map(|(h, _)| h)
            .collect();
        self.measure_collapsed(heads);
    }

    fn measure_collapsed(&mut self, heads: Vec<usize>) {
        self.collapsed = heads
            .into_iter()
            .filter_map(|h| Some((h, block_end(&self.buf.lines, h)?)))
            .collect();
        self.nest_collapsed();
    }

    fn nest_collapsed(&mut self) {
        self.collapsed.sort_unstable();
        self.collapsed.dedup_by_key(|f| f.0);
        for i in (0..self.collapsed.len()).rev() {
            let (h, mut e) = self.collapsed[i];
            for &(h2, e2) in &self.collapsed[i + 1..] {
                if h2 <= e {
                    e = e.max(e2);
                }
            }
            self.collapsed[i] = (h, e);
        }
    }
}

pub(crate) fn block_end(lines: &[String], h: usize) -> Option<usize> {
    let head = lines.get(h).filter(|s| !s.trim().is_empty())?;
    let ind = indent(head);
    let mut end = None;
    for (i, line) in lines.iter().enumerate().skip(h + 1) {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if indent(line) > ind {
            end = Some(i);
            continue;
        }
        if indent(line) < ind {
            break;
        }
        if t == "{" || (t.starts_with(')') && !closes(t)) {
            end = Some(i);
            continue;
        }
        if end.is_some() && closes(t) {
            end = Some(i);
        }
        break;
    }
    end
}

fn closes(t: &str) -> bool {
    let brackets = t.starts_with([')', ']', '}'])
        && t.chars()
            .all(|c| matches!(c, ')' | ']' | '}' | ';' | ',' | ' '));
    let end = t
        .strip_prefix("end")
        .is_some_and(|r| r.chars().all(|c| matches!(c, ';' | ')' | ',' | ' ')));
    brackets || end || (t.starts_with("</") && t.ends_with('>'))
}

fn indent(s: &str) -> usize {
    s.len() - s.trim_start().len()
}

fn depth(lines: &[String], l: usize) -> usize {
    (lines[l..].iter())
        .find(|s| !s.trim().is_empty())
        .map_or(0, |s| indent(s))
}
