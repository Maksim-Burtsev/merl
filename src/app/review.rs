//! Review mode: the marks on the buffer, the hunks `c` and `C` step through.

use super::*;

impl App {
    /// Enters review mode on a freshly built app: marks against the base, the cursor on the
    /// first hunk of the open file (unless a line was asked for).
    pub fn start_review(&mut self, mut review: git::Review) {
        let note = review.note.take();
        self.review = Some(review);
        self.follow_branch();
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
                match failed {
                    Some(why) => self.message = why,
                    None => self.say_skipped(skipped),
                }
                // After the walk's word: a mark that could not be saved says so over it.
                self.mark_viewed(read.take());
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
        self.mark_viewed(read.take());
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
        let hunks = self.review_hunks(r, f);
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
        if self.viewed.remove(&rel).is_some() {
            self.message = "not viewed".into();
            self.save_viewed();
        } else {
            self.message = "viewed".into();
            self.mark_viewed(Some(rel));
        }
    }

    fn mark_viewed(&mut self, rel: Option<PathBuf>) {
        if let Some(rel) = rel {
            let hash = self.disk_hash(&rel);
            self.changed.remove(&rel);
            self.viewed.insert(rel, hash);
            self.save_viewed();
        }
    }

    /// What is on disk at `rel`, hashed; a file the branch deleted hashes as 0. FNV-1a, not
    /// `DefaultHasher`: the hashes are kept on disk, and std's may change with the compiler.
    fn disk_hash(&self, rel: &Path) -> u64 {
        std::fs::read(self.root.join(rel)).map_or(0, |bytes| {
            let fnv = |h: u64, &b: &u8| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
            bytes.iter().fold(0xcbf2_9ce4_8422_2325, fnv)
        })
    }

    /// Sorts every mark by the listing and the disk: a listed file whose hash matches is viewed,
    /// one that changed since is `↻` until it is viewed again (or changes back), and a file the
    /// listing does not have keeps its mark, unseen, for when it comes back. A mark leaves only
    /// when `m` takes it off, or with its review after [`FORGET_DAYS`]. Returns whether a mark on
    /// screen moved.
    // ponytail: reads every listed marked file on each refresh; compare mtimes first if a review
    // of thousands of viewed files ever makes the refresh slow.
    fn recheck_viewed(&mut self) -> bool {
        let (mut viewed, mut changed, mut unlisted) =
            (HashMap::new(), HashMap::new(), HashMap::new());
        let marks = self
            .viewed
            .iter()
            .chain(&self.changed)
            .chain(&self.unlisted);
        for (p, &h) in marks {
            let to = if !self.review.as_ref().is_some_and(|r| r.file(p).is_some()) {
                &mut unlisted
            } else if self.disk_hash(p) == h {
                &mut viewed
            } else {
                &mut changed
            };
            to.insert(p.clone(), h);
        }
        let moved = viewed != self.viewed || changed != self.changed;
        (self.viewed, self.changed, self.unlisted) = (viewed, changed, unlisted);
        moved
    }

    /// Where the viewed marks of every review of the repository are kept: in its common git dir,
    /// so a worktree's review shares them and a deleted clone takes them along.
    fn viewed_store(&self) -> Option<PathBuf> {
        git::dirs(&self.root).map(|(_, common)| common.join("merl/viewed"))
    }

    /// Keeps the marks under the branch the listing names, the one reading of HEAD: another
    /// branch checked out is another review, with the marks it left last time. A detached HEAD
    /// names none and keeps the branch it left. A review that had none until now keeps what it
    /// marked, over the branch's own marks, and saves them.
    fn follow_branch(&mut self) {
        let Some(r) = &self.review else { return };
        let Some(branch) = r.branch_name().map(str::to_string) else {
            return;
        };
        if self.viewed_branch.as_ref() == Some(&branch) {
            return;
        }
        let base = r.base.clone();
        let first = self.viewed_branch.replace(branch.clone()).is_none();
        let Some(store) = self.viewed_store() else {
            return;
        };
        let mut marks = read_viewed(&store, &branch, &base);
        let session = [&mut self.viewed, &mut self.changed, &mut self.unlisted].map(std::mem::take);
        let kept = first && session.iter().any(|m| !m.is_empty());
        if first {
            marks.extend(session.into_iter().flatten());
        }
        self.viewed = marks;
        self.recheck_viewed();
        if kept {
            self.save_viewed();
        }
    }

    /// Writes this review's marks, the `↻` and the unlisted ones too, so they outlive the
    /// session as well.
    fn save_viewed(&mut self) {
        let (Some(store), Some(r), Some(branch)) =
            (self.viewed_store(), &self.review, &self.viewed_branch)
        else {
            return;
        };
        let marks = (self.viewed.iter().chain(&self.changed)).chain(&self.unlisted);
        if let Err(e) = write_viewed(&store, branch, &r.base, marks, crate::stats::today()) {
            self.message = format!("viewed marks not saved: {e:#}");
        }
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
        let hunks = self.review_hunks(r, f);
        let h = if last { hunks.last() } else { hunks.first() };
        self.jump_to(&path, h.map_or(1, |h| h + 1));
        self.center = true;
        self.buf.path.as_deref() == Some(&path)
    }

    /// The lines `c` stops on in a file of the review: where its hunks start, and the top of a
    /// deleted one, which has none to step through.
    fn review_hunks(&self, r: &git::Review, f: &git::ReviewFile) -> Vec<usize> {
        match f.status {
            'D' => vec![0],
            _ => r.diff(&self.root, &self.root.join(&f.path), Some(f)).hunks,
        }
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
            return self.recheck_viewed();
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
        self.follow_branch();
        self.recheck_viewed();
        true
    }
}

/// A review's marks untouched for this many days go the next time the store is written.
const FORGET_DAYS: i64 = 30;

/// The store is a line per mark: `YYYY-MM-DD<TAB>branch<TAB>base<TAB>hash<TAB>path`, the day
/// being when its review was last written. The path comes last, so a tab in it reads back, and
/// lines end at `\n` alone, so does a `\r` (macOS's `Icon\r`).
fn read_viewed(store: &Path, branch: &str, base: &str) -> HashMap<PathBuf, u64> {
    let text = std::fs::read_to_string(store).unwrap_or_default();
    let mark = |line: &str| {
        let [_, b, s, hash, path] = line.splitn(5, '\t').collect::<Vec<_>>()[..] else {
            return None;
        };
        let hash = u64::from_str_radix(hash, 16).ok()?;
        (b == branch && s == base).then(|| (PathBuf::from(path), hash))
    };
    text.split('\n').filter_map(mark).collect()
}

/// Replaces the review's lines in the store with `marks`, dated `today`, and drops the reviews
/// untouched for [`FORGET_DAYS`]. The store is read again right before, so the marks another
/// merl wrote for another review stay; a store that cannot be read is left as it is.
fn write_viewed<'a>(
    store: &Path,
    branch: &str,
    base: &str,
    marks: impl Iterator<Item = (&'a PathBuf, &'a u64)>,
    today: i64,
) -> anyhow::Result<()> {
    use std::fmt::Write as _;
    let old = match std::fs::read_to_string(store) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        read => read?,
    };
    let mut text = String::new();
    for line in old.split('\n') {
        let mut cols = line.splitn(4, '\t');
        let (Some(day), Some(b), Some(s)) =
            (cols.next().map(crate::stats::day), cols.next(), cols.next())
        else {
            continue;
        };
        if day.is_some_and(|d| today - d < FORGET_DAYS) && (b, s) != (branch, base) {
            text.push_str(line);
            text.push('\n');
        }
    }
    let day = crate::stats::date(today);
    for (path, hash) in marks {
        // A name that is not UTF-8, or holds a newline, would not read back as itself.
        if let Some(path) = path.to_str().filter(|p| !p.contains('\n')) {
            _ = writeln!(text, "{day}\t{branch}\t{base}\t{hash:016x}\t{path}");
        }
    }
    crate::stats::replace(store, &text)
}
