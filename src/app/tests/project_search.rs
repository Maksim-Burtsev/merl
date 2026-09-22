//! `s`: the project search.

use super::*;

#[test]
fn project_search_is_literal_not_a_regex() {
    let (path, mut a) = temp_file("literal-s", "def foo(x):\nself.foo(1)\nfoo_bar\na.b\naXb\n");
    for (query, hits, why) in [
        ("foo(", 2, "a paren is text, not a regex error"),
        ("a.b", 1, "a dot matches only a dot"),
    ] {
        press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
        typed(&mut a, query);
        a.settle_search();
        let p = a.picker.as_ref().expect(why);
        assert_eq!(p.counts().1, hits, "{why}: {}", a.message);
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    }
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

/// The hits follow the query. An answer to a query that has changed since is dropped, and
/// the cursor stays on its hit while the new query still finds it.
#[test]
fn project_search_refreshes_while_typing() {
    let (path, mut a) = temp_file(
        "live-s",
        "foo
food
foo
",
    );
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    assert!(!a.search_pending(), "nothing asked yet");
    typed(&mut a, "foo");
    assert!(
        a.search_tick().is_none(),
        "the grep waits for a pause in the typing"
    );
    assert!(a.search_pending(), "pending through the pause");
    std::thread::sleep(SEARCH_PAUSE);
    let stale = a.search_tick().expect("the grep for foo").seq;
    assert!(a.search_pending(), "and while the grep runs");
    typed(&mut a, "d");
    assert!(!a.search_done(stale, Vec::new()), "foo is not on screen");
    assert!(a.search_pending(), "a stale answer settles nothing");
    a.settle_search();
    assert!(!a.search_pending());
    assert_eq!(a.picker.as_ref().unwrap().counts().1, 1);

    press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
    a.settle_search();
    let p = a.picker.as_ref().unwrap();
    assert_eq!((p.counts().1, p.current().unwrap().line), (3, 2));
    // An emptied query empties the list at once.
    press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
    assert!(a.search_tick().is_none());
    a.settle_search();
    assert_eq!(a.picker.as_ref().unwrap().counts().1, 0);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

/// Enter before the hits of the query on screen: the jump waits for them.
#[test]
fn enter_waits_for_the_project_search() {
    let (path, mut a) = temp_file(
        "enter-s", "one
two
",
    );
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "two");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert!(a.picker.is_some(), "nothing to jump to yet");
    a.settle_search();
    assert_eq!(
        (a.picker.is_none(), a.mode, a.line),
        (true, Mode::Normal, 1)
    );

    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "three");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    a.settle_search();
    assert_eq!(
        (a.picker.is_none(), &*a.message),
        (true, "no results for three")
    );

    // Past the pause, with the grep running: the list on screen is still the older one.
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "one");
    std::thread::sleep(SEARCH_PAUSE);
    let job = a.search_tick().expect("the grep for one");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert!(a.picker.is_some(), "the grep for one is still running");
    a.search_done(job.seq, job.items());
    assert_eq!((a.picker.is_none(), a.line), (true, 0));
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

/// Enter once the hits are in: the same jump, and the same word for a query that found
/// nothing, as an Enter that came before them. An empty query closes without a word.
#[test]
fn enter_after_the_project_search_answered() {
    let (path, mut a) = temp_file("enter-late-s", "one\ntwo\n");
    // The very next key after the answer, before the event loop has ticked or drawn.
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "two");
    std::thread::sleep(SEARCH_PAUSE);
    let job = a.search_tick().expect("the grep for two");
    a.search_done(job.seq, job.items());
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        (a.picker.is_none(), a.mode, a.line, &*a.message),
        (true, Mode::Normal, 1, "")
    );

    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "three");
    a.settle_search();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        (a.picker.is_none(), &*a.message),
        (true, "no results for three")
    );

    for keys in ["", "x\u{15}"] {
        a.message.clear();
        press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
        for c in keys.chars() {
            match c {
                '\u{15}' => press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL),
                c => press(&mut a, KeyCode::Char(c), KeyModifiers::NONE),
            };
        }
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            (a.picker.is_none(), a.mode, &*a.message),
            (true, Mode::Normal, ""),
            "{keys:?}"
        );
    }
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

/// The grep stops at MAX_HITS. The open file's hits sort to the top, so they are read first
/// and never the ones cut, even when other files alone fill the cap.
#[test]
fn the_cap_never_cuts_the_open_files_hits() {
    let dir = std::env::temp_dir().join(format!("merl-cap-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.py"), "x\n".repeat(search::MAX_HITS)).unwrap();
    std::fs::write(dir.join("z.py"), "one\nx\n").unwrap();
    let files = vec![PathBuf::from("a.py"), PathBuf::from("z.py")];
    let buf = Buffer::load(&dir.join("z.py")).unwrap();
    let mut a = App::new(dir.clone(), Tree::default(), files, buf, None);
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "x");
    a.settle_search();
    let p = a.picker.as_ref().unwrap();
    let top = p.current().unwrap();
    assert_eq!((top.path.as_path(), top.line), (Path::new("z.py"), 2));
    assert_eq!(p.counts().1 as usize, search::MAX_HITS);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A hidden or ignored file is not in the startup walk, but it is still the file under
/// the cursor: `u` finds the usages in it.
#[test]
fn the_open_file_is_searched_even_when_the_walk_skipped_it() {
    let (dir, mut a) = files_app("hidden");
    let x = dir.join("a.rs");
    assert!(a.files.is_empty());
    a.jump_to(&x, 1);
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    assert_eq!(a.mode, Mode::Picker(PickerKind::Usages));
    let p = a.picker.as_mut().unwrap();
    p.settle();
    assert_eq!(p.counts().1, 40);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_truncated_result_list_says_so_in_its_title() {
    let (dir, mut a) = files_app("truncated");
    let x = dir.join("a.rs");
    std::fs::write(&x, "x\n".repeat(search::MAX_HITS + 1)).unwrap();
    a.jump_to(&x, 1);
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    let p = a.picker.as_mut().unwrap();
    p.settle();
    // The split counts what the list holds, and the cut still says so behind it.
    assert_eq!(
        p.title,
        format!(
            "Usages of x: {} in code (first {})",
            search::MAX_HITS,
            search::MAX_HITS
        )
    );
    assert_eq!(p.counts().1 as usize, search::MAX_HITS);
    std::fs::remove_dir_all(&dir).unwrap();
}
