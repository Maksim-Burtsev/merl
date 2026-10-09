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
fn a_fold_in_json_stays_with_its_file_measured_as_json() {
    let (dir, mut a) = files_app("fold-json");
    std::fs::write(dir.join("a.json"), "{\n  \"e\": {\n  },\n  \"x\": 1\n}\n").unwrap();
    std::fs::write(dir.join("b.py"), "x = 1\n").unwrap();
    a.jump_to(&dir.join("a.json"), 0);
    a.go((1, 0));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(1, 2)]);
    a.jump_to(&dir.join("b.py"), 0);
    a.jump_to(&dir.join("a.json"), 0);
    assert_eq!(
        a.collapsed,
        vec![(1, 2)],
        "an empty object, which indentation never folds"
    );
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
fn an_objective_c_header_folds_as_objective_c_and_a_c_header_as_cpp() {
    let objc = "@interface Box : NSObject\n- (void)open {\n  go();\n}\n@end\n";
    let mut a = app_as("h", objc);
    a.go((2, 2));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(1, 3)]);
    a.collapsed.clear();
    a.go((0, 0));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(0, 4)]);
    let mut a = app_as("h", "namespace a {\nclass B {\n  int c;\n};\n}\n");
    a.go((1, 0));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(1, 3)]);
    let objcpp = "#import \"a.h\"\nnamespace a {\nclass B {\n  void c() {\n    go();\n  }\n};\n}\n@class D;\n";
    let mut a = app_as("h", objcpp);
    a.go((2, 0));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(
        a.collapsed,
        vec![(2, 6)],
        "a C++ class in an Objective-C++ header"
    );
}

#[test]
fn f_folds_python_and_says_so_in_other_languages() {
    let mut a = app_as("txt", "a\n  b\n");
    key(&mut a, KeyCode::Char('f'));
    assert!(a.collapsed.is_empty());
    assert_eq!(a.message, "no fold rules for .txt");
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
fn a_block_a_word_opens_folds_from_its_line_with_its_closing_word_shown() {
    let mut a = app_as("sh", "f() {\n  if a; then\n    b\n  fi\n}\n");
    a.go((1, 0));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(1, 3)]);
    assert_eq!(a.collapsed_tail(1), Some("fi"));
    let mut a = app_as("rb", "def f\n  if a\n    b\n  end\nend\n");
    a.go((1, 0));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(1, 3)], "the `if`, not `f`");
    assert_eq!(a.collapsed_tail(1), Some("end"));
    a.go((0, 0));
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Enter);
    assert_eq!(
        a.collapsed,
        vec![(2, 4)],
        "it moves with a line typed above"
    );
    let mut a = app_as("sh", "for x in y; do\n  b\ndone < list\n");
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed_tail(0), Some("done < list"));
}

#[test]
fn a_fold_inside_a_callback_is_the_same_fold_after_a_reload() {
    let text = "vim.keymap.set(\"n\", \"K\", function()\n  hover()\nend, {\n  desc = \"x\",\n})\n";
    let mut a = app_as("lua", text);
    a.go((1, 2));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(0, 4)], "as f on its first line");
    let path = a.buf.path.clone().unwrap();
    std::fs::write(&path, format!("local k = 1\n{text}")).unwrap();
    a.reload(false);
    assert_eq!(a.collapsed, vec![(1, 5)]);
}

#[test]
fn an_erlang_file_has_no_fold_rules_though_elixir_does() {
    let mut a = app_as(
        "erl",
        "f(X) ->\n    case X of\n        1 -> one\n    end.\n",
    );
    key(&mut a, KeyCode::Char('f'));
    assert!(a.collapsed.is_empty());
    assert_eq!(a.message, "no fold rules for .erl");
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
    assert_eq!(a.collapsed, vec![(0, 1)]);
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
        let ext = path.extension().unwrap().to_str().unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let mut a = app_as(ext, &text);
        let mut checked = 0;
        for (l, line) in text.lines().enumerate() {
            let Some((_, want)) = line.rsplit_once(" f: ") else {
                continue;
            };
            let want: String = want
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '-')
                .collect();
            let want = want.split_once('-').map(|(h, e)| {
                (
                    h.parse::<usize>().unwrap() - 1,
                    e.parse::<usize>().unwrap() - 1,
                )
            });
            a.collapsed.clear();
            a.go((l, 0));
            key(&mut a, KeyCode::Char('f'));
            let name = path.file_name().unwrap().to_string_lossy();
            assert_eq!(
                a.collapsed,
                Vec::from_iter(want),
                "f on {name}:{}: {line}",
                l + 1
            );
            checked += 1;
        }
        assert!(checked > 10, "{checked} annotations in {}", path.display());
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

const CS: &str = "\
class A
{
    void F()
    {
        Go();
    }
}
";

#[test]
fn f_on_a_header_line_folds_its_body_and_unfolds_it_again() {
    let mut a = app_as("cs", CS);
    a.go((2, 4));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(3, 5)], "the body under `void F()`");
    a.go((2, 4));
    key(&mut a, KeyCode::Char('f'));
    assert!(a.collapsed.is_empty(), "f again on the header unfolds it");
}

#[test]
fn a_header_line_fold_stays_through_edits_switches_and_reloads() {
    let (dir, mut a) = files_app("fold-cs");
    std::fs::write(dir.join("a.cs"), CS).unwrap();
    std::fs::write(dir.join("b.cs"), "class B { }\n").unwrap();
    a.jump_to(&dir.join("a.cs"), 3);
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(3, 5)]);
    a.jump_to(&dir.join("b.cs"), 1);
    a.jump_to(&dir.join("a.cs"), 1);
    assert_eq!(a.collapsed, vec![(3, 5)]);

    let mut a = app_as("cs", CS);
    a.go((2, 4));
    key(&mut a, KeyCode::Char('f'));
    a.go((0, 0));
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Enter);
    assert_eq!(a.collapsed, vec![(4, 6)]);

    let mut a = app_as("cs", CS);
    a.go((2, 4));
    key(&mut a, KeyCode::Char('f'));
    let path = a.buf.path.clone().unwrap();
    std::fs::write(&path, format!("using X;\n\n{CS}")).unwrap();
    a.reload(false);
    assert_eq!(a.collapsed, vec![(5, 7)]);
}

#[test]
fn f_folds_a_jsx_element_in_a_jsx_file() {
    let mut a = app_as("jsx", "const a = (\n  <div>\n    <p>hi</p>\n  </div>\n);\n");
    a.go((1, 2));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(1, 3)]);
}

#[test]
fn f_survives_every_prefix_of_the_swift_php_objc_c_cpp_elixir_rbs_zsh_dart_and_scala_fixtures() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/folds");
    let fixtures = [
        ("dart", "dart.dart"),
        ("scala", "scala.scala"),
        ("ex", "elixir.ex"),
        ("rbs", "rbs.rbs"),
        ("zsh", "zsh.zsh"),
        ("swift", "swift.swift"),
        ("php", "php.php"),
        ("m", "objc.m"),
        ("c", "c.c"),
        ("cc", "cpp.cc"),
    ];
    for (ext, name) in fixtures {
        let text = std::fs::read_to_string(dir.join(name)).unwrap();
        let mut a = app_as(ext, "");
        let path = a.buf.path.clone().unwrap();
        for (cut, _) in text.char_indices() {
            a.buf = Buffer::from_bytes(path.clone(), &text.as_bytes()[..cut]);
            a.collapsed.clear();
            a.go((a.buf.lines.len() - 1, 0));
            key(&mut a, KeyCode::Char('f'));
        }
    }
}

#[test]
fn f_in_rbs_survives_an_annotation_opened_by_a_non_ascii_char() {
    let mut a = app_as("rbs", "class A\n  %aé\nend\n");
    a.go((1, 2));
    key(&mut a, KeyCode::Char('f'));
    assert_eq!(a.collapsed, vec![(0, 2)]);
}

#[test]
fn f_folds_zsh_in_the_zsh_startup_files() {
    for name in [".zshrc", ".zshenv", ".zprofile"] {
        let mut a = app("");
        let path = a.buf.path.clone().unwrap().with_file_name(name);
        a.buf = Buffer::from_bytes(path, b"() {\n  print hi\n}\n");
        a.go((0, 0));
        key(&mut a, KeyCode::Char('f'));
        assert_eq!(a.collapsed, vec![(0, 2)], "{name}");
    }
}

#[test]
fn php_uses_inside_a_braced_namespace_and_a_phtml_file_fold() {
    let text =
        "<?php\nnamespace App {\n    use A;\n    use B;\n\n    function f()\n    {\n    }\n}\n";
    for ext in ["php", "phtml"] {
        let mut a = app_as(ext, text);
        a.go((2, 4));
        key(&mut a, KeyCode::Char('f'));
        assert_eq!(a.collapsed, vec![(2, 3)], ".{ext}");
    }
}
