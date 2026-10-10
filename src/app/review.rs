use anyhow::Context as _;

use super::*;
use crate::reviews::{Session, Spot, Stop};

impl App {
    pub fn start_review(&mut self, mut review: git::Review) {
        let note = review.note.take();
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
        let first = (self.review.as_ref()).and_then(|r| {
            r.files
                .iter()
                .find(|f| f.is_stop(&self.root, self.diagrams.on()))
                .cloned()
        });
        if self.buf.path.is_none()
            && let Some(f) = first
        {
            self.open_review_file(&f, false);
            self.remember_hunk();
        }
        if let (Some(r), Some(rel)) = (&self.review, self.rel_current()) {
            let skipped = r
                .files
                .iter()
                .take_while(|f| !f.is_stop(&self.root, self.diagrams.on()))
                .count();
            if r.files.get(skipped).is_some_and(|f| f.path == rel) {
                self.say_skipped(skipped);
            }
        }
        if let Some(note) = note {
            self.message = note;
        }
        self.load_viewed();
        self.open_session();
    }

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

    pub(super) fn review_spot(&self) -> Spot {
        let r = self.review.as_ref();
        Spot {
            open_file: self.buf.path.clone(),
            cursor_line: self.at(),
            file_in_review: (self.rel_current())
                .is_some_and(|rel| r.is_some_and(|r| r.file(&rel).is_some())),
        }
    }

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

    pub(super) fn review_count(
        &mut self,
        at: Option<Instant>,
        from: Spot,
        action: Option<&str>,
        quit: bool,
    ) {
        let to = self.review_spot();
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

    pub fn review_row(&mut self) -> Option<(String, String, String)> {
        let s = self.session.as_mut()?;
        let columns = s.columns(self.viewed.keys())?;
        Some((s.repo.clone(), s.branch.clone(), columns))
    }

    pub fn review_rows(&mut self) -> Vec<(String, String, String)> {
        let closed = std::mem::take(&mut self.sessions_closed_by_switch_with_viewed).into_iter();
        let mut rows: Vec<_> = closed
            .filter_map(|(mut s, viewed)| {
                let columns = s.columns(viewed.iter())?;
                Some((s.repo, s.branch, columns))
            })
            .collect();
        rows.extend(self.review_row());
        rows
    }

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
        if let Some((rel, h)) = self.hunk_left(&r) {
            self.jump_to_hunk(&self.root.join(rel), h);
            return;
        }
        let mut read = self
            .rel_current()
            .filter(|rel| dir > 0 && r.file(rel).is_some());
        let (mut skipped, mut failed) = (0, None);
        for f in self.ahead(&r, dir) {
            if !f.is_stop(&self.root, self.diagrams.on()) {
                skipped += 1;
            } else if self.open_review_file(f, dir < 0) {
                self.remember_hunk();
                match failed {
                    Some(why) => self.message = why,
                    None => self.say_skipped(skipped),
                }
                self.mark_viewed(read.take());
                return;
            } else if self.dirty {
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

    pub(super) fn hunk_ahead(&self, dir: isize) -> Option<TextLine> {
        if self.folded_here().is_some() {
            return None;
        }
        let at = self.hunk_place(self.at());
        match dir > 0 {
            true => self.diff.hunks.iter().find(|&&h| h > at).copied(),
            false => self.diff.hunks.iter().rev().find(|&&h| h < at).copied(),
        }
    }

    pub(super) fn hunk_place(&self, t: TextLine) -> TextLine {
        match t {
            TextLine::Deleted(k, _) if self.diff.hunks.contains(&TextLine::File(k)) => {
                TextLine::File(k)
            }
            t => t,
        }
    }

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

    fn remember_hunk(&mut self) {
        let i = self.diff.hunks.iter().filter(|&&h| h < self.at()).count();
        self.last_hunk = self.rel_current().map(|rel| (rel, i));
    }

    // ponytail: two hunks removed at or above the one left can pass an unread hunk, and a way
    // back that fell to the file's last hunk keeps the larger index, not the one it landed on;
    // recognise the hunk by its text if agents ever rewrite that much under a reader.
    pub(super) fn hunk_left(&self, r: &git::Review) -> Option<(PathBuf, TextLine)> {
        if self.rel_current().is_some_and(|rel| r.file(&rel).is_some()) {
            return None;
        }
        let (rel, i) = self.last_hunk.clone()?;
        let f = r
            .file(&rel)
            .filter(|f| f.is_stop(&self.root, self.diagrams.on()))?;
        let hunks = review_hunks(&self.root, r, f);
        let h = hunks.get(i).or(hunks.last()).copied();
        Some((rel, h.unwrap_or(TextLine::File(0))))
    }

    pub fn folded_here(&self) -> Option<&git::ReviewFile> {
        let rel = self.rel_current()?;
        let f = self.review.as_ref()?.file(&rel)?;
        folded(f, &self.unfolded).then_some(f)
    }

    pub fn empty_note(&self) -> Option<String> {
        let rel = self.rel_current()?;
        let f = self.review.as_ref()?.file(&rel)?;
        let blank = matches!(self.buf.lines.as_slice(), [l] if l.is_empty());
        if f.has_hunks() || f.binary || !blank {
            return None;
        }
        Some(match (f.status, &f.old) {
            ('A', _) => "empty file, added".into(),
            ('D', _) => "empty file, deleted".into(),
            (_, Some(old)) => format!("empty file, renamed from {}", old.display()),
            _ => "empty file".into(),
        })
    }

    pub fn renamed_here(&self) -> Option<(&Path, &Path)> {
        let rel = self.rel_current()?;
        let f = self.review.as_ref()?.file(&rel)?;
        let blank = matches!(self.buf.lines.as_slice(), [l] if l.is_empty());
        let pure = f.status == 'R' && !f.has_hunks() && !f.binary && !self.buf.binary();
        let in_use = self.mode == Mode::Edit || self.dirty || self.previewing();
        (pure && !blank && !in_use).then_some((f.old.as_deref()?, f.path.as_path()))
    }

    pub fn picture_sides(&self) -> Option<Vec<Side>> {
        if !self.diagrams.on() {
            return None;
        }
        let rel = self.rel_current()?;
        let r = self.review.as_ref()?;
        let f = r.file(&rel).filter(|f| f.is_picture())?;
        let old = || Side {
            old: true,
            path: self.old_picture(r, f.old.as_deref().unwrap_or(&f.path)),
        };
        let new = || Side {
            old: false,
            path: Some(self.root.join(&f.path))
                .filter(|p| p.is_file())
                .ok_or_else(|| "not on disk".to_string()),
        };
        Some(match f.status {
            'A' => vec![new()],
            'D' => vec![old()],
            _ => vec![old(), new()],
        })
    }

    fn old_picture(&self, r: &git::Review, rel: &Path) -> Result<PathBuf, String> {
        let key = (r.merge_base.clone(), rel.to_path_buf());
        let mut memo = self.old_picture.borrow_mut();
        match &*memo {
            Some((k, got)) if *k == key => got.clone(),
            _ => {
                let got = (r.base_file(&self.root, rel)).map_err(|e| super::error_text(&e));
                *memo = Some((key, got.clone()));
                got
            }
        }
    }

    pub fn text_hidden(&self) -> bool {
        self.folded_here().is_some() || self.renamed_here().is_some()
    }

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

    fn mark_viewed(&mut self, rels: impl IntoIterator<Item = PathBuf>) {
        let mut any = false;
        for rel in rels {
            let hash = self.disk_hash(&rel);
            self.hidden.remove(&rel);
            self.viewed.insert(rel, hash);
            any = true;
        }
        if any {
            self.save_viewed();
        }
    }

    fn disk_hash(&self, rel: &Path) -> u64 {
        std::fs::read(self.root.join(rel)).map_or(0, |bytes| {
            let fnv = |h: u64, &b: &u8| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
            bytes.iter().fold(0xcbf2_9ce4_8422_2325, fnv)
        })
    }

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

    fn viewed_store(&self) -> Option<PathBuf> {
        git::dirs(&self.root).map(|d| d.common_refs_dir.join("merl/viewed"))
    }

    fn unfolded_store(&self) -> Option<PathBuf> {
        git::dirs(&self.root).map(|d| d.common_refs_dir.join("merl/unfolded"))
    }

    fn load_viewed(&mut self) {
        let (Some(branch), Some(r)) = (&self.viewed_branch, &self.review) else {
            return;
        };
        let read = (self.viewed_store())
            .context("no git directory")
            .and_then(|store| read_viewed(&store, branch, &r.base, crate::stats::utc_today()));
        self.viewed.clear();
        self.hidden.clear();
        self.unfolded = (self.unfolded_store())
            .and_then(|store| read_viewed(&store, branch, &r.base, crate::stats::utc_today()).ok())
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

    fn save_viewed(&mut self) {
        let (Some(store), Some(r), Some(branch)) =
            (self.viewed_store(), &self.review, &self.viewed_branch)
        else {
            return;
        };
        let marks = self.viewed.iter().chain(&self.hidden);
        if let Err(e) = write_viewed(&store, branch, &r.base, marks, crate::stats::utc_today()) {
            self.message = format!("viewed marks not saved: {}", super::error_text(&e));
        }
    }

    fn save_unfolded(&mut self) {
        let (Some(store), Some(r), Some(branch)) =
            (self.unfolded_store(), &self.review, &self.viewed_branch)
        else {
            return;
        };
        let files = self.unfolded.iter().map(|p| (p, &0));
        if let Err(e) = write_viewed(&store, branch, &r.base, files, crate::stats::utc_today()) {
            self.message = format!("unfolded files not saved: {e:#}");
        }
    }

    fn say_skipped(&mut self, skipped: usize) {
        if skipped > 0 {
            let s = plural(skipped);
            self.message = format!("skipped {skipped} file{s} without hunks");
        }
    }

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

    fn stand_on(&mut self, h: TextLine) {
        self.set_at(self.clamp_line(h));
        (self.col, self.want_x, self.center) = (0, 0, true);
    }

    pub fn review_status(&self) -> Option<String> {
        let r = self.review.as_ref()?;
        let rel = self.rel_current()?;
        let file = r.files.iter().position(|f| f.path == rel)?;
        if self.folded_here().is_some() {
            return Some(format!("folded  file {}/{}", file + 1, r.files.len()));
        }
        let at = self.hunk_place(self.at());
        let hunk = self.diff.hunks.iter().filter(|&&h| h <= at).count();
        Some(format!(
            "hunk {hunk}/{}  file {}/{}",
            self.diff.hunks.len(),
            file + 1,
            r.files.len()
        ))
    }

    pub fn review_refreshed(&mut self, fresh: git::Review) -> bool {
        let Some(old) = &self.review else {
            return false;
        };
        if *old == fresh {
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
        let shown = rel.clone().filter(|_| self.folded_here().is_none());
        self.review = Some(fresh);
        self.refresh_tree(crate::tree::from_listing(&paths));
        if stale {
            self.refresh_diff();
        }
        let closed = if switched { self.session.take() } else { None };
        let reopen = closed.is_some();
        if let Some(s) = closed {
            self.sessions_closed_by_switch_with_viewed
                .push((s, self.viewed.keys().cloned().collect()));
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

pub struct Side {
    pub old: bool,
    pub path: Result<PathBuf, String>,
}

pub(super) type OldPicture = ((String, PathBuf), Result<PathBuf, String>);

fn count_stops(
    r: &git::Review,
    root: &Path,
    unfolded: &HashSet<PathBuf>,
) -> Option<HashMap<PathBuf, usize>> {
    (r.files.iter())
        .map(|f| Some((f.path.clone(), stops_in(r, root, f, unfolded)?)))
        .collect()
}

fn folded(f: &git::ReviewFile, unfolded: &HashSet<PathBuf>) -> bool {
    f.generated && f.has_hunks() && !unfolded.contains(&f.path)
}

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

fn review_hunks(root: &Path, r: &git::Review, f: &git::ReviewFile) -> Vec<TextLine> {
    match f.status {
        'D' => vec![TextLine::File(0)],
        _ => r.diff(root, &root.join(&f.path), Some(f)).hunks,
    }
}

const FORGET_DAYS: i64 = 30;

fn fresh(day: &str, today: i64) -> bool {
    crate::stats::day(day).is_some_and(|d| today - d < FORGET_DAYS)
}

fn read_store(store: &Path) -> anyhow::Result<String> {
    match std::fs::read_to_string(store) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        read => read.with_context(|| format!("{}", store.display())),
    }
}

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
        if let Some(path) = path.to_str().filter(|p| !p.contains('\n')) {
            _ = writeln!(text, "{day}\t{branch}\t{base}\t{hash:016x}\t{path}");
        }
    }
    crate::stats::write(store, &text)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn a_path_with_a_tab_or_a_carriage_return_reads_back() {
        let dir = std::env::temp_dir().join(format!("merl-viewedtab-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let store = dir.join("viewed");
        let paths = [PathBuf::from("a\tb.rs"), PathBuf::from("Icon\r")];
        let marks: Vec<(&PathBuf, &u64)> = paths.iter().map(|p| (p, &7)).collect();
        write_viewed(&store, "feature", "main", marks.into_iter(), 20_000).unwrap();
        let read = read_viewed(&store, "feature", "main", 20_000).unwrap();
        assert_eq!(read.len(), 2);
        assert!(paths.iter().all(|p| read.get(p) == Some(&7)), "{read:?}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
