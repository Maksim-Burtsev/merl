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
/// elsewhere, five minutes at most per gap; `o` out of the review and `[` back, one excursion of
/// one jump; the hunk the review opened on and the one `c` stopped on; the two files `c` marked
/// viewed, and `c` reaching the last hunk.
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
    let (_, _, columns) = a.review_row().unwrap();
    let cols: Vec<&str> = columns.split('\t').collect();
    assert_eq!((cols[6], cols[7]), ("1", "1"), "{columns}");
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
