//! Editing the buffer: typing, pasting, undo and saving.

use super::*;

impl App {
    /// Enter in the code pane: the cursor becomes a text cursor.
    pub(super) fn start_edit(&mut self) {
        if self.buf.path.is_none() {
            return;
        }
        if let Some(why) = self.locked(std::iter::once(&self.buf.lines[self.line])) {
            self.message = why;
            return;
        }
        self.mode = Mode::Edit;
        self.undo_break = true;
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

    /// Ctrl+C: the selection goes to the clipboard, without one the whole line, as in VS Code.
    /// Returns the copied range, which Ctrl+X then removes.
    pub(super) fn copy(&mut self) -> ((usize, usize), (usize, usize)) {
        let (from, to, text) = match self.selection() {
            Some((from, to)) => (from, to, self.selected_text().unwrap()),
            None if self.line + 1 < self.buf.lines.len() => (
                (self.line, 0),
                (self.line + 1, 0),
                format!("{}\n", self.line_str()),
            ),
            None => (
                (self.line, 0),
                (self.line, self.line_str().len()),
                self.line_str().to_string(),
            ),
        };
        // A piece of one line is just `copied`; anything that holds a whole line is counted.
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
        match code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.flush();
            }
            KeyCode::Char('c' | 'x') if ctrl => {
                let (from, to) = self.copy();
                if code == KeyCode::Char('x') {
                    self.replace(from, to, "");
                }
            }
            KeyCode::Char(c) if !ctrl => self.insert(&c.to_string()),
            KeyCode::Enter => {
                let head = &self.line_str()[..self.col];
                let indent = &head[..head.len() - head.trim_start().len()];
                self.insert(&format!("\n{indent}"));
            }
            KeyCode::Tab => self.insert(if self.buf.tabs { "\t" } else { buffer::TAB }),
            KeyCode::Backspace | KeyCode::Delete if self.selection().is_some() => self.insert(""),
            // Option+Backspace / Option+Delete: up to where Alt+Left / Right would land.
            KeyCode::Backspace | KeyCode::Delete if alt => {
                let at = (self.line, self.col);
                if code == KeyCode::Backspace {
                    self.word_left();
                } else {
                    self.word_right();
                }
                let to = (self.line, self.col);
                // Undo puts the cursor back where the key was pressed, and takes the word alone:
                // not the typing before it, not the typing after.
                (self.line, self.col) = at;
                self.undo_break = true;
                self.replace(at.min(to), at.max(to), "");
                self.undo_break = true;
            }
            KeyCode::Backspace => {
                let from = if self.col > 0 {
                    (self.line, prev_char(self.line_str(), self.col))
                } else if self.line > 0 {
                    (self.line - 1, self.buf.lines[self.line - 1].len())
                } else {
                    return true;
                };
                self.replace(from, (self.line, self.col), "");
            }
            KeyCode::Delete => {
                let to = if self.col < self.line_str().len() {
                    (self.line, next_char(self.line_str(), self.col))
                } else if self.line + 1 < self.buf.lines.len() {
                    (self.line + 1, 0)
                } else {
                    return true;
                };
                self.replace((self.line, self.col), to, "");
            }
            _ => return false,
        }
        true
    }

    /// Inserts `text` at the cursor, or in place of the selection; a `\n` in it splits the line.
    fn insert(&mut self, text: &str) {
        let (from, to) = self
            .selection()
            .unwrap_or(((self.line, self.col), (self.line, self.col)));
        self.replace(from, to, text);
    }

    /// Text the terminal pasted (Cmd+V): inserted while editing, typed into a prompt or a picker
    /// query (its first line), ignored in navigation, where every letter is a command.
    pub fn paste(&mut self, text: &str) {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        if self.mode == Mode::Edit {
            self.insert(&text);
        } else if self.picker.is_some() || matches!(self.mode, Mode::Goto | Mode::Find | Mode::New)
        {
            for c in text.lines().next().unwrap_or_default().chars() {
                self.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            }
        }
    }

    /// The one way the text changes: what lies between `from` and `to` (ordered (line, col))
    /// becomes `text`, and the cursor lands after it. Recorded for undo; typing that carries
    /// on where the previous step ended extends that step, as VS Code groups keystrokes. Refused,
    /// with the reason in the status bar, where `locked` says the text cannot change.
    fn replace(&mut self, from: (usize, usize), to: (usize, usize), text: &str) {
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
            return;
        }
        self.buf.lines.splice(from.0..=to.0, new.iter().cloned());
        (self.line, self.col) = (from.0 + last, col);
        let edit = Edit {
            line: from.0,
            old,
            new,
            before,
            after: (self.line, self.col),
            format: None,
        };
        let continues = !self.undo_break
            && self.undo.last().is_some_and(|e| {
                e.after == before && e.new.len() == 1 && edit.old.len() == 1 && edit.new.len() == 1
            });
        if continues {
            let last = self.undo.last_mut().unwrap();
            last.new = edit.new;
            last.after = edit.after;
        } else {
            self.undo.push(edit);
        }
        self.undo_break = false;
        self.redo.clear();
        self.touched(from.0);
    }

    /// Ctrl+Z / Ctrl+Y: swaps one step between the two stacks and applies it.
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
        (self.line, self.col) = at;
        if let Some((was, is)) = edit.format {
            self.buf.set_format(if back { was } else { is });
        }
        self.touched(edit.line);
        self.undo_break = true;
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
            Ok(b) => buffer::hash(&b) != self.buf.disk,
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
                self.buf.disk = buffer::hash(&bytes);
                self.dirty = false;
                self.conflict = false;
                self.last_edit = None;
                self.refresh_diff();
            }
            Err(e) => {
                // Retried on the next autosave; the message stays until then.
                self.message = format!("save failed: {e}");
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
        let due = self.last_edit.is_some_and(|t| t.elapsed() >= self.autosave);
        if !due || !self.dirty || self.conflict {
            return false;
        }
        self.save();
        true
    }
}
