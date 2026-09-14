//! `merl --tutor`: a vimtutor-style walkthrough of the navigation keys.
//!
//! A small Python project is embedded in the binary, unpacked into a temporary directory and
//! opened like any other project. Lessons advance when a predicate over [`App`] becomes true —
//! what the key did, not that a key was pressed — so there is no way to fake progress.

use std::path::PathBuf;

use anyhow::Result;

use crate::app::{App, Focus, Mode};

/// One step of the tutorial: what to do, and how to tell that it happened.
pub struct Lesson {
    pub title: &'static str,
    pub text: &'static str,
    pub done: fn(&App) -> bool,
}

pub struct Tutor {
    /// Index into [`LESSONS`]; `LESSONS.len()` means the tutorial is over.
    pub step: usize,
    /// The unpacked sample project, removed when merl exits.
    pub dir: PathBuf,
}

/// The sample project, as it sits in `tutor/notes/`.
pub const FILES: &[(&str, &str)] = &[
    ("cli.py", include_str!("../tutor/notes/cli.py")),
    ("config.py", include_str!("../tutor/notes/config.py")),
    ("models.py", include_str!("../tutor/notes/models.py")),
    ("store.py", include_str!("../tutor/notes/store.py")),
    (
        "tests/test_store.py",
        include_str!("../tutor/notes/tests/test_store.py"),
    ),
];

/// 1-based lines in `store.py` two lessons land on; the test checks them against the file.
const CALL_LINE: usize = 13;
const CLASS_LINE: usize = 9;
/// The blank lines around `remove` in `store.py`, where the paragraph lessons land.
const PARA_DOWN_LINE: usize = 47;
const PARA_UP_LINE: usize = 38;

fn at(app: &App, file: &str) -> bool {
    app.rel_path() == file
}

/// The open file no longer reads like the sample it was unpacked from.
fn edited(app: &App) -> bool {
    FILES
        .iter()
        .find(|(name, _)| at(app, name))
        .is_some_and(|(_, text)| text.lines().ne(app.buf.lines.iter().map(String::as_str)))
}

/// Shown once every lesson is done.
pub const DONE: &str = "That is all of merl. Everything else is a VS Code habit. `q` quits.";

pub const LESSONS: &[Lesson] = &[
    Lesson {
        title: "Open a file",
        text: "Press `o` (or Ctrl+E), type `store`, and press Enter to open store.py.",
        done: |a| at(a, "store.py"),
    },
    Lesson {
        title: "Find in the file",
        text: "Press `/` (or Ctrl+F), type `load_config`, Enter. The search runs as you type.",
        done: |a| {
            a.mode == Mode::Normal
                && a.find_re
                    .as_ref()
                    .is_some_and(|r| r.as_str() == "load_config")
        },
    },
    Lesson {
        title: "Next match",
        text: "`n` goes to the next match, `N` to the previous one. Press `n`: \
               the call inside NoteStore.",
        done: |a| at(a, "store.py") && a.line + 1 == CALL_LINE,
    },
    Lesson {
        title: "Go to definition",
        text: "With the cursor on `load_config`, press `d` (or F12) to jump to its definition.",
        done: |a| at(a, "config.py"),
    },
    Lesson {
        title: "Back",
        text: "`[` goes back in the jump history — to store.py, where you came from.",
        done: |a| at(a, "store.py"),
    },
    Lesson {
        title: "Forward",
        text: "`]` goes forward again, back into config.py.",
        done: |a| at(a, "config.py"),
    },
    Lesson {
        title: "Usages",
        text: "Shift+Right moves a word: press it twice to reach `load_config`, then `u` \
               (or Shift+F12). Pick the hit in cli.py with Down and Enter.",
        done: |a| at(a, "cli.py"),
    },
    Lesson {
        title: "Search the project",
        text: "`s` searches every file. Type `TODO`, Enter, then Enter on the result.",
        done: |a| at(a, "models.py"),
    },
    Lesson {
        title: "Project symbols",
        text: "`D` lists every class and function. Type `NoteStore` and press Enter.",
        done: |a| at(a, "store.py") && a.line + 1 == CLASS_LINE,
    },
    Lesson {
        title: "Go to line",
        text: "`:` (or Ctrl+G), then a number: type `42` and press Enter.",
        done: |a| a.line + 1 == 42,
    },
    Lesson {
        title: "Next paragraph",
        text: "`}` jumps to the next blank line. In code that is the end of the current \
               function: press it once.",
        done: |a| at(a, "store.py") && a.line + 1 == PARA_DOWN_LINE,
    },
    Lesson {
        title: "Previous paragraph",
        text: "`{` jumps to the previous blank line. Press it twice to reach the line above `remove`.",
        done: |a| at(a, "store.py") && a.line + 1 == PARA_UP_LINE,
    },
    Lesson {
        title: "File tree",
        text: "`t` shows and hides the file tree.",
        done: |a| a.show_tree,
    },
    Lesson {
        title: "Focus the tree",
        text: "Tab switches the focus between the code and the tree. Press it once.",
        done: |a| a.focus == Focus::Tree,
    },
    Lesson {
        title: "Open from the tree",
        text: "Up four times to `tests`, Right to expand it, Down to test_store.py, Enter.",
        done: |a| {
            at(
                a,
                &format!("tests{}test_store.py", std::path::MAIN_SEPARATOR),
            )
        },
    },
    Lesson {
        title: "Hide the tree",
        text: "Opening a file gave the focus back to the code. Press `t` to hide the tree again.",
        done: |a| !a.show_tree,
    },
    Lesson {
        title: "Edit",
        text: "Enter starts editing at the cursor: type `# hi` and press Esc. merl saves a \
               second after you stop typing; Ctrl+S saves now, Ctrl+R reloads the file from disk.",
        done: |a| a.mode == Mode::Normal && edited(a),
    },
    Lesson {
        title: "Undo",
        text: "Ctrl+Z takes an edit back, Ctrl+Y brings it again. Press Ctrl+Z once: the \
               file is as it was, and a second later so is the disk.",
        done: |a| a.mode == Mode::Normal && !edited(a),
    },
    Lesson {
        title: "Help",
        text: "`?` lists every key merl knows.",
        done: |a| a.mode == Mode::Help,
    },
    Lesson {
        title: "Close it",
        text: "Esc closes any overlay, and clears the selection and the find pattern.",
        done: |a| a.mode == Mode::Normal,
    },
];

/// Called after every key: advances at most one step, so a state that happens to satisfy two
/// predicates cannot skip a lesson.
pub fn check(app: &mut App) {
    let Some(tutor) = &app.tutor else { return };
    let step = tutor.step;
    let Some(lesson) = LESSONS.get(step) else {
        return;
    };
    if !(lesson.done)(app) {
        return;
    }
    app.message = format!("\u{2713} {}", lesson.title);
    if let Some(tutor) = &mut app.tutor {
        tutor.step = step + 1;
    }
}

/// Unpacks the sample project into a fresh temporary directory.
pub fn extract() -> Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("merl-tutor-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("tests"))?;
    for (name, text) in FILES {
        std::fs::write(dir.join(name), text)?;
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;
    use crate::buffer::Buffer;

    /// One key, with the picker's matcher run to completion first — the event loop ticks it
    /// between keys, so a test that does not would navigate an empty result list.
    fn press(a: &mut App, code: KeyCode) {
        if let Some(p) = &mut a.picker {
            p.settle();
        }
        a.key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn typed(a: &mut App, text: &str) {
        for c in text.chars() {
            press(a, KeyCode::Char(c));
        }
    }

    fn shift(a: &mut App, code: KeyCode) {
        a.key(KeyEvent::new(code, KeyModifiers::SHIFT));
    }

    #[test]
    fn the_sample_project_has_exactly_one_todo() {
        let todos: usize = FILES.iter().map(|(_, t)| t.matches("TODO").count()).sum();
        assert_eq!(todos, 1, "step 8 searches for the single TODO");
        let store = FILES.iter().find(|(n, _)| *n == "store.py").unwrap().1;
        let line = |n: usize| store.lines().nth(n - 1).unwrap();
        assert!(line(CALL_LINE).contains("load_config()"), "{CALL_LINE}");
        assert!(
            line(CLASS_LINE).starts_with("class NoteStore"),
            "{CLASS_LINE}"
        );
        assert!(store.lines().count() >= 42, "step 10 goes to line 42");
        for n in [PARA_DOWN_LINE, PARA_UP_LINE] {
            assert!(line(n).trim().is_empty(), "line {n} must be blank");
        }
        assert!(
            line(PARA_UP_LINE + 1).contains("def remove"),
            "{PARA_UP_LINE}"
        );
    }

    /// Keys in `KEYS` that the tutorial deliberately skips: VS Code habits and the keys inside
    /// the tree and the pickers. A new row in `KEYS` fails this test until it is either taught by
    /// a lesson or listed here on purpose.
    const NOT_TAUGHT: &[&str] = &[
        "Arrows",
        "Shift+Up / Shift+Down",
        "Alt+Shift+Left / Right",
        "Ctrl+Shift+Left / Right",
        "Ctrl+D / Ctrl+U",
        "PgUp / PgDn",
        "Home / End",
        "Ctrl+Home / Ctrl+End",
        "Tree: Up / Down",
        "Tree: Enter",
        "Tree: Left / Right",
        "Picker: Up / Down, Ctrl+P / Ctrl+N",
        "Picker: Enter",
        "Picker: Esc",
        "Picker: PgUp / PgDn",
        "Help: Up / Down",
    ];

    #[test]
    fn every_key_is_taught_or_skipped_on_purpose() {
        let text: String = LESSONS
            .iter()
            .map(|l| l.text)
            .chain([DONE])
            .collect::<Vec<_>>()
            .join(" ");
        // A key counts as taught when a lesson names it: in backticks, or as a word.
        let taught: Vec<&str> = text
            .split('`')
            .skip(1)
            .step_by(2)
            .chain(text.split_whitespace())
            .collect();
        for (label, _) in crate::app::KEYS {
            if NOT_TAUGHT.contains(label) {
                continue;
            }
            assert!(
                label.split(" / ").any(|k| taught.contains(&k)),
                "`{label}` is in KEYS but no lesson mentions it: add a lesson to LESSONS, \
                 or add it to NOT_TAUGHT if it is a VS Code habit"
            );
        }
    }

    /// Drives the exact keys a learner types and checks that every lesson is reached.
    #[test]
    fn tutorial_is_completable() {
        let dir = extract().unwrap();
        let (tree, files) = crate::tree::build(&dir);
        let mut a = App::new(dir.clone(), tree, files, Buffer::empty(), None);
        a.show_tree = false;
        a.focus = Focus::Code;
        a.view_w = 80;
        a.view_h = 24;
        a.tutor = Some(Tutor {
            step: 0,
            dir: dir.clone(),
        });

        let mut lesson = 0;
        let mut done = |a: &mut App| {
            lesson += 1;
            assert_eq!(
                a.tutor.as_ref().unwrap().step,
                lesson,
                "lesson {lesson} ({}) did not advance",
                LESSONS[lesson - 1].title
            );
        };

        // 1: open a file
        press(&mut a, KeyCode::Char('o'));
        typed(&mut a, "store");
        press(&mut a, KeyCode::Enter);
        done(&mut a);

        // 2: find in the file
        press(&mut a, KeyCode::Char('/'));
        typed(&mut a, "load_config");
        press(&mut a, KeyCode::Enter);
        done(&mut a);

        // 3: next match
        press(&mut a, KeyCode::Char('n'));
        done(&mut a);

        // 4: go to definition
        press(&mut a, KeyCode::Char('d'));
        done(&mut a);

        // 5 and 6: back and forward
        press(&mut a, KeyCode::Char('['));
        done(&mut a);
        press(&mut a, KeyCode::Char(']'));
        done(&mut a);

        // 7: usages, then the hit in cli.py
        shift(&mut a, KeyCode::Right);
        shift(&mut a, KeyCode::Right);
        press(&mut a, KeyCode::Char('u'));
        press(&mut a, KeyCode::Down);
        press(&mut a, KeyCode::Enter);
        done(&mut a);

        // 8: search the project
        press(&mut a, KeyCode::Char('s'));
        typed(&mut a, "TODO");
        press(&mut a, KeyCode::Enter);
        press(&mut a, KeyCode::Enter);
        done(&mut a);

        // 9: project symbols
        press(&mut a, KeyCode::Char('D'));
        typed(&mut a, "NoteStore");
        press(&mut a, KeyCode::Enter);
        done(&mut a);

        // 10: go to line
        press(&mut a, KeyCode::Char(':'));
        typed(&mut a, "42");
        press(&mut a, KeyCode::Enter);
        done(&mut a);

        // 11 and 12: paragraph down, then up twice
        press(&mut a, KeyCode::Char('}'));
        done(&mut a);
        press(&mut a, KeyCode::Char('{'));
        press(&mut a, KeyCode::Char('{'));
        done(&mut a);

        // 13 and 14: the tree, then its focus
        press(&mut a, KeyCode::Char('t'));
        done(&mut a);
        press(&mut a, KeyCode::Tab);
        done(&mut a);

        // 15: open a file from the tree
        for _ in 0..4 {
            press(&mut a, KeyCode::Up);
        }
        press(&mut a, KeyCode::Right);
        press(&mut a, KeyCode::Down);
        press(&mut a, KeyCode::Enter);
        done(&mut a);

        // 16: hide the tree again
        press(&mut a, KeyCode::Char('t'));
        done(&mut a);

        // 17: edit, and the edit reaches the disk on Esc
        press(&mut a, KeyCode::Enter);
        typed(&mut a, "# hi");
        press(&mut a, KeyCode::Esc);
        done(&mut a);
        let saved = std::fs::read_to_string(dir.join("tests/test_store.py")).unwrap();
        assert!(saved.starts_with("# hi"), "{saved:?}");
        assert!(!a.dirty);

        // 18: undo, which reaches the disk on quit at the latest
        a.key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        done(&mut a);
        a.flush();
        let saved = std::fs::read_to_string(dir.join("tests/test_store.py")).unwrap();
        assert!(!saved.starts_with("# hi"), "{saved:?}");

        // 19 and 20: help, and closing it
        press(&mut a, KeyCode::Char('?'));
        done(&mut a);
        press(&mut a, KeyCode::Esc);
        done(&mut a);

        assert_eq!(a.tutor.as_ref().unwrap().step, LESSONS.len());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
