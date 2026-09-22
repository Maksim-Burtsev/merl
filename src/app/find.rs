//! `/`: find in the open file.

use super::*;

impl App {
    /// An overlay closed and the open file stays: back to the mode it was opened from.
    pub(super) fn close_overlay(&mut self) {
        self.mode = if std::mem::take(&mut self.resume_edit) {
            // The cursor may have moved: what is typed next is a new undo step.
            self.undo_break = true;
            Mode::Edit
        } else {
            Mode::Normal
        };
    }

    /// Starts an incremental search from the current cursor position. The selection is set aside
    /// meanwhile, so it does not stretch to every match the cursor visits. A pattern still active
    /// comes back selected, as Cmd+F in a browser: typing replaces it, an arrow edits it.
    pub(super) fn start_find(&mut self) {
        self.mode = Mode::Find;
        let last = self.find_re.as_ref().map_or("", |_| &self.find_query);
        self.prompt = LineEdit::selected(last);
        self.find_anchor = (self.line, self.col);
        self.find_sel = self.anchor.take();
        if let Some(re) = &self.find_re {
            self.message = self.match_count(re);
        }
    }

    pub(super) fn find_key(&mut self, key: KeyEvent) {
        match key.code {
            // Enter keeps both the position and the pattern, so `n` carries on from here. The
            // selection comes back unless the search moved the cursor.
            KeyCode::Enter => {
                self.close_overlay();
                self.anchor = self.find_sel.take();
                self.drop_selection_if_moved(self.clamp_pos(self.find_anchor));
                self.hist_note(true);
            }
            // Esc puts back both the cursor and the selection.
            KeyCode::Esc => {
                self.close_overlay();
                self.message.clear();
                (self.line, self.col) = self.clamp_pos(self.find_anchor);
                self.anchor = self.find_sel.take();
                self.sync_want_x();
            }
            _ => {
                if self.prompt.key(key) {
                    self.refresh_find();
                }
            }
        }
    }

    /// Recompiles the query and moves to the first match at or after the anchor.
    /// The query is literal text and ignores case: `migrator(` hits `Migrator()`, `sameCancel`
    /// hits `SameCancel`.
    fn refresh_find(&mut self) {
        if self.prompt.is_empty() {
            // Nothing to match: drop the previous pattern so its highlights go with it,
            // and put the cursor back where the search started.
            self.find_re = None;
            self.message.clear();
            let (l, c) = self.clamp_pos(self.find_anchor);
            self.go_to_match(l, c);
            return;
        }
        let re = RegexBuilder::new(&regex::escape(&self.prompt))
            .case_insensitive(true)
            .build()
            .expect("an escaped literal always compiles");
        let (l, c) = self.find_anchor;
        let hit = self
            .match_at_or_after(&re, l, c)
            .or_else(|| self.match_at_or_after(&re, 0, 0));
        match hit {
            Some((l, c)) => {
                self.go_to_match(l, c);
                self.message = self.match_count(&re);
            }
            None => self.message = "no match".into(),
        }
        self.find_re = Some(re);
        self.find_query = self.prompt.to_string();
    }

    /// `3/17`: which match the cursor is on, out of how many in the file; `no match` for none.
    fn match_count(&self, re: &Regex) -> String {
        let (mut at, mut total) = (0, 0);
        for (l, text) in self.buf.lines.iter().enumerate() {
            for m in re.find_iter(text) {
                total += 1;
                if (l, m.start()) <= (self.line, self.col) {
                    at = total;
                }
            }
        }
        if total == 0 {
            return "no match".into();
        }
        format!("{at}/{total}")
    }

    /// `n` / `N`: the next or previous match, wrapping around the file.
    pub(super) fn step_find(&mut self, forward: bool) {
        let Some(re) = self.find_re.clone() else {
            self.message = "no pattern".into();
            return;
        };
        let last = self.buf.lines.len() - 1;
        let found = if forward {
            let (l, c) = self.after_cursor();
            self.match_at_or_after(&re, l, c)
                .or_else(|| self.match_at_or_after(&re, 0, 0))
        } else {
            self.match_before(&re, self.line, self.col)
                .or_else(|| self.match_before(&re, last, usize::MAX))
        };
        match found {
            Some((l, c)) => {
                self.go_to_match(l, c);
                self.message = self.match_count(&re);
            }
            None => self.message = "no match".into(),
        }
    }

    /// First match starting at or after `(line, col)`, searching down the file.
    fn match_at_or_after(&self, re: &Regex, line: usize, col: usize) -> Option<(usize, usize)> {
        for (l, text) in self.buf.lines.iter().enumerate().skip(line) {
            let from = if l == line { col.min(text.len()) } else { 0 };
            if let Some(m) = re.find_at(text, from) {
                return Some((l, m.start()));
            }
        }
        None
    }

    /// Last match starting strictly before `(line, col)`, searching up the file.
    fn match_before(&self, re: &Regex, line: usize, col: usize) -> Option<(usize, usize)> {
        for l in (0..=line.min(self.buf.lines.len() - 1)).rev() {
            let text = &self.buf.lines[l];
            let limit = if l == line { col } else { usize::MAX };
            if let Some(m) = re.find_iter(text).take_while(|m| m.start() < limit).last() {
                return Some((l, m.start()));
            }
        }
        None
    }

    /// One char past the cursor, so `n` cannot land on the match it is already sitting on.
    fn after_cursor(&self) -> (usize, usize) {
        let s = self.line_str();
        if self.col < s.len() {
            (self.line, next_char(s, self.col))
        } else if self.line + 1 < self.buf.lines.len() {
            (self.line + 1, 0)
        } else {
            (self.line, s.len())
        }
    }

    fn go_to_match(&mut self, line: usize, col: usize) {
        self.line = line;
        self.col = col;
        self.sync_want_x();
        self.center = true;
    }
}
