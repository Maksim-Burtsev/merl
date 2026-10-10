//! Missed keys (#210): a key that would have reached the same end in at most half the presses.

use std::time::Duration;

use super::*;

const FAST: Duration = Duration::from_millis(30);
const SLOW: Duration = Duration::from_millis(250);
const NONE: KeyModifiers = KeyModifiers::NONE;

/// Presses `code` `n` times, `gap` apart, then Esc: a run is judged when another key comes.
fn hold(a: &mut App, code: KeyCode, m: KeyModifiers, n: u32, gap: Duration) {
    let t = Instant::now();
    for i in 0..n {
        a.key_at(KeyEvent::new(code, m), t + gap * i);
    }
    a.key_at(KeyEvent::new(KeyCode::Esc, NONE), t + gap * n);
}

fn missed(of: &[(&'static str, u64)]) -> HashMap<&'static str, u64> {
    HashMap::from_iter(of.iter().copied())
}

fn open_by_name(a: &mut App, query: &str) {
    press(a, KeyCode::Char('o'), NONE);
    typed(a, query);
    a.picker.as_mut().unwrap().settle();
    press(a, KeyCode::Enter, NONE);
}

#[test]
fn a_held_arrow_misses_the_key_that_covers_the_distance() {
    let lines = "x\n".repeat(60);
    let words = "alpha beta gamma delta\n";
    for (name, text, code, m, n, key) in [
        (
            "two screens of ten rows: PgDn twice",
            lines.as_str(),
            KeyCode::Down,
            NONE,
            20,
            "PgDn",
        ),
        (
            "half a screen and two rows: Ctrl+D Down Down, three presses of seven",
            lines.as_str(),
            KeyCode::Down,
            NONE,
            7,
            "Ctrl+D",
        ),
        (
            "two words and a gap",
            words,
            KeyCode::Right,
            NONE,
            10,
            "Alt+Right",
        ),
        (
            "two screens selected: Shift+PgDn twice",
            lines.as_str(),
            KeyCode::Down,
            KeyModifiers::SHIFT,
            20,
            "Shift+PgDn",
        ),
        (
            "`alpha` selected: Alt+Shift+Right, and `v` too, which comes later in the list",
            words,
            KeyCode::Right,
            KeyModifiers::SHIFT,
            5,
            "Alt+Shift+Right",
        ),
    ] {
        let mut a = app(text);
        hold(&mut a, code, m, n, FAST);
        assert_eq!(
            a.missed,
            missed(&[(key, 1)]),
            "{name}: {n} × {m:?} {code:?}"
        );
    }
}

#[test]
fn a_held_backspace_or_delete_misses_the_word_chord() {
    for (col, code, key) in [
        (21, KeyCode::Backspace, "Edit: Alt+Backspace"),
        (11, KeyCode::Delete, "Edit: Alt+Delete"),
    ] {
        let mut a = app("let name = value_here;\n");
        a.col = col;
        press(&mut a, KeyCode::Enter, NONE);
        hold(&mut a, code, NONE, 10, FAST);
        assert_eq!(a.line_str(), "let name = ;");
        assert_eq!(
            a.missed,
            missed(&[(key, 1)]),
            "{code:?} held in edit mode took a word"
        );
    }
}

#[test]
fn a_held_arrow_in_a_picker_misses_its_page_key() {
    let names: Vec<String> = (0..30).map(|i| format!("f{i:02}.txt")).collect();
    let files: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "x\n")).collect();
    let (dir, mut a) = project_app("missed-picker", &files);
    press(&mut a, KeyCode::Char('o'), NONE);
    a.picker.as_mut().unwrap().settle();
    hold(&mut a, KeyCode::Down, NONE, 20, FAST);
    assert_eq!(a.missed, missed(&[("Picker: PgDn", 1)]));
    a.missed.clear();
    press(&mut a, KeyCode::Char('o'), NONE);
    a.picker.as_mut().unwrap().settle();
    hold(&mut a, KeyCode::Down, NONE, 20, SLOW);
    assert!(
        a.missed.is_empty(),
        "the same rows read one by one count nothing: {:?}",
        a.missed
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_run_that_is_slow_short_or_needs_a_typed_key_counts_nothing() {
    let lines = "x\n".repeat(60);
    let mut a = app(&lines);
    hold(&mut a, KeyCode::Down, NONE, 20, SLOW);
    assert!(a.missed.is_empty(), "every gap over 200 ms: {:?}", a.missed);
    let mut a = app(&lines);
    hold(&mut a, KeyCode::Down, NONE, 4, FAST);
    assert!(a.missed.is_empty(), "Ctrl+D Up saves two: {:?}", a.missed);
    let mut a = app(&lines);
    a.line = 58;
    hold(&mut a, KeyCode::Down, NONE, 10, FAST);
    assert_eq!(
        a.line, 59,
        "held on the last line but one: one press moved, and the rest are no presses spent"
    );
    assert!(a.missed.is_empty(), "past the end: {:?}", a.missed);
    let mut a = app("let name = value_here;\n");
    a.col = 21;
    press(&mut a, KeyCode::Enter, NONE);
    hold(&mut a, KeyCode::Backspace, NONE, 4, FAST);
    assert!(
        a.missed.is_empty(),
        "`here` alone: Alt+Backspace takes all of `value_here`: mid-word: {:?}",
        a.missed
    );
    let text = format!("{}\n{}", "x\n".repeat(8), "x\n".repeat(30));
    let mut a = app(&text);
    a.view_h = 40;
    hold(&mut a, KeyCode::Down, NONE, 8, FAST);
    assert_eq!(
        a.missed,
        missed(&[("}", 1)]),
        "onto the blank line: `}}` in navigation, nothing in edit mode, where `}}` types. A \
         screen of forty rows puts the paging keys far past it"
    );
    let mut a = app(&text);
    a.view_h = 40;
    press(&mut a, KeyCode::Enter, NONE);
    hold(&mut a, KeyCode::Down, NONE, 8, FAST);
    assert!(a.missed.is_empty(), "edit mode: {:?}", a.missed);
    assert_eq!(
        a.buf.lines.join("\n") + "\n",
        text,
        "nor is it tried there: a try would have typed it"
    );
}

const HANDLER: &str = "def handler(x):\n    pass\n\nhandler(1)\nhandler(2)\nHandler = 3\n";

/// `s` from `handler(1)`: the query, `downs` rows down, Enter. `settle`: the grep answers before
/// the Enter; without it the Enter waits for the answer, which then jumps.
fn s(a: &mut App, dir: &Path, query: &str, downs: usize, settle: bool) {
    a.jump_to(&dir.join("app.py"), 4);
    press(a, KeyCode::Char('s'), NONE);
    typed(a, query);
    if settle {
        a.settle_search();
    }
    for _ in 0..downs {
        press(a, KeyCode::Down, NONE);
    }
    press(a, KeyCode::Enter, NONE);
    a.settle_search();
}

#[test]
fn s_for_the_word_under_the_cursor_misses_d_or_u() {
    let (dir, mut a) = project_app("missed-s", &[("app.py", HANDLER)]);
    s(&mut a, &dir, "handler", 0, true);
    assert_eq!(at(&a), (dir.join("app.py"), 0));
    assert_eq!(
        a.missed,
        missed(&[("d", 1)]),
        "nine presses to the declaration; `d` is one"
    );
    s(&mut a, &dir, "handler", 0, false);
    assert_eq!(at(&a), (dir.join("app.py"), 0));
    assert_eq!(
        a.missed,
        missed(&[("d", 2)]),
        "the same with the Enter ahead of the grep"
    );
    s(&mut a, &dir, "Handler", 2, true);
    assert_eq!(at(&a), (dir.join("app.py"), 4));
    assert_eq!(
        a.missed,
        missed(&[("d", 2), ("u", 1)]),
        "eleven presses to `handler(2)`, the third row of `u`: `u` Down Down Enter is four"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn s_for_another_word_or_hit_counts_nothing() {
    let (dir, mut a) = project_app("missed-s-not", &[("app.py", HANDLER)]);
    s(&mut a, &dir, "handle", 0, true);
    assert_eq!(at(&a), (dir.join("app.py"), 0));
    s(&mut a, &dir, "handler", 3, true);
    assert_eq!(a.line_str(), "Handler = 3");
    assert!(
        a.missed.is_empty(),
        "another query, and a hit `u` does not list, of another case: {:?}",
        a.missed
    );
    let (dir2, mut a) = project_app(
        "missed-s-short",
        &[("app.py", "def x():\n    pass\n\nx()\n")],
    );
    a.jump_to(&dir2.join("app.py"), 4);
    press(&mut a, KeyCode::Char('s'), NONE);
    typed(&mut a, "x");
    a.settle_search();
    press(&mut a, KeyCode::Enter, NONE);
    assert_eq!(at(&a), (dir2.join("app.py"), 0));
    assert!(
        a.missed.is_empty(),
        "`s x Enter` to `def x`: three presses, `d` saves two: {:?}",
        a.missed
    );
    std::fs::remove_dir_all(dir).unwrap();
    std::fs::remove_dir_all(dir2).unwrap();
}

#[test]
fn s_for_a_primed_name_under_the_cursor_misses_d() {
    let text = "let\n  discount' = 4;\n  discount = 5;\nin\n  discount' + discount\n";
    let (dir, mut a) = project_app("missed-s-primed", &[("default.nix", text)]);
    a.jump_to(&dir.join("default.nix"), 5);
    a.col = 2;
    press(&mut a, KeyCode::Char('s'), NONE);
    typed(&mut a, "discount'");
    a.settle_search();
    press(&mut a, KeyCode::Enter, NONE);
    a.settle_search();
    assert_eq!(at(&a), (dir.join("default.nix"), 1));
    assert_eq!(
        a.missed,
        missed(&[("d", 1)]),
        "a Nix name keeps its trailing primes"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn s_for_a_ruby_instance_variable_under_the_cursor_misses_d() {
    let text = "class Cart\n  def initialize\n    @subtotal = 0\n  end\n\n  def add(x)\n    @subtotal += x\n  end\nend\n";
    let (dir, mut a) = project_app("missed-s-ivar", &[("cart.rb", text)]);
    a.jump_to(&dir.join("cart.rb"), 7);
    a.col = 5;
    press(&mut a, KeyCode::Char('s'), NONE);
    typed(&mut a, "subtotal");
    a.settle_search();
    press(&mut a, KeyCode::Enter, NONE);
    a.settle_search();
    assert_eq!(at(&a), (dir.join("cart.rb"), 2));
    assert_eq!(
        a.missed,
        missed(&[("d", 1)]),
        "on `@subtotal`, `s subtotal` is the query for the word"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn symbols_for_the_word_under_the_cursor_miss_d() {
    let text = "def handler(x):\n    pass\n\ndef handle(y):\n    pass\n\nhandler(1)\ndef x():\n    pass\nx()\n";
    let (dir, mut a) = project_app("missed-symbols", &[("app.py", text)]);
    let symbols = |a: &mut App, line: usize, query: &str| {
        a.jump_to(&dir.join("app.py"), line);
        press(a, KeyCode::Char('D'), NONE);
        typed(a, query);
        a.picker.as_mut().unwrap().settle();
        press(a, KeyCode::Enter, NONE);
    };
    symbols(&mut a, 7, "handl");
    assert!(a.missed.is_empty(), "another query: {:?}", a.missed);
    symbols(&mut a, 10, "x");
    assert_eq!(at(&a), (dir.join("app.py"), 7));
    assert!(
        a.missed.is_empty(),
        "a word too short to save three presses: {:?}",
        a.missed
    );
    symbols(&mut a, 7, "handler");
    assert_eq!(at(&a), (dir.join("app.py"), 0));
    assert_eq!(a.missed, missed(&[("d", 1)]));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn o_to_a_file_of_the_jump_history_misses_the_brackets() {
    let (dir, mut a) = project_app("missed-o", &[("a.rs", "x\n"), ("b.rs", "x\n")]);
    a.jump_to(&dir.join("a.rs"), 1);
    a.jump_to(&dir.join("b.rs"), 1);
    open_by_name(&mut a, "a.rs");
    assert_eq!(at(&a), (dir.join("a.rs"), 0));
    assert_eq!(a.missed, missed(&[("[", 1)]));
    press(&mut a, KeyCode::Char('['), NONE);
    press(&mut a, KeyCode::Char('['), NONE);
    open_by_name(&mut a, "b.rs");
    assert_eq!(at(&a), (dir.join("b.rs"), 0));
    assert_eq!(
        a.missed,
        missed(&[("[", 1), ("]", 1)]),
        "back to the first stop, `b.rs` one step forward"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn o_too_far_too_short_or_from_edit_mode_counts_nothing() {
    let names = ["alpha.rs", "bravo.rs", "cedar.rs", "dune.rs", "kiwi.rs"];
    let files: Vec<(&str, &str)> = names.iter().map(|n| (*n, "x\n")).collect();
    let (dir, mut a) = project_app("missed-o-not", &files);
    for name in names {
        a.jump_to(&dir.join(name), 1);
    }
    open_by_name(&mut a, "alpha.rs");
    assert_eq!(
        at(&a),
        (dir.join("alpha.rs"), 0),
        "ten presses, and `[` four times would save six, but four steps is past the reach"
    );
    open_by_name(&mut a, "k");
    assert_eq!(
        at(&a),
        (dir.join("kiwi.rs"), 0),
        "`kiwi` is one step back, and `o k` Enter is three presses"
    );
    press(&mut a, KeyCode::Enter, NONE);
    press(&mut a, KeyCode::Char('e'), KeyModifiers::CONTROL);
    typed(&mut a, "dune.rs");
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Enter, NONE);
    assert_eq!(
        at(&a),
        (dir.join("dune.rs"), 0),
        "`dune` is three steps back, nine presses away, but from edit mode"
    );
    assert!(a.missed.is_empty(), "{:?}", a.missed);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn review_runs_into_a_hunk_or_o_to_the_next_file_miss_c() {
    let (dir, mut a) = review_app("missed-review");
    open_by_name(&mut a, "tail");
    assert_eq!(a.rel_path(), "tail");
    assert_eq!(
        a.missed,
        missed(&[("c", 1)]),
        "`new` is open on its only hunk: `c` goes on to `tail`"
    );
    a.jump_to(&dir.join("src/a.rs"), 2);
    hold(&mut a, KeyCode::Down, NONE, 4, FAST);
    assert_eq!(
        a.line_str(),
        "f",
        "src/a.rs: hunks on `B` and `F`, lines 2 and 6, each under the line it rewrites (#439), \
         where `c` stands (#690); a run onto the rewritten line is into the hunk too"
    );
    hold(&mut a, KeyCode::Up, NONE, 4, FAST);
    assert_eq!(a.line_str(), "B");
    assert_eq!(
        a.missed,
        missed(&[("c", 2), ("C", 1)]),
        "a run of arrows into the next hunk misses `c`, into the previous one `C`"
    );
    press(&mut a, KeyCode::Char('c'), NONE);
    press(&mut a, KeyCode::Char('c'), NONE);
    assert_eq!(a.rel_path(), "crlf.txt");
    a.jump_to(&dir.join("src/keep.rs"), 1);
    open_by_name(&mut a, "src/a.rs");
    assert_eq!(a.rel_path(), "src/a.rs");
    assert_eq!(
        a.missed["c"], 2,
        "#239: from a file outside the review, `c` goes back to the hunk it left, in crlf.txt: {:?}",
        a.missed
    );
    a.jump_to(&dir.join("src/keep.rs"), 1);
    open_by_name(&mut a, "crlf.txt");
    assert_eq!(
        a.missed["c"], 3,
        "`o` from outside the review to the file of the hunk `c` left: {:?}",
        a.missed
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn review_runs_that_are_slow_pass_the_hunk_or_type_count_no_c() {
    let (dir, mut a) = review_app("missed-review-not");
    a.jump_to(&dir.join("src/a.rs"), 2);
    hold(&mut a, KeyCode::Down, NONE, 4, SLOW);
    assert_eq!(a.line_str(), "f");
    a.jump_to(&dir.join("src/a.rs"), 1);
    hold(&mut a, KeyCode::Down, NONE, 4, FAST);
    assert_eq!(a.line_str(), "d");
    assert!(
        a.missed.is_empty(),
        "a run read line by line, and one that passes the hunk: {:?}",
        a.missed
    );
    a.jump_to(&dir.join("src/a.rs"), 2);
    press(&mut a, KeyCode::Enter, NONE);
    hold(&mut a, KeyCode::Down, NONE, 4, FAST);
    assert_eq!(a.line_str(), "f");
    assert!(!a.missed.contains_key("c"), "edit mode: {:?}", a.missed);
    a.missed.clear();
    a.jump_to(&dir.join("src/a.rs"), 1);
    open_by_name(&mut a, "crlf.txt");
    assert_eq!(a.rel_path(), "crlf.txt");
    assert!(
        a.missed.is_empty(),
        "`crlf.txt` comes after `src/a.rs`, but `c` still has `B` and `F` to go to: {:?}",
        a.missed
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn o_from_a_fold_to_the_next_file_misses_c() {
    let (dir, mut a) = review_app_with_lock("missed-fold");
    press(&mut a, KeyCode::Char('c'), NONE);
    assert!(a.folded_here().is_some());
    a.missed.clear();
    open_by_name(&mut a, "tail");
    assert_eq!(a.rel_path(), "tail");
    assert_eq!(
        a.missed,
        missed(&[("c", 1)]),
        "on a fold `c` goes on to the next file, whatever hunks the fold hides"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_held_arrow_may_miss_a_key_that_overshoots_and_steps_back() {
    let mut a = app(&"x\n".repeat(60));
    hold(&mut a, KeyCode::Down, NONE, 9, FAST);
    assert_eq!((a.line, &a.missed), (9, &missed(&[("PgDn", 1)])));
}

#[test]
fn a_run_the_file_changed_under_counts_nothing() {
    let mut a = app(&"x\n".repeat(60));
    let t = Instant::now();
    for i in 0..20 {
        a.key_at(KeyEvent::new(KeyCode::Down, NONE), t + FAST * i);
    }
    std::fs::write(a.buf.path.clone().unwrap(), "x\n".repeat(70)).unwrap();
    a.reload(false);
    a.key_at(KeyEvent::new(KeyCode::Esc, NONE), t + FAST * 20);
    assert!(a.missed.is_empty(), "{:?}", a.missed);
}

fn added_hunk_review(tag: &str) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    let lines = |r: std::ops::Range<usize>| r.map(|i| format!("l{i}\n")).collect::<String>();
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "t@t"]);
    git(&["config", "user.name", "t"]);
    std::fs::write(dir.join("m.rs"), lines(0..30)).unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    let added: String = (0..5).map(|i| format!("new{i}\n")).collect();
    std::fs::write(dir.join("m.rs"), lines(0..10) + &added + &lines(10..30)).unwrap();
    git(&["commit", "-qam", "work"]);
    let review = git::Review::open(&dir, None, None).unwrap();
    let (_, files) = crate::tree::build(&dir, false);
    let mut a = App::new(dir.clone(), Tree::default(), files, Buffer::empty(), None);
    a.start_review(review);
    a.view_w = 40;
    a.view_h = 16;
    (dir, a)
}

#[test]
fn a_run_into_a_hunk_misses_c_before_any_other_key() {
    let (dir, mut a) = added_hunk_review("missed-hunk-first");
    let m = dir.join("m.rs");
    for (downs, line) in [(8, 10), (10, 12)] {
        a.jump_to(&m, 3);
        a.missed.clear();
        hold(&mut a, KeyCode::Down, NONE, downs, FAST);
        assert_eq!(
            (a.line, &a.missed),
            (line, &missed(&[("c", 1)])),
            "{downs} × Down: Ctrl+D reaches it too, and anywhere in the hunk is `c`'s"
        );
    }
    a.jump_to(&m, 3);
    a.missed.clear();
    hold(&mut a, KeyCode::Down, KeyModifiers::SHIFT, 8, FAST);
    assert!(
        !a.missed.contains_key("c"),
        "`c` selects nothing: {:?}",
        a.missed
    );
    std::fs::remove_dir_all(dir).unwrap();
}
