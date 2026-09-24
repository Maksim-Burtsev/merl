//! `merl --tutor`: a vimtutor-style walkthrough of the navigation keys.
//!
//! A small Python project is embedded in the binary, unpacked into a temporary directory and
//! opened like any other project. The lessons are tasks of [`POOL`], each training one action
//! from a start of its own, so they run in any order. A lesson advances when its predicate over
//! [`App`] becomes true — what the key did, not that a key was pressed — so there is no way to
//! fake progress. `merl --drill` asks the same pool without naming the keys: [`drill`].

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Focus};
use crate::buffer::Buffer;

pub mod drill;
mod pool;

pub use drill::Drill;
pub use pool::POOL;

/// One task: trains one action, starts from its own state, checks the effect.
pub struct Task {
    /// The action it trains, as the key stats name it: one side of a `KEYS` row, an alias
    /// folded into its primary.
    pub key: &'static str,
    pub title: &'static str,
    /// `--tutor`'s text: names the key. The drill shows it too, for the redo after a miss.
    pub tutor: &'static str,
    /// The drill's: what to do, never the key, nor how many keys it takes.
    pub drill: &'static str,
    /// The file, 1-based line and word the cursor starts on (`""` is the line start); `None`
    /// starts with nothing open.
    pub start: Option<(&'static str, usize, &'static str)>,
    /// Pressed silently after `start`, in [`press`] notation: the start is always a state real
    /// work reaches.
    pub keys: &'static str,
    /// The shortest keys that do it, in [`press`] notation: the tests press it.
    #[allow(dead_code)]
    pub answer: &'static str,
    /// It is done: a predicate over the effect, never over the size of the terminal.
    pub done: fn(&App) -> bool,
}

pub struct Tutor {
    /// Index into [`TUTOR`]; `TUTOR.len()` means the tutorial is over.
    pub step: usize,
    /// The unpacked sample project, removed when merl exits.
    pub dir: PathBuf,
    /// `--drill`: the session, asked in place of the lessons.
    pub drill: Option<Drill>,
}

/// The tutorial: the keys of the [`POOL`] tasks it walks, in order.
pub const TUTOR: &[&str] = &[
    "o",
    "/",
    "n",
    "d",
    "[",
    "]",
    "u",
    "s",
    "D",
    ":",
    "}",
    "{",
    "v",
    "t",
    "Tab",
    "Tree: Enter",
    "Enter",
    "Ctrl+Z",
    "w",
    "Picker: Esc",
    "Esc",
];

/// The sample project, as it sits in `tutor/notes/`.
pub const FILES: &[(&str, &str)] = &[
    ("Makefile", include_str!("../tutor/notes/Makefile")),
    ("cli.py", include_str!("../tutor/notes/cli.py")),
    ("config.py", include_str!("../tutor/notes/config.py")),
    ("export.csv", include_str!("../tutor/notes/export.csv")),
    ("models.py", include_str!("../tutor/notes/models.py")),
    ("notes.json", include_str!("../tutor/notes/notes.json")),
    ("store.py", include_str!("../tutor/notes/store.py")),
    (
        "tests/test_store.py",
        include_str!("../tutor/notes/tests/test_store.py"),
    ),
];

/// Shown once every lesson is done.
pub const DONE: &str = "That is all of merl. Everything else is a VS Code habit. `q` quits.";

/// The lesson at `step` of the tutorial, `None` past its end.
pub fn lesson(step: usize) -> Option<&'static Task> {
    let key = TUTOR.get(step)?;
    POOL.iter().find(|t| t.key == *key)
}

/// Called after every key, with the action it was routed to: advances at most one step, so a
/// state that happens to satisfy two predicates cannot skip a lesson. The next lesson is set up
/// at once: a learner who did what the last one asked sees nothing move, one who wandered off is
/// put back. Under `--drill` the drill checks instead.
pub fn check(app: &mut App, action: Option<&str>) {
    let Some(tutor) = &app.tutor else {
        return;
    };
    if tutor.drill.is_some() {
        return drill::check(app, action);
    }
    let step = tutor.step;
    let Some(task) = lesson(step) else {
        return;
    };
    if !(task.done)(app) {
        return;
    }
    // The tick goes in front of what the key itself reported, such as how `d` found its target.
    let tick = match app.message.as_str() {
        "" => format!("\u{2713} {}", task.title),
        said => format!("\u{2713} {}  {said}", task.title),
    };
    if let Some(tutor) = &mut app.tutor {
        tutor.step = step + 1;
    }
    app.message = match begin(app) {
        Ok(()) => tick,
        // The next lesson goes on from where this one left off.
        Err(e) => format!("{tick}  {e:#}"),
    };
}

/// Sets the current lesson, or the drill's first task, up on a fresh App; past the last lesson
/// nothing changes.
pub fn begin(app: &mut App) -> Result<()> {
    if app.tutor.as_ref().is_some_and(|t| t.drill.is_some()) {
        return drill::next(app);
    }
    let Some(task) = app.tutor.as_ref().and_then(|t| lesson(t.step)) else {
        return Ok(());
    };
    fresh(app)?;
    set_up(app, task);
    Ok(())
}

/// Puts `a` back where a fresh `--tutor` starts: the sample project unpacked anew, which drops
/// edits and the files a task made, and nothing open, no history, no undo, no find. What `main`
/// set on the old App is carried over. Outside the tutor it does nothing.
///
/// A new `App` rather than a field-by-field reset: a field someone forgets to carry over shows
/// at once (the theme falls back), one someone forgets to reset would leak from a task into the
/// next unseen.
pub fn fresh(a: &mut App) -> Result<()> {
    let Some(dir) = a.tutor.as_ref().map(|t| t.dir.clone()) else {
        return Ok(());
    };
    unpack(&dir).context("the sample project could not be unpacked again")?;
    let tutor = a.tutor.take();
    let (tree, files) = crate::tree::build(&dir, false);
    let mut new = App::new(dir, tree, files, Buffer::empty(), None);
    (new.show_tree, new.focus) = (false, Focus::Code);
    new.theme = std::mem::take(&mut a.theme);
    new.config = a.config.take();
    new.autosave = a.autosave;
    new.no_watch = a.no_watch;
    (new.view_w, new.view_h) = (a.view_w, a.view_h);
    new.tutor = tutor;
    *a = new;
    Ok(())
}

/// Opens the task's start and presses its keys. Silently: they answer no lesson, count as no
/// press, and what they said is not on the status bar.
pub fn set_up(a: &mut App, task: &Task) {
    let tutor = a.tutor.take();
    if let Some((file, line, word)) = task.start {
        a.jump_to(&a.root.join(file), line);
        // The one thing set by hand: the column, where the jump history follows it too.
        a.col = a.line_str().find(word).unwrap_or(0);
        a.sync_want_x();
        a.hist_note(false);
    }
    press(a, task.keys);
    a.pressed.clear();
    a.message.clear();
    a.tutor = tutor;
}

/// Presses `notation`: characters as typed, and `<Enter>`, `<A-Right>`, `<C-z>`, `<A-S-Right>`
/// in angle brackets. A picker's matcher runs to completion before every key, as the event
/// loop's tick lets it.
pub fn press(a: &mut App, notation: &str) {
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

/// `A-S-Right` in [`press`] notation: `C`, `A` and `S` for Ctrl, Alt and Shift, then the key.
fn chord(name: &str) -> (KeyCode, KeyModifiers) {
    let mut mods = KeyModifiers::NONE;
    let mut name = name;
    while let Some((m, rest)) = name.split_once('-').filter(|(m, _)| m.len() == 1) {
        mods |= match m {
            "C" => KeyModifiers::CONTROL,
            "A" => KeyModifiers::ALT,
            "S" => KeyModifiers::SHIFT,
            _ => panic!("modifier {m} in <{name}>"),
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
        _ => panic!("key <{name}>"),
    };
    (code, mods)
}

/// Unpacks the sample project into `dir`, over whatever is there.
fn unpack(dir: &Path) -> Result<()> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir.join("tests"))?;
    for (name, text) in FILES {
        std::fs::write(dir.join(name), text)?;
    }
    Ok(())
}

/// Unpacks the sample project into a fresh temporary directory.
pub fn extract() -> Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("merl-tutor-{}", std::process::id()));
    unpack(&dir)?;
    Ok(dir)
}

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

/// The selected text, when the selection sits on one line.
fn selected(a: &App) -> Option<&str> {
    let ((l1, c1), (l2, c2)) = a.selection()?;
    (l1 == l2).then(|| &a.buf.lines[l1][c1..c2])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::ACTIONS;

    /// Keys in `KEYS` no task trains. A new row in `KEYS` fails the test below until a task of
    /// [`POOL`] trains each of its sides, or it is listed here on purpose.
    const NOT_TAUGHT: &[&str] = &[
        // Nothing to train.
        "Ctrl+S", "Ctrl+R", "T", "?", "q",
        // Review mode only: the sample project has no branch to walk yet (#158).
        "c", "C", "m",
    ];

    /// The copy of the sample project a test works in, apart from the other tests' copies.
    pub(super) fn dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("merl-tutor-{}-{name}", std::process::id()))
    }

    /// A tutor over the copy `name`, its lessons over, so that only the test sets tasks up.
    pub(super) fn app(name: &str, (w, h): (usize, usize)) -> App {
        let dir = dir(name);
        unpack(&dir).unwrap();
        let (tree, files) = crate::tree::build(&dir, false);
        let mut a = App::new(dir.clone(), tree, files, Buffer::empty(), None);
        (a.show_tree, a.focus) = (false, Focus::Code);
        (a.view_w, a.view_h) = (w, h);
        a.tutor = Some(Tutor {
            step: TUTOR.len(),
            dir,
            drill: None,
        });
        a
    }

    /// One line of state, for a failure to say where a task went.
    fn state(a: &App) -> String {
        let picker = a
            .picker
            .as_ref()
            .map_or(String::new(), |p| format!(" picker row {}", p.selected));
        let sel = selected(a).map_or(String::new(), |s| format!(" sel {s:?}"));
        format!(
            "{}:{}:{} {:?}{picker}{sel} {:?}",
            a.rel_path(),
            a.line + 1,
            a.col + 1,
            a.mode,
            a.message
        )
    }

    /// Sets `t` up on a fresh App and presses its answer: the start is not done yet, the
    /// answer does it, and the answer presses the task's own key.
    fn run(a: &mut App, t: &Task) {
        fresh(a).unwrap();
        set_up(a, t);
        let before = state(a);
        assert!(!(t.done)(a), "{}: done before a key: {before}", t.key);
        // Out of the tutor the presses are counted.
        let tutor = a.tutor.take();
        press(a, t.answer);
        a.settle_search();
        a.tutor = tutor;
        let after = state(a);
        assert!((t.done)(a), "{}: {before} --{}--> {after}", t.key, t.answer);
        assert!(
            a.pressed.contains_key(t.key),
            "{}: the answer {} presses {:?}",
            t.key,
            t.answer,
            a.pressed.keys()
        );
    }

    /// Every task from cold, then the whole pool in order and reversed on one App, each run at
    /// another size: nothing a task leaves behind, on disk or in a field carried over, decides
    /// the next, and no `done` hangs on the size of the terminal.
    #[test]
    fn every_task_starts_undone_and_its_answer_does_it() {
        for t in POOL {
            run(&mut app("cold", (80, 24)), t);
        }
        let mut a = app("walk", (200, 60));
        for t in POOL {
            run(&mut a, t);
        }
        (a.view_w, a.view_h) = (60, 16);
        for t in POOL.iter().rev() {
            run(&mut a, t);
        }
        for name in ["cold", "walk"] {
            std::fs::remove_dir_all(dir(name)).unwrap();
        }
    }

    /// Pressing each lesson's answer walks the whole tutorial: `check` moves on and sets the
    /// next lesson up, and the tick keeps what the key itself said.
    #[test]
    fn the_tutor_sets_up_each_lesson_it_moves_on_to() {
        let mut a = app("tutor", (80, 24));
        a.tutor.as_mut().unwrap().step = 0;
        begin(&mut a).unwrap();
        for (i, key) in TUTOR.iter().enumerate() {
            let before = state(&a);
            press(&mut a, lesson(i).unwrap().answer);
            a.settle_search();
            assert_eq!(
                a.tutor.as_ref().unwrap().step,
                i + 1,
                "lesson {key} did not advance: {before} --> {}",
                state(&a)
            );
            if *key == "d" {
                assert_eq!(
                    a.message,
                    "\u{2713} Go to definition  load_config: via import config.py"
                );
            }
        }
        assert!(lesson(TUTOR.len()).is_none());
        std::fs::remove_dir_all(dir("tutor")).unwrap();
    }

    #[test]
    fn every_key_is_taught_or_skipped_on_purpose() {
        for a in ACTIONS.iter() {
            let tasks = POOL.iter().filter(|t| t.key == a.name).count();
            let skipped = usize::from(NOT_TAUGHT.contains(&a.name.as_str()));
            assert_eq!(
                tasks + skipped,
                1,
                "`{}` is in KEYS: add a task to POOL that trains it, or list it in NOT_TAUGHT \
                 if there is nothing to train (one or the other, not both)",
                a.name
            );
        }
        for t in POOL {
            assert!(
                ACTIONS.iter().any(|a| a.name == t.key),
                "{} is no action of KEYS",
                t.key
            );
        }
    }

    /// What a text names: its backtick spans, and its words with the punctuation around them
    /// trimmed. A key is one of them or it is not named: the `d` in "defined" is no `d`.
    fn named(text: &str) -> Vec<&str> {
        let words = text
            .split_whitespace()
            .map(|w| w.trim_matches(['.', ',', ';', ':', '(', ')']));
        text.split('`').skip(1).step_by(2).chain(words).collect()
    }

    #[test]
    fn a_tutor_text_names_its_key_and_a_drill_text_never_does() {
        for t in POOL {
            let key = t.key.rsplit(": ").next().unwrap();
            assert!(named(t.tutor).contains(&key), "{}: tutor text", t.key);
            assert!(!named(t.drill).contains(&key), "{}: drill text", t.key);
        }
    }

    #[test]
    fn the_sample_project_has_exactly_one_todo() {
        let todos: usize = FILES.iter().map(|(_, t)| t.matches("TODO").count()).sum();
        assert_eq!(todos, 1, "the `s` task searches for the single TODO");
    }
}
