//! Review mode: the marks on the buffer, the hunks `c` and `C` step through.

use anyhow::Context as _;

use super::*;
use crate::reviews::{Session, Spot, Stop};

impl App {
    /// Enters review mode on a freshly built app: marks against the base, the cursor on the
    /// first hunk of the open file (unless a line was asked for).
    pub fn start_review(&mut self, mut review: git::Review) {
        let note = review.note.take();
        // Kept under the branch the review starts on, or rebases; one started detached is kept
        // in memory for its whole life.
        self.viewed_branch = review.branch_name().map(str::to_string);
        self.review = Some(review);
        self.refresh_diff();
        if let Some(path) = self.buf.path.clone() {
            self.reveal(&path);
            if self.line == 0
                && let Some(&h) = self.diff.hunks.first()
            {
                self.stand_on(h);
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
        // And marks that cannot be read, more than both.
        self.load_viewed();
        self.open_session();
    }

    /// Review stats (#242): a session of the review as it stands now starts here, its stops
    /// counted on a thread.
    fn open_session(&mut self) {
        if let Some(r) = &self.review {
            let (repo, branch) = (self.root_name(), r.branch_or_commit(&self.root));
            let (opened, root, unfolded) = (r.clone(), self.root.clone(), self.unfolded.clone());
            let counting = std::thread::spawn(move || count_stops(&opened, &root, &unfolded));
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
        (self.buf.path.clone(), self.at(), on)
    }

    /// The stop the cursor stands on, numbered as [`stops_in`] counts them: the hunk as the
    /// status bar's `hunk i/n` has it, or the top of a deleted file, its one line in
    /// [`review_hunks`].
    // ponytail: a stop is a hunk by its index at the time of the stop, checked against the
    // count taken when the review opened, so an agent's edits mid-review can credit a new hunk
    // or drop an unreached one; and a file the walk cannot open (a broken symlink) still counts
    // its hunk. Identify a hunk by its text if the numbers need to hold under edits.
    fn review_stop(&self) -> Option<Stop> {
        let rel = self.rel_current()?;
        let f = self.review.as_ref()?.file(&rel)?;
        let i = match f.status {
            _ if !f.has_hunks() => return None,
            _ if folded(f, &self.unfolded) => 0,
            'D' => (self.line == 0).then_some(0)?,
            _ => self.diff.hunks.iter().position(|&h| h == self.at())?,
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

    /// Every session's line, oldest first: those a `git switch` closed, then the one under way.
    pub fn review_rows(&mut self) -> Vec<(String, String, String)> {
        let closed = std::mem::take(&mut self.closed).into_iter();
        let mut rows: Vec<_> = closed
            .filter_map(|(mut s, viewed)| {
                let columns = s.columns(viewed.iter())?;
                Some((s.repo, s.branch, columns))
            })
            .collect();
        rows.extend(self.review_row());
        rows
    }

    /// `c` / `C`: the next / previous hunk, crossing into the next file of the review. From a
    /// file outside it, back to the hunk they last stopped on (#239).
    pub(super) fn hunk(&mut self, dir: isize) {
        let Some(r) = self.review.clone() else {
            return;
        };
        if let Some(h) = self.hunk_ahead(dir) {
            let path = self.buf.path.clone().unwrap();
            self.jump_to_hunk(&path, h);
            self.remember_hunk();
            return;
        }
        // An excursion (`d`, `u`, `s`) left the review: the way back is one key.
        if let Some((rel, h)) = self.hunk_left(&r) {
            self.jump_to_hunk(&self.root.join(rel), h);
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

    /// The hunk of the open file `c` (`dir` 1) or `C` (-1) goes to; `None` when it goes on to
    /// another file. A fold is one stop: the walk goes on from it (#243).
    pub(super) fn hunk_ahead(&self, dir: isize) -> Option<TextLine> {
        if self.folded_here().is_some() {
            return None;
        }
        let at = self.at();
        match dir > 0 {
            true => self.diff.hunks.iter().find(|&&h| h > at).copied(),
            false => self.diff.hunks.iter().rev().find(|&&h| h < at).copied(),
        }
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
        let i = self.diff.hunks.iter().filter(|&&h| h < self.at()).count();
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
    pub(super) fn hunk_left(&self, r: &git::Review) -> Option<(PathBuf, TextLine)> {
        if self.rel_current().is_some_and(|rel| r.file(&rel).is_some()) {
            return None;
        }
        let (rel, i) = self.last_hunk.clone()?;
        let f = r.file(&rel).filter(|f| f.has_hunks())?;
        let hunks = review_hunks(&self.root, r, f);
        let h = *hunks.get(i).or(hunks.last())?;
        Some((rel, h))
    }

    /// The open file when it is a folded file of the review (#243): the code pane shows its
    /// fold instead of the text, `c` stops on it once, and the keys that read the text wait for
    /// Enter. The one answer the draw and the keys both ask; a refresh never folds the text in
    /// front of the user (see [`review_refreshed`](Self::review_refreshed)).
    pub fn folded_here(&self) -> Option<&git::ReviewFile> {
        let rel = self.rel_current()?;
        let f = self.review.as_ref()?.file(&rel)?;
        folded(f, &self.unfolded).then_some(f)
    }

    /// Enter on a fold: the diff loads, and the file stays unfolded in this review and the next
    /// ones of the branch. The walk left the cursor on a hunk: it starts at the first one. A jump
    /// (`d`, `u`, `s`, `D`, `[`) left it on its line: the line is kept.
    pub(super) fn unfold(&mut self) {
        let Some(rel) = self.folded_here().map(|f| f.path.clone()) else {
            return;
        };
        self.unfolded.insert(rel);
        self.save_unfolded();
        match self.diff.hunks.first() {
            Some(&h) if self.diff.hunks.contains(&self.at()) => self.stand_on(h),
            _ => self.center = true,
        }
        self.remember_hunk();
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
            self.hidden.remove(&rel);
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

    /// Sorts every mark by the listing and the disk: a listed file whose hash matches is viewed;
    /// one that changed since loses its tick, as on GitLab, and one the listing does not have is
    /// not shown either. Both keep their mark, hidden, for when the file comes back as it was
    /// viewed. A mark leaves only when `m` takes it off, or with its review after
    /// [`FORGET_DAYS`]. Returns whether a tick moved.
    // ponytail: reads every listed marked file on each refresh; compare mtimes first if a review
    // of thousands of viewed files ever makes the refresh slow.
    fn recheck_viewed(&mut self) -> bool {
        let (mut viewed, mut hidden) = (HashMap::new(), HashMap::new());
        for (p, &h) in self.viewed.iter().chain(&self.hidden) {
            let listed = self.review.as_ref().is_some_and(|r| r.file(p).is_some());
            let to = if listed && self.disk_hash(p) == h {
                &mut viewed
            } else {
                &mut hidden
            };
            to.insert(p.clone(), h);
        }
        let moved = viewed != self.viewed;
        (self.viewed, self.hidden) = (viewed, hidden);
        moved
    }

    /// Where the viewed marks of every review of the repository are kept: in its common git dir,
    /// so a worktree's review shares them and a deleted clone takes them along.
    fn viewed_store(&self) -> Option<PathBuf> {
        git::dirs(&self.root).map(|(_, common)| common.join("merl/viewed"))
    }

    /// Where the unfolded files are kept, beside the viewed marks and in their format, with no
    /// hash: a file stays unfolded whatever is written into it.
    fn unfolded_store(&self) -> Option<PathBuf> {
        git::dirs(&self.root).map(|(_, common)| common.join("merl/unfolded"))
    }

    /// The marks the review's branch left, sorted by what is on disk now; loading writes nothing.
    /// A store that cannot be found or read gives none, and the review keeps its marks in memory
    /// for the rest of the session instead of writing over it.
    fn load_viewed(&mut self) {
        let (Some(branch), Some(r)) = (&self.viewed_branch, &self.review) else {
            return;
        };
        let read = (self.viewed_store())
            .context("no git directory")
            .and_then(|store| read_viewed(&store, branch, &r.base, crate::stats::today()));
        self.viewed.clear();
        self.hidden.clear();
        // Unfolded files that cannot be read fold again: the diff is one Enter away.
        self.unfolded = (self.unfolded_store())
            .and_then(|store| read_viewed(&store, branch, &r.base, crate::stats::today()).ok())
            .map(|marks| marks.into_keys().collect())
            .unwrap_or_default();
        match read {
            Ok(marks) => self.viewed = marks,
            Err(e) => {
                self.viewed_branch = None;
                self.message = format!("viewed marks not read: {}", super::error_text(&e));
            }
        }
        self.recheck_viewed();
    }

    /// Keeps the marks under the branch the listing names, the one reading of HEAD: another
    /// branch checked out is another review, with the marks it left last time. A detached HEAD
    /// names none and keeps the branch it left, and so does a refresh on which git cannot say
    /// where the store is. A review with no branch to keep its marks under follows none. Returns
    /// whether it took another branch's marks.
    fn follow_branch(&mut self) -> bool {
        let (Some(key), Some(r)) = (&self.viewed_branch, &self.review) else {
            return false;
        };
        let Some(branch) = r.branch_name().filter(|b| b != key).map(str::to_string) else {
            return false;
        };
        if self.viewed_store().is_none() {
            return false;
        }
        self.viewed_branch = Some(branch);
        self.load_viewed();
        true
    }

    /// Writes this review's marks, the hidden ones too, so they outlive the session as well.
    fn save_viewed(&mut self) {
        let (Some(store), Some(r), Some(branch)) =
            (self.viewed_store(), &self.review, &self.viewed_branch)
        else {
            return;
        };
        let marks = self.viewed.iter().chain(&self.hidden);
        if let Err(e) = write_viewed(&store, branch, &r.base, marks, crate::stats::today()) {
            self.message = format!("viewed marks not saved: {}", super::error_text(&e));
        }
    }

    /// Writes the review's unfolded files, as [`save_viewed`](Self::save_viewed) writes the marks.
    fn save_unfolded(&mut self) {
        let (Some(store), Some(r), Some(branch)) =
            (self.unfolded_store(), &self.review, &self.viewed_branch)
        else {
            return;
        };
        let files = self.unfolded.iter().map(|p| (p, &0));
        if let Err(e) = write_viewed(&store, branch, &r.base, files, crate::stats::today()) {
            self.message = format!("unfolded files not saved: {e:#}");
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
        let hunks = review_hunks(&self.root, r, f);
        let h = if last { hunks.last() } else { hunks.first() };
        self.jump_to_hunk(&path, h.copied().unwrap_or(TextLine::File(0)));
        self.buf.path.as_deref() == Some(&path)
    }

    fn jump_to_hunk(&mut self, path: &Path, h: TextLine) {
        self.jump_to(path, h.key() + 1);
        if self.buf.path.as_deref() == Some(path) {
            self.stand_on(h);
        }
        self.center = true;
    }

    /// The cursor to the start of line `h` of the open file's text, the view centred on it.
    fn stand_on(&mut self, h: TextLine) {
        self.set_at(self.clamp_line(h));
        (self.col, self.want_x, self.center) = (0, 0, true);
    }

    /// `hunk 2/5  file 1/3` for the status bar; nothing on a file the branch did not change,
    /// which then reads as it does outside a review (#286).
    pub fn review_status(&self) -> Option<String> {
        let r = self.review.as_ref()?;
        let rel = self.rel_current()?;
        let file = r.files.iter().position(|f| f.path == rel)?;
        if self.folded_here().is_some() {
            return Some(format!("folded  file {}/{}", file + 1, r.files.len()));
        }
        let at = match self.at() {
            TextLine::Deleted(k, _) => TextLine::File(k),
            at => at,
        };
        let hunk = self.diff.hunks.iter().filter(|&&h| h <= at).count();
        Some(format!(
            "hunk {hunk}/{}  file {}/{}",
            self.diff.hunks.len(),
            file + 1,
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
            // An edit can leave every count as it was; a key left behind on a refresh where git
            // could not say where the store is catches up.
            return self.follow_branch() | self.recheck_viewed();
        }
        let rel = self.rel_current();
        let kind = |r: &git::Review| {
            let f = r.file(rel.as_deref()?)?;
            Some((f.status, f.old.clone(), f.untracked))
        };
        let stale = old.merge_base != fresh.merge_base || kind(old) != kind(&fresh);
        let switched = old.branch != fresh.branch;
        let paths: Vec<PathBuf> = fresh.files.iter().map(|f| f.path.clone()).collect();
        // A file whose text is on screen stays on screen when the listing comes to call it
        // generated (#243): the user may be typing into it.
        let shown = rel.clone().filter(|_| self.folded_here().is_none());
        self.review = Some(fresh);
        self.refresh_tree(crate::tree::from_listing(&paths));
        if stale {
            self.refresh_diff();
        }
        // A `git switch`: the session so far is the old branch's, closed as it stands with the
        // marks it had, and the new branch's starts here, so every line is about one branch.
        // Its stops are counted with its own unfolded files: the marks load first.
        let closed = if switched { self.session.take() } else { None };
        let reopen = closed.is_some();
        if let Some(s) = closed {
            self.closed.push((s, self.viewed.keys().cloned().collect()));
        }
        self.follow_branch();
        if let Some(rel) = shown
            && self.folded_here().is_some()
        {
            self.unfolded.insert(rel);
            self.save_unfolded();
        }
        if reopen {
            self.open_session();
        }
        self.recheck_viewed();
        true
    }
}

/// The stops `c` can make in each file of `r`, by path; `None` when a file failed to count.
fn count_stops(
    r: &git::Review,
    root: &Path,
    unfolded: &HashSet<PathBuf>,
) -> Option<HashMap<PathBuf, usize>> {
    (r.files.iter())
        .map(|f| Some((f.path.clone(), stops_in(r, root, f, unfolded)?)))
        .collect()
}

/// Is `f` folded: generated (#243), with lines to read, and its diff not loaded with Enter in
/// this review. A binary file, a mode change or a pure rename has nothing to fold.
fn folded(f: &git::ReviewFile, unfolded: &HashSet<PathBuf>) -> bool {
    f.generated && f.has_hunks() && !unfolded.contains(&f.path)
}

/// The stops `c` makes in a file of the review: those of [`review_hunks`] in a file the walk
/// stops at, one on a fold, none in one it passes (binary, a mode change, a pure rename, a
/// submodule). `None` for a file with lines to read and no hunk: a `git diff` that failed.
fn stops_in(
    r: &git::Review,
    root: &Path,
    f: &git::ReviewFile,
    unfolded: &HashSet<PathBuf>,
) -> Option<usize> {
    match f.has_hunks() {
        true if folded(f, unfolded) => Some(1),
        true => Some(review_hunks(root, r, f).len()).filter(|&n| n > 0),
        false => Some(0),
    }
}

/// The lines `c` stops on in a file of the review: where its hunks start, and the top of a
/// deleted one, which has none to step through.
fn review_hunks(root: &Path, r: &git::Review, f: &git::ReviewFile) -> Vec<TextLine> {
    match f.status {
        'D' => vec![TextLine::File(0)],
        _ => r.diff(root, &root.join(&f.path), Some(f)).hunks,
    }
}

/// A review's marks untouched for this many days are not read, and go the next time the store
/// is written.
const FORGET_DAYS: i64 = 30;

/// Is a line dated `day` (as the store writes it) younger than [`FORGET_DAYS`] on `today`?
fn fresh(day: &str, today: i64) -> bool {
    crate::stats::day(day).is_some_and(|d| today - d < FORGET_DAYS)
}

/// The store's text; no store yet is an empty one.
fn read_store(store: &Path) -> anyhow::Result<String> {
    match std::fs::read_to_string(store) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        read => read.with_context(|| format!("{}", store.display())),
    }
}

/// The store is a line per mark: `YYYY-MM-DD<TAB>branch<TAB>base<TAB>hash<TAB>path`, the day
/// being when its review was last written. The path comes last, so a tab in it reads back, and
/// lines end at `\n` alone, so does a `\r` (macOS's `Icon\r`).
fn read_viewed(
    store: &Path,
    branch: &str,
    base: &str,
    today: i64,
) -> anyhow::Result<HashMap<PathBuf, u64>> {
    let text = read_store(store)?;
    let mark = |line: &str| {
        let [day, b, s, hash, path] = line.splitn(5, '\t').collect::<Vec<_>>()[..] else {
            return None;
        };
        let hash = u64::from_str_radix(hash, 16).ok()?;
        (fresh(day, today) && b == branch && s == base).then(|| (PathBuf::from(path), hash))
    };
    Ok(text.split('\n').filter_map(mark).collect())
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
    let old = read_store(store)?;
    let mut text = String::new();
    for line in old.split('\n') {
        let mut cols = line.splitn(4, '\t');
        let (Some(day), Some(b), Some(s)) = (cols.next(), cols.next(), cols.next()) else {
            continue;
        };
        if fresh(day, today) && (b, s) != (branch, base) {
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
    crate::stats::write(store, &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #240: the 30 days, on a fixed day: a review written 29 days ago is read and kept by a
    /// write, one written 30 or 31 days ago is neither.
    #[test]
    fn a_review_is_forgotten_on_its_thirtieth_day() {
        let dir = std::env::temp_dir().join(format!("merl-viewededge-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let store = dir.join("viewed");
        let today = 20_000;
        let line = |days, branch: &str| {
            let day = crate::stats::date(today - days);
            format!("{day}\t{branch}\tmain\t{:016x}\t{branch}.rs\n", 7)
        };
        let old = line(29, "recent") + &line(30, "edge") + &line(31, "stale");
        std::fs::write(&store, old).unwrap();
        let read = |branch| read_viewed(&store, branch, "main", today).unwrap();
        assert_eq!(read("recent").len(), 1);
        assert!(read("edge").is_empty() && read("stale").is_empty());
        write_viewed(&store, "feature", "main", std::iter::empty(), today).unwrap();
        assert_eq!(std::fs::read_to_string(&store).unwrap(), line(29, "recent"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
