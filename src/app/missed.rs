use super::cursor::{word_end, word_start};
use super::keys::named;
use super::*;

const RUN_GAP: Duration = Duration::from_millis(200);
const SPENT_PER_MISSED_PRESS: usize = 2;
const MIN_PRESSES_SAVED: usize = 3;
const HISTORY_STEPS_OFFERED_EITHER_WAY: usize = 3;

const SHORTCUTS: [(KeyCode, KeyModifiers); 21] = [
    (KeyCode::Char('d'), KeyModifiers::CONTROL),
    (KeyCode::Char('u'), KeyModifiers::CONTROL),
    (KeyCode::PageUp, KeyModifiers::NONE),
    (KeyCode::PageDown, KeyModifiers::NONE),
    (KeyCode::Char('{'), KeyModifiers::NONE),
    (KeyCode::Char('}'), KeyModifiers::NONE),
    (KeyCode::Home, KeyModifiers::CONTROL),
    (KeyCode::End, KeyModifiers::CONTROL),
    (KeyCode::Home, KeyModifiers::NONE),
    (KeyCode::End, KeyModifiers::NONE),
    (KeyCode::Left, KeyModifiers::ALT),
    (KeyCode::Right, KeyModifiers::ALT),
    (KeyCode::Left, KeyModifiers::ALT.union(KeyModifiers::SHIFT)),
    (KeyCode::Right, KeyModifiers::ALT.union(KeyModifiers::SHIFT)),
    (
        KeyCode::Left,
        KeyModifiers::CONTROL.union(KeyModifiers::SHIFT),
    ),
    (
        KeyCode::Right,
        KeyModifiers::CONTROL.union(KeyModifiers::SHIFT),
    ),
    (KeyCode::PageUp, KeyModifiers::SHIFT),
    (KeyCode::PageDown, KeyModifiers::SHIFT),
    (
        KeyCode::Home,
        KeyModifiers::CONTROL.union(KeyModifiers::SHIFT),
    ),
    (
        KeyCode::End,
        KeyModifiers::CONTROL.union(KeyModifiers::SHIFT),
    ),
    (KeyCode::Char('v'), KeyModifiers::NONE),
];
const PAGES: [(KeyCode, KeyModifiers); 2] = [
    (KeyCode::PageUp, KeyModifiers::NONE),
    (KeyCode::PageDown, KeyModifiers::NONE),
];

#[derive(Default)]
pub(super) struct Watch {
    previous_key_at: Option<Instant>,
    run: Option<Run>,
    trip: Option<Trip>,
}

struct Run {
    key: (KeyCode, KeyModifiers),
    mode: Mode,
    path: Option<PathBuf>,
    lines: usize,
    before_first_press: Spot,
    deleting_line_before_first_press: String,
    last_end: End,
    presses_that_moved: usize,
}

struct Trip {
    kind: PickerKind,
    n: usize,
    opened_at: (PathBuf, TextLine),
    opened_at_rel: Option<PathBuf>,
    word_at_open: Option<String>,
    last_query: String,
    history_files_by_step: Vec<(isize, PathBuf)>,
    file_c_goes_on_to: Option<PathBuf>,
}

#[derive(Clone)]
struct Spot {
    line: usize,
    col: usize,
    deleted: Option<(usize, usize)>,
    want_x: usize,
    anchor: Option<(TextLine, usize)>,
    top: (usize, usize),
    left: usize,
    center: bool,
    row: Option<usize>,
}

impl Spot {
    fn at(&self) -> TextLine {
        self.deleted
            .map_or(TextLine::File(self.line), |(k, i)| TextLine::Deleted(k, i))
    }
}

type End = ((TextLine, usize), Option<(TextLine, usize)>);

fn max_missed_presses(spent: usize) -> usize {
    (spent / SPENT_PER_MISSED_PRESS).min(spent.saturating_sub(MIN_PRESSES_SAVED))
}

impl App {
    pub(super) fn watch_before(&mut self, key: KeyEvent, at: Instant) {
        let fast = self
            .watch
            .previous_key_at
            .is_some_and(|t| at.saturating_duration_since(t) < RUN_GAP);
        self.watch.previous_key_at = Some(at);
        let query = self.picker.as_ref().map(|p| p.query.to_string());
        if let Some(trip) = &mut self.watch.trip {
            trip.n += 1;
            trip.last_query = query.unwrap_or_default();
        }
        let end = self.end();
        if let Some(run) = &mut self.watch.run {
            if run.last_end != end {
                (run.last_end, run.presses_that_moved) = (end, run.presses_that_moved + 1);
            }
            if fast && run.key == (key.code, key.modifiers) {
                return;
            }
        }
        if let Some(run) = self.watch.run.take() {
            self.judge_run(run);
        }
        self.watch.run = self.start_run(key);
    }

    pub(super) fn watch_after(&mut self, key: KeyEvent, was: Mode, had_picker: bool) {
        if self.picker.is_some() {
            if !had_picker && was == Mode::Normal {
                self.watch.trip = self.start_trip();
            }
            return;
        }
        if let Some(trip) = self.watch.trip.take()
            && key.code == KeyCode::Enter
        {
            self.judge_trip(trip);
        }
    }

    pub(super) fn watch_jumped(&mut self) {
        if let Some(trip) = self.watch.trip.take() {
            self.judge_trip(trip);
        }
    }

    fn start_run(&self, key: KeyEvent) -> Option<Run> {
        let (code, m) = (key.code, key.modifiers);
        let arrow = matches!(
            code,
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right
        );
        let deleting = matches!(code, KeyCode::Backspace | KeyCode::Delete)
            && m.is_empty()
            && self.mode == Mode::Edit
            && self.selection().is_none();
        let paging = matches!(code, KeyCode::PageUp | KeyCode::PageDown) && m.is_empty()
            || matches!(code, KeyCode::Char('d' | 'u')) && m == KeyModifiers::CONTROL;
        let runs = match self.mode {
            Mode::Picker(_) => {
                self.picker.is_some() && matches!(code, KeyCode::Up | KeyCode::Down) && m.is_empty()
            }
            Mode::Normal | Mode::Edit => {
                self.focus == Focus::Code
                    && self.buf.path.is_some()
                    && !self.previewing()
                    && (arrow && (m.is_empty() || m == KeyModifiers::SHIFT)
                        || deleting
                        || paging && self.review.is_some() && self.mode == Mode::Normal)
            }
            _ => false,
        };
        runs.then(|| Run {
            key: (code, m),
            mode: self.mode,
            path: self.buf.path.clone(),
            lines: self.buf.lines.len(),
            before_first_press: self.spot(),
            deleting_line_before_first_press: if deleting {
                self.line_str().to_string()
            } else {
                String::new()
            },
            last_end: self.end(),
            presses_that_moved: 0,
        })
    }

    fn judge_run(&mut self, run: Run) {
        let from = (run.before_first_press.at(), run.before_first_press.col);
        let deleting = !run.deleting_line_before_first_press.is_empty();
        if run.mode != self.mode
            || run.path != self.buf.path
            || run.before_first_press.row.is_some() != self.picker.is_some()
            || !deleting && (run.lines != self.buf.lines.len() || self.clamp_place(from) != from)
        {
            return;
        }
        let found = if deleting {
            self.word_chord(&run)
        } else {
            let budget = max_missed_presses(run.presses_that_moved);
            if budget == 0 {
                return;
            }
            self.hunk_reached(&run)
                .or_else(|| self.shortcut(&run, budget))
        };
        if let Some((_, action)) = found {
            *self.missed.entry(action).or_default() += 1;
        }
    }

    fn hunk_reached(&self, run: &Run) -> Option<(usize, &'static str)> {
        let (code, m) = run.key;
        let plain = m.is_empty()
            && matches!(
                code,
                KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::PageUp
                    | KeyCode::PageDown
            )
            || m == KeyModifiers::CONTROL && matches!(code, KeyCode::Char('d' | 'u'));
        if self.review.is_none() || run.mode != Mode::Normal || !plain {
            return None;
        }
        let hunks = &self.diff.hunks;
        let (from, to) = (
            self.hunk_place(run.before_first_press.at()),
            self.hunk_place(self.at()),
        );
        let (&start, key) = match to.cmp(&from) {
            std::cmp::Ordering::Greater => (hunks.iter().find(|&&h| h > from)?, "c"),
            std::cmp::Ordering::Less => (hunks.iter().rev().find(|&&h| h < from)?, "C"),
            std::cmp::Ordering::Equal => return None,
        };
        let changed = |t: TextLine| match t {
            TextLine::Deleted(..) => true,
            TextLine::File(l) => self.diff.marks.contains_key(&l),
        };
        let mut end = start;
        while let Some(t) = self
            .next_line(end)
            .filter(|&t| changed(t) && !hunks.contains(&t))
        {
            end = t;
        }
        let hunk = start..=end;
        (hunk.contains(&to) && !hunk.contains(&from)).then_some((1, key))
    }

    fn shortcut(&mut self, run: &Run, budget: usize) -> Option<(usize, &'static str)> {
        let (code, m) = run.key;
        let back = match code {
            KeyCode::Up => KeyCode::Down,
            KeyCode::Down => KeyCode::Up,
            KeyCode::Left => KeyCode::Right,
            KeyCode::Right => KeyCode::Left,
            _ => return None,
        };
        let arrows = [KeyEvent::new(code, m), KeyEvent::new(back, m)];
        let (scope, keys) = match run.before_first_press.row {
            Some(_) => ("Picker: ", &PAGES[..]),
            None => ("", &SHORTCUTS[..]),
        };
        let end = self.end();
        let now = self.spot();
        let kept = (
            self.history.clone(),
            self.hist_idx,
            std::mem::take(&mut self.message),
        );
        let mut best: Option<(usize, &'static str)> = None;
        for &(kc, km) in keys {
            if run.mode == Mode::Edit && matches!(kc, KeyCode::Char(_)) && km.is_empty() {
                continue;
            }
            let key = KeyEvent::new(kc, km);
            let limit = best.map_or(budget, |(cost, _)| cost - 1);
            if let Some(cost) = self.presses(&run.before_first_press, &end, key, arrows, limit)
                && let Some(action) = named(scope, key)
            {
                best = Some((cost, action));
            }
        }
        self.put(&now);
        (self.history, self.hist_idx, self.message) = kept;
        self.keys_action_of_key_in_hand = None;
        best
    }

    fn presses(
        &mut self,
        from: &Spot,
        end: &End,
        key: KeyEvent,
        arrows: [KeyEvent; 2],
        limit: usize,
    ) -> Option<usize> {
        let mut best: Option<usize> = None;
        self.put(from);
        for m in 1..=limit {
            let cap = best.map_or(limit, |b| b - 1);
            if m > cap {
                break;
            }
            let was = self.end();
            self.press(key);
            if self.end() == was {
                break;
            }
            let here = self.spot();
            for arrow in arrows {
                self.put(&here);
                let mut f = 0;
                loop {
                    let cap = best.map_or(limit, |b| b - 1);
                    if self.end() == *end {
                        best = Some(m + f);
                        break;
                    }
                    // An arrow moves one line or one row at most.
                    let apart = self.end().0.0.key().abs_diff(end.0.0.key());
                    if m + f + apart.max(1) > cap {
                        break;
                    }
                    let was = self.end();
                    self.press(arrow);
                    f += 1;
                    if self.end() == was {
                        break;
                    }
                }
            }
            self.put(&here);
        }
        best
    }

    fn press(&mut self, key: KeyEvent) {
        self.key_inner(key);
        self.clamp_scroll();
    }

    fn word_chord(&self, run: &Run) -> Option<(usize, &'static str)> {
        if self.line != run.before_first_press.line || self.buf.lines.len() != run.lines {
            return None;
        }
        let (s, now, from) = (
            run.deleting_line_before_first_press.as_str(),
            self.line_str(),
            run.before_first_press.col,
        );
        let gone = s.len().checked_sub(now.len())?;
        let (to, step): (usize, fn(&str, usize) -> usize) = match run.key.0 {
            KeyCode::Backspace => (self.col, word_start),
            _ => (from + gone, word_end),
        };
        let (lo, hi) = (from.min(to), from.max(to));
        if hi - lo != gone || s.get(..lo).zip(s.get(hi..)) != now.split_at_checked(lo) {
            return None;
        }
        let mut at = from;
        for m in 1..=max_missed_presses(s[lo..hi].graphemes(true).count()) {
            at = step(s, at);
            if at == to {
                let key = KeyEvent::new(run.key.0, KeyModifiers::ALT);
                return named("Edit: ", key).map(|action| (m, action));
            }
        }
        None
    }

    fn start_trip(&self) -> Option<Trip> {
        let Mode::Picker(kind @ (PickerKind::Files | PickerKind::Search | PickerKind::Symbols)) =
            self.mode
        else {
            return None;
        };
        let reach = HISTORY_STEPS_OFFERED_EITHER_WAY as isize;
        let stops = (1..=reach)
            .flat_map(|k| [-k, k])
            .filter_map(|k| {
                let i = self.hist_idx.checked_add_signed(k)?;
                self.history.get(i).map(|(p, ..)| (k, p.clone()))
            })
            .collect();
        let next = self
            .review
            .as_ref()
            .filter(|_| self.hunk_ahead(1).is_none())
            .and_then(|r| {
                let back = self.hunk_left(r).map(|(rel, _)| rel);
                back.or_else(|| {
                    let f = self
                        .ahead(r, 1)
                        .into_iter()
                        .find(|f| f.is_stop(&self.root, self.diagrams.on()));
                    f.map(|f| f.path.clone())
                })
            });
        Some(Trip {
            kind,
            n: 1,
            opened_at: (self.buf.path.clone()?, self.at()),
            opened_at_rel: self.rel_current(),
            word_at_open: (!self.previewing()).then(|| self.usage_word()).flatten(),
            last_query: String::new(),
            history_files_by_step: stops,
            file_c_goes_on_to: next,
        })
    }

    fn judge_trip(&mut self, trip: Trip) {
        let budget = max_missed_presses(trip.n);
        let Some(path) = self.buf.path.clone() else {
            return;
        };
        if budget == 0 || (&path, self.at()) == (&trip.opened_at.0, trip.opened_at.1) {
            return;
        }
        let found = match trip.kind {
            PickerKind::Files => self.to_file(&trip, &path),
            _ => self.to_word(&trip),
        };
        if let Some((cost, key)) = found
            && cost <= budget
            && let Some(action) = crate::stats::action(key)
        {
            *self.missed.entry(action).or_default() += 1;
        }
    }

    fn to_file(&self, trip: &Trip, path: &Path) -> Option<(usize, &'static str)> {
        if trip.file_c_goes_on_to.is_some() && trip.file_c_goes_on_to == self.rel_current() {
            return Some((1, "c"));
        }
        if *path == trip.opened_at.0 {
            return None;
        }
        trip.history_files_by_step
            .iter()
            .filter(|(_, p)| p == path)
            .map(|&(k, _)| (k.unsigned_abs(), if k < 0 { "[" } else { "]" }))
            .min_by_key(|&(cost, _)| cost)
    }

    fn to_word(&self, trip: &Trip) -> Option<(usize, &'static str)> {
        let word = (trip.word_at_open.as_ref()).filter(|w| {
            super::usages::bare_name(w).to_lowercase() == trip.last_query.to_lowercase()
        })?;
        let landed = self.rel_current()?;
        let hits = self.usage_hits(word, trip.opened_at_rel.as_deref()).ranked;
        let row = hits
            .iter()
            .position(|(_, h)| h.path == landed && h.place() == self.at())?;
        let from_base = matches!(trip.opened_at.1, TextLine::Deleted(..))
            || (self.review.as_ref())
                .zip(trip.opened_at_rel.as_deref())
                .and_then(|(r, here)| r.file(here))
                .is_some_and(|f| f.status == 'D');
        match hits[row].0 {
            Tier::Declaration if !from_base && hits[row].1.deleted.is_none() => Some((1, "d")),
            _ if trip.kind == PickerKind::Search => Some((row + 2, "u")),
            _ => None,
        }
    }

    fn spot(&self) -> Spot {
        Spot {
            line: self.line,
            col: self.col,
            deleted: self.deleted,
            want_x: self.want_x,
            anchor: self.anchor,
            top: (self.top_line, self.top_row),
            left: self.left,
            center: self.center,
            row: self.picker.as_ref().map(|p| p.selected),
        }
    }

    fn put(&mut self, s: &Spot) {
        (self.line, self.col, self.deleted) = (s.line, s.col, s.deleted);
        (self.want_x, self.anchor) = (s.want_x, s.anchor);
        (self.top_line, self.top_row) = s.top;
        (self.left, self.center) = (s.left, s.center);
        if let (Some(p), Some(row)) = (&mut self.picker, s.row) {
            p.selected = row;
        }
    }

    fn end(&self) -> End {
        match &self.picker {
            Some(p) => ((TextLine::File(p.selected), 0), None),
            None => ((self.at(), self.col), self.selection().and(self.anchor)),
        }
    }
}
