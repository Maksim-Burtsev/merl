use std::ops::Range;

use crate::picture::Width;

#[derive(Debug, Clone, PartialEq)]
pub struct Img {
    pub tag: Range<usize>,
    pub dest: String,
    pub width: Option<Width>,
    pub shown: bool,
}

pub fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let lower = tag.to_ascii_lowercase();
    for (at, _) in lower.match_indices(name) {
        let (Some(before), Some(after)) = (lower.get(..at), tag.get(at + name.len()..)) else {
            continue;
        };
        if !before.ends_with(char::is_whitespace) {
            continue;
        }
        let Some(rest) = after.trim_start().strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim_start();
        return match rest.chars().next()? {
            q @ ('"' | '\'') => rest.strip_prefix(q)?.split_once(q).map(|(v, _)| v),
            _ => rest.split([' ', '\t', '\n', '>']).next(),
        };
    }
    None
}

pub fn variant_shown(dest: &str, light: bool) -> bool {
    match dest.rsplit_once('#').map(|(_, f)| f) {
        Some("gh-dark-mode-only") => !light,
        Some("gh-light-mode-only") => light,
        _ => true,
    }
}

fn width(tag: &str) -> Option<Width> {
    let w = attr(tag, "width")?.trim();
    match w.strip_suffix('%') {
        Some(p) => p.trim().parse().ok().map(Width::Percent),
        None => w.trim_end_matches("px").parse().ok().map(Width::Px),
    }
}

pub fn html_images(html: &str, light: bool) -> Vec<Img> {
    let mut out = Vec::new();
    let mut picture: Option<Option<String>> = None;
    for (at, _) in html.match_indices('<') {
        let Some(tag) = html.get(at..).and_then(|t| t.split_inclusive('>').next()) else {
            continue;
        };
        let end = at + tag.len();
        let name: String = tag
            .chars()
            .skip(1)
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '/')
            .collect::<String>()
            .to_ascii_lowercase();
        match name.as_str() {
            "picture" => picture = Some(None),
            "/picture" => picture = None,
            "source" => {
                if let Some(chosen @ None) = &mut picture {
                    let media = attr(tag, "media").unwrap_or_default().to_ascii_lowercase();
                    let src = attr(tag, "srcset")
                        .and_then(|s| s.split(',').next())
                        .and_then(|s| s.split_whitespace().next());
                    if media.contains("prefers-color-scheme") && media.contains("dark") != light {
                        *chosen = src.map(String::from);
                    }
                }
            }
            "img" => {
                let Some(src) = attr(tag, "src") else {
                    continue;
                };
                let dest = picture.clone().flatten().unwrap_or_else(|| src.to_owned());
                out.push(Img {
                    tag: at..end,
                    shown: variant_shown(&dest, light),
                    dest,
                    width: width(tag),
                });
            }
            _ => {}
        }
    }
    out
}

pub fn only_tags(line: &str) -> bool {
    let mut rest = line.trim();
    while let Some(r) = rest.strip_prefix('<') {
        match r.split_once('>') {
            Some((_, after)) => rest = after.trim_start(),
            None => return false,
        }
    }
    rest.is_empty()
}

pub fn centred(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    [
        "align=\"center\"",
        "align='center'",
        "align=center",
        "<center",
    ]
    .iter()
    .any(|a| lower.contains(a))
}
