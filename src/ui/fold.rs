use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::App;
use crate::theme::Theme;

/// The fold of the open file, as GitHub and GitLab fold a generated file's diff: a box in the
/// accent, centred where the binary file's note sits, ` generated file ` in its border; inside
/// the file, its hunks and counts, the Enter button that loads the diff, and `c` under it.
pub(super) fn draw_fold(frame: &mut Frame, app: &App, theme: &Theme, area: Rect, base: Style) {
    let Some(f) = app.folded_here() else {
        return;
    };
    let name = f
        .path
        .file_name()
        .map_or_else(|| f.path.to_string_lossy(), |n| n.to_string_lossy());
    // A deleted file has no hunks, only lines gone.
    let size = match f.status {
        'D' => "deleted".to_string(),
        _ => {
            let n = app.diff.hunks.len();
            format!("{n} hunk{}", if n == 1 { "" } else { "s" })
        }
    };
    let bold = base.add_modifier(Modifier::BOLD);
    let dim = base.fg(theme.ghost_fg);
    let accent = base.fg(theme.accent);
    let button = base.bg(theme.accent).fg(theme.bg);
    let lines = vec![
        Line::styled(name.into_owned(), bold),
        Line::styled(format!("{size}  +{} \u{2212}{}", f.added, f.deleted), dim),
        Line::default(),
        Line::from(vec![
            Span::styled(" Enter", button.add_modifier(Modifier::BOLD)),
            Span::styled("  Load diff ", button),
        ]),
        Line::default(),
        Line::from(vec![
            Span::styled("c", bold),
            Span::styled("  next file", dim),
        ]),
    ];
    // Room either side of the widest line, a row above and below the lines.
    let w = lines.iter().map(Line::width).max().unwrap_or(0) as u16 + 14;
    let h = lines.len() as u16 + 4;
    let [_, b] = Layout::vertical([
        Constraint::Length(area.height.saturating_sub(h) * 2 / 5),
        Constraint::Length(h.min(area.height)),
    ])
    .areas(area);
    let [b] = Layout::horizontal([Constraint::Length(w.min(area.width))])
        .flex(Flex::Center)
        .areas(b);
    let title = Line::styled(" generated file ", accent.add_modifier(Modifier::BOLD));
    let block = Block::bordered()
        .border_style(accent)
        .style(base)
        .title(title.centered());
    let inner = block.inner(b);
    frame.render_widget(block, b);
    let [_, inner, _] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(inner);
    frame.render_widget(
        Paragraph::new(lines)
            .style(base)
            .alignment(Alignment::Center),
        inner,
    );
}
