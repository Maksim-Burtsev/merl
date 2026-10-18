use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write as _;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use merman::runtime::RuntimePolicy;
use merman::svg::export::{RasterOptions, RasterSizeLimit};
use merman::svg::{RootBackgroundPostprocessor, SvgPipeline};
use merman::{OperationControl, PngRequest, RenderOutput, RenderRequest, Renderer, SvgRequest};
use ratatui::style::Color;

use crate::picture::{self, Drawn};
use crate::theme::Theme;

const SCALE: f32 = 2.0;
pub const FONT_PX: f32 = 16.0;
pub const TEXT_IN_CELL: f32 = 0.85;
const FILE: u64 = 0;
const KEEP_FILES: usize = 4;
const LEAST_SHRINK: f32 = 0.5;
const RENDERS: usize = 2;
const MAX_SIDE: u32 = 8192;
pub const THREAD: &str = "mermaid";
const MERMAN_THREAD: &str = "merman-svg-backend";

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

#[derive(Clone, PartialEq, Eq, Debug)]
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
    recent: VecDeque<Key>,
    used: HashSet<Key>,
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
            self.drop_pics(&key);
        }
        self.asked.retain(keep);
        self.queue.retain(|j| keep(&j.key));
    }

    pub fn fit(&mut self, src: &str, room: usize) -> Option<(u16, u16)> {
        let cell = self.cell?;
        let natural = if let Some(size) = self.sizes.get(&hash(src)) {
            (*size)?
        } else {
            self.ask(self.key(src), Src::Diagram(src.to_owned()));
            return None;
        };
        cells(natural, cell, room)
    }

    pub fn pic(&mut self, src: &str) -> Option<Arc<Pic>> {
        self.cell?;
        let key = self.key(src);
        if let Some(pic) = self.pics.get(&key) {
            pic.as_ref().map(|frames| Arc::clone(&frames[0]))
        } else {
            self.ask(key, Src::Diagram(src.to_owned()));
            None
        }
    }

    pub fn cell(&self) -> Option<(u32, u32)> {
        self.cell
    }

    pub fn file_size(&mut self, path: &Path) -> Option<(u32, u32)> {
        self.cell?;
        let key = file_key(path)?;
        if let Some(size) = self.sizes.get(&key.src) {
            *size
        } else {
            self.ask(key, Src::File(path.to_path_buf()));
            None
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
        if let Some(i) = self.recent.iter().position(|k| *k == key) {
            self.recent.remove(i);
        }
        self.recent.push_front(key);
        self.used.insert(key);
        if frames.len() < 2 {
            return frames.first().cloned();
        }
        let ms: Vec<u32> = frames.iter().map(|f| f.ms).collect();
        let start = *self.started.entry(key).or_insert_with(Instant::now);
        let (i, left) = picture::frame_at(
            &ms,
            u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
        );
        let left = Duration::from_millis(left.max(10));
        self.wake = Some(self.wake.map_or(left, |w| w.min(left)));
        Some(Arc::clone(&frames[i]))
    }

    pub fn begin(&mut self) {
        self.want.clear();
        self.wake = None;
        let keep = KEEP_FILES.max(self.used.len());
        self.used.clear();
        while self.recent.len() > keep {
            if let Some(key) = self.recent.pop_back() {
                self.drop_pics(&key);
                self.asked.remove(&key);
                self.started.remove(&key);
            }
        }
    }

    fn drop_pics(&mut self, key: &Key) {
        for pic in self.pics.remove(key).flatten().into_iter().flatten() {
            if self.sent.remove(&pic.id) {
                self.stale.push(pic.id);
            }
        }
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
            _ = write!(buf, "\x1b_Ga=d,d=I,i={id},q=2\x1b\\");
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
            _ = write!(
                buf,
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
        let name = std::thread::current().name().map(str::to_owned);
        if !matches!(
            name.as_deref(),
            Some(THREAD | MERMAN_THREAD | crate::markdown::data::THREAD)
        ) {
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

#[cfg(target_family = "wasm")]
fn cell() -> Option<(u32, u32)> {
    None
}

#[cfg(not(target_family = "wasm"))]
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

#[expect(
    clippy::cast_possible_truncation,
    reason = "pixel and cell counts sit far below 2^24; the cell counts are clamped to 1..=u16::MAX before `as`"
)]
#[expect(
    clippy::cast_precision_loss,
    reason = "pixel and cell counts sit far below 2^24; the cell counts are clamped to 1..=u16::MAX before `as`"
)]
#[expect(
    clippy::cast_sign_loss,
    reason = "pixel and cell counts sit far below 2^24; the cell counts are clamped to 1..=u16::MAX before `as`"
)]
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
        .chain([r#""fontSize":"16px""#.to_owned()])
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
        return src.to_owned();
    }
    format!("{colours}\n{src}")
}

pub fn render(job: Job) -> Done {
    let Job { key, src, colours } = job;
    let drawn = std::panic::catch_unwind(|| match &src {
        Src::Diagram(src) => draw(&with_colours(src, &colours)).ok_or_else(String::new),
        Src::File(path) => picture::decode(path),
    })
    .unwrap_or_else(|_| Err("not drawn".into()));
    Done { key, drawn }
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

#[expect(
    clippy::cast_possible_truncation,
    reason = "a diagram's sides in pixels sit far below 2^24, and `as` saturates a float into u32"
)]
#[expect(
    clippy::cast_sign_loss,
    reason = "a diagram's sides in pixels sit far below 2^24, and `as` saturates a float into u32"
)]
fn draw(src: &str) -> Option<Drawn> {
    let mut pipeline = SvgPipeline::resvg_safe();
    pipeline.push_postprocessor(RootBackgroundPostprocessor::new("transparent"));
    let request = PngRequest {
        svg: SvgRequest {
            pipeline: Some(pipeline),
            ..SvgRequest::default()
        },
        options: RasterOptions {
            scale: SCALE,
            size_limit: RasterSizeLimit::new(Some(MAX_SIDE), Some(MAX_SIDE), None),
            background: Some("transparent".into()),
            ..RasterOptions::default()
        },
    };
    let RenderOutput::Png(Some(out)) = renderer()
        .render(RenderRequest::png(src, OperationControl::new(), request))
        .ok()?
    else {
        return None;
    };
    let plan = out.plan;
    let natural = |px: f64| (px / f64::from(SCALE)).ceil() as u32;
    (out.bytes.starts_with(b"\x89PNG") && !plan.limited).then(|| Drawn {
        natural: (
            natural(plan.requested_width_px),
            natural(plan.requested_height_px),
        ),
        h: plan.height_px,
        frames: vec![(out.bytes, 0)],
    })
}

fn renderer() -> Renderer {
    let now = chrono::Local::now();
    let clock = RuntimePolicy::deterministic().with_fixed_unix_millis(now.timestamp_millis());
    let policy = clock
        .clone()
        .try_with_fixed_local_offset_minutes(now.offset().local_minus_utc() / 60)
        .unwrap_or(clock);
    Renderer::new().with_runtime_policy(policy)
}

fn transmit(buf: &mut String, pic: &Pic) {
    let data = crate::base64(&pic.png);
    let chunks: Vec<&[u8]> = data.as_bytes().chunks(4096).collect();
    for (i, chunk) in chunks.iter().enumerate() {
        let more = u8::from(i + 1 < chunks.len());
        let chunk = std::str::from_utf8(chunk).unwrap_or_default();
        if i == 0 {
            _ = write!(
                *buf,
                "\x1b_Ga=t,f=100,i={},q=2,m={more};{chunk}\x1b\\",
                pic.id
            );
        } else {
            _ = write!(*buf, "\x1b_Gm={more};{chunk}\x1b\\");
        }
    }
}

#[cfg(test)]
mod tests;
