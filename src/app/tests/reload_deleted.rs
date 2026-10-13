use super::*;
use TextLine::Deleted;

fn reviewed(tag: &str, base: &str, branch: &str) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-reload-{tag}-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).unwrap();
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
    std::fs::write(dir.join("a.txt"), base).unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    std::fs::write(dir.join("a.txt"), branch).unwrap();
    git(&["commit", "-q", "-am", "work"]);
    let review = git::Review::open(&dir, None, None).unwrap();
    let (_, files) = crate::tree::build(&dir, false);
    let mut a = App::new(dir.clone(), Tree::default(), files, Buffer::empty(), None);
    a.start_review(review);
    (dir, a)
}

fn reload_on(a: &mut App, dir: &Path, line: usize, ups: usize) -> (TextLine, TextLine) {
    a.jump_to(&dir.join("a.txt"), line);
    for _ in 0..ups {
        press(a, KeyCode::Up, KeyModifiers::NONE);
    }
    let before = a.at();
    a.reload(true);
    (before, a.at())
}

#[test]
fn a_reload_keeps_the_cursor_on_the_copy_of_a_deleted_line_at_its_own_key() {
    let (dir, mut a) = reviewed("nearest-key", "a\nx\nb\nc\nx\nd\n", "a\nb\nc\nd\n");
    let (before, after) = reload_on(&mut a, &dir, 4, 1);
    assert_eq!(before, Deleted(3, 0));
    assert_eq!(after, before, "the copy above `b` is another line");
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn a_reload_keeps_the_cursor_on_the_copy_of_a_deleted_line_at_its_own_index() {
    let (dir, mut a) = reviewed("same-key", "a\nx\nx\nb\n", "a\nb\n");
    let (before, after) = reload_on(&mut a, &dir, 2, 1);
    assert_eq!(before, Deleted(1, 1));
    assert_eq!(after, before, "the first of the two is another line");
    drop(std::fs::remove_dir_all(dir));
}
