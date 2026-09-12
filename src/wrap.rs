//! Soft wrap: pure byte-range arithmetic, no rendering, no state.

use std::ops::Range;

use unicode_width::UnicodeWidthChar;

/// Display width of one char; zero for combining and other zero-width chars.
pub fn char_width(c: char) -> usize {
    c.width().unwrap_or(0)
}

/// Display width of a string, using the same rules as [`wrap_line`].
pub fn width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

/// Splits `line` into byte ranges that each fit in `width` display columns.
///
/// Greedy, char-level breaks. Zero-width chars attach to the preceding char; a width-2 char
/// that does not fit is pushed to the next row. An empty line yields one empty row, so the
/// result is never empty.
pub fn wrap_line(line: &str, width: usize) -> Vec<Range<usize>> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut start = 0usize;
    let mut used = 0usize;
    for (i, c) in line.char_indices() {
        let w = c.width().unwrap_or(0);
        if w == 0 {
            continue; // attaches to the previous char, never opens a row
        }
        if used + w > width && i > start {
            rows.push(start..i);
            start = i;
            used = 0;
        }
        used += w;
    }
    rows.push(start..line.len());
    rows
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
        for line in ["abcdefghij", "漢字a漢字b", "", "e\u{301}xyz"] {
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
