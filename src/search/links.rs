use std::collections::HashMap;
use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};
use regex::Regex;

/// What a byte of a Markdown file stands on, for `d`.
#[derive(Debug, PartialEq, Eq)]
pub enum MdAt {
    /// A link, a reference link or its `[label]: target` definition, an `<a href>`: the
    /// target as written.
    Link(String),
    /// A code span's text, which may name a file of the project.
    Code(String),
    /// Nothing to follow: prose, an image, or code, a comment or the front matter.
    Nothing,
}

fn options() -> Options {
    // Footnotes, so that `[^1]` is no shortcut link; the front matter, so that it is no text.
    Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
}

fn fenced_code_html_comment_or_front_matter(text: &str, ev: &Event, r: &Range<usize>) -> bool {
    match ev {
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_)) | Tag::MetadataBlock(_)) => true,
        Event::Start(Tag::HtmlBlock) | Event::InlineHtml(_) | Event::Html(_) => {
            text[r.clone()].trim_start().starts_with("<!--")
        }
        _ => false,
    }
}

pub fn markdown_at(text: &str, at: usize) -> MdAt {
    static HREF: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"(?is)<a\s[^>]*?\bhref\s*=\s*(?:"([^"]*)"|'([^']*)')[^>]*>.*?</a>"#).unwrap()
    });
    let parser = Parser::new_ext(text, options());
    let defs: Vec<(Range<usize>, String)> = parser
        .reference_definitions()
        .iter()
        .map(|(_, d)| (d.span.clone(), d.dest.to_string()))
        .collect();
    let mut found = MdAt::Nothing;
    // A start event's range is the whole element's, so the last one holding `at` is the
    // innermost: an image inside a link is the image.
    for (ev, r) in parser.into_offset_iter() {
        if !r.contains(&at) {
            continue;
        }
        if fenced_code_html_comment_or_front_matter(text, &ev, &r) {
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
                })
            }
            Event::Start(Tag::Image { .. }) => found = MdAt::Nothing,
            // A code span inside a link's text is the link's.
            Event::Code(code) if !matches!(found, MdAt::Link(_)) => {
                found = MdAt::Code(code.to_string())
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
        .and_then(|c| c.get(1).or(c.get(2)))
        .map_or(MdAt::Nothing, |m| MdAt::Link(m.as_str().to_owned()))
}

pub(super) fn markdown_literal_lines(text: &str) -> Vec<bool> {
    let mut out = vec![false; text.split('\n').count()];
    let line = |at: usize| text[..at].matches('\n').count();
    for (ev, r) in Parser::new_ext(text, options()).into_offset_iter() {
        if !r.is_empty() && fenced_code_html_comment_or_front_matter(text, &ev, &r) {
            out[line(r.start)..=line(r.end - 1)].fill(true);
        }
    }
    out
}

/// GitHub's anchor for a heading's text: lower-cased, every character but a letter, a digit, a
/// space, `-` and `_` dropped, each space made a `-`.
pub fn github_anchor(heading: &str) -> String {
    heading
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

/// The line of the Markdown `text` that `anchor` names: a heading, by its GitHub anchor
/// (the second heading of an anchor gets `-1`, the third `-2`), or an `<a id>` or `<a name>`.
/// Case is ignored, as GitHub ignores it.
pub fn anchor_line1(text: &str, anchor: &str) -> Option<usize> {
    static ID: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"(?i)<a\s[^>]*?\b(?:id|name)\s*=\s*["']([^"']+)["']"#).unwrap()
    });
    let anchor = anchor.to_lowercase();
    let line = |at: usize| text[..at].matches('\n').count() + 1;
    // github-slugger's count: a repeat takes the next free `-N`.
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
        .map(|c| line(c.get(0).unwrap().start()))
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
