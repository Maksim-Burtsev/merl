//! #313: random edit sessions, played on merl's `App` and on a plain model of what the keys
//! mean (lines, a cursor, a selection, a clipboard, an undo stack), which have to agree after
//! every key: the text, the cursor, the selection, the mode, the clipboard and the file on disk.
//!
//! `cargo test edit_fuzz` plays seeds 1..=500; `MERL_FUZZ_SESSIONS=N` plays 1..=N and
//! `MERL_FUZZ_SEED=S` seed S alone. A failure prints the seed, the file it started from and the
//! shortest run of keys that still fails the same way; `MERL_FUZZ_ALL=1` goes on to list every
//! failing seed.
//!
//! The pane is wide enough that no line wraps: Up and Down move a line, not a screen row.
//! When an edit key changes meaning, the model changes with it.

use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use unicode_segmentation::UnicodeSegmentation;

use super::*;

type Pos = (usize, usize);

/// xorshift64: enough randomness for key sequences, and no crate.
struct Rng(u64);

impl Rng {
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
        (self.next() % n as u64) as usize
    }
    fn pick<'a, T>(&mut self, of: &'a [T]) -> &'a T {
        &of[self.below(of.len())]
    }
}

#[derive(Clone)]
enum Op {
    Key(KeyCode, KeyModifiers),
    /// Cmd+V of this text.
    Paste(String),
    /// Cmd+V of what merl last put on the clipboard.
    PasteClip,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Op::Key(KeyCode::Char(c), m) if m.is_empty() => write!(f, "{c:?}"),
            Op::Key(code, m) => f.write_str(&crate::stats::name(KeyEvent::new(*code, *m))),
            Op::Paste(t) => write!(f, "paste {t:?}"),
            Op::PasteClip => f.write_str("paste clipboard"),
        }
    }
}

impl Op {
    /// Keys that are text only in edit mode; in navigation they are commands (`q` quits, Tab
    /// goes to the tree), so a session that is not editing skips them.
    fn edit_only(&self) -> bool {
        match self {
            Op::Key(KeyCode::Char(c), m) => m.is_empty() && *c != 'v',
            Op::Key(KeyCode::Tab | KeyCode::Backspace | KeyCode::Delete, _) => true,
            _ => false,
        }
    }
}

const NONE: KeyModifiers = KeyModifiers::NONE;
const SHIFT: KeyModifiers = KeyModifiers::SHIFT;
const ALT: KeyModifiers = KeyModifiers::ALT;
const CTRL: KeyModifiers = KeyModifiers::CONTROL;

/// Typed one char at a time: ASCII, Cyrillic, CJK, and the pieces emoji are built of (a
/// variation selector, a ZWJ, a skin tone, a combining accent), so clusters form under the cursor.
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

/// A file to start from: a few lines, LF or CRLF, with or without the trailing newline, with or
/// without a BOM.
fn gen_file(rng: &mut Rng) -> Vec<u8> {
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

fn gen_op(rng: &mut Rng) -> Op {
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
        79..81 => key(*rng.pick(&[KeyCode::Home, KeyCode::End])),
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
    let mut rng = Rng::new(seed);
    let file = gen_file(&mut rng);
    let n = 40 + rng.below(120);
    // Into edit mode first, or the session starts by skipping its letters.
    let ops = std::iter::once(Op::Key(KeyCode::Enter, NONE))
        .chain((0..n).map(|_| gen_op(&mut rng)))
        .collect();
    (file, ops)
}

// The model.

#[derive(Clone, Copy, PartialEq, Debug)]
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
        .map(|l| l.strip_suffix('\r').unwrap_or(l).to_string())
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

/// One undo step: the whole text on either side of it.
struct Step {
    old: Vec<String>,
    new: Vec<String>,
    before: Pos,
    after: Pos,
    format: Option<(Format, Format)>,
    /// What it left is one line, so typing on that line can carry it on.
    one_line: bool,
}

struct Model {
    lines: Vec<String>,
    cur: Pos,
    /// Screen column Up / Down aim at.
    want_x: usize,
    anchor: Option<Pos>,
    editing: bool,
    clipboard: Option<String>,
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// The next edit starts an undo step of its own.
    fresh: bool,
    format: Format,
    disk: Vec<u8>,
    dirty: bool,
}

fn next(s: &str, i: usize) -> usize {
    s[i..].graphemes(true).next().map_or(i, |g| i + g.len())
}
fn prev(s: &str, i: usize) -> usize {
    s[..i]
        .graphemes(true)
        .next_back()
        .map_or(0, |g| i - g.len())
}
fn wordy(s: &str, i: usize) -> bool {
    s[i..].chars().next().is_some_and(is_word)
}
fn width(s: &str) -> usize {
    s.graphemes(true).map(crate::wrap::cluster_width).sum()
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
            fresh: false,
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
            return self.lines[from.0][from.1..to.1].to_string();
        }
        let mut parts = vec![&self.lines[from.0][from.1..]];
        parts.extend(self.lines[from.0 + 1..to.0].iter().map(String::as_str));
        parts.push(&self.lines[to.0][..to.1]);
        parts.join("\n")
    }

    fn sync_x(&mut self) {
        self.want_x = width(&self.line()[..self.cur.1]);
    }

    /// The cursor onto line `l`, on the first cluster at or past `want_x`, else at its end.
    fn aim(&mut self, l: usize) {
        let s = &self.lines[l];
        let mut x = 0;
        let mut col = s.len();
        for (i, g) in s.grapheme_indices(true) {
            if x >= self.want_x {
                col = i;
                break;
            }
            x += crate::wrap::cluster_width(g);
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

    /// Where Alt+Left lands: back over the gap, then over the word; at a line start, the end
    /// of the line above.
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

    /// Where Alt+Right lands: past the gap, then past the word; at a line end, the start of the
    /// line below.
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
            // On the last line Down scrolls; the cursor stays.
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

    /// The one way the text changes: `from..to` becomes `text`, the cursor after it. Typing
    /// that carries on where the last step ended, on one line, extends that step.
    fn replace(&mut self, from: Pos, to: Pos, text: &str) -> bool {
        // Nothing to take and nothing to put: no step, the redo kept (#455).
        if from == to && text.is_empty() {
            return false;
        }
        let before = self.cur;
        let old = self.lines.clone();
        let joined = format!(
            "{}{text}{}",
            &self.lines[from.0][..from.1],
            &self.lines[to.0][to.1..]
        );
        let new: Vec<String> = joined.split('\n').map(String::from).collect();
        let n = new.len();
        let tail = self.lines[to.0].len() - to.1;
        self.lines.splice(from.0..=to.0, new);
        let last = from.0 + n - 1;
        self.cur = (last, self.lines[last].len() - tail);
        let one_line = from.0 == to.0 && !text.contains('\n');
        match self.undo.last_mut() {
            Some(s) if !self.fresh && s.after == before && s.one_line && one_line => {
                s.new = self.lines.clone();
                s.after = self.cur;
            }
            _ => self.undo.push(Step {
                old,
                new: self.lines.clone(),
                before,
                after: self.cur,
                format: None,
                one_line: n == 1,
            }),
        }
        self.fresh = false;
        self.redo.clear();
        self.dirty = true;
        self.anchor = None;
        self.sync_x();
        true
    }

    fn insert(&mut self, text: &str) {
        let (from, to) = self.selection().unwrap_or((self.cur, self.cur));
        self.replace(from, to, text);
    }

    /// Ctrl+C: the selection, else the line with its break. Returns what Ctrl+X takes: on the
    /// last line, the break before it, so no empty line is left.
    fn copy(&mut self) -> (Pos, Pos) {
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

    /// Tab over lines: each line the selection touches gets an indent, in one step, and the
    /// selection keeps its text.
    fn indent(&mut self, from: Pos, to: Pos, indent: &str) {
        let last = if to.1 == 0 { to.0 - 1 } else { to.0 };
        let text = self.lines[from.0..=last]
            .iter()
            .map(|l| format!("{indent}{l}"))
            .collect::<Vec<_>>()
            .join("\n");
        let (anchor, cur) = (self.anchor, self.cur);
        self.fresh = true;
        self.replace((from.0, 0), (last, self.lines[last].len()), &text);
        self.fresh = true;
        let moved = |(l, c): Pos| match (from.0..=last).contains(&l) && c > 0 {
            true => (l, c + indent.len()),
            false => (l, c),
        };
        self.anchor = anchor.map(moved);
        self.cur = moved(cur);
        self.sync_x();
        // Redo lands where the Tab left the cursor (#456).
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
        self.fresh = true;
        self.sync_x();
        (if back { &mut self.redo } else { &mut self.undo }).push(step);
    }

    fn save(&mut self) {
        self.disk = write(&self.lines, self.format);
        self.dirty = false;
    }

    /// Ctrl+R: the file as the disk has it; the edits it drops are one undo step.
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
                one_line: false,
            });
        }
        (self.lines, self.format) = (lines, format);
        self.dirty = false;
        self.redo.clear();
        self.fresh = true;
        self.cur = self.clamp(self.cur);
        self.sync_x();
    }

    /// `v`: the word under the cursor, then the line, then the paragraph between blank lines:
    /// the first of them wider than the selection.
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
            self.insert(&text.replace("\r\n", "\n").replace('\r', "\n"));
        }
    }

    fn key(&mut self, code: KeyCode, m: KeyModifiers) {
        let (ctrl, shift, alt) = (m.contains(CTRL), m.contains(SHIFT), m.contains(ALT));
        if ctrl && code == KeyCode::Char('c') {
            self.copy();
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
                self.fresh = true;
            }
            KeyCode::Char('v') => self.grow(),
            KeyCode::Char('s') if ctrl => self.save(),
            KeyCode::Char('r') if ctrl => self.reload(),
            KeyCode::Char('z') if ctrl => self.undo(true),
            KeyCode::Char('y') if ctrl => self.undo(false),
            KeyCode::Home => {
                self.cur.1 = 0;
                self.sync_x();
            }
            KeyCode::End => {
                self.cur.1 = self.line().len();
                self.sync_x();
            }
            _ if shift => {
                self.anchor.get_or_insert(self.cur);
                self.arrow(code, alt);
            }
            // A plain arrow on a selection collapses it to its end on that side.
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
    }

    /// The keys edit mode has of its own; `false` for the rest, which move as in navigation.
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
                let last = self.selection().is_none() && l + 1 == self.lines.len();
                let (from, to) = self.copy();
                let want = self.want_x;
                if self.replace(from, to, "") && last {
                    // The last line cut: onto the line above, at the column it had.
                    self.want_x = want;
                    self.aim(self.cur.0);
                    self.undo.last_mut().unwrap().after = self.cur;
                }
            }
            KeyCode::Char(ch) if !ctrl => self.insert(&ch.to_string()),
            KeyCode::Enter => {
                let head = &self.line()[..c];
                let indent = &head[..head.len() - head.trim_start().len()];
                self.insert(&format!("\n{indent}"));
            }
            KeyCode::Tab => {
                let indent = if self.format.tabs { "\t" } else { "    " };
                match self.selection() {
                    Some((from, to)) if to.0 > from.0 => self.indent(from, to, indent),
                    _ => self.insert(indent),
                }
            }
            KeyCode::Backspace | KeyCode::Delete if self.selection().is_some() => self.insert(""),
            KeyCode::Backspace | KeyCode::Delete if alt => {
                let to = if code == KeyCode::Backspace {
                    self.word_left_of(self.cur)
                } else {
                    self.word_right_of(self.cur)
                };
                // A word of its own in the undo history.
                self.fresh = true;
                self.replace(self.cur.min(to), self.cur.max(to), "");
                self.fresh = true;
            }
            KeyCode::Backspace if c > 0 => _ = self.replace((l, prev(self.line(), c)), (l, c), ""),
            KeyCode::Backspace if l > 0 => {
                self.replace((l - 1, self.lines[l - 1].len()), (l, c), "");
            }
            KeyCode::Delete if c < self.line().len() => {
                self.replace((l, c), (l, next(self.line(), c)), "");
            }
            KeyCode::Delete if l + 1 < self.lines.len() => _ = self.replace((l, c), (l + 1, 0), ""),
            KeyCode::Backspace | KeyCode::Delete => {}
            _ => return false,
        }
        true
    }
}

// Playing a session.

/// How a session went wrong: the step, and what disagreed. `what` alone decides whether a
/// shorter session still fails "the same way".
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
    // No line wraps: the model moves Up / Down by lines.
    a.view_w = 10_000;
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
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()));
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
    // The end of the session drawn in a narrow pane, where its lines wrap: whatever the keys
    // left, emoji halves and all, has to draw.
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(24, 8)).unwrap();
    let drawn = catch_unwind(AssertUnwindSafe(|| {
        terminal
            .draw(|f| crate::ui::draw(f, &mut a, &theme))
            .map(|_| ())
    }));
    if !matches!(drawn, Ok(Ok(()))) {
        let step = ops.len().saturating_sub(1);
        return Err(Failure {
            step,
            what: "draw",
            detail: "the last screen did not draw".into(),
        });
    }
    Ok(())
}

/// Drops keys while the session still fails the same way: halves, then quarters, down to
/// single keys.
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
        // One is enough to read; MERL_FUZZ_ALL=1 lists every failing seed.
        if std::env::var_os("MERL_FUZZ_ALL").is_none() {
            break;
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}
