//! Review mode.

use super::*;

#[test]
fn review_walks_past_files_without_hunks() {
    let png: &[u8] = b"\x89PNG\0\0";
    let (dir, mut a) = review_app_with("reviewbin", &[("a.png", png), ("z.png", png)]);
    // Panel order: src/a.rs, a.png, crlf.txt, gone, new, tail, z.png.
    let r = a.review.as_ref().unwrap();
    assert!(r.files[1].binary && !r.files[1].has_hunks());
    assert!(!r.files[2].binary && r.files[2].has_hunks());
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("tail"), 0));
    // Only z.png is left: not a stop.
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("tail"), 0));
    assert_eq!(a.message, "last hunk of the review");
    for _ in 0..3 {
        press(&mut a, KeyCode::Char('C'), KeyModifiers::NONE);
    }
    assert_eq!(at(&a), (dir.join("crlf.txt"), 1));
    press(&mut a, KeyCode::Char('C'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("src/a.rs"), 5));
    assert_eq!(a.message, "skipped 1 file without hunks");
    assert_eq!(a.review_status().unwrap(), "hunk 2/2  file 1/7");
    let _ = std::fs::remove_dir_all(dir);
}

/// #162: `c` leaving a file forward marks it as viewed, the last file on the press that has
/// nowhere to go; `C` marks nothing, `m` toggles, and a changed file loses its tick.
#[test]
fn viewed_marks_follow_the_walk_the_key_and_the_disk() {
    let (dir, mut a) = review_app("viewed");
    let c = |a: &mut App| press(a, KeyCode::Char('c'), KeyModifiers::NONE);
    let viewed = |a: &App| {
        let mut v: Vec<_> = a.viewed.keys().cloned().collect();
        v.sort();
        v
    };
    // The review opens on `new`; `C` walks back to the first hunk and marks nothing.
    while a.message != "first hunk of the review" {
        press(&mut a, KeyCode::Char('C'), KeyModifiers::NONE);
    }
    a.message.clear();
    while at(&a).0 == dir.join("src/a.rs") {
        assert!(a.viewed.is_empty(), "still inside the file");
        c(&mut a);
    }
    assert_eq!(viewed(&a), [PathBuf::from("src/a.rs")]);
    assert_eq!(a.message, "", "the walk marks in silence");
    press(&mut a, KeyCode::Char('C'), KeyModifiers::NONE);
    assert_eq!(
        at(&a).0,
        dir.join("src/a.rs"),
        "a viewed file is still a stop"
    );
    assert_eq!(viewed(&a), [PathBuf::from("src/a.rs")], "`C` marks nothing");

    while a.message != "last hunk of the review" {
        c(&mut a);
    }
    assert_eq!(at(&a).0, dir.join("tail"));
    assert_eq!(viewed(&a).len(), a.review.as_ref().unwrap().files.len());

    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    assert_eq!(a.message, "not viewed");
    assert!(!a.viewed.contains_key(Path::new("tail")));
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    assert_eq!(a.message, "viewed");
    // #240: FNV-1a of `t1\n`. The hashes are kept on disk: a new one takes every kept tick off.
    assert_eq!(a.viewed[Path::new("tail")], 0x5634_7619_43bd_e994);
    // The panel's row, not the open file; a directory is not a file of the review.
    a.focus = Focus::Tree;
    a.tree.reveal(Path::new("new"));
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    assert!(!a.viewed.contains_key(Path::new("new")) && a.viewed.contains_key(Path::new("tail")));
    a.tree.reveal(Path::new("src"));
    a.message.clear();
    let before = viewed(&a);
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    assert_eq!((viewed(&a), a.message.as_str()), (before, ""));

    // The agent rewrites a viewed line: the counts are the same, the content is not.
    let text = std::fs::read_to_string(dir.join("tail")).unwrap();
    std::fs::write(dir.join("tail"), text.to_uppercase()).unwrap();
    let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    assert!(a.review_refreshed(fresh), "the tick goes");
    assert!(!a.viewed.contains_key(Path::new("tail")));
    assert!(
        a.hidden.contains_key(Path::new("tail")),
        "#240: viewed before"
    );
    assert!(a.viewed.contains_key(Path::new("src/a.rs")));
    let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    assert!(!a.review_refreshed(fresh), "nothing moved");
    assert!(
        a.hidden.contains_key(Path::new("tail")),
        "no tick until viewed again"
    );
    // Written back as it was viewed, it is viewed again; rewritten, no tick again.
    std::fs::write(dir.join("tail"), &text).unwrap();
    let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    assert!(a.review_refreshed(fresh));
    assert!(a.viewed.contains_key(Path::new("tail")) && a.hidden.is_empty());
    std::fs::write(dir.join("tail"), text.to_uppercase()).unwrap();
    let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    assert!(a.review_refreshed(fresh));
    assert!(a.hidden.contains_key(Path::new("tail")));
    // Viewed again, it has its tick.
    a.tree.reveal(Path::new("tail"));
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    assert!(a.viewed.contains_key(Path::new("tail")) && a.hidden.is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

/// The files viewed and those with a hidden mark, sorted.
fn marks(a: &App) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let sorted = |m: &HashMap<PathBuf, u64>| {
        let mut v: Vec<_> = m.keys().cloned().collect();
        v.sort();
        v
    };
    (sorted(&a.viewed), sorted(&a.hidden))
}

/// #240: the marks outlive the session, per branch and base. On the next start a file changed
/// since has no tick, and does not get it back from a mark written for another file; an
/// unchanged one keeps its tick.
#[test]
fn viewed_marks_outlive_the_session_and_a_changed_file_loses_its_tick() {
    let (dir, mut a) = review_app("viewedkept");
    let git = |args: &[&str]| {
        let mut cmd = std::process::Command::new("git");
        let out = cmd.arg("-C").arg(&dir).args(args).output().unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    let paths = |p: &[&str]| p.iter().map(PathBuf::from).collect::<Vec<_>>();
    a.focus = Focus::Tree;
    for f in ["tail", "src/a.rs"] {
        a.tree.reveal(Path::new(f));
        press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    }
    let a = review_start(&dir, None);
    assert_eq!(marks(&a), (paths(&["src/a.rs", "tail"]), vec![]));
    // The same branch against another base is another review.
    git(&["branch", "dev", "main"]);
    let a = review_start(&dir, Some("dev"));
    assert_eq!(marks(&a), (vec![], vec![]));

    // Between two starts the agent rewrites `tail` and commits.
    std::fs::write(dir.join("tail"), "t1\nfixed\n").unwrap();
    git(&["commit", "-qam", "fix"]);
    let mut a = review_start(&dir, None);
    assert_eq!(marks(&a), (paths(&["src/a.rs"]), paths(&["tail"])));
    // A mark written for another file does not bring back the tick at the next start.
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    let mut a = review_start(&dir, None);
    assert_eq!(marks(&a), (paths(&["new", "src/a.rs"]), paths(&["tail"])));
    // Viewed again, `tail` is viewed at what it is now.
    a.focus = Focus::Tree;
    a.tree.reveal(Path::new("tail"));
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    let mut a = review_start(&dir, None);
    let all = (paths(&["new", "src/a.rs", "tail"]), vec![]);
    assert_eq!(marks(&a), all);
    // Another branch checked out under a live review is another review, and back is this one.
    // The listing is the one reading of HEAD: taken on `other`, it is `other`'s review, even
    // with HEAD back on `feature` by the time it arrives, and the next listing brings `feature`.
    let refresh = |a: &mut App| {
        let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
        a.review_refreshed(fresh)
    };
    git(&["switch", "-qc", "other"]);
    let listed = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    git(&["switch", "-q", "feature"]);
    a.review_refreshed(listed);
    assert_eq!(a.review.as_ref().unwrap().branch, "other");
    assert_eq!(marks(&a), (vec![], vec![]));
    refresh(&mut a);
    assert_eq!(marks(&a), all);
    // A tick taken off stays off.
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    assert_eq!(a.message, "not viewed");
    let a = review_start(&dir, None);
    assert_eq!(marks(&a), (paths(&["src/a.rs", "tail"]), vec![]));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_agents_order_moves_the_files_and_leaves_the_viewed_marks_alone() {
    let (dir, mut a) = review_app("ordermarks");
    let git = |args: &[&str]| {
        let mut cmd = std::process::Command::new("git");
        let out = cmd.arg("-C").arg(&dir).args(args).output().unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    let paths = |p: &[&str]| p.iter().map(PathBuf::from).collect::<Vec<_>>();
    let rows = |a: &App| {
        (a.tree.visible().iter())
            .map(|&i| a.tree.nodes[i].path.display().to_string())
            .collect::<Vec<_>>()
    };
    let listed = |a: &App| {
        a.review
            .as_ref()
            .unwrap()
            .files
            .iter()
            .map(|f| f.path.clone())
            .collect::<Vec<_>>()
    };
    a.focus = Focus::Tree;
    for f in ["tail", "src/a.rs"] {
        a.tree.reveal(Path::new(f));
        press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    }
    let order = dir.join(".git/merl/review/feature");
    std::fs::create_dir_all(order.parent().unwrap()).unwrap();
    std::fs::write(&order, "# commit 1\ntail\nsrc/a.rs\nnew\n").unwrap();
    let mut a = review_start(&dir, None);
    assert_eq!(
        listed(&a),
        paths(&["tail", "src/a.rs", "new", "crlf.txt", "gone"])
    );
    assert_eq!(
        rows(&a),
        ["tail", "src", "src/a.rs", "new", "crlf.txt", "gone"]
    );
    std::fs::write(&order, "new\ncrlf.txt\n").unwrap();
    let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    assert!(a.review_refreshed(fresh));
    assert_eq!(
        rows(&a),
        ["new", "crlf.txt", "src", "src/a.rs", "gone", "tail"]
    );
    assert_eq!(marks(&a), (paths(&["src/a.rs", "tail"]), vec![]));

    std::fs::write(dir.join("tail"), "t1\nfixed\n").unwrap();
    std::fs::write(dir.join("added.rs"), "fn added() {}\n").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-qm", "fix"]);
    std::fs::write(&order, "# commit 2\nadded.rs\nsrc/a.rs\nnew\ntail\n").unwrap();
    let a = review_start(&dir, None);
    assert_eq!(
        listed(&a),
        paths(&["added.rs", "src/a.rs", "new", "tail", "crlf.txt", "gone"])
    );
    assert_eq!(marks(&a), (paths(&["src/a.rs"]), paths(&["tail"])));
    let _ = std::fs::remove_dir_all(dir);
}

/// #240: the marks are a branch's, never `HEAD`'s. A tag named like the branch changes nothing;
/// a live review keeps its ticks through a detached HEAD, and a mark made then is the branch's;
/// a review started detached keeps its marks in memory for its whole life, and reads and writes
/// nothing, not under a branch checked out later either.
#[test]
fn viewed_marks_are_the_branch_s_through_a_detached_head_and_a_namesake_tag() {
    let (dir, mut a) = review_app("viewedhead");
    let git = |args: &[&str]| {
        let mut cmd = std::process::Command::new("git");
        let out = cmd.arg("-C").arg(&dir).args(args).output().unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    let refresh = |a: &mut App| {
        let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
        a.review_refreshed(fresh)
    };
    let paths = |p: &[&str]| p.iter().map(PathBuf::from).collect::<Vec<_>>();
    a.focus = Focus::Tree;
    for f in ["tail", "src/a.rs"] {
        a.tree.reveal(Path::new(f));
        press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    }
    a.focus = Focus::Code;

    // `git rev-parse --abbrev-ref HEAD` reads `heads/feature` once a tag has the name.
    git(&["tag", "feature"]);
    refresh(&mut a);
    assert_eq!(a.review.as_ref().unwrap().branch, "feature");
    assert_eq!(marks(&a).0, paths(&["src/a.rs", "tail"]));
    let mut a = review_start(&dir, None);
    assert_eq!(marks(&a).0, paths(&["src/a.rs", "tail"]));

    git(&["switch", "-q", "--detach"]);
    assert!(refresh(&mut a));
    assert_eq!(
        marks(&a).0,
        paths(&["src/a.rs", "tail"]),
        "the ticks stay while detached"
    );
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    git(&["switch", "-q", "feature"]);
    refresh(&mut a);
    let all = paths(&["new", "src/a.rs", "tail"]);
    assert_eq!(marks(&a).0, all);
    assert_eq!(marks(&review_start(&dir, None)).0, all);

    // Started detached, on a commit of no branch: nothing read, nothing written.
    let store = dir.join(".git/merl/viewed");
    let kept = std::fs::read_to_string(&store).unwrap();
    git(&["switch", "-q", "--detach"]);
    std::fs::write(dir.join("src/a.rs"), "elsewhere\n").unwrap();
    git(&["commit", "-qam", "elsewhere"]);
    let mut a = review_start(&dir, None);
    assert_eq!(a.review.as_ref().unwrap().branch, "HEAD");
    assert_eq!(marks(&a), (vec![], vec![]));
    a.focus = Focus::Tree;
    a.tree.reveal(Path::new("src/a.rs"));
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    assert_eq!(marks(&a).0, paths(&["src/a.rs"]), "in memory");
    // `main`, then `feature` checked out: neither gets the session's mark, nor gives its own.
    for branch in ["main", "feature"] {
        git(&["switch", "-q", branch]);
        refresh(&mut a);
        assert_eq!(std::fs::read_to_string(&store).unwrap(), kept, "{branch}");
    }
    assert_eq!(
        marks(&a),
        (vec![], paths(&["src/a.rs"])),
        "`elsewhere` was viewed"
    );
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    assert_eq!(std::fs::read_to_string(&store).unwrap(), kept);
    let _ = std::fs::remove_dir_all(dir);
}

/// #240: a mark that cannot be saved says so, over the walk's word and over `m`'s, and a store
/// that cannot be read is left as it is.
#[test]
fn a_viewed_mark_that_cannot_be_saved_says_so() {
    let png: &[u8] = b"\x89PNG\0\0";
    let (dir, mut a) = review_app_with("viewedunsaved", &[("o.png", png)]);
    // After the start, another hand leaves a store that is not UTF-8: it cannot be read, so it
    // is not written.
    let store = dir.join(".git/merl/viewed");
    std::fs::create_dir_all(store.parent().unwrap()).unwrap();
    std::fs::write(&store, b"\xff\n").unwrap();
    // From `new`, `c` skips `o.png` for `tail` (`skipped 1 file…`), then leaves `tail`, the
    // last one (`last hunk…`).
    for _ in 0..2 {
        a.message.clear();
        press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
        assert!(
            a.message.starts_with("viewed marks not saved"),
            "{}",
            a.message
        );
    }
    assert_eq!(std::fs::read(&store).unwrap(), b"\xff\n");
    // The marks it could not save stay for the session: a new listing on the same branch does
    // not read the store again.
    std::fs::write(dir.join("zz.txt"), "z\n").unwrap();
    let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    a.review_refreshed(fresh);
    let both = vec![PathBuf::from("new"), PathBuf::from("tail")];
    assert_eq!(marks(&a).0, both);
    for _ in 0..2 {
        press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
        assert!(
            a.message.starts_with("viewed marks not saved"),
            "{}",
            a.message
        );
    }
    assert_eq!(std::fs::read(&store).unwrap(), b"\xff\n");
    let _ = std::fs::remove_dir_all(dir);
}

/// #516: a viewed store merl cannot read or write says why in a few words, never with the OS
/// text's `(os error N)`: here the store's path is a folder.
#[test]
fn a_viewed_store_that_fails_says_why_in_a_few_words() {
    let (dir, _) = review_app("viewedio");
    let store = dir.join(".git/merl/viewed");
    std::fs::create_dir_all(&store).unwrap();
    let a = review_start(&dir, None);
    assert!(
        a.message.starts_with("viewed marks not read: ") && a.message.ends_with(": is a directory"),
        "{}",
        a.message
    );
    // A store read at the start, then turned into a folder, fails at the first mark.
    let _ = std::fs::remove_dir_all(dir);
    let (dir, mut a) = review_app("viewedio2");
    std::fs::create_dir_all(dir.join(".git/merl/viewed")).unwrap();
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    assert!(
        a.message.starts_with("viewed marks not saved: ")
            && a.message.ends_with(": is a directory"),
        "{}",
        a.message
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// #240: a store that cannot be read when the review starts gives no marks and says so once;
/// the review keeps its marks in memory and never writes over the store.
#[test]
fn a_viewed_store_that_cannot_be_read_is_not_taken_for_an_empty_one() {
    let (dir, _) = review_app("viewedunread");
    let store = dir.join(".git/merl/viewed");
    std::fs::create_dir_all(store.parent().unwrap()).unwrap();
    std::fs::write(&store, b"\xff\n").unwrap();
    let mut a = review_start(&dir, None);
    assert!(
        a.message.starts_with("viewed marks not read: "),
        "{}",
        a.message
    );
    assert_eq!(marks(&a), (vec![], vec![]));
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    assert_eq!(a.message, "viewed", "said once");
    std::fs::write(dir.join("zz.txt"), "z\n").unwrap();
    let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    a.review_refreshed(fresh);
    assert_eq!(marks(&a).0, [PathBuf::from("new")]);
    assert_eq!(std::fs::read(&store).unwrap(), b"\xff\n");
    let _ = std::fs::remove_dir_all(dir);
}

/// #240: on a refresh where git cannot say where the store is, the key stays and nothing is
/// written; the next listing takes the branch checked out.
#[test]
fn a_refresh_that_cannot_find_the_store_keeps_the_key() {
    let (dir, mut a) = review_app("viewedgitless");
    let git = |args: &[&str]| {
        let mut cmd = std::process::Command::new("git");
        let out = cmd.arg("-C").arg(&dir).args(args).output().unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    git(&["switch", "-qc", "other"]);
    let listed = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    let (real, hidden) = (dir.join(".git"), dir.join(".git-hidden"));
    std::fs::rename(&real, &hidden).unwrap();
    a.review_refreshed(listed);
    std::fs::rename(&hidden, &real).unwrap();
    assert_eq!(marks(&a).0, [PathBuf::from("new")], "still `feature`'s");
    let store = std::fs::read_to_string(dir.join(".git/merl/viewed")).unwrap();
    assert!(!store.contains("\tother\t"), "{store}");
    let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    a.review_refreshed(fresh);
    assert_eq!(marks(&a), (vec![], vec![]), "`other`'s");
    a.focus = Focus::Tree;
    a.tree.reveal(Path::new("tail"));
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    let store = std::fs::read_to_string(dir.join(".git/merl/viewed")).unwrap();
    let other: Vec<_> = store.lines().filter(|l| l.contains("\tother\t")).collect();
    assert!(other.len() == 1 && other[0].ends_with("\ttail"), "{store}");
    let _ = std::fs::remove_dir_all(dir);
}

/// #240: a name that holds a newline is not written, rather than read back as another file.
#[test]
fn a_name_with_a_newline_is_not_kept_as_another_file() {
    let (dir, mut a) = review_app_with("viewednl", &[("new\nx", b"x\n")]);
    a.focus = Focus::Tree;
    for f in ["new\nx", "tail"] {
        a.tree.reveal(Path::new(f));
        press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    }
    let a = review_start(&dir, None);
    assert_eq!(marks(&a), (vec![PathBuf::from("tail")], vec![]));
    let _ = std::fs::remove_dir_all(dir);
}

/// #240: a start that changes no mark leaves the store as it is, so a review opened and not
/// marked does not start its 30 days again.
#[test]
fn a_start_leaves_the_viewed_store_as_it_is() {
    let (dir, mut a) = review_app("viewedstart");
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    let store = dir.join(".git/merl/viewed");
    let today = crate::stats::today();
    let text = std::fs::read_to_string(&store).unwrap();
    // As if written ten days ago.
    let old = text.replace(&crate::stats::date(today), &crate::stats::date(today - 10));
    std::fs::write(&store, &old).unwrap();
    let a = review_start(&dir, None);
    assert_eq!(marks(&a).0, [PathBuf::from("new")]);
    assert_eq!(std::fs::read_to_string(&store).unwrap(), old);
    let _ = std::fs::remove_dir_all(dir);
}

/// #240: an agent's rebase stopped on a conflict lists only what it has replayed. A file of a
/// later commit keeps its tick, unseen, through a mark made meanwhile, and has it back when the
/// rebase is done. A review started during the stop is the rebased branch's.
#[test]
fn a_stopped_rebase_keeps_the_ticks_of_files_not_replayed_yet() {
    let (dir, _) = review_app("viewedrebase");
    let git = |args: &[&str]| {
        let mut cmd = std::process::Command::new("git");
        cmd.arg("-C").arg(&dir).args(args).env("GIT_EDITOR", "true");
        cmd.output().unwrap().status.success()
    };
    // `late` comes in a second commit; `main` moves under the first, which then conflicts.
    std::fs::write(dir.join("late"), "l\n").unwrap();
    assert!(git(&["add", "late"]) && git(&["commit", "-qm", "two"]));
    assert!(git(&["switch", "-q", "main"]));
    std::fs::write(dir.join("src/a.rs"), "conflict\n").unwrap();
    assert!(git(&["commit", "-qam", "main moves"]) && git(&["switch", "-q", "feature"]));
    let mut a = review_start(&dir, None);
    let refresh = |a: &mut App| {
        let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
        a.review_refreshed(fresh)
    };
    a.focus = Focus::Tree;
    a.tree.reveal(Path::new("late"));
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);

    assert!(!git(&["rebase", "main"]), "stops on the conflict");
    refresh(&mut a);
    assert!(a.review.as_ref().unwrap().file(Path::new("late")).is_none());
    a.tree.reveal(Path::new("tail"));
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    let during = review_start(&dir, None);
    assert_eq!(during.review.as_ref().unwrap().branch, "feature");
    let hidden = vec![PathBuf::from("late")];
    assert_eq!(marks(&during), (vec![PathBuf::from("tail")], hidden));

    std::fs::write(dir.join("src/a.rs"), "a\nB\nc\nd\ne\nF\n").unwrap();
    assert!(git(&["add", "src/a.rs"]) && git(&["rebase", "--continue"]));
    refresh(&mut a);
    let both = (vec![PathBuf::from("late"), PathBuf::from("tail")], vec![]);
    assert_eq!(marks(&a), both);
    assert_eq!(marks(&review_start(&dir, None)), both);
    let _ = std::fs::remove_dir_all(dir);
}

/// #240: a write keeps the lines of the same branch against another base, and a name ending in
/// `\r` (macOS's `Icon\r`) reads back as itself, from this review's lines and from those a
/// write of another review carries over.
#[test]
fn the_viewed_store_keeps_other_bases_and_names_as_they_are() {
    let (dir, mut a) = review_app_with("viewedbases", &[("cr\r", b"x\n")]);
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(&dir)
        .args(["branch", "dev", "main"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let mark = |a: &mut App, f: &str| {
        a.focus = Focus::Tree;
        a.tree.reveal(Path::new(f));
        press(a, KeyCode::Char('m'), KeyModifiers::NONE);
    };
    mark(&mut a, "cr\r");
    mark(&mut a, "tail");
    let mut dev = review_start(&dir, Some("dev"));
    assert_eq!(marks(&dev), (vec![], vec![]));
    mark(&mut dev, "new");
    let paths = |p: &[&str]| p.iter().map(PathBuf::from).collect::<Vec<_>>();
    assert_eq!(marks(&review_start(&dir, None)).0, paths(&["cr\r", "tail"]));
    assert_eq!(marks(&review_start(&dir, Some("dev"))).0, paths(&["new"]));
    let _ = std::fs::remove_dir_all(dir);
}

/// #240: the store is in the repository's common git dir, so a worktree's review of the branch
/// has the marks made in the main checkout, and the other way round.
#[test]
fn a_worktree_review_shares_the_viewed_marks() {
    let (dir, mut a) = review_app("viewedwt");
    let wt = PathBuf::from(format!("{}-wt", dir.display()));
    let git = |args: &[&str]| {
        let mut cmd = std::process::Command::new("git");
        let out = cmd.arg("-C").arg(&dir).args(args).output().unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    git(&["switch", "-q", "main"]);
    git(&["worktree", "add", "-q", wt.to_str().unwrap(), "feature"]);
    let mut b = review_start(&wt, None);
    assert_eq!(marks(&b).0, [PathBuf::from("new")]);
    b.focus = Focus::Tree;
    b.tree.reveal(Path::new("tail"));
    press(&mut b, KeyCode::Char('m'), KeyModifiers::NONE);
    let store = std::fs::read_to_string(dir.join(".git/merl/viewed")).unwrap();
    assert!(store.contains("\ttail\n"), "{store}");
    assert!(
        !dir.join(".git/worktrees")
            .join(wt.file_name().unwrap())
            .join("merl")
            .exists()
    );
    let _ = std::fs::remove_dir_all(wt);
    let _ = std::fs::remove_dir_all(dir);
}

/// #240: a review whose marks nobody wrote for 30 days is not read, and leaves the store the
/// next time it is written (the edge, with a fixed day, is in `review.rs`). The store is
/// replaced whole: a merl reading it meanwhile has the old one entire.
#[test]
fn the_viewed_store_forgets_old_reviews_and_is_replaced_whole() {
    let (dir, _) = review_app("viewedstore");
    let store = dir.join(".git/merl/viewed");
    let today = crate::stats::today();
    let line = |days, branch| {
        let day = crate::stats::date(today - days);
        format!("{day}\t{branch}\tmain\t{:016x}\tx.rs\n", 7)
    };
    let own = format!(
        "{}\tfeature\tmain\t{:016x}\ttail\n",
        crate::stats::date(today - 40),
        7
    );
    let old = line(40, "stale") + &line(10, "recent") + &own;
    std::fs::create_dir_all(store.parent().unwrap()).unwrap();
    std::fs::write(&store, &old).unwrap();
    let reader = dir.join(".git/viewed-open-elsewhere");
    std::fs::hard_link(&store, &reader).unwrap();
    let mut a = review_start(&dir, None);
    assert_eq!(marks(&a), (vec![], vec![]), "its own marks, 40 days old");
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    let text = std::fs::read_to_string(&store).unwrap();
    assert!(
        !text.contains("\tstale\t") && !text.contains("\ttail\n"),
        "{text}"
    );
    assert!(text.contains(&line(10, "recent")), "{text}");
    let mark = "\tfeature\tmain\t";
    assert!(text.contains(mark) && text.ends_with("\tnew\n"), "{text}");
    assert_eq!(std::fs::read_to_string(&reader).unwrap(), old);
    let left: Vec<_> = std::fs::read_dir(store.parent().unwrap())
        .unwrap()
        .collect();
    assert_eq!(left.len(), 1, "no temp file left behind");
    let _ = std::fs::remove_dir_all(dir);
}

/// #76: the review is left open next to an agent. What `main` does on a change is
/// `Review::refresh` and `review_refreshed`; the open file, the cursor and both scrolls stay.
#[test]
fn a_live_review_follows_edits_untracked_files_and_commits() {
    let (dir, mut a) = review_app("livereview");
    let git = |args: &[&str]| {
        let mut cmd = std::process::Command::new("git");
        let out = cmd.arg("-C").arg(&dir).args(args).output().unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    let refresh = |a: &mut App| {
        let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
        a.review_refreshed(fresh)
    };
    let rows = |a: &App| -> Vec<(char, String, usize, usize)> {
        let files = a.review.as_ref().unwrap().files.iter();
        files
            .map(|f| (f.status, f.path.display().to_string(), f.added, f.deleted))
            .collect()
    };
    let row = |s: char, p: &str, n: usize, m: usize| (s, p.to_string(), n, m);
    // Where the reader is: the code pane, the panel cursor and its row on screen.
    let place = |a: &App| {
        let vis = a.tree.visible();
        let row = vis.iter().position(|&i| i == a.tree.cursor).unwrap();
        let selected = a.tree.selected().unwrap().path.clone();
        (at(a), a.col, a.top_line, selected, row - a.tree_top)
    };
    assert!(!refresh(&mut a), "nothing changed: nothing to draw");
    a.tree.reveal(Path::new("new"));
    a.tree_top = 1;
    let before = place(&a);
    assert_eq!(a.review_status().unwrap(), "hunk 1/1  file 4/5");

    // The agent touches a file for the first time, writes a module in a new directory and
    // one at the root, neither added, and a log that is ignored.
    std::fs::write(dir.join("src/keep.rs"), "k1\nk2\nK\nk3\n").unwrap();
    std::fs::create_dir(dir.join("pkg")).unwrap();
    std::fs::write(dir.join("pkg/mod.py"), "x = 1\ny = 2\n").unwrap();
    std::fs::write(dir.join("pkg/logo.png"), b"\x89PNG\0").unwrap();
    std::fs::write(dir.join("zz.py"), "z = 1\nz = 2\nlast").unwrap();
    std::fs::write(dir.join(".gitignore"), "*.log\n").unwrap();
    std::fs::write(dir.join("agent.log"), "noise\n").unwrap();
    assert!(refresh(&mut a));
    assert_eq!(
        rows(&a),
        vec![
            row('A', "pkg/logo.png", 0, 0),
            row('A', "pkg/mod.py", 2, 0),
            row('M', "src/a.rs", 2, 2),
            row('M', "src/keep.rs", 1, 0),
            row('A', ".gitignore", 1, 0),
            row('M', "crlf.txt", 1, 1),
            row('D', "gone", 0, 2),
            row('A', "new", 1, 0),
            row('M', "tail", 0, 2),
            row('A', "zz.py", 3, 0),
        ]
    );
    assert_eq!(a.review_status().unwrap(), "hunk 1/1  file 8/10");
    assert_eq!(place(&a), before, "rows came in above: nothing moved");
    let shown = |a: &App, p: &str| {
        a.tree
            .visible()
            .iter()
            .any(|&i| a.tree.nodes[i].path == Path::new(p))
    };
    assert!(shown(&a, "pkg/mod.py"), "a new directory comes open");
    // A directory the reader closed stays closed.
    a.tree.reveal(Path::new("src"));
    a.tree.collapse();
    a.tree.reveal(Path::new("new"));
    std::fs::write(dir.join("pkg/mod.py"), "x = 1\n").unwrap();
    assert!(refresh(&mut a));
    assert_eq!(rows(&a)[1], row('A', "pkg/mod.py", 1, 0));
    assert!(a.review.as_ref().unwrap().files[0].binary);
    assert!(!shown(&a, "src/a.rs"));

    // `c` walks into the untracked file as into any added one: every line is added.
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("zz.py"), 0));
    assert_eq!(
        (a.diff.marks.len(), &a.diff.hunks),
        (3, &vec![TextLine::File(0)])
    );
    assert_eq!(a.review_status().unwrap(), "hunk 1/1  file 10/10");

    // A reverted file leaves the panel. The open one stays open, without marks.
    a.jump_to(&dir.join("src/keep.rs"), 3);
    assert_eq!(a.diff.marks.len(), 1);
    std::fs::write(dir.join("src/keep.rs"), "k1\nk2\nk3\n").unwrap();
    a.reload(false);
    assert!(refresh(&mut a));
    assert!(
        a.review
            .as_ref()
            .unwrap()
            .file(Path::new("src/keep.rs"))
            .is_none()
    );
    assert_eq!(at(&a), (dir.join("src/keep.rs"), 2));
    assert!(a.diff.marks.is_empty());
    assert_eq!(a.review_status(), None);

    // The agent commits: the rows are the same ones, tracked now. Then the base moves up to
    // the branch's first commit, and only the second one is left to review; `new`, open
    // and not touched on disk, loses its marks.
    a.jump_to(&dir.join("new"), 1);
    assert_eq!(a.diff.marks.len(), 1);
    let listed = rows(&a);
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "more work"]);
    assert!(
        refresh(&mut a),
        "the rows are tracked now: `untracked` went"
    );
    assert_eq!(rows(&a), listed);
    assert_eq!(a.review_status().unwrap(), "hunk 1/1  file 7/9");
    git(&["branch", "-f", "main", "HEAD~1"]);
    assert!(refresh(&mut a));
    assert_eq!(
        rows(&a),
        vec![
            row('A', "pkg/logo.png", 0, 0),
            row('A', "pkg/mod.py", 1, 0),
            row('A', ".gitignore", 1, 0),
            row('A', "zz.py", 3, 0),
        ]
    );
    assert_eq!(at(&a), (dir.join("new"), 0));
    assert!(a.diff.marks.is_empty());
    assert_eq!(a.review_status(), None);
    // The base caught up with the branch: an empty panel, and keys that find no row.
    git(&["branch", "-f", "main", "HEAD"]);
    assert!(refresh(&mut a));
    assert_eq!(a.review_status(), None);
    a.focus = Focus::Tree;
    for key in [KeyCode::Down, KeyCode::Enter, KeyCode::Char('c')] {
        press(&mut a, key, KeyModifiers::NONE);
    }
    assert_eq!(at(&a), (dir.join("new"), 0));
    let _ = std::fs::remove_dir_all(dir);
}

/// #76: the marks of the open file are read again when the base moves under it or its row
/// changes kind, with no write to the file itself, which is what `reload` answers.
#[test]
fn the_open_file_gets_new_marks_when_only_the_branch_changed() {
    let (dir, mut a) = review_app("livemarks");
    let git = |args: &[&str]| {
        let mut cmd = std::process::Command::new("git");
        let out = cmd.arg("-C").arg(&dir).args(args).output().unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    let refresh = |a: &mut App| {
        let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
        a.review_refreshed(fresh)
    };
    let keep = dir.join("src/keep.rs");
    std::fs::write(&keep, "k1\nK2\nk3\n").unwrap();
    git(&["commit", "-qam", "second"]);
    std::fs::write(&keep, "k1\nK2\nK3\n").unwrap();
    git(&["commit", "-qam", "third"]);
    assert!(refresh(&mut a));
    a.jump_to(&keep, 1);
    assert_eq!(a.diff.marks.len(), 2);
    // The base moves up to `second`: the row stays `M`, one mark is left.
    git(&["branch", "-f", "main", "HEAD~1"]);
    assert!(refresh(&mut a));
    assert_eq!(a.diff.marks.len(), 1);
    // The file leaves the index: the same merge base, a row of another kind, every line
    // added. One row: the file on disk, not the deletion.
    git(&["rm", "-q", "--cached", "src/keep.rs"]);
    assert!(refresh(&mut a));
    let rows = a.review.as_ref().unwrap().files.iter();
    let rows: Vec<_> = rows
        .filter(|f| f.path == Path::new("src/keep.rs"))
        .collect();
    assert_eq!(
        (rows.len(), rows[0].status, rows[0].untracked),
        (1, 'A', true)
    );
    assert_eq!(a.diff.marks.len(), 3);
    assert_eq!(at(&a), (keep, 0));
    let _ = std::fs::remove_dir_all(dir);
}

/// #76: the branch deleted `gone` and the agent writes it again, not added: git lists the
/// path as deleted and as untracked. It is one row, read from disk, and `c` walks past it.
#[test]
fn a_deleted_file_written_again_is_one_row() {
    let (dir, mut a) = review_app("liveagain");
    std::fs::write(dir.join("gone"), "fresh\n").unwrap();
    let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    assert!(a.review_refreshed(fresh));
    let names = a.tree.nodes.iter().map(|n| n.path.to_str().unwrap());
    assert_eq!(
        names.collect::<Vec<_>>(),
        vec!["src", "src/a.rs", "crlf.txt", "gone", "new", "tail"]
    );
    press(&mut a, KeyCode::Char('C'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("gone"), 0));
    assert_eq!(
        (a.buf.lines.clone(), a.buf.readonly),
        (vec!["fresh".to_string()], None)
    );
    press(&mut a, KeyCode::Char('C'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("crlf.txt"), 1));
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("new"), 0));
    let _ = std::fs::remove_dir_all(dir);
}

/// #76: the reader is in the second hunk when the agent adds one above it. The cursor and
/// the scroll go down with the text, so the hunk is still the current one and `c` goes on.
#[test]
fn the_current_hunk_stays_current_when_one_is_added_above() {
    let (dir, mut a) = review_app("livehunk");
    a.jump_to(&dir.join("src/a.rs"), 6);
    a.view_h = 4;
    a.clamp_scroll();
    a.top_line = 3;
    a.anchor = Some((TextLine::File(4), 0));
    let stop = |a: &App| a.history[a.hist_idx].clone();
    assert_eq!(stop(&a), (dir.join("src/a.rs"), TextLine::File(5), 0));
    assert_eq!(a.review_status().unwrap(), "hunk 2/2  file 1/5");
    std::fs::write(dir.join("src/a.rs"), "new\nnew\na\nB\nc\nd\ne\nF\n").unwrap();
    assert!(a.reload(false));
    assert_eq!(
        (a.line, a.top_line, a.buf.lines[a.line].as_str()),
        (7, 5, "F")
    );
    assert_eq!(a.review_status().unwrap(), "hunk 3/3  file 1/5");
    // What is selected is still selected, and the history stop is still under the cursor.
    assert_eq!(a.anchor, Some((TextLine::File(6), 0)));
    assert_eq!(stop(&a), (dir.join("src/a.rs"), TextLine::File(7), 0));
    a.anchor = None;
    // A block deleted above the pane, from a file longer than what is left of it: the
    // scroll is carried from where it was, not from where the shorter file clamps it.
    let text = |r: std::ops::Range<usize>| r.map(|i| format!("l{i}\n")).collect::<String>();
    std::fs::write(dir.join("src/a.rs"), text(0..40)).unwrap();
    a.reload(false);
    (a.view_h, a.line, a.top_line) = (10, 35, 30);
    std::fs::write(dir.join("src/a.rs"), text(25..40)).unwrap();
    a.reload(false);
    assert_eq!(
        (a.line, a.top_line, a.buf.lines[a.line].as_str()),
        (10, 5, "l35")
    );
    std::fs::write(dir.join("src/a.rs"), "new\nnew\na\nB\nc\nd\ne\nF\n").unwrap();
    a.reload(false);
    a.jump_to(&dir.join("src/a.rs"), 8);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("crlf.txt"), 1));
    // Written below the cursor, or the cursor's own line: nothing to follow.
    let text = |n: usize| (0..n).map(|i| format!("{i}\n")).collect::<String>();
    for (old, new, l, want) in [
        (text(6), text(6) + "x\n", 3, 3),
        (text(6), "x\n".to_string() + &text(6), 3, 4),
        (text(6), text(6).replace("3\n", "x\ny\n"), 3, 3),
        (text(6), text(6).replace("1\n", ""), 3, 2),
        ("a\nb\n".into(), "a\na\nb\n".into(), 0, 0),
        // The cursor's own lines went: the line that followed them.
        (text(6), text(6).replace("2\n3\n", ""), 3, 2),
    ] {
        let lines = |s: &str| s.lines().map(String::from).collect::<Vec<_>>();
        assert_eq!(
            carried(&lines(&old), &lines(&new), l),
            want,
            "{old:?} {new:?}"
        );
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// A submodule is a gitlink, which `git diff --numstat` counts as one added line: listed,
/// not a stop. And a file that does not open is walked past with the reason, not retried.
#[test]
fn review_walks_past_a_submodule_and_a_file_that_does_not_open() {
    let (dir, mut a) = review_app("reviewsub");
    let git = |at: &Path, args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(at)
            .args(["-c", "user.email=t@t", "-c", "user.name=t"])
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    let sub = dir.join("sub");
    std::fs::create_dir(&sub).unwrap();
    git(&sub, &["init", "-q"]);
    git(&sub, &["commit", "-q", "--allow-empty", "-m", "sub"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "submodule"]);
    a.start_review(git::Review::open(&dir, None, None).unwrap());
    // Panel order: src/a.rs, crlf.txt, gone, new, sub, tail.
    let r = a.review.as_ref().unwrap();
    assert_eq!(r.files[4].path, Path::new("sub"));
    assert!(!r.files[4].has_hunks());
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("tail"), 0));
    assert_eq!(a.message, "skipped 1 file without hunks");
    assert!(a.viewed.contains_key(Path::new("new")) && !a.viewed.contains_key(Path::new("sub")));
    // `new` stops opening: `c` from `gone` goes past it to `tail`, and the status says
    // what happened to `new`.
    std::fs::remove_file(dir.join("new")).unwrap();
    std::fs::create_dir(dir.join("new")).unwrap();
    a.jump_to(&dir.join("gone"), 1);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("tail"), 0));
    assert!(a.message.contains("new: "), "{}", a.message);
    // Edits that cannot be saved hold merl on the file; what the status says stays.
    a.jump_to(&dir.join("gone"), 1);
    (a.dirty, a.conflict) = (true, true);
    a.message = "save failed: x".into();
    a.hunk(1);
    assert_eq!(at(&a), (dir.join("gone"), 0));
    assert_eq!(a.message, "save failed: x");
    (a.dirty, a.conflict) = (false, false);
    // Nothing ahead opens: the reason again, not a retry that hides it.
    std::fs::remove_file(dir.join("tail")).unwrap();
    a.jump_to(&dir.join("gone"), 1);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("gone"), 0));
    assert!(a.message.contains("tail: "), "{}", a.message);
    let _ = std::fs::remove_dir_all(dir);
}

/// #404: git counts a symlink as one line, where it points, and a link to a directory has no
/// text to read: the review opens on the first file with a hunk, and `c` and `C` pass the link
/// as they pass a submodule, whether it comes first, in the middle or last.
#[cfg(unix)]
#[test]
fn review_walks_past_a_link_to_a_directory_wherever_it_is() {
    for (link, to) in [("a/link", "../src"), ("m_link", "src"), ("z_link", "src")] {
        let (dir, _) = review_app(&format!("dirlink-{}", link.replace('/', "-")));
        std::fs::create_dir_all(dir.join(link).parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(to, dir.join(link)).unwrap();
        for args in [&["add", "-A"][..], &["commit", "-q", "-m", "link"]] {
            let mut git = std::process::Command::new("git");
            assert!(
                git.arg("-C")
                    .arg(&dir)
                    .args(args)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let r = git::Review::open(&dir, None, None).unwrap();
        assert!(!r.file(Path::new(link)).unwrap().has_hunks(), "{link}");
        // The file `main` opens.
        let first = r.first_file(&dir).unwrap();
        assert_eq!(first, dir.join("src/a.rs"), "{link}");
        let (_, files) = crate::tree::build(&dir, false);
        let panel: Vec<_> = r.files.iter().map(|f| f.path.clone()).collect();
        let buf = Buffer::load(&first).unwrap();
        let mut a = App::new(
            dir.clone(),
            crate::tree::from_files(&panel),
            files,
            buf,
            None,
        );
        a.start_review(r);
        // Every stop, forward and back: never the link, and no error on the way.
        let mut walk = |key: char, end: &str| {
            let mut stops = vec![a.rel_current().unwrap()];
            loop {
                press(&mut a, KeyCode::Char(key), KeyModifiers::NONE);
                assert!(!a.message.contains("error"), "{link}: {}", a.message);
                if a.message == end {
                    return stops;
                }
                let rel = a.rel_current().unwrap();
                if stops.last() != Some(&rel) {
                    stops.push(rel);
                }
            }
        };
        let forward = ["src/a.rs", "crlf.txt", "gone", "new", "tail"].map(PathBuf::from);
        assert_eq!(walk('c', "last hunk of the review"), forward, "{link}");
        let back = walk('C', "first hunk of the review");
        assert_eq!(
            back.iter().rev().collect::<Vec<_>>(),
            forward.iter().collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// #449: Enter on the panel row of a link to a directory opens nothing: the file shown and the
/// status bar stay as they were, with no OS error and no absolute path.
#[cfg(unix)]
#[test]
fn enter_on_a_link_to_a_directory_in_the_review_panel_opens_nothing() {
    let (dir, _) = review_app("dirlink-enter");
    std::os::unix::fs::symlink("src", dir.join("alink")).unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "link"]] {
        let mut git = std::process::Command::new("git");
        assert!(
            git.arg("-C")
                .arg(&dir)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    let r = git::Review::open(&dir, None, None).unwrap();
    let (_, files) = crate::tree::build(&dir, false);
    let panel: Vec<_> = r.files.iter().map(|f| f.path.clone()).collect();
    let first = r.first_file(&dir).unwrap();
    let mut a = App::new(
        dir.clone(),
        crate::tree::from_files(&panel),
        files,
        Buffer::load(&first).unwrap(),
        None,
    );
    a.start_review(r);
    a.focus = Focus::Tree;
    a.tree.reveal(Path::new("alink"));
    let before = (at(&a), a.message.clone());
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((at(&a), a.message.clone()), before);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn review_walks_hunks_across_files_and_opens_deleted_files_from_the_base() {
    let (dir, mut a) = review_app("reviewapp");
    let c = |a: &mut App| press(a, KeyCode::Char('c'), KeyModifiers::NONE);
    let big_c = |a: &mut App| press(a, KeyCode::Char('C'), KeyModifiers::NONE);
    // Files in panel order: src/a.rs (M), crlf.txt (M), gone (D), new (A), tail (M).
    assert_eq!(at(&a), (dir.join("new"), 0));
    assert_eq!(a.review_status().unwrap(), "hunk 1/1  file 4/5");
    // A deletion at the end of `tail` is a stop on its last line, with the ghosts under it.
    c(&mut a);
    assert_eq!(at(&a), (dir.join("tail"), 0));
    assert_eq!(a.diff.ghosts[&1], vec!["t2", "t3"]);
    assert_eq!(a.review_status().unwrap(), "hunk 1/1  file 5/5");
    c(&mut a);
    assert_eq!(at(&a), (dir.join("tail"), 0));
    assert_eq!(a.message, "last hunk of the review");
    // A file crossing is one stop: `[` goes straight back to the previous file.
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("new"), 0));
    // Back over the deleted file: read from the base, read-only, all marked.
    big_c(&mut a);
    assert_eq!(at(&a), (dir.join("gone"), 0));
    assert_eq!(a.buf.lines, vec!["x", "y"]);
    assert_eq!(a.diff.marks.len(), 2);
    assert!(a.diff.marks.values().all(|m| *m == git::Mark::Deleted));
    assert_eq!(a.buf.readonly, Some("deleted in this branch"));
    assert!(a.review_status().unwrap().starts_with("hunk 0/0"));
    // A read-only buffer that the branch did not delete keeps its real diff.
    big_c(&mut a);
    assert_eq!(at(&a), (dir.join("crlf.txt"), 1));
    assert_eq!(a.buf.readonly, Some("mixed line endings"));
    assert_eq!(a.diff.hunks, vec![TextLine::File(1)]);
    assert_eq!(a.diff.marks.len(), 1);
    big_c(&mut a);
    assert_eq!(at(&a), (dir.join("src/a.rs"), 5));
    assert_eq!(a.review_status().unwrap(), "hunk 2/2  file 1/5");
    big_c(&mut a);
    assert_eq!(at(&a), (dir.join("src/a.rs"), 1));
    assert_eq!(a.diff.ghosts[&1], vec!["b"]);
    big_c(&mut a);
    assert_eq!(a.message, "first hunk of the review");
    c(&mut a);
    assert_eq!(at(&a), (dir.join("src/a.rs"), 5));
    // `[` walks back through the same stops.
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("src/a.rs"), 1));
    // The panel lists only the branch's files.
    let names: Vec<_> = a
        .tree
        .nodes
        .iter()
        .map(|n| n.path.to_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec!["src", "src/a.rs", "crlf.txt", "gone", "new", "tail"]
    );
    assert_eq!(
        a.review
            .as_ref()
            .unwrap()
            .files
            .iter()
            .map(|f| f.status)
            .collect::<Vec<_>>(),
        vec!['M', 'M', 'D', 'A', 'M']
    );
    // Enter in the panel opens a file on its first hunk; on the open file it stays put.
    a.tree.reveal(Path::new("src/a.rs"));
    a.focus = Focus::Tree;
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("src/a.rs"), 1));
    // From `B`, the hunk's added line (#690), down onto `c`.
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    a.focus = Focus::Tree;
    a.tree.reveal(Path::new("src/a.rs"));
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((at(&a), a.line_str()), ((dir.join("src/a.rs"), 2), "c"));
    // Opening a shorter file from far down a long one (Enter in the panel, #61 follow-up):
    // the viewport of the old file must not be read against the new one.
    a.jump_to(&dir.join("src/a.rs"), 6);
    (a.top_line, a.top_row) = (5, 0);
    a.jump_to(&dir.join("new"), 0);
    assert_eq!((at(&a), a.top_line), ((dir.join("new"), 0), 0));
    // Outside the project (the standard library, a dependency) nothing is marked deleted.
    let outside = std::env::temp_dir().join(format!("merl-outside-{}", std::process::id()));
    std::fs::write(&outside, "fn x() {}\n").unwrap();
    a.jump_to(&outside, 1);
    assert_eq!(a.buf.readonly, Some("outside the project"));
    assert!(a.diff.marks.is_empty() && a.diff.hunks.is_empty());
    std::fs::remove_file(&outside).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #239: `d` from a hunk into a file outside the review, then `c`: back on the hunk `c` last
/// stopped on, and the next `c` goes on from there; `C` the same, backwards. Before the first
/// `c`, and from another file of the review, the walk is what it always was.
#[test]
fn c_and_big_c_from_outside_the_review_go_back_to_the_hunk_left() {
    let (dir, mut a) = review_app("reviewback");
    let c = |a: &mut App| press(a, KeyCode::Char('c'), KeyModifiers::NONE);
    let big_c = |a: &mut App| press(a, KeyCode::Char('C'), KeyModifiers::NONE);
    // Files in panel order: src/a.rs (M), crlf.txt (M), gone (D), new (A), tail (M);
    // src/keep.rs is not one of them.
    let outside = dir.join("src/keep.rs");
    // Nothing to go back to yet: `c` starts from the first file.
    a.jump_to(&outside, 2);
    c(&mut a);
    assert_eq!(at(&a), (dir.join("src/a.rs"), 1));
    // Back to `B` itself, not to `F` after it.
    a.jump_to(&outside, 2);
    c(&mut a);
    assert_eq!(at(&a), (dir.join("src/a.rs"), 1));
    c(&mut a);
    assert_eq!(at(&a), (dir.join("src/a.rs"), 5));
    a.jump_to(&outside, 2);
    big_c(&mut a);
    assert_eq!(at(&a), (dir.join("src/a.rs"), 5), "`C` goes back too");
    c(&mut a);
    assert_eq!(at(&a), (dir.join("crlf.txt"), 1));
    assert_eq!(a.review_status().unwrap(), "hunk 1/1  file 2/5");
    a.jump_to(&outside, 2);
    assert_eq!(a.review_status(), None);
    c(&mut a);
    assert_eq!(at(&a), (dir.join("crlf.txt"), 1));
    c(&mut a);
    assert_eq!(at(&a), (dir.join("gone"), 0), "on from the hunk left");
    // A deleted file is one stop at its top: `C` goes back there, then on backwards.
    a.jump_to(&outside, 2);
    big_c(&mut a);
    assert_eq!(at(&a), (dir.join("gone"), 0));
    big_c(&mut a);
    assert_eq!(at(&a), (dir.join("crlf.txt"), 1));
    big_c(&mut a);
    assert_eq!(at(&a), (dir.join("src/a.rs"), 5));
    // Walked back into src/a.rs onto its second hunk: the way back is the second hunk too.
    a.jump_to(&outside, 2);
    c(&mut a);
    assert_eq!(at(&a), (dir.join("src/a.rs"), 5));
    // An excursion into another file of the review walks on from the cursor there.
    a.jump_to(&dir.join("tail"), 1);
    big_c(&mut a);
    assert_eq!(at(&a), (dir.join("new"), 0));
    let _ = std::fs::remove_dir_all(dir);
}

/// #239: the hunk left is found again by its place among its file's hunks. Lines an agent
/// writes or deletes around the hunks move them, not their order: written while the reader is
/// still in the file, or deleted above while away, and no unread hunk is passed. When fewer
/// hunks are left, the file's last; a deleted file's one stop is its top. A file that left the
/// review, or that the walk no longer stops at, is not gone back to: `c` and `C` walk as they
/// did before there was a hunk to go back to.
#[test]
fn the_hunk_left_is_found_by_its_place_in_its_file() {
    let (dir, mut a) = review_app("reviewbackgone");
    let c = |a: &mut App| press(a, KeyCode::Char('c'), KeyModifiers::NONE);
    let big_c = |a: &mut App| press(a, KeyCode::Char('C'), KeyModifiers::NONE);
    let outside = dir.join("src/keep.rs");
    let refresh = |a: &mut App| {
        let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
        a.review_refreshed(fresh);
    };
    // The agent writes `name` while the reader is in src/keep.rs, and the review is listed again.
    let write = |a: &mut App, name: &str, text: &str| {
        a.jump_to(&outside, 2);
        std::fs::write(dir.join(name), text).unwrap();
        refresh(a);
    };
    // The hunk by the file line it rewrites, where `c` stands (#690).
    let here = |a: &App| (at(a).0, a.buf.lines[a.line].clone());
    let src = dir.join("src/a.rs");
    let on = |word: &str| (src.clone(), word.to_string());
    // Written while the reader is on `F`, the third hunk: `B` grows by three lines, and `D`
    // comes down past the line `F` was on. The reload carries the cursor, `c` finds `F`.
    write(&mut a, "src/a.rs", "a\nB\nc\nD\ne\nF\n");
    a.jump_to(&src, 4);
    c(&mut a);
    assert_eq!((here(&a), a.line), (on("F"), 5));
    std::fs::write(&src, "a\nB\nB2\nB3\nB4\nc\nD\ne\nF\n").unwrap();
    assert!(a.reload(false));
    assert_eq!((here(&a), a.line), (on("F"), 8));
    a.jump_to(&outside, 2);
    refresh(&mut a);
    c(&mut a);
    assert_eq!((here(&a), a.line_str()), (on("F"), "F"));
    // Deleted above while away: `B` goes from line 6 up to 2 and `D`, unread, to 4, both past
    // the line `B` was left on. `c` goes back to `B`, and the next one to `D`.
    write(&mut a, "src/a.rs", "p1\np2\np3\np4\np5\na\nB\nc\nD\ne\nF\n");
    a.jump_to(&src, 1);
    c(&mut a);
    assert_eq!((here(&a), a.line), (on("B"), 6));
    write(&mut a, "src/a.rs", "p1\na\nB\nc\nD\ne\nF\n");
    c(&mut a);
    assert_eq!((here(&a), a.line), (on("B"), 2));
    c(&mut a);
    assert_eq!(here(&a), on("D"));
    // `D` and `F` reverted: two hunks left for the third, and `C` goes to the file's last.
    write(&mut a, "src/a.rs", "p1\na\nB\nc\nd\ne\nf\n");
    big_c(&mut a);
    assert_eq!(here(&a), on("B"));
    a.jump_to(&dir.join("gone"), 1);
    c(&mut a);
    assert_eq!(at(&a), (dir.join("new"), 0));
    write(&mut a, "new", "");
    let r = a.review.as_ref().unwrap();
    assert!(r.file(Path::new("new")).is_some_and(|f| !f.has_hunks()));
    c(&mut a);
    assert_eq!(at(&a), (dir.join("new"), 0));
    // crlf.txt as it was at the base: the review drops the file.
    a.jump_to(&src, 3);
    c(&mut a);
    assert_eq!(at(&a), (dir.join("crlf.txt"), 1));
    write(&mut a, "crlf.txt", "one\r\ntwo\n");
    big_c(&mut a);
    assert_eq!(at(&a), (dir.join("tail"), 0));
    assert_eq!(a.review_status().unwrap(), "hunk 1/1  file 4/4");
    // A stop on `F`, line 5, then the agent deletes the file: back to its top.
    write(&mut a, "src/a.rs", "a\nB\nc\nd\ne\nF\n");
    a.jump_to(&src, 2);
    c(&mut a);
    assert_eq!(at(&a), (src.clone(), 5));
    a.jump_to(&outside, 2);
    std::fs::remove_file(&src).unwrap();
    refresh(&mut a);
    big_c(&mut a);
    assert_eq!(at(&a), (src.clone(), 0));
    assert_eq!(a.buf.readonly, Some("deleted in this branch"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_deleted_file_with_nothing_to_read_is_gone_back_to() {
    let (dir, mut a) = review_app("reviewbackempty");
    let git = |args: &[&str]| {
        let mut cmd = std::process::Command::new("git");
        let out = cmd.arg("-C").arg(&dir).args(args).output().unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    git(&["switch", "-q", "main"]);
    std::fs::write(dir.join("empty"), "").unwrap();
    git(&["add", "empty"]);
    git(&["commit", "-q", "-m", "empty"]);
    git(&["switch", "-q", "feature"]);
    git(&["merge", "-q", "main", "-m", "merge"]);
    std::fs::write(dir.join("empty"), "e\n").unwrap();
    git(&["commit", "-qam", "fill"]);
    a.start_review(git::Review::open(&dir, None, None).unwrap());
    a.jump_to(&dir.join("crlf.txt"), 2);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("empty"), 0));
    a.jump_to(&dir.join("src/keep.rs"), 2);
    std::fs::remove_file(dir.join("empty")).unwrap();
    let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    a.review_refreshed(fresh);
    let f = a.review.as_ref().unwrap().file(Path::new("empty")).unwrap();
    assert!(f.status == 'D' && !f.has_hunks());
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("empty"), 0));
    assert_eq!(a.buf.readonly, Some("deleted in this branch"));
    let _ = std::fs::remove_dir_all(dir);
}

/// #246: in a review `u` and `s` give a row on a line the branch added or changed the gutter's
/// `▎` in its colour, an untouched line a blank column; `o` lists the review's files first, with
/// their panel letter, then the rest. Either config key off draws its list as before.
#[test]
fn review_marks_changed_rows_in_u_and_s_and_lists_its_files_first_in_o() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;
    let (dir, mut a) = review_app("reviewlists");
    a.show_tree = false;
    // Line 2 changed, line 3 as at the base; both hold a whole `c`.
    std::fs::write(dir.join("src/a.rs"), "a\nB c\nc\nd\ne\nF\n").unwrap();
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    // Each row of the screen with the colours of its cells.
    let screen = |a: &mut App| {
        if let Some(p) = &mut a.picker {
            p.settle();
        }
        let mut t = Terminal::new(TestBackend::new(70, 16)).unwrap();
        t.draw(|f| crate::ui::draw(f, a, &theme)).unwrap();
        let buf = t.backend().buffer();
        (0..buf.area.height)
            .map(|y| {
                let cells = (0..buf.area.width).map(|x| &buf[(x, y)]);
                let text: String = cells.clone().map(|c| c.symbol()).collect();
                (text, cells.map(|c| c.fg).collect::<Vec<_>>())
            })
            .collect::<Vec<_>>()
    };
    // The colour of the cell right after the first char of `needle`, a picker's border.
    let row = |rows: &[(String, Vec<Color>)], needle: &str| {
        rows.iter()
            .find_map(|(t, c)| {
                let at = t.find(needle)?;
                Some(c[t[..at].chars().count() + 1])
            })
            .unwrap_or_else(|| panic!("{needle:?} not in {rows:#?}"))
    };
    let esc = |a: &mut App| press(a, KeyCode::Esc, KeyModifiers::NONE);

    a.jump_to(&dir.join("src/a.rs"), 3);
    a.col = 0;
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    let rows = screen(&mut a);
    assert_eq!(row(&rows, "\u{2502}\u{258e}  2  B c"), Color::Green);
    row(&rows, "\u{2502}   3  c");
    esc(&mut a);

    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "c");
    a.settle_search();
    let rows = screen(&mut a);
    row(&rows, "\u{2502}\u{258e}  2  B c");
    row(&rows, "\u{2502}   3  c");
    esc(&mut a);

    // The review's files on disk in the panel's order, then the rest; `gone` is not on disk.
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
    let rows = screen(&mut a);
    let listed: Vec<&str> = rows
        .iter()
        .filter_map(|(t, _)| {
            let t = t.trim_end_matches([' ', '\u{2502}']);
            t.rsplit_once('\u{2502}').map(|(_, r)| r.trim_end())
        })
        .skip_while(|r| !r.starts_with('>'))
        .skip(1)
        .collect();
    assert_eq!(
        listed,
        [
            "M src/a.rs",
            "M crlf.txt",
            "A new",
            "M tail",
            "  src/keep.rs",
        ],
        "{:#?}",
        rows.iter().map(|(t, _)| t).collect::<Vec<_>>()
    );
    // The letter in the row's style, as the panel draws it (#450): the name's colour.
    let (t, c) = rows
        .iter()
        .find(|(t, _)| t.contains("\u{2502}M src/a.rs"))
        .unwrap();
    let at = t[..t.find("\u{2502}M src/a.rs").unwrap()].chars().count() + 1;
    assert_eq!(c[at], c[at + 2]);
    esc(&mut a);

    a.review_list_marks = false;
    a.review_open_files_first = false;
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    row(&screen(&mut a), "\u{2502}src/a.rs ");
    esc(&mut a);
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
    let rows = screen(&mut a);
    assert!(!rows.iter().any(|(t, _)| t.contains("M src/a.rs")));
    let _ = std::fs::remove_dir_all(dir);
}

/// #439: the lines a branch deleted are lines of the text. `c` and `C` stand on the first added
/// line of a hunk, its first deleted one when it adds none (#690); Up reaches the deleted lines
/// above, `/` finds them, Ctrl+C copies them as they were, and nothing edits them.
#[test]
fn deleted_lines_are_lines_of_the_text() {
    use TextLine::{Deleted, File};
    let (dir, mut a) = review_app("onetext");
    use_roots(&mut a, Kind::Rust, &[]);
    let key = |a: &mut App, c| press(a, c, KeyModifiers::NONE);
    // src/a.rs reads a, [b], B, c, d, e, [f], F: `b` and `f` are deleted.
    a.jump_to(&dir.join("src/a.rs"), 1);
    key(&mut a, KeyCode::Char('c'));
    assert_eq!((a.at(), a.line_str()), (File(1), "B"));
    key(&mut a, KeyCode::Char('c'));
    assert_eq!((a.at(), a.line_str()), (File(5), "F"));
    key(&mut a, KeyCode::Char('C'));
    assert_eq!((a.at(), a.line_str()), (File(1), "B"));
    assert_eq!(a.review_status().unwrap(), "hunk 1/2  file 1/5");
    key(&mut a, KeyCode::Up);
    assert_eq!((a.at(), a.line_str()), (Deleted(1, 0), "b"));
    assert_eq!(a.review_status().unwrap(), "hunk 1/2  file 1/5");
    // From a rewritten line, `c` and `C` go on to the next and the previous hunk.
    key(&mut a, KeyCode::Char('c'));
    assert_eq!((a.at(), a.line_str()), (File(5), "F"));
    key(&mut a, KeyCode::Up);
    assert_eq!((a.at(), a.line_str()), (Deleted(5, 0), "f"));
    assert_eq!(a.review_status().unwrap(), "hunk 2/2  file 1/5");
    key(&mut a, KeyCode::Char('C'));
    assert_eq!((a.at(), a.line_str()), (File(1), "B"));
    key(&mut a, KeyCode::Up);

    // `/` finds the deleted `b` and the added `B` alike, in the order they are drawn.
    key(&mut a, KeyCode::Up);
    key(&mut a, KeyCode::Char('/'));
    typed(&mut a, "b");
    assert_eq!((a.at(), a.message.as_str()), (Deleted(1, 0), "1/2"));
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Char('n'));
    assert_eq!((a.at(), a.message.as_str()), (File(1), "2/2"));
    key(&mut a, KeyCode::Char('n'));
    assert_eq!(a.at(), Deleted(1, 0));
    key(&mut a, KeyCode::Esc);

    // Ctrl+C copies the deleted line; a selection from it into the added one copies both.
    press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(
        (a.clipboard.take().as_deref(), a.message.as_str()),
        (Some("b\n"), "copied 1 line")
    );
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(
        (a.clipboard.take().as_deref(), a.message.as_str()),
        (Some("b\nB"), "copied 2 lines")
    );

    // Nothing edits a deleted line: not Enter on it, not typing after moving onto it, not the
    // break between it and the line under it, not a selection that holds it.
    let file = a.buf.lines.clone();
    key(&mut a, KeyCode::Esc);
    key(&mut a, KeyCode::Up);
    key(&mut a, KeyCode::Enter);
    assert_eq!((a.mode, a.message.as_str()), (Mode::Normal, "deleted"));
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Home);
    key(&mut a, KeyCode::Enter);
    assert_eq!(a.mode, Mode::Edit);
    key(&mut a, KeyCode::Backspace);
    assert_eq!(
        a.message, "deleted",
        "the break the deleted line is drawn after"
    );
    key(&mut a, KeyCode::Up);
    key(&mut a, KeyCode::Char('x'));
    assert_eq!((a.at(), a.message.as_str()), (Deleted(1, 0), "deleted"));
    key(&mut a, KeyCode::Up);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    assert_eq!(a.at(), File(1));
    key(&mut a, KeyCode::Backspace);
    assert_eq!(a.message, "deleted", "a selection from `a` over `b`");
    assert_eq!(a.buf.lines, file);
    // The file's own lines are edited as ever.
    key(&mut a, KeyCode::End);
    key(&mut a, KeyCode::Char('!'));
    assert_eq!(a.buf.lines[1], "B!");
    key(&mut a, KeyCode::Esc);
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);

    // `d` reads the word on a deleted line (#440): nothing declares `b`.
    key(&mut a, KeyCode::Up);
    key(&mut a, KeyCode::Char('d'));
    assert_eq!(
        (a.at(), a.message.as_str()),
        (Deleted(1, 0), "no definition for b")
    );

    // The lines deleted at the end of a file are lines too: `tail` reads t1, [t2], [t3].
    a.jump_to(&dir.join("tail"), 1);
    key(&mut a, KeyCode::Char('c'));
    assert_eq!((a.at(), a.line_str()), (Deleted(1, 0), "t2"));
    press(&mut a, KeyCode::End, KeyModifiers::CONTROL);
    assert_eq!(a.line_str(), "t3");
    key(&mut a, KeyCode::Char('C'));
    assert_eq!((a.at(), a.line_str()), (Deleted(1, 0), "t2"));
    // A hunk that only adds: `new` reads n.
    key(&mut a, KeyCode::Char('C'));
    assert_eq!((at(&a), a.at()), ((dir.join("new"), 0), File(0)));
    key(&mut a, KeyCode::Char('c'));
    assert_eq!((a.line_str(), a.at()), ("t2", Deleted(1, 0)));
    let _ = std::fs::remove_dir_all(dir);
}

/// #439, each way into a deleted line on its own: a selection inside one line of a file with
/// deleted lines is copied and typed over as anywhere; paste refuses a deleted line, `u` and F12
/// read it (#440); Esc from `/` and `N` come back to one; a reload finds the deleted line the
/// cursor was on.
#[test]
fn a_deleted_line_is_refused_and_returned_to_every_way() {
    use TextLine::{Deleted, File};
    let (dir, mut a) = review_app("onetext-ways");
    use_roots(&mut a, Kind::Rust, &[]);
    let key = |a: &mut App, c| press(a, c, KeyModifiers::NONE);
    let shift = |a: &mut App, c| press(a, c, KeyModifiers::SHIFT);
    a.jump_to(&dir.join("src/a.rs"), 1);
    shift(&mut a, KeyCode::Right);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(a.clipboard.take().as_deref(), Some("a"));
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Char('z'));
    assert_eq!(a.buf.lines[0], "z");
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    key(&mut a, KeyCode::Esc);

    // A paste over a selection that holds the deleted `b` is refused.
    let file = a.buf.lines.clone();
    a.jump_to(&dir.join("src/a.rs"), 1);
    shift(&mut a, KeyCode::Down);
    shift(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Enter);
    a.paste("zz");
    assert_eq!((a.message.as_str(), &a.buf.lines), ("deleted", &file));
    key(&mut a, KeyCode::Esc);
    key(&mut a, KeyCode::Esc);

    // `u` and F12 read the word on a deleted line (#440): `b` is only there.
    a.jump_to(&dir.join("src/a.rs"), 2);
    key(&mut a, KeyCode::Up);
    assert_eq!(a.at(), Deleted(1, 0));
    key(&mut a, KeyCode::Char('u'));
    let p = a.picker.as_mut().expect("the uses of b");
    p.settle();
    let rows: Vec<String> = p.window(9).0.iter().map(|r| r.item.label.clone()).collect();
    assert_eq!(rows, ["src/a.rs:2: b"]);
    key(&mut a, KeyCode::Esc);
    key(&mut a, KeyCode::F(12));
    assert_eq!(
        (a.message.as_str(), a.picker.is_none()),
        ("no definition for b", true)
    );

    // Esc from `/` puts the cursor back on the deleted line it started from.
    key(&mut a, KeyCode::Char('/'));
    typed(&mut a, "d");
    assert_ne!(a.at(), Deleted(1, 0));
    key(&mut a, KeyCode::Esc);
    assert_eq!(a.at(), Deleted(1, 0));

    // `N` goes back over the end of the file onto the lines deleted there: tail reads t1,
    // [t2], [t3].
    a.jump_to(&dir.join("tail"), 1);
    key(&mut a, KeyCode::Char('/'));
    typed(&mut a, "t");
    key(&mut a, KeyCode::Enter);
    assert_eq!((a.at(), a.message.as_str()), (File(0), "1/3"));
    key(&mut a, KeyCode::Char('N'));
    assert_eq!((a.at(), a.message.as_str()), (Deleted(1, 1), "3/3"));
    key(&mut a, KeyCode::Char('N'));
    assert_eq!((a.at(), a.message.as_str()), (Deleted(1, 0), "2/3"));
    key(&mut a, KeyCode::Esc);

    // An agent appends to the file: git keys the deletion above the first new line, and the
    // cursor stays on the deleted line it was reading.
    key(&mut a, KeyCode::Down);
    assert_eq!(a.line_str(), "t3");
    std::fs::write(dir.join("tail"), "t1\nA\nB\n").unwrap();
    assert!(a.reload(false));
    assert_eq!((a.at(), a.line_str()), (Deleted(1, 1), "t3"));
    let _ = std::fs::remove_dir_all(dir);
}

/// #243: a generated file is folded, as on GitHub and GitLab. `c` stops on the fold once and
/// goes on, ticking it; `C` comes back to it; the keys that read the text do nothing on it;
/// Enter loads the diff, and `c` walks its hunks. It stays unfolded when the review opens again.
#[test]
fn a_generated_file_is_one_stop_until_enter_loads_its_diff() {
    let (dir, mut a) = review_app_with_lock("reviewfold");
    let lock = dir.join("poetry.lock");
    let key = |a: &mut App, code| press(a, code, KeyModifiers::NONE);
    // Panel order: src/a.rs, crlf.txt, gone, new, poetry.lock, tail.
    assert_eq!(at(&a).0, dir.join("new"));
    key(&mut a, KeyCode::Char('c'));
    assert_eq!(at(&a).0, lock);
    assert!(a.folded_here().is_some());
    assert_eq!(a.review_status().unwrap(), "folded  file 5/6");
    key(&mut a, KeyCode::Char('c'));
    assert_eq!(at(&a).0, dir.join("tail"), "one stop, then the next file");
    assert!(a.viewed.contains_key(Path::new("poetry.lock")));
    key(&mut a, KeyCode::Char('C'));
    assert_eq!(at(&a).0, lock, "`C` comes back to the fold");
    assert!(a.folded_here().is_some());
    key(&mut a, KeyCode::Char('C'));
    assert_eq!(at(&a).0, dir.join("new"), "`C` goes on from the fold too");
    key(&mut a, KeyCode::Char('c'));
    let folded_at = at(&a);
    assert_eq!(folded_at.0, lock);
    for code in [
        KeyCode::Down,
        KeyCode::PageDown,
        KeyCode::Char('v'),
        KeyCode::Char('d'),
    ] {
        key(&mut a, code);
    }
    press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(
        at(&a),
        folded_at,
        "the keys that read the text wait for the diff"
    );
    assert_eq!(a.mode, Mode::Normal);
    assert!(a.selection().is_none());
    assert_eq!(
        a.message, "",
        "no definition looked up, no hidden line copied"
    );
    // The panel works as anywhere: Enter there opens its row, and the fold stays.
    key(&mut a, KeyCode::Tab);
    a.tree.reveal(Path::new("tail"));
    key(&mut a, KeyCode::Enter);
    assert_eq!(at(&a).0, dir.join("tail"));
    key(&mut a, KeyCode::Char('C'));
    assert!(a.folded_here().is_some(), "the panel's Enter loads no diff");
    key(&mut a, KeyCode::Enter);
    assert!(a.folded_here().is_none());
    assert_eq!(
        a.mode,
        Mode::Normal,
        "Enter loads the diff, it does not edit"
    );
    assert_eq!(at(&a), (lock.clone(), 1));
    assert_eq!(a.review_status().unwrap(), "hunk 1/3  file 5/6");
    key(&mut a, KeyCode::Char('c'));
    assert_eq!(at(&a), (lock.clone(), 4));
    // Loaded stays loaded when the branch is reviewed again.
    let mut again = review_start(&dir, None);
    again.jump_to(&lock, 1);
    assert!(again.folded_here().is_none());
    let _ = std::fs::remove_dir_all(dir);
}

/// #243: Enter on a fold a jump reached keeps the jump's line; a file with nothing to fold (a
/// pure rename) is not folded; a file whose text is on screen is not folded under the user by a
/// refresh that comes to list it as generated.
#[test]
fn a_fold_keeps_a_jumps_line_and_never_covers_text_in_sight() {
    let (dir, _) = review_app("reviewfold2");
    let git = |args: &[&str]| {
        let mut cmd = std::process::Command::new("git");
        let out = cmd.arg("-C").arg(&dir).args(args).output().unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    let lines: String = (0..9).map(|i| format!("l{i}\n")).collect();
    git(&["switch", "-q", "main"]);
    std::fs::write(dir.join("poetry.lock"), &lines).unwrap();
    std::fs::write(dir.join("Cargo.lock"), &lines).unwrap();
    std::fs::write(dir.join("uv.lock"), &lines).unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "locks"]);
    git(&["switch", "-q", "feature"]);
    git(&["merge", "-q", "main", "-m", "merge"]);
    let bumped = lines.replace("l1", "L1").replace("l7", "L7");
    std::fs::write(dir.join("poetry.lock"), bumped).unwrap();
    git(&["mv", "uv.lock", "src/uv.lock"]);
    git(&["commit", "-qam", "bump"]);
    let mut a = review_start(&dir, None);
    // A jump onto line 6 of the folded lock file: Enter shows it there.
    let lock = dir.join("poetry.lock");
    a.jump_to(&lock, 6);
    assert!(a.folded_here().is_some());
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert!(a.folded_here().is_none());
    assert_eq!(at(&a), (lock, 5), "the jump's line, not the first hunk");
    // A renamed lock file has nothing to fold.
    a.jump_to(&dir.join("src/uv.lock"), 1);
    let r = a.review.as_ref().unwrap();
    assert!(
        r.file(Path::new("src/uv.lock"))
            .is_some_and(|f| f.generated)
    );
    assert!(a.folded_here().is_none());
    // Cargo.lock, not in the branch, edited in sight: the refresh that lists it keeps it shown.
    a.jump_to(&dir.join("Cargo.lock"), 1);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(a.mode, Mode::Edit);
    typed(&mut a, "x");
    press(&mut a, KeyCode::Char('s'), KeyModifiers::CONTROL);
    let fresh = a.review.as_ref().unwrap().refresh(&dir).unwrap();
    assert!(
        fresh
            .file(Path::new("Cargo.lock"))
            .is_some_and(|f| f.generated)
    );
    a.review_refreshed(fresh);
    assert!(a.folded_here().is_none());
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert!(a.folded_here().is_none(), "still in sight after the edit");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn review_stops_on_every_empty_file_and_walks_past_a_binary_one() {
    let png: &[u8] = b"\x89PNG\0\0";
    let extra: &[(&str, &[u8])] = &[("e", b""), ("z.png", png), ("zy", b""), ("zz", b"")];
    let (dir, mut a) = review_app_with("reviewempty", extra);
    let order = |a: &App| -> Vec<_> {
        let r = a.review.as_ref().unwrap();
        r.files.iter().map(|f| f.path.clone()).collect()
    };
    assert_eq!(
        order(&a),
        [
            "src/a.rs", "crlf.txt", "e", "gone", "new", "tail", "z.png", "zy", "zz"
        ]
        .map(PathBuf::from)
    );
    let viewed = |a: &App, p: &str| a.viewed.contains_key(Path::new(p));
    let c = |a: &mut App| press(a, KeyCode::Char('c'), KeyModifiers::NONE);
    c(&mut a);
    c(&mut a);
    assert_eq!(at(&a), (dir.join("zy"), 0));
    assert_eq!(a.message, "skipped 1 file without hunks");
    assert!(viewed(&a, "tail") && !viewed(&a, "zy"));
    c(&mut a);
    assert_eq!(at(&a), (dir.join("zz"), 0));
    assert!(viewed(&a, "zy") && !viewed(&a, "zz"));
    c(&mut a);
    assert_eq!(at(&a), (dir.join("zz"), 0));
    assert_eq!(a.message, "last hunk of the review");
    assert!(viewed(&a, "zz") && !viewed(&a, "z.png"));
    a.jump_to(&dir.join("src/keep.rs"), 1);
    c(&mut a);
    assert_eq!(at(&a), (dir.join("zz"), 0));
    let mut back = vec![];
    while at(&a) != (dir.join("crlf.txt"), 1) {
        press(&mut a, KeyCode::Char('C'), KeyModifiers::NONE);
        back.push(at(&a).0.strip_prefix(&dir).unwrap().to_path_buf());
    }
    assert_eq!(
        back,
        ["zy", "tail", "new", "gone", "e", "crlf.txt"].map(PathBuf::from)
    );

    std::fs::write(dir.join("src/__init__.py"), "").unwrap();
    let mut a = review_start(&dir, None);
    a.buf = Buffer::empty();
    a.start_review(git::Review::open(&dir, None, None).unwrap());
    assert_eq!(order(&a)[0], Path::new("src/__init__.py"));
    assert_eq!(at(&a), (dir.join("src/__init__.py"), 0));
}

fn branch_repo(tag: &str, base: &[(&str, &str)], work: &[&[&str]]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("merl-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let git = |args: &[&str]| {
        let mut cmd = std::process::Command::new("git");
        let out = cmd.arg("-C").arg(&dir).args(args).output().unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    for (name, text) in base {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "t@t"]);
    git(&["config", "user.name", "t"]);
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    for args in work {
        git(args);
    }
    git(&["commit", "-qam", "work"]);
    dir
}

fn started_as_main_starts(dir: &Path) -> App {
    let r = git::Review::open(dir, None, None).unwrap();
    let buf = (r.first_file(dir)).map_or_else(Buffer::empty, |p| Buffer::load(&p).unwrap());
    let paths: Vec<PathBuf> = r.files.iter().map(|f| f.path.clone()).collect();
    let (_, files) = crate::tree::build(dir, false);
    let mut a = App::new(
        dir.to_path_buf(),
        crate::tree::from_listing(&paths),
        files,
        buf,
        None,
    );
    a.start_review(r);
    a
}

#[test]
fn a_review_whose_first_file_is_deleted_opens_on_it() {
    let base = [("a/empty", ""), ("a/gone.py", "g\n"), ("b.py", "x\n")];
    for (gone, note) in [
        ("a/empty", Some("empty file, deleted")),
        ("a/gone.py", None),
    ] {
        let dir = branch_repo("reviewfirstgone", &base, &[&["rm", "-q", gone]]);
        std::fs::write(dir.join("b.py"), "y\n").unwrap();
        let mut a = started_as_main_starts(&dir);
        assert_eq!(at(&a), (dir.join(gone), 0));
        assert_eq!(a.buf.readonly, Some("deleted in this branch"));
        assert_eq!(a.empty_note().as_deref(), note);
        press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("b.py"), 0));
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn an_empty_file_of_the_review_shows_its_note_until_it_has_text() {
    let base = [("a.py", "x\n"), ("old", ""), ("keep.py", "k\n")];
    let moves: &[&[&str]] = &[&["mv", "old", "new"], &["mv", "keep.py", "kept.py"]];
    let dir = branch_repo("reviewnote", &base, moves);
    std::fs::write(dir.join("a.py"), "y\n").unwrap();
    std::fs::write(dir.join("e"), "").unwrap();
    let mut a = started_as_main_starts(&dir);
    assert_eq!(a.empty_note(), None);
    let note = |a: &mut App, file: &str| {
        a.jump_to(&dir.join(file), 1);
        a.empty_note()
    };
    assert_eq!(note(&mut a, "new").unwrap(), "empty file, renamed from old");
    assert_eq!(note(&mut a, "kept.py"), None);
    assert_eq!(note(&mut a, "e").unwrap(), "empty file, added");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('x'), KeyModifiers::NONE);
    assert_eq!(a.buf.lines, ["x"]);
    assert_eq!(a.empty_note(), None);

    let (plain_dir, mut plain) = project_app("emptynote", &[("blank", "")]);
    plain.jump_to(&plain_dir.join("blank"), 1);
    assert_eq!(plain.empty_note(), None);
    let _ = std::fs::remove_dir_all(plain_dir);
    let _ = std::fs::remove_dir_all(dir);
}
