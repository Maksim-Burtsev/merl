use std::path::{Path, PathBuf};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer as Cells;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::Color;

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
    paint_in(app, w, h, crate::theme::DEFAULT)
}

fn paint_in(app: &mut App, w: u16, h: u16, theme: &str) -> Terminal<TestBackend> {
    let theme = crate::theme::load(theme).unwrap();
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

fn key(app: &mut App, code: KeyCode) {
    app.key(KeyEvent::new(code, KeyModifiers::NONE));
}

#[test]
fn keys_move_through_an_svg_whose_picture_failed() {
    let root = dir("svg-failed");
    let lines: String = (0..40).map(|i| format!("<!-- {i} -->\n")).collect();
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"9\" height=\"9\">\n<image href=\"gone.png\"/>\n{lines}</svg>\n"
    );
    std::fs::write(root.join("x.svg"), svg).unwrap();
    let mut app = open(&root, "x.svg", true);
    press(&mut app, 'p');
    let rows = rows(&paint(&mut app, 80, 24));
    assert!(
        rows[23].contains("SVG not drawn: a linked file is missing"),
        "{}",
        rows[23]
    );
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Down);
    assert_eq!(app.line, 2);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_theme_round_trip_keeps_each_code_block_its_own_colours() {
    let root = dir("round-trip");
    png(&root.join("d.png"), 20, 20);
    png(&root.join("l.png"), 20, 20);
    let text = "![d](d.png#gh-dark-mode-only)\n\n```\n\u{436}\u{436}\u{436}\u{436}\n```\n\n```rust\nlet x = 1;\n```\n\n![l](l.png#gh-light-mode-only)\n";
    std::fs::write(root.join("README.md"), text).unwrap();
    let mut app = open(&root, "README.md", true);
    press(&mut app, 'p');
    for theme in [crate::theme::DEFAULT, "github-light", crate::theme::DEFAULT] {
        let rows = rows(&paint_in(&mut app, 80, 24, theme));
        assert!(
            rows.iter().any(|r| r.contains("let x = 1;")),
            "{theme}: {rows:#?}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_centred_preview_image_stands_past_the_gutter_and_its_lead_without_a_tint() {
    let root = dir("lead");
    png(&root.join("shot.png"), 64, 64);
    let text = "<p align=\"center\">\n<img src=\"shot.png\">\n</p>\n";
    std::fs::write(root.join("README.md"), text).unwrap();
    let mut app = open(&root, "README.md", true);
    press(&mut app, 'p');
    let terminal = paint(&mut app, 80, 24);
    let place = app.diagrams.want.first().cloned().expect("placed");
    let gutter = 2;
    let room = 80 - gutter;
    assert_eq!(place.x, gutter + (room - place.cols) / 2);
    let buf = terminal.backend().buffer();
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let tint = crate::markdown::Palette::new(&theme).code_bg;
    let last = place.y + place.rows - 1;
    assert_eq!(buf[(79, last)].bg, theme.bg);
    assert_ne!(buf[(79, last)].bg, tint);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_preview_takes_the_variant_of_the_theme_it_is_drawn_in() {
    let root = dir("variant");
    png(&root.join("dark.png"), 20, 20);
    png(&root.join("light.png"), 40, 20);
    let text = "<picture>\n<source media=\"(prefers-color-scheme: dark)\" srcset=\"dark.png\">\n<img src=\"light.png\">\n</picture>\n";
    std::fs::write(root.join("README.md"), text).unwrap();
    let mut app = open(&root, "README.md", true);
    press(&mut app, 'p');
    paint(&mut app, 80, 24);
    let dark = app.diagrams.want[0].cols;
    paint_in(&mut app, 80, 24, "github-light");
    assert_eq!(app.diagrams.want[0].cols, dark * 2);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_image_rewritten_on_disk_is_drawn_anew() {
    let root = dir("rewrite");
    png(&root.join("shot.png"), 30, 12);
    let mut app = open(&root, "shot.png", true);
    paint(&mut app, 80, 24);
    let before = app.diagrams.want[0].cols;
    png(&root.join("shot.png"), 60, 12);
    let rows = rows(&paint(&mut app, 80, 24));
    assert!(rows[23].contains("60\u{d7}12"), "{}", rows[23]);
    assert_eq!(app.diagrams.want[0].cols, before * 2);
    let _ = std::fs::remove_dir_all(&root);
}

fn picture_review(tag: &str) -> (PathBuf, App) {
    let root = dir(tag);
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "t@t"]);
    git(&["config", "user.name", "t"]);
    png(&root.join("shot.png"), 300, 120);
    png(&root.join("gone.png"), 100, 50);
    png(&root.join("cut.png"), 20, 10);
    let big: Vec<u8> = (0..4000u32).map(|i| (i * 7 % 251) as u8).collect();
    std::fs::write(root.join("big.png"), &big).unwrap();
    std::fs::write(root.join("same.png"), b"s\0same").unwrap();
    std::fs::write(root.join("blob.bin"), b"b\0").unwrap();
    std::fs::write(root.join("keep.txt"), "k\n").unwrap();
    std::fs::write(root.join("z.txt"), "z\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    png(&root.join("shot.png"), 200, 100);
    png(&root.join("new.png"), 40, 20);
    std::fs::remove_file(root.join("gone.png")).unwrap();
    std::fs::write(
        root.join("cut.png"),
        b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x10",
    )
    .unwrap();
    git(&["mv", "same.png", "moved.png"]);
    std::fs::remove_file(root.join("big.png")).unwrap();
    std::fs::write(
        root.join("big2.png"),
        [&big[..3990], b"0123456789"].concat(),
    )
    .unwrap();
    std::fs::write(root.join("blob.bin"), b"c\0").unwrap();
    std::fs::write(root.join("z.txt"), "Z\n").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "work"]);
    let review = crate::git::Review::open(&root, None, None).unwrap();
    let mut app = App::new(
        root.clone(),
        Tree::default(),
        Vec::new(),
        Buffer::empty(),
        None,
    );
    app.diagrams = Diagrams::with_cell(Some((10, 20)));
    app.start_review(review);
    app.show_tree = false;
    (root, app)
}

fn frame_colour(terminal: &Terminal<TestBackend>, place: &crate::mermaid::Place) -> Color {
    let cell = &terminal.backend().buffer()[(place.x - 1, place.y - 1)];
    assert_eq!(cell.symbol(), "\u{250c}");
    cell.fg
}

#[test]
fn a_changed_image_is_its_old_picture_in_red_beside_its_new_one_in_green() {
    let (root, mut app) = picture_review("review2up");
    app.jump_to(&root.join("shot.png"), 1);
    let terminal = paint(&mut app, 100, 30);
    let [old, new] = &app.diagrams.want[..] else {
        panic!("{:?}", app.diagrams.want)
    };
    assert!(old.x + old.cols < 50 && new.x > 50, "{old:?} {new:?}");
    assert_eq!(frame_colour(&terminal, old), Color::Red);
    assert_eq!(frame_colour(&terminal, new), Color::Green);
    let rows = rows(&terminal);
    let under = |p: &crate::mermaid::Place| rows[usize::from(p.y + p.rows + 1)].clone();
    assert!(under(old).starts_with("300\u{d7}120"), "{rows:#?}");
    assert!(under(new).ends_with("200\u{d7}100"), "{rows:#?}");
    assert!(!rows[29].contains("1:1"), "{}", rows[29]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_added_image_stands_alone_in_green_and_a_deleted_one_in_red() {
    let (root, mut app) = picture_review("reviewalone");
    for (file, colour) in [("new.png", Color::Green), ("gone.png", Color::Red)] {
        app.jump_to(&root.join(file), 1);
        let terminal = paint(&mut app, 100, 30);
        let [place] = &app.diagrams.want[..] else {
            panic!("{file}: {:?}", app.diagrams.want)
        };
        assert_eq!(place.x, (100 - place.cols) / 2, "{file}");
        assert_eq!(frame_colour(&terminal, place), colour, "{file}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_side_that_cannot_be_read_shows_the_note_in_its_frame_and_the_other_is_drawn() {
    let (root, mut app) = picture_review("reviewcut");
    app.jump_to(&root.join("cut.png"), 1);
    let terminal = paint(&mut app, 100, 30);
    let [old] = &app.diagrams.want[..] else {
        panic!("{:?}", app.diagrams.want)
    };
    assert!(old.x + old.cols < 50);
    let rows = rows(&terminal);
    let note = rows
        .iter()
        .position(|r| r.ends_with("\u{2502} binary file, not shown \u{2502}"));
    let note = note.unwrap_or_else(|| panic!("{rows:#?}"));
    let buf = terminal.backend().buffer();
    let x = (50..100)
        .find(|&x| buf[(x, note as u16)].symbol() == "\u{2502}")
        .unwrap();
    assert_eq!(buf[(x, note as u16)].fg, Color::Green);
    assert!(
        rows[29].contains("PNG not drawn: broken file"),
        "{}",
        rows[29]
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn halves_leave_a_gap_and_a_frame_hugs_its_picture() {
    use crate::ui::picture::{framed, halves};
    let area = Rect::new(30, 0, 101, 40);
    assert_eq!(halves(area, 1), [area]);
    assert_eq!(
        halves(area, 2),
        [Rect::new(30, 0, 49, 40), Rect::new(82, 0, 49, 40)]
    );
    assert_eq!(framed(Rect::new(5, 3, 10, 4)), Rect::new(4, 2, 12, 6));
}

#[test]
fn only_an_image_whose_bytes_changed_is_a_stop_and_only_where_pictures_are_drawn() {
    let (root, mut app) = picture_review("reviewstops");
    let r = app.review.clone().unwrap();
    let f = |name: &str| {
        r.file(Path::new(name))
            .unwrap_or_else(|| panic!("{name}: {:#?}", r.files))
    };
    assert_eq!(f("big2.png").status, 'R', "{:?}", r.files);
    assert!(f("moved.png").binary && !f("moved.png").is_stop(&root, true));
    assert!(!f("blob.bin").is_stop(&root, true));
    assert!(f("shot.png").is_stop(&root, true) && !f("shot.png").is_stop(&root, false));
    assert_eq!(r.first_file(&root, true), Some(root.join("big2.png")));
    assert_eq!(r.first_file(&root, false), Some(root.join("z.txt")));
    assert_eq!(app.buf.path, Some(root.join("big2.png")));
    let sides = app.picture_sides().unwrap();
    let old = sides[0].path.as_ref().unwrap();
    assert!(sides[0].old && old.ends_with("big.png"), "{old:?}");
    let mut walk = vec![];
    for _ in 0..5 {
        press(&mut app, 'c');
        let at = app.buf.path.clone().unwrap();
        walk.push(at.strip_prefix(&root).unwrap().display().to_string());
    }
    assert_eq!(
        walk,
        ["cut.png", "gone.png", "new.png", "shot.png", "z.txt"]
    );
    press(&mut app, 'C');
    app.jump_to(&root.join("keep.txt"), 1);
    press(&mut app, 'c');
    assert_eq!(
        app.buf.path,
        Some(root.join("shot.png")),
        "back to the image left"
    );
    paint(&mut app, 100, 30);
    assert_eq!(app.diagrams.want.len(), 2);
    press(&mut app, '?');
    paint(&mut app, 100, 30);
    assert!(
        app.diagrams.want.is_empty(),
        "an overlay hides both pictures"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_side_git_cannot_give_says_why_in_the_status_bar() {
    let (root, mut app) = picture_review("reviewnobase");
    app.jump_to(&root.join("shot.png"), 1);
    let r = app.review.as_mut().unwrap();
    r.merge_base = "0".repeat(40);
    let terminal = paint(&mut app, 100, 30);
    let rows = rows(&terminal);
    assert_eq!(app.diagrams.want.len(), 1, "the new side is drawn");
    assert!(
        rows.iter().any(|r| r.contains("binary file, not shown")),
        "{rows:#?}"
    );
    assert!(rows[29].contains("fatal"), "{}", rows[29]);
    let _ = std::fs::remove_dir_all(&root);
}
