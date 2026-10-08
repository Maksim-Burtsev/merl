use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::metadata::Orientation;
use image::{AnimationDecoder, DynamicImage, ImageDecoder, ImageEncoder, ImageError, ImageFormat};
use resvg::{tiny_skia, usvg};

use crate::mermaid::{FONT_PX, TEXT_IN_CELL};

const NAMES: &[(&str, &str)] = &[
    ("png", "PNG"),
    ("jpg", "JPEG"),
    ("jpeg", "JPEG"),
    ("gif", "GIF"),
    ("webp", "WebP"),
    ("bmp", "BMP"),
    ("ico", "ICO"),
    ("tif", "TIFF"),
    ("tiff", "TIFF"),
    ("avif", "AVIF"),
    ("heic", "HEIC"),
    ("heif", "HEIF"),
];
const MAX_SIDE: u32 = 8192;
const MAX_PIXELS: u64 = 16_000_000;
const MAX_FRAMES: usize = 1000;
const SVG_SCALE: f32 = 2.0;

pub struct Drawn {
    pub frames: Vec<(Vec<u8>, u32)>,
    pub natural: (u32, u32),
    pub h: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Width {
    Px(f32),
    Percent(f32),
}

fn ext(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

pub fn raster_name(path: &Path) -> Option<&'static str> {
    let ext = ext(path);
    NAMES.iter().find(|(e, _)| *e == ext).map(|(_, n)| *n)
}

pub fn is_svg(path: &Path) -> bool {
    ext(path) == "svg"
}

pub fn is_picture(path: &Path) -> bool {
    raster_name(path).is_some() || is_svg(path)
}

fn format_name(f: ImageFormat) -> Option<&'static str> {
    Some(match f {
        ImageFormat::Png => "PNG",
        ImageFormat::Jpeg => "JPEG",
        ImageFormat::Gif => "GIF",
        ImageFormat::WebP => "WebP",
        ImageFormat::Bmp => "BMP",
        ImageFormat::Ico => "ICO",
        ImageFormat::Tiff => "TIFF",
        _ => return None,
    })
}

fn turned(o: Orientation) -> bool {
    matches!(
        o,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    )
}

pub fn header(path: &Path) -> Option<(&'static str, u32, u32)> {
    let reader = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    let name = format_name(reader.format()?)?;
    let mut decoder = reader.into_decoder().ok()?;
    let (w, h) = decoder.dimensions();
    let turn = decoder.orientation().is_ok_and(turned);
    Some(if turn { (name, h, w) } else { (name, w, h) })
}

pub fn not_shown_note(path: &Path) -> Option<String> {
    raster_name(path)?;
    let (name, w, h) = header(path)?;
    Some(format!("{name} {w}\u{d7}{h}, not shown"))
}

pub fn resolve(root: &Path, file: &Path, dest: &str) -> Option<PathBuf> {
    let dest = dest.trim();
    if dest.starts_with("//") || dest.contains(':') {
        return None;
    }
    let dest = dest.split(['?', '#']).next()?.replace("%20", " ");
    let path = match dest.strip_prefix('/') {
        Some(rest) => root.join(rest),
        None => file.parent()?.join(dest),
    };
    (is_picture(&path) && path.is_file()).then_some(path)
}

pub fn cells(
    natural: (u32, u32),
    width: Option<Width>,
    cell: (u32, u32),
    room: usize,
    room_rows: Option<usize>,
) -> (u16, u16) {
    let (cw, ch) = (cell.0.max(1) as f32, cell.1.max(1) as f32);
    let px = ch * TEXT_IN_CELL / FONT_PX;
    let (mut w, mut h) = (natural.0.max(1) as f32, natural.1.max(1) as f32);
    if let Some(want) = width {
        let want = match want {
            Width::Px(px) => px,
            Width::Percent(p) => p / 100.0 * room as f32 * cw / px,
        };
        if want > 0.0 {
            h = h * want / w;
            w = want;
        }
    }
    let (w, h) = (w * px, h * px);
    let mut k = (room as f32 * cw / w).min(1.0);
    if let Some(rows) = room_rows {
        k = k.min(rows as f32 * ch / h);
    }
    let cols = (w * k / cw).round().max(1.0);
    let rows = (h * k / ch).round().max(1.0);
    (
        cols.min(f32::from(u16::MAX)) as u16,
        rows.min(f32::from(u16::MAX)) as u16,
    )
}

pub fn frame_at(ms: &[u32], elapsed: u64) -> (usize, u64) {
    let total: u64 = ms.iter().map(|&m| u64::from(m)).sum();
    if ms.len() < 2 || total == 0 {
        return (0, u64::MAX);
    }
    let mut t = elapsed % total;
    for (i, &m) in ms.iter().enumerate() {
        if t < u64::from(m) {
            return (i, u64::from(m) - t);
        }
        t -= u64::from(m);
    }
    (ms.len() - 1, 1)
}

pub fn decode(path: &Path) -> Result<Drawn, String> {
    if is_svg(path) {
        return svg(path);
    }
    let name = raster_name(path).unwrap_or("picture");
    let not_drawn = |e: ImageError| match e {
        ImageError::Unsupported(_) => format!("{name} not drawn"),
        _ => format!("{name} not drawn: broken file"),
    };
    let bytes = std::fs::read(path).map_err(|_| format!("{name} not drawn: unreadable"))?;
    let format = image::guess_format(&bytes).map_err(|_| format!("{name} not drawn"))?;
    let frames = match format {
        ImageFormat::Gif => animation(
            image::codecs::gif::GifDecoder::new(Cursor::new(&bytes)).map_err(not_drawn)?,
            name,
        )?,
        ImageFormat::WebP => {
            let d =
                image::codecs::webp::WebPDecoder::new(Cursor::new(&bytes)).map_err(not_drawn)?;
            match d.has_animation() {
                true => animation(d, name)?,
                false => Vec::new(),
            }
        }
        _ => Vec::new(),
    };
    if frames.len() > 1 {
        let natural = frames[0].1;
        let h = frames[0].2;
        return Ok(Drawn {
            frames: frames
                .into_iter()
                .map(|(png, _, _, ms)| (png, ms))
                .collect(),
            natural,
            h,
        });
    }
    let mut decoder = image::ImageReader::with_format(Cursor::new(&bytes), format)
        .into_decoder()
        .map_err(not_drawn)?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder).map_err(not_drawn)?;
    img.apply_orientation(orientation);
    let natural = (img.width(), img.height());
    let rgba = shrunk(img).to_rgba8();
    let png = encode(&rgba).ok_or_else(|| format!("{name} not drawn"))?;
    Ok(Drawn {
        frames: vec![(png, 0)],
        natural,
        h: rgba.height(),
    })
}

type Frame = (Vec<u8>, (u32, u32), u32, u32);

fn animation<'a>(d: impl AnimationDecoder<'a>, name: &str) -> Result<Vec<Frame>, String> {
    let broken = || format!("{name} not drawn: broken file");
    let mut out = Vec::new();
    for f in d.into_frames() {
        if out.len() == MAX_FRAMES {
            return Err(format!("{name} not drawn: over {MAX_FRAMES} frames"));
        }
        let f = f.map_err(|_| broken())?;
        let (n, d) = f.delay().numer_denom_ms();
        let ms = n.checked_div(d).unwrap_or(0);
        let ms = if ms <= 10 { 100 } else { ms };
        let buf = f.into_buffer();
        let natural = buf.dimensions();
        let rgba = shrunk(DynamicImage::ImageRgba8(buf)).to_rgba8();
        let png = encode(&rgba).ok_or_else(broken)?;
        out.push((png, natural, rgba.height(), ms));
    }
    Ok(out)
}

fn shrunk(img: DynamicImage) -> DynamicImage {
    let (w, h) = (img.width(), img.height());
    let pixels = u64::from(w) * u64::from(h);
    let k = (MAX_SIDE as f64 / f64::from(w.max(h)))
        .min((MAX_PIXELS as f64 / pixels.max(1) as f64).sqrt())
        .min(1.0);
    if k >= 1.0 {
        return img;
    }
    let (nw, nh) = (
        ((f64::from(w) * k) as u32).max(1),
        ((f64::from(h) * k) as u32).max(1),
    );
    img.resize_exact(nw, nh, image::imageops::FilterType::Triangle)
}

fn encode(img: &image::RgbaImage) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    PngEncoder::new_with_quality(&mut out, CompressionType::Fast, FilterType::Adaptive)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
        )
        .ok()?;
    Some(out)
}

fn fonts() -> Arc<usvg::fontdb::Database> {
    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            Arc::new(db)
        })
        .clone()
}

pub fn links_missing_file(svg: &str, dir: &Path) -> bool {
    let lower = svg.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find("<image") {
        let at = from + i;
        let end = lower[at..].find('>').map_or(lower.len(), |e| at + e);
        from = end;
        let tag = &svg[at..end];
        let href = crate::markdown::images::attr(tag, "href")
            .or_else(|| crate::markdown::images::attr(tag, "xlink:href"));
        match href {
            Some(h) if h.trim_start().starts_with("data:") => {}
            Some(h) if !h.contains(':') && dir.join(h).is_file() => {}
            _ => return true,
        }
    }
    false
}

fn svg(path: &Path) -> Result<Drawn, String> {
    let broken = || "SVG not drawn: broken file".to_string();
    let data = std::fs::read(path).map_err(|_| "SVG not drawn: unreadable".to_string())?;
    let dir = path.parent().unwrap_or(Path::new(""));
    if links_missing_file(&String::from_utf8_lossy(&data), dir) {
        return Err("SVG not drawn: a linked file is missing".into());
    }
    let opt = usvg::Options {
        resources_dir: Some(dir.to_path_buf()),
        fontdb: fonts(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_data(&data, &opt).map_err(|_| broken())?;
    let size = tree.size();
    let (w, h) = (size.width(), size.height());
    let k = SVG_SCALE.min(MAX_SIDE as f32 / w.max(h));
    let mut pixmap =
        tiny_skia::Pixmap::new((w * k).ceil() as u32, (h * k).ceil() as u32).ok_or_else(broken)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(k, k),
        &mut pixmap.as_mut(),
    );
    let png = pixmap.encode_png().map_err(|_| broken())?;
    Ok(Drawn {
        frames: vec![(png, 0)],
        natural: (w.ceil() as u32, h.ceil() as u32),
        h: pixmap.height(),
    })
}

#[cfg(test)]
mod tests;
