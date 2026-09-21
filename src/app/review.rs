//! Review mode: the marks on the buffer, the hunks `c` and `C` step through.

use super::*;

impl App {
    /// Enters review mode on a freshly built app: marks against the base, the cursor on the
    /// first hunk of the open file (unless a line was asked for).
    pub fn start_review(&mut self, review: git::Review) {
        self.review = Some(review);
        self.refresh_diff();
        if let Some(path) = self.buf.path.clone() {
            self.reveal(&path);
            if self.line == 0
                && let Some(&h) = self.diff.hunks.first()
            {
                self.goto_line(h + 1);
                self.hist_note(true);
            }
        }
        // `main` opens the first file with a hunk; say so when that is not the first file.
        if let (Some(r), Some(rel)) = (&self.review, self.rel_current()) {
            let skipped = r.files.iter().take_while(|f| !f.has_hunks()).count();
            if r.files.get(skipped).is_some_and(|f| f.path == rel) {
                self.say_skipped(skipped);
            }
        }
    }

    /// `c` / `C`: the next / previous hunk, crossing into the next file of the review.
    pub(super) fn hunk(&mut self, dir: isize) {
        let Some(r) = self.review.clone() else {
            self.message = "not in review mode".into();
            return;
        };
        let here = if dir > 0 {
            self.diff.hunks.iter().find(|&&h| h > self.line)
        } else {
            self.diff.hunks.iter().rev().find(|&&h| h < self.line)
        };
        if let Some(&h) = here {
            let path = self.buf.path.clone().unwrap();
            self.jump_to(&path, h + 1);
            self.center = true;
            return;
        }
        let at = self
            .rel_current()
            .and_then(|rel| r.files.iter().position(|f| f.path == rel));
        let ahead: Vec<&git::ReviewFile> = match (at, dir > 0) {
            (Some(i), true) => r.files[i + 1..].iter().collect(),
            (Some(i), false) => r.files[..i].iter().rev().collect(),
            (None, true) => r.files.iter().collect(),
            (None, false) => r.files.iter().rev().collect(),
        };
        // Files with nothing to read (binary, a mode change, a pure rename, a submodule) are not
        // stops. Nor is one that does not open: the walk goes on, and the status says why.
        let (mut skipped, mut failed) = (0, None);
        for f in ahead {
            if !f.has_hunks() {
                skipped += 1;
            } else if self.open_review_file(f, dir < 0) {
                match failed {
                    Some(why) => self.message = why,
                    None => self.say_skipped(skipped),
                }
                return;
            } else if self.dirty {
                // Edits that could not be saved hold merl on this file, whatever is ahead.
                return;
            } else {
                failed = Some(std::mem::take(&mut self.message));
            }
        }
        self.message = failed.unwrap_or_else(|| {
            let end = if dir > 0 { "last" } else { "first" };
            format!("{end} hunk of the review")
        });
    }

    /// Why `file 1` became `file 74`.
    fn say_skipped(&mut self, skipped: usize) {
        if skipped > 0 {
            let s = plural(skipped);
            self.message = format!("skipped {skipped} file{s} without hunks");
        }
    }

    /// Opens a file of the review on its first (or `last`) hunk. The hunks are read before
    /// the file opens, so this is one stop in the history.
    pub(super) fn open_review_file(&mut self, f: &git::ReviewFile, last: bool) -> bool {
        let Some(r) = &self.review else { return false };
        let path = self.root.join(&f.path);
        let hunks = match f.status {
            'D' => Vec::new(),
            _ => r.diff(&self.root, &path, Some(f)).hunks,
        };
        let h = if last { hunks.last() } else { hunks.first() };
        self.jump_to(&path, h.map_or(1, |h| h + 1));
        self.center = true;
        self.buf.path.as_deref() == Some(&path)
    }

    /// `hunk 2/5 · file 1/3` for the status bar.
    pub fn review_status(&self) -> Option<String> {
        let r = self.review.as_ref()?;
        let file = self
            .rel_current()
            .and_then(|rel| r.files.iter().position(|f| f.path == rel))
            .map_or("-".to_string(), |i| (i + 1).to_string());
        let hunk = self.diff.hunks.iter().filter(|&&h| h <= self.line).count();
        Some(format!(
            "hunk {hunk}/{}  file {file}/{}",
            self.diff.hunks.len(),
            r.files.len()
        ))
    }

    /// The branch was listed again after a commit or an edit (`git::Review::refresh`): the
    /// panel takes the new rows and counts around its cursor, which stays on its file. Nothing
    /// opens and the code pane does not move; the open file gets new marks when the base moved
    /// or its row came, went or changed its kind (a reverted file stays open, without marks).
    /// Returns whether anything on screen changed.
    pub fn review_refreshed(&mut self, fresh: git::Review) -> bool {
        let Some(old) = &self.review else {
            return false;
        };
        if *old == fresh {
            return false;
        }
        let rel = self.rel_current();
        let kind = |r: &git::Review| {
            let f = r.file(rel.as_deref()?)?;
            Some((f.status, f.old.clone(), f.untracked))
        };
        let stale = old.merge_base != fresh.merge_base || kind(old) != kind(&fresh);
        let paths: Vec<PathBuf> = fresh.files.iter().map(|f| f.path.clone()).collect();
        self.review = Some(fresh);
        self.refresh_tree(crate::tree::from_files(&paths));
        if stale {
            self.refresh_diff();
        }
        true
    }
}
