//! Keeping the cursor on the screen: the vertical and horizontal scroll.

use super::*;

/// Most declarations pinned over the code: the innermost ones when more enclose the view. Two
/// covers a method in its `impl` or `class`, two rows of a pane of thirty.
const MAX_PINNED: usize = 2;

impl App {
    /// Scrolls the minimum amount that puts the cursor back on screen.
    pub fn clamp_scroll(&mut self) {
        // A wider or narrower pane wraps the top line into other rows (#412).
        self.clamp_top();
        self.clamp_left();
        let cur = (self.line, self.cursor_row());
        if std::mem::take(&mut self.center) {
            self.center_cursor();
            return;
        }
        // The top is on the cursor line's own ghosts: they are being read, the cursor waits
        // below the pane (`move_rows` scrolls them in one at a time).
        if self.top_line == self.line && self.top_row < self.ghost_rows(self.line) {
            return;
        }
        // Scrolled on past the end of the text (`move_rows`), the cursor waits above the pane.
        if cur < (self.top_line, self.top_row) && self.at_text_end() {
            (self.top_line, self.top_row) = (self.top_line, self.top_row).min(self.bottom_top());
            return;
        }
        if cur < (self.top_line, self.top_row) {
            // Moving up shows the line's ghosts with it.
            (self.top_line, self.top_row) = (self.line, cur.1 - self.ghost_rows(self.line));
            return;
        }
        (self.top_line, self.top_row) = self.fit_top((self.top_line, self.top_row), cur);
    }

    /// The declarations the code pane pins over the text with `top` as the first line of the
    /// view (#248): those enclosing it whose own line has scrolled off above, outermost first,
    /// the innermost [`MAX_PINNED`] of them. A short pane keeps its rows for the code.
    pub fn pinned(&self, top: usize) -> Vec<usize> {
        let max = self.max_pinned();
        if max == 0 || top >= self.buf.lines.len() {
            return Vec::new();
        }
        let mut pins = search::enclosing_declarations(self.kind(), &self.buf.lines, top);
        pins.drain(..pins.len().saturating_sub(max));
        pins
    }

    /// Most lines the pane pins: none under 8 rows, one under 16.
    fn max_pinned(&self) -> usize {
        MAX_PINNED.min(self.view_h / 8)
    }

    /// Rows a page down (or up) moves: the rows of code the pane shows under the lines pinned
    /// now, fewer when the top a page away pins more, so no line is skipped between two pages.
    pub(super) fn page_rows(&self, down: bool) -> usize {
        let rows = |top: (usize, usize)| self.view_h.saturating_sub(self.pinned(top.0).len());
        let top = (self.top_line, self.top_row);
        let here = rows(top);
        let there = match down {
            // On the lines deleted after the last one nothing is pinned, and no row is ahead.
            true if top.0 >= self.buf.lines.len() => return here.max(1),
            true => self.forward_rows(top, here),
            false => self.back_rows(top, here),
        };
        here.min(rows(there)).max(1)
    }

    /// The first top at or below `from` that shows `bottom` on the pane, under the lines pinned
    /// for that top: nothing ever stands behind them. The pins do not grow with the top (past a
    /// method's `}` two become one), so the tops are tried one row at a time from the first that
    /// shows `bottom` with no pins; every pin takes a row, so one of the next [`MAX_PINNED`] fits.
    fn fit_top(&self, from: (usize, usize), bottom: (usize, usize)) -> (usize, usize) {
        let mut top = from.max(self.back_rows(bottom, self.view_h.saturating_sub(1)));
        for _ in 0..MAX_PINNED {
            let rows = self.view_h.saturating_sub(1 + self.pinned(top.0).len());
            if self.back_rows(bottom, rows) <= top {
                break;
            }
            top = self.forward_rows(top, 1);
        }
        top
    }

    /// Not wrapped, the view follows the cursor sideways, keeping [`SIDE_OFF`] columns between it
    /// and the edge but never scrolling past the end of its line.
    fn clamp_left(&mut self) {
        if !self.nowrap() {
            self.left = 0;
            return;
        }
        let off = SIDE_OFF.min(self.view_w.saturating_sub(1) / 2);
        let x = self.cursor_x();
        if x < self.left + off {
            self.left = x.saturating_sub(off);
        } else if x + off >= self.left + self.view_w {
            let end = (wrap::width(self.buf.shown(self.line)) + 1).saturating_sub(self.view_w);
            self.left = (x + off + 1 - self.view_w).min(end);
        }
    }

    /// The cursor is on the last screen row of the text: Down has no row left to go to.
    pub(super) fn at_text_end(&self) -> bool {
        self.line + 1 == self.buf.lines.len() && self.cursor_row() + 1 == self.row_count(self.line)
    }

    /// The top of the view scrolled down as far as it goes: the last line the branch deleted at
    /// the end of the file, or the last row of text when there is none, on the bottom row.
    pub(super) fn bottom_top(&self) -> (usize, usize) {
        let end = self.buf.lines.len();
        let last = match self.ghost_rows(end) {
            0 => (end - 1, self.row_count(end - 1) - 1),
            g => (end, g - 1),
        };
        self.fit_top((0, 0), last)
    }

    /// The top made valid for the text and ghosts as they are now. It may stay on the lines
    /// deleted at the end of the file, keyed `lines.len()`, which have ghost rows only.
    pub(super) fn clamp_top(&mut self) {
        let end = self.buf.lines.len();
        if self.top_line != end || self.top_row >= self.ghost_rows(end) {
            self.top_line = self.top_line.min(end - 1);
            self.top_row = self.top_row.min(self.row_count(self.top_line) - 1);
        }
    }

    /// Puts the cursor in the middle of the rows of code, under the lines pinned for the top
    /// that centres it.
    fn center_cursor(&mut self) {
        let cur = (self.line, self.cursor_row());
        let pins = self.pinned(self.back_rows(cur, self.view_h / 2).0).len();
        (self.top_line, self.top_row) = self.back_rows(cur, (self.view_h - pins) / 2);
    }

    /// Walks `n` wrapped rows backwards from `(line, row)`, stopping at the top of the file.
    pub(super) fn back_rows(
        &self,
        (mut line, mut row): (usize, usize),
        n: usize,
    ) -> (usize, usize) {
        for _ in 0..n {
            if row > 0 {
                row -= 1;
            } else if line > 0 {
                line -= 1;
                row = self.row_count(line) - 1;
            } else {
                break;
            }
        }
        (line, row)
    }
}
