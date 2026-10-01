//! `f`: folding a function or a block of the open file into its first line (#598).
//!
//! merl has no parser, so a block is read off the indentation, as VS Code's
//! `editor.foldingStrategy: indentation` reads it: a line and the lines under it indented
//! deeper, with the line that closes it (`}`, `end`, `)`) when that stands at the first line's
//! own indent. Called `collapsed` here: a fold is already the review's folded generated file.

use super::*;

impl App {
    /// `f`: unfolds the fold on the cursor's line. Else it folds the block the line opens when
    /// the line declares something or ends in `{` or `:`, not a call or a sentence wrapped onto
    /// the next line; else the declaration the cursor is in, the one the pane pins over the text
    /// (#248); else the innermost block around the cursor. The cursor goes to the folded line.
    pub(super) fn toggle_collapse(&mut self) {
        if self.deleted.is_some() || self.preview.is_some() {
            return;
        }
        let l = self.line;
        if let Some(i) = self.collapsed.iter().position(|&(h, _)| h == l) {
            self.collapsed.remove(i);
            return;
        }
        let (kind, lines) = (self.kind(), &self.buf.lines);
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
        self.collapsed.sort_unstable();
        self.anchor = None;
        self.set_at(TextLine::File(h));
        self.apply_want_x(0);
    }

    /// Whether file line `l` is inside a fold: drawn in no row, stepped over by the cursor.
    pub fn hidden(&self, l: usize) -> bool {
        self.collapsed.iter().any(|&(h, e)| h < l && l <= e)
    }

    /// What the pane draws after the folded line `l`, behind its `⋯`: the bracket or `end` that
    /// closes the fold, when its last hidden line is one at `l`'s own indent, else nothing.
    pub fn collapsed_tail(&self, l: usize) -> Option<&str> {
        let e = self.collapsed_at(l)?;
        let (head, end) = (&self.buf.lines[l], &self.buf.lines[e]);
        let level = indent(head) == indent(end) && closes(end.trim());
        Some(if level { end.trim() } else { "" })
    }

    /// The fold whose first line is `l`, as its last hidden line.
    pub fn collapsed_at(&self, l: usize) -> Option<usize> {
        self.collapsed
            .iter()
            .find(|&&(h, _)| h == l)
            .map(|&(_, e)| e)
    }

    /// Opens every fold the cursor's line is hidden in: whatever brought it there (`:`, `/`,
    /// `d`, `[`, an undo) meant to show that line.
    pub(super) fn reveal_cursor(&mut self) {
        let l = self.at().key();
        self.collapsed.retain(|&(h, e)| !(h < l && l <= e));
    }

    /// After `old` lines from `at` on became `new` lines: the folds below move with their text,
    /// one whose first line was edited opens, and every fold is measured again, since its body
    /// may have changed.
    pub(super) fn shift_collapsed(&mut self, at: usize, old: usize, new: usize) {
        let moved = |h: usize| match h {
            h if h < at => Some(h),
            h if h < at + old => None,
            h => Some(h + new - old),
        };
        let heads: Vec<usize> = self
            .collapsed
            .iter()
            .filter_map(|&(h, _)| moved(h))
            .collect();
        self.measure_collapsed(heads);
    }

    /// The folds of a file whose text was replaced by `old` → the buffer (a reload): each one
    /// goes with its first line, and opens when that line no longer says the same.
    pub(super) fn carry_collapsed(&mut self, old: &[String]) {
        let heads: Vec<usize> = (self.collapsed.iter())
            .map(|&(h, _)| (h, super::open::carried(old, &self.buf.lines, h)))
            .filter(|&(h, n)| self.buf.lines.get(n) == old.get(h))
            .map(|(_, n)| n)
            .collect();
        self.measure_collapsed(heads);
    }

    /// Opening `path` over `old`: the folds of `old` are put aside with what their first lines
    /// said, and those of `path` come back where their lines still say the same.
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

    /// The folds on these first lines, each down to where its block ends now; a line that opens
    /// no block any more folds nothing.
    fn measure_collapsed(&mut self, heads: Vec<usize>) {
        self.collapsed = heads
            .into_iter()
            .filter_map(|h| Some((h, block_end(&self.buf.lines, h)?)))
            .collect();
    }
}

/// The last line of the block `lines[h]` opens: the lines after it indented deeper, blank lines
/// among them, and at its own indent a `{` or a `)`-led tail that goes on with the header, and
/// the bracket or `end` that closes the block. `None` when no deeper line follows.
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
        // `{` under a C header, `) -> Result<()> {` under a wrapped signature: still the header.
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

/// Whether trimmed line `t` only closes a block: brackets with a `;` or `,`, `end`, `</tag>`.
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

/// How deep line `l` stands: its indent, or for a blank line that of the next line with text.
fn depth(lines: &[String], l: usize) -> usize {
    (lines[l..].iter())
        .find(|s| !s.trim().is_empty())
        .map_or(0, |s| indent(s))
}
