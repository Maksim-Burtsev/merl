use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::app::{App, Focus};
use crate::buffer::Buffer;
use crate::key::{KeyCode, KeyEvent, KeyModifiers};

pub mod drill;
mod pool;

pub use drill::Drill;
pub use pool::POOL;

pub struct Task {
    pub key: &'static str,
    pub title: &'static str,
    pub tutor: &'static str,
    pub drill: &'static str,
    pub start: Option<(&'static str, usize, &'static str)>,
    pub keys: &'static str,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "only the tests press it, to prove each task's answer does it"
        )
    )]
    pub answer: &'static str,
    pub done: fn(&App) -> bool,
}

pub struct Tutor {
    pub step: usize,
    pub dir: PathBuf,
    pub drill: Option<Drill>,
}

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
    "p",
    "f",
    "Picker: Esc",
    "Esc",
    "c",
    "C",
    "m",
];

pub(super) const REVIEW_LESSONS: &[&str] = &["c", "C", "m"];

const AGENT_S_WORK: &[(&str, &str)] = &[
    ("PLAN.md", include_str!("../tutor/review/PLAN.md")),
    ("cli.py", include_str!("../tutor/review/cli.py")),
    ("models.py", include_str!("../tutor/review/models.py")),
    ("store.py", include_str!("../tutor/review/store.py")),
    (
        "tests/test_cli.py",
        include_str!("../tutor/review/tests/test_cli.py"),
    ),
    (
        "tests/test_store.py",
        include_str!("../tutor/review/tests/test_store.py"),
    ),
];

pub const SAMPLE_FILES: &[(&str, &str)] = &[
    ("Makefile", include_str!("../tutor/notes/Makefile")),
    ("PLAN.md", include_str!("../tutor/notes/PLAN.md")),
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

pub const ALL_LESSONS_DONE: &str =
    "That is all of merl. Everything else is a VS Code habit. `q` quits.";

pub fn lesson(step: usize) -> Option<&'static Task> {
    let key = TUTOR.get(step)?;
    POOL.iter().find(|t| t.key == *key)
}

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
    let tick = match app.message.as_str() {
        "" => format!("\u{2713} {}", task.title),
        said => format!("\u{2713} {}  {said}", task.title),
    };
    if let Some(tutor) = &mut app.tutor {
        tutor.step = step + 1;
    }
    app.message = match begin(app) {
        Ok(()) => tick,
        Err(e) => format!("{tick}  {}", crate::app::error_text(&e)),
    };
}

pub fn begin(app: &mut App) -> Result<()> {
    app.config = None;
    if app.tutor.as_ref().is_some_and(|t| t.drill.is_some()) {
        return drill::next(app);
    }
    let Some(task) = app.tutor.as_ref().and_then(|t| lesson(t.step)) else {
        return Ok(());
    };
    fresh(app)?;
    set_up(app, task).inspect_err(|_| {
        if let Some(t) = app
            .tutor
            .as_mut()
            .filter(|_| REVIEW_LESSONS.contains(&task.key))
        {
            t.step = TUTOR.len();
        }
    })
}

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

pub fn set_up(a: &mut App, task: &Task) -> Result<()> {
    if REVIEW_LESSONS.contains(&task.key) {
        open_sample_review(a).context("review lessons skipped")?;
    }
    let tutor = a.tutor.take();
    if let Some((file, line, word)) = task.start {
        a.jump_to(&a.root.join(file), line);
        a.col = a.line_str().find(word).unwrap_or(0);
        a.sync_want_x();
        a.hist_note(false);
        a.remember_hunk();
    }
    press(a, task.keys);
    a.pressed.clear();
    a.message.clear();
    a.tutor = tutor;
    Ok(())
}

const REPO_REDIRECTS: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_COMMON_DIR",
];

fn redirect_set(is_set: impl Fn(&str) -> bool) -> Option<&'static str> {
    REPO_REDIRECTS.iter().copied().find(|v| is_set(v))
}

fn sample_git(root: &Path, args: &[&str]) -> Result<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(["-c", "user.name=merl", "-c", "user.email=merl@localhost"])
        .args(args)
        .output()
        .map_err(|_| anyhow::anyhow!("git not found"))?;
    anyhow::ensure!(
        out.status.success(),
        "git {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr).trim()
    );
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn open_sample_review(a: &mut App) -> Result<()> {
    if let Some(var) = redirect_set(|v| std::env::var_os(v).is_some()) {
        anyhow::bail!("{var} is set");
    }
    let git = |args: &[&str]| sample_git(&a.root, args);
    git(&["-c", "init.defaultBranch=agent/find-by-tag", "init", "-q"])?;
    git(&["add", "-A"])?;
    git(&["commit", "-q", "--no-verify", "-m", "notes"])?;
    for (name, text) in AGENT_S_WORK {
        std::fs::write(a.root.join(name), text)?;
    }
    let r = crate::git::Review::open(&a.root, None, Some("HEAD"))?;
    a.tree = crate::tree::from_listing(&r.files.iter().map(|f| f.path.clone()).collect::<Vec<_>>());
    a.show_tree = true;
    a.start_review(r);
    a.session = None;
    a.message.clear();
    Ok(())
}

pub fn press(a: &mut App, notation: &str) {
    let mut rest = notation;
    while let Some(c) = rest.chars().next() {
        let (code, mods, after) =
            if let Some((inner, after)) = rest.strip_prefix('<').and_then(|r| r.split_once('>')) {
                let (code, mods) = chord(inner);
                (code, mods, after)
            } else {
                let mut tail = rest.chars();
                tail.next();
                (KeyCode::Char(c), KeyModifiers::NONE, tail.as_str())
            };
        rest = after;
        if let Some(p) = &mut a.picker {
            p.settle();
        }
        a.key(KeyEvent::new(code, mods));
    }
}

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
        c if c.chars().count() == 1 => KeyCode::Char(c.chars().next().expect("one char")),
        _ => panic!("key <{name}>"),
    };
    (code, mods)
}

fn unpack(dir: &Path) -> Result<()> {
    drop(std::fs::remove_dir_all(dir));
    std::fs::create_dir_all(dir.join("tests"))?;
    for (name, text) in SAMPLE_FILES {
        std::fs::write(dir.join(name), text)?;
    }
    Ok(())
}

pub fn unpack_to_temp_dir() -> Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("merl-tutor-{}", std::process::id()));
    unpack(&dir)?;
    Ok(dir)
}

fn at(app: &App, file: &str) -> bool {
    app.rel_path() == file
}

fn edited_since_unpacked(app: &App) -> bool {
    SAMPLE_FILES
        .iter()
        .find(|(name, _)| at(app, name))
        .is_some_and(|(_, text)| text.lines().ne(app.buf.lines.iter().map(String::as_str)))
}

fn selected_on_one_line(a: &App) -> Option<&str> {
    let ((l1, c1), (l2, c2)) = a.selection()?;
    (l1 == l2).then(|| a.text(l1).get(c1..c2)).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::ACTIONS;

    const NOT_TAUGHT: &[(&str, &str)] = &[
        ("Ctrl+S", "nothing to train"),
        ("Ctrl+R", "nothing to train"),
        ("T", "nothing to train"),
        ("?", "nothing to train"),
        ("q", "nothing to train"),
        ("Fold: Enter", REVIEW_ONLY),
        ("Shift+Home", REVIEW_ONLY),
        ("Shift+End", REVIEW_ONLY),
    ];
    const REVIEW_ONLY: &str = "review mode only: the tutor's review lessons teach c, C and m";

    pub(super) fn dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("merl-tutor-{}-{name}", std::process::id()))
    }

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

    fn state(a: &App) -> String {
        let picker = a
            .picker
            .as_ref()
            .map_or(String::new(), |p| format!(" picker row {}", p.selected));
        let sel = selected_on_one_line(a).map_or(String::new(), |s| format!(" sel {s:?}"));
        format!(
            "{}:{}:{} {:?}{picker}{sel} {:?}",
            a.rel_path(),
            a.line + 1,
            a.col + 1,
            a.mode,
            a.message
        )
    }

    fn run(a: &mut App, t: &Task) {
        fresh(a).unwrap();
        set_up(a, t).unwrap();
        let before = state(a);
        assert!(!(t.done)(a), "{}: done before a key: {before}", t.key);
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

    #[test]
    fn the_tutor_sets_up_each_lesson_it_moves_on_to() {
        let mut a = app("tutor", (80, 24));
        a.tutor.as_mut().unwrap().step = 0;
        begin(&mut a).unwrap();
        for (i, key) in TUTOR.iter().enumerate() {
            let before = state(&a);
            if REVIEW_LESSONS.contains(key) {
                assert!(
                    a.review.is_some() && a.show_tree,
                    "{key}: no review on screen"
                );
            }
            press(&mut a, lesson(i).unwrap().answer);
            assert!(a.session.is_none(), "{key}: the tutor's review is logged");
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
    fn a_start_is_pressed_silently() {
        let mut a = app("silent", (80, 24));
        for t in POOL.iter().filter(|t| !t.keys.is_empty()) {
            fresh(&mut a).unwrap();
            set_up(&mut a, t).unwrap();
            assert!(a.pressed.is_empty(), "{}: {:?}", t.key, a.pressed);
            assert_eq!(a.message, "", "{}", t.key);
        }
        std::fs::remove_dir_all(dir("silent")).unwrap();
    }

    #[test]
    fn a_fresh_app_keeps_what_main_set() {
        let mut a = app("carried", (99, 33));
        (a.autosave, a.no_watch) = (
            a.autosave * 3 + std::time::Duration::from_millis(7),
            !a.no_watch,
        );
        let (autosave, no_watch) = (a.autosave, a.no_watch);
        fresh(&mut a).unwrap();
        assert_eq!(
            (a.autosave, a.no_watch, a.view_w, a.view_h),
            (autosave, no_watch, 99, 33)
        );
        assert!(a.tutor.is_some());
        std::fs::remove_dir_all(dir("carried")).unwrap();
    }

    #[test]
    fn the_tutor_never_saves_the_theme() {
        let mut a = app("config", (80, 24));
        let config = dir("config").join("config.toml");
        a.config = Some(config.clone());
        begin(&mut a).unwrap();
        press(&mut a, "T<Down><Enter>");
        assert!(a.picker.is_none(), "{}", state(&a));
        assert!(!config.exists());
        std::fs::remove_dir_all(dir("config")).unwrap();
    }

    #[test]
    fn every_key_is_taught_or_skipped_on_purpose() {
        for a in ACTIONS.iter() {
            let tasks = POOL.iter().filter(|t| t.key == a.name).count();
            let skipped = usize::from(NOT_TAUGHT.iter().any(|(k, _)| *k == a.name));
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
        let todos: usize = SAMPLE_FILES
            .iter()
            .map(|(_, t)| t.matches("TODO").count())
            .sum();
        assert_eq!(todos, 1, "the `s` task searches for the single TODO");
    }

    #[test]
    fn the_sample_review_reads_no_git_config_of_the_user() {
        let dir = dir("config-free");
        std::fs::create_dir_all(&dir).unwrap();
        let listed = sample_git(&dir, &["config", "--list", "--show-origin"]).unwrap();
        let lines: Vec<&str> = listed.lines().collect();
        assert_eq!(
            lines,
            [
                "command line:\tuser.name=merl",
                "command line:\tuser.email=merl@localhost"
            ]
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_repository_named_by_the_environment_stops_the_sample_review() {
        assert_eq!(redirect_set(|_| false), None);
        for var in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
            assert_eq!(redirect_set(|v| v == var), Some(var));
        }
    }
}
