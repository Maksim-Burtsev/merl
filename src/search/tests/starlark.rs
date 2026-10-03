use super::*;

fn declares(line: &str, word: &str) -> bool {
    Regex::new(&starlark_patterns(word).join("|"))
        .unwrap()
        .is_match(line)
}

#[test]
fn starlark_kinds_by_name() {
    for name in [
        "BUILD",
        "BUILD.bazel",
        "WORKSPACE",
        "WORKSPACE.bazel",
        "MODULE.bazel",
        "REPO.bazel",
        "defs.bzl",
        "x.star",
        "Tiltfile",
        "BUCK",
    ] {
        assert_eq!(kind_of(Path::new(name)), Some(Kind::Starlark), "{name}");
    }
    assert_eq!(kind_of(Path::new("BUILD.md")), Some(Kind::Markdown));
}

#[test]
fn starlark_declaration_forms() {
    for (line, word) in [
        ("def shop_binary(name, srcs, **kwargs):", "shop_binary"),
        ("    def _inner(ctx):", "_inner"),
        (
            "SHOP_VISIBILITY = [\"//visibility:public\"]",
            "SHOP_VISIBILITY",
        ),
        ("go_library = rule(", "go_library"),
        ("GoInfo = provider(", "GoInfo"),
        ("go_sdk =", "go_sdk"),
    ] {
        assert!(declares(line, word), "{line}: {word}");
    }
    for (line, word) in [
        ("go_library(", "go_library"),
        ("    srcs = [\"api.go\"],", "srcs"),
        ("    visibility = SHOP_VISIBILITY,", "visibility"),
        ("x = foo(srcs = 1)", "srcs"),
        ("SHOP == 1", "SHOP"),
        ("    native.genrule(name = name + \"_gen\")", "genrule"),
        ("def shop_binary_x(name):", "shop_binary"),
    ] {
        assert!(!declares(line, word), "{line}: {word}");
    }
}

#[test]
fn starlark_symbol_names() {
    let star = |line| one(Kind::Starlark, line);
    for (line, name) in [
        ("def shop_binary(name):", "shop_binary"),
        ("go_library = rule(", "go_library"),
        ("GoInfo = provider(fields = [])", "GoInfo"),
        ("my_aspect = aspect(implementation = _impl)", "my_aspect"),
        ("go_repository = repository_rule(", "go_repository"),
        ("go_sdk = module_extension(", "go_sdk"),
        ("    name = \"api\",", "api"),
        ("go_library(name = \"api\", srcs = [\"api.go\"])", "api"),
    ] {
        assert_eq!(star(line).as_deref(), Some(name), "{line}");
    }
    for line in [
        "SHOP_VISIBILITY = [\"//visibility:public\"]",
        "go_library = _go_library",
        "go_library(",
        "    name = name + \"_gen\",",
        "    repo_name = \"x\",",
    ] {
        assert_eq!(star(line), None, "{line}");
    }
}

#[test]
fn a_starlark_label_reads_its_repository_package_and_name() {
    let target = |repo: Option<&str>, package: Option<&str>, name: &str| {
        Some(StarlarkLabel::Target {
            repo: repo.map(str::to_owned),
            package: package.map(str::to_owned),
            name: name.to_owned(),
        })
    };
    for (label, want) in [
        (":api", target(None, None, "api")),
        ("api", target(None, None, "api")),
        ("api.go", target(None, None, "api.go")),
        ("sub/file.go", target(None, None, "sub/file.go")),
        ("//pkg/api:api", target(None, Some("pkg/api"), "api")),
        ("//pkg/api", target(None, Some("pkg/api"), "api")),
        ("//:defs.bzl", target(None, Some(""), "defs.bzl")),
        (
            "@rules_go//go:def.bzl",
            target(Some("rules_go"), Some("go"), "def.bzl"),
        ),
        (
            "@@rules_go+//go",
            target(Some("rules_go+"), Some("go"), "go"),
        ),
        ("@//api", target(Some(""), Some("api"), "api")),
        (
            "@rules_go//",
            target(Some("rules_go"), Some(""), "rules_go"),
        ),
        ("@rules_go", Some(StarlarkLabel::Repo("rules_go".into()))),
        ("@@rules_go", Some(StarlarkLabel::Repo("rules_go".into()))),
    ] {
        assert_eq!(starlark_label(label), want, "{label}");
    }
    for label in [
        "",
        "cp $< $@",
        "*.go",
        "@",
        "/abs/path",
        "../up",
        "//pkg/../x:y",
        "a:b:c",
        "Hello world",
        "%s.out",
    ] {
        assert_eq!(starlark_label(label), None, "{label}");
    }
}

#[test]
fn a_starlark_string_is_read_from_quote_to_quote() {
    let line = r#"    srcs = [":api", 'b.go'], cmd = "a \" b",  # "c""#;
    let at = |col| starlark_string_at(line, col).map(|(r, plain)| (&line[r], plain));
    assert_eq!(at(12), Some((":api", true)));
    assert_eq!(at(13), Some((":api", true)));
    assert_eq!(at(17), Some((":api", true)));
    assert_eq!(at(18), None);
    assert_eq!(at(21), Some(("b.go", true)));
    assert_eq!(at(37), Some((r#"a \" b"#, true)));
    assert_eq!(at(4), None);
    assert_eq!(at(48), None);
    let doc = r#"x = """//api:api""" + 'y"#;
    let at = |col| starlark_string_at(doc, col).map(|(r, plain)| (&doc[r], plain));
    assert_eq!(at(9), Some(("//api:api", false)));
    assert_eq!(at(22), Some(("y", false)));
}

#[test]
fn starlark_loads_bind_their_names_and_aliases() {
    let text = "load(\"//tools:defs.bzl\", \"shop_binary\", my_alias = \"go_binary\")\n\
                load(\n    \":x.bzl\",  # the macros\n    \"a\",\n    b = 'c',\n)\n\
                \"\"\"\nload(\"//no:t.bzl\", \"z\")\n\"\"\"\n    load(\"//indented:t.bzl\", \"z\")\n";
    let loads = starlark_loads(text);
    assert_eq!(
        loads,
        [
            StarlarkLoad {
                label: "//tools:defs.bzl".into(),
                names: vec![
                    ("shop_binary".into(), "shop_binary".into()),
                    ("my_alias".into(), "go_binary".into()),
                ],
                name_strings: vec![(0, 26), (0, 52)],
            },
            StarlarkLoad {
                label: ":x.bzl".into(),
                names: vec![("a".into(), "a".into()), ("b".into(), "c".into())],
                name_strings: vec![(3, 5), (4, 9)],
            },
        ]
    );
}

#[test]
fn starlark_finds_a_target_and_a_repository_by_its_name_line() {
    let build = "load(\":defs.bzl\", \"x\")\n\
                 go_library(\n    name = \"api\",\n)\n\
                 go_test(name = \"api_test\", embed = [\":api\"])\n\
                 \"\"\"\n    name = \"doc\",\n\"\"\"\n\
                 x(\n    name = name + \"_gen\",\n)\n";
    assert_eq!(starlark_target_line(build, "api"), Some(3));
    assert_eq!(starlark_target_line(build, "api_test"), Some(5));
    assert_eq!(starlark_target_line(build, "doc"), None);
    assert_eq!(starlark_target_line(build, "_gen"), None);
    let module = "module(name = \"shop\", version = \"1\")\n\
                  # bazel_dep(name = \"old\")\n\
                  bazel_dep(name = \"rules_go\", version = \"0.50.1\")\n\
                  bazel_dep(name = \"gazelle\", repo_name = \"bazel_gazelle\")\n";
    assert_eq!(starlark_repo_line(module, "rules_go"), Some(3));
    assert_eq!(starlark_repo_line(module, "bazel_gazelle"), Some(4));
    assert_eq!(starlark_repo_line(module, "old"), None);
    assert_eq!(starlark_project_names(module), ["shop"]);
    let module = "module(\n    name = \"rules_go\",\n    repo_name = \"io_bazel_rules_go\",\n)\n";
    assert_eq!(
        starlark_project_names(module),
        ["rules_go", "io_bazel_rules_go"]
    );
    let workspace = "workspace(\n    name = \"io_bazel_rules_go\",\n)\n";
    assert_eq!(starlark_project_names(workspace), ["io_bazel_rules_go"]);
}

#[test]
fn a_bzlmod_repository_is_found_by_its_canonical_name() {
    let (dir, _) = scratch(
        "starlark-repos",
        &[
            ("external/rules_go+/go/def.bzl", ""),
            ("external/rules_go++go_sdk+go_sdk/x", ""),
            ("external/gazelle~/def.bzl", ""),
            ("external/rules_python/x", ""),
            ("external/rules_gox+/x", ""),
        ],
    );
    let ext = dir.join("external");
    let repo = |r| starlark_repo_dir(&ext, r).map(|p| p.strip_prefix(&ext).unwrap().to_owned());
    assert_eq!(repo("rules_go"), Some(PathBuf::from("rules_go+")));
    assert_eq!(repo("gazelle"), Some(PathBuf::from("gazelle~")));
    assert_eq!(repo("rules_python"), Some(PathBuf::from("rules_python")));
    assert_eq!(repo("rules"), None);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_starlark_keyword_argument_names_the_callee_s_parameter() {
    let text = "X = 1\nload(\":a.bzl\", alias = \"b\")\ndef f(name, srcs = []):\n    native.genrule(name = name, outs = [x == 1])\n    y = 2\n    g(\n        srcs = srcs,\n    )\n";
    let kwarg = |word: &str, nth: usize| {
        let at = text.match_indices(word).nth(nth).unwrap().0;
        starlark_keyword_argument(text, at, at + word.len())
    };
    assert!(kwarg("name", 1));
    assert!(kwarg("srcs", 1));
    assert!(!kwarg("name", 2));
    assert!(!kwarg("X", 0));
    assert!(!kwarg("alias", 0));
    assert!(!kwarg("srcs", 0));
    assert!(!kwarg("x", 0));
    assert!(!kwarg("y", 0));
}
