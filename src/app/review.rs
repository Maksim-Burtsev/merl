//! Review mode: the marks on the buffer, the hunks `c` and `C` step through.

use super::*;
use crate::reviews::{Session, Spot, Stop};

impl App {
    /// Enters review mode on a freshly built app: marks against the base, the cursor on the
    /// first hunk of the open file (unless a line was asked for).
    pub fn start_review(&mut self, mut review: git::Review) {
        let note = review.note.take();
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
        // A branch older than what was pushed matters more than the files skipped.
        if let Some(note) = note {
            self.message = note;
        }
        if let Some(r) = &self.review {
            let (repo, branch) = (self.root_name(), r.branch_or_commit(&self.root));
            let (opened, root) = (r.clone(), self.root.clone());
            let counting = std::thread::spawn(move || count_stops(&opened, &root));
            let stop = self.review_stop();
            self.session = Some(Session::new(
                Instant::now(),
                repo,
                branch,
                r,
                counting,
                stop,
            ));
        }
    }

    /// The open file, the cursor's line, and whether the file is one of the review's.
    pub(super) fn review_spot(&self) -> Spot {
        let r = self.review.as_ref();
        let on = (self.rel_current()).is_some_and(|rel| r.is_some_and(|r| r.file(&rel).is_some()));
        (self.buf.path.clone(), self.line, on)
    }

    /// The stop the cursor stands on, numbered as [`stops_in`] counts them: the hunk as the
    /// status bar's `hunk i/n` has it, or the top of a deleted file, its one line in
    /// [`review_hunks`].
    fn review_stop(&self) -> Option<Stop> {
        let rel = self.rel_current()?;
        let f = self.review.as_ref()?.file(&rel)?;
        let i = match f.status {
            _ if !f.has_hunks() => return None,
            'D' => (self.line == 0).then_some(0)?,
            _ => self.diff.hunks.iter().position(|&h| h == self.line)?,
        };
        Some((rel, i + 1))
    }

    /// Review stats (#242): a press at `at` done as `action`, the cursor at `from` before it, or
    /// with no `at` a jump that came after its press (`s` answering an Enter that did not wait).
    /// `quit`: the press quits merl.
    pub(super) fn review_count(
        &mut self,
        at: Option<Instant>,
        from: Spot,
        action: Option<&str>,
        quit: bool,
    ) {
        let to = self.review_spot();
        // A stop the review walk (`c`, `C`, the panel) put the cursor on, judged against the
        // cursor before this press: a reload that moved it since the last one does not make a
        // press that stayed put a stop.
        let walked = from != to && matches!(action, Some("c" | "C" | "Tree: Enter"));
        let stop = walked.then(|| self.review_stop()).flatten();
        let end = action == Some("c") && self.message == "last hunk of the review";
        if let Some(s) = &mut self.session {
            if let Some(at) = at {
                s.pressed(at, &from, quit);
            }
            s.moved(action, &from, &to, stop, end);
        }
    }

    /// The session's line in the review stats: the repository, the branch and the columns
    /// after the round. None for a session without a press but the one that quits, nor for
    /// one whose stops could not be counted: a wrong number in the stats is worse than a
    /// session missing from them.
    pub fn review_row(&mut self) -> Option<(String, String, String)> {
        let s = self.session.as_mut()?;
        let columns = s.columns(self.viewed.keys())?;
        Some((s.repo.clone(), s.branch.clone(), columns))
    }

    /// `c` / `C`: the next / previous hunk, crossing into the next file of the review. From a
    /// file outside it, back to the hunk they last stopped on (#239).
    pub(super) fn hunk(&mut self, dir: isize) {
        let Some(r) = self.review.clone() else {
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
            self.remember_hunk();
            return;
        }
        // An excursion (`d`, `u`, `s`) left the review: the way back is one key.
        if let Some((rel, line)) = self.hunk_left(&r) {
            self.jump_to(&self.root.join(rel), line + 1);
            self.center = true;
            return;
        }
        // `c` stopped on every hunk of this file and now leaves it: the file is viewed. The last
        // file of the review has nowhere to go, and the same press marks it.
        let mut read = self
            .rel_current()
            .filter(|rel| dir > 0 && r.file(rel).is_some());
        // Files with nothing to read (binary, a mode change, a pure rename, a submodule) are not
        // stops. Nor is one that does not open: the walk goes on, and the status says why.
        let (mut skipped, mut failed) = (0, None);
        for f in self.ahead(&r, dir) {
            if !f.has_hunks() {
                skipped += 1;
            } else if self.open_review_file(f, dir < 0) {
                self.remember_hunk();
                self.mark_viewed(read.take());
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
        self.mark_viewed(read.take());
        self.message = failed.unwrap_or_else(|| {
            let end = if dir > 0 { "last" } else { "first" };
            format!("{end} hunk of the review")
        });
    }

    /// The files of the review `c` (`dir` 1) or `C` (-1) walks on to from the open one, nearest
    /// first: from a file outside the review, all of them.
    pub(super) fn ahead<'r>(&self, r: &'r git::Review, dir: isize) -> Vec<&'r git::ReviewFile> {
        let at = self
            .rel_current()
            .and_then(|rel| r.files.iter().position(|f| f.path == rel));
        match (at, dir > 0) {
            (Some(i), true) => r.files[i + 1..].iter().collect(),
            (Some(i), false) => r.files[..i].iter().rev().collect(),
            (None, true) => r.files.iter().collect(),
            (None, false) => r.files.iter().rev().collect(),
        }
    }

    /// `c` / `C` stopped on the hunk under the cursor: its file and its place among the file's
    /// hunks, for the way back from outside the review. A place, not a line: lines an agent
    /// writes or deletes around it move the hunk, and only a hunk added or removed above it
    /// changes its place.
    fn remember_hunk(&mut self) {
        let i = self.diff.hunks.iter().filter(|&&h| h < self.line).count();
        self.last_hunk = self.rel_current().map(|rel| (rel, i));
    }

    /// Where `c` / `C` go back to while the open file is outside the review: the line of the
    /// hunk they last stopped on, found by its place in its file while the walk still stops at
    /// that file; its last hunk when fewer are left. A hunk added above lands one earlier, and
    /// the next `c` reaches the one left; a hunk removed above lands on the next, past the one
    /// left, which was read.
    // ponytail: two hunks removed at or above the one left can pass an unread hunk, and a way
    // back that fell to the file's last hunk keeps the larger index, not the one it landed on;
    // recognise the hunk by its text if agents ever rewrite that much under a reader.
    pub(super) fn hunk_left(&self, r: &git::Review) -> Option<(PathBuf, usize)> {
        if self.rel_current().is_some_and(|rel| r.file(&rel).is_some()) {
            return None;
        }
        let (rel, i) = self.last_hunk.clone()?;
        let f = r.file(&rel).filter(|f| f.has_hunks())?;
        let hunks = review_hunks(&self.root, r, f);
        let h = *hunks.get(i).or(hunks.last())?;
        Some((rel, h))
    }

    /// `m`: the open file, or the panel's row, is viewed; again, and it is not. A key for the
    /// review's files only: anywhere else it does nothing and says nothing.
    pub(super) fn toggle_viewed(&mut self) {
        let rel = match self.focus {
            Focus::Tree => self.tree.selected().map(|n| n.path.clone()),
            Focus::Code => self.rel_current(),
        };
        let Some(rel) = rel.filter(|p| self.review.as_ref().is_some_and(|r| r.file(p).is_some()))
        else {
            return;
        };
        self.message = if self.viewed.remove(&rel).is_some() {
            "not viewed".into()
        } else {
            self.mark_viewed(Some(rel));
            "viewed".into()
        };
    }

    fn mark_viewed(&mut self, rel: Option<PathBuf>) {
        if let Some(rel) = rel {
            let hash = self.disk_hash(&rel);
            self.viewed.insert(rel, hash);
        }
    }

    /// What is on disk at `rel`, hashed; a file the branch deleted hashes as nothing.
    fn disk_hash(&self, rel: &Path) -> u64 {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut h = DefaultHasher::new();
        std::fs::read(self.root.join(rel)).ok().hash(&mut h);
        h.finish()
    }

    /// A viewed file that changed on disk, or left the review, is not viewed any more. Returns
    /// whether a mark went.
    // ponytail: reads every viewed file on each refresh; compare mtimes first if a review of
    // thousands of viewed files ever makes the refresh slow.
    fn drop_stale_viewed(&mut self) -> bool {
        let before = self.viewed.len();
        let mut viewed = std::mem::take(&mut self.viewed);
        viewed.retain(|p, h| {
            self.review.as_ref().is_some_and(|r| r.file(p).is_some()) && self.disk_hash(p) == *h
        });
        self.viewed = viewed;
        self.viewed.len() != before
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
        let hunks = review_hunks(&self.root, r, f);
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
            // An edit can leave every count as it was.
            return self.drop_stale_viewed();
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
        self.drop_stale_viewed();
        true
    }
}

/// The stops `c` can make in each file of `r`, by path; `None` when a file failed to count.
fn count_stops(r: &git::Review, root: &Path) -> Option<HashMap<PathBuf, usize>> {
    (r.files.iter())
        .map(|f| Some((f.path.clone(), stops_in(r, root, f)?)))
        .collect()
}

/// The stops `c` makes in a file of the review: those of [`review_hunks`] in a file the walk
/// stops at, none in one it passes (binary, a mode change, a pure rename, a submodule). `None`
/// for a file with lines to read and no hunk: a `git diff` that failed.
fn stops_in(r: &git::Review, root: &Path, f: &git::ReviewFile) -> Option<usize> {
    match f.has_hunks() {
        true => Some(review_hunks(root, r, f).len()).filter(|&n| n > 0),
        false => Some(0),
    }
}

/// The lines `c` stops on in a file of the review: where its hunks start, and the top of a
/// deleted one, which has none to step through.
fn review_hunks(root: &Path, r: &git::Review, f: &git::ReviewFile) -> Vec<usize> {
    match f.status {
        'D' => vec![0],
        _ => r.diff(root, &root.join(&f.path), Some(f)).hunks,
    }
}
