//! Markdown's links (#421).

use super::*;

/// What `d` reads at the first byte of `on` in `text`.
fn at(text: &str, on: &str) -> MdAt {
    markdown_at(text, text.find(on).unwrap())
}

#[test]
fn a_link_is_read_whole_from_its_text_its_target_or_its_definition() {
    let link = |s: &str| MdAt::Link(s.into());
    let text =
        "A [spaced](<my notes.md>) and a [titled](a.md \"T\") and [ref][R].\n\n[r]: b.md#x\n";
    assert_eq!(at(text, "spaced"), link("my notes.md"));
    assert_eq!(at(text, "notes.md>"), link("my notes.md"));
    assert_eq!(at(text, "\"T\""), link("a.md"));
    assert_eq!(at(text, "ref]"), link("b.md#x"));
    assert_eq!(at(text, "r]:"), link("b.md#x"));
    assert_eq!(at(text, "and a"), MdAt::Nothing);
    let text = "Mail <me@example.com>, go to <https://x.org>, or <a href='c.md'>here</a>.\n";
    assert_eq!(at(text, "me@"), link("mailto:me@example.com"));
    assert_eq!(at(text, "x.org"), link("https://x.org"));
    assert_eq!(at(text, "here"), link("c.md"));
}

#[test]
fn an_image_code_a_comment_and_the_front_matter_are_no_links() {
    let text = "---\nsee: [a](a.md)\n---\n\n[![badge](b.png)](c.md) `x.rs` [`y.rs`](y.rs)\n\n~~~\n[a](a.md)\n~~~\n\n<!--\n[a](a.md)\n-->\n";
    assert_eq!(at(text, "[a]"), MdAt::Nothing);
    assert_eq!(at(text, "badge"), MdAt::Nothing);
    assert_eq!(at(text, "(c.md"), MdAt::Link("c.md".into()));
    assert_eq!(at(text, "x.rs"), MdAt::Code("x.rs".into()));
    assert_eq!(at(text, "y.rs`"), MdAt::Link("y.rs".into()));
    let fenced = text.find("~~~\n[a]").unwrap() + 5;
    let commented = text.find("<!--\n[a]").unwrap() + 6;
    assert_eq!(markdown_at(text, fenced), MdAt::Nothing);
    assert_eq!(markdown_at(text, commented), MdAt::Nothing);
    assert_eq!(
        literal_lines(Kind::Markdown, text),
        [
            true, true, true, false, false, false, true, true, true, false, true, true, true, false
        ]
    );
}

#[test]
fn headings_get_githubs_anchors() {
    assert_eq!(
        github_anchor("What `d` recognises, language by language"),
        "what-d-recognises-language-by-language"
    );
    assert_eq!(github_anchor("[0.8.0] - 2026-09-27"), "080---2026-09-27");
    assert_eq!(github_anchor("Émoji 🚀 _x_"), "émoji--_x_");
    let text = "# Setup\n\nSetup\n=====\n\n## Setup-1\n\n## Setup\n\n    # not a heading\n";
    // `Setup-1` finds its anchor taken by the second `Setup` and takes the next free one, as
    // github-slugger does.
    assert_eq!(anchor_line(text, "setup"), Some(1));
    assert_eq!(anchor_line(text, "setup-1"), Some(3));
    assert_eq!(anchor_line(text, "setup-1-1"), Some(6));
    assert_eq!(anchor_line(text, "SETUP-2"), Some(8));
    assert_eq!(anchor_line(text, "not-a-heading"), None);
    assert_eq!(percent_decoded("my%20notes%2Emd%zz%"), "my notes.md%zz%");
}
