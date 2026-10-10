use super::*;

const MAX_PINNED: usize = 2;

impl App {
    pub fn drawn_aside(&mut self, draw: impl FnOnce(&mut Self)) {
        let kept = (
            self.top_line,
            self.top_row,
            self.left,
            self.view_w,
            self.view_h,
            self.center,
        );
        draw(self);
        (
            self.top_line,
            self.top_row,
            self.left,
            self.view_w,
            self.view_h,
            self.center,
        ) = kept;
    }

    pub fn clamp_scroll(&mut self) {
        self.clamp_top();
        self.clamp_left();
        let cur = self.cursor_at();
        if std::mem::take(&mut self.center) {
            self.center_cursor();
            return;
        }
        if cur < (self.top_line, self.top_row) {
            (self.top_line, self.top_row) = cur;
            return;
        }
        (self.top_line, self.top_row) = self.fit_top((self.top_line, self.top_row), cur);
    }

    pub fn pinned(&self, top: usize) -> Vec<usize> {
        let max = self.max_pinned();
        if max == 0 || top >= self.buf.lines.len() {
            return Vec::new();
        }
        let mut pins = search::enclosing_declarations(self.kind(), &self.buf.lines, top);
        pins.drain(..pins.len().saturating_sub(max));
        pins
    }

    fn max_pinned(&self) -> usize {
        MAX_PINNED.min(self.view_h / 8)
    }

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
            let end = (wrap::width(shown_str(self.line_str())) + 1).saturating_sub(self.view_w);
            self.left = (x + off + 1 - self.view_w).min(end);
        }
    }

    pub(super) fn clamp_top(&mut self) {
        let end = self.buf.lines.len();
        if self.top_line != end || self.top_row >= self.ghost_rows(end) {
            self.top_line = self.top_line.min(end - 1);
            while self.hidden(self.top_line) {
                (self.top_line, self.top_row) = (self.top_line - 1, 0);
            }
            self.top_row = self.top_row.min(self.row_count(self.top_line) - 1);
        }
    }

    pub fn bottom_line(&self) -> usize {
        let rows = self.view_h.saturating_sub(1);
        self.forward_rows((self.top_line, self.top_row), rows).0
    }

    fn center_cursor(&mut self) {
        let cur = self.cursor_at();
        let pins = self.pinned(self.back_rows(cur, self.view_h / 2).0).len();
        (self.top_line, self.top_row) = self.back_rows(cur, (self.view_h - pins) / 2);
    }

    pub(super) fn back_rows(
        &self,
        (mut line, mut row): (usize, usize),
        n: usize,
    ) -> (usize, usize) {
        for _ in 0..n {
            if row > 0 {
                row -= 1;
            } else if line > 0 {
                line = (0..line).rev().find(|&l| !self.hidden(l)).unwrap_or(0);
                row = self.row_count(line) - 1;
            } else {
                break;
            }
        }
        (line, row)
    }
}
