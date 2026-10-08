use std::path::{Path, PathBuf};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer as Cells;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;

use crate::app::App;
use crate::buffer::Buffer;
use crate::mermaid::Diagrams;
use crate::tree::Tree;
use crate::ui::picture::{CHECKER, centred, checker};

use super::rows;

fn dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("merl-ui-picture-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn png(path: &Path, w: u32, h: u32) {
    image::RgbaImage::from_pixel(w, h, image::Rgba([10, 20, 30, 128]))
        .save(path)
        .unwrap();
}

fn open(root: &Path, file: &str, pictures: bool) -> App {
    let path = root.join(file);
    let bytes = std::fs::read(&path).unwrap();
    let mut app = App::new(
        root.to_path_buf(),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(path, &bytes),
        None,
    );
    app.show_tree = false;
    if pictures {
        app.diagrams = Diagrams::with_cell(Some((10, 20)));
    }
    app
}

fn paint(app: &mut App, w: u16, h: u16) -> Terminal<TestBackend> {
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    for _ in 0..3 {
        terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
        for job in app.diagrams.jobs() {
            app.diagrams.done(crate::mermaid::render(job));
        }
    }
    terminal
}

fn press(app: &mut App, c: char) {
    app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
}

#[test]
fn the_checkerboard_is_squares_two_cells_wide_and_one_row_high() {
    let mut buf = Cells::empty(Rect::new(0, 0, 10, 4));
    checker(&mut buf, Rect::new(1, 1, 5, 2));
    let bg = |x, y| buf[(x, y)].bg;
    assert_eq!(
        [bg(1, 1), bg(2, 1), bg(3, 1), bg(4, 1), bg(5, 1)],
        [CHECKER[0], CHECKER[0], CHECKER[1], CHECKER[1], CHECKER[0]]
    );
    assert_eq!([bg(1, 2), bg(3, 2)], [CHECKER[1], CHECKER[0]]);
    assert_eq!(
        bg(0, 1),
        ratatui::style::Color::Reset,
        "only under the picture"
    );
    assert_eq!(bg(6, 1), ratatui::style::Color::Reset);
}

#[test]
fn a_picture_stands_in_the_middle_of_its_room() {
    let room = Rect::new(4, 2, 60, 20);
    assert_eq!(centred(room, (20, 10)), Rect::new(24, 7, 20, 10));
    assert_eq!(centred(room, (80, 30)), room);
}

#[test]
fn an_image_file_is_its_picture_centred_with_its_size_in_the_status_bar() {
    let root = dir("file");
    png(&root.join("shot.png"), 300, 120);
    let mut app = open(&root, "shot.png", true);
    let terminal = paint(&mut app, 80, 24);
    let place = app
        .diagrams
        .want
        .first()
        .cloned()
        .expect("the picture is placed");
    let px = 20.0 * crate::mermaid::TEXT_IN_CELL / crate::mermaid::FONT_PX;
    assert_eq!(place.cols, (300.0 * px / 10.0_f32).round() as u16);
    assert_eq!(place.rows, (120.0 * px / 20.0_f32).round() as u16);
    assert_eq!(place.x, (80 - place.cols) / 2);
    assert_eq!(place.y, (23 - place.rows) / 2);
    let buf = terminal.backend().buffer();
    assert_eq!(buf[(place.x, place.y)].bg, CHECKER[0]);
    assert_eq!(buf[(place.x + 2, place.y)].bg, CHECKER[1]);
    let rows = rows(&terminal);
    assert!(rows.iter().all(|r| !r.contains("not shown")), "{rows:#?}");
    let status = &rows[23];
    assert!(
        status.contains("shot.png  300\u{d7}120  [code]"),
        "{status}"
    );
    assert!(
        !status.contains("read-only") && !status.contains("1:1"),
        "{status}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn without_pictures_the_note_names_the_image() {
    let root = dir("note");
    png(&root.join("shot.png"), 300, 120);
    let mut app = open(&root, "shot.png", false);
    let terminal = paint(&mut app, 80, 24);
    let rows = rows(&terminal);
    assert!(
        rows.contains(&"PNG 300\u{d7}120, not shown".to_string()),
        "{rows:#?}"
    );
    assert!(rows[23].contains("read-only"), "{}", rows[23]);
    assert!(app.diagrams.want.is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_broken_image_stays_a_binary_file_and_the_status_bar_says_why() {
    let root = dir("broken");
    std::fs::write(
        root.join("cut.png"),
        b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x10",
    )
    .unwrap();
    let mut app = open(&root, "cut.png", true);
    let terminal = paint(&mut app, 80, 24);
    let rows = rows(&terminal);
    assert!(
        rows.contains(&"binary file, not shown".to_string()),
        "{rows:#?}"
    );
    assert!(
        rows[23].contains("1:1  [code]  PNG not drawn: broken file"),
        "{}",
        rows[23]
    );
    assert!(!rows[23].contains("read-only"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_overlay_over_the_picture_takes_it_away() {
    let root = dir("overlay");
    png(&root.join("shot.png"), 30, 12);
    let mut app = open(&root, "shot.png", true);
    paint(&mut app, 80, 24);
    assert_eq!(app.diagrams.want.len(), 1);
    press(&mut app, '?');
    paint(&mut app, 80, 24);
    assert!(app.diagrams.want.is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_gif_on_screen_asks_for_its_next_frame_and_one_off_it_asks_for_nothing() {
    use image::codecs::gif::GifEncoder;
    let root = dir("gif");
    let mut enc = GifEncoder::new(std::fs::File::create(root.join("movie.gif")).unwrap());
    for shade in [0u8, 255] {
        let buf = image::RgbaImage::from_pixel(8, 4, image::Rgba([shade, 0, 0, 255]));
        let delay = image::Delay::from_numer_denom_ms(500, 1);
        enc.encode_frame(image::Frame::from_parts(buf, 0, 0, delay))
            .unwrap();
    }
    drop(enc);
    let mut app = open(&root, "movie.gif", true);
    paint(&mut app, 80, 24);
    let wake = app.diagrams.wake.expect("a playing GIF wakes the loop");
    assert!(wake.as_millis() <= 500, "{wake:?}");
    press(&mut app, '?');
    paint(&mut app, 80, 24);
    assert_eq!(app.diagrams.wake, None);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn p_draws_an_svg_and_p_again_shows_its_source() {
    let root = dir("svg");
    let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"40\" height=\"20\">\n<circle cx=\"10\" cy=\"10\" r=\"8\"/>\n</svg>\n";
    std::fs::write(root.join("logo.svg"), svg).unwrap();
    let mut app = open(&root, "logo.svg", true);
    let rows0 = rows(&paint(&mut app, 80, 24));
    assert!(
        rows0.iter().any(|r| r.contains("<circle")),
        "opens as its source"
    );
    assert!(app.diagrams.want.is_empty());
    press(&mut app, 'p');
    let rows1 = rows(&paint(&mut app, 80, 24));
    assert!(!rows1.iter().any(|r| r.contains("<circle")), "{rows1:#?}");
    assert_eq!(app.diagrams.want.len(), 1);
    assert!(
        rows1[23].contains("logo.svg  40\u{d7}20  [preview]"),
        "{}",
        rows1[23]
    );
    press(&mut app, 'p');
    let rows2 = rows(&paint(&mut app, 80, 24));
    assert_eq!(rows2, rows0);
    assert!(app.diagrams.want.is_empty());
    let mut plain = open(&root, "logo.svg", false);
    press(&mut plain, 'p');
    assert_eq!(
        plain.message, "not Markdown",
        "a terminal without pictures: as before"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_preview_draws_a_local_image_where_its_line_stands() {
    let root = dir("preview");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    png(&root.join("docs/shot.png"), 64, 64);
    let text = "# Title\n\n![shot](docs/shot.png)\n\nAfter\n";
    std::fs::write(root.join("README.md"), text).unwrap();
    let mut app = open(&root, "README.md", true);
    press(&mut app, 'p');
    let rows = rows(&paint(&mut app, 80, 24));
    assert!(!rows.iter().any(|r| r.contains('\u{25a3}')), "{rows:#?}");
    let place = app.diagrams.want.first().cloned().expect("placed");
    let after = rows.iter().position(|r| r.ends_with("After")).unwrap();
    assert_eq!(
        place.y as usize + place.rows as usize + 1,
        after,
        "{rows:#?}"
    );
    let mut plain = open(&root, "README.md", false);
    press(&mut plain, 'p');
    let rows = super::rows(&paint(&mut plain, 80, 24));
    assert!(
        rows.iter().any(|r| r.contains("\u{25a3} shot")),
        "{rows:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
