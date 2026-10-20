use super::*;

const NB: &str = r##"{
 "cells": [
  {
   "cell_type": "markdown",
   "metadata": {},
   "source": [
    "# Plots\n",
    "Some *prose*."
   ]
  },
  {
   "cell_type": "code",
   "execution_count": 1,
   "metadata": {},
   "outputs": [
    {
     "name": "stdout",
     "output_type": "stream",
     "text": [
      "hello\n",
      "world\n"
     ]
    },
    {
     "data": {
      "image/png": "iVBORw0KGgo=",
      "text/plain": [
       "<Figure>"
      ]
     },
     "metadata": {},
     "output_type": "display_data"
    },
    {
     "data": {
      "text/plain": [
       "42"
      ]
     },
     "execution_count": 1,
     "metadata": {},
     "output_type": "execute_result"
    },
    {
     "ename": "ZeroDivisionError",
     "evalue": "division by zero",
     "output_type": "error",
     "traceback": [
      "\u001b[0;31mZeroDivisionError\u001b[0m: division by zero"
     ]
    }
   ],
   "source": [
    "import numpy as np\n",
    "print('hello')"
   ]
  }
 ],
 "metadata": {
  "language_info": {
   "name": "python"
  }
 },
 "nbformat": 4,
 "nbformat_minor": 5
}"##;

fn lines(text: &str) -> Vec<String> {
    text.lines().map(String::from).collect()
}

fn doc(text: &str) -> (Vec<String>, Doc) {
    let lines = lines(text);
    let Ok(Parsed::Notebook(nb)) = parse(&lines) else {
        panic!("not a notebook");
    };
    let doc = layout(&nb, lines.len(), 60, false, &mut |_, _| None);
    (lines, doc)
}

#[test]
fn a_notebook_shows_its_markdown_its_code_and_its_outputs_dimmed() {
    let (_, doc) = doc(NB);
    let texts: Vec<&str> = doc.rows.iter().map(|r| r.text.trim_end()).collect();
    assert_eq!(
        texts,
        [
            "# Plots",
            "\u{2501}".repeat(60).as_str(),
            "Some prose.",
            "",
            " import numpy as np",
            " print('hello')",
            "",
            "hello",
            "world",
            "\u{25a3} image/png",
            "42",
            "ZeroDivisionError: division by zero",
        ]
    );
    assert_eq!(doc.code.len(), 1);
    assert_eq!(doc.code[0].lang, "python");
    let dim = |t: &str| {
        let r = doc.rows.iter().find(|r| r.text == t).unwrap();
        r.looks.iter().all(|(l, _)| l.ink == Ink::Dim)
    };
    assert!(dim("hello") && dim("42") && dim("\u{25a3} image/png"));
}

#[test]
fn every_row_points_at_the_line_of_the_json_it_came_from() {
    let (lines, doc) = doc(NB);
    let line_of = |text: &str| {
        let r = doc.rows.iter().find(|r| r.text.trim() == text).unwrap();
        r.src.0
    };
    let json_line = |needle: &str| lines.iter().position(|l| l.contains(needle)).unwrap();
    assert_eq!(line_of("Some prose."), json_line("Some *prose*"));
    assert_eq!(line_of("print('hello')"), json_line("print('hello')\""));
    assert_eq!(line_of("world"), json_line("\"world"));
    assert_eq!(line_of("\u{25a3} image/png"), json_line("image/png"));
    let r = doc
        .rows
        .iter()
        .find(|r| r.text.trim() == "print('hello')")
        .unwrap();
    assert_eq!(lines[r.src.0].get(r.src.1..r.src.1 + 5), Some("print"));
}

#[test]
fn a_notebook_that_is_not_nbformat_4_or_not_json_says_why() {
    let err = |text: &str| parse(&lines(text)).unwrap_err();
    assert_eq!(
        err("{\"nbformat\": 3, \"worksheets\": []}"),
        "nbformat 3 is not 4"
    );
    assert_eq!(err("{\"cells\": []}"), "no nbformat");
    assert_eq!(err("{\n\"nbformat\": 4,\n"), "line 2 is not JSON");
    assert_eq!(
        err("{\"nbformat\": 4,\n \"cells\": [{\"cell_type\": 1}]}"),
        "line 2 is not nbformat 4"
    );
}

#[test]
fn a_single_string_source_and_an_empty_code_cell_still_lay_out() {
    let nb = r#"{"nbformat": 4, "metadata": {"kernelspec": {"language": "julia"}}, "cells": [
{"cell_type": "code", "source": "", "outputs": []},
{"cell_type": "code", "source": "x = 1\ny = 2", "outputs": []},
{"cell_type": "markdown", "source": "Done."}]}"#;
    let (_, doc) = doc(nb);
    let texts: Vec<&str> = doc.rows.iter().map(|r| r.text.trim()).collect();
    assert_eq!(texts, ["", "", "x = 1", "y = 2", "", "Done."]);
    assert_eq!(doc.code[1].lang, "julia");
}

fn row<'a>(doc: &'a Doc, text: &str) -> &'a Row {
    doc.rows.iter().find(|r| r.text.trim() == text).unwrap()
}

#[test]
fn a_row_s_column_counts_the_escapes_and_line_breaks_of_the_json_string() {
    let nb = r#"{"nbformat": 4, "metadata": {}, "cells": [
{"cell_type": "code", "outputs": [], "source": "x = \"ab\" + \"cd\" + \"ef\"\n\tyy = 2"}]}"#;
    let lines = lines(nb);
    let Ok(Parsed::Notebook(nb)) = parse(&lines) else {
        panic!("not a notebook");
    };
    let doc = layout(&nb, lines.len(), 12, false, &mut |_, _| None);
    let texts: Vec<&str> = doc.rows.iter().map(|r| r.text.trim()).collect();
    assert_eq!(texts, ["x = \"ab\" +", "\"cd\" +", "\"ef\"", "yy = 2"]);
    let at = |text: &str| {
        let r = row(&doc, text);
        lines[r.src.0].get(r.src.1..).unwrap()
    };
    assert!(at("x = \"ab\" +").starts_with("x = "));
    assert!(at("\"cd\" +").starts_with("\\\"cd\\\""));
    assert!(at("\"ef\"").starts_with("\\\"ef\\\""));
    assert!(at("yy = 2").starts_with("\\tyy = 2"));
}

#[test]
fn each_traceback_entry_and_each_of_its_lines_is_a_row_of_its_own() {
    let nb = r#"{"nbformat": 4, "metadata": {}, "cells": [
{"cell_type": "code", "source": "1/0", "outputs": [{"output_type": "error", "traceback": [
"\u001b[31m---\u001b[0m",
"Cell In[1]\n\u001b[1mline two\u001b[0m"]}]}]}"#;
    let (lines, doc) = doc(nb);
    let texts: Vec<&str> = doc.rows.iter().map(|r| r.text.trim()).collect();
    assert_eq!(texts, ["1/0", "", "---", "Cell In[1]", "line two"]);
    let r = row(&doc, "line two");
    assert!(
        lines[r.src.0]
            .get(r.src.1..)
            .unwrap()
            .starts_with("line two")
    );
}

#[test]
fn the_notebook_s_language_wins_over_the_kernel_s_and_a_raw_cell_has_none() {
    let nb = r#"{"nbformat": 4, "metadata": {"language_info": {"name": "r"}, "kernelspec": {"language": "python"}}, "cells": [
{"cell_type": "markdown", "source": []},
{"cell_type": "code", "source": "x <- 1", "outputs": []},
{"cell_type": "raw", "source": "a: 1"}]}"#;
    let (_, doc) = doc(nb);
    let langs: Vec<&str> = doc.code.iter().map(|c| c.lang.as_str()).collect();
    assert_eq!(langs, ["r", ""]);
    let plain = r#"{"nbformat": 4, "metadata": {}, "cells": [{"cell_type": "code", "source": "x", "outputs": []}]}"#;
    assert_eq!(self::doc(plain).1.code[0].lang, "python");
}

#[test]
fn a_cell_s_own_lines_belong_to_its_first_row_though_its_outputs_come_first_in_the_json() {
    let nb = r#"{
 "cells": [
  {
   "cell_type": "code",
   "outputs": [
    {
     "output_type": "stream",
     "text": [
      "printed\n"
     ]
    }
   ],
   "source": [
    "x = 1"
   ]
  },
  {
   "cell_type": "markdown",
   "source": [
    "Next"
   ]
  }
 ],
 "metadata": {},
 "nbformat": 4
}"#;
    let (lines, doc) = doc(nb);
    let line = |needle: &str| lines.iter().rposition(|l| l.contains(needle)).unwrap();
    let at = |l: usize| doc.rows[doc.row_at((l, 0))].text.trim().to_owned();
    assert_eq!(at(0), "x = 1");
    assert_eq!(at(line("\"source\": [")), "Next");
    assert_eq!(
        at(line("\"source\": [") - 3),
        "x = 1",
        "the code's `\"source\": [`"
    );
    assert_eq!(at(line("\"cell_type\": \"markdown\"")), "Next");
    assert_eq!(at(line("printed")), "printed");
}
