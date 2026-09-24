//! Missed keys (#210): a key that would have reached the same end — the same place, or the same
//! picked result — in at most half the presses spent, saving at least three. Noted in
//! [`App::missed`] next to the presses, for the drill; real work only, nothing on screen (#112).

use super::cursor::{word_end, word_start};
use super::keys::named;
use super::*;

/// A run of one key counts only while every gap is shorter: a held or fast-tapped key, not
/// reading line by line.
const RUN_GAP: Duration = Duration::from_millis(200);
/// The missed key takes at most one press in this many of those spent,
const SHARE: usize = 2;
/// and saves at least this many.
const SAVED: usize = 3;
/// The steps of the jump history `[` / `]` are offered for, either way.
const REACH: usize = 3;

/// The keys a run of arrows or Shift+arrows may have missed. A tie goes to the first.
const SHORTCUTS: [(KeyCode, KeyModifiers); 17] = [
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
    (KeyCode::Char('v'), KeyModifiers::NONE),
];
/// And a run of arrows in a picker.
const PAGES: [(KeyCode, KeyModifiers); 2] = [
    (KeyCode::PageUp, KeyModifiers::NONE),
    (KeyCode::PageDown, KeyModifiers::NONE),
];

/// What the keys so far left open.
#[derive(Default)]
pub(super) struct Watch {
    /// When the previous key came.
    last: Option<Instant>,
    run: Option<Run>,
    trip: Option<Trip>,
}

/// Presses of one key, each within [`RUN_GAP`] of the one before.
struct Run {
    key: (KeyCode, KeyModifiers),
    mode: Mode,
    path: Option<PathBuf>,
    lines: usize,
    /// Where things stood before the first press.
    from: Spot,
    /// Backspace or Delete: the line before the first press.
    text: String,
    /// Where the last press left things, and how many presses moved them: Down on the last line
    /// is no press spent on the way.
    at: End,
    moved: usize,
}

/// A picker `s`, `D` or `o` opened from navigation, and the presses since, its own included.
struct Trip {
    kind: PickerKind,
    n: usize,
    /// The file and 0-based line it was opened on, and the file relative to the root.
    from: (PathBuf, usize),
    here: Option<PathBuf>,
    /// The word under the cursor there.
    word: Option<String>,
    /// The query as the last key found it.
    query: String,
    /// `o`: the file of every stop of the jump history up to [`REACH`] steps away, by step.
    stops: Vec<(isize, PathBuf)>,
    /// `o` in review past the last hunk: the file `c` goes on to.
    next: Option<PathBuf>,
}

/// All a moving key changes: the cursor, the selection's anchor, the view, a picker's row.
#[derive(Clone)]
struct Spot {
    line: usize,
    col: usize,
    want_x: usize,
    anchor: Option<(usize, usize)>,
    top: (usize, usize),
    left: usize,
    center: bool,
    row: Option<usize>,
}

/// Where a run ends: the cursor and a selection's anchor, or a picker's row.
type End = ((usize, usize), Option<(usize, usize)>);

/// The most presses a missed key may take when `spent` were: at most one in [`SHARE`], saving
/// at least [`SAVED`]; 0 when no key can.
fn budget(spent: usize) -> usize {
    (spent / SHARE).min(spent.saturating_sub(SAVED))
}

impl App {
    /// Before a key: a run it does not go on with is over and judged, and it may start one.
    pub(super) fn watch_before(&mut self, key: KeyEvent, at: Instant) {
        let fast = self
            .watch
            .last
            .is_some_and(|t| at.saturating_duration_since(t) < RUN_GAP);
        self.watch.last = Some(at);
        let query = self.picker.as_ref().map(|p| p.query.to_string());
        if let Some(trip) = &mut self.watch.trip {
            trip.n += 1;
            trip.query = query.unwrap_or_default();
        }
        let end = self.end();
        if let Some(run) = &mut self.watch.run {
            if run.at != end {
                (run.at, run.moved) = (end, run.moved + 1);
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

    /// After a key: a picker it opened from navigation starts a trip, one it closed with Enter
    /// ends it.
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

    /// `s` jumped on an Enter that came before its grep's answer.
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
            from: self.spot(),
            text: if deleting {
                self.line_str().to_string()
            } else {
                String::new()
            },
            at: self.end(),
            moved: 0,
        })
    }

    fn judge_run(&mut self, run: Run) {
        let from = (run.from.line, run.from.col);
        // The file changed under the run, or went: there is no same end to reach.
        let deleting = !run.text.is_empty();
        if run.mode != self.mode
            || run.path != self.buf.path
            || run.from.row.is_some() != self.picker.is_some()
            || !deleting && (run.lines != self.buf.lines.len() || self.clamp_pos(from) != from)
        {
            return;
        }
        let found = if deleting {
            self.word_chord(&run)
        } else {
            let budget = budget(run.moved);
            if budget == 0 {
                return;
            }
            // `c` / `C` cost one press: nothing is cheaper, so they go first.
            self.hunk_reached(&run)
                .or_else(|| self.shortcut(&run, budget))
        };
        if let Some((_, action)) = found {
            *self.missed.entry(action).or_default() += 1;
        }
    }

    /// Review: a run of plain arrows or paging from outside the next / previous hunk into it
    /// was `c` / `C`.
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
        let (from, to) = (run.from.line, self.line);
        let (&start, key) = match to.cmp(&from) {
            std::cmp::Ordering::Greater => (hunks.iter().find(|&&h| h > from)?, "c"),
            std::cmp::Ordering::Less => (hunks.iter().rev().find(|&&h| h < from)?, "C"),
            std::cmp::Ordering::Equal => return None,
        };
        let mut end = start;
        while self.diff.marks.contains_key(&(end + 1)) && !hunks.contains(&(end + 1)) {
            end += 1;
        }
        let hunk = start..=end;
        (hunk.contains(&to) && !hunk.contains(&from)).then_some((1, key))
    }

    /// A run of arrows or Shift+arrows: the cheapest key of [`SHORTCUTS`] — [`PAGES`] in a
    /// picker — that, pressed and pressed again, then followed by the run's arrow or the
    /// opposite one, ends where the run did within `budget`. Each is tried from where the run
    /// started through the key table itself; everything a try moves is put back.
    // ponytail: the tries grow with the square of the run: a 3 s hold is judged in about 10 ms,
    // a 10 s one in 60 ms, on the key after it. Bound a Left / Right walk by the chars left to
    // go on the line, not only the lines, if a long hold ever lags.
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
        let (scope, keys) = match run.from.row {
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
            // In edit mode a key with neither Ctrl nor Alt types: no `{`, `}`, `v`.
            if run.mode == Mode::Edit && matches!(kc, KeyCode::Char(_)) && km.is_empty() {
                continue;
            }
            let key = KeyEvent::new(kc, km);
            let limit = best.map_or(budget, |(cost, _)| cost - 1);
            if let Some(cost) = self.presses(&run.from, &end, key, arrows, limit)
                && let Some(action) = named(scope, key)
            {
                best = Some((cost, action));
            }
        }
        self.put(&now);
        (self.history, self.hist_idx, self.message) = kept;
        self.action = None;
        best
    }

    /// The fewest presses of `key`, then of one of `arrows`, that take things from `from` to
    /// `end`, if no more than `limit`. A key that moves nothing is no way there.
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
                    let apart = self.end().0.0.abs_diff(end.0.0);
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

    /// One press of a try, as the key table and the next frame take it.
    fn press(&mut self, key: KeyEvent) {
        self.key_inner(key);
        self.clamp_scroll();
    }

    /// Edit mode: a run of Backspace or Delete that stayed on its line was Alt+Backspace /
    /// Alt+Delete when the word chord, pressed and pressed again, takes exactly the same chars.
    /// Arrows delete nothing, so none follow. The presses spent are the chars taken: Backspace
    /// at the start of the file takes none.
    fn word_chord(&self, run: &Run) -> Option<(usize, &'static str)> {
        if self.line != run.from.line || self.buf.lines.len() != run.lines {
            return None;
        }
        let (s, now, from) = (run.text.as_str(), self.line_str(), run.from.col);
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
        for m in 1..=budget(s[lo..hi].graphemes(true).count()) {
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
        let reach = REACH as isize;
        let stops = (1..=reach)
            .flat_map(|k| [-k, k])
            .filter_map(|k| {
                let i = self.hist_idx.checked_add_signed(k)?;
                self.history.get(i).map(|(p, ..)| (k, p.clone()))
            })
            .collect();
        // `c` goes on to the next file once no hunk is left below.
        let next = self
            .review
            .as_ref()
            .filter(|_| !self.diff.hunks.iter().any(|&h| h > self.line))
            .and_then(|r| self.ahead(r, 1).into_iter().find(|f| f.has_hunks()))
            .map(|f| f.path.clone());
        Some(Trip {
            kind,
            n: 1,
            from: (self.buf.path.clone()?, self.line),
            here: self.rel_current(),
            word: self.word_under(search::word_chars(self.kind(), false)),
            query: String::new(),
            stops,
            next,
        })
    }

    fn judge_trip(&mut self, trip: Trip) {
        let budget = budget(trip.n);
        let Some(path) = self.buf.path.clone() else {
            return;
        };
        if budget == 0 || (&path, self.line) == (&trip.from.0, trip.from.1) {
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

    /// `o` to the file `c` goes on to, or to the file of a history stop.
    fn to_file(&self, trip: &Trip, path: &Path) -> Option<(usize, &'static str)> {
        if trip.next.is_some() && trip.next == self.rel_current() {
            return Some((1, "c"));
        }
        if *path == trip.from.0 {
            return None;
        }
        trip.stops
            .iter()
            .filter(|(_, p)| p == path)
            .map(|&(k, _)| (k.unsigned_abs(), if k < 0 { "[" } else { "]" }))
            .min_by_key(|&(cost, _)| cost)
    }

    /// `s` or `D` for the word under the cursor: on a declaration `u` marks, `d`; on another
    /// line `u` lists, after `s`, `u`, the arrows to its row and Enter.
    fn to_word(&self, trip: &Trip) -> Option<(usize, &'static str)> {
        let word = trip
            .word
            .as_ref()
            .filter(|w| w.to_lowercase() == trip.query.to_lowercase())?;
        let landed = self.rel_current()?;
        let hits = self.usage_hits(word, trip.here.as_deref());
        let row = hits
            .iter()
            .position(|(_, h)| h.path == landed && h.line == self.line + 1)?;
        match hits[row].0 {
            Tier::Declaration => Some((1, "d")),
            _ if trip.kind == PickerKind::Search => Some((row + 2, "u")),
            _ => None,
        }
    }

    fn spot(&self) -> Spot {
        Spot {
            line: self.line,
            col: self.col,
            want_x: self.want_x,
            anchor: self.anchor,
            top: (self.top_line, self.top_row),
            left: self.left,
            center: self.center,
            row: self.picker.as_ref().map(|p| p.selected),
        }
    }

    fn put(&mut self, s: &Spot) {
        (self.line, self.col, self.want_x, self.anchor) = (s.line, s.col, s.want_x, s.anchor);
        (self.top_line, self.top_row) = s.top;
        (self.left, self.center) = (s.left, s.center);
        if let (Some(p), Some(row)) = (&mut self.picker, s.row) {
            p.selected = row;
        }
    }

    fn end(&self) -> End {
        match &self.picker {
            Some(p) => ((p.selected, 0), None),
            None => ((self.line, self.col), self.selection().and(self.anchor)),
        }
    }
}
