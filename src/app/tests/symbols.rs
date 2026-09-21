//! `D`: the declarations of the project.

use super::*;

#[test]
fn symbols_read_each_kind_with_its_own_pattern() {
    let (dir, mut a) = project_app(
        "symbols",
        &[
            ("Makefile", "build: deps\n"),
            ("deploy.yaml", "apiVersion: apps/v1\nx-base: &base\n"),
            ("main.tf", "variable \"region\" {}\n"),
            ("Dockerfile", "FROM rust AS build\n"),
            ("app.py", "def serve():\n    pass\n"),
            // C is read from its own rows: `struct` must not be listed by the shared
            // pattern as well, and a prototype is not a symbol.
            (
                "invoice.h",
                "#define LIMIT 10\nstruct invoice {\n    int total;\n};\nint sum(struct invoice *i);\n",
            ),
            // Lua from its own rows too: the shared pattern reads `function M.setup(` as a
            // declaration of `M`.
            (
                "init.lua",
                "local M = {}\n\nfunction M.setup(opts)\n  return opts\nend\n",
            ),
            // Elixir from its own rows: the shared pattern knows `def` and nothing else of
            // the family, and `defp` would be missing.
            (
                "ledger.ex",
                "defmodule Ledger do\n  @timeout 5\n\n  defp normalise(raw), do: raw\nend\n",
            ),
            // Zig keeps the shared pattern and complements it: `pub fn` comes from there,
            // the test from a row of its own.
            (
                "ledger.zig",
                "pub fn total() u32 {\n    return 0;\n}\n\ntest \"it adds up\" {}\n",
            ),
        ],
    );
    press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
    let picker = a.picker.as_mut().unwrap();
    picker.settle();
    let names: Vec<String> = picker
        .window(20)
        .0
        .into_iter()
        .map(|r| r.item.label.split_whitespace().next().unwrap().to_string())
        .collect();
    // `apiVersion:` has the shape of a Makefile target; the target rule only reads Makefiles.
    assert_eq!(
        names,
        [
            "&base",
            "build",
            "build",
            "invoice",
            // The first word of `it adds up`, the Zig test's description.
            "it",
            "Ledger",
            "LIMIT",
            "normalise",
            "serve",
            "setup",
            "total",
            "var.region"
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A project past the cap: `a.go` fills the list on its own, so the walk stops before
/// `z.go` and `zebra` is behind the cut. `main.tf` is there for a name with a dot in it.
fn capped_project(tag: &str) -> (PathBuf, App) {
    let many: String = (0..search::MAX_HITS)
        .map(|i| format!("func a{i}() {{}}\n"))
        .collect();
    project_app(
        tag,
        &[
            ("a.go", &many),
            ("main.tf", "variable \"region\" {}\n"),
            ("z.go", "func zebra() {}\nfunc Zebra() {}\n"),
        ],
    )
}

/// Past the cap the rows are the declarations found before it, not the project's, so the
/// query greps for a name instead of filtering them: what the cut never reached is found by
/// typing it, in either case. An answer to a query that has changed since is dropped, and an
/// emptied query brings the opening list back. Under the cap nothing of this happens.
#[test]
fn symbols_past_the_cap_are_grepped_not_filtered() {
    let (small, mut b) = project_app("cap-under", &[("z.go", "func zebra() {}\n")]);
    press(&mut b, KeyCode::Char('D'), KeyModifiers::NONE);
    let p = b.picker.as_ref().unwrap();
    assert_eq!(
        (p.title.as_str(), p.live),
        ("Symbols", false),
        "the whole list"
    );
    std::fs::remove_dir_all(&small).unwrap();

    let (dir, mut a) = capped_project("cap-symbols");
    // A grep of the search that was open answers to a number `D` must not be waiting for.
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "zebra");
    std::thread::sleep(SEARCH_PAUSE);
    let of_the_search = a.search_tick().expect("the grep for the `s` query").seq;
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);

    press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
    let p = a.picker.as_mut().unwrap();
    p.settle();
    assert_eq!(
        p.counts().1 as usize,
        search::MAX_HITS + 1,
        "a.go fills the cap of the shared pattern; main.tf is another pattern's row"
    );
    assert!(
        p.live,
        "the query greps the project, it does not filter these rows"
    );
    assert!(
        !a.search_done(of_the_search, App::hit_items(Vec::new())),
        "the search's answer is not the symbol list"
    );

    typed(&mut a, "zebr");
    std::thread::sleep(SEARCH_PAUSE);
    let stale = a.search_tick().expect("the grep for zebr").seq;
    typed(&mut a, "a");
    assert!(!a.search_done(stale, Vec::new()), "zebr is not on screen");
    let cut = a.picker.as_ref().unwrap().counts().1 as usize;
    assert_eq!(cut, search::MAX_HITS + 1, "a stale answer settles nothing");

    // Smart case, as everywhere else: an all-lowercase query finds both spellings.
    a.settle_search();
    let p = a.picker.as_mut().unwrap();
    assert_eq!(p.counts(), (2, 2));
    let rows = p.window(5).0;
    let mut labels: Vec<&str> = rows.iter().map(|r| r.item.label.as_str()).collect();
    labels.sort_unstable();
    assert_eq!(labels, ["Zebra  z.go:2", "zebra  z.go:1"]);
    // nucleo ranks and marks what the grep brought back, as it does under the cap.
    assert_eq!(rows[0].matched, [0, 1, 2, 3, 4]);

    // One capital of its own makes the query case-sensitive, before the picker sees it.
    press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
    typed(&mut a, "Zebra");
    a.settle_search();
    let p = a.picker.as_mut().unwrap();
    assert_eq!(p.counts(), (1, 1));
    assert_eq!(p.window(5).0[0].item.label, "Zebra  z.go:2");

    press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
    a.settle_search();
    let p = a.picker.as_ref().unwrap();
    assert_eq!(
        p.counts().1 as usize,
        search::MAX_HITS + 1,
        "the list it opened on"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The cap belongs to each declaration pattern, not to the list: a project whose kinds add
/// up past it with none of them cut has its whole list, and the query filters it as ever.
#[test]
fn a_list_no_pattern_cut_short_is_the_whole_list() {
    let half = search::MAX_HITS / 2 + 100;
    let go: String = (0..half).map(|i| format!("func a{i}() {{}}\n")).collect();
    let rb: String = (0..half).map(|i| format!("def b{i}\nend\n")).collect();
    let (dir, mut a) = project_app("cap-two-kinds", &[("a.go", &go), ("b.rb", &rb)]);
    press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
    let p = a.picker.as_mut().unwrap();
    p.settle();
    assert!(2 * half > search::MAX_HITS, "past the cap in total");
    assert_eq!(p.counts().1 as usize, 2 * half, "both kinds, whole");
    assert_eq!(
        (p.title.as_str(), p.live),
        ("Symbols", false),
        "nothing was cut, so the query still filters the rows"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The query past the cap is a name to find, not a pattern: a regex would read `a1.*` as
/// a thousand of the names in `a.go`, and an escaped one would look for a backslash in
/// `var.region`.
#[test]
fn a_symbol_query_is_literal_not_a_regex() {
    let (dir, mut a) = capped_project("cap-regex");
    press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
    for (query, hits, why) in [
        ("a1.*", 0, "a dot and a star are characters of a name"),
        (
            "var.region",
            1,
            "and a name written with a dot is typed with it",
        ),
    ] {
        typed(&mut a, query);
        a.settle_search();
        let p = a.picker.as_ref().expect(why);
        assert_eq!(p.counts().1, hits, "{why}: {}", a.message);
        press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
        a.settle_search();
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
