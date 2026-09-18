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
    /// The directories of the last walk, relative to the root.
    dirs: HashSet<PathBuf>,
    /// Its directories and files.
    listed: HashSet<PathBuf>,
    changed: Debounce,
    walking: bool,
}

impl Project {
    pub fn new(root: &Path, tree: &Tree, files: &[PathBuf]) -> Self {
        let mut p = Self {
            root: root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
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
                let Ok(rel) = p.strip_prefix(&self.root) else {
                    return false;
                };
                let Some(dir) = rel.parent() else {
                    return true; // the root itself
                };
                let walked = dir.as_os_str().is_empty() || self.dirs.contains(dir);
                let name = rel.file_name().and_then(|n| n.to_str());
                walked
                    && (matches!(name, Some(".gitignore" | ".ignore"))
                        || self.listed.contains(rel) != p.symlink_metadata().is_ok())
            })
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
    /// the root recursively. inotify needs a watch per directory and a recursive watch would
    /// put one on every directory of `node_modules/`, so on Linux the directories of the walk
    /// are watched one by one. Every one of them every time: a directory deleted and made again
    /// has lost its watch, and a watch that failed (the inotify limit) deserves another try.
    pub fn watch(&self, watcher: &mut RecommendedWatcher, watched: &mut HashSet<PathBuf>) {
        if !cfg!(target_os = "linux") {
            if watched.insert(PathBuf::new()) {
                let _ = watcher.watch(&self.root, RecursiveMode::Recursive);
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
        let (tree, files) = tree::build(&dir);
        let p = Project::new(&dir, &tree, &files);
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
        // inotify overflowed: no path, something was missed.
        let ev = notify::Event::new(EventKind::Other).set_flag(Flag::Rescan);
        assert!(p.concerns(&ev));
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
        let (tree, files) = tree::build(&dir);
        p.walked(&tree, &files, ms(700));
        assert!(p.walk_due(ms(700)));
        let (tree, files) = tree::build(&dir);
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
        let (tree, files) = tree::build(&dir);
        p.walked(&tree, &files, ms(6300));
        assert!(!p.walk_due(ms(6400)) && p.walk_due(ms(6500)));
        // A walk that only lost a directory asks for nothing.
        std::fs::remove_dir_all(dir.join("services")).unwrap();
        let (tree, files) = tree::build(&dir);
        p.walked(&tree, &files, ms(6600));
        assert!(!p.walk_due(ms(9000)));
        std::fs::remove_dir_all(&dir).unwrap();
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
        let (tree, files) = tree::build(&dir);
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
