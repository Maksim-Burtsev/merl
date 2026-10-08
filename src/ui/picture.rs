use std::path::{Path, PathBuf};

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::app::App;
use crate::mermaid::{Diagrams, Place};
use crate::picture;

pub const CHECKER: [Color; 2] = [Color::Rgb(0x99, 0x99, 0x99), Color::Rgb(0x66, 0x66, 0x66)];
const FILE_PLACEMENT: u32 = 1 << 20;

pub enum Status {
    Size(String),
    Why(String),
}

fn picture_file(app: &App) -> Option<PathBuf> {
    let path = app.buf.path.as_ref()?;
    let raster = picture::raster_name(path).is_some() && app.buf.binary();
    let svg = picture::is_svg(path) && app.previewing();
    (raster || svg).then(|| app.root.join(path))
}

pub(super) fn shown(app: &App) -> Option<PathBuf> {
    let path = picture_file(app)?;
    (app.diagrams.on() && app.diagrams.file_failed(&path).is_none()).then_some(path)
}

pub(super) fn status(app: &App) -> Option<Status> {
    let path = picture_file(app)?;
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
        draw_picture(frame.buffer_mut(), &mut app.diagrams, path, area, FILE_PLACEMENT);
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
) -> Option<(u32, u32)> {
    let natural = diagrams.file_size(path)?;
    let room = (area.width.saturating_sub(2).max(1) as usize, area.height.max(1) as usize);
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
    Some(natural)
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
