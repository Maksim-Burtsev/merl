//! The file tree and the new file prompt.

use super::*;

impl App {
    pub(super) fn tree_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => self.tree.up(),
            KeyCode::Down => self.tree.down(),
            KeyCode::Right => self.tree.expand(),
            KeyCode::Left => self.tree.collapse(),
            KeyCode::Enter => match self.tree.selected() {
                Some(n) if n.is_dir => self.tree.toggle(),
                Some(n) => {
                    let path = self.root.join(&n.path);
                    // The review panel opens a file on its first hunk; the tree where it was.
                    match self.review.as_ref().and_then(|r| r.file(&n.path)).cloned() {
                        Some(f) if self.buf.path.as_deref() != Some(&*path) => {
                            self.open_review_file(&f, false);
                        }
                        _ => self.jump_to(&path, 0),
                    }
                }
                None => {}
            },
            _ => {}
        }
    }

    // ---- new file --------------------------------------------------------

    /// Ctrl+N: asks for a path from the project root. It starts in the directory of the tree
    /// row, or of the open file, as the explorers of VS Code and nvim-tree do.
    pub(super) fn start_new(&mut self) {
        let dir = match (self.focus, self.tree.selected()) {
            (Focus::Tree, Some(n)) if n.is_dir => Some(n.path.clone()),
            (Focus::Tree, Some(n)) => n.path.parent().map(Path::to_path_buf),
            _ => self
                .rel_current()
                .and_then(|p| Some(p.parent()?.to_path_buf())),
        }
        // A file opened from outside the project has no directory in it.
        .filter(|d| d.is_relative() && !d.as_os_str().is_empty());
        self.mode = Mode::New;
        self.prompt =
            LineEdit::typed(&dir.map(|d| format!("{}/", d.display())).unwrap_or_default());
    }

    pub(super) fn new_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                let typed = std::mem::take(&mut self.prompt).to_string();
                self.close_overlay();
                if key.code == KeyCode::Enter {
                    self.create(&typed);
                }
            }
            _ => {
                self.prompt.key(key);
            }
        }
    }

    /// Creates `typed`, a path from the project root, with the directories it needs, and opens
    /// it for editing. A file that is there already is opened as it is, never overwritten.
    fn create(&mut self, typed: &str) {
        // An empty prompt is a cancel, as in `:`.
        if typed.is_empty() {
            return;
        }
        let mut rel = PathBuf::new();
        for part in Path::new(typed).components() {
            match part {
                Component::Normal(name) => rel.push(name),
                Component::CurDir => {}
                Component::ParentDir if rel.pop() => {}
                _ => {
                    self.message = "outside the project".into();
                    return;
                }
            }
        }
        let path = self.root.join(&rel);
        if typed.ends_with('/') || rel.as_os_str().is_empty() {
            self.message = "no file name".into();
            return;
        }
        if path.is_dir() {
            self.message = format!("{} is a directory", rel.display());
            return;
        }
        // Before anything is created: edits that cannot be saved keep their file open.
        if !self.flush() {
            return;
        }
        let made = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::File::create_new(&path).map(drop));
        match made {
            Ok(()) => {
                // ponytail: a whole walk on the key press, as at startup, rather than one path
                // put into the tree and the list; a thread like the watcher's if it ever stalls.
                let (tree, files) = crate::tree::build(&self.root);
                self.project_walked(tree, files);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => {
                self.message = format!("{}: {e}", rel.display());
                return;
            }
        }
        self.jump_to(&path, 0);
        if self.buf.path.as_deref() == Some(&*path) {
            self.start_edit();
        }
    }
}
