use super::*;

#[test]
fn a_reload_carries_lines_past_one_rewritten_region_only() {
    let old: Vec<String> = (0..10).map(|i| i.to_string()).collect();
    let mut new = old.clone();
    new.insert(6, "y".into());
    new.insert(2, "x".into());
    assert_eq!(new[9], "7");
    assert_eq!(carried(&old, &new, 7), 9);
    assert_eq!(new[5], "4");
    assert_eq!(carried(&old, &new, 4), 4);
}

#[test]
fn the_files_outside_the_project_are_walked_once_per_session() {
    let (dir, mut a) = project_app("outside-once", &[("app.py", "import vendored\n")]);
    let root = external_root("outside-once", &[("vendored.py", "X = 1\n")]);
    use_roots(&mut a, Kind::Python, std::slice::from_ref(&root));
    assert_eq!(a.external_files(Kind::Python).len(), 1);
    std::fs::write(root.join("installed.py"), "Y = 1\n").unwrap();
    assert_eq!(a.external_files(Kind::Python).len(), 1);
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_hunk_left_is_its_index_so_a_hunk_written_above_it_takes_its_place() {
    let (dir, mut a) = review_app("limits-hunk-left");
    let src = dir.join("src/a.rs");
    a.jump_to(&src, 3);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(a.line_str(), "F");
    a.jump_to(&dir.join("src/keep.rs"), 2);
    std::fs::write(&src, "A\nB\nc\nD\ne\nF\n").unwrap();
    let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    a.review_refreshed(fresh);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!((at(&a), a.line_str()), ((src, 3), "D"));
    let _ = std::fs::remove_dir_all(dir);
}
