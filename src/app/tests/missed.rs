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

/// `o`, the query, Enter.
fn open_by_name(a: &mut App, query: &str) {
    press(a, KeyCode::Char('o'), NONE);
    typed(a, query);
    a.picker.as_mut().unwrap().settle();
    press(a, KeyCode::Enter, NONE);
}

/// A held arrow or Shift+arrow misses the key that covers the same distance in the fewest
/// presses, pressed again and followed by arrows if need be.
#[test]
fn a_held_arrow_misses_the_key_that_covers_the_distance() {
    let lines = "x\n".repeat(60);
    let words = "alpha beta gamma delta\n";
    for (text, code, m, n, key) in [
        // Two screens of ten rows: PgDn twice.
        (lines.as_str(), KeyCode::Down, NONE, 20, "PgDn"),
        // Half a screen and two rows: Ctrl+D Down Down, three presses of seven.
        (lines.as_str(), KeyCode::Down, NONE, 7, "Ctrl+D"),
        (words, KeyCode::Right, NONE, 10, "Alt+Right"),
        // `alpha` selected: Alt+Shift+Right, and `v` too, which comes later in the list.
        (
            words,
            KeyCode::Right,
            KeyModifiers::SHIFT,
            5,
            "Alt+Shift+Right",
        ),
    ] {
        let mut a = app(text);
        hold(&mut a, code, m, n, FAST);
        assert_eq!(a.missed, missed(&[(key, 1)]), "{n} × {m:?} {code:?}");
    }
}

/// In edit mode a held Backspace or Delete that took a word misses Alt+Backspace / Alt+Delete.
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
        assert_eq!(a.missed, missed(&[(key, 1)]), "{code:?}");
    }
}

/// A held arrow in a picker misses Picker: PgDn.
#[test]
fn a_held_arrow_in_a_picker_misses_its_page_key() {
    let names: Vec<String> = (0..30).map(|i| format!("f{i:02}.txt")).collect();
    let files: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "x\n")).collect();
    let (dir, mut a) = project_app("missed-picker", &files);
    press(&mut a, KeyCode::Char('o'), NONE);
    a.picker.as_mut().unwrap().settle();
    hold(&mut a, KeyCode::Down, NONE, 20, FAST);
    assert_eq!(a.missed, missed(&[("Picker: PgDn", 1)]));
    // The same rows read one by one count nothing.
    a.missed.clear();
    press(&mut a, KeyCode::Char('o'), NONE);
    a.picker.as_mut().unwrap().settle();
    hold(&mut a, KeyCode::Down, NONE, 20, SLOW);
    assert!(a.missed.is_empty(), "{:?}", a.missed);
    std::fs::remove_dir_all(dir).unwrap();
}

/// No miss for a run read line by line, for one where the key saves fewer than three presses,
/// for a deletion no count of the word chord matches, or for a key the run's mode types.
#[test]
fn a_run_that_is_slow_short_or_needs_a_typed_key_counts_nothing() {
    let lines = "x\n".repeat(60);
    let mut a = app(&lines);
    hold(&mut a, KeyCode::Down, NONE, 20, SLOW);
    assert!(a.missed.is_empty(), "every gap over 200 ms: {:?}", a.missed);
    let mut a = app(&lines);
    hold(&mut a, KeyCode::Down, NONE, 4, FAST);
    assert!(a.missed.is_empty(), "Ctrl+D Up saves two: {:?}", a.missed);
    // Held on the last line but one: one press moved, and the rest are no presses spent.
    let mut a = app(&lines);
    a.line = 58;
    hold(&mut a, KeyCode::Down, NONE, 10, FAST);
    assert_eq!(a.line, 59);
    assert!(a.missed.is_empty(), "past the end: {:?}", a.missed);
    // `here` alone: Alt+Backspace takes all of `value_here`.
    let mut a = app("let name = value_here;\n");
    a.col = 21;
    press(&mut a, KeyCode::Enter, NONE);
    hold(&mut a, KeyCode::Backspace, NONE, 4, FAST);
    assert!(a.missed.is_empty(), "mid-word: {:?}", a.missed);
    // Onto the blank line: `}` in navigation, nothing in edit mode, where `}` types. A screen
    // of forty rows puts the paging keys far past it.
    let text = format!("{}\n{}", "x\n".repeat(8), "x\n".repeat(30));
    let mut a = app(&text);
    a.view_h = 40;
    hold(&mut a, KeyCode::Down, NONE, 8, FAST);
    assert_eq!(a.missed, missed(&[("}", 1)]));
    let mut a = app(&text);
    a.view_h = 40;
    press(&mut a, KeyCode::Enter, NONE);
    hold(&mut a, KeyCode::Down, NONE, 8, FAST);
    assert!(a.missed.is_empty(), "edit mode: {:?}", a.missed);
    // Nor is it tried there: a try would have typed it.
    assert_eq!(a.buf.lines.join("\n") + "\n", text);
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

/// `s` with the word under the cursor as the query, case ignored, then Enter: on the word's
/// declaration that was `d`, on a use `u` lists it was `u` and the arrows to its row.
#[test]
fn s_for_the_word_under_the_cursor_misses_d_or_u() {
    let (dir, mut a) = project_app("missed-s", &[("app.py", HANDLER)]);
    // Nine presses to the declaration; `d` is one.
    s(&mut a, &dir, "handler", 0, true);
    assert_eq!(at(&a), (dir.join("app.py"), 0));
    assert_eq!(a.missed, missed(&[("d", 1)]));
    // The same with the Enter ahead of the grep.
    s(&mut a, &dir, "handler", 0, false);
    assert_eq!(at(&a), (dir.join("app.py"), 0));
    assert_eq!(a.missed, missed(&[("d", 2)]));
    // Eleven presses to `handler(2)`, the third row of `u`: `u` Down Down Enter is four.
    s(&mut a, &dir, "Handler", 2, true);
    assert_eq!(at(&a), (dir.join("app.py"), 4));
    assert_eq!(a.missed, missed(&[("d", 2), ("u", 1)]));
    std::fs::remove_dir_all(dir).unwrap();
}

/// No miss for `s` with another query, for a hit `u` does not list (another case), or when
/// the key saves fewer than three presses.
#[test]
fn s_for_another_word_or_hit_counts_nothing() {
    let (dir, mut a) = project_app("missed-s-not", &[("app.py", HANDLER)]);
    s(&mut a, &dir, "handle", 0, true);
    assert_eq!(at(&a), (dir.join("app.py"), 0));
    s(&mut a, &dir, "handler", 3, true);
    assert_eq!(a.line_str(), "Handler = 3");
    assert!(a.missed.is_empty(), "{:?}", a.missed);
    // `s x Enter` to `def x`: three presses, `d` saves two.
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
    assert!(a.missed.is_empty(), "{:?}", a.missed);
    std::fs::remove_dir_all(dir).unwrap();
    std::fs::remove_dir_all(dir2).unwrap();
}

/// `D` with the word under the cursor as the query, then Enter: that was `d`. Another query,
/// or a word too short to save three presses, counts nothing.
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
    assert!(a.missed.is_empty(), "{:?}", a.missed);
    symbols(&mut a, 10, "x");
    assert_eq!(at(&a), (dir.join("app.py"), 7));
    assert!(a.missed.is_empty(), "{:?}", a.missed);
    symbols(&mut a, 7, "handler");
    assert_eq!(at(&a), (dir.join("app.py"), 0));
    assert_eq!(a.missed, missed(&[("d", 1)]));
    std::fs::remove_dir_all(dir).unwrap();
}

/// `o` to the file of a jump-history stop one to three steps away misses `[` or `]`.
#[test]
fn o_to_a_file_of_the_jump_history_misses_the_brackets() {
    let (dir, mut a) = project_app("missed-o", &[("a.rs", "x\n"), ("b.rs", "x\n")]);
    a.jump_to(&dir.join("a.rs"), 1);
    a.jump_to(&dir.join("b.rs"), 1);
    open_by_name(&mut a, "a.rs");
    assert_eq!(at(&a), (dir.join("a.rs"), 0));
    assert_eq!(a.missed, missed(&[("[", 1)]));
    // Back to the first stop, `b.rs` one step forward.
    press(&mut a, KeyCode::Char('['), NONE);
    press(&mut a, KeyCode::Char('['), NONE);
    open_by_name(&mut a, "b.rs");
    assert_eq!(at(&a), (dir.join("b.rs"), 0));
    assert_eq!(a.missed, missed(&[("[", 1), ("]", 1)]));
    std::fs::remove_dir_all(dir).unwrap();
}

/// No miss for `o` to a stop four steps back, for a query too short to save three presses, or
/// from edit mode, where `[` types.
#[test]
fn o_too_far_too_short_or_from_edit_mode_counts_nothing() {
    let names = ["alpha.rs", "bravo.rs", "cedar.rs", "dune.rs", "kiwi.rs"];
    let files: Vec<(&str, &str)> = names.iter().map(|n| (*n, "x\n")).collect();
    let (dir, mut a) = project_app("missed-o-not", &files);
    for name in names {
        a.jump_to(&dir.join(name), 1);
    }
    // Ten presses, and `[` four times would save six, but four steps is past the reach.
    open_by_name(&mut a, "alpha.rs");
    assert_eq!(at(&a), (dir.join("alpha.rs"), 0));
    // `kiwi` is one step back, and `o k` Enter is three presses.
    open_by_name(&mut a, "k");
    assert_eq!(at(&a), (dir.join("kiwi.rs"), 0));
    // `dune` is three steps back, nine presses away, but from edit mode.
    press(&mut a, KeyCode::Enter, NONE);
    press(&mut a, KeyCode::Char('e'), KeyModifiers::CONTROL);
    typed(&mut a, "dune.rs");
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Enter, NONE);
    assert_eq!(at(&a), (dir.join("dune.rs"), 0));
    assert!(a.missed.is_empty(), "{:?}", a.missed);
    std::fs::remove_dir_all(dir).unwrap();
}

/// Review: a run of arrows that ends in the next / previous hunk misses `c` / `C`, and `o` to
/// the next file of the review past the last hunk misses `c`.
#[test]
fn review_runs_into_a_hunk_or_o_to_the_next_file_miss_c() {
    let (dir, mut a) = review_app("missed-review");
    // `new` is open on its only hunk: `c` goes on to `tail`.
    open_by_name(&mut a, "tail");
    assert_eq!(a.rel_path(), "tail");
    assert_eq!(a.missed, missed(&[("c", 1)]));
    // src/a.rs: hunks on `B` and `F`, lines 2 and 6.
    a.jump_to(&dir.join("src/a.rs"), 2);
    hold(&mut a, KeyCode::Down, NONE, 4, FAST);
    assert_eq!(a.line_str(), "F");
    hold(&mut a, KeyCode::Up, NONE, 4, FAST);
    assert_eq!(a.line_str(), "B");
    assert_eq!(a.missed, missed(&[("c", 2), ("C", 1)]));
    std::fs::remove_dir_all(dir).unwrap();
}

/// No `c` for a run read line by line, for one that passes the hunk, in edit mode, or for `o`
/// to the next file while a hunk is still below.
#[test]
fn review_runs_that_are_slow_pass_the_hunk_or_type_count_no_c() {
    let (dir, mut a) = review_app("missed-review-not");
    a.jump_to(&dir.join("src/a.rs"), 2);
    hold(&mut a, KeyCode::Down, NONE, 4, SLOW);
    assert_eq!(a.line_str(), "F");
    a.jump_to(&dir.join("src/a.rs"), 1);
    hold(&mut a, KeyCode::Down, NONE, 4, FAST);
    assert_eq!(a.line_str(), "e");
    assert!(a.missed.is_empty(), "{:?}", a.missed);
    a.jump_to(&dir.join("src/a.rs"), 2);
    press(&mut a, KeyCode::Enter, NONE);
    hold(&mut a, KeyCode::Down, NONE, 4, FAST);
    assert_eq!(a.line_str(), "F");
    assert!(!a.missed.contains_key("c"), "edit mode: {:?}", a.missed);
    a.missed.clear();
    // `crlf.txt` comes after `src/a.rs`, but `c` still has `B` and `F` to go to.
    a.jump_to(&dir.join("src/a.rs"), 1);
    open_by_name(&mut a, "crlf.txt");
    assert_eq!(a.rel_path(), "crlf.txt");
    assert!(a.missed.is_empty(), "{:?}", a.missed);
    std::fs::remove_dir_all(dir).unwrap();
}
