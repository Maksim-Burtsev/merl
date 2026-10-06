//! Drawing. Reads `App`, writes only the viewport size back into it.

use std::num::NonZeroU16;

use ratatui::Frame;
use ratatui::buffer::{CellDiffOption, CellWidth};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::Block;

use crate::app::{App, Mode};
use crate::theme::Theme;
use crate::wrap;

mod code;
mod fold;
mod overlays;
mod preview;
mod status;
mod welcome;

use code::{draw_binary, draw_code};
use overlays::{draw_help, draw_picker, draw_tree, lesson_panel};
use preview::draw_preview;
use status::draw_status;
use welcome::draw_welcome;

#[cfg(test)]
mod tests;

const TREE_W: u16 = 30;

pub fn draw(frame: &mut Frame, app: &mut App, theme: &Theme) {
    let area = frame.area();
    app.diagrams.want.clear();
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
    let tree_w = if app.show_tree { TREE_W } else { 0 };
    let [tree, code] =
        Layout::horizontal([Constraint::Length(tree_w), Constraint::Min(1)]).areas(main);

    if app.show_tree {
        draw_tree(frame, app, theme, tree, base);
    }
    if app.previewing() {
        draw_preview(frame, app, theme, code, base);
    } else if app.folded_here().is_some() {
        fold::draw_fold(frame, app, theme, code, base);
    } else if app.buf.binary() {
        draw_binary(frame, theme, code, base);
    } else if app.buf.path.is_some() {
        draw_code(frame, app, theme, code, base);
    } else {
        draw_welcome(frame, theme, code, base);
    }
    if let Some(panel) = panel {
        frame.render_widget(panel, lesson);
    }
    draw_status(frame, app, theme, status);
    // Over `--tutor`'s and `--drill`'s panel an overlay would hide the task it is part of: a
    // task can start with the help open. The panel and the status bar stay in sight.
    let over = if app.tutor.is_some() { main } else { area };
    if app.picker.is_some() {
        let (list, share) = if app.show_tree && code.width >= 80 {
            (
                Rect {
                    x: code.x,
                    width: code.width,
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
    if app.picker.is_some() || app.mode == Mode::Help {
        app.diagrams.want.clear();
    }
    // ratatui/ratatui#2651 in ratatui-crossterm 0.1.2: the diff sends the second column of an
    // emoji with U+FE0F as a cell of its own, and the backend prints it right after the emoji
    // without a move, so the rest of the row lands a column late. A forced width keeps the diff
    // off that column, and the backend moves the cursor past it. The upstream fix (#2721) moves
    // back to that column instead, and tmux erases the emoji there, so this outlives the bump.
    for cell in &mut frame.buffer_mut().content {
        if cell.symbol().contains('\u{fe0f}')
            && let Some(w) = NonZeroU16::new(cell.symbol().cell_width()).filter(|w| w.get() > 1)
        {
            cell.set_diff_option(CellDiffOption::ForcedWidth(w));
        }
    }
}

/// Tabs drawn as [`crate::buffer::TAB`] and hidden chars as their [`wrap::tag`], as wide as
/// [`wrap::width`] counts them; a piece with neither is borrowed as it is.
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
    let mut pos = 0;
    for (i, c) in s.char_indices().filter(|&(_, c)| wrap::hidden(c)) {
        if pos < i {
            out.push(Span::styled(expand(&s[pos..i]), style));
        }
        out.push(Span::styled(wrap::tag(c), tag));
        pos = i + c.len_utf8();
    }
    if pos < s.len() || pos == 0 {
        out.push(Span::styled(expand(&s[pos..]), style));
    }
}
