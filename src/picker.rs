use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Item, Matcher, Nucleo, Snapshot, Utf32Str};

use crate::buffer::Buffer;
use crate::git::Mark;
use crate::key::{KeyCode, KeyEvent};
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
    history: Option<(Vec<u32>, bool)>,
    order: Option<Vec<Ref>>,
    order_for: Option<String>,
    pub labels: Vec<(usize, &'static str)>,
}

#[derive(Clone, Copy)]
enum Ref {
    Item(u32),
    Named(u32),
    Matched(u32),
}

fn nth<'a>(
    snap: &'a Snapshot<PickItem>,
    order: Option<&[Ref]>,
    n: u32,
) -> Option<(Item<'a, PickItem>, bool)> {
    match order {
        None => snap.get_matched_item(n).map(|it| (it, false)),
        Some(o) => match *o.get(n as usize)? {
            Ref::Item(i) => snap.get_item(i).map(|it| (it, false)),
            Ref::Named(i) => snap.get_item(i).map(|it| (it, true)),
            Ref::Matched(m) => snap.get_matched_item(m).map(|it| (it, false)),
        },
    }
}

fn file_name(label: &str) -> &str {
    label.rsplit('/').next().unwrap_or(label)
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
            history: None,
            order: None,
            order_for: None,
            labels: Vec::new(),
        }
    }

    pub fn lead_with_history(&mut self, items: Vec<u32>, current_first: bool) {
        self.history = Some((items, current_first));
    }

    fn refresh_order(&mut self, matches_changed: bool) {
        let Some((history, current_first)) = &self.history else {
            return;
        };
        let query = self.query.to_string();
        let empty = query.trim().is_empty();
        let again = self.order_for.as_deref() == Some(query.as_str());
        if again && !matches_changed {
            return;
        }
        let snap = self.nucleo.snapshot();
        self.labels.clear();
        let mut order: Vec<Ref> = Vec::new();
        if empty {
            order.extend(
                history
                    .iter()
                    .filter(|&&i| snap.get_item(i).is_some())
                    .map(|&i| Ref::Item(i)),
            );
            if !order.is_empty() {
                self.labels.push((0, "recently opened"));
            }
            if !again && *current_first && order.len() > 1 {
                self.selected = 1;
            }
        } else {
            let pattern = snap.pattern().column_pattern(0);
            let whole = query.contains('/');
            let mut buf = Vec::new();
            let mut hits: Vec<(u32, u32)> = history
                .iter()
                .filter_map(|&i| {
                    let label = &snap.get_item(i)?.data.label;
                    let hay = if whole {
                        label.as_str()
                    } else {
                        file_name(label)
                    };
                    let score = pattern.score(Utf32Str::new(hay, &mut buf), &mut self.matcher)?;
                    Some((i, score))
                })
                .collect();
            hits.sort_by_key(|&(_, score)| std::cmp::Reverse(score));
            let shown: std::collections::HashSet<&PathBuf> = (hits.iter())
                .filter_map(|&(i, _)| snap.get_item(i).map(|it| &it.data.path))
                .collect();
            let by_name = |i| if whole { Ref::Item(i) } else { Ref::Named(i) };
            order.extend(hits.iter().map(|&(i, _)| by_name(i)));
            if !order.is_empty() {
                self.labels.push((0, "recently opened"));
            }
            let k = order.len();
            order.extend(
                (0..snap.matched_item_count())
                    .filter(|&m| {
                        snap.get_matched_item(m)
                            .is_some_and(|it| !shown.contains(&it.data.path))
                    })
                    .map(Ref::Matched),
            );
            if order.len() > k {
                self.labels.push((k, "file results"));
            }
        }
        self.order = Some(order);
        self.order_for = Some(query);
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn settle(&mut self) {
        for _ in 0..100 {
            if !self.tick_once().running {
                break;
            }
        }
    }

    pub fn tick(&mut self) -> bool {
        let shown = self.order_for.clone();
        self.tick_once().changed || self.order_for != shown
    }

    fn tick_once(&mut self) -> nucleo::Status {
        let status = self.nucleo.tick(10);
        self.refresh_order(status.changed);
        status
    }

    pub fn counts(&self) -> (u32, u32) {
        let snap = self.nucleo.snapshot();
        match &self.order {
            Some(order) => (
                u32::try_from(order.len()).unwrap_or(u32::MAX),
                snap.item_count(),
            ),
            None => (snap.matched_item_count(), snap.item_count()),
        }
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
        self.refresh_order(false);
        let total = self.counts().0 as usize;
        self.selected = self.selected.min(total.saturating_sub(1));
        total
    }

    pub fn set_page(&mut self, rows: usize) {
        self.page = rows.max(1);
    }

    pub fn rows(&mut self, start: usize, count: usize) -> Vec<Row> {
        self.refresh_order(false);
        let total = self.counts().0 as usize;
        let Self {
            nucleo,
            matcher,
            order,
            ..
        } = self;
        let snap = nucleo.snapshot();
        let start = start.min(total);
        let end = start.saturating_add(count).min(total);
        let pattern = snap.pattern().column_pattern(0);
        (start..end)
            .filter_map(|n| nth(snap, order.as_deref(), u32::try_from(n).ok()?))
            .map(|(item, named)| {
                let mut matched = Vec::new();
                if named {
                    let (label, mut buf) = (&item.data.label, Vec::new());
                    let name = file_name(label);
                    pattern.indices(Utf32Str::new(name, &mut buf), matcher, &mut matched);
                    let skip = u32::try_from(label.chars().count() - name.chars().count())
                        .unwrap_or(u32::MAX);
                    for m in &mut matched {
                        *m += skip;
                    }
                } else {
                    pattern.indices(item.matcher_columns[0].slice(..), matcher, &mut matched);
                }
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
        nth(
            snap,
            self.order.as_deref(),
            u32::try_from(self.selected).ok()?,
        )
        .map(|(item, _)| item.data)
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
        self.refresh_order(false);
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
            KeyCode::PageUp => move_by(
                &mut self.selected,
                -isize::try_from(self.page).unwrap_or(isize::MAX),
            ),
            KeyCode::PageDown => move_by(
                &mut self.selected,
                isize::try_from(self.page).unwrap_or(isize::MAX),
            ),
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
        if self.query.insert(text) {
            self.edited(&old)
        } else {
            Pick::Stay
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
                label: (*l).to_owned(),
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
