#![expect(clippy::too_many_lines, reason = "test tables")]
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;
use crate::app::notes::{Note, format, parse};

const NONE: KeyModifiers = KeyModifiers::NONE;
const CTRL: KeyModifiers = KeyModifiers::CONTROL;
const SHIFT: KeyModifiers = KeyModifiers::SHIFT;

type Keys<'a> = &'a [(KeyCode, KeyModifiers)];

fn screen(a: &mut App) -> String {
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut t = Terminal::new(TestBackend::new(70, 16)).unwrap();
    t.draw(|f| crate::ui::draw(f, a, &theme)).unwrap();
    let buf = t.backend().buffer();
    (0..buf.area.height)
        .map(|y| {
            let row: String = (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect();
            row.trim_end().to_owned() + "\n"
        })
        .collect()
}

fn notes_file(dir: &Path, branch: &str) -> PathBuf {
    dir.join(".git/merl/notes").join(branch)
}

fn read(dir: &Path) -> String {
    std::fs::read_to_string(notes_file(dir, "feature")).unwrap_or_default()
}

fn on_a_rs(tag: &str, line: usize) -> (PathBuf, App) {
    let (dir, mut a) = review_app(tag);
    a.notes.on = true;
    a.show_tree = false;
    a.jump_to(&dir.join("src/a.rs"), line);
    screen(&mut a);
    (dir, a)
}

fn keys(a: &mut App, keys: &[(KeyCode, KeyModifiers)]) {
    for &(k, m) in keys {
        press(a, k, m);
        screen(a);
    }
}

fn note(a: &mut App, text: &str) {
    keys(a, &[(KeyCode::Char('a'), NONE)]);
    typed(a, text);
    keys(a, &[(KeyCode::Esc, NONE)]);
}

#[test]
fn the_notes_file_reads_back_what_was_written() {
    let notes = vec![
        Note {
            path: "app/my file.py".into(),
            first: 9,
            last: 10,
            quote: vec!["> a = 1".into(), "- b = 2".into()],
            text: "# not a header\n> not a quote either\nsecond line".into(),
        },
        Note {
            path: "C:x/y.rs".into(),
            first: 3,
            last: 3,
            quote: vec![">     ".into()],
            text: "- a list item".into(),
        },
        Note {
            path: "b.md".into(),
            first: 1,
            last: 1,
            quote: vec!["> x".into()],
            text: "- first\n- second".into(),
        },
    ];
    let text = format(&notes, "feat/x \u{2190} main \u{b7} abc1234");
    assert!(text.starts_with("# merl notes \u{b7} feat/x \u{2190} main \u{b7} abc1234\n# "));
    assert!(text.contains("\napp/my file.py:9-10\n> a = 1\n- b = 2\n# not a header\n"));
    assert_eq!(parse(&text), notes);
    assert_eq!(parse(""), []);
    assert_eq!(parse("# only the header\n"), []);
}

#[test]
fn a_note_is_typed_edited_deleted_and_undone_like_a_line() {
    let (dir, mut a) = on_a_rs("notes-loop", 3);
    note(&mut a, "why c");
    assert_eq!(a.mode, Mode::Normal);
    assert!(
        read(&dir).ends_with("\nsrc/a.rs:3\n> c\nwhy c\n"),
        "{}",
        read(&dir)
    );
    let s = screen(&mut a);
    let row = s.lines().position(|l| l.contains("why c")).unwrap();
    assert!(s.lines().nth(row - 1).unwrap().contains("3 c"), "{s}");
    assert_eq!(
        a.notes.focus,
        Some(0),
        "the cursor stays on the note just typed"
    );

    keys(&mut a, &[(KeyCode::Down, NONE)]);
    assert_eq!(
        (a.notes.focus, a.line),
        (None, 3),
        "Down leaves the note for line 4"
    );
    keys(&mut a, &[(KeyCode::Up, NONE)]);
    assert_eq!(a.notes.focus, Some(0), "Up stops on the note as on a line");

    keys(&mut a, &[(KeyCode::Enter, NONE), (KeyCode::End, NONE)]);
    assert_eq!(a.mode, Mode::Note);
    keys(&mut a, &[(KeyCode::Enter, NONE)]);
    typed(&mut a, "and d");
    keys(&mut a, &[(KeyCode::Esc, NONE)]);
    assert!(
        read(&dir).ends_with("> c\nwhy c\nand d\n"),
        "Enter is a new line in the note"
    );

    keys(
        &mut a,
        &[(KeyCode::Enter, NONE), (KeyCode::Char('x'), CTRL)],
    );
    keys(&mut a, &[(KeyCode::Char('x'), CTRL)]);
    assert_eq!(a.clipboard.as_deref(), Some("why c\n"));
    assert!(
        !notes_file(&dir, "feature").exists(),
        "the last note gone, the file goes"
    );
    assert_eq!(
        a.mode,
        Mode::Normal,
        "cutting the last line leaves for the code, read as before"
    );

    keys(&mut a, &[(KeyCode::Esc, NONE), (KeyCode::Char('z'), CTRL)]);
    assert!(
        read(&dir).ends_with("> c\nwhy c\nand d\n"),
        "Ctrl+Z brings it back whole"
    );
    assert_eq!(a.notes.focus, Some(0));
    keys(&mut a, &[(KeyCode::Char('z'), CTRL)]);
    assert!(
        read(&dir).ends_with("> c\nwhy c\n"),
        "and then the edit before"
    );
    keys(
        &mut a,
        &[(KeyCode::Char('y'), CTRL), (KeyCode::Char('y'), CTRL)],
    );
    assert!(
        !notes_file(&dir, "feature").exists(),
        "Ctrl+Y cuts it again"
    );
    keys(&mut a, &[(KeyCode::Char('z'), CTRL)]);

    keys(
        &mut a,
        &[(KeyCode::Char('v'), NONE), (KeyCode::Enter, NONE)],
    );
    keys(&mut a, &[(KeyCode::Backspace, NONE), (KeyCode::Esc, NONE)]);
    assert!(
        !notes_file(&dir, "feature").exists(),
        "v selects the note whole"
    );
    keys(
        &mut a,
        &[(KeyCode::Char('z'), CTRL), (KeyCode::Enter, NONE)],
    );
    keys(
        &mut a,
        &[(KeyCode::Char('u'), CTRL), (KeyCode::Backspace, NONE)],
    );
    assert!(
        !notes_file(&dir, "feature").exists(),
        "Backspace on an empty note removes it"
    );

    keys(
        &mut a,
        &[
            (KeyCode::Esc, NONE),
            (KeyCode::Char('a'), NONE),
            (KeyCode::Esc, NONE),
        ],
    );
    assert!(
        !notes_file(&dir, "feature").exists(),
        "an empty note is never kept"
    );
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn code_and_note_edits_undo_in_the_order_they_were_made() {
    let (dir, mut a) = on_a_rs("notes-undo", 3);
    note(&mut a, "one");
    keys(&mut a, &[(KeyCode::Down, NONE), (KeyCode::Enter, NONE)]);
    typed(&mut a, "X");
    keys(&mut a, &[(KeyCode::Esc, NONE), (KeyCode::Up, NONE)]);
    note(&mut a, "two");
    assert_eq!(parse(&read(&dir)).len(), 2);
    keys(&mut a, &[(KeyCode::Char('z'), CTRL)]);
    assert_eq!(parse(&read(&dir)).len(), 1);
    assert_eq!(a.buf.lines[3], "Xd", "the code edit is still there");
    keys(&mut a, &[(KeyCode::Char('z'), CTRL)]);
    assert_eq!(a.buf.lines[3], "d");
    assert_eq!(parse(&read(&dir)).len(), 1, "the first note stays");
    keys(&mut a, &[(KeyCode::Char('z'), CTRL)]);
    assert!(!notes_file(&dir, "feature").exists());
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn copying_code_across_a_note_takes_only_the_code() {
    let (dir, mut a) = on_a_rs("notes-copy", 3);
    note(&mut a, "between");
    keys(&mut a, &[(KeyCode::Up, NONE), (KeyCode::Home, NONE)]);
    assert_eq!((a.notes.focus, a.line), (None, 2));
    keys(&mut a, &[(KeyCode::Down, SHIFT), (KeyCode::Down, SHIFT)]);
    assert_eq!(a.line, 4, "Shift+Down steps over the note row");
    keys(&mut a, &[(KeyCode::Char('c'), CTRL)]);
    assert_eq!(a.clipboard.as_deref(), Some("c\nd\n"));
    keys(
        &mut a,
        &[
            (KeyCode::Esc, NONE),
            (KeyCode::Up, NONE),
            (KeyCode::Up, NONE),
        ],
    );
    assert_eq!(a.notes.focus, Some(0));
    keys(&mut a, &[(KeyCode::Char('c'), CTRL)]);
    assert_eq!(
        a.clipboard.as_deref(),
        Some("between\n"),
        "on the note, its text"
    );
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn a_range_and_a_deleted_line_take_notes_and_the_agent_can_drop_them() {
    let (dir, mut a) = on_a_rs("notes-range", 2);
    keys(&mut a, &[(KeyCode::Up, NONE)]);
    assert_eq!(a.at(), TextLine::Deleted(1, 0), "on the ghost of `b`");
    note(&mut a, "keep b");
    keys(&mut a, &[(KeyCode::Down, NONE)]);
    assert_eq!(a.at(), TextLine::File(1));
    keys(&mut a, &[(KeyCode::Down, SHIFT), (KeyCode::Down, SHIFT)]);
    note(&mut a, "these");
    let text = read(&dir);
    assert!(text.contains("\nsrc/a.rs:2\n- b\nkeep b\n"), "{text}");
    assert!(text.contains("\nsrc/a.rs:2-3\n> B\n> c\nthese\n"), "{text}");
    let s = screen(&mut a);
    assert!(s.contains("lines 2 to 3  these"), "{s}");
    assert!(s.contains("\u{270e} keep b"), "{s}");
    assert!(
        a.notes_farewell()
            .unwrap()
            .ends_with("2 notes for your agent in .git/merl/notes/feature")
    );

    std::fs::remove_file(notes_file(&dir, "feature")).unwrap();
    let s = screen(&mut a);
    assert!(!s.contains("these") && !s.contains("keep b"), "{s}");
    assert_eq!(a.notes_farewell(), None);
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn a_branch_with_slashes_keeps_its_notes_in_folders() {
    let (dir, mut a) = review_app("notes-slash");
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(&dir)
        .args(["switch", "-q", "-c", "feat/x"])
        .output()
        .unwrap();
    assert!(out.status.success());
    a.notes.on = true;
    a.jump_to(&dir.join("src/a.rs"), 1);
    screen(&mut a);
    note(&mut a, "first");
    let text = std::fs::read_to_string(notes_file(&dir, "feat/x")).unwrap();
    assert!(
        text.starts_with("# merl notes \u{b7} feat/x \u{2190} "),
        "{text}"
    );
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn notes_off_leave_a_alone() {
    let (dir, mut a) = on_a_rs("notes-off", 3);
    a.notes.on = false;
    keys(&mut a, &[(KeyCode::Char('a'), NONE)]);
    assert_eq!(a.mode, Mode::Normal);
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn a_note_outlives_its_cut_line_and_reattaches_on_undo() {
    let (dir, mut a) = on_a_rs("notes-cut", 3);
    note(&mut a, "why c");
    keys(
        &mut a,
        &[
            (KeyCode::Up, NONE),
            (KeyCode::Enter, NONE),
            (KeyCode::Char('x'), CTRL),
        ],
    );
    keys(&mut a, &[(KeyCode::Esc, NONE)]);
    assert_eq!(a.buf.lines[2], "d");
    let s = screen(&mut a);
    assert!(s.contains("why c (changed)"), "{s}");
    assert!(
        read(&dir).ends_with("> c\nwhy c\n"),
        "the file keeps the note as written"
    );
    keys(&mut a, &[(KeyCode::Char('z'), CTRL)]);
    let s = screen(&mut a);
    let row = s.lines().position(|l| l.contains("why c")).unwrap();
    assert!(!s.contains("(changed)"), "{s}");
    assert!(s.lines().nth(row - 1).unwrap().contains("3 c"), "{s}");
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn the_agent_dropping_the_file_under_an_open_note_neither_crashes_nor_brings_it_back() {
    let (dir, mut a) = on_a_rs("notes-drop", 3);
    note(&mut a, "one");
    keys(&mut a, &[(KeyCode::Up, NONE), (KeyCode::Char('a'), NONE)]);
    std::fs::remove_file(notes_file(&dir, "feature")).unwrap();
    keys(&mut a, &[(KeyCode::Down, NONE)]);
    assert_eq!(a.mode, Mode::Normal);
    note(&mut a, "two");
    keys(&mut a, &[(KeyCode::Enter, NONE)]);
    std::fs::remove_file(notes_file(&dir, "feature")).unwrap();
    keys(&mut a, &[(KeyCode::Esc, NONE)]);
    assert!(
        !notes_file(&dir, "feature").exists(),
        "a note the agent dropped stays dropped"
    );
    keys(&mut a, &[(KeyCode::Char('z'), CTRL)]);
    assert_eq!(a.message, "note changed outside merl");
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn edit_mode_walks_through_a_note_and_home_keeps_to_its_line() {
    let (dir, mut a) = on_a_rs("notes-walk", 3);
    note(&mut a, "abc");
    keys(
        &mut a,
        &[
            (KeyCode::Up, NONE),
            (KeyCode::Enter, NONE),
            (KeyCode::Down, NONE),
        ],
    );
    assert_eq!(a.mode, Mode::Note, "Down from line 3 into its note");
    keys(&mut a, &[(KeyCode::End, NONE), (KeyCode::Enter, NONE)]);
    typed(&mut a, "def");
    keys(&mut a, &[(KeyCode::Home, NONE)]);
    typed(&mut a, "> ");
    a.paste("x\ny");
    keys(&mut a, &[(KeyCode::Down, NONE)]);
    assert_eq!(
        (a.mode, a.line),
        (Mode::Edit, 3),
        "Down past its last line, on to line 4"
    );
    assert_eq!(parse(&read(&dir))[0].text, "abc\n> x\nydef");
    keys(&mut a, &[(KeyCode::Up, NONE)]);
    assert_eq!(a.mode, Mode::Note, "Up from line 4 back into the note");
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn every_way_out_of_a_note_returns_to_the_mode_it_was_opened_from() {
    let esc = (KeyCode::Esc, NONE);
    let enter = (KeyCode::Enter, NONE);
    let up = (KeyCode::Up, NONE);
    let down = (KeyCode::Down, NONE);
    let back = (KeyCode::Backspace, NONE);
    let cut = (KeyCode::Char('x'), CTRL);
    let undo = (KeyCode::Char('z'), CTRL);
    let redo = (KeyCode::Char('y'), CTRL);
    let reading: &[(&str, Keys, &str, usize)] = &[
        ("esc", &[esc], "hi", 2),
        ("enter-esc", &[enter, esc], "hi", 2),
        ("down", &[down], "hi", 3),
        ("up", &[up], "hi", 2),
        ("back", &[back], "", 2),
        ("cut", &[cut], "hi", 3),
        ("undo", &[undo], "hi", 2),
        ("redo", &[redo], "hi", 2),
    ];
    for &(tag, exit, text, line) in reading {
        let (dir, mut a) = on_a_rs(&format!("notes-out-read-{tag}"), 3);
        keys(&mut a, &[(KeyCode::Char('a'), NONE)]);
        typed(&mut a, text);
        keys(&mut a, exit);
        assert_eq!((a.mode, a.line), (Mode::Normal, line), "a {text} {tag}");
        drop(std::fs::remove_dir_all(dir));
    }
    let editing: &[(&str, Keys, usize)] = &[
        ("esc", &[esc], 2),
        ("down", &[down], 3),
        ("up", &[up], 2),
        ("back", &[(KeyCode::Char('u'), CTRL), back], 2),
        ("cut", &[cut], 3),
        ("undo", &[undo], 2),
    ];
    for &(tag, exit, line) in editing {
        let (dir, mut a) = on_a_rs(&format!("notes-out-edit-{tag}"), 3);
        note(&mut a, "hi");
        keys(&mut a, &[up, enter, down]);
        assert_eq!(a.mode, Mode::Note, "{tag}");
        keys(&mut a, exit);
        assert_eq!((a.mode, a.line), (Mode::Edit, line), "edit {tag}");
        assert_eq!(a.notes.focus, None, "edit {tag}");
        drop(std::fs::remove_dir_all(dir));
    }
    let (dir, mut a) = on_a_rs("notes-out-edit-redo", 3);
    note(&mut a, "hi");
    keys(&mut a, &[up, enter, down, redo]);
    assert_eq!(a.mode, Mode::Note, "nothing to redo keeps the note open");
    keys(&mut a, &[esc]);
    assert_eq!((a.mode, a.line, a.notes.focus), (Mode::Edit, 2, None));
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn notes_follow_the_branch_checked_out_and_need_one() {
    let (dir, mut a) = on_a_rs("notes-switch", 3);
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    git(&["switch", "-q", "-c", "other"]);
    note(&mut a, "on other");
    assert!(notes_file(&dir, "other").exists());
    assert!(!notes_file(&dir, "feature").exists());
    git(&["switch", "-q", "--detach"]);
    keys(&mut a, &[(KeyCode::Down, NONE), (KeyCode::Char('a'), NONE)]);
    assert_eq!(
        (a.mode, a.message.as_str()),
        (Mode::Normal, "notes need a branch")
    );
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn a_note_row_keeps_its_plain_gutter_in_a_theme_with_a_number_strip() {
    let (dir, mut a) = on_a_rs("notes-strip", 2);
    note(&mut a, "keep b");
    keys(&mut a, &[(KeyCode::Down, NONE)]);
    let theme = crate::theme::load("kanagawa-wave").unwrap();
    assert_ne!(
        theme.gutter_bg(),
        theme.bg,
        "kanagawa-wave's LineNr has a strip"
    );
    let mut t = Terminal::new(TestBackend::new(70, 16)).unwrap();
    t.draw(|f| crate::ui::draw(f, &mut a, &theme)).unwrap();
    drop(std::fs::remove_dir_all(dir));
    let buf = t.backend().buffer();
    let (x, y) = (0..16)
        .find_map(|y| {
            (0..70)
                .find(|&x| buf[(x, y)].symbol() == "\u{270e}")
                .map(|x| (x, y))
        })
        .expect("the note is on screen");
    assert_eq!(
        (0..=x).map(|x| buf[(x, y)].bg).collect::<Vec<_>>(),
        vec![theme.bg; usize::from(x) + 1],
        "the note row's gutter stays plain, as before #913: no strip cut short at the ✎"
    );
    assert_eq!(
        buf[(0, y + 1)].bg,
        theme.gutter_bg(),
        "the file row under it has the strip"
    );
}

#[test]
fn a_note_being_typed_draws_the_reading_cursor_on_its_band() {
    let (dir, mut a) = on_a_rs("notes-cursor", 2);
    keys(&mut a, &[(KeyCode::Char('a'), NONE)]);
    typed(&mut a, "check the window");
    assert_eq!(a.mode, Mode::Note);
    let theme = crate::theme::load("github-light").unwrap();
    let mut t = Terminal::new(TestBackend::new(70, 16)).unwrap();
    t.draw(|f| crate::ui::draw(f, &mut a, &theme)).unwrap();
    drop(std::fs::remove_dir_all(dir));
    let at = a.cursor_cell.expect("the note's cursor");
    let cell = &t.backend().buffer()[(at.x, at.y)];
    let hex = |h: &str| h.parse::<ratatui::style::Color>().unwrap();
    assert_eq!(
        (cell.fg, cell.bg),
        (hex("#ffffff"), hex("#1f2328")),
        "github-light's Cursor, not the accent #6639ba at 2.34:1 on the note band"
    );
}
