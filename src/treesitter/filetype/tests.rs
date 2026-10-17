use std::path::Path;

use crate::treesitter::tests::sexp;

fn by_python(f: &str) -> Option<&'static str> {
    super::for_syntax("Python", Some(Path::new(f)), &[]).map(|l| l.name)
}

#[test]
fn nginx_files_get_their_grammar_as_neovim_names_them() {
    for f in [
        "nginx.conf",
        "test-nginx.conf",
        "nginx-site.conf",
        "a/nginx/b.conf",
        "/etc/nginx/x.conf",
        "/usr/local/nginx/conf/mime.types",
        "a.nginx",
    ] {
        assert!(sexp(f, b"a b;\n").starts_with("(source_file"), "{f}");
    }
    assert_eq!(sexp("a.conf", b"a b;\n"), "");
}

#[test]
fn bazel_and_starlark_files_get_starlark_and_other_python_stays_python() {
    for f in [
        "BUILD",
        "BUILD.bazel",
        "MODULE.bazel",
        "WORKSPACE",
        "WORKSPACE.bzlmod",
        "BUCK",
        "a.bzl",
        "a.bxl",
        "a.star",
        "a.sky",
        "a.ipd",
        "a.starlark",
        "third_party.BUILD",
    ] {
        assert!(sexp(f, b"a = 1\n").starts_with("(module "), "{f}");
        assert_eq!(by_python(f), Some("starlark"), "{f}");
    }
    for f in ["a.py", "Tiltfile", "SConstruct"] {
        assert_eq!(by_python(f), Some("python"), "{f}");
    }
}

#[test]
fn solidity_files_get_their_grammar() {
    assert!(sexp("a.sol", b"contract A {}\n").contains("(contract_declaration "));
}
