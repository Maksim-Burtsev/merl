//! The code pane: the text, the gutter, the review's ghost rows and the cursor.

use std::ops::Range;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use regex::Regex;

use crate::app::{App, Mode};
use crate::git::Mark;
use crate::theme::Theme;
use crate::wrap;

use super::expand;

pub(super) fn draw_code(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, base: Style) {
    let gutter_w = digits(app.buf.lines.len()) + 1;
    app.view_w = (area.width as usize).saturating_sub(gutter_w).max(1);
    app.view_h = area.height as usize;
    app.clamp_scroll();
    // Wrapping only ever costs rows, so this covers every visible line.
    app.buf.highlight_to(app.top_line + app.view_h, theme);

    let gutter_style = base.fg(theme.gutter_fg);
    let find_style = Style::new().bg(theme.find_bg).fg(theme.find_fg);
    let hl = base.bg(theme.line_hl);
    let hl_gutter = gutter_style.bg(theme.line_hl);
    let sel = base.bg(theme.selection);
    // Selected lines above the selection's last line are selected through their newline, so
    // their background runs to the right edge like VS Code's.
    let sel_lines = app.selection().map(|(start, end)| (start.0, end.0));
    // The cursor line keeps its highlight under a selection, so starting one inside a wrapped
    // line does not flash its other rows back to the plain background. The one exception is a
    // cursor line none of whose text is selected (the selection ends at its start, or begins at
    // its end and takes only the newline): a highlight close to the selection colour would pass
    // for selected (#48), so that line is highlighted in the gutter only, as in VS Code.
    let text_hl = app.selected_bytes(app.line).is_none_or(|r| !r.is_empty());

    let nowrap = app.nowrap();
    let ghost = base.fg(theme.ghost_fg);
    let ghost_row = |text: &str| {
        Line::from(vec![
            Span::styled(" ".repeat(gutter_w - 1), gutter_style),
            Span::styled("\u{258e}", gutter_style.fg(Color::Red)),
            Span::styled(ghost_text(text, nowrap, app.left, app.view_w), ghost),
        ])
    };
    let mut lines: Vec<Line> = Vec::with_capacity(area.height as usize);
    let mut l = app.top_line;
    let mut skip = app.top_row;
    while lines.len() < area.height as usize && l < app.buf.lines.len() {
        // Review: the lines the branch deleted here, above the text, greyed and unnumbered.
        for (i, text) in app.diff.ghosts.get(&l).into_iter().flatten().enumerate() {
            if i >= skip && lines.len() < area.height as usize {
                lines.push(ghost_row(text));
            }
        }
        skip = skip.saturating_sub(app.diff.ghost_n(l));
        let clipped = app.buf.shown(l);
        let spans = app.buf.hl.get(l).map_or(&[][..], Vec::as_slice);
        // Find matches paint over the syntax colours, so they are merged into the span list.
        let merged;
        let spans = match app.find_re.as_ref().map(|re| matches(re, clipped)) {
            Some(f) if !f.is_empty() => {
                merged = with_find(spans, &f, find_style);
                &merged[..]
            }
            _ => spans,
        };
        let cursor_line = l == app.line;
        let g = if cursor_line { hl_gutter } else { gutter_style };
        let t = if cursor_line && text_hl { hl } else { base };
        let selected = app
            .selected_bytes(l)
            .map(|r| r.start..r.end.min(clipped.len()));
        let pad_selected = sel_lines.is_some_and(|(first, last)| first <= l && l < last);
        let indent = wrap::indent(clipped, app.view_w);
        // Not wrapped, the one row is the columns from `left` on, less a column at either edge
        // that has text beyond it: `‹` and `›` stand there, so a cut line never reads as whole.
        let line_w = wrap::width(clipped);
        let before = nowrap && app.left > 0 && line_w > 0;
        let after = nowrap && line_w > app.left + app.view_w;
        let (shown, cut_lead) = wrap::cut(
            clipped,
            app.left + usize::from(before),
            (app.left + app.view_w).saturating_sub(usize::from(after)),
        );
        let rows = if nowrap { vec![shown] } else { app.rows(l) };
        for (i, r) in rows.into_iter().enumerate() {
            if i < skip {
                continue;
            }
            if lines.len() == area.height as usize {
                break;
            }
            let num = if i == 0 {
                format!("{:>w$}", l + 1, w = gutter_w - 1)
            } else {
                " ".repeat(gutter_w - 1)
            };
            // The column between the number and the text carries the git mark, VS Code style:
            // green added, blue changed, red where lines were deleted.
            let mark = match app.diff.marks.get(&l) {
                Some(Mark::Added) => Span::styled("\u{258e}", g.fg(Color::Green)),
                Some(Mark::Changed) => Span::styled("\u{258e}", g.fg(Color::Blue)),
                Some(Mark::DeletedBelow) => Span::styled("\u{2581}", g.fg(Color::Red)),
                None => Span::styled(" ", g),
            };
            let mut row = vec![Span::styled(num, g), mark];
            if before {
                row.push(Span::styled("\u{2039}", g));
            }
            // Rows after the first start under the text of the first.
            let lead = match (nowrap, i) {
                (true, _) => cut_lead,
                (false, 0) => 0,
                _ => indent,
            };
            if lead > 0 {
                row.push(Span::styled(" ".repeat(lead), t));
            }
            let pad = app.view_w.saturating_sub(
                lead + wrap::width(&clipped[r.clone()]) + usize::from(before) + usize::from(after),
            );
            // The selected part of the row keeps its syntax colours on the selection background.
            let (lo, hi) = match &selected {
                Some(s) => (s.start.clamp(r.start, r.end), s.end.clamp(r.start, r.end)),
                None => (r.end, r.end),
            };
            for (piece, style) in [(r.start..lo, t), (lo..hi, sel), (hi..r.end, t)] {
                if !piece.is_empty() {
                    row.extend(row_spans(clipped, spans, &piece, style));
                }
            }
            if cursor_line || pad_selected || after {
                // Pad so the line background reaches the right edge of the pane.
                let style = if pad_selected { sel } else { t };
                row.push(Span::styled(" ".repeat(pad), style));
            }
            if after {
                row.push(Span::styled("\u{203a}", g));
            }
            lines.push(Line::from(row));
        }
        skip = 0;
        l += 1;
    }
    // Lines deleted at the end of the file sit under the last line, and the view can scroll
    // on into them: `skip` is still the top row then.
    if l == app.buf.lines.len() {
        for text in app.diff.ghosts.get(&l).into_iter().flatten().skip(skip) {
            if lines.len() < area.height as usize {
                lines.push(ghost_row(text));
            }
        }
    }
    frame.render_widget(Paragraph::new(lines).style(base), area);

    let screen_row = rows_between(
        app,
        (app.top_line, app.top_row),
        (app.line, app.cursor_row()),
    );
    if let Some(y) = screen_row.filter(|y| *y < area.height as usize)
        && matches!(app.mode, Mode::Normal | Mode::Edit)
    {
        let x = area.x + (gutter_w + app.cursor_x().saturating_sub(app.left)) as u16;
        frame.set_cursor_position((x.min(area.right().saturating_sub(1)), area.y + y as u16));
    }
}

/// A deleted line of the review: clipped to the pane, or scrolled sideways with the text when the
/// file is not wrapped.
fn ghost_text(text: &str, nowrap: bool, left: usize, width: usize) -> String {
    if !nowrap {
        return expand(&crate::app::clip(text, width)).into_owned();
    }
    let (r, lead) = wrap::cut(text, left, left + width);
    format!("{}{}", " ".repeat(lead), expand(&text[r]))
}

/// Cuts one wrapped row `r` of `text` into spans, taking colours from the line's highlighting
/// and everything else from `base` (which carries the pane or cursor-line background).
fn row_spans<'a>(
    text: &'a str,
    hl: &[(Style, std::ops::Range<usize>)],
    r: &std::ops::Range<usize>,
    base: Style,
) -> Vec<Span<'a>> {
    let mut out = Vec::new();
    let mut pos = r.start;
    for (style, span) in hl {
        if span.end <= r.start {
            continue;
        }
        if span.start >= r.end {
            break;
        }
        let (start, end) = (span.start.max(pos), span.end.min(r.end));
        if pos < start {
            out.push(Span::styled(expand(&text[pos..start]), base));
        }
        out.push(Span::styled(expand(&text[start..end]), base.patch(*style)));
        pos = end;
    }
    if pos < r.end || out.is_empty() {
        out.push(Span::styled(expand(&text[pos..r.end]), base));
    }
    out
}

/// Non-empty match ranges of `re` in one file line.
fn matches(re: &Regex, text: &str) -> Vec<Range<usize>> {
    re.find_iter(text)
        .map(|m| m.range())
        .filter(|r| !r.is_empty())
        .collect()
}

/// Overlays `finds` (sorted, disjoint) on the syntax spans of one line: the syntax spans are
/// cut around them, and the matches are added in `style`. The result stays sorted and disjoint,
/// which is all [`row_spans`] needs.
pub(super) fn with_find(
    hl: &[(Style, Range<usize>)],
    finds: &[Range<usize>],
    style: Style,
) -> Vec<(Style, Range<usize>)> {
    let mut out: Vec<(Style, Range<usize>)> = Vec::with_capacity(hl.len() + finds.len() * 2);
    for (st, r) in hl {
        let mut pos = r.start;
        for f in finds.iter().filter(|f| f.end > r.start && f.start < r.end) {
            if pos < f.start {
                out.push((*st, pos..f.start));
            }
            pos = pos.max(f.end.min(r.end));
        }
        if pos < r.end {
            out.push((*st, pos..r.end));
        }
    }
    out.extend(finds.iter().map(|f| (style, f.clone())));
    out.sort_by_key(|(_, r)| r.start);
    out
}

pub(super) fn digits(n: usize) -> usize {
    n.max(1).ilog10() as usize + 1
}

/// Number of wrapped rows from `from` to `to`, or `None` when `to` is above `from`.
fn rows_between(app: &App, from: (usize, usize), to: (usize, usize)) -> Option<usize> {
    if to < from {
        return None;
    }
    let mut n = 0usize;
    let (mut l, mut r) = from;
    while (l, r) < to {
        r += 1;
        if r >= app.row_count(l) {
            l += 1;
            r = 0;
        }
        n += 1;
        if n > 10_000 {
            return None;
        }
    }
    Some(n)
}
