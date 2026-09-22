//! PROTOTYPE for #190 — throwaway, lives on the `prototype/task-pool` branch only.
//!
//! The task pool the tutor and the drill would share: four tasks (a jump, a picker key, an
//! edit-mode key, a selection key), each with its own start, two texts and the keys that solve
//! it. Nothing here is built into merl; run
//!
//!     cargo test task_pool -- --nocapture
//!
//! to drive every task from cold, after every other task, and print the state around each.

#![allow(dead_code)]

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::app::PickerKind;
use crate::buffer::Buffer;

/// 1-based line of `def load_config` in `config.py`, where `d` on the call lands.
const LOAD_CONFIG_DEF: usize = 18;

/// One task: trains one action, starts from its own state, checks the effect.
pub struct Task {
    /// The action it trains, as #189 names it: one side of a `KEYS` row, aliases folded in.
    pub key: &'static str,
    pub title: &'static str,
    /// `--tutor`: names the key.
    pub tutor: &'static str,
    /// The drill: what to do, never the key.
    pub drill: &'static str,
    /// Runs on a fresh App (see [`fresh`]), with the verbs a learner has: open a file at a word,
    /// press keys. No field of `App` is set by hand beyond the cursor column.
    pub setup: fn(&mut App),
    /// The shortest keys that do it, in [`keys`] notation. The test presses them; the drill can
    /// show them after a miss, and their count is a par for "slow".
    pub answer: &'static str,
    pub done: fn(&App) -> bool,
}

pub const POOL: &[Task] = &[
    Task {
        key: "d",
        title: "Go to definition",
        tutor: "With the cursor on `load_config`, press `d` (or F12) to jump to its definition.",
        drill: "Jump to where `load_config` is defined.",
        setup: |a| open_at(a, "store.py", CALL_LINE, "load_config"),
        answer: "d",
        done: |a| at(a, "config.py") && a.line + 1 == LOAD_CONFIG_DEF,
    },
    Task {
        key: "Picker: PgDn",
        title: "A page down a list",
        tutor: "The project symbols are open. PgDn moves the selection a page down the list.",
        drill: "Move the selection a whole page down this list, in one key.",
        setup: |a| {
            open_at(a, "store.py", 1, "");
            keys(a, "D");
        },
        answer: "<PgDn>",
        done: |a| {
            a.mode == Mode::Picker(PickerKind::Symbols)
                && a.picker.as_ref().is_some_and(|p| p.selected >= 10)
        },
    },
    Task {
        key: "Edit: Alt+Backspace",
        title: "Delete a word",
        tutor: "You are editing, right after `load_config`. Alt+Backspace deletes the word before \
                the cursor.",
        drill: "You are typing. Delete `load_config` before the cursor, all of it in one key.",
        setup: |a| {
            open_at(a, "store.py", CALL_LINE, "load_config");
            keys(a, "<A-Right><Enter>");
        },
        answer: "<A-BS>",
        done: |a| {
            a.mode == Mode::Edit && a.buf.lines[CALL_LINE - 1].trim() == "self.config = config or ()"
        },
    },
    Task {
        key: "Alt+Shift+Right",
        title: "Select by words",
        tutor: "Alt+Shift+Right extends the selection by a word. Press it three times to select \
                `config or load_config`.",
        drill: "Select `config or load_config`, a word at a time.",
        setup: |a| open_at(a, "store.py", CALL_LINE, "config or"),
        answer: "<A-S-Right><A-S-Right><A-S-Right>",
        done: |a| selected(a).as_deref() == Some("config or load_config"),
    },
    Task {
        key: "[",
        title: "Back",
        tutor: "`d` just jumped here from store.py. `[` goes back in the jump history.",
        drill: "Go back to where you were before this jump.",
        setup: |a| {
            open_at(a, "store.py", CALL_LINE, "load_config");
            keys(a, "d");
        },
        answer: "[",
        done: |a| at(a, "store.py") && a.line + 1 == CALL_LINE,
    },
    Task {
        key: "}",
        title: "Next paragraph",
        tutor: "`}` jumps to the next blank line: the end of `remove`.",
        drill: "Jump to the blank line after this function.",
        setup: |a| open_at(a, "store.py", 40, ""),
        answer: "}",
        done: |a| at(a, "store.py") && a.line + 1 == PARA_DOWN_LINE,
    },
    Task {
        key: "u",
        title: "Usages",
        tutor: "With the cursor on `load_config`, `u` lists every use of it.",
        drill: "List every place `load_config` is used.",
        setup: |a| open_at(a, "store.py", CALL_LINE, "load_config"),
        answer: "u",
        done: |a| a.mode == Mode::Picker(PickerKind::Usages),
    },
];

/// Puts `a` back where a fresh `--tutor` starts: the sample project unpacked anew (edits, new
/// files and all), nothing open, no history, no undo, no find. What `main` set is carried over.
///
/// A new `App` rather than a field-by-field reset: a field someone forgets to carry over shows
/// (the theme falls back), a field someone forgets to reset leaks from one task into the next.
pub fn fresh(a: &mut App) {
    let dir = extract().unwrap();
    let (tree, files) = crate::tree::build(&dir);
    let mut new = App::new(dir, tree, files, Buffer::empty(), None);
    new.show_tree = false;
    new.focus = Focus::Code;
    new.autosave = a.autosave;
    new.theme = std::mem::take(&mut a.theme);
    new.config = a.config.take();
    new.no_watch = a.no_watch;
    (new.view_w, new.view_h) = (a.view_w, a.view_h);
    new.tutor = a.tutor.take();
    *a = new;
}

/// Opens `file` at 1-based `line`, the cursor on the first `word` of it ("" is the line start).
fn open_at(a: &mut App, file: &str, line: usize, word: &str) {
    let path = a.root.join(file);
    a.jump_to(&path, line);
    a.col = a.line_str().find(word).unwrap();
    a.want_x = a.col;
}

/// Presses `notation`: characters as typed, `<Enter>`, `<A-Right>`, `<C-z>`, `<A-S-Right>`.
/// The picker's matcher runs to completion before every key, as the event loop's tick does.
pub fn keys(a: &mut App, notation: &str) {
    let mut rest = notation;
    while let Some(c) = rest.chars().next() {
        let (code, mods, len) = match rest.find('>').filter(|_| c == '<') {
            Some(end) => {
                let (code, mods) = chord(&rest[1..end]);
                (code, mods, end + 1)
            }
            None => (KeyCode::Char(c), KeyModifiers::NONE, c.len_utf8()),
        };
        rest = &rest[len..];
        if let Some(p) = &mut a.picker {
            p.settle();
        }
        a.key(KeyEvent::new(code, mods));
    }
}

pub fn chord(name: &str) -> (KeyCode, KeyModifiers) {
    let mut mods = KeyModifiers::NONE;
    let mut name = name;
    while let Some((m, rest)) = name.split_once('-').filter(|(m, _)| m.len() == 1) {
        mods |= match m {
            "C" => KeyModifiers::CONTROL,
            "A" => KeyModifiers::ALT,
            "S" => KeyModifiers::SHIFT,
            _ => panic!("modifier {m}"),
        };
        name = rest;
    }
    let code = match name {
        "Enter" => KeyCode::Enter,
        "Esc" => KeyCode::Esc,
        "Tab" => KeyCode::Tab,
        "BS" => KeyCode::Backspace,
        "Del" => KeyCode::Delete,
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PgUp" => KeyCode::PageUp,
        "PgDn" => KeyCode::PageDown,
        c if c.chars().count() == 1 => KeyCode::Char(c.chars().next().unwrap()),
        _ => panic!("key {name}"),
    };
    (code, mods)
}

/// The selected text, when the selection sits on one line.
fn selected(a: &App) -> Option<String> {
    let ((l1, c1), (l2, c2)) = a.selection()?;
    (l1 == l2).then(|| a.buf.lines[l1][c1..c2].to_string())
}

/// One line of state, printed around every task.
fn state(a: &App) -> String {
    let picker = a
        .picker
        .as_ref()
        .map_or(String::new(), |p| format!(" picker row {}", p.selected));
    let sel = selected(a).map_or(String::new(), |s| format!(" sel {s:?}"));
    format!(
        "{}:{}:{} {:?}{picker}{sel}",
        a.rel_path(),
        a.line + 1,
        a.col + 1,
        a.mode
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let dir = extract().unwrap();
        let (tree, files) = crate::tree::build(&dir);
        let mut a = App::new(dir, tree, files, Buffer::empty(), None);
        (a.view_w, a.view_h) = (80, 24);
        a
    }

    /// Every task after every other task, on one App: whatever the previous one left behind
    /// (an edit, an open picker, a selection, a history) must not decide the next.
    #[test]
    fn task_pool_every_task_runs_after_every_other() {
        let mut a = app();
        let run = |a: &mut App, t: &Task| {
            let clock = std::time::Instant::now();
            fresh(a);
            let reset = clock.elapsed();
            (t.setup)(a);
            let before = state(a);
            assert!(!(t.done)(a), "{}: done before a key: {before}", t.title);
            keys(a, t.answer);
            let after = state(a);
            assert!((t.done)(a), "{}: {before} --{}--> {after}", t.title, t.answer);
            eprintln!("{:<20} reset {reset:?}, task {:?}", t.key, clock.elapsed() - reset);
            (before, after)
        };
        for first in POOL {
            for then in POOL {
                run(&mut a, first);
                run(&mut a, then);
            }
        }
        for t in POOL {
            let (before, after) = run(&mut a, t);
            eprintln!("{:<20} {before:<34} --{}--> {after}", t.key, t.answer);
            eprintln!("{:<20} drill: {}\n", "", t.drill);
        }
    }

    /// The drill text never names the key; the tutor text always does.
    #[test]
    fn task_pool_drill_text_never_names_the_key() {
        // A key is named in backticks or as a word, as `every_key_is_taught_or_skipped_on_purpose`
        // reads it.
        let names = |text: &'static str| -> Vec<&'static str> {
            let words = text.split_whitespace();
            let words = words.map(|w| w.trim_end_matches(['.', ',', ':', ';']));
            text.split('`').skip(1).step_by(2).chain(words).collect()
        };
        for t in POOL {
            let name = t.key.rsplit(": ").next().unwrap();
            assert!(names(t.tutor).contains(&name), "{}: tutor text lacks {name}", t.title);
            assert!(!names(t.drill).contains(&name), "{}: drill text names {name}", t.title);
        }
    }
}
