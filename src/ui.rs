//! Drawing. Reads `App`, writes only the viewport size back into it.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::{App, Mode};
use crate::theme::Theme;
use crate::wrap;

/// Width of the (still empty) file tree pane.
const TREE_W: u16 = 30;
/// ponytail: a single line longer than this is truncated for rendering only.
const MAX_RENDER_BYTES: usize = 20_000;

pub fn draw(frame: &mut Frame, app: &mut App, theme: &Theme) {
    let area = frame.area();
    let base = Style::new().bg(theme.bg).fg(theme.fg);
    frame.render_widget(Block::new().style(base), area);

    let [main, status] = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);
    let [tree, code] =
        Layout::horizontal([Constraint::Length(TREE_W), Constraint::Min(1)]).areas(main);

    // Step 3 fills this; the border is here now so the layout never shifts again.
    frame.render_widget(Block::bordered().title(app.root_name()).style(base), tree);

    draw_code(frame, app, theme, code, base);
    draw_status(frame, app, theme, status);
}

fn draw_code(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, base: Style) {
    let gutter_w = digits(app.buf.lines.len()) + 1;
    app.view_w = (area.width as usize).saturating_sub(gutter_w).max(1);
    app.view_h = area.height as usize;
    app.clamp_scroll();

    let gutter_style = base.fg(theme.gutter_fg);
    let hl = base.bg(theme.line_hl);
    let hl_gutter = gutter_style.bg(theme.line_hl);

    let mut lines: Vec<Line> = Vec::with_capacity(area.height as usize);
    let mut l = app.top_line;
    let mut skip = app.top_row;
    while lines.len() < area.height as usize && l < app.buf.lines.len() {
        let text = &app.buf.lines[l];
        let clipped = &text[..floor_boundary(text, MAX_RENDER_BYTES)];
        let cursor_line = l == app.line;
        let (g, t) = if cursor_line {
            (hl_gutter, hl)
        } else {
            (gutter_style, base)
        };
        for (i, r) in wrap::wrap_line(clipped, app.view_w).into_iter().enumerate() {
            if i < skip {
                continue;
            }
            if lines.len() == area.height as usize {
                break;
            }
            let num = if i == 0 {
                format!("{:>w$} ", l + 1, w = gutter_w - 1)
            } else {
                " ".repeat(gutter_w)
            };
            let mut body = clipped[r].to_string();
            if cursor_line {
                // Pad so the cursor-line background reaches the right edge of the pane.
                body.push_str(&" ".repeat(app.view_w.saturating_sub(wrap::width(&body))));
            }
            lines.push(Line::from(vec![
                Span::styled(num, g),
                Span::styled(body, t),
            ]));
        }
        skip = 0;
        l += 1;
    }
    frame.render_widget(Paragraph::new(lines).style(base), area);

    let screen_row = rows_between(
        app,
        (app.top_line, app.top_row),
        (app.line, app.cursor_row()),
    );
    if let Some(y) = screen_row.filter(|y| *y < area.height as usize)
        && app.mode == Mode::Normal
    {
        let x = area.x + (gutter_w + app.cursor_x()) as u16;
        frame.set_cursor_position((x.min(area.right().saturating_sub(1)), area.y + y as u16));
    }
}

fn draw_status(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let style = Style::new().bg(theme.status_bg).fg(theme.status_fg);
    if app.mode == Mode::Goto {
        let text = format!(":{}", app.goto);
        frame.set_cursor_position((area.x + text.chars().count() as u16, area.y));
        frame.render_widget(Paragraph::new(text).style(style), area);
        return;
    }
    let mut spans = vec![
        Span::styled(app.rel_path(), style.add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("  {}:{}  [code]", app.line + 1, app.display_col()),
            style,
        ),
    ];
    if !app.message.is_empty() {
        spans.push(Span::styled(format!("  {}", app.message), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)).style(style), area);
}

fn digits(n: usize) -> usize {
    n.max(1).ilog10() as usize + 1
}

/// Largest byte index <= `max` that is a char boundary of `s`.
fn floor_boundary(s: &str, max: usize) -> usize {
    if s.len() <= max {
        return s.len();
    }
    let mut i = max;
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
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
        if r >= app.rows(l).len() {
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

#[cfg(test)]
mod tests {
    #[test]
    fn gutter_width() {
        assert_eq!(super::digits(0), 1);
        assert_eq!(super::digits(9), 1);
        assert_eq!(super::digits(10), 2);
        assert_eq!(super::digits(999), 3);
        assert_eq!(super::digits(1000), 4);
    }
}
