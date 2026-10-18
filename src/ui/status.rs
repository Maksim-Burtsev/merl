use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, Focus, Mode};
use crate::line_edit::LineEdit;
use crate::theme::Theme;
use crate::wrap;

use super::picture::Status;

#[expect(
    clippy::string_slice,
    reason = "LineEdit keeps its cursor and selection on char boundaries"
)]
pub(super) fn draw_prompt(
    frame: &mut Frame,
    prefix: &str,
    edit: &LineEdit,
    style: Style,
    area: Rect,
) -> Position {
    let sel = edit.selection().unwrap_or(0..0);
    let line = Line::from(vec![
        Span::raw(format!("{prefix}{}", &edit[..sel.start])),
        Span::styled(&edit[sel.clone()], style.add_modifier(Modifier::REVERSED)),
        Span::raw(&edit[sel.end..]),
    ]);
    let x = wrap::width(prefix) + wrap::width(&edit[..edit.cursor()]);
    frame.render_widget(Paragraph::new(line).style(style), area);
    let x = area.x.saturating_add(u16::try_from(x).unwrap_or(u16::MAX));
    Position::new(x.min(area.right().saturating_sub(1)), area.y)
}

pub(super) fn draw_status(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let style = Style::new()
        .bg(theme.status_bg)
        .fg(theme.status_fg)
        .add_modifier(theme.modifiers(&["StatusLine"]));
    let prefix = match app.mode {
        Mode::Goto => ":",
        Mode::Find => "/",
        Mode::New => "new: ",
        _ => "",
    };
    if !prefix.is_empty() {
        app.cursor_cell = Some(draw_typed(frame, app, prefix, style, area));
        return;
    }
    let pane = match (app.mode, app.focus) {
        (Mode::Edit | Mode::Note, _) => "edit",
        (_, Focus::Tree) => "tree",
        _ if app.previewing() => "preview",
        _ if app.preview_pending() => "parsing",
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
            if theme.groups.is_empty() {
                style.fg(theme.accent)
            } else {
                style
            }
            .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "{}  {}[{pane}]{}{}{}{}",
                if app.dirty { " \u{25cf}" } else { "" },
                position(app, size),
                if app.mode == Mode::Edit {
                    if app.buf.tabs { "  Tab" } else { "  Spaces: 4" }
                } else {
                    ""
                },
                why_note(app, picture.as_ref()),
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
    let rest = spans[1..]
        .iter()
        .map(|s| wrap::width(&s.content))
        .sum::<usize>();
    let clock = app
        .clock
        .as_deref()
        .filter(|t| !t.is_empty())
        .map(|t| (t, wrap::width(t) + 2))
        .filter(|(_, w)| wrap::width(&spans[0].content) + rest + 1 + w <= area.width as usize);
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
        spans.last_mut().expect("the bar has spans").content = format!("  {name}{why}").into();
    }
    frame.render_widget(Paragraph::new(Line::from(spans)).style(style), area);
    if let Some((time, w)) = clock {
        draw_clock(frame, time, w, style, area);
    }
}

fn draw_clock(frame: &mut Frame, time: &str, w: usize, style: Style, area: Rect) {
    let w = u16::try_from(w).unwrap_or(u16::MAX);
    let at = Rect {
        x: area.right().saturating_sub(w),
        width: w.min(area.width),
        ..area
    };
    frame.render_widget(Paragraph::new(format!(" {time} ")).style(style), at);
}

fn draw_typed(frame: &mut Frame, app: &App, prefix: &str, style: Style, area: Rect) -> Position {
    let cursor = draw_prompt(frame, prefix, &app.prompt, style, area);
    let found_w = u16::try_from(wrap::width(&app.message)).unwrap_or(u16::MAX) + 1;
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
    cursor
}

fn position(app: &App, size: Option<&String>) -> String {
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
    }
}

fn why_note(app: &App, picture: Option<&Status>) -> String {
    match picture {
        Some(Status::Why(why)) => format!("  {why}"),
        None if app.buf.readonly.is_some() && !app.message.starts_with("read-only") => {
            "  read-only".into()
        }
        _ => String::new(),
    }
}

pub(super) fn fit_path(path: &str, room: usize) -> String {
    if wrap::width(path) <= room {
        return path.to_owned();
    }
    let mut tails = path
        .trim_end_matches('/')
        .match_indices('/')
        .filter_map(|(i, _)| path.get(i..))
        .map(|tail| format!("\u{2026}{tail}"));
    let last = tails.next_back();
    tails
        .find(|t| wrap::width(t) <= room)
        .or(last)
        .unwrap_or_else(|| path.to_owned())
}
