use super::{
    App, Component, Focus, KeyCode, KeyEvent, KeyModifiers, LineEdit, Mode, Path, PathBuf, open,
};

pub(super) fn moves_the_tree(key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Up
        | KeyCode::Down
        | KeyCode::Left
        | KeyCode::Right
        | KeyCode::Enter
        | KeyCode::PageUp
        | KeyCode::PageDown
        | KeyCode::Home
        | KeyCode::End
        | KeyCode::Char('{' | '}') => true,
        KeyCode::Char('d' | 'u') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}

impl App {
    pub(super) fn tree_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let page = isize::try_from(self.tree_h.max(1)).unwrap_or(isize::MAX);
        let half = (page / 2).max(1);
        match key.code {
            KeyCode::Up => self.tree.up(),
            KeyCode::Down => self.tree.down(),
            KeyCode::PageUp => self.tree_scroll(-page),
            KeyCode::PageDown => self.tree_scroll(page),
            KeyCode::Char('u') if ctrl => self.tree_scroll(-half),
            KeyCode::Char('d') if ctrl => self.tree_scroll(half),
            KeyCode::Home => self.tree_scroll(isize::MIN),
            KeyCode::End => self.tree_scroll(isize::MAX),
            KeyCode::Char('{') => self.tree_folder(false),
            KeyCode::Char('}') => self.tree_folder(true),
            KeyCode::Right => self.tree.expand(),
            KeyCode::Left => self.tree.collapse(),
            KeyCode::Enter => match self.tree.selected() {
                Some(n) if n.is_dir => self.tree.toggle(),
                Some(n) if self.root.join(&n.path).is_dir() => {}
                Some(n) => {
                    let path = self.root.join(&n.path);
                    let path = self
                        .in_project(&path)
                        .map_or(path, |rel| self.root.join(rel));
                    let f = self.review.as_ref().and_then(|r| r.file(&n.path)).cloned();
                    match &f {
                        Some(f) if self.buf.path.as_deref() != Some(&*path) => {
                            self.open_review_file(f, false);
                        }
                        _ => self.jump_to(&path, 0),
                    }
                    if f.is_some() && self.buf.path.as_deref() == Some(&*path) {
                        self.remember_hunk();
                    }
                }
                None => {}
            },
            _ => {}
        }
    }

    fn tree_scroll(&mut self, rows: isize) {
        let len = self.tree.visible().len();
        self.tree.move_by(rows);
        self.tree_top = self
            .tree_top
            .saturating_add_signed(rows)
            .min(len.saturating_sub(self.tree_h));
    }

    fn tree_folder(&mut self, down: bool) {
        let vis = self.tree.visible();
        let Some(at) = vis.iter().position(|&i| i == self.tree.cursor) else {
            return;
        };
        let folder = |j: &usize| self.tree.nodes[vis[*j]].is_dir;
        let to = if down {
            (at + 1..vis.len()).find(folder).unwrap_or(vis.len() - 1)
        } else {
            (0..at).rev().find(folder).unwrap_or(0)
        };
        self.tree.cursor = vis[to];
    }

    pub(super) fn start_new(&mut self) {
        let dir = match (self.focus, self.tree.selected()) {
            (Focus::Tree, Some(n)) if n.is_dir => Some(n.path.clone()),
            (Focus::Tree, Some(n)) => n.path.parent().map(Path::to_path_buf),
            _ => self
                .rel_current()
                .and_then(|p| Some(p.parent()?.to_path_buf())),
        }
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

    fn create(&mut self, typed: &str) {
        if typed.is_empty() {
            return;
        }
        if self.pushed_folder.is_some() {
            self.message = super::edit::PUSHED.into();
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
        if typed.ends_with('/') || rel.as_os_str().is_empty() {
            self.message = "no file name".into();
            return;
        }
        let Some(rel) = self.in_project(&self.root.join(&rel)) else {
            self.message = "outside the project".into();
            return;
        };
        let path = self.root.join(&rel);
        if path.is_dir() {
            self.say_about(&path, " is a directory");
            return;
        }
        if !self.flush() {
            return;
        }
        let made = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::File::create_new(&path).map(drop));
        match made {
            Ok(()) => {
                let order = (self.tree_order.as_ref()).map(crate::tree::OrderFile::read);
                let order = order.unwrap_or_default();
                let (tree, files) = crate::tree::build_ordered(&self.root, self.shallow, &order);
                self.project_walked(tree, files);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => {
                self.say_about(&path, &format!(": {}", open::why_not(&e.into())));
                return;
            }
        }
        self.jump_to(&path, 0);
        if self.buf.path.as_deref() == Some(&*path) {
            self.start_edit();
        }
    }
}
