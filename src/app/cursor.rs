//! Moving the cursor and growing the selection.

use super::*;

impl App {
    /// Remembers the screen column the cursor stands on, for Up / Down to aim at.
    pub(crate) fn sync_want_x(&mut self) {
        self.want_x = self.cursor_x();
    }

    /// Places the cursor on screen row `row` of `self.line`, on the cluster under the column
    /// `want_x`, or as far right as the row goes. A row other than the last ends on its last
    /// char: its end is where the next row starts.
    fn apply_want_x(&mut self, row: usize) {
        let rows = self.rows(self.line);
        let row = row
            .saturating_sub(self.ghost_rows(self.line))
            .min(rows.len() - 1);
        let r = rows[row].clone();
        let s = &self.buf.lines[self.line];
        let mut col = if row + 1 < rows.len() {
            prev_char(s, r.end)
        } else {
            s.len()
        };
        let mut used = match row {
            0 => 0,
            _ => wrap::indent(self.buf.shown(self.line), self.view_w),
        };
        for (i, g) in wrap::clusters(&s[r.start..r.end]) {
            if used >= self.want_x {
                col = r.start + i;
                break;
            }
            used += wrap::cluster_width(g);
        }
        self.col = col;
    }

    /// A stored `(line, col)` made valid for the buffer as it is now: the line clamped to the
    /// file, the col to the line and back onto a char boundary. Every position that outlives a
    /// reload — history stops, the find anchor, the selection anchor — is read through here.
    pub(super) fn clamp_pos(&self, (line, col): (usize, usize)) -> (usize, usize) {
        let line = line.min(self.buf.lines.len() - 1);
        let s = &self.buf.lines[line];
        let mut col = col.min(s.len());
        while !s.is_char_boundary(col) {
            col -= 1;
        }
        (line, col)
    }

    /// The selection as ordered (line, col) ends: anchor and cursor, whichever comes first.
    /// None while the cursor stands on the anchor: an empty range selects nothing, so Ctrl+C
    /// copies the line and Backspace deletes a char, as with no selection. The anchor stays for
    /// the next Shift+move.
    pub fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let a = self.clamp_pos(self.anchor?);
        let b = (self.line, self.col);
        (a != b).then(|| (a.min(b), a.max(b)))
    }

    /// Bytes of line `l` inside the selection.
    pub fn selected_bytes(&self, l: usize) -> Option<std::ops::Range<usize>> {
        let (start, end) = self.selection()?;
        if l < start.0 || l > end.0 {
            return None;
        }
        let from = if l == start.0 { start.1 } else { 0 };
        let to = if l == end.0 {
            end.1
        } else {
            self.buf.lines[l].len()
        };
        Some(from..to)
    }

    /// The selected text, lines joined with `\n`.
    pub(super) fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection()?;
        let lines: Vec<&str> = (start.0..=end.0)
            .map(|l| &self.buf.lines[l][self.selected_bytes(l).unwrap()])
            .collect();
        Some(lines.join("\n"))
    }

    /// Shift+move, VS Code style: the anchor is set where the first extending move started and
    /// the selection runs from there to wherever `mv` takes the cursor.
    pub(super) fn extend(&mut self, mv: fn(&mut Self)) {
        self.anchor.get_or_insert((self.line, self.col));
        mv(self);
    }

    /// `v`: the selection grows to the word under the cursor (what `d` and `u` read), then the
    /// line, then the paragraph between blank lines. No state: the next step is the first of the
    /// three that is wider than what is selected, so a selection made by hand grows too.
    pub(super) fn grow_selection(&mut self) {
        let blank = |l: usize| self.buf.lines[l].trim().is_empty();
        let l = self.line;
        let (mut top, mut bottom) = (l, l);
        while top > 0 && !blank(top - 1) {
            top -= 1;
        }
        while bottom + 1 < self.buf.lines.len() && !blank(bottom + 1) {
            bottom += 1;
        }
        // The run of word chars under the cursor or, at its end, just before it.
        let s = self.line_str();
        let (mut from, mut to) = (self.col, self.col);
        while from > 0 && is_word(char_at(s, prev_char(s, from))) {
            from = prev_char(s, from);
        }
        while to < s.len() && is_word(char_at(s, to)) {
            to = next_char(s, to);
        }
        let word = (from < to).then_some(from..to);
        let steps = [
            word.map(|r| ((l, r.start), (l, r.end))),
            Some(((l, 0), (l, self.line_str().len()))),
            Some(((top, 0), (bottom, self.buf.lines[bottom].len()))),
        ];
        let cur = self.selection().unwrap_or(((l, self.col), (l, self.col)));
        let wider = |&(from, to): &((usize, usize), (usize, usize))| {
            !blank(l) && from <= cur.0 && cur.1 <= to && (from, to) != cur
        };
        if let Some((from, to)) = steps.into_iter().flatten().find(wider) {
            self.anchor = Some(from);
            (self.line, self.col) = to;
            self.sync_want_x();
        }
    }

    /// Any move that does not extend the selection drops it, unless it ends where it started:
    /// arrows, jumps and find all decide it here, from where the cursor finally is.
    pub(super) fn drop_selection_if_moved(&mut self, before: (usize, usize)) {
        if (self.line, self.col) != before {
            self.anchor = None;
        }
    }

    /// Home: the start of the screen row and, pressed there, of the line, as in VS Code.
    pub(super) fn line_start(&mut self) {
        let rows = self.rows(self.line);
        let start = rows[wrap::col_to_row(&rows, self.col)].start;
        self.col = if self.col == start { 0 } else { start };
        self.sync_want_x();
    }

    /// End: the end of the screen row and, pressed there, of the line, as in VS Code. A row
    /// other than the last ends on its last char, since its end is where the next row starts.
    pub(super) fn line_end(&mut self) {
        let rows = self.rows(self.line);
        let row = wrap::col_to_row(&rows, self.col);
        let len = self.line_str().len();
        let end = if row + 1 < rows.len() {
            prev_char(self.line_str(), rows[row].end)
        } else {
            len
        };
        self.col = if self.col == end { len } else { end };
        self.sync_want_x();
    }

    /// Up / Down, PgUp / PgDn: `n` screen rows, aiming at the column in `want_x`.
    pub(super) fn move_rows(&mut self, n: isize) {
        // Up on a line whose ghosts are scrolled off above it brings them in, one row at a
        // time, so a deletion taller than the pane can still be read through.
        if n < 0
            && self.top_line == self.line
            && (1..=self.ghost_rows(self.line)).contains(&self.top_row)
        {
            self.top_row -= 1;
            return;
        }
        // Down on the last row of the text scrolls on through the lines deleted after it, the
        // cursor staying where it is, until the last of them is on the bottom row.
        if n > 0 && self.at_text_end() {
            let (end, bottom) = (self.buf.lines.len(), self.bottom_top());
            for _ in 0..n {
                let (l, r) = (self.top_line, self.top_row);
                if (l, r) >= bottom {
                    break;
                }
                (self.top_line, self.top_row) = if l < end && r + 1 == self.row_count(l) {
                    (l + 1, 0)
                } else {
                    (l, r + 1)
                };
            }
            return;
        }
        let cur = (self.line, self.cursor_row());
        let (mut line, mut row) = if n < 0 {
            self.back_rows(cur, n.unsigned_abs())
        } else {
            self.forward_rows(cur, n as usize)
        };
        // Ghost rows are not for the cursor: going up onto one lands on the text above it.
        if n < 0 && row < self.ghost_rows(line) {
            if line > 0 {
                line -= 1;
                row = self.row_count(line) - 1;
            } else {
                row = self.ghost_rows(line);
            }
        }
        self.line = line;
        self.apply_want_x(row);
    }

    /// Ctrl+D / Ctrl+U: cursor and viewport both move half a screen, like vim and less,
    /// so the cursor keeps its place on screen and half the context stays visible.
    pub(super) fn half_page(&mut self, dir: isize) {
        let half = (self.view_h / 2).max(1);
        let before = (self.line, self.cursor_row());
        self.move_rows(dir * half as isize);
        let moved = self.rows_between(before, (self.line, self.cursor_row()));
        let top = (self.top_line, self.top_row);
        (self.top_line, self.top_row) = if dir < 0 {
            self.back_rows(top, moved)
        } else {
            self.forward_rows(top, moved)
        };
    }

    /// `{` / `}`: the previous / next blank line, like vim. A run of blank lines counts once, so
    /// from a blank line the jump crosses the next paragraph instead of stopping next door.
    /// In code, blank lines separate functions, so this is "next function" without a parser.
    pub(super) fn paragraph(&mut self, dir: isize) {
        let blank = |l: &String| l.trim().is_empty();
        let last = self.buf.lines.len() - 1;
        let step = |l: usize| l.saturating_add_signed(dir).min(last);
        let mut l = self.line;
        while l != step(l) && blank(&self.buf.lines[l]) {
            l = step(l);
        }
        while l != step(l) && !blank(&self.buf.lines[l]) {
            l = step(l);
        }
        self.line = l;
        self.apply_want_x(0);
    }

    /// Wrapped rows from `a` to `b` (either order).
    fn rows_between(&self, a: (usize, usize), b: (usize, usize)) -> usize {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let mut n = 0;
        let mut cur = lo;
        while cur < hi {
            cur = self.forward_rows(cur, 1);
            n += 1;
        }
        n
    }

    /// Walks `n` wrapped rows forwards from `(line, row)`, stopping at the end of the file.
    fn forward_rows(&self, (mut line, mut row): (usize, usize), n: usize) -> (usize, usize) {
        for _ in 0..n {
            if row + 1 < self.row_count(line) {
                row += 1;
            } else if line + 1 < self.buf.lines.len() {
                line += 1;
                row = 0;
            } else {
                break;
            }
        }
        (line, row)
    }

    pub(super) fn left(&mut self) {
        if self.col > 0 {
            self.col = prev_char(self.line_str(), self.col);
        } else if self.line > 0 {
            self.line -= 1;
            self.col = self.line_str().len();
        }
        self.sync_want_x();
    }

    pub(super) fn right(&mut self) {
        if self.col < self.line_str().len() {
            self.col = next_char(self.line_str(), self.col);
        } else if self.line + 1 < self.buf.lines.len() {
            self.line += 1;
            self.col = 0;
        }
        self.sync_want_x();
    }

    pub(super) fn word_right(&mut self) {
        if self.col >= self.line_str().len() {
            if self.line + 1 < self.buf.lines.len() {
                self.line += 1;
                self.col = 0;
            }
        } else {
            self.col = word_end(self.line_str(), self.col);
        }
        self.sync_want_x();
    }

    pub(super) fn word_left(&mut self) {
        if self.col == 0 {
            if self.line > 0 {
                self.line -= 1;
                self.col = self.line_str().len();
            }
        } else {
            self.col = word_start(self.line_str(), self.col);
        }
        self.sync_want_x();
    }

    /// Jumps to a 1-based line number, clamped to the file, and centers the view.
    pub fn goto_line(&mut self, n: usize) {
        self.line = n.max(1).min(self.buf.lines.len()) - 1;
        self.col = 0;
        self.want_x = 0;
        self.center = true;
    }
}

/// Where Alt+Right lands from `i` inside `s`: past the gap, then past the word.
pub(super) fn word_end(s: &str, mut i: usize) -> usize {
    while i < s.len() && !is_word(char_at(s, i)) {
        i = next_char(s, i);
    }
    while i < s.len() && is_word(char_at(s, i)) {
        i = next_char(s, i);
    }
    i
}

/// Where Alt+Left lands from `i` inside `s`, past its start: back over the gap, then over the
/// word.
pub(super) fn word_start(s: &str, i: usize) -> usize {
    if i == 0 {
        return 0;
    }
    let mut i = prev_char(s, i);
    while i > 0 && !is_word(char_at(s, i)) {
        i = prev_char(s, i);
    }
    while i > 0 && is_word(char_at(s, prev_char(s, i))) {
        i = prev_char(s, i);
    }
    i
}

fn char_at(s: &str, i: usize) -> char {
    s[i..].chars().next().unwrap_or(' ')
}
