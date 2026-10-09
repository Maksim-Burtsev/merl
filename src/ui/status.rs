use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, Focus, Mode};
use crate::line_edit::LineEdit;
use crate::theme::Theme;
use crate::wrap;

use super::picture::Status;

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
        let found_w = wrap::width(&app.message) as u16 + 1;
        let typed = wrap::width(prefix) + wrap::width(&app.prompt) + 2;
        if !app.message.is_empty() && area.width as usize > typed + found_w as usize {
            let found_at_right_edge = Rect {
                x: area.right() - found_w,
                width: found_w,
                ..area
            };
            frame.render_widget(
                Paragraph::new(app.message.as_str()).style(style),
                found_at_right_edge,
            );
        }
        return;
    }
    let pane = match (app.mode, app.focus) {
        (Mode::Edit, _) => "edit",
        (_, Focus::Tree) => "tree",
        _ if app.previewing() => "preview",
        _ => "code",
    };
    let picture = super::picture::status(app);
    let size = match &picture {
        Some(Status::Size(size)) => Some(size),
        _ => None,
    };
    let mut spans = vec![
        Span::styled(
            app.rel_path(),
            style.fg(theme.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "{}  {}[{pane}]{}{}{}{}",
                if app.dirty { " \u{25cf}" } else { "" },
                match app.deleted {
                    _ if app.buf.path.is_none() || app.folded_here().is_some() => String::new(),
                    _ if size.is_some() => size
                        .filter(|s| !s.is_empty())
                        .map_or(String::new(), |s| format!("{s}  ")),
                    Some((k, i)) => {
                        let from = app.diff.ghost_from.get(&k).copied().unwrap_or(0);
                        format!("-{}:{}  ", from + i + 1, app.display_col())
                    }
                    None => format!("{}:{}  ", app.line + 1, app.display_col()),
                },
                if app.mode == Mode::Edit {
                    if app.buf.tabs { "  Tab" } else { "  Spaces: 4" }
                } else {
                    ""
                },
                // Enter on such a file says why (`read-only: not UTF-8`): once is enough.
                match &picture {
                    Some(Status::Size(_)) => String::new(),
                    Some(Status::Why(why)) => format!("  {why}"),
                    None if app.buf.readonly.is_some() && !app.message.starts_with("read-only") => {
                        "  read-only".into()
                    }
                    None => String::new(),
                },
                if app.no_watch { "  no auto-reload" } else { "" },
                if app.nowrap() && !app.previewing() {
                    "  nowrap"
                } else {
                    ""
                }
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
    // With nothing to report, the right edge keeps `?` discoverable.
    const HINT: &str = "? help ";
    let hint = app.message.is_empty() && area.width as usize > HINT.len();
    let rest = spans[1..]
        .iter()
        .map(|s| wrap::width(&s.content))
        .sum::<usize>()
        + if hint { HINT.len() + 2 } else { 0 };
    spans[0].content = fit_path(
        &spans[0].content,
        (area.width as usize).saturating_sub(rest),
    )
    .into();
    if let Some((_, len)) = app.message_path.as_ref().filter(|(m, _)| *m == app.message) {
        let before = spans[..spans.len() - 1]
            .iter()
            .map(|s| wrap::width(&s.content))
            .sum::<usize>();
        let room = (area.width as usize).saturating_sub(before + 2);
        let (name, why) = app.message.split_at(*len);
        let name = fit_path(name, room.saturating_sub(wrap::width(why)));
        spans.last_mut().unwrap().content = format!("  {name}{why}").into();
    }
    frame.render_widget(Paragraph::new(Line::from(spans)).style(style), area);
    if hint {
        let hint = Rect {
            x: area.right() - HINT.len() as u16,
            width: HINT.len() as u16,
            ..area
        };
        frame.render_widget(Paragraph::new(HINT).style(style), hint);
    }
}

/// `path` in `room` columns: cut from the left at a `/` when it is wider, `…/json/__init__.py`,
/// but never into the file's name, however little room there is.
pub(super) fn fit_path(path: &str, room: usize) -> String {
    if wrap::width(path) <= room {
        return path.to_owned();
    }
    let mut tails = path
        .trim_end_matches('/')
        .match_indices('/')
        .map(|(i, _)| format!("\u{2026}{}", &path[i..]));
    let last = tails.next_back();
    tails
        .find(|t| wrap::width(t) <= room)
        .or(last)
        .unwrap_or_else(|| path.to_owned())
}
