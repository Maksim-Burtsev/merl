//! The tests, following the code they cover.

use super::definition::resolution;
use super::open::carried;
use super::*;

mod cursor;
mod edit;
mod find;
mod keys;
mod missed;
mod navigate;
mod navigate_binding;
mod navigate_call;
mod navigate_field;
mod navigate_go;
mod navigate_python;
mod navigate_reason;
mod navigate_receiver;
mod navigate_syntax;
mod open;
mod picker;
mod project_search;
mod review;
mod stats;
mod symbols;
mod tree;
mod usages;

fn app(text: &str) -> App {
    // Edits get saved, so each test has a file of its own.
    let name = std::thread::current()
        .name()
        .unwrap_or("main")
        .replace("::", "-");
    let path = std::env::temp_dir().join(format!("merl-{name}.txt"));
    std::fs::write(&path, text).unwrap();
    let mut a = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(path, text.as_bytes()),
        None,
    );
    a.view_w = 20;
    a.view_h = 10;
    a
}
fn press(a: &mut App, code: KeyCode, m: KeyModifiers) -> bool {
    a.key(KeyEvent::new(code, m))
}
/// Two small files in a scratch directory, for the jump-history tests.
fn files_app(tag: &str) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-hist-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["a.rs", "b.rs"] {
        std::fs::write(dir.join(name), "x\n".repeat(40)).unwrap();
    }
    let app = App::new(
        dir.clone(),
        Tree::default(),
        Vec::new(),
        Buffer::empty(),
        None,
    );
    (dir, app)
}
/// A repository with a `feature` branch checked out: `src/a.rs` changed twice, `new`
/// added, `gone` deleted, `src/keep.rs` as it was; the app is in review mode on it.
fn review_app(tag: &str) -> (PathBuf, App) {
    review_app_with(tag, &[])
}
/// `extra`: files the branch adds on top of the usual five.
fn review_app_with(tag: &str, extra: &[(&str, &[u8])]) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
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
    std::fs::write(dir.join("src/a.rs"), "a\nb\nc\nd\ne\nf\n").unwrap();
    std::fs::write(dir.join("gone"), "x\ny\n").unwrap();
    std::fs::write(dir.join("tail"), "t1\nt2\nt3\n").unwrap();
    std::fs::write(dir.join("crlf.txt"), "one\r\ntwo\n").unwrap();
    std::fs::write(dir.join("src/keep.rs"), "k1\nk2\nk3\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    std::fs::write(dir.join("src/a.rs"), "a\nB\nc\nd\ne\nF\n").unwrap();
    std::fs::write(dir.join("new"), "n\n").unwrap();
    std::fs::remove_file(dir.join("gone")).unwrap();
    std::fs::write(dir.join("tail"), "t1\n").unwrap();
    std::fs::write(dir.join("crlf.txt"), "one\r\nTWO\n").unwrap();
    for (name, bytes) in extra {
        std::fs::write(dir.join(name), bytes).unwrap();
    }
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "work"]);
    let review = git::Review::open(&dir, None, None).unwrap();
    let (_, files) = crate::tree::build(&dir, false);
    let tree = crate::tree::from_files(
        &review
            .files
            .iter()
            .map(|f| f.path.clone())
            .collect::<Vec<_>>(),
    );
    let first = dir.join("new");
    let mut a = App::new(
        dir.clone(),
        tree,
        files,
        Buffer::load(&first).unwrap(),
        None,
    );
    a.start_review(review);
    (dir, a)
}
fn at(a: &App) -> (PathBuf, usize) {
    (a.buf.path.clone().unwrap(), a.line)
}
/// A scratch project on disk with its startup walk, for the definition and symbol tests.
fn project_app(tag: &str, files: &[(&str, &str)]) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-nav-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (name, text) in files {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let (tree, files) = crate::tree::build(&dir, false);
    let app = App::new(dir.clone(), tree, files, Buffer::empty(), None);
    (dir, app)
}
/// One of the #68 projects in `tests/fixtures`, with nothing outside it, so the standard
/// library of the machine running the tests has no say in what `d` offers.
fn fixture_app(name: &str) -> App {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let (tree, files) = crate::tree::build(&dir, false);
    let mut a = App::new(dir, tree, files, Buffer::empty(), None);
    // The fixtures are read as a plain build reads them, whatever `GOFLAGS` the tests run under.
    a.go_build = search::GoBuild::host();
    for kind in [Kind::Python, Kind::TsJs, Kind::Go] {
        a.external.insert(kind, (Vec::new(), Arc::new(Vec::new())));
    }
    a
}
/// Presses `d` on the last word of the first line of `file` that contains `code`, or starts
/// with it when `code` starts with `^`. A `|` in `code` is not part of the line: it stands
/// after the word to press `d` on, and what follows it only picks the line.
fn d_on(a: &mut App, file: &str, code: &str) {
    let (code, rest) = code.split_once('|').unwrap_or((code, ""));
    let whole = format!("{code}{rest}");
    let (code, whole) = (code, whole.as_str());
    let path = a.root.join(file);
    let text = std::fs::read_to_string(&path).unwrap();
    let (n, line) = text
        .lines()
        .enumerate()
        .find(|(_, l)| match whole.strip_prefix('^') {
            Some(start) => l.starts_with(start),
            None => l.contains(whole),
        })
        .unwrap_or_else(|| panic!("no `{whole}` in {file}"));
    let code = code.trim_start_matches('^');
    a.jump_to(&path, n + 1);
    a.col = line.find(whole.trim_start_matches('^')).unwrap()
        + code.rfind(|c: char| !is_word(c)).map_or(0, |i| i + 1);
    press(a, KeyCode::Char('d'), KeyModifiers::NONE);
}
/// The rows of the open picker, each split into its qualified name, its reason and the
/// `path:line` it points at.
fn definition_rows(a: &mut App) -> Vec<(String, String, String)> {
    let picker = a.picker.as_mut().expect("a picker");
    picker.settle();
    picker
        .window(50)
        .0
        .into_iter()
        .map(|r| {
            let at = r.item.code_at.unwrap();
            let head = &r.item.label[..at];
            let cols: Vec<&str> = head
                .split("  ")
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .collect();
            let [name, why, place] = cols[..] else {
                panic!("{head}");
            };
            (name.into(), why.into(), place.trim_end_matches(':').into())
        })
        .collect()
}
/// What `d` shows: the status line, and for a jump where it landed, for a picker its rows.
#[derive(Debug, PartialEq)]
enum Shown {
    Jump(String, String),
    Picker(String, Vec<(String, String, String)>),
}
fn shown(a: &mut App) -> Shown {
    match a.picker {
        Some(_) => {
            let rows = definition_rows(a);
            press(a, KeyCode::Esc, KeyModifiers::NONE);
            Shown::Picker(a.message.clone(), rows)
        }
        None => Shown::Jump(
            a.message.clone(),
            format!("{}:{}", a.rel_path(), a.line + 1),
        ),
    }
}
fn jump(status: &str, place: &str) -> Shown {
    Shown::Jump(status.into(), place.into())
}
fn picker(status: &str, rows: &[(&str, &str)]) -> Shown {
    let rows = rows
        .iter()
        .map(|(name, place)| (name.to_string(), "by name".to_string(), place.to_string()))
        .collect();
    Shown::Picker(status.into(), rows)
}
/// A standard library or dependency root on disk, with `files` in it, for the lookups
/// outside the project; removed by the caller.
fn external_root(tag: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("merl-root-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (name, text) in files {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}
/// Makes `roots` the only places outside the project `d` looks in for `kind`.
fn use_roots(a: &mut App, kind: Kind, roots: &[PathBuf]) {
    let files = search::external_files(kind, roots);
    a.external.insert(kind, (roots.to_vec(), Arc::new(files)));
}
fn typed(a: &mut App, text: &str) {
    for c in text.chars() {
        press(a, KeyCode::Char(c), KeyModifiers::NONE);
    }
}
fn find(a: &mut App, query: &str) {
    press(a, KeyCode::Char('/'), KeyModifiers::NONE);
    typed(a, query);
}
fn temp_file(name: &str, text: &str) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("f.py");
    std::fs::write(&path, text).unwrap();
    let mut a = App::new(
        dir,
        Tree::default(),
        Vec::new(),
        Buffer::load(&path).unwrap(),
        None,
    );
    a.view_w = 20;
    a.view_h = 10;
    (path, a)
}
