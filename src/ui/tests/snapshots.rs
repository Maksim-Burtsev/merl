//! Whole screens as text, one golden file per state in `tests/snapshots/`: every overlay and
//! picker, the tree, the review panel, the tutor and the drill, the preview, the start screen
//! and the status bar, each at three sizes and in a dark and a light theme.
//!
//! A frame is its text, then per theme a grid of one code per cell and the legend that names
//! each code's colours, so a moved column and a changed colour both show in the diff.
//! `MERL_UPDATE_SNAPSHOTS=1 cargo test snapshots` writes the files anew; review them with
//! `git diff tests/snapshots`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;

use crate::app::{App, Focus};
use crate::buffer::Buffer;
use crate::tutor::{Drill, Tutor, press};

const SIZES: &[(u16, u16)] = &[(80, 24), (120, 33), (185, 55)];
const THEMES: &[&str] = &[crate::theme::DEFAULT, "github-light"];
/// One per style on a screen, in the order they first show, top left to bottom right.
const CODES: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789#$%&*+=@^~";

/// A state: its name, and what brings a fresh App on the sample project to it.
type State = (&'static str, fn(&mut App));

/// The sample project's `store.py`, on the call of `load_config` (the tutor's `CALL_LINE`).
fn on_call(a: &mut App) {
    at(a, "store.py", 13, "load_config");
}

const STATES: &[State] = &[
    ("start", |_| {}),
    ("code", |a| {
        on_call(a);
        a.focus = Focus::Code;
    }),
    ("tree", |a| {
        on_call(a);
        a.marks = [("store.py", 'M'), ("PLAN.md", 'A')]
            .map(|(p, m)| (p.into(), m))
            .into();
        press(a, "<Tab><Up><Up>");
    }),
    // Below 118 columns an action too long for its row wraps under its own column (#458).
    ("help", |a| {
        on_call(a);
        press(a, "?");
    }),
    ("files", |a| press(a, "ost")),
    ("search", |a| {
        press(a, "sload_config");
        a.settle_search();
    }),
    ("symbols", |a| {
        on_call(a);
        press(a, "DNote");
    }),
    ("usages", |a| {
        at(a, "config.py", 18, "load_config");
        press(a, "u");
    }),
    ("definitions", |a| {
        at(a, "cli.py", 24, "render");
        press(a, "d");
    }),
    ("definition-jump", |a| {
        on_call(a);
        press(a, "d");
    }),
    ("themes", |a| press(a, "T")),
    ("goto", |a| {
        on_call(a);
        press(a, ":42");
    }),
    ("find", |a| {
        on_call(a);
        press(a, "/note");
    }),
    ("new-file", |a| press(a, "<C-n>todo.txt")),
    ("edit", |a| {
        on_call(a);
        press(a, "<End><Enter>  # hi");
    }),
    ("selection", |a| {
        on_call(a);
        press(a, "vv");
    }),
    // Too wide to wrap: merl opens it cut at the edge.
    ("nowrap", |a| at(a, "export.csv", 1, "")),
    ("preview", |a| {
        at(a, "PLAN.md", 1, "");
        press(a, "p");
    }),
    ("image", |a| {
        let path = a.root.join("logo.png");
        let alpha = |x: u32| if x < 48 { 255 } else { 0 };
        image::RgbaImage::from_fn(96, 48, |x, _| image::Rgba([0, 0, 0, alpha(x)]))
            .save(&path)
            .unwrap();
        a.diagrams = crate::mermaid::Diagrams::with_cell(Some((10, 20)));
        a.jump_to(&path, 1);
        a.diagrams.file_size(&path);
        for job in a.diagrams.jobs() {
            a.diagrams.done(crate::mermaid::render(job));
        }
    }),
    ("review", |a| review(a, &[], None, Some("store.py"))),
    ("review-empty", |a| {
        review(a, &["tests/__init__.py"], None, None)
    }),
    ("review-rename", |a| {
        review(a, &[], Some(("config.py", "app/settings.py")), None);
    }),
    // Below 185 columns the lesson loses what passes its two rows (#261).
    ("tutor", |a| {
        a.tutor = Some(Tutor {
            step: 3,
            dir: a.root.clone(),
            drill: None,
        });
        crate::tutor::begin(a).unwrap();
    }),
    ("drill", |a| {
        let drill = Drill::new(5, None, None, crate::stats::today()).unwrap();
        a.tutor = Some(Tutor {
            step: 0,
            dir: a.root.clone(),
            drill: Some(drill),
        });
        crate::tutor::begin(a).unwrap();
    }),
];

/// Opens `file` at the 1-based `line`, the cursor on `word`, as a tutor task starts.
fn at(a: &mut App, file: &str, line: usize, word: &str) {
    a.jump_to(&a.root.join(file), line);
    a.col = a.line_str().find(word).unwrap_or(0);
    a.sync_want_x();
}

fn review(a: &mut App, empty: &[&str], moved: Option<(&str, &str)>, open: Option<&str>) {
    let dir = a.root.clone();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "t@t"]);
    git(&["config", "user.name", "t"]);
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    let store = std::fs::read_to_string(dir.join("store.py")).unwrap();
    let store = store.replace("load_config(", "load_settings(").replace(
        "    def remove",
        "    def count(self) -> int:\n        return len(self.notes)\n\n    def remove",
    );
    std::fs::write(dir.join("store.py"), store).unwrap();
    std::fs::write(dir.join("tags.py"), "TAGS = [\"work\", \"home\"]\n").unwrap();
    std::fs::remove_file(dir.join("Makefile")).unwrap();
    for name in empty {
        std::fs::write(dir.join(name), "").unwrap();
    }
    if let Some((from, to)) = moved {
        std::fs::create_dir_all(dir.join(to).parent().unwrap()).unwrap();
        std::fs::rename(dir.join(from), dir.join(to)).unwrap();
    }
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "work"]);

    let review = crate::git::Review::open(&dir, None, None).unwrap();
    let (_, files) = crate::tree::build(&dir, false);
    let paths: Vec<PathBuf> = review.files.iter().map(|f| f.path.clone()).collect();
    let mut new = App::new(
        dir.clone(),
        crate::tree::from_files(&paths),
        files,
        open.map_or_else(Buffer::empty, |f| Buffer::load(&dir.join(f)).unwrap()),
        None,
    );
    (new.theme, new.theme_dir) = (a.theme.clone(), None);
    (new.view_w, new.view_h) = (a.view_w, a.view_h);
    new.no_external();
    new.start_review(review);
    *a = new;
}

fn notes(state: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("merl-snapshots-{}", std::process::id()))
        .join(state)
        .join("notes");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    for (name, text) in crate::tutor::FILES {
        std::fs::write(dir.join(name), text).unwrap();
    }
    dir
}

fn frame(state: &State, theme: &str, (w, h): (u16, u16)) -> Terminal<TestBackend> {
    let dir = notes(state.0);
    let (tree, files) = crate::tree::build(&dir, false);
    let mut a = App::new(dir, tree, files, Buffer::empty(), None);
    // Nothing of the machine's own: its themes would join the `T` list, its libraries `d`'s.
    (a.theme, a.theme_dir) = (theme.to_string(), None);
    a.no_external();
    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    let draw = |t: &mut Terminal<TestBackend>, a: &mut App| {
        let theme = crate::theme::load_from(None, a.shown_theme()).unwrap();
        t.draw(|f| super::draw(f, a, &theme)).unwrap();
    };
    // The first frame gives the App its viewport, as the event loop's does before any key.
    draw(&mut terminal, &mut a);
    (state.1)(&mut a);
    if let Some(p) = &mut a.picker {
        p.settle();
    }
    draw(&mut terminal, &mut a);
    // Twice: a first frame lays out what the second scrolls to, as the preview does.
    draw(&mut terminal, &mut a);
    let _ = std::fs::remove_dir_all(a.root.parent().unwrap());
    terminal
}

/// Each row's text, a wide char standing for the cells it covers.
fn text(t: &Terminal<TestBackend>) -> String {
    let buf = t.backend().buffer();
    let mut out = String::new();
    for y in 0..buf.area.height {
        let mut row = String::new();
        let mut skip = 0;
        for x in 0..buf.area.width {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            let s = buf[(x, y)].symbol();
            skip = crate::wrap::width(s).saturating_sub(1);
            row.push_str(s);
        }
        out += row.trim_end();
        out.push('\n');
    }
    out
}

fn styles(t: &Terminal<TestBackend>) -> String {
    let buf = t.backend().buffer();
    let mut seen: Vec<String> = Vec::new();
    let mut grid = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let c = &buf[(x, y)];
            let mut style = format!("fg {} bg {}", hex(c.fg), hex(c.bg));
            if !c.modifier.is_empty() {
                let _ = write!(style, " {:?}", c.modifier);
            }
            let i = seen.iter().position(|s| *s == style).unwrap_or_else(|| {
                seen.push(style);
                seen.len() - 1
            });
            grid.push(CODES.chars().nth(i).expect("more styles than codes"));
        }
        grid.push('\n');
    }
    for (code, style) in CODES.chars().zip(&seen) {
        let _ = writeln!(grid, "{code} {style}");
    }
    grid
}

fn hex(c: Color) -> String {
    match c {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        c => format!("{c:?}").to_lowercase(),
    }
}

fn cursor(t: &Terminal<TestBackend>) -> String {
    let b = t.backend();
    match b.cursor_visible() {
        true => format!("cursor {},{}", b.cursor_position().x, b.cursor_position().y),
        false => "cursor hidden".to_string(),
    }
}

/// The golden file of `state`: per size the dark theme's text, then each theme's styles, and
/// a theme's own text only where it differs from the dark one's.
fn render(state: &State) -> String {
    let mut out = format!("merl snapshot: {}\n", state.0);
    for &size in SIZES {
        let mut first = String::new();
        for &theme in THEMES {
            let t = frame(state, theme, size);
            let (text, head) = (text(&t), format!("{}x{} {theme}", size.0, size.1));
            if first.is_empty() || text != first {
                let _ = write!(out, "\n==== {head} text, {}\n{text}", cursor(&t));
                first = text;
            }
            let _ = write!(out, "\n==== {head} styles\n{}", styles(&t));
        }
    }
    out
}

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots")
}

fn diff(old: &str, new: &str) -> String {
    let (a, b): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
    // lcs[i][j]: the longest common subsequence of a[i..] and b[j..].
    let mut lcs = vec![vec![0u32; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = match a[i] == b[j] {
                true => lcs[i + 1][j + 1] + 1,
                false => lcs[i + 1][j].max(lcs[i][j + 1]),
            };
        }
    }
    let (mut i, mut j, mut out, mut n) = (0, 0, String::new(), 0);
    // The section a changed line is in, said once above its first change.
    let (mut head, mut said) = ("", "");
    while (i < a.len() || j < b.len()) && n < 60 {
        if let Some(line) = b.get(j).filter(|l| l.starts_with("====")) {
            head = line;
        }
        if i < a.len() && j < b.len() && a[i] == b[j] {
            (i, j) = (i + 1, j + 1);
            continue;
        }
        if head != said {
            let _ = writeln!(out, "{head}");
            said = head;
        }
        if i < a.len() && (j == b.len() || lcs[i + 1][j] >= lcs[i][j + 1]) {
            let _ = writeln!(out, "-{:4} {}", i + 1, a[i]);
            i += 1;
        } else {
            let _ = writeln!(out, "+{:4} {}", j + 1, b[j]);
            j += 1;
        }
        n += 1;
    }
    out
}

fn check(name: &str) {
    let state = STATES.iter().find(|s| s.0 == name).unwrap();
    let path = dir().join(format!("{name}.txt"));
    let got = render(state);
    if std::env::var_os("MERL_UPDATE_SNAPSHOTS").is_some_and(|v| v == "1") {
        std::fs::create_dir_all(dir()).unwrap();
        std::fs::write(&path, &got).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        got == want,
        "the {name} screen changed; if on purpose, `MERL_UPDATE_SNAPSHOTS=1 cargo test \
         snapshots` and commit tests/snapshots/{name}.txt\n{}",
        diff(&want, &got)
    );
}

macro_rules! snapshots {
    ($($test:ident: $name:literal,)*) => {
        $(#[test]
        fn $test() {
            check($name);
        })*

        #[test]
        fn snapshots_match_the_states() {
            let tested = [$($name),*];
            let states: Vec<&str> = STATES.iter().map(|s| s.0).collect();
            assert_eq!(states, tested, "every state has its test");
            let mut files: Vec<String> = std::fs::read_dir(dir())
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            files.sort();
            let mut want: Vec<String> = states.iter().map(|s| format!("{s}.txt")).collect();
            want.sort();
            assert_eq!(files, want, "a golden file without its state, or none for one");
        }
    };
}

snapshots! {
    snapshot_start: "start",
    snapshot_code: "code",
    snapshot_tree: "tree",
    snapshot_help: "help",
    snapshot_files: "files",
    snapshot_search: "search",
    snapshot_symbols: "symbols",
    snapshot_usages: "usages",
    snapshot_definitions: "definitions",
    snapshot_definition_jump: "definition-jump",
    snapshot_themes: "themes",
    snapshot_goto: "goto",
    snapshot_find: "find",
    snapshot_new_file: "new-file",
    snapshot_edit: "edit",
    snapshot_selection: "selection",
    snapshot_nowrap: "nowrap",
    snapshot_preview: "preview",
    snapshot_image: "image",
    snapshot_review: "review",
    snapshot_review_empty: "review-empty",
    snapshot_review_rename: "review-rename",
    snapshot_tutor: "tutor",
    snapshot_drill: "drill",
}
