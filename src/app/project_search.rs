//! `s`: the project grep, run in a thread and answered when it lands.

use super::*;

impl App {
    /// The open file's path relative to the root, for sorting and labels.
    pub(super) fn rel_current(&self) -> Option<PathBuf> {
        let path = self.buf.path.as_ref()?;
        Some(path.strip_prefix(&self.root).unwrap_or(path).to_path_buf())
    }

    /// Greps the project files `wanted` accepts. The open file is searched whenever it is
    /// wanted, even when the startup walk skipped it (ignored), and first: its hits sort to the
    /// top, so they must not be the ones [`search::MAX_HITS`] cuts.
    pub(super) fn grep(
        &self,
        pattern: &str,
        whole_word: bool,
        smart_case: bool,
        wanted: impl Fn(&Path) -> bool,
    ) -> anyhow::Result<Vec<Hit>> {
        self.grep_job(0, pattern, wanted)
            .run(whole_word, smart_case)
    }

    /// Everything a grep for `pattern` needs, owned, so it can run in a thread.
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
            // With unsaved edits the open file is searched as it is on screen, so a hit's line
            // is a line of the buffer the jump lands in.
            unsaved: self.dirty.then(|| self.buf.to_bytes()),
        }
    }

    /// The word under the cursor, with `extra` characters counting as part of it.
    pub(super) fn word_under(&self, extra: &str) -> Option<String> {
        search::word_at(self.line_str(), self.col, extra).map(|(_, w)| w.to_string())
    }

    pub(super) fn kind(&self) -> Option<Kind> {
        self.buf.path.as_deref().and_then(search::kind_of)
    }

    /// `rel/path:line: text` rows for a result picker. The text keeps its tabs, as `Buffer`
    /// does, so `ui` can line it up with the file's highlighting; tabs are expanded when drawn.
    pub(crate) fn hit_items(hits: Vec<Hit>) -> Vec<PickItem> {
        hits.into_iter()
            .map(|h| {
                let label = format!("{}:{}: ", h.path.display(), h.line);
                let code_at = Some(label.len());
                PickItem {
                    label: label + &clip(h.text.trim(), MAX_LABEL_TEXT),
                    path: h.path,
                    line: h.line,
                    code_at,
                }
            })
            .collect()
    }

    /// `s`: the result picker, empty, with the query as its input line. The hits are a
    /// smart-case grep for the query as typed, over every file, refreshed as it changes. Literal
    /// like `/`: `foo(` finds the calls and the definition, not a regex error.
    pub(super) fn start_search(&mut self) {
        self.show_picker(PickerKind::Search, Vec::new());
        if let Some(p) = &mut self.picker {
            p.live = true;
        }
        self.drop_pending_search();
    }

    /// Forgets the grep on its way: its answer belongs to a picker that is gone, and its number
    /// must not be the one the next picker waits for.
    pub(super) fn drop_pending_search(&mut self) {
        self.search_seq += 1;
        (self.search_due, self.search_sent, self.search_enter) = (None, None, false);
    }

    /// Whether the query on screen has no answer yet: the pause is running or its grep is. The
    /// rows and their count belong to an older query until this is false.
    pub fn search_pending(&self) -> bool {
        self.search_due.is_some() || self.search_sent.is_some()
    }

    /// The grep to start now, once the query has stood still for [`SEARCH_PAUSE`]. The event
    /// loop runs it in a thread and hands the hits to [`App::search_done`].
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
        // `s` greps the query itself, as text; `D` past the cap greps the declaration patterns
        // and keeps the names the query matches.
        let symbols = self.mode == Mode::Picker(PickerKind::Symbols);
        let pattern = if symbols {
            query
        } else {
            regex::escape(&query)
        };
        Some(SearchJob {
            symbols,
            ..self.grep_job(self.search_seq, &pattern, |_| true)
        })
    }

    /// Test helper: the pending grep, run here, and its rows in the picker.
    #[cfg(test)]
    pub(crate) fn settle_search(&mut self) {
        self.search_due = self.search_due.map(|_| Instant::now());
        if let Some(job) = self.search_tick() {
            self.search_done(job.seq, job.items());
        }
    }

    /// Enter in the `s` picker, on the hits of the query on screen, whether they were in before
    /// the key or came after it: the jump to `item`, or a word that the query found nothing.
    pub(super) fn search_jump(&mut self, item: Option<PickItem>, query: &str) {
        self.picker = None;
        self.mode = Mode::Normal;
        match item {
            Some(item) => self.jump_to(&self.root.join(&item.path), item.line),
            None if query.is_empty() => {}
            None => self.message = format!("no results for {query}"),
        }
    }

    /// The rows of grep number `seq`. An answer to anything but the query on screen is dropped.
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
        // `s` keeps the order its grep found between queries, so the cursor stays on its hit.
        // `D` re-ranks below and takes the cursor to the best row instead.
        let selected = old
            .current()
            .and_then(|cur| {
                items
                    .iter()
                    .position(|it| (&it.path, it.line) == (&cur.path, cur.line))
            })
            .unwrap_or(0);
        if std::mem::take(&mut self.search_enter) {
            // Not through the new picker: nucleo has not seen its items yet.
            let query = old.query.to_string();
            self.search_jump(items.into_iter().nth(selected), &query);
            tutor::check(self);
            return true;
        }
        let mut new = Picker::new(old.title.clone(), items, false);
        new.live = true;
        new.selected = selected;
        new.query = std::mem::take(&mut old.query);
        new.bufs = std::mem::take(&mut old.bufs);
        // `D` hands nucleo the query too: the grep says which declarations the name reaches,
        // nucleo ranks them and marks the letters, and the cursor goes to the best of them —
        // the same reset a keystroke under the cap does. `s` keeps the order its grep found,
        // and with it the row the cursor was on.
        if self.mode == Mode::Picker(PickerKind::Symbols) {
            new.requery(false);
        }
        // Matched before it is shown: nothing is pending from here on, so an empty list must
        // mean the grep found nothing, and Enter and the cursor must see the rows it found.
        new.settle();
        self.picker = Some(new);
        true
    }
}
