use super::*;

fn doc(text: &str, width: usize) -> Doc {
    let lines: Vec<String> = text.lines().map(String::from).collect();
    layout(&lines, width, &mut |_, _| None)
}

fn texts(d: &Doc) -> Vec<&str> {
    d.rows.iter().map(|r| r.text.as_str()).collect()
}

fn look_of(d: &Doc, what: &str) -> Look {
    for r in &d.rows {
        if let Some(i) = r.text.find(what) {
            let (look, range) = r
                .looks
                .iter()
                .find(|(_, range)| range.start <= i && i < range.end)
                .unwrap_or_else(|| panic!("no look over {what:?} in {:?}", r.text));
            assert!(
                range.end >= i + what.len(),
                "{what:?} is cut: {:?}",
                r.looks
            );
            return *look;
        }
    }
    panic!("{what:?} is not in {:?}", texts(d));
}

fn look(ink: Ink, mods: Modifier) -> Look {
    Look { ink, mods }
}

#[test]
fn headings_lose_their_marks_and_the_first_two_get_a_rule() {
    let d = doc("# One\n## Two\n### Three\n#### Four\ntext", 10);
    assert_eq!(
        texts(&d),
        [
            "One",
            "\u{2501}".repeat(10).as_str(),
            "Two",
            "\u{2500}".repeat(10).as_str(),
            "Three",
            "Four",
            "text",
        ]
    );
    for h in ["One", "Two", "Three", "Four"] {
        assert_eq!(look_of(&d, h), look(Ink::Heading, Modifier::BOLD), "{h}");
    }
    assert_eq!(look_of(&d, "text"), Ink::Text.plain());
}

#[test]
fn inline_styles_and_links_without_their_urls() {
    let d = doc(
        "**bold** *it* ~~gone~~ `code` [site](https://example.com) ![logo](a.png) <b>",
        80,
    );
    assert_eq!(texts(&d), ["bold it gone code site \u{25a3} logo <b>"]);
    assert_eq!(look_of(&d, "bold"), look(Ink::Text, Modifier::BOLD));
    assert_eq!(look_of(&d, "it"), look(Ink::Text, Modifier::ITALIC));
    assert_eq!(look_of(&d, "gone"), look(Ink::Text, Modifier::CROSSED_OUT));
    assert_eq!(look_of(&d, "code"), Ink::Code.plain());
    assert_eq!(look_of(&d, "site"), look(Ink::Link, Modifier::UNDERLINED));
    assert_eq!(look_of(&d, "\u{25a3} logo"), Ink::Dim.plain());
    assert_eq!(look_of(&d, "<b>"), Ink::Dim.plain());
}

#[test]
fn lists_nest_and_wrap_under_their_text() {
    let d = doc(
        "- first item wraps here\n  1. one\n  2. two\n- [x] done\n- [ ] open\n\n3. three\n4. four",
        14,
    );
    assert_eq!(
        texts(&d),
        [
            "\u{2022} first item",
            "  wraps here",
            "  1. one",
            "  2. two",
            "\u{2611} done",
            "\u{2610} open",
            "",
            "3. three",
            "4. four",
        ]
    );
    assert_eq!(look_of(&d, "\u{2022}"), Ink::Bullet.plain());
    assert_eq!(look_of(&d, "3."), Ink::Bullet.plain());
    assert_eq!(
        texts(&doc("- a\n  - b", 10)),
        ["\u{2022} a", "  \u{25e6} b"],
        "a second level of bullets"
    );
}

#[test]
fn a_loose_list_keeps_its_items_apart() {
    assert_eq!(
        texts(&doc("- a\n\n- b\n- c", 10)),
        ["\u{2022} a", "", "\u{2022} b", "", "\u{2022} c"]
    );
}

#[test]
fn quotes_and_alerts_have_a_bar() {
    let d = doc(
        "> said\n> twice\n\n> [!WARNING]\n> Careful with this one",
        16,
    );
    assert_eq!(
        texts(&d),
        [
            "\u{2502} said twice",
            "",
            "\u{2502} Warning",
            "\u{2502} Careful with",
            "\u{2502} this one",
        ]
    );
    assert_eq!(look_of(&d, "said"), Ink::Quote.plain());
    let warning = Ink::Alert(BlockQuoteKind::Warning);
    assert_eq!(look_of(&d, "Warning"), look(warning, Modifier::BOLD));
    assert_eq!(
        look_of(&d, "Careful"),
        Ink::Text.plain(),
        "an alert's text is not greyed"
    );
    assert_eq!(
        d.rows[3].looks[0],
        (warning.plain(), 0..3),
        "an alert's bar is in its colour"
    );
}

#[test]
fn code_blocks_lose_their_fences_and_wrap_under_their_indent() {
    let d = doc(
        "```rust\nfn main() {\n\tlet x = compute(alpha, beta);\n}\n```\n\n    indented",
        24,
    );
    assert_eq!(
        texts(&d),
        [
            " fn main() {",
            "     let x = ",
            "     compute(alpha, ",
            "     beta);",
            " }",
            "",
            " indented",
        ]
    );
    assert_eq!(d.code[0].lang, "rust");
    assert_eq!(d.code[0].lines[1], "    let x = compute(alpha, beta);");
    assert_eq!(d.code[1].lang, "");
    let raw = "\tlet x = compute(alpha, beta);";
    assert_eq!(
        (d.rows[2].src, d.rows[3].src),
        ((2, 9), (2, 24)),
        "a row of a tab-indented line starts where its text is written"
    );
    assert!(raw[9..].starts_with("compute") && raw[24..].starts_with("beta"));
    // Rows met in order: each tab passed once, the second of two as the first.
    let mut tabs = Tabs::new("ab\t\tcd");
    assert_eq!(tabs.raw(1), 1);
    assert_eq!(tabs.raw(3), 2, "inside the first tab's spaces");
    assert_eq!(tabs.raw(7), 3, "inside the second's");
    assert_eq!(tabs.raw(11), 5, "past both");
    assert_eq!(Tabs::new("plain").raw(4), 4);
    assert_eq!(
        d.rows[2].kind,
        Kind::Code {
            block: 0,
            line: 1,
            from: 12,
            at: 5,
        },
        "each row knows the part of its line it shows, for the syntax colours"
    );
    assert!(
        d.rows
            .iter()
            .filter(|r| matches!(r.kind, Kind::Code { .. }))
            .all(|r| r.looks == [(Ink::Code.plain(), 0..r.text.len())])
    );
}

#[test]
fn tables_are_drawn_in_lines_and_aligned() {
    let d = doc(
        "| Left | Mid | Right |\n|:---|:---:|---:|\n| a | b | c |\n| longer | x | 1 |",
        80,
    );
    assert_eq!(
        texts(&d),
        [
            "\u{250c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{252c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{252c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2510}",
            "\u{2502} Left   \u{2502} Mid \u{2502} Right \u{2502}",
            "\u{251c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{253c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{253c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2524}",
            "\u{2502} a      \u{2502}  b  \u{2502}     c \u{2502}",
            "\u{2502} longer \u{2502}  x  \u{2502}     1 \u{2502}",
            "\u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2534}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2534}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2518}",
        ]
    );
    assert_eq!(look_of(&d, "Left"), look(Ink::Text, Modifier::BOLD));
    assert_eq!(look_of(&d, "longer"), Ink::Text.plain());
}

/// Every row of a table as wide as the top border: the table is whole.
fn whole(d: &Doc) -> usize {
    let w = wrap::width(&d.rows[0].text);
    for r in &d.rows {
        assert_eq!(wrap::width(&r.text), w, "{:?}", texts(d));
    }
    w
}

#[test]
fn a_table_wider_than_the_pane_narrows_its_widest_column_and_stays_whole() {
    let src = "| Step | Owner | Risk |\n|---|---|---|\n| watcher | Kimi | low |\n\
               | diff recompute | Claude | medium: the cursor must not jump |";
    let d = doc(src, 44);
    assert_eq!(whole(&d), 44);
    assert_eq!(
        texts(&d)[3..7],
        [
            "\u{2502} watcher        \u{2502} Kimi   \u{2502} low            \u{2502}",
            "\u{2502} diff recompute \u{2502} Claude \u{2502} medium: the    \u{2502}",
            "\u{2502}                \u{2502}        \u{2502} cursor must    \u{2502}",
            "\u{2502}                \u{2502}        \u{2502} not jump       \u{2502}",
        ]
    );
    assert!(
        whole(&doc(src, 20)) <= 20,
        "narrower still, every column gives way"
    );
    assert_eq!(
        whole(&doc(src, 200)),
        62,
        "at its natural width it is as wide as it needs, no wider"
    );
}

#[test]
fn emoji_and_cjk_take_the_columns_they_are_drawn_in() {
    let d = doc("漢字漢字 👍🏽👍🏽 ⚠️⚠️ ok", 8);
    assert_eq!(
        texts(&d),
        ["漢字漢字", "👍🏽👍🏽", "⚠️⚠️ ok"],
        "two-column clusters fill a row of eight in fours"
    );
    for r in &d.rows {
        assert!(wrap::width(&r.text) <= 8, "{:?}", r.text);
    }
    // In a table they line the borders up as ASCII does.
    let d = doc("| a | b |\n|---|---|\n| 漢字 | x |\n| 👍🏽 ok | ⚠️ |", 40);
    whole(&d);
    let d = doc(
        "| a | b |\n|---|---|\n| 漢字漢字漢字 | x |\n| 👍🏽 ok | ⚠️ |",
        12,
    );
    whole(&d);
}

#[test]
fn rules_front_matter_html_and_footnotes() {
    let d = doc(
        "---\ntitle: x\n---\nA claim[^src].\n\n---\n\n<div>raw</div>\n\n[^src]: The source \
         of it.",
        16,
    );
    let rule = "\u{2500}".repeat(16);
    assert_eq!(
        texts(&d),
        [
            "---",
            "title: x",
            "---",
            "",
            "A claim[1].",
            "",
            rule.as_str(),
            "",
            "<div>raw</div>",
            "",
            "[1] The source",
            "    of it.",
        ]
    );
    assert_eq!(look_of(&d, "title: x"), Ink::Dim.plain());
    assert_eq!(look_of(&d, "<div>"), Ink::Dim.plain());
    assert_eq!(look_of(&d, "[1]"), Ink::Link.plain());
}

#[test]
fn every_row_knows_where_it_starts_in_the_source() {
    let src = "# Title\n\nSome words that\nwrap on.\n\n- item\n- next\n\n```\ncode\n```\n\n\
               | a | b |\n|---|---|\n| 1 | 2 |";
    let d = doc(src, 10);
    let at: Vec<(&str, (usize, usize))> = d.rows.iter().map(|r| (r.text.as_str(), r.src)).collect();
    let m = usize::MAX;
    assert_eq!(
        at,
        [
            ("Title", (0, 2)),
            // What a heading heads follows its rule without a blank row.
            ("\u{2501}".repeat(10).as_str(), (0, 2)),
            ("Some words", (2, 0)),
            ("that wrap", (2, 11)),
            ("on.", (3, 5)),
            // A blank row stands for the blank line above the next block.
            ("", (4, 0)),
            ("\u{2022} item", (5, 2)),
            ("\u{2022} next", (6, 2)),
            ("", (7, 0)),
            (" code", (9, 0)),
            ("", (11, 0)),
            (
                "\u{250c}\u{2500}\u{2500}\u{2500}\u{252c}\u{2500}\u{2500}\u{2500}\u{2510}",
                (12, 0)
            ),
            ("\u{2502} a \u{2502} b \u{2502}", (12, 0)),
            (
                "\u{251c}\u{2500}\u{2500}\u{2500}\u{253c}\u{2500}\u{2500}\u{2500}\u{2524}",
                (13, 0)
            ),
            ("\u{2502} 1 \u{2502} 2 \u{2502}", (14, 0)),
            (
                "\u{2514}\u{2500}\u{2500}\u{2500}\u{2534}\u{2500}\u{2500}\u{2500}\u{2518}",
                (14, m)
            ),
        ]
    );
    // From the source back: the row that shows the position's line and column. Column 0 of a
    // heading or a list item, on its marker, is the row of its text.
    assert_eq!(d.row_at((0, 0)), 0);
    assert_eq!(d.row_at((2, 5)), 2);
    assert_eq!(d.row_at((2, 14)), 3);
    assert_eq!(d.row_at((3, 0)), 3);
    assert_eq!(d.row_at((5, 0)), 6);
    assert_eq!(d.row_at((6, 0)), 7);
    // A code block's fences are its rows'.
    assert_eq!(d.row_at((8, 0)), 9);
    assert_eq!(d.row_at((10, 3)), 9);
    assert_eq!(d.row_at((14, 4)), 14);
    for (i, r) in d.rows.iter().enumerate() {
        if d.rows[..i].iter().all(|o| o.src != r.src) && !r.lines.is_empty() {
            assert_eq!(
                d.row_at(r.src),
                i,
                "{:?} is not found again by its own position",
                r.text
            );
        }
    }
}

#[test]
fn an_empty_file_is_one_blank_row() {
    let d = doc("", 10);
    assert_eq!(texts(&d), [""]);
    assert_eq!(d.row_at((0, 0)), 0);
}

#[test]
fn fit_takes_the_room_from_the_widest_columns() {
    assert_eq!(fit(&[3, 4], &[3, 4], 10), [3, 4]);
    assert_eq!(fit(&[3, 20, 10], &[3, 5, 5], 20), [3, 9, 8]);
    assert_eq!(
        fit(&[19, 43, 6, 64], &[19, 6, 6, 7], 54),
        [19, 15, 6, 14],
        "a long word keeps its column that wide while every column's word fits"
    );
    assert_eq!(
        fit(&[19, 43, 6, 64], &[19, 6, 6, 7], 34),
        [10, 9, 6, 9],
        "the words do not all fit: they break, the widest columns first"
    );
    assert_eq!(fit(&[5, 5], &[5, 5], 3), [2, 1]);
    assert_eq!(
        fit(&[5, 5], &[5, 5], 0),
        [1, 1],
        "no room at all: a column each, and the table runs past the pane"
    );
}

#[test]
fn a_run_of_spaces_wider_than_the_pane_leaves_no_blank_rows() {
    assert_eq!(
        texts(&doc("`        ` word", 3)),
        ["wor", "d"],
        "a code span of spaces opens the paragraph"
    );
}

#[test]
fn a_task_box_indents_its_item_by_its_own_width() {
    assert_eq!(
        texts(&doc("1. [ ] alpha beta gamma", 10)),
        ["\u{2610} alpha", "  beta", "  gamma"],
        "the box replaces `1. `, so the rows after the first go two columns in"
    );
}

#[test]
fn every_line_belongs_to_a_row() {
    let d = doc(
        "Title\n=====\nText [a].\n\n[a]: http://x\n\n```py\nx = 1\n```",
        20,
    );
    let text = |l: usize| d.rows[d.row_at((l, 0))].text.trim_start().to_string();
    let texts: Vec<String> = (0..9).map(text).collect();
    assert_eq!(
        texts,
        [
            "Title", "Title", "Text a.", "Text a.", "Text a.", "", "x = 1", "x = 1", "x = 1"
        ],
        "a setext underline goes with its heading; a reference definition and the blank line \
         above it with the paragraph above; the fences with their code, the blank line before \
         them with the blank row"
    );
    for l in 0..9 {
        let n = d
            .rows
            .iter()
            .filter(|r| r.lines.contains(&l) || r.owns.contains(&l));
        assert_eq!(
            n.count(),
            1,
            "line {l} has more than one row: its marks are counted twice"
        );
    }
}

#[test]
fn text_a_container_took_part_of_a_tab_from_stays_on_its_line() {
    // pulldown-cmark hands the spaces left of a tab in front of the line as text of their own.
    let d = doc("- a\n\n\t\tcode\n\t\tmore\n", 40);
    assert_eq!(d.code[0].lines, ["  code", "  more"]);
    assert_eq!(texts(&d), ["\u{2022} a", "", "     code", "     more"]);
    let d = doc("- a\n\n\t<div>x</div>\n\n\tpara\n", 40);
    assert_eq!(
        texts(&d),
        ["\u{2022} a", "", "  <div>x</div>", "", "  para"],
        "before an HTML block they are no text that ends the blank row after it"
    );
}

#[test]
fn prose_rows_go_back_to_where_their_text_is_written() {
    let d = doc("aaaa\tbbbb cccc", 10);
    assert_eq!(
        (d.rows[1].text.as_str(), d.rows[1].src),
        ("bbbb cccc", (0, 5)),
        "after a tab"
    );
    let d = doc("see `aaaa bbbb cccc dddd` end", 10);
    assert_eq!(
        (d.rows[1].text.as_str(), d.rows[1].src),
        ("bbbb cccc", (0, 10)),
        "inside inline code, past its backtick"
    );
}

#[test]
fn footnotes_are_drawn_where_they_are_written() {
    let d = doc("B[^b] A[^a].\n\n[^a]: note a\n\n[^b]: note b\n\nAfter.", 40);
    assert_eq!(
        texts(&d),
        [
            "B[1] A[2].",
            "",
            "[2] note a",
            "",
            "[1] note b",
            "",
            "After."
        ],
        "numbered as they are first met, drawn in place under their label, in source order"
    );
    assert_eq!(d.rows[d.row_at((2, 0))].text, "[2] note a");
    assert_eq!(look_of(&d, "[2] "), Ink::Link.plain());
}

#[test]
fn a_footnote_is_numbered_as_its_reference_whatever_the_case() {
    let d = doc("Text[^Note].\n\n[^note]: body\n", 40);
    assert_eq!(
        texts(&d),
        ["Text[1].", "", "[1] body"],
        "the parser pairs `[^Note]` with `[^note]:`, folding case; so does the number"
    );
}

#[test]
fn an_empty_footnote_shows_its_label() {
    let d = doc("Text[^1].\n\n[^1]:\n\nAfter.\n", 40);
    let rows: Vec<&str> = d.rows.iter().map(|r| r.text.trim_end()).collect();
    assert_eq!(rows, ["Text[1].", "", "[1]", "", "After."]);
    assert_eq!(d.row_at((2, 0)), 2);
}

#[test]
fn an_info_string_names_its_language_by_its_first_word() {
    let d = doc(
        "```rust,ignore\nfn f() {}\n```\n\n```py title=x\npass\n```",
        40,
    );
    let langs: Vec<&str> = d.code.iter().map(|c| c.lang.as_str()).collect();
    assert_eq!(langs, ["rust", "py"]);
}

#[test]
fn lines_after_a_footnote_go_with_it() {
    let d = doc(
        "Intro[^a].\n\n[^a]: Note.\n\n[r]: http://x\n\nNext para.\n",
        40,
    );
    let note = d.row_at((2, 0));
    assert_eq!(d.rows[note].text, "[1] Note.");
    assert_eq!(
        (d.row_at((3, 0)), d.row_at((4, 0))),
        (note, note),
        "the blank line and the reference definition after a footnote go with it, as with any row"
    );
    assert!(d.row_at((6, 0)) > note);
}

fn in_source_order(d: &Doc) -> Result<(), String> {
    let mut last = 0;
    for (i, r) in d.rows.iter().enumerate() {
        let first = r.lines.start;
        if first < last {
            return Err(format!(
                "row {i} {:?} starts at line {first}, after {last}",
                r.text
            ));
        }
        last = first;
    }
    Ok(())
}

#[test]
fn the_rows_are_in_source_order() {
    let fixtures = [
        "B[^b] A[^a].\n\n[^a]: note a\n\n[^b]: note b\n\nAfter.",
        "Intro[^a].\n\n[^a]: Note.\n\n[r]: http://x\n\nNext para.\n",
        "---\ntitle: x\n---\nA claim[^src].\n\n---\n\n<div>raw</div>\n\n[^src]: The source.",
        "# Title\n\nSome words that\nwrap on.\n\n- item\n- next\n\n```\ncode\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |",
        "> said\n> twice\n\n> [!WARNING]\n> Careful with this one",
        "- first item wraps here\n  1. one\n  2. two\n- [x] done\n\n3. three",
        "Title\n=====\nText [a].\n\n[a]: http://x\n\n```py\nx = 1\n```",
        "[a]: http://x\n\n[b]: http://y\n",
        "",
        include_str!("../../tutor/notes/PLAN.md"),
        include_str!("../../README.md"),
        include_str!("../../CHANGELOG.md"),
        include_str!("../../docs/markdown.md"),
    ];
    for (k, text) in fixtures.iter().enumerate() {
        for width in [12, 40, 100] {
            if let Err(e) = in_source_order(&doc(text, width)) {
                panic!("fixture {k} at {width}: {e}");
            }
        }
    }
}

#[test]
fn the_spaces_the_parser_adds_are_no_source_columns() {
    let d = doc("- a\n\n\t\tlet total = compute(alpha, beta);\n", 20);
    let raw = "\t\tlet total = compute(alpha, beta);";
    let row = d
        .rows
        .iter()
        .find(|r| r.text.trim_start().starts_with("compute"))
        .unwrap();
    assert!(
        raw[row.src.1..].starts_with("compute"),
        "a wrapped row goes back to where its text is written, not {:?}",
        row.src
    );
}

#[test]
fn alerts_take_githubs_light_or_dark_colours_as_the_theme_is() {
    let note = |name: &str| {
        let theme = crate::theme::load(name).unwrap();
        let look = Look {
            ink: Ink::Alert(BlockQuoteKind::Note),
            mods: Modifier::BOLD,
        };
        Palette::new(&theme).style(look, false).fg
    };
    assert_eq!(note("github-light"), Some(Color::from_u32(0x0969da)));
    assert_eq!(note("tokyonight-moon"), Some(Color::from_u32(0x4493f8)));
}

fn with_pictures(text: &str, width: usize, size: (u16, u16)) -> (Doc, Vec<(String, usize)>) {
    let lines: Vec<String> = text.lines().map(String::from).collect();
    let mut asked = Vec::new();
    let doc = layout(&lines, width, &mut |src, room| {
        asked.push((src.to_string(), room));
        Some(size)
    });
    (doc, asked)
}

#[test]
fn a_mermaid_block_is_rows_for_its_picture_standing_for_its_lines() {
    let text = "Intro\n\n```mermaid\ngraph TD\n  A-->B\n  B-->C\n```\n\nAfter";
    let (d, asked) = with_pictures(text, 40, (12, 6));
    assert_eq!(asked, vec![("graph TD\n  A-->B\n  B-->C".to_string(), 38)]);
    assert_eq!(d.code[0].picture, Some((12, 6)));
    let pics: Vec<&Row> = d
        .rows
        .iter()
        .filter(|r| matches!(r.kind, Kind::Code { block: 0, .. }))
        .collect();
    assert_eq!(pics.len(), 6);
    assert!(pics.iter().all(|r| r.text.is_empty()));
    assert_eq!(pics[0].lines.start, 2);
    assert_eq!(pics[5].lines.end, 7);
    let at: Vec<usize> = pics.iter().map(|r| r.src.0).collect();
    assert!(at.windows(2).all(|w| w[0] <= w[1]));
    assert_eq!((at[0], at[5]), (3, 5));
    assert!(texts(&d).contains(&"After"));
    let first = d
        .rows
        .iter()
        .position(|r| r.text.is_empty() && matches!(r.kind, Kind::Code { .. }));
    assert_eq!(d.row_at((4, 0)), first.unwrap() + 2);
}

#[test]
fn only_mermaid_blocks_ask_for_a_picture() {
    let (d, asked) = with_pictures(
        "```rust\nfn main() {}\n```\n\n```\ngraph TD\n```",
        40,
        (5, 2),
    );
    assert!(asked.is_empty());
    assert!(d.code.iter().all(|c| c.picture.is_none()));
    assert!(texts(&d).iter().any(|t| t.contains("fn main()")));
}

#[test]
fn a_mermaid_block_without_a_picture_is_its_source() {
    let d = doc("```mermaid\ngraph TD\n  A-->B\n```", 40);
    assert_eq!(d.code[0].picture, None);
    assert!(texts(&d).iter().any(|t| t.contains("A-->B")));
}
