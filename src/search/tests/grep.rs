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
fn ignore_case_finds_every_spelling() {
    let (dir, files) = project("case");
    let all = [
        ("a.py".into(), 2),
        ("a.py".into(), 11),
        ("a.py".into(), 12),
        ("b.go".into(), 5),
    ];
    assert_eq!(lines(&grep(&dir, &files, "total", false, true)), all);
    assert_eq!(lines(&grep(&dir, &files, "Total", false, true)), all);
    assert_eq!(
        lines(&grep(&dir, &files, "tOTAL", false, true)),
        all,
        "whichever letters of the query are capitals: `sameCancel` typed from memory finds \
         `SameCancel`"
    );
    assert_eq!(
        lines(&grep(&dir, &files, "Total", false, false)),
        [("b.go".into(), 5)],
        "`u` and `d` look for a word taken from the code: they keep its case"
    );
    assert_eq!(lines(&grep(&dir, &files, "invoice", false, false)), []);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn fuzzy_match_ignores_case_even_with_a_capital() {
    assert!(fuzzy_match("sameCancel", "SameCancel"));
    assert!(fuzzy_match("SC", "sameCancel"));
    assert!(!fuzzy_match("sameCancel", "SameCall"));
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

#[test]
fn the_cap_keeps_the_first_files_in_the_order_given() {
    let names: Vec<String> = (0..400).map(|i| format!("f{i:03}.py")).collect();
    let many = "x = 1\n".repeat(MAX_HITS / 100);
    let texts: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), many.as_str())).collect();
    let (dir, mut given) = scratch("cap-order", &texts);
    given.reverse();
    for _ in 0..5 {
        let hits = grep(&dir, &given, "x", false, false);
        let mut kept: Vec<&Path> = hits.iter().map(|h| h.path.as_path()).collect();
        kept.dedup();
        let first: Vec<&Path> = (given[..100].iter().rev()).map(PathBuf::as_path).collect();
        assert_eq!(hits.len(), MAX_HITS);
        assert_eq!(
            kept, first,
            "the 100 files first in the list, sorted by path"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn unsaved_text_is_searched_in_place_of_its_file_and_an_unreadable_file_is_skipped() {
    let (dir, mut files) = scratch(
        "unsaved",
        &[("a.py", "total = 1\n"), ("b.py", "total = 2\n")],
    );
    files.insert(1, PathBuf::from("gone.py"));
    let hits = grep_project(
        &dir,
        &files,
        "total",
        false,
        false,
        Some(Path::new("b.py")),
        Some(b"x = 0\n  total = 3\n"),
    )
    .unwrap();
    assert_eq!(lines(&hits), [("b.py".into(), 2), ("a.py".into(), 1)]);
    assert_eq!(hits[0].text, "  total = 3");
    assert_eq!(hits[0].byte_col, Some(2));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_filter_runs_before_the_cap() {
    let names: Vec<String> = (0..200).map(|i| format!("f{i:03}.py")).collect();
    let many = "x = 1\n".repeat(MAX_HITS / 100);
    let mut texts: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), many.as_str())).collect();
    texts.push(("last.py", "x = 2\n"));
    let (dir, files) = scratch("filter-cap", &texts);
    let hits = grep_filtered(&dir, &files, "x", None, None, |l| l == "x = 2").unwrap();
    assert_eq!(lines(&hits), [("last.py".into(), 1)]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_panic_on_any_file_reaches_the_caller() {
    let names: Vec<String> = (0..400).map(|i| format!("f{i:03}.py")).collect();
    let texts: Vec<(&str, &str)> = (names.iter())
        .enumerate()
        .map(|(i, n)| (n.as_str(), if i == 399 { "boom\n" } else { "calm\n" }))
        .collect();
    let (dir, files) = scratch("panic-reaches", &texts);
    let run = std::panic::catch_unwind(|| {
        grep_filtered(&dir, &files, "boom|calm", None, None, |l| {
            assert_ne!(l, "boom");
            true
        })
    });
    assert!(run.is_err(), "the file a worker thread read panicked");
    std::fs::remove_dir_all(&dir).unwrap();
}

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
        ("src/contest.rs", false),
        ("latest/users.py", false),
        ("src/testimonials.py", false),
        ("docs/spec.md", false),
        ("src/users/service.py", false),
        ("src/protest.go", false),
        ("src/specs.py", false),
    ] {
        let tier = rank(Path::new(path), Some(here), false).tier;
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
    let declaration = |p: &str, i: usize| p == "src/users/repo.py" && i == 5;
    let mut order: Vec<(usize, &str)> = paths.iter().copied().enumerate().collect();
    order.sort_by_key(|&(i, p)| (rank(Path::new(p), Some(here), declaration(p, i)), p, i));
    assert_eq!(
        order.iter().map(|&(_, p)| p).collect::<Vec<_>>(),
        [
            "src/users/repo.py",
            "src/users/service.py",
            "src/users/repo.py",
            "src/users/admin/view.py",
            "src/api/admin.py",
            "tests/test_service.py",
        ],
        "the declaration, the open file, the same directory, one below, one up and one down, \
         and a test file whatever its distance"
    );
    let test = Path::new("tests/test_service.py");
    assert_eq!(
        rank(test, Some(test), false).tier,
        Tier::Open,
        "the open file is never demoted, even when it is a test file itself"
    );
    assert_eq!(
        rank(test, None, true).tier,
        Tier::Tests,
        "`d` asks for the tier alone: every candidate of its own is a declaration"
    );
    assert_eq!(rank(here, None, true).tier, Tier::Declaration);
}

#[test]
fn a_declaration_outside_is_found_from_the_lines_kept_of_its_file() {
    let dir = std::env::temp_dir().join(format!("merl-shaped-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let past_sniff = format!(
        "function value() {{}}\n{}\0\nfunction value() {{}}\n",
        "x\n".repeat(40_000)
    );
    let over_cap = format!(
        "{}function value() {{}}\n",
        "function other() {}\n".repeat(14_000)
    );
    let texts = [
        (
            "a.d.ts",
            "export declare function value(x: number): string;\nconst value$ = 1\n  value(): void;\n  value: string;\r\n  value<\n    T,\n  >(x: T): this {\nlet values = 2;\nexport { value as other };\n  value = () => 1",
        ),
        (
            "b.js",
            "function first() {}\n\tvalue: function () {},\nfoo(value);\n",
        ),
        ("c.js", past_sniff.as_str()),
        ("d.js", "\u{feff}function value() {}\n"),
        (
            "e.js",
            "class X {\n  static value = async (a) => a;\n  constructor(private value: V) {}\n}\n",
        ),
        ("f.js", over_cap.as_str()),
    ];
    for (name, text) in texts {
        std::fs::write(dir.join(name), text).unwrap();
    }
    let files: Vec<PathBuf> = texts.iter().map(|(n, _)| dir.join(n)).collect();
    let rows = |hits: Vec<Hit>| -> Vec<(PathBuf, usize, Option<usize>, String)> {
        hits.into_iter()
            .map(|h| (h.path, h.line1, h.byte_col, h.text))
            .collect()
    };
    let kept = ShapedFiles::default();
    let mut every = def_patterns(Kind::TsJs, "value");
    every.extend(member_or_signature(Kind::TsJs, "value").unwrap());
    for pattern in [
        def_patterns(Kind::TsJs, "value").join("|"),
        member_patterns(Kind::TsJs, "value").unwrap().join("|"),
        every.join("|"),
    ] {
        let shape = ts_shape("value", &pattern).expect("a pattern of `d`");
        let plain =
            rows(grep_project(Path::new(""), &files, &pattern, false, false, None, None).unwrap());
        assert!(plain.len() >= 5, "{plain:?}");
        assert_eq!(
            rows(grep_shaped(&files, &pattern, shape, &kept).unwrap()),
            plain
        );
        assert_eq!(
            rows(grep_shaped(&files, &pattern, shape, &kept).unwrap()),
            plain
        );
    }
    assert!(
        kept.lock().unwrap()[&dir.join("f.js")].is_none(),
        "lines over the cap are not kept"
    );
    assert!(ts_shape("value", "value").is_none());
    assert!(ts_shape("vä", &def_patterns(Kind::TsJs, "vä").join("|")).is_none());
    std::fs::remove_file(dir.join("b.js")).unwrap();
    let pattern = def_patterns(Kind::TsJs, "first").join("|");
    let shape = ts_shape("first", &pattern).unwrap();
    assert_eq!(
        lines(&grep_shaped(&files, &pattern, shape, &kept).unwrap()),
        [(dir.join("b.js").display().to_string(), 1)],
        "read from the lines kept, not the disk"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
