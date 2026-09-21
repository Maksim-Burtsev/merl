//! The file and theme pickers and the keys typed into a picker.

use super::*;

impl App {
    pub fn open_files_picker(&mut self) {
        let items = self
            .files
            .iter()
            .map(|p| PickItem {
                label: p.display().to_string(),
                path: p.clone(),
                line: 0,
                code_at: None,
            })
            .collect();
        // Only the file picker wants nucleo's path-aware scoring.
        self.picker = Some(Picker::new(PickerKind::Files.title(), items, true));
        self.mode = Mode::Picker(PickerKind::Files);
    }

    pub(crate) fn show_picker(&mut self, kind: PickerKind, items: Vec<PickItem>) {
        // The grep stops at MAX_HITS in file order: say so, or a missing hit looks absent.
        let title = if items.len() >= search::MAX_HITS {
            format!("{} (first {})", kind.title(), search::MAX_HITS)
        } else {
            kind.title().to_string()
        };
        self.picker = Some(Picker::new(title, items, false));
        self.mode = Mode::Picker(kind);
    }

    /// `T`: every theme, the built-ins then the user's own, with the cursor on the one in use.
    pub(super) fn open_themes_picker(&mut self) {
        let themes = crate::theme::entries();
        let items = themes
            .iter()
            .map(|name| PickItem {
                label: name.clone(),
                path: PathBuf::from(name),
                line: 0,
                code_at: None,
            })
            .collect();
        self.show_picker(PickerKind::Themes, items);
        if let Some(p) = &mut self.picker {
            p.selected = themes.iter().position(|n| *n == self.theme).unwrap_or(0);
        }
    }

    /// The theme to draw with: the one under the cursor while the theme picker is open, so
    /// moving previews it, and Esc, which drops the picker, puts `theme` back.
    pub fn shown_theme(&self) -> &str {
        match (&self.picker, self.mode) {
            (Some(p), Mode::Picker(PickerKind::Themes)) => p
                .current()
                .map_or(self.theme.as_str(), |it| it.label.as_str()),
            _ => &self.theme,
        }
    }

    pub(super) fn picker_key(&mut self, key: KeyEvent) {
        let pending = self.search_pending();
        let Some(picker) = &mut self.picker else {
            return;
        };
        if picker.live && key.code == KeyCode::Enter {
            if pending {
                // The list on screen answers an older query: Enter waits for this one's.
                self.search_enter = true;
                // No point in waiting out the pause.
                self.search_due = self.search_due.map(|_| Instant::now());
            } else {
                let (item, query) = (picker.current().cloned(), picker.query.to_string());
                self.search_jump(item, &query);
            }
            return;
        }
        match picker.key(key) {
            Pick::Stay => return,
            Pick::Typed => {
                // `s` has nothing to show without a query. `D` has the list it opened on, so an
                // emptied query asks for that list again rather than for nothing.
                let empty =
                    picker.query.is_empty() && self.mode == Mode::Picker(PickerKind::Search);
                self.search_seq += 1;
                (self.search_sent, self.search_enter) = (None, false);
                self.search_due = Some(Instant::now() + SEARCH_PAUSE);
                if empty {
                    self.search_due = None;
                    self.search_done(self.search_seq, Vec::new());
                }
                return;
            }
            Pick::Cancel => {
                self.picker = None;
                self.close_overlay();
                return;
            }
            Pick::Accept(item) if self.mode == Mode::Picker(PickerKind::Themes) => {
                self.theme = item.label;
                self.message = match &self.config {
                    Some(path) => match crate::theme::save(path, &self.theme) {
                        Ok(()) => format!("theme {} saved", self.theme),
                        Err(e) => format!("{e:#}"),
                    },
                    None => format!("theme {}", self.theme),
                };
            }
            Pick::Accept(item) => {
                let path = self.root.join(&item.path);
                self.jump_to(&path, item.line);
            }
        }
        // Dropping the picker stops nucleo's workers.
        self.picker = None;
        self.mode = Mode::Normal;
    }
}
