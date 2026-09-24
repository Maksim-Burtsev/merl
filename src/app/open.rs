//! Opening a file, reloading it, and the jump history `[` and `]` walk.

use super::*;

impl App {
    /// Puts the tree cursor on `path` (absolute) and expands everything above it.
    pub(super) fn reveal(&mut self, path: &Path) {
        if let Ok(rel) = path.strip_prefix(&self.root) {
            self.tree.reveal(rel);
        }
    }

    /// Shows `path` at `line` (1-based). Reloading is skipped when the file is already open, so
    /// this doubles as a plain cursor move. Line 0 is "no line in particular": the start of a
    /// file being opened, and the cursor where it is on the file that is already open, like
    /// VS Code's explorer.
    /// Returns `false`, with the reason in the status bar, when the file cannot be read or the
    /// open one has edits that could not be saved.
    fn open(&mut self, path: &Path, line: usize) -> bool {
        let same = self.buf.path.as_deref() == Some(path);
        if !same {
            // Replacing the buffer would drop edits the disk does not have; the status bar
            // already says why they are not there (a conflict, a failed save).
            if !self.flush() {
                return false;
            }
            match self.load(path) {
                Ok(mut buf) => {
                    self.lock_unwritable(&mut buf);
                    let old = std::mem::replace(&mut self.buf, buf);
                    let (undo, redo) = (
                        std::mem::take(&mut self.undo),
                        std::mem::take(&mut self.redo),
                    );
                    // Flushed, so what is stashed is the disk as it was left. A read-only
                    // buffer's text is not the file's: as in `reload`, nothing to go back to.
                    if let Some(p) = old.path.clone()
                        && old.readonly.is_none()
                        && !(undo.is_empty() && redo.is_empty())
                    {
                        let format = old.format();
                        self.stash.insert(p, (old.lines, format, undo, redo));
                    }
                    if let Some((lines, format, undo, redo)) = self.stash.remove(path) {
                        self.undo = undo;
                        // Written meanwhile: one more step, as if it had been on screen.
                        match reload_step(&lines, format, &self.buf) {
                            Some(step) => self.undo.push(step),
                            None => self.redo = redo,
                        }
                    }
                    self.undo_break = true;
                    self.anchor = None;
                    self.dirty = false;
                    self.conflict = false;
                    if self.mode == Mode::Edit {
                        self.mode = Mode::Normal;
                    }
                    (self.line, self.col, self.want_x) = (0, 0, 0);
                    (self.top_line, self.top_row) = (0, 0);
                    // After the viewport is reset: the diff clamps it against the new file.
                    self.diff = git::Diff::default();
                    self.refresh_diff();
                }
                Err(e) => {
                    self.message = format!("{e:#}");
                    return false;
                }
            }
        }
        if !(same && line == 0) {
            self.goto_line(line.max(1));
        }
        self.reveal(path);
        true
    }

    /// Where the file sits, rather than what it holds, says it cannot be edited: on open and on
    /// every reload.
    ///
    /// The standard library and dependencies are read here, never edited. A `.venv` or
    /// `node_modules` sits inside the root, so the roots decide, but not over a file the project
    /// walk listed: an editable install puts the project's own `src` on `sys.path`.
    ///
    /// Then the file merl has no permission to write (#123). The test is an open for writing,
    /// not the permission bits: the bits lie in both directions — root writes a 444 file, an ACL
    /// or a read-only mount refuses a 644 one — and a save is `fs::write` on this very path, so
    /// the open it would do is the honest question. Nothing is truncated, so the file is left as
    /// it was. It comes last because every other reason says more.
    fn lock_unwritable(&self, buf: &mut Buffer) {
        let Some(path) = buf.path.as_deref() else {
            return;
        };
        let listed = path
            .strip_prefix(&self.root)
            .is_ok_and(|rel| self.files.iter().any(|f| f == rel));
        let external = !path.starts_with(&self.root)
            || !listed
                && self
                    .external
                    .values()
                    .any(|(roots, _)| roots.iter().any(|r| path.starts_with(r)))
                // Another package's `node_modules`, walked from a file opened before.
                || !listed && self.node_modules.keys().any(|r| path.starts_with(r));
        if external {
            buf.readonly.get_or_insert("outside the project");
        }
        lock_no_write(buf);
    }

    /// The file from disk; in review mode a file the branch deleted comes from the base,
    /// read-only.
    fn load(&self, path: &Path) -> anyhow::Result<Buffer> {
        if let Some(r) = &self.review
            && let Ok(rel) = path.strip_prefix(&self.root)
            && r.file(rel).is_some_and(|f| f.status == 'D')
        {
            let mut buf = Buffer::from_bytes(path.to_path_buf(), &r.base_bytes(&self.root, rel)?);
            buf.readonly = Some("deleted in this branch");
            return Ok(buf);
        }
        Buffer::load(path)
    }

    /// Asks `main` for new marks; the old ones stay on screen until they arrive. In review
    /// mode they are taken here and now: `c` needs the hunks of a file the moment it opens, and
    /// one file against one commit is quick.
    pub(super) fn refresh_diff(&mut self) {
        let Some(r) = &self.review else {
            self.want_diff = true;
            return;
        };
        let Some(path) = self.buf.path.clone() else {
            return;
        };
        self.want_diff = false;
        let rel = path.strip_prefix(&self.root).ok();
        let file = rel.and_then(|rel| r.file(rel));
        self.diff = match file {
            Some(f) if f.status == 'D' => git::Diff {
                // A deleted file is all ghost: every line marked, nothing to step through.
                marks: (0..self.buf.lines.len())
                    .map(|l| (l, git::Mark::DeletedBelow))
                    .collect(),
                ..Default::default()
            },
            _ => r.diff(&self.root, &path, file),
        };
        // Ghosts change how many rows a line has; the viewport must not point past them.
        self.clamp_top();
    }

    /// The project changed on disk and was walked again: the file list is the new one for the
    /// next `o`, `s`, `u` and `d` (an open picker keeps its rows), and the tree takes the new rows
    /// around its cursor. The review panel lists the branch, not the walk, and is left alone.
    pub fn project_walked(&mut self, tree: Tree, files: Vec<PathBuf>) {
        self.files = files;
        self.ignored = tree.ignored_files();
        if self.review.is_none() {
            self.refresh_tree(tree);
        }
    }

    pub(super) fn refresh_tree(&mut self, tree: Tree) {
        let row = |t: &Tree| t.visible().iter().position(|&i| i == t.cursor).unwrap_or(0);
        let before = row(&self.tree);
        self.tree.refresh(tree);
        // A scrolled panel follows its cursor, so rows arriving above the pane move nothing on
        // screen; one showing its first row keeps showing it.
        if self.tree_top > 0 {
            self.tree_top = (self.tree_top + row(&self.tree)).saturating_sub(before);
        }
    }

    /// Re-reads the open file after it changed on disk. Cursor, scroll, history and find pattern
    /// survive; the cursor is clamped to whatever the file is now. The reload is one undo step,
    /// as in VS Code and Vim (#122): Ctrl+Z takes back what was written, then the edits before
    /// it. merl's own saves are recognised and ignored; a change under unsaved edits is a
    /// conflict, not a reload, unless `force` (Ctrl+R) says the edits go; Ctrl+Z brings them
    /// back. Returns whether anything on screen changed.
    pub fn reload(&mut self, force: bool) -> bool {
        let Some(path) = self.buf.path.clone() else {
            return false;
        };
        // Mid-save the file can be briefly gone; the rename that follows sends another event
        // and overwrites the message. Gone for good, the message stays.
        let Ok(bytes) = std::fs::read(&path) else {
            // Ctrl+R has no disk version to take, but the edits still go: merl is not held on
            // a file that is gone.
            if force {
                (self.dirty, self.conflict) = (false, false);
            }
            self.message = format!("{} gone", self.rel_path());
            return true;
        };
        if !force {
            if buffer::hash(&bytes) == self.buf.disk {
                // Gone and back as merl last saw it (a writer's delete-then-write): no conflict.
                return std::mem::take(&mut self.conflict);
            }
            if self.dirty {
                self.conflict = true;
                return true;
            }
        }
        let mut buf = Buffer::from_bytes(path.clone(), &bytes);
        self.lock_unwritable(&mut buf);
        let old = std::mem::replace(&mut self.buf, buf);
        if self.review.is_some() {
            // The reader stays in the hunk they are in when an agent writes above it: every
            // line kept of this file goes down with its text, before anything is clamped.
            let to = |l: &mut usize| *l = carried(&old.lines, &self.buf.lines, *l);
            let stop = self.history.get_mut(self.hist_idx);
            let stop = stop.filter(|s| s.0 == path && s.1 == self.line);
            [&mut self.line, &mut self.top_line, &mut self.find_anchor.0]
                .into_iter()
                .chain(self.anchor.as_mut().map(|a| &mut a.0))
                .chain(self.find_sel.as_mut().map(|a| &mut a.0))
                .chain(stop.map(|s| &mut s.1))
                .for_each(to);
        }
        self.dirty = false;
        self.conflict = false;
        self.last_edit = None;
        self.redo.clear();
        if old.readonly.is_some() {
            // The text on screen could not be edited, or was not the file's (a binary
            // placeholder, bytes read lossily): there is nothing to go back to.
            self.undo.clear();
        } else {
            self.undo
                .extend(reload_step(&old.lines, old.format(), &self.buf));
        }
        self.undo_break = true;
        self.refresh_diff();
        (self.line, self.col) = self.clamp_pos((self.line, self.col));
        self.clamp_top();
        self.sync_want_x();
        self.clamp_scroll();
        self.message = "reloaded".into();
        true
    }

    pub(super) fn pos(&self) -> Option<(PathBuf, usize, usize)> {
        self.buf.path.clone().map(|p| (p, self.line, self.col))
    }

    /// Records where the cursor is now, VS Code style: the current stop always tracks the
    /// cursor. A plain move within `HIST_NEAR` lines just updates it; a farther move, another
    /// file, or a `jump` (go to definition, `:`, find) becomes a new stop and drops the
    /// forward history. Paging (see `key_inner`) only updates it. Standing on the current stop
    /// records nothing, so walking with `[` / `]` is silent.
    pub(crate) fn hist_note(&mut self, jump: bool) {
        let Some(pos) = self.pos() else {
            return;
        };
        if let Some(cur) = self.history.get(self.hist_idx) {
            if *cur == pos {
                return;
            }
            if !jump && cur.0 == pos.0 && cur.1.abs_diff(pos.1) < HIST_NEAR {
                self.history[self.hist_idx] = pos;
                return;
            }
            self.history.truncate(self.hist_idx + 1);
        }
        self.history.push(pos);
        if self.history.len() > HIST_MAX {
            self.history.remove(0);
        }
        self.hist_idx = self.history.len() - 1;
    }

    /// Opens `path` at `line` and makes it a stop in the jump history. `:` and the pickers jump
    /// from outside `key_inner`'s move rule, so a jump that lands elsewhere drops the selection
    /// here.
    pub fn jump_to(&mut self, path: &Path, line: usize) {
        let before = (self.line, self.col);
        if self.open(path, line) {
            self.focus = Focus::Code;
        }
        self.drop_selection_if_moved(before);
        self.hist_note(true);
    }

    /// `[` and `]`: walks the recorded stops.
    pub(super) fn hist_go(&mut self, delta: isize) {
        // A stop whose file is gone (an agent renamed it) is dropped, and the walk goes on.
        let mut gone = Vec::new();
        let landed = loop {
            let next = self
                .hist_idx
                .checked_add_signed(delta)
                .filter(|i| *i < self.history.len());
            let Some(i) = next else {
                self.message = if delta < 0 {
                    "start of history".into()
                } else {
                    "end of history".into()
                };
                break None;
            };
            let (path, line, col) = self.history[i].clone();
            // With the stops between them dropped, this one can be where the cursor already
            // is: not a step, it goes too.
            let twin = self.history[i] == self.history[self.hist_idx];
            if !twin && self.open(&path, line + 1) {
                break Some((i, path, col));
            }
            // Edits that could not be saved, or a file that is there and does not open: the
            // stop stays, and `open` has said why.
            if !twin && (self.dirty || path.exists()) {
                return;
            }
            self.history.remove(i);
            self.hist_idx -= usize::from(i < self.hist_idx);
            if !twin && !gone.contains(&path) {
                gone.push(path);
            }
        };
        match &gone[..] {
            [] => {}
            [one] => {
                let rel = one.strip_prefix(&self.root).unwrap_or(one);
                self.message = format!("{} gone", rel.display());
            }
            _ => self.message = format!("{} files gone", gone.len()),
        }
        let Some((i, path, col)) = landed else { return };
        self.hist_idx = i;
        self.focus = Focus::Code;
        (self.line, self.col) = self.clamp_pos((self.line, col));
        self.sync_want_x();
        // The stop follows the file: after a reload shortened it, this is where `[` lands,
        // and `hist_note` must not read the clamp as a move that drops the forward history.
        self.history[i] = (path, self.line, self.col);
    }
}

/// Where line `l` of `old` is in `new` when the text was changed above it: the lines the two
/// end with alike move by the difference in length, the ones they start with alike stay, and
/// a line of the rewritten middle stays too, but not below the middle's new end.
///
/// ponytail: one rewritten region per reload, which is what an agent's edit is. After a write
/// that changed the file in several places the lines between them are off by what changed
/// above them; a line diff would place those too.
pub(super) fn carried(old: &[String], new: &[String], l: usize) -> usize {
    let (_, tail) = common_ends(old, new);
    if l >= old.len() - tail {
        l + new.len() - old.len()
    } else {
        l.min(new.len() - tail)
    }
}

/// How many lines `old` and `new` start with alike, and then end with alike: what lies between
/// is what changed, found as VS Code's `ModelService._computeEdits` finds it.
fn common_ends(old: &[String], new: &[String]) -> (usize, usize) {
    let same = |(a, b): &(&String, &String)| a == b;
    let head = old.iter().zip(new).take_while(same).count();
    let ends = old.iter().rev().zip(new.iter().rev());
    let tail = ends
        .take(old.len().min(new.len()) - head)
        .take_while(same)
        .count();
    (head, tail)
}

/// A reload as one undo step, as VS Code's `ModelService.updateModel` makes it: the lines that
/// changed from `old` to what `buf` holds now, and the format on either side. `None` when neither
/// changed.
fn reload_step(old: &[String], was: buffer::Format, buf: &Buffer) -> Option<Edit> {
    let (new, is) = (&buf.lines, buf.format());
    let (head, tail) = common_ends(old, new);
    if head == old.len() && head == new.len() && was == is {
        return None;
    }
    Some(Edit {
        line: head,
        old: old[head..old.len() - tail].to_vec(),
        new: new[head..new.len() - tail].to_vec(),
        // Undo and redo land where the file changed, so what they take back is in sight.
        before: (head.min(old.len() - 1), 0),
        after: (head.min(new.len() - 1), 0),
        format: Some((was, is)),
    })
}
