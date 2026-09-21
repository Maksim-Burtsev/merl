//! The grep itself: whole word, smart case and the order of the hits.

use super::*;

#[test]
fn whole_word_excludes_longer_identifiers() {
    let (dir, files) = project("word");
    let py = files[..1].to_vec();
    assert_eq!(
        lines(&grep(&dir, &py, "total", true, false)),
        [("a.py".into(), 2)],
        "total_foobar is not the word `total`"
    );
    assert_eq!(lines(&grep(&dir, &py, "total", false, false)).len(), 3);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn smart_case_only_ignores_case_for_lowercase_patterns() {
    let (dir, files) = project("case");
    // Lowercase: `total`, `total_foobar` and Go's `Total`.
    assert_eq!(
        lines(&grep(&dir, &files, "total", false, true)),
        [
            ("a.py".into(), 2),
            ("a.py".into(), 11),
            ("a.py".into(), 12),
            ("b.go".into(), 5)
        ]
    );
    // One uppercase letter makes the whole pattern case-sensitive.
    assert_eq!(
        lines(&grep(&dir, &files, "Total", false, true)),
        [("b.go".into(), 5)]
    );
    // Without smart case a lowercase pattern stays case-sensitive too.
    assert_eq!(lines(&grep(&dir, &files, "invoice", false, false)), []);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn current_file_sorts_first_then_path_and_line() {
    let (dir, files) = project("sort");
    let hits = grep_project(
        &dir,
        &files,
        "Invoice",
        false,
        false,
        Some(Path::new("b.go")),
        None,
    )
    .unwrap();
    assert_eq!(
        lines(&hits),
        [
            ("b.go".into(), 3),
            ("b.go".into(), 5),
            ("b.go".into(), 7),
            ("a.py".into(), 1),
            ("a.py".into(), 7)
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Every pattern of [`LAST`], and the near-misses that carry the same letters but are code.
#[test]
fn tests_mocks_fixtures_and_generated_files_rank_last() {
    let here = Path::new("src/users/service.py");
    for (path, last) in [
        ("test/test_users.py", true),
        ("tests/users.py", true),
        ("src/__tests__/users.ts", true),
        ("spec/users_spec.rb", true),
        ("specs/users.rb", true),
        ("pkg/testdata/golden.go", true),
        ("tests/fixtures/users.json", true),
        ("src/__fixtures__/users.ts", true),
        ("src/mocks/store.ts", true),
        ("src/__mocks__/store.ts", true),
        ("vendor/github.com/x/y/y.go", true),
        ("third_party/x/y.py", true),
        ("src/test_users.py", true),
        ("src/conftest.py", true),
        ("pkg/users_test.go", true),
        ("src/users_spec.rb", true),
        ("src/users.test.ts", true),
        ("src/users.spec.ts", true),
        ("api/users_pb2.py", true),
        ("api/users_pb2_grpc.py", true),
        ("api/users.pb.go", true),
        ("api/users.gen.go", true),
        ("api/users.generated.ts", true),
        // The near-misses: the letters are there, the pattern is not.
        ("src/contest.rs", false),
        ("latest/users.py", false),
        ("src/testimonials.py", false),
        ("docs/spec.md", false),
        ("src/users/service.py", false),
        ("src/protest.go", false),
        ("src/specs.py", false),
    ] {
        let tier = rank(Path::new(path), Some(here), false).0;
        assert_eq!(tier == Tier::Tests, last, "{path} ranked {tier:?}");
    }
}

#[test]
fn a_declaration_comes_first_and_the_nearest_directory_next() {
    let here = Path::new("src/users/service.py");
    let paths = [
        "src/api/admin.py",
        "tests/test_service.py",
        "src/users/repo.py",
        "src/users/service.py",
        "src/users/admin/view.py",
        "src/users/repo.py",
    ];
    // The last row is the declaration; the open file is the one that equals `here`.
    let declaration = |p: &str, i: usize| p == "src/users/repo.py" && i == 5;
    let mut order: Vec<(usize, &str)> = paths.iter().copied().enumerate().collect();
    order.sort_by_key(|&(i, p)| (rank(Path::new(p), Some(here), declaration(p, i)), p, i));
    assert_eq!(
        order.iter().map(|&(_, p)| p).collect::<Vec<_>>(),
        [
            "src/users/repo.py",       // the declaration
            "src/users/service.py",    // the open file
            "src/users/repo.py",       // the same directory
            "src/users/admin/view.py", // one below
            "src/api/admin.py",        // one up and one down
            "tests/test_service.py",   // a test file, whatever its distance
        ]
    );
    // The open file is never demoted, even when it is a test file itself.
    let test = Path::new("tests/test_service.py");
    assert_eq!(rank(test, Some(test), false).0, Tier::Open);
    // `d` asks for the tier alone: every candidate of its own is a declaration.
    assert_eq!(rank(test, None, true).0, Tier::Tests);
    assert_eq!(rank(here, None, true).0, Tier::Declaration);
}
