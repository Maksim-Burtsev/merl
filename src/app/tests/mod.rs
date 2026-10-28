#![expect(
    clippy::format_collect,
    reason = "a fixture is built one format! per line; the allocations cost a test nothing"
)]
use super::open::carried;
use super::resolution::resolution;
use super::*;

mod android;
mod annotated;
mod batch_tools;
mod budgets;
mod cmake;
mod collapse;
mod cursor;
mod d_bench;
mod definition_rules;
mod edit;
mod edit_fuzz;
mod find;
mod fold_bench;
mod grow;
mod haskell;
mod julia;
mod keys;
mod limits;
mod links;
mod lisp;
mod missed;
mod ml;
mod navigate;
mod navigate_binding;
mod navigate_call;
mod navigate_csharp;
mod navigate_field;
mod navigate_go;
mod navigate_python;
mod navigate_python_deps;
mod navigate_reason;
mod navigate_receiver;
mod navigate_ruby;
mod navigate_rust;
mod navigate_syntax;
mod navigate_ts_deps;
mod nix;
mod no_panic;
mod notes;
mod open;
mod outside;
mod perl;
mod picker;
mod preview;
mod preview_data;
mod project_search;
mod r;
mod razor;
mod reload_deleted;
mod review;
mod review_deleted;
mod review_folder;
mod smoke;
mod starlark;
mod stats;
mod symbols;
mod tree;
mod tree_sitter_on;
mod usages;

fn test_threads() -> usize {
    std::env::var("RUST_TEST_THREADS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
}
fn app(text: &str) -> App {
    let name = std::thread::current()
        .name()
        .unwrap_or("main")
        .replace("::", "-");
    let path = std::env::temp_dir().join(format!("merl-{name}.txt"));
    std::fs::write(&path, text).unwrap();
    let mut a = App::new(
        std::env::temp_dir(),
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
fn files_app(tag: &str) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-hist-{}-{tag}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
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
fn review_app(tag: &str) -> (PathBuf, App) {
    review_app_with(tag, &[])
}
fn review_app_with(tag: &str, extra: &[(&str, &[u8])]) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-{tag}-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
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
    let a = review_start(&dir, None);
    (dir, a)
}
fn review_app_with_lock(tag: &str) -> (PathBuf, App) {
    let (dir, _) = review_app(tag);
    let git = |args: &[&str]| {
        let mut cmd = std::process::Command::new("git");
        let out = cmd.arg("-C").arg(&dir).args(args).output().unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    git(&["switch", "-q", "main"]);
    std::fs::write(dir.join("poetry.lock"), lock_lines(&[])).unwrap();
    git(&["add", "poetry.lock"]);
    git(&["commit", "-q", "-m", "lock"]);
    git(&["switch", "-q", "feature"]);
    git(&["merge", "-q", "main", "-m", "merge"]);
    std::fs::write(dir.join("poetry.lock"), lock_lines(&[1, 4, 7])).unwrap();
    git(&["commit", "-qam", "bump"]);
    let a = review_start(&dir, None);
    (dir, a)
}
fn lock_lines(up: &[usize]) -> String {
    (0..9)
        .map(|i| {
            if up.contains(&i) {
                format!("L{i}\n")
            } else {
                format!("l{i}\n")
            }
        })
        .collect()
}
fn review_start(dir: &Path, base: Option<&str>) -> App {
    let review = git::Review::open(dir, None, base).unwrap();
    let (_, files) = crate::tree::build(dir, false);
    let tree = crate::tree::from_listing(
        &review
            .files
            .iter()
            .map(|f| f.path.clone())
            .collect::<Vec<_>>(),
    );
    let first = dir.join("new");
    let mut a = App::new(
        dir.to_path_buf(),
        tree,
        files,
        Buffer::load(&first).unwrap(),
        None,
    );
    a.start_review(review);
    a
}
fn at(a: &App) -> (PathBuf, usize) {
    (a.buf.path.clone().unwrap(), a.line)
}
fn project_app(tag: &str, files: &[(&str, &str)]) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-nav-{}-{tag}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    for (name, text) in files {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let (tree, files) = crate::tree::build(&dir, false);
    let app = App::new(dir.clone(), tree, files, Buffer::empty(), None);
    (dir, app)
}
fn fixture_app(name: &str) -> App {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let (tree, files) = crate::tree::build(&dir, false);
    let mut a = App::new(dir, tree, files, Buffer::empty(), None);
    a.go_build = search::GoBuild::host();
    a.no_external();
    a
}
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
fn definition_rows(a: &mut App) -> Vec<(String, String, String)> {
    let picker = a.picker.as_mut().expect("a picker");
    picker.settle();
    picker
        .window(50)
        .0
        .into_iter()
        .map(|r| {
            let at = r.item.code_at.unwrap();
            let head = r.item.label.get(..at).unwrap();
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
#[derive(Debug, PartialEq)]
enum Shown {
    Jump(String, String),
    Picker(String, Vec<(String, String, String)>),
}
fn shown(a: &mut App) -> Shown {
    if a.picker.is_some() {
        let rows = definition_rows(a);
        press(a, KeyCode::Esc, KeyModifiers::NONE);
        Shown::Picker(a.message.clone(), rows)
    } else {
        let path = a.buf.path.as_deref().expect("a file");
        let place = path.strip_prefix(&a.root).unwrap_or(path);
        Shown::Jump(
            a.message.clone(),
            format!("{}:{}", place.display(), a.line + 1),
        )
    }
}
fn jump(status: &str, place: &str) -> Shown {
    Shown::Jump(status.into(), place.into())
}
fn picker(status: &str, rows: &[(&str, &str)]) -> Shown {
    let rows = rows
        .iter()
        .map(|(name, place)| (name.to_string(), "by name".to_owned(), place.to_string()))
        .collect();
    Shown::Picker(status.into(), rows)
}
fn external_root(tag: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("merl-root-{}-{tag}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    for (name, text) in files {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}
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
