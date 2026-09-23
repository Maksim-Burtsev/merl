//! Moving the cursor and growing the selection.

use super::*;

#[test]
fn word_jumps_by_word_runs() {
    let mut a = app("foo bar_1 baz\nnext");
    press(&mut a, KeyCode::Right, KeyModifiers::ALT);
    assert_eq!(a.col, 3);
    press(&mut a, KeyCode::Right, KeyModifiers::ALT);
    assert_eq!(a.col, 9);
    press(&mut a, KeyCode::Left, KeyModifiers::ALT);
    assert_eq!(a.col, 4);
    // Past the end of the line, jump to the start of the next one.
    a.col = 13;
    press(&mut a, KeyCode::Right, KeyModifiers::ALT);
    assert_eq!(((a.line, a.col), a.selection()), ((1, 0), None));
}

#[test]
fn a_word_is_letters_of_any_script() {
    let mut a = app("// привет, мир foo");
    a.col = 3;
    press(
        &mut a,
        KeyCode::Right,
        KeyModifiers::ALT | KeyModifiers::SHIFT,
    );
    assert_eq!(a.selected_text().as_deref(), Some("привет"));
    press(&mut a, KeyCode::Right, KeyModifiers::ALT);
    press(&mut a, KeyCode::Left, KeyModifiers::ALT);
    assert_eq!(a.col, "// привет, ".len());
    press(&mut a, KeyCode::Char('v'), KeyModifiers::NONE);
    assert_eq!(a.selected_text().as_deref(), Some("мир"));
}

/// Ghostty, iTerm and Terminal.app send Option+Left / Right as Esc b / Esc f.
#[test]
fn alt_b_and_alt_f_are_the_word_jump_and_do_not_leave_edit_mode() {
    let mut a = app("foo bar baz");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('f'), KeyModifiers::ALT);
    press(&mut a, KeyCode::Char('f'), KeyModifiers::ALT);
    assert_eq!((a.col, a.mode), (7, Mode::Edit));
    press(&mut a, KeyCode::Char('b'), KeyModifiers::ALT);
    assert_eq!((a.col, a.mode), (4, Mode::Edit));
    assert_eq!(a.buf.lines, vec!["foo bar baz"], "nothing was typed");
}

#[test]
fn shift_left_right_select_by_char_and_cross_the_line_end() {
    let mut a = app("ab\ncd");
    a.col = 1;
    press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    assert_eq!(a.selection(), Some(((0, 1), (0, 2))));
    press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    assert_eq!(a.selection(), Some(((0, 1), (1, 0))));
    for _ in 0..3 {
        press(&mut a, KeyCode::Left, KeyModifiers::SHIFT);
    }
    assert_eq!(
        a.selection(),
        Some(((0, 0), (0, 1))),
        "back over the anchor"
    );
}

#[test]
fn ctrl_c_copies_in_navigation_and_never_quits() {
    let mut a = app("abc");
    press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    assert!(!press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_eq!(a.clipboard.take().as_deref(), Some("ab"));
    assert!(a.selection().is_some(), "copy leaves the selection");
    // Cmd+C, from a terminal that passes it on, copies too.
    assert!(!press(&mut a, KeyCode::Char('c'), KeyModifiers::SUPER));
    assert_eq!(a.clipboard.take().as_deref(), Some("ab"));
    // Without a selection the line goes, and merl stays open.
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_eq!(a.clipboard.take().as_deref(), Some("abc"));
    assert_eq!(a.message, "copied 1 line");
    // A prompt has nothing to copy, and Ctrl+C does not quit from it either.
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    assert!(!press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_eq!(a.clipboard, None);
}

#[test]
fn v_grows_the_selection_word_line_paragraph() {
    let mut a = app("x\n\n  let foo = 1;\n  bar\n\ny");
    (a.line, a.col) = (2, 7);
    let v = |a: &mut App| press(a, KeyCode::Char('v'), KeyModifiers::NONE);
    v(&mut a);
    assert_eq!(a.selection(), Some(((2, 6), (2, 9))), "the word");
    v(&mut a);
    assert_eq!(a.selection(), Some(((2, 0), (2, 14))), "the line");
    v(&mut a);
    assert_eq!(a.selection(), Some(((2, 0), (3, 5))), "the paragraph");
    v(&mut a);
    assert_eq!(a.selection(), Some(((2, 0), (3, 5))), "nothing wider");
    // Off a word the first step is the line; a hand-made selection grows from what it is.
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    (a.line, a.col) = (2, 11);
    v(&mut a);
    assert_eq!(a.selection(), Some(((2, 0), (2, 14))));
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    (a.line, a.col) = (2, 5);
    for _ in 0..3 {
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    }
    v(&mut a);
    assert_eq!(
        a.selection(),
        Some(((2, 0), (2, 14))),
        "` fo` is wider than no word"
    );
    // A blank line has nothing to select.
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    (a.line, a.col) = (1, 0);
    v(&mut a);
    assert_eq!(a.selection(), None);
}

#[test]
fn shift_up_down_extend_a_selection_from_the_cursor_column() {
    let mut a = app("abc\ndef\nghi\nj\n");
    assert_eq!(a.selection(), None);
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    assert_eq!((a.line, a.selection()), (2, Some(((0, 2), (2, 2)))));
    // Per line, the bytes the renderer paints: partial edges, whole lines in between.
    assert_eq!(a.selected_bytes(0), Some(2..3));
    assert_eq!(a.selected_bytes(1), Some(0..3));
    assert_eq!(a.selected_bytes(2), Some(0..2));
    assert_eq!(a.selected_bytes(3), None);
    // Back onto the anchor nothing is selected, and Shift+Down selects from there again.
    for _ in 0..3 {
        press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
    }
    assert_eq!((a.line, a.selection()), (0, None));
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    assert_eq!(a.selection(), Some(((0, 2), (1, 2))));
    // Any other cursor move drops it; Esc drops it too.
    press(&mut a, KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(a.selection(), None);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    assert!(a.selection().is_some());
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(a.selection(), None);
}

#[test]
fn a_selection_collapsed_onto_its_anchor_selects_nothing() {
    let mut a = app("abc\ndef\n");
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
    assert_eq!(a.selection(), None);
    // A plain arrow moves on instead of collapsing a range that is not there.
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(a.col, 2);
    // While editing, Ctrl+C copies the line and Backspace deletes a char, as with no selection.
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(a.clipboard.take().as_deref(), Some("abc\n"));
    press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(a.buf.lines, vec!["ac", "def"]);
}

#[test]
fn a_jump_or_a_find_match_drops_the_selection() {
    let mut a = app("abc\ndef\nghi\njkl\n");
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Char(':'), KeyModifiers::NONE);
    typed(&mut a, "4");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((a.line, a.selection()), (3, None));
    // Find sets the selection aside while it moves the cursor: Esc puts both back, a match
    // taken with Enter leaves it behind.
    press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
    let sel = a.selection();
    assert_eq!(sel, Some(((2, 0), (3, 0))));
    // A jump onto the cursor itself goes nowhere and keeps it.
    press(&mut a, KeyCode::Char(':'), KeyModifiers::NONE);
    typed(&mut a, "3");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(a.selection(), sel);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    typed(&mut a, "abc");
    assert_eq!((a.line, a.selection()), (0, None));
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!((a.line, a.selection()), (2, sel));
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    typed(&mut a, "abc");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((a.line, a.selection()), (0, None));
    // A search that has nowhere to move the cursor keeps it.
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    typed(&mut a, "zzz");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(a.selection(), Some(((0, 0), (1, 0))));
    // One that moved the cursor and then stopped matching has still moved it.
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    typed(&mut a, "ghz");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((a.line, a.selection()), (2, None));
    // So does a picker jump within the open file.
    press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
    let hit = Hit {
        path: a.buf.path.clone().unwrap(),
        line: 4,
        text: "jkl".into(),
    };
    a.show_picker(PickerKind::Usages, App::hit_items(vec![hit]));
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((a.line, a.selection()), (3, None));
}

#[test]
fn ctrl_shift_left_right_select_to_the_line_edges() {
    let mut a = app("foo bar\nbaz");
    for _ in 0..3 {
        press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    }
    press(
        &mut a,
        KeyCode::Right,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    assert_eq!((a.col, a.selection()), (7, Some(((0, 3), (0, 7)))));
    // A plain arrow collapses the selection to its end without moving on, VS Code style.
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(((a.line, a.col), a.selection()), ((0, 7), None));
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (1, 0));
    press(&mut a, KeyCode::Up, KeyModifiers::NONE);
    press(&mut a, KeyCode::End, KeyModifiers::NONE);
    press(
        &mut a,
        KeyCode::Left,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    assert_eq!((a.col, a.selection()), (0, Some(((0, 0), (0, 7)))));
    // Left collapses to the start, which is where the cursor already is: no move.
    press(&mut a, KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(((a.line, a.col), a.selection()), ((0, 0), None));
    press(
        &mut a,
        KeyCode::Right,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    press(&mut a, KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(((a.line, a.col), a.selection()), ((0, 0), None));
    // Alt+Right alone is a plain word jump: it drops the selection.
    press(
        &mut a,
        KeyCode::Right,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    press(&mut a, KeyCode::Left, KeyModifiers::ALT);
    assert_eq!((a.col, a.selection()), (4, None));
}

#[test]
fn alt_shift_left_right_select_by_word_without_touching_find() {
    let mut a = app("foo bar_1 baz");
    a.find_re = Some(Regex::new("foo").unwrap());
    let m = KeyModifiers::ALT | KeyModifiers::SHIFT;
    press(&mut a, KeyCode::Right, m);
    press(&mut a, KeyCode::Right, m);
    assert_eq!((a.col, a.selection()), (9, Some(((0, 0), (0, 9)))));
    press(&mut a, KeyCode::Left, m);
    assert_eq!((a.col, a.selection()), (4, Some(((0, 0), (0, 4)))));
    assert!(a.find_re.is_some(), "Alt on an arrow is not an Esc");
    assert_eq!(a.message, "");
}

#[test]
fn want_x_is_sticky_across_short_lines() {
    let mut a = app("abcdef\nxy\nabcdef");
    a.col = 5;
    a.want_x = 5;
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(a.col, 2);
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(a.col, 5);
}

#[test]
fn half_page_moves_cursor_and_viewport_together() {
    let text: String = (0..40).map(|i| format!("line {i}\n")).collect();
    let mut a = app(&text);
    a.view_h = 10;
    press(&mut a, KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert_eq!(a.line, 5);
    assert_eq!(a.top_line, 5, "viewport scrolls by the same amount");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert_eq!((a.line, a.top_line), (10, 10));
    press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
    assert_eq!((a.line, a.top_line), (5, 5));
    // Ctrl+D must not be mistaken for go-to-definition.
    assert_eq!(a.message, "");
}

#[test]
fn half_page_counts_screen_rows() {
    // The first line is four rows at 20 columns; half of a 6-row screen is 3 of them.
    let mut a = app(&format!("{}\nnext\n", "word ".repeat(16)));
    a.view_h = 6;
    press(&mut a, KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert_eq!((a.line, a.cursor_row()), (0, 3));
    assert_eq!(
        (a.top_line, a.top_row),
        (0, 3),
        "the viewport scrolls the same rows"
    );
}

#[test]
fn up_and_down_walk_screen_rows_and_keep_the_column() {
    // At 20 columns the first line is two rows: "aaaa bbbb cccc dddd " and "eeee ffff".
    let mut a = app("aaaa bbbb cccc dddd eeee ffff\nx\n");
    for _ in 0..7 {
        press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    }
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (0, 27), "the second row of the same line");
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (1, 1), "a shorter row: its end");
    press(&mut a, KeyCode::Up, KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (0, 27), "the column is kept");
    press(&mut a, KeyCode::Up, KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (0, 7));
}

#[test]
fn home_and_end_stop_at_the_screen_row_first() {
    let mut a = app("aaaa bbbb cccc dddd eeee ffff\n");
    press(&mut a, KeyCode::End, KeyModifiers::NONE);
    assert_eq!(a.col, 19, "on the space that ends the first row");
    press(&mut a, KeyCode::End, KeyModifiers::NONE);
    assert_eq!(a.col, 29, "the end of the line");
    press(&mut a, KeyCode::Home, KeyModifiers::NONE);
    assert_eq!(a.col, 20, "the start of the second row");
    press(&mut a, KeyCode::Home, KeyModifiers::NONE);
    assert_eq!(a.col, 0);
}

#[test]
fn paragraph_jumps_to_blank_lines() {
    let mut a = app("a\nb\n\n  \nc\nd\n\ne\nf");
    let go = |a: &mut App, c: char| press(a, KeyCode::Char(c), KeyModifiers::NONE);
    go(&mut a, '}');
    assert_eq!(a.line, 2);
    go(&mut a, '}');
    assert_eq!(a.line, 6, "a run of blank lines is one boundary");
    go(&mut a, '}');
    assert_eq!(a.line, 8, "no blank line left: the last line");
    go(&mut a, '{');
    assert_eq!(a.line, 6);
    go(&mut a, '{');
    assert_eq!(a.line, 3, "whitespace-only lines are blank");
    go(&mut a, '{');
    assert_eq!(a.line, 0, "no blank line left: the first line");
}

#[test]
fn ctrl_end_goes_to_last_line() {
    let mut a = app("a\nbb\nccc\n");
    press(&mut a, KeyCode::End, KeyModifiers::CONTROL);
    assert_eq!((a.line, a.col), (2, 3));
    press(&mut a, KeyCode::Home, KeyModifiers::CONTROL);
    assert_eq!((a.line, a.col), (0, 0));
}

/// #185: an emoji written with a selector, a skin tone or a ZWJ is one step and two columns.
#[test]
fn an_emoji_is_one_step_and_two_columns() {
    let mut a = app("\u{26a0}\u{fe0f}ab\n\u{1f468}\u{200d}\u{1f4bb}\u{1f44d}\u{1f3fd}");
    press(&mut a, KeyCode::End, KeyModifiers::NONE);
    press(&mut a, KeyCode::Left, KeyModifiers::NONE);
    assert_eq!((a.col, a.cursor_x(), a.display_col()), (7, 3, 4));
    press(&mut a, KeyCode::Left, KeyModifiers::NONE);
    press(&mut a, KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(a.col, 0);
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!((a.col, a.cursor_x()), (11, 2));
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!((a.col, a.cursor_x()), (19, 4));
}
