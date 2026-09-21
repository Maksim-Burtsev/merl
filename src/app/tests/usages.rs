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
