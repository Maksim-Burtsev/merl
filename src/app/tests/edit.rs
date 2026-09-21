//! Editing the buffer: typing, pasting, undo, reload and saving.

use super::*;

#[test]
fn a_paste_types_into_prompts_and_pickers_but_not_navigation() {
    let mut a = app("foo\n");
    a.paste("so");
    assert_eq!((a.mode, a.picker.is_none()), (Mode::Normal, true));
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    a.paste("parse_it\r\nsecond line");
    assert_eq!(a.mode, Mode::Picker(PickerKind::Search));
    assert_eq!(&*a.picker.as_ref().unwrap().query, "parse_it");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
    a.paste("app.rs");
    assert_eq!(&*a.picker.as_ref().unwrap().query, "app.rs");
}

#[test]
fn enter_edits_and_letters_are_text_until_esc() {
    let mut a = app("def f():\n    pass\n");
    typed(&mut a, "s");
    assert_eq!(
        a.mode,
        Mode::Picker(PickerKind::Search),
        "letters navigate outside edit mode"
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::End, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(a.mode, Mode::Edit);
    typed(&mut a, "  # sq");
    assert_eq!(a.line_str(), "def f():  # sq");
    assert!(a.dirty);
    // Enter keeps the indentation of the line it leaves; Backspace at column 0 joins.
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    press(&mut a, KeyCode::End, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (2, 4));
    assert_eq!(a.buf.lines[2], "    ");
    press(&mut a, KeyCode::Tab, KeyModifiers::NONE);
    typed(&mut a, "x");
    assert_eq!(a.buf.lines[2], "        x");
    press(&mut a, KeyCode::Home, KeyModifiers::NONE);
    press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(a.buf.lines, vec!["def f():  # sq", "    pass        x"]);
    press(&mut a, KeyCode::Delete, KeyModifiers::NONE);
    press(&mut a, KeyCode::Delete, KeyModifiers::NONE);
    assert_eq!(a.buf.lines[1], "    pass      x");
    // Delete at the end of the last line has nothing to join.
    press(&mut a, KeyCode::End, KeyModifiers::NONE);
    press(&mut a, KeyCode::Delete, KeyModifiers::NONE);
    assert_eq!(a.buf.lines.len(), 2);
    assert!(
        !press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE),
        "q types"
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(a.mode, Mode::Normal);
    assert!(
        press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE),
        "q quits again"
    );
}

#[test]
fn tab_follows_the_file_and_unicode_edits_stay_on_boundaries() {
    let mut a = app("\tif x:\n\t\tpass\n");
    assert!(a.buf.tabs);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Tab, KeyModifiers::NONE);
    typed(&mut a, "é");
    assert_eq!(a.line_str(), "\té\tif x:");
    assert_eq!(a.display_col(), 6);
    press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
    press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(a.line_str(), "\tif x:");
}

#[test]
fn alt_backspace_and_alt_delete_take_a_word_in_one_undo_step() {
    let mut a = app("x_1 = да мир;\nnext\n");
    press(&mut a, KeyCode::End, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "!");
    press(&mut a, KeyCode::Backspace, KeyModifiers::ALT);
    assert_eq!(a.buf.lines[0], "x_1 = да ");
    // The word alone comes back, with the cursor where the key was pressed.
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(
        (a.buf.lines[0].as_str(), a.col),
        ("x_1 = да мир;!", "x_1 = да мир;!".len())
    );
    press(&mut a, KeyCode::Home, KeyModifiers::NONE);
    press(&mut a, KeyCode::Delete, KeyModifiers::ALT);
    press(&mut a, KeyCode::Delete, KeyModifiers::ALT);
    assert_eq!(a.buf.lines[0], " мир;!");
    // At the start of a line it joins the line above, as Alt+Left goes there.
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    press(&mut a, KeyCode::Home, KeyModifiers::NONE);
    press(&mut a, KeyCode::Backspace, KeyModifiers::ALT);
    assert_eq!(a.buf.lines, vec![" мир;!next"]);
}

#[test]
fn undo_groups_typing_and_redo_replays_it() {
    let mut a = app("ab\ncd\n");
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(a.message, "nothing to undo");
    press(&mut a, KeyCode::End, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "12");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "34");
    press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(a.buf.lines, vec!["ab12", "3", "cd"]);
    // A run of keystrokes on one line is one step; the split and the next run are others.
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(a.buf.lines, vec!["ab12", "", "cd"]);
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(
        (a.buf.lines.clone(), a.line, a.col),
        (vec!["ab12".to_string(), "cd".into()], 0, 4)
    );
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(a.buf.lines, vec!["ab", "cd"]);
    assert!(a.dirty);
    press(&mut a, KeyCode::Char('y'), KeyModifiers::CONTROL);
    press(&mut a, KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert_eq!(
        (a.buf.lines.clone(), a.line, a.col),
        (vec!["ab12".to_string(), "".into(), "cd".into()], 1, 0)
    );
    // A new edit drops the redo stack; a cursor move starts a new step.
    typed(&mut a, "x");
    press(&mut a, KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert_eq!(a.message, "nothing to redo");
    press(&mut a, KeyCode::Up, KeyModifiers::NONE);
    typed(&mut a, "!");
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(a.buf.lines, vec!["ab12", "x", "cd"]);
    // Leaving and re-entering edit mode also ends the step; undo works from navigation.
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "y");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(a.buf.lines, vec!["ab12", "x", "cd"]);
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(a.buf.lines, vec!["ab12", "", "cd"]);
}

#[test]
fn typing_replaces_the_selection_and_the_clipboard_keys_copy_or_cut() {
    let mut a = app("abc\ndef\nghi\n");
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    assert_eq!(a.selection(), Some(((0, 1), (1, 1))));
    press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(a.clipboard.take().as_deref(), Some("bc\nd"));
    assert_eq!(a.message, "copied 2 lines");
    assert_eq!(a.buf.lines.len(), 3, "copy leaves the text alone");
    assert!(a.selection().is_some(), "and the selection");
    typed(&mut a, "X");
    assert_eq!(a.buf.lines, vec!["aXef", "ghi"]);
    assert_eq!((a.line, a.col, a.selection()), (0, 2, None));
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(a.buf.lines, vec!["abc", "def", "ghi"], "one step");
    assert_eq!((a.line, a.col), (1, 1), "undo puts the cursor where it was");
    // Delete on a selection removes it; Ctrl+X without one cuts the line.
    press(&mut a, KeyCode::Up, KeyModifiers::NONE);
    press(
        &mut a,
        KeyCode::Right,
        KeyModifiers::SHIFT | KeyModifiers::CONTROL,
    );
    press(&mut a, KeyCode::Delete, KeyModifiers::NONE);
    assert_eq!(a.buf.lines[0], "a");
    press(&mut a, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert_eq!(a.clipboard.take().as_deref(), Some("a\n"));
    assert_eq!(a.buf.lines, vec!["def", "ghi"]);
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert_eq!(a.clipboard.take().as_deref(), Some("ghi"));
    assert_eq!(a.buf.lines, vec!["def", ""]);
    // Pasted text is inserted only while editing, with CRLF normalised.
    a.paste("p\r\nq");
    assert_eq!(a.buf.lines, vec!["def", "p", "q"]);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    a.paste("nope");
    assert_eq!(a.buf.lines, vec!["def", "p", "q"]);
}

#[test]
fn readonly_and_clipped_lines_refuse_to_edit() {
    let mut a = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/x"), b"a\0b"),
        None,
    );
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        (a.mode, a.message.as_str()),
        (Mode::Normal, "read-only: binary file")
    );
    let mut a = app(&"x".repeat(30_000));
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        (a.mode, a.message.as_str()),
        (Mode::Normal, "line too long to edit")
    );
    let mut a = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::empty(),
        None,
    );
    a.focus = Focus::Code;
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        a.mode,
        Mode::Normal,
        "nothing to edit on the welcome screen"
    );
}

#[test]
fn overlays_opened_while_editing_go_back_to_editing() {
    let mut a = app("one\ntwo\nthree two\n");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    // Find, Enter: editing goes on at the match.
    press(&mut a, KeyCode::Char('f'), KeyModifiers::CONTROL);
    typed(&mut a, "two");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((a.mode, a.line), (Mode::Edit, 1));
    typed(&mut a, "d");
    assert_eq!(a.buf.lines[1], "dtwo");
    // Find, Esc: the cursor goes back, and so does the mode.
    press(&mut a, KeyCode::Char('f'), KeyModifiers::CONTROL);
    typed(&mut a, "three");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!((a.mode, a.line, a.col), (Mode::Edit, 1, 1));
    // Goto and a cancelled prompt.
    press(&mut a, KeyCode::Char('g'), KeyModifiers::CONTROL);
    typed(&mut a, "3");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((a.mode, a.line), (Mode::Edit, 2));
    press(&mut a, KeyCode::Char('g'), KeyModifiers::CONTROL);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(a.mode, Mode::Edit);
    // Opened from navigation, find still ends in navigation.
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    typed(&mut a, "one");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((a.mode, a.line), (Mode::Normal, 0));
}

#[test]
fn autosave_esc_and_quit_reach_the_disk() {
    let (path, mut a) = temp_file("autosave", "a\r\nb\r\n");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "x");
    assert!(a.dirty);
    assert!(!a.tick(), "the delay has not passed");
    a.autosave = Duration::ZERO;
    assert!(a.tick());
    assert!(!a.dirty);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        b"xa\r\nb\r\n",
        "CRLF survives"
    );
    a.autosave = Duration::from_secs(60);
    typed(&mut a, "y");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(std::fs::read(&path).unwrap(), b"xya\r\nb\r\n");
    // Undo from navigation dirties the buffer again; quitting flushes it.
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert!(a.dirty);
    assert!(press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        b"a\r\nb\r\n",
        "x and y were one step"
    );
    // merl's own save is not a change to react to.
    a.reload(false);
    assert_eq!(a.message, "");
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn a_change_under_unsaved_edits_is_a_conflict() {
    let (path, mut a) = temp_file("conflict", "one\n");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "mine ");
    std::fs::write(&path, "theirs\n").unwrap();
    a.reload(false);
    assert!(a.conflict);
    assert_eq!(a.line_str(), "mine one", "the buffer keeps the edits");
    a.autosave = Duration::ZERO;
    assert!(!a.tick(), "autosave is off while conflicted");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "theirs\n");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "theirs\n",
        "Esc does not overwrite"
    );
    press(&mut a, KeyCode::Char('s'), KeyModifiers::CONTROL);
    assert!(!a.conflict && !a.dirty);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "mine one\n");
    // Ctrl+R takes the disk's version, edits and all.
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "again ");
    let dropped = a.line_str().to_string();
    std::fs::write(&path, "theirs\n").unwrap();
    a.reload(false);
    assert!(a.conflict);
    press(&mut a, KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert_eq!(
        (a.conflict, a.dirty, a.line_str()),
        (false, false, "theirs")
    );
    // #122: the reload is a step, so the edits Ctrl+R dropped are one Ctrl+Z away, and then
    // they are edits like any other: the autosave writes them over the disk's version.
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!((a.dirty, a.line_str()), (true, dropped.as_str()));
    assert!(a.tick());
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        format!("{dropped}\n")
    );
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn switching_files_flushes_and_leaves_edit_mode() {
    let (path, mut a) = temp_file("switch", "one\n");
    let other = path.with_file_name("g.py");
    std::fs::write(&other, "two\n").unwrap();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "x");
    a.jump_to(&other, 1);
    assert_eq!((a.mode, a.dirty), (Mode::Normal, false));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "xone\n");
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn read_only_buffers_are_never_changed_or_written() {
    let (path, mut a) = temp_file("readonly", "one\n");
    // Another tool re-encodes the file while it is in edit mode with nothing unsaved.
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    std::fs::write(&path, b"caf\xe9\n").unwrap();
    a.reload(false);
    typed(&mut a, "x");
    assert_eq!(
        (a.dirty, a.message.as_str()),
        (false, "read-only: not UTF-8")
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('s'), KeyModifiers::CONTROL);
    assert_eq!(a.message, "read-only: not UTF-8");
    assert_eq!(std::fs::read(&path).unwrap(), b"caf\xe9\n");
    // Nor does a binary file's placeholder text reach the disk.
    std::fs::write(&path, b"\x89PNG\0\x01").unwrap();
    a.reload(false);
    press(&mut a, KeyCode::Char('s'), KeyModifiers::CONTROL);
    assert_eq!(std::fs::read(&path).unwrap(), b"\x89PNG\0\x01");
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn edits_that_touch_or_make_a_clipped_line_are_refused() {
    let mut a = app(&format!("ab\n{}\n", "x".repeat(20_000)));
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    a.paste(&"y".repeat(20_000));
    assert_eq!(
        (a.line_str(), a.message.as_str()),
        ("ab", "line too long to edit")
    );
    // A join into a line exactly as long as the screen shows.
    press(&mut a, KeyCode::End, KeyModifiers::NONE);
    press(&mut a, KeyCode::Delete, KeyModifiers::NONE);
    assert_eq!(
        (a.buf.lines.len(), a.message.as_str()),
        (2, "line too long to edit")
    );
    assert!(!a.dirty);
}

#[test]
fn a_save_never_overwrites_a_change_it_has_not_seen() {
    let (path, mut a) = temp_file("unseen", "one\n");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "mine ");
    // The other write lands before its watcher event is handled, or with no watcher at all.
    std::fs::write(&path, "theirs\n").unwrap();
    a.autosave = Duration::ZERO;
    a.tick();
    assert!(a.conflict);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "theirs\n");
    // With the conflict on screen, Ctrl+S is the answer that overwrites.
    press(&mut a, KeyCode::Char('s'), KeyModifiers::CONTROL);
    assert!(!a.conflict && !a.dirty);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "mine one\n");
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn edits_that_cannot_be_saved_keep_merl_on_the_file() {
    let (path, mut a) = temp_file("keep", "one\n");
    let other = path.with_file_name("g.py");
    std::fs::write(&other, "two\n").unwrap();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "mine ");
    std::fs::write(&path, "theirs\n").unwrap();
    a.reload(false);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    // In a conflict another file does not open over the edits, and the first quit is refused.
    a.jump_to(&other, 1);
    assert_eq!((at(&a).0, a.line_str()), (path.clone(), "mine one"));
    assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
    assert!(a.message.contains("q again"), "{}", a.message);
    assert!(press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "theirs\n");
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();

    // A save that fails holds merl the same way, and a key in between asks again.
    let (path, mut a) = temp_file("keep-gone", "one\n");
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "x");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert!(a.message.starts_with("save failed"), "{}", a.message);
    assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
    assert!(press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
}

/// README: a file that changes on disk under unsaved edits is "neither reloaded nor
/// overwritten". Deleted or renamed is changed: only Ctrl+S puts it back.
#[test]
fn autosave_does_not_recreate_a_file_that_is_gone() {
    let (path, mut a) = temp_file("gone-save", "one\n");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "X");
    std::fs::remove_file(&path).unwrap();
    a.last_edit = Some(Instant::now() - a.autosave);
    assert!(a.tick());
    assert!(!a.flush());
    assert!(!path.exists());
    assert!(a.conflict && a.dirty);
    press(&mut a, KeyCode::Char('s'), KeyModifiers::CONTROL);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "Xone\n");
    assert!(!a.conflict && !a.dirty);
    // Back as merl last saw it (a writer's delete-then-write) is no conflict any more.
    typed(&mut a, "Y");
    std::fs::remove_file(&path).unwrap();
    a.save();
    assert!(a.conflict);
    std::fs::write(&path, "Xone\n").unwrap();
    a.reload(false);
    assert!(!a.conflict && a.dirty);
    // Ctrl+R has no disk version to take; the edits go all the same, or nothing but
    // writing the file back would let merl off it.
    std::fs::remove_file(&path).unwrap();
    a.save();
    press(&mut a, KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert!(!a.conflict && !a.dirty && !path.exists());
    assert!(a.flush());
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn reload_clamps_the_cursor_into_a_shrunken_file() {
    let dir = std::env::temp_dir().join(format!("merl-reload-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("f.txt");
    std::fs::write(&path, "line\n".repeat(10)).unwrap();
    let mut a = App::new(
        dir.clone(),
        Tree::default(),
        Vec::new(),
        Buffer::load(&path).unwrap(),
        None,
    );
    a.view_w = 20;
    a.view_h = 10;
    a.line = 9;
    a.col = 4;

    // An event that changes nothing must not even report a reload.
    a.reload(false);
    assert_eq!(a.message, "");

    // Outside a review the cursor keeps its line number when lines are written above it.
    a.line = 3;
    std::fs::write(&path, "new\n".to_string() + &"line\n".repeat(10)).unwrap();
    a.reload(false);
    assert_eq!(a.line, 3);
    a.line = 9;

    std::fs::write(&path, "a\nb\nc\n").unwrap();
    a.reload(false);
    assert_eq!(a.buf.lines.len(), 3);
    assert_eq!(a.line, 2);
    assert_eq!(a.col, 1);
    assert_eq!(a.message, "reloaded");
    std::fs::remove_dir_all(&dir).unwrap();
}

fn ctrl(a: &mut App, c: char) {
    press(a, KeyCode::Char(c), KeyModifiers::CONTROL);
}

/// #122, the issue's steps: a write from outside is one step of the history, as in VS Code.
/// Ctrl+Z takes it back first, then the edits before it, and Ctrl+Y replays both.
#[test]
fn a_reload_is_an_undo_step_over_the_edits_before_it() {
    let (path, mut a) = temp_file("undo-reload", "one\ntwo\n");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "MINE");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    std::fs::write(&path, "MINEone\ntwo\nagent line\n").unwrap();
    assert!(a.reload(false));
    assert_eq!(a.message, "reloaded");
    let step = a.undo.last().unwrap();
    assert_eq!(
        (step.line, step.old.len(), step.new.len()),
        (2, 0, 1),
        "the step holds what changed, not the file"
    );
    ctrl(&mut a, 'z');
    assert_eq!(
        (a.buf.lines.join("|"), a.line),
        ("MINEone|two".into(), 1),
        "undo lands where the file changed"
    );
    ctrl(&mut a, 'z');
    assert_eq!(a.buf.lines.join("|"), "one|two");
    ctrl(&mut a, 'y');
    ctrl(&mut a, 'y');
    assert_eq!(a.buf.lines.join("|"), "MINEone|two|agent line");
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

/// #163, the issue's steps: a file left keeps its history, as a VS Code tab does. Written on
/// disk while away, it comes back with the write as one more step on top.
#[test]
fn leaving_a_file_keeps_its_undo_history() {
    let (dir, mut a) = project_app(
        "undo-away",
        &[
            ("a.py", "x = 1\nhelper()\n"),
            ("b.py", "def helper():\n    pass\n"),
        ],
    );
    a.external
        .insert(Kind::Python, (Vec::new(), Arc::new(Vec::new())));
    let away_and_back = |a: &mut App| {
        a.jump_to(&dir.join("a.py"), 2);
        press(a, KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(at(a), (dir.join("b.py"), 0), "{}", a.message);
        press(a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(at(a).0, dir.join("a.py"));
    };
    a.jump_to(&dir.join("a.py"), 1);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "MINE");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    away_and_back(&mut a);
    ctrl(&mut a, 'z');
    assert_eq!(a.buf.lines.join("|"), "x = 1|helper()");
    away_and_back(&mut a);
    ctrl(&mut a, 'y');
    assert_eq!(a.buf.lines.join("|"), "MINEx = 1|helper()", "redo too");
    a.jump_to(&dir.join("b.py"), 1);
    std::fs::write(dir.join("a.py"), "MINEx = 1\nhelper()\nagent line\n").unwrap();
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    ctrl(&mut a, 'z');
    assert_eq!(a.buf.lines.join("|"), "MINEx = 1|helper()");
    ctrl(&mut a, 'z');
    assert_eq!(a.buf.lines.join("|"), "x = 1|helper()");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// What is typed after a reload is a step of its own, also where the reload's step ends.
#[test]
fn typing_after_a_reload_is_a_step_of_its_own() {
    let (path, mut a) = temp_file("undo-reload-typing", "one\ntwo\n");
    a.autosave = Duration::ZERO;
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert!(a.tick());
    std::fs::write(&path, "\nONE\ntwo\n").unwrap();
    a.reload(false);
    assert_eq!((a.line, a.col), (1, 0));
    typed(&mut a, "x");
    ctrl(&mut a, 'z');
    assert_eq!(a.buf.lines.join("|"), "|ONE|two");
    ctrl(&mut a, 'z');
    assert_eq!(a.buf.lines.join("|"), "|one|two");
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

/// Undoing a reload puts the file's format back with its text. A reload to what no save can
/// write back as it came is undone, never redone; one from it starts a new history.
#[test]
fn undoing_a_reload_puts_the_format_back() {
    let (path, mut a) = temp_file("undo-format", "a\r\nb\r\n");
    a.autosave = Duration::ZERO;
    std::fs::write(&path, "a\nb\nc").unwrap();
    a.reload(false);
    ctrl(&mut a, 'z');
    assert_eq!(
        a.buf.to_bytes(),
        b"a\r\nb\r\n",
        "CRLF and the final newline"
    );
    ctrl(&mut a, 'y');
    assert_eq!(a.buf.to_bytes(), b"a\nb\nc");
    assert!(a.tick());
    std::fs::write(&path, "a\r\nb\nc\r\n").unwrap();
    a.reload(false);
    assert_eq!(a.buf.readonly, Some("mixed line endings"));
    ctrl(&mut a, 'z');
    assert_eq!(
        (a.buf.readonly, a.buf.to_bytes()),
        (None, b"a\nb\nc".to_vec())
    );
    ctrl(&mut a, 'y');
    assert_eq!(a.message, "nothing to redo");
    assert!(a.tick());
    std::fs::write(&path, "\0").unwrap();
    a.reload(false);
    std::fs::write(&path, "t\n").unwrap();
    a.reload(false);
    ctrl(&mut a, 'z');
    assert_eq!(a.message, "nothing to undo");
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

/// A dependency changed on disk (a `pip install -U`) or reloaded with Ctrl+R stays read-only.
#[test]
fn a_reload_keeps_a_file_outside_the_project_read_only() {
    let (path, mut a) = temp_file("reload-outside", "x\n");
    let outside = std::env::temp_dir().join(format!("merl-dep-{}.py", std::process::id()));
    std::fs::write(&outside, "def x(): pass\n").unwrap();
    a.jump_to(&outside, 1);
    std::fs::write(&outside, "def y(): pass\n").unwrap();
    a.reload(false);
    assert_eq!(a.buf.readonly, Some("outside the project"));
    std::fs::remove_file(&outside).unwrap();
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}
