use super::*;

impl App {
    pub(super) fn rel_current(&self) -> Option<PathBuf> {
        let path = self.buf.path.as_ref()?;
        Some(path.strip_prefix(&self.root).unwrap_or(path).to_path_buf())
    }

    pub(super) fn grep(
        &self,
        pattern: &str,
        whole_word: bool,
        ignore_case: bool,
        wanted: impl Fn(&Path) -> bool,
    ) -> anyhow::Result<Vec<Hit>> {
        self.grep_job(0, pattern, wanted)
            .run(whole_word, ignore_case)
    }

    pub(super) fn grep_job(
        &self,
        seq: u64,
        pattern: &str,
        wanted: impl Fn(&Path) -> bool,
    ) -> SearchJob {
        let mut files: Vec<PathBuf> = self.files.iter().filter(|p| wanted(p)).cloned().collect();
        let current = self.rel_current();
        if let Some(cur) = &current
            && wanted(cur)
        {
            files.retain(|p| p != cur);
            files.insert(0, cur.clone());
        }
        SearchJob {
            seq,
            root: self.root.clone(),
            files,
            pattern: pattern.to_string(),
            symbols: false,
            current,
            unsaved: self.dirty.then(|| self.buf.to_bytes()),
            deleted: self.deleted_lines(),
        }
    }

    pub(super) fn deleted_lines(&self) -> Arc<Vec<git::DeletedLine>> {
        self.review
            .as_ref()
            .map_or_else(Default::default, |r| r.deleted.clone())
    }

    pub(super) fn word_under(&self, word_chars: &str) -> Option<String> {
        search::word_at(self.line_str(), self.col, word_chars).map(|(_, w)| w.to_string())
    }

    pub(super) fn kind(&self) -> Option<Kind> {
        self.buf.path.as_deref().and_then(search::opened_kind)
    }

    pub(crate) fn hit_items(hits: Vec<Hit>) -> Vec<PickItem> {
        hits.into_iter()
            .map(|h| {
                let label = format!("{}: ", at_label(&h.path, h.line1));
                let code_at = Some(label.len());
                let path_at = Some(0..h.path.display().to_string().len());
                PickItem {
                    col: h.byte_col.unwrap_or(0),
                    label: label + &clip(h.text.trim(), MAX_LABEL_TEXT),
                    path: h.path,
                    line: h.line1,
                    code_at,
                    path_at,
                    deleted: h.deleted.is_some(),
                }
            })
            .collect()
    }

    pub(super) fn start_search(&mut self) {
        let seed = self.one_line_selection();
        self.show_picker(PickerKind::Search, Vec::new());
        if let Some(p) = &mut self.picker {
            p.live = true;
            if let Some(seed) = &seed {
                p.query = LineEdit::selected(seed);
            }
        }
        self.drop_pending_search();
        if seed.is_some() {
            self.search_typed();
            self.search_due = Some(Instant::now());
        }
    }

    pub(super) fn drop_pending_search(&mut self) {
        self.search_seq += 1;
        (self.search_due, self.search_sent, self.search_enter) = (None, None, false);
    }

    pub fn search_pending(&self) -> bool {
        self.search_due.is_some() || self.search_sent.is_some()
    }

    pub fn search_tick(&mut self) -> Option<SearchJob> {
        let query = self.picker.as_ref().filter(|p| p.live)?.query.to_string();
        if self.search_due? > Instant::now() {
            return None;
        }
        self.search_due = None;
        self.search_sent = Some(self.search_seq);
        // ponytail: nothing stops a walk whose answer is already stale — `D` past the cap reads
        // the project once per pause in the typing, and [`search::MAX_HITS`] does not bound it,
        // since a narrowing query never fills the cap. A stop flag on the job, read per file, is
        // the upgrade if typing on a large project ever waits on them.
        let symbols = self.mode == Mode::Picker(PickerKind::Symbols);
        let pattern = if symbols {
            query
        } else {
            regex::escape(&query)
        };
        let mut job = self.grep_job(self.search_seq, &pattern, |_| true);
        if symbols {
            job.deleted = self.symbol_deleted();
        }
        Some(SearchJob { symbols, ..job })
    }

    #[cfg(test)]
    pub(crate) fn settle_search(&mut self) {
        self.search_due = self.search_due.map(|_| Instant::now());
        if let Some(job) = self.search_tick() {
            self.search_done(job.seq, job.items());
        }
    }

    pub(super) fn search_jump(&mut self, item: PickItem) {
        self.picker = None;
        self.mode = Mode::Normal;
        self.jump_to_item(&item);
    }

    /// ponytail: no grep is cancelled. For `s` a short query stops at [`search::MAX_HITS`]; a
    /// selective one reads every file, and on gitea (6,000 files) typing at a human pace did not
    /// delay the answer to the final query (#53). `D` past the cap walks the project for every
    /// query (see [`App::search_tick`]). A stop flag checked per file is the upgrade if it does.
    /// Returns whether the screen changed.
    pub fn search_done(&mut self, seq: u64, items: Vec<PickItem>) -> bool {
        let Some(old) = self.picker.as_mut().filter(|p| p.live) else {
            return false;
        };
        if seq != self.search_seq {
            return false;
        }
        self.search_sent = None;
        let selected = old
            .current()
            .and_then(|cur| {
                items.iter().position(|it| {
                    (&it.path, it.line, it.deleted) == (&cur.path, cur.line, cur.deleted)
                })
            })
            .unwrap_or(0);
        let mut new = Picker::new(old.title.clone(), items, false);
        new.live = true;
        new.selected = selected;
        new.query = std::mem::take(&mut old.query);
        new.bufs = std::mem::take(&mut old.bufs);
        new.marks = std::mem::take(&mut old.marks);
        if self.mode == Mode::Picker(PickerKind::Symbols) {
            new.requery(false);
        }
        new.settle();
        if std::mem::take(&mut self.search_enter)
            && let Some(item) = new.current().cloned()
        {
            let from = self.review_spot();
            self.search_jump(item);
            self.watch_jumped();
            self.review_count(None, from, Some("Picker: Enter"), false);
            tutor::check(self, None);
            return true;
        }
        self.picker = Some(new);
        true
    }

    pub(super) fn project_grep(&self, kind: Kind, here: &Path, pattern: &str) -> Vec<Hit> {
        let sight = self.cs_sight(kind, here);
        let hits = self
            .grep(pattern, false, false, |p| {
                search::in_def_scope(kind, here, p) && sight.as_ref().is_none_or(|s| s.sees(p))
            })
            .unwrap_or_default();
        self.note_cut(&hits);
        hits
    }
}

pub(super) fn at_label(path: &Path, line: usize) -> String {
    format!("{}:{line}", path.display())
}
