use super::note_places::{
    DIM, LIVE, base, feature, feature_with, git, imported, open, placed, write,
};
use super::notes::{note, screen};
use super::*;

fn import7(mut l: Vec<String>) -> Vec<String> {
    let import = [
        "",
        "import (",
        "\t\"errors\"",
        "\t\"fmt\"",
        "\t\"io\"",
        "\t\"os\"",
        ")",
    ];
    l.splice(1..1, import.map(str::to_owned));
    l
}

fn noted(
    tag: &str,
    branch: fn(Vec<String>) -> Vec<String>,
    unsaved: fn(Vec<String>) -> Vec<String>,
    at: usize,
) -> (PathBuf, Vec<String>) {
    let (dir, lines) = feature_with(tag, "", branch);
    let lines = unsaved(lines);
    write(&dir, &lines);
    let mut a = open(&dir);
    a.jump_to(&dir.join("steps.go"), at);
    note(&mut a, "wrap this");
    screen(&mut a);
    (dir, lines)
}

fn same(l: Vec<String>) -> Vec<String> {
    l
}

fn agent_commits(dir: &Path, lines: &[String]) {
    write(dir, lines);
    git(dir, &["commit", "-qam", "agent"]);
}

fn reopened(dir: PathBuf) -> (Vec<(TextLine, bool, bool)>, String) {
    let mut b = open(&dir);
    let got = placed(&mut b);
    let shown = screen(&mut b);
    drop(std::fs::remove_dir_all(dir));
    (got, shown)
}

#[test]
fn moved7_a_note_moved_by_exactly_a_twin_s_gap_stays_live_on_its_line_after_a_reopen() {
    let (dir, lines) = noted("p-moved7", same, same, 14);
    let sha = std::process::Command::new("git")
        .arg("-C")
        .arg(&dir)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .unwrap();
    let sha = String::from_utf8_lossy(&sha.stdout).trim().to_owned();
    let text = std::fs::read_to_string(dir.join(".git/merl/notes/feature")).unwrap();
    assert_eq!(
        text,
        format!(
            "# merl notes \u{b7} feature \u{2190} main \u{b7} {sha}\n# The user's review notes, for you. Answer under a note with `agent:`, or delete its block when it is done.\n\nsteps.go:14\n> \t\treturn err\nwrap this\n"
        )
    );
    agent_commits(&dir, &import7(lines));
    let (got, s) = reopened(dir);
    assert_eq!(got, [(TextLine::File(20), LIVE, false)], "{s}");
}

#[test]
fn del7_a_note_whose_line_was_deleted_sits_dim_under_its_own_deleted_copy() {
    let (dir, mut lines) = noted("p-del7", same, same, 14);
    lines.splice(12..16, ["\treturn run(1)".to_owned()]);
    agent_commits(&dir, &import7(lines));
    let (got, s) = reopened(dir);
    assert_eq!(got, [(TextLine::Deleted(19, 1), DIM, false)], "{s}");
}

#[test]
fn delgrow_a_note_on_a_deleted_line_below_a_grown_block_sits_dim_under_its_copy() {
    let (dir, mut lines) = noted("p-delgrow", same, same, 28);
    lines.splice(26..30, ["\treturn run(3)".to_owned()]);
    lines.splice(12..12, (1..=3).map(|i| format!("\tlog({i})")));
    agent_commits(&dir, &imported(lines));
    let (got, s) = reopened(dir);
    assert_eq!(got, [(TextLine::Deleted(33, 1), DIM, false)], "{s}");
}

#[test]
fn movefn_a_note_in_a_function_moved_to_the_end_sits_dim_under_its_moved_away_line() {
    let (dir, mut lines) = noted("p-movefn", same, same, 21);
    let step2: Vec<String> = lines.drain(17..24).collect();
    lines.extend(step2);
    agent_commits(&dir, &imported(lines));
    let (got, s) = reopened(dir);
    assert_eq!(got, [(TextLine::Deleted(21, 3), DIM, false)], "{s}");
}

fn step6(mut l: Vec<String>) -> Vec<String> {
    l.push(String::new());
    l.push("func step6() error {".to_owned());
    for i in [6, 7] {
        l.push(format!("\tif err := run({i}); err != nil {{"));
        l.push("\t\treturn err".to_owned());
        l.push("\t}".to_owned());
    }
    l.push("\treturn nil".to_owned());
    l.push("}".to_owned());
    l
}

#[test]
fn case2_a_note_on_the_second_of_two_twins_in_an_added_function_follows_it_down() {
    let (dir, mut lines) = noted("p-case2", step6, same, 52);
    lines.splice(47..47, (1..=3).map(|i| format!("\tlog({i})")));
    agent_commits(&dir, &lines);
    let (got, s) = reopened(dir);
    assert_eq!(got, [(TextLine::File(54), LIVE, false)], "{s}");
}

#[test]
fn dirty_a_note_written_on_the_agent_s_unsaved_import_stays_on_its_line() {
    let (dir, _) = noted("p-dirty", same, import7, 21);
    let (got, s) = reopened(dir);
    assert_eq!(got, [(TextLine::File(20), LIVE, false)], "{s}");
}

#[test]
fn dirtycommit_the_agent_commits_the_import_as_is_and_the_note_stays() {
    let (dir, lines) = noted("p-dirtycommit", same, import7, 21);
    agent_commits(&dir, &lines);
    let (got, s) = reopened(dir);
    assert_eq!(got, [(TextLine::File(20), LIVE, false)], "{s}");
}

#[test]
fn uncommitted_the_agent_commits_its_import_then_adds_two_lines_and_the_note_follows() {
    let (dir, mut lines) = noted("p-uncommitted", same, import7, 21);
    git(&dir, &["commit", "-qam", "agent: import"]);
    lines.splice(
        1..1,
        [String::new(), "// Package steps runs steps.".to_owned()],
    );
    agent_commits(&dir, &lines);
    let (got, s) = reopened(dir);
    assert_eq!(got, [(TextLine::File(22), LIVE, false)], "{s}");
}

#[test]
fn later_a_second_note_after_the_agent_s_commit_keeps_the_first_on_its_line() {
    let (dir, lines) = noted("p-later", same, same, 14);
    agent_commits(&dir, &import7(lines));
    let mut a = open(&dir);
    a.jump_to(&dir.join("steps.go"), 10);
    note(&mut a, "make run real");
    screen(&mut a);
    let (got, s) = reopened(dir);
    let want = [
        (TextLine::File(20), LIVE, false),
        (TextLine::File(9), LIVE, false),
    ];
    assert_eq!(got, want, "{s}");
}

#[test]
fn amend_the_agent_amends_the_noted_commit_and_the_note_stays_on_its_line() {
    let (dir, lines) = noted("p-amend", same, same, 14);
    write(&dir, &import7(lines));
    git(
        &dir,
        &[
            "commit",
            "-qa",
            "--amend",
            "-m",
            "step5 runs 50, with imports",
        ],
    );
    let (got, s) = reopened(dir);
    assert_eq!(got, [(TextLine::File(20), LIVE, false)], "{s}");
}

#[test]
fn rebase_main_grows_above_and_the_branch_is_rebased_and_the_note_follows() {
    let (dir, _) = noted("p-rebase", same, same, 14);
    git(&dir, &["checkout", "-q", "main"]);
    let mut main = base();
    main.splice(
        1..1,
        [String::new(), "// one".to_owned(), "// two".to_owned()],
    );
    agent_commits(&dir, &main);
    git(&dir, &["checkout", "-q", "feature"]);
    git(&dir, &["rebase", "-q", "main"]);
    let (got, s) = reopened(dir);
    assert_eq!(got, [(TextLine::File(16), LIVE, false)], "{s}");
}

#[test]
fn gc_a_note_whose_version_git_dropped_is_placed_from_its_line_and_quote() {
    let (dir, lines) = noted("p-gc", same, same, 14);
    write(&dir, &import7(lines));
    git(
        &dir,
        &[
            "commit",
            "-qa",
            "--amend",
            "-m",
            "step5 runs 50, with imports",
        ],
    );
    git(&dir, &["reflog", "expire", "--expire=now", "--all"]);
    git(&dir, &["gc", "-q", "--prune=now"]);
    let (got, s) = reopened(dir);
    assert_eq!(got, [(TextLine::File(13), LIVE, false)], "{s}");
}

#[test]
fn older_a_note_with_no_fingerprint_is_placed_from_its_line_and_quote() {
    let (dir, lines) = feature("p-older", "steps.go:14\n> \t\treturn err\nwrap this\n");
    agent_commits(&dir, &import7(lines));
    let (got, s) = reopened(dir);
    assert_eq!(got, [(TextLine::File(13), LIVE, false)], "{s}");
}
