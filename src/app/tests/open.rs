//! Opening a file and the jump history.

use super::*;

/// #75: the walk is redone while merl runs. The tree cursor keeps its entry and its screen
/// row, an open `o` keeps its rows until it is reopened, and the review panel is not the walk.
#[test]
fn a_new_walk_keeps_the_cursor_row_and_an_open_picker() {
    let (dir, mut a) = files_app("live");
    let walk = |a: &mut App| {
        let (tree, files) = crate::tree::build(&a.root);
        a.project_walked(tree, files);
    };
    walk(&mut a);
    a.tree.reveal(Path::new("b.rs"));
    std::fs::write(dir.join("a2.rs"), "x\n").unwrap();
    walk(&mut a);
    assert_eq!(
        a.tree_top, 0,
        "a panel showing its first row keeps showing it"
    );
    std::fs::remove_file(dir.join("a2.rs")).unwrap();
    walk(&mut a);
    a.tree_top = 1;
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);

    std::fs::write(dir.join("a0.rs"), "x\n").unwrap();
    std::fs::remove_file(dir.join("a.rs")).unwrap();
    std::fs::write(dir.join("a1.rs"), "x\n").unwrap();
    walk(&mut a);
    assert_eq!(a.files, ["a0.rs", "a1.rs", "b.rs"].map(PathBuf::from));
    assert_eq!(a.tree.selected().unwrap().path, Path::new("b.rs"));
    assert_eq!(
        a.tree_top, 2,
        "one row more above the cursor: the scroll follows"
    );
    let p = a.picker.as_mut().unwrap();
    p.settle();
    assert_eq!(p.counts().1, 2, "the open picker still lists a.rs and b.rs");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
    let p = a.picker.as_mut().unwrap();
    p.settle();
    assert_eq!(p.counts().1, 3);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);

    // Rows leaving above the cursor pull the scroll back, never below the first row.
    a.tree.reveal(Path::new("b.rs"));
    a.tree_top = 2;
    std::fs::remove_file(dir.join("a0.rs")).unwrap();
    walk(&mut a);
    assert_eq!(a.tree_top, 1);
    a.tree_top = 1;
    std::fs::remove_file(dir.join("a1.rs")).unwrap();
    std::fs::write(dir.join("c.rs"), "x\n").unwrap();
    walk(&mut a);
    assert_eq!((a.tree_top, a.tree.cursor), (0, 0));

    let (_, mut r) = review_app("live-review");
    let panel = |r: &App| {
        let rows = r.tree.nodes.iter().map(|n| (n.path.clone(), n.expanded));
        (rows.collect::<Vec<_>>(), r.tree.cursor, r.tree_top)
    };
    let before = panel(&r);
    let (tree, files) = crate::tree::build(&r.root);
    r.project_walked(tree, files.clone());
    assert_eq!((panel(&r), &r.files), (before, &files));
}

/// A file merl cannot write is read-only from the moment it opens, not from the first failed
/// save (#123). Every load asks again, so a `chmod` outside merl is picked up by Ctrl+R, and
/// a reason that tells the reader more keeps its place.
#[test]
#[cfg(unix)]
fn a_file_without_write_permission_is_read_only() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, mut a) = project_app(
        "readonly",
        &[("src/a.rs", "fn x() {}\n"), (".env", "API_KEY=\n")],
    );
    let (env, other) = (dir.join(".env"), dir.join("src/a.rs"));
    let open = |a: &mut App, p: &Path| {
        a.jump_to(&other, 1);
        a.jump_to(p, 1);
    };
    let chmod = |p: &Path, mode| {
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode)).unwrap()
    };
    open(&mut a, &env);
    assert_eq!(a.buf.readonly, None);

    chmod(&env, 0o444);
    open(&mut a, &env);
    // As root every file is writable, and there is nothing to assert.
    if std::fs::OpenOptions::new().write(true).open(&env).is_ok() {
        std::fs::remove_dir_all(&dir).unwrap();
        return;
    }
    assert_eq!(a.buf.readonly, Some("no write permission"));

    // Where the file is says more than what it is missing: the standard library and a
    // dependency are unwritable on most machines, and `outside the project` is the reason
    // that explains them.
    let outside = std::env::temp_dir().join(format!("merl-ro-out-{}", std::process::id()));
    std::fs::write(&outside, "fn x() {}\n").unwrap();
    chmod(&outside, 0o444);
    open(&mut a, &outside);
    assert_eq!(a.buf.readonly, Some("outside the project"));
    chmod(&outside, 0o644);
    std::fs::remove_file(&outside).unwrap();

    // `merl .env`: the file named on the command line never reaches `open`.
    chmod(&env, 0o444);
    let started = App::new(
        dir.clone(),
        Tree::default(),
        Vec::new(),
        Buffer::load(&env).unwrap(),
        None,
    );
    assert_eq!(started.buf.readonly, Some("no write permission"));

    chmod(&env, 0o644);
    open(&mut a, &env);
    assert_eq!(a.buf.readonly, None);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn history_walks_back_and_forward() {
    let (dir, mut a) = files_app("walk");
    let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
    a.jump_to(&x, 1);
    a.jump_to(&y, 1);
    a.jump_to(&y, 5);
    assert_eq!(
        a.history,
        [(x.clone(), 0, 0), (y.clone(), 0, 0), (y.clone(), 4, 0)]
    );
    assert_eq!(a.hist_idx, 2);

    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!((a.buf.path.clone().unwrap(), a.line), (y.clone(), 0));
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!((a.buf.path.clone().unwrap(), a.line), (x.clone(), 0));
    press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
    assert_eq!((a.buf.path.clone().unwrap(), a.line), (y.clone(), 0));
    // Walking does not record anything.
    assert_eq!(a.history.len(), 3);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn history_truncates_forward_and_stops_at_the_ends() {
    let (dir, mut a) = files_app("ends");
    let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(a.message, "start of history");
    press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
    assert_eq!(a.message, "end of history");

    a.jump_to(&x, 1);
    a.jump_to(&y, 1);
    a.jump_to(&y, 5);
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    // A jump from the middle of the history drops everything after it.
    a.jump_to(&y, 8);
    assert_eq!(a.history, [(x, 0, 0), (y, 7, 0)]);
    assert_eq!(a.hist_idx, 1);
    press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
    assert_eq!(a.message, "end of history");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn repeated_jumps_to_the_same_spot_are_not_recorded() {
    let (dir, mut a) = files_app("dup");
    let x = dir.join("a.rs");
    a.jump_to(&x, 3);
    a.jump_to(&x, 3);
    a.jump_to(&x, 3);
    assert_eq!(a.history, [(x, 2, 0)]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn far_moves_become_stops_and_near_moves_update_the_current_one() {
    let (dir, mut a) = files_app("wander");
    let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
    a.jump_to(&x, 1);
    a.jump_to(&y, 1);
    // Three lines down is still "here": the current stop follows the cursor.
    for _ in 0..3 {
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    }
    assert_eq!(a.history, [(x.clone(), 0, 0), (y.clone(), 3, 0)]);
    // The end of the file (36 lines away) is somewhere else: a new stop.
    press(&mut a, KeyCode::End, KeyModifiers::CONTROL);
    assert_eq!(
        a.history,
        [(x.clone(), 0, 0), (y.clone(), 3, 0), (y.clone(), 39, 1)]
    );
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(at(&a), (y.clone(), 3));
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(at(&a), (x.clone(), 0));
    press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
    assert_eq!(at(&a), (y, 39));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Ctrl+D/U and PageUp/Down are scrolling, not jumps: however far a page is, the current
/// stop follows the cursor, so one `[` is back at the call site and `]` still works after
/// reading around it.
#[test]
fn paging_moves_the_current_stop_instead_of_adding_stops() {
    let (dir, mut a) = files_app("paging");
    let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
    a.jump_to(&x, 1);
    press(&mut a, KeyCode::Char('d'), KeyModifiers::CONTROL);
    press(&mut a, KeyCode::Char('d'), KeyModifiers::CONTROL);
    a.jump_to(&y, 1);
    press(&mut a, KeyCode::PageDown, KeyModifiers::NONE);
    assert_eq!(a.history, [(x.clone(), 24, 0), (y.clone(), 24, 0)]);
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(at(&a), (x.clone(), 24));
    press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
    press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
    assert_eq!(at(&a), (y.clone(), 24));
    assert_eq!(a.history, [(x, 12, 0), (y, 24, 0)]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn wandering_after_going_back_moves_that_stop() {
    let (dir, mut a) = files_app("stale");
    let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
    a.jump_to(&x, 1);
    a.jump_to(&y, 1);
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    for _ in 0..4 {
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    }
    press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(
        at(&a),
        (x, 4),
        "back lands where the cursor was left, not where it arrived"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn find_next_is_a_stop_even_when_near() {
    let (dir, mut a) = files_app("find");
    let x = dir.join("a.rs");
    a.jump_to(&x, 1);
    find(&mut a, "x");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('n'), KeyModifiers::NONE);
    assert_eq!(a.history, [(x.clone(), 0, 0), (x, 1, 0)]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_jump_that_goes_nowhere_keeps_the_forward_history() {
    let (dir, mut a) = files_app("noop");
    let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
    a.jump_to(&x, 1);
    a.jump_to(&y, 1);
    a.jump_to(&y, 5);
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    a.jump_to(&x, 1);
    assert_eq!(a.history.len(), 3);
    press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
    assert_eq!(at(&a), (y, 0));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn enter_on_the_open_file_in_the_tree_keeps_the_cursor() {
    let (dir, mut a) = files_app("tree");
    let x = dir.join("a.rs");
    a.tree = crate::tree::build(&dir).0;
    a.jump_to(&x, 5); // also puts the tree cursor on a.rs
    a.focus = Focus::Tree;
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((at(&a), a.focus), ((x.clone(), 4), Focus::Code));
    assert_eq!(a.history, [(x, 4, 0)]);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Picking the open file in the file picker keeps the cursor, like Enter in the tree: the
/// picker's items carry no line, and line 1 is not where the user was. The tree cursor
/// still lands on the file.
#[test]
fn picking_the_open_file_keeps_the_cursor() {
    let (dir, mut a) = files_app("pick");
    let x = dir.join("a.rs");
    a.tree = crate::tree::build(&dir).0;
    a.files = vec![PathBuf::from("a.rs"), PathBuf::from("b.rs")];
    a.jump_to(&x, 5);
    a.tree.reveal(Path::new("b.rs"));
    a.focus = Focus::Tree;
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        (at(&a), a.focus, a.mode),
        ((x.clone(), 4), Focus::Code, Mode::Normal)
    );
    assert_eq!(a.tree.selected().unwrap().path, Path::new("a.rs"));
    assert_eq!(a.history, [(x, 4, 0)]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn history_keeps_the_last_fifty_stops() {
    let (dir, mut a) = files_app("cap");
    let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
    for _ in 0..30 {
        a.jump_to(&x, 1);
        a.jump_to(&y, 1);
    }
    assert_eq!((a.history.len(), a.hist_idx), (50, 49));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The find anchor, the selection anchor and the history stops are byte positions taken
/// before a reload; reading them back must clamp to the file as it is now.
#[test]
fn stored_positions_survive_a_reload() {
    let dir = std::env::temp_dir().join(format!("merl-stale-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("f.txt");
    std::fs::write(&path, "abcdefghij\n".repeat(50)).unwrap();
    let mut a = App::new(
        dir.clone(),
        Tree::default(),
        Vec::new(),
        Buffer::load(&path).unwrap(),
        None,
    );
    a.jump_to(&path, 40);
    a.col = 8;
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    std::fs::write(&path, "éé\ny\n").unwrap();
    a.reload(false);
    // Empty query: back to the (clamped) find anchor.
    typed(&mut a, "z");
    press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (1, 0));
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    // A plain arrow collapses the selection to its clamped anchor.
    press(&mut a, KeyCode::Left, KeyModifiers::NONE);
    assert_eq!((a.line, a.col, a.selection()), (1, 0, None));
    // `[` onto the stop at line 40 lands on the clamped line, rewrites that stop, and keeps
    // the forward history instead of reading the clamp as a new move.
    a.jump_to(&path, 1);
    assert_eq!(a.history.len(), 4);
    assert_eq!(a.history[1], (path.clone(), 40, 0));
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!((a.line, a.col, a.hist_idx), (1, 0, 1));
    assert_eq!(a.history[1], (path.clone(), 1, 0));
    assert_eq!(a.history.len(), 4);
    press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
    assert_eq!(a.hist_idx, 2);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// An agent renames a file: its stops are dropped and `[` goes on to the next one that
/// opens, instead of retrying the dead stop with everything behind it out of reach.
#[test]
fn a_stop_whose_file_is_gone_is_dropped_and_walked_past() {
    let (dir, mut a) = files_app("gone");
    let (x, y, z) = (dir.join("a.rs"), dir.join("b.rs"), dir.join("c.rs"));
    std::fs::write(&z, "x\n".repeat(40)).unwrap();
    a.jump_to(&z, 3);
    a.jump_to(&x, 5);
    a.jump_to(&x, 30);
    a.jump_to(&y, 1);
    std::fs::remove_file(&x).unwrap();
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(
        (at(&a), a.hist_idx, a.history.len()),
        ((z.clone(), 2), 0, 2)
    );
    assert_eq!(a.message, "a.rs gone");
    press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
    assert_eq!(
        (at(&a), a.hist_idx, a.message.as_str()),
        ((y.clone(), 0), 1, "")
    );
    // `]` drops them the same way, two files at once; a walk that runs off the end says
    // what it dropped first, and then that this is the end.
    let w = dir.join("d.rs");
    for p in [&x, &w] {
        std::fs::write(p, "x\n".repeat(40)).unwrap();
        a.jump_to(p, 9);
    }
    a.jump_to(&y, 20);
    for _ in 0..3 {
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    }
    assert_eq!(
        (at(&a), a.hist_idx, a.history.len()),
        ((y.clone(), 0), 1, 5)
    );
    std::fs::remove_file(&x).unwrap();
    std::fs::remove_file(&w).unwrap();
    press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
    assert_eq!(
        (at(&a), a.hist_idx, a.history.len()),
        ((y.clone(), 19), 2, 3)
    );
    assert_eq!(a.message, "2 files gone");
    std::fs::remove_file(&z).unwrap();
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(
        (at(&a), a.hist_idx, a.history.len()),
        ((y.clone(), 0), 0, 2)
    );
    assert_eq!(a.message, "c.rs gone");
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(a.message, "start of history");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// b, a, b with `a` gone would leave `b` next to itself: a press that moves nothing.
#[test]
fn dropping_a_stop_does_not_leave_its_neighbours_as_twins() {
    let (dir, mut a) = files_app("twins");
    let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
    a.jump_to(&y, 1);
    a.jump_to(&x, 5);
    a.jump_to(&y, 1);
    std::fs::remove_file(&x).unwrap();
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!((at(&a), a.hist_idx, a.history.len()), ((y, 0), 0, 1));
    assert_eq!(a.message, "a.rs gone");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Edits that cannot be saved keep merl on the file; that is no reason to drop a stop.
#[test]
fn a_stop_that_does_not_open_for_another_reason_leaves_the_history_alone() {
    let (dir, mut a) = files_app("stuck");
    let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
    a.jump_to(&x, 5);
    a.jump_to(&y, 1);
    (a.dirty, a.conflict) = (true, true);
    std::fs::remove_file(&x).unwrap();
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(
        (at(&a), a.hist_idx, a.history.len()),
        ((y.clone(), 0), 1, 2)
    );
    // Nor is a file that is there and does not open.
    (a.dirty, a.conflict) = (false, false);
    std::fs::create_dir(&x).unwrap();
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!((at(&a), a.hist_idx, a.history.len()), ((y, 0), 1, 2));
    assert!(a.message.contains("a.rs: "), "{}", a.message);
    std::fs::remove_dir_all(&dir).unwrap();
}
