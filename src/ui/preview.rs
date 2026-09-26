//! The code pane showing a Markdown file rendered (#249): the rows `markdown` laid out, the
//! cursor row, and the source's git marks beside the rows they came from.

use std::collections::HashMap;
use std::ops::Range;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::buffer::Buffer;
use crate::git::Mark;
use crate::markdown::{Kind, Palette, Row};
use crate::theme::Theme;
use crate::wrap;

use super::code::{digits, mark_span, review_tint};

pub(super) fn draw_preview(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    base: Style,
) {
    // The text starts where the source's does, past a gutter as wide: `p` moves no word
    // sideways, and both lay out for the same width.
    let gutter_w = digits(app.buf.lines.len()) + 1;
    app.view_w = (area.width as usize).saturating_sub(gutter_w).max(1);
    app.view_h = area.height as usize;
    app.preview_sync();
    app.preview_clamp();
    let shown = app.shown_theme().to_string();
    let Some(p) = &mut app.preview else {
        return;
    };
    let end = (p.top + area.height as usize).min(p.doc.rows.len());
    // The code blocks on screen take their colours from the theme, highlighted as far as the
    // screen reaches, as the source is; another theme starts them over.
    if p.theme != shown {
        p.theme = shown;
        p.code.iter_mut().for_each(Buffer::clear_hl);
    }
    for r in &p.doc.rows[p.top..end] {
        if let Kind::Code { block, line, .. } = r.kind {
            p.code[block].highlight_to(line, theme);
        }
    }

    let p = app.preview.as_ref().unwrap();
    let pal = Palette::new(theme);
    let review = app.review.is_some();
    let gutter = base.fg(theme.gutter_fg);
    let ghosts = ghost_marks(&p.doc.rows, app.diff.ghosts.keys().copied());
    let mut lines: Vec<Line> = Vec::with_capacity(end - p.top);
    for i in p.top..end {
        let row = &p.doc.rows[i];
        let cursor = i == p.row;
        // A row's mark is the one of any line it shows: a changed line may start mid-row.
        let marks = || row.lines.clone().filter_map(|l| app.diff.marks.get(&l));
        let mark = marks()
            .find(|m| **m != Mark::DeletedBelow)
            .or_else(|| marks().next());
        let bg = match (review_tint(review, mark, theme), cursor) {
            (Some((_, c)), true) | (Some((c, _)), false) => Some(c),
            (None, true) => Some(theme.line_hl),
            (None, false) => None,
        };
        let t = bg.map_or(base, |c| base.bg(c));
        let g = if cursor {
            gutter.bg(theme.line_hl)
        } else {
            gutter
        };
        let mark = match (mark, ghosts.get(&i)) {
            (None, Some(glyph)) => Span::styled(*glyph, g.fg(Color::Red)),
            (mark, _) => mark_span(mark, g),
        };
        let looks: Vec<(Style, Range<usize>)> = row
            .looks
            .iter()
            .map(|(look, r)| (pal.style(*look, bg.is_some()), r.clone()))
            .collect();
        // A code row's syntax colours, cut to the part of its line the row shows.
        let syntax: Vec<(Style, Range<usize>)> = match row.kind {
            Kind::Code {
                block,
                line,
                from,
                at,
            } => {
                let to = from + row.text.len() - at;
                p.code[block]
                    .hl
                    .get(line)
                    .into_iter()
                    .flatten()
                    .filter_map(|(st, r)| {
                        let (a, b) = (r.start.max(from), r.end.min(to));
                        (a < b).then(|| (*st, a - from + at..b - from + at))
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        let mut spans = vec![Span::styled(" ".repeat(gutter_w - 1), g), mark];
        spans.extend(layered(&row.text, &[&looks, &syntax], t));
        // A code block's tint, the cursor row and a review's tint reach the right edge.
        let fill = match (bg, row.kind) {
            (Some(_), _) => Some(t),
            (None, Kind::Code { .. }) => Some(base.bg(pal.code_bg)),
            _ => None,
        };
        if let Some(fill) = fill {
            let pad = app.view_w.saturating_sub(wrap::width(&row.text));
            spans.push(Span::styled(" ".repeat(pad), fill));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines).style(base), area);
}

/// Where the lines a review deleted are marked, by row: `▔` on the first row of the line they
/// stood above, or, past the last line, `▁` on the last row of the line they stood below, as the
/// source marks lines deleted below a line.
fn ghost_marks(rows: &[Row], keys: impl Iterator<Item = usize>) -> HashMap<usize, &'static str> {
    let mut out = HashMap::new();
    for k in keys {
        let above = rows.iter().position(|r| r.lines.contains(&k));
        let before = |i: &usize| !rows[*i].lines.is_empty() && rows[*i].lines.end <= k;
        let below = || {
            (0..rows.len())
                .filter(before)
                .max_by_key(|&i| (rows[i].lines.end, i))
        };
        let at = match above {
            Some(i) => Some((i, "\u{2594}")),
            None => below().map(|i| (i, "\u{2581}")),
        };
        if let Some((i, glyph)) = at {
            out.insert(i, glyph);
        }
    }
    out
}

/// `text` in spans, each byte in `base` patched with the style every layer gives it, in order.
fn layered<'a>(text: &'a str, layers: &[&[(Style, Range<usize>)]], base: Style) -> Vec<Span<'a>> {
    let mut cuts = vec![0, text.len()];
    for layer in layers {
        cuts.extend(layer.iter().flat_map(|(_, r)| [r.start, r.end]));
    }
    cuts.sort_unstable();
    cuts.dedup();
    cuts.windows(2)
        .map(|w| {
            let style = layers.iter().fold(base, |st, layer| {
                match layer.iter().find(|(_, r)| r.start <= w[0] && w[1] <= r.end) {
                    Some((s, _)) => st.patch(*s),
                    None => st,
                }
            });
            Span::styled(&text[w[0]..w[1]], style)
        })
        .collect()
}
