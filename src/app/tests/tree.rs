//! The file tree and the new file prompt.

use super::*;

#[test]
fn tree_focus_keys_do_not_move_the_code_cursor() {
    let mut a = app("aaa\nbbb\nccc\n");
    a.focus = Focus::Tree;
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(a.line, 0, "Down belongs to the tree while it has focus");
    press(&mut a, KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(a.focus, Focus::Code);
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(a.line, 1);
    // `t` hides the tree and hands the keys to the code pane for good.
    a.focus = Focus::Tree;
    press(&mut a, KeyCode::Char('t'), KeyModifiers::NONE);
    assert!(!a.show_tree);
    assert_eq!(a.focus, Focus::Code);
    press(&mut a, KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(a.focus, Focus::Code);
}

/// A project on disk, `src/a.py` and `README.md`, walked as `main` walks it; `src/a.py` open.
fn new_file_project(name: &str) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-new-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/a.py"), "x = 1\n").unwrap();
    std::fs::write(dir.join("README.md"), "hi\n").unwrap();
    let dir = dir.canonicalize().unwrap();
    let (tree, files) = crate::tree::build(&dir, false);
    let buf = Buffer::load(&dir.join("src/a.py")).unwrap();
    let mut a = App::new(dir.clone(), tree, files, buf, None);
    a.focus = Focus::Code;
    a.view_w = 40;
    a.view_h = 10;
    (dir, a)
}

fn ctrl_n(a: &mut App) {
    press(a, KeyCode::Char('n'), KeyModifiers::CONTROL);
}

#[test]
fn ctrl_n_creates_a_file_next_to_the_open_one_and_edits_it() {
    let (dir, mut a) = new_file_project("code");
    ctrl_n(&mut a);
    assert_eq!((a.mode, &*a.prompt), (Mode::New, "src/"));
    typed(&mut a, "sub/b.py");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    let made = dir.join("src/sub/b.py");
    assert_eq!(std::fs::read_to_string(&made).unwrap(), "");
    assert_eq!((a.buf.path.as_deref(), a.mode), (Some(&*made), Mode::Edit));
    // In the file list and the tree at once, with no watcher to wait for.
    let rel = Path::new("src/sub/b.py");
    assert!(a.files.iter().any(|f| f == rel));
    assert_eq!(a.tree.selected().map(|n| &*n.path), Some(rel));
    // `[` is the way back.
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(a.buf.path.as_deref(), Some(&*dir.join("src/a.py")));
}

#[test]
fn ctrl_n_in_the_tree_starts_from_the_selected_row() {
    let (_, mut a) = new_file_project("tree");
    a.focus = Focus::Tree;
    a.tree.reveal(Path::new("README.md"));
    ctrl_n(&mut a);
    assert_eq!(&*a.prompt, "");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    a.tree.reveal(Path::new("src/a.py"));
    ctrl_n(&mut a);
    assert_eq!(&*a.prompt, "src/");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    a.tree.reveal(Path::new("src"));
    ctrl_n(&mut a);
    assert_eq!(&*a.prompt, "src/");
    typed(&mut a, "c.py");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((a.focus, a.mode), (Focus::Code, Mode::Edit));
}

#[test]
fn ctrl_n_from_edit_mode_saves_the_open_file_and_esc_resumes_it() {
    let (dir, mut a) = new_file_project("edit");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "y");
    ctrl_n(&mut a);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(a.mode, Mode::Edit);
    ctrl_n(&mut a);
    typed(&mut a, "b.py");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        std::fs::read_to_string(dir.join("src/a.py")).unwrap(),
        "yx = 1\n"
    );
    assert_eq!(a.buf.path.as_deref(), Some(&*dir.join("src/b.py")));
}

#[test]
fn ctrl_n_never_overwrites_and_never_leaves_the_project() {
    let (dir, mut a) = new_file_project("refuse");
    let new = |a: &mut App, path: &str| {
        ctrl_n(a);
        press(a, KeyCode::Char('u'), KeyModifiers::CONTROL);
        typed(a, path);
        press(a, KeyCode::Enter, KeyModifiers::NONE);
    };
    // A file that is there opens as it is.
    new(&mut a, "README.md");
    assert_eq!(a.buf.path.as_deref(), Some(&*dir.join("README.md")));
    assert_eq!(
        std::fs::read_to_string(dir.join("README.md")).unwrap(),
        "hi\n"
    );
    for (path, why) in [
        ("../out.py", "outside the project"),
        ("/tmp/merl-out.py", "outside the project"),
        ("src/../../out.py", "outside the project"),
        ("src/", "no file name"),
        ("src", "src is a directory"),
    ] {
        new(&mut a, path);
        assert_eq!(a.message, why, "{path}");
    }
    assert!(!dir.parent().unwrap().join("out.py").exists());
    // An empty prompt is a cancel, as in `:`.
    new(&mut a, "");
    assert_eq!(a.message, "");
}

/// #404: the directories on the typed path that exist are resolved, links included. A link out
/// of the project is refused as a `..` out of it is, and nothing is created; a link that stays
/// inside is followed.
#[cfg(unix)]
#[test]
fn ctrl_n_resolves_the_links_on_the_path() {
    let (dir, mut a) = new_file_project("links");
    let outside = dir.with_extension("outside");
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, dir.join("out")).unwrap();
    std::os::unix::fs::symlink("src", dir.join("inside")).unwrap();
    let new = |a: &mut App, path: &str| {
        ctrl_n(a);
        press(a, KeyCode::Char('u'), KeyModifiers::CONTROL);
        typed(a, path);
        press(a, KeyCode::Enter, KeyModifiers::NONE);
    };
    for path in [
        "out/created.txt",
        "out/deep/created.txt",
        "src/../out/x.txt",
    ] {
        new(&mut a, path);
        assert_eq!(a.message, "outside the project", "{path}");
    }
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
    // A link to a file out there is refused as well, not opened (#448).
    std::fs::write(outside.join("far.py"), "far = 1\n").unwrap();
    std::os::unix::fs::symlink(outside.join("far.py"), dir.join("far.py")).unwrap();
    new(&mut a, "far.py");
    assert_eq!(a.message, "outside the project");
    assert_eq!(a.buf.path.as_deref(), Some(&*dir.join("src/a.py")));
    new(&mut a, "inside/b.py");
    let made = dir.join("src/b.py");
    assert!(made.is_file());
    assert_eq!((a.buf.path.as_deref(), a.mode), (Some(&*made), Mode::Edit));
    assert_eq!(
        a.tree.selected().map(|n| &*n.path),
        Some(Path::new("src/b.py"))
    );
    std::fs::remove_dir_all(&outside).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #405: Ctrl+N on the name of a FIFO that is there opens it as it opens any file that is
/// there, and reading one waited for a writer forever, on the UI thread. Here it runs on a thread
/// of its own, so a wait fails the test instead of hanging it.
#[cfg(unix)]
#[test]
fn ctrl_n_on_a_fifo_does_not_read_it() {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let (dir, mut a) = new_file_project("fifo");
        let mkfifo = std::process::Command::new("mkfifo")
            .arg(dir.join("pipe"))
            .status();
        assert!(mkfifo.unwrap().success());
        ctrl_n(&mut a);
        press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
        typed(&mut a, "pipe");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        let _ = tx.send((a.message.clone(), a.buf.path.clone()));
        std::fs::remove_dir_all(&dir).unwrap();
    });
    let (message, open) = rx.recv_timeout(std::time::Duration::from_secs(60)).unwrap();
    assert!(message.ends_with("/pipe: not a regular file"), "{message}");
    assert!(open.is_some_and(|p| p.ends_with("src/a.py")));
}

/// #404: a symlink to a directory expands in the tree, and its files open; one that resolves
/// outside the project opens read-only, as a file `d` reaches out there does. A file behind a
/// link that stays inside opens by its own path: under the link's, `s` and `u` counted it twice.
#[cfg(unix)]
#[test]
fn the_files_behind_a_link_to_a_directory_open_from_the_tree() {
    let (dir, _) = new_file_project("dirlink");
    let outside = dir.with_extension("outside");
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("far.txt"), "far\n").unwrap();
    std::os::unix::fs::symlink(&outside, dir.join("out")).unwrap();
    std::os::unix::fs::symlink("src", dir.join("inside")).unwrap();
    let (tree, files) = crate::tree::build(&dir, false);
    let mut a = App::new(dir.clone(), tree, files, Buffer::empty(), None);
    for (link, file, readonly) in [
        ("inside", "src/a.py", None),
        ("out", "out/far.txt", Some("outside the project")),
    ] {
        a.focus = Focus::Tree;
        a.tree.reveal(Path::new(link));
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            a.buf.path.as_deref(),
            Some(&*dir.join(file)),
            "{}",
            a.message
        );
        assert_eq!(a.buf.readonly, readonly, "{file}");
    }
    std::fs::remove_dir_all(&outside).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #448: a link to a file that resolves outside the project opens read-only, as a file behind a
/// directory link out does, though the walk lists it. One to a file inside stays editable.
#[cfg(unix)]
#[test]
fn a_link_to_a_file_outside_the_project_opens_read_only() {
    let (dir, _) = new_file_project("filelink");
    let outside = dir.with_extension("outside");
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("far.py"), "far = 1\n").unwrap();
    std::os::unix::fs::symlink(outside.join("far.py"), dir.join("out.py")).unwrap();
    std::os::unix::fs::symlink("src/a.py", dir.join("in.py")).unwrap();
    let (tree, files) = crate::tree::build(&dir, false);
    let mut a = App::new(dir.clone(), tree, files, Buffer::empty(), None);
    for (link, readonly) in [("in.py", None), ("out.py", Some("outside the project"))] {
        a.focus = Focus::Tree;
        a.tree.reveal(Path::new(link));
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            a.buf.path.as_deref(),
            Some(&*dir.join(link)),
            "{}",
            a.message
        );
        assert_eq!(a.buf.readonly, readonly, "{link}");
    }
    std::fs::remove_dir_all(&outside).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn ctrl_n_over_an_overlay_does_nothing() {
    let (_, mut a) = new_file_project("overlay");
    for open in ['o', ':', '/', '?'] {
        press(&mut a, KeyCode::Char(open), KeyModifiers::NONE);
        ctrl_n(&mut a);
        assert_ne!(a.mode, Mode::New, "{open}");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    }
}
