//! `/`: find in the open file.

use super::*;

/// The most chars a `/` query holds (#267): nobody searches a file for a longer literal. Ignoring
/// case, 1,000 Cyrillic letters compile to about 200 KiB, Greek iotas (four case forms, the worst
/// found) to 330 KiB: far below the `regex` crate's 10 MiB limit, which some 60,000 Cyrillic
/// letters went past.
const FIND_CAP: usize = 1_000;

impl App {
    /// An overlay closed and the open file stays: back to the mode it was opened from.
    pub(super) fn close_overlay(&mut self) {
        self.mode = Mode::Normal;
        if std::mem::take(&mut self.resume_edit) {
            // The cursor may have moved: what is typed next is a new undo step.
            self.undo_break = true;
            self.edit_mode();
        }
    }

    pub(super) fn start_find(&mut self) {
        self.mode = Mode::Find;
        let seed = self.one_line_selection();
        let last = self.find_re.as_ref().map_or("", |_| &self.find_query);
        self.prompt = LineEdit::selected(seed.as_deref().unwrap_or(last)).capped(FIND_CAP);
        self.find_anchor = (self.at(), self.col);
        self.find_sel = self.anchor.take();
        if seed.is_some() {
            self.refresh_find();
        } else if let Some(re) = &self.find_re {
            self.message = self.match_count(re);
        }
    }

    pub(super) fn one_line_selection(&self) -> Option<String> {
        let (start, end) = self.selection()?;
        let text = self.selected_text().filter(|_| start.0 == end.0)?;
        Some(text.chars().take(FIND_CAP).collect())
    }

    pub(super) fn find_key(&mut self, key: KeyEvent) {
        match key.code {
            // Enter keeps both the position and the pattern, so `n` carries on from here. The
            // selection comes back unless the search moved the cursor.
            KeyCode::Enter => {
                self.close_overlay();
                self.anchor = self.find_sel.take();
                self.drop_selection_if_moved(self.clamp_place(self.find_anchor));
                self.hist_note(true);
            }
            // Esc puts back both the cursor and the selection.
            KeyCode::Esc => {
                self.close_overlay();
                self.message.clear();
                let (t, col) = self.clamp_place(self.find_anchor);
                self.set_at(t);
                self.col = col;
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
    pub(super) fn refresh_find(&mut self) {
        if self.prompt.is_empty() {
            // Nothing to match: drop the previous pattern so its highlights go with it,
            // and put the cursor back where the search started.
            self.find_re = None;
            self.message.clear();
            let at = self.clamp_place(self.find_anchor);
            self.go_to_match(at);
            return;
        }
        let re = RegexBuilder::new(&regex::escape(&self.prompt))
            .case_insensitive(true)
            .build()
            .expect("a literal of at most FIND_CAP chars compiles");
        let hit = self
            .match_at_or_after(&re, self.clamp_place(self.find_anchor))
            .or_else(|| self.match_at_or_after(&re, (self.first_line(), 0)));
        match hit {
            Some(at) => {
                self.go_to_match(at);
                self.message = self.match_count(&re);
            }
            None => self.message = "no match".into(),
        }
        self.find_re = Some(re);
        self.find_query = self.prompt.to_string();
    }

    /// Every line of the text in order, from `from` on: the file's and, in a review, the ones
    /// the branch deleted, which `/` searches as it searches the file's (#439).
    fn lines_from(&self, from: TextLine) -> impl Iterator<Item = (TextLine, &str)> {
        std::iter::successors(Some(from), |&t| self.next_line(t)).map(|t| (t, self.text(t)))
    }

    /// `3/17`: which match the cursor is on, out of how many in the file; `no match` for none.
    fn match_count(&self, re: &Regex) -> String {
        let (mut at, mut total) = (0, 0);
        let here = (self.at(), self.col);
        for (t, text) in self.lines_from(self.first_line()) {
            for m in re.find_iter(text) {
                total += 1;
                if (t, m.start()) <= here {
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
        let found = if forward {
            self.match_at_or_after(&re, self.after_cursor())
                .or_else(|| self.match_at_or_after(&re, (self.first_line(), 0)))
        } else {
            self.match_before(&re, (self.at(), self.col))
                .or_else(|| self.match_before(&re, (self.last_line(), usize::MAX)))
        };
        match found {
            Some(at) => {
                self.go_to_match(at);
                self.message = self.match_count(&re);
            }
            None => self.message = "no match".into(),
        }
    }

    /// First match starting at or after `(line, col)`, searching down the file.
    fn match_at_or_after(
        &self,
        re: &Regex,
        (line, col): (TextLine, usize),
    ) -> Option<(TextLine, usize)> {
        for (t, text) in self.lines_from(line) {
            let from = if t == line { col.min(text.len()) } else { 0 };
            if let Some(m) = re.find_at(text, from) {
                return Some((t, m.start()));
            }
        }
        None
    }

    /// Last match starting strictly before `(line, col)`, searching up the file.
    fn match_before(
        &self,
        re: &Regex,
        (line, col): (TextLine, usize),
    ) -> Option<(TextLine, usize)> {
        for t in std::iter::successors(Some(line), |&t| self.prev_line(t)) {
            let limit = if t == line { col } else { usize::MAX };
            let text = self.text(t);
            if let Some(m) = re.find_iter(text).take_while(|m| m.start() < limit).last() {
                return Some((t, m.start()));
            }
        }
        None
    }

    /// One char past the cursor, so `n` cannot land on the match it is already sitting on.
    fn after_cursor(&self) -> (TextLine, usize) {
        let s = shown_str(self.line_str());
        match self.next_line(self.at()) {
            _ if self.col < s.len() => (self.at(), next_char(s, self.col)),
            Some(t) => (t, 0),
            None => (self.at(), s.len()),
        }
    }

    fn go_to_match(&mut self, at: (TextLine, usize)) {
        let (t, col) = self.clamp_place(at);
        self.set_at(t);
        self.col = col;
        self.sync_want_x();
        self.center = true;
    }
}
