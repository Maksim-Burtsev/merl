use std::fmt::Write as _;

use super::{
    App, HashMap, Instant, KeyCode, KeyEvent, Mode, PathBuf, Pick, PickItem, Picker, PickerKind,
    SEARCH_PAUSE, error_text, search,
};

const RECENT_MAX: usize = 200;

impl App {
    pub fn open_files_picker(&mut self) {
        self.read_recent();
        let order: HashMap<&PathBuf, usize> = match &self.review {
            Some(r) if self.review_open_files_first => r
                .files
                .iter()
                .enumerate()
                .map(|(i, f)| (&f.path, i))
                .collect(),
            _ => HashMap::new(),
        };
        let (mut review, rest): (Vec<_>, Vec<_>) = self
            .files
            .iter()
            .chain(&self.ignored)
            .partition(|p| order.contains_key(p));
        review.sort_by_key(|p| order[p]);
        let listed: Vec<&PathBuf> = review.into_iter().chain(rest).collect();
        let lead = Some(self.history_rows(&listed))
            .filter(|(rows, _)| self.review.is_none() && self.tutor.is_none() && !rows.is_empty());
        let items = listed
            .into_iter()
            .map(|p| PickItem {
                label: p.display().to_string(),
                path: p.clone(),
                line: 0,
                col: 0,
                code_at: None,
                path_at: None,
                deleted: false,
            })
            .collect();
        let mut picker = Picker::new(PickerKind::Files.title(), items, true);
        if let Some((rows, current_first)) = lead {
            picker.lead_with_history(rows, current_first);
        }
        self.picker = Some(picker);
        self.mode = Mode::Picker(PickerKind::Files);
    }

    fn history_rows(&self, listed: &[&PathBuf]) -> (Vec<u32>, bool) {
        let row: HashMap<PathBuf, u32> = (listed.iter().enumerate())
            .map(|(i, p)| (self.root.join(p), u32::try_from(i).unwrap_or(u32::MAX)))
            .collect();
        let current = self.buf.path.as_ref().and_then(|p| row.get(p)).copied();
        let rest = (self.recent.iter())
            .filter_map(|p| row.get(p).copied())
            .filter(|&i| Some(i) != current);
        (current.into_iter().chain(rest).collect(), current.is_some())
    }

    fn read_recent(&mut self) {
        if let Some(text) =
            (self.recent_file.as_ref()).and_then(|f| std::fs::read_to_string(f).ok())
        {
            self.recent = text.lines().map(PathBuf::from).collect();
        }
    }

    pub fn note_recent(&mut self) {
        let Some(path) = self.buf.path.clone() else {
            return;
        };
        self.read_recent();
        self.recent.retain(|p| *p != path && p.is_file());
        self.recent.insert(0, path);
        self.recent.truncate(RECENT_MAX);
        let Some(file) = &self.recent_file else {
            return;
        };
        let mut text = String::new();
        for p in &self.recent {
            _ = writeln!(text, "{}", p.display());
        }
        let tmp = file.with_file_name(format!(".recent-{}", crate::real::pid()));
        let written = (file.parent().map_or(Ok(()), std::fs::create_dir_all))
            .and_then(|()| std::fs::write(&tmp, text))
            .and_then(|()| std::fs::rename(&tmp, file));
        if written.is_err() {
            drop(std::fs::remove_file(&tmp));
        }
    }

    pub(crate) fn show_picker(&mut self, kind: PickerKind, items: Vec<PickItem>) {
        let cut = items.len() >= search::MAX_HITS;
        self.show_cut_picker(kind, items, cut);
    }

    pub(crate) fn show_cut_picker(&mut self, kind: PickerKind, items: Vec<PickItem>, cut: bool) {
        let title = if cut {
            format!("{} (first {})", kind.title(), search::MAX_HITS)
        } else {
            kind.title().to_owned()
        };
        self.picker = Some(Picker::new(title, items, false));
        self.mode = Mode::Picker(kind);
    }

    pub(super) fn open_themes_picker(&mut self) {
        let themes = crate::theme::entries_in(self.theme_dir.as_deref());
        let items = themes
            .iter()
            .map(|name| PickItem {
                label: name.clone(),
                path: PathBuf::from(name),
                line: 0,
                col: 0,
                code_at: None,
                path_at: None,
                deleted: false,
            })
            .collect();
        self.show_picker(PickerKind::Themes, items);
        if let Some(p) = &mut self.picker {
            p.selected = themes.iter().position(|n| *n == self.theme).unwrap_or(0);
        }
    }

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
                self.search_enter = true;
                self.search_due = self.search_due.map(|_| Instant::now());
            } else if let Some(item) = picker.current().cloned() {
                self.search_jump(&item);
            }
            return;
        }
        match picker.key(key) {
            Pick::Stay => return,
            Pick::Typed => return self.search_typed(),
            Pick::Cancel => {
                self.picker = None;
                self.close_overlay();
                return;
            }
            Pick::Accept(item) if self.mode == Mode::Picker(PickerKind::Themes) => {
                if let Err(e) = crate::theme::load_from(self.theme_dir.as_deref(), &item.label) {
                    self.message = error_text(&e);
                    return;
                }
                self.theme = item.label;
                self.message = match &self.config {
                    Some(path) => match crate::theme::save(path, &self.theme) {
                        Ok(()) => format!("theme {} saved", self.theme),
                        Err(e) => error_text(&e),
                    },
                    None => format!("theme {}", self.theme),
                };
            }
            Pick::Accept(item) => self.jump_to_item(&item),
        }
        self.picker = None;
        self.mode = Mode::Normal;
    }

    pub(super) fn search_typed(&mut self) {
        let empty = self.picker.as_ref().is_some_and(|p| p.query.is_empty())
            && self.mode == Mode::Picker(PickerKind::Search);
        self.search_seq += 1;
        (self.search_sent, self.search_enter) = (None, false);
        self.search_due = Some(Instant::now() + SEARCH_PAUSE);
        if empty {
            self.search_due = None;
            self.search_done(self.search_seq, Vec::new());
        }
    }
}
