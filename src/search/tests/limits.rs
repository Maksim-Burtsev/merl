use super::*;

fn hidden(kind: Kind, text: &str) -> Vec<usize> {
    (literal_lines(kind, text).iter().enumerate())
        .filter(|(_, l)| **l)
        .map(|(i, _)| i + 1)
        .collect()
}

fn numbered(head: &str, each: impl Fn(usize) -> String, n: usize, tail: &str) -> String {
    let mut text = format!("{head}\n");
    for i in 0..n {
        text.push_str(&each(i));
        text.push('\n');
    }
    text.push_str(tail);
    text
}

#[test]
fn a_ts_type_header_is_read_eight_lines_up() {
    let text = numbered("class Big<", |_| "  T,".into(), 9, ">{}\n");
    assert_eq!(type_decl_at(Kind::TsJs, &text, 9), Some(1));
    assert_eq!(type_decl_at(Kind::TsJs, &text, 10), None);
}

#[test]
fn a_ts_declarator_list_is_read_for_two_hundred_lines() {
    let list = |n: usize| {
        let text = numbered(
            "const a0 = 0,",
            |i| format!("  a{} = 1,", i + 1),
            n - 2,
            "  last = 1;",
        );
        let lines: Vec<&str> = text.lines().collect();
        let last = ts_declarators(&lines, 0).unwrap().pop().unwrap();
        (last.line0, last.as_own_statement)
    };
    assert_eq!(list(200), (199, "const last = 1".to_owned()));
    assert_eq!(list(201), (199, "const a199 = 1".to_owned()));
}

#[test]
fn a_ts_declarator_list_without_a_semicolon_ends_back_at_its_indent() {
    let text = "  const a = 1,\n    b = {\n  x: 1,\n    },\n    c = 2\n  render(a)\n";
    let lines: Vec<&str> = text.lines().collect();
    let found: Vec<(usize, String)> = ts_declarators(&lines, 0)
        .unwrap()
        .into_iter()
        .map(|d| (d.line0, d.as_own_statement))
        .collect();
    assert_eq!(
        found,
        [
            (0, "const a = 1".to_owned()),
            (1, "const b = { x: 1, }".to_owned()),
            (4, "const c = 2".to_owned())
        ],
        "a line at the list's indent inside braces goes on; one after them ends it"
    );
}

#[test]
fn a_ts_header_is_read_for_forty_lines() {
    let header = |n: usize| {
        let text = numbered("class Big<", |i| format!("  T{i},"), n - 1, "> {\n}");
        let lines: Vec<&str> = text.lines().collect();
        ts_header(&lines, 0).last_line0
    };
    assert_eq!(header(39), 39);
    assert_eq!(header(40), 0);
}

#[test]
fn tsconfig_extends_is_followed_for_eight_hops() {
    let missing = |hops: usize| {
        let names: Vec<String> = (0..=hops)
            .map(|i| match i {
                0 => "tsconfig.json".to_owned(),
                i => format!("c{i}.json"),
            })
            .collect();
        let texts: Vec<String> = (0..=hops)
            .map(|i| match i == hops {
                true => r#"{ "compilerOptions": { "paths": { "helpers": ["src/helpers.ts"] } } }"#
                    .to_owned(),
                false => format!(r#"{{ "extends": "./c{}" }}"#, i + 1),
            })
            .collect();
        let mut files: Vec<(&str, &str)> = (names.iter().zip(&texts))
            .map(|(n, t)| (n.as_str(), t.as_str()))
            .collect();
        files.push(("src/helpers.ts", ""));
        let (dir, files) = scratch(&format!("extends-{hops}"), &files);
        let missing = package_missing(&dir, &files, Path::new("src/pages"), &["helpers".into()]);
        std::fs::remove_dir_all(&dir).unwrap();
        missing
    };
    assert!(!missing(7));
    assert!(missing(8));
}

#[test]
fn a_label_is_looked_for_up_to_eighty_lines_back() {
    let label = |n: usize| {
        let text = numbered("f(", |_| "    a=1,".into(), n - 1, "    b=2,\n)\n");
        label_at(Kind::Python, false, &text, n, 4..5)
    };
    assert!(label(80).is_some());
    assert_eq!(label(81), None);
}

#[test]
fn go_build_reads_the_environment_only() {
    let host = GoBuild::host();
    assert!(host.cgo && host.tags.is_empty());
    let set = GoBuild::host().env(Some("0"), Some("-race -tags=a,b"));
    assert!(!set.cgo);
    assert_eq!(set.tags, ["a", "b"]);
    let unset = GoBuild::host().env(None, None);
    assert!(unset.cgo && unset.tags.is_empty());
}

#[test]
fn a_proto_option_reads_as_a_field() {
    let field = def_patterns(Kind::Proto, "x")
        .iter()
        .any(|p| Regex::new(p).unwrap().is_match("  option x = 1;"));
    assert!(field);
}

#[test]
fn a_gitlab_stages_flow_list_is_read_on_one_line_only() {
    let entries = |text: &str| stage_entries("  stage: ", text, "test");
    assert_eq!(entries("stages: [build, test]\n"), Some(vec![1]));
    assert_eq!(entries("stages:\n  - build\n  - test\n"), Some(vec![3]));
    assert_eq!(entries("stages: [\n  build,\n  test\n]\n"), Some(vec![]));
}

#[test]
fn an_enum_body_is_read_for_two_thousand_lines() {
    let member = |n: usize| {
        let text = numbered("enum Big {", |i| format!("  A{i},"), n, "  Last,\n}\n");
        enum_member(Kind::TsJs, &text, 1, "Last")
    };
    assert_eq!(member(1997), Some((1999, 2)));
    assert_eq!(member(1998), None);
}

#[test]
fn a_go_literal_is_looked_for_two_thousand_lines_up() {
    let key = |n: usize| {
        let text = numbered(
            "var t = Tariff{",
            |_| "\tA: 1,".into(),
            n,
            "\tRate: 1,\n}\n",
        );
        go_key(&text, n + 2, 1, 5)
    };
    assert_eq!(key(1999), GoKey::Of("Tariff".to_owned()));
    assert_eq!(key(2000), GoKey::No);
}

#[test]
fn a_go_literal_is_read_eight_elided_levels_deep() {
    let key = |n: usize| {
        let mut text = format!("var all = {}Tariff{{\n", "[]".repeat(n));
        text.push_str(&"{\n".repeat(n - 1));
        text.push_str("{Rate: 1},\n");
        text.push_str(&"},\n".repeat(n - 1));
        text.push_str("}\n");
        go_key(&text, n + 1, 1, 5)
    };
    assert_eq!(key(8), GoKey::Of("Tariff".to_owned()));
    assert_eq!(key(9), GoKey::Unknown);
}

#[test]
fn a_rust_attribute_is_read_eight_lines_up() {
    let path = |n: usize| {
        let mut lines = vec!["#[serde(".to_owned()];
        lines.extend((1..n).map(|_| "    default,".to_owned()));
        lines.push("    with = \"my::path\")]".to_owned());
        let start = lines[n].find("my").unwrap();
        rust_attribute_path(&lines, n, start)
    };
    assert!(path(8));
    assert!(!path(9));
}

#[test]
fn a_php_attribute_counts_its_brackets_as_written() {
    let class = |attribute: &str| {
        let text = format!("/**\n * @property int $x\n */\n{attribute}\nclass A {{}}\n");
        let lines: Vec<&str> = text.lines().collect();
        php_tag_class(&lines, 1)
    };
    assert_eq!(class("#[Attr(\"x\")]"), Some(4));
    assert_eq!(class("#[Attr(\"[\")]"), None);
}

#[test]
fn a_cpp_constructor_head_is_read_for_four_kilobytes() {
    let ctor = |pad: usize| {
        let text = format!("Foo::Foo() : a({}), b() {{}}\n", "1".repeat(pad));
        let col = text.find("b()").unwrap();
        c_initialized_member(&text, 1, col)
    };
    assert_eq!(ctor(4078), Some("Foo".to_owned()));
    assert_eq!(ctor(4079), None);
}

#[test]
fn an_elixir_alias_group_is_read_on_one_line_only() {
    let un = |text: &str| {
        let line = text.lines().count() - 1;
        elixir_unalias(text, line, vec!["Coupon".into()]).join(".")
    };
    assert_eq!(
        un("  alias Shop.{Tariff, Coupon}\n  Coupon.x\n"),
        "Shop.Coupon"
    );
    assert_eq!(
        un("  alias Shop.{\n    Tariff,\n    Coupon\n  }\n  Coupon.x\n"),
        "Coupon"
    );
}

#[test]
fn a_chain_led_by_dots_is_joined_for_forty_lines() {
    let joined = |n: usize| {
        let mut lines = vec!["    repo".to_owned()];
        lines.extend((1..n).map(|_| "        .a()".to_owned()));
        lines.push("        .find(1)".to_owned());
        unbroken(Kind::TsJs, &lines, n, 9).is_some()
    };
    assert!(joined(40));
    assert!(!joined(41));
}

#[test]
fn a_bare_module_function_counts_for_the_whole_module() {
    let below = "module Util\n  def rate\n  end\n  module_function\nend\n";
    assert!(ruby_singleton(below, 2));
    let listed = "module Util\n  module_function :rate\n  def rate\n  end\nend\n";
    assert!(!ruby_singleton(listed, 3));
}

#[test]
fn apostrophes_in_jsx_text_read_as_a_string() {
    let at = |line: &str| line.rfind("repo").unwrap();
    let one = "<p>Don't {items.map(repo => repo.name)}</p>";
    let pair = "<p>Don't {items.map(repo => repo.name)} it's</p>";
    assert!(!ts_arrow_binds(one, "repo", at(one)));
    assert!(ts_arrow_binds(pair, "repo", at(pair)));
}

#[test]
fn a_csharp_type_header_is_read_for_eight_lines() {
    let bases = |n: usize| {
        let text = numbered(
            "public class Big :",
            |i| format!("    I{i},"),
            n,
            "    Last\n{\n}\n",
        );
        cs_bases(&text, 1)
    };
    assert_eq!(bases(6).last().map(String::as_str), Some("Last"));
    assert_eq!(bases(7).last().map(String::as_str), Some("I6"));
}

#[test]
fn a_csharp_initializer_is_read_two_hundred_lines_up() {
    let initialized = |n: usize| {
        let text = numbered(
            "class Shop\n{\nvoid F()\n{\nvar o = new Order {",
            |_| "    X = 1,".into(),
            n,
            "    Id = 1,\n};\n}\n}\n",
        );
        let lines: Vec<String> = text.lines().map(str::to_owned).collect();
        let k = lines.iter().position(|l| l.contains("Id = 1")).unwrap();
        cs_initialized(&lines, k, 4, 6)
    };
    assert_eq!(initialized(199), Some("Order".to_owned()));
    assert_eq!(initialized(200), None);
}

#[test]
fn a_bracket_group_is_read_for_two_thousand_lines() {
    let close = |n: usize| {
        let text = numbered("f(", |_| "  a,".into(), n, ")");
        let lines: Vec<&str> = text.lines().collect();
        group(Kind::Python, &lines, 0, 1).map(|g| g.close_line)
    };
    assert_eq!(close(1998), Some(1999));
    assert_eq!(close(1999), None);
}

#[test]
fn elixir_sigils_are_read_by_their_quotes() {
    assert_eq!(
        hidden(
            Kind::Elixir,
            "x = ~S\"\"\"\ndef fake do\n\"\"\"\ndef real do\n"
        ),
        [2, 3]
    );
    assert_eq!(
        hidden(Kind::Elixir, "x = ~s(\"\"\")\ndef fake do\nend\n"),
        [2, 3, 4]
    );
}

#[test]
fn ruby_reads_percent_strings_as_code_and_a_slash_after_a_word_as_division() {
    assert_eq!(
        hidden(Kind::Ruby, "x = %q{\ndef fake\n}\n"),
        Vec::<usize>::new()
    );
    assert_eq!(
        hidden(Kind::Ruby, "x = (/<<~EOS/)\ndef fake\nend\n"),
        Vec::<usize>::new()
    );
    for line in ["when /<<~EOS/ then 1", "x.split /<<~EOS/"] {
        assert_eq!(
            hidden(Kind::Ruby, &format!("{line}\ndef fake\nend\n")),
            [2, 3, 4],
            "{line}"
        );
    }
}

#[test]
fn a_css_url_reads_as_a_line_comment() {
    assert_eq!(
        hidden(
            Kind::Css,
            ".a { background: url(//x/a.png) /* a\n.fake {}\n*/\n"
        ),
        Vec::<usize>::new()
    );
}

#[test]
fn a_regex_in_a_template_hole_counts_its_braces() {
    let text = "const s = `${a.replace(/[{]/, '')}`;\nclass Fake {}\n`\n";
    assert_eq!(hidden(Kind::TsJs, text), [2, 3]);
}

#[test]
fn a_backtick_after_a_slash_opens_no_template() {
    assert_eq!(
        hidden(Kind::TsJs, "const r = /`/;\nclass Fake {}\n`\n"),
        [4]
    );
    assert_eq!(
        hidden(Kind::TsJs, "const r = a /`b`;\nclass Fake {}\n`\n"),
        [2, 3]
    );
}

#[test]
fn zig_roots_leave_the_package_cache_out() {
    let env = "{\n \"lib_dir\": \"/opt/lib/zig\",\n \"std_dir\": \"/opt/lib/zig/std\",\n \"global_cache_dir\": \"/home/u/.cache/zig\"\n}\n";
    assert_eq!(zig_roots(env), [PathBuf::from("/opt/lib/zig/std")]);
}

#[test]
fn proto_and_jvm_roots_leave_the_package_caches_out() {
    let (dir, _) = scratch(
        "no-caches",
        &[(".cache/buf/v1/a.proto", ""), ("build.gradle", "")],
    );
    let include = [
        "/opt/homebrew/include",
        "/usr/local/include",
        "/usr/include",
    ];
    let proto = external_roots(Kind::Proto, &dir);
    assert!(
        proto
            .iter()
            .all(|r| include.iter().any(|i| r == Path::new(i))),
        "{proto:?}"
    );
    assert_eq!(external_roots(Kind::Jvm, &dir), Vec::<PathBuf>::new());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rust_use_files_try_only_the_default_module_paths() {
    let find = |files: &[&str], text: &str| -> Vec<PathBuf> {
        let files: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
        rust_use_files(&files, Path::new("src/lib.rs"), text, "Cache")
    };
    let text = "#[path = \"elsewhere/net_impl.rs\"]\nmod net;\nuse crate::net::Cache;\n";
    assert_eq!(
        find(&["Cargo.toml", "src/lib.rs", "src/net.rs"], text),
        [PathBuf::from("src/net.rs")]
    );
    assert_eq!(
        find(
            &["Cargo.toml", "src/lib.rs", "src/elsewhere/net_impl.rs"],
            text
        ),
        Vec::<PathBuf>::new()
    );
}

#[test]
fn a_shell_function_is_told_by_its_header_and_indent() {
    let at = |text: &str, line: usize| shell_function_at(&text.lines().collect::<Vec<_>>(), line);
    let braced = "f() {\n  x=1\n}\ny=2\n";
    assert_eq!(at(braced, 1), Some(0));
    assert_eq!(at(braced, 3), None);
    assert_eq!(at("function f {\n  x=1\n}\n", 1), Some(0));
    assert_eq!(at("f() { x; }\ny=2\n", 1), None);
    assert_eq!(at("f() { x; } # done\ny=2\n", 1), None);
    assert_eq!(at("f() (\n  x=1\n)\n", 1), None);
}
