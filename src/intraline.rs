use std::ops::Range;

pub fn changes(old: &str, new: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let (a, b) = (tokens(old), tokens(new));
    let (pre, suf) = common(old, new, &a, &b);
    let (mid_a, mid_b) = (&a[pre..a.len() - suf], &b[pre..b.len() - suf]);
    let (kept_a, kept_b) = if mid_a.len() * mid_b.len() > 250_000 {
        (vec![false; mid_a.len()], vec![false; mid_b.len()])
    } else {
        in_lcs(&texts(old, mid_a), &texts(new, mid_b))
    };
    (changed(mid_a, &kept_a), changed(mid_b, &kept_b))
}

pub fn similarity(old: &str, new: &str) -> f64 {
    let (a, b) = (tokens(old), tokens(new));
    let (pre, suf) = common(old, new, &a, &b);
    let (mid_a, mid_b) = (&a[pre..a.len() - suf], &b[pre..b.len() - suf]);
    let mut kept: usize = a[..pre]
        .iter()
        .chain(&a[a.len() - suf..])
        .map(|t| non_space_chars(old, t))
        .sum();
    if mid_a.len() * mid_b.len() <= 250_000 {
        let (kept_a, _) = in_lcs(&texts(old, mid_a), &texts(new, mid_b));
        kept += mid_a
            .iter()
            .zip(&kept_a)
            .filter(|&(_, k)| *k)
            .map(|(t, _)| non_space_chars(old, t))
            .sum::<usize>();
    }
    let total = non_ws_len(old) + non_ws_len(new);
    if total == 0 {
        return 1.0;
    }
    2.0 * kept as f64 / total as f64
}

pub fn pair(deleted: &[String], added: &[String]) -> Vec<(usize, usize)> {
    if deleted.len() == added.len() {
        return (0..deleted.len()).map(|i| (i, i)).collect();
    }
    if deleted.len() * added.len() > 400 {
        return Vec::new();
    }
    let similar = |i: usize, j: usize| similarity(&deleted[i], &added[j]);
    let mut best = vec![vec![0.0_f64; added.len() + 1]; deleted.len() + 1];
    for i in 1..=deleted.len() {
        for j in 1..=added.len() {
            best[i][j] = best[i - 1][j].max(best[i][j - 1]);
            let s = similar(i - 1, j - 1);
            if s >= 0.5 {
                best[i][j] = best[i][j].max(best[i - 1][j - 1] + s);
            }
        }
    }
    let mut out = Vec::new();
    let (mut i, mut j) = (deleted.len(), added.len());
    while i > 0 && j > 0 {
        let s = similar(i - 1, j - 1);
        if s >= 0.5 && best[i][j] == best[i - 1][j - 1] + s {
            out.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if best[i][j] == best[i][j - 1] {
            j -= 1;
        } else {
            i -= 1;
        }
    }
    out.reverse();
    out
}

fn tokens(s: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut word: Option<usize> = None;
    for (i, c) in s.char_indices() {
        if c.is_alphanumeric() || c == '_' {
            word.get_or_insert(i);
        } else {
            if let Some(start) = word.take() {
                out.push(start..i);
            }
            out.push(i..i + c.len_utf8());
        }
    }
    if let Some(start) = word.take() {
        out.push(start..s.len());
    }
    out
}

fn common(old: &str, new: &str, a: &[Range<usize>], b: &[Range<usize>]) -> (usize, usize) {
    let mut pre = 0;
    while pre < a.len().min(b.len())
        && old[a[pre].start..a[pre].end] == new[b[pre].start..b[pre].end]
    {
        pre += 1;
    }
    let mut suf = 0;
    while suf < a.len() - pre
        && suf < b.len() - pre
        && old[a[a.len() - 1 - suf].start..a[a.len() - 1 - suf].end]
            == new[b[b.len() - 1 - suf].start..b[b.len() - 1 - suf].end]
    {
        suf += 1;
    }
    (pre, suf)
}

fn texts<'a>(s: &'a str, tokens: &[Range<usize>]) -> Vec<&'a str> {
    tokens.iter().map(|t| &s[t.start..t.end]).collect()
}

fn in_lcs(a: &[&str], b: &[&str]) -> (Vec<bool>, Vec<bool>) {
    let (m, n) = (a.len(), b.len());
    let mut len = vec![vec![0u32; n + 1]; m + 1];
    for i in (0..m).rev() {
        for j in (0..n).rev() {
            len[i][j] = if a[i] == b[j] {
                len[i + 1][j + 1] + 1
            } else {
                len[i + 1][j].max(len[i][j + 1])
            };
        }
    }
    let (mut kept_a, mut kept_b) = (vec![false; m], vec![false; n]);
    let (mut i, mut j) = (0, 0);
    while i < m && j < n {
        if a[i] == b[j] {
            kept_a[i] = true;
            kept_b[j] = true;
            i += 1;
            j += 1;
        } else if len[i + 1][j] >= len[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    (kept_a, kept_b)
}

fn changed(tokens: &[Range<usize>], kept: &[bool]) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = Vec::new();
    for (t, kept) in tokens.iter().zip(kept) {
        if *kept {
            continue;
        }
        match out.last_mut() {
            Some(last) if last.end == t.start => last.end = t.end,
            _ => out.push(t.clone()),
        }
    }
    out
}

fn non_space_chars(s: &str, t: &Range<usize>) -> usize {
    let text = &s[t.start..t.end];
    if text.starts_with(char::is_whitespace) {
        0
    } else {
        text.chars().count()
    }
}

fn non_ws_len(s: &str) -> usize {
    s.chars().filter(|c| !c.is_whitespace()).count()
}

#[cfg(test)]
// Single-element vecs of ranges are exactly what `changes` returns.
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::*;

    fn found_ranges(s: &str, needles: &[&str]) -> Vec<Range<usize>> {
        needles
            .iter()
            .map(|n| {
                let i = s.find(n).unwrap();
                i..i + n.len()
            })
            .collect()
    }

    #[test]
    fn one_token_becomes_another() {
        let (old, new) = changes("if a == b:", "if a >= b:");
        assert_eq!(old, [5..6]);
        assert_eq!(new, [5..6]);
    }

    #[test]
    fn a_word_in_the_middle_of_a_line() {
        let old = "return [n for n in notes if q in n.text.lower()]";
        let new = "return [n for n in notes if q in n.title.lower()]";
        assert_eq!(
            changes(old, new),
            (found_ranges(old, &["text"]), found_ranges(new, &["title"]))
        );
    }

    #[test]
    fn two_runs_of_change_merge_each_to_one_range() {
        let old = "elif isinstance(data, _SupportsRead):";
        let new = "elif _t.has_read(data):";
        assert_eq!(
            changes(old, new),
            (
                found_ranges(old, &["isinstance", ", _SupportsRead"]),
                found_ranges(new, &["_t.has_read"])
            )
        );
    }

    #[test]
    fn a_removed_tail_is_old_only() {
        let old = r#"__version__ = "2.34.0.dev1""#;
        let new = r#"__version__ = "2.34.0""#;
        assert_eq!(changes(old, new), (found_ranges(old, &[".dev1"]), vec![]));
    }

    #[test]
    fn added_indentation_is_a_change() {
        let old = format!("{}body = f(x)", " ".repeat(14));
        let new = format!("{}body = f(x)", " ".repeat(16));
        let at = new.find("body").unwrap();
        assert_eq!(changes(&old, &new), (vec![], vec![at - 2..at]));
    }

    #[test]
    fn trailing_whitespace_is_a_change() {
        assert_eq!(changes("x = 1   ", "x = 1"), (vec![5..8], vec![]));
    }

    #[test]
    fn a_rewrite_is_whole_runs() {
        let old = "    pass";
        let new = "    raise TypeError('max-age')";
        assert_eq!(
            changes(old, new),
            (found_ranges(old, &["pass"]), vec![4..new.len()])
        );
    }

    #[test]
    fn ranges_are_byte_ranges_on_char_boundaries() {
        let old = "привет мир";
        let new = "привет world";
        assert_eq!(
            changes(old, new),
            (found_ranges(old, &["мир"]), found_ranges(new, &["world"]))
        );
    }

    #[test]
    fn an_underscore_is_part_of_a_word() {
        assert_eq!(changes("a_b = 1", "a_c = 1"), (vec![0..3], vec![0..3]));
    }

    #[test]
    fn the_common_prefix_and_suffix_never_overlap() {
        assert_eq!(changes("x x", "x x x"), (vec![], vec![3..5]));
        assert_eq!(changes("x x x", "x x"), (vec![3..5], vec![]));
    }

    #[test]
    fn nothing_or_everything_changed() {
        assert_eq!(changes("same line", "same line"), (vec![], vec![]));
        assert_eq!(changes("", "foo"), (vec![], vec![0..3]));
    }

    #[test]
    fn similarity_is_the_share_of_kept_text() {
        assert_eq!(similarity("same line", "same line"), 1.0);
        assert_eq!(similarity("  ", "    "), 1.0, "no non-whitespace at all");
        assert_eq!(similarity("a", "b"), 0.0);
        let fs = r#"            ("fs".into(), p(&["fs", "default"])),"#;
        let node_fs = r#"            ("fs".into(), p(&["node:fs", "default"])),"#;
        let comment = "            // `node:` stays: the module is Node's own, whatever is \
                       installed under its name.";
        assert!(similarity(fs, node_fs) >= 0.5);
        assert!(similarity(fs, comment) < 0.5);
    }

    #[test]
    fn an_added_line_pairs_with_the_ghost_it_rewrites() {
        let fs = r#"            ("fs".into(), p(&["fs", "default"])),"#.to_string();
        let node_fs = r#"            ("fs".into(), p(&["node:fs", "default"])),"#.to_string();
        let comment = "            // `node:` stays: the module is Node's own, whatever is \
                       installed under its name."
            .to_string();
        assert_eq!(pair(&[fs], &[comment, node_fs]), vec![(0, 1)]);
        let deleted = &["total = price * qty".to_string()];
        let added = &[
            r#"log.debug("pricing %s", item)"#.to_string(),
            "total = price * qty * rate".to_string(),
        ];
        assert_eq!(pair(deleted, added), vec![(0, 1)]);
    }

    #[test]
    fn the_closest_deleted_line_pairs() {
        let deleted = &[
            "a = 1".to_string(),
            "total = price * qty".to_string(),
            "z = 3".to_string(),
        ];
        assert_eq!(
            pair(deleted, &["total = price * qty * rate".to_string()]),
            vec![(1, 0)]
        );
    }

    #[test]
    fn equal_counts_pair_in_order_whatever_the_lines() {
        let added = &["fn main() {".to_string(), "}".to_string()];
        assert_eq!(
            pair(&["apples".to_string(), "oranges".to_string()], added),
            vec![(0, 0), (1, 1)]
        );
        assert!(pair(&["let x = 1;".to_string()], added).is_empty());
    }

    fn words_between(first: &str, words: usize, last: &str) -> String {
        format!("{first}{} {last}", " w".repeat(words))
    }

    #[test]
    fn the_middles_are_matched_up_to_250_000_pairs_of_tokens() {
        let (old, new) = (words_between("A", 248, "C"), words_between("B", 248, "D"));
        assert_eq!(changes(&old, &new).0, [0..1, old.len() - 1..old.len()]);
        assert!(similarity(&old, &new) > 0.9);
        let (old, new) = (words_between("A", 249, "C"), words_between("B", 249, "D"));
        assert_eq!(changes(&old, &new).0, [0..old.len()]);
        assert_eq!(similarity(&old, &new), 0.0);
    }

    #[test]
    fn lines_are_paired_up_to_400_pairs_of_lines() {
        let lines = |n: usize| (0..n).map(|i| format!("line {i}")).collect::<Vec<_>>();
        assert_eq!(pair(&lines(16), &lines(25)).len(), 16);
        assert!(pair(&lines(17), &lines(25)).is_empty());
    }
}
