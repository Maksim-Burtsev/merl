//! `p`: a Markdown file rendered in place of its source (#249), at the same place both ways.

use std::hash::{Hash, Hasher};

use super::*;
use crate::markdown::{self, Doc, Kind};

/// The preview of the open file: its rows at the pane's width, the row the cursor is on and the
/// row at the top of the pane.
pub struct Preview {
    pub doc: Doc,
    laid_out_from: LayoutInput,
    pub row: usize,
    pub top: usize,
    /// The source position the cursor row was found for, or put the cursor at. A cursor found
    /// anywhere else was moved by something else (`:`, `[`, `c`, a reload), and the row follows
    /// it; `None` after a jump, which always finds the row of the line it lands on anew.
    at: Option<(usize, usize)>,
    /// The code blocks, highlighted as far as they have been on screen with the theme `theme`
    /// names, as the source is highlighted: a block costs what is drawn of it.
    pub code: Vec<Buffer>,
    pub theme: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct LayoutInput {
    lines_hash: u64,
    width: usize,
    diagrams_laid: u64,
}

impl App {
    pub fn previewing(&self) -> bool {
        self.buf
            .path
            .as_ref()
            .is_some_and(|p| self.previewed.contains(p))
    }

    /// `p`: the open Markdown file rendered, or its source again. The cursor row stays as far
    /// down the pane, on the same place of the file.
    pub(super) fn toggle_preview(&mut self) {
        let Some(path) = self.buf.path.clone() else {
            return;
        };
        if self.previewed.remove(&path) {
            let off = self
                .preview
                .as_ref()
                .map_or(0, |p| p.row.saturating_sub(p.top));
            let cur = self.cursor_at();
            (self.top_line, self.top_row) = self.back_rows(cur, off);
            self.sync_want_x();
            return;
        }
        if !markdown::is_markdown(&path) {
            self.message = "not Markdown".into();
            return;
        }
        // A cursor above the pane (the pane scrolled onto the lines deleted after the last one,
        // where no row of text reaches) goes to the top.
        let (top, cur) = ((self.top_line, self.top_row), self.cursor_at());
        let off = match cur < top {
            true => 0,
            false => self
                .rows_between(top, cur)
                .min(self.view_h.saturating_sub(1)),
        };
        self.previewed.insert(path);
        if self.deleted.is_some() {
            self.go((self.line, 0));
        }
        // A selection the preview cannot draw would be what Ctrl+C copies.
        self.anchor = None;
        // Back to the row the preview left, when the cursor has not moved since; else the row
        // of the cursor's line, where the cursor was on screen.
        if self.preview_sync()
            && let Some(p) = &mut self.preview
        {
            p.top = p.row.saturating_sub(off);
        }
        self.preview_clamp();
    }

    /// Lays the file out when the text or the width changed, and puts the cursor row on the
    /// cursor's source position when something else moved the cursor. Returns whether the row
    /// was found anew.
    pub(crate) fn preview_sync(&mut self) -> bool {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.buf.lines.hash(&mut h);
        let input = LayoutInput {
            lines_hash: h.finish(),
            width: self.view_w,
            diagrams_laid: self.diagrams.laid,
        };
        let pos = (self.line, self.col);
        match &mut self.preview {
            Some(p) if p.laid_out_from == input => {
                if p.at == Some(pos) {
                    return false;
                }
                p.row = p.doc.row_at(pos);
                p.at = Some(pos);
            }
            slot => {
                // Laid out anew, for a rewrite or another width: the row stays where it was on
                // screen, the same row when the cursor has not moved.
                let old = slot.take();
                let off = old.as_ref().map_or(0, |p| p.row.saturating_sub(p.top));
                let diagrams = &mut self.diagrams;
                let doc = markdown::layout(&self.buf.lines, self.view_w, &mut |src, room| {
                    diagrams.fit(src, room)
                });
                let row = match &old {
                    Some(p) if p.at == Some(pos) => doc.same_row(&p.doc, p.row, pos),
                    _ => doc.row_at(pos),
                };
                // Code keeps its colours across another width: the text is the same.
                let (code, theme) = match old {
                    Some(p) if p.laid_out_from.lines_hash == input.lines_hash => (p.code, p.theme),
                    _ => (
                        doc.code
                            .iter()
                            .map(|c| Buffer::block(&c.lang, c.lines.clone()))
                            .collect(),
                        String::new(),
                    ),
                };
                *slot = Some(Preview {
                    doc,
                    laid_out_from: input,
                    row,
                    top: row.saturating_sub(off),
                    at: Some(pos),
                    code,
                    theme,
                });
            }
        }
        true
    }

    /// Scrolls the least that shows the cursor row, or centres it after a jump, and never past
    /// the last row on the bottom of the pane.
    pub(crate) fn preview_clamp(&mut self) {
        let h = self.view_h.max(1);
        let center = std::mem::take(&mut self.center);
        let Some(p) = &mut self.preview else {
            return;
        };
        let last = p.doc.rows.len() - 1;
        p.row = p.row.min(last);
        if center {
            p.top = p.row.saturating_sub(h / 2);
        }
        if p.row < p.top {
            p.top = p.row;
        }
        if p.row >= p.top + h {
            p.top = p.row + 1 - h;
        }
        p.top = p.top.min((last + 1).saturating_sub(h));
    }

    /// The keys of the preview, whichever pane has the focus. Reading keys move the cursor row;
    /// Enter edits the source at it. The keys that act on the file, a pane or merl do what they
    /// do everywhere (`false`), and so do the tree's own when it has the focus. Every other key
    /// acts on a word, a column or a selection, has nothing to act on here, and does nothing.
    pub(super) fn preview_key(&mut self, key: KeyEvent) -> bool {
        if !self.previewing() {
            return false;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let plain = key.modifiers.is_empty();
        let tree = matches!(
            key.code,
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right | KeyCode::Enter
        );
        // `c` / `C` walk the review on the source: the preview is left for them.
        if self.review.is_some() && matches!(key.code, KeyCode::Char('c' | 'C')) {
            self.toggle_preview();
            return false;
        }
        let elsewhere = match key.code {
            KeyCode::Esc | KeyCode::Tab => true,
            KeyCode::Char(c) if ctrl => matches!(c, 'g' | 'n' | 'e' | 's' | 'r' | 'z' | 'y'),
            KeyCode::Char(c) => "q?:sDotT[]cCmp".contains(c),
            _ => false,
        };
        if elsewhere || tree && self.focus == Focus::Tree {
            return false;
        }
        self.preview_sync();
        let Some(p) = &self.preview else {
            return false;
        };
        let (row, top, last) = (p.row, p.top, p.doc.rows.len() - 1);
        let (h, half) = (self.view_h.max(1), (self.view_h / 2).max(1));
        let gap = |i: &usize| p.doc.rows[*i].kind == Kind::Gap;
        // Where the key goes, how the view goes with it, and whether it reads on by rows (the
        // current stop of the jump history follows it, as paging does: one row can stand for
        // many lines) or goes by the source's lines (a far one adds a stop, as in the source).
        let (to, scroll, reads) = match key.code {
            KeyCode::Up if plain => (row.saturating_sub(1), 0, true),
            KeyCode::Down if plain => ((row + 1).min(last), 0, true),
            KeyCode::PageUp if plain => (row.saturating_sub(h), 0, true),
            KeyCode::PageDown if plain => ((row + h).min(last), 0, true),
            KeyCode::Char('u') if ctrl => (row.saturating_sub(half), -1, true),
            KeyCode::Char('d') if ctrl => ((row + half).min(last), 1, true),
            KeyCode::Home if ctrl => (0, 0, false),
            KeyCode::End if ctrl => (last, 0, false),
            KeyCode::Char('{') => ((0..row).rev().find(gap).unwrap_or(0), 0, false),
            KeyCode::Char('}') => ((row + 1..=last).find(gap).unwrap_or(last), 0, false),
            KeyCode::Enter if plain => {
                self.start_edit();
                return true;
            }
            _ => return true,
        };
        let moved = to.abs_diff(row);
        let top = match scroll {
            -1 => top.saturating_sub(moved),
            1 => top + moved,
            _ => top,
        };
        // A row that shows no line (a border, a rule, a blank row with no blank line under it)
        // stands before the block below it: the cursor goes to the first line that block shows,
        // where Enter edits, or to the end of the file when there is none.
        let rows = &p.doc.rows;
        let src = match rows[to].lines.is_empty() {
            false => rows[to].src,
            true => rows[to..]
                .iter()
                .find(|r| !r.lines.is_empty())
                .map_or((usize::MAX, usize::MAX), |r| (r.lines.start, 0)),
        };
        self.go(self.clamp_pos(src));
        let at = (self.line, self.col);
        if let Some(p) = &mut self.preview {
            (p.row, p.top, p.at) = (to, top, Some(at));
        }
        if reads && let (Some(pos), Some(cur)) = (self.pos(), self.history.get_mut(self.hist_idx)) {
            *cur = pos;
        }
        self.preview_clamp();
        true
    }

    /// The cursor row of the preview shows no line: Ctrl+C has nothing to copy.
    pub(super) fn preview_blank(&self) -> bool {
        let pos = Some((self.line, self.col));
        let p = self.preview.as_ref().filter(|p| p.at == pos);
        self.previewing() && p.is_some_and(|p| p.doc.rows[p.row].lines.is_empty())
    }

    /// A jump moved the cursor to a line: the preview shows the row of that line, found anew,
    /// never the row it stood on because the two share a position.
    pub(super) fn preview_jumped(&mut self) {
        if let Some(p) = &mut self.preview {
            p.at = None;
        }
    }
}
