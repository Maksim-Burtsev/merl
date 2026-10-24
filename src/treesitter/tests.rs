use std::path::PathBuf;

use ratatui::style::{Color, Modifier};

use super::{resolve, resolve_by_extension};
use crate::buffer::Buffer;
use crate::theme;

mod lazyvim;

#[test]
fn a_capture_keeps_its_background_reversed_or_not() {
    let added = |name: &str| {
        let theme = theme::load(name).unwrap();
        let text = b"--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n";
        let mut b = Buffer::from_bytes(PathBuf::from("a.diff"), text);
        b.parse_more(None);
        b.highlight_to(usize::MAX, &theme);
        let (style, _) = b.hl[4].iter().find(|(_, r)| r.contains(&1)).unwrap();
        (style.bg, style.add_modifier.contains(Modifier::REVERSED))
    };
    assert_eq!(added("nord"), (Some(Color::Rgb(0x3b, 0x42, 0x52)), true));
    assert_eq!(
        added("tokyonight-moon"),
        (Some(Color::Rgb(0x2a, 0x45, 0x56)), false)
    );
}

pub(super) fn sexp(f: &str, src: &[u8]) -> String {
    let mut b = Buffer::from_bytes(PathBuf::from(f), src);
    b.parse_more(None);
    b.sexp()
}

#[test]
fn script_files_get_their_grammar_and_astro_and_zsh_stay_on_syntect() {
    let jsx = b"const a = <b />;\n";
    for f in ["a.tsx", "a.jsx", "a.js", "a.mjs", "a.cjs"] {
        assert!(sexp(f, jsx).contains("jsx_self_closing_element"), "{f}");
    }
    let ts = b"let a = <b>c;\n";
    for f in ["a.ts", "a.mts", "a.cts"] {
        assert!(sexp(f, ts).contains("type_assertion"), "{f}");
    }
    assert_eq!(sexp("a.astro", jsx), "");
    for f in [
        "a.sh", "a.bash", ".bashrc", ".profile", "PKGBUILD", "a.ebuild",
    ] {
        assert!(sexp(f, b"echo hi\n").contains("(command "), "{f}");
    }
    for f in [
        "a.zsh",
        ".zshrc",
        ".zshenv",
        "zshrc",
        ".zprofile",
        ".zlogin",
    ] {
        assert_eq!(sexp(f, b"echo hi\n"), "", "{f}");
    }
}

#[test]
fn data_files_get_their_grammar() {
    for f in ["a.json", "a.jsonc", "tsconfig.json", "a.ipynb"] {
        assert!(sexp(f, b"{\"a\": 1}\n").contains("(object "), "{f}");
    }
    for f in ["a.yaml", "a.yml"] {
        assert!(sexp(f, b"a: 1\n").contains("(block_mapping "), "{f}");
    }
    for f in ["a.toml", "Cargo.lock"] {
        assert!(sexp(f, b"a = 1\n").contains("(pair "), "{f}");
    }
}

#[test]
fn c_family_jvm_and_dotnet_files_get_their_grammar() {
    assert!(sexp("a.c", b"int x;\n").contains("(translation_unit "));
    for f in [
        "a.h", "a.cc", "a.cpp", "a.cxx", "a.c++", "a.hh", "a.hpp", "a.hxx", "a.inl",
    ] {
        assert!(sexp(f, b"int x;\n").contains("(translation_unit "), "{f}");
    }
    assert_eq!(sexp("a.cp", b"int x;\n"), "");
    assert_eq!(sexp("a.h", b"#import <A.h>\n@interface A\n@end\n"), "");
    assert!(sexp("A.java", b"class A {}\n").contains("(program "));
    assert_eq!(sexp("a.bsh", b"class A {}\n"), "");
    for f in ["A.cs", "build.csx"] {
        assert!(
            sexp(f, b"class A {}\n").contains("(compilation_unit "),
            "{f}"
        );
    }
}

#[test]
fn build_and_config_files_get_their_grammar() {
    for f in [
        "Dockerfile",
        "dockerfile",
        "Containerfile",
        "Dockerfile.prod",
        "Containerfile.dev",
        "api.Dockerfile",
        "api.dockerfile",
    ] {
        assert!(sexp(f, b"FROM a\n").contains("(from_instruction "), "{f}");
    }
    for f in ["CMakeLists.txt", "a.cmake"] {
        assert!(sexp(f, b"set(A 1)\n").contains("(normal_command "), "{f}");
    }
    assert_eq!(sexp("CMakeCache.txt", b"A:BOOL=ON\n"), "");
    assert!(sexp("a.sql", b"SELECT 1;\n").contains("(program "));
    for f in ["a.psql", "a.pgsql", "a.mysql", "a.ddl"] {
        assert_eq!(sexp(f, b"SELECT 1;\n"), "", "{f}");
    }
    assert!(sexp("a.proto", b"message A {}\n").contains("(message "));
    assert_eq!(sexp("a.protodevel", b"message A {}\n"), "");
    for f in ["a.graphql", "a.graphqls", "a.gql"] {
        assert!(
            sexp(f, b"type A { b: Int }\n").contains("(object_type_definition "),
            "{f}"
        );
    }
    assert!(sexp("a.gd", b"var a = 1\n").starts_with("(source "));
    for f in ["a.hcl", "a.tf", "a.tfvars"] {
        assert!(sexp(f, b"a = 1\n").contains("(config_file "), "{f}");
    }
    assert_eq!(sexp("a.tf", b"; tinyfugue\n/def x\n"), "");
    assert_eq!(sexp("a.tf", b"; tinyfugue\n\n  ; x\n\t/def y\n"), "");
    for f in [
        "a.ini",
        "A.INI",
        "setup.cfg",
        ".coveragerc",
        ".pylintrc",
        ".npmrc",
    ] {
        assert!(sexp(f, b"[a]\nb = 1\n").contains("(document "), "{f}");
    }
    assert_eq!(sexp("a.cfg", b"[a]\nb = 1\n"), "");
    assert_eq!(sexp(".editorconfig", b"[a]\nb = 1\n"), "");
}

#[test]
fn nix_lisp_erlang_and_r_files_get_their_grammar() {
    assert!(sexp("a.nix", b"{ a = 1; }\n").contains("(source_code "));
    for f in ["a.lisp", "a.asd", "init.el", ".emacs"] {
        assert!(sexp(f, b"(defun a () 1)\n").contains("(source "), "{f}");
    }
    assert!(sexp("a.rkt", b"#lang racket\n(define a 1)\n").contains("(program "));
    for f in ["a.erl", "a.hrl"] {
        assert!(sexp(f, b"-module(a).\n").contains("(source_file "), "{f}");
    }
    let escript = b"#!/usr/bin/env escript\nmain(_) -> ok.\n";
    assert!(sexp("a.escript", escript).contains("(source_file "));
    assert_eq!(sexp("a.escript", b"main(_) -> ok.\n"), "");
    for f in ["a.R", "a.r", ".Rprofile"] {
        assert!(sexp(f, b"# a\nx <- 1\n").contains("(program "), "{f}");
    }
    assert!(sexp("a.r", b"x <- 1\n").contains("(program "));
    assert_eq!(sexp("a.r", b"/* rexx */\nsay 1\n"), "");
    assert_eq!(sexp("a.r", b"# a\nREBOL [Title: \"a\"]\n"), "");
    for f in ["a.scm", "a.ss", "a.sld"] {
        assert!(sexp(f, b"(define a 1)\n").contains("(program "), "{f}");
    }
}

#[test]
fn kotlin_swift_zig_ruby_and_php_files_get_their_grammar() {
    for f in ["A.kt", "build.gradle.kts"] {
        assert!(sexp(f, b"class A\n").contains("(source_file "), "{f}");
    }
    assert!(sexp("A.swift", b"class A {}\n").contains("(source_file "));
    for f in ["A.scala", "build.sbt", "run.sc", "build.mill"] {
        assert!(
            sexp(f, b"val a = 1\n").contains("(compilation_unit "),
            "{f}"
        );
    }
    for src in ["var <x;\n", "{ |freq| SinOsc.ar(freq) }\n", "^this.value\n"] {
        assert_eq!(sexp("a.sc", src.as_bytes()), "", "{src}");
    }
    assert!(sexp("A.scala", b"var <x;\n").contains("(compilation_unit "));
    for f in ["a.zig", "build.zig.zon"] {
        assert!(sexp(f, b"const a = 1;\n").contains("(source_file "), "{f}");
    }
    for f in [
        "a.rb",
        "a.rbi",
        "a.rjs",
        "a.gemspec",
        "a.rake",
        "Brewfile",
        "Gemfile",
        "irbrc",
        "rakefile",
        "Rakefile",
        "rantfile",
        "Rantfile",
        "Vagrantfile",
        "config.ru",
    ] {
        assert!(sexp(f, b"x = 1\n").contains("(program "), "{f}");
    }
    for f in ["Dangerfile", "Podfile", "a.thor"] {
        assert_eq!(sexp(f, b"x = 1\n"), "", "{f}");
    }
    for f in [
        "a.php", "a.phtml", "a.phpt", "a.php3", "a.php4", "a.php5", "a.php7",
    ] {
        assert!(sexp(f, b"<?php echo 1;\n").contains("(program "), "{f}");
    }
    assert_eq!(sexp("a.blade.php", b"<?php echo 1;\n"), "");
}

#[test]
fn vue_and_svelte_files_get_their_grammar() {
    assert!(sexp("A.vue", b"<template><p>{{ a }}</p></template>\n").contains("(interpolation "));
    assert!(sexp("B.svelte", b"{#if a}<p>b</p>{/if}\n").contains("(if_statement "));
}

#[test]
fn scripts_by_their_shebang_and_elixir_files_get_their_grammar() {
    for (f, src) in [
        ("bin/rails", &b"#!/usr/bin/env ruby\nx = 1\n"[..]),
        ("exe/tool", b"#!/usr/bin/env -S ruby -w\nx = 1\n"),
        ("script", b"#!/usr/local/bin/ruby3.3\nx = 1\n"),
    ] {
        assert!(sexp(f, src).contains("(assignment "), "{f}");
    }
    for (f, src) in [
        ("artisan", &b"#!/usr/bin/env php\n<?php echo 1;\n"[..]),
        ("bin/console", b"#!/usr/bin/env php8.2\n<?php echo 1;\n"),
    ] {
        assert!(sexp(f, src).contains("(php_tag"), "{f}");
    }
    assert!(sexp("artisan", b"#!/bin/sh\necho hi\n").contains("(command "));
    for f in ["a.ex", "a.exs"] {
        assert!(sexp(f, b"x = 1\n").contains("(source "), "{f}");
    }
    assert_eq!(sexp("a.ex", b"include std/io.e\nx = 1\n"), "");
    assert_eq!(sexp("a.ex", b"-- c\nx = 1\n"), "");
    assert!(sexp("a.ex", b"includes = 1\n").contains("(source "));
    assert!(sexp("a.exs", b"include std/io.e\nx = 1\n").contains("(source "));
}

#[test]
fn html_css_and_xml_files_get_their_grammar() {
    for f in ["a.html", "a.htm"] {
        assert!(sexp(f, b"<p>a</p>\n").contains("(element "), "{f}");
    }
    for text in [
        "{% extends \"base.html\" %}\n",
        "<div *ngIf=\"a\"></div>\n",
        "@if (a) {\n}\n",
        "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\">\n",
    ] {
        assert_eq!(sexp("a.html", text.as_bytes()), "", "{text}");
    }
    assert!(sexp("a.css", b"a { color: red; }\n").contains("(rule_set "));
    for f in ["a.xml", "a.svg", "a.xsd", "a.xslt", "a.rss"] {
        assert!(sexp(f, b"<a>b</a>\n").contains("(element "), "{f}");
    }
    for f in ["a.xhtml", "a.rng", "a.xaml", "a.tld", "a.opml"] {
        assert_eq!(sexp(f, b"<a>b</a>\n"), "", "{f}");
    }
    assert!(sexp("a.dtml", b"<p>a</p>\n").contains("(start_tag "));
}

#[test]
fn xml_files_are_told_by_their_prolog_and_namespace() {
    let prolog = "<?xml version=\"1.0\"?>\n<a>b</a>\n";
    for f in [
        "a.csproj",
        "a.xsl",
        "a.lpr",
        "a.grc",
        "a.smil",
        "Directory.Build.props",
        "a.csproj.user",
        "/etc/xdg/menus/a.menu",
    ] {
        assert!(sexp(f, prolog.as_bytes()).contains("(element "), "{f}");
    }
    for f in ["a.conf", "a.user", "a.menu", "a", "a.rng"] {
        assert_eq!(sexp(f, prolog.as_bytes()), "", "{f}");
    }
    assert_eq!(sexp("a.lpr", b"<!DOCTYPE a>\n<a>b</a>\n"), "");
    let docbook = "<book xmlns=\"http://docbook.org/ns/docbook\"/>\n";
    assert!(sexp("a.svg", docbook.as_bytes()).contains("(element "));
    for text in [
        "<!DOCTYPE book PUBLIC \"-//OASIS//DTD DocBook XML V4.5//EN\">\n<book/>\n",
        "<book xmlns=\"http://docbook.org/ns/docbook\"/>\n",
    ] {
        assert_eq!(sexp("a.xml", text.as_bytes()), "", "{text}");
    }
}

#[test]
fn diff_and_query_files_get_their_grammar() {
    let hunk = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n";
    for f in ["a.diff", "a.patch", "/src/debian/patches/series.diff"] {
        assert!(sexp(f, hunk.as_bytes()).contains("(addition)"), "{f}");
    }
    let header = "From 6e8e2a6f9efd71dbe6917816343ed8415ad696c3 Mon Sep 17 00:00:00 2001\n";
    assert!(sexp("a.diff", format!("{header}{hunk}").as_bytes()).contains("(addition)"));
    assert_eq!(sexp("a.patch", format!("{header}{hunk}").as_bytes()), "");
    let capture = b"(identifier) @variable\n";
    for f in [
        "/a/queries/rust/highlights.scm",
        "queries/x.scm",
        "/a/queries/b/c/d.scm",
    ] {
        assert!(sexp(f, capture).contains("(named_node "), "{f}");
    }
    assert!(sexp("/a/scheme/x.scm", capture).contains("(program (list "));
    let dep3 = format!("Description: fix\n{hunk}");
    assert_eq!(sexp("/src/debian/patches/fix.patch", dep3.as_bytes()), "");
    assert!(sexp("/src/fix.patch", dep3.as_bytes()).contains("(addition)"));
}

#[test]
fn every_vendored_query_compiles_with_what_it_uses() {
    let inherited: Vec<&str> = super::QUERIES
        .iter()
        .flat_map(|(_, highlights, injections)| highlights.lines().chain(injections.lines()))
        .filter_map(|l| l.strip_prefix("; inherits:"))
        .flat_map(|bases| bases.split(','))
        .map(str::trim)
        .collect();
    for (name, _, _) in super::QUERIES {
        match super::grammar(name) {
            Some(grammar) => {
                super::compile(name, grammar).unwrap();
                assert!(super::lang(name).is_some(), "{name}");
            }
            None => assert!(inherited.contains(name), "{name}"),
        }
    }
}

#[test]
fn a_query_using_what_merl_does_not_implement_is_refused() {
    let python = || tree_sitter_python::LANGUAGE.into();
    for text in [
        "((identifier) @x (#match? @x \"<a\"))",
        "((identifier) @x (#vim-match? @x \"a{-2}\"))",
        "((identifier) @x (#kind-eq? @x call))",
        "((identifier) @x (#offset! @x 0 1))",
        "((identifier) @x (#set! injection.language \"regex\"))",
        "((identifier) @x (#set! @x priority 101))",
        "((identifier) @x (#lua-match? @x \"%bxy\"))",
    ] {
        let got = super::compile_text("python", python(), text);
        assert!(got.is_err(), "{text} compiled");
    }
    super::compile_text(
        "python",
        python(),
        "((identifier) @x (#lua-match? @x \"^%u\"))",
    )
    .unwrap();
}

#[test]
fn contains_and_has_parent_and_their_not_forms_test_as_neovim_does() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let keyword = theme.group("@keyword.python").unwrap().fg;
    let painted = |query: &str, src: &str, col: usize| {
        let lang =
            super::compile_text("python", tree_sitter_python::LANGUAGE.into(), query).unwrap();
        let lang: &'static super::Lang = Box::leak(Box::new(lang));
        let lines = vec![src.to_owned()];
        let mut parsed = super::Parsed::new(lang, &lines).unwrap();
        parsed.parse(None);
        let mut styles = super::Styles::new(&theme);
        let spans = parsed.spans(&lines, 0, 0, &mut styles);
        let fg = spans[0]
            .iter()
            .find(|(_, r)| r.contains(&col))
            .unwrap()
            .0
            .fg;
        fg == keyword
    };
    let contains = "((identifier) @keyword (#contains? @keyword \"ss\" \"er\"))";
    assert!(painted(contains, "classer = 1", 0));
    assert!(!painted(contains, "class_ = 1", 0));
    let not_contains = "((identifier) @keyword (#not-contains? @keyword \"ss\" \"er\"))";
    assert!(!painted(not_contains, "classer = 1", 0));
    assert!(painted(not_contains, "class_ = 1", 0));
    let parent = "((identifier) @keyword (#has-parent? @keyword call decorator))";
    assert!(painted(parent, "f(x)", 0));
    assert!(!painted(parent, "f(x)", 2));
    let not_parent = "((identifier) @keyword (#not-has-parent? @keyword call))";
    assert!(!painted(not_parent, "f(x)", 0));
    assert!(painted(not_parent, "f(x)", 2));
}

#[test]
fn lua_patterns_match_as_lua_does() {
    for (pattern, yes, no) in [
        ("^[A-Z].*[a-z]", "Order", "MAX"),
        ("^[A-Z][A-Z_0-9]*$", "MAX_2", "Max"),
        ("^__[a-zA-Z0-9_]*__$", "__init__", "_x_"),
        ("^#!/", "#!/usr/bin/env", "# !/"),
        ("^[%l_].*$", "_private", "Public"),
        ("^%u", "Point", "point"),
        ("a%.b-c", "xa.c", "axbc"),
        ("^[^%d]+$", "abc", "a1"),
    ] {
        let re = super::lua_pattern(pattern).unwrap();
        assert!(re.is_match(yes.as_bytes()), "{pattern} on {yes}");
        assert!(!re.is_match(no.as_bytes()), "{pattern} on {no}");
    }
}

#[test]
fn vim_patterns_match_as_neovim_reads_them_very_magic() {
    for (pattern, yes, no) in [
        ("/\\*!([a-zA-Z]+:)?re2c", "/*!local:re2c", "/*!re2"),
        ("/\\*!([a-zA-Z]+:)?re2c", "x /*!re2c", "/*!:re2c"),
        ("^a(b|c)+$", "abcb", "a(b|c)+"),
        ("\\v^\\d+$", "42", "4a"),
        ("\\<a\\>", "<a>", "a"),
        ("[]a]+$", "x]a", "x]b"),
        ("^[^]a]$", "b", "]"),
        ("[a-c]x", "bx", "dx"),
        ("\\c^(break|continue)$", "BREAK", "breaks"),
        ("^a\\cb$", "AB", "ac"),
        ("^ab\\C$", "ab", "AB"),
        ("\\C^a\\c$", "A", "b"),
        ("^ab=c$", "ac", "abbc"),
        ("^(.{1,2}/|src/)", "../a", "abc/d"),
        ("^([+*-+=<>]|<=|>=|/=)$", "<", "-"),
        ("^([+*-+=<>]|<=|>=|/=)$", "/", "<="),
        ("^([+*-+=<>]|<=|>=|/=)$", "", "/="),
    ] {
        let re = super::vim_pattern(pattern).unwrap();
        assert!(re.is_match(yes.as_bytes()), "{pattern} on {yes}");
        assert!(!re.is_match(no.as_bytes()), "{pattern} on {no}");
    }
    for pattern in ["<a>", "\\Va", "a{-1,2}", "a{,2}", "a\\zsb"] {
        assert!(super::vim_pattern(pattern).is_err(), "{pattern}");
    }
}

#[test]
fn vim_patterns_merl_cannot_read_as_neovim_does_are_refused() {
    for (pattern, neovim) in [
        ("a\\sb", "\\s is a space or a tab, never a newline"),
        ("a\\Sb", "\\S takes a newline"),
        ("a\\_sb", "\\_s is \\s or a newline"),
        ("a\\_.b", "\\_. is any character or a newline"),
        ("a\\_[bc]", "\\_[ is a collection with a newline"),
        (
            "[\\d]",
            "\\d in a collection is a character by its decimal code",
        ),
        ("[a\\]]", "\\] in a collection is ]"),
        ("[[a]]", "[ in a collection is ["),
        ("[a--b]", "-- in a collection is a range up to -"),
    ] {
        assert!(super::vim_pattern(pattern).is_err(), "{pattern}: {neovim}");
    }
}

#[test]
fn a_theme_without_groups_still_colours_every_kind_of_capture() {
    let theme = theme::load("shokunin-dark").unwrap();
    assert!(theme.groups.is_empty());
    let src = "# c\ndef f(x):\n    return \"s\"\n";
    let mut b = Buffer::from_bytes(PathBuf::from("a.py"), src.as_bytes());
    b.parse_more(None);
    b.highlight_to(usize::MAX, &theme);
    let fg = |l: usize, col: usize| b.hl[l].iter().find(|(_, r)| r.contains(&col)).unwrap().0.fg;
    let (comment, keyword, function, string) = (fg(0, 0), fg(1, 0), fg(1, 4), fg(2, 11));
    assert_ne!(comment, keyword);
    assert_ne!(keyword, string);
    assert_ne!(
        function,
        theme
            .syntect
            .settings
            .foreground
            .map(|c| Color::Rgb(c.r, c.g, c.b))
    );
}

#[test]
fn a_theme_without_groups_colours_markdown_by_its_markup_scopes() {
    for name in ["shokunin-dark", "seneca"] {
        let theme = theme::load(name).unwrap();
        assert!(theme.groups.is_empty());
        let src = "# Title\n\n**bold** `code` [l](u)\n";
        let mut b = Buffer::from_bytes(PathBuf::from("a.md"), src.as_bytes());
        b.parse_more(None);
        b.highlight_to(usize::MAX, &theme);
        let normal = theme
            .syntect
            .settings
            .foreground
            .map(|c| Color::Rgb(c.r, c.g, c.b));
        let heading = b.hl[0].iter().find(|(_, r)| r.contains(&3)).unwrap().0;
        assert!(
            heading.fg != normal || !heading.add_modifier.is_empty(),
            "{name}"
        );
    }
}

#[test]
fn a_block_label_resolves_as_neovim_s_resolve_lang_and_in_the_preview_by_its_extension_too() {
    for (alias, neovim, preview) in [
        ("rs", None, Some("rust")),
        ("md", None, Some("markdown")),
        ("rust", Some("rust"), Some("rust")),
        ("pandoc", Some("markdown"), Some("markdown")),
        ("astro", None, None),
        ("yml", None, Some("yaml")),
        ("sh", Some("bash"), Some("bash")),
        ("jsonc", Some("json"), Some("json")),
        ("zsh", None, None),
        ("cpp", Some("cpp"), Some("cpp")),
        ("hpp", None, Some("cpp")),
        ("java", Some("java"), Some("java")),
        ("cs", Some("c_sharp"), Some("c_sharp")),
        ("csharp", Some("c_sharp"), Some("c_sharp")),
        ("kotlin", Some("kotlin"), Some("kotlin")),
        ("kt", None, Some("kotlin")),
        ("dockerfile", Some("dockerfile"), Some("dockerfile")),
        ("cmake", Some("cmake"), Some("cmake")),
        ("proto", Some("proto"), Some("proto")),
        ("graphql", Some("graphql"), Some("graphql")),
        ("gql", None, Some("graphql")),
        ("nginx", Some("nginx"), Some("nginx")),
        ("conf", None, None),
        ("gdscript", Some("gdscript"), Some("gdscript")),
        ("gd", None, Some("gdscript")),
        ("starlark", Some("starlark"), Some("starlark")),
        ("bzl", Some("starlark"), Some("starlark")),
        ("star", None, Some("starlark")),
        ("python", Some("python"), Some("python")),
        ("solidity", Some("solidity"), Some("solidity")),
        ("sol", None, Some("solidity")),
        ("hcl", Some("hcl"), Some("hcl")),
        ("terraform", Some("terraform"), Some("terraform")),
        ("tfvars", None, Some("terraform")),
        ("terraform-vars", None, None),
        ("tf", None, None),
        ("ini", Some("ini"), Some("ini")),
        ("dosini", Some("ini"), Some("ini")),
        ("sql", Some("sql"), Some("sql")),
        ("swift", Some("swift"), Some("swift")),
        ("ruby", Some("ruby"), Some("ruby")),
        ("rb", None, Some("ruby")),
        ("php", Some("php"), Some("php")),
        ("elixir", Some("elixir"), Some("elixir")),
        ("ex", Some("elixir"), Some("elixir")),
        ("scala", Some("scala"), Some("scala")),
        ("zig", Some("zig"), Some("zig")),
        ("nix", Some("nix"), Some("nix")),
        ("scheme", Some("scheme"), Some("scheme")),
        ("scm", None, Some("scheme")),
        ("r", Some("r"), Some("r")),
        ("erlang", Some("erlang"), Some("erlang")),
        ("erl", Some("erlang"), Some("erlang")),
        ("racket", Some("racket"), Some("racket")),
        ("rkt", None, Some("racket")),
        ("lisp", Some("commonlisp"), Some("commonlisp")),
        ("commonlisp", Some("commonlisp"), Some("commonlisp")),
        ("vue", Some("vue"), Some("vue")),
        ("svelte", Some("svelte"), Some("svelte")),
    ] {
        assert_eq!(resolve(alias).map(|l| l.name), neovim, "{alias}");
        assert_eq!(resolve_by_extension(alias).map(|l| l.name), preview, "{alias}");
    }
}

#[test]
fn a_preview_code_block_is_coloured_as_the_same_code_in_a_file() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let src = "import re\nWORD = re.compile(r\"[a-z]+\\d\")\n";
    let lines: Vec<String> = src.lines().map(str::to_owned).collect();
    let mut file = Buffer::from_bytes(PathBuf::from("a.py"), src.as_bytes());
    file.parse_more(None);
    file.paint_to(usize::MAX, &theme);
    for token in ["python", "py", " Python "] {
        let mut plain = Buffer::block(token, lines.clone());
        plain.without_tree_sitter();
        plain.highlight_to(usize::MAX, &theme);
        let syntect = plain.hl;
        let mut block = Buffer::block(token, lines.clone());
        block.parse_more(None);
        block.paint_to(usize::MAX, &theme);
        assert_eq!(block.hl, file.hl, "{token}");
        assert_ne!(block.hl, syntect, "{token}");
    }
    let dart = vec![
        "void main() {".to_owned(),
        "  print('x');".to_owned(),
        "}".to_owned(),
    ];
    let mut plain = Buffer::block("dart", dart.clone());
    plain.without_tree_sitter();
    plain.highlight_to(usize::MAX, &theme);
    let syntect = plain.hl;
    let mut block = Buffer::block("dart", dart);
    block.parse_more(None);
    block.paint_to(usize::MAX, &theme);
    assert_eq!(block.hl, syntect);
    let colours: std::collections::HashSet<_> =
        syntect.iter().flatten().map(|(s, _)| s.fg).collect();
    assert!(colours.len() > 2, "{colours:?}");
}

#[test]
fn elixir_docs_are_coloured_as_markdown() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let style_at = |src: &str, word: &str| {
        let lines = vec![src.to_owned()];
        let mut parsed = super::Parsed::new(super::lang("elixir").unwrap(), &lines).unwrap();
        parsed.parse(None);
        let mut styles = super::Styles::new(&theme);
        let col = src.find(word).unwrap();
        parsed.spans(&lines, 0, 0, &mut styles)[0]
            .iter()
            .find(|(_, r)| r.contains(&col))
            .unwrap()
            .0
    };
    let doc = "@doc \"Use `put` for *one*.\"";
    let plain = "x = \"Use `put` for *one*.\"";
    assert_eq!(
        style_at(doc, "put").fg,
        theme.group("@markup.raw.markdown_inline").unwrap().fg
    );
    assert_ne!(style_at(doc, "put").fg, style_at(plain, "put").fg);
    assert!(style_at(doc, "one").add_modifier.contains(Modifier::ITALIC));
    assert!(
        !style_at(plain, "one")
            .add_modifier
            .contains(Modifier::ITALIC)
    );
}

#[test]
fn a_higher_priority_paints_over_a_later_capture_and_equal_ones_paint_in_order() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let paint = |query: &str| {
        let lang =
            super::compile_text("python", tree_sitter_python::LANGUAGE.into(), query).unwrap();
        let lang: &'static super::Lang = Box::leak(Box::new(lang));
        let lines = vec!["x".to_owned()];
        let mut parsed = super::Parsed::new(lang, &lines).unwrap();
        parsed.parse(None);
        let mut styles = super::Styles::new(&theme);
        parsed.spans(&lines, 0, 0, &mut styles)[0][0].0
    };
    let keyword = theme.group("@keyword.python").unwrap().fg;
    let string = theme.group("@string.python").unwrap().fg;
    let raised = "((identifier) @keyword (#set! priority 101))\n(identifier) @string";
    assert_eq!(paint(raised).fg, keyword);
    assert_eq!(
        paint("(identifier) @keyword\n(identifier) @string").fg,
        string
    );
    assert_eq!(
        paint("(identifier) @string\n(identifier) @keyword").fg,
        keyword
    );
}

#[test]
fn a_parse_cut_short_resumes_and_an_edit_in_between_restarts_it() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let src = include_str!("../../tests/colors/python/classes.py").repeat(20);
    let mut b = Buffer::from_bytes(PathBuf::from("a.py"), src.as_bytes());
    assert!(!b.parse_more(Some(std::time::Duration::ZERO)));
    b.paint_to(10, &theme);
    assert!(b.hl.is_empty(), "no colours before the first tree");
    b.lines[3].insert(0, '"');
    b.edited(3);
    let mut steps = 0;
    while !b.parse_more(Some(std::time::Duration::ZERO)) {
        steps += 1;
    }
    assert!(steps > 1, "the parse was cut short {steps} times");
    b.lines[3].remove(0);
    b.lines[40].insert(0, 'x');
    b.edited(3);
    b.parse_more(Some(std::time::Duration::ZERO));
    b.paint_to(usize::MAX, &theme);
    b.parse_more(None);
    b.paint_to(usize::MAX, &theme);
    let text = b.lines.join("\n") + "\n";
    let mut fresh = Buffer::from_bytes(PathBuf::from("a.py"), text.as_bytes());
    fresh.parse_more(None);
    fresh.paint_to(usize::MAX, &theme);
    assert_eq!(b.sexp(), fresh.sexp());
    eprintln!("{:?}\n{:?}", b.hl[2], fresh.hl[2]);
    assert_eq!(b.hl, fresh.hl);
}

type Timings = Vec<(f64, f64)>;

fn ms(t: std::time::Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn settle(b: &mut Buffer, last: usize, theme: &theme::Theme) {
    while b.parsing() {
        b.parse_more(Some(crate::buffer::PARSE_IDLE_STEP));
    }
    b.highlight_to(last, theme);
}

fn speed_rows(
    file: &str,
    bytes: &[u8],
    ts: bool,
    theme: &theme::Theme,
) -> Vec<(&'static str, Option<Timings>)> {
    let open = |at: usize| {
        let t = std::time::Instant::now();
        let mut b = Buffer::from_bytes(PathBuf::from(file), bytes);
        if !ts {
            b.without_tree_sitter();
        }
        b.highlight_to(at + 54, theme);
        let frame = ms(t);
        settle(&mut b, at + 54, theme);
        ((frame, ms(t)), b)
    };
    let n = open(0).1.lines.len();
    let top: Vec<_> = (0..15).map(|_| open(0).0).collect();
    let middle: Vec<_> = (0..15).map(|_| open(n / 2).0).collect();
    let typing = |col0: bool| {
        let (_, mut b) = open(0);
        let indented = |l: &usize| !col0 || b.lines[*l].starts_with([' ', '\t']);
        let lines: Vec<usize> = (0..n).filter(indented).collect();
        if lines.is_empty() {
            return None;
        }
        let mut keys = Vec::new();
        for k in 0..60 {
            let l = lines[(k * 7919) % lines.len()];
            settle(&mut b, l + 54, theme);
            let t = std::time::Instant::now();
            let indent = b.lines[l].len() - b.lines[l].trim_start().len();
            b.lines[l].insert(if col0 { 0 } else { indent }, 'x');
            b.edited(l);
            b.highlight_to(l + 54, theme);
            let frame = ms(t);
            settle(&mut b, l + 54, theme);
            keys.push((frame, ms(t)));
            if col0 {
                b.lines[l].remove(0);
                b.edited(l);
            }
        }
        Some(keys)
    };
    vec![
        ("first paint, top", Some(top)),
        ("first paint, middle", Some(middle)),
        ("keystroke after the indentation", typing(false)),
        ("keystroke at column 0 of an indented line", typing(true)),
    ]
}

#[test]
fn the_speed_rows_finish_on_a_file_with_few_or_no_indented_lines() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let few = "def f():\n    return 1\n\n\nx = 2\n".repeat(3);
    let measured = |src: &str| {
        speed_rows("a.py", src.as_bytes(), true, &theme)
            .into_iter()
            .map(|(_, keys)| keys.map(|k| k.len()))
            .collect::<Vec<_>>()
    };
    assert_eq!(measured(&few), [Some(15), Some(15), Some(60), Some(60)]);
    assert_eq!(
        measured("x = 1\ny = 2\n"),
        [Some(15), Some(15), Some(60), None]
    );
}

#[test]
#[ignore = "times parsing against syntect in release; MERL_TS_SPEED=FILES runs it"]
fn speed() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let files = std::env::var("MERL_TS_SPEED").unwrap_or_default();
    let pct = |v: &[(f64, f64)], coloured: bool, p: usize| {
        let mut v: Vec<f64> = v.iter().map(|x| if coloured { x.1 } else { x.0 }).collect();
        v.sort_by(f64::total_cmp);
        v[(v.len() * p / 100).min(v.len() - 1)]
    };
    for file in files.split(',').filter(|f| !f.is_empty()) {
        let bytes = std::fs::read(file).unwrap();
        for ts in [false, true] {
            let engine = if ts { "tree-sitter" } else { "syntect" };
            let n = Buffer::from_bytes(PathBuf::from(file), &bytes).lines.len();
            for (what, v) in speed_rows(file, &bytes, ts, &theme) {
                let Some(v) = v else {
                    println!(
                        "{file} {n} lines {engine}: {what}: skipped, the file has no indented line"
                    );
                    continue;
                };
                println!(
                    "{file} {n} lines {engine}: {what}: frame p50 {:.2} p90 {:.2}, coloured p50 {:.2} p90 {:.2} ms",
                    pct(&v, false, 50),
                    pct(&v, false, 90),
                    pct(&v, true, 50),
                    pct(&v, true, 90),
                );
            }
        }
    }
    let body = |line: &dyn Fn(usize) -> String| (0..19_998).map(line).collect::<String>();
    let containers = [
        (
            "a dict literal",
            format!("X = {{\n{}}}\n", body(&|i| format!("    \"k{i}\": 1,\n"))),
        ),
        (
            "a function body",
            format!("def f():\n{}", body(&|i| format!("    x{i} = 1\n"))),
        ),
    ];
    for (what, src) in containers {
        let mut b = Buffer::from_bytes(PathBuf::from("a.py"), src.as_bytes());
        let l = b.lines.len() - 50;
        let keys: Vec<_> = (0..20)
            .map(|_| {
                settle(&mut b, l + 20, &theme);
                let t = std::time::Instant::now();
                b.lines[l] = if b.lines[l].contains("[1]") {
                    b.lines[l].replace("[1]", "1")
                } else {
                    b.lines[l].replace(" 1", " [1]")
                };
                b.edited(l);
                b.highlight_to(l + 20, &theme);
                let frame = ms(t);
                settle(&mut b, l + 20, &theme);
                (frame, ms(t))
            })
            .collect();
        println!(
            "{what}, {} lines, 1 to [1] 50 rows from its end: frame p50 {:.2} p90 {:.2}, coloured p50 {:.2} p90 {:.2} ms",
            b.lines.len(),
            pct(&keys, false, 50),
            pct(&keys, false, 90),
            pct(&keys, true, 50),
            pct(&keys, true, 90),
        );
    }
}

#[test]
fn a_block_neovim_finds_no_parser_for_stays_plain() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let fg_at = |info: &str| {
        let lines: Vec<String> = [format!("```{info}"), "$ mix phx".into(), "```".into()].into();
        let mut parsed = super::Parsed::new(super::lang("markdown").unwrap(), &lines).unwrap();
        parsed.parse(None);
        let mut styles = super::Styles::new(&theme);
        parsed.spans(&lines, 0, 2, &mut styles)[1]
            .iter()
            .find(|(_, r)| r.contains(&2))
            .map(|(s, _)| s.fg)
    };
    assert_ne!(fg_at("bash"), fg_at("nothing"));
    for info in ["console", "shell", "objc"] {
        assert_eq!(fg_at(info), fg_at("nothing"), "{info}");
    }
}

#[test]
fn a_label_neovim_does_not_resolve_draws_its_block_plain_as_lazyvim() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let fg_at = |host: &str, lines: [&str; 3]| {
        let lines: Vec<String> = lines.map(str::to_owned).into();
        let mut parsed = super::Parsed::new(super::lang(host).unwrap(), &lines).unwrap();
        parsed.parse(None);
        let mut styles = super::Styles::new(&theme);
        parsed.spans(&lines, 0, 2, &mut styles)[1]
            .iter()
            .find(|(_, r)| r.contains(&0))
            .map(|(s, _)| s.fg)
    };
    for (name, short, code) in [
        ("ruby", "rb", "def a; end"),
        ("rust", "rs", "fn a() {}"),
        ("yaml", "yml", "a: 1"),
    ] {
        let fence = |info: &str| fg_at("markdown", [&format!("```{info}"), code, "```"]);
        assert_ne!(fence(name), fence("nothing"), "{name}");
        assert_eq!(fence(short), fence("nothing"), "{short}");
    }
    let heredoc = |label: &str| fg_at("ruby", [&format!("x = <<~{label}"), "def a; end", label]);
    assert_ne!(heredoc("RUBY"), heredoc("TEXT"));
    assert_eq!(heredoc("RB"), heredoc("TEXT"));
    let fence = |info: &str| fg_at("markdown", [&format!("```{info}"), "fn a() {}", "```"]);
    assert_ne!(fence("fish"), fence("nothing"));
}

#[test]
fn an_injected_language_without_a_grammar_stays_plain_outside_markdown() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let lang = super::lang("tsx").unwrap();
    let fg_at = |src: &str, word: &str| {
        let lines = vec![src.to_owned()];
        let mut parsed = super::Parsed::new(lang, &lines).unwrap();
        parsed.parse(None);
        let mut styles = super::Styles::new(&theme);
        let col = src.find(word).unwrap();
        parsed.spans(&lines, 0, 0, &mut styles)[0]
            .iter()
            .find(|(_, r)| r.contains(&col))
            .unwrap()
            .0
            .fg
    };
    assert_eq!(
        fg_at("t(`Add people to x`);", "Add"),
        fg_at("x(`Add people to x`);", "Add")
    );
}

#[test]
fn a_doc_comment_is_parsed_as_jsdoc_in_javascript_and_typescript() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let group = |g: &str| theme.group(g).unwrap().fg;
    let src = "/** @param {string} name */";
    for name in ["javascript", "typescript", "tsx"] {
        let lines = vec![src.to_owned()];
        let mut parsed = super::Parsed::new(super::lang(name).unwrap(), &lines).unwrap();
        parsed.parse(None);
        let mut styles = super::Styles::new(&theme);
        let spans = parsed.spans(&lines, 0, 0, &mut styles);
        let fg = |word: &str| {
            let col = src.find(word).unwrap();
            spans[0]
                .iter()
                .find(|(_, r)| r.contains(&col))
                .unwrap()
                .0
                .fg
        };
        assert_eq!(fg("@param"), group("@keyword"), "{name}");
        assert_eq!(fg("string"), group("@type"), "{name}");
        assert_eq!(fg("name"), group("@variable"), "{name}");
        assert_ne!(fg("@param"), fg("/**"), "{name}");
    }
}

#[test]
fn an_edit_repaints_a_name_whose_pattern_spans_the_changed_lines() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let mut b = Buffer::from_bytes(PathBuf::from("a.lua"), b"local x\nf =\n  1\nlocal y\n");
    b.parse_more(None);
    b.paint_to(usize::MAX, &theme);
    for value in ["  function() end", "  1"] {
        b.lines[2] = value.to_owned();
        b.edited(2);
        b.parse_more(None);
        b.paint_to(usize::MAX, &theme);
        let text = b.lines.join("\n") + "\n";
        let mut fresh = Buffer::from_bytes(PathBuf::from("a.lua"), text.as_bytes());
        fresh.parse_more(None);
        fresh.paint_to(usize::MAX, &theme);
        assert_eq!(b.hl, fresh.hl, "{value}");
    }
}

#[test]
fn an_edit_inside_a_large_container_repaints_from_its_own_row() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let body = |line: &dyn Fn(usize) -> String| (0..30).map(line).collect::<String>();
    let dict = format!("X = {{\n{}}}\n", body(&|i| format!("    \"k{i}\": 1,\n")));
    let function = format!("def f():\n{}", body(&|i| format!("    x{i} = 1\n")));
    for src in [dict, function] {
        let mut b = Buffer::from_bytes(PathBuf::from("a.py"), src.as_bytes());
        b.parse_more(None);
        b.paint_to(usize::MAX, &theme);
        b.lines[25] = b.lines[25].replace(" 1", " [1]");
        b.edited(25);
        b.parse_more(None);
        assert_eq!(b.hl.len(), 25, "{}", b.lines[0]);
    }
}

#[test]
fn an_edit_inside_a_name_repaints_a_capture_its_predicate_moves_on_an_earlier_line() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let mut b = Buffer::from_bytes(PathBuf::from("a.py"), b"T = \\\n    TypeVa(\"T\")\n");
    b.parse_more(None);
    b.paint_to(usize::MAX, &theme);
    for value in [
        "    TypeVar(\"T\")",
        "    TypeVa(\"T\")",
        "    TypeVar(\"T\")",
        "    TypeVr(\"T\")",
    ] {
        b.lines[1] = value.to_owned();
        b.edited(1);
        b.parse_more(None);
        b.paint_to(usize::MAX, &theme);
        let text = b.lines.join("\n") + "\n";
        let mut fresh = Buffer::from_bytes(PathBuf::from("a.py"), text.as_bytes());
        fresh.parse_more(None);
        fresh.paint_to(usize::MAX, &theme);
        assert_eq!(b.hl, fresh.hl, "{value}");
    }
}

#[test]
fn an_edit_on_a_later_line_repaints_a_string_its_injection_moves_on_an_earlier_line() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let mut b = Buffer::from_bytes(PathBuf::from("a.py"), b"x = (\"%d items\"\n     % n)\n");
    b.parse_more(None);
    b.paint_to(usize::MAX, &theme);
    for value in ["     + n)", "     % n)"] {
        b.lines[1] = value.to_owned();
        b.edited(1);
        b.parse_more(None);
        b.paint_to(usize::MAX, &theme);
        let text = b.lines.join("\n") + "\n";
        let mut fresh = Buffer::from_bytes(PathBuf::from("a.py"), text.as_bytes());
        fresh.parse_more(None);
        fresh.paint_to(usize::MAX, &theme);
        assert_eq!(b.hl, fresh.hl, "{value}");
    }
}

#[test]
fn a_keyword_error_recovery_rereads_as_a_name_above_an_edit_is_repainted() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let mut b = Buffer::from_bytes(
        PathBuf::from("a.py"),
        b"#\ns\ns\nclass r:\ns.d()\nc={}\ndef _()->e:\nk()\ne\n",
    );
    b.parse_more(None);
    b.paint_to(usize::MAX, &theme);
    let broken = [
        "\u{1f4bb}def f(class A:",
        " x=1",
        "f\"{x}\"\u{1f3fd}T=\\match x",
    ];
    for (row, tail, halted) in [
        (9, &broken[..], false),
        (12, &[""][..], false),
        (12, &[][..], true),
        (12, &[" :"][..], false),
    ] {
        b.lines.truncate(row);
        b.lines.extend(tail.iter().map(|l| (*l).to_owned()));
        b.edited(row);
        if halted {
            b.parse_more(Some(std::time::Duration::ZERO));
        }
        b.parse_more(None);
        b.paint_to(usize::MAX, &theme);
        let incremental = b.hl.clone();
        b.clear_hl();
        b.paint_to(usize::MAX, &theme);
        assert_eq!(incremental, b.hl, "{tail:?}");
    }
}

#[test]
fn overlapping_combined_groups_keep_query_order_whatever_came_first() {
    let keys = |b: &Buffer| b.groups().into_iter().map(|g| g.key).collect::<Vec<_>>();
    for (was, now) in [
        ("build = /* bash */", "buildPhase = /* bash */"),
        ("buildPhase =", "buildPhase = /* bash */"),
    ] {
        let src =
            format!("{{\n  {was} ''\n    echo \"a\n  '';\n  checkPhase = ''\n    b\"\n  '';\n}}\n");
        let mut b = Buffer::from_bytes(PathBuf::from("a.nix"), src.as_bytes());
        b.parse_more(None);
        b.lines[1] = format!("  {now} ''");
        b.edited(1);
        b.parse_more(None);
        let text = b.lines.join("\n") + "\n";
        let mut fresh = Buffer::from_bytes(PathBuf::from("a.nix"), text.as_bytes());
        fresh.parse_more(None);
        assert_eq!(keys(&b).len(), 2);
        assert_eq!(keys(&b), keys(&fresh), "{was}");
    }
}

#[test]
fn a_finished_top_tree_paints_before_its_combined_groups_are_parsed() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let src = include_str!("../../tests/colors/php/Ledger.php").repeat(7);
    let mut fresh = Buffer::from_bytes(PathBuf::from("a.php"), src.as_bytes());
    fresh.parse_more(None);
    fresh.paint_to(usize::MAX, &theme);
    let mut b = Buffer::from_bytes(PathBuf::from("a.php"), src.as_bytes());
    while !b.parse_more(Some(std::time::Duration::ZERO)) {}
    assert!(b.parsing(), "the groups are not parsed yet");
    b.paint_to(usize::MAX, &theme);
    assert_eq!(b.hl, fresh.hl);
    while b.parsing() {
        b.parse_more(Some(std::time::Duration::ZERO));
    }
    b.paint_to(usize::MAX, &theme);
    assert_eq!(b.hl, fresh.hl);
}
