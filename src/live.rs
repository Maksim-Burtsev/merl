//! The live project: which filesystem events change the file list, and when to walk again.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use notify::event::ModifyKind;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

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

/// Does this event change the file list? `dirs` are the directories of the last walk, relative
/// to `root`: an event below any other directory is inside `.git` or an ignored `target/` or
/// `node_modules/`, and a build running next to merl must not cost a walk per file. A write
/// counts only for an ignore file, which changes what the walk lists.
pub fn changes_project(root: &Path, dirs: &HashSet<PathBuf>, ev: &notify::Event) -> bool {
    let listing = match ev.kind {
        EventKind::Access(_) => return false,
        EventKind::Modify(ModifyKind::Name(_)) => true,
        EventKind::Modify(_) => false,
        _ => true,
    };
    ev.paths.iter().any(|p| {
        let Ok(rel) = p.strip_prefix(root) else {
            return false;
        };
        // The root itself: FSEvents asking for a rescan.
        let walked = rel
            .parent()
            .is_none_or(|d| d.as_os_str().is_empty() || dirs.contains(d));
        let name = rel.file_name().and_then(|n| n.to_str());
        walked && (listing || matches!(name, Some(".gitignore" | ".ignore")))
    })
}

/// Watches the project. FSEvents and Windows take the root recursively. inotify needs a watch
/// per directory and a recursive watch would put one on every directory of `node_modules/`, so
/// on Linux the directories of the walk are watched one by one, again after every walk.
pub fn watch(
    watcher: &mut RecommendedWatcher,
    root: &Path,
    dirs: &HashSet<PathBuf>,
    watched: &mut HashSet<PathBuf>,
) {
    if !cfg!(target_os = "linux") {
        if watched.insert(PathBuf::new()) {
            let _ = watcher.watch(root, RecursiveMode::Recursive);
        }
        return;
    }
    let mut want = dirs.clone();
    want.insert(PathBuf::new());
    // A failing watch (the inotify limit) only costs the refresh below that directory. A deleted
    // directory has lost its watch already; one that became ignored loses it here.
    for d in watched.difference(&want) {
        let _ = watcher.unwatch(&root.join(d));
    }
    for d in want.difference(watched) {
        let _ = watcher.watch(&root.join(d), RecursiveMode::NonRecursive);
    }
    *watched = want;
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, DataChange, RemoveKind, RenameMode};

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

    #[test]
    fn only_events_that_change_the_list_ask_for_a_walk() {
        let root = Path::new("/p");
        let dirs: HashSet<PathBuf> = ["src", "src/deep"].into_iter().map(Into::into).collect();
        let create = EventKind::Create(CreateKind::File);
        let remove = EventKind::Remove(RemoveKind::Any);
        let rename = EventKind::Modify(ModifyKind::Name(RenameMode::Any));
        let write = EventKind::Modify(ModifyKind::Data(DataChange::Any));
        let access = EventKind::Access(notify::event::AccessKind::Any);
        for (kind, path, want) in [
            (create, "/p/new.rs", true),
            (create, "/p/services", true),
            (create, "/p/src/deep/x.rs", true),
            (remove, "/p/src/app.rs", true),
            (rename, "/p/src/app.rs", true),
            (EventKind::Any, "/p", true),
            // A directory the walk did not list: ignored, or `.git`.
            (create, "/p/node_modules/a/index.js", false),
            (create, "/p/target/debug/merl", false),
            (rename, "/p/.git/index.lock", false),
            // A save changes no row, an ignore file changes what is listed.
            (write, "/p/src/app.rs", false),
            (write, "/p/.gitignore", true),
            (write, "/p/src/.ignore", true),
            (write, "/p/target/.gitignore", false),
            (access, "/p/src/app.rs", false),
            (create, "/elsewhere/x.rs", false),
        ] {
            let ev = notify::Event::new(kind).add_path(path.into());
            assert_eq!(changes_project(root, &dirs, &ev), want, "{kind:?} {path}");
        }
    }
}
