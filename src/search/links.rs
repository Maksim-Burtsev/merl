use std::collections::HashMap;
use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};
use regex::Regex;

#[derive(Debug, PartialEq, Eq)]
pub enum MdAt {
    Link(String),
    Code(String),
    Nothing,
}

fn options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
}

fn fenced_code_html_comment_or_front_matter(text: &str, ev: &Event, r: &Range<usize>) -> bool {
    match ev {
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_)) | Tag::MetadataBlock(_)) => true,
        Event::Start(Tag::HtmlBlock) | Event::InlineHtml(_) | Event::Html(_) => text
            .get(r.clone())
            .is_some_and(|t| t.trim_start().starts_with("<!--")),
        _ => false,
    }
}

fn in_html_comment(text: &str, r: &Range<usize>, at: usize) -> bool {
    let Some(t) = text.get(r.clone()) else {
        return false;
    };
    let mut from = 0;
    while let Some(open) = t.get(from..).and_then(|s| s.find("<!--")) {
        let start = from + open;
        let end = t
            .get(start + 4..)
            .and_then(|s| s.find("-->"))
            .map_or(t.len(), |e| start + 4 + e + 3);
        if (r.start + start..r.start + end).contains(&at) {
            return true;
        }
        from = end;
    }
    false
}

static HREF: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"(?is)<a\s[^>]*?\bhref\s*=\s*(?:"([^"]*)"|'([^']*)')[^>]*>.*?</a>"#)
        .expect("the regex is a literal")
});

pub fn first_link_in(text: &str, span: &Range<usize>) -> Option<usize> {
    let meets = |r: &Range<usize>| r.start < span.end && span.start < r.end;
    let links = Parser::new_ext(text, options())
        .into_offset_iter()
        .filter(|(ev, r)| matches!(ev, Event::Start(Tag::Link { .. })) && meets(r))
        .map(|(_, r)| r);
    let hrefs = HREF.find_iter(text).map(|m| m.range()).filter(meets);
    let mut starts: Vec<usize> = links
        .chain(hrefs)
        .map(|r| r.start.max(span.start))
        .collect();
    starts.sort_unstable();
    starts
        .into_iter()
        .find(|&at| matches!(markdown_at(text, at), MdAt::Link(_)))
}

pub fn markdown_at(text: &str, at: usize) -> MdAt {
    let parser = Parser::new_ext(text, options());
    #[expect(
        clippy::needless_collect,
        reason = "the definitions are read before `into_offset_iter` consumes the parser"
    )]
    let defs: Vec<(Range<usize>, String)> = parser
        .reference_definitions()
        .iter()
        .map(|(_, d)| (d.span.clone(), d.dest.to_string()))
        .collect();
    let mut found = MdAt::Nothing;
    for (ev, r) in parser.into_offset_iter() {
        if !r.contains(&at) {
            continue;
        }
        let html = matches!(
            ev,
            Event::Start(Tag::HtmlBlock) | Event::InlineHtml(_) | Event::Html(_)
        );
        if html && in_html_comment(text, &r, at)
            || !html && fenced_code_html_comment_or_front_matter(text, &ev, &r)
        {
            return MdAt::Nothing;
        }
        match ev {
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                ..
            }) => {
                found = MdAt::Link(match link_type {
                    LinkType::Email => format!("mailto:{dest_url}"),
                    _ => dest_url.to_string(),
                });
            }
            Event::Start(Tag::Image { .. }) => found = MdAt::Nothing,
            Event::Code(code) if !matches!(found, MdAt::Link(_)) => {
                found = MdAt::Code(code.to_string());
            }
            _ => {}
        }
    }
    if found != MdAt::Nothing {
        return found;
    }
    if let Some((_, dest)) = defs.into_iter().find(|(r, _)| r.contains(&at)) {
        return MdAt::Link(dest);
    }
    HREF.captures_iter(text)
        .find(|c| c.get(0).is_some_and(|m| m.range().contains(&at)))
        .and_then(|c| c.get(1).or_else(|| c.get(2)))
        .map_or(MdAt::Nothing, |m| MdAt::Link(m.as_str().to_owned()))
}

pub(super) fn markdown_literal_lines(text: &str) -> Vec<bool> {
    let mut out = vec![false; text.split('\n').count()];
    #[expect(
        clippy::string_slice,
        reason = "pulldown-cmark's offsets fall on character boundaries"
    )]
    let line = |at: usize| text[..at].matches('\n').count();
    for (ev, r) in Parser::new_ext(text, options()).into_offset_iter() {
        if !r.is_empty() && fenced_code_html_comment_or_front_matter(text, &ev, &r) {
            out[line(r.start)..=line(r.end - 1)].fill(true);
        }
    }
    out
}

pub fn github_anchor(heading: &str) -> String {
    heading
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

pub fn anchor_line1(text: &str, anchor: &str) -> Option<usize> {
    static ID: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"(?i)<a\s[^>]*?\b(?:id|name)\s*=\s*["']([^"']+)["']"#)
            .expect("the regex is a literal")
    });
    let anchor = anchor.to_lowercase();
    #[expect(
        clippy::string_slice,
        reason = "pulldown-cmark's offsets fall on character boundaries"
    )]
    let line = |at: usize| text[..at].matches('\n').count() + 1;
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut heading: Option<(usize, String)> = None;
    for (ev, r) in Parser::new_ext(text, options()).into_offset_iter() {
        match ev {
            Event::Start(Tag::Heading { .. }) => heading = Some((r.start, String::new())),
            Event::Text(t) | Event::Code(t) => {
                if let Some((_, h)) = &mut heading {
                    h.push_str(&t);
                }
            }
            Event::SoftBreak => {
                if let Some((_, h)) = &mut heading {
                    h.push(' ');
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                let Some((start, h)) = heading.take() else {
                    continue;
                };
                let base = github_anchor(&h);
                let mut slug = base.clone();
                while seen.contains_key(&slug) {
                    let n = seen.entry(base.clone()).or_default();
                    *n += 1;
                    slug = format!("{base}-{n}");
                }
                seen.insert(slug.clone(), 0);
                if slug == anchor {
                    return Some(line(start));
                }
            }
            _ => {}
        }
    }
    ID.captures_iter(text)
        .find(|c| c[1].to_lowercase() == anchor)
        .map(|c| line(c.get_match().start()))
}

pub fn percent_decoded(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = b
            .get(i + 1..i + 3)
            .filter(|h| h.iter().all(u8::is_ascii_hexdigit))
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (b[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (c, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
