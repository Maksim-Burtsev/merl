use super::*;

impl App {
    pub(crate) fn sync_want_x(&mut self) {
        self.want_x = self.cursor_x();
    }

    pub(super) fn apply_want_x(&mut self, row: usize) {
        let rows = self.text_rows(self.line_str());
        let row = row.min(rows.len() - 1);
        let r = rows[row].clone();
        let s = shown_str(self.line_str());
        let mut col = if row + 1 < rows.len() {
            prev_char(s, r.end)
        } else {
            s.len()
        };
        let mut used = match row {
            0 => 0,
            _ => wrap::indent(s, self.view_w),
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

    fn land(&mut self, (key, row): (usize, usize)) {
        let (t, row) = self.line_at_row(key, row);
        self.set_at(t);
        self.apply_want_x(row);
    }

    pub(super) fn clamp_pos(&self, (line, col): (usize, usize)) -> (usize, usize) {
        let line = line.min(self.buf.lines.len() - 1);
        let s = self.buf.shown(line);
        let mut col = col.min(s.len());
        while !s.is_char_boundary(col) {
            col -= 1;
        }
        (line, col)
    }

    pub(super) fn clamp_place(&self, (t, col): (TextLine, usize)) -> (TextLine, usize) {
        let t = self.clamp_line(t);
        let s = shown_str(self.text(t));
        let mut col = col.min(s.len());
        while !s.is_char_boundary(col) {
            col -= 1;
        }
        (t, col)
    }

    pub(super) fn clamp_cursor(&mut self) {
        let (t, col) = self.clamp_place((self.at(), self.col));
        self.set_at(t);
        self.col = col;
    }

    pub fn selection(&self) -> Option<((TextLine, usize), (TextLine, usize))> {
        let a = self.clamp_place(self.anchor?);
        let b = (self.at(), self.col);
        (a != b).then(|| (a.min(b), a.max(b)))
    }

    pub fn file_selection(&self) -> Option<((usize, usize), (usize, usize))> {
        match self.selection()? {
            ((TextLine::File(a), ca), (TextLine::File(b), cb)) if !self.on_deleted() => {
                Some(((a, ca), (b, cb)))
            }
            _ => None,
        }
    }

    pub(super) fn on_deleted(&self) -> bool {
        match self.selection() {
            _ if self.deleted.is_some() => true,
            Some(((TextLine::File(a), _), (TextLine::File(b), _))) => {
                a < b && self.diff.ghosts.range(a + 1..=b).next().is_some()
            }
            Some(_) => true,
            None => false,
        }
    }

    pub fn selected_bytes(&self, t: TextLine) -> Option<std::ops::Range<usize>> {
        let (start, end) = self.selection()?;
        if t < start.0 || t > end.0 {
            return None;
        }
        let from = if t == start.0 { start.1 } else { 0 };
        let to = if t == end.0 {
            end.1
        } else {
            self.text(t).len()
        };
        Some(from..to)
    }

    pub(super) fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection()?;
        let lines: Vec<&str> = std::iter::successors(Some(start.0), |&t| self.next_line(t))
            .take_while(|&t| t <= end.0)
            .map(|t| {
                let line = self.text(t);
                let cut = shown_str(line).len();
                let end_of = |col: usize| if col == cut { line.len() } else { col };
                let r = self.selected_bytes(t).unwrap();
                &line[end_of(r.start)..end_of(r.end)]
            })
            .collect();
        Some(lines.join("\n"))
    }

    pub(super) fn extend(&mut self, mv: fn(&mut Self)) {
        self.anchor.get_or_insert((self.at(), self.col));
        mv(self);
    }

    pub(super) fn grow_selection(&mut self) {
        let blank = |t: TextLine| self.text(t).trim().is_empty();
        let l = self.at();
        let (mut top, mut bottom) = (l, l);
        while let Some(t) = self.prev_line(top).filter(|&t| !blank(t)) {
            top = t;
        }
        while let Some(t) = self.next_line(bottom).filter(|&t| !blank(t)) {
            bottom = t;
        }
        let s = shown_str(self.text(l));
        let (mut from, mut to) = (self.col, self.col);
        while from > 0 && is_word(char_at(s, prev_char(s, from))) {
            from = prev_char(s, from);
        }
        while to < s.len() && is_word(char_at(s, to)) {
            to = next_char(s, to);
        }
        let word = (from < to).then_some(from..to);
        let last = self.last_line();
        let steps = [
            word.map(|r| ((l, r.start), (l, r.end))),
            Some(((l, 0), (l, s.len()))),
            Some(((top, 0), (bottom, shown_str(self.text(bottom)).len()))),
            Some((
                (self.first_line(), 0),
                (last, shown_str(self.text(last)).len()),
            )),
        ];
        let cur = self.selection().unwrap_or(((l, self.col), (l, self.col)));
        type Span = ((TextLine, usize), (TextLine, usize));
        let wider =
            |&(from, to): &Span| !blank(l) && from <= cur.0 && cur.1 <= to && (from, to) != cur;
        if let Some((from, to)) = steps.into_iter().flatten().find(wider) {
            self.anchor = Some(from);
            self.set_at(to.0);
            self.col = to.1;
            self.sync_want_x();
        }
    }

    pub(super) fn drop_selection_if_moved(&mut self, before: (TextLine, usize)) {
        if (self.at(), self.col) != before {
            self.anchor = None;
        }
    }

    pub(super) fn line_start(&mut self) {
        let rows = self.text_rows(self.line_str());
        let start = rows[wrap::col_to_row(&rows, self.col)].start;
        self.col = if self.col == start { 0 } else { start };
        self.sync_want_x();
    }

    pub(super) fn line_end(&mut self) {
        let rows = self.text_rows(self.line_str());
        let row = wrap::col_to_row(&rows, self.col);
        let len = self.shown_len();
        let end = if row + 1 < rows.len() {
            prev_char(self.line_str(), rows[row].end)
        } else {
            len
        };
        self.col = if self.col == end { len } else { end };
        self.sync_want_x();
    }

    pub(super) fn move_rows(&mut self, n: isize) {
        let cur = self.cursor_at();
        let to = if n < 0 {
            self.back_rows(cur, n.unsigned_abs())
        } else {
            self.forward_rows(cur, n as usize)
        };
        if n > 0 && to == cur {
            return;
        }
        self.land(to);
    }

    pub(super) fn half_page(&mut self, dir: isize) {
        let half = (self.page_rows(dir > 0) / 2).max(1);
        let before = self.cursor_at();
        self.move_rows(dir * half as isize);
        let moved = self.rows_between(before, self.cursor_at());
        let top = (self.top_line, self.top_row);
        (self.top_line, self.top_row) = if dir < 0 {
            self.back_rows(top, moved)
        } else {
            self.forward_rows(top, moved)
        };
    }

    pub(super) fn paragraph(&mut self, dir: isize) {
        let blank = |t: TextLine| self.text(t).trim().is_empty();
        let step = |t: TextLine| match dir < 0 {
            true => self.prev_shown(t),
            false => self.next_shown(t),
        };
        let mut t = self.at();
        while let Some(s) = step(t).filter(|_| blank(t)) {
            t = s;
        }
        while let Some(s) = step(t).filter(|_| !blank(t)) {
            t = s;
        }
        self.set_at(t);
        self.apply_want_x(0);
    }

    pub fn rows_between(&self, a: (usize, usize), b: (usize, usize)) -> usize {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let ((mut line, mut row), mut n) = (lo, 0);
        while line < hi.0 {
            n += self.row_count(line) - row;
            (line, row) = (line + 1, 0);
        }
        n + hi.1 - row
    }

    pub(super) fn forward_rows(
        &self,
        (mut line, mut row): (usize, usize),
        n: usize,
    ) -> (usize, usize) {
        let last = self.last_line().key();
        let mut count = self.row_count(line);
        for _ in 0..n {
            if row + 1 < count {
                row += 1;
            } else if let Some(next) = (line + 1..=last).find(|&l| !self.hidden(l)) {
                (line, row) = (next, 0);
                count = self.row_count(line);
            } else {
                break;
            }
        }
        (line, row)
    }

    pub(super) fn left(&mut self) {
        if self.col > 0 {
            self.col = prev_char(self.line_str(), self.col);
        } else if let Some(t) = self.prev_shown(self.at()) {
            self.set_at(t);
            self.col = self.shown_len();
        }
        self.sync_want_x();
    }

    pub(super) fn right(&mut self) {
        if self.col < self.shown_len() {
            self.col = next_char(self.line_str(), self.col);
        } else if let Some(t) = self.next_shown(self.at()) {
            self.set_at(t);
            self.col = 0;
        }
        self.sync_want_x();
    }

    pub(super) fn word_right(&mut self) {
        if self.col < self.shown_len() {
            self.col = word_end(shown_str(self.line_str()), self.col);
        } else if let Some(t) = self.next_shown(self.at()) {
            self.set_at(t);
            self.col = 0;
        }
        self.sync_want_x();
    }

    pub(super) fn word_left(&mut self) {
        if self.col > 0 {
            self.col = word_start(self.line_str(), self.col);
        } else if let Some(t) = self.prev_shown(self.at()) {
            self.set_at(t);
            self.col = self.shown_len();
        }
        self.sync_want_x();
    }

    pub(super) fn shown_len(&self) -> usize {
        shown_str(self.line_str()).len()
    }

    pub fn goto_line(&mut self, line1: usize) {
        self.go((line1.max(1).min(self.buf.lines.len()) - 1, 0));
        self.want_x = 0;
        self.center = true;
    }

    pub(super) fn written(&self, kind: Option<Kind>, start: usize) -> search::LineAsRead {
        let joined = kind
            .and_then(|k| search::unbroken(k, &self.buf.lines, self.line, start))
            .unwrap_or_else(|| search::LineAsRead {
                line: self.line_str().to_owned(),
                word_start: start,
            });
        match kind {
            Some(k) => search::plain_access(k, &joined.line, joined.word_start),
            None => joined,
        }
    }
}

pub(super) fn word_end(s: &str, mut i: usize) -> usize {
    while i < s.len() && !is_word(char_at(s, i)) {
        i = next_char(s, i);
    }
    while i < s.len() && is_word(char_at(s, i)) {
        i = next_char(s, i);
    }
    i
}

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
