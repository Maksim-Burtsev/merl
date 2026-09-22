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
