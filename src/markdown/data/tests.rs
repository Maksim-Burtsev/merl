use super::*;

fn lines(text: &str) -> Vec<String> {
    text.lines().map(String::from).collect()
}

fn table(data: Data, text: &str) -> Vec<Vec<String>> {
    match parse(data, &lines(text)) {
        Ok(Parsed::Table(rows)) => rows.into_iter().map(|(_, r)| r).collect(),
        other => panic!("{other:?}"),
    }
}

fn texts(data: Data, text: &str, width: usize) -> Vec<String> {
    let lines = lines(text);
    let parsed = parse(data, &lines).unwrap();
    let doc = data.layout(Some(&parsed), &lines, width, &mut |_, _| None);
    doc.rows.into_iter().map(|r| r.text).collect()
}

#[test]
fn the_extension_names_the_format_and_nothing_is_sniffed() {
    let of = |name: &str| data(Path::new(name));
    assert_eq!(of("a.CSV"), Some(Data::Csv));
    assert_eq!(of("a.tsv"), Some(Data::Tsv));
    assert_eq!(of("a.ndjson"), Some(Data::Jsonl));
    assert_eq!(of("a.jsonl"), Some(Data::Jsonl));
    assert_eq!(of("a.mmd"), Some(Data::Mermaid));
    assert_eq!(of("a.mermaid"), Some(Data::Mermaid));
    assert_eq!(of("a.json"), None);
    assert_eq!(of("a.txt"), None);
}

#[test]
fn csv_reads_quotes_doubled_quotes_and_line_breaks_inside_a_cell() {
    let rows = table(
        Data::Csv,
        "a,b\n\"x, y\",\"say \"\"hi\"\"\"\n\"two\nlines\",\n",
    );
    assert_eq!(
        rows,
        [
            vec!["a", "b"],
            vec!["x, y", "say \"hi\""],
            vec!["two\nlines", ""],
        ]
    );
}

#[test]
fn csv_that_is_not_whole_says_where() {
    let err = |text: &str| parse(Data::Csv, &lines(text)).unwrap_err();
    assert_eq!(err("a,b\n1,2,3\n"), "line 2 has 3 fields, not 2");
    assert_eq!(err("a,b\n\"1,2\n3,4\n"), "line 2 has an unclosed quote");
    assert_eq!(err("a,b\n1\"x,2\n"), "line 2 has a stray quote");
    assert_eq!(err("a,b\n\"1\"x,2\n"), "line 2 has a stray quote");
    assert_eq!(err("a,b\n\n"), "line 2 has 1 field, not 2");
}

#[test]
fn tsv_splits_on_tabs_and_takes_quotes_as_text() {
    let rows = table(Data::Tsv, "a\tb\n\"x\"\t, y\n");
    assert_eq!(rows, [vec!["a", "b"], vec!["\"x\"", ", y"]]);
    let err = parse(Data::Tsv, &lines("a\tb\nc\n")).unwrap_err();
    assert_eq!(err, "line 2 has 1 field, not 2");
}

#[test]
fn an_empty_file_is_an_empty_preview() {
    for d in [Data::Csv, Data::Tsv, Data::Jsonl] {
        let empty = vec![String::new()];
        let parsed = parse(d, &empty).unwrap();
        let doc = d.layout(Some(&parsed), &empty, 40, &mut |_, _| None);
        assert_eq!(doc.rows.len(), 1, "{d:?}");
        assert_eq!(doc.rows[0].text, "");
    }
}

#[test]
fn a_csv_is_a_table_with_its_first_row_bold_and_a_line_break_kept() {
    let rows = texts(Data::Csv, "id,note\n1,\"two\nlines\"\n", 40);
    assert_eq!(
        rows,
        [
            "┌────┬───────┐",
            "│ id │ note  │",
            "├────┼───────┤",
            "│ 1  │ two   │",
            "│    │ lines │",
            "└────┴───────┘",
        ]
    );
    let lines = lines("id,note\n1,x\n");
    let parsed = parse(Data::Csv, &lines).unwrap();
    let doc = Data::Csv.layout(Some(&parsed), &lines, 40, &mut |_, _| None);
    let bold = |r: usize| {
        doc.rows[r]
            .looks
            .iter()
            .any(|(l, _)| l.mods.contains(ratatui::style::Modifier::BOLD))
    };
    assert!(bold(1) && !bold(3));
    assert_eq!(doc.row_at((1, 0)), 3);
}

#[test]
fn jsonl_is_each_record_pretty_with_its_text_kept_as_written() {
    let text = "{\"b\":1.50,\"a\":[1, 2e3,{}],\"s\":\"x\\\"{,\"}\n\n  42 \n[]\n";
    assert_eq!(
        texts(Data::Jsonl, text, 60),
        [
            " {",
            "   \"b\": 1.50,",
            "   \"a\": [",
            "     1,",
            "     2e3,",
            "     {}",
            "   ],",
            "   \"s\": \"x\\\"{,\"",
            " }",
            "─".repeat(60).as_str(),
            " 42",
            "─".repeat(60).as_str(),
            " []",
        ]
    );
}

#[test]
fn jsonl_with_a_line_that_is_not_json_says_which() {
    let err = parse(Data::Jsonl, &lines("{}\n{\"a\":}\n")).unwrap_err();
    assert_eq!(err, "line 2 is not JSON");
    let err = parse(Data::Jsonl, &lines("{} {}\n")).unwrap_err();
    assert_eq!(err, "line 1 is not JSON");
}

#[test]
fn jsonl_rows_go_back_to_their_record_and_key() {
    let lines = lines("{\"a\":1}\n{\"b\":2}\n");
    let parsed = parse(Data::Jsonl, &lines).unwrap();
    let doc = Data::Jsonl.layout(Some(&parsed), &lines, 40, &mut |_, _| None);
    let srcs: Vec<_> = doc
        .rows
        .iter()
        .map(|r| (r.src, r.kind == Kind::Gap))
        .collect();
    assert_eq!(srcs[1], ((0, 1), false), "the key's row starts at the key");
    assert_eq!(doc.row_at((1, 1)), 5);
    assert!(
        srcs[3].1,
        "the rule between records is where {{ and }} stop"
    );
}

#[test]
fn a_mermaid_file_is_its_diagram_or_its_source() {
    let lines = lines("graph TD\n  A --> B\n");
    let picture = Data::Mermaid.layout(None, &lines, 40, &mut |_, _| Some((10, 3)));
    assert_eq!(picture.rows.len(), 3);
    assert_eq!(picture.code[0].picture, Some((10, 3)));
    let source = Data::Mermaid.layout(None, &lines, 40, &mut |_, _| None);
    let texts: Vec<&str> = source.rows.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(texts, [" graph TD", "   A --> B"]);
}

#[test]
fn a_record_over_two_lines_answers_for_its_first() {
    let err = parse(Data::Csv, &lines("a,b\n\"x\ny\",1,2\n")).unwrap_err();
    assert_eq!(err, "line 2 has 3 fields, not 2");
    let lines = lines("id,note\n1,\"two\nlines\"\n");
    let parsed = parse(Data::Csv, &lines).unwrap();
    let doc = Data::Csv.layout(Some(&parsed), &lines, 40, &mut |_, _| None);
    assert_eq!(doc.rows[doc.row_at((1, 0))].text, "│ 1  │ two   │");
}

#[test]
fn a_markdown_table_border_still_stands_for_its_delimiter_line() {
    let doc = crate::markdown::layout(&lines("| a |\n|---|\n| 1 |\n"), 40, false, &mut |_, _| None);
    assert!(doc.rows[doc.row_at((1, 0))].text.starts_with('├'));
}
