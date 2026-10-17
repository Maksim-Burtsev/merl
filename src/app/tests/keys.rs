use super::*;
use std::fmt::Write as _;

#[test]
fn goto_prompt_clamps() {
    let mut a = app("a\nb\nc\n");
    for code in [KeyCode::Char(':'), KeyCode::Char('9'), KeyCode::Enter] {
        press(&mut a, code, KeyModifiers::NONE);
    }
    assert_eq!(a.mode, Mode::Normal);
    assert_eq!(a.line, 2);
    press(&mut a, KeyCode::Char(':'), KeyModifiers::NONE);
    assert!(
        !press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE),
        "`q` inside the prompt is a digit-less keystroke, not a quit"
    );
    assert!(!press(&mut a, KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(a.mode, Mode::Normal);
    assert!(press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
}

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
    assert_eq!(
        (a.line, a.col, a.display_col()),
        (1, 3, 3),
        "`merl FILE:LINE:COL` counts chars, as compilers do"
    );
    assert_eq!(
        a.history,
        [(path.clone(), TextLine::File(1), 3)],
        "the first stop in the jump history has the column too"
    );
    assert_eq!(
        at(2, 99).col,
        "héllo".len(),
        "a column past the end of the line is its end"
    );
}

#[test]
fn shift_on_char_is_stripped() {
    let mut a = app("a\nb\n");
    assert!(press(&mut a, KeyCode::Char('q'), KeyModifiers::SHIFT));
    let mut a = app("a\nb\n");
    press(&mut a, KeyCode::Char(':'), KeyModifiers::SHIFT);
    assert_eq!(a.mode, Mode::Goto);
}

#[test]
fn ctrl_shift_letter_is_never_the_bare_letter() {
    let mut a = app(&"x\n".repeat(200));
    let ctrl_shift = KeyModifiers::CONTROL | KeyModifiers::SHIFT;
    for code in [KeyCode::Char('D'), KeyCode::Char('d'), KeyCode::Char('U')] {
        press(&mut a, code, ctrl_shift);
        assert!(a.picker.is_none() && a.message.is_empty());
        assert_eq!((a.mode, a.line), (Mode::Normal, 0));
    }
    for c in ['D', '?', ':', '}'] {
        press(&mut a, KeyCode::Char(c), KeyModifiers::CONTROL);
        assert!(a.picker.is_none() && a.message.is_empty(), "Ctrl+{c}");
        assert_eq!((a.mode, a.line), (Mode::Normal, 0), "Ctrl+{c}");
    }
    assert!(!press(&mut a, KeyCode::Char('q'), ctrl_shift));
    press(&mut a, KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert_ne!(a.line, 0);
}

#[test]
fn unbound_ctrl_letter_is_never_the_bare_letter() {
    let mut a = app(&"x\n".repeat(200));
    a.show_tree = false;
    let ctrl = KeyModifiers::CONTROL;
    for c in ['q', 't', 'w', 'p', 'o', 'k', 'x'] {
        assert!(!press(&mut a, KeyCode::Char(c), ctrl), "Ctrl+{c}");
        assert!(a.picker.is_none() && a.message.is_empty(), "Ctrl+{c}");
        assert!(
            !a.show_tree && a.wrap_opposite_of_kind.is_empty(),
            "Ctrl+{c}"
        );
        assert_eq!((a.mode, a.line), (Mode::Normal, 0), "Ctrl+{c}");
    }
    press(&mut a, KeyCode::Char('?'), KeyModifiers::NONE);
    assert!(!press(&mut a, KeyCode::Char('q'), ctrl));
    assert_eq!(a.mode, Mode::Help);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    for c in ['q', 't'] {
        assert!(!press(&mut a, KeyCode::Char(c), ctrl), "Edit: Ctrl+{c}");
        assert!(!a.show_tree && a.mode == Mode::Edit, "Edit: Ctrl+{c}");
    }
    press(&mut a, KeyCode::Char('x'), ctrl);
    assert_eq!(a.buf.lines.len(), 199);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    for c in "ab cd".chars() {
        press(&mut a, KeyCode::Char(c), KeyModifiers::NONE);
    }
    press(&mut a, KeyCode::Char('w'), ctrl);
    assert_eq!(&*a.prompt, "ab ");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('d'), ctrl);
    assert_ne!(a.line, 0);
}

#[test]
fn cmd_letter_other_than_c_and_x_does_nothing() {
    let mut a = app(&"x\n".repeat(200));
    a.show_tree = false;
    let cmd = KeyModifiers::SUPER;
    for c in ['o', 's', 't', 'w', 'p', 'q', 'j', '?', 'C'] {
        assert!(!press(&mut a, KeyCode::Char(c), cmd), "Cmd+{c}");
        assert!(a.picker.is_none() && !a.show_tree, "Cmd+{c}");
        assert_eq!((a.mode, a.line), (Mode::Normal, 0), "Cmd+{c}");
    }
    assert!(!press(&mut a, KeyCode::Char('o'), cmd | KeyModifiers::ALT));
    assert!(!press(&mut a, KeyCode::Char('d'), cmd | KeyModifiers::CONTROL));
    assert_eq!((a.picker.is_none(), a.line), (true, 0));
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('o'), cmd);
    assert_eq!(a.buf.lines[0], "x");
    press(&mut a, KeyCode::Char('x'), cmd);
    assert_eq!(a.buf.lines.len(), 199, "Cmd+X still cuts the line");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('o'), cmd);
    assert_eq!(&*a.prompt, "");
}

#[test]
fn alt_ctrl_letter_with_an_unbound_ctrl_letter_does_nothing() {
    let mut a = app(&"x\n".repeat(200));
    let alt_ctrl = KeyModifiers::ALT | KeyModifiers::CONTROL;
    press(&mut a, KeyCode::Char('?'), KeyModifiers::NONE);
    assert!(!press(&mut a, KeyCode::Char('q'), alt_ctrl));
    assert_eq!(a.mode, Mode::Help, "the overlay stays open");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    find(&mut a, "x");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    for c in ['q', 't', 'o', 'D'] {
        assert!(!press(&mut a, KeyCode::Char(c), alt_ctrl), "Alt+Ctrl+{c}");
        assert!(a.find_re.is_some() && a.anchor.is_some(), "Alt+Ctrl+{c}");
    }
    assert!(!press(&mut a, KeyCode::Char('t'), alt_ctrl | KeyModifiers::SHIFT));
    assert!(a.find_re.is_some() && a.anchor.is_some());
    let line = a.line;
    press(&mut a, KeyCode::Char('d'), alt_ctrl);
    assert!(a.find_re.is_none() && a.anchor.is_none(), "a bound one is Esc first");
    assert!(a.line > line, "then the Ctrl+letter");
}

#[test]
fn aliases_reach_the_same_actions() {
    let mut a = app("foo bar\n");
    std::fs::remove_file(a.buf.path.as_ref().unwrap()).unwrap();
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

#[test]
fn alt_letter_over_an_overlay_only_closes_it() {
    let mut a = app("foo\n");
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
    assert!(
        !press(&mut a, KeyCode::Char('q'), KeyModifiers::ALT),
        "Alt+q over a picker must not quit"
    );
    assert_eq!((a.picker.is_none(), a.mode), (true, Mode::Normal));
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('d'), KeyModifiers::ALT);
    assert_eq!(
        (a.mode, a.message.as_str()),
        (Mode::Normal, ""),
        "Alt+d over a prompt closes it and must not run go-to-definition"
    );
    assert!(
        press(&mut a, KeyCode::Char('q'), KeyModifiers::ALT),
        "in normal mode the letter still counts"
    );
}

#[test]
fn an_unbound_alt_letter_while_editing_is_ignored() {
    let mut a = app("foo\n");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('p'), KeyModifiers::ALT);
    assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::ALT));
    assert!(
        !press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE),
        "Option+letter mid-word must not drop into navigation, where `q` quits"
    );
    assert_eq!((a.mode, a.buf.lines[0].as_str()), (Mode::Edit, "qfoo"));
}

#[test]
fn help_opens_and_closes_and_esc_clears_the_find() {
    let mut a = app("foo\nbar\n");
    press(&mut a, KeyCode::Char('?'), KeyModifiers::NONE);
    assert_eq!(a.mode, Mode::Help);
    assert!(
        !press(&mut a, KeyCode::Down, KeyModifiers::NONE),
        "keys are inert while the overlay is up"
    );
    assert_eq!((a.mode, a.line), (Mode::Help, 0));
    assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
    assert_eq!(a.mode, Mode::Normal, "`q` closes it instead of quitting");

    find(&mut a, "foo");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert!(a.find_re.is_some());
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert!(a.find_re.is_none());
    assert_eq!(a.message, "find cleared");
}

#[test]
fn readme_documents_every_key() {
    let readme = include_str!("../../../README.md");
    let mut every = String::from("<summary>Every key</summary>\n");
    let mut group = "";
    for (key, action, g) in KEYS {
        if *g != group {
            group = g;
            _ = write!(every, "\n**{g}**\n\n| Key | Action |\n|---|---|\n");
        }
        _ = writeln!(every, "| {key} | {action} |");
    }
    every += "\n</details>\n";
    assert!(
        readme.contains(&every),
        "README should list every key as\n{every}"
    );
    let top = readme
        .split("\n## Keys\n")
        .nth(1)
        .expect("README has a Keys section")
        .split("<details>")
        .next()
        .unwrap();
    let rows = top
        .lines()
        .filter_map(|l| l.strip_prefix("| "))
        .filter_map(|l| l.split(" | ").next())
        .filter(|&c| c != "Key");
    for row in rows {
        assert!(
            KEYS.iter().any(|(k, _, _)| *k == row),
            "README documents {row:?}, which is not a binding"
        );
    }
}

#[test]
fn every_group_of_keys_is_one_run() {
    let mut runs: Vec<&str> = Vec::new();
    for (_, _, g) in KEYS {
        if runs.last() != Some(g) {
            assert!(
                !runs.contains(g),
                "{g} is split in two, and `?` and the README head a group once"
            );
            runs.push(g);
        }
    }
}

#[test]
fn an_empty_goto_prompt_is_a_cancel() {
    let mut a = app("a\nb\n");
    a.line = 1;
    press(&mut a, KeyCode::Char(':'), KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((a.mode, a.line, a.message.as_str()), (Mode::Normal, 1, ""));
}

#[test]
fn the_goto_prompt_takes_only_digits_but_ctrl_letters_still_edit_it() {
    let mut a = app("a\nb\n");
    press(&mut a, KeyCode::Char(':'), KeyModifiers::NONE);
    typed(&mut a, "1x2");
    assert_eq!(&*a.prompt, "12");
    press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
    assert_eq!(&*a.prompt, "");
}
