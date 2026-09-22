//! The welcome screen: no file open, the logo and the keys that work without one.

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::theme::Theme;
use crate::wrap;

/// The blues of `assets/icon.png`: beak, body, background. The logo fades through them top
/// to bottom.
pub(super) const LOGO_COLORS: [Color; 3] = [
    Color::Rgb(0x6c, 0x94, 0xfc),
    Color::Rgb(0x4e, 0x72, 0xfc),
    Color::Rgb(0x24, 0x39, 0xb5),
];

/// `merl` in a block font, six rows.
pub(super) const LOGO: &[&str] = &[
    "███╗   ███╗███████╗██████╗ ██╗     ",
    "████╗ ████║██╔════╝██╔══██╗██║     ",
    "██╔████╔██║█████╗  ██████╔╝██║     ",
    "██║╚██╔╝██║██╔══╝  ██╔══██╗██║     ",
    "██║ ╚═╝ ██║███████╗██║  ██║███████╗",
    "╚═╝     ╚═╝╚══════╝╚═╝  ╚═╝╚══════╝",
];

/// Actions listed under the logo; the keys come from [`crate::app::KEYS`].
pub(super) const WELCOME_ACTIONS: [&str; 5] = [
    "Open a file (fuzzy)",
    "Search the project",
    "Project symbols (fuzzy)",
    "This help",
    "Quit",
];

/// `(key, action)` rows of the welcome screen. Panics on an action `KEYS` no longer lists,
/// which the test below turns into a failure.
pub(super) fn welcome_hints() -> Vec<(&'static str, &'static str)> {
    WELCOME_ACTIONS
        .iter()
        .map(|a| {
            let (k, _) = crate::app::KEYS
                .iter()
                .find(|(_, x)| x == a)
                .unwrap_or_else(|| panic!("welcome action {a:?} is not in KEYS"));
            (*k, *a)
        })
        .collect()
}

/// No file open: the logo and the keys that work without one, a little above the centre of
/// the pane. Too small for the logo: the keys alone; too small for those: a one-line hint.
pub(super) fn draw_welcome(frame: &mut Frame, theme: &Theme, area: Rect, base: Style) {
    let logo: &[&str] = LOGO;
    let hints = welcome_hints();
    let key_w = hints.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    let hint_w = hints
        .iter()
        .map(|(_, a)| key_w + 3 + a.len())
        .max()
        .unwrap_or(0);
    let logo_w = logo.iter().map(|l| wrap::width(l)).max().unwrap_or(0);

    let (rows, w) = if area.height as usize >= logo.len() + 1 + hints.len()
        && area.width as usize >= logo_w.max(hint_w)
    {
        (logo.len() + 1 + hints.len(), logo_w.max(hint_w))
    } else if area.height as usize >= hints.len() && area.width as usize >= hint_w {
        (hints.len(), hint_w)
    } else {
        let hint = Line::from(Span::styled(
            " o: open file   ?: help",
            base.fg(theme.ghost_fg),
        ));
        frame.render_widget(Paragraph::new(hint).style(base), area);
        return;
    };
    let with_logo = rows > hints.len();

    let mut lines: Vec<Line> = Vec::with_capacity(rows);
    if with_logo {
        // Left-align the logo inside the block, and colour it in three bands.
        let band = logo.len().div_ceil(LOGO_COLORS.len());
        for (i, l) in logo.iter().enumerate() {
            let color = LOGO_COLORS[(i / band).min(LOGO_COLORS.len() - 1)];
            lines.push(Line::from(Span::styled(*l, base.fg(color))));
        }
        lines.push(Line::default());
    }
    for (k, a) in &hints {
        lines.push(Line::from(vec![
            Span::styled(format!("{k:<key_w$}"), base.fg(LOGO_COLORS[0])),
            Span::styled(format!("   {a}"), base),
        ]));
    }

    // Two fifths of the free space above, three below: the block sits a little above the
    // centre, where the eye expects it.
    let top = (area.height as usize - rows) * 2 / 5;
    let [_, block] = Layout::vertical([
        Constraint::Length(top as u16),
        Constraint::Length(rows as u16),
    ])
    .areas(area);
    let [block] = Layout::horizontal([Constraint::Length(w as u16)])
        .flex(Flex::Center)
        .areas(block);
    frame.render_widget(Paragraph::new(lines).style(base), block);
}
