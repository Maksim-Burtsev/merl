use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use merman::render::{
    LayoutOptions, RootBackgroundPostprocessor, SvgPipeline, SvgRenderOptions,
    VendoredFontMetricsTextMeasurer,
};
use merman::{Engine, ParseOptions};
use ratatui::style::Color;

use crate::picture::{self, Drawn};
use crate::theme::Theme;

const SCALE: f32 = 2.0;
pub const FONT_PX: f32 = 16.0;
pub const TEXT_IN_CELL: f32 = 0.85;
const FILE: u64 = 0;
const LEAST_SHRINK: f32 = 0.5;
const RENDERS: usize = 2;
pub const THREAD: &str = "mermaid";

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Key {
    src: u64,
    colours: u64,
}

pub enum Src {
    Diagram(String),
    File(PathBuf),
}

pub struct Job {
    key: Key,
    src: Src,
    colours: String,
}

pub struct Done {
    key: Key,
    drawn: Result<Drawn, String>,
}

pub struct Pic {
    id: u32,
    png: Vec<u8>,
    h: u32,
    ms: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Place {
    pub id: u32,
    pub placement: u32,
    pub x: u16,
    pub y: u16,
    pub cols: u16,
    pub rows: u16,
    pub crop_y: u32,
    pub crop_h: u32,
}

#[derive(Default)]
pub struct Diagrams {
    cell: Option<(u32, u32)>,
    colours: String,
    pics: HashMap<Key, Option<Vec<Arc<Pic>>>>,
    sizes: HashMap<u64, Option<(u32, u32)>>,
    why: HashMap<u64, String>,
    started: HashMap<Key, Instant>,
    pub wake: Option<Duration>,
    pub light: bool,
    asked: HashSet<Key>,
    queue: VecDeque<Job>,
    running: usize,
    next_id: u32,
    pub laid: u64,
    pub want: Vec<Place>,
    shown: Vec<Place>,
    sent: HashSet<u32>,
    stale: Vec<u32>,
}

impl Diagrams {
    pub fn detect() -> Self {
        let env = |k: &str| std::env::var(k).ok();
        Self::with_cell(supported(&env).then(cell).flatten())
    }

    pub fn with_cell(cell: Option<(u32, u32)>) -> Self {
        Self {
            cell,
            ..Self::default()
        }
    }

    pub fn on(&self) -> bool {
        self.cell.is_some()
    }

    pub fn resized(&mut self) {
        self.shown.clear();
        if self.on() {
            let now = cell();
            if now.is_some() && now != self.cell {
                self.cell = now;
                self.laid += 1;
            }
        }
    }

    pub fn theme(&mut self, theme: &Theme, label_bg: Color) {
        self.light = theme.light;
        if !self.on() {
            return;
        }
        let colours = colours(theme, label_bg);
        if colours == self.colours {
            return;
        }
        let now = hash(&colours);
        self.colours = colours;
        let keep = |k: &Key| k.colours == now || k.colours == FILE;
        let old: Vec<Key> = self.pics.keys().filter(|k| !keep(k)).copied().collect();
        for key in old {
            for pic in self.pics.remove(&key).flatten().into_iter().flatten() {
                if self.sent.remove(&pic.id) {
                    self.stale.push(pic.id);
                }
            }
        }
        self.asked.retain(keep);
        self.queue.retain(|j| keep(&j.key));
    }

    pub fn fit(&mut self, src: &str, room: usize) -> Option<(u16, u16)> {
        let cell = self.cell?;
        let natural = match self.sizes.get(&hash(src)) {
            Some(size) => (*size)?,
            None => {
                self.ask(self.key(src), Src::Diagram(src.to_string()));
                return None;
            }
        };
        cells(natural, cell, room)
    }

    pub fn pic(&mut self, src: &str) -> Option<Arc<Pic>> {
        self.cell?;
        let key = self.key(src);
        match self.pics.get(&key) {
            Some(pic) => pic.as_ref().map(|frames| frames[0].clone()),
            None => {
                self.ask(key, Src::Diagram(src.to_string()));
                None
            }
        }
    }

    pub fn cell(&self) -> Option<(u32, u32)> {
        self.cell
    }

    pub fn file_size(&mut self, path: &Path) -> Option<(u32, u32)> {
        self.cell?;
        let key = file_key(path)?;
        match self.sizes.get(&key.src) {
            Some(size) => *size,
            None => {
                self.ask(key, Src::File(path.to_path_buf()));
                None
            }
        }
    }

    pub fn known_size(&self, path: &Path) -> Option<(u32, u32)> {
        self.sizes.get(&file_key(path)?.src).copied().flatten()
    }

    pub fn file_failed(&self, path: &Path) -> Option<&str> {
        self.cell?;
        self.why.get(&file_key(path)?.src).map(String::as_str)
    }

    pub fn file_pic(&mut self, path: &Path) -> Option<Arc<Pic>> {
        self.cell?;
        let key = file_key(path)?;
        let Some(frames) = self.pics.get(&key) else {
            self.ask(key, Src::File(path.to_path_buf()));
            return None;
        };
        let frames = frames.as_ref()?;
        if frames.len() < 2 {
            return frames.first().cloned();
        }
        let ms: Vec<u32> = frames.iter().map(|f| f.ms).collect();
        let start = *self.started.entry(key).or_insert_with(Instant::now);
        let (i, left) = picture::frame_at(&ms, start.elapsed().as_millis() as u64);
        let left = Duration::from_millis(left.max(10));
        self.wake = Some(self.wake.map_or(left, |w| w.min(left)));
        Some(frames[i].clone())
    }

    fn key(&self, src: &str) -> Key {
        Key {
            src: hash(src),
            colours: hash(&self.colours),
        }
    }

    fn ask(&mut self, key: Key, src: Src) {
        if self.asked.insert(key) {
            self.queue.push_back(Job {
                key,
                src,
                colours: self.colours.clone(),
            });
        }
    }

    pub fn jobs(&mut self) -> Vec<Job> {
        let n = RENDERS.saturating_sub(self.running).min(self.queue.len());
        self.running += n;
        self.queue.drain(..n).collect()
    }

    pub fn done(&mut self, done: Done) {
        self.running = self.running.saturating_sub(1);
        let size = done.drawn.as_ref().ok().map(|d| d.natural);
        if self.sizes.insert(done.key.src, size).is_none() {
            self.laid += 1;
        }
        let file = done.key.colours == FILE;
        if let (true, Err(why)) = (file, &done.drawn) {
            self.why.insert(done.key.src, why.clone());
        }
        if !file && done.key.colours != hash(&self.colours) {
            return;
        }
        let frames = done.drawn.ok().map(|d| {
            let h = d.h;
            d.frames
                .into_iter()
                .map(|(png, ms)| {
                    self.next_id += 1;
                    Arc::new(Pic {
                        id: self.next_id,
                        png,
                        h,
                        ms,
                    })
                })
                .collect()
        });
        self.pics.insert(done.key, frames);
    }

    pub fn flush(&mut self, out: &mut impl Write) -> io::Result<()> {
        if self.want == self.shown && self.stale.is_empty() {
            return Ok(());
        }
        let mut buf = String::from("\x1b7\x1b_Ga=d,d=a,q=2\x1b\\");
        for id in self.stale.drain(..) {
            buf += &format!("\x1b_Ga=d,d=I,i={id},q=2\x1b\\");
        }
        for p in &self.want {
            if !self.sent.contains(&p.id)
                && let Some(pic) = self
                    .pics
                    .values()
                    .flatten()
                    .flatten()
                    .find(|q| q.id == p.id)
            {
                transmit(&mut buf, pic);
                self.sent.insert(p.id);
            }
            buf += &format!(
                "\x1b[{};{}H\x1b_Ga=p,i={},p={},y={},h={},c={},r={},C=1,z=-1,q=2\x1b\\",
                p.y + 1,
                p.x + 1,
                p.id,
                p.placement,
                p.crop_y,
                p.crop_h,
                p.cols,
                p.rows
            );
        }
        buf += "\x1b8";
        out.write_all(buf.as_bytes())?;
        out.flush()?;
        self.shown = self.want.clone();
        Ok(())
    }

    pub fn clear(&self, out: &mut impl Write) -> io::Result<()> {
        if !self.sent.is_empty() || !self.stale.is_empty() {
            out.write_all(b"\x1b_Ga=d,d=A,q=2\x1b\\")?;
            out.flush()?;
        }
        Ok(())
    }
}

impl Pic {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn height(&self) -> u32 {
        self.h
    }
}

pub fn keep_panics_inside() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current().name() != Some(THREAD) {
            hook(info);
        }
    }));
}

pub fn supported(env: &dyn Fn(&str) -> Option<String>) -> bool {
    if ["TMUX", "STY", "ZELLIJ"].iter().any(|k| env(k).is_some()) {
        return false;
    }
    let term = env("TERM").unwrap_or_default();
    let program = env("TERM_PROGRAM").unwrap_or_default();
    matches!(term.as_str(), "xterm-kitty" | "xterm-ghostty")
        || matches!(program.as_str(), "ghostty" | "WezTerm")
        || env("KITTY_WINDOW_ID").is_some()
}

fn cell() -> Option<(u32, u32)> {
    let ws = ratatui::crossterm::terminal::window_size().ok()?;
    if ws.width == 0 || ws.height == 0 || ws.columns == 0 || ws.rows == 0 {
        return None;
    }
    Some((
        u32::from(ws.width) / u32::from(ws.columns),
        u32::from(ws.height) / u32::from(ws.rows),
    ))
}

pub fn cells(natural: (u32, u32), cell: (u32, u32), room: usize) -> Option<(u16, u16)> {
    let (cw, ch) = (cell.0.max(1) as f32, cell.1.max(1) as f32);
    let px = ch * TEXT_IN_CELL / FONT_PX;
    let (w, h) = (natural.0 as f32 * px, natural.1 as f32 * px);
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let k = (room as f32 * cw / w).min(1.0);
    if k < LEAST_SHRINK {
        return None;
    }
    let cols = (w * k / cw).ceil().max(1.0);
    let rows = (h * k / ch).ceil().max(1.0);
    Some((
        cols.min(f32::from(u16::MAX)) as u16,
        rows.min(f32::from(u16::MAX)) as u16,
    ))
}

fn hash(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

fn hex(c: Color) -> Option<String> {
    match c {
        Color::Rgb(r, g, b) => Some(format!("#{r:02x}{g:02x}{b:02x}")),
        _ => None,
    }
}

fn colours(theme: &Theme, label_bg: Color) -> String {
    let base = if theme.light { "default" } else { "dark" };
    let (Some(text), Some(line), Some(border), Some(fill), Some(bg)) = (
        hex(theme.fg),
        hex(theme.ghost_fg),
        hex(theme.accent),
        hex(theme.line_hl),
        hex(label_bg),
    ) else {
        return format!(r#"%%{{init: {{"theme":"{base}"}}}}%%"#);
    };
    let vars = [
        ("background", &bg),
        ("primaryColor", &fill),
        ("primaryTextColor", &text),
        ("primaryBorderColor", &border),
        ("lineColor", &line),
        ("textColor", &text),
        ("edgeLabelBackground", &bg),
        ("labelBackgroundColor", &bg),
        ("transitionColor", &line),
        ("transitionLabelColor", &text),
        ("stateLabelColor", &text),
        ("stateBkg", &fill),
        ("mainBkg", &fill),
        ("nodeBorder", &border),
        ("clusterBkg", &bg),
        ("actorBkg", &fill),
        ("actorBorder", &border),
        ("actorTextColor", &text),
        ("actorLineColor", &line),
        ("signalColor", &line),
        ("signalTextColor", &text),
        ("labelBoxBkgColor", &fill),
        ("labelBoxBorderColor", &border),
        ("labelTextColor", &text),
        ("loopTextColor", &text),
        ("noteBkgColor", &fill),
        ("noteTextColor", &text),
        ("noteBorderColor", &border),
        ("attributeBackgroundColorOdd", &fill),
        ("attributeBackgroundColorEven", &bg),
    ];
    let vars: Vec<String> = vars
        .iter()
        .map(|(k, v)| format!(r#""{k}":"{v}""#))
        .chain([r#""fontSize":"16px""#.to_string()])
        .collect();
    format!(
        r#"%%{{init: {{"theme":"{base}","themeVariables":{{{}}}}}}}%%"#,
        vars.join(",")
    )
}

pub fn with_colours(src: &str, colours: &str) -> String {
    let mut lines = src.lines();
    if src.trim_start().starts_with("---") {
        let mut head = Vec::new();
        for (i, line) in lines.by_ref().enumerate() {
            head.push(line);
            if i > 0 && line.trim() == "---" {
                let rest: Vec<&str> = lines.collect();
                return format!("{}\n{colours}\n{}", head.join("\n"), rest.join("\n"));
            }
        }
        return src.to_string();
    }
    format!("{colours}\n{src}")
}

pub fn render(job: Job) -> Done {
    let drawn = std::panic::catch_unwind(|| match &job.src {
        Src::Diagram(src) => draw(&with_colours(src, &job.colours)).ok_or_else(String::new),
        Src::File(path) => picture::decode(path),
    })
    .unwrap_or_else(|_| Err("not drawn".into()));
    Done {
        key: job.key,
        drawn,
    }
}

fn file_key(path: &Path) -> Option<Key> {
    let meta = std::fs::metadata(path).ok()?;
    let mut h = DefaultHasher::new();
    (path, meta.len(), meta.modified().ok()).hash(&mut h);
    Some(Key {
        src: h.finish(),
        colours: FILE,
    })
}

fn draw(src: &str) -> Option<Drawn> {
    let layout = LayoutOptions {
        viewport_width: 800.0,
        viewport_height: 600.0,
        text_measurer: Arc::new(VendoredFontMetricsTextMeasurer::default()),
        math_renderer: None,
        use_manatee_layout: true,
    };
    let mut pipeline = SvgPipeline::parity();
    pipeline.push_postprocessor(RootBackgroundPostprocessor::new("transparent"));
    let svg = merman::render::render_svg_with_pipeline_sync(
        &Engine::new(),
        src,
        ParseOptions::default(),
        &layout,
        &SvgRenderOptions::default(),
        &pipeline,
    )
    .ok()??;
    let svg = merman::render::svg_resvg_safe(&svg).ok()?;
    let (vw, vh) = view_box(&svg)?;
    let options = merman::render::raster::RasterOptions {
        scale: SCALE,
        background: Some("transparent".into()),
        ..Default::default()
    };
    let png = merman::render::raster::svg_to_png(&svg, &options).ok()?;
    let size = |at: usize| png.get(at..at + 4)?.try_into().ok().map(u32::from_be_bytes);
    let (w, h) = (size(16)?, size(20)?);
    let whole = w as f32 >= (vw * SCALE).floor() - 1.0 && h as f32 >= (vh * SCALE).floor() - 1.0;
    (png.starts_with(b"\x89PNG") && whole).then(|| Drawn {
        frames: vec![(png, 0)],
        natural: (vw.ceil() as u32, vh.ceil() as u32),
        h,
    })
}

fn view_box(svg: &str) -> Option<(f32, f32)> {
    let at = svg.find("viewBox=\"")? + "viewBox=\"".len();
    let rest = &svg[at..];
    let nums: Vec<f32> = rest[..rest.find('"')?]
        .split([' ', ','])
        .filter(|n| !n.is_empty())
        .filter_map(|n| n.parse().ok())
        .collect();
    match nums[..] {
        [_, _, w, h] if w > 0.0 && h > 0.0 => Some((w, h)),
        _ => None,
    }
}

fn transmit(buf: &mut String, pic: &Pic) {
    let data = crate::base64(&pic.png);
    let chunks: Vec<&[u8]> = data.as_bytes().chunks(4096).collect();
    for (i, chunk) in chunks.iter().enumerate() {
        let more = u8::from(i + 1 < chunks.len());
        let chunk = std::str::from_utf8(chunk).unwrap_or_default();
        if i == 0 {
            *buf += &format!("\x1b_Ga=t,f=100,i={},q=2,m={more};{chunk}\x1b\\", pic.id);
        } else {
            *buf += &format!("\x1b_Gm={more};{chunk}\x1b\\");
        }
    }
}

#[cfg(test)]
mod tests;
