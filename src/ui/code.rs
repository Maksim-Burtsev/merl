#![expect(clippy::cognitive_complexity, reason = "#858")]
#![expect(clippy::excessive_nesting, reason = "#858")]
#![expect(clippy::ref_option, reason = "#858")]
#![expect(clippy::string_slice, reason = "#858")]
#![expect(clippy::too_many_lines, reason = "#858")]
#![expect(clippy::unwrap_used, reason = "#858")]
use std::collections::HashMap;
use std::ops::Range;
use std::path::Path;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use regex::Regex;

use crate::app::{App, Mode};
use crate::buffer::{Buffer, shown_str};
use crate::git::{Mark, TextLine};
use crate::intraline;
use crate::theme::Theme;
use crate::wrap;

use super::status::fit_path;
use super::tagged;

const FOLDED: &str = " \u{22ef} ";

pub(super) fn draw_code(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, base: Style) {
    app.notes_sync();
    let gutter_w = digits(app.buf.lines.len()) + 1;
    app.view_w = (area.width as usize).saturating_sub(gutter_w).max(1);
    app.view_h = area.height as usize;
    app.clamp_scroll();
    app.notes_reveal_input();
    let bottom = (app.top_line + app.view_h).max(app.bottom_line() + 1);
    app.buf.highlight_to(bottom, theme);
    if let Some((_, b)) = &mut app.base
        && let Some((k, from)) = app.diff.ghost_from.range(..=bottom).next_back()
    {
        b.highlight_to(from + app.diff.ghosts.get(k).map_or(0, Vec::len), theme);
    }

    let gutter_style = base.fg(theme.gutter_fg);
    let find_style = |fg: Option<Color>, bg, groups: &[&str]| {
        fg.map_or(Style::new(), |fg| Style::new().fg(fg))
            .bg(bg)
            .add_modifier(theme.modifiers(groups))
    };
    let find_styles = (
        find_style(theme.find_fg, theme.find_bg, &["Search"]),
        find_style(
            theme.current_find_fg,
            theme.current_find_bg,
            &["CurSearch", "IncSearch", "Search"],
        ),
    );
    let tag = Style::new().bg(theme.tag_bg).fg(theme.tag_fg);
    let hl = base.bg(theme.line_hl);
    let hl_gutter = base
        .fg(theme.cursor_gutter_fg)
        .bg(theme.line_hl)
        .add_modifier(theme.modifiers(&["CursorLineNr"]));
    let sel = theme
        .selected(base)
        .add_modifier(theme.modifiers(&["Visual"]));
    let ellipsis = |bg: Style| Span::styled("\u{2026}", bg.fg(theme.gutter_fg));
    let sel_lines = app.selection().map(|(start, end)| (start.0, end.0));
    let text_hl = app.selected_bytes(app.at()).is_none_or(|r| !r.is_empty());

    let nowrap = app.nowrap();
    let review = app.review.is_some();
    let ghost = base.bg(theme.del_bg);
    let word = |bg| {
        Style::new()
            .bg(bg)
            .fg(theme.word_fg)
            .remove_modifier(Modifier::BOLD | Modifier::ITALIC | Modifier::UNDERLINED)
    };
    let partner: HashMap<(usize, usize), usize> =
        app.diff.pairs.iter().map(|(&l, &g)| (g, l)).collect();
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
    let accent = base.fg(theme.accent);
    let input_bg = Style::new().bg(theme.status_bg).fg(theme.status_fg);
    let slot_lines = |s: &crate::app::Slot| -> (Vec<Line<'static>>, Option<(usize, usize)>) {
        let input = matches!(s, crate::app::Slot::Input);
        let (_, _, stale) = app.slot_text(s);
        let (rows, at) = app.slot_layout(s);
        let focused = matches!(s, crate::app::Slot::Note(i) if Some(app.notes.here[*i].idx) == app.notes.focus)
            && app.mode == Mode::Normal;
        let picked = focused && app.notes.selected;
        let hl_bg = base.bg(theme.line_hl);
        let (bg, style) = match (input, stale) {
            (true, _) => (input_bg, input_bg),
            (false, _) if picked => (
                base.bg(theme.selection),
                base.bg(theme.selection).fg(theme.accent),
            ),
            (false, _) if focused => (
                hl_bg,
                if stale {
                    hl_bg.fg(theme.ghost_fg)
                } else {
                    hl_bg.fg(theme.accent)
                },
            ),
            (false, true) => (base, base.fg(theme.ghost_fg)),
            (false, false) => (base, accent),
        };
        let label_style = bg.fg(theme.gutter_fg);
        let mark_style = bg.fg(theme.accent);
        let lines = rows
            .into_iter()
            .enumerate()
            .map(|(i, (r, lab, mark))| {
                let mut row = vec![
                    Span::styled(" ".repeat(gutter_w - 1), gutter_style),
                    Span::styled(if i == 0 { "\u{270e}" } else { " " }, mark_style),
                    Span::styled(" ", bg),
                ];
                if lab > 0 {
                    row.push(Span::styled(r[..lab].to_string(), label_style));
                }
                match mark {
                    Some(m) => {
                        let m = m.start.max(lab)..m.end.max(lab);
                        row.push(Span::styled(r[lab..m.start].to_string(), style));
                        row.push(Span::styled(
                            r[m.clone()].to_string(),
                            base.bg(theme.selection).fg(theme.fg),
                        ));
                        row.push(Span::styled(r[m.end..].to_string(), style));
                    }
                    None if lab < r.len() => row.push(Span::styled(r[lab..].to_string(), style)),
                    None => {}
                }
                if input || focused {
                    let used = 2 + wrap::width(&r);
                    row.push(Span::styled(
                        " ".repeat(app.view_w.saturating_sub(used) + 1),
                        bg,
                    ));
                }
                Line::from(row)
            })
            .collect();
        (lines, at)
    };
    let mut input_at: Option<(usize, usize)> = None;
    let span = app.note_span();
    let in_span = |t: TextLine| span.is_some_and(|(a, b)| a <= t && t <= b);
    let mut lines = pinned_lines(app, theme, base, gutter_w);
    let pinned = lines.len();
    let mut l = app.top_line;
    let mut skip = app.top_row;
    while lines.len() < area.height as usize && l <= app.buf.lines.len() {
        if app.hidden(l) {
            l += 1;
            continue;
        }
        for (i, raw) in app.diff.ghosts.get(&l).into_iter().flatten().enumerate() {
            let line = TextLine::Deleted(l, i);
            let cursor_line =
                app.at() == line && app.notes.focus.is_none() && app.mode != Mode::Note;
            let bg = if (cursor_line && text_hl) || in_span(line) {
                base.bg(theme.del_bg_hl)
            } else {
                ghost
            };
            let g = if cursor_line { hl_gutter } else { gutter_style };
            let text = shown_str(raw);
            let at = app.diff.ghost_from.get(&l).map(|from| from + i);
            let syntax = match (&app.base, at) {
                (Some((_, b)), Some(at)) if b.lines.get(at) == Some(raw) => {
                    b.hl.get(at).map_or(&[][..], Vec::as_slice)
                }
                _ => &[],
            };
            let words = partner
                .get(&(l, i))
                .and_then(|&n| app.buf.lines.get(n))
                .map(|line| intraline::changes(text, shown_str(line)).0)
                .unwrap_or_default();
            let finds = app
                .find_re
                .as_ref()
                .map_or_else(Vec::new, |re| matches(re, text));
            let here = cursor_line.then_some(app.col);
            let spans = paint_finds(
                &with_find(syntax, &words, word(theme.del_word_bg)),
                &finds,
                here,
                find_styles,
            );
            let plain = paint_finds(
                if theme.selection_fg.is_some() {
                    &[]
                } else {
                    syntax
                },
                &finds,
                here,
                find_styles,
            );
            let selected = app
                .selected_bytes(line)
                .map(|r| r.start..r.end.min(text.len()));
            let pad_style = if sel_lines.is_some_and(|(first, last)| first <= line && line < last) {
                sel
            } else {
                bg
            };
            let rows = ghost_wrap(raw);
            let last = rows.len() - 1;
            for (i, (r, lead)) in rows.into_iter().enumerate() {
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
                    tag,
                ));
                if ell {
                    row.push(ellipsis(pad_style));
                }
                row.push(Span::styled(" ".repeat(pad), pad_style));
                lines.push(Line::from(row));
            }
            {
                for slot in app.slots_after(line) {
                    let (nls, at) = slot_lines(&slot);
                    for (j, nl) in nls.into_iter().enumerate() {
                        if skip > 0 {
                            skip -= 1;
                            continue;
                        }
                        if lines.len() < area.height as usize {
                            if at.is_some_and(|(r, _)| r == j) {
                                input_at = Some((lines.len(), at.unwrap().1));
                            }
                            lines.push(nl);
                        }
                    }
                }
            }
        }
        if l == app.buf.lines.len() {
            break;
        }
        let clipped = app.buf.shown(l);
        let cut = Buffer::clips(&app.buf.lines[l]);
        let cursor_line =
            app.at() == TextLine::File(l) && app.notes.focus.is_none() && app.mode != Mode::Note;
        let lit = cursor_line && text_hl;
        let tint = match (review, app.diff.marks.get(&l)) {
            (true, Some(Mark::Added)) => Some((theme.add_bg, theme.add_bg_hl)),
            (true, Some(Mark::Deleted)) => Some((theme.del_bg, theme.del_bg_hl)),
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
        let word_bg = if lit {
            theme.add_word_bg_hl
        } else {
            theme.add_word_bg
        };
        let here = cursor_line.then_some(app.col);
        let spans = paint_finds(
            &with_find(syntax, &words, word(word_bg)),
            &finds,
            here,
            find_styles,
        );
        let plain = paint_finds(
            if theme.selection_fg.is_some() {
                &[]
            } else {
                syntax
            },
            &finds,
            here,
            find_styles,
        );
        let g = if cursor_line { hl_gutter } else { gutter_style };
        let lit = lit || in_span(TextLine::File(l));
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
        let UnwrappedRow {
            left_mark: before,
            bytes: shown,
            lead: cut_lead,
            right_mark: after,
        } = cut_unwrapped(app, &app.buf.lines[l]);
        let (before, after) = (nowrap && before, nowrap && after);
        let rows = if nowrap { vec![shown] } else { app.rows(l) };
        let last = rows.len() - 1;
        let folded = app.collapsed_tail(l).map(|tail| {
            let gap = if clipped.ends_with(['{', '(', '[']) {
                ""
            } else {
                " "
            };
            (gap, tail)
        });
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
            let mark = match app.diff.marks.get(&l) {
                Some(Mark::Added) => Span::styled("\u{258e}", g.fg(Color::Green)),
                Some(Mark::Changed) => Span::styled("\u{258e}", g.fg(Color::Blue)),
                Some(Mark::Deleted) => Span::styled("\u{258e}", g.fg(Color::Red)),
                Some(Mark::DeletedBelow) => Span::styled("\u{2581}", g.fg(Color::Red)),
                None => Span::styled(" ", g),
            };
            let mut row = vec![Span::styled(num, g), mark];
            if before {
                row.push(Span::styled("\u{2039}", g));
            }
            let lead = match (nowrap, i) {
                (true, _) => cut_lead,
                (false, 0) => 0,
                _ => indent,
            };
            if lead > 0 {
                row.push(Span::styled(" ".repeat(lead), t));
            }
            let fold = folded.as_ref().filter(|_| i == last);
            let pad = app.view_w.saturating_sub(
                lead + wrap::width(&clipped[r.clone()])
                    + usize::from(before)
                    + usize::from(after)
                    + usize::from(ell)
                    + fold.map_or(0, |(gap, tail)| {
                        gap.len() + wrap::width(FOLDED) + wrap::width(tail)
                    }),
            );
            row.extend(selected_row(
                clipped,
                (&spans, &plain),
                &r,
                &selected,
                (t, sel),
                tag,
            ));
            let pad_style = if pad_selected { sel } else { t };
            if ell {
                row.push(ellipsis(pad_style));
            }
            if let Some((gap, tail)) = fold {
                let chip = if cursor_line {
                    theme.selected(t.fg(theme.fg))
                } else {
                    t.bg(theme.line_hl).fg(theme.fg)
                };
                row.push(Span::styled(*gap, t));
                row.push(Span::styled(FOLDED, chip));
                row.push(Span::styled(*tail, t));
            }
            if cursor_line || lit || pad_selected || after || tint.is_some() {
                row.push(Span::styled(" ".repeat(pad), pad_style));
            }
            if after {
                row.push(Span::styled("\u{203a}", g));
            }
            lines.push(Line::from(row));
        }
        {
            let mut k = last + 1;
            for slot in app.slots_after(TextLine::File(l)) {
                let (nls, at) = slot_lines(&slot);
                for (j, nl) in nls.into_iter().enumerate() {
                    if k < skip {
                        k += 1;
                        continue;
                    }
                    k += 1;
                    if lines.len() < area.height as usize {
                        if at.is_some_and(|(r, _)| r == j) {
                            input_at = Some((lines.len(), at.unwrap().1));
                        }
                        lines.push(nl);
                    }
                }
            }
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
        let x = match app.notes.focus {
            Some(_) => area.x + u16::try_from(gutter_w).unwrap_or(u16::MAX) - 1,
            None => {
                area.x
                    + u16::try_from(gutter_w + app.cursor_x().saturating_sub(app.left))
                        .unwrap_or(u16::MAX)
            }
        };
        frame.set_cursor_position((
            x.min(area.right().saturating_sub(1)),
            area.y + u16::try_from(y).unwrap_or(u16::MAX),
        ));
    }
    if app.mode == Mode::Note
        && let Some((y, x)) = input_at
    {
        let x = area.x + u16::try_from(gutter_w + 1 + x).unwrap_or(u16::MAX);
        frame.set_cursor_position((
            x.min(area.right().saturating_sub(1)),
            area.y + u16::try_from(y).unwrap_or(u16::MAX),
        ));
    }
}

pub(super) fn draw_binary(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    base: Style,
    note: Option<String>,
) {
    let note = note.unwrap_or_else(|| "binary file, not shown".into());
    let note = Line::styled(note, base.fg(theme.ghost_fg));
    draw_note(frame, area, base, vec![note]);
}

pub(super) fn draw_empty(frame: &mut Frame, theme: &Theme, area: Rect, base: Style, note: String) {
    let dim = base.fg(theme.ghost_fg);
    let lines = vec![
        Line::styled(note, dim),
        Line::default(),
        next_file(base, dim),
    ];
    draw_note(frame, area, base, lines);
}

pub(super) fn draw_renamed(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    base: Style,
    old: &Path,
    new: &Path,
) {
    let (old, new) = (old.display().to_string(), new.display().to_string());
    let dim = base.fg(theme.ghost_fg);
    let changed = renamed_part(&old, &new);
    let del = base.bg(theme.del_word_bg).fg(theme.word_fg);
    let add = base.bg(theme.add_word_bg).fg(theme.word_fg);
    let room = (area.width as usize).saturating_sub(2);
    let mut lines = if wrap::width(&old) + 5 + wrap::width(&new) <= room {
        let mut row = tinted(&old, changed, base, del, room);
        row.push(Span::styled("  \u{2192}  ", dim));
        row.extend(tinted(&new, changed, base, add, room));
        vec![Line::from(row)]
    } else {
        let mut second = vec![Span::styled("\u{2192}  ", dim)];
        second.extend(tinted(&new, changed, base, add, room.saturating_sub(3)));
        vec![
            Line::from(tinted(&old, changed, base, del, room)),
            Line::from(second),
        ]
    };
    lines.extend([
        Line::styled("renamed, no changes", dim),
        Line::default(),
        next_file(base, dim),
    ]);
    draw_note(frame, area, base, lines);
}

fn next_file(base: Style, dim: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled("c", base.add_modifier(Modifier::BOLD)),
        Span::styled("  next file", dim),
    ])
}

fn renamed_part(old: &str, new: &str) -> (usize, usize) {
    let (o, n): (Vec<char>, Vec<char>) = (old.chars().collect(), new.chars().collect());
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    let inside = |s: &[char], i: usize| 0 < i && i < s.len() && word(s[i - 1]) && word(s[i]);
    let mut pre = o.iter().zip(&n).take_while(|(a, b)| a == b).count();
    while pre > 0 && (inside(&o, pre) || inside(&n, pre)) {
        pre -= 1;
    }
    let same_end = o
        .iter()
        .rev()
        .zip(n.iter().rev())
        .take_while(|(a, b)| a == b);
    let mut suf = same_end.count().min(o.len().min(n.len()) - pre);
    while suf > 0 && (inside(&o, o.len() - suf) || inside(&n, n.len() - suf)) {
        suf -= 1;
    }
    (pre, suf)
}

fn tinted(
    path: &str,
    (pre, suf): (usize, usize),
    base: Style,
    word: Style,
    room: usize,
) -> Vec<Span<'static>> {
    let fitted = fit_path(path, room);
    let shown: Vec<char> = fitted.chars().collect();
    let whole = path.chars().count();
    let (lead, cut) = if fitted == path {
        (0, 0)
    } else {
        (1, whole + 1 - shown.len())
    };
    let start = lead + pre.saturating_sub(cut);
    let end = shown.len() - suf.min(whole - cut);
    let part = |r: Range<usize>, style| Span::styled(shown[r].iter().collect::<String>(), style);
    vec![
        part(0..start, base),
        part(start..end, word),
        part(end..shown.len(), base),
    ]
}

fn draw_note(frame: &mut Frame, area: Rect, base: Style, lines: Vec<Line>) {
    let [_, rows] = Layout::vertical([
        Constraint::Length(area.height.saturating_sub(1) * 2 / 5),
        Constraint::Length(u16::try_from(lines.len()).unwrap_or(u16::MAX)),
    ])
    .areas(area);
    frame.render_widget(Paragraph::new(lines).style(base).centered(), rows);
}

fn pinned_lines<'a>(app: &'a App, theme: &Theme, base: Style, gutter_w: usize) -> Vec<Line<'a>> {
    let band = base.bg(theme.line_hl);
    let tag = Style::new().bg(theme.tag_bg).fg(theme.tag_fg);
    app.pinned(app.top_line)
        .iter()
        .map(|&p| {
            let text = app.buf.shown(p);
            let UnwrappedRow {
                left_mark: before,
                bytes: r,
                lead,
                right_mark: after,
            } = if app.nowrap() {
                cut_unwrapped(app, &app.buf.lines[p])
            } else {
                UnwrappedRow {
                    left_mark: false,
                    bytes: app.rows(p).swap_remove(0),
                    lead: 0,
                    right_mark: false,
                }
            };
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
            row.extend(row_spans(text, syntax, &r, band, tag));
            if ell {
                row.push(Span::styled("\u{2026}", g));
            }
            row.push(Span::styled(" ".repeat(pad), band));
            if after {
                row.push(Span::styled("\u{203a}", g));
            }
            Line::from(row)
        })
        .collect()
}

fn cut_unwrapped(app: &App, raw: &str) -> UnwrappedRow {
    let text = shown_str(raw);
    let w = wrap::width(text) + usize::from(Buffer::clips(raw));
    let before = app.left > 0 && w > 0;
    let after = w > app.left + app.view_w;
    let (r, lead) = wrap::cut(
        text,
        app.left + usize::from(before),
        (app.left + app.view_w).saturating_sub(usize::from(after)),
    );
    UnwrappedRow {
        left_mark: before,
        bytes: r,
        lead,
        right_mark: after,
    }
}

struct UnwrappedRow {
    left_mark: bool,
    bytes: Range<usize>,
    lead: usize,
    right_mark: bool,
}

fn ellipsis_in_view(app: &App, text: &str, before: bool, after: bool) -> bool {
    let end = wrap::width(text);
    app.left + usize::from(before) <= end && end + usize::from(after) < app.left + app.view_w
}

type Spans = [(Style, Range<usize>)];

fn selected_row<'a>(
    text: &'a str,
    (spans, plain): (&Spans, &Spans),
    r: &Range<usize>,
    selected: &Option<Range<usize>>,
    (bg, sel): (Style, Style),
    tag: Style,
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
            out.extend(row_spans(text, spans, &piece, style, tag));
        }
    }
    out
}

fn row_spans<'a>(
    text: &'a str,
    hl: &[(Style, Range<usize>)],
    r: &Range<usize>,
    base: Style,
    tag: Style,
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
            tagged(&mut out, &text[pos..start], base, tag);
        }
        tagged(&mut out, &text[start..end], base.patch(*style), tag);
        pos = end;
    }
    if pos < r.end || out.is_empty() {
        tagged(&mut out, &text[pos..r.end], base, tag);
    }
    out
}

fn matches(re: &Regex, text: &str) -> Vec<Range<usize>> {
    re.find_iter(text)
        .map(|m| m.range())
        .filter(|r| !r.is_empty())
        .collect()
}

fn paint_finds(
    hl: &[(Style, Range<usize>)],
    finds: &[Range<usize>],
    cursor: Option<usize>,
    (find, current): (Style, Style),
) -> Vec<(Style, Range<usize>)> {
    let (on, off): (Vec<_>, Vec<_>) = finds
        .iter()
        .cloned()
        .partition(|f| cursor.is_some_and(|c| f.contains(&c)));
    with_find(&with_find(hl, &off, find), &on, current)
}

pub(super) fn with_find(
    hl: &[(Style, Range<usize>)],
    finds: &[Range<usize>],
    style: Style,
) -> Vec<(Style, Range<usize>)> {
    let mut filled = Vec::with_capacity(hl.len() * 2 + 1);
    let mut pos = 0;
    for (st, r) in hl {
        if pos < r.start {
            filled.push((Style::new(), pos..r.start));
        }
        filled.push((*st, r.clone()));
        pos = r.end;
    }
    if let Some(f) = finds.last().filter(|f| f.end > pos) {
        filled.push((Style::new(), pos..f.end));
    }
    let mut out = Vec::with_capacity(filled.len() + finds.len() * 2);
    for (st, r) in filled {
        let mut pos = r.start;
        for f in finds.iter().filter(|f| f.end > r.start && f.start < r.end) {
            let (start, end) = (f.start.max(r.start), f.end.min(r.end));
            if pos < start {
                out.push((st, pos..start));
            }
            out.push((st.patch(style), start..end));
            pos = end;
        }
        if pos < r.end {
            out.push((st, pos..r.end));
        }
    }
    out
}

pub(super) fn digits(n: usize) -> usize {
    n.max(1).ilog10() as usize + 1
}

fn rows_between(app: &App, from: (usize, usize), to: (usize, usize)) -> Option<usize> {
    (from <= to).then(|| app.rows_between(from, to))
}
