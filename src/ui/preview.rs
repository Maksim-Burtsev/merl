//! The code pane showing a Markdown file rendered (#249): the rows `markdown` laid out, the
//! cursor row, and the source's git marks beside the rows they came from.

use std::ops::Range;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::buffer;
use crate::git::Mark;
use crate::markdown::{Kind, Palette};
use crate::theme::Theme;
use crate::wrap;

use super::code::digits;

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
    // The code blocks on screen take their colours from the theme, each highlighted whole once.
    if p.hl.0 != shown {
        p.hl = (shown, Default::default());
    }
    for r in &p.doc.rows[p.top..end] {
        if let Kind::Code { block, .. } = r.kind {
            p.hl.1.entry(block).or_insert_with(|| {
                let c = &p.doc.code[block];
                buffer::highlight_lines(&c.lang, &c.lines, theme).unwrap_or_default()
            });
        }
    }

    let p = app.preview.as_ref().unwrap();
    let pal = Palette::new(theme);
    let review = app.review.is_some();
    let gutter = base.fg(theme.gutter_fg);
    let mut lines: Vec<Line> = Vec::with_capacity(end - p.top);
    for i in p.top..end {
        let row = &p.doc.rows[i];
        let cursor = i == p.row;
        let l = row.src.0;
        let mark = app.diff.marks.get(&l);
        // As in the source: a review tints the rows it added and those of a deleted file.
        let tint = match (review, mark) {
            (true, Some(Mark::Added)) => Some((theme.add_bg, theme.add_bg_hl)),
            (true, Some(Mark::DeletedBelow)) => Some((theme.del_bg, theme.del_bg_hl)),
            _ => None,
        };
        let bg = match (tint, cursor) {
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
        // Lines a review deleted stand above the line after them in the source: here a mark on
        // the first row of that line says they were there.
        let first = i == 0 || p.doc.rows[i - 1].src.0 != l;
        let mark = match mark {
            Some(Mark::Added) => Span::styled("\u{258e}", g.fg(Color::Green)),
            Some(Mark::Changed) => Span::styled("\u{258e}", g.fg(Color::Blue)),
            Some(Mark::DeletedBelow) => Span::styled("\u{2581}", g.fg(Color::Red)),
            None if first && app.diff.ghosts.contains_key(&l) => {
                Span::styled("\u{2594}", g.fg(Color::Red))
            }
            None => Span::styled(" ", g),
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
                p.hl.1
                    .get(&block)
                    .and_then(|h| h.get(line))
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
