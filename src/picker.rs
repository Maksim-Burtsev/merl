//! The fuzzy-picker overlay: one nucleo matcher over a list of labelled targets.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Item, Matcher, Nucleo, Snapshot};
use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::buffer::Buffer;
use crate::git::Mark;
use crate::line_edit::LineEdit;

/// One pickable target. `line` 0 means "no specific line", i.e. keep the file's start.
#[derive(Clone, Debug)]
pub struct PickItem {
    pub label: String,
    pub path: PathBuf,
    pub line: usize,
    /// The byte of line `line` where the word the row is about starts: Enter puts the cursor
    /// there. 0 for a row with no word.
    pub col: usize,
    /// Byte offset in `label` where a copy of the file's line `line` (trimmed, maybe clipped)
    /// starts, so the row can be drawn with that line's syntax colours. `None`: no code text.
    pub code_at: Option<usize>,
    /// A line the branch under review deleted (#440): `line` is its number in the file at the
    /// base, drawn red, and Enter lands on it.
    pub deleted: bool,
}

/// One rendered row: the item plus the char indices of its label that matched the query.
pub struct Row {
    pub item: PickItem,
    pub matched: Vec<u32>,
}

/// What a key did to the picker.
pub enum Pick {
    Stay,
    Cancel,
    Accept(PickItem),
    /// The query of a `live` picker changed: its owner fills the list, not nucleo.
    Typed,
}

pub struct Picker {
    nucleo: Nucleo<PickItem>,
    matcher: Matcher,
    pub query: LineEdit,
    /// The query is not a fuzzy filter over the items: a key that changes it returns
    /// [`Pick::Typed`] and the list stays as it is. `s` greps for it instead.
    pub live: bool,
    pub selected: usize,
    pub title: String,
    /// List height of the last drawn frame, so PgUp/PgDn know how far a page is.
    page: usize,
    /// Files of the rows drawn so far, highlighted up to the deepest row shown. Filled lazily by
    /// `ui`, so a picker over thousands of hits only ever parses what is on screen.
    pub bufs: HashMap<PathBuf, Buffer>,
    /// Review: the gutter marks of the review's files drawn so far, filled lazily by `ui` like
    /// `bufs` (#246).
    pub marks: HashMap<PathBuf, HashMap<usize, Mark>>,
    pub lead: Lead,
    order: Option<Vec<Ref>>,
    order_key: (String, u32),
    pub labels: Vec<(usize, &'static str)>,
}

#[derive(Clone, Debug, Default)]
pub enum Lead {
    #[default]
    None,
    VsCode(Vec<u32>, bool),
    Smart(Vec<(u32, u32)>),
}

#[derive(Clone, Copy, Debug)]
enum Ref {
    Item(u32),
    Matched(u32),
}

fn get<'a>(snap: &'a Snapshot<PickItem>, order: &Option<Vec<Ref>>, n: u32) -> Option<Item<'a, PickItem>> {
    match order {
        None => snap.get_matched_item(n),
        Some(o) => match *o.get(n as usize)? {
            Ref::Item(i) => snap.get_item(i),
            Ref::Matched(m) => snap.get_matched_item(m),
        },
    }
}

impl Picker {
    /// `paths` switches on nucleo's path-aware scoring (it favours matches in the file name).
    pub fn new(title: impl Into<String>, items: Vec<PickItem>, paths: bool) -> Self {
        let config = if paths {
            Config::DEFAULT.match_paths()
        } else {
            Config::DEFAULT
        };
        // No notify: the event loop ticks an open picker every 10 ms anyway, and nucleo calls
        // notify for every item pushed, which was a redraw per row: 5000 frames queued in front
        // of the next key after an `s` refresh.
        let nucleo = Nucleo::new(config, Arc::new(|| {}), None, 1);
        let injector = nucleo.injector();
        // The items arrive already sorted, so injecting them here keeps that order for the
        // empty query and costs nothing worth a background thread.
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
            lead: Lead::None,
            order: None,
            order_key: (String::from("\u{0}"), 0),
            labels: Vec::new(),
        }
    }

    fn refresh_order(&mut self) {
        if matches!(self.lead, Lead::None) {
            return;
        }
        let snap = self.nucleo.snapshot();
        let query = self.query.to_string();
        let key = (query.clone(), snap.matched_item_count());
        if key == self.order_key && self.order.is_some() {
            return;
        }
        self.order_key = key;
        self.labels.clear();
        let matched = snap.matched_item_count();
        let empty = snap.pattern().is_empty();
        let mut order = Vec::new();
        match &self.lead {
            Lead::None => {}
            Lead::VsCode(recent, current_first) if empty => {
                order.extend(recent.iter().map(|&i| Ref::Item(i)));
                if !order.is_empty() {
                    self.labels.push((0, "recently opened"));
                }
                if order.len() > 1 && *current_first {
                    self.selected = 1;
                }
            }
            Lead::VsCode(recent, _) => {
                let pat = nucleo::pattern::Pattern::parse(
                    &query,
                    CaseMatching::Ignore,
                    Normalization::Smart,
                );
                let mut buf = Vec::new();
                let mut hits: Vec<(u32, u32)> = recent
                    .iter()
                    .filter_map(|&i| {
                        let item = snap.get_item(i)?;
                        let label = &item.data.label;
                        let hay = if query.contains('/') {
                            label.as_str()
                        } else {
                            label.rsplit('/').next().unwrap_or(label)
                        };
                        let s = pat.score(nucleo::Utf32Str::new(hay, &mut buf), &mut self.matcher)?;
                        Some((i, s))
                    })
                    .collect();
                hits.sort_by_key(|&(_, s)| std::cmp::Reverse(s));
                let shown: std::collections::HashSet<&PathBuf> = hits
                    .iter()
                    .filter_map(|&(i, _)| snap.get_item(i).map(|it| &it.data.path))
                    .collect();
                order.extend(hits.iter().map(|&(i, _)| Ref::Item(i)));
                if !order.is_empty() {
                    self.labels.push((0, "recently opened"));
                }
                let k = order.len();
                order.extend(
                    (0..matched)
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
            Lead::Smart(recent) if empty => {
                let lead: std::collections::HashSet<u32> = recent.iter().map(|r| r.0).collect();
                order.extend(recent.iter().map(|&(i, _)| Ref::Item(i)));
                let mut rest: Vec<u32> = (0..snap.item_count()).filter(|i| !lead.contains(i)).collect();
                rest.sort_by_key(|&i| snap.get_item(i).map_or(0, |it| it.data.label.len()));
                order.extend(rest.into_iter().map(Ref::Item));
            }
            Lead::Smart(recent) => {
                let bonus: HashMap<&PathBuf, u32> = recent
                    .iter()
                    .filter_map(|&(i, b)| Some((&snap.get_item(i)?.data.path, b)))
                    .collect();
                let mut scored: Vec<(u32, u32, usize)> = (0..matched)
                    .filter_map(|m| {
                        let it = snap.get_matched_item(m)?;
                        let s = snap.pattern().score(it.matcher_columns, &mut self.matcher)?;
                        let b = bonus.get(&it.data.path).copied().unwrap_or(0);
                        Some((m, s + b, it.data.label.len()))
                    })
                    .collect();
                scored.sort_by_key(|&(_, s, len)| (std::cmp::Reverse(s), len));
                order.extend(scored.iter().map(|&(m, _, _)| Ref::Matched(m)));
            }
        }
        self.order = Some(order);
    }

    /// Rows a PgUp / PgDn moves: the list's height in the last frame.
    pub fn page(&self) -> usize {
        self.page
    }

    /// Runs the matcher to completion, for a list that must be readable at once.
    pub fn settle(&mut self) {
        for _ in 0..100 {
            if !self.nucleo.tick(10).running {
                self.refresh_order();
                return;
            }
        }
    }

    /// Lets the matcher work for up to 10 ms. Returns `true` when the results changed.
    pub fn tick(&mut self) -> bool {
        let changed = self.nucleo.tick(10).changed;
        let before = self.order_key.clone();
        self.refresh_order();
        changed || before != self.order_key
    }

    /// `(matched, total)` for the overlay title.
    pub fn counts(&self) -> (u32, u32) {
        let snap = self.nucleo.snapshot();
        match &self.order {
            Some(o) => (o.len() as u32, snap.item_count()),
            None => (snap.matched_item_count(), snap.item_count()),
        }
    }

    /// The rows to draw for a list `height` tall, plus the selected row's offset in them.
    pub fn window(&mut self, height: usize) -> (Vec<Row>, usize) {
        self.page = height.max(1);
        self.refresh_order();
        let total = self.counts().0 as usize;
        let Self {
            nucleo,
            matcher,
            selected,
            order,
            ..
        } = self;
        let snap = nucleo.snapshot();
        if total == 0 {
            *selected = 0;
            return (Vec::new(), 0);
        }
        *selected = (*selected).min(total - 1);
        // Scroll the window so the selection is inside it.
        let height = height.max(1).min(total);
        let start = (*selected + 1).saturating_sub(height).min(total - height);
        let pattern = snap.pattern().column_pattern(0);
        let rows = (start as u32..(start + height) as u32)
            .filter_map(|n| get(snap, order, n))
            .map(|item| {
                let mut matched = Vec::new();
                // Indices are only needed for what is on screen; scoring the whole list would
                // be the expensive part.
                pattern.indices(item.matcher_columns[0].slice(..), matcher, &mut matched);
                matched.sort_unstable();
                matched.dedup();
                Row {
                    item: item.data.clone(),
                    matched,
                }
            })
            .collect();
        (rows, *selected - start)
    }

    /// The item under the cursor, if the query matches anything.
    pub fn current(&self) -> Option<&PickItem> {
        let snap = self.nucleo.snapshot();
        get(snap, &self.order, self.selected as u32).map(|item| item.data)
    }

    /// Enter with nothing matched does nothing, as in VS Code's quick open: the list and the
    /// query stay, so a typo costs a Backspace, not the query (#288). Only Esc closes.
    fn accept(&self) -> Pick {
        match self.current() {
            Some(item) => Pick::Accept(item.clone()),
            None => Pick::Stay,
        }
    }

    /// Hands the query to nucleo, read as it is written in every list (#293, #519): none of
    /// nucleo's pattern syntax, spaces still separating words matched in any order. `append`:
    /// the new pattern extends the old one, so nucleo can refine the previous result set instead
    /// of rescoring everything.
    pub(crate) fn requery(&mut self, append: bool) {
        let mut words: Vec<&str> = self.query.split(' ').collect();
        // A `\` ending a word would escape the space behind it: that word goes last, as the
        // order of the words matches nothing.
        // ponytail: a second such word still joins the next; nobody types two.
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

    /// Text pasted into the query, taken in one go.
    pub fn paste(&mut self, text: &str) -> Pick {
        let old = self.query.to_string();
        match self.query.insert(text) {
            true => self.edited(&old),
            false => Pick::Stay,
        }
    }

    /// The query changed from `old`: a live picker's owner searches again, any other picker
    /// filters itself.
    fn edited(&mut self, old: &str) -> Pick {
        if self.live {
            return Pick::Typed;
        }
        self.requery(self.query.starts_with(old));
        Pick::Stay
    }
}

/// One word of a query, escaped so nucleo reads it as the characters typed: a leading `!`, `^`
/// or `'` and a trailing `$` are its pattern syntax otherwise. nucleo drops the `\` of a leading
/// `\!`, `\^` or `\'` whatever comes before it, so such a word is matched as a substring (`'`),
/// which keeps it.
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
        // Backspacing widens the result set again.
        p.key(KeyCode::Backspace.into());
        p.key(KeyCode::Backspace.into());
        p.key(KeyCode::Backspace.into());
        p.settle();
        assert_eq!(p.counts().0, 3);
    }

    /// A capital in the query does not make it exact: `sameCancel` finds `SameCancel` (#174).
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

    /// `tick` is what the event loop polls; it must report the new results after a query
    /// change, or the list on screen stays one keystroke behind.
    #[test]
    fn tick_reports_a_changed_result_set() {
        let mut p = picker(&["src/wrap.rs", "src/app.rs"]);
        p.key(KeyCode::Char('w').into());
        let changed = (0..100).any(|_| p.tick());
        assert!(changed);
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

    /// A query that matches nothing: Enter keeps the list and the query (#288).
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
    fn window_scrolls_to_keep_the_selection_visible() {
        let mut p = picker(&["a", "b", "c", "d", "e"]);
        p.selected = 4;
        let (rows, sel) = p.window(2);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].item.label, "e");
        assert_eq!(sel, 1);
    }
}
