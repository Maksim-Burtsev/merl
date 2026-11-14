use super::*;

fn texts(kind: Kind, file: &str, src: &str) -> Vec<bool> {
    let marks: Vec<usize> = src.match_indices('^').map(|(i, _)| i).collect();
    let text = src.replace('^', "");
    (marks.iter().enumerate())
        .map(|(n, &m)| in_string_text(kind, Path::new(file), &text, m - n))
        .collect()
}

#[test]
fn a_word_in_a_string_s_text_is_quoted_and_one_in_its_hole_is_code() {
    let py = r#"f"^a {^b + 'x'} ^c {{^d}}" + ^e + rf"\{^g}" # "^f"#;
    assert_eq!(
        texts(Kind::Python, "a.py", py),
        [true, false, true, true, false, false, false]
    );
    let ts = "`^a ${`^b ${^c}`}` + '^d' + ^e + s.split(/[/`]/) + ^g / 2 + '^h' // '^f";
    assert_eq!(
        texts(Kind::TsJs, "a.ts", ts),
        [true, true, false, true, false, false, true, false]
    );
    let cs = r#"$@"^a {^b} ""^c""" + @"\" + ^d + """^e""" + '"' + ^f"#;
    assert_eq!(
        texts(Kind::CSharp, "a.cs", cs),
        [true, false, true, false, true, false]
    );
    let go = "`^a\\` + \"^b\\\"\" + '\"' + ^c /* \" */ ^d";
    assert_eq!(texts(Kind::Go, "a.go", go), [true, true, false, false]);
    let dart = r"'^a ${^b} $^c.^d' + r'${^e}' + ^f";
    assert_eq!(
        texts(Kind::Dart, "a.dart", dart),
        [true, false, true, true, true, false]
    );
    let jvm = r#""^a ${^b}" + """^c""" + '"' + ^d"#;
    assert_eq!(texts(Kind::Jvm, "A.kt", jvm), [true, false, true, false]);
    assert_eq!(texts(Kind::Jvm, "A.java", jvm), [true, true, true, false]);
    let scala = r#"s"^a ${^b}" + "${^c}" + 'x + ^d"#;
    assert_eq!(
        texts(Kind::Jvm, "A.scala", scala),
        [true, false, true, false]
    );
    let block = "x = \"\"\"\n  s.^a() \"\n  // b\n  \"\"\" + s.^b()";
    assert_eq!(texts(Kind::CSharp, "a.cs", block), [true, false]);
    assert_eq!(texts(Kind::Jvm, "A.scala", block), [true, false]);
}
