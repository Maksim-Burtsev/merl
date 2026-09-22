//! The status bar and the prompt line it shares with the picker.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, Focus, Mode};
use crate::line_edit::LineEdit;
use crate::theme::Theme;
use crate::wrap;

/// A prompt line: the prefix, the text with its selection reversed, the cursor where it stands.
pub(super) fn draw_prompt(
    frame: &mut Frame,
    prefix: &str,
    edit: &LineEdit,
    style: Style,
    area: Rect,
) {
    let sel = edit.selection().unwrap_or(0..0);
    let line = Line::from(vec![
        Span::raw(format!("{prefix}{}", &edit[..sel.start])),
        Span::styled(&edit[sel.clone()], style.add_modifier(Modifier::REVERSED)),
        Span::raw(&edit[sel.end..]),
    ]);
    let x = wrap::width(prefix) + wrap::width(&edit[..edit.cursor()]);
    frame.set_cursor_position((area.x + x as u16, area.y));
    frame.render_widget(Paragraph::new(line).style(style), area);
}

pub(super) fn draw_status(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let style = Style::new().bg(theme.status_bg).fg(theme.status_fg);
    let prefix = match app.mode {
        Mode::Goto => ":",
        Mode::Find => "/",
        Mode::New => "new: ",
        _ => "",
    };
    if !prefix.is_empty() {
        draw_prompt(frame, prefix, &app.prompt, style, area);
        // What the query found so far (`3/17`, `no match`), out of the way of the typing.
        let w = wrap::width(&app.message) as u16 + 1;
        let typed = wrap::width(prefix) + wrap::width(&app.prompt) + 2;
        if !app.message.is_empty() && area.width as usize > typed + w as usize {
            let right = Rect {
                x: area.right() - w,
                width: w,
                ..area
            };
            frame.render_widget(Paragraph::new(app.message.as_str()).style(style), right);
        }
        return;
    }
    let pane = match (app.mode, app.focus) {
        (Mode::Edit, _) => "edit",
        (_, Focus::Tree) => "tree",
        _ => "code",
    };
    let mut spans = vec![
        Span::styled(
            app.rel_path(),
            style.fg(theme.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "{}  {}:{}  [{pane}]{}{}{}{}",
                if app.dirty { " \u{25cf}" } else { "" },
                app.line + 1,
                app.display_col(),
                if app.mode == Mode::Edit {
                    if app.buf.tabs { "  Tab" } else { "  Spaces: 4" }
                } else {
                    ""
                },
                // Enter on such a file says why (`read-only: not UTF-8`): once is enough.
                if app.buf.readonly.is_some() && !app.message.starts_with("read-only") {
                    "  read-only"
                } else {
                    ""
                },
                if app.no_watch { "  no auto-reload" } else { "" },
                if app.nowrap() { "  nowrap" } else { "" }
            ),
            style,
        ),
    ];
    if app.conflict {
        spans.push(Span::styled(
            "  changed on disk: Ctrl+S overwrites, Ctrl+R reloads",
            style.fg(theme.accent),
        ));
    }
    if let Some(review) = app.review_status() {
        spans.push(Span::styled(format!("  {review}"), style));
    }
    if !app.message.is_empty() {
        spans.push(Span::styled(format!("  {}", app.message), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)).style(style), area);
    // With nothing to report, the right edge keeps `?` discoverable.
    const HINT: &str = "? help ";
    if app.message.is_empty() && area.width as usize > HINT.len() {
        let hint = Rect {
            x: area.right() - HINT.len() as u16,
            width: HINT.len() as u16,
            ..area
        };
        frame.render_widget(Paragraph::new(HINT).style(style), hint);
    }
}
