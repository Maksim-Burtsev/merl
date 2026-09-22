//! Drawing. Reads `App`, writes only the viewport size back into it.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::Block;

use crate::app::{App, Mode};
use crate::theme::Theme;

mod code;
mod overlays;
mod status;
mod welcome;

use code::draw_code;
use overlays::{draw_help, draw_lesson, draw_picker, draw_tree};
use status::draw_status;
use welcome::draw_welcome;

#[cfg(test)]
mod tests;

/// Width of the file tree pane.
const TREE_W: u16 = 30;

pub fn draw(frame: &mut Frame, app: &mut App, theme: &Theme) {
    let area = frame.area();
    let base = Style::new().bg(theme.bg).fg(theme.fg);
    frame.render_widget(Block::new().style(base), area);

    // The lesson panel is 0 rows tall outside `--tutor`, so nothing else moves.
    // PROTOTYPE (#194): the drill's three looks.
    use crate::drill_prototype::{self as drill, Variant};
    let variant = app.drill.as_ref().map(|d| d.variant);
    let lesson_h = match (&app.drill, variant) {
        _ if app.tutor.is_some() => 3,
        (Some(d), Some(Variant::A)) => drill::panel_h(d),
        _ => 0,
    };
    let top_h = u16::from(variant == Some(Variant::B));
    let side_w = if variant == Some(Variant::C) { 34 } else { 0 };
    let [top, main, lesson, status] = Layout::vertical([
        Constraint::Length(top_h),
        Constraint::Min(1),
        Constraint::Length(lesson_h),
        Constraint::Length(1),
    ])
    .areas(area);
    let tree_w = if app.show_tree { TREE_W } else { 0 };
    let [tree, code, side] = Layout::horizontal([
        Constraint::Length(tree_w),
        Constraint::Min(1),
        Constraint::Length(side_w),
    ])
    .areas(main);

    if app.show_tree {
        draw_tree(frame, app, theme, tree, base);
    }
    if app.buf.path.is_some() {
        draw_code(frame, app, theme, code, base);
    } else {
        draw_welcome(frame, theme, code, base);
    }
    if app.tutor.is_some() {
        draw_lesson(frame, app, theme, lesson, base);
    }
    if let Some(d) = &app.drill {
        match d.variant {
            Variant::A => drill::draw_panel(frame, d, theme, lesson, base),
            Variant::B => drill::draw_strip(frame, d, theme, top),
            Variant::C => drill::draw_side(frame, d, theme, side, base),
        }
    }
    draw_status(frame, app, theme, status);
    if let Some(d) = app.drill.as_ref().filter(|d| d.over && d.variant == Variant::C) {
        drill::draw_card(frame, d, theme, main, base);
    }
    // C's log stays in view beside the overlays.
    let area = Rect {
        width: area.width - side.width,
        ..area
    };
    if app.picker.is_some() {
        draw_picker(frame, app, theme, area, base);
    }
    if app.mode == Mode::Help {
        draw_help(frame, app, theme, area, base);
    }
}

/// Tabs drawn as [`crate::buffer::TAB`]; a tab-free piece is borrowed as it is.
pub(super) fn expand(s: &str) -> std::borrow::Cow<'_, str> {
    if s.contains('\t') {
        s.replace('\t', crate::buffer::TAB).into()
    } else {
        s.into()
    }
}
