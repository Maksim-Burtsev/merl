//! PROTOTYPE for #194 — throwaway, lives on the `prototype/drill-session` branch only.
//!
//! `merl --drill [N]` over the #190 task pool, drawn three ways. `MERL_DRILL` picks one:
//!
//! - `A` the tutor's panel: the task in the three rows above the status bar, a ticking clock,
//!   a miss names the key at once and the task comes back two tasks later, the summary in the
//!   panel.
//! - `B` one line on top: no clock, a miss says `miss` and asks the same task again, the key is
//!   named on the second miss, the summary is printed to the shell after merl exits.
//! - `C` a log on the right: every answer with its time, a miss names the key and asks the same
//!   task again right away, the summary in a card over the code.
//!
//! Opening `?` during a task makes it a miss in every variant: it is how you give up.
//!
//!     MERL_DRILL=C cargo run --release -- --drill

use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, Mode};
use crate::theme::Theme;
use crate::tutor::task_pool_prototype::{POOL, Task, chord, fresh};

/// A correct answer slower than this is weak (#191's cap).
const SLOW: f32 = 2.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variant {
    A,
    B,
    C,
}

pub struct Drill {
    pub variant: Variant,
    /// The unpacked sample project, removed when merl exits.
    pub dir: PathBuf,
    /// Tasks still to ask, as indexes into `POOL`.
    queue: VecDeque<usize>,
    /// Tasks asked so far, a re-ask later in the session included, the same task again not.
    asked: usize,
    n: usize,
    cur: Option<Cur>,
    log: Vec<Attempt>,
    started: Instant,
    pub over: bool,
}

struct Cur {
    task: usize,
    since: Instant,
    /// The task's own key was pressed.
    hit: bool,
    /// `?` was opened.
    looked: bool,
    /// Misses on this task in a row, for B and C, which ask it again at once.
    misses: u8,
    /// The text names the key.
    named: bool,
}

struct Attempt {
    task: usize,
    secs: f32,
    miss: bool,
    named: bool,
}

impl Drill {
    pub fn new(dir: PathBuf, n: usize) -> Self {
        let variant = match std::env::var("MERL_DRILL").as_deref() {
            Ok("B" | "b") => Variant::B,
            Ok("C" | "c") => Variant::C,
            _ => Variant::A,
        };
        Drill {
            variant,
            dir,
            // First run, no stats: the placement pass, one task per key in order.
            queue: (0..POOL.len()).collect(),
            asked: 0,
            n,
            cur: None,
            log: Vec::new(),
            started: Instant::now(),
            over: false,
        }
    }

    fn total(&self) -> usize {
        (self.asked + self.queue.len()).min(self.n)
    }

    /// A clock on screen that needs a frame every tick.
    pub fn ticks(&self) -> bool {
        self.variant == Variant::A && self.cur.is_some()
    }
}

pub fn start(app: &mut App) {
    next(app);
}

/// Called after every key. Returns `true` when merl should quit (B's end).
pub fn check(app: &mut App, key: KeyEvent) -> bool {
    let Some(idx) = app.drill.as_ref().and_then(|d| d.cur.as_ref()).map(|c| c.task) else {
        return false;
    };
    let task = &POOL[idx];
    let done = (task.done)(app);
    let help = app.mode == Mode::Help;
    let d = app.drill.as_mut().unwrap();
    let cur = d.cur.as_mut().unwrap();
    cur.hit |= pressed(key, task.answer);
    cur.looked |= help;
    if !done {
        return false;
    }
    let secs = cur.since.elapsed().as_secs_f32();
    let miss = !cur.hit || cur.looked;
    let (named, misses) = (cur.named, cur.misses + u8::from(miss));
    d.log.push(Attempt {
        task: idx,
        secs,
        miss,
        named,
    });
    let name = key_name(task);
    let again = match (d.variant, miss) {
        (_, false) => None,
        (Variant::A, true) => {
            // Asked once more, two tasks later, without the key.
            if d.log.iter().filter(|a| a.task == idx).count() == 1 {
                let at = d.queue.len().min(2);
                d.queue.insert(at, idx);
            }
            None
        }
        (Variant::B, true) => (misses < 3).then_some((misses, misses >= 2)),
        (Variant::C, true) => (misses < 2).then_some((misses, true)),
    };
    let message = match (d.variant, miss) {
        (Variant::A, false) => format!("\u{2713} {secs:.1} s"),
        (Variant::A, true) => format!("\u{2717} {name}"),
        (Variant::B, false) => "\u{2713}".into(),
        // B's strip and C's log say it.
        (Variant::B | Variant::C, _) => String::new(),
    };
    let quit = match again {
        Some((misses, named)) => {
            run(app, idx, misses, named);
            false
        }
        None => next(app),
    };
    app.message = message;
    quit
}

fn next(app: &mut App) -> bool {
    let d = app.drill.as_mut().unwrap();
    let idx = if d.asked < d.n { d.queue.pop_front() } else { None };
    match idx {
        Some(idx) => {
            run(app, idx, 0, false);
            false
        }
        None => {
            // The last task's picker or selection would sit over the summary.
            let mut d = app.drill.take().unwrap();
            fresh(app);
            let store = app.root.join("store.py");
            app.jump_to(&store, 1);
            d.cur = None;
            d.over = true;
            let quit = d.variant == Variant::B;
            app.drill = Some(d);
            quit
        }
    }
}

/// A fresh sample project and a fresh `App`, then the task's own start, pressed silently.
fn run(app: &mut App, idx: usize, misses: u8, named: bool) {
    let mut d = app.drill.take().unwrap();
    fresh(app);
    (POOL[idx].setup)(app);
    app.message.clear();
    if misses == 0 {
        d.asked += 1;
    }
    d.cur = Some(Cur {
        task: idx,
        since: Instant::now(),
        hit: false,
        looked: false,
        misses,
        named,
    });
    app.drill = Some(d);
}

/// The key the task trains, as `KEYS` names it: `Alt+Backspace`, `PgDn`, `d`.
fn key_name(task: &Task) -> &'static str {
    task.key.rsplit(": ").next().unwrap()
}

/// `key` is the first chord of `answer`.
fn pressed(key: KeyEvent, answer: &str) -> bool {
    let first = match answer.find('>').filter(|_| answer.starts_with('<')) {
        Some(end) => chord(&answer[1..end]),
        None => (
            KeyCode::Char(answer.chars().next().unwrap()),
            KeyModifiers::NONE,
        ),
    };
    let mut mods = key.modifiers;
    if matches!(key.code, KeyCode::Char(_)) {
        mods.remove(KeyModifiers::SHIFT);
    }
    (key.code, mods) == first
}

/// The keys to train more, the weakest first: a miss, then a slow answer.
fn weak(d: &Drill) -> Vec<(&'static str, String, &'static str)> {
    let mut rows: Vec<(u32, f32, usize)> = Vec::new();
    for (idx, _) in POOL.iter().enumerate() {
        let tries: Vec<&Attempt> = d.log.iter().filter(|a| a.task == idx).collect();
        let misses = tries.iter().filter(|a| a.miss).count() as u32;
        let slow = tries
            .iter()
            .filter(|a| !a.miss)
            .map(|a| a.secs)
            .last()
            .unwrap_or(0.0);
        if misses > 0 || slow > SLOW {
            rows.push((misses, slow, idx));
        }
    }
    rows.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.total_cmp(&a.1)));
    rows.into_iter()
        .map(|(misses, slow, idx)| {
            let verdict = match misses {
                0 => format!("{slow:.1} s"),
                1 => "miss".to_string(),
                n => format!("{n} misses"),
            };
            (key_name(&POOL[idx]), verdict, POOL[idx].title)
        })
        .collect()
}

fn elapsed(d: &Drill) -> String {
    let s = d.started.elapsed().as_secs();
    format!("{}:{:02}", s / 60, s % 60)
}

fn text(cur: &Cur) -> &'static str {
    let task = &POOL[cur.task];
    if cur.named { task.tutor } else { task.drill }
}

/// B: what merl prints once the terminal is back.
pub fn summary(d: &Drill) -> String {
    let mut out = format!("drill: {} tasks in {}\n", d.asked, elapsed(d));
    let rows = weak(d);
    if rows.is_empty() {
        out.push_str("no weak keys\n");
    }
    for (name, verdict, title) in rows {
        out.push_str(&format!("  {name:<16} {verdict:<10} {title}\n"));
    }
    out
}

/// A: rows of the panel above the status bar.
pub fn panel_h(d: &Drill) -> u16 {
    if d.over { weak(d).len().max(1) as u16 + 3 } else { 3 }
}

/// A: the tutor's panel.
pub fn draw_panel(frame: &mut Frame, d: &Drill, theme: &Theme, area: Rect, base: Style) {
    let head = Style::new()
        .bg(theme.status_bg)
        .fg(theme.status_fg)
        .add_modifier(Modifier::BOLD);
    let mut lines = Vec::new();
    match &d.cur {
        Some(cur) => {
            let title = format!(
                " Drill {}/{} \u{00b7} {:.1} s",
                d.asked,
                d.total(),
                cur.since.elapsed().as_secs_f32()
            );
            lines.push(bar(title, area.width, head));
            lines.push(Line::from(Span::styled(format!(" {}", text(cur)), base)));
        }
        None => {
            let title = format!(" Drill \u{2713} {} tasks in {}", d.asked, elapsed(d));
            lines.push(bar(title, area.width, head));
            lines.extend(weak_lines(d, theme, base));
            lines.push(Line::from(Span::styled(" `q` quits.", base)));
        }
    }
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }).style(base),
        area,
    );
}

/// B: one line over the code.
pub fn draw_strip(frame: &mut Frame, d: &Drill, theme: &Theme, area: Rect) {
    let style = Style::new().bg(theme.status_bg).fg(theme.status_fg);
    let Some(cur) = &d.cur else { return };
    let mut spans = vec![Span::styled(
        format!(" {}/{}  ", d.asked, d.total()),
        style.add_modifier(Modifier::BOLD),
    )];
    if cur.misses > 0 {
        spans.push(Span::styled("miss  ", style.fg(theme.accent)));
    }
    spans.push(Span::styled(text(cur), style));
    frame.render_widget(Paragraph::new(Line::from(spans)).style(style), area);
}

/// C: the task and the log of the session, beside the code.
pub fn draw_side(frame: &mut Frame, d: &Drill, theme: &Theme, area: Rect, base: Style) {
    let block = Block::new()
        .borders(Borders::LEFT)
        .border_style(base.fg(theme.gutter_fg))
        .style(base);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let w = inner.width.saturating_sub(2) as usize;
    let bold = base.add_modifier(Modifier::BOLD);
    let mut lines = vec![Line::from(Span::styled(
        format!(" Drill {}/{}", d.asked, d.total()),
        bold,
    ))];
    lines.push(Line::default());
    if let Some(cur) = &d.cur {
        let style = if cur.named {
            base.fg(theme.accent)
        } else {
            base
        };
        for l in textwrap(text(cur), w) {
            lines.push(Line::from(Span::styled(format!(" {l}"), style)));
        }
        lines.push(Line::default());
    }
    for a in &d.log {
        let (mark, style) = match (a.miss, a.named) {
            (true, _) => ("\u{2717}", base.fg(theme.accent)),
            (false, true) => ("\u{21bb}", base),
            (false, false) => ("\u{2713}", base.fg(theme.ghost_fg)),
        };
        let time = if a.miss {
            "miss".to_string()
        } else {
            format!("{:.1} s", a.secs)
        };
        let name = key_name(&POOL[a.task]);
        let pad = w.saturating_sub(2 + name.len() + time.len());
        lines.push(Line::from(Span::styled(
            format!(" {mark} {name}{}{time}", " ".repeat(pad)),
            style,
        )));
    }
    // The newest answers stay in view.
    let skip = lines.len().saturating_sub(inner.height as usize);
    let head: Vec<Line> = lines.drain(..2.min(lines.len())).collect();
    let tail: Vec<Line> = lines.into_iter().skip(skip).collect();
    frame.render_widget(
        Paragraph::new(head.into_iter().chain(tail).collect::<Vec<_>>()).style(base),
        inner,
    );
}

/// C: the weak keys in a card over the code, once the session is over.
pub fn draw_card(frame: &mut Frame, d: &Drill, theme: &Theme, area: Rect, base: Style) {
    let mut lines = weak_lines(d, theme, base);
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(" `q` quits.", base)));
    let w = 52.min(area.width);
    let [area] = Layout::horizontal([Constraint::Length(w)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Length(lines.len() as u16 + 2)])
        .flex(Flex::Center)
        .areas(area);
    let title = format!(" Drill \u{2713} {} tasks in {} ", d.asked, elapsed(d));
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(title).style(base)),
        area,
    );
}

fn weak_lines<'a>(d: &Drill, theme: &Theme, base: Style) -> Vec<Line<'a>> {
    let rows = weak(d);
    if rows.is_empty() {
        return vec![Line::from(Span::styled(" no weak keys", base))];
    }
    rows.into_iter()
        .map(|(name, verdict, title)| {
            Line::from(vec![
                Span::styled(format!(" {name:<16}"), base.add_modifier(Modifier::BOLD)),
                Span::styled(format!("{verdict:<10}"), base.fg(theme.accent)),
                Span::styled(title, base.fg(theme.ghost_fg)),
            ])
        })
        .collect()
}

fn bar<'a>(title: String, width: u16, style: Style) -> Line<'a> {
    let pad = (width as usize).saturating_sub(crate::wrap::width(&title));
    Line::from(Span::styled(format!("{title}{}", " ".repeat(pad)), style))
}

/// Greedy word wrap to `w` columns.
fn textwrap(text: &str, w: usize) -> Vec<String> {
    let mut out = vec![String::new()];
    for word in text.split_whitespace() {
        let last = out.last_mut().unwrap();
        if !last.is_empty() && last.len() + 1 + word.len() > w {
            out.push(word.to_string());
        } else {
            if !last.is_empty() {
                last.push(' ');
            }
            last.push_str(word);
        }
    }
    out
}
