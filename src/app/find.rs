use super::*;

const FIND_CAP: usize = 1_000;

impl App {
    pub(super) fn close_overlay(&mut self) {
        self.mode = Mode::Normal;
        if std::mem::take(&mut self.resume_edit) {
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
            KeyCode::Enter => {
                self.close_overlay();
                self.anchor = self.find_sel.take();
                self.drop_selection_if_moved(self.clamp_place(self.find_anchor));
                self.hist_note(true);
            }
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

    pub(super) fn refresh_find(&mut self) {
        if self.prompt.is_empty() {
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

    fn lines_from(&self, from: TextLine) -> impl Iterator<Item = (TextLine, &str)> {
        std::iter::successors(Some(from), |&t| self.next_line(t)).map(|t| (t, self.text(t)))
    }

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
