use super::*;

impl App {
    pub(super) fn reveal(&mut self, path: &Path) {
        if let Ok(rel) = path.strip_prefix(&self.root) {
            self.tree.reveal(rel);
        }
    }

    fn open(&mut self, path: &Path, line: usize) -> bool {
        let same = self.buf.path.as_deref() == Some(path);
        if !same {
            if !self.flush() {
                return false;
            }
            match self.load(path) {
                Ok(mut buf) => {
                    self.lock_unwritable(&mut buf);
                    let old = std::mem::replace(&mut self.buf, buf);
                    self.swap_collapsed(&old, path);
                    let (undo, redo) = (
                        std::mem::take(&mut self.undo),
                        std::mem::take(&mut self.redo),
                    );
                    if let Some(p) = old.path.clone()
                        && old.readonly.is_none()
                        && !(undo.is_empty() && redo.is_empty())
                    {
                        let format = old.format();
                        self.stash.insert(p, (old.lines, format, undo, redo));
                    }
                    if let Some((lines, format, undo, redo)) = self.stash.remove(path) {
                        self.undo = undo;
                        match reload_step(&lines, format, &self.buf) {
                            Some(step) => self.undo.push(step),
                            None => self.redo = redo,
                        }
                    }
                    self.undo_break = true;
                    self.edit_kind = edit::Kind::Other;
                    self.anchor = None;
                    self.preview = None;
                    let rel = path.strip_prefix(&self.root).ok();
                    if rel.is_some_and(|rel| {
                        self.review.as_ref().is_some_and(|r| r.file(rel).is_some())
                    }) {
                        self.previewed.remove(path);
                    }
                    self.dirty = false;
                    self.conflict = false;
                    if self.mode == Mode::Edit {
                        self.mode = Mode::Normal;
                    }
                    self.go((0, 0));
                    self.want_x = 0;
                    (self.top_line, self.top_row) = (0, 0);
                    // After the viewport is reset: the diff clamps it against the new file.
                    self.diff = git::Diff::default();
                    self.refresh_diff();
                }
                Err(e) => {
                    self.say_about(path, &format!(": {}", why_not(&e)));
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
            || !listed && self.walked_roots.keys().any(|r| path.starts_with(r))
            || self.in_project(path).is_none();
        if external {
            buf.readonly.get_or_insert("outside the project");
        }
        lock_no_write(buf);
    }

    pub(super) fn in_project(&self, path: &Path) -> Option<PathBuf> {
        let root = self.root.canonicalize().ok()?;
        if path
            .canonicalize()
            .is_ok_and(|real| !real.starts_with(&root))
        {
            return None;
        }
        let dir = path.parent()?;
        let (real, rest) = (dir.ancestors())
            .find_map(|a| Some((a.canonicalize().ok()?, dir.strip_prefix(a).ok()?)))?;
        let resolved = real.join(rest).join(path.file_name()?);
        let rel = resolved.strip_prefix(root).ok()?;
        Some(rel.to_path_buf())
    }

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
                marks: (0..self.buf.lines.len())
                    .map(|l| (l, git::Mark::Deleted))
                    .collect(),
                ..Default::default()
            },
            _ => r.diff(&self.root, &path, file),
        };
        // The ghosts take their syntax colours from the file at the base. This runs after every
        // autosave, and the base does not change under an edit: the one loaded is kept.
        self.base = match rel {
            Some(rel) if !self.diff.ghosts.is_empty() => {
                let from = file.and_then(|f| f.old.as_deref()).unwrap_or(rel);
                let key = format!("{}:{}", r.merge_base, from.display());
                match self.base.take() {
                    Some((k, b)) if k == key => Some((k, b)),
                    _ => r
                        .base_bytes(&self.root, from)
                        .ok()
                        .map(|bytes| (key, Buffer::from_bytes(self.root.join(from), &bytes))),
                }
            }
            _ => None,
        };
        self.clamp_top();
        self.clamp_cursor();
    }

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
        if self.tree_top > 0 {
            self.tree_top = (self.tree_top + row(&self.tree)).saturating_sub(before);
        }
    }

    pub fn reload(&mut self, force: bool) -> bool {
        let Some(path) = self.buf.path.clone() else {
            return false;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            if force {
                (self.dirty, self.conflict) = (false, false);
            }
            self.message = format!("{} gone", self.rel_path());
            return true;
        };
        if !force {
            if buffer::hash(&bytes) == self.buf.disk_hash {
                return std::mem::take(&mut self.conflict);
            }
            if self.dirty {
                self.conflict = true;
                return true;
            }
        }
        let mut buf = Buffer::from_bytes(path.clone(), &bytes);
        self.lock_unwritable(&mut buf);
        let reading = self.deleted.map(|(_, i)| (self.line_str().to_string(), i));
        let old = std::mem::replace(&mut self.buf, buf);
        self.carry_collapsed(&old.lines);
        if self.review.is_some() {
            let to = |l: &mut usize| *l = carried(&old.lines, &self.buf.lines, *l);
            let here = self.at();
            let stop = self.history.get_mut(self.hist_idx);
            let stop = stop.filter(|s| s.0 == path && s.1 == here);
            let deleted = self.deleted.as_mut().map(|(k, _)| k);
            [&mut self.line, &mut self.top_line]
                .into_iter()
                .chain(deleted)
                .for_each(to);
            let carry = |t: &mut TextLine| match t {
                TextLine::File(l) | TextLine::Deleted(l, _) => to(l),
            };
            [&mut self.find_anchor.0]
                .into_iter()
                .chain(self.anchor.as_mut().map(|a| &mut a.0))
                .chain(self.find_sel.as_mut().map(|a| &mut a.0))
                .chain(stop.map(|s| &mut s.1))
                .for_each(carry);
        }
        self.dirty = false;
        self.conflict = false;
        self.last_edit = None;
        self.redo.clear();
        if old.readonly.is_some() {
            self.undo.clear();
        } else {
            self.undo
                .extend(reload_step(&old.lines, old.format(), &self.buf));
        }
        self.undo_break = true;
        self.edit_kind = edit::Kind::Other;
        self.refresh_diff();
        if let Some(reading) = reading {
            self.find_deleted_again(reading);
        }
        self.clamp_cursor();
        self.sync_want_x();
        self.clamp_scroll();
        self.message = "reloaded".into();
        true
    }

    /// The cursor back on the deleted line `text`, the `i`th at its key before a reload: git keys
    /// a deletion by where the lines around it went, which the carry of the file's lines cannot
    /// know (lines appended under those deleted at the end key them above the first new one).
    /// The copy nearest the carried key, then the index, wins; none, and `clamp_cursor` decides.
    fn find_deleted_again(&mut self, (text, i): (String, usize)) {
        let near = self.at().key();
        let found = (self.diff.ghosts.iter())
            .flat_map(|(&k, g)| g.iter().enumerate().map(move |(j, t)| (k, j, t)))
            .filter(|&(_, _, t)| *t == text)
            .min_by_key(|&(k, j, _)| (k.abs_diff(near), j.abs_diff(i)));
        if let Some((k, j, _)) = found {
            self.set_at(TextLine::Deleted(k, j));
        }
    }

    fn near(&self, a: TextLine, b: TextLine) -> bool {
        let (lo, hi) = (a.min(b), a.max(b));
        std::iter::successors(Some(lo), |&t| self.next_line(t))
            .take(HIST_NEAR)
            .any(|t| t == hi)
    }

    pub(super) fn pos(&self) -> Option<(PathBuf, TextLine, usize)> {
        self.buf.path.clone().map(|p| (p, self.at(), self.col))
    }

    pub(crate) fn hist_note(&mut self, jump: bool) {
        let Some(pos) = self.pos() else {
            return;
        };
        if let Some(cur) = self.history.get(self.hist_idx) {
            if *cur == pos {
                return;
            }
            if !jump && cur.0 == pos.0 && self.near(cur.1, pos.1) {
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

    pub(super) fn note_hist_row(&mut self) {
        let Some(pos) = self
            .pos()
            .filter(|p| self.history.get(self.hist_idx) == Some(p))
        else {
            return;
        };
        let (top, cur) = ((self.top_line, self.top_row), self.cursor_at());
        let row = self.rows_between(top, cur);
        if top <= cur && row < self.view_h {
            self.hist_rows.insert(pos, row);
        }
        let history = &self.history;
        self.hist_rows.retain(|stop, _| history.contains(stop));
    }

    pub fn jump_to(&mut self, path: &Path, line: usize) {
        self.jump_to_col(path, line, 0);
    }

    pub(super) fn jump_to_col(&mut self, path: &Path, line: usize, col: usize) {
        let before = (self.at(), self.col);
        self.preview_jumped();
        if self.open(path, line) {
            self.focus = Focus::Code;
            if col > 0 {
                self.go(self.clamp_pos((self.line, col)));
                self.sync_want_x();
            }
        }
        self.drop_selection_if_moved(before);
        self.hist_note(true);
    }

    pub(super) fn jump_to_item(&mut self, item: &PickItem) {
        let path = self.root.join(&item.path);
        match item.deleted {
            true => self.jump_to_deleted(&path, item.line, item.col),
            false => self.jump_to_col(&path, item.line, item.col),
        }
    }

    pub(super) fn jump_to_deleted(&mut self, path: &Path, n: usize, col: usize) {
        let before = (self.at(), self.col);
        self.preview_jumped();
        if self.open(path, 0) {
            self.focus = Focus::Code;
            let nearest = (self.diff.ghost_from.iter())
                .flat_map(|(&k, &from)| {
                    (0..self.deleted_at(k)).map(move |i| (TextLine::Deleted(k, i), from + i + 1))
                })
                .min_by_key(|&(_, line)| line.abs_diff(n));
            let t = nearest.map_or(TextLine::File(n.saturating_sub(1)), |(t, _)| t);
            let (t, col) = self.clamp_place((t, col));
            self.set_at(t);
            self.col = col;
            self.sync_want_x();
            self.center = true;
        }
        self.drop_selection_if_moved(before);
        self.hist_note(true);
    }

    pub(super) fn hist_go(&mut self, delta: isize) {
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
            let twin = self.history[i] == self.history[self.hist_idx];
            if !twin && self.open(&path, line.key() + 1) {
                break Some((i, path, line, col));
            }
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
            [one] => self.say_about(one, " gone"),
            _ => self.message = format!("{} files gone", gone.len()),
        }
        let Some((i, path, line, col)) = landed else {
            return;
        };
        self.hist_idx = i;
        self.focus = Focus::Code;
        let (line, col) = self.clamp_place((line, col));
        self.set_at(line);
        self.col = col;
        self.sync_want_x();
        if let Some(&row) = self.hist_rows.get(&self.history[i]) {
            self.center = false;
            (self.top_line, self.top_row) = self.back_rows(self.cursor_at(), row);
        }
        self.history[i] = (path, self.at(), self.col);
    }
}

/// ponytail: one rewritten region per reload, which is what an agent's edit is. After a write
/// that changed the file in several places the lines between them are off by what changed
/// above them; a line diff would place those too.
pub(super) fn carried(old: &[String], new: &[String], l: usize) -> usize {
    let (_, tail) = lines_alike_at_start_and_end(old, new);
    if l >= old.len() - tail {
        l + new.len() - old.len()
    } else {
        l.min(new.len() - tail)
    }
}

fn lines_alike_at_start_and_end(old: &[String], new: &[String]) -> (usize, usize) {
    let same = |(a, b): &(&String, &String)| a == b;
    let head = old.iter().zip(new).take_while(same).count();
    let ends = old.iter().rev().zip(new.iter().rev());
    let tail = ends
        .take(old.len().min(new.len()) - head)
        .take_while(same)
        .count();
    (head, tail)
}

fn reload_step(old: &[String], was: buffer::Format, buf: &Buffer) -> Option<Edit> {
    let (new, is) = (&buf.lines, buf.format());
    let (head, tail) = lines_alike_at_start_and_end(old, new);
    if head == old.len() && head == new.len() && was == is {
        return None;
    }
    Some(Edit {
        line: head,
        old: old[head..old.len() - tail].to_vec(),
        new: new[head..new.len() - tail].to_vec(),
        before: (head.min(old.len() - 1), 0),
        after: (head.min(new.len() - 1), 0),
        format: Some((was, is)),
    })
}

pub(crate) fn error_text(e: &anyhow::Error) -> String {
    let mut text = Vec::new();
    for cause in e.chain() {
        if cause.is::<std::io::Error>() {
            text.push(why_not(e));
            break;
        }
        text.push(cause.to_string());
    }
    text.join(": ")
}

pub(super) fn why_not(e: &anyhow::Error) -> String {
    use std::io::ErrorKind::*;
    match e.downcast_ref::<std::io::Error>().map(std::io::Error::kind) {
        Some(PermissionDenied) => "permission denied".into(),
        Some(NotFound) => "no such file".into(),
        Some(IsADirectory) => "is a directory".into(),
        _ => {
            let mut why = e.root_cause().to_string();
            if let Some(i) = why.find(" (os error ") {
                why.truncate(i);
            }
            why
        }
    }
}
