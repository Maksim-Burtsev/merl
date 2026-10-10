//! Time budgets (#314): how long merl takes on a large repository, against a budget per measure.
//!
//! `#[ignore]`d, and meaningful only in release mode with the machine quiet (`sysctl -n
//! vm.loadavg` under ~8):
//!
//! ```sh
//! cargo test --release time_budgets -- --ignored --nocapture
//! ```
//!
//! The repository is generated once into the temp directory (`merl-budgets-v1`, ~35 s) and
//! reused. Each measure runs [`REPS`] times; its median is held against its budget, and the test
//! fails when one is over. `d` needs a real project with its dependencies installed:
//! paperless-ngx from the `d` bench's cache (#308), which `tools/d-bench/run --project
//! paperless-ngx` fills (`$D_BENCH_CACHE`, default `~/.cache/merl-d-bench`), or the checkout
//! `MERL_BUDGET_D_PROJECT` names; without it that measure is skipped with a note. The numbers
//! below were measured on the same clone (6a5d06e, `uv sync`) made by the bench's prototype.
//!
//! The table printed has a `last` column: the numbers of the last release, from
//! `tests/budgets.tsv`. `MERL_BUDGETS_SAVE=1` writes this run's medians there; the release
//! gate (`.claude/skills/smoke-test/SKILL.md`) runs it so, and the release PR commits the file.
//!
//! Budgets: three times the medians of a quiet M4 MacBook (see the constants). The factor
//! covers the load: other sessions' builds stretch timings two- to threefold (AGENTS.md,
//! `## Changing d`); a measure that doubles on a quiet machine still shows in the `last` column.

use super::*;
use ratatui::{Terminal, backend::TestBackend};
use std::time::Instant;

/// Runs per measure, after one that warms the disk cache and is dropped; the median counts.
const REPS: usize = 3;
/// Bumped when the generated repository changes, so a stale one is not reused.
const REPO: &str = "merl-budgets-v1";
/// Code files in the generated repository, in five languages.
const FILES: usize = 30_000;
/// Files the `feature` branch changes, for `--review`.
const CHANGED: usize = 1_000;

// Budgets, in milliseconds: three times today's median (in brackets, three runs at load 6-7 on
// 2026-09-28), rounded up, and never under 50 ms, where a scheduler hiccup outweighs the work.
/// `merl` in the repository: the walk, the project watch list, the first frame [78 ms].
const STARTUP: u64 = 250;
/// `merl big/generated.py`: startup on the 10 MB file, to the first frame [105 ms].
const STARTUP_LARGE: u64 = 350;
/// `o`, a query typed, the list filtered [5 ms].
const O_FILTER: u64 = 50;
/// `s` on a word in every file: the grep stops at the hit cap, the first page of hits [7 ms].
const S_FIRST: u64 = 50;
/// `s` on a word in one file: the grep reads the whole project before it answers [620 ms; with
/// the disk cache cold, 1.5-2.5 s].
const S_DONE: u64 = 2_000;
/// `D`: every declaration of the project read, the list shown [150 ms].
const D_OPEN: u64 = 500;
/// A query typed in `D`: past the cap it greps the project again [940 ms].
const D_QUERY: u64 = 3_000;
/// The first `d` on a fresh merl in paperless-ngx, `info` of `logger.info`: a picker of 115
/// declarations read out of the project and its dependencies [1,390 ms at load 25; `title=` in a
/// test, its cursor until #315, took 1,170 ms, the #308 bench saw 3.5 s with the disk cache
/// cold, and a run at load 20 took 5.2 s].
const D_FIRST: u64 = 4_000;
/// The slowest of the next seven `d` presses there [860 ms].
const D_NEXT: u64 = 3_000;
/// `--review` of the branch with [`CHANGED`] files, to the first frame [310 ms].
const REVIEW: u64 = 1_000;

const D_FINDS_NOTHING: usize = 1;

/// `d` cursors in paperless-ngx (`file`, 1-based line, byte column), from the #308 bench's
/// slowest Python presses and three quick ones.
const D_CURSORS: &[(&str, usize, usize)] = &[
    // The first press reads the dependencies cold. `title=` in test_workflows.py was the first
    // cursor until #315 made a named argument answer `argument label` without a search.
    ("src/paperless/parsers/registry.py", 287, 19),
    // The one `d` here that finds nothing, today (`returned_account1`): it times the search that
    // ends in a miss. Every other press must jump or open a picker, or it timed nothing.
    ("src/paperless_mail/tests/test_api.py", 176, 8),
    (
        "src/documents/tests/search/test_migration_fulltext_query_field_prefixes.py",
        117,
        27,
    ),
    // `Document` of `Document.objects.create(`, over the `title=` line.
    ("src/documents/tests/test_workflows.py", 3067, 14),
    ("src/documents/tests/test_file_handling.py", 313, 35),
    ("src/paperless/parsers/tesseract.py", 516, 32),
    ("src/documents/filters.py", 672, 26),
    ("src/paperless_ai/base_model.py", 260, 23),
];

#[test]
#[ignore]
fn time_budgets() {
    let root = generated();
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let draw = |a: &mut App| {
        let mut t = Terminal::new(TestBackend::new(185, 55)).unwrap();
        t.draw(|f| crate::ui::draw(f, a, &theme)).unwrap();
    };
    // As `main` starts: `review`, the changed files, takes the tree's place.
    let start = |file: Option<&Path>, review: Option<&[PathBuf]>| {
        let (mut tree, files) = crate::tree::build(&root, false);
        let _project = crate::live::Project::new(&root, false, &tree);
        let ignored = tree.ignored_files();
        if let Some(paths) = review {
            tree = crate::tree::from_listing(paths);
        }
        let buf = file.map_or_else(Buffer::empty, |p| Buffer::load(p).unwrap());
        let mut a = App::new(root.clone(), tree, files, buf, None);
        a.ignored = ignored;
        a
    };
    let mut rows: Vec<(&str, u64, Vec<f64>)> = Vec::new();

    rows.push((
        "startup",
        STARTUP,
        timed(|| {
            let t = Instant::now();
            draw(&mut start(None, None));
            t.elapsed()
        }),
    ));
    let big = root.join("big/generated.py");
    rows.push((
        "startup on a 10 MB file",
        STARTUP_LARGE,
        timed(|| {
            let t = Instant::now();
            draw(&mut start(Some(&big), None));
            t.elapsed()
        }),
    ));

    let mut a = start(None, None);
    rows.push((
        "o filter",
        O_FILTER,
        timed(|| {
            let t = Instant::now();
            press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
            typed(&mut a, "item04242");
            let p = a.picker.as_mut().expect("the o list");
            p.settle();
            let found = p.window(50).0.len();
            let took = t.elapsed();
            assert!(found > 0, "o finds item04242");
            press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
            took
        }),
    ));
    for (name, budget, query, cut) in [
        ("s first hits", S_FIRST, "value", true),
        ("s done", S_DONE, NEEDLE, false),
    ] {
        rows.push((
            name,
            budget,
            timed(|| {
                let t = Instant::now();
                press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
                typed(&mut a, query);
                a.settle_search();
                let took = t.elapsed();
                let hits = a.picker.as_ref().expect("the s list").counts().1 as usize;
                assert_eq!(hits >= search::MAX_HITS, cut, "s {query}: {hits} hits");
                press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
                took
            }),
        ));
    }
    let mut query = Vec::new();
    rows.push((
        "D open",
        D_OPEN,
        timed(|| {
            let t = Instant::now();
            press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
            let opened = t.elapsed();
            let t = Instant::now();
            typed(&mut a, "handle_4242");
            a.settle_search();
            let p = a.picker.as_mut().expect("the D list");
            p.settle();
            assert!(p.counts().0 > 0, "D finds handle_4242");
            query.push(t.elapsed().as_secs_f64() * 1000.0);
            press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
            opened
        }),
    ));
    rows.push(("D query", D_QUERY, query.split_off(1)));

    let mut two_modes = Vec::new();
    match paperless() {
        Some(project) => {
            let (first, next) = d_presses(&project);
            two_modes = [("d first", &first), ("d next", &next)]
                .into_iter()
                .filter(|(_, s)| !one_mode(s))
                .map(|(name, s)| format!("{name} {s:.0?}"))
                .collect();
            rows.push(("d first, paperless-ngx", D_FIRST, first));
            rows.push(("d next, slowest", D_NEXT, next));
        }
        None => println!(
            "d: skipped, no paperless-ngx: run `tools/d-bench/run --project paperless-ngx` \
             or set MERL_BUDGET_D_PROJECT"
        ),
    }

    rows.push((
        "review of 1,000 files",
        REVIEW,
        timed(|| {
            let t = Instant::now();
            let r = git::Review::open(&root, None, Some("main")).unwrap();
            let first = r.first_file(&root, false).expect("a changed file");
            let paths: Vec<PathBuf> = r.files.iter().map(|f| f.path.clone()).collect();
            assert_eq!(paths.len(), CHANGED);
            let mut a = start(Some(&first), Some(&paths));
            a.start_review(r);
            draw(&mut a);
            t.elapsed()
        }),
    ));

    report(&rows);
    assert!(
        two_modes.is_empty(),
        "d on a fresh App takes one of two times (#640): {}",
        two_modes.join(", ")
    );
}

fn one_mode(samples: &[f64]) -> bool {
    let max = samples.iter().copied().fold(0.0, f64::max);
    let min = samples.iter().copied().fold(f64::MAX, f64::min);
    max <= 2.0 * min
}

/// paperless-ngx with its dependencies: `MERL_BUDGET_D_PROJECT`, else the `d` bench's cache.
fn paperless() -> Option<PathBuf> {
    let cache = || match std::env::var_os("D_BENCH_CACHE") {
        Some(c) => Some(PathBuf::from(c)),
        None => std::env::var_os("HOME").map(|h| Path::new(&h).join(".cache/merl-d-bench")),
    };
    let dir = match std::env::var_os("MERL_BUDGET_D_PROJECT") {
        Some(p) => PathBuf::from(p),
        None => cache()?.join("paperless-ngx"),
    };
    dir.join(D_CURSORS[0].0).exists().then_some(dir)
}

fn timed(mut f: impl FnMut() -> std::time::Duration) -> Vec<f64> {
    f();
    (0..REPS).map(|_| f().as_secs_f64() * 1000.0).collect()
}

fn median(samples: &[f64]) -> f64 {
    let mut s = samples.to_vec();
    s.sort_by(f64::total_cmp);
    s[s.len() / 2]
}

/// Per run, a fresh merl in `project`: the first `d` press, and the slowest of the others.
fn d_presses(project: &Path) -> (Vec<f64>, Vec<f64>) {
    let (mut first, mut next) = (Vec::new(), Vec::new());
    for _ in 0..=REPS {
        let (tree, files) = crate::tree::build(project, false);
        let mut a = App::new(project.to_path_buf(), tree, files, Buffer::empty(), None);
        let mut ms = Vec::new();
        for (i, (file, line, col)) in D_CURSORS.iter().enumerate() {
            a.picker = None;
            a.mode = Mode::Normal;
            a.jump_to(&project.join(file), *line);
            a.col = *col;
            let before = (a.buf.path.clone(), a.line);
            let t = Instant::now();
            press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
            ms.push(t.elapsed().as_secs_f64() * 1000.0);
            // A fast "no definition" (dependencies not installed, a cursor off its word) is no
            // measure of `d`.
            assert!(
                i == D_FINDS_NOTHING
                    || a.picker.is_some()
                    || (a.buf.path.clone(), a.line) != before,
                "d found nothing at {file}:{line}:{col}: {}",
                a.message
            );
        }
        first.push(ms[0]);
        next.push(ms[1..].iter().copied().fold(0.0, f64::max));
    }
    // The first run warms the disk cache, as in `timed`.
    (first.split_off(1), next.split_off(1))
}

fn report(rows: &[(&str, u64, Vec<f64>)]) {
    let saved = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/budgets.tsv");
    let last: HashMap<String, String> = std::fs::read_to_string(&saved)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_once('\t'))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let load = std::process::Command::new("sysctl")
        .args(["-n", "vm.loadavg"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    println!("\ntime budgets, load {load}, ms, median of {REPS}\n");
    println!(
        "{:<26} {:>8} {:>8} {:>8}  samples",
        "measure", "now", "last", "budget"
    );
    let mut over = Vec::new();
    let mut tsv = String::from("# measure\tmedian ms: the last release's `time_budgets` (#314)\n");
    for (name, budget, samples) in rows {
        let m = median(samples);
        let all: Vec<String> = samples.iter().map(|s| format!("{s:.0}")).collect();
        let last = last.get(*name).map_or("-", String::as_str);
        let flag = if m > *budget as f64 { "  OVER" } else { "" };
        println!(
            "{name:<26} {m:>8.0} {last:>8} {budget:>8}  {}{flag}",
            all.join(" ")
        );
        tsv.push_str(&format!("{name}\t{m:.0}\n"));
        if !flag.is_empty() {
            over.push(format!("{name} {m:.0} ms > {budget} ms"));
        }
    }
    if std::env::var_os("MERL_BUDGETS_SAVE").is_some() {
        std::fs::write(&saved, tsv).unwrap();
        println!("\nsaved to {}", saved.display());
    }
    assert!(over.is_empty(), "over budget: {}", over.join(", "));
}

/// The word `s` finds in one file only.
const NEEDLE: &str = "zebra_quartz_needle";

/// The generated repository, made on the first run and kept: [`FILES`] code files in five
/// languages under `svcNN/mNN/`, a 40-level deep directory, a 10 MB Python file, a 2 MB
/// one-line bundle; committed on `main`, with `feature` checked out changing [`CHANGED`] files.
fn generated() -> PathBuf {
    let dir = std::env::temp_dir().join(REPO);
    let done = dir.join(".git/merl-budgets-done");
    if done.exists() {
        return dir;
    }
    let _ = std::fs::remove_dir_all(&dir);
    let write = |rel: &str, text: &str| {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    };
    for i in 0..FILES {
        write(&code_path(i), &code(i, false));
    }
    let deep: String = (1..=40).map(|l| format!("l{l}/")).collect();
    write(
        &format!("deep/{deep}leaf.rs"),
        &format!("pub fn {NEEDLE}() {{}}\n"),
    );
    let big: String = (0..200_000)
        .map(|n| format!("def generated_{n}(value):\n    return value * {n}  # a generated row\n"))
        .collect();
    write("big/generated.py", &big);
    let bundle: String = (0..40_000)
        .map(|n| format!("function b{n}(value){{return value+{n}}};"))
        .collect();
    write("big/bundle.min.js", &bundle);
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["-c", "user.email=t@t", "-c", "user.name=t"])
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}: {out:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    for i in (0..FILES).step_by(FILES / CHANGED) {
        write(&code_path(i), &code(i, true));
    }
    git(&["commit", "-q", "-am", "work"]);
    std::fs::write(done, "").unwrap();
    dir
}

fn code_path(i: usize) -> String {
    let ext = ["rs", "py", "ts", "go", "md"][i % 5];
    format!("svc{:02}/m{:02}/item{i:05}.{ext}", i / 1000, i / 100 % 10)
}

/// ~40 lines of `i`'s language: two declarations named after it and filler with `value`;
/// `changed`, the `feature` branch's version, rewrites one line and adds two.
fn code(i: usize, changed: bool) -> String {
    let (decl, call) = match i % 5 {
        0 => (
            format!("pub struct Item{i} {{ value: u32 }}\npub fn handle_{i}(value: u32) -> u32 {{"),
            "    let value = value + 1;",
        ),
        1 => (
            format!("class Item{i}:\n    pass\n\ndef handle_{i}(value):"),
            "    value = value + 1",
        ),
        2 => (
            format!("export class Item{i} {{}}\nexport function handle_{i}(value: number) {{"),
            "  value = value + 1;",
        ),
        3 => (
            format!("type Item{i} struct{{ value int }}\nfunc handle_{i}(value int) int {{"),
            "\tvalue = value + 1",
        ),
        _ => (
            format!("# Item {i}\n\nThe handle_{i} notes on value."),
            "The value grows by one.",
        ),
    };
    let mut s = decl + "\n";
    for n in 0..36 {
        if changed && n == 10 {
            s.push_str("    // changed on the feature branch\n");
        } else {
            s.push_str(&format!("{call} // step {n} of item {i}\n"));
        }
    }
    if changed {
        s.push_str("// added\n// added\n");
    }
    s
}

#[test]
fn one_mode_takes_the_fast_runs_and_not_the_slow_one_beside_them() {
    assert!(one_mode(&[666.0, 758.0, 616.0]));
    assert!(!one_mode(&[4420.0, 3042.0, 1544.0]));
}
