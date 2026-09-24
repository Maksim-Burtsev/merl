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
