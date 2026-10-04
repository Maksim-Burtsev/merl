use super::*;

#[test]
fn d_in_starlark_reads_the_external_repositories_bazel_fetched() {
    let (dir, mut a) = project_app(
        "starlark-outside",
        &[
            (
                "MODULE.bazel",
                "module(name = \"shop\")\nbazel_dep(name = \"rules_go\", version = \"0.50.1\")\nbazel_dep(name = \"gazelle\", version = \"0.38.0\")\n",
            ),
            (
                "api/BUILD.bazel",
                "load(\"@rules_go//go:def.bzl\", \"go_library\")\nload(\"@gazelle//:def.bzl\", \"gazelle\")\n\ngo_library(\n    name = \"api\",\n    deps = [\"@rules_go//go/tools:sdk\"],\n)\ngazelle(name = \"g\")\n",
            ),
        ],
    );
    let base = external_root(
        "starlark",
        &[
            ("execroot/_main/.keep", ""),
            (
                "external/rules_go+/go/def.bzl",
                "load(\"//go/private:rules.bzl\", _go_library = \"go_library\")\n\ngo_library = _go_library\n",
            ),
            (
                "external/rules_go+/go/private/rules.bzl",
                "def go_library(name):\n    pass\n",
            ),
            (
                "external/rules_go+/go/tools/BUILD.bazel",
                "filegroup(\n    name = \"sdk\",\n)\n",
            ),
            (
                "external/gazelle~/def.bzl",
                "gazelle = rule(implementation = _impl)\n",
            ),
        ],
    );
    let name = dir.file_name().unwrap().to_str().unwrap();
    std::os::unix::fs::symlink(
        base.join("execroot/_main"),
        dir.join(format!("bazel-{name}")),
    )
    .unwrap();
    let ext = std::fs::canonicalize(base.join("external")).unwrap();
    let at = |p: &str| format!("{}", ext.join(p).display());
    for (code, want) in [
        (
            "^go_library|(",
            jump(
                "go_library: via import @rules_go//go:def.bzl",
                &at("rules_go+/go/def.bzl:3"),
            ),
        ),
        (
            "go_library|\")",
            jump("no definition for go_library", "api/BUILD.bazel:1"),
        ),
        (
            "@rules_|go//go:def",
            jump(
                &format!("def.bzl: module {}", at("rules_go+/go/def.bzl")),
                &at("rules_go+/go/def.bzl:1"),
            ),
        ),
        (
            "go/tools:s|dk",
            jump("sdk: target", &at("rules_go+/go/tools/BUILD.bazel:2")),
        ),
        (
            "^gazelle|(",
            jump(
                "gazelle: via import @gazelle//:def.bzl",
                &at("gazelle~/def.bzl:1"),
            ),
        ),
    ] {
        d_on(&mut a, "api/BUILD.bazel", code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
    assert_eq!(a.buf.readonly, Some("outside the project"));
    d_on(&mut a, "api/BUILD.bazel", "^go_library|(");
    let def = a.buf.path.clone().unwrap();
    a.jump_to(&def, 1);
    a.col = a.line_str().find("private").unwrap();
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        shown(&mut a),
        jump(
            &format!("rules.bzl: module {}", at("rules_go+/go/private/rules.bzl")),
            &at("rules_go+/go/private/rules.bzl:1"),
        )
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&base).unwrap();
}

#[test]
fn d_in_starlark_reads_labels_from_the_nearest_workspace() {
    let (dir, mut a) = project_app(
        "starlark-nested",
        &[
            ("README.md", "# monorepo\n"),
            (
                "shop/MODULE.bazel",
                "bazel_dep(name = \"rules_go\", version = \"0.50.1\")\n",
            ),
            ("shop/tools/defs.bzl", "def shop_binary(name):\n    pass\n"),
            (
                "shop/api/BUILD",
                "load(\"//tools:defs.bzl\", \"shop_binary\")\nload(\"@rules_go//go:def.bzl\", \"go_library\")\n\nshop_binary(name = \"x\")\ngo_library(name = \"y\", deps = [\"@rules_go\"])\n",
            ),
        ],
    );
    for (code, want) in [
        (
            "^shop_binary|(",
            jump(
                "shop_binary: via import shop/tools/defs.bzl",
                "shop/tools/defs.bzl:1",
            ),
        ),
        (
            "^go_library|(",
            jump("no definition for go_library", "shop/api/BUILD:5"),
        ),
        (
            "\"@rules_|go//",
            jump(
                "no definition for @rules_go//go:def.bzl",
                "shop/api/BUILD:2",
            ),
        ),
        (
            "\"@rules_|go\"]",
            jump("rules_go: repository", "shop/MODULE.bazel:1"),
        ),
    ] {
        d_on(&mut a, "shop/api/BUILD", code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
