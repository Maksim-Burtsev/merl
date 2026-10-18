use std::num::NonZeroU16;

use ratatui::Frame;
use ratatui::buffer::{CellDiffOption, CellWidth};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::Block;

use crate::app::{App, Focus, Mode};
use crate::theme::Theme;
use crate::wrap;

mod code;
mod fold;
mod overlays;
pub mod picture;
mod preview;
mod status;
mod welcome;

use code::{draw_binary, draw_code, draw_empty, draw_renamed};
use overlays::{draw_help, draw_picker, draw_tree, lesson_panel};
use preview::draw_preview;
use status::draw_status;
use welcome::draw_welcome;

#[cfg(test)]
mod tests;

const TREE_W: u16 = 30;

pub fn draw(frame: &mut Frame, app: &mut App, theme: &Theme) {
    let area = frame.area();
    app.cursor_cell = None;
    app.diagrams.begin();
    app.diagrams.theme(theme, theme.line_hl_dim);
    let base = Style::new().bg(theme.bg).fg(theme.fg);
    frame.render_widget(Block::new().style(base), area);

    let panel = lesson_panel(app, theme, area.width, base);
    let lesson_h = panel.as_ref().map_or(0, |p| {
        u16::try_from(p.line_count(area.width))
            .unwrap_or(u16::MAX)
            .max(3)
    });
    let [main, lesson, status] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(lesson_h),
        Constraint::Length(1),
    ])
    .areas(area);
    if app.focus != Focus::Tree {
        app.tree_width = None;
    }
    let half = (main.width / 2).max(TREE_W);
    let tree_w = match (app.show_tree, app.tree_width) {
        (false, _) => 0,
        (true, Some(w)) => w.min(half),
        (true, None) => TREE_W,
    };
    let [tree, code] =
        Layout::horizontal([Constraint::Length(tree_w), Constraint::Min(1)]).areas(main);

    let need = if app.show_tree {
        draw_tree(frame, app, theme, tree, base)
    } else {
        None
    };
    if app.show_tree && app.focus == Focus::Tree && app.tree_width.is_none() {
        let w = need.map_or(TREE_W, |n| {
            (u16::try_from(n + 2).unwrap_or(u16::MAX)).clamp(TREE_W, half)
        });
        app.tree_width = Some(w);
        if w > TREE_W {
            frame.render_widget(ratatui::widgets::Clear, frame.area());
            return draw(frame, app, theme);
        }
    }
    let mut pane = |app: &mut App| draw_pane(frame, app, theme, code, base);
    if app.tree_width.is_some_and(|w| w > TREE_W) {
        app.drawn_aside(pane);
    } else {
        pane(app);
    }
    if let Some(panel) = panel {
        frame.render_widget(panel, lesson);
    }
    draw_status(frame, app, theme, status);
    draw_overlays(frame, app, theme, (area, main), base);
    force_emoji_widths(frame);
    if let Some(cell) = app
        .cursor_cell
        .and_then(|at| frame.buffer_mut().cell_mut(at))
    {
        cell.set_style(if matches!(app.mode, Mode::Normal | Mode::Help) {
            theme.cursor
        } else {
            theme.typing_cursor
        });
    }
}

fn draw_pane(frame: &mut Frame, app: &mut App, theme: &Theme, code: Rect, base: Style) {
    app.preview_fresh();
    if let Some(sides) = app.picture_sides() {
        picture::draw_review(frame, app, code, &sides, theme.ghost_fg);
    } else if let Some(path) = app.picture_shown() {
        picture::draw_file(frame, app, code, &path);
    } else if app.previewing() && !app.buf.path.as_deref().is_some_and(crate::picture::is_svg) {
        draw_preview(frame, app, theme, code, base);
    } else if app.folded_here().is_some() {
        fold::draw_fold(frame, app, theme, code, base);
    } else if let Some(note) = app.empty_note() {
        draw_empty(frame, theme, code, base, note);
    } else if let Some((old, new)) = app.renamed_here() {
        draw_renamed(frame, theme, code, base, old, new);
    } else if app.buf.binary() {
        let note = app
            .buf
            .path
            .as_deref()
            .and_then(crate::picture::not_shown_note);
        draw_binary(frame, theme, code, base, note);
    } else if app.buf.path.is_some() {
        let code = code::draw_rename_band(frame, app, theme, code, base);
        app.cursor_cell = draw_code(frame, app, theme, code, base);
    } else {
        draw_welcome(frame, theme, code, base);
    }
}

fn draw_overlays(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    (area, main): (Rect, Rect),
    base: Style,
) {
    let over = if app.tutor.is_some() { main } else { area };
    if app.picker.is_some() {
        let home = main.width.saturating_sub(TREE_W);
        let (list, share) = if app.show_tree && home >= 80 {
            (
                Rect {
                    x: main.x + TREE_W,
                    width: home,
                    ..over
                },
                90,
            )
        } else {
            (over, 80)
        };
        draw_picker(frame, app, theme, list, share, base);
    }
    if app.mode == Mode::Help {
        draw_help(frame, app, theme, over, base);
    }
}

fn force_emoji_widths(frame: &mut Frame) {
    for cell in &mut frame.buffer_mut().content {
        if cell.symbol().contains('\u{fe0f}')
            && let Some(w) = NonZeroU16::new(cell.symbol().cell_width()).filter(|w| w.get() > 1)
        {
            cell.set_diff_option(CellDiffOption::ForcedWidth(w));
        }
    }
}

pub(super) fn covers_code(app: &App) -> bool {
    app.picker.is_some() || app.mode == Mode::Help
}

pub(super) fn expand(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains(|c| c == '\t' || wrap::hidden(c)) {
        return s.into();
    }
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '\t' => out.push_str(crate::buffer::TAB),
            c if wrap::hidden(c) => out.push_str(&wrap::tag(c)),
            c => out.push(c),
        }
    }
    out.into()
}

pub(super) fn tagged<'a>(out: &mut Vec<Span<'a>>, s: &'a str, style: Style, tag: Style) {
    let mut rest = s;
    while let Some((i, c)) = rest.char_indices().find(|&(_, c)| wrap::hidden(c)) {
        let (before, after) = rest.split_at(i);
        if !before.is_empty() {
            out.push(Span::styled(expand(before), style));
        }
        out.push(Span::styled(wrap::tag(c), tag));
        let mut tail = after.chars();
        tail.next();
        rest = tail.as_str();
    }
    if !rest.is_empty() || rest.len() == s.len() {
        out.push(Span::styled(expand(rest), style));
    }
}
