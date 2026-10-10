use super::*;

fn file_app(tag: &str, name: &str, text: &str) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-data-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    let mut a = App::new(
        dir.clone(),
        Tree::default(),
        Vec::new(),
        Buffer::load(&path).unwrap(),
        None,
    );
    (a.view_w, a.view_h) = (40, 8);
    a.parse.wait = Duration::ZERO;
    (dir, a)
}

fn p(a: &mut App) {
    press(a, KeyCode::Char('p'), KeyModifiers::NONE);
}

fn row(a: &App) -> String {
    let p = a.preview.as_ref().expect("a preview");
    p.doc.rows[p.row].text.clone()
}

#[test]
fn p_on_a_csv_parses_it_in_the_background_then_shows_the_table() {
    let (dir, mut a) = file_app("csv", "t.csv", "id,name\n1,ann\n2,bob\n");
    a.line = 2;
    p(&mut a);
    assert!(!a.previewing(), "the source stays until the parse is done");
    assert!(a.preview_pending());
    a.settle_parse();
    assert!(a.previewing());
    assert_eq!(a.message, "");
    assert_eq!(row(&a), "│ 2  │ bob  │", "on the cursor's record");
    p(&mut a);
    assert!(!a.previewing());
    p(&mut a);
    assert!(a.previewing(), "the same text needs no second parse");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_the_parser_cannot_read_whole_stays_source_and_says_why() {
    let (dir, mut a) = file_app("bad", "log.jsonl", "{\"a\":1}\nnot json\n");
    p(&mut a);
    a.settle_parse();
    assert!(!a.previewing());
    assert_eq!(a.message, "p: line 2 is not JSON");
    p(&mut a);
    assert!(a.preview_pending(), "`p` again tries again");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn p_while_parsing_stays_on_the_source() {
    let (dir, mut a) = file_app("cancel", "t.tsv", "a\tb\n");
    p(&mut a);
    p(&mut a);
    assert!(!a.preview_pending());
    a.settle_parse();
    assert!(!a.previewing());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_rewritten_file_is_parsed_again_and_shown_only_when_whole() {
    let (dir, mut a) = file_app("rewrite", "t.csv", "a,b\n1,2\n");
    p(&mut a);
    a.settle_parse();
    assert!(a.previewing());
    std::fs::write(dir.join("t.csv"), "a,b\n1,2,3\n").unwrap();
    a.reload(true);
    a.preview_sync();
    assert!(
        !a.previewing(),
        "the old table is not shown for the new text"
    );
    a.settle_parse();
    assert!(!a.previewing());
    assert_eq!(a.message, "p: line 2 has 3 fields, not 2");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_mermaid_file_opens_at_once_and_other_files_have_no_preview() {
    let (dir, mut a) = file_app("mmd", "flow.mmd", "graph TD\n  A --> B\n");
    p(&mut a);
    assert!(a.previewing());
    let (dir2, mut b) = file_app("none", "notes.xyz", "x\n");
    p(&mut b);
    assert!(!b.previewing());
    assert_eq!(b.message, "no preview for .xyz");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

#[test]
fn a_csv_parsed_before_leaves_a_markdown_preview_alone() {
    let (dir, mut a) = file_app("then-md", "t.csv", "a,b\n1,2\n");
    p(&mut a);
    a.settle_parse();
    std::fs::write(dir.join("PLAN.md"), "# Plan\n").unwrap();
    a.jump_to(&dir.join("PLAN.md"), 1);
    p(&mut a);
    assert!(a.previewing());
    assert_eq!(row(&a), "Plan");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn edit_mode_while_parsing_never_gets_the_preview_over_it() {
    let (dir, mut a) = file_app("edit", "t.csv", "a,b\n1,2\n");
    p(&mut a);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(a.mode, Mode::Edit);
    a.settle_parse();
    assert!(!a.previewing(), "the parse that lands opens nothing");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_parse_of_a_file_left_behind_touches_nothing_where_the_user_is() {
    let (dir, mut a) = file_app("left", "t.csv", "a,b\n1,2,3\n");
    p(&mut a);
    std::fs::write(dir.join("PLAN.md"), "# Plan\n").unwrap();
    a.jump_to(&dir.join("PLAN.md"), 1);
    p(&mut a);
    a.settle_parse();
    assert!(a.previewing(), "the Markdown preview stays");
    assert_ne!(a.message, "p: line 2 has 3 fields, not 2");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_previewed_file_returned_to_is_parsed_again_and_opens() {
    let (dir, mut a) = file_app("back", "a.csv", "a,b\n1,2\n");
    p(&mut a);
    a.settle_parse();
    std::fs::write(dir.join("b.csv"), "x\ny\n").unwrap();
    a.jump_to(&dir.join("b.csv"), 1);
    p(&mut a);
    a.settle_parse();
    a.jump_to(&dir.join("a.csv"), 1);
    assert!(!a.previewing());
    assert!(a.tick(), "the parse starts again");
    assert!(a.preview_pending());
    a.settle_parse();
    assert!(a.previewing());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_rewritten_file_never_draws_the_old_table() {
    let (dir, mut a) = file_app("frame", "t.csv", "a,b\n1,2\n");
    p(&mut a);
    a.settle_parse();
    std::fs::write(dir.join("t.csv"), "a,b\n3,4\n").unwrap();
    a.reload(true);
    a.preview_fresh();
    assert!(!a.previewing());
    assert!(a.preview_pending());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_with_no_extension_is_named_without_its_directories() {
    let (dir, mut a) = file_app("noext", "Makefile", "all:\n");
    p(&mut a);
    assert_eq!(a.message, "no preview for Makefile");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_small_file_opens_on_the_press_with_no_parsing_shown() {
    let (dir, mut a) = file_app("small", "t.csv", "a,b\n1,2\n");
    a.parse.wait = Duration::from_secs(5);
    p(&mut a);
    assert!(a.previewing());
    let _ = std::fs::remove_dir_all(&dir);
}
