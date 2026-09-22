//! Keeping the cursor on the screen: the vertical and horizontal scroll.

use super::*;

impl App {
    /// Scrolls the minimum amount that puts the cursor back on screen.
    pub fn clamp_scroll(&mut self) {
        self.clamp_left();
        if std::mem::take(&mut self.center) {
            self.center_cursor();
            return;
        }
        // The top is on the cursor line's own ghosts: they are being read, the cursor waits
        // below the pane (`move_rows` scrolls them in one at a time).
        if self.top_line == self.line && self.top_row < self.diff.ghost_n(self.line) {
            return;
        }
        let cur = (self.line, self.cursor_row());
        // Scrolled on past the end of the text (`move_rows`), the cursor waits above the pane.
        if cur < (self.top_line, self.top_row) && self.at_text_end() {
            (self.top_line, self.top_row) = (self.top_line, self.top_row).min(self.bottom_top());
            return;
        }
        if cur < (self.top_line, self.top_row) {
            // Moving up shows the line's ghosts with it.
            (self.top_line, self.top_row) = (self.line, cur.1 - self.diff.ghost_n(self.line));
            return;
        }
        let top = self.back_rows(cur, self.view_h.saturating_sub(1));
        if (self.top_line, self.top_row) < top {
            (self.top_line, self.top_row) = top;
        }
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
        let last = match self.diff.ghost_n(end) {
            0 => (end - 1, self.row_count(end - 1) - 1),
            g => (end, g - 1),
        };
        self.back_rows(last, self.view_h.saturating_sub(1))
    }

    /// The top made valid for the text and ghosts as they are now. It may stay on the lines
    /// deleted at the end of the file, keyed `lines.len()`, which have ghost rows only.
    pub(super) fn clamp_top(&mut self) {
        let end = self.buf.lines.len();
        if self.top_line != end || self.top_row >= self.diff.ghost_n(end) {
            self.top_line = self.top_line.min(end - 1);
            self.top_row = self.top_row.min(self.row_count(self.top_line) - 1);
        }
    }

    fn center_cursor(&mut self) {
        let cur = (self.line, self.cursor_row());
        (self.top_line, self.top_row) = self.back_rows(cur, self.view_h / 2);
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
