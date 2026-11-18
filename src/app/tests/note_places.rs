use super::notes::{note, screen};
use super::*;

fn base() -> Vec<String> {
    let mut v = vec![
        "package steps".to_owned(),
        String::new(),
        "func run(n int) error { return nil }".to_owned(),
    ];
    for i in 0..6 {
        v.push(String::new());
        v.push(format!("func step{i}() error {{"));
        v.push(format!("\tif err := run({i}); err != nil {{"));
        v.push("\t\treturn err".to_owned());
        v.push("\t}".to_owned());
        v.push("\treturn nil".to_owned());
        v.push("}".to_owned());
    }
    v
}

fn git(dir: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}");
}

fn write(dir: &Path, lines: &[String]) {
    std::fs::write(dir.join("steps.go"), lines.join("\n") + "\n").unwrap();
}

fn feature(tag: &str, notes: &str) -> (PathBuf, Vec<String>) {
    let dir = std::env::temp_dir().join(format!("merl-places-{tag}-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    let mut lines = base();
    write(&dir, &lines);
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-qm", "base"]);
    git(&dir, &["switch", "-qc", "feature"]);
    lines[40] = "\tif err := run(50); err != nil {".to_owned();
    write(&dir, &lines);
    std::fs::write(dir.join("new"), "n\n").unwrap();
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-qm", "step5 runs 50"]);
    if !notes.is_empty() {
        std::fs::create_dir_all(dir.join(".git/merl/notes")).unwrap();
        std::fs::write(
            dir.join(".git/merl/notes/feature"),
            format!("# merl notes\n\n{notes}"),
        )
        .unwrap();
    }
    (dir, lines)
}

fn agent(case: &str, mut l: Vec<String>) -> Vec<String> {
    let fix = |s: &str| format!("\t\treturn fmt.Errorf(\"{s}: %w\", err)");
    match case {
        "changed" | "block" => l[13] = fix("step1"),
        "deleted" => {
            l.splice(12..16, ["\treturn run(1)".to_owned()]);
        }
        "fmt-changed" => (l[13], l[34]) = (fix("step1"), fix("step4")),
        "fmt-other" => (l[13], l[34]) = (fix("wrapped"), fix("wrapped")),
        "added" => l[40] = "\tif err := run(500); err != nil {".to_owned(),
        _ => {}
    }
    let import = ["", "import (", "\t\"fmt\"", ")"].map(str::to_owned);
    l.splice(1..1, import);
    l
}

fn open(dir: &Path) -> App {
    let mut a = review_start(dir, None);
    a.show_tree = false;
    a.jump_to(&dir.join("steps.go"), 16);
    screen(&mut a);
    a
}

type Place = (TextLine, bool, bool);

fn placed(a: &mut App) -> Vec<Place> {
    screen(a);
    let mut v: Vec<_> = (a.notes.here.iter())
        .map(|h| (h.idx, (h.at, h.stale, h.top.is_some())))
        .collect();
    v.sort_by_key(|h| h.0);
    v.into_iter().map(|h| h.1).collect()
}

fn both_ways(case: &str, notes: &str) -> (Vec<Place>, String) {
    let (dir, lines) = feature(case, notes);
    let mut a = open(&dir);
    let after = agent(case, lines);
    write(&dir, &after);
    git(&dir, &["commit", "-qam", "agent"]);
    a.reload(false);
    let live = placed(&mut a);
    let shown = screen(&mut a);
    let mut b = open(&dir);
    let reopened = placed(&mut b);
    drop(std::fs::remove_dir_all(dir));
    assert_eq!(
        live, reopened,
        "with merl open, then after a reopen: {shown}"
    );
    (live, shown)
}

const ON_STEP1: &str = "steps.go:14\n> \t\treturn err\nwrap this\n";
const LIVE: bool = false;
const DIM: bool = true;

#[test]
fn row_1_two_notes_of_one_text_stay_on_their_own_twin_lines() {
    let (dir, _) = feature("twins", "");
    let mut a = open(&dir);
    a.jump_to(&dir.join("steps.go"), 7);
    note(&mut a, "wrap");
    a.notes.focus = None;
    a.jump_to(&dir.join("steps.go"), 21);
    note(&mut a, "wrap");
    let want = [
        (TextLine::File(6), LIVE, false),
        (TextLine::File(20), LIVE, false),
    ];
    assert_eq!(placed(&mut a), want, "{}", screen(&mut a));
    let mut b = open(&dir);
    assert_eq!(placed(&mut b), want, "after a reopen: {}", screen(&mut b));
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn row_2_a_note_whose_line_moved_by_a_twin_s_gap_stays_live_on_it() {
    let (got, s) = both_ways("moved", ON_STEP1);
    assert_eq!(got, [(TextLine::File(17), LIVE, false)], "{s}");
}

#[test]
fn row_3_a_note_whose_line_changed_sits_dim_under_its_deleted_copy() {
    let (got, s) = both_ways("changed", ON_STEP1);
    assert_eq!(got, [(TextLine::Deleted(17, 0), DIM, false)], "{s}");
    assert!(s.contains("wrap this (changed)"), "{s}");
}

#[test]
fn row_4_a_note_whose_line_was_deleted_sits_dim_in_the_red_block() {
    let (got, s) = both_ways("deleted", ON_STEP1);
    assert_eq!(got, [(TextLine::Deleted(16, 1), DIM, false)], "{s}");
}

#[test]
fn row_5_a_formatter_write_that_changed_the_noted_line_leaves_it_dim_under_its_copy() {
    let (got, s) = both_ways("fmt-changed", ON_STEP1);
    assert_eq!(got, [(TextLine::Deleted(17, 0), DIM, false)], "{s}");
}

#[test]
fn row_6_a_formatter_write_around_an_untouched_note_leaves_it_live_on_its_line() {
    let (got, s) = both_ways("fmt-other", "steps.go:21\n> \t\treturn err\nwrap this\n");
    assert_eq!(got, [(TextLine::File(24), LIVE, false)], "{s}");
}

#[test]
fn row_7_a_block_note_whose_block_changed_waits_above_the_file() {
    let block = "steps.go:13-15\n> \tif err := run(1); err != nil {\n> \t\treturn err\n> \t}\nwrap this block\n";
    let (got, s) = both_ways("block", block);
    assert_eq!(got, [(TextLine::File(0), DIM, true)], "{s}");
}

#[test]
fn row_8_a_note_on_a_line_the_branch_added_and_changed_again_waits_above_the_file() {
    let added = "steps.go:41\n> \tif err := run(50); err != nil {\nwhy 50\n";
    let (got, s) = both_ways("added", added);
    assert_eq!(got, [(TextLine::File(0), DIM, true)], "{s}");
}

#[test]
fn row_9_a_note_on_a_unique_line_follows_it_down() {
    let plain = "steps.go:3\n> func run(n int) error { return nil }\nmake run real\n";
    let (got, s) = both_ways("plain", plain);
    assert_eq!(got, [(TextLine::File(6), LIVE, false)], "{s}");
}

fn agent_notes(dir: &Path, notes: &str) {
    let path = dir.join(".git/merl/notes/feature");
    std::fs::write(&path, format!("# merl notes\n\n{notes}")).unwrap();
    let later = std::time::SystemTime::now() + Duration::from_secs(5);
    (std::fs::File::options().write(true).open(&path))
        .and_then(|f| f.set_modified(later))
        .unwrap();
}

#[test]
fn a_new_note_with_a_gone_note_s_text_lands_where_it_was_written() {
    let (dir, mut lines) = feature("regone", ON_STEP1);
    let mut a = open(&dir);
    lines.splice(0..0, ["// one".to_owned(), "// two".to_owned()]);
    write(&dir, &lines);
    a.reload(false);
    assert_eq!(placed(&mut a), [(TextLine::File(15), LIVE, false)]);
    agent_notes(&dir, &ON_STEP1.replace("steps.go:14", "steps.go:9"));
    let got = placed(&mut a);
    let mut b = open(&dir);
    assert_eq!(placed(&mut b), got, "after a reopen");
    drop(std::fs::remove_dir_all(dir));
    assert_eq!(got, [(TextLine::File(8), LIVE, false)]);
}

#[test]
fn with_merl_open_a_note_follows_its_line_down_by_exactly_a_twin_s_gap() {
    let (dir, mut lines) = feature("gap", ON_STEP1);
    let mut a = open(&dir);
    lines.splice(0..0, (0..7).map(|i| format!("// {i}")));
    write(&dir, &lines);
    a.reload(false);
    let got = placed(&mut a);
    drop(std::fs::remove_dir_all(dir));
    assert_eq!(got, [(TextLine::File(20), LIVE, false)]);
}

#[test]
fn a_line_typed_above_a_twin_note_keeps_it_live_on_its_line_before_and_after_the_save() {
    let (dir, _) = feature("typed", ON_STEP1);
    let mut a = open(&dir);
    a.jump_to(&dir.join("steps.go"), 10);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert!(a.dirty);
    let typed = placed(&mut a);
    a.flush();
    let saved = placed(&mut a);
    drop(std::fs::remove_dir_all(dir));
    assert_eq!(typed, [(TextLine::File(14), LIVE, false)]);
    assert_eq!(saved, typed);
}
