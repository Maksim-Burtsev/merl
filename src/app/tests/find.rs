use super::*;

#[test]
fn find_prompt_is_edited_at_the_cursor() {
    let mut a = app("now()\nfunc now()\n");
    for c in "/now".chars() {
        press(&mut a, KeyCode::Char(c), KeyModifiers::NONE);
    }
    assert_eq!((a.line, a.col), (0, 0));
    press(&mut a, KeyCode::Home, KeyModifiers::NONE);
    for c in "func ".chars() {
        press(&mut a, KeyCode::Char(c), KeyModifiers::NONE);
    }
    assert_eq!(
        (&*a.prompt, a.line, a.col),
        ("func now", 1, 0),
        "text typed in front of the query narrows the search at once"
    );
    press(&mut a, KeyCode::Char('b'), KeyModifiers::ALT);
    assert_eq!(
        (a.mode, a.prompt.cursor()),
        (Mode::Find, 0),
        "Option+Left as Ghostty sends it, Esc b, moves by a word and leaves the prompt open"
    );
    press(&mut a, KeyCode::Right, KeyModifiers::ALT);
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    press(&mut a, KeyCode::End, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(
        (&*a.prompt, a.line),
        ("func ", 1),
        "a selected part of the query goes with one Backspace"
    );
    press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
    assert_eq!(
        (&*a.prompt, a.line, a.mode),
        ("", 0, Mode::Find),
        "Ctrl+U is the line's, not a `u` typed into it"
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    for c in ":1x2".chars() {
        press(&mut a, KeyCode::Char(c), KeyModifiers::NONE);
    }
    assert_eq!(&*a.prompt, "12", "the goto prompt still takes digits only");
}

#[test]
fn find_reopens_with_the_active_query_selected() {
    let mut a = app("now\nfunc now\nx\n");
    find(&mut a, "now");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    assert_eq!((&*a.prompt, a.prompt.selection()), ("now", Some(0..3)));
    press(&mut a, KeyCode::Home, KeyModifiers::NONE);
    typed(&mut a, "func ");
    assert_eq!(
        (&*a.prompt, a.line),
        ("func now", 1),
        "Home edits the old query; the search follows"
    );
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    find(&mut a, "x");
    assert_eq!((&*a.prompt, a.line), ("x", 2), "typing replaces it whole");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    assert_eq!(
        &*a.prompt, "",
        "Esc in navigation clears the pattern, so the next `/` opens empty"
    );
}

#[test]
fn emptying_the_query_drops_the_pattern_and_returns_to_the_anchor() {
    let mut a = app("foo\nbar\nbaz\n");
    find(&mut a, "ba");
    assert_eq!(a.line, 1);
    assert!(a.find_re.is_some());
    press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
    press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
    assert!(
        a.find_re.is_none(),
        "an empty query must not keep old highlights"
    );
    assert_eq!(a.line, 0, "cursor returns to the anchor");
}

#[test]
fn find_ignores_case() {
    let mut a = app("foo\nFoo\nbar\nfmt.Println(SameCancel(a, b))\n");
    a.line = 1;
    find(&mut a, "foo");
    assert_eq!(
        (a.line, a.col),
        (1, 0),
        "a lowercase query stops on `Foo` under the anchor"
    );
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(a.mode, Mode::Normal);

    a.line = 0;
    find(&mut a, "Foo");
    assert_eq!(
        (a.line, a.col),
        (0, 0),
        "so does a capital: `Foo` stops on `foo` under the anchor"
    );
    assert_eq!(a.message, "1/2");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);

    find(&mut a, "sameCancel");
    assert_eq!(
        (a.line, a.col),
        (3, 12),
        "a camelCase name typed with the wrong first letter (#174)"
    );
    assert_eq!(a.message, "1/1");
}

#[test]
fn reopening_find_on_a_pattern_with_no_match_says_so() {
    let mut a = app("foo\n");
    find(&mut a, "bar");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    assert_eq!((&*a.prompt, a.message.as_str()), ("bar", "no match"));
}

#[test]
fn next_and_prev_wrap_around() {
    let mut a = app("foo\nbar\nfoo\n");
    find(&mut a, "foo");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (0, 0));

    press(&mut a, KeyCode::Char('n'), KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (2, 0));
    assert_eq!(a.message, "2/2");
    press(&mut a, KeyCode::Char('n'), KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (0, 0));
    assert_eq!(a.message, "1/2");

    press(&mut a, KeyCode::Char('N'), KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (2, 0));
    assert_eq!(a.message, "2/2");
    press(&mut a, KeyCode::Char('N'), KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (0, 0));
    assert_eq!(a.message, "1/2");

    let mut a = app("nothing here\n");
    find(&mut a, "zzz");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('n'), KeyModifiers::NONE);
    assert_eq!(a.message, "no match");
}

#[test]
fn no_key_is_silent_on_an_empty_line() {
    let mut a = app("\nfoo\n");
    for key in ['d', 'u', 'D', 'n', 'N', '[', ']'] {
        press(&mut a, KeyCode::Char(key), KeyModifiers::NONE);
        assert!(
            !a.message.is_empty(),
            "`{key}` said nothing, and \"not found\" reads as \"not pressed\""
        );
        assert_eq!(a.mode, Mode::Normal, "`{key}`");
    }
    a.message.clear();
    for key in ['c', 'C', 'm'] {
        press(&mut a, KeyCode::Char(key), KeyModifiers::NONE);
        assert_eq!(
            a.message, "",
            "`{key}` is no key outside a review: nothing happens, nothing is said"
        );
    }
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(
        a.message, "",
        "Esc with nothing to clear no longer claims `find cleared`"
    );
    find(&mut a, "zzz");
    assert_eq!(
        a.message, "no match",
        "`/` typing a query the file does not have"
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(a.message, "");
    find(&mut a, "foo");
    assert_eq!(a.message, "1/1", "and one it has");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    assert_eq!(
        a.message, "1/1",
        "reopened with the query still active, the count is there before a key is typed"
    );
}

#[test]
fn find_is_literal_not_a_regex() {
    let mut a = app("foo\nMigrator()\nbar\n");
    find(&mut a, "migrator(");
    assert_eq!((a.line, a.col), (1, 0));
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);

    find(&mut a, "f.o");
    let re = a.find_re.as_ref().unwrap();
    assert!(!re.is_match("foo"), "`.` is a dot, not any-char");
    assert!(re.is_match("f.o"));
}

#[test]
fn esc_restores_the_anchor() {
    let mut a = app("foo\nbar\nbaz\n");
    a.line = 2;
    a.col = 1;
    find(&mut a, "foo");
    assert_eq!(
        (a.line, a.col),
        (0, 0),
        "incremental search moved the cursor"
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(a.mode, Mode::Normal);
    assert_eq!((a.line, a.col), (2, 1));
}

#[test]
fn the_query_is_capped_so_it_always_compiles() {
    let mut a = app("яяя\n");
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    a.paste(&"Я".repeat(70_000));
    assert_eq!(
        (a.prompt.chars().count(), a.message.as_str()),
        (1_000, "no match"),
        "a paste is cut to the 1,000 chars that fit"
    );
    assert!(
        a.find_re
            .as_ref()
            .is_some_and(|re| re.is_match(&"я".repeat(1_000))),
        "1,000 Cyrillic letters compile ignoring case, under the `regex` crate's size limit"
    );
    typed(&mut a, "я");
    assert_eq!(
        a.prompt.chars().count(),
        1_000,
        "typing past the cap does nothing"
    );
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    a.paste(&"я".repeat(1_001));
    assert_eq!(
        (a.prompt.chars().count(), a.message.as_str()),
        (1_000, "no match"),
        "reopened, the query is selected: a paste replaces it, as much of it as fits"
    );
    press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
    a.paste("яя\r\nmore");
    assert_eq!(
        (&*a.prompt, a.message.as_str()),
        ("яя", "1/1"),
        "a paste whose first line is empty types nothing, and leaves the query as it was"
    );
    press(&mut a, KeyCode::Home, KeyModifiers::SHIFT);
    a.paste("\nyy");
    assert_eq!((&*a.prompt, a.prompt.selection()), ("яя", Some(0..4)));
    a.paste(&"ι".repeat(1_001));
    assert_eq!(
        (a.prompt.chars().count(), a.message.as_str()),
        (1_000, "no match"),
        "Greek iota, with four case forms, compiles to a bigger regex still"
    );
}

#[test]
fn a_selection_in_one_line_seeds_find() {
    let mut a = app("now = 1\nx = now\nNOW\n");
    press(&mut a, KeyCode::Char('v'), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    assert_eq!(
        (&*a.prompt, a.prompt.selection(), a.line, a.message.as_str()),
        ("now", Some(0..3), 1, "2/3")
    );
    typed(&mut a, "x");
    assert_eq!((&*a.prompt, a.line), ("x", 1));
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!((a.line, a.selected_text().as_deref()), (0, Some("now")));
}

#[test]
fn a_selection_over_two_lines_opens_find_as_without_one() {
    let mut a = app("now = 1\nx = now\n");
    find(&mut a, "x");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    a.line = 0;
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    assert_eq!((&*a.prompt, a.prompt.selection()), ("x", Some(0..1)));
}

#[test]
fn a_seed_is_cut_to_the_cap() {
    let mut a = app(&format!("{}\n", "a".repeat(1_005)));
    press(&mut a, KeyCode::Char('v'), KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    assert_eq!(a.prompt.chars().count(), 1_000);
}

#[test]
fn typing_after_a_find_from_edit_mode_is_an_undo_step_of_its_own() {
    let mut a = app("ab\n");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "x");
    press(&mut a, KeyCode::Char('f'), KeyModifiers::CONTROL);
    typed(&mut a, "a");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((a.mode, a.line, a.col), (Mode::Edit, 0, 1));
    typed(&mut a, "y");
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(a.buf.lines[0], "xab");
}

#[test]
fn find_reaches_a_match_past_the_cut_and_draws_the_line_on_to_it() {
    let pad = "ab ".repeat(10_000);
    let text = format!("{pad}THE END {pad}THE END\nTHE END\n{pad}\n");
    let (_, mut a) = temp_file("find-past-cut", &text);
    assert_eq!(a.buf.shown(0).len(), 20_000);
    find(&mut a, "the end");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        (a.line, a.col, a.message.as_str(), a.buf.shown(0).len()),
        (0, 30_000, "1/3", 50_000),
        "the first match is past the cut: the cursor is on it and the line is drawn 20 KB past it"
    );
    assert_eq!(
        a.buf.shown(2).len(),
        20_000,
        "another long line keeps its cut"
    );
    press(&mut a, KeyCode::Char('n'), KeyModifiers::NONE);
    assert_eq!((a.line, a.col, a.message.as_str()), (0, 60_008, "2/3"));
    assert!(
        !Buffer::clips(&a.buf.lines[0]),
        "the line is now drawn whole"
    );
    press(&mut a, KeyCode::Char('n'), KeyModifiers::NONE);
    assert_eq!((a.line, a.col, a.message.as_str()), (1, 0, "3/3"));
    press(&mut a, KeyCode::Char('N'), KeyModifiers::NONE);
    assert_eq!((a.line, a.col, a.message.as_str()), (0, 60_008, "2/3"));
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        (a.mode, a.message.as_str()),
        (Mode::Normal, "line too long to edit"),
        "a line drawn on past 20 KB stays read-only"
    );
}

#[test]
fn an_s_hit_past_the_cut_opens_on_the_hit() {
    let text = format!("{}THE END\n", "ab ".repeat(10_000));
    let (path, mut a) = temp_file("open-past-cut", &text);
    a.jump_to_col(&path, 1, 30_000);
    assert_eq!(
        (a.line, a.col, a.buf.shown(0).len()),
        (0, 30_000, 30_007),
        "the hit opens on itself and the line is drawn on past it"
    );
}

#[test]
fn n_from_the_cut_goes_on_past_it_and_esc_takes_the_reach_back() {
    let text = format!("{}THE END\nTHE END\n", "ab ".repeat(10_000));
    let (_, mut a) = temp_file("find-from-cut", &text);
    press(&mut a, KeyCode::End, KeyModifiers::NONE);
    press(&mut a, KeyCode::End, KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (0, 20_000));
    find(&mut a, "the end");
    assert_eq!(a.buf.shown(0).len(), 30_007);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(
        (a.line, a.col, a.buf.shown(0).len()),
        (0, 20_000, 20_000),
        "Esc puts the cut back with the cursor"
    );
    press(&mut a, KeyCode::Char('n'), KeyModifiers::NONE);
    assert_eq!(
        (a.line, a.col),
        (0, 30_000),
        "`n` from the `…` finds the match past it, not the next line's"
    );
}

#[test]
fn a_line_drawn_whole_by_a_find_gets_its_colour() {
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let text = format!("x = \"{}\"\ny = 1\n", "ab ".repeat(10_000));
    let (_, mut a) = temp_file("find-colour", &text);
    a.buf.parse_more(None);
    a.buf.paint_to(1, &theme);
    assert!(a.buf.hl[0].is_empty(), "a cut line is drawn plain");
    find(&mut a, "\"");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('n'), KeyModifiers::NONE);
    a.buf.parse_more(None);
    a.buf.paint_to(1, &theme);
    assert!(!a.buf.hl[0].is_empty());
}
