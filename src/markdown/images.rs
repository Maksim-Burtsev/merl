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
    let mut from = 0;
    while let Some(i) = lower[from..].find(name) {
        let at = from + i;
        from = at + name.len();
        if !lower[..at].ends_with(char::is_whitespace) {
            continue;
        }
        let Some(rest) = tag[from..].trim_start().strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim_start();
        return match rest.chars().next()? {
            q @ ('"' | '\'') => rest[1..].find(q).map(|e| &rest[1..1 + e]),
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
    let mut from = 0;
    while let Some(i) = html[from..].find('<') {
        let at = from + i;
        let end = html[at..].find('>').map_or(html.len(), |e| at + e + 1);
        from = at + 1;
        let tag = &html[at..end];
        let name: String = tag[1..]
            .chars()
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
                let dest = picture.clone().flatten().unwrap_or_else(|| src.to_string());
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
        match r.find('>') {
            Some(e) => rest = r[e + 1..].trim_start(),
            None => return false,
        }
    }
    rest.is_empty()
}

pub fn centred(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    ["align=\"center\"", "align='center'", "align=center", "<center"]
        .iter()
        .any(|a| lower.contains(a))
}
