use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Padding, Paragraph, Wrap};

use crate::app::App;
use crate::theme::Theme;

pub(super) const HINT: &str = " Enter starts \u{00b7} i shows this again ";

pub(super) struct Lesson {
    title: String,
    text: &'static str,
}

pub(super) fn of(app: &App) -> Option<Lesson> {
    let tutor = app.tutor.as_ref()?;
    let (title, text) = match (&tutor.drill, crate::tutor::lesson(tutor.step)) {
        (Some(drill), _) => drill.panel(),
        (None, Some(t)) => (
            format!(
                " Tutor {}/{} \u{00b7} {}",
                tutor.step + 1,
                crate::tutor::TUTOR.len(),
                t.title
            ),
            t.tutor,
        ),
        (None, None) => (
            " Tutor \u{2713} done".to_owned(),
            crate::tutor::ALL_LESSONS_DONE,
        ),
    };
    Some(Lesson { title, text })
}

fn float(theme: &Theme) -> Color {
    theme
        .groups
        .get("NormalFloat")
        .and_then(|s| s.bg)
        .unwrap_or(theme.status_bg)
}

impl Lesson {
    fn boxed(
        &self,
        theme: &Theme,
        base: Style,
        card: bool,
    ) -> (Block<'static>, Paragraph<'static>) {
        let style = base.bg(float(theme));
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(style.fg(theme.ghost_fg))
            .title(Span::styled(
                format!("{} ", self.title),
                style.add_modifier(Modifier::BOLD),
            ))
            .style(style);
        let block = if card {
            block
                .padding(Padding::new(2, 2, 1, 1))
                .title_bottom(Line::styled(HINT, style.fg(theme.ghost_fg)).right_aligned())
        } else {
            block.padding(Padding::horizontal(1))
        };
        let text = Paragraph::new(self.text)
            .wrap(Wrap { trim: true })
            .style(style);
        (block, text)
    }

    pub(super) fn height(&self, theme: &Theme, base: Style, card: bool, width: u16) -> u16 {
        let (block, text) = self.boxed(theme, base, card);
        let tall = Rect::new(0, 0, width, u16::MAX / 2);
        let inner = block.inner(tall);
        let rows = u16::try_from(text.line_count(inner.width)).unwrap_or(u16::MAX / 2);
        rows.saturating_add(tall.height - inner.height)
    }

    pub(super) fn draw_box(&self, frame: &mut Frame, theme: &Theme, base: Style, at: Rect) {
        self.draw(frame, theme, base, at, false);
    }

    fn draw(&self, frame: &mut Frame, theme: &Theme, base: Style, at: Rect, card: bool) {
        let (block, text) = self.boxed(theme, base, card);
        let inner = block.inner(at);
        frame.render_widget(Clear, at);
        frame.render_widget(block, at);
        frame.render_widget(text, inner);
    }

    pub(super) fn draw_card(
        &self,
        frame: &mut Frame,
        (theme, base): (&Theme, Style),
        (above, bottom): (Rect, Rect),
        slid: u32,
    ) {
        let w = above.width.saturating_sub(8).min(84);
        let h = self.height(theme, base, true, w).min(above.height);
        let middle = Rect {
            x: above.x + (above.width - w) / 2,
            y: above.y + (above.height - h) * 2 / 5,
            width: w,
            height: h,
        };
        let left = 1000 - slid.min(1000);
        let eased = 1000 - left * left / 1000 * left / 1000;
        let at = Rect {
            x: lerp(middle.x, bottom.x, eased),
            y: lerp(middle.y, bottom.y, eased),
            width: lerp(middle.width, bottom.width, eased),
            height: lerp(middle.height, bottom.height, eased),
        };
        dim(frame, theme, at, 333 + eased * 667 / 1000);
        self.draw(frame, theme, base, at, eased < 500);
    }
}

fn lerp(from: u16, to: u16, permille: u32) -> u16 {
    let (from, to) = (i64::from(from), i64::from(to));
    u16::try_from(from + (to - from) * i64::from(permille) / 1000).unwrap_or(0)
}

fn dim(frame: &mut Frame, theme: &Theme, keep: Rect, permille: u32) {
    if permille >= 1000 {
        return;
    }
    let area = frame.area();
    let buf = frame.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if keep.contains((x, y).into()) {
                continue;
            }
            if let Some(c) = buf.cell_mut((x, y)) {
                let fg = if c.fg == Color::Reset { theme.fg } else { c.fg };
                c.set_fg(mix(fg, theme.bg, permille));
                if c.bg != Color::Reset {
                    c.set_bg(mix(c.bg, theme.bg, permille));
                }
                c.modifier.remove(Modifier::BOLD);
            }
        }
    }
}

fn mix(colour: Color, bg: Color, permille: u32) -> Color {
    let (Color::Rgb(r, g, b), Color::Rgb(br, bgr, bb)) = (colour, bg) else {
        return colour;
    };
    let m = |x: u8, y: u8| {
        let v = (u32::from(x) * permille + u32::from(y) * (1000 - permille)) / 1000;
        u8::try_from(v).unwrap_or(x)
    };
    Color::Rgb(m(r, br), m(g, bgr), m(b, bb))
}
