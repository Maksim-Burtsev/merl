//! `/`: find in the open file.

use super::*;

/// The find prompt is edited in place: text typed in front of the query narrows the search
/// at once, and a selected query goes with one Backspace, which puts the cursor back.
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
    assert_eq!((&*a.prompt, a.line, a.col), ("func now", 1, 0));
    // Option+Left as Ghostty sends it, Esc b, moves by a word and leaves the prompt open.
    press(&mut a, KeyCode::Char('b'), KeyModifiers::ALT);
    assert_eq!((a.mode, a.prompt.cursor()), (Mode::Find, 0));
    press(&mut a, KeyCode::Right, KeyModifiers::ALT);
    press(&mut a, KeyCode::Right, KeyModifiers::NONE);
    press(&mut a, KeyCode::End, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!((&*a.prompt, a.line), ("func ", 1));
    // Ctrl+U is the line's, not a `u` typed into it.
    press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
    assert_eq!((&*a.prompt, a.line, a.mode), ("", 0, Mode::Find));
    // The goto prompt still takes digits only.
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    for c in ":1x2".chars() {
        press(&mut a, KeyCode::Char(c), KeyModifiers::NONE);
    }
    assert_eq!(&*a.prompt, "12");
}

#[test]
fn find_reopens_with_the_active_query_selected() {
    let mut a = app("now\nfunc now\nx\n");
    find(&mut a, "now");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    assert_eq!((&*a.prompt, a.prompt.selection()), ("now", Some(0..3)));
    // Home edits the old query; the search follows.
    press(&mut a, KeyCode::Home, KeyModifiers::NONE);
    typed(&mut a, "func ");
    assert_eq!((&*a.prompt, a.line), ("func now", 1));
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    // Typing replaces it whole.
    find(&mut a, "x");
    assert_eq!((&*a.prompt, a.line), ("x", 2));
    // Esc in navigation clears the pattern, so the next `/` opens empty.
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    assert_eq!(&*a.prompt, "");
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
fn find_is_smart_case() {
    let mut a = app("foo\nFoo\nbar\n");
    // An all-lowercase query is case-insensitive: it stops on `Foo` under the anchor.
    a.line = 1;
    find(&mut a, "foo");
    assert_eq!((a.line, a.col), (1, 0));
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(a.mode, Mode::Normal);

    // One uppercase letter makes it case-sensitive: `foo` on line 1 is skipped.
    a.line = 0;
    find(&mut a, "Foo");
    assert_eq!((a.line, a.col), (1, 0));
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

/// #82: a key that cannot act says why, so "not found" never reads as "not pressed".
#[test]
fn no_key_is_silent_on_an_empty_line() {
    let mut a = app("\nfoo\n");
    for key in ['d', 'u', 'D', 'n', 'N', '[', ']', 'c', 'C'] {
        press(&mut a, KeyCode::Char(key), KeyModifiers::NONE);
        assert!(!a.message.is_empty(), "`{key}` said nothing");
        assert_eq!(a.mode, Mode::Normal, "`{key}`");
    }
    assert_eq!(a.message, "not in review mode");
    // Esc with nothing to clear no longer claims `find cleared`.
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(a.message, "");
    // `/` typing a query the file does not have, and one it has.
    find(&mut a, "zzz");
    assert_eq!(a.message, "no match");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(a.message, "");
    find(&mut a, "foo");
    assert_eq!(a.message, "1/1");
    // Reopened with the query still active, the count is there before a key is typed.
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
    assert_eq!(a.message, "1/1");
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
