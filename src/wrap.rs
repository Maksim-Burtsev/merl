//! Soft wrap: pure byte-range arithmetic, no rendering, no state.

use std::ops::Range;

use unicode_width::UnicodeWidthChar;

/// Display width of one char; zero for combining and other zero-width chars. A tab is drawn as
/// [`crate::buffer::TAB`], so it is that wide.
pub fn char_width(c: char) -> usize {
    if c == '\t' {
        return crate::buffer::TAB.len();
    }
    c.width().unwrap_or(0)
}

/// Display width of a string, using the same rules as [`wrap_line`].
pub fn width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

/// Splits `line` into byte ranges that each fit in `width` display columns. Rows after the first
/// are drawn [`indent`] columns in, so they get that much less room.
///
/// Greedy, breaking between words: a row ends after the spaces that follow a word, and those
/// spaces may run past the edge rather than open the next row with a blank. A word that fits a
/// row moves down whole. A longer one (a URL, a call chain, a hash) starts where it stands, like
/// Claude Code, and breaks after its last `/ . , ; ) ] }` that fits, like VS Code, or else by
/// char; so does indentation wider than the row. Zero-width chars attach to the preceding char; a
/// width-2 char that does not fit is pushed to the next row. An empty line yields one empty row,
/// so the result is never empty.
pub fn wrap_line(line: &str, width: usize) -> Vec<Range<usize>> {
    let width = width.max(1);
    let rest = width - indent(line, width);
    let mut rows = Vec::new();
    let mut start = 0usize;
    let mut used = 0usize;
    // The row has a non-space char, so its spaces separate words rather than indent.
    let mut has_text = false;
    let mut after_space = false;
    // Start of the row's last word after a space: where the row ends instead of mid-word.
    let mut word = None;
    // The row's last break after punctuation: where a word longer than a row ends it.
    let mut punct = None;
    let mut prev = ' ';
    for (i, c) in line.char_indices() {
        let w = char_width(c);
        if w == 0 {
            continue; // attaches to the previous char, never opens a row
        }
        // Not `is_whitespace`: a no-break space must keep its words on one row.
        let space = c == ' ' || c == '\t';
        if !(space && has_text) {
            if after_space && has_text {
                word = Some(i);
            }
            if breaks_after(prev) && !breaks_after(c) {
                punct = Some(i);
            }
            let room = if rows.is_empty() { width } else { rest };
            if used + w > room && i > start {
                let end = match word {
                    Some(b) if self::width(&line[b..word_end(line, b)]) <= rest => b,
                    Some(b) => punct.filter(|&p| p > b).unwrap_or(i),
                    None => punct.unwrap_or(i),
                };
                rows.push(start..end);
                start = end;
                used = self::width(&line[end..i]);
                (word, punct) = (None, None);
            }
        }
        used += w;
        has_text |= !space;
        after_space = space;
        prev = c;
    }
    rows.push(start..line.len());
    rows
}

/// Punctuation a word longer than a row may break after.
fn breaks_after(c: char) -> bool {
    matches!(c, '/' | '.' | ',' | ';' | ')' | ']' | '}')
}

/// Byte offset where the word starting at `i` ends: the next space or tab, or the end of `line`.
fn word_end(line: &str, i: usize) -> usize {
    line[i..].find([' ', '\t']).map_or(line.len(), |n| i + n)
}

/// Display columns a continuation row of `line` is drawn in by: its indentation and, for a list
/// item (`- `, `* `, `+ `, `1. `, `1) `), the marker as well, so the rows line up under the text.
/// Zero when that is more than half of `width`, which would leave the rows too little room.
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

/// What shows of `line` in display columns `from..to` when it is not wrapped: the byte range of
/// the chars that fit whole, and the blank columns before them where a tab or a wide char
/// straddles `from`. Empty at the end of the line when it does not reach `from`.
pub fn cut(line: &str, from: usize, to: usize) -> (Range<usize>, usize) {
    let (mut start, mut lead) = (None, 0);
    let mut x = 0;
    for (i, c) in line.char_indices() {
        let w = char_width(c);
        if start.is_none() && x >= from && w > 0 {
            start = Some(i);
            lead = (x - from).min(to.saturating_sub(from));
        }
        if start.is_some() && x + w > to {
            return (start.unwrap_or(i)..i, lead);
        }
        x += w;
    }
    (start.unwrap_or(line.len())..line.len(), lead)
}

/// Index of the row containing byte offset `col` (the last row for `col == line.len()`).
pub fn col_to_row(rows: &[Range<usize>], col: usize) -> usize {
    rows.iter().rposition(|r| r.start <= col).unwrap_or(0)
}

/// Byte offset where `row` starts. Out-of-range rows clamp to the last row.
pub fn row_to_col(rows: &[Range<usize>], row: usize) -> usize {
    rows[row.min(rows.len() - 1)].start
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cut_keeps_the_chars_that_fit_whole() {
        assert_eq!(cut("abcdefgh", 0, 4), (0..4, 0));
        assert_eq!(cut("abcdefgh", 2, 5), (2..5, 0));
        assert_eq!(cut("abcdefgh", 6, 20), (6..8, 0));
        // A line that ends left of the window shows nothing.
        assert_eq!(cut("abc", 5, 9), (3..3, 0));
        assert_eq!(cut("", 0, 9), (0..0, 0));
        // Cyrillic is two bytes a char.
        assert_eq!(cut("сбоев", 1, 3), (2..6, 0));
        // A tab (four columns) straddling the left edge leaves its visible part blank; a wide
        // char that does not fit at the right edge is left out.
        assert_eq!(cut("\tab", 2, 8), (1..3, 2));
        assert_eq!(cut("a\u{4e2d}b", 0, 2), (0..1, 0));
    }

    #[test]
    fn breaks_between_words() {
        // The space stays at the end of the upper row, so the lower one starts with a word.
        assert_eq!(wrap_line("foo bar baz", 9), vec![0..8, 8..11]);
        // Cyrillic is two bytes a char.
        assert_eq!(wrap_line("сбоев лимит", 8), vec![0..11, 11..21]);
        // Spaces that do not fit hang past the edge instead of opening a row.
        assert_eq!(wrap_line("foo  bar", 3), vec![0..5, 5..8]);
        // A no-break space is part of the word.
        assert_eq!(wrap_line("a\u{a0}bc d", 3), vec![0..4, 4..7]);
    }

    #[test]
    fn a_word_longer_than_a_row_starts_in_place() {
        // It breaks after punctuation inside it when it can, by char when it cannot.
        assert_eq!(wrap_line("see a/b/c/d", 6), vec![0..6, 6..11]);
        assert_eq!(wrap_line("sum: 0123456789", 8), vec![0..8, 8..15]);
        assert_eq!(wrap_line("a bcdefgh", 4), vec![0..4, 4..8, 8..9]);
        // A word that fits a row moves down whole, dot and all.
        assert_eq!(wrap_line("read CLAUDE.md now", 13), vec![0..5, 5..18]);
        // Indentation wider than the row breaks by char rather than leave a blank row.
        assert_eq!(wrap_line("    abcdef", 6), vec![0..6, 6..10]);
    }

    #[test]
    fn continuation_rows_line_up_under_the_text() {
        assert_eq!(indent("    foo bar", 20), 4);
        assert_eq!(indent("- item", 20), 2);
        assert_eq!(indent("  12. item", 20), 6);
        assert_eq!(indent("\t* item", 20), 6);
        // Not list items: an SQL comment, a negative number, a decimal.
        assert_eq!(indent("-- note", 20), 0);
        assert_eq!(indent("-1 + x", 20), 0);
        assert_eq!(indent("1.5 m", 20), 0);
        // More than half the width would leave the rows too little room.
        assert_eq!(indent("        deep", 15), 0);
        // So rows after the first get less room: "  - aa ", "bb ", "cc".
        assert_eq!(wrap_line("  - aa bb cc", 8), vec![0..7, 7..10, 10..12]);
    }

    #[test]
    fn ascii_exact_width() {
        assert_eq!(wrap_line("abcdef", 3), vec![0..3, 3..6]);
        assert_eq!(wrap_line("abc", 3), vec![0..3]);
        assert_eq!(wrap_line("abcd", 3), vec![0..3, 3..4]);
    }

    #[test]
    fn cjk_at_boundary() {
        // Each CJK char is 2 columns wide and 3 bytes long.
        assert_eq!(wrap_line("漢字漢", 4), vec![0..6, 6..9]);
        // A width-2 char that does not fit moves to the next row, leaving a ragged edge.
        assert_eq!(wrap_line("a漢", 2), vec![0..1, 1..4]);
        assert_eq!(wrap_line("a漢字", 3), vec![0..4, 4..7]);
    }

    #[test]
    fn combining_chars_attach() {
        // "e" + U+0301 combining acute: one column, two chars.
        let s = "e\u{301}e\u{301}e\u{301}";
        assert_eq!(width(s), 3);
        assert_eq!(wrap_line(s, 2), vec![0..6, 6..9]);
        // A leading combining char cannot open its own row.
        assert_eq!(wrap_line("\u{301}ab", 1), vec![0..3, 3..4]);
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
        // Width 2 cannot fit in a 1-column pane: keep it rather than loop forever.
        assert_eq!(wrap_line("漢漢", 1), vec![0..3, 3..6]);
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
                // Every byte boundary maps into the row that contains it.
                for (i, _) in line
                    .char_indices()
                    .chain(std::iter::once((line.len(), ' ')))
                {
                    let r = col_to_row(&rows, i);
                    assert!(
                        rows[r].start <= i && i <= rows[r].end,
                        "{line:?} w={w} i={i}"
                    );
                }
            }
        }
    }
}
