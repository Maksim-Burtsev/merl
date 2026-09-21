//! The key table: the aliases, the overlays and the goto prompt.

use super::*;

#[test]
fn goto_prompt_clamps() {
    let mut a = app("a\nb\nc\n");
    for code in [KeyCode::Char(':'), KeyCode::Char('9'), KeyCode::Enter] {
        press(&mut a, code, KeyModifiers::NONE);
    }
    assert_eq!(a.mode, Mode::Normal);
    assert_eq!(a.line, 2);
    // `q` inside the prompt is a digit-less keystroke, not a quit.
    press(&mut a, KeyCode::Char(':'), KeyModifiers::NONE);
    assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
    assert!(!press(&mut a, KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(a.mode, Mode::Normal);
    assert!(press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
}

/// `merl FILE:LINE:COL` (#161): the column counts chars, as compilers do, and a column past
/// the end of the line is its end. The first stop in the jump history has it too.
#[test]
fn command_line_column_counts_chars() {
    let path = PathBuf::from("/tmp/merl-column.txt");
    let at = |line, col| {
        let buf = Buffer::from_bytes(path.clone(), "one\nhéllo\n".as_bytes());
        App::new(
            PathBuf::from("/tmp"),
            Tree::default(),
            Vec::new(),
            buf,
            Some((line, col)),
        )
    };
    let a = at(2, 3);
    assert_eq!((a.line, a.col, a.display_col()), (1, 3, 3));
    assert_eq!(a.history, [(path.clone(), 1, 3)]);
    assert_eq!(at(2, 99).col, "héllo".len());
}

#[test]
fn shift_on_char_is_stripped() {
    let mut a = app("a\nb\n");
    assert!(press(&mut a, KeyCode::Char('q'), KeyModifiers::SHIFT));
    let mut a = app("a\nb\n");
    press(&mut a, KeyCode::Char(':'), KeyModifiers::SHIFT);
    assert_eq!(a.mode, Mode::Goto);
}

/// Every alias in `KEYS` reaches the same action as its primary key.
#[test]
fn aliases_reach_the_same_actions() {
    let mut a = app("foo bar\n");
    // Not on disk: the grep for usages has nothing to read.
    std::fs::remove_file(a.buf.path.as_ref().unwrap()).unwrap();
    press(&mut a, KeyCode::F(12), KeyModifiers::NONE);
    assert_eq!(a.message, "no rules for .txt");
    press(&mut a, KeyCode::F(12), KeyModifiers::SHIFT);
    assert_eq!(a.message, "no usages of foo");
    press(&mut a, KeyCode::Char('e'), KeyModifiers::CONTROL);
    assert_eq!(a.mode, Mode::Picker(PickerKind::Files));
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert!(a.picker.is_none());
    press(&mut a, KeyCode::Char('f'), KeyModifiers::CONTROL);
    assert_eq!(a.mode, Mode::Find);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('g'), KeyModifiers::CONTROL);
    assert_eq!(a.mode, Mode::Goto);
}

#[test]
fn help_scrolls_and_reopens_at_the_top() {
    let mut a = app("foo\n");
    press(&mut a, KeyCode::Char('?'), KeyModifiers::NONE);
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    press(&mut a, KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(a.help_top, 1);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('?'), KeyModifiers::NONE);
    assert_eq!(a.help_top, 0);
}

/// Alt+letter is Esc then the letter. Over a picker or a prompt the Esc closes it and the
/// letter is dropped: Alt+q must not quit, Alt+d must not run go-to-definition.
#[test]
fn alt_letter_over_an_overlay_only_closes_it() {
    let mut a = app("foo\n");
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
    assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::ALT));
    assert!((a.picker.is_none(), a.mode) == (true, Mode::Normal));
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('d'), KeyModifiers::ALT);
    assert_eq!((a.mode, a.message.as_str()), (Mode::Normal, ""));
    // In normal mode the letter still counts.
    assert!(press(&mut a, KeyCode::Char('q'), KeyModifiers::ALT));
}

/// Edit mode is not an overlay for an Esc to close: Option+letter mid-word must not drop
/// into navigation, where the rest of the word runs as commands and its `q` quits.
#[test]
fn an_unbound_alt_letter_while_editing_is_ignored() {
    let mut a = app("foo\n");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('p'), KeyModifiers::ALT);
    assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::ALT));
    assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
    assert_eq!((a.mode, a.buf.lines[0].as_str()), (Mode::Edit, "qfoo"));
}

#[test]
fn help_opens_and_closes_and_esc_clears_the_find() {
    let mut a = app("foo\nbar\n");
    press(&mut a, KeyCode::Char('?'), KeyModifiers::NONE);
    assert_eq!(a.mode, Mode::Help);
    // Keys are inert while the overlay is up; `q` closes it instead of quitting.
    assert!(!press(&mut a, KeyCode::Down, KeyModifiers::NONE));
    assert_eq!((a.mode, a.line), (Mode::Help, 0));
    assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
    assert_eq!(a.mode, Mode::Normal);

    find(&mut a, "foo");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert!(a.find_re.is_some());
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert!(a.find_re.is_none());
    assert_eq!(a.message, "find cleared");
}

/// The README key table and `KEYS` are the same list.
#[test]
fn readme_documents_every_key() {
    let readme = include_str!("../../../README.md");
    let section = readme
        .split("\n## Keys\n")
        .nth(1)
        .expect("README has a Keys section")
        .split("\n## ")
        .next()
        .unwrap();
    let rows: Vec<&str> = section
        .lines()
        .filter(|l| l.starts_with('|'))
        .filter_map(|l| l.split('|').nth(1))
        .map(str::trim)
        .filter(|c| !c.is_empty() && !c.starts_with('-') && *c != "Key")
        .collect();
    for (key, _) in KEYS {
        assert!(rows.contains(key), "README is missing {key:?}");
    }
    for row in &rows {
        assert!(
            KEYS.iter().any(|(k, _)| k == row),
            "README documents {row:?}, which is not a binding"
        );
    }
}
