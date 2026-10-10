use super::*;

fn hidden(kind: Kind, text: &str) -> Vec<usize> {
    (literal_lines(kind, text).iter().enumerate())
        .filter(|(_, l)| **l)
        .map(|(i, _)| i + 1)
        .collect()
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
