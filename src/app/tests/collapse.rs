//! `f`: folding a function or a block into its first line.

use super::*;
use crate::app::collapse::block_end;

fn lines(text: &str) -> Vec<String> {
    text.lines().map(String::from).collect()
}

/// An app on `text` in a file of the test's own with the extension `ext`, so the file's kind
/// picks its declarations.
fn app_as(ext: &str, text: &str) -> App {
    let mut a = app(text);
    let path = a.buf.path.clone().unwrap().with_extension(ext);
    std::fs::write(&path, text).unwrap();
    a.buf = Buffer::from_bytes(path, text.as_bytes());
    a
}

fn key(a: &mut App, code: KeyCode) {
    press(a, code, KeyModifiers::NONE);
}

const PY: &str = "\
class Store:
    def read(self):
        if self.path:
            return 1
        return 0

    def write(self):
        pass
";

#[test]
fn a_block_ends_where_its_indentation_does() {
    let rust = lines("fn f(\n    a: u8,\n) -> u8 {\n    a\n}\nfn g() {}\n");
    assert_eq!(
        block_end(&rust, 0),
        Some(4),
        "the wrapped signature and the `}}`"
    );
    let py = lines(PY);
    assert_eq!(
        block_end(&py, 1),
        Some(4),
        "not the blank line before `write`"
    );
    assert_eq!(block_end(&py, 0), Some(7));
    assert_eq!(block_end(&py, 3), None, "a line that opens nothing");
    let branches = lines("if a {\n    b\n} else {\n    c\n}\n");
    assert_eq!(
        block_end(&branches, 0),
        Some(1),
        "`}} else {{` stays in view"
    );
    let allman = lines("void f()\n{\n    x;\n}\n");
    assert_eq!(block_end(&allman, 0), Some(3));
    let ruby = lines("def f\n  1\nend\n");
    assert_eq!(block_end(&ruby, 0), Some(2));
}

#[test]
fn f_folds_the_function_on_its_line_and_unfolds_it_again() {
    let mut a = app_as("py", PY);
    a.go((1, 4));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(1, 4)]);
    // Down steps over the hidden body, Up comes back to the folded line.
    key(&mut a, KeyCode::Down);
    assert_eq!(a.line, 5);
    key(&mut a, KeyCode::Up);
    assert_eq!(a.line, 1);
    key(&mut a, KeyCode::Char('f'));
    assert!(a.collapsed.is_empty());
}

#[test]
fn f_inside_a_body_folds_the_function_around_it() {
    let mut a = app_as("py", PY);
    a.go((3, 12));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(1, 4)], "`read`, not the `if`");
    assert_eq!(a.line, 1);
    // A call wrapped onto the next lines opens no block of its own for `f`.
    let mut a = app_as("py", "def send(self):\n    self.post(\n        1,\n    )\n");
    a.go((1, 4));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(0, 3)]);
}

#[test]
fn a_jump_into_a_fold_opens_it() {
    let mut a = app_as("py", PY);
    a.go((1, 0));
    key(&mut a, KeyCode::Char('f'));
    key(&mut a, KeyCode::Char(':'));
    key(&mut a, KeyCode::Char('4'));
    key(&mut a, KeyCode::Enter);
    assert_eq!(a.line, 3);
    assert!(a.collapsed.is_empty());
}

#[test]
fn a_fold_moves_with_the_lines_typed_above_it() {
    let mut a = app_as("py", PY);
    a.go((6, 0));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(6, 7)]);
    a.go((0, 0));
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Enter);
    assert_eq!(a.collapsed, vec![(7, 8)]);
    // Typing on the folded line itself opens it.
    a.go((7, 0));
    key(&mut a, KeyCode::Char('x'));
    assert!(a.collapsed.is_empty());
}

#[test]
fn a_fold_stays_with_its_file() {
    let (dir, mut a) = files_app("fold");
    std::fs::write(dir.join("a.rs"), "fn f() {\n    1\n}\n").unwrap();
    a.jump_to(&dir.join("a.rs"), 1);
    key(&mut a, KeyCode::Char('f'));
    a.jump_to(&dir.join("b.rs"), 1);
    assert!(a.collapsed.is_empty());
    a.jump_to(&dir.join("a.rs"), 0);
    assert_eq!(a.collapsed, vec![(0, 2)]);
}

#[test]
fn a_reload_keeps_a_fold_whose_line_is_still_there() {
    let mut a = app_as("py", PY);
    a.go((6, 0));
    key(&mut a, KeyCode::Char('f'));
    let path = a.buf.path.clone().unwrap();
    std::fs::write(&path, format!("import os\n\n{PY}")).unwrap();
    a.reload(false);
    assert_eq!(a.collapsed, vec![(8, 9)]);
    std::fs::write(&path, PY.replace("def write", "def save")).unwrap();
    a.reload(false);
    assert!(a.collapsed.is_empty());
}
