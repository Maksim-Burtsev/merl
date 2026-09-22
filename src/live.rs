//! The live project: which filesystem events change the file list, and when to walk again.
//! Nothing here touches the screen, so another list that follows the disk can reuse it.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::tree::Tree;

/// How long the events have to stand still before the refresh: an agent writing twenty files is
/// one walk.
const QUIET: Duration = Duration::from_millis(200);
/// A stream of events that never pauses still gets a refresh this often.
const MAX_WAIT: Duration = Duration::from_secs(1);

/// Collects a burst of events into one refresh.
#[derive(Default)]
pub struct Debounce {
    /// The first and the last event since the last refresh.
    pending: Option<(Instant, Instant)>,
}

impl Debounce {
    pub fn touch(&mut self, now: Instant) {
        let first = self.pending.map_or(now, |(first, _)| first);
        self.pending = Some((first, now));
    }

    /// Is a refresh due? Answers `true` once per burst.
    pub fn due(&mut self, now: Instant) -> bool {
        let due = self
            .pending
            .is_some_and(|(first, last)| now - last >= QUIET || now - first >= MAX_WAIT);
        if due {
            self.pending = None;
        }
        due
    }
}

/// The project as the last walk listed it, and when to walk it again. Owned by the event loop:
/// it feeds [`Project::event`] every filesystem event, asks [`Project::walk_due`] on every pass
/// and hands the result of the walk to [`Project::walked`].
pub struct Project {
    /// Canonical: FSEvents reports canonical paths.
    root: PathBuf,
    /// The walk lists only the files right in the root, and nothing below it is watched.
    shallow: bool,
    /// The directories of the last walk, relative to the root.
    dirs: HashSet<PathBuf>,
    /// Its directories and files.
    listed: HashSet<PathBuf>,
    changed: Debounce,
    walking: bool,
}

impl Project {
    pub fn new(root: &Path, shallow: bool, tree: &Tree, files: &[PathBuf]) -> Self {
        let mut p = Self {
            root: root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
            shallow,
            dirs: HashSet::new(),
            listed: HashSet::new(),
            changed: Debounce::default(),
            walking: false,
        };
        p.list(tree, files);
        p
    }

    fn list(&mut self, tree: &Tree, files: &[PathBuf]) {
        self.dirs = tree.dirs();
        self.listed = self.dirs.iter().chain(files).cloned().collect();
    }

    pub fn event(&mut self, ev: &notify::Event, now: Instant) {
        if self.concerns(ev) {
            self.changed.touch(now);
        }
    }

    /// Does this event change the file list? The kind does not say: FSEvents calls a write to
    /// a file created seconds ago a create. The disk does: the event counts when its path is
    /// listed and gone, or there and not listed. An event below a directory the walk did not
    /// list is inside `.git` or an ignored `target/` or `node_modules/` and never counts, so a
    /// build running next to merl costs no walk. A write to an ignore file changes what the
    /// walk lists, and a watcher that lost events asks for a rescan.
    ///
    /// ponytail: a file ignored by a pattern in a listed directory (`*.pyc` beside the source)
    /// is there and not listed, so every write to it is a walk. Match the rules if it shows.
    fn concerns(&self, ev: &notify::Event) -> bool {
        if matches!(ev.kind, EventKind::Access(_)) {
            return false;
        }
        ev.need_rescan()
            || ev.paths.iter().any(|p| {
                let Some(rel) = self.in_walk(p) else {
                    return false;
                };
                let name = rel.file_name().and_then(|n| n.to_str());
                rel.as_os_str().is_empty() // the root itself
                    || matches!(name, Some(".gitignore" | ".ignore"))
                    || self.listed.contains(rel) != p.symlink_metadata().is_ok()
            })
    }

    /// `p` relative to the root, when it is the root or in a directory the last walk listed.
    fn in_walk<'a>(&self, p: &'a Path) -> Option<&'a Path> {
        let rel = p.strip_prefix(&self.root).ok()?;
        let dir = rel.parent().unwrap_or(rel);
        (dir.as_os_str().is_empty() || self.dirs.contains(dir)).then_some(rel)
    }

    /// Did something happen where the walk lists files, a plain save included? It changes no
    /// row of the tree and may change one of the review panel. Ignored directories and `.git`
    /// stay silent, as in [`Project::concerns`].
    pub fn touched(&self, ev: &notify::Event) -> bool {
        !matches!(ev.kind, EventKind::Access(_))
            && (ev.need_rescan() || ev.paths.iter().any(|p| self.in_walk(p).is_some()))
    }

    /// Is it time to walk? One walk at a time: what changes during one is due after it.
    pub fn walk_due(&mut self, now: Instant) -> bool {
        let due = !self.walking && self.changed.due(now);
        self.walking |= due;
        due
    }

    /// The walk [`Project::walk_due`] asked for is back. Files written into a directory before
    /// it was listed (or, on Linux, watched) reported nothing: a new directory is one more walk.
    pub fn walked(&mut self, tree: &Tree, files: &[PathBuf], now: Instant) {
        self.walking = false;
        let known = std::mem::take(&mut self.dirs);
        self.list(tree, files);
        if !self.dirs.is_subset(&known) {
            self.changed.touch(now);
        }
    }

    /// Watches the project, at startup and again after every walk. FSEvents and Windows take
    /// the root recursively, a shallow one alone. inotify needs a watch per directory and a recursive watch would
    /// put one on every directory of `node_modules/`, so on Linux the directories of the walk
    /// are watched one by one. Every one of them every time: a directory deleted and made again
    /// has lost its watch, and a watch that failed (the inotify limit) deserves another try.
    pub fn watch(&self, watcher: &mut RecommendedWatcher, watched: &mut HashSet<PathBuf>) {
        if !cfg!(target_os = "linux") {
            if watched.insert(PathBuf::new()) {
                let mode = if self.shallow {
                    RecursiveMode::NonRecursive
                } else {
                    RecursiveMode::Recursive
                };
                let _ = watcher.watch(&self.root, mode);
            }
            return;
        }
        let mut want = self.dirs.clone();
        want.insert(PathBuf::new());
        // A deleted directory is an error here; one that became ignored loses its watch.
        for d in watched.difference(&want) {
            let _ = watcher.unwatch(&self.root.join(d));
        }
        for d in &want {
            let _ = watcher.watch(&self.root.join(d), RecursiveMode::NonRecursive);
        }
        *watched = want;
    }
}

/// The live review panel: when to ask git for the branch again. The files come through the
/// project watch ([`Project::touched`]); a commit, a rebase or a switch moves `HEAD` or a ref,
/// inside `.git`, which no walk lists and which Linux therefore does not watch.
pub struct Review {
    /// Canonical. `HEAD` is the worktree's own; the refs belong to the repository.
    head: PathBuf,
    refs: PathBuf,
    packed_refs: PathBuf,
    /// Where a repository made with `--ref-format=reftable` keeps its refs: `HEAD` and
    /// `refs/` are placeholders there and never change.
    reftables: [PathBuf; 2],
    changed: Debounce,
    listing: bool,
}

impl Review {
    /// `git_dir` and `common_dir` as `git::dirs` answers: one directory, or two in a worktree.
    pub fn new(git_dir: &Path, common_dir: &Path) -> Self {
        Self {
            head: git_dir.join("HEAD"),
            refs: common_dir.join("refs"),
            packed_refs: common_dir.join("packed-refs"),
            reftables: [git_dir.join("reftable"), common_dir.join("reftable")],
            changed: Debounce::default(),
            listing: false,
        }
    }

    /// `in_project`: what [`Project::touched`] said of this event.
    pub fn event(&mut self, ev: &notify::Event, in_project: bool, now: Instant) {
        if in_project || self.moves_the_branch(ev) {
            self.changed.touch(now);
        }
    }

    /// The project was walked again: files may have been written before their directory was
    /// listed, and those events did not count.
    pub fn touch(&mut self, now: Instant) {
        self.changed.touch(now);
    }

    /// `HEAD`, a loose ref under `refs/`, `packed-refs` or a reftable was written (git renames
    /// a `.lock` over them). The index and the objects are not the branch: merl's own `git diff` and an
    /// editor's `git status` refresh the index, and must not ask for another listing.
    fn moves_the_branch(&self, ev: &notify::Event) -> bool {
        let lock = |p: &Path| p.extension().is_some_and(|e| e == "lock");
        !matches!(ev.kind, EventKind::Access(_))
            && ev.paths.iter().any(|p| {
                let store = [&self.refs, &self.reftables[0], &self.reftables[1]];
                *p == self.head
                    || *p == self.packed_refs
                    || !lock(p) && store.iter().any(|dir| p.starts_with(dir))
            })
    }

    /// Is it time to list the branch? One listing at a time, as with the walk.
    pub fn list_due(&mut self, now: Instant) -> bool {
        let due = !self.listing && self.changed.due(now);
        self.listing |= due;
        due
    }

    pub fn listed(&mut self) {
        self.listing = false;
    }

    /// `HEAD` and `packed-refs` are replaced by a rename, so their directories are watched,
    /// not they. Needed on Linux, and everywhere in a worktree, whose `.git` is a file and the
    /// real directory is outside the root; a second watch of what FSEvents already reports
    /// under the root costs a repeated event, which the debounce takes.
    pub fn watch(&self, watcher: &mut RecommendedWatcher) {
        for (path, mode) in [
            (self.head.parent(), RecursiveMode::NonRecursive),
            (self.packed_refs.parent(), RecursiveMode::NonRecursive),
            (Some(self.refs.as_path()), RecursiveMode::Recursive),
            // Not there with the files backend: the error is the answer.
            (
                Some(self.reftables[0].as_path()),
                RecursiveMode::NonRecursive,
            ),
            (
                Some(self.reftables[1].as_path()),
                RecursiveMode::NonRecursive,
            ),
        ] {
            let _ = watcher.watch(path.unwrap_or(Path::new("")), mode);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree;
    use notify::event::{CreateKind, DataChange, Flag, ModifyKind, RemoveKind, RenameMode};

    #[test]
    fn a_burst_of_events_is_one_refresh() {
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let mut d = Debounce::default();
        assert!(!d.due(ms(0)), "nothing happened");
        // Ten files, 50 ms apart: not due while they keep coming.
        for i in 0..10 {
            d.touch(ms(i * 50));
            assert!(!d.due(ms(i * 50 + 49)));
        }
        assert!(!d.due(ms(450 + 199)));
        assert!(d.due(ms(450 + 200)));
        assert!(!d.due(ms(5000)), "once per burst");
        // Events that never pause are answered after MAX_WAIT.
        let mut d = Debounce::default();
        let fired = (0..40).find(|i| {
            d.touch(ms(i * 50));
            d.due(ms(i * 50 + 1))
        });
        assert_eq!(fired, Some(20));
    }

    /// A project on disk, walked: `src/app.rs`, `src/deep/x.rs`, an ignored `target/`.
    fn project(tag: &str) -> (PathBuf, Project) {
        let dir = std::env::temp_dir().join(format!("merl-watch-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for f in [
            "src/app.rs",
            "src/deep/x.rs",
            "target/debug/merl",
            ".git/index",
        ] {
            std::fs::create_dir_all(dir.join(f).parent().unwrap()).unwrap();
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        std::fs::write(dir.join(".gitignore"), "target/\n").unwrap();
        let dir = dir.canonicalize().unwrap();
        let (tree, files) = tree::build(&dir, false);
        let p = Project::new(&dir, false, &tree, &files);
        (dir, p)
    }

    #[test]
    fn only_events_that_change_the_list_ask_for_a_walk() {
        let (dir, p) = project("events");
        for f in [
            "new.rs",
            "src/deep/new.rs",
            "target/new",
            "node_modules/a/index.js",
        ] {
            std::fs::create_dir_all(dir.join(f).parent().unwrap()).unwrap();
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        std::fs::remove_file(dir.join("src/app.rs")).unwrap();
        let create = EventKind::Create(CreateKind::File);
        let remove = EventKind::Remove(RemoveKind::Any);
        let rename = EventKind::Modify(ModifyKind::Name(RenameMode::Any));
        let write = EventKind::Modify(ModifyKind::Data(DataChange::Any));
        let access = EventKind::Access(notify::event::AccessKind::Any);
        for (kind, paths, want) in [
            (create, vec!["new.rs"], true),
            (create, vec!["node_modules"], true),
            (write, vec!["src/deep/new.rs"], true), // whatever the kind says
            (remove, vec!["src/app.rs"], true),
            (rename, vec!["src/app.rs"], true),
            (EventKind::Any, vec![""], true),
            // A directory the walk did not list: ignored, or `.git`.
            (create, vec!["node_modules/a/index.js"], false),
            (create, vec!["target/new"], false),
            (rename, vec![".git/index"], false),
            // Both ends of a rename in one event: one of them is enough.
            (rename, vec!["target/new", "new.rs"], true),
            (rename, vec!["target/new", ".git/index"], false),
            // A save changes no row, even when FSEvents calls it a create; an ignore file
            // changes what is listed.
            (write, vec!["src/deep/x.rs"], false),
            (create, vec!["src/deep/x.rs"], false),
            (write, vec![".gitignore"], true),
            (write, vec!["src/.ignore"], true),
            (write, vec!["target/.gitignore"], false),
            (access, vec!["new.rs"], false),
            (create, vec!["../elsewhere.rs"], false),
        ] {
            let mut ev = notify::Event::new(kind);
            for path in &paths {
                ev = ev.add_path(dir.join(path));
            }
            assert_eq!(p.concerns(&ev), want, "{kind:?} {paths:?}");
        }
        // The review panel counts a plain save too, in the same directories.
        for (kind, path, want) in [
            (write, "src/deep/x.rs", true),
            (create, "new.rs", true),
            (write, "target/new", false),
            (rename, ".git/index", false),
            (access, "src/deep/x.rs", false),
            (write, "../elsewhere.rs", false),
        ] {
            let ev = notify::Event::new(kind).add_path(dir.join(path));
            assert_eq!(p.touched(&ev), want, "{kind:?} {path}");
        }
        // inotify overflowed: no path, something was missed.
        let ev = notify::Event::new(EventKind::Other).set_flag(Flag::Rescan);
        assert!(p.concerns(&ev) && p.touched(&ev));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn one_walk_at_a_time_and_one_more_for_a_new_directory() {
        let (dir, mut p) = project("walks");
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let create = |p: &mut Project, f: &str, at| {
            std::fs::create_dir_all(dir.join(f).parent().unwrap()).unwrap();
            std::fs::write(dir.join(f), b"x").unwrap();
            let ev = notify::Event::new(EventKind::Create(CreateKind::File));
            p.event(&ev.add_path(dir.join(f)), at);
        };
        assert!(!p.walk_due(ms(0)));
        create(&mut p, "a.rs", ms(0));
        assert!(!p.walk_due(ms(100)) && p.walk_due(ms(200)));
        // Changes during the walk wait for it, and are not lost.
        create(&mut p, "b.rs", ms(250));
        assert!(!p.walk_due(ms(600)));
        let (tree, files) = tree::build(&dir, false);
        p.walked(&tree, &files, ms(700));
        assert!(p.walk_due(ms(700)));
        let (tree, files) = tree::build(&dir, false);
        p.walked(&tree, &files, ms(800));
        assert!(
            !p.walk_due(ms(5000)),
            "no new directory: nothing more to find"
        );

        // `services/` is reported, the file written into it right after is not: its
        // directory was not listed yet. The walk that finds `services/` asks for one more.
        std::fs::create_dir_all(dir.join("services")).unwrap();
        let ev = notify::Event::new(EventKind::Create(CreateKind::Folder));
        p.event(&ev.add_path(dir.join("services")), ms(6000));
        assert!(p.walk_due(ms(6200)));
        let (tree, files) = tree::build(&dir, false);
        p.walked(&tree, &files, ms(6300));
        assert!(!p.walk_due(ms(6400)) && p.walk_due(ms(6500)));
        // A walk that only lost a directory asks for nothing.
        std::fs::remove_dir_all(dir.join("services")).unwrap();
        let (tree, files) = tree::build(&dir, false);
        p.walked(&tree, &files, ms(6600));
        assert!(!p.walk_due(ms(9000)));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_commit_asks_for_a_listing_and_the_index_does_not() {
        // A linked worktree: `HEAD` is its own, the refs are the repository's.
        let common = Path::new("/repo/.git");
        let mut r = Review::new(&common.join("worktrees/wt"), common);
        let rename = EventKind::Modify(ModifyKind::Name(RenameMode::Any));
        for (path, want) in [
            ("worktrees/wt/HEAD", true),
            ("refs/heads/feature", true),
            ("refs/heads/feat/live", true),
            ("refs/remotes/origin/master", true),
            ("packed-refs", true),
            ("reftable/tables.list", true),
            (
                "worktrees/wt/reftable/0x000000000002-0x000000000002-a1b2c3d4.ref",
                true,
            ),
            ("reftable/tables.list.lock", false),
            ("worktrees/wt/HEAD.lock", false),
            ("refs/heads/feature.lock", false),
            ("HEAD", false), // of the main checkout
            ("worktrees/wt/index", false),
            ("worktrees/wt/ORIG_HEAD", false),
            ("objects/ab/cdef", false),
        ] {
            let ev = notify::Event::new(rename).add_path(common.join(path));
            assert_eq!(r.moves_the_branch(&ev), want, "{path}");
        }
        let read = notify::Event::new(EventKind::Access(notify::event::AccessKind::Any));
        assert!(!r.moves_the_branch(&read.add_path(common.join("worktrees/wt/HEAD"))));
        // Debounced, and one listing at a time.
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let head = notify::Event::new(rename).add_path(common.join("worktrees/wt/HEAD"));
        let index = notify::Event::new(rename).add_path(common.join("worktrees/wt/index"));
        r.event(&index, false, ms(0));
        assert!(!r.list_due(ms(1000)));
        r.event(&head, false, ms(1000));
        r.event(&index, true, ms(1100)); // a file of the project
        assert!(!r.list_due(ms(1200)) && r.list_due(ms(1300)));
        r.touch(ms(1400));
        assert!(!r.list_due(ms(2000)), "the first one is not back");
        r.listed();
        assert!(r.list_due(ms(2000)) && !r.list_due(ms(9000)));
    }

    /// inotify drops the watch of a deleted directory by itself; the one made again under the
    /// same name has to be watched again.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_directory_deleted_and_made_again_is_watched_again() {
        let (dir, mut p) = project("rewatch");
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |ev: notify::Result<notify::Event>| {
            if let Ok(ev) = ev {
                let _ = tx.send(ev);
            }
        })
        .unwrap();
        let mut watched = HashSet::new();
        p.watch(&mut watcher, &mut watched);
        std::fs::remove_dir_all(dir.join("src/deep")).unwrap();
        std::fs::create_dir_all(dir.join("src/deep")).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        let (tree, files) = tree::build(&dir, false);
        p.walked(&tree, &files, Instant::now());
        p.watch(&mut watcher, &mut watched);
        while rx.try_recv().is_ok() {}
        std::fs::write(dir.join("src/deep/again.rs"), b"x").unwrap();
        let seen = std::iter::from_fn(|| rx.recv_timeout(Duration::from_secs(5)).ok())
            .any(|ev| ev.paths.iter().any(|f| f.ends_with("src/deep/again.rs")));
        assert!(seen, "no event from the directory that was made again");
        // An ignored directory is not watched.
        std::fs::write(dir.join("target/debug/new"), b"x").unwrap();
        let seen = std::iter::from_fn(|| rx.recv_timeout(Duration::from_millis(500)).ok())
            .any(|ev| ev.paths.iter().any(|f| f.starts_with(dir.join("target"))));
        assert!(!seen);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
