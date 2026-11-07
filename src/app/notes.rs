use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{App, Edit, Mode};
use crate::git::{self, TextLine};
use crate::key::{KeyCode, KeyEvent, KeyModifiers};
use crate::line_edit::LineEdit;

pub type SlotRow = (String, usize, Option<std::ops::Range<usize>>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub path: PathBuf,
    pub first: usize,
    pub last: usize,
    pub quote: Vec<String>,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Here {
    pub first: TextLine,
    pub at: TextLine,
    pub idx: usize,
    pub stale: bool,
}

pub enum Slot {
    Note(usize),
    Input,
}

pub struct Editing {
    pub old: Option<Note>,
    pub first: TextLine,
    pub last: TextLine,
    quote: Vec<String>,
    from: Mode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteEdit {
    idx: usize,
    old: Option<Note>,
    new: Option<Note>,
}

#[derive(Default)]
#[expect(
    clippy::option_option,
    reason = "`dirs` is None until looked up, then the lookup's own Option"
)]
pub struct Notes {
    pub on: bool,
    pub list: Vec<Note>,
    pub here: Vec<Here>,
    pub editing: Option<Editing>,
    pub focus: Option<usize>,
    pub selected: bool,
    dirs: Option<Option<git::GitDirs>>,
    seen: Option<SystemTime>,
    loaded: bool,
    last_seen: HashMap<String, TextLine>,
}

pub fn store(dirs: &git::GitDirs) -> Option<(PathBuf, String)> {
    let head = std::fs::read_to_string(dirs.worktree_head_dir.join("HEAD")).ok()?;
    let branch = match head.trim_end().strip_prefix("ref: refs/heads/") {
        Some(b) => b.to_owned(),
        None => git::folder_branch(dirs)?,
    };
    Some((
        dirs.common_refs_dir.join("merl/notes").join(&branch),
        branch,
    ))
}

pub fn parse(text: &str) -> Vec<Note> {
    let mut lines = text.lines().skip_while(|l| l.starts_with('#')).peekable();
    let mut out = Vec::new();
    loop {
        while lines.next_if(|l| l.trim().is_empty()).is_some() {}
        let Some(place) = lines.next() else {
            return out;
        };
        let mut block = Vec::new();
        while let Some(l) = lines.next_if(|l| !l.trim().is_empty()) {
            block.push(l);
        }
        let Some((path, range)) = place.rsplit_once(':') else {
            continue;
        };
        let (a, b) = range.split_once('-').unwrap_or((range, range));
        let (Ok(first), Ok(last)) = (a.parse(), b.parse()) else {
            continue;
        };
        let quotes = block
            .iter()
            .take_while(|l| is_quote(l))
            .count()
            .min(block.len().saturating_sub(1));
        let text = block[quotes..].join("\n");
        let text = match text.strip_prefix(' ') {
            Some(t) if is_quote(t) => t.to_owned(),
            _ => text,
        };
        out.push(Note {
            path: path.into(),
            first,
            last,
            quote: block[..quotes].iter().map(ToString::to_string).collect(),
            text,
        });
    }
}

fn is_quote(line: &str) -> bool {
    line.starts_with("> ") || line.starts_with("- ")
}

pub fn format(notes: &[Note], head: &str) -> String {
    let mut s = format!(
        "# merl notes \u{b7} {head}\n# The user's review notes, for you. Delete this file when you have dealt with them.\n"
    );
    for n in notes {
        let range = if n.first == n.last {
            n.last.to_string()
        } else {
            format!("{}-{}", n.first, n.last)
        };
        _ = write!(s, "\n{}:{range}\n", n.path.display());
        for q in &n.quote {
            s += q;
            s += "\n";
        }
        if is_quote(&n.text) {
            s += " ";
        }
        s += &n.text;
        s += "\n";
    }
    s
}

fn quote_of(app: &App, l: TextLine) -> String {
    match l {
        TextLine::File(n) => format!("> {}", app.buf.lines.get(n).map_or("", |s| s.as_str())),
        TextLine::Deleted(k, i) => format!(
            "- {}",
            app.diff
                .ghosts
                .get(&k)
                .and_then(|g| g.get(i))
                .map_or("", |s| s.as_str())
        ),
    }
}

fn prev(app: &App, l: TextLine) -> TextLine {
    match l {
        TextLine::Deleted(k, i) if i > 0 => TextLine::Deleted(k, i - 1),
        TextLine::Deleted(k, _) => TextLine::File(k.saturating_sub(1)),
        TextLine::File(n) => match app.diff.ghosts.get(&n).map_or(0, Vec::len) {
            0 => TextLine::File(n.saturating_sub(1)),
            g => TextLine::Deleted(n, g - 1),
        },
    }
}

fn next(app: &App, l: TextLine) -> TextLine {
    match l {
        TextLine::Deleted(k, i) if i + 1 < app.diff.ghosts.get(&k).map_or(0, Vec::len) => {
            TextLine::Deleted(k, i + 1)
        }
        TextLine::Deleted(k, _) => TextLine::File(k),
        TextLine::File(n) => match app.diff.ghosts.get(&(n + 1)).map_or(0, Vec::len) {
            0 => TextLine::File(n + 1),
            _ => TextLine::Deleted(n + 1, 0),
        },
    }
}

impl App {
    fn notes_store(&mut self) -> Option<(PathBuf, String)> {
        let root = &self.root;
        let dirs = self.notes.dirs.get_or_insert_with(|| git::dirs(root));
        store(dirs.as_ref()?)
    }

    fn notes_reload(&mut self) {
        let Some((path, _)) = self.notes_store() else {
            return;
        };
        let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if self.notes.loaded && mtime == self.notes.seen {
            return;
        }
        let list = std::fs::read_to_string(&path)
            .map(|t| parse(&t))
            .unwrap_or_default();
        if self.notes.loaded && list != self.notes.list {
            self.notes.focus = None;
            self.notes.selected = false;
        }
        self.notes.list = list;
        self.notes.seen = mtime;
        self.notes.loaded = true;
        self.notes_resolve();
    }

    pub fn notes_sync(&mut self) {
        if !self.notes.on {
            return;
        }
        self.notes_reload();
        self.notes_resolve();
    }

    pub fn notes_resolve(&mut self) {
        let rel = self.rel_current();
        let len = self.buf.lines.len();
        let mut here = Vec::new();
        let mut seen_now = Vec::new();
        for (idx, n) in self.notes.list.iter().enumerate() {
            if rel.as_deref() != Some(n.path.as_path()) {
                continue;
            }
            let Some(last_q) = n.quote.last() else {
                continue;
            };
            let id = format!("{}:{}", n.path.display(), n.text);
            let seen = self.notes.last_seen.get(&id).copied().filter(|t| match *t {
                TextLine::File(l) => l < len,
                TextLine::Deleted(key, i) => {
                    self.diff.ghosts.get(&key).is_some_and(|g| i < g.len())
                }
            });
            let k = seen.map_or_else(|| n.last.saturating_sub(1), TextLine::key);
            let found = if let Some(text) = last_q.strip_prefix("- ") {
                (self.diff.ghosts.iter())
                    .flat_map(|(&key, g)| {
                        g.iter()
                            .enumerate()
                            .filter(|(_, s)| s.as_str() == text)
                            .map(move |(i, _)| TextLine::Deleted(key, i))
                    })
                    .min_by_key(|t| t.key().abs_diff(k))
            } else {
                let text = last_q.get(2..).unwrap_or("");
                (0..len)
                    .filter(|&l| self.buf.lines[l] == text)
                    .min_by_key(|l| l.abs_diff(k))
                    .map(TextLine::File)
            };
            let (at, stale) = match found {
                Some(at) => (at, false),
                None => (
                    seen.unwrap_or_else(|| TextLine::File(k.min(len.saturating_sub(1)))),
                    true,
                ),
            };
            if !stale {
                seen_now.push((id, at));
            }
            let mut first = at;
            for _ in 1..n.quote.len() {
                first = prev(self, first);
            }
            here.push(Here {
                first,
                at,
                idx,
                stale,
            });
        }
        self.notes.here = here;
        self.notes.last_seen.extend(seen_now);
    }

    fn here_of(&self, idx: usize) -> Option<&Here> {
        self.notes.here.iter().find(|h| h.idx == idx)
    }

    pub fn note_label(&self, first: TextLine, at: TextLine) -> String {
        if first == at {
            return String::new();
        }
        let num = |t: TextLine| match t {
            TextLine::File(l) => (l + 1).to_string(),
            TextLine::Deleted(k, i) => {
                format!(
                    "-{}",
                    self.diff.ghost_from.get(&k).copied().unwrap_or(0) + i + 1
                )
            }
        };
        format!("lines {} to {}", num(first), num(at))
    }

    pub fn note_span(&self) -> Option<(TextLine, TextLine)> {
        match self.mode {
            Mode::Note => self.notes.editing.as_ref().map(|e| (e.first, e.last)),
            _ => self
                .notes
                .focus
                .and_then(|i| self.here_of(i))
                .map(|h| (h.first, h.at)),
        }
    }

    pub fn slots_after(&self, l: TextLine) -> Vec<Slot> {
        let editing = self
            .notes
            .editing
            .as_ref()
            .filter(|_| self.mode == Mode::Note);
        let editing_idx = editing
            .and_then(|e| e.old.as_ref())
            .and_then(|o| self.notes.list.iter().position(|n| n == o));
        let mut v: Vec<Slot> = (self.notes.here.iter().enumerate())
            .filter(|(_, h)| h.at == l)
            .map(|(i, h)| {
                if editing_idx == Some(h.idx) {
                    Slot::Input
                } else {
                    Slot::Note(i)
                }
            })
            .collect();
        if let Some(e) = editing
            && editing_idx.is_none()
            && e.last == l
        {
            v.push(Slot::Input);
        }
        v
    }

    pub fn slot_text(&self, s: &Slot) -> (String, String, bool) {
        match s {
            Slot::Note(i) => {
                let h = &self.notes.here[*i];
                let t = &self.notes.list[h.idx].text;
                let text = if h.stale {
                    format!("{t} (changed)")
                } else {
                    t.clone()
                };
                (self.note_label(h.first, h.at), text, h.stale)
            }
            Slot::Input => {
                let (first, last) = self
                    .notes
                    .editing
                    .as_ref()
                    .map_or((TextLine::File(0), TextLine::File(0)), |e| {
                        (e.first, e.last)
                    });
                (
                    self.note_label(first, last),
                    format!("{} ", &*self.prompt),
                    false,
                )
            }
        }
    }

    #[expect(
        clippy::string_slice,
        reason = "r is a row of wrap_shown and p is the prompt's cursor, char boundaries of full"
    )]
    pub fn slot_layout(&self, s: &Slot) -> (Vec<SlotRow>, Option<(usize, usize)>) {
        let (label, text, _) = self.slot_text(s);
        let head = if label.is_empty() {
            String::new()
        } else {
            format!("{label}  ")
        };
        let pad = head.len();
        let w = self.view_w.saturating_sub(2).max(1);
        let input = matches!(s, Slot::Input);
        let cursor = input.then(|| self.prompt.cursor());
        let sel = input.then(|| self.prompt.selection()).flatten();
        let mut rows = Vec::new();
        let mut at = None;
        let mut off = 0;
        for (i, line) in text.split('\n').enumerate() {
            let lead = match i {
                0 => head.clone(),
                _ => " ".repeat(pad),
            };
            let full = format!("{lead}{line}");
            let pos = cursor
                .filter(|&c| c >= off && c <= off + line.len())
                .map(|c| pad + c - off);
            for r in crate::wrap::wrap_shown(&full, w) {
                if let Some(p) = pos
                    && at.is_none()
                    && (r.contains(&p) || r.end == p)
                {
                    at = Some((rows.len(), crate::wrap::width(&full[r.start..p])));
                }
                let lab = match i {
                    0 => r.end.min(pad).saturating_sub(r.start),
                    _ => 0,
                };
                let mark = sel.as_ref().and_then(|x| {
                    let a = (x.start.max(off).min(off + line.len()) - off + pad).max(r.start);
                    let b = (x.end.max(off).min(off + line.len()) - off + pad).min(r.end);
                    (a < b).then(|| a - r.start..b - r.start)
                });
                rows.push((full[r].to_string(), lab, mark));
            }
            off += line.len() + 1;
        }
        (rows, at)
    }

    pub fn notes_reveal_input(&mut self) {
        if self.mode != Mode::Note {
            return;
        }
        let Some(k) = self.notes.editing.as_ref().map(|e| e.last.key()) else {
            return;
        };
        let need = (k, self.row_count(k).saturating_sub(1));
        let top = (self.top_line, self.top_row);
        if top > need {
            return;
        }
        (self.top_line, self.top_row) = self.fit_top(top, need);
    }

    fn slot_rows(&self, s: &Slot) -> usize {
        self.slot_layout(s).0.len()
    }

    fn slot_idx(&self, s: &Slot) -> Option<usize> {
        match s {
            Slot::Note(i) => Some(self.notes.here[*i].idx),
            Slot::Input => None,
        }
    }

    fn key_layout(&self, key: usize) -> Vec<(usize, Option<usize>)> {
        let mut out = Vec::new();
        let ghosts = self.diff.ghosts.get(&key).map_or(&[][..], Vec::as_slice);
        for (i, g) in ghosts.iter().enumerate() {
            out.push((self.text_rows(g).len(), None));
            for s in self.slots_after(TextLine::Deleted(key, i)) {
                out.push((self.slot_rows(&s), self.slot_idx(&s)));
            }
        }
        if key < self.buf.lines.len() {
            out.push((self.rows(key).len(), None));
            for s in self.slots_after(TextLine::File(key)) {
                out.push((self.slot_rows(&s), self.slot_idx(&s)));
            }
        }
        out
    }

    pub fn note_at_row(&self, key: usize, row: usize) -> Option<usize> {
        if self.notes.here.is_empty() {
            return None;
        }
        let mut r = row;
        for (n, idx) in self.key_layout(key) {
            if r < n {
                return idx;
            }
            r -= n;
        }
        None
    }

    pub fn note_row(&self, idx: usize) -> Option<usize> {
        let h = self.here_of(idx)?;
        let mut r = 0;
        for (n, i) in self.key_layout(h.at.key()) {
            if i == Some(idx) {
                return Some(r);
            }
            r += n;
        }
        None
    }

    pub fn note_rows_after(&self, l: TextLine) -> usize {
        if self.notes.here.is_empty() && self.mode != Mode::Note {
            return 0;
        }
        self.slots_after(l).iter().map(|s| self.slot_rows(s)).sum()
    }

    pub fn notes_in(&self, rel: &Path) -> usize {
        self.notes.list.iter().filter(|n| n.path == rel).count()
    }

    pub(super) fn add_note(&mut self) {
        let on = self
            .notes
            .focus
            .take()
            .and_then(|idx| self.here_of(idx))
            .map(|h| (h.first, h.at));
        if let Some((first, at)) = on {
            self.anchor = (first != at).then_some((first, 0));
            self.set_at(at);
            self.col = 1;
        }
        self.start_note();
    }

    pub(super) fn start_note(&mut self) {
        if !self.notes.on {
            return;
        }
        if self.rel_current().is_none() {
            return;
        }
        if self.notes_store().is_none() {
            self.message = "notes need a branch".into();
            return;
        }
        self.notes_sync();
        let focused = (self.notes.focus)
            .and_then(|idx| self.here_of(idx))
            .map(|h| (h.idx, h.first, h.at));
        let (first, last, old) = match (focused, self.selection()) {
            (Some((i, f, a)), _) => (f, a, Some(self.notes.list[i].clone())),
            (None, Some(((a, _), (b, col)))) => {
                let b = if col == 0 && b != a { prev(self, b) } else { b };
                (a, b, None)
            }
            (None, None) => (self.at(), self.at(), None),
        };
        let mut quote = Vec::new();
        let mut l = first;
        loop {
            quote.push(quote_of(self, l));
            if l >= last {
                break;
            }
            l = next(self, l);
        }
        let text = old.as_ref().map_or("", |n| n.text.as_str());
        self.prompt = if std::mem::take(&mut self.notes.selected) {
            LineEdit::selected(text)
        } else {
            LineEdit::typed(text)
        };
        self.notes.editing = Some(Editing {
            old,
            first,
            last,
            quote,
            from: self.mode,
        });
        self.message.clear();
        self.mode = Mode::Note;
    }

    fn note_commit(&mut self) -> Option<usize> {
        let typed = std::mem::take(&mut self.prompt).to_string();
        let e = self.notes.editing.take()?;
        let text = (typed.lines())
            .map(str::trim_end)
            .filter(|l| !l.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        self.notes_reload();
        let pos = (e.old.as_ref()).and_then(|o| self.notes.list.iter().position(|n| n == o));
        let old = pos.map(|p| self.notes.list[p].clone());
        if old.is_none() && e.old.as_ref().is_some_and(|o| o.text == text) {
            return None;
        }
        let new = match (&old, text.is_empty()) {
            (Some(o), _) if o.text == text => return pos,
            (_, true) => None,
            (Some(o), false) => Some(Note { text, ..o.clone() }),
            (None, false) => Some(Note {
                path: self.rel_current()?,
                first: e.first.key() + 1,
                last: e.last.key() + 1,
                quote: e.quote,
                text,
            }),
        };
        if old.is_none() && new.is_none() {
            return None;
        }
        let change = NoteEdit {
            idx: pos.unwrap_or(self.notes.list.len()),
            old,
            new,
        };
        self.apply_note(&change, false);
        let at = (self.line, self.col);
        self.undo.push(Edit {
            line: self.line,
            old: Vec::new(),
            new: Vec::new(),
            before: at,
            after: at,
            format: None,
            note: Some(Box::new(change.clone())),
        });
        self.undo_break = true;
        self.redo.clear();
        change
            .new
            .and_then(|n| self.notes.list.iter().position(|m| *m == n))
    }

    fn apply_note(&mut self, change: &NoteEdit, back: bool) -> Option<usize> {
        let (from, to) = if back {
            (&change.new, &change.old)
        } else {
            (&change.old, &change.new)
        };
        self.notes_reload();
        let pos = from
            .as_ref()
            .and_then(|f| self.notes.list.iter().position(|n| n == f));
        match (from, pos, to.clone()) {
            (Some(_), None, _) | (None, _, None) => return None,
            (_, Some(p), Some(t)) => self.notes.list[p] = t,
            (_, Some(p), None) => {
                self.notes.list.remove(p);
            }
            (None, _, Some(t)) => {
                let i = change.idx.min(self.notes.list.len());
                self.notes.list.insert(i, t);
            }
        }
        self.notes_write();
        self.notes_resolve();
        to.as_ref()
            .and_then(|t| self.notes.list.iter().position(|n| n == t))
    }

    pub(super) fn undo_note(&mut self, change: &NoteEdit, back: bool) {
        let gone_at = if back { &change.new } else { &change.old }
            .as_ref()
            .and_then(|f| self.notes.list.iter().position(|n| n == f))
            .and_then(|i| self.here_of(i))
            .map(|h| h.at);
        let from = if back { &change.new } else { &change.old };
        let gone = from.is_some() && gone_at.is_none() && {
            self.notes_reload();
            !from.as_ref().is_some_and(|f| self.notes.list.contains(f))
        };
        if gone {
            self.message = "note changed outside merl".into();
            return;
        }
        let kept = self.apply_note(change, back);
        self.notes.focus = None;
        if let Some(i) = kept
            && let Some(at) = self.here_of(i).map(|h| h.at)
        {
            self.set_at(at);
            self.notes.focus = Some(i);
            if self.mode == Mode::Edit {
                self.start_note();
            }
        } else if let Some(at) = gone_at {
            self.set_at(at);
        }
    }

    #[expect(
        clippy::string_slice,
        reason = "LineEdit keeps its cursor on a char boundary"
    )]
    fn note_line(&self) -> (usize, usize, usize) {
        let c = self.prompt.cursor();
        let start = self.prompt[..c].rfind('\n').map_or(0, |i| i + 1);
        let end = self.prompt[c..]
            .find('\n')
            .map_or(self.prompt.len(), |i| c + i);
        (start, end, c - start)
    }

    fn note_from(&self) -> Mode {
        self.notes.editing.as_ref().map_or(Mode::Normal, |e| e.from)
    }

    fn note_leave(&mut self, n: isize) {
        let last = self.notes.editing.as_ref().map(|e| e.last);
        let from = self.note_from();
        let kept = self.note_commit();
        self.mode = from;
        self.anchor = None;
        self.notes.focus = kept;
        match (kept, last) {
            (Some(_), _) => self.move_rows(n),
            (None, Some(l)) => {
                self.set_at(l);
                if n > 0 {
                    self.move_rows(n);
                }
            }
            (None, None) => {}
        }
    }

    fn note_esc(&mut self) {
        let from = self.note_from();
        self.notes.focus = self.note_commit();
        self.anchor = None;
        if let Some(h) = self.notes.focus.and_then(|i| self.here_of(i)) {
            self.set_at(h.at);
        }
        if from == Mode::Edit {
            self.notes.focus = None;
        }
        self.mode = from;
    }

    fn note_undo(&mut self, back: bool) {
        let from = self.note_from();
        self.notes.focus = self.note_commit();
        self.mode = from;
        self.undo(back);
        if self.mode == Mode::Edit && self.notes.focus.is_some() {
            self.start_note();
        }
    }

    #[expect(
        clippy::string_slice,
        reason = "every cut is the prompt's cursor or selection, or next to a newline"
    )]
    pub(super) fn note_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        match key.code {
            KeyCode::Esc => self.note_esc(),
            KeyCode::Char('a') if ctrl => {}
            KeyCode::Enter => {
                self.prompt.insert("\n");
            }
            KeyCode::Home | KeyCode::End if !ctrl => {
                let (start, end, _) = self.note_line();
                let to = if key.code == KeyCode::Home {
                    start
                } else {
                    end
                };
                if shift {
                    while self.prompt.cursor() != to {
                        let step = if self.prompt.cursor() < to {
                            KeyCode::Right
                        } else {
                            KeyCode::Left
                        };
                        self.prompt.key(KeyEvent::new(step, KeyModifiers::SHIFT));
                    }
                } else {
                    self.prompt.put(to);
                }
            }
            KeyCode::Up | KeyCode::Down if !shift => {
                let (start, end, col) = self.note_line();
                let up = key.code == KeyCode::Up;
                let to = if up {
                    (start > 0).then(|| {
                        let s = self.prompt[..start - 1].rfind('\n').map_or(0, |i| i + 1);
                        s + col.min(start - 1 - s)
                    })
                } else {
                    (end < self.prompt.len()).then(|| {
                        let s = end + 1;
                        let e = self.prompt[s..]
                            .find('\n')
                            .map_or(self.prompt.len(), |i| s + i);
                        s + col.min(e - s)
                    })
                };
                match to {
                    Some(t) => self.prompt.put(t),
                    None => self.note_leave(if up { -1 } else { 1 }),
                }
            }
            KeyCode::Char('c' | 'x') if ctrl => {
                let cut = key.code == KeyCode::Char('x');
                if let Some(r) = self.prompt.selection() {
                    self.clipboard = Some(self.prompt[r].to_string());
                    if cut {
                        self.prompt
                            .key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
                    }
                    self.message = "copied".into();
                    return;
                }
                let (start, end, _) = self.note_line();
                self.clipboard = Some(format!("{}\n", &self.prompt[start..end]));
                self.message = "copied 1 line".into();
                if !cut {
                    return;
                }
                let mut t = self.prompt.to_string();
                let gone = if end < t.len() {
                    start..end + 1
                } else {
                    start.saturating_sub(1)..end
                };
                t.replace_range(gone.clone(), "");
                self.prompt = LineEdit::typed(&t);
                self.prompt.put(gone.start.min(t.len()));
                if t.is_empty() {
                    self.note_leave(1);
                }
            }
            KeyCode::Char('z' | 'y') if ctrl => self.note_undo(key.code == KeyCode::Char('z')),
            KeyCode::Backspace if self.prompt.is_empty() && self.prompt.selection().is_none() => {
                self.note_leave(-1);
            }
            _ => {
                self.prompt.key(key);
            }
        }
    }

    fn notes_write(&mut self) {
        let Some((path, branch)) = self.notes_store() else {
            return;
        };
        if self.notes.list.is_empty() {
            drop(std::fs::remove_file(&path));
            self.notes.seen = None;
            return;
        }
        let sha = git::git(&self.root, &["rev-parse", "--short", "HEAD"]).unwrap_or_default();
        let head = match self.review.as_ref().map(|r| &r.base) {
            Some(b) => format!("{branch} \u{2190} {b} \u{b7} {sha}"),
            None => format!("{branch} \u{b7} {sha}"),
        };
        if let Some(dir) = path.parent() {
            drop(std::fs::create_dir_all(dir));
        }
        if let Err(e) = std::fs::write(&path, format(&self.notes.list, &head)) {
            self.message = format!("notes not saved: {e}");
        }
        self.notes.seen = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    }

    pub fn notes_farewell(&mut self) -> Option<String> {
        if !self.notes.on {
            return None;
        }
        let (path, _) = self.notes_store()?;
        let n = std::fs::read_to_string(&path)
            .map(|t| parse(&t).len())
            .ok()?;
        let root = self
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone());
        let shown = (path.strip_prefix(&self.root))
            .or_else(|_| path.strip_prefix(&root))
            .map_or_else(|_| path.clone(), Path::to_path_buf);
        Some(format!(
            "merl: {n} note{} for your agent in {}",
            super::plural(n),
            shown.display()
        ))
    }
}
