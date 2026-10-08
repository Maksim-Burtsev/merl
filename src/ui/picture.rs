use std::path::Path;

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Widget};

use crate::app::{App, Side};
use crate::mermaid::{Diagrams, Place};
use crate::picture;

pub const CHECKER: [Color; 2] = [Color::Rgb(0x99, 0x99, 0x99), Color::Rgb(0x66, 0x66, 0x66)];
const FILE_PLACEMENT: u32 = 1 << 20;
const GAP: u16 = 2;

pub enum Status {
    Size(String),
    Why(String),
}

pub(super) fn status(app: &App) -> Option<Status> {
    if let Some(sides) = app.picture_sides() {
        let why = (sides.iter())
            .filter_map(|s| s.path.as_deref())
            .find_map(|p| app.diagrams.file_failed(p));
        return Some(why.map_or(Status::Size(String::new()), |w| Status::Why(w.to_string())));
    }
    let path = app.picture_here()?;
    if let Some(why) = app.diagrams.file_failed(&path) {
        return Some(Status::Why(why.to_string()));
    }
    app.diagrams.on().then(|| {
        Status::Size(
            app.diagrams
                .known_size(&path)
                .map_or(String::new(), |(w, h)| format!("{w}\u{d7}{h}")),
        )
    })
}

pub(super) fn draw_file(frame: &mut Frame, app: &mut App, area: Rect, path: &Path) {
    if !super::covers_code(app) {
        draw_picture(
            frame.buffer_mut(),
            &mut app.diagrams,
            path,
            area,
            FILE_PLACEMENT,
        );
    }
}

pub(super) fn draw_review(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    sides: &[Side],
    note_fg: Color,
) {
    if super::covers_code(app) {
        return;
    }
    let buf = frame.buffer_mut();
    for (i, (side, half)) in sides.iter().zip(halves(area, sides.len())).enumerate() {
        let border = Style::new().fg(if side.old { Color::Red } else { Color::Green });
        let note = Style::new().fg(note_fg);
        let failed = side
            .path
            .as_deref()
            .is_none_or(|p| app.diagrams.file_failed(p).is_some());
        if failed {
            let text = (side.path.as_deref())
                .and_then(picture::not_shown_note)
                .unwrap_or_else(|| "binary file, not shown".into());
            let room = Rect {
                height: half.height.saturating_sub(1),
                ..half
            };
            let at = centred(room, (text.chars().count() as u16 + 4, 3));
            Block::bordered().border_style(border).render(at, buf);
            let row = Rect {
                y: at.y + 1,
                height: 1,
                ..at
            }
            .inner(ratatui::layout::Margin::new(1, 0));
            Line::styled(text, note).centered().render(row, buf);
            continue;
        }
        let Some(path) = side.path.as_deref() else {
            continue;
        };
        let inner = Rect {
            y: half.y + 1,
            height: half.height.saturating_sub(3),
            ..half
        };
        let placement = FILE_PLACEMENT + i as u32;
        if let Some((at, (w, h))) = draw_picture(buf, &mut app.diagrams, path, inner, placement) {
            let framed = framed(at);
            Block::bordered().border_style(border).render(framed, buf);
            let row = Rect {
                y: framed.bottom(),
                height: 1,
                ..half
            };
            Line::styled(format!("{w}\u{d7}{h}"), note)
                .centered()
                .render(row.intersection(buf.area), buf);
        }
    }
}

pub fn halves(area: Rect, n: usize) -> Vec<Rect> {
    if n < 2 {
        return vec![area];
    }
    let w = area.width.saturating_sub(GAP) / 2;
    let right = Rect {
        x: area.right() - w,
        width: w,
        ..area
    };
    vec![Rect { width: w, ..area }, right]
}

pub fn framed(at: Rect) -> Rect {
    Rect {
        x: at.x.saturating_sub(1),
        y: at.y.saturating_sub(1),
        width: at.width + 2,
        height: at.height + 2,
    }
}

pub fn centred(area: Rect, (cols, rows): (u16, u16)) -> Rect {
    let (w, h) = (cols.min(area.width), rows.min(area.height));
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

pub fn draw_picture(
    buf: &mut Buffer,
    diagrams: &mut Diagrams,
    path: &Path,
    area: Rect,
    placement: u32,
) -> Option<(Rect, (u32, u32))> {
    let natural = diagrams.file_size(path)?;
    let room = (
        area.width.saturating_sub(2).max(1) as usize,
        area.height.max(1) as usize,
    );
    let fit = picture::cells(natural, None, diagrams.cell()?, room.0, Some(room.1));
    let at = centred(area, fit);
    checker(buf, at);
    if let Some(pic) = diagrams.file_pic(path) {
        diagrams.want.push(Place {
            id: pic.id(),
            placement,
            x: at.x,
            y: at.y,
            cols: at.width,
            rows: at.height,
            crop_y: 0,
            crop_h: pic.height(),
        });
    }
    Some((at, natural))
}

pub fn checker(buf: &mut Buffer, at: Rect) {
    let at = at.intersection(buf.area);
    for y in at.top()..at.bottom() {
        for x in at.left()..at.right() {
            let square = ((x - at.x) / 2 + (y - at.y)) % 2;
            buf[(x, y)].set_bg(CHECKER[square as usize]);
        }
    }
}
