use super::{App, Buffer, Path, search};

impl App {
    pub(super) fn answered_in_razor(&mut self) -> bool {
        let Some(path) = self.buf.path.clone().filter(|p| search::razor(p)) else {
            return false;
        };
        if self.razor_view || self.base_place().is_some() {
            return false;
        }
        let imports: String = (path.ancestors().skip(1))
            .take_while(|d| d.starts_with(&self.root))
            .filter_map(|d| std::fs::read_to_string(d.join("_Imports.razor")).ok())
            .collect::<Vec<_>>()
            .join("\n");
        let (view, class) = search::razor_view(&path, &self.buf.lines, self.line, &imports);
        let mut b = Self::new(
            self.root.clone(),
            crate::tree::Tree::default(),
            self.files.clone(),
            Buffer::read_only_view(path.clone(), view),
            None,
        );
        b.razor_view = true;
        (b.line, b.col) = (self.line, self.col);
        let (message, picked, jumped) = b.answer_of_d();
        let (lines, root) = (self.buf.lines.len(), self.root.clone());
        let real = |p: &Path, line1: usize| {
            root.join(p) != path || line1 == 0 || line1 <= lines && Some(line1 - 1) != class
        };
        match (picked, jumped) {
            (Some((kind, title, items)), _) => {
                let all = items.len();
                let items: Vec<_> = items
                    .into_iter()
                    .filter(|it| real(&it.path, it.line))
                    .collect();
                let count = |now: &str| message.replacen(&format!("{all} declarations"), now, 1);
                let message = match items.len() {
                    0 => {
                        self.message = "no definition".into();
                        return true;
                    }
                    n if n == all => message,
                    1 => {
                        self.jump_to_item(&items[0]);
                        if self.buf.path.as_deref() == Some(&*root.join(&items[0].path)) {
                            self.message = count("1 match");
                        }
                        return true;
                    }
                    n => count(&format!("{n} declarations")),
                };
                self.show_picker(kind, items);
                if let Some(p) = &mut self.picker {
                    p.title = title;
                }
                self.message = message;
            }
            (None, Some((Some(to), line1, col))) if real(&to, line1) => {
                self.jump_to_col(&to, line1, col);
                if self.buf.path.as_deref() == Some(to.as_path()) {
                    self.message = message;
                }
            }
            (None, Some(_)) => self.message = "no definition".into(),
            (None, None) => self.message = message,
        }
        true
    }
}
