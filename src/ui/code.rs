//! The code pane: the text, the gutter, the review's ghost rows and the cursor.

use std::collections::HashMap;
use std::ops::Range;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use regex::Regex;

use crate::app::{App, Mode};
use crate::buffer::{Buffer, shown_str};
use crate::git::{Mark, TextLine};
use crate::intraline;
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
    // Review: the ghosts on screen take their colours from the base file, highlighted as far as
    // the last of them. Base lines grow with the keys, so the last key on screen reaches furthest.
    if let Some((_, b)) = &mut app.base
        && let Some((k, from)) = app
            .diff
            .ghost_from
            .range(..=app.top_line + app.view_h)
            .next_back()
    {
        b.highlight_to(from + app.diff.ghosts.get(k).map_or(0, Vec::len), theme);
    }

    let gutter_style = base.fg(theme.gutter_fg);
    let find_style = Style::new().bg(theme.find_bg).fg(theme.find_fg);
    let hl = base.bg(theme.line_hl);
    let hl_gutter = gutter_style.bg(theme.line_hl);
    let sel = base.bg(theme.selection);
    // A line longer than merl draws ends in a dim `…` after its last drawn char (#283), in the
    // colour of `‹` and `›`, on the row's own background.
    let ellipsis = |bg: Style| Span::styled("\u{2026}", bg.fg(theme.gutter_fg));
    // Selected lines above the selection's last line are selected through their newline, so
    // their background runs to the right edge like VS Code's.
    let sel_lines = app.selection().map(|(start, end)| (start.0, end.0));
    // The cursor line keeps its highlight under a selection, so starting one inside a wrapped
    // line does not flash its other rows back to the plain background. The one exception is a
    // cursor line none of whose text is selected (the selection ends at its start, or begins at
    // its end and takes only the newline): a highlight close to the selection colour would pass
    // for selected (#48), so that line is highlighted in the gutter only, as in VS Code.
    let text_hl = app.selected_bytes(app.at()).is_none_or(|r| !r.is_empty());

    let nowrap = app.nowrap();
    // Review paints the diff as GitHub does: the deleted and the added rows on a red and a green
    // tint, the words that changed on a stronger one in `word_fg`. The tints come from the theme.
    let review = app.review.is_some();
    let ghost = base.bg(theme.del_bg);
    let word = |bg| Style::new().bg(bg).fg(theme.word_fg);
    // The added line each ghost was replaced by, for the ghost's side of the changed words.
    let partner: HashMap<(usize, usize), usize> =
        app.diff.pairs.iter().map(|(&l, &g)| (g, l)).collect();
    // A ghost wraps exactly like a file line: its rows after the first start under its indent.
    // Not wrapped, the one row is the columns from `left` on, as the text's are.
    let ghost_wrap = |raw: &str| -> Vec<(Range<usize>, usize)> {
        let text = shown_str(raw);
        if nowrap {
            let (r, lead) = wrap::cut(text, app.left, app.left + app.view_w);
            return vec![(r, lead)];
        }
        let indent = wrap::indent(text, app.view_w);
        wrap::wrap_shown(raw, app.view_w)
            .into_iter()
            .enumerate()
            .map(|(i, r)| (r, if i == 0 { 0 } else { indent }))
            .collect()
    };
    let mut lines = pinned_lines(app, theme, base, gutter_w);
    let pinned = lines.len();
    let mut l = app.top_line;
    let mut skip = app.top_row;
    while lines.len() < area.height as usize && l <= app.buf.lines.len() {
        // Review: the lines the branch deleted here, above the text, unnumbered. `skip` counts
        // rows: the top of the view can sit inside a wrapped ghost, as it can sit on the lines
        // deleted at the end of the file, under the last line.
        for (i, raw) in app.diff.ghosts.get(&l).into_iter().flatten().enumerate() {
            // A line like any other for the cursor, the selection and `/` (#439).
            let line = TextLine::Deleted(l, i);
            let cursor_line = app.at() == line;
            let bg = match cursor_line && text_hl {
                true => base.bg(theme.del_bg_hl),
                false => ghost,
            };
            let g = if cursor_line { hl_gutter } else { gutter_style };
            let text = shown_str(raw);
            // Its syntax colours are the base file's, as long as the base says the same line.
            let at = app.diff.ghost_from.get(&l).map(|from| from + i);
            let syntax = match (&app.base, at) {
                (Some((_, b)), Some(at)) if b.lines.get(at) == Some(raw) => {
                    b.hl.get(at).map_or(&[][..], Vec::as_slice)
                }
                _ => &[],
            };
            let words = partner
                .get(&(l, i))
                .map(|&n| intraline::changes(text, app.buf.shown(n)).0)
                .unwrap_or_default();
            let finds = app
                .find_re
                .as_ref()
                .map_or_else(Vec::new, |re| matches(re, text));
            let spans = with_find(
                &with_find(syntax, &words, word(theme.del_word_bg)),
                &finds,
                find_style,
            );
            let plain = with_find(syntax, &finds, find_style);
            let selected = app
                .selected_bytes(line)
                .map(|r| r.start..r.end.min(text.len()));
            let pad_style =
                match sel_lines.is_some_and(|(first, last)| first <= line && line < last) {
                    true => sel,
                    false => bg,
                };
            let rows = ghost_wrap(raw);
            let last = rows.len() - 1;
            for (i, (r, lead)) in rows.into_iter().enumerate() {
                // Not wrapped, a ghost has no `‹` or `›`.
                let ell = Buffer::clips(raw)
                    && i == last
                    && (!nowrap || ellipsis_in_view(app, text, false, false));
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                if lines.len() == area.height as usize {
                    break;
                }
                // The tint runs to the right edge, as a row's does on GitHub.
                let pad = app
                    .view_w
                    .saturating_sub(lead + wrap::width(&text[r.clone()]) + usize::from(ell));
                let mut row = vec![
                    Span::styled(" ".repeat(gutter_w - 1), g),
                    Span::styled("\u{258e}", g.fg(Color::Red)),
                    Span::styled(" ".repeat(lead), bg),
                ];
                row.extend(selected_row(
                    text,
                    (&spans, &plain),
                    &r,
                    &selected,
                    (bg, sel),
                ));
                if ell {
                    row.push(ellipsis(pad_style));
                }
                row.push(Span::styled(" ".repeat(pad), pad_style));
                lines.push(Line::from(row));
            }
        }
        if l == app.buf.lines.len() {
            break;
        }
        let clipped = app.buf.shown(l);
        let cut = Buffer::clips(&app.buf.lines[l]);
        let cursor_line = app.at() == TextLine::File(l);
        let lit = cursor_line && text_hl;
        // A review tints the rows the branch added, or those of a file it deleted; the gutter
        // marks against the index outside a review get no tint.
        let tint = match (review, app.diff.marks.get(&l)) {
            (true, Some(Mark::Added)) => Some((theme.add_bg, theme.add_bg_hl)),
            (true, Some(Mark::DeletedBelow)) => Some((theme.del_bg, theme.del_bg_hl)),
            _ => None,
        };
        let words = app
            .diff
            .pairs
            .get(&l)
            .filter(|_| review)
            .and_then(|&(k, i)| app.diff.ghosts.get(&k)?.get(i))
            .map(|old| intraline::changes(shown_str(old), clipped).1)
            .unwrap_or_default();
        let syntax = app.buf.hl.get(l).map_or(&[][..], Vec::as_slice);
        let finds = app
            .find_re
            .as_ref()
            .map_or_else(Vec::new, |re| matches(re, clipped));
        // Changed words paint over the syntax colours and find matches over both, so they are
        // merged into the span list. The selection wins over a changed word: the selected part
        // of a row is drawn from the spans without them.
        let word_bg = if lit {
            theme.add_word_bg_hl
        } else {
            theme.add_word_bg
        };
        let spans = with_find(
            &with_find(syntax, &words, word(word_bg)),
            &finds,
            find_style,
        );
        let plain = with_find(syntax, &finds, find_style);
        let g = if cursor_line { hl_gutter } else { gutter_style };
        let t = match (tint, lit) {
            (Some((_, row)), true) | (Some((row, _)), false) => base.bg(row),
            (None, true) => hl,
            (None, false) => base,
        };
        let selected = app
            .selected_bytes(TextLine::File(l))
            .map(|r| r.start..r.end.min(clipped.len()));
        let pad_selected = sel_lines
            .is_some_and(|(first, last)| first <= TextLine::File(l) && TextLine::File(l) < last);
        let indent = wrap::indent(clipped, app.view_w);
        let (before, shown, cut_lead, after) = cut_unwrapped(app, &app.buf.lines[l]);
        let (before, after) = (nowrap && before, nowrap && after);
        let rows = if nowrap { vec![shown] } else { app.rows(l) };
        let last = rows.len() - 1;
        for (i, r) in rows.into_iter().enumerate() {
            let ell =
                cut && i == last && (!nowrap || ellipsis_in_view(app, clipped, before, after));
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
                lead + wrap::width(&clipped[r.clone()])
                    + usize::from(before)
                    + usize::from(after)
                    + usize::from(ell),
            );
            row.extend(selected_row(
                clipped,
                (&spans, &plain),
                &r,
                &selected,
                (t, sel),
            ));
            let pad_style = if pad_selected { sel } else { t };
            if ell {
                row.push(ellipsis(pad_style));
            }
            if cursor_line || pad_selected || after || tint.is_some() {
                // Pad so the line background reaches the right edge of the pane.
                row.push(Span::styled(" ".repeat(pad), pad_style));
            }
            if after {
                row.push(Span::styled("\u{203a}", g));
            }
            lines.push(Line::from(row));
        }
        skip = 0;
        l += 1;
    }
    frame.render_widget(Paragraph::new(lines).style(base), area);

    let screen_row =
        rows_between(app, (app.top_line, app.top_row), app.cursor_at()).map(|y| y + pinned);
    if let Some(y) = screen_row.filter(|y| *y < area.height as usize)
        && matches!(app.mode, Mode::Normal | Mode::Edit)
    {
        let x = area.x + (gutter_w + app.cursor_x().saturating_sub(app.left)) as u16;
        frame.set_cursor_position((x.min(area.right().saturating_sub(1)), area.y + y as u16));
    }
}

/// A binary file (#287), as VS Code shows one: an empty pane with one dimmed line, centred
/// where the welcome screen's block sits, and no gutter or cursor.
pub(super) fn draw_binary(frame: &mut Frame, theme: &Theme, area: Rect, base: Style) {
    let [_, row] = Layout::vertical([
        Constraint::Length(area.height.saturating_sub(1) * 2 / 5),
        Constraint::Length(1),
    ])
    .areas(area);
    let note = Line::styled("binary file, not shown", base.fg(theme.ghost_fg)).centered();
    frame.render_widget(Paragraph::new(note).style(base), row);
}

/// The declarations enclosing the top of the view, pinned over the text (#248): the first row
/// of each in its syntax colours, beside its line number, on a band of the cursor line's colour
/// across the pane. The band is what tells them from the code under them at a glance.
fn pinned_lines<'a>(app: &'a App, theme: &Theme, base: Style, gutter_w: usize) -> Vec<Line<'a>> {
    let band = base.bg(theme.line_hl);
    app.pinned(app.top_line)
        .iter()
        .map(|&p| {
            let text = app.buf.shown(p);
            let (before, r, lead, after) = match app.nowrap() {
                true => cut_unwrapped(app, &app.buf.lines[p]),
                false => (false, app.rows(p).swap_remove(0), 0, false),
            };
            // Only a first row is pinned: wrapped, the end of a cut line is never on it.
            let ell = app.nowrap()
                && Buffer::clips(&app.buf.lines[p])
                && ellipsis_in_view(app, text, before, after);
            let syntax = app.buf.hl.get(p).map_or(&[][..], Vec::as_slice);
            let g = band.fg(theme.gutter_fg);
            let num = format!("{:>w$} ", p + 1, w = gutter_w - 1);
            let mut row = vec![Span::styled(num, g)];
            if before {
                row.push(Span::styled("\u{2039}", g));
            }
            row.push(Span::styled(" ".repeat(lead), band));
            let pad = app.view_w.saturating_sub(
                lead + wrap::width(&text[r.clone()])
                    + usize::from(before)
                    + usize::from(after)
                    + usize::from(ell),
            );
            row.extend(row_spans(text, syntax, &r, band));
            if ell {
                row.push(Span::styled("\u{2026}", g));
            }
            // Pad so the band reaches the right edge of the pane.
            row.push(Span::styled(" ".repeat(pad), band));
            if after {
                row.push(Span::styled("\u{203a}", g));
            }
            Line::from(row)
        })
        .collect()
}

/// The one row of `text` shown when lines are not wrapped: the columns from `left` on, less a
/// column at either edge that has text beyond it, where `‹` and `›` stand, so a cut line never
/// reads as whole. Whether `‹` stands, the bytes shown, the blank columns before them (a wide
/// character cut at the edge) and whether `›` stands. `raw` is the whole line: one cut at
/// [`Buffer::shown`] takes a column more, for its `…`.
fn cut_unwrapped(app: &App, raw: &str) -> (bool, std::ops::Range<usize>, usize, bool) {
    let text = shown_str(raw);
    let w = wrap::width(text) + usize::from(Buffer::clips(raw));
    let before = app.left > 0 && w > 0;
    let after = w > app.left + app.view_w;
    let (r, lead) = wrap::cut(
        text,
        app.left + usize::from(before),
        (app.left + app.view_w).saturating_sub(usize::from(after)),
    );
    (before, r, lead, after)
}

/// Not wrapped: whether the column after `text`, where a cut line's `…` stands, is in view,
/// between the columns `‹` and `›` take when they stand. Drawn right after the row's text and
/// its lead, the `…` then lands on that column, even when no char of the line is in view.
fn ellipsis_in_view(app: &App, text: &str, before: bool, after: bool) -> bool {
    let end = wrap::width(text);
    app.left + usize::from(before) <= end && end + usize::from(after) < app.left + app.view_w
}

/// Styled byte ranges of one line, sorted and disjoint, as [`row_spans`] reads them.
type Spans = [(Style, Range<usize>)];

/// Row `r` of `text` on the background `bg`, the part of it in `selected` on the selection's
/// `sel`: there it keeps its syntax colours and find matches (`plain`) but not the changed words
/// the rest of it has (`spans`).
fn selected_row<'a>(
    text: &'a str,
    (spans, plain): (&Spans, &Spans),
    r: &Range<usize>,
    selected: &Option<Range<usize>>,
    (bg, sel): (Style, Style),
) -> Vec<Span<'a>> {
    let (lo, hi) = match selected {
        Some(s) => (s.start.clamp(r.start, r.end), s.end.clamp(r.start, r.end)),
        None => (r.end, r.end),
    };
    let mut out = Vec::new();
    for (piece, style, spans) in [
        (r.start..lo, bg, spans),
        (lo..hi, sel, plain),
        (hi..r.end, bg, spans),
    ] {
        if !piece.is_empty() {
            out.extend(row_spans(text, spans, &piece, style));
        }
    }
    out
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

/// Overlays `finds` (sorted, disjoint: find matches, or the words a review says changed) on the
/// syntax spans of one line: the syntax spans are cut around them, and they are added in
/// `style`. The result stays sorted and disjoint, which is all [`row_spans`] needs.
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
    (from <= to).then(|| app.rows_between(from, to))
}
