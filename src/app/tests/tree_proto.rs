use super::*;
use crate::treesitter::syntax;
use std::fmt::Write as _;
use tree_sitter::{InputEdit, Point};

fn p50(mut v: Vec<Duration>) -> f64 {
    v.sort_unstable();
    v[v.len() / 2].as_secs_f64() * 1000.0
}

fn timed<T>(runs: &mut Vec<Duration>, f: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let out = f();
    runs.push(started.elapsed());
    out
}

#[test]
#[ignore = "times #1550's prototype; MERL_TREE_SPEED=a.ts,b.tsx"]
fn tree_speed() {
    let Ok(files) = std::env::var("MERL_TREE_SPEED") else {
        return;
    };
    for path in files.split(',') {
        let text = std::fs::read_to_string(path).unwrap();
        let ext = Path::new(path).extension().unwrap().to_str().unwrap();
        let rules = syntax::rules(ext).unwrap();
        let mut parser = rules.parser().unwrap();
        let rows = text.lines().count();
        let mid = rows / 2;
        let at = text.lines().take(mid).map(|l| l.len() + 1).sum::<usize>();
        let mut edited = text.clone();
        edited.insert(at, 'x');
        let line = text.lines().nth(mid).unwrap_or("");
        let name: String = line
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let col = line.len() - line.trim_start().len();
        let (mut parse, mut edit, mut folds, mut scope) = (vec![], vec![], vec![], vec![]);
        let mut error = false;
        for _ in 0..21 {
            let tree = timed(&mut parse, || parser.parse(&text, None).unwrap());
            error = tree.root_node().has_error();
            let mut old = tree.clone();
            timed(&mut edit, || {
                old.edit(&InputEdit {
                    start_byte: at,
                    old_end_byte: at,
                    new_end_byte: at + 1,
                    start_position: Point::new(mid, 0),
                    old_end_position: Point::new(mid, 0),
                    new_end_position: Point::new(mid, 1),
                });
                parser.parse(&edited, Some(&old)).unwrap()
            });
            timed(&mut folds, || rules.folds(&tree, &text));
            timed(&mut scope, || {
                rules.bound_rows(&tree, &text, Point::new(mid, col), &name).len()
            });
        }
        println!(
            "{path}\t{rows} lines\terror {error}\tparse {:.2}\treparse after a key {:.2}\tfolds query {:.2}\tscope of `{name}` {:.2}",
            p50(parse),
            p50(edit),
            p50(folds),
            p50(scope)
        );
    }
}

#[test]
#[ignore = "#1550's prototype on broken files; BREAK_IN=folds/ts.tsv BREAK_CACHE=dir BREAK_TMP=dir"]
fn tree_breaks() {
    let (Ok(input), Ok(cache), Ok(tmp)) = (
        std::env::var("BREAK_IN"),
        std::env::var("BREAK_CACHE"),
        std::env::var("BREAK_TMP"),
    ) else {
        return;
    };
    let mut out = String::new();
    let mut tally: HashMap<(&str, &str, &str), usize> = HashMap::new();
    for row in std::fs::read_to_string(input).unwrap().lines() {
        if row.starts_with('#') {
            continue;
        }
        let (path, folds) = row.split_once('\t').unwrap();
        let file = Path::new(&cache).join(path);
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let mut lines: Vec<&str> = text.lines().collect();
        let m = lines.len() / 2;
        lines.insert(m, "const broken = (");
        let ext = file.extension().unwrap().to_str().unwrap();
        let broken = Path::new(&tmp).join(format!("broken.{ext}"));
        std::fs::write(&broken, lines.join("\n") + "\n").unwrap();
        let shift = |l: usize| if l > m { l + 1 } else { l };
        for (rules, off) in [("tree", false), ("lines", true)] {
            syntax::FORCE_OFF.set(off);
            let buf = Buffer::load(&broken).unwrap();
            let mut a = App::new(PathBuf::from("/"), Tree::default(), Vec::new(), buf, None);
            for fold in folds.split_whitespace() {
                let (h, e) = fold.split_once(':').unwrap();
                let (h, e): (usize, usize) = (h.parse().unwrap(), e.parse().unwrap());
                let side = if e <= m {
                    "above"
                } else if h > m {
                    "below"
                } else {
                    "around"
                };
                let (h, e) = (shift(h), shift(e));
                a.collapsed.clear();
                a.go((h - 1, 0));
                press(&mut a, KeyCode::Char('f'), KeyModifiers::NONE);
                let exact = a.collapsed.first() == Some(&(h - 1, e - 1));
                *tally
                    .entry((rules, side, if exact { "exact" } else { "not" }))
                    .or_default() += 1;
            }
        }
        syntax::FORCE_OFF.set(false);
    }
    for rules in ["tree", "lines"] {
        for side in ["above", "below", "around"] {
            let x = tally.get(&(rules, side, "exact")).copied().unwrap_or(0);
            let n = x + tally.get(&(rules, side, "not")).copied().unwrap_or(0);
            _ = writeln!(out, "{rules}\t{side}\t{x}/{n}");
        }
    }
    println!("{out}");
}
