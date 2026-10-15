use super::{App, Component, Focus, KeyCode, KeyEvent, LineEdit, Mode, Path, PathBuf, open};

impl App {
    pub(super) fn tree_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => self.tree.up(),
            KeyCode::Down => self.tree.down(),
            KeyCode::Right => self.tree.expand(),
            KeyCode::Left => self.tree.collapse(),
            KeyCode::Char(c @ ('<' | '>')) if std::env::var("MERL_709").as_deref() == Ok("a") => {
                let now = self.tree_hand.or(self.tree_width).unwrap_or(30);
                self.tree_hand = Some(if c == '>' {
                    now.saturating_add(5)
                } else {
                    now.saturating_sub(5)
                });
            }
            KeyCode::Char('A') if std::env::var("MERL_709").as_deref() == Ok("b") => {
                self.tree_zoom = !self.tree_zoom;
            }
            KeyCode::Enter if self.tree.selected().is_some_and(|n| !n.is_dir) && self.tree_zoom => {
                self.tree_zoom = false;
                self.tree_key(code);
            }
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
