use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use ratatui::Terminal;
use ratatui::backend::{Backend, TestBackend};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};

use crate::app::App;
use crate::buffer::Buffer;
use crate::key::{KeyCode, KeyEvent, KeyModifiers};
use crate::theme::{self, Theme};

struct Web {
    app: App,
    term: Terminal<TestBackend>,
    theme: Theme,
    drawn_theme: String,
}

thread_local! {
    static WEB: RefCell<Option<Web>> = const { RefCell::new(None) };
    static IO: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static REPLAY: RefCell<Replay> = RefCell::new(Replay::default());
}

pub struct WasmOnce<T>(std::sync::OnceLock<T>);

unsafe impl<T> Sync for WasmOnce<T> {}

impl<T> WasmOnce<T> {
    pub const fn new() -> Self {
        Self(std::sync::OnceLock::new())
    }

    pub fn get_or_init(&self, f: impl FnOnce() -> T) -> &T {
        self.0.get_or_init(f)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn merl_buf(len: usize) -> *mut u8 {
    IO.with_borrow_mut(|io| {
        io.clear();
        io.resize(len, 0);
        io.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn merl_out_len() -> usize {
    IO.with_borrow(Vec::len)
}

fn input() -> String {
    IO.with_borrow(|io| String::from_utf8_lossy(io).into_owned())
}

fn output(s: String) -> *const u8 {
    IO.with_borrow_mut(|io| {
        *io = s.into_bytes();
        io.as_ptr()
    })
}

fn start(made: anyhow::Result<Web>) -> *const u8 {
    output(match made {
        Ok(w) => {
            WEB.set(Some(w));
            String::new()
        }
        Err(e) => format!("{e:#}"),
    })
}

fn grid<'a>(mut it: impl Iterator<Item = &'a str>) -> (u16, u16) {
    let cols = it.next().and_then(|n| n.parse().ok()).unwrap_or(120);
    let rows = it.next().and_then(|n| n.parse().ok()).unwrap_or(36);
    (cols, rows)
}

fn build(
    root: PathBuf,
    file: Option<&Path>,
    at: Option<(usize, usize)>,
    review: Option<crate::git::Review>,
    (cols, rows): (u16, u16),
) -> anyhow::Result<Web> {
    let (walked, files) = crate::tree::build(&root, false);
    let tree = match &review {
        Some(r) => {
            crate::tree::from_listing(&r.files.iter().map(|f| f.path.clone()).collect::<Vec<_>>())
        }
        None => walked,
    };
    let buf = match file {
        Some(p) => Buffer::load(p)?,
        None => Buffer::empty(),
    };
    let mut app = App::new(root, tree, files, buf, at);
    app.no_external();
    if let Some(r) = review {
        app.start_review(r);
    }
    let config = theme::Config::default();
    crate::configure(&mut app, &config, config.theme.clone());
    app.config = None;
    let theme = theme::load(&app.theme)?;
    Ok(Web {
        drawn_theme: app.theme.clone(),
        app,
        term: Terminal::new(TestBackend::new(cols, rows))?,
        theme,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn merl_open() -> *const u8 {
    let arg = input();
    let mut it = arg.split('\n');
    let root = PathBuf::from(it.next().unwrap_or("/"));
    let mut target = it.next().unwrap_or_default().splitn(3, ':');
    let file = target
        .next()
        .filter(|f| !f.is_empty())
        .map(|f| root.join(f));
    let file = file.as_deref();
    let line = target.next().and_then(|l| l.parse().ok());
    let col = target.next().and_then(|c| c.parse().ok()).unwrap_or(1);
    start(build(root, file, line.map(|l| (l, col)), None, grid(it)))
}

#[unsafe(no_mangle)]
pub extern "C" fn merl_review() -> *const u8 {
    let arg = input();
    let mut it = arg.split('\n');
    let root = PathBuf::from(it.next().unwrap_or("/"));
    let base = it.next().filter(|b| !b.is_empty()).map(str::to_owned);
    let size = grid(it);
    start(
        crate::git::Review::open(&root, None, base.as_deref())
            .and_then(|r| build(root, None, None, Some(r), size)),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn merl_tutor() -> *const u8 {
    let arg = input();
    let mut it = arg.split('\n');
    let dir = PathBuf::from(it.next().unwrap_or("/tutor"));
    let size = grid(it);
    start(crate::tutor::unpack(&dir).and_then(|()| {
        let mut w = build(dir.clone(), None, None, None, size)?;
        crate::start_tutor(&mut w.app, None, dir)?;
        Ok(w)
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn merl_theme() {
    let name = input();
    WEB.with_borrow_mut(|w| {
        if let Some(w) = w
            && theme::load(&name).is_ok()
        {
            w.app.theme = name;
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn merl_resize(cols: u16, rows: u16) {
    WEB.with_borrow_mut(|w| {
        if let Some(w) = w {
            w.term.backend_mut().resize(cols, rows);
            let Ok(()) = w.term.resize(Rect::new(0, 0, cols, rows));
        }
    });
}

const NAMED: [KeyCode; 15] = [
    KeyCode::Backspace,
    KeyCode::Enter,
    KeyCode::Left,
    KeyCode::Right,
    KeyCode::Up,
    KeyCode::Down,
    KeyCode::Home,
    KeyCode::End,
    KeyCode::PageUp,
    KeyCode::PageDown,
    KeyCode::Tab,
    KeyCode::BackTab,
    KeyCode::Delete,
    KeyCode::Insert,
    KeyCode::Esc,
];

fn key_code(code: u32) -> KeyCode {
    match code.checked_sub(0x11_0000) {
        Some(i) => NAMED.get(i as usize).copied().unwrap_or_else(|| {
            let f = (i as usize - NAMED.len() + 1).min(usize::from(u8::MAX));
            KeyCode::F(u8::try_from(f).unwrap_or(u8::MAX))
        }),
        None => KeyCode::Char(char::from_u32(code).unwrap_or('\0')),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn merl_key(code: u32, mods: u32) -> u32 {
    let mods = KeyModifiers::from_bits_truncate(u8::try_from(mods & 0xff).unwrap_or(0));
    let ev = KeyEvent::new(key_code(code), mods);
    WEB.with_borrow_mut(|w| {
        let Some(w) = w else { return 0 };
        let quit = w.app.key(ev);
        w.app.clipboard = None;
        settle(&mut w.app);
        u32::from(quit)
    })
}

fn settle(app: &mut App) -> bool {
    let mut dirty = false;
    if let Some(p) = &mut app.picker {
        dirty |= p.tick();
    }
    dirty |= app.tick();
    dirty |= app.tick_clock();
    if let Some(job) = app.search_tick() {
        dirty |= app.search_done(job.seq, job.items());
    }
    dirty | app.parse_more()
}

#[unsafe(no_mangle)]
pub extern "C" fn merl_tick() -> u32 {
    WEB.with_borrow_mut(|w| w.as_mut().map_or(0, |w| u32::from(settle(&mut w.app))))
}

#[unsafe(no_mangle)]
pub extern "C" fn merl_frame() -> *const u8 {
    let s = WEB.with_borrow_mut(|w| {
        let Some(w) = w else {
            return String::from("null");
        };
        crate::restyle(&mut w.app, &mut w.theme, &mut w.drawn_theme);
        let (app, theme) = (&mut w.app, &w.theme);
        if let Err(e) = w.term.draw(|f| crate::ui::draw(f, app, theme)) {
            return format!("{{\"error\":{}}}", json_str(&e.to_string()));
        }
        serialize(w.term.backend_mut(), theme)
    });
    output(s)
}

fn hex(c: Color) -> String {
    match c {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Reset => String::new(),
        Color::Indexed(i) => format!("i{i}"),
        c => format!("{c:?}").to_lowercase(),
    }
}

type Run = (String, String, String, u16);

fn serialize(b: &mut TestBackend, theme: &Theme) -> String {
    let cursor = match b.get_cursor_position() {
        Ok(p) if b.cursor_visible() => format!("[{},{}]", p.x, p.y),
        _ => "null".to_owned(),
    };
    let buf = b.buffer();
    let (w, h) = (buf.area.width, buf.area.height);
    let (bg, fg, accent) = (hex(theme.bg), hex(theme.fg), hex(theme.accent));
    let mut out = format!(
        "{{\"w\":{w},\"h\":{h},\"bg\":\"{bg}\",\"fg\":\"{fg}\",\"accent\":\"{accent}\",\"cursor\":{cursor},\"rows\":["
    );
    let shown = (Modifier::BOLD
        | Modifier::ITALIC
        | Modifier::UNDERLINED
        | Modifier::REVERSED
        | Modifier::DIM
        | Modifier::CROSSED_OUT)
        .bits();
    for y in 0..h {
        if y > 0 {
            out.push(',');
        }
        out.push('[');
        let mut runs: Vec<Run> = Vec::new();
        let mut skip = 0;
        for x in 0..w {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            let c = &buf[(x, y)];
            let sym = c.symbol();
            skip = crate::wrap::width(sym).saturating_sub(1);
            let (fg, bg, m) = (hex(c.fg), hex(c.bg), c.modifier.bits() & shown);
            match runs.last_mut() {
                Some((t, f, k, mm)) if *f == fg && *k == bg && *mm == m => t.push_str(sym),
                _ => runs.push((sym.to_owned(), fg, bg, m)),
            }
        }
        for (i, (t, fg, bg, m)) in runs.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            _ = write!(out, "[{},\"{fg}\",\"{bg}\",{m}]", json_str(t));
        }
        out.push(']');
    }
    out.push_str("]}");
    out
}

fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => _ = write!(o, "\\u{:04x}", c as u32),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

#[unsafe(no_mangle)]
pub extern "C" fn merl_state() -> *const u8 {
    let s = WEB.with_borrow(|w| {
        let Some(w) = w else {
            return String::from("null");
        };
        let a = &w.app;
        let text = a.buf.lines.get(a.line).map_or("", String::as_str);
        let review = a.review.as_ref().map_or_else(
            || String::from("null"),
            |r| {
                format!(
                    "{{\"files\":{},\"viewed\":{},\"renamed\":{}}}",
                    r.files.len(),
                    r.files.iter().filter(|f| a.viewed.contains_key(&f.path)).count(),
                    a.renamed_here().is_some()
                )
            },
        );
        format!(
            "{{\"file\":{},\"line\":{},\"col\":{},\"mode\":{},\"text\":{},\"message\":{},\"review\":{review}}}",
            json_str(&a.rel_path()),
            a.line + 1,
            a.col + 1,
            json_str(&format!("{:?}", a.mode)),
            json_str(text),
            json_str(&a.message)
        )
    });
    output(s)
}

#[derive(Default)]
struct Replay {
    root: Option<String>,
    answers: Option<HashMap<String, (bool, Vec<u8>)>>,
    misses: Vec<String>,
}

impl Replay {
    fn load(&mut self, root: &str) {
        if self.root.as_deref() == Some(root) {
            return;
        }
        self.answers = std::fs::read(format!("{root}/.git/merl-replay"))
            .ok()
            .map(|b| parse_replay(&b));
        self.root = Some(root.to_owned());
    }
}

fn parse_replay(b: &[u8]) -> HashMap<String, (bool, Vec<u8>)> {
    let mut map = HashMap::new();
    let mut i = 0;
    let line = |i: &mut usize| -> Option<String> {
        let end = b.get(*i..)?.iter().position(|&c| c == b'\n')? + *i;
        let s = String::from_utf8_lossy(b.get(*i..end)?).into_owned();
        *i = end + 1;
        Some(s)
    };
    while i < b.len() {
        let (Some(key), Some(head)) = (line(&mut i), line(&mut i)) else {
            break;
        };
        let mut h = head.split(' ');
        let ok = h.next() == Some("0");
        let n: usize = h.next().and_then(|n| n.parse().ok()).unwrap_or(0);
        let out = b.get(i..i + n).unwrap_or_default().to_vec();
        i += n + 1;
        map.insert(key, (ok, out));
    }
    map
}

#[unsafe(no_mangle)]
pub extern "C" fn merl_git_misses() -> *const u8 {
    let s = REPLAY.with_borrow_mut(|r| std::mem::take(&mut r.misses).join("\n"));
    output(s)
}

pub struct GitStatus(bool);

impl GitStatus {
    pub fn success(&self) -> bool {
        self.0
    }
}

pub struct GitOutput {
    pub status: GitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub struct GitChild {
    pub stdin: Option<std::io::Sink>,
    args: Vec<String>,
}

impl GitChild {
    pub fn wait_with_output(self) -> std::io::Result<GitOutput> {
        Err(no_git(&self.args))
    }
}

fn no_git(args: &[String]) -> std::io::Error {
    std::io::Error::other(format!("no git in the browser: git {}", args.join(" ")))
}

#[derive(Default)]
pub struct GitCommand {
    args: Vec<String>,
}

impl GitCommand {
    pub fn new(_: &str) -> Self {
        Self::default()
    }

    pub fn arg(&mut self, a: impl AsRef<OsStr>) -> &mut Self {
        self.args.push(a.as_ref().to_string_lossy().into_owned());
        self
    }

    pub fn args<I: IntoIterator<Item = S>, S: AsRef<OsStr>>(&mut self, a: I) -> &mut Self {
        for x in a {
            self.arg(x);
        }
        self
    }

    pub fn env(&mut self, _: impl AsRef<OsStr>, _: impl AsRef<OsStr>) -> &mut Self {
        self
    }

    pub fn stdin(&mut self, _: std::process::Stdio) -> &mut Self {
        self
    }

    pub fn stdout(&mut self, _: std::process::Stdio) -> &mut Self {
        self
    }

    pub fn stderr(&mut self, _: std::process::Stdio) -> &mut Self {
        self
    }

    pub fn spawn(&self) -> std::io::Result<GitChild> {
        Err(no_git(&self.args))
    }

    pub fn status(&self) -> std::io::Result<GitStatus> {
        Err(no_git(&self.args))
    }

    pub fn output(&self) -> std::io::Result<GitOutput> {
        let mut root = String::new();
        let mut key = Vec::new();
        let mut it = self.args.iter();
        while let Some(a) = it.next() {
            match a.as_str() {
                "-C" => root = it.next().cloned().unwrap_or_default(),
                "-c" => {
                    it.next();
                }
                _ if key.is_empty() && a.starts_with('-') => {}
                _ => key.push(a.as_str()),
            }
        }
        let prefix = format!("{}/", root.trim_end_matches('/'));
        let key: Vec<&str> = key
            .into_iter()
            .map(|a| a.strip_prefix(&prefix).unwrap_or(a))
            .collect();
        let key = key.join("\u{1f}");
        let hit = REPLAY.with_borrow_mut(|r| {
            r.load(&root);
            let hit = r.answers.as_ref()?.get(&key).cloned();
            if hit.is_none() {
                r.misses.push(key);
            }
            Some(hit)
        });
        let (ok, stdout) = hit.ok_or_else(|| no_git(&self.args))?.unwrap_or_default();
        Ok(GitOutput {
            status: GitStatus(ok),
            stdout,
            stderr: Vec::new(),
        })
    }
}
