use super::*;

fn linked(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("merl-767-{tag}-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("AGENTS.md"),
        "# Agents\n\nThe public API is small.\n",
    )
    .unwrap();
    std::os::unix::fs::symlink("AGENTS.md", dir.join("CLAUDE.md")).unwrap();
    dir
}

fn listed(a: &mut App) -> Vec<PathBuf> {
    let picker = a.picker.as_mut().expect("a picker");
    picker.settle();
    let rows = picker.window(50).0;
    rows.into_iter().map(|r| r.item.path).collect()
}

fn searched(a: &mut App, query: &str) -> Vec<PathBuf> {
    (a.view_w, a.view_h) = (80, 20);
    press(a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(a, query);
    a.settle_search();
    let rows = listed(a);
    press(a, KeyCode::Esc, KeyModifiers::NONE);
    rows
}

#[cfg(unix)]
#[test]
fn s_and_u_list_a_linked_file_once_under_the_name_it_is_open_by_else_its_own() {
    let dir = linked("search");
    std::fs::create_dir_all(dir.join("hidden")).unwrap();
    std::fs::write(dir.join(".gitignore"), "hidden/\n").unwrap();
    std::fs::write(dir.join("hidden/notes.md"), "The public API, unlisted.\n").unwrap();
    std::os::unix::fs::symlink("hidden/notes.md", dir.join("notes.md")).unwrap();
    let (_, files) = crate::tree::build(&dir, false);
    let buf = Buffer::load(&dir.join("AGENTS.md")).unwrap();
    let mut a = App::new(dir.clone(), Tree::default(), files, buf, None);
    for open in ["AGENTS.md", "CLAUDE.md"] {
        a.jump_to(&dir.join(open), 3);
        let want = [PathBuf::from(open), PathBuf::from("notes.md")];
        assert_eq!(searched(&mut a, "public API"), want, "s in {open} (#767)");
        a.col = a.line_str().find("public").unwrap();
        press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
        assert_eq!(listed(&mut a), want, "u in {open} (#767)");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

fn git_in(dir: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}");
}

fn link_review(tag: &str) -> (PathBuf, App) {
    let dir = linked(tag);
    std::fs::remove_file(dir.join("CLAUDE.md")).unwrap();
    git_in(&dir, &["init", "-q", "-b", "main"]);
    git_in(&dir, &["config", "user.email", "t@t"]);
    git_in(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join("CLAUDE.md"), "# Claude\nold\n").unwrap();
    git_in(&dir, &["add", "."]);
    git_in(&dir, &["commit", "-q", "-m", "base"]);
    git_in(&dir, &["switch", "-q", "-c", "links"]);
    std::fs::remove_file(dir.join("CLAUDE.md")).unwrap();
    std::os::unix::fs::symlink("AGENTS.md", dir.join("CLAUDE.md")).unwrap();
    git_in(&dir, &["commit", "-qam", "link"]);
    std::os::unix::fs::symlink("AGENTS.md", dir.join("NEW.md")).unwrap();
    let review = git::Review::open(&dir, None, None).unwrap();
    let (_, files) = crate::tree::build(&dir, false);
    let buf = Buffer::load(&dir.join("CLAUDE.md")).unwrap();
    let mut a = App::new(dir.clone(), Tree::default(), files, buf, None);
    a.start_review(review);
    (dir, a)
}

fn shown(a: &App) -> (&[String], Option<&str>) {
    (&a.buf.lines, a.buf.readonly)
}

fn is_the_link(a: &App) -> bool {
    a.buf.lines == ["AGENTS.md"] && a.buf.readonly == Some("a link")
}

#[cfg(unix)]
#[test]
fn review_shows_a_file_turned_into_a_link_as_the_link_s_own_change() {
    let (dir, a) = link_review("review");
    assert!(
        is_the_link(&a),
        "as GitHub draws it (#767): {:?}",
        shown(&a)
    );
    assert_eq!(
        a.diff.ghosts.get(&0).map(Vec::as_slice),
        Some(&["# Claude".to_owned(), "old".to_owned()][..])
    );
    assert_eq!(a.diff.marks.get(&0), Some(&git::Mark::Added));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[cfg(unix)]
#[test]
fn a_reviewed_link_reads_as_a_link_when_opened_again_and_reloaded() {
    let (dir, mut a) = link_review("review-open");
    a.jump_to(&dir.join("AGENTS.md"), 1);
    a.jump_to(&dir.join("CLAUDE.md"), 1);
    assert!(is_the_link(&a), "opened: {:?}", shown(&a));
    assert!(a.reload(true));
    assert!(is_the_link(&a), "reloaded: {:?}", shown(&a));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[cfg(unix)]
#[test]
fn s_from_a_reviewed_link_lists_the_lines_under_the_file_it_leads_to() {
    let (dir, mut a) = link_review("review-s");
    assert_eq!(searched(&mut a, "public API"), [PathBuf::from("AGENTS.md")]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[cfg(unix)]
#[test]
fn a_viewed_link_stays_viewed_when_the_file_it_leads_to_changes() {
    let (dir, mut a) = link_review("review-viewed");
    a.tree.reveal(Path::new("CLAUDE.md"));
    press(&mut a, KeyCode::Char('m'), KeyModifiers::NONE);
    assert!(a.viewed.contains_key(Path::new("CLAUDE.md")));
    std::fs::write(dir.join("AGENTS.md"), "# Agents, rewritten\n").unwrap();
    let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    a.review_refreshed(fresh);
    assert!(a.viewed.contains_key(Path::new("CLAUDE.md")));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[cfg(unix)]
#[test]
fn an_untracked_link_reads_as_its_file_until_git_adds_it() {
    let (dir, mut a) = link_review("review-untracked");
    a.jump_to(&dir.join("NEW.md"), 1);
    assert_eq!(
        (a.buf.lines[0].as_str(), a.buf.readonly),
        ("# Agents", None),
        "git has no blob of it to show yet"
    );
    git_in(&dir, &["add", "NEW.md"]);
    let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    a.review_refreshed(fresh);
    assert!(is_the_link(&a), "added: {:?}", shown(&a));
    std::fs::remove_dir_all(&dir).unwrap();
}
