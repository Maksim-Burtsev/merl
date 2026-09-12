//! Drawing. Reads `App`, writes only the viewport size back into it.

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::app::{App, Focus, Mode};
use crate::theme::Theme;
use crate::wrap;

/// Width of the file tree pane.
const TREE_W: u16 = 30;
/// ponytail: a single line longer than this is truncated for rendering only.
const MAX_RENDER_BYTES: usize = 20_000;

pub fn draw(frame: &mut Frame, app: &mut App, theme: &Theme) {
    let area = frame.area();
    let base = Style::new().bg(theme.bg).fg(theme.fg);
    frame.render_widget(Block::new().style(base), area);

    let [main, status] = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);
    let tree_w = if app.show_tree { TREE_W } else { 0 };
    let [tree, code] =
        Layout::horizontal([Constraint::Length(tree_w), Constraint::Min(1)]).areas(main);

    if app.show_tree {
        draw_tree(frame, app, theme, tree, base);
    }
    if app.buf.path.is_some() {
        draw_code(frame, app, theme, code, base);
    } else {
        let hint = Line::from(Span::styled(
            " o: open file   ?: help",
            base.fg(theme.gutter_fg),
        ));
        frame.render_widget(Paragraph::new(hint).style(base), code);
    }
    draw_status(frame, app, theme, status);
    if app.picker.is_some() {
        draw_picker(frame, app, theme, area, base);
    }
}

fn draw_tree(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, base: Style) {
    let block = Block::bordered().title(app.root_name()).style(base);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let focused = app.focus == Focus::Tree;
    let visible = app.tree.visible();
    let height = inner.height as usize;
    let at = visible
        .iter()
        .position(|&i| i == app.tree.cursor)
        .unwrap_or(0);
    // Scroll the minimum amount that keeps the cursor row on screen.
    app.tree_top = app.tree_top.min(at).max((at + 1).saturating_sub(height));

    let width = inner.width as usize;
    let rows: Vec<Line> = visible
        .iter()
        .skip(app.tree_top)
        .take(height)
        .map(|&i| {
            let n = &app.tree.nodes[i];
            let marker = match (n.is_dir, n.expanded) {
                (true, true) => "\u{25be} ",
                (true, false) => "\u{25b8} ",
                (false, _) => "  ",
            };
            let text = format!("{}{marker}{}", "  ".repeat(n.depth), n.name());
            let style = if i != app.tree.cursor {
                base
            } else if focused {
                base.bg(theme.line_hl)
            } else {
                // The tree keeps its place while the code pane has the keys, just dimmed.
                base.bg(theme.line_hl).fg(theme.gutter_fg)
            };
            let pad = width.saturating_sub(wrap::width(&text));
            Line::from(vec![
                Span::styled(text, style),
                Span::styled(" ".repeat(pad), style),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(rows).style(base), inner);
}

fn draw_picker(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, base: Style) {
    let Some(picker) = &mut app.picker else {
        return;
    };
    let [area] = Layout::horizontal([Constraint::Percentage(80)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Percentage(60)])
        .flex(Flex::Center)
        .areas(area);

    let (matched, total) = picker.counts();
    let block = Block::bordered()
        .title(format!("{} ({matched}/{total})", picker.title))
        .style(base);
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }

    let [prompt, list] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    frame.render_widget(
        Paragraph::new(format!("> {}", picker.query)).style(base),
        prompt,
    );
    frame.set_cursor_position((prompt.x + 2 + wrap::width(&picker.query) as u16, prompt.y));

    let (rows, selected) = picker.window(list.height as usize);
    let width = list.width as usize;
    let lines: Vec<Line> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let style = if i == selected {
                base.bg(theme.line_hl)
            } else {
                base
            };
            let mut spans: Vec<Span> = row
                .label
                .chars()
                .enumerate()
                .map(|(c, ch)| {
                    let matched = row.matched.binary_search(&(c as u32)).is_ok();
                    let style = if matched {
                        style.add_modifier(Modifier::BOLD)
                    } else {
                        style
                    };
                    Span::styled(ch.to_string(), style)
                })
                .collect();
            spans.push(Span::styled(
                " ".repeat(width.saturating_sub(wrap::width(&row.label))),
                style,
            ));
            Line::from(spans)
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).style(base), list);
}

fn draw_code(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, base: Style) {
    let gutter_w = digits(app.buf.lines.len()) + 1;
    app.view_w = (area.width as usize).saturating_sub(gutter_w).max(1);
    app.view_h = area.height as usize;
    app.clamp_scroll();
    // Wrapping only ever costs rows, so this covers every visible line.
    app.buf.highlight_to(app.top_line + app.view_h, theme);

    let gutter_style = base.fg(theme.gutter_fg);
    let hl = base.bg(theme.line_hl);
    let hl_gutter = gutter_style.bg(theme.line_hl);

    let mut lines: Vec<Line> = Vec::with_capacity(area.height as usize);
    let mut l = app.top_line;
    let mut skip = app.top_row;
    while lines.len() < area.height as usize && l < app.buf.lines.len() {
        let text = &app.buf.lines[l];
        let clipped = &text[..floor_boundary(text, MAX_RENDER_BYTES)];
        let spans = app.buf.hl.get(l).map_or(&[][..], Vec::as_slice);
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
            let mut row = vec![Span::styled(num, g)];
            let pad = app.view_w.saturating_sub(wrap::width(&clipped[r.clone()]));
            row.extend(row_spans(clipped, spans, &r, t));
            if cursor_line {
                // Pad so the cursor-line background reaches the right edge of the pane.
                row.push(Span::styled(" ".repeat(pad), t));
            }
            lines.push(Line::from(row));
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
            format!(
                "  {}:{}  [{}]",
                app.line + 1,
                app.display_col(),
                if app.focus == Focus::Tree {
                    "tree"
                } else {
                    "code"
                }
            ),
            style,
        ),
    ];
    if !app.message.is_empty() {
        spans.push(Span::styled(format!("  {}", app.message), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)).style(style), area);
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
            out.push(Span::styled(&text[pos..start], base));
        }
        out.push(Span::styled(&text[start..end], base.patch(*style)));
        pos = end;
    }
    if pos < r.end || out.is_empty() {
        out.push(Span::styled(&text[pos..r.end], base));
    }
    out
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
    use std::path::PathBuf;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use crate::app::{App, PickerKind};
    use crate::buffer::Buffer;
    use crate::tree::Tree;

    #[test]
    fn picker_overlay_renders_border_prompt_and_items() {
        let files = ["src/app.rs", "src/wrap.rs"].map(PathBuf::from).to_vec();
        let mut app = App::new(
            PathBuf::from("/demo"),
            Tree::default(),
            files,
            Buffer::empty(),
            None,
        );
        app.open_picker(PickerKind::Files);
        app.picker.as_mut().unwrap().settle();

        let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();

        let buf = terminal.backend().buffer();
        let text: Vec<String> = (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect();
        assert_eq!(text, SNAPSHOT);
    }

    /// The tree pane and the status bar frame the overlay; the overlay itself is the border,
    /// the `>` prompt and the two items.
    #[rustfmt::skip]
    const SNAPSHOT: [&str; 12] = [
        "┌demo────────────────────────┐ o: open file   ?: help",
        "│                            │",
        "│     ┌Files (2/2)───────────────────────────────────┐",
        "│     │>                                             │",
        "│     │src/app.rs                                    │",
        "│     │src/wrap.rs                                   │",
        "│     │                                              │",
        "│     │                                              │",
        "│     │                                              │",
        "│     └──────────────────────────────────────────────┘",
        "└────────────────────────────┘",
        "demo/  1:1  [tree]",
    ];

    #[test]
    fn gutter_width() {
        assert_eq!(super::digits(0), 1);
        assert_eq!(super::digits(9), 1);
        assert_eq!(super::digits(10), 2);
        assert_eq!(super::digits(999), 3);
        assert_eq!(super::digits(1000), 4);
    }
}
