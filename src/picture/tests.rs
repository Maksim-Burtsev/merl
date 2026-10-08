use super::*;

fn dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("merl-picture-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn png(path: &Path, w: u32, h: u32) {
    image::RgbaImage::from_pixel(w, h, image::Rgba([10, 20, 30, 128]))
        .save(path)
        .unwrap();
}

#[test]
fn a_picture_smaller_than_the_room_is_never_enlarged() {
    let cell = (10, 20);
    let px = 20.0 * TEXT_IN_CELL / FONT_PX;
    let (cols, rows) = cells((100, 40), None, cell, 200, Some(100));
    assert_eq!(cols, (100.0 * px / 10.0_f32).round() as u16);
    assert_eq!(rows, (40.0 * px / 20.0_f32).round() as u16);
}

#[test]
fn a_picture_larger_than_the_room_shrinks_until_the_whole_fits() {
    let cell = (10, 20);
    assert_eq!(cells((4000, 400), None, cell, 80, Some(50)), (80, 4));
    let (cols, rows) = cells((400, 4000), None, cell, 80, Some(20));
    assert_eq!(rows, 20);
    assert!(cols < 10, "{cols}");
    let (_, tall) = cells((400, 4000), None, cell, 80, None);
    assert!(
        tall > 20,
        "without a height to fit, only the width shrinks it: {tall}"
    );
}

#[test]
fn a_width_attribute_sets_the_size_and_a_percent_takes_the_room() {
    let cell = (10, 20);
    let px = 20.0 * TEXT_IN_CELL / FONT_PX;
    let (cols, rows) = cells((400, 200), Some(Width::Px(100.0)), cell, 200, None);
    assert_eq!(cols, (100.0 * px / 10.0_f32).round() as u16);
    assert_eq!(rows, (50.0 * px / 20.0_f32).round() as u16);
    assert_eq!(
        cells((400, 200), Some(Width::Percent(50.0)), cell, 40, None).0,
        20
    );
}

#[test]
fn an_animation_shows_each_frame_for_its_time_and_loops() {
    assert_eq!(frame_at(&[100, 50, 200], 0), (0, 100));
    assert_eq!(frame_at(&[100, 50, 200], 120), (1, 30));
    assert_eq!(frame_at(&[100, 50, 200], 150), (2, 200));
    assert_eq!(frame_at(&[100, 50, 200], 350 + 99), (0, 1));
    assert_eq!(frame_at(&[0], 5).0, 0);
}

#[test]
fn a_png_decodes_whole_with_its_size() {
    let dir = dir("png");
    let path = dir.join("shot.png");
    png(&path, 30, 12);
    let drawn = decode(&path).unwrap();
    assert_eq!(drawn.natural, (30, 12));
    assert_eq!(drawn.frames.len(), 1);
    assert_eq!(drawn.h, 12);
    assert!(drawn.frames[0].0.starts_with(b"\x89PNG"));
    assert_eq!(header(&path), Some(("PNG", 30, 12)));
    assert_eq!(not_shown_note(&path).unwrap(), "PNG 30\u{d7}12, not shown");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_gif_keeps_every_frame_and_its_delay() {
    use image::codecs::gif::{GifEncoder, Repeat};
    let dir = dir("gif");
    let path = dir.join("movie.gif");
    let file = std::fs::File::create(&path).unwrap();
    let mut enc = GifEncoder::new(file);
    enc.set_repeat(Repeat::Infinite).unwrap();
    for (shade, ms) in [(0u8, 200u32), (255, 0)] {
        let buf = image::RgbaImage::from_pixel(4, 3, image::Rgba([shade, shade, shade, 255]));
        let delay = image::Delay::from_numer_denom_ms(ms, 1);
        enc.encode_frame(image::Frame::from_parts(buf, 0, 0, delay))
            .unwrap();
    }
    drop(enc);
    let drawn = decode(&path).unwrap();
    assert_eq!(drawn.natural, (4, 3));
    let ms: Vec<u32> = drawn.frames.iter().map(|f| f.1).collect();
    assert_eq!(
        ms,
        vec![200, 100],
        "a delay of 0 plays at 100 ms, as browsers do"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_merl_cannot_read_whole_says_why() {
    let dir = dir("broken");
    let png = dir.join("cut.png");
    std::fs::write(&png, b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x10").unwrap();
    assert_eq!(decode(&png).err().unwrap(), "PNG not drawn: broken file");
    let avif = dir.join("photo.avif");
    std::fs::write(&avif, b"\0\0\0\x20ftypavif\0\0\0\0avifmif1miaf").unwrap();
    assert_eq!(decode(&avif).err().unwrap(), "AVIF not drawn");
    assert_eq!(not_shown_note(&avif), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_svg_draws_at_twice_its_size_unless_it_links_a_missing_file() {
    let dir = dir("svg");
    let path = dir.join("logo.svg");
    let circle = r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><circle cx="10" cy="10" r="8"/></svg>"#;
    std::fs::write(&path, circle).unwrap();
    let drawn = decode(&path).unwrap();
    assert_eq!(drawn.natural, (40, 20));
    assert_eq!(drawn.h, 40);
    let linked = r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><image href="gone.png" width="10" height="10"/></svg>"#;
    std::fs::write(&path, linked).unwrap();
    assert_eq!(
        decode(&path).err().unwrap(),
        "SVG not drawn: a linked file is missing"
    );
    png(&dir.join("gone.png"), 2, 2);
    assert!(decode(&path).is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_markdown_image_is_found_beside_the_file_or_from_the_root_and_never_on_the_web() {
    let root = dir("resolve");
    std::fs::create_dir_all(root.join("docs/img")).unwrap();
    png(&root.join("docs/img/a b.png"), 1, 1);
    png(&root.join("logo.png"), 1, 1);
    let file = root.join("docs/README.md");
    assert_eq!(
        resolve(&root, &file, "img/a%20b.png?raw=true"),
        Some(root.join("docs/img/a b.png"))
    );
    assert_eq!(
        resolve(&root, &file, "/logo.png#gh-dark-mode-only"),
        Some(root.join("logo.png"))
    );
    assert_eq!(
        resolve(&root, &file, "../logo.png"),
        Some(root.join("docs/../logo.png"))
    );
    assert_eq!(resolve(&root, &file, "https://example.com/logo.png"), None);
    assert_eq!(resolve(&root, &file, "//example.com/logo.png"), None);
    assert_eq!(resolve(&root, &file, "img/missing.png"), None);
    assert_eq!(resolve(&root, &file, "README.md"), None);
    let _ = std::fs::remove_dir_all(&root);
}
