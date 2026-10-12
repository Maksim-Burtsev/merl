use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Matcher, Nucleo};
use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::buffer::Buffer;
use crate::git::Mark;
use crate::line_edit::LineEdit;

#[derive(Clone, Debug)]
pub struct PickItem {
    pub label: String,
    pub path: PathBuf,
    pub line: usize,
    pub col: usize,
    pub code_at: Option<usize>,
    pub path_at: Option<std::ops::Range<usize>>,
    pub deleted: bool,
}

pub struct Row {
    pub item: PickItem,
    pub matched: Vec<u32>,
}

pub enum Pick {
    Stay,
    Cancel,
    Accept(PickItem),
    Typed,
}

pub struct Picker {
    nucleo: Nucleo<PickItem>,
    matcher: Matcher,
    pub query: LineEdit,
    pub live: bool,
    pub selected: usize,
    pub title: String,
    page: usize,
    pub bufs: HashMap<PathBuf, Buffer>,
    pub marks: HashMap<PathBuf, HashMap<usize, Mark>>,
    pub first: usize,
}

impl Picker {
    pub fn new(title: impl Into<String>, items: Vec<PickItem>, paths: bool) -> Self {
        let config = if paths {
            Config::DEFAULT.match_paths()
        } else {
            Config::DEFAULT
        };
        let nucleo = Nucleo::new(config, Arc::new(|| {}), None, 1);
        let injector = nucleo.injector();
        for item in items {
            injector.push(item, |it, cols| cols[0] = it.label.as_str().into());
        }
        Self {
            nucleo,
            matcher: Matcher::new(Config::DEFAULT),
            query: LineEdit::default(),
            live: false,
            selected: 0,
            title: title.into(),
            page: 10,
            bufs: HashMap::new(),
            marks: HashMap::new(),
            first: 0,
        }
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn settle(&mut self) {
        for _ in 0..100 {
            if !self.nucleo.tick(10).running {
                return;
            }
        }
    }

    pub fn tick(&mut self) -> bool {
        self.nucleo.tick(10).changed
    }

    pub fn counts(&self) -> (u32, u32) {
        let snap = self.nucleo.snapshot();
        (snap.matched_item_count(), snap.item_count())
    }

    pub fn window(&mut self, height: usize) -> (Vec<Row>, usize) {
        self.page = height.max(1);
        let total = self.clamp_selected();
        if total == 0 {
            return (Vec::new(), 0);
        }
        let height = height.max(1).min(total);
        let start = (self.selected + 1)
            .saturating_sub(height)
            .min(total - height);
        (self.rows(start, height), self.selected - start)
    }

    pub fn clamp_selected(&mut self) -> usize {
        let total = self.counts().0 as usize;
        self.selected = self.selected.min(total.saturating_sub(1));
        total
    }

    pub fn set_page(&mut self, rows: usize) {
        self.page = rows.max(1);
    }

    pub fn rows(&mut self, start: usize, count: usize) -> Vec<Row> {
        let Self {
            nucleo, matcher, ..
        } = self;
        let snap = nucleo.snapshot();
        let total = snap.matched_item_count() as usize;
        let start = start.min(total);
        let end = start.saturating_add(count).min(total);
        let pattern = snap.pattern().column_pattern(0);
        snap.matched_items(start as u32..end as u32)
            .map(|item| {
                let mut matched = Vec::new();
                pattern.indices(item.matcher_columns[0].slice(..), matcher, &mut matched);
                matched.sort_unstable();
                matched.dedup();
                Row {
                    item: item.data.clone(),
                    matched,
                }
            })
            .collect()
    }

    pub fn current(&self) -> Option<&PickItem> {
        let snap = self.nucleo.snapshot();
        snap.get_matched_item(self.selected as u32)
            .map(|item| item.data)
    }

    fn accept(&self) -> Pick {
        match self.current() {
            Some(item) => Pick::Accept(item.clone()),
            None => Pick::Stay,
        }
    }

    pub(crate) fn requery(&mut self, append: bool) {
        let mut words: Vec<&str> = self.query.split(' ').collect();
        words.sort_by_key(|w| w.ends_with('\\'));
        let query = words.into_iter().map(escape).collect::<Vec<_>>().join(" ");
        self.nucleo.pattern.reparse(
            0,
            &query,
            CaseMatching::Ignore,
            Normalization::Smart,
            append,
        );
        self.selected = 0;
    }

    pub fn key(&mut self, key: KeyEvent) -> Pick {
        let matched = self.counts().0 as usize;
        let move_by = |sel: &mut usize, delta: isize| {
            *sel = sel
                .saturating_add_signed(delta)
                .min(matched.saturating_sub(1));
        };
        match key.code {
            KeyCode::Esc => return Pick::Cancel,
            KeyCode::Enter => return self.accept(),
            KeyCode::Up => move_by(&mut self.selected, -1),
            KeyCode::Down => move_by(&mut self.selected, 1),
            KeyCode::PageUp => move_by(&mut self.selected, -(self.page as isize)),
            KeyCode::PageDown => move_by(&mut self.selected, self.page as isize),
            _ => {
                let old = self.query.to_string();
                if self.query.key(key) {
                    return self.edited(&old);
                }
            }
        }
        Pick::Stay
    }

    pub fn paste(&mut self, text: &str) -> Pick {
        let old = self.query.to_string();
        match self.query.insert(text) {
            true => self.edited(&old),
            false => Pick::Stay,
        }
    }

    fn edited(&mut self, old: &str) -> Pick {
        if self.live {
            return Pick::Typed;
        }
        self.requery(self.query.starts_with(old));
        Pick::Stay
    }
}

fn escape(word: &str) -> String {
    let lead = if word.starts_with(['!', '^', '\'']) {
        "\\"
    } else if word
        .strip_prefix('\\')
        .is_some_and(|w| w.starts_with(['!', '^', '\'']))
    {
        "'"
    } else {
        ""
    };
    match word.strip_suffix('$') {
        Some(rest) => format!("{lead}{rest}\\$"),
        None => format!("{lead}{word}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picker(labels: &[&str]) -> Picker {
        let items = labels
            .iter()
            .map(|l| PickItem {
                label: (*l).to_string(),
                code_at: None,
                path_at: None,
                path: PathBuf::from(l),
                line: 0,
                col: 0,
                deleted: false,
            })
            .collect();
        let mut p = Picker::new("Files", items, true);
        p.settle();
        p
    }

    #[test]
    fn filters_and_reports_match_positions() {
        let mut p = picker(&["src/wrap.rs", "src/app.rs", "README.md"]);
        assert_eq!(p.counts(), (3, 3));
        for c in "wra".chars() {
            p.key(KeyCode::Char(c).into());
        }
        p.settle();
        assert_eq!(p.counts().0, 1);
        let (rows, sel) = p.window(5);
        assert_eq!(sel, 0);
        assert_eq!(rows[0].item.label, "src/wrap.rs");
        assert_eq!(rows[0].matched, [4, 5, 6]);
        p.key(KeyCode::Backspace.into());
        p.key(KeyCode::Backspace.into());
        p.key(KeyCode::Backspace.into());
        p.settle();
        assert_eq!(p.counts().0, 3, "backspacing widens the result set again");
    }

    #[test]
    fn of_two_words_ending_in_a_backslash_the_first_joins_the_second() {
        let mut p = picker(&["r p\\q\\", "r p q\\"]);
        for c in "p\\ q\\ r".chars() {
            p.key(KeyCode::Char(c).into());
        }
        p.settle();
        assert_eq!(p.counts().0, 1);
        assert_eq!(p.window(5).0[0].item.label, "r p q\\");
    }

    #[test]
    fn a_capital_in_the_query_still_ignores_case() {
        let mut p = picker(&["SameCancel", "Walk"]);
        for c in "sameCancel".chars() {
            p.key(KeyCode::Char(c).into());
        }
        p.settle();
        assert_eq!(p.counts().0, 1);
        assert_eq!(p.window(5).0[0].item.label, "SameCancel");
    }

    #[test]
    fn tick_reports_a_changed_result_set() {
        let mut p = picker(&["src/wrap.rs", "src/app.rs"]);
        p.key(KeyCode::Char('w').into());
        let changed = (0..100).any(|_| p.tick());
        assert!(
            changed,
            "the event loop polls tick: without the change the list stays a keystroke behind"
        );
        assert_eq!(p.counts().0, 1);
    }

    #[test]
    fn movement_clamps_and_enter_accepts() {
        let mut p = picker(&["a.rs", "b.rs"]);
        p.key(KeyCode::Up.into());
        assert_eq!(p.selected, 0);
        for _ in 0..5 {
            p.key(KeyCode::Down.into());
        }
        assert_eq!(p.selected, 1);
        match p.key(KeyCode::Enter.into()) {
            Pick::Accept(it) => assert_eq!(it.label, "b.rs"),
            _ => panic!("enter must accept"),
        }
        assert!(matches!(p.key(KeyCode::Esc.into()), Pick::Cancel));
    }

    #[test]
    fn enter_with_nothing_matched_stays() {
        let mut p = picker(&["store.py", "cli.py"]);
        for c in "storx".chars() {
            p.key(KeyCode::Char(c).into());
        }
        p.settle();
        assert_eq!(p.counts().0, 0);
        assert!(matches!(p.key(KeyCode::Enter.into()), Pick::Stay));
        assert_eq!(p.query.to_string(), "storx");
    }

    #[test]
    fn a_live_picker_hands_the_query_to_its_owner_and_keeps_its_list() {
        let mut p = picker(&["src/wrap.rs", "src/app.rs"]);
        p.live = true;
        assert!(matches!(p.key(KeyCode::Char('w').into()), Pick::Typed));
        assert!(matches!(p.paste("ra"), Pick::Typed));
        p.settle();
        assert_eq!(p.counts(), (2, 2));
    }

    #[test]
    fn a_page_is_as_tall_as_the_list_last_drawn() {
        let mut p = picker(&["a", "b", "c", "d", "e", "f", "g"]);
        p.window(3);
        p.key(KeyCode::PageDown.into());
        assert_eq!(p.selected, 3);
        p.key(KeyCode::PageUp.into());
        assert_eq!(p.selected, 0);
    }

    #[test]
    fn window_scrolls_to_keep_the_selection_visible() {
        let mut p = picker(&["a", "b", "c", "d", "e"]);
        p.selected = 4;
        let (rows, sel) = p.window(2);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].item.label, "e");
        assert_eq!(sel, 1);
    }
}
