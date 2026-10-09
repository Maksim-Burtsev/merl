use super::*;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Kind {
    Other,
    Typing,
    FirstSpace,
    Spaces,
    DeletingLeft,
    DeletingRight,
}

impl Kind {
    fn spaces(self) -> Self {
        match self {
            Kind::FirstSpace => Kind::Spaces,
            k => k,
        }
    }
}

impl App {
    pub(super) fn start_edit(&mut self) {
        if self.buf.path.is_none() {
            return;
        }
        if self.deleted.is_some() {
            self.message = "deleted".into();
            return;
        }
        if let Some(why) = self.locked(std::iter::once(&self.buf.lines[self.line])) {
            self.message = why;
            return;
        }
        self.edit_mode();
        self.undo_break = true;
    }

    /// Into edit mode, the one way there: the text being typed is never under the preview.
    pub(super) fn edit_mode(&mut self) {
        if self.previewing() {
            self.toggle_preview();
        }
        self.mode = Mode::Edit;
    }

    /// Why the text cannot change or be written, if it cannot: the buffer is not what would be
    /// written back, or one of `lines` is longer than the screen shows, so its tail would be
    /// edited blind.
    fn locked<'a>(&self, mut lines: impl Iterator<Item = &'a String>) -> Option<String> {
        if let Some(why) = self.buf.readonly {
            return Some(format!("read-only: {why}"));
        }
        lines
            .any(|l| Buffer::clips(l))
            .then(|| "line too long to edit".to_string())
    }

    /// Ctrl+C: the selection goes to the clipboard, without one the whole line with its break,
    /// as in VS Code; the lines a review deleted are copied as they were. Returns the range
    /// Ctrl+X then removes: the last line goes with the break before it, so no empty line is left
    /// behind (a file's only line is emptied). Ctrl+X is refused on a deleted line before this.
    pub(super) fn copy(&mut self) -> ((usize, usize), (usize, usize)) {
        let here = ((self.line, self.col), (self.line, self.col));
        let (from, to, text) = match self.selection() {
            Some(_) => {
                let (from, to) = self.file_selection().unwrap_or(here);
                (from, to, self.selected_text().unwrap())
            }
            None if self.deleted.is_some() => (here.0, here.1, format!("{}\n", self.line_str())),
            None if self.line + 1 < self.buf.lines.len() => (
                (self.line, 0),
                (self.line + 1, 0),
                format!("{}\n", self.line_str()),
            ),
            None => (
                match self.line {
                    0 => (0, 0),
                    l => (l - 1, self.buf.lines[l - 1].len()),
                },
                (self.line, self.line_str().len()),
                format!("{}\n", self.line_str()),
            ),
        };
        self.message = match text.lines().count() {
            1 if self.selection().is_some() && !text.ends_with('\n') => "copied".into(),
            1 => "copied 1 line".into(),
            n => format!("copied {n} lines"),
        };
        self.clipboard = Some(text);
        (from, to)
    }

    /// Keys that only mean something while editing. Returns `false` for every other key, which
    /// then falls through to the navigation keys: arrows, Home / End, the chord aliases.
    pub(super) fn edit_key(&mut self, code: KeyCode, ctrl: bool, alt: bool) -> bool {
        let changes = matches!(code, KeyCode::Char(c) if !ctrl || c == 'x')
            || matches!(
                code,
                KeyCode::Enter | KeyCode::Tab | KeyCode::Backspace | KeyCode::Delete
            );
        let joins = self.selection().is_none()
            && match code {
                KeyCode::Backspace => self.col == 0 && self.deleted_at(self.line) > 0,
                KeyCode::Delete => {
                    self.col == self.line_str().len() && self.deleted_at(self.line + 1) > 0
                }
                _ => false,
            };
        if changes && (self.on_deleted() || joins) {
            self.message = "deleted".into();
            return true;
        }
        let beside = match code {
            KeyCode::Backspace if self.col == 0 => self.line.checked_sub(1),
            KeyCode::Delete if self.col == self.line_str().len() => Some(self.line + 1),
            _ => None,
        };
        if let Some(l) = beside.filter(|&l| self.selection().is_none() && self.hidden(l)) {
            self.unfold_over(l);
            return true;
        }
        match code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.flush();
            }
            KeyCode::Char('c' | 'x') if ctrl => {
                let last = self.selection().is_none() && self.line + 1 == self.buf.lines.len();
                let (from, to) = self.copy();
                let want = self.want_x;
                if code == KeyCode::Char('x') && self.replace(from, to, "", Kind::Other) && last {
                    // The last line cut: the cursor goes up onto the line above, aiming at the
                    // column it had; redo lands there too.
                    self.want_x = want;
                    self.apply_want_x(0);
                    self.undo.last_mut().unwrap().after = (self.line, self.col);
                }
            }
            KeyCode::Char(c) if !ctrl => self.insert(&c.to_string(), Kind::Typing),
            KeyCode::Enter => {
                let head = &self.line_str()[..self.col];
                let indent = &head[..head.len() - head.trim_start().len()];
                self.insert(&format!("\n{indent}"), Kind::Typing);
            }
            KeyCode::Tab => {
                let indent = if self.buf.tabs { "\t" } else { buffer::TAB };
                match self.file_selection() {
                    // Over lines Tab indents them, as in VS Code; over a piece of one line it
                    // is typed in place of it, as any other letter is.
                    Some((from, to)) if to.0 > from.0 => self.indent(from, to, indent),
                    _ => self.insert(indent, Kind::Other),
                }
            }
            KeyCode::Backspace | KeyCode::Delete if self.selection().is_some() => {
                let kind = match code {
                    KeyCode::Backspace => Kind::DeletingLeft,
                    _ => Kind::DeletingRight,
                };
                self.insert("", kind);
            }
            // Option+Backspace / Option+Delete: up to where Alt+Left / Right would land.
            KeyCode::Backspace | KeyCode::Delete if alt => {
                let (at, want) = ((self.line, self.col), self.want_x);
                if code == KeyCode::Backspace {
                    self.word_left();
                } else {
                    self.word_right();
                }
                let to = (self.line, self.col);
                // Undo puts the cursor back where the key was pressed, and takes the word alone:
                // not the typing before it, not the typing after. With no word to take, the
                // column Up / Down aim at stays too (#455).
                self.go(at);
                self.want_x = want;
                self.replace(at.min(to), at.max(to), "", Kind::Other);
            }
            KeyCode::Backspace => {
                let from = if self.col > 0 {
                    (self.line, prev_char(self.line_str(), self.col))
                } else if self.line > 0 {
                    (self.line - 1, self.buf.lines[self.line - 1].len())
                } else {
                    return true;
                };
                self.replace(from, (self.line, self.col), "", Kind::DeletingLeft);
            }
            KeyCode::Delete => {
                let to = if self.col < self.line_str().len() {
                    (self.line, next_char(self.line_str(), self.col))
                } else if self.line + 1 < self.buf.lines.len() {
                    (self.line + 1, 0)
                } else {
                    return true;
                };
                self.replace((self.line, self.col), to, "", Kind::DeletingRight);
            }
            _ => return false,
        }
        true
    }

    fn insert(&mut self, text: &str, kind: Kind) {
        let (from, to) = self
            .file_selection()
            .unwrap_or(((self.line, self.col), (self.line, self.col)));
        self.replace(from, to, text, kind);
    }

    /// Tab over a selection of more than one line: one indent at the start of every line the
    /// selection touches, in one undo step. A line where the selection ends at column 0 is not
    /// touched, as in VS Code.
    fn indent(&mut self, from: (usize, usize), to: (usize, usize), indent: &str) {
        let last = if to.1 == 0 { to.0 - 1 } else { to.0 };
        let text = (from.0..=last)
            .map(|l| format!("{indent}{}", self.buf.lines[l]))
            .collect::<Vec<_>>()
            .join("\n");
        let (anchor, cursor) = (self.anchor, (self.line, self.col));
        let anchor = anchor.map(|(t, c)| (t.key(), c));
        let end = (last, self.buf.lines[last].len());
        let indented = self.replace((from.0, 0), end, &text, Kind::Other);
        // The selection keeps the text it had. A position at the start of a line stays there,
        // so the lines stay whole and a second Tab indents the same ones again.
        let moved = |(l, c): (usize, usize)| match (from.0..=last).contains(&l) && c > 0 {
            true => (l, c + indent.len()),
            false => (l, c),
        };
        if indented {
            self.anchor = anchor.map(moved).map(|(l, c)| (TextLine::File(l), c));
            self.go(moved(cursor));
            self.sync_want_x();
            self.undo.last_mut().unwrap().after = (self.line, self.col);
        }
    }

    pub fn paste(&mut self, text: &str) {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let line = text.lines().next().unwrap_or_default();
        if self.mode == Mode::Edit && self.on_deleted() {
            self.message = "deleted".into();
        } else if self.mode == Mode::Edit {
            self.insert(&text, Kind::Other);
        } else if let Some(picker) = &mut self.picker {
            if let Pick::Typed = picker.paste(line) {
                self.search_typed();
            }
        } else if self.mode == Mode::Goto {
            // `:` computes nothing per key: its chars go in as typed, by the prompt's own rule.
            for c in line.chars() {
                self.goto_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            }
        } else if self.mode == Mode::Find {
            if self.prompt.insert(line) {
                self.refresh_find();
            }
        } else if self.mode == Mode::New {
            self.prompt.insert(line);
        }
    }

    fn replace(
        &mut self,
        from: (usize, usize),
        to: (usize, usize),
        text: &str,
        kind: Kind,
    ) -> bool {
        if kind == Kind::Other {
            self.undo_break = true;
        }
        if self.deleted.is_some() {
            self.message = "deleted".into();
            return false;
        }
        if from == to && text.is_empty() {
            return false;
        }
        let before = (self.line, self.col);
        let old: Vec<String> = self.buf.lines[from.0..=to.0].to_vec();
        let head = &old[0][..from.1];
        let tail = &old[old.len() - 1][to.1..];
        let mut new: Vec<String> = format!("{head}{text}")
            .split('\n')
            .map(String::from)
            .collect();
        let last = new.len() - 1;
        let col = new[last].len();
        new[last].push_str(tail);
        if let Some(why) = self.locked(old.iter().chain(&new)) {
            self.message = why;
            return false;
        }
        self.buf.lines.splice(from.0..=to.0, new.iter().cloned());
        self.shift_collapsed(from.0, old.len(), new.len());
        self.go((from.0 + last, col));
        let edit = Edit {
            line: from.0,
            old,
            new,
            before,
            after: (self.line, self.col),
            format: None,
        };
        let prev = self.edit_kind;
        let kind = match kind {
            Kind::Typing if text == " " && prev.spaces() == Kind::Spaces => Kind::Spaces,
            Kind::Typing if text == " " => Kind::FirstSpace,
            k => k,
        };
        let stops = match kind {
            Kind::DeletingLeft | Kind::DeletingRight => prev != kind || from.0 != to.0,
            _ if text.contains('\n') => true,
            _ => prev != Kind::FirstSpace && prev.spaces() != kind.spaces(),
        };
        match self.undo.last_mut() {
            Some(last)
                if !stops
                    && !self.undo_break
                    && last.after == before
                    && edit.line <= last.line + last.new.len()
                    && last.line <= edit.line + edit.old.len() =>
            {
                last.merge(edit)
            }
            _ => self.undo.push(edit),
        }
        self.undo_break = kind == Kind::Other;
        self.edit_kind = kind;
        self.redo.clear();
        self.touched(from.0);
        true
    }

    pub(super) fn undo(&mut self, back: bool) {
        let Some(edit) = (if back { &mut self.undo } else { &mut self.redo }).pop() else {
            self.message = if back {
                "nothing to undo"
            } else {
                "nothing to redo"
            }
            .into();
            return;
        };
        let (from, to, at) = if back {
            (&edit.new, &edit.old, edit.before)
        } else {
            (&edit.old, &edit.new, edit.after)
        };
        self.buf
            .lines
            .splice(edit.line..edit.line + from.len(), to.iter().cloned());
        self.shift_collapsed(edit.line, from.len(), to.len());
        self.go(at);
        if let Some((was, is)) = edit.format {
            self.buf.set_format(if back { was } else { is });
        }
        self.touched(edit.line);
        self.undo_break = true;
        self.edit_kind = Kind::Other;
        // A reload to what no save can write back as it came (binary, not UTF-8, mixed line
        // endings) is not redone: it would be an edit left on screen for good.
        if back && edit.format.is_some_and(|(_, is)| is.readonly.is_some()) {
            return;
        }
        (if back { &mut self.redo } else { &mut self.undo }).push(edit);
    }

    /// After every change to `buf.lines` from line `l` on: the highlighting there may be stale,
    /// the disk is behind, and the autosave clock restarts.
    fn touched(&mut self, l: usize) {
        self.buf.edited(l);
        self.dirty = true;
        self.last_edit = Some(Instant::now());
        self.anchor = None;
        self.sync_want_x();
    }

    /// Writes the buffer to its file. A file someone else changed since merl last read or wrote
    /// it is not overwritten: that is a conflict, and with the conflict on screen Ctrl+S (the one
    /// save that still runs) ends it in the buffer's favour.
    pub fn save(&mut self) {
        let Some(path) = &self.buf.path else { return };
        if let Some(why) = self.locked(std::iter::empty()) {
            self.message = why;
            return;
        }
        // ponytail: read, compare, then write; a writer landing between the read and the write
        // still loses. Files have no compare-and-swap, and the window is one read long.
        // Gone (deleted, renamed) is changed too: only Ctrl+S puts the file back. Any other read
        // error, its directory gone among them, is left to the write below to report and retry.
        let changed = match std::fs::read(path) {
            Ok(b) => buffer::hash(&b) != self.buf.disk_hash,
            Err(e) => {
                e.kind() == std::io::ErrorKind::NotFound && path.parent().is_some_and(Path::exists)
            }
        };
        if !self.conflict && changed {
            self.conflict = true;
            return;
        }
        let bytes = self.buf.to_bytes();
        match std::fs::write(path, &bytes) {
            Ok(()) => {
                if let Ok(mut kept) = self.shaped.lock() {
                    kept.remove(path);
                }
                self.buf.disk_hash = buffer::hash(&bytes);
                self.dirty = false;
                self.conflict = false;
                self.last_edit = None;
                self.refresh_diff();
            }
            Err(e) => {
                // Retried on the next autosave; the message stays until then.
                self.message = format!("save failed: {}", super::open::why_not(&e.into()));
                self.last_edit = Some(Instant::now());
            }
        }
    }

    /// Saves unsaved edits now: leaving edit mode, switching files, quitting. Returns `false`
    /// when edits are left that the disk does not have: a conflict, or a failed save.
    pub fn flush(&mut self) -> bool {
        if self.dirty && !self.conflict {
            self.save();
        }
        !self.dirty
    }

    /// Autosave, called by the event loop between events. Returns `true` when it tried, which
    /// changes the status bar whether the file was written, refused or found changed on disk.
    pub fn tick(&mut self) -> bool {
        let parsed = self.parse_tick();
        let due = self.last_edit.is_some_and(|t| t.elapsed() >= self.autosave);
        if !due || !self.dirty || self.conflict {
            return parsed;
        }
        self.save();
        true
    }
}

impl Edit {
    fn merge(&mut self, next: Edit) {
        let (a1, b1) = (self.line, self.line + self.new.len());
        let (a2, b2) = (next.line, next.line + next.old.len());
        let lo = a1.min(a2);
        let between: Vec<String> = (lo..b1.max(b2))
            .map(|i| match (a1..b1).contains(&i) {
                true => self.new[i - a1].clone(),
                false => next.old[i - a2].clone(),
            })
            .collect();
        self.old = [&between[..a1 - lo], &self.old, &between[b1 - lo..]].concat();
        self.new = [&between[..a2 - lo], &next.new, &between[b2 - lo..]].concat();
        self.line = lo;
        self.after = next.after;
    }
}
