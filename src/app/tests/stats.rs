//! Key stats: the action each press counts toward (#207).

use super::*;
use crate::stats;

/// A side of a `KEYS` row read back into the key that presses it: `Ctrl+Shift+Left`, `PgUp`,
/// `Shift+F12`, `N`. A name no key has panics, so a row the split cannot read fails.
fn read(name: &str) -> KeyEvent {
    let (mods, key) = name.rsplit_once('+').unwrap_or(("", name));
    let cannot = || -> ! { panic!("cannot read the key {name:?}") };
    let mut m = KeyModifiers::NONE;
    for part in mods.split('+').filter(|p| !p.is_empty()) {
        m |= match part {
            "Ctrl" => KeyModifiers::CONTROL,
            "Alt" => KeyModifiers::ALT,
            "Shift" => KeyModifiers::SHIFT,
            _ => cannot(),
        };
    }
    let code = match key {
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Enter" => KeyCode::Enter,
        "Esc" => KeyCode::Esc,
        "Tab" => KeyCode::Tab,
        "Backspace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PgUp" => KeyCode::PageUp,
        "PgDn" => KeyCode::PageDown,
        _ if key.len() > 1 && key.starts_with('F') => {
            KeyCode::F(key[1..].parse().unwrap_or_else(|_| cannot()))
        }
        // A terminal sends Ctrl+E as Ctrl and a lowercase `e`.
        _ if key.chars().count() == 1 && m.contains(KeyModifiers::CONTROL) => {
            KeyCode::Char(key.to_ascii_lowercase().chars().next().unwrap())
        }
        _ if key.chars().count() == 1 => KeyCode::Char(key.chars().next().unwrap()),
        _ => cannot(),
    };
    KeyEvent::new(code, m)
}

/// Every action of `KEYS`, pressed where it is pressed (the tree, a picker, the help, edit
/// mode), counts once under its own name; an alias counts toward its primary, an arrow toward
/// `Arrows`.
#[test]
fn every_action_counts_where_it_is_pressed() {
    let mut presses: Vec<(&str, &str)> = stats::ACTIONS
        .iter()
        .map(|a| (a.name.as_str(), a.name.as_str()))
        .filter(|(name, _)| *name != "Arrows")
        .collect();
    presses.extend([
        ("Ctrl+E", "o"),
        ("Ctrl+F", "/"),
        ("F12", "d"),
        ("Shift+F12", "u"),
        ("Ctrl+G", ":"),
        ("Up", "Arrows"),
        ("Down", "Arrows"),
        ("Left", "Arrows"),
        ("Right", "Arrows"),
    ]);
    for (key, action) in presses {
        let (scope, name) = key.split_once(": ").unwrap_or(("", key));
        let mut a = app("foo bar\n");
        match scope {
            "" => {}
            "Tree" => a.focus = Focus::Tree,
            "Picker" => _ = press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE),
            "Help" => _ = press(&mut a, KeyCode::Char('?'), KeyModifiers::NONE),
            "Edit" => _ = press(&mut a, KeyCode::Enter, KeyModifiers::NONE),
            _ => panic!("no scope {scope:?} for {key:?}"),
        }
        a.pressed.clear();
        a.key(read(name));
        assert_eq!(a.pressed, HashMap::from([(action, 1)]), "{key}");
    }
}

/// Typing is no action: letters, Enter and Backspace in edit mode, in the find, go-to and
/// new-file prompts and in a picker's filter count nothing. The keys that open and close them
/// do.
#[test]
fn typing_counts_nothing() {
    let mut a = app("foo\n");
    let mut typing = |open: &str, text: &str| {
        a.key(read(open));
        for c in text.chars() {
            let code = if c == '\n' {
                KeyCode::Enter
            } else {
                KeyCode::Char(c)
            };
            press(&mut a, code, KeyModifiers::NONE);
        }
        press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    };
    typing("Enter", "dq[\n");
    typing("/", "fo");
    typing(":", "1");
    typing("Ctrl+N", "n.rs");
    typing("o", "fo");
    assert_eq!(
        a.pressed,
        HashMap::from([
            ("Enter", 1),
            ("/", 1),
            (":", 1),
            ("Ctrl+N", 1),
            ("o", 1),
            ("Esc", 4),
            ("Picker: Esc", 1),
        ])
    );
}

/// `--tutor` is no real work: its presses count nothing and miss nothing, so there is nothing
/// to write on exit.
#[test]
fn a_press_under_the_tutor_writes_nothing() {
    let mut a = app("foo bar\n");
    a.tutor = Some(Tutor {
        step: 0,
        dir: PathBuf::from("/nonexistent"),
        drill: None,
    });
    // Right held to the end of the line misses a key in real work.
    let keys = ["d", "]", "[", "v", "Ctrl+D", "Down", "Enter", "Esc"]
        .into_iter()
        .chain(["Right"; 7])
        .chain(["Esc"]);
    let mut work = app("foo bar\n");
    for key in keys.clone() {
        work.key(read(key));
    }
    assert!(!work.missed.is_empty());
    for key in keys {
        a.key(read(key));
    }
    assert!(a.pressed.is_empty(), "{:?}", a.pressed);
    assert!(a.missed.is_empty(), "{:?}", a.missed);
    let file = std::env::temp_dir().join(format!("merl-keys-tutor-{}.tsv", std::process::id()));
    let _ = std::fs::remove_file(&file);
    stats::add(&file, stats::today(), &a.pressed, &a.missed).unwrap();
    assert!(!file.exists());
}

/// Review stats (#242), one session: the time between presses on the review's files and
/// elsewhere, five minutes at most per gap, the reading before `q` included; the six stops `c`
/// can make, the top of the deleted `gone` one of them; `o` out of the review and `[` back, one
/// excursion of one jump; the hunk the review opened on and the one `c` stopped on; the two
/// files `c` marked viewed, and `c` reaching the last hunk.
#[test]
fn a_review_session_counts_its_presses() {
    let (dir, mut a) = review_app("reviewstats");
    assert_eq!(a.review_row(), None, "a session without a press is none");
    let t0 = Instant::now();
    let mut secs = 0;
    let mut key = |a: &mut App, name: &str, after: u64| {
        secs += after;
        a.key_at(read(name), t0 + Duration::from_secs(secs))
    };
    key(&mut a, "c", 10);
    assert_eq!(at(&a), (dir.join("tail"), 0));
    key(&mut a, "o", 20);
    for c in ["k", "e", "e", "p"] {
        key(&mut a, c, 1);
    }
    a.picker.as_mut().unwrap().settle();
    key(&mut a, "Enter", 1);
    assert_eq!(at(&a), (dir.join("src/keep.rs"), 0));
    key(&mut a, "Down", 4 * 3600);
    key(&mut a, "[", 30);
    assert_eq!(at(&a), (dir.join("tail"), 0));
    key(&mut a, "c", 5);
    assert_eq!(a.message, "last hunk of the review");
    assert!(key(&mut a, "q", 2));
    let (repo, branch, columns) = a.review_row().unwrap();
    assert_eq!((repo, branch.as_str()), (a.root_name(), "feature"));
    // files, hunks, added, deleted; seconds on the review and elsewhere; excursions, their
    // jumps, `[`; hunks stopped on, files viewed, the last hunk reached.
    assert_eq!(columns, "5\t6\t4\t7\t42\t330\t1\t1\t1\t2\t2\t1");
    let _ = std::fs::remove_dir_all(dir);
}

/// The key that quits is no press: a review opened, forgotten and closed with `q` hours later
/// has no line, and takes no round.
#[test]
fn a_review_closed_with_q_alone_has_no_line() {
    let (dir, mut a) = review_app("reviewstats-q");
    assert!(a.key_at(read("q"), Instant::now() + Duration::from_secs(4 * 3600)));
    assert_eq!(a.review_row(), None);
    let _ = std::fs::remove_dir_all(dir);
}

/// An Enter in `s` that came before the hits: the jump out of the review lands after the key,
/// and is an excursion all the same.
#[test]
fn a_search_that_answers_after_enter_is_an_excursion() {
    let (dir, mut a) = review_app("reviewstats-s");
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "k2");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert!(a.picker.is_some(), "nothing to jump to yet");
    a.settle_search();
    assert_eq!(at(&a), (dir.join("src/keep.rs"), 1));
    let cols = columns(&mut a);
    assert_eq!((cols[6], cols[7]), (1, 1), "{cols:?}");
    let _ = std::fs::remove_dir_all(dir);
}

/// `--tutor` and `--drill` record no review: their presses count nothing, so there is no line.
#[test]
fn a_review_under_the_tutor_has_no_line() {
    let (dir, mut a) = review_app("reviewstats-tutor");
    a.tutor = Some(Tutor {
        step: 0,
        dir: PathBuf::from("/nonexistent"),
        drill: None,
    });
    for key in ["c", "c", "[", "q"] {
        a.key(read(key));
    }
    assert_eq!(a.review_row(), None);
    let _ = std::fs::remove_dir_all(dir);
}

/// The session's columns after the round, as `review_row` has them.
fn columns(a: &mut App) -> Vec<usize> {
    let (_, _, columns) = a.review_row().expect("a line");
    columns.split('\t').map(|n| n.parse().unwrap()).collect()
}

/// A stop is a hunk, however the walk reached it: one only `C` got to counts as one `c` got to,
/// and walking every hunk reads as many stops as hunks. It is the hunk, not the line: lines an
/// agent writes above the cursor move it, and a `c` that stays put there is no new stop.
#[test]
fn a_stop_is_a_hunk_reached_by_c_or_c_back() {
    let (dir, mut a) = review_app("reviewstats-stops");
    // Opened on the hunk of `new`: one stop. Into `src/a.rs` by `o`, down past its first hunk,
    // and `C` back onto it.
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
    typed(&mut a, "a.rs");
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    for _ in 0..3 {
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    }
    press(&mut a, KeyCode::Char('C'), KeyModifiers::SHIFT);
    assert_eq!(at(&a), (dir.join("src/a.rs"), 1));
    assert_eq!(columns(&mut a)[9], 2);
    // `c` on to the end of the review: the second hunk of `src/a.rs`, `crlf.txt`, the top of
    // the deleted `gone`, `new` again, `tail`.
    for _ in 0..5 {
        press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    }
    assert_eq!(at(&a), (dir.join("tail"), 0));
    assert_eq!(columns(&mut a)[9], 6);
    // An agent writes two lines at the top of `tail`: its hunk moves down under the cursor, and
    // `c` stays on it.
    std::fs::write(dir.join("tail"), "x\ny\nt1\n").unwrap();
    a.reload(false);
    assert_eq!(at(&a), (dir.join("tail"), 2));
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(
        (at(&a), a.message.as_str()),
        ((dir.join("tail"), 2), "last hunk of the review")
    );
    let cols = columns(&mut a);
    // Stops, hunks, the last hunk reached.
    assert_eq!((cols[9], cols[1], cols[11]), (6, 6, 1));
    let _ = std::fs::remove_dir_all(dir);
}

/// A file opened from the review panel lands on its first hunk, and that hunk is a stop as if
/// `c` had gone there: a walk that starts from the panel reads every hunk too.
#[test]
fn a_hunk_the_panel_opens_on_is_a_stop() {
    let (dir, mut a) = review_app("reviewstats-panel");
    a.reveal(&dir.join("src/a.rs"));
    a.focus = Focus::Tree;
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("src/a.rs"), 1));
    for _ in 0..5 {
        press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    }
    assert_eq!(at(&a), (dir.join("tail"), 0));
    let cols = columns(&mut a);
    assert_eq!((cols[9], cols[1]), (6, 6), "{cols:?}");
    let _ = std::fs::remove_dir_all(dir);
}

/// Stops are numbered from 1, as the status bar numbers hunks: a hunk an agent adds after the
/// review opened pushes the last one past what the file had then, and that stop does not count.
#[test]
fn a_stop_past_the_hunks_the_review_opened_with_does_not_count() {
    let (dir, mut a) = review_app("reviewstats-numbered");
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
    typed(&mut a, "a.rs");
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    // Two hunks when the review opened (lines 2 and 6), counted in the second after it did; the
    // agent's edit comes after, and there are three now.
    assert_eq!(columns(&mut a)[1], 6);
    std::fs::write(dir.join("src/a.rs"), "a\nB\nc\nD\ne\nF\n").unwrap();
    a.reload(false);
    assert_eq!(a.diff.hunks, [1, 3, 5]);
    for _ in 0..3 {
        press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    }
    assert_eq!(at(&a), (dir.join("src/a.rs"), 5));
    // `new`, where the review opened, and the first two hunks of `src/a.rs`.
    assert_eq!(columns(&mut a)[9], 3);
    let _ = std::fs::remove_dir_all(dir);
}

/// The hunks are the stops `c` can make: an untracked file with lines is one, an empty one
/// none.
#[test]
fn an_untracked_file_is_one_hunk_and_an_empty_one_none() {
    let (dir, mut a) = review_app("reviewstats-untracked");
    std::fs::write(dir.join("fresh.txt"), "1\n2\n3\n").unwrap();
    std::fs::write(dir.join("empty.txt"), "").unwrap();
    a.start_review(git::Review::open(&dir, None, None).unwrap());
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(columns(&mut a)[..2], [7, 7]);
    let _ = std::fs::remove_dir_all(dir);
}

/// A submodule is listed but has no line to read: `c` walks past it, and it counts no hunk.
#[test]
fn a_submodule_counts_no_hunk() {
    let (dir, mut a) = review_app("reviewstats-sub");
    let git = |at: &Path, args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(at)
            .args(["-c", "user.email=t@t", "-c", "user.name=t"])
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    let sub = dir.join("sub");
    std::fs::create_dir(&sub).unwrap();
    git(&sub, &["init", "-q"]);
    git(&sub, &["commit", "-q", "--allow-empty", "-m", "sub"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "submodule"]);
    a.start_review(git::Review::open(&dir, None, None).unwrap());
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(columns(&mut a)[..2], [6, 6]);
    let _ = std::fs::remove_dir_all(dir);
}

/// A `git diff` that fails leaves the session out of the stats, not in them with too few hunks.
#[test]
fn a_failed_diff_writes_no_line() {
    let (dir, mut a) = review_app("reviewstats-failed");
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert!(a.review_row().is_some());
    let mut broken = a.review.clone().unwrap();
    broken.merge_base = "0".repeat(40);
    a.start_review(broken);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(a.review_row(), None);
    let _ = std::fs::remove_dir_all(dir);
}

/// The session is filed under the branch the review opened on, with that review's size: a
/// `git switch` during it moves neither. A detached HEAD is its short commit, so detached
/// reviews do not share rounds.
#[test]
fn a_session_keeps_the_review_it_opened_on() {
    let (dir, mut a) = review_app("reviewstats-branch");
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    };
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    // The stops are counted in the second after the review opens; the switch comes after.
    assert_eq!(columns(&mut a)[..2], [5, 6]);
    git(&["switch", "-q", "main"]);
    let fresh = a.review.as_ref().unwrap().refresh(&dir).unwrap();
    a.review_refreshed(fresh);
    assert_eq!(a.review.as_ref().unwrap().branch, "main");
    let (_, branch, columns) = a.review_row().unwrap();
    assert_eq!((branch.as_str(), &columns[..4]), ("feature", "5\t6\t"));
    git(&["switch", "-q", "--detach", "feature"]);
    a.start_review(git::Review::open(&dir, None, None).unwrap());
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    let short = git(&["rev-parse", "--short", "HEAD"]);
    assert_eq!(a.review_row().unwrap().1, short);
    let _ = std::fs::remove_dir_all(dir);
}
