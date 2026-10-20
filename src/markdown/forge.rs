use std::ops::Range;
use std::sync::LazyLock;

use pulldown_cmark::{Event, Tag, TagEnd};
use regex::Regex;

pub fn without_comments(line: &str, in_comment: &mut bool) -> String {
    let mut out = String::new();
    let mut rest = line;
    loop {
        if *in_comment {
            let Some((_, after)) = rest.split_once("-->") else {
                return out;
            };
            rest = after;
            *in_comment = false;
        }
        let Some((before, after)) = rest.split_once("<!--") else {
            out.push_str(rest);
            return out;
        };
        out.push_str(before);
        rest = after;
        *in_comment = true;
    }
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
