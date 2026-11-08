use super::*;

fn hidden(kind: Kind, text: &str) -> Vec<usize> {
    (literal_lines(kind, text).iter().enumerate())
        .filter(|(_, l)| **l)
        .map(|(i, _)| i + 1)
        .collect()
}

#[test]
fn a_kotlin_and_a_java_file_of_one_text_keep_their_own_comments() {
    let text = "/* a /* b */\nfun fake() {}\n*/\nfun real() {}";
    let read = |name: &str| path_literal_lines(Kind::Jvm, Path::new(name), text);
    let (java, nested) = (vec![false; 4], vec![false, true, true, false]);
    assert_eq!(read("A.java"), java);
    assert_eq!(read("A.kt"), nested, "Kotlin's comments nest");
    assert_eq!(
        read("A.java"),
        java,
        "Java keeps its reading after Kotlin's"
    );
    assert_eq!(read("A.scala"), nested);
    assert_eq!(read("A.groovy"), java);
    let imports = "/* a /* b */\nimport app.Fake\n*/\nimport app.Real\n";
    let bound = |name: &str| -> Vec<String> {
        let imports = jvm_imports(Path::new(name), imports);
        imports.into_iter().map(|i| i.bound_name).collect()
    };
    assert_eq!(bound("A.kt"), ["Real"], "an import inside a nested comment");
    assert_eq!(bound("A.java"), ["Fake", "Real"]);
    let at = |name: &str| jvm_imports_at(Path::new(name), imports, 4).len();
    assert_eq!((at("A.kt"), at("A.java")), (1, 2));
}

#[test]
fn a_kind_runs_over_lines_only_the_literals_it_has() {
    for (kind, text) in [
        (Kind::Graphql, "/*\ntype Fake\n*/"),
        (Kind::Graphql, "`\ntype Fake\n`"),
        (Kind::Graphql, "'''\ntype Fake\n'''"),
        (Kind::Proto, "`\nmessage Fake {}\n`"),
        (Kind::Css, "`\n.fake {}\n`"),
        (Kind::Css, "// a /* in a line comment\n.fake {}\n"),
        (Kind::Jvm, "`\nclass Fake\n`"),
        (Kind::Shell, "echo \"a\nfake() {\n\""),
        (Kind::Shell, "echo 'a\nfake() {\n'"),
        (Kind::Shell, ": 'a\\'\nreal() {\n"),
        (Kind::Shell, "x='a' y='b\nfake() {\n'"),
        (Kind::Ruby, "items <<lower\ndef fake\nlower\n"),
        (Kind::PowerShell, "$a = @\"x\"\nfunction Fake {}\n"),
        (Kind::PowerShell, "x `<#\nfunction Fake {}\n#>"),
    ] {
        assert_eq!(hidden(kind, text), Vec::<usize>::new(), "{kind:?}: {text}");
    }
    for (kind, text, want) in [
        (Kind::Html, "<!--\n<div id=\"fake\">\n-->\n<p>", vec![2, 3]),
        (
            Kind::PowerShell,
            "$a = 'x`'; <#\nfunction Fake {}\n#>",
            vec![2, 3],
        ),
        (Kind::CSharp, "var s = $@\"\nclass Fake {}\n\";", vec![2, 3]),
        (Kind::Shell, ": 'a\nfake() {\n'\nreal() {", vec![2, 3]),
        (
            Kind::Shell,
            "  usage+='\nfake() {\n' # it's\nreal() {",
            vec![2, 3],
        ),
        (
            Kind::Rust,
            "const S: &'static str = \"\nfn fake() {}\n\";",
            vec![2, 3],
        ),
    ] {
        assert_eq!(hidden(kind, text), want, "{kind:?}: {text}");
    }
}

#[test]
fn a_split_skips_what_only_looks_like_its_separator() {
    assert_eq!(
        split_top(Kind::TsJs, "f: (x: A) => B, g", b','),
        ["f: (x: A) => B", " g"]
    );
    for s in ["a == b", "a <= b", "a >= b", "a != b", "a => b"] {
        assert_eq!(split_top(Kind::Python, s, b'='), [s]);
    }
    assert_eq!(split_top(Kind::Python, "a = b", b'='), ["a ", " b"]);
}

#[test]
fn sql_block_comments_nest_only_in_a_file_written_for_it() {
    for (text, want) in [
        (
            "/* a\n/* b */\nCREATE TABLE ghost (id int);\n*/\nSELECT 1;",
            vec![2, 3, 4],
        ),
        (
            "/* a /* b */\nCREATE TABLE muffin (id int);\nINSERT INTO muffin VALUES ('*/');",
            vec![],
        ),
        (
            "/* a /* b */\nCREATE TABLE muffin (id int); -- */\nSELECT 1;",
            vec![],
        ),
        (
            "SELECT '/*';\n/* a\nCREATE TABLE ghost (id int);\n*/",
            vec![3, 4],
        ),
    ] {
        assert_eq!(hidden(Kind::Sql, text), want, "{text}");
    }
}
