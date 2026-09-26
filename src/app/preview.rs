//! `p`: a Markdown file rendered in place of its source (#249), at the same place both ways.

use std::hash::{Hash, Hasher};

use super::*;
use crate::markdown::{self, Doc, Kind};

/// The preview of the open file: its rows at the pane's width, the row the cursor is on and the
/// row at the top of the pane.
pub struct Preview {
    pub doc: Doc,
    /// What `doc` was laid out from: a hash of the lines, and the width.
    key: (u64, usize),
    pub row: usize,
    pub top: usize,
    /// The source position the preview last put the cursor at. A cursor found anywhere else was
    /// moved by something else (`:`, `[`, a reload), and the row follows it.
    at: (usize, usize),
    /// The code blocks, highlighted as far as they have been on screen with the theme `theme`
    /// names, as the source is highlighted: a block costs what is drawn of it.
    pub code: Vec<Buffer>,
    pub theme: String,
}

impl App {
    /// The open file is shown rendered.
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
            let cur = (self.line, self.cursor_row());
            (self.top_line, self.top_row) = self.back_rows(cur, off);
            self.sync_want_x();
            return;
        }
        if !markdown::is_markdown(&path) {
            self.message = "not Markdown".into();
            return;
        }
        let top = (self.top_line, self.top_row);
        let off = self
            .rows_between(top, (self.line, self.cursor_row()))
            .min(self.view_h.saturating_sub(1));
        self.previewed.insert(path);
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
        let key = (h.finish(), self.view_w);
        let pos = (self.line, self.col);
        match &mut self.preview {
            Some(p) if p.key == key => {
                if p.at == pos {
                    return false;
                }
                p.row = p.doc.row_at(pos);
                p.at = pos;
            }
            slot => {
                // Laid out anew, for a rewrite or another width: the row stays where it was on
                // screen.
                let off = slot.as_ref().map_or(0, |p| p.row.saturating_sub(p.top));
                let mut doc = markdown::layout(&self.buf.lines, self.view_w);
                let code = doc
                    .code
                    .drain(..)
                    .map(|c| Buffer::block(&c.lang, c.lines))
                    .collect();
                let row = doc.row_at(pos);
                *slot = Some(Preview {
                    doc,
                    key,
                    row,
                    top: row.saturating_sub(off),
                    at: pos,
                    code,
                    theme: String::new(),
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
    /// Enter edits the source at it. What acts on a word, a column or a selection has nothing to
    /// act on here and does nothing. Returns `false` for every other key, which does what it does
    /// everywhere, and for the keys the tree takes when it has the focus.
    pub(super) fn preview_key(&mut self, key: KeyEvent) -> bool {
        if !self.previewing() {
            return false;
        }
        let tree = matches!(
            key.code,
            KeyCode::Up
                | KeyCode::Down
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::Enter
        );
        if tree && self.focus == Focus::Tree {
            return false;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let plain = key.modifiers.is_empty();
        self.preview_sync();
        let Some(p) = &self.preview else {
            return false;
        };
        let (row, top, last) = (p.row, p.top, p.doc.rows.len() - 1);
        let (h, half) = (self.view_h.max(1), (self.view_h / 2).max(1));
        let gap = |i: &usize| p.doc.rows[*i].kind == Kind::Gap;
        let (to, scroll) = match key.code {
            KeyCode::Up if plain => (row.saturating_sub(1), 0),
            KeyCode::Down if plain => ((row + 1).min(last), 0),
            KeyCode::PageUp => (row.saturating_sub(h), 0),
            KeyCode::PageDown => ((row + h).min(last), 0),
            // Half a screen, the view with the cursor, as in the source.
            KeyCode::Char('u') if ctrl => (row.saturating_sub(half), -1),
            KeyCode::Char('d') if ctrl => ((row + half).min(last), 1),
            KeyCode::Home if ctrl => (0, 0),
            KeyCode::End if ctrl => (last, 0),
            KeyCode::Char('{') => ((0..row).rev().find(gap).unwrap_or(0), 0),
            KeyCode::Char('}') => ((row + 1..=last).find(gap).unwrap_or(last), 0),
            KeyCode::Enter => {
                self.start_edit();
                return true;
            }
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::F(12) => return true,
            KeyCode::Char('/' | 'n' | 'N' | 'v' | 'd' | 'u' | 'w') if !ctrl => return true,
            KeyCode::Char('f') if ctrl => return true,
            _ => return false,
        };
        let moved = to.abs_diff(row);
        let top = match scroll {
            -1 => top.saturating_sub(moved),
            1 => top + moved,
            _ => top,
        };
        let src = p.doc.rows[to].src;
        (self.line, self.col) = self.clamp_pos(src);
        let at = (self.line, self.col);
        if let Some(p) = &mut self.preview {
            (p.row, p.top, p.at) = (to, top, at);
        }
        // Reading moves the current stop of the jump history, as paging does, and adds none:
        // a row's source line can be far from the one above it, a footnote's is.
        if let (Some(pos), Some(cur)) = (self.pos(), self.history.get_mut(self.hist_idx)) {
            *cur = pos;
        }
        self.preview_clamp();
        true
    }
}
