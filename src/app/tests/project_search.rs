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
    assert_eq!(
        (p.counts().1, p.current().unwrap().line),
        (3, 2),
        "the cursor stays on its hit while the new query still finds it"
    );
    press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
    assert!(
        a.search_tick().is_none(),
        "an emptied query empties the list at once"
    );
    a.settle_search();
    assert_eq!(a.picker.as_ref().unwrap().counts().1, 0);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

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
        (true, Mode::Normal, 1),
        "the jump waited for the hits"
    );

    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "three");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    a.settle_search();
    let query = a.picker.as_ref().map(|p| p.query.to_string());
    assert_eq!(
        (query.as_deref(), a.mode, &*a.message, a.search_enter),
        (Some("three"), Mode::Picker(PickerKind::Search), "", false),
        "no hit: the Enter is spent, the list stays open with its query (#288)"
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);

    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "one");
    std::thread::sleep(SEARCH_PAUSE);
    let job = a.search_tick().expect("the grep for one");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert!(
        a.picker.is_some(),
        "past the pause, with the grep running: the list on screen is still the older one"
    );
    a.search_done(job.seq, job.items());
    assert_eq!((a.picker.is_none(), a.line), (true, 0));
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn enter_after_the_project_search_answered() {
    let (path, mut a) = temp_file("enter-late-s", "one\ntwo\n");
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "two");
    std::thread::sleep(SEARCH_PAUSE);
    let job = a.search_tick().expect("the grep for two");
    a.search_done(job.seq, job.items());
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        (a.picker.is_none(), a.mode, a.line, &*a.message),
        (true, Mode::Normal, 1, ""),
        "the very next key after the answer, before the event loop has ticked or drawn"
    );

    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "three");
    a.settle_search();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    let query = a.picker.as_ref().map(|p| p.query.to_string());
    assert_eq!(
        (query.as_deref(), a.mode, &*a.message),
        (Some("three"), Mode::Picker(PickerKind::Search), ""),
        "a query that found nothing keeps the list open with its query"
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);

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
            (a.picker.is_some(), a.mode, &*a.message),
            (true, Mode::Picker(PickerKind::Search), ""),
            "no query, {keys:?}: the list stays open"
        );
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    }
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

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
    assert_eq!(
        (top.path.as_path(), top.line),
        (Path::new("z.py"), 2),
        "the open file's hits sort to the top, read first, though `a.py` alone fills the cap"
    );
    assert_eq!(p.counts().1 as usize, search::MAX_HITS);
    std::fs::remove_dir_all(&dir).unwrap();
}

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
    assert_eq!(
        p.title,
        format!(
            "Usages of x: {} in code (first {})",
            search::MAX_HITS,
            search::MAX_HITS
        ),
        "the split counts what the list holds, and the cut still says so behind it"
    );
    assert_eq!(p.counts().1 as usize, search::MAX_HITS);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_selection_in_one_line_seeds_the_project_search() {
    let (path, mut a) = temp_file(
        "seed-s",
        "raise Not authenticated now\nnot authenticated\nnot\n",
    );
    a.col = 6;
    for _ in 0..3 {
        press(
            &mut a,
            KeyCode::Right,
            KeyModifiers::ALT | KeyModifiers::SHIFT,
        );
    }
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    let p = a.picker.as_ref().unwrap();
    assert_eq!(
        (&*p.query, p.query.selection()),
        ("Not authenticated now", Some(0..21))
    );
    assert!(a.search_tick().is_some(), "the grep goes out at once");
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn without_a_one_line_selection_the_project_search_opens_empty() {
    let (path, mut a) = temp_file("seed-none-s", "foo\nbar\n");
    for shift_down in [false, true] {
        if shift_down {
            press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
        }
        press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(&*a.picker.as_ref().unwrap().query, "");
        assert!(!a.search_pending());
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    }
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}
