use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use unicode_segmentation::UnicodeSegmentation;

use super::*;

type Pos = (usize, usize);

struct Xorshift64(u64);

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % n as u64).unwrap()
    }
    fn pick<'a, T>(&mut self, of: &'a [T]) -> &'a T {
        &of[self.below(of.len())]
    }
}

#[derive(Clone)]
enum Op {
    Key(KeyCode, KeyModifiers),
    Paste(String),
    PasteClip,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Key(KeyCode::Char(c), m) if m.is_empty() => write!(f, "{c:?}"),
            Self::Key(code, m) => f.write_str(&crate::stats::name(KeyEvent::new(*code, *m))),
            Self::Paste(t) => write!(f, "paste {t:?}"),
            Self::PasteClip => f.write_str("paste clipboard"),
        }
    }
}

impl Op {
    fn edit_only(&self) -> bool {
        match self {
            Self::Key(KeyCode::Char(c), m) => m.is_empty() && *c != 'v',
            Self::Key(KeyCode::Tab | KeyCode::Backspace | KeyCode::Delete, _) => true,
            _ => false,
        }
    }
}

const WIDER_THAN_ANY_LINE: usize = 10_000;

const NONE: KeyModifiers = KeyModifiers::NONE;
const SHIFT: KeyModifiers = KeyModifiers::SHIFT;
const ALT: KeyModifiers = KeyModifiers::ALT;
const CTRL: KeyModifiers = KeyModifiers::CONTROL;

const CHARS: &[char] = &[
    'a',
    'b',
    'x',
    '_',
    '1',
    ' ',
    ' ',
    '.',
    '(',
    ')',
    'д',
    'я',
    '中',
    '文',
    '😀',
    '⚠',
    '\u{fe0f}',
    '\u{200d}',
    '👨',
    '💻',
    '\u{1f3fd}',
    '\u{301}',
];
const PASTES: &[&str] = &[
    "foo",
    "a b\nc",
    "x\r\ny\r\n",
    "\n",
    "⚠\u{fe0f}",
    "👨\u{200d}💻",
    "中文\n    мир",
    "  indented\n",
    "",
];
const LINES: &[&str] = &[
    "",
    "    indented = 1",
    "\tif x:",
    "x_1 = да мир;",
    "中文 字",
    "⚠\u{fe0f} x 👨\u{200d}💻",
    "a",
    "  ",
    "}",
];

fn gen_file(rng: &mut Xorshift64) -> Vec<u8> {
    let lines: Vec<&str> = (0..rng.below(6)).map(|_| *rng.pick(LINES)).collect();
    let nl = if rng.below(4) == 0 { "\r\n" } else { "\n" };
    let mut text = lines.join(nl);
    if !lines.is_empty() && rng.below(3) > 0 {
        text += nl;
    }
    let mut bytes = if rng.below(5) == 0 {
        b"\xEF\xBB\xBF".to_vec()
    } else {
        Vec::new()
    };
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

fn gen_op(rng: &mut Xorshift64) -> Op {
    let key = |code| Op::Key(code, NONE);
    let arrows = [KeyCode::Left, KeyCode::Right, KeyCode::Up, KeyCode::Down];
    match rng.below(100) {
        0..22 => key(KeyCode::Char(*rng.pick(CHARS))),
        22..25 => Op::Paste(rng.pick(PASTES).to_string()),
        25..28 => Op::PasteClip,
        28..34 => key(KeyCode::Backspace),
        34..38 => key(KeyCode::Delete),
        38..41 => Op::Key(KeyCode::Backspace, ALT),
        41..43 => Op::Key(KeyCode::Delete, ALT),
        43..48 => key(KeyCode::Enter),
        48..51 => key(KeyCode::Esc),
        51..55 => key(KeyCode::Tab),
        55..67 => key(*rng.pick(&arrows)),
        67..77 => Op::Key(*rng.pick(&arrows), SHIFT),
        77..79 => Op::Key(*rng.pick(&arrows[..2]), ALT),
        79..81 => Op::Key(
            *rng.pick(&[KeyCode::Home, KeyCode::End]),
            *rng.pick(&[NONE, SHIFT, CTRL, CTRL | SHIFT]),
        ),
        81..84 => key(KeyCode::Char('v')),
        84..87 => Op::Key(KeyCode::Char('c'), CTRL),
        87..90 => Op::Key(KeyCode::Char('x'), CTRL),
        90..95 => Op::Key(KeyCode::Char('z'), CTRL),
        95..98 => Op::Key(KeyCode::Char('y'), CTRL),
        98 => Op::Key(KeyCode::Char('s'), CTRL),
        _ => Op::Key(KeyCode::Char('r'), CTRL),
    }
}

fn gen_session(seed: u64) -> (Vec<u8>, Vec<Op>) {
    let mut rng = Xorshift64::new(seed);
    let file = gen_file(&mut rng);
    let n = 40 + rng.below(120);
    let into_edit_mode = Op::Key(KeyCode::Enter, NONE);
    let ops = std::iter::once(into_edit_mode)
        .chain((0..n).map(|_| gen_op(&mut rng)))
        .collect();
    (file, ops)
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent fact about the file's bytes"
)]
struct Format {
    bom: bool,
    crlf: bool,
    newline_at_end: bool,
    tabs: bool,
}

fn parse(bytes: &[u8]) -> (Vec<String>, Format) {
    let bom = bytes.starts_with(b"\xEF\xBB\xBF");
    let text = std::str::from_utf8(&bytes[if bom { 3 } else { 0 }..]).unwrap();
    let crlf = text.contains("\r\n");
    let body = text.strip_suffix('\n').unwrap_or(text);
    let lines: Vec<String> = body
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l).to_owned())
        .collect();
    let format = Format {
        bom,
        crlf,
        newline_at_end: text.ends_with('\n'),
        tabs: lines.iter().any(|l| l.starts_with('\t')),
    };
    (lines, format)
}

fn write(lines: &[String], f: Format) -> Vec<u8> {
    let nl = if f.crlf { "\r\n" } else { "\n" };
    let mut out = if f.bom {
        b"\xEF\xBB\xBF".to_vec()
    } else {
        Vec::new()
    };
    out.extend_from_slice(lines.join(nl).as_bytes());
    if f.newline_at_end {
        out.extend_from_slice(nl.as_bytes());
    }
    out
}

struct Step {
    old: Vec<String>,
    new: Vec<String>,
    before: Pos,
    after: Pos,
    format: Option<(Format, Format)>,
}

struct Model {
    lines: Vec<String>,
    cur: Pos,
    want_x: usize,
    anchor: Option<Pos>,
    editing: bool,
    clipboard: Option<String>,
    undo: Vec<Step>,
    redo: Vec<Step>,
    next_edit_starts_a_step: bool,
    last: &'static str,
    format: Format,
    disk: Vec<u8>,
    dirty: bool,
}

fn next(s: &str, i: usize) -> usize {
    s.get(i..)
        .unwrap()
        .graphemes(true)
        .next()
        .map_or(i, |g| i + g.len())
}
fn prev(s: &str, i: usize) -> usize {
    s.get(..i)
        .unwrap()
        .graphemes(true)
        .next_back()
        .map_or(0, |g| i - g.len())
}
fn wordy(s: &str, i: usize) -> bool {
    s.get(i..).unwrap().chars().next().is_some_and(is_word)
}
fn width(s: &str) -> usize {
    s.graphemes(true).map(wrap::cluster_width).sum()
}

impl Model {
    fn new(bytes: &[u8]) -> Self {
        let (lines, format) = parse(bytes);
        Self {
            lines,
            cur: (0, 0),
            want_x: 0,
            anchor: None,
            editing: false,
            clipboard: None,
            undo: Vec::new(),
            redo: Vec::new(),
            next_edit_starts_a_step: false,
            last: "other",
            format,
            disk: bytes.to_vec(),
            dirty: false,
        }
    }

    fn line(&self) -> &str {
        &self.lines[self.cur.0]
    }

    fn clamp(&self, (l, c): Pos) -> Pos {
        let l = l.min(self.lines.len() - 1);
        let s = &self.lines[l];
        let mut c = c.min(s.len());
        while !s.is_char_boundary(c) {
            c -= 1;
        }
        (l, c)
    }

    fn selection(&self) -> Option<(Pos, Pos)> {
        let a = self.clamp(self.anchor?);
        (a != self.cur).then(|| (a.min(self.cur), a.max(self.cur)))
    }

    fn text(&self, (from, to): (Pos, Pos)) -> String {
        if from.0 == to.0 {
            return self.lines[from.0].get(from.1..to.1).unwrap().to_owned();
        }
        let mut parts = vec![self.lines[from.0].get(from.1..).unwrap()];
        parts.extend(self.lines[from.0 + 1..to.0].iter().map(String::as_str));
        parts.push(self.lines[to.0].get(..to.1).unwrap());
        parts.join("\n")
    }

    fn sync_x(&mut self) {
        self.want_x = width(self.line().get(..self.cur.1).unwrap());
    }

    fn aim(&mut self, l: usize) {
        let s = &self.lines[l];
        let mut x = 0;
        let mut col = s.len();
        for (i, g) in s.grapheme_indices(true) {
            if x >= self.want_x {
                col = i;
                break;
            }
            x += wrap::cluster_width(g);
        }
        self.cur = (l, col);
    }

    fn left(&mut self) {
        let (l, c) = self.cur;
        if c > 0 {
            self.cur.1 = prev(self.line(), c);
        } else if l > 0 {
            self.cur = (l - 1, self.lines[l - 1].len());
        }
        self.sync_x();
    }

    fn right(&mut self) {
        let (l, c) = self.cur;
        if c < self.line().len() {
            self.cur.1 = next(self.line(), c);
        } else if l + 1 < self.lines.len() {
            self.cur = (l + 1, 0);
        }
        self.sync_x();
    }

    fn word_left_of(&self, (l, c): Pos) -> Pos {
        if c == 0 {
            return if l > 0 {
                (l - 1, self.lines[l - 1].len())
            } else {
                (l, c)
            };
        }
        let s = &self.lines[l];
        let mut i = prev(s, c);
        while i > 0 && !wordy(s, i) {
            i = prev(s, i);
        }
        while i > 0 && wordy(s, prev(s, i)) {
            i = prev(s, i);
        }
        (l, i)
    }

    fn word_right_of(&self, (l, c): Pos) -> Pos {
        let s = &self.lines[l];
        if c >= s.len() {
            return if l + 1 < self.lines.len() {
                (l + 1, 0)
            } else {
                (l, c)
            };
        }
        let mut i = c;
        while i < s.len() && !wordy(s, i) {
            i = next(s, i);
        }
        while i < s.len() && wordy(s, i) {
            i = next(s, i);
        }
        (l, i)
    }

    fn vertical(&mut self, down: bool) {
        let l = self.cur.0;
        if down {
            if l + 1 < self.lines.len() {
                self.aim(l + 1);
            }
        } else {
            self.aim(l.saturating_sub(1));
        }
    }

    fn arrow(&mut self, code: KeyCode, alt: bool) {
        match code {
            KeyCode::Left if alt => {
                self.cur = self.word_left_of(self.cur);
                self.sync_x();
            }
            KeyCode::Right if alt => {
                self.cur = self.word_right_of(self.cur);
                self.sync_x();
            }
            KeyCode::Left => self.left(),
            KeyCode::Right => self.right(),
            KeyCode::Up => self.vertical(false),
            _ => self.vertical(true),
        }
    }

    fn replace(&mut self, from: Pos, to: Pos, text: &str, kind: &'static str) -> bool {
        self.next_edit_starts_a_step |= kind == "other";
        if from == to && text.is_empty() {
            return false;
        }
        let before = self.cur;
        let old = self.lines.clone();
        let joined = format!(
            "{}{text}{}",
            self.lines[from.0].get(..from.1).unwrap(),
            self.lines[to.0].get(to.1..).unwrap()
        );
        let new: Vec<String> = joined.split('\n').map(String::from).collect();
        let n = new.len();
        let tail = self.lines[to.0].len() - to.1;
        self.lines.splice(from.0..=to.0, new);
        let last = from.0 + n - 1;
        self.cur = (last, self.lines[last].len() - tail);
        let kind = match (kind, text) {
            ("type", " ") if self.last.starts_with("space") => "spaces",
            ("type", " ") => "space",
            _ => kind,
        };
        let stops = match kind {
            "other" => true,
            "left" | "right" => self.last != kind || from.0 != to.0,
            _ => text.contains('\n') || self.last != "space" && self.last != kind,
        };
        match self.undo.last_mut() {
            Some(s) if !self.next_edit_starts_a_step && !stops && s.after == before => {
                s.new = self.lines.clone();
                s.after = self.cur;
            }
            _ => self.undo.push(Step {
                old,
                new: self.lines.clone(),
                before,
                after: self.cur,
                format: None,
            }),
        }
        self.next_edit_starts_a_step = kind == "other";
        self.last = kind;
        self.redo.clear();
        self.dirty = true;
        self.anchor = None;
        self.sync_x();
        true
    }

    fn insert(&mut self, text: &str, kind: &'static str) {
        let (from, to) = self.selection().unwrap_or((self.cur, self.cur));
        self.replace(from, to, text, kind);
    }

    fn copy_and_cut_range(&mut self) -> (Pos, Pos) {
        let (l, n) = (self.cur.0, self.lines.len());
        let (range, text) = match self.selection() {
            Some(r) => (r, self.text(r)),
            None if l + 1 < n => (((l, 0), (l + 1, 0)), format!("{}\n", self.line())),
            None => {
                let from = if l == 0 {
                    (0, 0)
                } else {
                    (l - 1, self.lines[l - 1].len())
                };
                ((from, (l, self.line().len())), format!("{}\n", self.line()))
            }
        };
        self.clipboard = Some(text);
        range
    }

    fn indent(&mut self, from: Pos, to: Pos, indent: &str) {
        let last = if to.1 == 0 { to.0 - 1 } else { to.0 };
        let text = self.lines[from.0..=last]
            .iter()
            .map(|l| format!("{indent}{l}"))
            .collect::<Vec<_>>()
            .join("\n");
        let (anchor, cur) = (self.anchor, self.cur);
        self.replace((from.0, 0), (last, self.lines[last].len()), &text, "other");
        let moved = |(l, c): Pos| {
            if (from.0..=last).contains(&l) && c > 0 {
                (l, c + indent.len())
            } else {
                (l, c)
            }
        };
        self.anchor = anchor.map(moved);
        self.cur = moved(cur);
        self.sync_x();
        self.undo.last_mut().unwrap().after = self.cur;
    }

    fn undo(&mut self, back: bool) {
        let Some(step) = (if back { &mut self.undo } else { &mut self.redo }).pop() else {
            return;
        };
        if back {
            self.lines = step.old.clone();
            self.cur = step.before;
        } else {
            self.lines = step.new.clone();
            self.cur = step.after;
        }
        if let Some((was, is)) = step.format {
            self.format = if back { was } else { is };
        }
        self.dirty = true;
        self.anchor = None;
        self.next_edit_starts_a_step = true;
        self.last = "other";
        self.sync_x();
        (if back { &mut self.redo } else { &mut self.undo }).push(step);
    }

    fn save(&mut self) {
        self.disk = write(&self.lines, self.format);
        self.dirty = false;
    }

    fn reload(&mut self) {
        let (lines, format) = parse(&self.disk);
        if lines != self.lines || format != self.format {
            let head = self.lines.iter().zip(&lines).take_while(|(a, b)| a == b);
            let head = head.count();
            self.undo.push(Step {
                old: self.lines.clone(),
                new: lines.clone(),
                before: (head.min(self.lines.len() - 1), 0),
                after: (head.min(lines.len() - 1), 0),
                format: Some((self.format, format)),
            });
        }
        (self.lines, self.format) = (lines, format);
        self.dirty = false;
        self.redo.clear();
        self.next_edit_starts_a_step = true;
        self.last = "other";
        self.cur = self.clamp(self.cur);
        self.sync_x();
    }

    fn grow(&mut self) {
        let blank = |l: usize| self.lines[l].trim().is_empty();
        let l = self.cur.0;
        if blank(l) {
            return;
        }
        let (mut top, mut bottom) = (l, l);
        while top > 0 && !blank(top - 1) {
            top -= 1;
        }
        while bottom + 1 < self.lines.len() && !blank(bottom + 1) {
            bottom += 1;
        }
        let s = self.line();
        let (mut from, mut to) = (self.cur.1, self.cur.1);
        while from > 0 && wordy(s, prev(s, from)) {
            from = prev(s, from);
        }
        while to < s.len() && wordy(s, to) {
            to = next(s, to);
        }
        let now = self.selection().unwrap_or((self.cur, self.cur));
        let steps = [
            (from < to).then_some(((l, from), (l, to))),
            Some(((l, 0), (l, s.len()))),
            Some(((top, 0), (bottom, self.lines[bottom].len()))),
            Some((
                (0, 0),
                (self.lines.len() - 1, self.lines[self.lines.len() - 1].len()),
            )),
        ];
        let wider = |&(a, b): &(Pos, Pos)| a <= now.0 && now.1 <= b && (a, b) != now;
        if let Some((a, b)) = steps.into_iter().flatten().find(wider) {
            self.anchor = Some(a);
            self.cur = b;
            self.sync_x();
        }
    }

    fn paste(&mut self, text: &str) {
        if self.editing {
            self.insert(&text.replace("\r\n", "\n").replace('\r', "\n"), "other");
        }
    }

    fn key(&mut self, code: KeyCode, m: KeyModifiers) {
        let (ctrl, shift, alt) = (m.contains(CTRL), m.contains(SHIFT), m.contains(ALT));
        if ctrl && code == KeyCode::Char('c') {
            self.copy_and_cut_range();
            return;
        }
        if self.editing && self.edit_key(code, ctrl, alt) {
            return;
        }
        let before = self.cur;
        let extending = code == KeyCode::Char('v') || shift;
        match code {
            KeyCode::Esc => self.anchor = None,
            KeyCode::Enter => {
                self.editing = true;
                self.next_edit_starts_a_step = true;
            }
            KeyCode::Char('v') => self.grow(),
            KeyCode::Char('s') if ctrl => self.save(),
            KeyCode::Char('r') if ctrl => self.reload(),
            KeyCode::Char('z') if ctrl => self.undo(true),
            KeyCode::Char('y') if ctrl => self.undo(false),
            KeyCode::Home | KeyCode::End => {
                if shift {
                    self.anchor.get_or_insert(self.cur);
                }
                let last = self.lines.len() - 1;
                self.cur = match (code, ctrl) {
                    (KeyCode::Home, true) => (0, 0),
                    (KeyCode::Home, false) => (self.cur.0, 0),
                    (_, true) => (last, self.lines[last].len()),
                    _ => (self.cur.0, self.line().len()),
                };
                self.sync_x();
            }
            _ if shift => {
                self.anchor.get_or_insert(self.cur);
                self.arrow(code, alt);
            }
            KeyCode::Left | KeyCode::Right if !alt && self.selection().is_some() => {
                let (a, b) = self.selection().unwrap();
                self.cur = if code == KeyCode::Left { a } else { b };
                self.sync_x();
                self.anchor = None;
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => self.arrow(code, alt),
            _ => {}
        }
        if !extending && self.cur != before {
            self.anchor = None;
        }
        let moves = matches!(
            code,
            KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Home
                | KeyCode::End
        );
        self.next_edit_starts_a_step |= moves || self.cur != before;
    }

    fn edit_key(&mut self, code: KeyCode, ctrl: bool, alt: bool) -> bool {
        let (l, c) = self.cur;
        match code {
            KeyCode::Esc => {
                self.editing = false;
                if self.dirty {
                    self.save();
                }
            }
            KeyCode::Char('x') if ctrl => {
                let cuts_last_line = self.selection().is_none() && l + 1 == self.lines.len();
                let (from, to) = self.copy_and_cut_range();
                let want = self.want_x;
                if self.replace(from, to, "", "other") && cuts_last_line {
                    self.want_x = want;
                    self.aim(self.cur.0);
                    self.undo.last_mut().unwrap().after = self.cur;
                }
            }
            KeyCode::Char(ch) if !ctrl => self.insert(&ch.to_string(), "type"),
            KeyCode::Enter => {
                let head = self.line().get(..c).unwrap();
                let indent = head.strip_suffix(head.trim_start()).unwrap();
                self.insert(&format!("\n{indent}"), "type");
            }
            KeyCode::Tab => {
                let indent = if self.format.tabs { "\t" } else { "    " };
                match self.selection() {
                    Some((from, to)) if to.0 > from.0 => self.indent(from, to, indent),
                    _ => self.insert(indent, "other"),
                }
            }
            KeyCode::Backspace if self.selection().is_some() => self.insert("", "left"),
            KeyCode::Delete if self.selection().is_some() => self.insert("", "right"),
            KeyCode::Backspace | KeyCode::Delete if alt => {
                let to = if code == KeyCode::Backspace {
                    self.word_left_of(self.cur)
                } else {
                    self.word_right_of(self.cur)
                };
                self.replace(self.cur.min(to), self.cur.max(to), "", "other");
            }
            KeyCode::Backspace if c > 0 => {
                self.replace((l, prev(self.line(), c)), (l, c), "", "left");
            }
            KeyCode::Backspace if l > 0 => {
                self.replace((l - 1, self.lines[l - 1].len()), (l, c), "", "left");
            }
            KeyCode::Delete if c < self.line().len() => {
                self.replace((l, c), (l, next(self.line(), c)), "", "right");
            }
            KeyCode::Delete if l + 1 < self.lines.len() => {
                self.replace((l, c), (l + 1, 0), "", "right");
            }
            KeyCode::Backspace | KeyCode::Delete => {}
            _ => return false,
        }
        true
    }
}

struct Failure {
    step: usize,
    what: &'static str,
    detail: String,
}

fn play(path: &Path, file: &[u8], ops: &[Op]) -> Result<(), Failure> {
    std::fs::write(path, file).unwrap();
    let mut a = App::new(
        path.parent().unwrap().to_path_buf(),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(path.to_path_buf(), file),
        None,
    );
    a.view_w = WIDER_THAN_ANY_LINE;
    a.view_h = 40;
    let mut m = Model::new(file);
    let mut clipboard = None;
    for (step, op) in ops.iter().enumerate() {
        if op.edit_only() && !m.editing {
            continue;
        }
        let fail = |what, detail| Err(Failure { step, what, detail });
        let pressed = catch_unwind(AssertUnwindSafe(|| match op {
            Op::Key(code, mods) => _ = a.key(KeyEvent::new(*code, *mods)),
            Op::Paste(t) => a.paste(t),
            Op::PasteClip => a.paste(m.clipboard.as_deref().unwrap_or_default()),
        }));
        if let Err(e) = pressed {
            let msg = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(ToString::to_string));
            return fail("panic", msg.unwrap_or_default());
        }
        match op {
            Op::Key(code, mods) => m.key(*code, *mods),
            Op::Paste(t) => m.paste(t),
            Op::PasteClip => m.paste(&m.clipboard.clone().unwrap_or_default()),
        }
        if let Some(t) = a.clipboard.take() {
            clipboard = Some(t);
        }
        let checks: [(&str, String, String); 6] = [
            (
                "text",
                format!("{:?}", a.buf.lines),
                format!("{:?}", m.lines),
            ),
            (
                "cursor",
                format!("{:?}", (a.line, a.col)),
                format!("{:?}", m.cur),
            ),
            (
                "selection",
                format!("{:?}", a.file_selection()),
                format!("{:?}", m.selection()),
            ),
            (
                "mode",
                format!("{:?}", a.mode == Mode::Edit),
                format!("{:?}", m.editing),
            ),
            (
                "clipboard",
                format!("{clipboard:?}"),
                format!("{:?}", m.clipboard),
            ),
            (
                "disk",
                format!(
                    "{:?}",
                    String::from_utf8_lossy(&std::fs::read(path).unwrap())
                ),
                format!("{:?}", String::from_utf8_lossy(&m.disk)),
            ),
        ];
        for (what, merl, model) in checks {
            if merl != model {
                return fail(what, format!("merl {merl}\n  model {model}"));
            }
        }
    }
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut narrow = Terminal::new(TestBackend::new(24, 8)).unwrap();
    let drawn = catch_unwind(AssertUnwindSafe(|| {
        narrow
            .draw(|f| crate::ui::draw(f, &mut a, &theme))
            .map(|_| ())
    }));
    if !matches!(drawn, Ok(Ok(()))) {
        let step = ops.len().saturating_sub(1);
        return Err(Failure {
            step,
            what: "draw",
            detail: "the last screen did not draw in a pane where its lines wrap".into(),
        });
    }
    Ok(())
}

fn shrink(path: &Path, file: &[u8], ops: &[Op], what: &str) -> Vec<Op> {
    let mut ops = ops.to_vec();
    let mut chunk = ops.len() / 2;
    while chunk > 0 {
        let mut i = 0;
        while i < ops.len() {
            let mut shorter = ops.clone();
            shorter.drain(i..(i + chunk).min(ops.len()));
            match play(path, file, &shorter) {
                Err(f) if f.what == what => ops = shorter,
                _ => i += chunk,
            }
        }
        chunk /= 2;
    }
    ops
}

#[test]
fn random_edit_sessions_agree_with_the_model() {
    let env = |name| std::env::var(name).ok().and_then(|v| v.parse::<u64>().ok());
    let seeds = match env("MERL_FUZZ_SEED") {
        Some(s) => s..=s,
        None => 1..=env("MERL_FUZZ_SESSIONS").unwrap_or(500),
    };
    let dir = std::env::temp_dir().join(format!("merl-edit-fuzz-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("f.txt");
    let mut failures = Vec::new();
    for seed in seeds {
        let (file, ops) = gen_session(seed);
        let Err(f) = play(&path, &file, &ops) else {
            continue;
        };
        let short = shrink(&path, &file, &ops[..=f.step], f.what);
        let last = play(&path, &file, &short).err().unwrap_or(f);
        let keys: Vec<String> = short.iter().map(Op::to_string).collect();
        failures.push(format!(
            "seed {seed}: {} after key {} of [{}]\n  file {:?}\n  {}",
            last.what,
            last.step + 1,
            keys.join(", "),
            String::from_utf8_lossy(&file),
            last.detail,
        ));
        if std::env::var_os("MERL_FUZZ_ALL").is_none() {
            break;
        }
    }
    drop(std::fs::remove_dir_all(&dir));
    assert!(
        failures.is_empty(),
        "\n{}\n\nMERL_FUZZ_SEED=S replays seed S; MERL_FUZZ_ALL=1 lists every failing seed",
        failures.join("\n\n")
    );
}

const PYTHON: &[&str] = &[
    "\"",
    "'''",
    "\"\"\"",
    "#",
    "def f(",
    "):\n    ",
    "class A:\n    x = 1\n",
    "self.",
    "import os\n",
    "f\"{x}\"",
    "@property\n",
    "[",
    "]",
    "match x:\n    case _:\n        pass\n",
    "MAX = 1\n",
    "lambda x: x",
    "re.compile(r\"[a-z]+\\d\")",
    "\"%s %d\" % (a, b)",
    "T = \\\n    TypeVar(\"T\")\n",
    "TypeVar",
    "r",
];

const MARKDOWN: &[&str] = &[
    "```python\n",
    "```\n",
    "`",
    "*",
    "**",
    "# ",
    "- [ ] ",
    "> ",
    "[link](http://x)",
    "<b>",
    "</b>",
    "---\n",
    "| a | b |\n| - | - |\n",
    "re.compile(\"x\")",
    "\n\n",
    "    ",
];

const RUST: &[&str] = &[
    "println!(\"{}\", f(x));",
    "vec![a.b(), 2]",
    "assert_eq!(x, y);",
    "macro_rules! m {\n    ($x:expr) => { $x.len() };\n}\n",
    "Regex::new(r\"^a+$\")",
    "(",
    ")",
    "{",
    "}",
    "!",
    "\"",
    "\n\n",
    "    ",
];

const SHELL: &[&str] = &[
    "$(",
    ")",
    "${x:-y}",
    "\"",
    "'",
    "<<EOF\nbody\nEOF\n",
    "if [[ -n $a ]]; then\n",
    "fi\n",
    "case $a in\n",
    "esac\n",
    "printf '%s' x",
    "# ",
];

const DATA: &[&str] = &[
    "{",
    "}",
    "[",
    "]",
    "\"",
    "'",
    ": ",
    " = ",
    "- ",
    "run: |\n  echo $HOME\n",
    "[[bin]]\n",
    "\"\"\"",
    "# ",
    "    ",
];

const C: &[&str] = &[
    "printf(\"%d\\n\", x);",
    "#define M(x) ((x) + 1)\n",
    "#if 0\n",
    "#endif\n",
    "struct s { int a : 3; };",
    "/* ",
    " */",
    "// ",
    "(",
    ")",
    "{",
    "}",
    "\"",
    "'",
    "\n\n",
];

const DIFF: &[&str] = &[
    "\n+",
    "\n-",
    "\n ",
    "@@ -1,2 +1,3 @@",
    "diff --git a/x b/x\n",
    "--- a/x\n+++ b/x\n",
    "\\ No newline at end of file\n",
    "index 1a2b3c4..5d6e7f8 100644\n",
    "\n\n",
];

const QUERY: &[&str] = &[
    "(identifier) @variable",
    "((a) @b (#eq? @b \"c\"))",
    "[\"if\" \"else\"] @keyword",
    "; ",
    "\"",
    "(",
    ")",
    "[",
    "]",
    "@",
    "#",
    "\n\n",
];

const NIX: &[&str] = &[
    "\"${x}\"", "''\n", "''${", "/* c */", "# c\n", "let\n", "in\n", "a: ", "[", "]", "{", "}",
    "\"", "\n\n",
];

const CPP: &[&str] = &[
    "template <typename T>\n",
    "R\"x(",
    ")x\"",
    "std::printf(\"%s\", s);",
    "[](auto& a) { return a; }",
    "class A : public B {\npublic:\n",
    "::",
    "<",
    ">",
    "{",
    "}",
    "\"",
    "/* ",
    "\n\n",
];

const JAVA: &[&str] = &[
    "String.format(\"%d\", x);",
    "\"\"\"\n",
    "/** ",
    " */",
    "@Override\n",
    "case 1 -> {\n",
    "<T extends A>",
    "(",
    ")",
    "{",
    "}",
    "\"",
    "\n\n",
];

const KOTLIN: &[&str] = &[
    "\"${x}\"",
    "\"\"\"\n",
    "/** ",
    " */",
    "@JvmStatic\n",
    "when (x) {\n",
    "<T : A>",
    "(",
    ")",
    "{",
    "}",
    "\"",
    "\n\n",
];

const SWIFT: &[&str] = &[
    "\"\\(x)\"",
    "\"\"\"\n",
    "/// ",
    "/[a-z]+/",
    "@MainActor\n",
    "switch x {\n",
    "<T: A>",
    "(",
    ")",
    "{",
    "}",
    "\"",
    "\n\n",
];

const SCALA: &[&str] = &[
    "s\"${x}\"",
    "\"\"\"\n",
    "/** ",
    " */",
    "@main ",
    "x match {\n",
    "case _ => ",
    "[A]",
    "(",
    ")",
    "{",
    "}",
    "\"",
    "\n\n",
];

const ZIG: &[&str] = &[
    "\\\\ text\n",
    "\"{s}\\n\"",
    "/// ",
    "//! ",
    "label: {\n",
    "|x| ",
    "switch (x) {\n",
    "(",
    ")",
    "{",
    "}",
    "\"",
    "\n\n",
];

const SCHEME: &[&str] = &[
    "#| c |#",
    "#;",
    "; c\n",
    "'(a b)",
    "`(a ,b)",
    "#\\x",
    "(define (f x)\n",
    "(",
    ")",
    "[",
    "]",
    "\"",
    "\n\n",
];

const R: &[&str] = &[
    "r\"(x)\"",
    "\\(x) ",
    "function(x) {\n",
    "# c\n",
    "|> ",
    "%>% ",
    "x$y",
    "(",
    ")",
    "{",
    "}",
    "\"",
    "\n\n",
];

const ERLANG: &[&str] = &[
    "%% c\n",
    "-define(X, 1).\n",
    "?X",
    "#{a => 1}",
    "<<\"b\">>",
    "fun(X) -> X end",
    "case X of\n",
    "end.\n",
    "(",
    ")",
    "{",
    "}",
    "\"",
    "\n\n",
];

const RACKET: &[&str] = &[
    "#| c |#",
    "#;",
    "; c\n",
    "'(a b)",
    "#rx\"a\"",
    "#:key ",
    "(define (f x)\n",
    "(",
    ")",
    "[",
    "]",
    "\"",
    "\n\n",
];

const COMMON_LISP: &[&str] = &[
    "#| c |#",
    "#+sbcl ",
    "; c\n",
    "'(a b)",
    "`(a ,b)",
    "#'f",
    "(defun f (x)\n",
    "(loop for x in y collect x)",
    "(",
    ")",
    "\"",
    "\n\n",
];

const RUBY: &[&str] = &[
    "\"#{x}\"", "<<~SQL\n", "SQL\n", "=begin\n", "=end\n", "/[a-z]+/", "do |x|\n", "end\n", "(",
    ")", "{", "}", "\"", "\n\n",
];

const PROTO: &[&str] = &[
    "message A {\n",
    "}\n",
    "string a = 1;\n",
    "[default = \"x\"]",
    "map<string, int32>",
    "rpc A(B) returns (stream C);\n",
    "/*",
    "*/",
    "// ",
    "\"",
    "{",
    "}",
    "\n\n",
];

const HCL: &[&str] = &[
    "resource \"a\" \"b\" {\n",
    "}\n",
    "a = var.b[0]\n",
    "\"${x}\"",
    "%{ if x }",
    "<<EOT\n",
    "EOT\n",
    "[for s in x : s]",
    "/*",
    "*/",
    "# ",
    "\"",
    "{",
    "}",
];

const INI: &[&str] = &["[a]\n", "b = c\n", "; ", "# ", "[", "]", "=", ":", "\n"];

const SQL: &[&str] = &[
    "SELECT ",
    " FROM t",
    " WHERE x = 1",
    "'a''b'",
    "$$",
    "/* ",
    " */",
    "-- ",
    "CASE WHEN x THEN 1 END",
    "(",
    ")",
    "'",
    ";",
    "\n\n",
];

const NGINX: &[&str] = &[
    "server {\n",
    "}\n",
    "location ~* \\.(js)$ {\n",
    "proxy_pass http://a;\n",
    "if ($a = b) {\n",
    "\"",
    "'",
    "# ",
    "$host",
    ";",
    "{",
];

const GDSCRIPT: &[&str] = &[
    "func a(b: int) -> void:\n",
    "\tif b:\n",
    "\t\tpass\n",
    "@export var c := 1\n",
    "\"\"\"",
    "$Node",
    "\t",
    "# ",
    "\"",
    ":",
    "\n",
];

const STARLARK: &[&str] = &[
    "def a(b, c = 1):\n",
    "    return b\n",
    "load(\":d.bzl\", \"e\")\n",
    "x = [f for f in g if f]\n",
    "\"%s\" % h",
    "\"\"\"",
    "    ",
    "# ",
    "\"",
    "(",
    ")",
];

const SOLIDITY: &[&str] = &[
    "contract A is B {\n",
    "}\n",
    "function f(uint256 a) public view returns (bool) {\n",
    "emit E(msg.sender);\n",
    "/// @notice ",
    "/**",
    "*/",
    "// ",
    "\"",
    "{",
    ";",
];

const GRAPHQL: &[&str] = &[
    "type A {\n",
    "}\n",
    "a(b: Int = 1): [B!]!\n",
    "\"\"\"",
    "query Q($a: ID) {\n",
    "...F\n",
    "@include(if: $a)",
    "# ",
    "\"",
    "{",
    "}",
];

const CMAKE: &[&str] = &[
    "set(",
    "${X}",
    "$ENV{X}",
    "#[[",
    "]]",
    "[=[",
    "]=]",
    "if(NOT X)\n",
    "endif()\n",
    "(",
    ")",
    "\"",
    "# ",
    "\n\n",
];

const ELIXIR: &[&str] = &[
    "\"#{x}\"",
    "\"\"\"\n",
    "@doc \"\"\"\n",
    "~r/[a-z]+/",
    "do\n",
    "end\n",
    "fn x -> ",
    "|> ",
    "(",
    ")",
    "%{",
    "}",
    "\"",
    "\n\n",
];

const CSHARP: &[&str] = &[
    "$\"{x,-10}\"",
    "@\"",
    "\"\"\"\n",
    "#region A\n",
    "#endregion\n",
    "/// <summary>",
    "[Obsolete]\n",
    "x switch { _ => 1 };",
    "(",
    ")",
    "{",
    "}",
    "\"",
    "\n\n",
];

const MARKUP: &[&str] = &[
    "<style>a { color: red; }</style>",
    "<script>let x = 1;</script>",
    "<p style=\"margin: 0\">",
    "</p>",
    "<!-- ",
    " -->",
    "<",
    ">",
    "{",
    "}",
    "\"",
    "&amp;",
    "\n\n",
];

const COMPONENT: &[&str] = &[
    "<script lang=\"ts\">let x: number = 1;</script>",
    "<style>a { color: red; }</style>",
    "<p :class=\"{ a: b }\" @click=\"go(1)\">",
    "</p>",
    "{{ a + b }}",
    "{#if a}",
    "{:else}",
    "{/if}",
    "{a.b}",
    "<!-- ",
    " -->",
    "<",
    ">",
    "{",
    "}",
    "\"",
    "\n\n",
];

const LUA: &[&str] = &[
    "---@param x number\n",
    "string.format(\"%d\", x)",
    "local t = { a = 1 }",
    "function f(...) end",
    "--[[",
    "]]",
    "[[",
    "--",
    "(",
    ")",
    "{",
    "}",
    "\"",
    "\n\n",
    "  ",
];

const PHP: &[&str] = &[
    "\"{$x}\"",
    "<<<TEXT\n",
    "TEXT;\n",
    "/** ",
    " */",
    "?>\n<p>",
    "<?php ",
    "#[Attr]\n",
    "(",
    ")",
    "{",
    "}",
    "\"",
    "\n\n",
];

const DOCKERFILE: &[&str] = &[
    "RUN ", " \\\n", "<<EOT\n", "EOT\n", "${X}", "$X", " && ", "[\"", "\"]", "\"", "# ", "\n\n",
];

const SESSIONS: &[(&str, &[u8], &[&str])] = &[
    (
        "f.php",
        include_bytes!("../../../tests/colors/php/Ledger.php").as_slice(),
        PHP,
    ),
    (
        "Dockerfile",
        include_bytes!("../../../tests/colors/dockerfile/Dockerfile").as_slice(),
        DOCKERFILE,
    ),
    (
        "f.py",
        include_bytes!("../../../tests/colors/python/classes.py").as_slice(),
        PYTHON,
    ),
    (
        "f.md",
        include_bytes!("../../../tests/colors/markdown/code.md").as_slice(),
        MARKDOWN,
    ),
    (
        "f.rs",
        include_bytes!("../../../tests/colors/rust/exprs.rs").as_slice(),
        RUST,
    ),
    (
        "f.sh",
        include_bytes!("../../../tests/colors/bash/script.sh").as_slice(),
        SHELL,
    ),
    (
        "f.yml",
        include_bytes!("../../../tests/colors/yaml/workflow.yml").as_slice(),
        DATA,
    ),
    (
        "f.json",
        include_bytes!("../../../tests/colors/json/package.json").as_slice(),
        DATA,
    ),
    (
        "f.toml",
        include_bytes!("../../../tests/colors/toml/Cargo.toml").as_slice(),
        DATA,
    ),
    (
        "f.lua",
        include_bytes!("../../../tests/colors/lua/decls.lua").as_slice(),
        LUA,
    ),
    (
        "f.c",
        include_bytes!("../../../tests/colors/c/ledger.c").as_slice(),
        C,
    ),
    (
        "f.cpp",
        include_bytes!("../../../tests/colors/cpp/report.cpp").as_slice(),
        CPP,
    ),
    (
        "F.java",
        include_bytes!("../../../tests/colors/java/Ledger.java").as_slice(),
        JAVA,
    ),
    (
        "F.kt",
        include_bytes!("../../../tests/colors/kotlin/Ledger.kt").as_slice(),
        KOTLIN,
    ),
    (
        "F.swift",
        include_bytes!("../../../tests/colors/swift/Ledger.swift").as_slice(),
        SWIFT,
    ),
    (
        "F.scala",
        include_bytes!("../../../tests/colors/scala/Ledger.scala").as_slice(),
        SCALA,
    ),
    (
        "f.zig",
        include_bytes!("../../../tests/colors/zig/ledger.zig").as_slice(),
        ZIG,
    ),
    (
        "f.scm",
        include_bytes!("../../../tests/colors/scheme/ledger.scm").as_slice(),
        SCHEME,
    ),
    (
        "f.R",
        include_bytes!("../../../tests/colors/r/ledger.R").as_slice(),
        R,
    ),
    (
        "f.erl",
        include_bytes!("../../../tests/colors/erlang/ledger.erl").as_slice(),
        ERLANG,
    ),
    (
        "f.rkt",
        include_bytes!("../../../tests/colors/racket/ledger.rkt").as_slice(),
        RACKET,
    ),
    (
        "f.lisp",
        include_bytes!("../../../tests/colors/commonlisp/ledger.lisp").as_slice(),
        COMMON_LISP,
    ),
    (
        "f.rb",
        include_bytes!("../../../tests/colors/ruby/ledger.rb").as_slice(),
        RUBY,
    ),
    (
        "f.ex",
        include_bytes!("../../../tests/colors/elixir/ledger.ex").as_slice(),
        ELIXIR,
    ),
    (
        "F.cs",
        include_bytes!("../../../tests/colors/c_sharp/Ledger.cs").as_slice(),
        CSHARP,
    ),
    (
        "f.html",
        include_bytes!("../../../tests/colors/html/page.html").as_slice(),
        MARKUP,
    ),
    (
        "f.css",
        include_bytes!("../../../tests/colors/css/site.css").as_slice(),
        MARKUP,
    ),
    (
        "f.xml",
        include_bytes!("../../../tests/colors/xml/project.xml").as_slice(),
        MARKUP,
    ),
    (
        "F.vue",
        include_bytes!("../../../tests/colors/vue/Ledger.vue").as_slice(),
        COMPONENT,
    ),
    (
        "F.svelte",
        include_bytes!("../../../tests/colors/svelte/Ledger.svelte").as_slice(),
        COMPONENT,
    ),
    (
        "f.proto",
        include_bytes!("../../../tests/colors/proto/ledger.proto").as_slice(),
        PROTO,
    ),
    (
        "f.hcl",
        include_bytes!("../../../tests/colors/hcl/config.hcl").as_slice(),
        HCL,
    ),
    (
        "f.tfvars",
        include_bytes!("../../../tests/colors/terraform/main.tf").as_slice(),
        HCL,
    ),
    (
        "f.ini",
        include_bytes!("../../../tests/colors/ini/settings.ini").as_slice(),
        INI,
    ),
    (
        "f.sql",
        include_bytes!("../../../tests/colors/sql/ledger.sql").as_slice(),
        SQL,
    ),
    (
        "f.graphql",
        include_bytes!("../../../tests/colors/graphql/schema.graphql").as_slice(),
        GRAPHQL,
    ),
    (
        "nginx.conf",
        include_bytes!("../../../tests/colors/nginx/site.conf").as_slice(),
        NGINX,
    ),
    (
        "f.gd",
        include_bytes!("../../../tests/colors/gdscript/player.gd").as_slice(),
        GDSCRIPT,
    ),
    (
        "defs.bzl",
        include_bytes!("../../../tests/colors/starlark/defs.bzl").as_slice(),
        STARLARK,
    ),
    (
        "f.sol",
        include_bytes!("../../../tests/colors/solidity/Token.sol").as_slice(),
        SOLIDITY,
    ),
    (
        "CMakeLists.txt",
        include_bytes!("../../../tests/colors/cmake/CMakeLists.txt").as_slice(),
        CMAKE,
    ),
    (
        "f.diff",
        include_bytes!("../../../tests/colors/diff/change.diff").as_slice(),
        DIFF,
    ),
    (
        "queries/f.scm",
        include_bytes!("../../../tests/colors/query/queries/highlights.scm").as_slice(),
        QUERY,
    ),
    (
        "f.nix",
        include_bytes!("../../../tests/colors/nix/ledger.nix").as_slice(),
        NIX,
    ),
];

macro_rules! tree_sitter_shards {
    ($($test:ident: $shard:literal,)*) => {
        const TREE_SITTER_SHARDS: usize = [$($shard),*].len();
        $(#[test]
        fn $test() {
            for (name, start, snippets) in SESSIONS.iter().skip($shard).step_by(TREE_SITTER_SHARDS) {
                fuzz_tree_sitter(name, start, snippets);
            }
        })*
    };
}

tree_sitter_shards! {
    tree_sitter_spans_after_edits_equal_a_fresh_parse_or_tree_sitter_s_own_replay_0: 0,
    tree_sitter_spans_after_edits_equal_a_fresh_parse_or_tree_sitter_s_own_replay_1: 1,
    tree_sitter_spans_after_edits_equal_a_fresh_parse_or_tree_sitter_s_own_replay_2: 2,
    tree_sitter_spans_after_edits_equal_a_fresh_parse_or_tree_sitter_s_own_replay_3: 3,
    tree_sitter_spans_after_edits_equal_a_fresh_parse_or_tree_sitter_s_own_replay_4: 4,
    tree_sitter_spans_after_edits_equal_a_fresh_parse_or_tree_sitter_s_own_replay_5: 5,
    tree_sitter_spans_after_edits_equal_a_fresh_parse_or_tree_sitter_s_own_replay_6: 6,
    tree_sitter_spans_after_edits_equal_a_fresh_parse_or_tree_sitter_s_own_replay_7: 7,
}

#[test]
fn a_node_error_recovery_rebuilds_above_the_edit_is_repainted() {
    replay_tree_sitter(
        "f.py",
        include_bytes!("../../../tests/colors/python/classes.py"),
        PYTHON,
        2204..=2204,
    );
    replay_tree_sitter(
        "F.java",
        include_bytes!("../../../tests/colors/java/Ledger.java"),
        JAVA,
        2445..=2445,
    );
    replay_tree_sitter(
        "f.sol",
        include_bytes!("../../../tests/colors/solidity/Token.sol"),
        SOLIDITY,
        614..=614,
    );
}

#[test]
fn an_edit_in_one_member_of_a_combined_injection_repaints_the_others() {
    let php = include_bytes!("../../../tests/colors/php/Ledger.php");
    let xml = include_bytes!("../../../tests/colors/xml/project.xml");
    let docker = include_bytes!("../../../tests/colors/dockerfile/Dockerfile");
    let cases: &[(&str, &[u8], &[&str], u64)] = &[
        ("f.php", php, PHP, 5),
        ("f.php", php, PHP, 1085),
        ("f.php", php, PHP, 1807),
        ("f.xml", xml, MARKUP, 385),
        ("f.xml", xml, MARKUP, 1085),
        ("f.xml", xml, MARKUP, 1269),
        ("Dockerfile", docker, DOCKERFILE, 1),
        ("Dockerfile", docker, DOCKERFILE, 453),
        ("Dockerfile", docker, DOCKERFILE, 256),
    ];
    for &(name, start, snippets, seed) in cases {
        replay_tree_sitter(name, start, snippets, seed..=seed);
    }
}

#[test]
fn a_tree_tree_sitter_builds_from_the_same_edits_is_painted_as_built() {
    replay_tree_sitter(
        "F.swift",
        include_bytes!("../../../tests/colors/swift/Ledger.swift"),
        SWIFT,
        754..=754,
    );
}

#[test]
fn a_nested_combined_injection_joins_members_above_the_paint() {
    replay_tree_sitter(
        "f.md",
        include_bytes!("../../../tests/colors/markdown/code.md"),
        MARKDOWN,
        4366..=4366,
    );
}

#[test]
fn captures_of_one_byte_range_paint_in_the_order_of_a_paint_from_the_top() {
    replay_tree_sitter(
        "f.nix",
        include_bytes!("../../../tests/colors/nix/ledger.nix"),
        NIX,
        32..=32,
    );
}

#[test]
fn a_combined_injection_over_many_scan_steps_equals_a_fresh_parse() {
    let page = include_str!("../../../tests/colors/php/Ledger.php").repeat(7);
    assert!(page.len() > 16 * 1024);
    replay_tree_sitter("f.php", page.as_bytes(), PHP, 1..=4);
}

#[test]
fn two_combined_patterns_into_one_language_keep_a_tree_each() {
    let text = include_str!("../../../tests/colors/nix/ledger.nix")
        .replace("buildPhase = ''", "buildPhase = /* bash */ ''");
    let mut fresh = Buffer::from_bytes(PathBuf::from("f.nix"), text.as_bytes());
    fresh.parse_more(None);
    let bash = fresh
        .groups()
        .iter()
        .filter(|g| g.key.starts_with("bash"))
        .count();
    assert!(bash >= 2, "{:?}", fresh.groups());
    let snippets = &[
        "/* bash */ ",
        "''",
        "buildPhase = ",
        "preBuild = ",
        "$out ",
        "${x}",
        ";",
        "\n",
    ];
    replay_tree_sitter("f.nix", text.as_bytes(), snippets, 1..=20);
}

#[test]
fn a_paint_from_any_row_orders_captures_at_one_byte_as_a_paint_from_the_top() {
    replay_tree_sitter(
        "f.cpp",
        include_bytes!("../../../tests/colors/cpp/report.cpp"),
        CPP,
        1950..=1950,
    );
}

#[test]
fn a_resumed_parse_that_stops_short_of_the_text_is_redone_uncut() {
    replay_tree_sitter(
        "f.yml",
        include_bytes!("../../../tests/colors/yaml/workflow.yml"),
        DATA,
        3020..=3020,
    );
}

fn fuzz_tree_sitter(name: &str, start: &[u8], snippets: &[&str]) {
    let env = |name| std::env::var(name).ok().and_then(|v| v.parse::<u64>().ok());
    let seeds = match env("MERL_FUZZ_SEED") {
        Some(s) => s..=s,
        None => 1..=env("MERL_FUZZ_SESSIONS").unwrap_or(40),
    };
    replay_tree_sitter(name, start, snippets, seeds);
}

fn replay_tree_sitter(
    name: &str,
    start: &[u8],
    snippets: &[&str],
    seeds: std::ops::RangeInclusive<u64>,
) {
    crate::treesitter::RECORD.set(true);
    let themes = ["tokyonight-moon", "github-dark"].map(|t| crate::theme::load(t).unwrap());
    let dir = std::env::temp_dir().join(format!(
        "merl-ts-fuzz-{}-{}",
        std::process::id(),
        seeds.start()
    ));
    let path = dir.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    for seed in seeds {
        let theme = &themes[usize::from(seed % 2 == 0)];
        let mut rng = Xorshift64::new(seed);
        std::fs::write(&path, start).unwrap();
        let buf = Buffer::from_bytes(path.clone(), start);
        let mut a = App::new(dir.clone(), Tree::default(), Vec::new(), buf, None);
        (a.view_w, a.view_h) = (WIDER_THAN_ANY_LINE, 40);
        a.line = rng.below(a.buf.lines.len());
        a.key(KeyEvent::new(KeyCode::Enter, NONE));
        let mut done = Vec::new();
        for _ in 0..60 {
            let op = match rng.below(4) {
                0 => Op::Paste(rng.pick(snippets).to_string()),
                _ => gen_op(&mut rng),
            };
            match &op {
                Op::Key(KeyCode::Char('r'), m) if m.contains(CTRL) => continue,
                Op::Key(code, mods) => _ = a.key(KeyEvent::new(*code, *mods)),
                Op::Paste(t) => a.paste(t),
                Op::PasteClip => a.paste("x"),
            }
            done.push(op.to_string());
            for _ in 0..rng.below(3) {
                a.buf.parse_more(Some(Duration::ZERO));
                a.buf.paint_to(rng.below(a.buf.lines.len()), theme);
            }
            if rng.below(3) > 0 {
                continue;
            }
            a.buf.parse_more(None);
            a.buf.paint_to(usize::MAX, theme);
            let text = a.buf.lines.join("\n") + "\n";
            let mut fresh = Buffer::from_bytes(path.clone(), text.as_bytes());
            fresh.parse_more(None);
            fresh.paint_to(usize::MAX, theme);
            let (shape, fresh_shape) = (a.buf.shape(), fresh.shape());
            let at = format!(
                "{name} seed {seed} in {:?}, after [{}]",
                theme.builtin,
                done.join(", ")
            );
            let want = if groups_as_fresh(&a.buf, &fresh, shape == fresh_shape, &at) {
                fresh.hl
            } else {
                if shape != fresh_shape {
                    let (own, own_sexp) = a.buf.own();
                    assert!(
                        recovered(&shape)
                            || recovered(&fresh_shape)
                            || tokens(&shape) == tokens(&fresh_shape)
                            || shape == own,
                        "{at}: the tree differs from a fresh parse without an error in either, and from the one tree-sitter builds from the same edits\nincremental {}\nfresh {}\ntree-sitter {own_sexp}",
                        a.buf.sexp(),
                        fresh.sexp()
                    );
                    assert_eq!(
                        a.buf.root_bytes(),
                        fresh.root_bytes(),
                        "{at}: the tree does not span the text"
                    );
                }
                let kept = a.buf.hl.clone();
                a.buf.clear_hl();
                a.buf.paint_to(usize::MAX, theme);
                std::mem::replace(&mut a.buf.hl, kept)
            };
            if let Some(l) = (0..a.buf.lines.len()).find(|&l| a.buf.hl[l] != want[l]) {
                panic!(
                    "{at}: line {} {:?}\nincremental {:?}\nfresh {:?}",
                    l + 1,
                    a.buf.lines[l],
                    a.buf.hl[l],
                    want[l]
                );
            }
        }
    }
}

fn recovered(s: &crate::treesitter::Shape) -> bool {
    s.iter()
        .any(|(kind, _, missing, _)| *missing || *kind == u16::MAX)
}

fn tokens(s: &crate::treesitter::Shape) -> Vec<(u16, std::ops::Range<usize>)> {
    s.iter()
        .filter(|n| n.3)
        .map(|n| (n.0, n.1.clone()))
        .collect()
}

fn groups_as_fresh(buf: &Buffer, fresh: &Buffer, top_same: bool, at: &str) -> bool {
    let (groups, fresh_groups) = (buf.groups(), fresh.groups());
    if let Some(g) = groups.iter().find(|g| g.stale) {
        panic!(
            "{at}: the {} group keeps a tree parsed from text its members no longer hold",
            g.key
        );
    }
    if !top_same {
        return false;
    }
    let members = |gs: &[crate::treesitter::GroupShape]| {
        gs.iter()
            .map(|g| (g.key.clone(), g.ranges.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        members(&groups),
        members(&fresh_groups),
        "{at}: the combined injections' members differ from a fresh parse's"
    );
    groups.iter().zip(&fresh_groups).fold(true, |same, (g, f)| {
        if g.shape == f.shape {
            return same;
        }
        assert!(
            recovered(&g.shape)
                || recovered(&f.shape)
                || tokens(&g.shape) == tokens(&f.shape)
                || g.shape == g.own,
            "{at}: the {} group's tree differs from a fresh parse's without an error in either, and from the one tree-sitter builds from the same edits and ranges\nincremental {}\nfresh {}\ntree-sitter {}",
            g.key,
            g.sexp,
            f.sexp,
            g.own_sexp
        );
        false
    })
}
