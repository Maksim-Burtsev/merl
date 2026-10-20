use std::ops::Range;
use std::sync::LazyLock;

use pulldown_cmark::{Event, Tag, TagEnd};
use regex::Regex;

use super::{Frame, Kind, Lay};

pub fn holds_raw_text(first_line: &str) -> bool {
    let start = first_line.trim_start().to_ascii_lowercase();
    ["<script", "<pre", "<style", "<textarea"]
        .iter()
        .any(|tag| {
            start
                .strip_prefix(tag)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(['>', ' ', '\t']))
        })
}

pub fn without_comments(line: &str, in_comment: &mut bool) -> (String, Vec<(usize, usize)>) {
    let mut out = String::new();
    let mut cuts = Vec::new();
    let mut rest = line;
    loop {
        if *in_comment {
            let Some((inside, after)) = rest.split_once("-->") else {
                cuts.push((out.len(), rest.len()));
                return (out, cuts);
            };
            cuts.push((out.len(), inside.len() + "-->".len()));
            rest = after;
            *in_comment = false;
        }
        let Some((before, after)) = rest.split_once("<!--") else {
            out.push_str(rest);
            return (out, cuts);
        };
        out.push_str(before);
        cuts.push((out.len(), "<!--".len()));
        rest = after;
        *in_comment = true;
    }
}

pub fn removed_before(cuts: &[(usize, usize)], kept: usize) -> usize {
    cuts.iter()
        .filter(|&&(at, _)| at <= kept)
        .map(|&(_, n)| n)
        .sum()
}

pub fn bare_urls(src: &str, events: &[(Event, Range<usize>)]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut run: Option<Range<usize>> = None;
    let mut inside = 0_usize;
    for (ev, r) in events {
        match ev {
            Event::Text(_) if inside == 0 => {
                match &mut run {
                    Some(run) if run.end == r.start => run.end = r.end,
                    _ => {
                        urls_in(src, run.replace(r.clone()), &mut out);
                    }
                }
                continue;
            }
            Event::Start(Tag::Link { .. } | Tag::Image { .. } | Tag::CodeBlock(_)) => inside += 1,
            Event::End(TagEnd::Link | TagEnd::Image | TagEnd::CodeBlock) => {
                inside = inside.saturating_sub(1);
            }
            _ => {}
        }
        urls_in(src, run.take(), &mut out);
    }
    urls_in(src, run, &mut out);
    out
}

fn urls_in(src: &str, run: Option<Range<usize>>, out: &mut Vec<Range<usize>>) {
    static URL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[\s*_~(])((?:https?://|www\.)[^\s<]+)").expect("the regex is a literal")
    });
    let Some(run) = run else {
        return;
    };
    let Some(text) = src.get(run.clone()) else {
        return;
    };
    for m in URL.captures_iter(text).filter_map(|c| c.get(1)) {
        let url = trimmed(m.as_str());
        let head = match url.find("://") {
            Some(i) => i + 3,
            None => "www.".len(),
        };
        if url.len() > head {
            let start = run.start + m.start();
            out.push(start..start + url.len());
        }
    }
}

fn trimmed(mut url: &str) -> &str {
    loop {
        let cut = url.trim_end_matches(['?', '!', '.', ',', ':', ';', '*', '_', '~', '\'', '"']);
        let cut = match cut.strip_suffix(')') {
            Some(less) if cut.matches(')').count() > cut.matches('(').count() => less,
            _ => cut,
        };
        if cut.len() == url.len() {
            return url;
        }
        url = cut;
    }
}

impl Lay<'_> {
    pub(super) fn text_with_urls(&mut self, t: &str, r: Range<usize>) {
        let src = self.src;
        let literal = self.link == 0 && self.image == 0 && src.get(r.clone()) == Some(t);
        let first = self.urls.partition_point(|u| u.end <= r.start);
        let urls: Vec<Range<usize>> = self.urls[first..]
            .iter()
            .take_while(|u| literal && u.start < r.end)
            .map(|u| u.start.max(r.start)..u.end.min(r.end))
            .collect();
        if urls.is_empty() {
            let in_url = self.link == 0
                && self.image == 0
                && self
                    .urls
                    .get(first)
                    .is_some_and(|u| u.start <= r.start && r.end <= u.end);
            self.link += usize::from(in_url);
            self.text(t, self.look(), r);
            self.link -= usize::from(in_url);
            return;
        }
        let mut at = r.start;
        for u in urls {
            let end = u.end;
            self.piece(src, at..u.start, 0);
            self.piece(src, u, 1);
            at = end;
        }
        self.piece(src, at..r.end, 0);
    }

    fn piece(&mut self, src: &str, r: Range<usize>, link: usize) {
        let Some(s) = src.get(r.clone()).filter(|s| !s.is_empty()) else {
            return;
        };
        self.link += link;
        self.text(s, self.look(), r);
        self.link -= link;
    }

    pub(super) fn end_html_block(&mut self, start: usize) {
        let lines = self.html.take().unwrap_or_default();
        let gap_row = self.rows.len();
        self.html_block(lines);
        let marker_waits = matches!(
            self.stack.last(),
            Some(Frame::Item {
                first_row_marker: Some(_),
                ..
            })
        );
        if self.rows.len() == gap_row && marker_waits {
            let at = self.line_and_byte_col(start);
            self.push("", Vec::new(), at, Kind::Text);
            self.gap_before_next_block = true;
        } else if self.rows.len() == gap_row {
            self.rows.truncate(self.html_from.0);
            self.gap_before_next_block = self.html_from.1;
        } else {
            self.gap_before_next_block = true;
        }
    }

    fn html_block(&mut self, lines: Vec<(String, (usize, usize))>) {
        let raw_text = lines.first().is_some_and(|(l, _)| holds_raw_text(l));
        let mut in_comment = false;
        let mut cut_lines = Vec::new();
        let lines: Vec<_> = lines
            .into_iter()
            .filter_map(|(line, at)| {
                if raw_text {
                    return Some((line, at));
                }
                let was_in_comment = in_comment;
                let (kept, cuts) = without_comments(&line, &mut in_comment);
                let shown = !kept.trim().is_empty() || !was_in_comment && kept == line;
                if shown && !cuts.is_empty() {
                    cut_lines.push((at, cuts));
                }
                shown.then_some((kept, at))
            })
            .collect();
        let first = self.rows.len();
        self.html_lines(lines);
        for row in &mut self.rows[first..] {
            if let Some(((_, c), cuts)) = cut_lines.iter().find(|((l, _), _)| *l == row.src.0) {
                let kept = row.src.1.saturating_sub(*c);
                row.src.1 += removed_before(cuts, kept);
            }
        }
    }
}
