//! The code pane showing a Markdown file rendered (#249): the rows `markdown` laid out and the
//! cursor row. The git marks stay on the source, where the diff is.

use std::ops::Range;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::buffer::Buffer;
use crate::markdown::{Kind, Palette};
use crate::theme::Theme;
use crate::wrap;

use super::code::digits;
use super::tagged;

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
    let gutter = base.fg(theme.gutter_fg);
    let mut lines: Vec<Line> = Vec::with_capacity(end - p.top);
    for i in p.top..end {
        let row = &p.doc.rows[i];
        let bg = (i == p.row).then_some(theme.line_hl);
        let t = bg.map_or(base, |c| base.bg(c));
        let g = bg.map_or(gutter, |c| gutter.bg(c));
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
        let mut spans = vec![Span::styled(" ".repeat(gutter_w), g)];
        spans.extend(layered(
            &row.text,
            &[&looks, &syntax],
            t,
            Style::new().bg(theme.tag_bg).fg(theme.tag_fg),
        ));
        // A code block's tint and the cursor row reach the right edge.
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

/// `text` in spans, each byte in `base` patched with the style every layer gives it, in order;
/// a hidden char is its tag, in `tag`.
fn layered<'a>(
    text: &'a str,
    layers: &[&[(Style, Range<usize>)]],
    base: Style,
    tag: Style,
) -> Vec<Span<'a>> {
    let mut cuts = vec![0, text.len()];
    for layer in layers {
        cuts.extend(layer.iter().flat_map(|(_, r)| [r.start, r.end]));
    }
    cuts.sort_unstable();
    cuts.dedup();
    let mut out = Vec::new();
    for w in cuts.windows(2) {
        let style = layers.iter().fold(base, |st, layer| {
            match layer.iter().find(|(_, r)| r.start <= w[0] && w[1] <= r.end) {
                Some((s, _)) => st.patch(*s),
                None => st,
            }
        });
        tagged(&mut out, &text[w[0]..w[1]], style, tag);
    }
    out
}
