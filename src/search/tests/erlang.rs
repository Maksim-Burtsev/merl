use super::*;

fn declares(line: &str, word: &str) -> bool {
    let lines = [line];
    Regex::new(&def_patterns(Kind::Elixir, word).join("|"))
        .unwrap()
        .is_match(line)
        && declares_where(
            Kind::Elixir,
            Path::new("a.erl"),
            word,
            1,
            line,
            || &lines[..],
        )
}

#[test]
fn erlang_declaration_forms() {
    for (line, word) in [
        ("total(#order{total = Total}) ->", "total"),
        ("parse(Bin, Opts) when is_binary(Bin) ->", "parse"),
        ("-module(shop_order).", "shop_order"),
        ("-record(order, {id, total = 0}).", "order"),
        ("-record(order, {id, total = 0}).", "total"),
        ("-define(DEFAULT_TOTAL, 0).", "DEFAULT_TOTAL"),
        ("-define(LOG(Fmt, Args), io:format(Fmt, Args)).", "LOG"),
        ("-type order() :: #order{}.", "order"),
        ("-opaque t() :: binary().", "t"),
    ] {
        assert!(declares(line, word), "{line} declares {word}");
    }
    for (line, word) in [
        ("-spec total(order()) -> integer().", "total"),
        ("-callback total(order()) -> integer().", "total"),
        ("-export([total/1]).", "total"),
        ("    total(X),", "total"),
        ("main().", "main"),
        ("    F = fun total/1,", "total"),
        ("    #order{total = T},", "order"),
        ("    ?DEFAULT_TOTAL,", "DEFAULT_TOTAL"),
    ] {
        assert!(!declares(line, word), "{line} declares no {word}");
    }
}

#[test]
fn an_erlang_clause_head_wraps_to_its_arrow() {
    let lines = [
        "parse(Bin,",
        "      Opts) % (",
        "  when Bin =/= <<>> ->",
        "    ok.",
    ];
    assert!(erlang_clause(&lines, 1));
    assert!(!erlang_clause(&["main(),", "ok."], 1));
    assert!(erlang_clause(&["quote($)) ->"], 1));
    assert!(erlang_clause(&["f(A, % )", "  B) ->"], 1));
    assert!(erlang_clause(&[r#"f(")", ')', $\") ->"#], 1));
    assert!(!erlang_clause(&["f(A.", ") ->"], 1));
}

#[test]
fn erlang_literals() {
    let text = "% a \"\"\" in a comment\nbanner() -> $\".\nx() -> \"\nmake(R) ->\n\".\ny() -> \"\"\"\n  make(R) ->\n  \"\"\".\nz() -> #r{}.";
    let hidden: Vec<usize> = (file_literal_lines(Kind::Elixir, Path::new("a.erl"), text).iter())
        .enumerate()
        .filter(|(_, h)| **h)
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(hidden, [4, 5, 7, 8]);
    let shown = |text: &str| {
        !file_literal_lines(Kind::Elixir, Path::new("a.erl"), text)
            .last()
            .copied()
            .unwrap()
    };
    assert!(shown("y() -> \"\"\"\n  say \"hi\n  \"\"\".\nz() -> ok."));
    assert!(shown("x() -> '\"'.\ny() -> ok."));
    assert!(shown("x() -> \"a\\\"b\".\ny() -> ok."));
    assert!(shown("x() -> $\\\".\ny() -> ok."));
}

#[test]
fn erlang_record_fields_over_lines() {
    let lines = [
        "-record(crate,",
        "        {label,",
        "         parcels = [] :: [term()],",
        "         sealed = false}).",
        "x() -> sealed.",
    ];
    assert_eq!(erlang_record_field(&lines, 1, "label"), Some(2));
    assert_eq!(erlang_record_field(&lines, 1, "sealed"), Some(4));
    assert_eq!(erlang_record_field(&lines, 1, "term"), None);
    assert_eq!(
        erlang_record_field(&["-record(r, {a}).", "-record(s, {b})."], 1, "b"),
        None
    );
    assert_eq!(
        erlang_record_field(&["-record(r, {a, % b,", "  c})."], 1, "b"),
        None
    );
    assert_eq!(
        erlang_record_field(&["-record(r, {a = 1, b})."], 1, "b"),
        Some(1)
    );
}

#[test]
fn erlang_includes_name_files() {
    let files: Vec<PathBuf> = ["src/a.erl", "include/shop.hrl", "src/local.hrl"]
        .map(PathBuf::from)
        .to_vec();
    let outside = [PathBuf::from("/otp/lib/kernel-9.0/include/file.hrl")];
    let here = Path::new("src/a.erl");
    assert_eq!(
        erlang_include_files(false, "local.hrl", here, &files, &outside),
        [PathBuf::from("src/local.hrl")]
    );
    assert_eq!(
        erlang_include_files(false, "shop.hrl", here, &files, &outside),
        [PathBuf::from("include/shop.hrl")]
    );
    assert_eq!(
        erlang_include_files(true, "kernel/include/file.hrl", here, &files, &outside),
        outside
    );
    assert!(
        erlang_include_files(true, "stdlib/include/file.hrl", here, &files, &outside).is_empty()
    );
    assert_eq!(
        erlang_include(r#"-include_lib("kernel/include/file.hrl")."#, 16),
        Some((true, "kernel/include/file.hrl".into()))
    );
    assert_eq!(erlang_include(r#"-include("shop.hrl")."#, 2), None);
}

#[test]
fn erlang_symbols() {
    for (line, name) in [
        ("-module(shop_order).", "shop_order"),
        ("-record(order, {id}).", "order"),
        ("-define(LOG(F), F).", "LOG"),
        ("-type order() :: term().", "order"),
        ("total(#order{total = T}) -> T.", "total"),
    ] {
        assert_eq!(one(Kind::Elixir, line).as_deref(), Some(name), "{line}");
    }
    for line in [
        "-spec total(o()) -> integer().",
        "main().",
        "-export([total/1]).",
        "spawn(fn -> work() end)",
    ] {
        assert_eq!(one(Kind::Elixir, line), None, "{line}");
    }
}

#[test]
fn erlang_is_elixir_kind() {
    for f in ["a.erl", "a.hrl", "a.escript"] {
        assert_eq!(kind_of(Path::new(f)), Some(Kind::Elixir));
        assert!(erlang(Path::new(f)));
    }
    assert!(kind_of(Path::new("rebar.config")).is_none());
    assert!(kind_of(Path::new("shop.app.src")).is_none());
}

#[test]
fn erlang_dependencies_are_rebar3s_and_erlang_mks() {
    let root = std::env::temp_dir().join(format!("merl-erlang-deps-{}", std::process::id()));
    for d in [
        "_build/default/lib/ranch/src",
        "deps/cowlib/src",
        "apps/web/src",
    ] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    std::fs::write(root.join("rebar.config"), "").unwrap();
    std::fs::write(root.join("erlang.mk"), "").unwrap();
    assert_eq!(
        rebar_deps(&root, &root.join("apps/web/src")),
        [root.join("_build/default/lib"), root.join("deps")]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn an_erlang_import_names_the_module_of_a_call() {
    let text = "-module(a).\n-import(lists, [map/2,\n               foldl/3]).\n";
    assert_eq!(erlang_imported(text, "foldl").as_deref(), Some("lists"));
    assert_eq!(erlang_imported(text, "map").as_deref(), Some("lists"));
    assert_eq!(erlang_imported(text, "filter"), None);
}

#[test]
fn elixir_and_erlang_search_by_name_in_their_own_language() {
    let ex = Path::new("lib/a.ex");
    let erl = Path::new("src/a.erl");
    assert!(in_def_scope(Kind::Elixir, ex, Path::new("lib/b.exs")));
    assert!(!in_def_scope(Kind::Elixir, ex, Path::new("src/b.erl")));
    assert!(in_def_scope(Kind::Elixir, erl, Path::new("include/b.hrl")));
    assert!(!in_def_scope(Kind::Elixir, erl, Path::new("lib/b.ex")));
}
