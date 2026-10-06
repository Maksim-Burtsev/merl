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
    a.col = 13;
    press(&mut a, KeyCode::Right, KeyModifiers::ALT);
    assert_eq!(
        ((a.line, a.col), a.file_selection()),
        ((1, 0), None),
        "past the end of the line, jump to the start of the next one"
    );
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
    assert_eq!(a.file_selection(), Some(((0, 1), (0, 2))));
    press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    assert_eq!(a.file_selection(), Some(((0, 1), (1, 0))));
    for _ in 0..3 {
        press(&mut a, KeyCode::Left, KeyModifiers::SHIFT);
    }
    assert_eq!(
        a.file_selection(),
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
    assert!(a.file_selection().is_some(), "copy leaves the selection");
    assert!(!press(&mut a, KeyCode::Char('c'), KeyModifiers::SUPER));
    assert_eq!(
        a.clipboard.take().as_deref(),
        Some("ab"),
        "Cmd+C, from a terminal that passes it on, copies too"
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_eq!(
        a.clipboard.take().as_deref(),
        Some("abc\n"),
        "without a selection the line goes, and merl stays open"
    );
    assert_eq!(a.message, "copied 1 line");
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    assert!(!press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_eq!(
        a.clipboard, None,
        "a prompt has nothing to copy, and Ctrl+C does not quit from it either"
    );
}

#[test]
fn v_grows_the_selection_word_line_paragraph_file() {
    let mut a = app("x\n\n  let foo = 1;\n  bar\n\ny");
    (a.line, a.col) = (2, 7);
    let v = |a: &mut App| press(a, KeyCode::Char('v'), KeyModifiers::NONE);
    v(&mut a);
    assert_eq!(a.file_selection(), Some(((2, 6), (2, 9))), "the word");
    v(&mut a);
    assert_eq!(a.file_selection(), Some(((2, 0), (2, 14))), "the line");
    v(&mut a);
    assert_eq!(a.file_selection(), Some(((2, 0), (3, 5))), "the paragraph");
    v(&mut a);
    assert_eq!(a.file_selection(), Some(((0, 0), (5, 1))), "the file");
    v(&mut a);
    assert_eq!(a.file_selection(), Some(((0, 0), (5, 1))), "nothing wider");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    (a.line, a.col) = (2, 11);
    v(&mut a);
    assert_eq!(
        a.file_selection(),
        Some(((2, 0), (2, 14))),
        "off a word the first step is the line"
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    (a.line, a.col) = (2, 5);
    for _ in 0..3 {
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    }
    v(&mut a);
    assert_eq!(
        a.file_selection(),
        Some(((2, 0), (2, 14))),
        "` fo` is wider than no word"
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    (a.line, a.col) = (1, 0);
    v(&mut a);
    assert_eq!(
        a.file_selection(),
        None,
        "a blank line has nothing to select"
    );
}

#[test]
fn shift_up_down_extend_a_selection_from_the_cursor_column() {
    let mut a = app("abc\ndef\nghi\nj\n");
    assert_eq!(a.file_selection(), None);
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    assert_eq!((a.line, a.file_selection()), (2, Some(((0, 2), (2, 2)))));
    assert_eq!(
        a.selected_bytes(TextLine::File(0)),
        Some(2..3),
        "per line, the bytes the renderer paints: partial edges, whole lines in between"
    );
    assert_eq!(a.selected_bytes(TextLine::File(1)), Some(0..3));
    assert_eq!(a.selected_bytes(TextLine::File(2)), Some(0..2));
    assert_eq!(a.selected_bytes(TextLine::File(3)), None);
    for _ in 0..3 {
        press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
    }
    assert_eq!(
        (a.line, a.file_selection()),
        (0, None),
        "back onto the anchor nothing is selected"
    );
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    assert_eq!(
        a.file_selection(),
        Some(((0, 2), (1, 2))),
        "Shift+Down selects from the anchor again"
    );
    press(&mut a, KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(a.file_selection(), None, "any other cursor move drops it");
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    assert!(a.file_selection().is_some());
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(a.file_selection(), None, "Esc drops it too");
}

#[test]
fn a_selection_collapsed_onto_its_anchor_selects_nothing() {
    let mut a = app("abc\ndef\n");
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
    assert_eq!(a.file_selection(), None);
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(
        a.col, 2,
        "a plain arrow moves on instead of collapsing a range that is not there"
    );
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(
        a.clipboard.take().as_deref(),
        Some("abc\n"),
        "while editing, Ctrl+C copies the line and Backspace deletes a char, as with no selection"
    );
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
    assert_eq!((a.line, a.file_selection()), (3, None));
    press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
    let sel = a.file_selection();
    assert_eq!(sel, Some(((2, 0), (3, 0))));
    press(&mut a, KeyCode::Char(':'), KeyModifiers::NONE);
    typed(&mut a, "3");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        a.file_selection(),
        sel,
        "a jump onto the cursor itself goes nowhere and keeps it"
    );
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    typed(&mut a, "abc");
    assert_eq!((a.line, a.file_selection()), (0, None));
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(
        (a.line, a.file_selection()),
        (2, sel),
        "find sets the selection aside while it moves the cursor: Esc puts both back"
    );
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    typed(&mut a, "abc");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        (a.line, a.file_selection()),
        (0, None),
        "a match taken with Enter leaves it behind"
    );
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    typed(&mut a, "zzz");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        a.file_selection(),
        Some(((0, 0), (1, 0))),
        "a search that has nowhere to move the cursor keeps it"
    );
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    typed(&mut a, "ghz");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        (a.line, a.file_selection()),
        (2, None),
        "one that moved the cursor and then stopped matching has still moved it"
    );
    press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
    let hit = Hit {
        path: a.buf.path.clone().unwrap(),
        line: 4,
        col: 0,
        text: "jkl".into(),
        deleted: None,
    };
    a.show_picker(PickerKind::Usages, App::hit_items(vec![hit]));
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        (a.line, a.file_selection()),
        (3, None),
        "so does a picker jump within the open file"
    );
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
    assert_eq!((a.col, a.file_selection()), (7, Some(((0, 3), (0, 7)))));
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(
        ((a.line, a.col), a.file_selection()),
        ((0, 7), None),
        "a plain arrow collapses the selection to its end without moving on, VS Code style"
    );
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (1, 0));
    press(&mut a, KeyCode::Up, KeyModifiers::NONE);
    press(&mut a, KeyCode::End, KeyModifiers::NONE);
    press(
        &mut a,
        KeyCode::Left,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    assert_eq!((a.col, a.file_selection()), (0, Some(((0, 0), (0, 7)))));
    press(&mut a, KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(
        ((a.line, a.col), a.file_selection()),
        ((0, 0), None),
        "Left collapses to the start, which is where the cursor already is: no move"
    );
    press(
        &mut a,
        KeyCode::Right,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    press(&mut a, KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(((a.line, a.col), a.file_selection()), ((0, 0), None));
    press(
        &mut a,
        KeyCode::Right,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    press(&mut a, KeyCode::Left, KeyModifiers::ALT);
    assert_eq!(
        (a.col, a.file_selection()),
        (4, None),
        "Alt+Right alone is a plain word jump: it drops the selection"
    );
}

#[test]
fn alt_shift_left_right_select_by_word_without_touching_find() {
    let mut a = app("foo bar_1 baz");
    a.find_re = Some(Regex::new("foo").unwrap());
    let m = KeyModifiers::ALT | KeyModifiers::SHIFT;
    press(&mut a, KeyCode::Right, m);
    press(&mut a, KeyCode::Right, m);
    assert_eq!((a.col, a.file_selection()), (9, Some(((0, 0), (0, 9)))));
    press(&mut a, KeyCode::Left, m);
    assert_eq!((a.col, a.file_selection()), (4, Some(((0, 0), (0, 4)))));
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
    assert_eq!(
        a.message, "",
        "Ctrl+D must not be mistaken for go-to-definition"
    );
}

#[test]
fn half_page_counts_screen_rows() {
    let mut a = app(&format!("{}\nnext\n", "word ".repeat(16)));
    a.view_h = 6;
    press(&mut a, KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert_eq!(
        (a.line, a.cursor_row()),
        (0, 3),
        "the first line is four rows at 20 columns; half of a 6-row screen is 3 of them"
    );
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

/// #284: a line longer than merl draws holds the cursor to the part on screen, whatever moves it
/// there, and Right / Alt+Right from the last drawn char go on to the next line.
#[test]
fn the_cursor_stays_on_the_drawn_part_of_a_cut_line() {
    let text = format!("{}THE END\nTHE END", "ab ".repeat(10_000));
    let (path, mut a) = temp_file("cut-line", &text);
    let end = a.buf.shown(0).len();
    assert!(end < a.buf.lines[0].len());
    let key = |a: &mut App, code, m| {
        press(a, code, m);
        (a.line, a.col)
    };
    key(&mut a, KeyCode::End, KeyModifiers::NONE);
    assert_eq!(
        key(&mut a, KeyCode::End, KeyModifiers::NONE),
        (0, end),
        "wrapped: End goes to the row's end, then to the drawn end"
    );
    assert_eq!(key(&mut a, KeyCode::Right, KeyModifiers::NONE), (1, 0));
    assert_eq!(key(&mut a, KeyCode::Left, KeyModifiers::NONE), (0, end));
    assert_eq!(key(&mut a, KeyCode::Right, KeyModifiers::ALT), (1, 0));
    assert_eq!(key(&mut a, KeyCode::Left, KeyModifiers::ALT), (0, end));
    key(&mut a, KeyCode::Down, KeyModifiers::NONE);
    press(&mut a, KeyCode::End, KeyModifiers::NONE);
    a.want_x = 100;
    assert_eq!(
        key(&mut a, KeyCode::Up, KeyModifiers::NONE),
        (0, end),
        "up onto the last drawn row, aiming past its end"
    );
    press(&mut a, KeyCode::Char('w'), KeyModifiers::NONE);
    key(&mut a, KeyCode::Home, KeyModifiers::NONE);
    assert_eq!(
        key(&mut a, KeyCode::End, KeyModifiers::NONE),
        (0, end),
        "not wrapped"
    );
    press(&mut a, KeyCode::Char('v'), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('v'), KeyModifiers::NONE);
    assert_eq!(
        (a.line, a.col),
        (0, end),
        "the selection's line step ends there too"
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    a.jump_to_col(&path, 1, 25_000);
    assert_eq!(
        (a.line, a.col),
        (0, end),
        "a jump to a column past the drawn end lands on it"
    );
    press(&mut a, KeyCode::Home, KeyModifiers::CONTROL);
    find(&mut a, "THE END");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (0, end), "so does a find");
    assert_eq!(
        key(&mut a, KeyCode::Char('n'), KeyModifiers::NONE),
        (1, 0),
        "`n` goes on to the next line"
    );
    let (_, mut a) = temp_file("cut-line-last", &format!("top\n{}", "ab ".repeat(10_000)));
    let end = a.buf.shown(1).len();
    assert_eq!(
        key(&mut a, KeyCode::End, KeyModifiers::CONTROL),
        (1, end),
        "Ctrl+End on a file that ends in the cut line"
    );
    assert_eq!(key(&mut a, KeyCode::Right, KeyModifiers::NONE), (1, end));
    assert_eq!(key(&mut a, KeyCode::Right, KeyModifiers::ALT), (1, end));
}

/// #284: a selection that ends where a cut line's drawn part ends copies the line to its real
/// end, as Ctrl+C with no selection does: `v v` or Shift+Ctrl+Right, then Ctrl+C, never copies a
/// line cut at 20 KB.
#[test]
fn a_selection_to_the_end_of_a_cut_line_copies_all_of_it() {
    let line = format!("{}THE END", "ab ".repeat(10_000));
    let (_, mut a) = temp_file("cut-copy", &format!("{line}\nnext"));
    press(&mut a, KeyCode::Char('v'), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('v'), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(a.clipboard.take().as_deref(), Some(line.as_str()));
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Home, KeyModifiers::CONTROL);
    let to_end = KeyModifiers::SHIFT | KeyModifiers::CONTROL;
    press(&mut a, KeyCode::Right, to_end);
    press(&mut a, KeyCode::Right, to_end);
    assert!(a.file_selection().is_some());
    press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(a.clipboard.take().as_deref(), Some(line.as_str()));
}

/// #284: `merl file:1:25000` on a line cut at 20 KB opens on the drawn end.
#[test]
fn opening_at_a_column_past_the_cut_lands_on_the_drawn_end() {
    let (path, a) = temp_file("cut-open", &format!("{}THE END\n", "ab ".repeat(10_000)));
    let a = App::new(
        a.root.clone(),
        Tree::default(),
        Vec::new(),
        Buffer::load(&path).unwrap(),
        Some((1, 25_000)),
    );
    assert_eq!((a.line, a.col), (0, a.buf.shown(0).len()));
}
