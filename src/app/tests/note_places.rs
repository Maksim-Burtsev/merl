use super::notes::{note, screen};
use super::*;
use crate::app::note_prints::escape;
use crate::app::notes::note_id;

pub(super) fn base() -> Vec<String> {
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

pub(super) fn git(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

pub(super) fn write(dir: &Path, lines: &[String]) {
    std::fs::write(dir.join("steps.go"), lines.join("\n") + "\n").unwrap();
}

pub(super) fn feature(tag: &str, notes: &str) -> (PathBuf, Vec<String>) {
    feature_with(tag, notes, |l| l)
}

pub(super) fn feature_with(
    tag: &str,
    notes: &str,
    branch: fn(Vec<String>) -> Vec<String>,
) -> (PathBuf, Vec<String>) {
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
    let lines = branch(lines);
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
        "changed" | "block" | "two-lines" => l[13] = fix("step1"),
        "deleted" => {
            l.splice(12..16, ["\treturn run(1)".to_owned()]);
        }
        "fmt-changed" => (l[13], l[34]) = (fix("step1"), fix("step4")),
        "fmt-other" => (l[13], l[34]) = (fix("wrapped"), fix("wrapped")),
        "added" => l[40] = "\tif err := run(500); err != nil {".to_owned(),
        "delfirst" => {
            l.remove(13);
        }
        _ => {}
    }
    let import = ["", "import (", "\t\"fmt\"", ")"].map(str::to_owned);
    l.splice(1..1, import);
    l
}

pub(super) fn imported(mut l: Vec<String>) -> Vec<String> {
    let import = ["", "import (", "\t\"fmt\"", ")"].map(str::to_owned);
    l.splice(1..1, import);
    l
}

fn started_on_steps(dir: &Path) -> App {
    let review = git::Review::open(dir, None, None).unwrap();
    let (tree, files) = crate::tree::build(dir, false);
    let buf = Buffer::load(&dir.join("steps.go")).unwrap();
    let mut a = App::new(dir.to_path_buf(), tree, files, buf, None);
    a.start_review(review);
    a.show_tree = false;
    screen(&mut a);
    a
}

pub(super) fn open(dir: &Path) -> App {
    let mut a = review_start(dir, None);
    a.show_tree = false;
    a.jump_to(&dir.join("steps.go"), 16);
    screen(&mut a);
    a
}

type Place = (TextLine, bool, bool);

pub(super) fn placed(a: &mut App) -> Vec<Place> {
    screen(a);
    let mut v: Vec<_> = (a.notes.here.iter())
        .map(|h| (h.idx, (h.at, h.stale, h.top.is_some())))
        .collect();
    v.sort_by_key(|h| h.0);
    v.into_iter().map(|h| h.1).collect()
}

fn both_ways(case: &str, notes: &str) -> (Vec<Place>, String) {
    let (from_quote, _) = one_way(case, notes, false);
    let (printed, shown) = one_way(case, notes, true);
    assert_eq!(printed, from_quote, "with a fingerprint: {shown}");
    (printed, shown)
}

fn one_way(case: &str, notes: &str, printed: bool) -> (Vec<Place>, String) {
    let (dir, lines) = feature(&format!("{case}-{printed}"), notes);
    if printed {
        let blob = git(&dir, &["hash-object", "-w", "steps.go"]);
        let prints: String = (crate::app::notes::parse(notes).iter())
            .map(|n| format!("{blob} {}\n", escape(&note_id(n))))
            .collect();
        std::fs::create_dir_all(dir.join(".git/merl/prints")).unwrap();
        std::fs::write(dir.join(".git/merl/prints/feature"), prints).unwrap();
    }
    let mut a = open(&dir);
    let answered = (a.notes.printed.values()).filter(|p| p.1.is_some()).count();
    let notes_n = notes.lines().filter(|l| l.starts_with("steps.go:")).count();
    assert_eq!(answered, if printed { notes_n } else { 0 }, "{case}");
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
pub(super) const LIVE: bool = false;
pub(super) const DIM: bool = true;

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

const TWO_LINES: &str =
    "steps.go:13-14\n> \tif err := run(1); err != nil {\n> \t\treturn err\nwrap these\n";

#[test]
fn a_two_line_note_whose_second_line_changed_names_the_lines_it_was_written_on() {
    let (got, s) = both_ways("two-lines", TWO_LINES);
    assert_eq!(got, [(TextLine::Deleted(17, 0), DIM, false)], "{s}");
    assert!(s.contains("lines 13 to 14  wrap these (changed)"), "{s}");
    let (dir, lines) = feature("two-lines-answered", TWO_LINES);
    agent_wrote(&dir, &agent("two-lines", lines));
    let mut a = open(&dir);
    a.notes.focus = Some(0);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "ok");
    let answering = screen(&mut a);
    agent_notes(&dir, &format!("{TWO_LINES}agent: wrapped\n"));
    let replied = screen(&mut a);
    drop(std::fs::remove_dir_all(dir));
    assert_eq!(a.mode, Mode::Note);
    assert!(
        answering.contains("lines 13 to 14  wrap theseok"),
        "{answering}"
    );
    assert!(
        replied.contains("lines 13 to 14  wrap theseok"),
        "{replied}"
    );
    assert!(!replied.contains(" to -"), "{replied}");
}

#[test]
fn a_two_line_note_typed_into_after_it_moved_keeps_the_numbers_it_is_drawn_on() {
    let (dir, lines) = feature("two-lines-typed", TWO_LINES);
    agent_wrote(&dir, &imported(lines));
    let mut a = open(&dir);
    assert_eq!(placed(&mut a), [(TextLine::File(17), LIVE, false)]);
    a.jump_to(&dir.join("steps.go"), 18);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    typed(&mut a, "x");
    let got = placed(&mut a);
    let s = screen(&mut a);
    drop(std::fs::remove_dir_all(dir));
    assert_eq!(got, [(TextLine::File(17), DIM, false)], "{s}");
    assert!(s.contains("lines 17 to 18  wrap these (changed)"), "{s}");
}

#[test]
fn a_note_written_on_a_deleted_line_and_the_next_is_not_named_by_its_one_stored_number() {
    let (dir, _) = feature_with("ghost-and-next", "", |mut l| {
        l.remove(13);
        l
    });
    let mut a = open(&dir);
    a.jump_to(&dir.join("steps.go"), 14);
    press(&mut a, KeyCode::Up, KeyModifiers::NONE);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    note(&mut a, "both");
    let stored = std::fs::read_to_string(dir.join(".git/merl/notes/feature")).unwrap();
    let s = screen(&mut a);
    drop(std::fs::remove_dir_all(dir));
    assert!(stored.contains("steps.go:14\n- "), "{stored}");
    assert!(s.contains("both"), "{s}");
    assert!(!s.contains("lines 14 to 14"), "{s}");
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

#[test]
fn a_block_note_whose_first_line_was_deleted_stays_dim_in_the_code_with_a_fingerprint() {
    let block = "steps.go:14-15\n> \t\treturn err\n> \t}\nwrap two\n";
    let (got, s) = both_ways("delfirst", block);
    assert!(matches!(got[..], [(_, DIM, false)]), "{got:?} {s}");
    assert!(s.contains("lines 14 to 15  wrap two (changed)"), "{s}");
}

#[test]
fn row_1_after_a_reopen_without_fingerprints_keeps_twins_on_their_own_lines() {
    let (dir, _) = feature("twins-unprinted", "");
    let mut a = open(&dir);
    a.jump_to(&dir.join("steps.go"), 7);
    note(&mut a, "wrap");
    a.notes.focus = None;
    a.jump_to(&dir.join("steps.go"), 21);
    note(&mut a, "wrap");
    std::fs::remove_file(dir.join(".git/merl/prints/feature")).unwrap();
    let mut b = open(&dir);
    let got = placed(&mut b);
    let printed = b.notes.printed.len();
    drop(std::fs::remove_dir_all(dir));
    assert_eq!(printed, 0);
    let want = [
        (TextLine::File(6), LIVE, false),
        (TextLine::File(20), LIVE, false),
    ];
    assert_eq!(got, want);
}

fn agent_notes(dir: &Path, notes: &str) {
    let path = dir.join(".git/merl/notes/feature");
    std::fs::create_dir_all(dir.join(".git/merl/notes")).unwrap();
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

fn agent_wrote(dir: &Path, lines: &[String]) {
    write(dir, lines);
    git(dir, &["commit", "-qam", "agent"]);
}

#[test]
fn a_note_in_a_block_the_branch_added_stays_live_when_the_block_grows_above_it() {
    let note = "steps.go:5\n> )\nclose it\n";
    let (dir, mut lines) = feature_with("block-grows", note, imported);
    let mut a = open(&dir);
    lines.insert(4, "\t\"os\"".to_owned());
    agent_wrote(&dir, &lines);
    a.reload(false);
    let live = placed(&mut a);
    let mut b = open(&dir);
    let reopened = placed(&mut b);
    drop(std::fs::remove_dir_all(dir));
    assert_eq!(live, [(TextLine::File(5), LIVE, false)]);
    assert_eq!(reopened, live);
}

#[test]
fn a_note_below_a_branch_block_that_grew_or_shrank_stays_live_on_its_line() {
    let note = "steps.go:18\n> \t\treturn err\nwrap this\n";
    for (tag, grow) in [("grew", true), ("shrank", false)] {
        let (dir, mut lines) = feature_with(tag, note, imported);
        if grow {
            lines.insert(4, "\t\"os\"".to_owned());
        } else {
            lines.remove(3);
        }
        let want = TextLine::File(if grow { 18 } else { 16 });
        let mut a = open(&dir);
        agent_wrote(&dir, &lines);
        a.reload(false);
        let live = placed(&mut a);
        let reopened = placed(&mut open(&dir));
        drop(std::fs::remove_dir_all(dir));
        assert_eq!(live, [(want, LIVE, false)], "{tag}");
        assert_eq!(reopened, live, "{tag}");
    }
}

#[test]
fn a_reopen_started_on_the_file_places_a_note_as_one_that_walks_to_it() {
    let (dir, mut lines) = feature("started", "");
    lines.splice(0..0, (0..7).map(|i| format!("// {i}")));
    agent_wrote(&dir, &lines);
    agent_notes(&dir, ON_STEP1);
    let mut a = started_on_steps(&dir);
    let started = placed(&mut a);
    let walked = placed(&mut open(&dir));
    drop(std::fs::remove_dir_all(dir));
    assert_eq!(started, [(TextLine::File(13), LIVE, false)]);
    assert_eq!(walked, started);
}

#[test]
fn a_deleted_twin_that_fits_as_well_loses_to_the_note_s_own_untouched_line() {
    let (dir, mut lines) = feature("deleted-twin", ON_STEP1);
    let mut a = open(&dir);
    lines[6] = "\t\treturn fmt.Errorf(\"step0: %w\", err)".to_owned();
    lines.splice(0..0, (0..7).map(|i| format!("// {i}")));
    agent_wrote(&dir, &lines);
    a.reload(false);
    let live = placed(&mut a);
    let reopened = placed(&mut open(&dir));
    drop(std::fs::remove_dir_all(dir));
    assert_eq!(live, [(TextLine::File(20), LIVE, false)]);
    assert_eq!(reopened, live);
}

#[test]
fn a_reply_or_a_dropped_note_while_the_buffer_is_unsaved_moves_no_other_note() {
    let first = "steps.go:7\n> \t\treturn err\nfirst\n";
    let second = "steps.go:21\n> \t\treturn err\nsecond\n";
    let (dir, _) = feature("unsaved-reply", &format!("{first}\n{second}"));
    let mut a = open(&dir);
    a.jump_to(&dir.join("steps.go"), 3);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert!(a.dirty);
    let typed = placed(&mut a);
    agent_notes(&dir, &format!("{first}agent: ok\n\n{second}"));
    let replied = placed(&mut a);
    agent_notes(&dir, second);
    let dropped = placed(&mut a);
    drop(std::fs::remove_dir_all(dir));
    let both = [
        (TextLine::File(7), LIVE, false),
        (TextLine::File(21), LIVE, false),
    ];
    assert_eq!(typed, both);
    assert_eq!(replied, both);
    assert_eq!(dropped, [(TextLine::File(21), LIVE, false)]);
}

#[test]
fn editing_a_note_that_followed_its_line_by_a_twin_s_gap_keeps_it_there() {
    let (dir, mut lines) = feature("edited", ON_STEP1);
    let mut a = open(&dir);
    lines.splice(0..0, (0..7).map(|i| format!("// {i}")));
    write(&dir, &lines);
    a.reload(false);
    assert_eq!(placed(&mut a), [(TextLine::File(20), LIVE, false)]);
    a.notes.focus = Some(0);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::End, KeyModifiers::NONE);
    typed(&mut a, " please");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    let got = placed(&mut a);
    let text = std::fs::read_to_string(dir.join(".git/merl/notes/feature")).unwrap();
    drop(std::fs::remove_dir_all(dir));
    assert!(text.contains("wrap this please"), "{text}");
    assert_eq!(got, [(TextLine::File(20), LIVE, false)]);
}
