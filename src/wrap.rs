use std::ops::Range;

use ratatui::buffer::CellWidth;
use unicode_segmentation::UnicodeSegmentation;

pub fn cluster_width(g: &str) -> usize {
    match g.chars().next() {
        Some('\t') => crate::buffer::TAB.len(),
        Some(c) if hidden(c) => TAG_W,
        Some(c) if c.is_control() => 0,
        _ => g.cell_width().into(),
    }
}

pub fn hidden(c: char) -> bool {
    matches!(
        c,
        '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{061c}'
            | '\u{200b}'
            | '\u{2060}'
            | '\u{feff}'
    )
}

pub fn tag(c: char) -> String {
    format!("<{:04x}>", c as u32)
}

const TAG_W: usize = 6;

pub fn clusters(s: &str) -> impl Iterator<Item = (usize, &str)> {
    let ascii = s.is_ascii();
    let bytes = ascii.then(|| (0..s.len()).map(move |i| (i, &s[i..=i])));
    let graphemes = (!ascii).then(|| s.grapheme_indices(true));
    bytes
        .into_iter()
        .flatten()
        .chain(graphemes.into_iter().flatten())
}

pub fn width(s: &str) -> usize {
    clusters(s).map(|(_, g)| cluster_width(g)).sum()
}

pub fn wrap_line(line: &str, width: usize) -> Vec<Range<usize>> {
    let width = width.max(1);
    let rest = width - indent(line, width);
    let mut rows = Vec::new();
    let mut start = 0usize;
    let mut used = 0usize;
    let mut row_has_text = false;
    let mut after_space = false;
    let mut last_word_start = None;
    let mut last_punct_break = None;
    let mut prev = ' ';
    for (i, g) in clusters(line) {
        let w = cluster_width(g);
        if w == 0 {
            continue;
        }
        let c = g.chars().next().unwrap_or(' ');
        let space = c == ' ' || c == '\t';
        if !(space && row_has_text) {
            if after_space && row_has_text {
                last_word_start = Some(i);
            }
            if breaks_after(prev) && !breaks_after(c) {
                last_punct_break = Some(i);
            }
            let room = if rows.is_empty() { width } else { rest };
            if used + w > room && i > start {
                let end = match last_word_start {
                    Some(b) if self::width(&line[b..word_end(line, b)]) <= rest => b,
                    Some(b) => last_punct_break.filter(|&p| p > b).unwrap_or(i),
                    None => last_punct_break.unwrap_or(i),
                };
                rows.push(start..end);
                start = end;
                used = self::width(&line[end..i]);
                (last_word_start, last_punct_break) = (None, None);
            }
        }
        used += w;
        row_has_text |= !space;
        after_space = space;
        prev = c;
    }
    rows.push(start..line.len());
    rows
}

pub fn wrap_shown(line: &str, width: usize) -> Vec<Range<usize>> {
    let shown = crate::buffer::shown_str(line);
    let mut rows = wrap_line(shown, width);
    if crate::buffer::Buffer::clips(line) {
        let lead = if rows.len() > 1 {
            indent(shown, width)
        } else {
            0
        };
        let last = rows[rows.len() - 1].clone();
        if lead + self::width(&shown[last]) >= width.max(1) {
            rows.push(shown.len()..shown.len());
        }
    }
    rows
}

fn breaks_after(c: char) -> bool {
    matches!(c, '/' | '.' | ',' | ';' | ')' | ']' | '}')
}

fn word_end(line: &str, i: usize) -> usize {
    line[i..].find([' ', '\t']).map_or(line.len(), |n| i + n)
}

pub fn indent(line: &str, width: usize) -> usize {
    let text = line.trim_start_matches([' ', '\t']);
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    let marker = match text.as_bytes().get(digits) {
        Some(b'-' | b'*' | b'+') if digits == 0 => 1,
        Some(b'.' | b')') if digits > 0 => digits + 1,
        _ => 0,
    };
    let after = &text[marker..];
    let gap = after.len() - after.trim_start_matches(' ').len();
    let lead = line.len() - text.len() + if gap > 0 { marker + gap } else { 0 };
    let cols = self::width(&line[..lead]);
    if cols * 2 > width { 0 } else { cols }
}

pub fn cut(line: &str, from: usize, to: usize) -> (Range<usize>, usize) {
    let (mut start, mut lead) = (None, 0);
    let mut x = 0;
    for (i, g) in clusters(line) {
        let w = cluster_width(g);
        if start.is_none() && x >= from && w > 0 {
            start = Some(i);
            lead = (x - from).min(to.saturating_sub(from));
        }
        if start.is_some() && x + w > to {
            return (start.unwrap_or(i)..i, lead);
        }
        x += w;
    }
    match start {
        Some(s) => (s..line.len(), lead),
        None => (
            line.len()..line.len(),
            x.saturating_sub(from).min(to.saturating_sub(from)),
        ),
    }
}

pub fn col_to_row(rows: &[Range<usize>], byte_col: usize) -> usize {
    rows.iter().rposition(|r| r.start <= byte_col).unwrap_or(0)
}

pub fn row_to_col(rows: &[Range<usize>], row: usize) -> usize {
    rows[row.min(rows.len() - 1)].start
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every hidden char is as wide as its tag, and wraps and cuts as a whole (#401); a ZWJ in an
    /// emoji and a ZWNJ in Persian stay what they were.
    #[test]
    fn a_hidden_char_is_as_wide_as_its_tag() {
        let all = "\u{202a}\u{202b}\u{202c}\u{202d}\u{202e}\u{2066}\u{2067}\u{2068}\u{2069}\
                   \u{200e}\u{200f}\u{061c}\u{200b}\u{2060}\u{feff}";
        for c in all.chars() {
            assert!(hidden(c), "{c:?}");
            assert_eq!(width(&c.to_string()), tag(c).len(), "{c:?}");
        }
        assert_eq!(tag('\u{202e}'), "<202e>");
        assert_eq!(tag('\u{061c}'), "<061c>");
        assert_eq!(width("abc\u{200b}def"), 12);
        assert_eq!(width("\u{1f468}\u{200d}\u{1f4bb}"), 2);
        assert_eq!(width("\u{0645}\u{06cc}\u{200c}\u{062e}"), 3);
        assert_eq!(
            wrap_line("ab\u{202e}cd", 7),
            vec![0..2, 2..6, 6..7],
            "a tag moves to the next row whole, as a wide char does"
        );
        assert_eq!(cut("ab\u{202e}cd", 0, 8), (0..5, 0));
    }

    #[test]
    fn cut_keeps_the_chars_that_fit_whole() {
        assert_eq!(cut("abcdefgh", 0, 4), (0..4, 0));
        assert_eq!(cut("abcdefgh", 2, 5), (2..5, 0));
        assert_eq!(cut("abcdefgh", 6, 20), (6..8, 0));
        assert_eq!(
            cut("abc", 5, 9),
            (3..3, 0),
            "a line that ends left of the window shows nothing"
        );
        assert_eq!(cut("", 0, 9), (0..0, 0));
        assert_eq!(
            cut("сбоев", 1, 3),
            (2..6, 0),
            "Cyrillic is two bytes a char"
        );
        assert_eq!(
            cut("\tab", 2, 8),
            (1..3, 2),
            "a tab straddling the left edge leaves its visible part blank"
        );
        assert_eq!(
            cut("a\u{4e2d}b", 0, 2),
            (0..1, 0),
            "a wide char that does not fit at the right edge is left out"
        );
        assert_eq!(
            cut("a\t", 2, 8),
            (2..2, 3),
            "a tab straddling the left edge at the end of the line: the line ends past its blank \
             part, where a cut line's `…` goes"
        );
        assert_eq!(
            cut("a\u{4e2d}", 2, 8),
            (4..4, 1),
            "a wide char straddling the left edge at the end of the line"
        );
    }

    #[test]
    fn breaks_between_words() {
        assert_eq!(
            wrap_line("foo bar baz", 9),
            vec![0..8, 8..11],
            "the space stays at the end of the upper row, so the lower one starts with a word"
        );
        assert_eq!(
            wrap_line("сбоев лимит", 8),
            vec![0..11, 11..21],
            "Cyrillic is two bytes a char"
        );
        assert_eq!(
            wrap_line("foo  bar", 3),
            vec![0..5, 5..8],
            "spaces that do not fit hang past the edge instead of opening a row"
        );
        assert_eq!(
            wrap_line("a\u{a0}bc d", 3),
            vec![0..4, 4..7],
            "a no-break space is part of the word"
        );
    }

    #[test]
    fn a_word_longer_than_a_row_starts_in_place() {
        assert_eq!(
            wrap_line("see a/b/c/d", 6),
            vec![0..6, 6..11],
            "it breaks after punctuation inside it when it can"
        );
        assert_eq!(wrap_line("sum: 0123456789", 8), vec![0..8, 8..15]);
        assert_eq!(
            wrap_line("a bcdefgh", 4),
            vec![0..4, 4..8, 8..9],
            "by char when it cannot"
        );
        assert_eq!(
            wrap_line("read CLAUDE.md now", 13),
            vec![0..5, 5..18],
            "a word that fits a row moves down whole, dot and all"
        );
        assert_eq!(
            wrap_line("    abcdef", 6),
            vec![0..6, 6..10],
            "indentation wider than the row breaks by char rather than leave a blank row"
        );
    }

    #[test]
    fn continuation_rows_line_up_under_the_text() {
        assert_eq!(indent("    foo bar", 20), 4);
        assert_eq!(indent("- item", 20), 2);
        assert_eq!(indent("  12. item", 20), 6);
        assert_eq!(indent("\t* item", 20), 6);
        for not_an_item in ["-- note", "-1 + x", "1.5 m"] {
            assert_eq!(
                indent(not_an_item, 20),
                0,
                "{not_an_item:?} is no list item"
            );
        }
        assert_eq!(
            indent("        deep", 15),
            0,
            "more than half the width would leave the rows too little room"
        );
        assert_eq!(
            wrap_line("  - aa bb cc", 8),
            vec![0..7, 7..10, 10..12],
            "rows after the first get less room: \"  - aa \", \"bb \", \"cc\""
        );
    }

    #[test]
    fn ascii_exact_width() {
        assert_eq!(wrap_line("abcdef", 3), vec![0..3, 3..6]);
        assert_eq!(wrap_line("abc", 3), vec![0..3]);
        assert_eq!(wrap_line("abcd", 3), vec![0..3, 3..4]);
    }

    #[test]
    fn cjk_at_boundary() {
        assert_eq!(
            wrap_line("漢字漢", 4),
            vec![0..6, 6..9],
            "each CJK char is 2 columns wide and 3 bytes long"
        );
        assert_eq!(
            wrap_line("a漢", 2),
            vec![0..1, 1..4],
            "a width-2 char that does not fit moves to the next row, leaving a ragged edge"
        );
        assert_eq!(wrap_line("a漢字", 3), vec![0..4, 4..7]);
    }

    #[test]
    fn combining_chars_attach() {
        let s = "e\u{301}e\u{301}e\u{301}";
        assert_eq!(
            width(s),
            3,
            "e + U+0301 combining acute: one column, two chars"
        );
        assert_eq!(wrap_line(s, 2), vec![0..6, 6..9]);
        assert_eq!(
            wrap_line("\u{301}ab", 1),
            vec![0..3, 3..4],
            "a leading combining char cannot open its own row"
        );
    }

    #[test]
    fn an_emoji_of_several_code_points_is_one_cell_pair() {
        for e in [
            "\u{26a0}\u{fe0f}",
            "\u{1f44d}\u{1f3fd}",
            "\u{1f468}\u{200d}\u{1f4bb}",
            "1\u{fe0f}\u{20e3}",
        ] {
            assert_eq!(width(e), 2, "{e:?} is not two columns, as drawn");
        }
        assert_eq!(
            wrap_line("\u{26a0}\u{fe0f}ab", 3),
            vec![0..7, 7..8],
            "\"⚠️\" is six bytes: it and `a` fill a row of three, `b` goes down"
        );
        assert_eq!(cut("\u{26a0}\u{fe0f}ab", 0, 2), (0..6, 0));
        assert_eq!(cut("\u{26a0}\u{fe0f}ab", 1, 4), (6..8, 1));
    }

    #[test]
    fn a_control_char_takes_no_column() {
        assert_eq!(width("a\u{7}b"), 2);
        assert_eq!(wrap_line("a\u{7}b", 1), vec![0..2, 2..3]);
    }

    #[test]
    fn tabs_are_four_wide() {
        assert_eq!(width("\ta"), 5);
        assert_eq!(wrap_line("\tab", 5), vec![0..2, 2..3]);
    }

    #[test]
    fn empty_line_is_one_row() {
        assert_eq!(wrap_line("", 10), vec![0..0]);
        assert_eq!(wrap_line("", 1), vec![0..0]);
    }

    #[test]
    fn width_one_splits_every_char() {
        assert_eq!(wrap_line("abc", 1), vec![0..1, 1..2, 2..3]);
        assert_eq!(
            wrap_line("漢漢", 1),
            vec![0..3, 3..6],
            "width 2 cannot fit in a 1-column pane: keep it rather than loop forever"
        );
        assert_eq!(wrap_line("abc", 0), wrap_line("abc", 1));
    }

    #[test]
    fn col_row_roundtrip() {
        for line in [
            "abcdefghij",
            "漢字a漢字b",
            "",
            "e\u{301}xyz",
            "ab cd  ef gh",
            "  - a.b/cd ef",
        ] {
            for w in 1..8 {
                let rows = wrap_line(line, w);
                for r in 0..rows.len() {
                    assert_eq!(col_to_row(&rows, row_to_col(&rows, r)), r, "{line:?} w={w}");
                }
                for (i, _) in line
                    .char_indices()
                    .chain(std::iter::once((line.len(), ' ')))
                {
                    let r = col_to_row(&rows, i);
                    assert!(
                        rows[r].start <= i && i <= rows[r].end,
                        "{line:?} w={w}: byte {i} maps outside the row that contains it"
                    );
                }
            }
        }
    }
}
