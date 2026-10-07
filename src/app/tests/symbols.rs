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
            (
                "invoice.h",
                "#define LIMIT 10\nstruct invoice {\n    int total;\n};\nint sum(struct invoice *i);\n",
            ),
            (
                "init.lua",
                "local M = {}\n\nfunction M.setup(opts)\n  return opts\nend\n",
            ),
            (
                "ledger.ex",
                "defmodule Ledger do\n  @timeout 5\n\n  defp normalise(raw), do: raw\nend\n",
            ),
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
    assert_eq!(
        names,
        [
            "&base",
            "build",
            "build",
            "invoice",
            "it",
            "Ledger",
            "LIMIT",
            "normalise",
            "serve",
            "setup",
            "total",
            "var.region"
        ],
        "`apiVersion:` is no target outside a Makefile; C lists `struct` once and no prototype; \
         Lua's `function M.setup(` declares `setup`, not `M`; Elixir has its `defp`; Zig's \
         `pub fn` comes from the shared pattern, its test `it adds up` from a row of its own"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn symbols_of_a_component_are_its_script() {
    let (dir, mut a) = project_app(
        "symbols-component",
        &[
            (
                "Card.vue",
                "<script setup lang=\"ts\">\nfunction save() {}\n</script>\n\n<template>\n  function shout() {}\n</template>\n<style>\n.card {\n  display: flex;\n}\n</style>\n",
            ),
            (
                "Hero.astro",
                "---\nconst title = 1;\n---\n<h1>{title}</h1>\nclass Fake {}\n",
            ),
        ],
    );
    press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
    let picker = a.picker.as_mut().unwrap();
    picker.settle();
    let names: Vec<String> = (picker.window(20).0.into_iter())
        .map(|r| r.item.label.split_whitespace().next().unwrap().to_string())
        .collect();
    assert_eq!(
        names,
        ["save", "title"],
        "nothing of a component's template or its `<style>`"
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

#[test]
fn symbols_past_the_cap_are_grepped_not_filtered() {
    let (small, mut b) = project_app("cap-under", &[("z.go", "func zebra() {}\n")]);
    press(&mut b, KeyCode::Char('D'), KeyModifiers::NONE);
    let p = b.picker.as_ref().unwrap();
    assert_eq!(
        (p.title.as_str(), p.live),
        ("Symbols", false),
        "under the cap, the whole list, filtered"
    );
    std::fs::remove_dir_all(&small).unwrap();

    let (dir, mut a) = capped_project("cap-symbols");
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
        "the answer to the grep of the `s` query open before is not the symbol list's"
    );

    typed(&mut a, "zebr");
    std::thread::sleep(SEARCH_PAUSE);
    let stale = a.search_tick().expect("the grep for zebr").seq;
    typed(&mut a, "a");
    assert!(!a.search_done(stale, Vec::new()), "zebr is not on screen");
    let cut = a.picker.as_ref().unwrap().counts().1 as usize;
    assert_eq!(cut, search::MAX_HITS + 1, "a stale answer settles nothing");

    a.settle_search();
    let p = a.picker.as_mut().unwrap();
    assert_eq!(
        p.counts(),
        (2, 2),
        "case is ignored, as everywhere else: the query finds both spellings"
    );
    let rows = p.window(5).0;
    let mut labels: Vec<&str> = rows.iter().map(|r| r.item.label.as_str()).collect();
    labels.sort_unstable();
    assert_eq!(labels, ["Zebra  z.go:2", "zebra  z.go:1"]);
    assert_eq!(
        rows[0].matched,
        [0, 1, 2, 3, 4],
        "nucleo ranks and marks what the grep brought back, as it does under the cap"
    );

    press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
    typed(&mut a, "zeBra");
    a.settle_search();
    let p = a.picker.as_mut().unwrap();
    assert_eq!(
        p.counts(),
        (2, 2),
        "a capital of its own does not make the query exact: `zeBra` still finds both (#174)"
    );

    press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
    typed(&mut a, "qqqqzz");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    a.settle_search();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    let query = a.picker.as_ref().map(|p| p.query.to_string());
    assert_eq!(
        (a.mode, query.as_deref()),
        (Mode::Picker(PickerKind::Symbols), Some("qqqqzz")),
        "a name no declaration has: Enter keeps the list and the query, as under the cap (#288)"
    );

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
    assert_eq!(
        p.counts().1 as usize,
        2 * half,
        "the cap belongs to each pattern, not to the list: both kinds, whole"
    );
    assert_eq!(
        (p.title.as_str(), p.live),
        ("Symbols", false),
        "nothing was cut, so the query still filters the rows"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

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

#[test]
fn enter_past_the_cap_opens_the_same_row_before_and_after_the_answer() {
    let many: String = (0..search::MAX_HITS)
        .map(|i| format!("func a{i}() {{}}\n"))
        .collect();
    let (dir, mut a) = project_app(
        "cap-enter",
        &[
            ("a.go", &many),
            ("z.go", "func azebra() {}\nfunc zebra() {}\n"),
        ],
    );
    let mut jump = |pending: bool| {
        press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
        typed(&mut a, "zebra");
        if pending {
            press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
            a.settle_search();
        } else {
            a.settle_search();
            press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        }
        assert_eq!(a.mode, Mode::Normal, "Enter opened a row");
        at(&a)
    };
    let (before, after) = (jump(true), jump(false));
    assert_eq!(
        before, after,
        "the same row, whether the answer came before Enter or after"
    );
    assert_eq!(
        after,
        (dir.join("z.go"), 1),
        "nucleo ranks `zebra`, matched from its first letter, over `azebra`, which the grep hands \
         first"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_symbol_query_reads_no_pattern_syntax() {
    let tests = "test \"costs 5$\" {}\ntest \"^up\" {}\ntest \"!bang\" {}\ntest \"'quoted\" {}\ntest \"a\\\\$\" {}\n";
    let many: String = (0..search::MAX_HITS)
        .map(|i| format!("func a{i}() {{}}\n"))
        .collect();
    for files in [
        vec![("t.zig", tests)],
        vec![("a.go", &many), ("t.zig", tests)],
    ] {
        let live = files.len() > 1;
        let (dir, mut a) = project_app("syntax", &files);
        for (line, query) in [
            (0, "5$"),
            (1, "^up"),
            (2, "!bang"),
            (3, "'quoted"),
            (4, "a\\\\$"),
        ] {
            for pending in [true, false] {
                press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
                typed(&mut a, query);
                a.picker.as_mut().unwrap().settle();
                if pending {
                    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
                    a.settle_search();
                } else {
                    a.settle_search();
                    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
                }
                assert_eq!(
                    (a.mode, at(&a)),
                    (Mode::Normal, (dir.join("t.zig"), line)),
                    "nucleo's `^`, `$`, a leading `!` or `'` and `\\` read nothing into \
                     {query}, live {live}, pending {pending}"
                );
            }
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[test]
fn d_lists_a_scala_def_value_and_no_groovy_def_variable() {
    let (dir, mut a) = project_app(
        "symbols-groovy",
        &[
            ("Ledger.scala", "object Ledger:\n  def tally = 0\n"),
            (
                "build.gradle",
                "def stamp = 0\ndef bump() {\n}\ntasks.register('pack')\n",
            ),
        ],
    );
    press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
    let picker = a.picker.as_mut().unwrap();
    picker.settle();
    let mut names: Vec<String> = picker
        .window(20)
        .0
        .into_iter()
        .map(|r| r.item.label.split_whitespace().next().unwrap().to_string())
        .collect();
    names.sort();
    assert_eq!(names, ["Ledger", "bump", "pack", "tally"]);
    let _ = std::fs::remove_dir_all(dir);
}
