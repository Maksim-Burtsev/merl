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

const FOLDED_MARK: char = '\u{22ef}';

pub(super) fn draw_code(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, base: Style) {
    let gutter_w = prepare_view(app, theme, area);
    let app: &App = app;
    let p = Painter::new(app, theme, base, gutter_w);
    let mut out = Rows {
        lines: pinned_lines(app, theme, base, gutter_w),
        height: area.height as usize,
        input_at: None,
    };
    let pinned = out.lines.len();
    let mut l = app.top_line;
    let mut skip = app.top_row;
    while out.lines.len() < out.height && l <= app.buf.lines.len() {
        if app.hidden(l) {
            l += 1;
            continue;
        }
        for (i, raw) in app.diff.ghosts.get(&l).into_iter().flatten().enumerate() {
            p.ghost(&mut out, &mut skip, l, i, raw);
        }
        if l == app.buf.lines.len() {
            break;
        }
        p.file_line(&mut out, skip, l);
        skip = 0;
        l += 1;
    }
    let input_at = out.input_at;
    frame.render_widget(Paragraph::new(out.lines).style(base), area);
    place_cursor(frame, app, area, (gutter_w, pinned), input_at);
}

fn prepare_view(app: &mut App, theme: &Theme, area: Rect) -> usize {
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
    gutter_w
}

fn place_cursor(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    (gutter_w, pinned): (usize, usize),
    input_at: Option<(usize, usize)>,
) {
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

struct Rows<'a> {
    lines: Vec<Line<'a>>,
    height: usize,
    input_at: Option<(usize, usize)>,
}

impl<'a> Rows<'a> {
    fn full(&self) -> bool {
        self.lines.len() == self.height
    }

    fn push_slot(&mut self, at: Option<(usize, usize)>, j: usize, nl: Line<'a>) {
        if self.lines.len() < self.height {
            if let Some((_, c)) = at.filter(|&(r, _)| r == j) {
                self.input_at = Some((self.lines.len(), c));
            }
            self.lines.push(nl);
        }
    }
}

struct Painter<'a> {
    app: &'a App,
    theme: &'a Theme,
    base: Style,
    gutter_w: usize,
    gutter_style: Style,
    find_styles: (Style, Style),
    tag: Style,
    hl: Style,
    hl_gutter: Style,
    sel: Style,
    sel_lines: Option<(TextLine, TextLine)>,
    text_hl: bool,
    nowrap: bool,
    review: bool,
    partner: HashMap<(usize, usize), usize>,
    span: Option<(TextLine, TextLine)>,
}

impl<'a> Painter<'a> {
    fn new(app: &'a App, theme: &'a Theme, base: Style, gutter_w: usize) -> Self {
        let find_style = |fg: Option<Color>, bg, groups: &[&str]| {
            fg.map_or(Style::new(), |fg| Style::new().fg(fg))
                .bg(bg)
                .add_modifier(theme.modifiers(groups))
        };
        let text_hl = app.selection().is_none();
        let hl_gutter = base
            .fg(theme.cursor_gutter_fg)
            .add_modifier(theme.modifiers(&["CursorLineNr"]));
        Self {
            app,
            theme,
            base,
            gutter_w,
            gutter_style: base.fg(theme.gutter_fg),
            find_styles: (
                find_style(theme.find_fg, theme.find_bg, &["Search"]),
                find_style(
                    theme.current_find_fg,
                    theme.current_find_bg,
                    &["CurSearch", "IncSearch", "Search"],
                ),
            ),
            tag: Style::new().bg(theme.tag_bg).fg(theme.tag_fg),
            hl: base.bg(theme.line_hl),
            hl_gutter: hl_gutter.bg(if text_hl {
                theme.line_hl
            } else {
                theme.cursor_gutter_bg()
            }),
            sel: theme
                .selected(base)
                .add_modifier(theme.modifiers(&["Visual"])),
            sel_lines: app.selection().map(|(start, end)| (start.0, end.0)),
            text_hl,
            nowrap: app.nowrap(),
            review: app.review.is_some(),
            partner: app.diff.pairs.iter().map(|(&l, &g)| (g, l)).collect(),
            span: app.note_span(),
        }
    }

    fn ellipsis(&self, bg: Style) -> Span<'static> {
        Span::styled("\u{2026}", bg.fg(self.theme.gutter_fg))
    }

    fn word(&self, bg: Color) -> Style {
        Style::new()
            .bg(bg)
            .fg(self.theme.word_fg)
            .remove_modifier(Modifier::BOLD | Modifier::ITALIC | Modifier::UNDERLINED)
    }

    fn in_span(&self, t: TextLine) -> bool {
        self.span.is_some_and(|(a, b)| a <= t && t <= b)
    }

    fn selected_pad(&self, line: TextLine) -> bool {
        self.sel_lines
            .is_some_and(|(first, last)| first <= line && line < last)
    }

    fn plain_syntax(&self, syntax: &Spans) -> Painted {
        if self.theme.selection_fg.is_some() {
            return Vec::new();
        }
        without_bg(syntax)
    }

    fn ghost_wrap(&self, raw: &str) -> Vec<(Range<usize>, usize)> {
        let app = self.app;
        let text = shown_str(raw);
        if self.nowrap {
            let (r, lead) = wrap::cut(text, app.left, app.left + app.view_w);
            return vec![(r, lead)];
        }
        let indent = wrap::indent(text, app.view_w);
        wrap::wrap_shown(raw, app.view_w)
            .into_iter()
            .enumerate()
            .map(|(i, r)| (r, if i == 0 { 0 } else { indent }))
            .collect()
    }

    fn slot_styles(&self, s: &crate::app::Slot) -> (bool, bool, Style, Style) {
        let (app, theme, base) = (self.app, self.theme, self.base);
        let input = matches!(s, crate::app::Slot::Input);
        let (_, _, stale) = app.slot_text(s);
        let focused = matches!(s, crate::app::Slot::Note(i) if Some(app.notes.here[*i].idx) == app.notes.focus)
            && app.mode == Mode::Normal;
        let picked = focused && app.notes.selected;
        let hl_bg = base.bg(theme.line_hl);
        let input_bg = Style::new().bg(theme.status_bg).fg(theme.status_fg);
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
            (false, false) => (base, base.fg(theme.accent)),
        };
        (input, focused, bg, style)
    }

    #[expect(
        clippy::string_slice,
        reason = "slot_layout cuts the rows at wrap_shown's clusters and the prompt's char boundaries"
    )]
    fn slot_lines(&self, s: &crate::app::Slot) -> (Vec<Line<'static>>, Option<(usize, usize)>) {
        let (app, theme, base) = (self.app, self.theme, self.base);
        let (input, focused, bg, style) = self.slot_styles(s);
        let (rows, at) = app.slot_layout(s);
        let label_style = bg.fg(theme.gutter_fg);
        let mark_style = bg.fg(theme.accent);
        let lines = rows
            .into_iter()
            .enumerate()
            .map(|(i, (r, lab, mark))| {
                let mut row = vec![
                    Span::styled(" ".repeat(self.gutter_w - 1), self.gutter_style),
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
    }

    fn ghost_spans(
        &self,
        l: usize,
        i: usize,
        raw: &str,
        here: Option<usize>,
    ) -> (Painted, Painted) {
        let app = self.app;
        let text = shown_str(raw);
        let at = app.diff.ghost_from.get(&l).map(|from| from + i);
        let syntax = match (&app.base, at) {
            (Some((_, b)), Some(at)) if b.lines.get(at).map(String::as_str) == Some(raw) => {
                b.hl.get(at).map_or(&[][..], Vec::as_slice)
            }
            _ => &[],
        };
        let syntax = &without_bg(syntax);
        let words = self
            .partner
            .get(&(l, i))
            .and_then(|&n| app.buf.lines.get(n))
            .map(|line| intraline::changes(text, shown_str(line)).0)
            .unwrap_or_default();
        let finds = app
            .find_re
            .as_ref()
            .map_or_else(Vec::new, |re| matches(re, text));
        let spans = paint_finds(
            &with_find(syntax, &words, self.word(self.theme.del_word_bg)),
            &finds,
            here,
            self.find_styles,
        );
        let plain = paint_finds(&self.plain_syntax(syntax), &finds, here, self.find_styles);
        (spans, plain)
    }

    #[expect(
        clippy::string_slice,
        reason = "r is a row of wrap_line, on cluster boundaries"
    )]
    fn ghost(&self, out: &mut Rows<'a>, skip: &mut usize, l: usize, i: usize, raw: &'a str) {
        let (app, theme, base) = (self.app, self.theme, self.base);
        let line = TextLine::Deleted(l, i);
        let cursor_line = app.at() == line && app.notes.focus.is_none() && app.mode != Mode::Note;
        let bg = if (cursor_line && self.text_hl) || self.in_span(line) {
            base.bg(theme.del_bg_hl)
        } else {
            base.bg(theme.del_bg)
        };
        let g = if cursor_line {
            self.hl_gutter
        } else {
            self.gutter_style
        };
        let text = shown_str(raw);
        let (spans, plain) = self.ghost_spans(l, i, raw, cursor_line.then_some(app.col));
        let selected = app
            .selected_bytes(line)
            .map(|r| r.start..r.end.min(text.len()));
        let pad_style = if self.selected_pad(line) {
            self.sel
        } else {
            bg
        };
        let rows = self.ghost_wrap(raw);
        let last = rows.len() - 1;
        for (i, (r, lead)) in rows.into_iter().enumerate() {
            let ell = Buffer::clips(raw)
                && i == last
                && (!self.nowrap || ellipsis_in_view(app, text, false, false));
            if *skip > 0 {
                *skip -= 1;
                continue;
            }
            if out.full() {
                break;
            }
            let pad = app
                .view_w
                .saturating_sub(lead + wrap::width(&text[r.clone()]) + usize::from(ell));
            let mut row = vec![
                Span::styled(" ".repeat(self.gutter_w - 1), g),
                Span::styled("\u{258e}", g.fg(Color::Red)),
                Span::styled(" ".repeat(lead), bg),
            ];
            row.extend(selected_row(
                text,
                (&spans, &plain),
                &r,
                selected.as_ref(),
                (bg, self.sel),
                self.tag,
            ));
            if ell {
                row.push(self.ellipsis(pad_style));
            }
            row.push(Span::styled(" ".repeat(pad), pad_style));
            out.lines.push(Line::from(row));
        }
        for slot in app.slots_after(line) {
            let (nls, at) = self.slot_lines(&slot);
            for (j, nl) in nls.into_iter().enumerate() {
                if *skip > 0 {
                    *skip -= 1;
                    continue;
                }
                out.push_slot(at, j, nl);
            }
        }
    }

    fn file_spans(
        &self,
        l: usize,
        clipped: &str,
        (lit, tinted): (bool, bool),
        here: Option<usize>,
    ) -> (Painted, Painted) {
        let (app, theme) = (self.app, self.theme);
        let words = app
            .diff
            .pairs
            .get(&l)
            .filter(|_| self.review)
            .and_then(|&(k, i)| app.diff.ghosts.get(&k)?.get(i))
            .map(|old| intraline::changes(shown_str(old), clipped).1)
            .unwrap_or_default();
        let syntax = app.buf.hl.get(l).map_or(&[][..], Vec::as_slice);
        let syntax = &if tinted {
            without_bg(syntax)
        } else {
            syntax.to_vec()
        };
        let finds = app
            .find_re
            .as_ref()
            .map_or_else(Vec::new, |re| matches(re, clipped));
        let word_bg = if lit {
            theme.add_word_bg_hl
        } else {
            theme.add_word_bg
        };
        let spans = paint_finds(
            &with_find(syntax, &words, self.word(word_bg)),
            &finds,
            here,
            self.find_styles,
        );
        let plain = paint_finds(&self.plain_syntax(syntax), &finds, here, self.find_styles);
        (spans, plain)
    }

    fn file_line_parts(&self, l: usize) -> (FileLine<'a>, Vec<Range<usize>>) {
        let (app, theme, base) = (self.app, self.theme, self.base);
        let clipped = app.buf.shown(l);
        let cursor_line =
            app.at() == TextLine::File(l) && app.notes.focus.is_none() && app.mode != Mode::Note;
        let lit = cursor_line && self.text_hl;
        let tint = match (self.review, app.diff.marks.get(&l)) {
            (true, Some(Mark::Added)) => Some((theme.add_bg, theme.add_bg_hl)),
            (true, Some(Mark::Deleted)) => Some((theme.del_bg, theme.del_bg_hl)),
            _ => None,
        };
        let (spans, plain) = self.file_spans(
            l,
            clipped,
            (lit, tint.is_some()),
            cursor_line.then_some(app.col),
        );
        let g = if cursor_line {
            self.hl_gutter
        } else {
            self.gutter_style
        };
        let lit = lit || self.in_span(TextLine::File(l));
        let t = match (tint, lit) {
            (Some((_, row)), true) | (Some((row, _)), false) => base.bg(row),
            (None, true) => self.hl,
            (None, false) => base,
        };
        let selected = app
            .selected_bytes(TextLine::File(l))
            .map(|r| r.start..r.end.min(clipped.len()));
        let pad_selected = self.selected_pad(TextLine::File(l));
        let UnwrappedRow {
            left_mark: before,
            bytes: shown,
            lead: cut_lead,
            right_mark: after,
        } = cut_unwrapped(app, &app.buf.lines[l]);
        let rows = if self.nowrap {
            vec![shown]
        } else {
            app.rows(l)
        };
        let folded = app
            .collapsed_at(l)
            .zip(app.collapsed_tail(l))
            .map(|(e, tail)| {
                let gap = if clipped.ends_with(['{', '(', '[']) {
                    ""
                } else {
                    " "
                };
                (gap, tail, e - l - usize::from(!tail.is_empty()))
            });
        let line = FileLine {
            l,
            clipped,
            cut: Buffer::clips(&app.buf.lines[l]),
            cursor_line,
            padded: cursor_line || lit || pad_selected || tint.is_some(),
            spans,
            plain,
            g,
            t,
            selected,
            pad_style: if pad_selected { self.sel } else { t },
            edges: (self.nowrap && before, self.nowrap && after),
            leads: (cut_lead, wrap::indent(clipped, app.view_w)),
            last: rows.len() - 1,
            folded,
        };
        (line, rows)
    }

    fn file_line(&self, out: &mut Rows<'a>, skip: usize, l: usize) {
        let (line, rows) = self.file_line_parts(l);
        for (i, r) in rows.into_iter().enumerate() {
            if i < skip {
                continue;
            }
            if out.full() {
                break;
            }
            out.lines.push(self.file_row(&line, i, &r));
        }
        let mut k = line.last + 1;
        for slot in self.app.slots_after(TextLine::File(l)) {
            let (nls, at) = self.slot_lines(&slot);
            for (j, nl) in nls.into_iter().enumerate() {
                if k < skip {
                    k += 1;
                    continue;
                }
                k += 1;
                out.push_slot(at, j, nl);
            }
        }
    }

    #[expect(
        clippy::string_slice,
        reason = "r is a row of wrap_line over clipped, on cluster boundaries"
    )]
    fn file_row(&self, f: &FileLine<'a>, i: usize, r: &Range<usize>) -> Line<'a> {
        let (app, theme) = (self.app, self.theme);
        let (before, after) = f.edges;
        let (g, t) = (f.g, f.t);
        let ell = f.cut
            && i == f.last
            && (!self.nowrap || ellipsis_in_view(app, f.clipped, before, after));
        let num = if i == 0 {
            format!("{:>w$}", f.l + 1, w = self.gutter_w - 1)
        } else {
            " ".repeat(self.gutter_w - 1)
        };
        let mark = match app.diff.marks.get(&f.l) {
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
        let lead = match (self.nowrap, i) {
            (true, _) => f.leads.0,
            (false, 0) => 0,
            _ => f.leads.1,
        };
        if lead > 0 {
            row.push(Span::styled(" ".repeat(lead), t));
        }
        let used = lead
            + wrap::width(&f.clipped[r.clone()])
            + usize::from(before)
            + usize::from(after)
            + usize::from(ell);
        let fold = f.folded.filter(|_| i == f.last).map(|(gap, tail, n)| {
            let used = used + gap.len() + wrap::width(tail);
            let counted = format!(" {n} line{} {FOLDED_MARK} ", if n == 1 { "" } else { "s" });
            let chip = if used + wrap::width(&counted) > app.view_w {
                format!(" {FOLDED_MARK} ")
            } else {
                counted
            };
            (gap, chip, tail, used)
        });
        let pad = app.view_w.saturating_sub(
            fold.as_ref()
                .map_or(used, |(_, chip, _, used)| used + wrap::width(chip)),
        );
        row.extend(selected_row(
            f.clipped,
            (&f.spans, &f.plain),
            r,
            f.selected.as_ref(),
            (t, self.sel),
            self.tag,
        ));
        let pad_style = f.pad_style;
        if ell {
            row.push(self.ellipsis(pad_style));
        }
        if let Some((gap, text, tail, _)) = fold {
            let chip = if f.cursor_line {
                theme.selected(t.fg(theme.fg))
            } else {
                t.bg(theme.line_hl).fg(theme.fg)
            };
            row.push(Span::styled(gap, t));
            row.push(Span::styled(text, chip));
            row.push(Span::styled(tail, t));
        }
        if f.padded || after {
            row.push(Span::styled(" ".repeat(pad), pad_style));
        }
        if after {
            row.push(Span::styled("\u{203a}", g));
        }
        Line::from(row)
    }
}

type Painted = Vec<(Style, Range<usize>)>;

fn without_bg(syntax: &Spans) -> Painted {
    syntax
        .iter()
        .map(|(s, r)| (Style { bg: None, ..*s }, r.clone()))
        .collect()
}

struct FileLine<'a> {
    l: usize,
    clipped: &'a str,
    cut: bool,
    cursor_line: bool,
    padded: bool,
    spans: Painted,
    plain: Painted,
    g: Style,
    t: Style,
    selected: Option<Range<usize>>,
    pad_style: Style,
    edges: (bool, bool),
    leads: (usize, usize),
    last: usize,
    folded: Option<(&'static str, &'a str, usize)>,
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

#[expect(
    clippy::string_slice,
    reason = "r is a row of wrap_line, on cluster boundaries"
)]
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
    selected: Option<&Range<usize>>,
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

#[expect(
    clippy::string_slice,
    reason = "r is a row of wrap_line and the spans come from the highlighter's tokens, all on char boundaries"
)]
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
