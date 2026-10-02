use super::*;
use crate::app::collapse::block_end;

fn lines(text: &str) -> Vec<String> {
    text.lines().map(String::from).collect()
}

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
    let mut a = app_as("py", "def send(self):\n    self.post(\n        1,\n    )\n");
    a.go((1, 4));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(
        a.collapsed,
        vec![(1, 3)],
        "a call wrapped over lines folds itself"
    );
    let mut a = app_as("py", "def f(\n    self,\n) -> None:\n    pass\n");
    a.go((2, 0));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(
        a.collapsed,
        vec![(0, 3)],
        "the tail of a signature folds the function"
    );
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
    a.go((7, 0));
    key(&mut a, KeyCode::Char('x'));
    assert!(a.collapsed.is_empty());
}

#[test]
fn a_fold_stays_with_its_file() {
    let (dir, mut a) = files_app("fold");
    std::fs::write(dir.join("a.py"), "def f():\n    return 1\n").unwrap();
    std::fs::write(dir.join("b.py"), "x = 1\n").unwrap();
    a.jump_to(&dir.join("a.py"), 1);
    key(&mut a, KeyCode::Char('f'));
    a.jump_to(&dir.join("b.py"), 1);
    assert!(a.collapsed.is_empty());
    a.jump_to(&dir.join("a.py"), 0);
    assert_eq!(a.collapsed, vec![(0, 1)]);
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

#[test]
fn f_on_a_line_ending_in_a_colon_folds_that_block() {
    let mut a = app_as("py", PY);
    a.go((2, 8));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(2, 3)], "the `if`, not `read`");
}

#[test]
fn f_folds_python_and_says_so_in_other_languages() {
    let mut a = app_as("json", "{\n  \"a\": [\n    1\n  ]\n}\n");
    a.go((2, 4));
    key(&mut a, KeyCode::Char('f'));
    assert!(a.collapsed.is_empty());
    assert_eq!(a.message, "no fold rules for .json");
    let mut a = app_as("py", "x = [\n    1,\n]\n");
    a.go((1, 4));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(0, 2)], "a block outside any function");
    let mut a = app_as("py", "x = 1\ny = 2\n");
    key(&mut a, KeyCode::Char('f'));
    assert!(a.collapsed.is_empty());
    assert_eq!(a.message, "nothing to fold");
}

#[test]
fn the_arrows_step_over_a_fold_and_keep_it() {
    let mut a = app_as("py", PY);
    a.go((1, 0));
    key(&mut a, KeyCode::Char('f'));
    a.go((5, 0));
    key(&mut a, KeyCode::Left);
    assert_eq!((a.line, a.col), (1, PY.lines().nth(1).unwrap().len()));
    key(&mut a, KeyCode::Right);
    assert_eq!((a.line, a.col), (5, 0));
    assert_eq!(a.collapsed, vec![(1, 4)]);
}

#[test]
fn an_undo_puts_a_fold_back_with_its_lines() {
    let mut a = app_as("py", PY);
    a.go((6, 0));
    key(&mut a, KeyCode::Char('f'));
    a.go((0, 0));
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Enter);
    assert_eq!(a.collapsed, vec![(7, 8)]);
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(a.collapsed, vec![(6, 7)]);
}

#[test]
fn backspace_or_delete_into_a_fold_opens_it_and_changes_nothing() {
    let text = "def a():\n    x()\ny()\n";
    let mut a = app_as("py", text);
    key(&mut a, KeyCode::Char('f'));
    a.go((2, 0));
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Backspace);
    assert!(a.collapsed.is_empty());
    assert_eq!(a.buf.lines.join("\n") + "\n", text);
    key(&mut a, KeyCode::Esc);
    a.go((0, 0));
    key(&mut a, KeyCode::Char('f'));
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::End);
    key(&mut a, KeyCode::Delete);
    assert!(a.collapsed.is_empty());
    assert_eq!(a.buf.lines.join("\n") + "\n", text);
}

#[test]
fn a_line_indented_under_a_fold_leaves_it_folded() {
    let mut a = app_as("py", "def a():\n    x = 1\ny = 1\n");
    key(&mut a, KeyCode::Char('f'));
    a.go((2, 0));
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Char(' '));
    key(&mut a, KeyCode::Char(' '));
    assert_eq!(a.collapsed, vec![(0, 1)]);
}

#[test]
fn a_find_that_passes_through_a_fold_and_is_cancelled_leaves_it_folded() {
    let mut a = app_as("py", PY);
    a.go((1, 0));
    key(&mut a, KeyCode::Char('f'));
    a.go((0, 0));
    key(&mut a, KeyCode::Char('/'));
    for c in "return 1".chars() {
        key(&mut a, KeyCode::Char(c));
    }
    assert_eq!(a.line, 3, "the match shows, inside the fold");
    key(&mut a, KeyCode::Esc);
    assert_eq!((a.line, a.collapsed.clone()), (0, vec![(1, 4)]));
    key(&mut a, KeyCode::Char('/'));
    for c in "return 1".chars() {
        key(&mut a, KeyCode::Char(c));
    }
    key(&mut a, KeyCode::Enter);
    assert_eq!((a.line, a.collapsed.clone()), (3, vec![]));
}

#[test]
fn folds_nest_even_when_their_blocks_overlap() {
    let mut a = app_as("py", "a\nb\nc\nd\ne\n");
    a.collapsed = vec![(2, 3), (0, 2), (2, 3)];
    a.nest_collapsed();
    assert_eq!(a.collapsed, vec![(0, 3), (2, 3)]);
}

#[test]
fn f_answers_on_a_markdown_source_shown_again_after_its_preview() {
    let mut a = app_as("md", "- a\n  - b\n");
    key(&mut a, KeyCode::Char('p'));
    key(&mut a, KeyCode::Char('p'));
    a.go((0, 0));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.message, "no fold rules for .md");
}

#[test]
fn a_blank_line_between_methods_is_the_class_and_one_inside_a_body_the_method() {
    let mut a = app_as("py", PY);
    a.go((5, 0));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(
        a.collapsed,
        vec![(0, 7)],
        "the class, as VS Code, JetBrains, Zed and Vim"
    );
    let mut a = app_as("py", "def f():\n    a = 1\n\n    b = 2\n");
    a.go((2, 0));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(0, 3)]);
}

#[test]
fn a_docstring_folds_from_its_first_line_and_the_class_from_inside_it() {
    let text = "class A:\n    \"\"\"\n    Doc.\n    \"\"\"\n\n    x = 1\n";
    let mut a = app_as("py", text);
    a.go((1, 4));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(1, 3)]);
    let mut a = app_as("py", text);
    a.go((2, 4));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(0, 5)]);
}

#[test]
fn every_fold_fixture_folds_as_annotated() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/folds");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let ext = path.extension().unwrap().to_str().unwrap().to_owned();
        let mark = if ext == "py" { "# f: " } else { "// f: " };
        let text = std::fs::read_to_string(&path).unwrap();
        let mut a = app_as(&ext, &text);
        let mut checked = 0;
        for (l, line) in text.lines().enumerate() {
            let Some((_, want)) = line.split_once(mark) else {
                continue;
            };
            let (h, e) = want.split_once('-').unwrap();
            let want = (
                h.parse::<usize>().unwrap() - 1,
                e.parse::<usize>().unwrap() - 1,
            );
            a.collapsed.clear();
            a.go((l, 0));
            key(&mut a, KeyCode::Char('f'));
            assert_eq!(
                a.collapsed,
                vec![want],
                "{path:?}, f on line {}: {line}",
                l + 1
            );
            checked += 1;
        }
        assert!(checked > 40, "{path:?}: {checked} annotations");
    }
}

const GO: &str = "\
func f(n int) string {
\tswitch n {
\tcase 1:
\t\treturn \"one\"

\tcase 2:
\t\treturn \"two\"
\t}
\treturn \"\"
}
";

#[test]
fn a_go_fold_keeps_go_rules_through_edits_switches_and_reloads() {
    let (dir, mut a) = files_app("fold-go");
    std::fs::write(dir.join("a.go"), GO).unwrap();
    std::fs::write(dir.join("b.go"), "package b\n").unwrap();
    a.jump_to(&dir.join("a.go"), 3);
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(2, 4)], "the case and its blank line");
    a.jump_to(&dir.join("b.go"), 1);
    a.jump_to(&dir.join("a.go"), 1);
    assert_eq!(a.collapsed, vec![(2, 4)]);

    let mut a = app_as("go", GO);
    a.go((2, 0));
    key(&mut a, KeyCode::Char('f'));
    a.go((0, 0));
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Enter);
    assert_eq!(a.collapsed, vec![(3, 5)]);

    let mut a = app_as("go", GO);
    a.go((2, 0));
    key(&mut a, KeyCode::Char('f'));
    let path = a.buf.path.clone().unwrap();
    std::fs::write(&path, format!("package a\n\n{GO}")).unwrap();
    a.reload(false);
    assert_eq!(a.collapsed, vec![(4, 6)]);
}

#[test]
fn f_folds_a_jsx_element_in_a_jsx_file() {
    let mut a = app_as("jsx", "const a = (\n  <div>\n    <p>hi</p>\n  </div>\n);\n");
    a.go((1, 2));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(1, 3)]);
}
