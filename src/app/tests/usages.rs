//! `u`: the usages of the word under the cursor.

use super::*;

/// The rows of the open result picker, each split into its mark and the `path:line` it
/// points at.
fn usage_rows(a: &mut App) -> Vec<(String, String)> {
    let picker = a.picker.as_mut().expect("a picker");
    picker.settle();
    picker
        .window(50)
        .0
        .into_iter()
        .map(|r| {
            let head = r.item.label[..r.item.code_at.unwrap()].trim_end();
            let (mark, place) = head.rsplit_once(' ').unwrap_or(("", head));
            (mark.trim().into(), place.trim_end_matches(':').into())
        })
        .collect()
}

/// Puts the cursor on `word` in `file` at `line` and presses `u`.
fn usages_at(a: &mut App, dir: &Path, file: &str, line: usize, word: &str) {
    a.jump_to(&dir.join(file), line);
    a.col = a.line_str().find(word).expect(word);
    press(a, KeyCode::Char('u'), KeyModifiers::NONE);
}

/// #81: the declaration first and marked as one, then the open file, then the rest of the
/// project's code nearest first, then the tests — a test file next door still sorts below
/// code a package away.
#[test]
fn usages_put_the_declaration_first_and_the_tests_last() {
    let (dir, mut a) = project_app(
        "u-order",
        &[
            (
                "src/users/repo.py",
                "class Repo:\n    def delete_user(self, id):\n        pass\n",
            ),
            (
                "src/users/admin.py",
                "def purge(repo, id):\n    repo.delete_user(id)\n",
            ),
            (
                "src/users/repo_test.py",
                "def check(repo):\n    repo.delete_user(2)\n",
            ),
            (
                "src/api/view.py",
                "def view(repo):\n    repo.delete_user(1)\n",
            ),
            (
                "tests/test_repo.py",
                "def test_delete(repo):\n    repo.delete_user(1)\n",
            ),
        ],
    );
    usages_at(&mut a, &dir, "src/users/admin.py", 2, "delete_user");
    assert_eq!(
        usage_rows(&mut a),
        [
            ("declaration".to_string(), "src/users/repo.py:2".to_string()),
            (String::new(), "src/users/admin.py:2".into()),
            (String::new(), "src/api/view.py:2".into()),
            (String::new(), "src/users/repo_test.py:2".into()),
            (String::new(), "tests/test_repo.py:2".into()),
        ]
    );
    assert_eq!(
        a.picker.as_ref().unwrap().title,
        "Usages of delete_user: 1 declaration, 2 in code, 2 in tests"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #405: a FIFO in the project is not read. `u`, `D` and `s` opened it and waited for a writer
/// forever, `u` and `D` on the UI thread. Here they run on a thread of their own, so a wait
/// fails the test instead of hanging it.
#[test]
#[cfg(unix)]
fn a_fifo_in_the_project_stalls_no_search() {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let code = "def hello():\n    return 1\n\nhello()\n";
        let (dir, mut a) = project_app("u-fifo", &[("a.py", code)]);
        let mkfifo = std::process::Command::new("mkfifo")
            .arg(dir.join("pipe"))
            .status();
        assert!(mkfifo.unwrap().success());
        let (tree, files) = crate::tree::build(&dir, false);
        a.project_walked(tree, files);
        usages_at(&mut a, &dir, "a.py", 1, "hello");
        let _ = tx.send(format!("u: {}", a.picker.as_ref().unwrap().title));
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
        a.picker.as_mut().unwrap().settle();
        let _ = tx.send(format!("D: {}", a.picker.as_ref().unwrap().counts().1));
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
        typed(&mut a, "hello");
        a.settle_search();
        let _ = tx.send(format!("s: {}", a.picker.as_ref().unwrap().counts().1));
        std::fs::remove_dir_all(&dir).unwrap();
    });
    for want in [
        "u: Usages of hello: 1 declaration, 1 in code",
        "D: 1",
        "s: 2",
    ] {
        // `D` in a debug build takes seconds on a loaded machine; the wait on a FIFO is forever.
        let got = rx.recv_timeout(std::time::Duration::from_secs(60));
        assert_eq!(got.as_deref(), Ok(want));
    }
}

/// The title says how the list splits, and a part with no hits is left out rather than
/// printed as a zero.
#[test]
fn the_usages_title_splits_the_counts_and_omits_what_is_not_there() {
    let (dir, mut a) = project_app(
        "u-title",
        &[
            (
                "src/repo.py",
                "class Repo:\n    def delete_user(self, id):\n        pass\n",
            ),
            ("src/api/repo.py", "class Repo:\n    pass\n"),
            (
                "src/admin.py",
                "import log\n\ndef purge(repo, id):\n    log.info(\"purge\")\n    repo.delete_user(id)\n",
            ),
            (
                "tests/test_repo.py",
                "import log\n\ndef test_delete(repo):\n    log.info(\"t\")\n    repo.delete_user(1)\n",
            ),
        ],
    );
    for (file, line, word, title) in [
        (
            "src/admin.py",
            5,
            "delete_user",
            "Usages of delete_user: 1 declaration, 1 in code, 1 in tests",
        ),
        // Two declarations and nothing else: only the one part, and it is plural.
        ("src/repo.py", 1, "Repo", "Usages of Repo: 2 declarations"),
        // A name no rule declares: no declaration part at all.
        (
            "src/admin.py",
            4,
            "info",
            "Usages of info: 1 in code, 1 in tests",
        ),
    ] {
        usages_at(&mut a, &dir, file, line, word);
        let p = a.picker.as_mut().unwrap();
        p.settle();
        assert_eq!(p.title, title);
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #81: the candidates of `d` share the demotion — the copy of the declaration under `spec/`
/// is offered last, though its path sorts first.
#[test]
fn d_offers_the_test_copy_of_a_declaration_last() {
    let (dir, mut a) = project_app(
        "d-tests",
        &[
            ("spec/repo.py", "def delete_user(id):\n    pass\n"),
            ("src/repo.py", "def delete_user(id):\n    pass\n"),
            ("src/admin.py", "def purge(id):\n    delete_user(id)\n"),
        ],
    );
    let rows = |a: &mut App| {
        definition_rows(a)
            .into_iter()
            .map(|(_, _, place)| place)
            .collect::<Vec<_>>()
    };
    a.jump_to(&dir.join("src/admin.py"), 2);
    a.col = a.line_str().find("delete_user").unwrap();
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(rows(&mut a), ["src/repo.py:1", "spec/repo.py:1"]);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    // Read from inside `spec/` and that file is not a test copy, it is what is on screen:
    // its declaration stays first.
    a.jump_to(&dir.join("spec/repo.py"), 2);
    a.col = 4;
    a.buf.lines[1] = "    delete_user(id)".into();
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(rows(&mut a), ["spec/repo.py:1", "src/repo.py:1"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #81: the mark says what `d` would call a declaration — not a `def` line inside a
/// docstring, nothing at all where the kind has no rule for the word, and nothing in a file
/// the list demotes, whose count belongs to the tests.
#[test]
fn the_usages_mark_is_only_for_a_declaration_d_would_offer() {
    let (dir, mut a) = project_app(
        "u-marks",
        &[
            (
                "src/repo.py",
                "class Repo:\n    def delete_user(self, id):\n        pass\n",
            ),
            (
                "docs/guide.py",
                "HELP = \"\"\"\nUsage:\n\ndef delete_user(id):\n\"\"\"\n",
            ),
            (
                "src/admin.py",
                "def purge(repo, id):\n    repo.delete_user(id)\n\ndef build():\n    return make_repo()\n",
            ),
            ("tests/test_repo.py", "def make_repo():\n    return None\n"),
            (
                "main.tf",
                "resource \"aws_s3_bucket\" \"logs\" {\n  count = 2\n}\n",
            ),
            (
                "other.tf",
                "resource \"aws_s3_bucket\" \"data\" {\n  count = 3\n}\n",
            ),
        ],
    );
    usages_at(&mut a, &dir, "src/admin.py", 2, "delete_user");
    assert_eq!(
        usage_rows(&mut a),
        [
            ("declaration".to_string(), "src/repo.py:2".to_string()),
            (String::new(), "src/admin.py:2".into()),
            (String::new(), "docs/guide.py:4".into()),
        ]
    );
    assert_eq!(
        a.picker.as_ref().unwrap().title,
        "Usages of delete_user: 1 declaration, 2 in code"
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    // Terraform declares no bare `count`: with no rule there is no mark and no column for it.
    usages_at(&mut a, &dir, "main.tf", 2, "count");
    assert_eq!(
        usage_rows(&mut a),
        [
            (String::new(), "main.tf:2".to_string()),
            (String::new(), "other.tf:2".into()),
        ]
    );
    assert_eq!(
        a.picker.as_ref().unwrap().title,
        "Usages of count: 2 in code"
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    // A word declared only in a test file: the declaration is a test row, marked as neither.
    usages_at(&mut a, &dir, "src/admin.py", 5, "make_repo");
    assert_eq!(
        usage_rows(&mut a),
        [
            (String::new(), "src/admin.py:5".to_string()),
            (String::new(), "tests/test_repo.py:1".into()),
        ]
    );
    assert_eq!(
        a.picker.as_ref().unwrap().title,
        "Usages of make_repo: 1 in code, 1 in tests"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #281: to grep `-` ends a word, so `db-main` also found `db-main-2` and `db-main-replica`.
/// Where `-` is part of a name (Make, YAML) those lines go; in Python, where `db-main-2` is a
/// subtraction, the line stays.
#[test]
fn usages_of_a_hyphenated_name_leave_out_longer_names() {
    let (dir, mut a) = project_app(
        "u-hyphen",
        &[
            (
                "Makefile",
                "db-main:\n\techo main\n\ndb-main-2:\n\techo two\n\nall: db-main\n",
            ),
            (
                "app.yaml",
                "services:\n  db-main:\n    image: postgres\n  db-main-replica:\n    image: postgres\n",
            ),
            ("calc.py", "x = db-main-2\n"),
        ],
    );
    usages_at(&mut a, &dir, "Makefile", 7, "db-main");
    assert_eq!(
        usage_rows(&mut a),
        [
            ("declaration".to_string(), "Makefile:1".to_string()),
            ("declaration".into(), "app.yaml:2".into()),
            (String::new(), "Makefile:7".into()),
            (String::new(), "calc.py:1".into()),
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #504: in a Makefile `u` marks what `d` counts. A recipe's `GO=$(GO) cmd` sets a variable of
/// one shell command, not make's; `+=` declares a name nothing assigns plainly, and only then.
#[test]
fn makefile_usages_mark_the_declarations_d_jumps_to() {
    let (dir, mut a) = project_app(
        "u-make",
        &[(
            "Makefile",
            "GO ?= go\nCFLAGS += -Wall\nLDFLAGS = -s\nLDFLAGS += -w\n\nbuild:\n\tGO=$(GO) ./build.sh $(CFLAGS) $(LDFLAGS)\n",
        )],
    );
    // On make's `$(GO)`: on the recipe's own `GO=` the line declares the shell variable of its
    // command, for `d` and `u` alike (`makefile_usages_ask_the_rule_d_asks`).
    let rows = |a: &mut App, word: &str| {
        a.jump_to(&dir.join("Makefile"), 7);
        a.col = a.line_str().find(&format!("$({word})")).unwrap() + 2;
        press(a, KeyCode::Char('u'), KeyModifiers::NONE);
        let rows = usage_rows(a);
        press(a, KeyCode::Esc, KeyModifiers::NONE);
        rows
    };
    let row = |mark: &str, line: usize| (mark.to_string(), format!("Makefile:{line}"));
    assert_eq!(rows(&mut a, "GO"), [row("declaration", 1), row("", 7)]);
    assert_eq!(rows(&mut a, "CFLAGS"), [row("declaration", 2), row("", 7)]);
    assert_eq!(
        rows(&mut a, "LDFLAGS"),
        [row("declaration", 3), row("", 4), row("", 7)]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #281: a grep that stopped at its cap stays marked as cut after the filter drops the longer
/// names, or the hits past the cap would look absent.
#[test]
fn a_cut_usages_list_says_so_after_the_filter() {
    let make = format!("db-main:\n{}", "\tdb-main-2\n".repeat(search::MAX_HITS));
    let (dir, mut a) = project_app("u-hyphen-cut", &[("Makefile", &make)]);
    usages_at(&mut a, &dir, "Makefile", 1, "db-main");
    let picker = a.picker.as_mut().expect("a picker");
    picker.settle();
    assert_eq!(picker.counts().0, 1);
    assert!(picker.title.ends_with("(first 5000)"), "{}", picker.title);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #387: `u` reads a Ruby name as `d` does, its `?`, `!` or setter `=` included, so the method it
/// asks about is the one its own `def` or `attr_writer` declares, and that row comes first.
#[test]
fn usages_of_a_ruby_suffixed_method_put_its_declaration_first() {
    let (dir, mut a) = project_app(
        "u-ruby-suffix",
        &[
            (
                "app/user.rb",
                "class User\n  attr_writer :name\n\n  def valid?\n    true\n  end\n\n  def save!\n    true\n  end\nend\n",
            ),
            ("app/use.rb", "user.valid?\nuser.save!\nuser.name = \"x\"\n"),
        ],
    );
    for (line, word, declared) in [(1, "valid", 4), (2, "save", 8), (3, "name", 2)] {
        usages_at(&mut a, &dir, "app/use.rb", line, word);
        assert_eq!(
            usage_rows(&mut a),
            [
                ("declaration".to_string(), format!("app/user.rb:{declared}")),
                (String::new(), format!("app/use.rb:{line}")),
            ],
            "{word}"
        );
        assert!(
            a.picker
                .as_ref()
                .unwrap()
                .title
                .ends_with("1 declaration, 1 in code"),
            "{}",
            a.picker.as_ref().unwrap().title
        );
        a.picker = None;
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #383: `u` on a Ruby `@total` lists every `total` as before, and marks its assignment
/// `@total = 0` a declaration beside `def total`; on a bare `total` only the `def` declares.
#[test]
fn usages_of_a_ruby_ivar_mark_its_assignment() {
    let (dir, mut a) = project_app(
        "u-ruby-ivar",
        &[(
            "app/cart.rb",
            "class Cart\n  def initialize\n    @total = 0\n  end\n\n  def total\n    @total\n  end\nend\n",
        )],
    );
    usages_at(&mut a, &dir, "app/cart.rb", 7, "total");
    let marked = |rows: Vec<(String, String)>| {
        rows.into_iter()
            .filter(|(m, _)| m == "declaration")
            .map(|(_, at)| at)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        marked(usage_rows(&mut a)),
        ["app/cart.rb:3", "app/cart.rb:6"]
    );
    a.picker = None;
    usages_at(&mut a, &dir, "app/cart.rb", 6, "total");
    assert_eq!(marked(usage_rows(&mut a)), ["app/cart.rb:6"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #459: `u` reads an Elixir name as `d` does, its `?` or `!` included, so the `def` of `ship!`
/// is its declaration and comes first.
#[test]
fn usages_of_an_elixir_suffixed_function_put_its_declaration_first() {
    let (dir, mut a) = project_app(
        "u-elixir-suffix",
        &[
            (
                "lib/w.ex",
                "defmodule W do\n  def full?(c), do: c\n  def ship!(c), do: c\nend\n",
            ),
            ("lib/use.ex", "W.full?(c)\nW.ship!(c)\n"),
        ],
    );
    for (line, word, declared) in [(1, "full", 2), (2, "ship", 3)] {
        usages_at(&mut a, &dir, "lib/use.ex", line, word);
        assert_eq!(
            usage_rows(&mut a),
            [
                ("declaration".to_string(), format!("lib/w.ex:{declared}")),
                (String::new(), format!("lib/use.ex:{line}")),
            ],
            "{word}"
        );
        a.picker = None;
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #419: `u` asks what declares a GraphQL field as `d` does. An indented `email` is a field only
/// directly inside a type; a selection of a query, in the file on screen or not, is a use.
#[test]
fn usages_in_graphql_mark_the_field_not_the_selection() {
    let (dir, mut a) = project_app(
        "u-graphql",
        &[
            ("schema.graphql", "type User {\n  email: String\n}\n"),
            ("ops.graphql", "query Me {\n  me {\n    email\n  }\n}\n"),
        ],
    );
    usages_at(&mut a, &dir, "ops.graphql", 3, "email");
    assert_eq!(
        usage_rows(&mut a),
        [
            ("declaration".to_string(), "schema.graphql:2".to_string()),
            (String::new(), "ops.graphql:3".into()),
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Review of #515: `u` asks the rule `d` asks. A recipe's `ARCH=x86` declares the shell variable
/// of its own command, `$${ARCH}`, for both; a recipe's `X += 1` declares nothing, even with no
/// other line setting `X`.
#[test]
fn makefile_usages_ask_the_rule_d_asks() {
    let (dir, mut a) = project_app(
        "u-make-rule",
        &[(
            "Makefile",
            "build:\n\tARCH=x86 ./b.sh $${ARCH}\n\tX += 1\n\techo $(X)\n",
        )],
    );
    let row = |mark: &str, line: usize| (mark.to_string(), format!("Makefile:{line}"));
    usages_at(&mut a, &dir, "Makefile", 2, "ARCH");
    assert_eq!(usage_rows(&mut a), [row("declaration", 2)]);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    usages_at(&mut a, &dir, "Makefile", 4, "X");
    assert_eq!(usage_rows(&mut a), [row("", 3), row("", 4)]);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    a.jump_to(&dir.join("Makefile"), 2);
    a.col = a.line_str().rfind("ARCH").unwrap();
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("Makefile"), 1));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #353 took the Rust `let` out of `def_patterns`, since `d` reads a local by scope; `u` still
/// marks a `let` of the word as its declaration, as on master.
#[test]
fn usages_mark_a_rust_let_as_a_declaration() {
    let (dir, mut a) = project_app(
        "u-rust-let",
        &[(
            "src/lib.rs",
            "fn f() -> u32 {\n    let mut total = 1;\n    total + 1\n}\n",
        )],
    );
    usages_at(&mut a, &dir, "src/lib.rs", 3, "total");
    assert_eq!(
        usage_rows(&mut a),
        [
            ("declaration".to_string(), "src/lib.rs:2".to_string()),
            (String::new(), "src/lib.rs:3".into()),
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
