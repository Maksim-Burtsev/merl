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
/// nowhere to go; `C` marks nothing, `m` toggles, and a changed file loses its mark.
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
    assert!(a.review_refreshed(fresh), "the tick leaves the screen");
    assert!(!a.viewed.contains_key(Path::new("tail")));
    assert!(a.viewed.contains_key(Path::new("src/a.rs")));
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
    assert_eq!((a.diff.marks.len(), &a.diff.hunks), (3, &vec![0]));
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
    assert_eq!(a.review_status().unwrap(), "hunk 0/0  file -/9");

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
    assert_eq!(a.review_status().unwrap(), "hunk 0/0  file -/4");
    // The base caught up with the branch: an empty panel, and keys that find no row.
    git(&["branch", "-f", "main", "HEAD"]);
    assert!(refresh(&mut a));
    assert_eq!(a.review_status().unwrap(), "hunk 0/0  file -/0");
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
    a.anchor = Some((4, 0));
    let stop = |a: &App| a.history[a.hist_idx].clone();
    assert_eq!(stop(&a), (dir.join("src/a.rs"), 5, 0));
    assert_eq!(a.review_status().unwrap(), "hunk 2/2  file 1/5");
    std::fs::write(dir.join("src/a.rs"), "new\nnew\na\nB\nc\nd\ne\nF\n").unwrap();
    assert!(a.reload(false));
    assert_eq!(
        (a.line, a.top_line, a.buf.lines[a.line].as_str()),
        (7, 5, "F")
    );
    assert_eq!(a.review_status().unwrap(), "hunk 3/3  file 1/5");
    // What is selected is still selected, and the history stop is still under the cursor.
    assert_eq!(a.anchor, Some((6, 0)));
    assert_eq!(stop(&a), (dir.join("src/a.rs"), 7, 0));
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
    assert_eq!(a.buf.readonly, Some("deleted in this branch"));
    assert!(a.review_status().unwrap().starts_with("hunk 0/0"));
    // A read-only buffer that the branch did not delete keeps its real diff.
    big_c(&mut a);
    assert_eq!(at(&a), (dir.join("crlf.txt"), 1));
    assert_eq!(a.buf.readonly, Some("mixed line endings"));
    assert_eq!(a.diff.hunks, vec![1]);
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
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    a.focus = Focus::Tree;
    a.tree.reveal(Path::new("src/a.rs"));
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("src/a.rs"), 3));
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
