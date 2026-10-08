use std::panic::{AssertUnwindSafe, catch_unwind};

use unicode_segmentation::UnicodeSegmentation;

use super::*;

/// Lines no fixture holds: non-ASCII names, a combining mark, emoji built of several code points,
/// a tab, and constructs left unfinished, down to an open bracket as the file's last byte.
const ODD: &str = "café.load(x)\n\
naïve = Straße(ñ, ü)\n\
cafe\u{301} = e\u{301}tude.me\u{301}thode(1)\n\
👍🏽 = \"🇫🇷\" + 👨‍👩‍👧.größe\n\
def 名前(引数): return 引数.値\n\
func (r *Répo) Méthode() { return r.ñ }\n\
\tx := new(\n\
s = \"open string\n\
let t = `open template ${ü\n\
class Ω extends Δ { λ(): Ж { return this.ψ } }\n\
@ivär = :sÿm\n\
$ßar->bäz();\n\
$x = new ©Ns\\Bar();\n\
void nam\u{301}(int x) {\n\
x = (";

/// The files [`ODD`] is written into: one per kind, so every kind's rules read it.
const ODD_FILES: [&str; 37] = [
    "odd.py",
    "odd.go",
    "odd.rs",
    "odd.ts",
    "odd.java",
    "odd.rb",
    "odd.c",
    "odd.cs",
    "odd.swift",
    "odd.php",
    "odd.lua",
    "odd.ex",
    "odd.zig",
    "odd.proto",
    "odd.sh",
    "odd.sql",
    "Makefile",
    "odd.tf",
    "odd.yaml",
    "odd.md",
    "odd.graphql",
    "Dockerfile",
    "odd.nix",
    "odd.groovy",
    "odd.hs",
    "odd.ml",
    "odd.fs",
    "odd.jl",
    "odd.R",
    "odd.pm",
    "odd.gd",
    "odd.sol",
    "odd.clj",
    "odd.el",
    "odd.rkt",
    "odd.lisp",
    "odd.bzl",
];

/// Panics found and not fixed yet: `(project/file, line from 1, byte column, key, issue)`.
const KNOWN: &[(&str, usize, usize, char, u32)] = &[];

#[test]
#[ignore = "half an hour in debug; CI runs it in release"]
fn no_panic_on_d_u_or_shift_d_anywhere() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let odd: Vec<(&str, &str)> = ODD_FILES.iter().map(|f| (*f, ODD)).collect();
    let (dir, _) = project_app("no-panic", &odd);
    let projects: Vec<(String, PathBuf)> = std::fs::read_dir(&root)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
        .chain([("odd".to_owned(), dir.clone())])
        .collect();
    let mut jobs: Vec<(u64, &str, &Path, PathBuf)> = Vec::new();
    for (name, path) in &projects {
        let (_, files) = crate::tree::build(path, false);
        let size = files
            .iter()
            .filter_map(|f| std::fs::metadata(path.join(f)).ok())
            .map(|m| m.len())
            .sum();
        jobs.extend(
            files
                .into_iter()
                .map(|f| (size, name.as_str(), path.as_path(), f)),
        );
    }
    jobs.sort_by_key(|j| j.0);
    let (shard, shards) = shard();
    let jobs: Vec<_> = jobs
        .into_iter()
        .enumerate()
        .filter(|(i, _)| i % shards == shard)
        .map(|(_, j)| j)
        .collect();
    assert!(!jobs.is_empty(), "shard {shard}/{shards} holds no file");
    let jobs = std::sync::Mutex::new(jobs);
    let fails = std::sync::Mutex::new(Vec::new());
    let threads = super::test_threads();
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    // Its own statement, so the lock is let go before the sweep.
                    let next = jobs.lock().unwrap().pop();
                    let Some((_, name, path, file)) = next else {
                        break;
                    };
                    let new = || {
                        let (tree, files) = crate::tree::build(path, false);
                        let mut a = App::new(path.to_owned(), tree, files, Buffer::empty(), None);
                        a.go_build = search::GoBuild::host();
                        a.no_external();
                        a
                    };
                    let found = sweep(name, &new, &file);
                    fails.lock().unwrap().extend(found);
                }
            });
        }
    });
    let _ = std::fs::remove_dir_all(&dir);
    let fails = fails.into_inner().unwrap();
    assert!(
        fails.is_empty(),
        "{} panics:\n{}",
        fails.len(),
        fails.join("\n")
    );
}

fn shard() -> (usize, usize) {
    let Ok(spec) = std::env::var("MERL_NO_PANIC_SHARD") else {
        return (0, 1);
    };
    let parsed = spec
        .split_once('/')
        .and_then(|(i, n)| Some((i.parse().ok()?, n.parse().ok()?)))
        .filter(|&(i, n): &(usize, usize)| i < n);
    parsed.unwrap_or_else(|| panic!("MERL_NO_PANIC_SHARD={spec}: want I/N with I below N"))
}

/// Presses the keys everywhere in `file` of the project `new` makes, and returns each panic that
/// [`KNOWN`] does not list, as `project/file:line:col key: message`.
fn sweep(name: &str, new: &dyn Fn() -> App, file: &Path) -> Vec<String> {
    let mut fails = Vec::new();
    let mut a = new();
    let path = a.root.join(file);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return fails;
    };
    let place = format!("{name}/{}", file.display());
    let mut cols = vec![(0, None)];
    for (n, line) in text.lines().enumerate() {
        cols.extend(
            line.grapheme_indices(true)
                .map(|(c, _)| c)
                .chain([line.len()])
                .map(|c| (n, Some(c))),
        );
    }
    a.jump_to(&path, 0);
    for (line, col) in cols {
        let keys: &[char] = match col {
            Some(_) => &['d', 'u'],
            None => &['D'],
        };
        let col = col.unwrap_or(0);
        for &key in keys {
            let pressed = catch_unwind(AssertUnwindSafe(|| {
                a.go((line, col));
                press(&mut a, KeyCode::Char(key), KeyModifiers::NONE);
            }));
            let Err(e) = pressed else {
                // Back where the sweep is, with nothing open over the code.
                a.picker = None;
                a.mode = Mode::Normal;
                if a.buf.path.as_deref() != Some(&path) {
                    a.jump_to(&path, line);
                }
                continue;
            };
            let known = KNOWN
                .iter()
                .find(|k| k.0 == place && k.1 == line + 1 && k.2 == col && k.3 == key);
            if known.is_none() {
                let msg = e
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| e.downcast_ref::<&str>().copied())
                    .unwrap_or("?");
                fails.push(format!("{place}:{}:{col} {key}: {msg}", line + 1));
            }
            // A panic can leave the app half-changed: start over on a fresh one.
            a = new();
            a.jump_to(&path, line);
        }
    }
    fails
}
