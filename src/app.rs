//! All editor state and every key binding. Rendering lives in `ui.rs`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use regex::{Regex, RegexBuilder};

use crate::buffer::{self, Buffer};
use crate::git;
use crate::line_edit::LineEdit;
use crate::picker::{Pick, PickItem, Picker};
use crate::search::{self, Candidate, Hit, Kind, Reason};
use crate::tree::Tree;
use crate::tutor::{self, Tutor};
use crate::wrap;

/// Longest hit text kept in a picker label; the rest is off the screen anyway.
const MAX_LABEL_TEXT: usize = 120;
/// Longest symbol name the symbol list pads to.
const MAX_NAME_PAD: usize = 40;

/// Every binding, in the order the `?` overlay and the README table list them. Single source of
/// truth: a test checks that the README says exactly this.
pub const KEYS: &[(&str, &str)] = &[
    ("o / Ctrl+E", "Open a file (fuzzy)"),
    ("/ / Ctrl+F", "Find in the open file"),
    ("n / N", "Next / previous match"),
    ("s", "Search the project"),
    (
        "d / F12",
        "Go to definition of the word under the cursor, or its implementations",
    ),
    ("D", "Project symbols (fuzzy)"),
    ("u / Shift+F12", "Usages of the word under the cursor"),
    ("[ / ]", "Back / forward in the jump history"),
    ("c / C", "Review: next / previous hunk, on to the next file"),
    (": / Ctrl+G", "Go to line"),
    ("t", "Show or hide the file tree"),
    ("T", "Pick a theme (live preview)"),
    ("Tab", "Switch focus between tree and code"),
    ("Enter", "Edit at the cursor (Esc returns to navigation)"),
    (
        "Ctrl+S",
        "Save now (edits are saved on their own after a pause)",
    ),
    ("Ctrl+R", "Reload from disk, dropping unsaved edits"),
    ("Ctrl+Z / Ctrl+Y", "Undo / redo"),
    ("Ctrl+C", "Copy the selection (with none, quit)"),
    (
        "Edit: Ctrl+C / Ctrl+X",
        "Copy / cut the selection, or the line, to the clipboard",
    ),
    ("Arrows", "Move the cursor; Up / Down go by screen row"),
    (
        "Shift+Up / Shift+Down",
        "Extend the selection by a screen row",
    ),
    ("Shift+Left / Shift+Right", "Extend the selection by a char"),
    ("Alt+Left / Alt+Right", "Move one word"),
    ("Alt+Shift+Left / Right", "Extend the selection by a word"),
    (
        "Ctrl+Shift+Left / Right",
        "Extend the selection to the start / end of the screen row, then of the line",
    ),
    ("v", "Select the word, then the line, then the paragraph"),
    ("Ctrl+D / Ctrl+U", "Move half a screen down / up"),
    ("{ / }", "Previous / next paragraph (blank line)"),
    ("PgUp / PgDn", "Move one screen"),
    (
        "Home / End",
        "Start / end of the screen row, then of the line",
    ),
    ("Ctrl+Home / Ctrl+End", "Start / end of the file"),
    (
        "Esc",
        "Close an overlay, leave edit mode, or clear selection and find",
    ),
    ("?", "This help"),
    ("q / Ctrl+C", "Quit"),
    ("Tree: Up / Down", "Move"),
    ("Tree: Enter", "Open the file, or expand the directory"),
    ("Tree: Left / Right", "Collapse / expand"),
    ("Picker: Up / Down, Ctrl+P / Ctrl+N", "Move"),
    ("Picker: Enter", "Accept"),
    ("Picker: Esc", "Cancel"),
    ("Picker: PgUp / PgDn", "Move one page"),
    ("Help: Up / Down", "Scroll"),
];

/// Which picker is open, and what its overlay is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerKind {
    Files,
    Search,
    Definitions,
    Usages,
    Symbols,
    Themes,
}

impl PickerKind {
    fn title(self) -> &'static str {
        match self {
            Self::Files => "Files",
            Self::Search => "Search",
            Self::Definitions => "Definitions",
            Self::Usages => "Usages",
            Self::Symbols => "Symbols",
            Self::Themes => "Themes",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    /// `/`: incremental find in the open file.
    Find,
    /// `s`: the project-search query.
    Search,
    Goto,
    Picker(PickerKind),
    /// `?`: the list of bindings, over everything else.
    Help,
    /// Enter: typing changes the file. Letters insert; the chord aliases still navigate.
    Edit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Tree,
    Code,
}

/// A plain cursor move closer than this many lines to the current stop updates it instead of
/// adding a new one. VS Code's `TEXT_EDITOR_SELECTION_THRESHOLD`.
const HIST_NEAR: usize = 10;
const HIST_MAX: usize = 50;

pub struct App {
    pub root: PathBuf,
    pub buf: Buffer,
    pub tree: Tree,
    /// Every file under the root, sorted like the tree; the file picker's item list.
    pub files: Vec<PathBuf>,
    /// Per kind, the standard library and dependency roots outside the project and the files of
    /// that kind under them; filled the first time `d` leaves the project.
    external: HashMap<Kind, (Vec<PathBuf>, Arc<Vec<PathBuf>>)>,
    pub focus: Focus,
    pub show_tree: bool,
    /// First visible row of the tree pane, clamped by `ui`.
    pub tree_top: usize,
    pub picker: Option<Picker>,
    /// Stops of the jump history, oldest first; `hist_idx` is the current one and follows
    /// the cursor (see `hist_note`).
    pub history: Vec<(PathBuf, usize, usize)>,
    pub hist_idx: usize,
    /// Wakes the event loop when nucleo has new results. Set by `main`.
    pub wake: Arc<dyn Fn() + Send + Sync>,
    /// Cursor: file line, byte offset into that line, and the display column Up/Down aims for.
    pub line: usize,
    pub col: usize,
    pub want_x: usize,
    /// (line, col) where the selection started; it runs from here to the cursor.
    anchor: Option<(usize, usize)>,
    /// Top of the viewport: a file line plus which wrapped row of it is first on screen.
    pub top_line: usize,
    pub top_row: usize,
    pub mode: Mode,
    /// What has been typed into the `:`, `/` or `s>` prompt.
    pub prompt: LineEdit,
    /// The current query, kept for `n`/`N` and for painting the matches.
    pub find_re: Option<Regex>,
    /// Where the cursor was when `/` was pressed: the start of the incremental search.
    find_anchor: (usize, usize),
    /// The selection anchor, set aside while `/` moves the cursor; Esc puts it back.
    find_sel: Option<(usize, usize)>,
    pub message: String,
    /// Code text area, in cells, written by `ui::draw` before every frame.
    pub view_w: usize,
    pub view_h: usize,
    center: bool,
    /// `--tutor` only: the running tutorial. `None` in a normal session.
    pub tutor: Option<Tutor>,
    /// First row of the `?` overlay, clamped by `ui`.
    pub help_top: usize,
    /// Set by `main` when no file watcher could be started: auto-reload is off.
    pub no_watch: bool,
    /// The buffer has edits the disk does not.
    pub dirty: bool,
    /// The file changed on disk under unsaved edits: autosave is off until Ctrl+S overwrites
    /// or Ctrl+R reloads. VS Code's `files.saveConflictResolution: askUser`, with keys.
    pub conflict: bool,
    /// When the last edit was made; autosave fires `autosave` after it.
    last_edit: Option<Instant>,
    pub autosave: Duration,
    /// Linear per-file undo history, oldest first, and what undo took back.
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    /// Set when the next edit must start its own undo step even if it continues the last one.
    undo_break: bool,
    /// Text for the system clipboard, taken by `main` and sent to the terminal (OSC 52).
    pub clipboard: Option<String>,
    /// Lines that differ from the git index, painted in the gutter. Refreshed by `main` after
    /// every load, save and reload; between an edit and its autosave they lag by a second.
    /// In review mode: against the branch's base, with ghosts, taken on open (see `refresh_diff`).
    pub diff: git::Diff,
    pub want_diff: bool,
    /// `--review`: the branch under review. The tree pane then lists its files.
    pub review: Option<git::Review>,
    /// The theme in use, by name. Set by `main`; the theme picker previews others over it.
    pub theme: String,
    /// Where Enter in the theme picker saves the choice. Set by `main`; `None` saves nothing.
    pub config: Option<PathBuf>,
    /// A quit was just refused over edits that could not be saved: quitting again right away
    /// leaves them behind.
    quit_again: bool,
}

/// One undoable change: `old` lines from `line` on became `new`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Edit {
    line: usize,
    old: Vec<String>,
    new: Vec<String>,
    /// Cursor before and after, for undo and redo to put it back.
    before: (usize, usize),
    after: (usize, usize),
}

impl App {
    pub fn new(
        root: PathBuf,
        tree: Tree,
        files: Vec<PathBuf>,
        buf: Buffer,
        line: Option<usize>,
    ) -> Self {
        let focus = if buf.path.is_some() {
            Focus::Code
        } else {
            Focus::Tree
        };
        let mut app = Self {
            root,
            buf,
            tree,
            files,
            external: HashMap::new(),
            focus,
            show_tree: true,
            tree_top: 0,
            picker: None,
            history: Vec::new(),
            hist_idx: 0,
            wake: Arc::new(|| {}),
            line: 0,
            col: 0,
            want_x: 0,
            anchor: None,
            top_line: 0,
            top_row: 0,
            mode: Mode::Normal,
            prompt: LineEdit::default(),
            find_re: None,
            find_anchor: (0, 0),
            find_sel: None,
            message: String::new(),
            view_w: 80,
            view_h: 24,
            center: false,
            tutor: None,
            help_top: 0,
            no_watch: false,
            dirty: false,
            conflict: false,
            last_edit: None,
            autosave: Duration::from_secs(1),
            undo: Vec::new(),
            redo: Vec::new(),
            undo_break: false,
            clipboard: None,
            diff: git::Diff::default(),
            want_diff: true,
            review: None,
            theme: crate::theme::DEFAULT.to_string(),
            config: None,
            quit_again: false,
        };
        if let Some(n) = line {
            app.goto_line(n);
        }
        if let Some(path) = app.buf.path.clone() {
            app.reveal(&path);
        }
        app.hist_note(true);
        app
    }

    /// Path shown in the status bar: relative to the project root when it is below it.
    pub fn rel_path(&self) -> String {
        let Some(path) = &self.buf.path else {
            return format!("{}/", self.root_name());
        };
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .display()
            .to_string()
    }

    pub fn root_name(&self) -> String {
        self.root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.root.display().to_string())
    }

    pub fn line_str(&self) -> &str {
        &self.buf.lines[self.line]
    }

    /// Wrapped rows of file line `l` at the current viewport width, over the text `ui` draws.
    pub fn rows(&self, l: usize) -> Vec<std::ops::Range<usize>> {
        wrap::wrap_line(self.buf.shown(l), self.view_w)
    }

    /// Screen rows of line `l`: its ghosts (review mode, one row each, drawn above the text)
    /// and then its wrapped rows. A `(line, row)` pair counts rows from the first ghost.
    pub fn row_count(&self, l: usize) -> usize {
        self.diff.ghost_n(l) + self.rows(l).len()
    }

    pub fn cursor_row(&self) -> usize {
        self.diff.ghost_n(self.line) + wrap::col_to_row(&self.rows(self.line), self.col)
    }

    /// Display column of the cursor on its wrapped row, counting the indent rows after the first
    /// are drawn with.
    pub fn cursor_x(&self) -> usize {
        let rows = self.rows(self.line);
        let row = wrap::col_to_row(&rows, self.col);
        let indent = match row {
            0 => 0,
            _ => wrap::indent(self.buf.shown(self.line), self.view_w),
        };
        indent + wrap::width(&self.line_str()[wrap::row_to_col(&rows, row)..self.col])
    }

    /// 1-based display column, for the status bar.
    pub fn display_col(&self) -> usize {
        wrap::width(&self.line_str()[..self.col]) + 1
    }

    // ---- scrolling -------------------------------------------------------

    /// Scrolls the minimum amount that puts the cursor back on screen.
    pub fn clamp_scroll(&mut self) {
        if std::mem::take(&mut self.center) {
            self.center_cursor();
            return;
        }
        // The top is on the cursor line's own ghosts: they are being read, the cursor waits
        // below the pane (`move_rows` scrolls them in one at a time).
        if self.top_line == self.line && self.top_row < self.diff.ghost_n(self.line) {
            return;
        }
        let cur = (self.line, self.cursor_row());
        if cur < (self.top_line, self.top_row) {
            // Moving up shows the line's ghosts with it.
            (self.top_line, self.top_row) = (self.line, cur.1 - self.diff.ghost_n(self.line));
            return;
        }
        let top = self.back_rows(cur, self.view_h.saturating_sub(1));
        if (self.top_line, self.top_row) < top {
            (self.top_line, self.top_row) = top;
        }
    }

    fn center_cursor(&mut self) {
        let cur = (self.line, self.cursor_row());
        (self.top_line, self.top_row) = self.back_rows(cur, self.view_h / 2);
    }

    /// Walks `n` wrapped rows backwards from `(line, row)`, stopping at the top of the file.
    fn back_rows(&self, (mut line, mut row): (usize, usize), n: usize) -> (usize, usize) {
        for _ in 0..n {
            if row > 0 {
                row -= 1;
            } else if line > 0 {
                line -= 1;
                row = self.row_count(line) - 1;
            } else {
                break;
            }
        }
        (line, row)
    }

    // ---- cursor movement -------------------------------------------------

    /// Remembers the screen column the cursor stands on, for Up / Down to aim at.
    fn sync_want_x(&mut self) {
        self.want_x = self.cursor_x();
    }

    /// Places the cursor on screen row `row` of `self.line`, on the char under the column
    /// `want_x`, or as far right as the row goes. A row other than the last ends on its last
    /// char: its end is where the next row starts.
    fn apply_want_x(&mut self, row: usize) {
        let rows = self.rows(self.line);
        let row = row
            .saturating_sub(self.diff.ghost_n(self.line))
            .min(rows.len() - 1);
        let r = rows[row].clone();
        let s = &self.buf.lines[self.line];
        let mut col = if row + 1 < rows.len() {
            prev_char(s, r.end)
        } else {
            s.len()
        };
        let mut used = match row {
            0 => 0,
            _ => wrap::indent(self.buf.shown(self.line), self.view_w),
        };
        for (i, c) in s[r.start..r.end].char_indices() {
            if used >= self.want_x {
                col = r.start + i;
                break;
            }
            used += wrap::char_width(c);
        }
        self.col = col;
    }

    /// A stored `(line, col)` made valid for the buffer as it is now: the line clamped to the
    /// file, the col to the line and back onto a char boundary. Every position that outlives a
    /// reload — history stops, the find anchor, the selection anchor — is read through here.
    fn clamp_pos(&self, (line, col): (usize, usize)) -> (usize, usize) {
        let line = line.min(self.buf.lines.len() - 1);
        let s = &self.buf.lines[line];
        let mut col = col.min(s.len());
        while !s.is_char_boundary(col) {
            col -= 1;
        }
        (line, col)
    }

    /// The selection as ordered (line, col) ends: anchor and cursor, whichever comes first.
    /// None while the cursor stands on the anchor: an empty range selects nothing, so Ctrl+C
    /// copies the line and Backspace deletes a char, as with no selection. The anchor stays for
    /// the next Shift+move.
    pub fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let a = self.clamp_pos(self.anchor?);
        let b = (self.line, self.col);
        (a != b).then(|| (a.min(b), a.max(b)))
    }

    /// Bytes of line `l` inside the selection.
    pub fn selected_bytes(&self, l: usize) -> Option<std::ops::Range<usize>> {
        let (start, end) = self.selection()?;
        if l < start.0 || l > end.0 {
            return None;
        }
        let from = if l == start.0 { start.1 } else { 0 };
        let to = if l == end.0 {
            end.1
        } else {
            self.buf.lines[l].len()
        };
        Some(from..to)
    }

    /// The selected text, lines joined with `\n`.
    fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection()?;
        let lines: Vec<&str> = (start.0..=end.0)
            .map(|l| &self.buf.lines[l][self.selected_bytes(l).unwrap()])
            .collect();
        Some(lines.join("\n"))
    }

    /// Shift+move, VS Code style: the anchor is set where the first extending move started and
    /// the selection runs from there to wherever `mv` takes the cursor.
    fn extend(&mut self, mv: fn(&mut Self)) {
        self.anchor.get_or_insert((self.line, self.col));
        mv(self);
    }

    /// `v`: the selection grows to the word under the cursor (what `d` and `u` read), then the
    /// line, then the paragraph between blank lines. No state: the next step is the first of the
    /// three that is wider than what is selected, so a selection made by hand grows too.
    fn grow_selection(&mut self) {
        let blank = |l: usize| self.buf.lines[l].trim().is_empty();
        let l = self.line;
        let (mut top, mut bottom) = (l, l);
        while top > 0 && !blank(top - 1) {
            top -= 1;
        }
        while bottom + 1 < self.buf.lines.len() && !blank(bottom + 1) {
            bottom += 1;
        }
        // The run of word chars under the cursor or, at its end, just before it.
        let s = self.line_str();
        let (mut from, mut to) = (self.col, self.col);
        while from > 0 && is_word(char_at(s, prev_char(s, from))) {
            from = prev_char(s, from);
        }
        while to < s.len() && is_word(char_at(s, to)) {
            to = next_char(s, to);
        }
        let word = (from < to).then_some(from..to);
        let steps = [
            word.map(|r| ((l, r.start), (l, r.end))),
            Some(((l, 0), (l, self.line_str().len()))),
            Some(((top, 0), (bottom, self.buf.lines[bottom].len()))),
        ];
        let cur = self.selection().unwrap_or(((l, self.col), (l, self.col)));
        let wider = |&(from, to): &((usize, usize), (usize, usize))| {
            !blank(l) && from <= cur.0 && cur.1 <= to && (from, to) != cur
        };
        if let Some((from, to)) = steps.into_iter().flatten().find(wider) {
            self.anchor = Some(from);
            (self.line, self.col) = to;
            self.sync_want_x();
        }
    }

    /// Any move that does not extend the selection drops it, unless it ends where it started:
    /// arrows, jumps and find all decide it here, from where the cursor finally is.
    fn drop_selection_if_moved(&mut self, before: (usize, usize)) {
        if (self.line, self.col) != before {
            self.anchor = None;
        }
    }

    /// Home: the start of the screen row and, pressed there, of the line, as in VS Code.
    fn line_start(&mut self) {
        let rows = self.rows(self.line);
        let start = rows[wrap::col_to_row(&rows, self.col)].start;
        self.col = if self.col == start { 0 } else { start };
        self.sync_want_x();
    }

    /// End: the end of the screen row and, pressed there, of the line, as in VS Code. A row
    /// other than the last ends on its last char, since its end is where the next row starts.
    fn line_end(&mut self) {
        let rows = self.rows(self.line);
        let row = wrap::col_to_row(&rows, self.col);
        let len = self.line_str().len();
        let end = if row + 1 < rows.len() {
            prev_char(self.line_str(), rows[row].end)
        } else {
            len
        };
        self.col = if self.col == end { len } else { end };
        self.sync_want_x();
    }

    /// Up / Down, PgUp / PgDn: `n` screen rows, aiming at the column in `want_x`.
    fn move_rows(&mut self, n: isize) {
        // Up on a line whose ghosts are scrolled off above it brings them in, one row at a
        // time, so a deletion taller than the pane can still be read through.
        if n < 0
            && self.top_line == self.line
            && (1..=self.diff.ghost_n(self.line)).contains(&self.top_row)
        {
            self.top_row -= 1;
            return;
        }
        let cur = (self.line, self.cursor_row());
        let (mut line, mut row) = if n < 0 {
            self.back_rows(cur, n.unsigned_abs())
        } else {
            self.forward_rows(cur, n as usize)
        };
        // Ghost rows are not for the cursor: going up onto one lands on the text above it.
        if n < 0 && row < self.diff.ghost_n(line) {
            if line > 0 {
                line -= 1;
                row = self.row_count(line) - 1;
            } else {
                row = self.diff.ghost_n(line);
            }
        }
        self.line = line;
        self.apply_want_x(row);
    }

    /// Ctrl+D / Ctrl+U: cursor and viewport both move half a screen, like vim and less,
    /// so the cursor keeps its place on screen and half the context stays visible.
    fn half_page(&mut self, dir: isize) {
        let half = (self.view_h / 2).max(1);
        let before = (self.line, self.cursor_row());
        self.move_rows(dir * half as isize);
        let moved = self.rows_between(before, (self.line, self.cursor_row()));
        let top = (self.top_line, self.top_row);
        (self.top_line, self.top_row) = if dir < 0 {
            self.back_rows(top, moved)
        } else {
            self.forward_rows(top, moved)
        };
    }

    /// `{` / `}`: the previous / next blank line, like vim. A run of blank lines counts once, so
    /// from a blank line the jump crosses the next paragraph instead of stopping next door.
    /// In code, blank lines separate functions, so this is "next function" without a parser.
    fn paragraph(&mut self, dir: isize) {
        let blank = |l: &String| l.trim().is_empty();
        let last = self.buf.lines.len() - 1;
        let step = |l: usize| l.saturating_add_signed(dir).min(last);
        let mut l = self.line;
        while l != step(l) && blank(&self.buf.lines[l]) {
            l = step(l);
        }
        while l != step(l) && !blank(&self.buf.lines[l]) {
            l = step(l);
        }
        self.line = l;
        self.apply_want_x(0);
    }

    /// Wrapped rows from `a` to `b` (either order).
    fn rows_between(&self, a: (usize, usize), b: (usize, usize)) -> usize {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let mut n = 0;
        let mut cur = lo;
        while cur < hi {
            cur = self.forward_rows(cur, 1);
            n += 1;
        }
        n
    }

    /// Walks `n` wrapped rows forwards from `(line, row)`, stopping at the end of the file.
    fn forward_rows(&self, (mut line, mut row): (usize, usize), n: usize) -> (usize, usize) {
        for _ in 0..n {
            if row + 1 < self.row_count(line) {
                row += 1;
            } else if line + 1 < self.buf.lines.len() {
                line += 1;
                row = 0;
            } else {
                break;
            }
        }
        (line, row)
    }

    fn left(&mut self) {
        if self.col > 0 {
            self.col = prev_char(self.line_str(), self.col);
        } else if self.line > 0 {
            self.line -= 1;
            self.col = self.line_str().len();
        }
        self.sync_want_x();
    }

    fn right(&mut self) {
        if self.col < self.line_str().len() {
            self.col = next_char(self.line_str(), self.col);
        } else if self.line + 1 < self.buf.lines.len() {
            self.line += 1;
            self.col = 0;
        }
        self.sync_want_x();
    }

    fn word_right(&mut self) {
        let s = self.line_str();
        if self.col >= s.len() {
            if self.line + 1 < self.buf.lines.len() {
                self.line += 1;
                self.col = 0;
            }
        } else {
            let mut i = self.col;
            while i < s.len() && !is_word(char_at(s, i)) {
                i = next_char(s, i);
            }
            while i < s.len() && is_word(char_at(s, i)) {
                i = next_char(s, i);
            }
            self.col = i;
        }
        self.sync_want_x();
    }

    fn word_left(&mut self) {
        if self.col == 0 {
            if self.line > 0 {
                self.line -= 1;
                self.col = self.line_str().len();
            }
        } else {
            let s = self.line_str();
            let mut i = prev_char(s, self.col);
            while i > 0 && !is_word(char_at(s, i)) {
                i = prev_char(s, i);
            }
            while i > 0 && is_word(char_at(s, prev_char(s, i))) {
                i = prev_char(s, i);
            }
            self.col = i;
        }
        self.sync_want_x();
    }

    /// Jumps to a 1-based line number, clamped to the file, and centers the view.
    pub fn goto_line(&mut self, n: usize) {
        self.line = n.max(1).min(self.buf.lines.len()) - 1;
        self.col = 0;
        self.want_x = 0;
        self.center = true;
    }

    // ---- opening files and jump history ----------------------------------

    /// Puts the tree cursor on `path` (absolute) and expands everything above it.
    fn reveal(&mut self, path: &Path) {
        if let Ok(rel) = path.strip_prefix(&self.root) {
            self.tree.reveal(rel);
        }
    }

    /// Shows `path` at `line` (1-based). Reloading is skipped when the file is already open, so
    /// this doubles as a plain cursor move. Line 0 is "no line in particular": the start of a
    /// file being opened, and the cursor where it is on the file that is already open, like
    /// VS Code's explorer.
    /// Returns `false`, with the reason in the status bar, when the file cannot be read or the
    /// open one has edits that could not be saved.
    fn open(&mut self, path: &Path, line: usize) -> bool {
        let same = self.buf.path.as_deref() == Some(path);
        if !same {
            // Replacing the buffer would drop edits the disk does not have; the status bar
            // already says why they are not there (a conflict, a failed save).
            if !self.flush() {
                return false;
            }
            match self.load(path) {
                Ok(mut buf) => {
                    // The standard library and dependencies are read here, never edited. A
                    // `.venv` or `node_modules` sits inside the root, so the roots decide, but
                    // not over a file the project walk listed: an editable install puts the
                    // project's own `src` on `sys.path`.
                    let listed = path
                        .strip_prefix(&self.root)
                        .is_ok_and(|rel| self.files.iter().any(|f| f == rel));
                    let external = !path.starts_with(&self.root)
                        || !listed
                            && self
                                .external
                                .values()
                                .any(|(roots, _)| roots.iter().any(|r| path.starts_with(r)));
                    if external {
                        buf.readonly.get_or_insert("outside the project");
                    }
                    self.buf = buf;
                    self.anchor = None;
                    self.dirty = false;
                    self.conflict = false;
                    self.undo.clear();
                    self.redo.clear();
                    if self.mode == Mode::Edit {
                        self.mode = Mode::Normal;
                    }
                    (self.line, self.col, self.want_x) = (0, 0, 0);
                    (self.top_line, self.top_row) = (0, 0);
                    // After the viewport is reset: the diff clamps it against the new file.
                    self.diff = git::Diff::default();
                    self.refresh_diff();
                }
                Err(e) => {
                    self.message = format!("{e:#}");
                    return false;
                }
            }
        }
        if !(same && line == 0) {
            self.goto_line(line.max(1));
        }
        self.reveal(path);
        true
    }

    /// The file from disk; in review mode a file the branch deleted comes from the base,
    /// read-only.
    fn load(&self, path: &Path) -> anyhow::Result<Buffer> {
        if let Some(r) = &self.review
            && let Ok(rel) = path.strip_prefix(&self.root)
            && r.file(rel).is_some_and(|f| f.status == 'D')
        {
            let mut buf = Buffer::from_bytes(path.to_path_buf(), &r.base_bytes(&self.root, rel)?);
            buf.readonly = Some("deleted in this branch");
            return Ok(buf);
        }
        Buffer::load(path)
    }

    /// Asks `main` for new marks; the old ones stay on screen until they arrive. In review
    /// mode they are taken here and now: `c` needs the hunks of a file the moment it opens, and
    /// one file against one commit is quick.
    fn refresh_diff(&mut self) {
        let Some(r) = &self.review else {
            self.want_diff = true;
            return;
        };
        let Some(path) = self.buf.path.clone() else {
            return;
        };
        self.want_diff = false;
        let rel = path.strip_prefix(&self.root).ok();
        let file = rel.and_then(|rel| r.file(rel));
        self.diff = match file {
            Some(f) if f.status == 'D' => git::Diff {
                // A deleted file is all ghost: every line marked, nothing to step through.
                marks: (0..self.buf.lines.len())
                    .map(|l| (l, git::Mark::DeletedBelow))
                    .collect(),
                ..Default::default()
            },
            _ => git::diff(
                &self.root,
                &path,
                Some(&r.merge_base),
                file.and_then(|f| f.old.as_deref()),
            ),
        };
        // Ghosts change how many rows a line has; the viewport must not point past them.
        self.top_line = self.top_line.min(self.buf.lines.len() - 1);
        self.top_row = self.top_row.min(self.row_count(self.top_line) - 1);
    }

    /// Enters review mode on a freshly built app: marks against the base, the cursor on the
    /// first hunk of the open file (unless a line was asked for).
    pub fn start_review(&mut self, review: git::Review) {
        self.review = Some(review);
        self.refresh_diff();
        if let Some(path) = self.buf.path.clone() {
            self.reveal(&path);
            if self.line == 0
                && let Some(&h) = self.diff.hunks.first()
            {
                self.goto_line(h + 1);
                self.hist_note(true);
            }
        }
    }

    /// `c` / `C`: the next / previous hunk, crossing into the next file of the review.
    fn hunk(&mut self, dir: isize) {
        let Some(r) = self.review.clone() else {
            self.message = "not in review mode: merl --review".into();
            return;
        };
        let here = if dir > 0 {
            self.diff.hunks.iter().find(|&&h| h > self.line)
        } else {
            self.diff.hunks.iter().rev().find(|&&h| h < self.line)
        };
        if let Some(&h) = here {
            let path = self.buf.path.clone().unwrap();
            self.jump_to(&path, h + 1);
            self.center = true;
            return;
        }
        let at = self
            .rel_current()
            .and_then(|rel| r.files.iter().position(|f| f.path == rel));
        let next = match (at, dir > 0) {
            (Some(i), true) => i + 1,
            (Some(i), false) => i.wrapping_sub(1),
            (None, true) => 0,
            (None, false) => r.files.len().wrapping_sub(1),
        };
        let Some(f) = r.files.get(next) else {
            self.message = if dir > 0 {
                "last hunk of the review".into()
            } else {
                "first hunk of the review".into()
            };
            return;
        };
        self.open_review_file(f, dir < 0);
    }

    /// Opens a file of the review on its first (or `last`) hunk. The hunks are read before
    /// the file opens, so this is one stop in the history.
    fn open_review_file(&mut self, f: &git::ReviewFile, last: bool) {
        let Some(r) = &self.review else { return };
        let path = self.root.join(&f.path);
        let hunks = match f.status {
            'D' => Vec::new(),
            _ => git::diff(&self.root, &path, Some(&r.merge_base), f.old.as_deref()).hunks,
        };
        let h = if last { hunks.last() } else { hunks.first() };
        self.jump_to(&path, h.map_or(1, |h| h + 1));
        self.center = true;
    }

    /// `hunk 2/5 · file 1/3` for the status bar.
    pub fn review_status(&self) -> Option<String> {
        let r = self.review.as_ref()?;
        let file = self
            .rel_current()
            .and_then(|rel| r.files.iter().position(|f| f.path == rel))
            .map_or("-".to_string(), |i| (i + 1).to_string());
        let hunk = self.diff.hunks.iter().filter(|&&h| h <= self.line).count();
        Some(format!(
            "hunk {hunk}/{}  file {file}/{}",
            self.diff.hunks.len(),
            r.files.len()
        ))
    }

    /// Re-reads the open file after it changed on disk. Cursor, scroll, history and find pattern
    /// survive; the cursor is clamped to whatever the file is now. merl's own saves are
    /// recognised and ignored; a change under unsaved edits is a conflict, not a reload,
    /// unless `force` (Ctrl+R) says the edits go.
    pub fn reload(&mut self, force: bool) {
        let Some(path) = self.buf.path.clone() else {
            return;
        };
        // Mid-save the file can be briefly gone; the rename that follows sends another event
        // and overwrites the message. Gone for good, the message stays.
        let Ok(bytes) = std::fs::read(&path) else {
            self.message = "file gone".into();
            return;
        };
        if !force {
            if buffer::hash(&bytes) == self.buf.disk {
                return;
            }
            if self.dirty {
                self.conflict = true;
                return;
            }
        }
        self.buf = Buffer::from_bytes(path, &bytes);
        self.dirty = false;
        self.conflict = false;
        self.last_edit = None;
        self.undo.clear();
        self.redo.clear();
        self.refresh_diff();
        let last = self.buf.lines.len() - 1;
        (self.line, self.col) = self.clamp_pos((self.line, self.col));
        self.top_line = self.top_line.min(last);
        self.top_row = self.top_row.min(self.row_count(self.top_line) - 1);
        self.sync_want_x();
        self.clamp_scroll();
        self.message = "reloaded".into();
    }

    fn pos(&self) -> Option<(PathBuf, usize, usize)> {
        self.buf.path.clone().map(|p| (p, self.line, self.col))
    }

    /// Records where the cursor is now, VS Code style: the current stop always tracks the
    /// cursor. A plain move within `HIST_NEAR` lines just updates it; a farther move, another
    /// file, or a `jump` (go to definition, `:`, find) becomes a new stop and drops the
    /// forward history. Paging (see `key_inner`) only updates it. Standing on the current stop
    /// records nothing, so walking with `[` / `]` is silent.
    fn hist_note(&mut self, jump: bool) {
        let Some(pos) = self.pos() else {
            return;
        };
        if let Some(cur) = self.history.get(self.hist_idx) {
            if *cur == pos {
                return;
            }
            if !jump && cur.0 == pos.0 && cur.1.abs_diff(pos.1) < HIST_NEAR {
                self.history[self.hist_idx] = pos;
                return;
            }
            self.history.truncate(self.hist_idx + 1);
        }
        self.history.push(pos);
        if self.history.len() > HIST_MAX {
            self.history.remove(0);
        }
        self.hist_idx = self.history.len() - 1;
    }

    /// Opens `path` at `line` and makes it a stop in the jump history. `:` and the pickers jump
    /// from outside `key_inner`'s move rule, so a jump that lands elsewhere drops the selection
    /// here.
    pub fn jump_to(&mut self, path: &Path, line: usize) {
        let before = (self.line, self.col);
        if self.open(path, line) {
            self.focus = Focus::Code;
        }
        self.drop_selection_if_moved(before);
        self.hist_note(true);
    }

    /// `[` and `]`: walks the recorded stops.
    fn hist_go(&mut self, delta: isize) {
        let next = self
            .hist_idx
            .checked_add_signed(delta)
            .filter(|i| *i < self.history.len());
        let Some(i) = next else {
            self.message = if delta < 0 {
                "start of history".into()
            } else {
                "end of history".into()
            };
            return;
        };
        let (path, line, col) = self.history[i].clone();
        if !self.open(&path, line + 1) {
            return;
        }
        self.hist_idx = i;
        self.focus = Focus::Code;
        (self.line, self.col) = self.clamp_pos((self.line, col));
        self.sync_want_x();
        // The stop follows the file: after a reload shortened it, this is where `[` lands,
        // and `hist_note` must not read the clamp as a move that drops the forward history.
        self.history[i] = (path, self.line, self.col);
    }

    // ---- pickers ---------------------------------------------------------

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
        self.picker = Some(Picker::new(
            PickerKind::Files.title(),
            items,
            true,
            self.wake.clone(),
        ));
        self.mode = Mode::Picker(PickerKind::Files);
    }

    pub(crate) fn show_picker(&mut self, kind: PickerKind, items: Vec<PickItem>) {
        // The grep stops at MAX_HITS in file order: say so, or a missing hit looks absent.
        let title = if items.len() >= search::MAX_HITS {
            format!("{} (first {})", kind.title(), search::MAX_HITS)
        } else {
            kind.title().to_string()
        };
        self.picker = Some(Picker::new(title, items, false, self.wake.clone()));
        self.mode = Mode::Picker(kind);
    }

    /// `T`: every theme, the built-ins then the user's own, with the cursor on the one in use.
    fn open_themes_picker(&mut self) {
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

    fn picker_key(&mut self, key: KeyEvent) {
        let Some(picker) = &mut self.picker else {
            return;
        };
        match picker.key(key) {
            Pick::Stay => return,
            Pick::Cancel => {}
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

    // ---- find in file ----------------------------------------------------

    /// Starts an incremental search from the current cursor position. The selection is set aside
    /// meanwhile, so it does not stretch to every match the cursor visits.
    fn start_find(&mut self) {
        self.mode = Mode::Find;
        self.prompt.clear();
        self.find_anchor = (self.line, self.col);
        self.find_sel = self.anchor.take();
    }

    fn find_key(&mut self, key: KeyEvent) {
        match key.code {
            // Enter keeps both the position and the pattern, so `n` carries on from here. The
            // selection comes back unless the search moved the cursor.
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                self.anchor = self.find_sel.take();
                self.drop_selection_if_moved(self.clamp_pos(self.find_anchor));
                self.hist_note(true);
            }
            // Esc puts back both the cursor and the selection.
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                (self.line, self.col) = self.clamp_pos(self.find_anchor);
                self.anchor = self.find_sel.take();
                self.sync_want_x();
            }
            _ => {
                if self.prompt.key(key) {
                    self.refresh_find();
                }
            }
        }
    }

    /// Recompiles the query and moves to the first match at or after the anchor.
    /// The query is literal text with smart case, as in VS Code: `migrator(` hits `Migrator()`.
    fn refresh_find(&mut self) {
        if self.prompt.is_empty() {
            // Nothing to match: drop the previous pattern so its highlights go with it,
            // and put the cursor back where the search started.
            self.find_re = None;
            let (l, c) = self.clamp_pos(self.find_anchor);
            self.go_to_match(l, c);
            return;
        }
        let insensitive = !self.prompt.chars().any(char::is_uppercase);
        let re = RegexBuilder::new(&regex::escape(&self.prompt))
            .case_insensitive(insensitive)
            .build()
            .expect("an escaped literal always compiles");
        let (l, c) = self.find_anchor;
        let hit = self
            .match_at_or_after(&re, l, c)
            .or_else(|| self.match_at_or_after(&re, 0, 0));
        if let Some((l, c)) = hit {
            self.go_to_match(l, c);
        }
        self.find_re = Some(re);
    }

    /// `n` / `N`: the next or previous match, wrapping around the file.
    fn step_find(&mut self, forward: bool) {
        let Some(re) = self.find_re.clone() else {
            self.message = "no pattern".into();
            return;
        };
        let last = self.buf.lines.len() - 1;
        let found = if forward {
            let (l, c) = self.after_cursor();
            self.match_at_or_after(&re, l, c)
                .map(|p| (p, false))
                .or_else(|| self.match_at_or_after(&re, 0, 0).map(|p| (p, true)))
        } else {
            self.match_before(&re, self.line, self.col)
                .map(|p| (p, false))
                .or_else(|| self.match_before(&re, last, usize::MAX).map(|p| (p, true)))
        };
        match found {
            Some(((l, c), wrapped)) => {
                self.go_to_match(l, c);
                if wrapped {
                    self.message = "wrapped".into();
                }
            }
            None => self.message = "no match".into(),
        }
    }

    /// First match starting at or after `(line, col)`, searching down the file.
    fn match_at_or_after(&self, re: &Regex, line: usize, col: usize) -> Option<(usize, usize)> {
        for (l, text) in self.buf.lines.iter().enumerate().skip(line) {
            let from = if l == line { col.min(text.len()) } else { 0 };
            if let Some(m) = re.find_at(text, from) {
                return Some((l, m.start()));
            }
        }
        None
    }

    /// Last match starting strictly before `(line, col)`, searching up the file.
    fn match_before(&self, re: &Regex, line: usize, col: usize) -> Option<(usize, usize)> {
        for l in (0..=line.min(self.buf.lines.len() - 1)).rev() {
            let text = &self.buf.lines[l];
            let limit = if l == line { col } else { usize::MAX };
            if let Some(m) = re.find_iter(text).take_while(|m| m.start() < limit).last() {
                return Some((l, m.start()));
            }
        }
        None
    }

    /// One char past the cursor, so `n` cannot land on the match it is already sitting on.
    fn after_cursor(&self) -> (usize, usize) {
        let s = self.line_str();
        if self.col < s.len() {
            (self.line, next_char(s, self.col))
        } else if self.line + 1 < self.buf.lines.len() {
            (self.line + 1, 0)
        } else {
            (self.line, s.len())
        }
    }

    fn go_to_match(&mut self, line: usize, col: usize) {
        self.line = line;
        self.col = col;
        self.sync_want_x();
        self.center = true;
    }

    // ---- project search, definitions, symbols, usages ---------------------

    /// The open file's path relative to the root, for sorting and labels.
    fn rel_current(&self) -> Option<PathBuf> {
        let path = self.buf.path.as_ref()?;
        Some(path.strip_prefix(&self.root).unwrap_or(path).to_path_buf())
    }

    /// Greps the project files `wanted` accepts. The open file is searched whenever it is
    /// wanted, even when the startup walk skipped it (ignored).
    fn grep(
        &self,
        pattern: &str,
        whole_word: bool,
        smart_case: bool,
        wanted: impl Fn(&Path) -> bool,
    ) -> anyhow::Result<Vec<Hit>> {
        let mut files: Vec<PathBuf> = self.files.iter().filter(|p| wanted(p)).cloned().collect();
        let current = self.rel_current();
        if let Some(cur) = &current
            && wanted(cur)
            && !files.contains(cur)
        {
            files.push(cur.clone());
        }
        // With unsaved edits the open file is searched as it is on screen, so a hit's line is a
        // line of the buffer the jump lands in.
        let unsaved = self.dirty.then(|| self.buf.to_bytes());
        search::grep_project(
            &self.root,
            &files,
            pattern,
            whole_word,
            smart_case,
            current.as_deref(),
            unsaved.as_deref(),
        )
    }

    /// The word under the cursor, with `extra` characters counting as part of it.
    fn word_under(&self, extra: &str) -> Option<String> {
        search::word_at(self.line_str(), self.col, extra).map(|(_, w)| w.to_string())
    }

    fn kind(&self) -> Option<Kind> {
        self.buf.path.as_deref().and_then(search::kind_of)
    }

    /// `rel/path:line: text` rows for a result picker. The text keeps its tabs, as `Buffer`
    /// does, so `ui` can line it up with the file's highlighting; tabs are expanded when drawn.
    pub(crate) fn hit_items(hits: Vec<Hit>) -> Vec<PickItem> {
        hits.into_iter()
            .map(|h| {
                let label = format!("{}:{}: ", h.path.display(), h.line);
                let code_at = Some(label.len());
                PickItem {
                    label: label + &clip(h.text.trim(), MAX_LABEL_TEXT),
                    path: h.path,
                    line: h.line,
                    code_at,
                }
            })
            .collect()
    }

    /// Enter in the `s>` prompt: a smart-case search for the query as typed, over every file.
    /// Literal like `/`: `foo(` finds the calls and the definition, not a regex error.
    fn run_search(&mut self) {
        let query = self.prompt.take();
        self.mode = Mode::Normal;
        if query.is_empty() {
            return;
        }
        let hits = self
            .grep(&regex::escape(&query), false, true, |_| true)
            .expect("an escaped literal always compiles");
        if hits.is_empty() {
            self.message = format!("no results for {query}");
            return;
        }
        self.show_picker(PickerKind::Search, Self::hit_items(hits));
    }

    /// `d` / F12. A file of a known [`Kind`] gets its declaration patterns. A word or a qualifier
    /// an import binds is looked for in the module the import names: the project's file or
    /// package (`from app.repos import X` in `app/repos.py`, `store.Open` in the `store`
    /// directory), else the standard library and the installed dependencies
    /// ([`search::external_roots`]) narrowed to that module (`np.array` in `numpy`). Everything
    /// else, and a project module that does not declare the word, is searched in the project where
    /// such a definition can live (`.tsx` finds `.ts`, a Terraform variable stays in its module),
    /// and nothing there sends the same patterns outside it.
    ///
    /// `x.word` on a value — a `.` qualifier no import binds, other than a bare `self` — is a
    /// member whose type is not known: every member declaration of the name, in the project and
    /// outside it, is a candidate. The status line says how the target was found, and a single
    /// candidate found only by name says so too. A field, a variant or a parameter has no
    /// declaration the rules know and gets "no definition": `u` lists the uses.
    fn goto_definition(&mut self) {
        let kind = self.kind();
        let extra = search::word_chars(kind, true);
        let Some((range, word)) = search::word_at(self.line_str(), self.col, extra) else {
            return;
        };
        let word = word.to_owned();
        let before = &self.line_str()[..range.start];
        let dotted = before.ends_with('.') && !before.ends_with("..");
        let chain = search::qualifier(self.line_str(), range.start);
        let here = self.rel_current();
        let (Some(kind), Some(here)) = (kind, here) else {
            self.message = format!("no definition for {word}");
            return;
        };
        let text = self.buf.lines.join("\n");
        let mut imports = search::imports(kind, &text);
        // A parameter or a local of the same name hides the import where the cursor is: `json`
        // in `def handler(json)` is a value, and `via import` would be a proof of nothing.
        let first = chain.first().map_or(word.as_str(), String::as_str);
        let locals: Vec<usize> = search::bindings(kind, &text, self.line + 1, first)
            .iter()
            .map(|b| b.line)
            .filter(|&n| {
                // An import is what the name hides, and a class, a function or a namespace of
                // that name is no value: `Outer.Inner` reads a declaration, not a member.
                let t = self.buf.lines[n - 1].trim_start();
                let t = t.strip_prefix("export ").unwrap_or(t);
                let t = t.strip_prefix("abstract ").unwrap_or(t);
                let declares = [
                    "class ",
                    "def ",
                    "async def ",
                    "function ",
                    "func ",
                    "type ",
                    "interface ",
                    "namespace ",
                    "enum ",
                ]
                .iter()
                .filter_map(|k| t.strip_prefix(k))
                .any(|rest| {
                    rest.strip_prefix(first)
                        .is_some_and(|after| !after.starts_with(is_word))
                });
                !declares && !t.starts_with("import ") && !t.starts_with("from ")
            })
            .collect();
        if !locals.is_empty() {
            imports.retain(|(name, _)| name != first);
        }
        // The word itself is that parameter or local: its declarations in this scope are the
        // answer, and a function of the same name elsewhere is not.
        if !dotted && !locals.is_empty() && locals != [self.line + 1] {
            let found = locals
                .iter()
                .map(|&line| Candidate {
                    hit: Hit {
                        path: here.clone(),
                        line,
                        text: self.buf.lines[line - 1].clone(),
                    },
                    reason: Reason::Local,
                })
                .collect();
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        let own = matches!(chain.as_slice(), [s] if s == "self" || s == "cls" || s == "this");
        let on_value = dotted && !own && chain.first().is_none_or(|f| bound(&imports, f).is_none());
        let members = on_value
            .then(|| search::member_patterns(kind, &word))
            .flatten()
            .map(|m| m.join("|"));
        let patterns = search::def_patterns(kind, &word);
        if patterns.is_empty() {
            self.message = format!("no definition for {word}");
            return;
        }
        let pattern = patterns.join("|");
        // On the declaration of a member of an interface, a protocol, an abstract or a base
        // class, `d` offers what implements it (#68, step 6).
        if !dotted {
            let found = self.implementations(kind, &here, &text, &word);
            if !found.is_empty() {
                self.show_definitions(kind, &word, &here, found, None);
                return;
            }
        }
        // A receiver whose type is proven narrows the member to that type (#68, steps 2 to 4).
        let mut broke = None;
        if dotted
            && matches!(kind, Kind::Python | Kind::TsJs | Kind::Go)
            && chain.first().is_some_and(|f| bound(&imports, f).is_none())
        {
            match self.typed_definitions(kind, &here, &word, &chain) {
                Ok(found) if !found.is_empty() => {
                    self.show_definitions(kind, &word, &here, found, None);
                    return;
                }
                Ok(_) => {}
                // With one name in front of the word, `by name` already says where.
                Err(at) => broke = (chain.len() > 1).then_some(at),
            }
        }
        // An import names where the word is declared: the project's module, else the one outside
        // it. Only a module of the project that does not declare it (a re-export) leaves the word
        // to the search by name.
        let import = (!on_value)
            .then(|| {
                bound(
                    &imports,
                    chain.first().map_or(word.as_str(), String::as_str),
                )
            })
            .flatten();
        let mut outside = false;
        let mut found = match import {
            Some(path) => {
                let mut found = self
                    .imported_definitions(kind, &here, &word, &chain, &path)
                    .unwrap_or_else(|| {
                        outside = true;
                        self.external_definitions(kind, &word, &chain, dotted, &pattern, &imports)
                    });
                // `try: from a import pick` / `except ImportError: from b import pick` names
                // two sources: both are offered, and which one ran is not for `d` to guess.
                let others: Vec<Vec<String>> = imports
                    .iter()
                    .filter(|(name, other)| name == first && *other != path)
                    .map(|(_, other)| other.clone())
                    .collect();
                for other in others {
                    let more = self
                        .imported_definitions(kind, &here, &word, &chain, &other)
                        .unwrap_or_default();
                    found.extend(more);
                }
                found
            }
            None => Vec::new(),
        };
        if !found.is_empty() {
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // A member in the project first. A qualifier no import names can still be a class, a
        // namespace or a module of the project, which declares the word at its top level.
        // A qualifier that is no value and no import can be what declares the word: a namespace,
        // a class with a static member, a nested class. A declaration that reads `Outer.find`
        // is the answer then, and a method `find` of some other class is not.
        if on_value && locals.is_empty() && !chain.is_empty() {
            let full = format!("{}.{word}", chain.join("."));
            let named: Vec<Candidate> = self
                .project_definitions(kind, &here, &word, &pattern)
                .into_iter()
                .filter(|h| {
                    self.text_of(&h.path)
                        .and_then(|t| search::qualified(kind, &t, h.line, &word))
                        .is_some_and(|q| q == full || q.ends_with(&format!(".{full}")))
                })
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Path(chain.join(".")),
                })
                .collect();
            if !named.is_empty() {
                self.show_definitions(kind, &word, &here, named, None);
                return;
            }
        }
        // A parameter or a local in front of the word is a value for certain: it has members,
        // and a function or a variable at the top of a module is not one of them.
        let hits = members
            .as_ref()
            .map(|m| self.project_definitions(kind, &here, &word, m))
            .filter(|hits| !hits.is_empty() || !locals.is_empty())
            .unwrap_or_else(|| self.project_definitions(kind, &here, &word, &pattern));
        found = hits
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect();
        if let Some(members) = &members {
            // A value's type is unknown: its member may come from a dependency as well. Outside
            // the project TypeScript is read from its declaration files, as VS Code lands on
            // them: the `.d.ts` says what a type offers, the bundled JavaScript is noise.
            let all = self.external_files(kind);
            let files: Vec<PathBuf> = all
                .iter()
                .filter(|p| kind != Kind::TsJs || search::declaration_file(p))
                .cloned()
                .collect();
            let external = self.external_grep(kind, &files, members);
            let seen: Vec<PathBuf> = found.iter().map(|c| self.root.join(&c.hit.path)).collect();
            found.extend(
                external
                    .into_iter()
                    .filter(|h| !seen.contains(&h.path))
                    .map(|hit| Candidate {
                        hit,
                        reason: Reason::ByName,
                    }),
            );
        } else if found.is_empty()
            && !outside
            && !matches!(
                chain.first().map(String::as_str),
                Some("self" | "cls" | "this")
            )
        {
            found = self.external_definitions(kind, &word, &chain, dotted, &pattern, &imports);
        }
        self.show_definitions(kind, &word, &here, found, broke.as_deref());
    }

    /// Jumps to the one candidate, or opens the picker over several, and says how they were found
    /// and at which name of the chain in front of the word the typed lookup `broke`, if it did.
    fn show_definitions(
        &mut self,
        kind: Kind,
        word: &str,
        here: &Path,
        mut found: Vec<Candidate>,
        broke: Option<&str>,
    ) {
        // Standing on one of the definitions is not a reason to go nowhere. But what is left are
        // namesakes nothing ties to this one, so they are offered, never jumped to: a second `d`
        // after a proven jump would walk out of the type it has just proven (#68).
        // A line inside a raw string, a docstring or a block comment declares nothing. Past a
        // few hundred candidates the picker is a list to filter, and reading every file is not
        // worth what it would drop.
        if found.len() <= 500 {
            let mut literal: HashMap<PathBuf, Vec<bool>> = HashMap::new();
            found.retain(|c| {
                let lines = literal.entry(c.hit.path.clone()).or_insert_with(|| {
                    self.text_of(&c.hit.path)
                        .map_or_else(Vec::new, |t| search::literal_lines(kind, &t))
                });
                !lines.get(c.hit.line - 1).copied().unwrap_or(false)
            });
        }
        let all = found.len();
        if all > 1 {
            found.retain(|c| c.hit.line != self.line + 1 || c.hit.path != here);
        }
        let namesakes = found.len() < all && found.iter().all(|c| !c.reason.proven());
        // The project and the outside are each cut at MAX_HITS; the picker holds that many.
        found.truncate(search::MAX_HITS);
        match found.as_slice() {
            [] => self.message = resolution(word, None, &found, broke),
            [one] if !namesakes => {
                let path = self.root.join(&one.hit.path);
                let target = self
                    .text_of(&one.hit.path)
                    .and_then(|text| search::qualified(kind, &text, one.hit.line, word));
                let status = resolution(word, target.as_deref(), &found, broke);
                self.jump_to(&path, one.hit.line);
                // A refused jump (edits that cannot be saved) leaves its own reason, not a
                // resolution nobody followed.
                if self.buf.path.as_deref() == Some(path.as_path()) {
                    // The cursor lands on the word rather than at the start of the line, so a
                    // second `d` there asks the next question about the same name: what
                    // implements the declaration it just landed on (#68, step 6).
                    let text = self.line_str().to_owned();
                    let whole = |(i, _): &(usize, &str)| {
                        !text[..*i].ends_with(is_word)
                            && !text[i + word.len()..].starts_with(is_word)
                    };
                    if let Some((i, _)) = text.match_indices(word).find(whole) {
                        self.col = i;
                        self.sync_want_x();
                    }
                    self.message = status;
                }
            }
            _ => {
                let status = if namesakes {
                    let others = if found.len() == 1 { "other" } else { "others" };
                    format!("{word}: at a declaration, {} {others} by name", found.len())
                } else {
                    resolution(word, None, &found, broke)
                };
                let items = self.definition_items(kind, word, found);
                self.show_picker(PickerKind::Definitions, items);
                if let Some(p) = &mut self.picker {
                    p.title = p
                        .title
                        .replacen(PickerKind::Definitions.title(), &status, 1);
                }
                self.message = status;
            }
        }
    }

    /// The lines `pattern` matches where a definition of `word` in `here`, a file of `kind`, can
    /// live in the project. Escaped or built-in patterns always compile.
    fn project_definitions(&self, kind: Kind, here: &Path, word: &str, pattern: &str) -> Vec<Hit> {
        let mut hits = self
            .grep(pattern, false, false, |p| {
                search::in_def_scope(kind, here, p)
            })
            .unwrap_or_default();
        if let Some(block) = search::def_block(kind, word) {
            hits.retain(|h| {
                std::fs::read_to_string(self.root.join(&h.path))
                    .is_ok_and(|text| search::directly_inside(&text, h.line, block))
            });
        }
        hits
    }

    /// The declarations of `word` in the project module an import names, where `path` is what
    /// [`search::imports`] binds the word or its qualifier to; `None` when the module is not the
    /// project's. The word sits directly in the module, or in the class the chain goes through
    /// (`UserRepo.create`), as [`search::qualified`] names it: `store.Open` is never a method
    /// `Open`. An alias finds the imported name. A default import finds a declaration of its local
    /// name, else the module's `export default`. An empty list is a module that does not declare
    /// the word, a re-export: the search by name takes over.
    fn imported_definitions(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        chain: &[String],
        path: &[String],
    ) -> Option<Vec<Candidate>> {
        let module_files =
            |module: &[String]| search::module_files(kind, &self.root, &self.files, here, module);
        // The names from the module down to the word: what the import takes, then the chain.
        let tail = |mut names: Vec<String>| {
            if let Some((_, after)) = chain.split_first() {
                names.extend(after.iter().cloned());
                names.push(word.to_owned());
            }
            names
        };
        let (files, inside) = match kind {
            // `from a import b` takes a module or a name from `a`: the longest module that exists,
            // never shorter than the one the import names.
            Kind::Python => {
                let target = tail(path.to_vec());
                let floor = path.len().saturating_sub(1).max(1);
                (floor..target.len()).rev().find_map(|n| {
                    let files = module_files(&target[..n]);
                    (!files.is_empty()).then(|| (files, target[n..].to_vec()))
                })?
            }
            Kind::TsJs => {
                let (taken, module) = path.split_last()?;
                let files = module_files(module);
                if files.is_empty() {
                    return None;
                }
                let inside = match taken.as_str() {
                    "*" => tail(Vec::new()),
                    // What a default export is called is known only on its own line.
                    "default" if !chain.is_empty() => return Some(Vec::new()),
                    "default" => vec![word.to_owned()],
                    name => tail(vec![name.to_owned()]),
                };
                (files, inside)
            }
            Kind::Go => (module_files(path), tail(Vec::new())),
            // No module rules: the search by name, then outside the project, as before.
            _ => return Some(Vec::new()),
        };
        if files.is_empty() {
            return None;
        }
        let Some(name) = inside.last() else {
            return Some(Vec::new());
        };
        let wanted = |p: &Path| files.iter().any(|f| f == p);
        let pattern = search::def_patterns(kind, name).join("|");
        let mut hits = self
            .grep(&pattern, false, false, wanted)
            .unwrap_or_default();
        let within = (inside.len() > 1).then(|| inside.join("."));
        hits.retain(|h| {
            self.text_of(&h.path)
                .and_then(|text| search::qualified(kind, &text, h.line, name))
                == within
        });
        if hits.is_empty() && kind == Kind::TsJs && path.last().is_some_and(|t| t == "default") {
            hits = self
                .grep(r"^export\s+default\b", false, false, wanted)
                .unwrap_or_default();
        }
        // A file, or a Go package's directory.
        let label = |hit: &Hit| match (kind, hit.path.parent()) {
            (Kind::Go, Some(dir)) if dir != Path::new("") => format!("{}/", dir.display()),
            (Kind::Go, _) => "./".to_owned(),
            _ => hit.path.display().to_string(),
        };
        Some(
            hits.into_iter()
                .map(|hit| Candidate {
                    reason: Reason::Import(label(&hit)),
                    hit,
                })
                .collect(),
        )
    }

    /// The declarations of `word` in the type of the receiver `chain`, `x.f.g…`, when every link is
    /// proven: every declaration of `x` in scope reads the same type (through one call's return
    /// type at most), each field is declared in the type before it or one that type extends or
    /// embeds, and each type is declared once where the file that names it can see it
    /// ([`App::declaration`]). The member is looked for in the last type, then in the types it
    /// extends or embeds. `Err` names the first name that is not proven; an empty list is a type
    /// without the member. Both leave the word to the search by name.
    fn typed_definitions(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        chain: &[String],
    ) -> Result<Vec<Candidate>, String> {
        let text = self.buf.lines.join("\n");
        let (mut ty, call) = self
            .value_type(kind, here, &text, self.line + 1, &chain[0], 1)
            .ok_or_else(|| chain[0].clone())?;
        // What the status line lists: `repo: UserRepository`, or `self.uow: UnitOfWork` for the
        // receiver and its field, then `users: UserRepository` for each field after them.
        let mut links = vec![call.unwrap_or_else(|| format!("{}: {}", chain[0], ty.name))];
        for (i, field) in chain.iter().enumerate().skip(1) {
            // ponytail: six names in front of the word; a longer chain breaks at the seventh.
            if i == 6 {
                return Err(field.clone());
            }
            let (next, call) = self
                .hierarchy(kind, &ty, 0, &mut |t| self.field_type(kind, t, field))
                .flatten()
                .ok_or_else(|| field.clone())?;
            let name = match i {
                1 => format!("{}.{field}", chain[0]),
                _ => field.clone(),
            };
            let link = call.unwrap_or_else(|| format!("{name}: {}", next.name));
            if i == 1 {
                links[0] = link;
            } else {
                links.push(link);
            }
            ty = next;
        }
        let hits = self
            .hierarchy(kind, &ty, 0, &mut |t| {
                Some(self.members_of(kind, t, word)).filter(|hits| !hits.is_empty())
            })
            .unwrap_or_default();
        let label = links.join(" \u{2192} ");
        Ok(hits
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::Receiver(label.clone()),
            })
            .collect())
    }

    /// The type of `name` on 1-based `line` of `text`, the text of `file`: the one type all its
    /// declarations in scope read, with the signature of the call it came from, if any. `hops` is
    /// how many times a declaration may hand over to another name (`self.repo = repo`).
    fn value_type(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        line: usize,
        name: &str,
        hops: usize,
    ) -> Option<(Typed, Option<String>)> {
        let bindings = search::bindings(kind, text, line, name);
        self.agree(kind, file, text, &bindings, hops)
    }

    /// The field `field` as `ty` itself declares it: `None` when it does not, `Some(None)` when
    /// its declarations cannot be read or disagree.
    fn field_type(
        &self,
        kind: Kind,
        ty: &Typed,
        field: &str,
    ) -> Option<Option<(Typed, Option<String>)>> {
        let text = self.text_of(&ty.path)?;
        let bindings = search::field_bindings(kind, &text, ty.line, field);
        (!bindings.is_empty()).then(|| self.agree(kind, &ty.path, &text, &bindings, 1))
    }

    /// The one type every binding reads, or `None` when there is none, one cannot be read or
    /// two disagree.
    fn agree(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        bindings: &[search::Binding],
        hops: usize,
    ) -> Option<(Typed, Option<String>)> {
        let mut found: Option<(Typed, Option<String>)> = None;
        for b in bindings {
            let this = self.binding_type(kind, file, text, b, hops)?;
            match &found {
                Some((ty, _)) if (&ty.path, ty.line) != (&this.0.path, this.0.line) => return None,
                Some(_) => {}
                None => found = Some(this),
            }
        }
        found
    }

    /// The type one binding in `file` reads, and the signature of the call it came through.
    fn binding_type(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        b: &search::Binding,
        hops: usize,
    ) -> Option<(Typed, Option<String>)> {
        match &b.value {
            search::Value::Type(t) | search::Value::New(t) => {
                self.type_decl(kind, file, t).map(|ty| (ty, None))
            }
            search::Value::Class(line) => {
                let decl = text.lines().nth(line - 1)?;
                let name = decl
                    .split("class")
                    .nth(1)?
                    .trim_start()
                    .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
                    .next()
                    .filter(|n| !n.is_empty())?;
                let ty = Typed {
                    name: name.to_owned(),
                    path: file.to_path_buf(),
                    line: *line,
                };
                Some((ty, None))
            }
            search::Value::Call(callee) => self.call_type(kind, file, callee),
            search::Value::Name(n) if hops > 0 => {
                self.value_type(kind, file, text, b.line, n, hops - 1)
            }
            search::Value::Name(_) | search::Value::Unknown => None,
        }
    }

    /// What a call of `callee` in `file` gives: the class it constructs, or the declared return
    /// type of the function, resolved in the file declaring it. One hop: a return type is never
    /// followed through another call. The signature is spelled as the language writes it.
    fn call_type(&self, kind: Kind, file: &Path, callee: &str) -> Option<(Typed, Option<String>)> {
        let parts: Vec<String> = callee.split('.').map(str::to_owned).collect();
        let decl = self.declaration(kind, file, &parts)?;
        if search::declares_type(kind, &decl.text) {
            let ty = Typed {
                name: parts.last()?.clone(),
                path: decl.path,
                line: decl.line,
            };
            return Some((ty, None));
        }
        let text = self.text_of(&decl.path)?;
        let (written, signature) = match search::returns(kind, &text, decl.line)? {
            search::Value::New(t) => (t.clone(), format!("{callee}() returns new {t}()")),
            search::Value::Type(t) => {
                let signature = match kind {
                    Kind::Python => format!("{callee}() -> {t}"),
                    Kind::TsJs => format!("{callee}(): {t}"),
                    _ => format!("{callee}() {t}"),
                };
                (t, signature)
            }
            _ => return None,
        };
        let ty = self.type_decl(kind, &decl.path, &written)?;
        Some((ty, Some(signature)))
    }

    /// The declaration of the type written as `written` in `file`.
    fn type_decl(&self, kind: Kind, file: &Path, written: &str) -> Option<Typed> {
        let parts = search::type_path(kind, written)?;
        let decl = self.declaration(kind, file, &parts)?;
        search::declares_type(kind, &decl.text).then(|| Typed {
            name: parts.last().cloned().unwrap_or_default(),
            path: decl.path,
            line: decl.line,
        })
    }

    /// The one top-level declaration `parts` names as `file` sees it: in the file itself (for Go,
    /// its package), else in the project module an import binds the first part to. `None` when
    /// there is none or more than one, and for a module outside the project.
    fn declaration(&self, kind: Kind, file: &Path, parts: &[String]) -> Option<Hit> {
        let (name, chain) = parts.split_last()?;
        let one = |hits: Vec<Hit>| <[Hit; 1]>::try_from(hits).ok().map(|[hit]| hit);
        if chain.is_empty() {
            let own = self.package_files(kind, file);
            let pattern = search::def_patterns(kind, name).join("|");
            let hits: Vec<Hit> = self
                .grep(&pattern, false, false, |p| own.iter().any(|f| f == p))
                .unwrap_or_default()
                .into_iter()
                .filter(|h| {
                    self.text_of(&h.path)
                        .is_some_and(|text| search::qualified(kind, &text, h.line, name).is_none())
                })
                .collect();
            if !hits.is_empty() {
                return one(hits);
            }
        }
        let imports = search::imports(kind, &self.text_of(file)?);
        let path = bound(&imports, chain.first().unwrap_or(name))?;
        let found = self.imported_definitions(kind, file, name, chain, &path)?;
        one(found.into_iter().map(|c| c.hit).collect())
    }

    /// The files a name of `file` is declared in without an import: the file, or for Go every
    /// file of its package (a `_test.go` file only from a test).
    fn package_files(&self, kind: Kind, file: &Path) -> Vec<PathBuf> {
        let mut files = vec![file.to_path_buf()];
        if kind == Kind::Go {
            let test = |f: &Path| f.to_string_lossy().ends_with("_test.go");
            files.extend(
                self.files
                    .iter()
                    .filter(|f| f.parent() == file.parent() && f.as_path() != file)
                    .filter(|f| search::kind_of(f) == Some(Kind::Go) && (test(file) || !test(f)))
                    .cloned(),
            );
        }
        files
    }

    /// The declarations of `word` inside `ty` itself: its methods, Go's methods on it in its
    /// package and the method lines of a Go interface, named `Type.word` by
    /// [`search::qualified`].
    fn members_of(&self, kind: Kind, ty: &Typed, word: &str) -> Vec<Hit> {
        let Some(text) = self.text_of(&ty.path) else {
            return Vec::new();
        };
        let owner = search::qualified(kind, &text, ty.line, &ty.name).unwrap_or(ty.name.clone());
        let want = Some(format!("{owner}.{word}"));
        let patterns = search::member_or_signature(kind, word).unwrap_or_default();
        let files = self.package_files(kind, &ty.path);
        self.grep(&patterns.join("|"), false, false, |p| {
            files.iter().any(|f| f == p)
        })
        .unwrap_or_default()
        .into_iter()
        .filter(|h| {
            self.text_of(&h.path)
                .and_then(|text| search::qualified(kind, &text, h.line, word))
                == want
        })
        .collect()
    }

    /// #68 step 6. What implements the member the cursor stands on: the same member in the types
    /// that implement the interface, protocol, abstract or base class declaring it. Empty when
    /// the cursor is not on such a declaration, and when nothing implements it, and `d` then goes
    /// on as before.
    ///
    /// A Python class and a TypeScript class or interface implement another by naming it, so the
    /// subtypes are walked down from the type ([`Self::subtype_impls`]). A Go type and a Python
    /// `Protocol` implementer name nothing, so there the rule is structural: a member of the same
    /// name taking the same number of parameters, which is what Go's implicit interfaces and a
    /// protocol ask for. Only the project is searched: an interface is opened to find what this
    /// project does with it.
    fn implementations(&self, kind: Kind, here: &Path, text: &str, word: &str) -> Vec<Candidate> {
        if !matches!(kind, Kind::Python | Kind::TsJs | Kind::Go) {
            return Vec::new();
        }
        let line = self.line + 1;
        let declares = search::member_or_signature(kind, word)
            .and_then(|p| Regex::new(&p.join("|")).ok())
            .is_some_and(|re| re.is_match(self.line_str()));
        let Some(owner_line) = search::owner_decl(kind, text, line).filter(|_| declares) else {
            return Vec::new();
        };
        // `Notifier.send`, what the status line and every picker row name the member by.
        let Some(member) = search::qualified(kind, text, line, word) else {
            return Vec::new();
        };
        let owner = Typed {
            name: member.rsplit('.').nth(1).unwrap_or_default().to_owned(),
            path: here.to_path_buf(),
            line: owner_line,
        };
        let decl = text.lines().nth(owner_line - 1).unwrap_or_default();
        let structural = match kind {
            // A method beside its type is no interface method and has no implementations.
            Kind::Go => decl.contains("interface"),
            Kind::Python => is_protocol(text, owner_line),
            _ => false,
        };
        if owner.name.is_empty() || (kind == Kind::Go && !structural) {
            return Vec::new();
        }
        let hits = if structural {
            self.structural_impls(kind, here, word, text, line, owner_line)
        } else {
            self.subtype_impls(kind, here, word, &owner)
        };
        hits.into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::Implementation(member.clone()),
            })
            .collect()
    }

    /// The member `word` in the types that name `owner` as a base, and in the types that name
    /// those: one grep of the project per level. A hit counts only when its own `extends`,
    /// `implements` or Python bases name a type already found and its file can see that name —
    /// it declares it, or an import binds it — so a same-named class in another package is no
    /// subtype. A type that inherits the member without declaring it is no implementation.
    fn subtype_impls(&self, kind: Kind, here: &Path, word: &str, owner: &Typed) -> Vec<Hit> {
        let mut names = vec![owner.name.clone()];
        let mut types = vec![(owner.name.clone(), owner.path.clone())];
        let mut seen = vec![(owner.path.clone(), owner.line)];
        let mut out = Vec::new();
        // ponytail: four levels of subtypes, one grep each. Deeper hierarchies want an index.
        for _ in 0..4 {
            let Some(pattern) = search::subtype_patterns(kind, &names) else {
                break;
            };
            let hits = self
                .grep(&pattern, false, false, |p| {
                    search::in_def_scope(kind, here, p)
                })
                .unwrap_or_default();
            let mut next = Vec::new();
            let mut found = Vec::new();
            for hit in hits {
                let Some(text) = self.text_of(&hit.path) else {
                    continue;
                };
                // The clause the grep matched may be one line of a header wrapped over
                // several, and the type is declared on the first of them.
                let Some(decl) = search::type_decl_at(kind, &text, hit.line) else {
                    continue;
                };
                let name = text
                    .lines()
                    .nth(decl - 1)
                    .and_then(|l| search::type_name(kind, l));
                let Some(name) = name else {
                    continue;
                };
                if seen.contains(&(hit.path.clone(), decl)) {
                    continue;
                }
                let declares = |text: &str, name: &str| {
                    text.lines()
                        .any(|l| search::type_name(kind, l).as_deref() == Some(name))
                };
                // The import may name another type of the same name: a module of the project
                // that declares one, in a file none of the types found so far lives in. A
                // module that only hands the name on (a barrel) says nothing either way.
                let another = |first: &str, module: &[String]| {
                    let module = match kind {
                        Kind::TsJs => &module[..module.len().saturating_sub(1)],
                        _ => module,
                    };
                    (1..=module.len()).rev().any(|n| {
                        let files = search::module_files(
                            kind,
                            &self.root,
                            &self.files,
                            &hit.path,
                            &module[..n],
                        );
                        let ours = |f: &PathBuf| types.iter().any(|(t, p)| t == first && p == f);
                        !files.iter().any(ours)
                            && files
                                .iter()
                                .any(|f| self.text_of(f).is_some_and(|t| declares(&t, first)))
                    })
                };
                let sees = |first: &str| {
                    if declares(&text, first) {
                        // Its own type of that name is ours only in the file ours is in.
                        return !types.iter().any(|(t, _)| t == first)
                            || types.iter().any(|(t, p)| t == first && *p == hit.path);
                    }
                    bound(&search::imports(kind, &text), first)
                        .is_some_and(|module| !another(first, &module))
                };
                let derives = search::bases(kind, &text, hit.line)
                    .into_iter()
                    .chain(search::interfaces(kind, &text, hit.line))
                    .filter_map(|b| search::type_path(kind, &b))
                    .any(|p| p.last().is_some_and(|n| names.contains(n)) && sees(&p[0]));
                if !derives {
                    continue;
                }
                seen.push((hit.path.clone(), decl));
                found.push((name.clone(), hit.path.clone()));
                next.push(name);
                if let Some(at) = search::member_decl(kind, &text, decl, word) {
                    out.push(Hit {
                        text: text.lines().nth(at - 1).unwrap_or_default().to_owned(),
                        path: hit.path,
                        line: at,
                    });
                }
            }
            if next.is_empty() || out.len() >= search::MAX_HITS {
                break;
            }
            names = next;
            types = found;
        }
        out
    }

    /// Every member of `word` the project declares with as many parameters as the one on `line`,
    /// outside the type declaring it: what implements a Go interface or a Python protocol, since
    /// an implementer of either names nothing. A Python protocol is answered by classes, so
    /// another protocol declaring the same member is not one of them.
    fn structural_impls(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        text: &str,
        line: usize,
        owner_line: usize,
    ) -> Vec<Hit> {
        let want = search::params(kind, text, line);
        let signature = search::go_signature(text, line);
        // A Go interface's own method lines carry no receiver: only the methods answer.
        let patterns = match kind {
            Kind::Go => search::member_patterns(kind, word),
            _ => search::member_or_signature(kind, word),
        };
        let (Some(want), Some(patterns)) = (want, patterns) else {
            return Vec::new();
        };
        self.project_definitions(kind, here, word, &patterns.join("|"))
            .into_iter()
            .filter(|h| h.line != line || h.path != here)
            .filter(|h| {
                let Some(text) = self.text_of(&h.path) else {
                    return false;
                };
                if search::params(kind, &text, h.line) != Some(want) {
                    return false;
                }
                // Go writes its types: the same number of parameters of other types, or another
                // result, implements nothing. Where either cannot be read, the count stands.
                if kind == Kind::Go
                    && let (Some(a), Some(b)) = (&signature, search::go_signature(&text, h.line))
                    && *a != b
                {
                    return false;
                }
                kind != Kind::Python
                    || search::owner_decl(kind, &text, h.line)
                        .filter(|&d| d != owner_line || h.path != here)
                        .is_some_and(|d| !is_protocol(&text, d))
            })
            .collect()
    }

    /// `find` in `ty`, then in the types it extends or embeds, nearest first: the first answer.
    fn hierarchy<R>(
        &self,
        kind: Kind,
        ty: &Typed,
        depth: usize,
        find: &mut dyn FnMut(&Typed) -> Option<R>,
    ) -> Option<R> {
        if let Some(found) = find(ty) {
            return Some(found);
        }
        // ponytail: eight levels up, which also ends a cycle.
        if depth == 8 {
            return None;
        }
        let text = self.text_of(&ty.path)?;
        search::bases(kind, &text, ty.line)
            .iter()
            .filter_map(|base| self.type_decl(kind, &ty.path, base))
            .find_map(|base| self.hierarchy(kind, &base, depth + 1, &mut *find))
    }

    /// `pattern` over the standard library and dependencies of `kind`, in the module the file's
    /// imports bind `chain` (or `word` itself) to. The module path is relaxed from the end until
    /// files match: `from json import load` is `json/load`, then `json`. Relaxing past the
    /// module the import names (`github.com/foo/bar` down to `github.com`) is a search by name,
    /// and says so. A relative import names a project file, so nothing outside is searched. A
    /// qualifier no import binds is taken as the module path itself: `std::fs::read` spells one,
    /// while a `dotted` qualifier is a value whose name merely matches a module, found by name. A
    /// name nothing installed declares matches no file and the search stops. A bare word no
    /// import binds (`Vec`, `open`) searches every file, by name.
    fn external_definitions(
        &mut self,
        kind: Kind,
        word: &str,
        chain: &[String],
        dotted: bool,
        pattern: &str,
        imports: &[(String, Vec<String>)],
    ) -> Vec<Candidate> {
        let mut bound_path = bound(imports, chain.first().map_or(word, String::as_str));
        // What a TypeScript import takes (a name, `default`, `*`) is no part of a file's path.
        if kind == Kind::TsJs {
            bound_path.as_mut().map(Vec::pop);
        }
        let imported = bound_path.is_some();
        if bound_path
            .as_ref()
            .and_then(|p| p.first())
            .is_some_and(|p| p.starts_with('.'))
        {
            return Vec::new();
        }
        // The import's own item may go (`from json import load` is `json`), and so may whatever
        // the chain adds past it; the module itself may not.
        let floor = bound_path
            .as_ref()
            .map_or(chain.len(), Vec::len)
            .saturating_sub(1)
            .max(1);
        let mut module = match chain.first() {
            Some(first) => {
                let mut p = bound_path.unwrap_or_else(|| vec![first.clone()]);
                p.extend(chain[1..].iter().cloned());
                Some(p)
            }
            None => bound_path,
        };
        let all = self.external_files(kind);
        let mut files: Vec<PathBuf> = Vec::new();
        while let Some(m) = &mut module {
            files = all
                .iter()
                .filter(|p| search::in_module(p, m))
                .cloned()
                .collect();
            if !files.is_empty() {
                break;
            }
            m.pop();
            if m.is_empty() {
                // An import of something not installed: nothing outside says what it is.
                return Vec::new();
            }
        }
        let by_name = |hits: Vec<Hit>| -> Vec<Candidate> {
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::ByName,
                })
                .collect()
        };
        let Some(module) = module else {
            return by_name(self.external_grep(kind, &all, pattern));
        };
        // `from lib import pick` names something at the top of a module: a method called `pick`
        // is not it, however alone it stands (the real one may be native code).
        let top_level = imported && !dotted && matches!(kind, Kind::Python | Kind::TsJs | Kind::Go);
        let at_top = |this: &Self, mut hits: Vec<Hit>| {
            if top_level {
                hits.retain(|h| {
                    this.text_of(&h.path)
                        .is_some_and(|t| search::qualified(kind, &t, h.line, word).is_none())
                });
            }
            hits
        };
        let hits = at_top(self, self.external_grep(kind, &files, pattern));
        // An imported module that does not declare the name re-exports it (`std::sync::Arc`
        // lives in `alloc`, a package's `__init__` pulls from its submodules): look everywhere.
        if hits.is_empty() && imported {
            return by_name(at_top(self, self.external_grep(kind, &all, pattern)));
        }
        let sep = match kind {
            Kind::Python => ".",
            Kind::Rust => "::",
            _ => "/",
        };
        let reason = if module.len() < floor || (!imported && dotted) {
            Reason::ByName
        } else if imported {
            Reason::Import(module.join(sep))
        } else {
            Reason::Path(module.join(sep))
        };
        hits.into_iter()
            .map(|hit| Candidate {
                hit,
                reason: reason.clone(),
            })
            .collect()
    }

    /// `pattern` over `files` outside the project, standard library first. The paths are
    /// absolute: `root.join` leaves them alone, so a hit opens where it is.
    fn external_grep(&self, kind: Kind, files: &[PathBuf], pattern: &str) -> Vec<Hit> {
        let roots = self
            .external
            .get(&kind)
            .map(|(roots, _)| roots.as_slice())
            .unwrap_or_default();
        let mut hits = search::grep_project(&self.root, files, pattern, false, false, None, None)
            .unwrap_or_default();
        hits.sort_by_cached_key(|h| {
            (
                roots.iter().position(|r| h.path.starts_with(r)),
                h.path.clone(),
                h.line,
            )
        });
        hits
    }

    /// The files of `kind` outside the project, walked once per kind.
    ///
    /// ponytail: lives for the session, like the project walk. A `pip install` mid-session
    /// needs a restart, as a new project file does.
    fn external_files(&mut self, kind: Kind) -> Arc<Vec<PathBuf>> {
        self.external
            .entry(kind)
            .or_insert_with(|| {
                let roots = search::external_roots(kind, &self.root);
                let files = search::external_files(kind, &roots);
                (roots, Arc::new(files))
            })
            .1
            .clone()
    }

    /// The text of `path` as the search read it: the open file as it is on screen.
    fn text_of(&self, path: &Path) -> Option<String> {
        if self.rel_current().as_deref() == Some(path) {
            Some(self.buf.lines.join("\n"))
        } else {
            std::fs::read_to_string(self.root.join(path)).ok()
        }
    }

    /// Picker rows for `d`: the qualified name, the reason, then `path:line: code`, the columns
    /// padded so the names and reasons line up. An external hit is shown relative to the root it
    /// came from: `json/__init__.py:278:` rather than the whole path to the interpreter.
    fn definition_items(&self, kind: Kind, word: &str, found: Vec<Candidate>) -> Vec<PickItem> {
        let roots = self
            .external
            .get(&kind)
            .map(|(roots, _)| roots.as_slice())
            .unwrap_or_default();
        let mut texts: HashMap<PathBuf, Option<String>> = HashMap::new();
        let named: Vec<(String, String, Candidate)> = found
            .into_iter()
            .map(|c| {
                let text = texts
                    .entry(c.hit.path.clone())
                    .or_insert_with(|| self.text_of(&c.hit.path));
                let name = text
                    .as_deref()
                    .and_then(|text| search::qualified(kind, text, c.hit.line, word))
                    .unwrap_or_else(|| word.to_owned());
                (name, c.reason.to_string(), c)
            })
            .collect();
        let pad = |width: usize, s: &str| " ".repeat(width.saturating_sub(wrap::width(s)));
        let name_w = named
            .iter()
            .map(|(n, ..)| wrap::width(n))
            .max()
            .unwrap_or(0);
        let name_w = name_w.min(MAX_NAME_PAD);
        let why_w = named
            .iter()
            .map(|(_, r, _)| wrap::width(r))
            .max()
            .unwrap_or(0);
        named
            .into_iter()
            .map(|(name, why, c)| {
                let shown = roots
                    .iter()
                    .find_map(|r| c.hit.path.strip_prefix(r).ok())
                    .unwrap_or(&c.hit.path);
                let head = format!(
                    "{name}{}  {why}{}  {}:{}: ",
                    pad(name_w, &name),
                    pad(why_w, &why),
                    shown.display(),
                    c.hit.line
                );
                PickItem {
                    code_at: Some(head.len()),
                    label: head + &clip(c.hit.text.trim(), MAX_LABEL_TEXT),
                    path: c.hit.path,
                    line: c.hit.line,
                }
            })
            .collect()
    }

    /// `u` / Shift+F12: every whole-word occurrence of the identifier, case-sensitive.
    fn usages(&mut self) {
        let Some(word) = self.word_under(search::word_chars(self.kind(), false)) else {
            return;
        };
        let hits = self
            .grep(&regex::escape(&word), true, false, |_| true)
            .unwrap_or_default();
        if hits.is_empty() {
            self.message = format!("no usages of {word}");
            return;
        }
        self.show_picker(PickerKind::Usages, Self::hit_items(hits));
    }

    /// `D`: every declaration in the project, recomputed on each press.
    fn symbols(&mut self) {
        let mut named: Vec<(String, Hit)> = Vec::new();
        for (kind, pattern) in search::SYMBOLS {
            let re = Regex::new(pattern).expect("built-in symbol patterns are valid");
            let wanted = |p: &Path| match kind {
                Some(k) => search::kind_of(p) == Some(*k),
                None => search::shared_symbols(search::kind_of(p)),
            };
            let hits = self.grep(pattern, false, false, wanted).unwrap_or_default();
            named.extend(
                hits.into_iter()
                    .filter_map(|h| Some((search::symbol_name(&re, &h.text)?, h))),
            );
        }
        if named.is_empty() {
            self.message = "no symbols".into();
            return;
        }
        named.sort_by_cached_key(|(n, h)| (n.to_lowercase(), h.path.clone(), h.line));
        let width = named
            .iter()
            .map(|(n, _)| wrap::width(n))
            .max()
            .unwrap_or(0)
            .min(MAX_NAME_PAD);
        let items = named
            .into_iter()
            .map(|(name, h)| PickItem {
                label: format!(
                    "{name}{}  {}:{}",
                    " ".repeat(width.saturating_sub(wrap::width(&name))),
                    h.path.display(),
                    h.line
                ),
                path: h.path,
                line: h.line,
                code_at: None,
            })
            .collect();
        self.show_picker(PickerKind::Symbols, items);
    }

    // ---- tree ------------------------------------------------------------

    fn tree_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => self.tree.up(),
            KeyCode::Down => self.tree.down(),
            KeyCode::Right => self.tree.expand(),
            KeyCode::Left => self.tree.collapse(),
            KeyCode::Enter => match self.tree.selected() {
                Some(n) if n.is_dir => self.tree.toggle(),
                Some(n) => {
                    let path = self.root.join(&n.path);
                    // The review panel opens a file on its first hunk; the tree where it was.
                    match self.review.as_ref().and_then(|r| r.file(&n.path)).cloned() {
                        Some(f) if self.buf.path.as_deref() != Some(&*path) => {
                            self.open_review_file(&f, false)
                        }
                        _ => self.jump_to(&path, 0),
                    }
                }
                None => {}
            },
            _ => {}
        }
    }

    // ---- editing ---------------------------------------------------------

    /// Enter in the code pane: the cursor becomes a text cursor.
    fn start_edit(&mut self) {
        if self.buf.path.is_none() {
            return;
        }
        if let Some(why) = self.locked(std::iter::once(&self.buf.lines[self.line])) {
            self.message = why;
            return;
        }
        self.mode = Mode::Edit;
        self.undo_break = true;
    }

    /// Why the text cannot change or be written, if it cannot: the buffer is not what would be
    /// written back, or one of `lines` is longer than the screen shows, so its tail would be
    /// edited blind.
    fn locked<'a>(&self, mut lines: impl Iterator<Item = &'a String>) -> Option<String> {
        if let Some(why) = self.buf.readonly {
            return Some(format!("read-only: {why}"));
        }
        lines
            .any(|l| Buffer::clips(l))
            .then(|| "line too long to edit".to_string())
    }

    /// Keys that only mean something while editing. Returns `false` for every other key, which
    /// then falls through to the navigation keys: arrows, Home / End, the chord aliases.
    fn edit_key(&mut self, code: KeyCode, ctrl: bool) -> bool {
        match code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.flush();
            }
            // Without a selection the whole line goes, as in VS Code.
            KeyCode::Char('c' | 'x') if ctrl => {
                let (from, to, text) = match self.selection() {
                    Some((from, to)) => (from, to, self.selected_text().unwrap()),
                    None if self.line + 1 < self.buf.lines.len() => (
                        (self.line, 0),
                        (self.line + 1, 0),
                        format!("{}\n", self.line_str()),
                    ),
                    None => (
                        (self.line, 0),
                        (self.line, self.line_str().len()),
                        self.line_str().to_string(),
                    ),
                };
                self.clipboard = Some(text);
                self.message = "copied".into();
                if code == KeyCode::Char('x') {
                    self.replace(from, to, "");
                }
            }
            KeyCode::Char(c) if !ctrl => self.insert(&c.to_string()),
            KeyCode::Enter => {
                let head = &self.line_str()[..self.col];
                let indent = &head[..head.len() - head.trim_start().len()];
                self.insert(&format!("\n{indent}"));
            }
            KeyCode::Tab => self.insert(if self.buf.tabs { "\t" } else { buffer::TAB }),
            KeyCode::Backspace | KeyCode::Delete if self.selection().is_some() => self.insert(""),
            KeyCode::Backspace => {
                let from = if self.col > 0 {
                    (self.line, prev_char(self.line_str(), self.col))
                } else if self.line > 0 {
                    (self.line - 1, self.buf.lines[self.line - 1].len())
                } else {
                    return true;
                };
                self.replace(from, (self.line, self.col), "");
            }
            KeyCode::Delete => {
                let to = if self.col < self.line_str().len() {
                    (self.line, next_char(self.line_str(), self.col))
                } else if self.line + 1 < self.buf.lines.len() {
                    (self.line + 1, 0)
                } else {
                    return true;
                };
                self.replace((self.line, self.col), to, "");
            }
            _ => return false,
        }
        true
    }

    /// Inserts `text` at the cursor, or in place of the selection; a `\n` in it splits the line.
    fn insert(&mut self, text: &str) {
        let (from, to) = self
            .selection()
            .unwrap_or(((self.line, self.col), (self.line, self.col)));
        self.replace(from, to, text);
    }

    /// Text the terminal pasted (Cmd+V): inserted while editing, typed into a prompt or a picker
    /// query (its first line), ignored in navigation, where every letter is a command.
    pub fn paste(&mut self, text: &str) {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        if self.mode == Mode::Edit {
            self.insert(&text);
        } else if self.picker.is_some()
            || matches!(self.mode, Mode::Goto | Mode::Find | Mode::Search)
        {
            for c in text.lines().next().unwrap_or_default().chars() {
                self.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            }
        }
    }

    /// The one way the text changes: what lies between `from` and `to` (ordered (line, col))
    /// becomes `text`, and the cursor lands after it. Recorded for undo; typing that carries
    /// on where the previous step ended extends that step, as VS Code groups keystrokes. Refused,
    /// with the reason in the status bar, where `locked` says the text cannot change.
    fn replace(&mut self, from: (usize, usize), to: (usize, usize), text: &str) {
        let before = (self.line, self.col);
        let old: Vec<String> = self.buf.lines[from.0..=to.0].to_vec();
        let head = &old[0][..from.1];
        let tail = &old[old.len() - 1][to.1..];
        let mut new: Vec<String> = format!("{head}{text}")
            .split('\n')
            .map(String::from)
            .collect();
        let last = new.len() - 1;
        let col = new[last].len();
        new[last].push_str(tail);
        if let Some(why) = self.locked(old.iter().chain(&new)) {
            self.message = why;
            return;
        }
        self.buf.lines.splice(from.0..=to.0, new.iter().cloned());
        (self.line, self.col) = (from.0 + last, col);
        let edit = Edit {
            line: from.0,
            old,
            new,
            before,
            after: (self.line, self.col),
        };
        let continues = !self.undo_break
            && self.undo.last().is_some_and(|e| {
                e.after == before && e.new.len() == 1 && edit.old.len() == 1 && edit.new.len() == 1
            });
        if continues {
            let last = self.undo.last_mut().unwrap();
            last.new = edit.new;
            last.after = edit.after;
        } else {
            self.undo.push(edit);
        }
        self.undo_break = false;
        self.redo.clear();
        self.touched(from.0);
    }

    /// Ctrl+Z / Ctrl+Y: swaps one step between the two stacks and applies it.
    fn undo(&mut self, back: bool) {
        let Some(edit) = (if back { &mut self.undo } else { &mut self.redo }).pop() else {
            self.message = if back {
                "nothing to undo"
            } else {
                "nothing to redo"
            }
            .into();
            return;
        };
        let (from, to, at) = if back {
            (&edit.new, &edit.old, edit.before)
        } else {
            (&edit.old, &edit.new, edit.after)
        };
        self.buf
            .lines
            .splice(edit.line..edit.line + from.len(), to.iter().cloned());
        (self.line, self.col) = at;
        self.touched(edit.line);
        self.undo_break = true;
        (if back { &mut self.redo } else { &mut self.undo }).push(edit);
    }

    /// After every change to `buf.lines` from line `l` on: the highlighting there may be stale,
    /// the disk is behind, and the autosave clock restarts.
    fn touched(&mut self, l: usize) {
        self.buf.edited(l);
        self.dirty = true;
        self.last_edit = Some(Instant::now());
        self.anchor = None;
        self.sync_want_x();
    }

    /// Writes the buffer to its file. A file someone else changed since merl last read or wrote
    /// it is not overwritten: that is a conflict, and with the conflict on screen Ctrl+S (the one
    /// save that still runs) ends it in the buffer's favour.
    pub fn save(&mut self) {
        let Some(path) = &self.buf.path else { return };
        if let Some(why) = self.locked(std::iter::empty()) {
            self.message = why;
            return;
        }
        // ponytail: read, compare, then write; a writer landing between the read and the write
        // still loses. Files have no compare-and-swap, and the window is one read long.
        if !self.conflict && std::fs::read(path).is_ok_and(|b| buffer::hash(&b) != self.buf.disk) {
            self.conflict = true;
            return;
        }
        let bytes = self.buf.to_bytes();
        match std::fs::write(path, &bytes) {
            Ok(()) => {
                self.buf.disk = buffer::hash(&bytes);
                self.dirty = false;
                self.conflict = false;
                self.last_edit = None;
                self.refresh_diff();
            }
            Err(e) => {
                // Retried on the next autosave; the message stays until then.
                self.message = format!("save failed: {e}");
                self.last_edit = Some(Instant::now());
            }
        }
    }

    /// Saves unsaved edits now: leaving edit mode, switching files, quitting. Returns `false`
    /// when edits are left that the disk does not have: a conflict, or a failed save.
    pub fn flush(&mut self) -> bool {
        if self.dirty && !self.conflict {
            self.save();
        }
        !self.dirty
    }

    /// Autosave, called by the event loop between events. Returns `true` when it tried, which
    /// changes the status bar whether the file was written, refused or found changed on disk.
    pub fn tick(&mut self) -> bool {
        let due = self.last_edit.is_some_and(|t| t.elapsed() >= self.autosave);
        if !due || !self.dirty || self.conflict {
            return false;
        }
        self.save();
        true
    }

    // ---- keys ------------------------------------------------------------

    /// Handles one key. Returns `true` when merl should quit. A quit over edits that could not be
    /// saved is refused once, with the ways out in the status bar; quitting again right away
    /// leaves the edits behind.
    pub fn key(&mut self, key: KeyEvent) -> bool {
        let quit = self.key_inner(key);
        if !quit {
            self.quit_again = false;
            tutor::check(self);
            return false;
        }
        if self.flush() || std::mem::replace(&mut self.quit_again, true) {
            return true;
        }
        self.message = "unsaved edits: Ctrl+S saves, Ctrl+R drops them, q again quits".into();
        false
    }

    fn key_inner(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }
        let mut key = key;
        // Option+Left / Right arrive as Esc b / Esc f from Ghostty, iTerm and Terminal.app. The
        // prompts and the pickers move by word too, so this holds over them as well.
        if key.modifiers == KeyModifiers::ALT {
            match key.code {
                KeyCode::Char('b') => key.code = KeyCode::Left,
                KeyCode::Char('f') => key.code = KeyCode::Right,
                _ => {}
            }
        }
        // Legacy terminals report Alt+X as Esc followed by X; treat it that way. When the Esc
        // closes a picker, a prompt or the help, the letter belonged to that overlay and is
        // dropped, so Alt+q over a picker cannot quit merl. Alt+arrow is unambiguous everywhere.
        if key.modifiers.contains(KeyModifiers::ALT) && matches!(key.code, KeyCode::Char(_)) {
            key.modifiers.remove(KeyModifiers::ALT);
            let overlay = self.picker.is_some() || self.mode != Mode::Normal;
            self.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            return !overlay && self.key(key);
        }
        // In kitty mode `:`, `?` and `D` arrive with SHIFT set; legacy sends none.
        if matches!(key.code, KeyCode::Char(_)) {
            key.modifiers.remove(KeyModifiers::SHIFT);
        }
        // Cmd+C / Cmd+X reach merl only from a terminal told to pass them on (see the README);
        // they are the Ctrl chords then, except that Cmd+C never quits.
        let cmd =
            key.modifiers == KeyModifiers::SUPER && matches!(key.code, KeyCode::Char('c' | 'x'));
        if cmd {
            key.modifiers = KeyModifiers::CONTROL;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let alt = key.modifiers.contains(KeyModifiers::ALT);

        if ctrl && key.code == KeyCode::Char('c') && self.mode != Mode::Edit {
            // With a selection Ctrl+C is the copy it is everywhere else; without one it quits.
            let text = self.selected_text();
            if self.mode != Mode::Normal || self.picker.is_some() || text.is_none() {
                return !cmd;
            }
            self.clipboard = text;
            self.message = "copied".into();
            return false;
        }
        if self.picker.is_some() {
            self.picker_key(key);
            return false;
        }
        match self.mode {
            Mode::Goto => {
                self.goto_key(key);
                return false;
            }
            Mode::Find => {
                self.find_key(key);
                return false;
            }
            Mode::Search => {
                self.search_key(key);
                return false;
            }
            Mode::Help => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                        self.mode = Mode::Normal;
                    }
                    KeyCode::Up => self.help_top = self.help_top.saturating_sub(1),
                    KeyCode::Down => self.help_top += 1,
                    _ => {}
                }
                return false;
            }
            _ => {}
        }

        self.message.clear();
        if self.mode == Mode::Edit && self.edit_key(key.code, ctrl) {
            self.hist_note(false);
            return false;
        }
        let extending = key.code == KeyCode::Char('v')
            || shift
                && matches!(
                    key.code,
                    KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right
                );
        // Ctrl+D/U and PageUp/Down are how merl scrolls, and a page is always farther than
        // `HIST_NEAR`: the current stop follows the cursor anyway, so paging through a file adds
        // no stops and drops no forward history.
        let paging = matches!(key.code, KeyCode::PageUp | KeyCode::PageDown)
            || ctrl && matches!(key.code, KeyCode::Char('d' | 'u'));
        let before = (self.line, self.col);
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('?') => {
                self.mode = Mode::Help;
                self.help_top = 0;
            }
            KeyCode::Esc => {
                self.anchor = None;
                self.find_re = None;
                self.message = "find cleared".into();
            }
            KeyCode::Char('g') if ctrl => {
                self.mode = Mode::Goto;
                self.prompt.clear();
            }
            KeyCode::Char('s') if ctrl => self.save(),
            KeyCode::Char('r') if ctrl => self.reload(true),
            KeyCode::Char('z') if ctrl => self.undo(true),
            KeyCode::Char('y') if ctrl => self.undo(false),
            KeyCode::Char(':') => {
                self.mode = Mode::Goto;
                self.prompt.clear();
            }
            KeyCode::Char('/') => self.start_find(),
            KeyCode::Char('f') if ctrl => self.start_find(),
            KeyCode::Char('n') if !ctrl => {
                self.step_find(true);
                self.hist_note(true);
            }
            KeyCode::Char('N') => {
                self.step_find(false);
                self.hist_note(true);
            }
            KeyCode::Char('s') => {
                self.mode = Mode::Search;
                self.prompt.clear();
            }
            KeyCode::Char('d') if ctrl => self.half_page(1),
            KeyCode::Char('u') if ctrl => self.half_page(-1),
            KeyCode::Char('{') => self.paragraph(-1),
            KeyCode::Char('}') => self.paragraph(1),
            KeyCode::Char('d') | KeyCode::F(12) if !shift => self.goto_definition(),
            KeyCode::Char('u') | KeyCode::F(12) => self.usages(),
            KeyCode::Char('D') => self.symbols(),
            KeyCode::Char('t') => {
                self.show_tree = !self.show_tree;
                if !self.show_tree {
                    self.focus = Focus::Code;
                }
            }
            KeyCode::Char('T') => self.open_themes_picker(),
            KeyCode::Tab if self.show_tree => {
                self.focus = match self.focus {
                    Focus::Tree => Focus::Code,
                    Focus::Code => Focus::Tree,
                };
            }
            KeyCode::Char('o') if !ctrl => self.open_files_picker(),
            KeyCode::Char('e') if ctrl => self.open_files_picker(),
            KeyCode::Char('[') => self.hist_go(-1),
            KeyCode::Char(']') => self.hist_go(1),
            KeyCode::Char('c') if !ctrl => self.hunk(1),
            KeyCode::Char('C') => self.hunk(-1),
            KeyCode::Char('v') if self.focus == Focus::Code => self.grow_selection(),
            _ if self.focus == Focus::Tree => self.tree_key(key.code),
            KeyCode::Enter => self.start_edit(),
            KeyCode::Up if shift => self.extend(|s| s.move_rows(-1)),
            KeyCode::Down if shift => self.extend(|s| s.move_rows(1)),
            KeyCode::Up => self.move_rows(-1),
            KeyCode::Down => self.move_rows(1),
            KeyCode::Left if shift && ctrl => self.extend(Self::line_start),
            KeyCode::Right if shift && ctrl => self.extend(Self::line_end),
            KeyCode::Left if shift && alt => self.extend(Self::word_left),
            KeyCode::Right if shift && alt => self.extend(Self::word_right),
            // A plain arrow on a selection collapses it to the matching end, VS Code style.
            KeyCode::Left | KeyCode::Right if !shift && !alt && self.selection().is_some() => {
                let (start, end) = self.selection().unwrap();
                (self.line, self.col) = self.clamp_pos(if key.code == KeyCode::Left {
                    start
                } else {
                    end
                });
                self.sync_want_x();
                self.anchor = None;
            }
            KeyCode::Left if shift => self.extend(Self::left),
            KeyCode::Right if shift => self.extend(Self::right),
            KeyCode::Left if alt => self.word_left(),
            KeyCode::Right if alt => self.word_right(),
            KeyCode::Left => self.left(),
            KeyCode::Right => self.right(),
            KeyCode::PageUp => self.move_rows(-(self.view_h.max(1) as isize)),
            KeyCode::PageDown => self.move_rows(self.view_h.max(1) as isize),
            KeyCode::Home if ctrl => {
                self.line = 0;
                self.col = 0;
                self.want_x = 0;
            }
            KeyCode::End if ctrl => {
                self.line = self.buf.lines.len() - 1;
                self.col = self.line_str().len();
                self.sync_want_x();
            }
            KeyCode::Home => self.line_start(),
            KeyCode::End => self.line_end(),
            _ => {}
        }
        if !extending {
            self.drop_selection_if_moved(before);
        }
        if paging && let (Some(pos), Some(cur)) = (self.pos(), self.history.get_mut(self.hist_idx))
        {
            *cur = pos;
        }
        self.hist_note(false);
        false
    }

    fn goto_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if let Ok(n) = self.prompt.parse::<usize>() {
                    match self.buf.path.clone() {
                        Some(path) => self.jump_to(&path, n),
                        None => self.goto_line(n),
                    }
                }
                self.mode = Mode::Normal;
                self.prompt.clear();
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.prompt.clear();
            }
            // Only digits are typed here; Ctrl+letter still edits the line.
            KeyCode::Char(c) if !c.is_ascii_digit() && key.modifiers.is_empty() => {}
            _ => {
                self.prompt.key(key);
            }
        }
    }

    fn search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => self.run_search(),
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.prompt.clear();
            }
            _ => {
                self.prompt.key(key);
            }
        }
    }
}

/// A type `d` followed a receiver to: its name and the line that declares it.
#[derive(Debug, Clone)]
struct Typed {
    name: String,
    path: PathBuf,
    line: usize,
}

/// Whether the Python class declared on 1-based `decl` of `text` is a `typing.Protocol`, which
/// anything with its members implements without naming it.
fn is_protocol(text: &str, decl: usize) -> bool {
    search::bases(Kind::Python, text, decl)
        .iter()
        .filter_map(|b| search::type_path(Kind::Python, b))
        .any(|p| p.last().is_some_and(|n| n == "Protocol"))
}

/// The module path an import in `imports` binds `name` to.
fn bound(imports: &[(String, Vec<String>)], name: &str) -> Option<Vec<String>> {
    imports
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, p)| p.clone())
}

/// What the status line says after `d` on `word`: `word → Target.word (reason)` for a jump,
/// `word: reason` when the target is not declared inside anything, `word: reason, N
/// declarations` over a picker. A jump by name says it had one match, so a guess that happened to
/// be unique never reads as a resolution. A chain in front of the word that `broke` says at which
/// name: `(by name, 1 match, chain broke at users)`, `by name, 2 declarations (chain broke at
/// users)`.
fn resolution(
    word: &str,
    target: Option<&str>,
    found: &[Candidate],
    broke: Option<&str>,
) -> String {
    let note = broke.map_or(String::new(), |at| format!(" (chain broke at {at})"));
    let Some(first) = found.first() else {
        return format!("no definition for {word}{note}");
    };
    let reason = &first.reason;
    if let [_] = found {
        let mut why = reason.to_string();
        if !reason.proven() {
            why.push_str(", 1 match");
        }
        if let Some(at) = broke {
            why.push_str(&format!(", chain broke at {at}"));
        }
        return match target {
            Some(target) => format!("{word} \u{2192} {target} ({why})"),
            None => format!("{word}: {why}"),
        };
    }
    // The candidates stop at MAX_HITS, so that many is a lower bound.
    let n = match found.len() {
        n if n >= search::MAX_HITS => format!("{n}+"),
        n => n.to_string(),
    };
    if found.iter().all(|c| c.reason == *reason) {
        format!("{word}: {reason}, {n} declarations{note}")
    } else {
        // Each row says its own reason.
        format!("{word}: {n} declarations{note}")
    }
}

/// Cuts `s` to `max` chars, marking the cut.
pub(crate) fn clip(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((i, _)) => format!("{}\u{2026}", &s[..i]),
        None => s.to_string(),
    }
}

/// A word for the cursor: letters of any script, so a comment in Russian moves by word too.
pub fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn char_at(s: &str, i: usize) -> char {
    s[i..].chars().next().unwrap_or(' ')
}

fn next_char(s: &str, i: usize) -> usize {
    s[i..].chars().next().map_or(i, |c| i + c.len_utf8())
}

fn prev_char(s: &str, i: usize) -> usize {
    s[..i].chars().next_back().map_or(0, |c| i - c.len_utf8())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(text: &str) -> App {
        // Edits get saved, so each test has a file of its own, absent until the first save.
        let name = std::thread::current()
            .name()
            .unwrap_or("main")
            .replace("::", "-");
        let path = std::env::temp_dir().join(format!("merl-{name}.txt"));
        let _ = std::fs::remove_file(&path);
        let mut a = App::new(
            PathBuf::from("/tmp"),
            Tree::default(),
            Vec::new(),
            Buffer::from_bytes(path, text.as_bytes()),
            None,
        );
        a.view_w = 20;
        a.view_h = 10;
        a
    }

    fn press(a: &mut App, code: KeyCode, m: KeyModifiers) -> bool {
        a.key(KeyEvent::new(code, m))
    }

    #[test]
    fn word_jumps_by_word_runs() {
        let mut a = app("foo bar_1 baz\nnext");
        press(&mut a, KeyCode::Right, KeyModifiers::ALT);
        assert_eq!(a.col, 3);
        press(&mut a, KeyCode::Right, KeyModifiers::ALT);
        assert_eq!(a.col, 9);
        press(&mut a, KeyCode::Left, KeyModifiers::ALT);
        assert_eq!(a.col, 4);
        // Past the end of the line, jump to the start of the next one.
        a.col = 13;
        press(&mut a, KeyCode::Right, KeyModifiers::ALT);
        assert_eq!(((a.line, a.col), a.selection()), ((1, 0), None));
    }

    #[test]
    fn a_word_is_letters_of_any_script() {
        let mut a = app("// привет, мир foo");
        a.col = 3;
        press(
            &mut a,
            KeyCode::Right,
            KeyModifiers::ALT | KeyModifiers::SHIFT,
        );
        assert_eq!(a.selected_text().as_deref(), Some("привет"));
        press(&mut a, KeyCode::Right, KeyModifiers::ALT);
        press(&mut a, KeyCode::Left, KeyModifiers::ALT);
        assert_eq!(a.col, "// привет, ".len());
        press(&mut a, KeyCode::Char('v'), KeyModifiers::NONE);
        assert_eq!(a.selected_text().as_deref(), Some("мир"));
    }

    /// Ghostty, iTerm and Terminal.app send Option+Left / Right as Esc b / Esc f.
    #[test]
    fn alt_b_and_alt_f_are_the_word_jump_and_do_not_leave_edit_mode() {
        let mut a = app("foo bar baz");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('f'), KeyModifiers::ALT);
        press(&mut a, KeyCode::Char('f'), KeyModifiers::ALT);
        assert_eq!((a.col, a.mode), (7, Mode::Edit));
        press(&mut a, KeyCode::Char('b'), KeyModifiers::ALT);
        assert_eq!((a.col, a.mode), (4, Mode::Edit));
        assert_eq!(a.buf.lines, vec!["foo bar baz"], "nothing was typed");
    }

    #[test]
    fn shift_left_right_select_by_char_and_cross_the_line_end() {
        let mut a = app("ab\ncd");
        a.col = 1;
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
        assert_eq!(a.selection(), Some(((0, 1), (0, 2))));
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
        assert_eq!(a.selection(), Some(((0, 1), (1, 0))));
        for _ in 0..3 {
            press(&mut a, KeyCode::Left, KeyModifiers::SHIFT);
        }
        assert_eq!(
            a.selection(),
            Some(((0, 0), (0, 1))),
            "back over the anchor"
        );
    }

    #[test]
    fn ctrl_c_copies_a_selection_in_navigation_and_quits_without_one() {
        let mut a = app("abc");
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
        assert!(!press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(a.clipboard.take().as_deref(), Some("ab"));
        assert!(a.selection().is_some(), "copy leaves the selection");
        // Cmd+C, from a terminal that passes it on, copies too and never quits.
        assert!(!press(&mut a, KeyCode::Char('c'), KeyModifiers::SUPER));
        assert_eq!(a.clipboard.take().as_deref(), Some("ab"));
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert!(!press(&mut a, KeyCode::Char('c'), KeyModifiers::SUPER));
        assert!(press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(a.clipboard, None);
    }

    #[test]
    fn v_grows_the_selection_word_line_paragraph() {
        let mut a = app("x\n\n  let foo = 1;\n  bar\n\ny");
        (a.line, a.col) = (2, 7);
        let v = |a: &mut App| press(a, KeyCode::Char('v'), KeyModifiers::NONE);
        v(&mut a);
        assert_eq!(a.selection(), Some(((2, 6), (2, 9))), "the word");
        v(&mut a);
        assert_eq!(a.selection(), Some(((2, 0), (2, 14))), "the line");
        v(&mut a);
        assert_eq!(a.selection(), Some(((2, 0), (3, 5))), "the paragraph");
        v(&mut a);
        assert_eq!(a.selection(), Some(((2, 0), (3, 5))), "nothing wider");
        // Off a word the first step is the line; a hand-made selection grows from what it is.
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        (a.line, a.col) = (2, 11);
        v(&mut a);
        assert_eq!(a.selection(), Some(((2, 0), (2, 14))));
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        (a.line, a.col) = (2, 5);
        for _ in 0..3 {
            press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
        }
        v(&mut a);
        assert_eq!(
            a.selection(),
            Some(((2, 0), (2, 14))),
            "` fo` is wider than no word"
        );
        // A blank line has nothing to select.
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        (a.line, a.col) = (1, 0);
        v(&mut a);
        assert_eq!(a.selection(), None);
    }

    #[test]
    fn goto_prompt_clamps() {
        let mut a = app("a\nb\nc\n");
        for code in [KeyCode::Char(':'), KeyCode::Char('9'), KeyCode::Enter] {
            press(&mut a, code, KeyModifiers::NONE);
        }
        assert_eq!(a.mode, Mode::Normal);
        assert_eq!(a.line, 2);
        // `q` inside the prompt is a digit-less keystroke, not a quit.
        press(&mut a, KeyCode::Char(':'), KeyModifiers::NONE);
        assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(!press(&mut a, KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(a.mode, Mode::Normal);
        assert!(press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
    }

    /// The find prompt is edited in place: text typed in front of the query narrows the search
    /// at once, and a selected query goes with one Backspace, which puts the cursor back.
    #[test]
    fn find_prompt_is_edited_at_the_cursor() {
        let mut a = app("now()\nfunc now()\n");
        for c in "/now".chars() {
            press(&mut a, KeyCode::Char(c), KeyModifiers::NONE);
        }
        assert_eq!((a.line, a.col), (0, 0));
        press(&mut a, KeyCode::Home, KeyModifiers::NONE);
        for c in "func ".chars() {
            press(&mut a, KeyCode::Char(c), KeyModifiers::NONE);
        }
        assert_eq!((&*a.prompt, a.line, a.col), ("func now", 1, 0));
        // Option+Left as Ghostty sends it, Esc b, moves by a word and leaves the prompt open.
        press(&mut a, KeyCode::Char('b'), KeyModifiers::ALT);
        assert_eq!((a.mode, a.prompt.cursor()), (Mode::Find, 0));
        press(&mut a, KeyCode::Right, KeyModifiers::ALT);
        press(&mut a, KeyCode::Right, KeyModifiers::NONE);
        press(&mut a, KeyCode::End, KeyModifiers::SHIFT);
        press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!((&*a.prompt, a.line), ("func ", 1));
        // Ctrl+U is the line's, not a `u` typed into it.
        press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
        assert_eq!((&*a.prompt, a.line, a.mode), ("", 0, Mode::Find));
        // The goto prompt still takes digits only.
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        for c in ":1x2".chars() {
            press(&mut a, KeyCode::Char(c), KeyModifiers::NONE);
        }
        assert_eq!(&*a.prompt, "12");
    }

    /// `T` previews the theme under the cursor without committing to it: Esc puts the one in
    /// use back, Enter keeps the new one.
    #[test]
    fn theme_picker_previews_reverts_and_keeps() {
        let names: Vec<&str> = crate::theme::names().collect();
        let mut a = app("x\n");
        a.theme = names[2].to_string();
        press(&mut a, KeyCode::Char('T'), KeyModifiers::SHIFT);
        a.picker.as_mut().unwrap().settle();
        assert_eq!(a.mode, Mode::Picker(PickerKind::Themes));
        assert_eq!(
            a.shown_theme(),
            names[2],
            "the cursor starts on the theme in use"
        );

        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!((a.shown_theme(), a.theme.as_str()), (names[3], names[2]));
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!((a.shown_theme(), a.picker.is_none()), (names[2], true));

        press(&mut a, KeyCode::Char('T'), KeyModifiers::NONE);
        a.picker.as_mut().unwrap().settle();
        press(&mut a, KeyCode::Up, KeyModifiers::NONE);
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!((a.shown_theme(), a.theme.as_str()), (names[1], names[1]));
        // Named from the table, so reordering or renaming a theme cannot fail this test.
        let kept = format!("theme {}", names[1]);
        assert_eq!((a.mode, a.message.as_str()), (Mode::Normal, kept.as_str()));
    }

    #[test]
    fn enter_in_the_theme_picker_writes_the_config() {
        let dir = std::env::temp_dir().join(format!("merl-theme-config-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut a = app("x\n");
        a.config = Some(dir.join("config.toml"));
        press(&mut a, KeyCode::Char('T'), KeyModifiers::NONE);
        a.picker.as_mut().unwrap().settle();
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        let name = crate::theme::names().nth(1).unwrap();
        assert_eq!(a.message, format!("theme {name} saved"));
        assert_eq!(
            std::fs::read_to_string(dir.join("config.toml")).unwrap(),
            format!("theme = \"{name}\"\n")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_paste_types_into_prompts_and_pickers_but_not_navigation() {
        let mut a = app("foo\n");
        a.paste("so");
        assert_eq!((a.mode, a.picker.is_none()), (Mode::Normal, true));
        press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
        a.paste("parse_it\r\nsecond line");
        assert_eq!((a.mode, &*a.prompt), (Mode::Search, "parse_it"));
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
        a.paste("app.rs");
        assert_eq!(&*a.picker.as_ref().unwrap().query, "app.rs");
    }

    #[test]
    fn shift_on_char_is_stripped() {
        let mut a = app("a\nb\n");
        assert!(press(&mut a, KeyCode::Char('q'), KeyModifiers::SHIFT));
        let mut a = app("a\nb\n");
        press(&mut a, KeyCode::Char(':'), KeyModifiers::SHIFT);
        assert_eq!(a.mode, Mode::Goto);
    }

    #[test]
    fn shift_up_down_extend_a_selection_from_the_cursor_column() {
        let mut a = app("abc\ndef\nghi\nj\n");
        assert_eq!(a.selection(), None);
        press(&mut a, KeyCode::Right, KeyModifiers::NONE);
        press(&mut a, KeyCode::Right, KeyModifiers::NONE);
        press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
        press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
        assert_eq!((a.line, a.selection()), (2, Some(((0, 2), (2, 2)))));
        // Per line, the bytes the renderer paints: partial edges, whole lines in between.
        assert_eq!(a.selected_bytes(0), Some(2..3));
        assert_eq!(a.selected_bytes(1), Some(0..3));
        assert_eq!(a.selected_bytes(2), Some(0..2));
        assert_eq!(a.selected_bytes(3), None);
        // Back onto the anchor nothing is selected, and Shift+Down selects from there again.
        for _ in 0..3 {
            press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
        }
        assert_eq!((a.line, a.selection()), (0, None));
        press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
        assert_eq!(a.selection(), Some(((0, 2), (1, 2))));
        // Any other cursor move drops it; Esc drops it too.
        press(&mut a, KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(a.selection(), None);
        press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
        assert!(a.selection().is_some());
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(a.selection(), None);
    }

    #[test]
    fn a_selection_collapsed_onto_its_anchor_selects_nothing() {
        let mut a = app("abc\ndef\n");
        press(&mut a, KeyCode::Right, KeyModifiers::NONE);
        press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
        press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
        assert_eq!(a.selection(), None);
        // A plain arrow moves on instead of collapsing a range that is not there.
        press(&mut a, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(a.col, 2);
        // While editing, Ctrl+C copies the line and Backspace deletes a char, as with no selection.
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
        press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
        press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(a.clipboard.take().as_deref(), Some("abc\n"));
        press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(a.buf.lines, vec!["ac", "def"]);
    }

    #[test]
    fn a_jump_or_a_find_match_drops_the_selection() {
        let mut a = app("abc\ndef\nghi\njkl\n");
        press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
        press(&mut a, KeyCode::Char(':'), KeyModifiers::NONE);
        typed(&mut a, "4");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!((a.line, a.selection()), (3, None));
        // Find sets the selection aside while it moves the cursor: Esc puts both back, a match
        // taken with Enter leaves it behind.
        press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
        let sel = a.selection();
        assert_eq!(sel, Some(((2, 0), (3, 0))));
        // A jump onto the cursor itself goes nowhere and keeps it.
        press(&mut a, KeyCode::Char(':'), KeyModifiers::NONE);
        typed(&mut a, "3");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(a.selection(), sel);
        press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
        typed(&mut a, "abc");
        assert_eq!((a.line, a.selection()), (0, None));
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!((a.line, a.selection()), (2, sel));
        press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
        typed(&mut a, "abc");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!((a.line, a.selection()), (0, None));
        // A search that has nowhere to move the cursor keeps it.
        press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
        press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
        typed(&mut a, "zzz");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(a.selection(), Some(((0, 0), (1, 0))));
        // One that moved the cursor and then stopped matching has still moved it.
        press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
        typed(&mut a, "ghz");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!((a.line, a.selection()), (2, None));
        // So does a picker jump within the open file.
        press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
        let hit = Hit {
            path: a.buf.path.clone().unwrap(),
            line: 4,
            text: "jkl".into(),
        };
        a.show_picker(PickerKind::Usages, App::hit_items(vec![hit]));
        a.picker.as_mut().unwrap().settle();
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!((a.line, a.selection()), (3, None));
    }

    #[test]
    fn ctrl_shift_left_right_select_to_the_line_edges() {
        let mut a = app("foo bar\nbaz");
        for _ in 0..3 {
            press(&mut a, KeyCode::Right, KeyModifiers::NONE);
        }
        press(
            &mut a,
            KeyCode::Right,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        assert_eq!((a.col, a.selection()), (7, Some(((0, 3), (0, 7)))));
        // A plain arrow collapses the selection to its end without moving on, VS Code style.
        press(&mut a, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(((a.line, a.col), a.selection()), ((0, 7), None));
        press(&mut a, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (1, 0));
        press(&mut a, KeyCode::Up, KeyModifiers::NONE);
        press(&mut a, KeyCode::End, KeyModifiers::NONE);
        press(
            &mut a,
            KeyCode::Left,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        assert_eq!((a.col, a.selection()), (0, Some(((0, 0), (0, 7)))));
        // Left collapses to the start, which is where the cursor already is: no move.
        press(&mut a, KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(((a.line, a.col), a.selection()), ((0, 0), None));
        press(
            &mut a,
            KeyCode::Right,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        press(&mut a, KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(((a.line, a.col), a.selection()), ((0, 0), None));
        // Alt+Right alone is a plain word jump: it drops the selection.
        press(
            &mut a,
            KeyCode::Right,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        press(&mut a, KeyCode::Left, KeyModifiers::ALT);
        assert_eq!((a.col, a.selection()), (4, None));
    }

    #[test]
    fn alt_shift_left_right_select_by_word_without_touching_find() {
        let mut a = app("foo bar_1 baz");
        a.find_re = Some(Regex::new("foo").unwrap());
        let m = KeyModifiers::ALT | KeyModifiers::SHIFT;
        press(&mut a, KeyCode::Right, m);
        press(&mut a, KeyCode::Right, m);
        assert_eq!((a.col, a.selection()), (9, Some(((0, 0), (0, 9)))));
        press(&mut a, KeyCode::Left, m);
        assert_eq!((a.col, a.selection()), (4, Some(((0, 0), (0, 4)))));
        assert!(a.find_re.is_some(), "Alt on an arrow is not an Esc");
        assert_eq!(a.message, "");
    }

    #[test]
    fn want_x_is_sticky_across_short_lines() {
        let mut a = app("abcdef\nxy\nabcdef");
        a.col = 5;
        a.want_x = 5;
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(a.col, 2);
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(a.col, 5);
    }

    /// Two small files in a scratch directory, for the jump-history tests.
    fn files_app(tag: &str) -> (PathBuf, App) {
        let dir = std::env::temp_dir().join(format!("merl-hist-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["a.rs", "b.rs"] {
            std::fs::write(dir.join(name), "x\n".repeat(40)).unwrap();
        }
        let app = App::new(
            dir.clone(),
            Tree::default(),
            Vec::new(),
            Buffer::empty(),
            None,
        );
        (dir, app)
    }

    /// A repository with a `feature` branch checked out: `src/a.rs` changed twice, `new`
    /// added, `gone` deleted; the app is in review mode on it.
    fn review_app(tag: &str) -> (PathBuf, App) {
        let dir = std::env::temp_dir().join(format!("merl-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .output()
                .unwrap();
            assert!(out.status.success(), "git {args:?}");
        };
        git(&["init", "-q", "-b", "main"]);
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        std::fs::write(dir.join("src/a.rs"), "a\nb\nc\nd\ne\nf\n").unwrap();
        std::fs::write(dir.join("gone"), "x\ny\n").unwrap();
        std::fs::write(dir.join("tail"), "t1\nt2\nt3\n").unwrap();
        std::fs::write(dir.join("crlf.txt"), "one\r\ntwo\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "base"]);
        git(&["switch", "-q", "-c", "feature"]);
        std::fs::write(dir.join("src/a.rs"), "a\nB\nc\nd\ne\nF\n").unwrap();
        std::fs::write(dir.join("new"), "n\n").unwrap();
        std::fs::remove_file(dir.join("gone")).unwrap();
        std::fs::write(dir.join("tail"), "t1\n").unwrap();
        std::fs::write(dir.join("crlf.txt"), "one\r\nTWO\n").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "work"]);
        let review = git::Review::open(&dir, None, None).unwrap();
        let (_, files) = crate::tree::build(&dir);
        let tree = crate::tree::from_files(
            &review
                .files
                .iter()
                .map(|f| f.path.clone())
                .collect::<Vec<_>>(),
        );
        let first = dir.join("new");
        let mut a = App::new(
            dir.clone(),
            tree,
            files,
            Buffer::load(&first).unwrap(),
            None,
        );
        a.start_review(review);
        (dir, a)
    }

    #[test]
    fn review_walks_hunks_across_files_and_opens_deleted_files_from_the_base() {
        let (dir, mut a) = review_app("reviewapp");
        let c = |a: &mut App| press(a, KeyCode::Char('c'), KeyModifiers::NONE);
        let big_c = |a: &mut App| press(a, KeyCode::Char('C'), KeyModifiers::NONE);
        // Files in panel order: src/a.rs (M), crlf.txt (M), gone (D), new (A), tail (M).
        assert_eq!(at(&a), (dir.join("new"), 0));
        assert_eq!(a.review_status().unwrap(), "hunk 1/1  file 4/5");
        // A deletion at the end of `tail` is a stop on its last line, with the ghosts under it.
        c(&mut a);
        assert_eq!(at(&a), (dir.join("tail"), 0));
        assert_eq!(a.diff.ghosts[&1], vec!["t2", "t3"]);
        assert_eq!(a.review_status().unwrap(), "hunk 1/1  file 5/5");
        c(&mut a);
        assert_eq!(at(&a), (dir.join("tail"), 0));
        assert_eq!(a.message, "last hunk of the review");
        // A file crossing is one stop: `[` goes straight back to the previous file.
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("new"), 0));
        // Back over the deleted file: read from the base, read-only, all marked.
        big_c(&mut a);
        assert_eq!(at(&a), (dir.join("gone"), 0));
        assert_eq!(a.buf.lines, vec!["x", "y"]);
        assert_eq!(a.diff.marks.len(), 2);
        assert_eq!(a.buf.readonly, Some("deleted in this branch"));
        assert!(a.review_status().unwrap().starts_with("hunk 0/0"));
        // A read-only buffer that the branch did not delete keeps its real diff.
        big_c(&mut a);
        assert_eq!(at(&a), (dir.join("crlf.txt"), 1));
        assert_eq!(a.buf.readonly, Some("mixed line endings"));
        assert_eq!(a.diff.hunks, vec![1]);
        assert_eq!(a.diff.marks.len(), 1);
        big_c(&mut a);
        assert_eq!(at(&a), (dir.join("src/a.rs"), 5));
        assert_eq!(a.review_status().unwrap(), "hunk 2/2  file 1/5");
        big_c(&mut a);
        assert_eq!(at(&a), (dir.join("src/a.rs"), 1));
        assert_eq!(a.diff.ghosts[&1], vec!["b"]);
        big_c(&mut a);
        assert_eq!(a.message, "first hunk of the review");
        c(&mut a);
        assert_eq!(at(&a), (dir.join("src/a.rs"), 5));
        // `[` walks back through the same stops.
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("src/a.rs"), 1));
        // The panel lists only the branch's files.
        let names: Vec<_> = a
            .tree
            .nodes
            .iter()
            .map(|n| n.path.to_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec!["src", "src/a.rs", "crlf.txt", "gone", "new", "tail"]
        );
        assert_eq!(
            a.review
                .as_ref()
                .unwrap()
                .files
                .iter()
                .map(|f| f.status)
                .collect::<Vec<_>>(),
            vec!['M', 'M', 'D', 'A', 'M']
        );
        // Enter in the panel opens a file on its first hunk; on the open file it stays put.
        a.tree.reveal(Path::new("src/a.rs"));
        a.focus = Focus::Tree;
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("src/a.rs"), 1));
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        a.focus = Focus::Tree;
        a.tree.reveal(Path::new("src/a.rs"));
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("src/a.rs"), 3));
        // Opening a shorter file from far down a long one (Enter in the panel, #61 follow-up):
        // the viewport of the old file must not be read against the new one.
        a.jump_to(&dir.join("src/a.rs"), 6);
        (a.top_line, a.top_row) = (5, 0);
        a.jump_to(&dir.join("new"), 0);
        assert_eq!((at(&a), a.top_line), ((dir.join("new"), 0), 0));
        // Outside the project (the standard library, a dependency) nothing is marked deleted.
        let outside = std::env::temp_dir().join(format!("merl-outside-{}", std::process::id()));
        std::fs::write(&outside, "fn x() {}\n").unwrap();
        a.jump_to(&outside, 1);
        assert_eq!(a.buf.readonly, Some("outside the project"));
        assert!(a.diff.marks.is_empty() && a.diff.hunks.is_empty());
        std::fs::remove_file(&outside).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn history_walks_back_and_forward() {
        let (dir, mut a) = files_app("walk");
        let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
        a.jump_to(&x, 1);
        a.jump_to(&y, 1);
        a.jump_to(&y, 5);
        assert_eq!(
            a.history,
            [(x.clone(), 0, 0), (y.clone(), 0, 0), (y.clone(), 4, 0)]
        );
        assert_eq!(a.hist_idx, 2);

        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!((a.buf.path.clone().unwrap(), a.line), (y.clone(), 0));
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!((a.buf.path.clone().unwrap(), a.line), (x.clone(), 0));
        press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
        assert_eq!((a.buf.path.clone().unwrap(), a.line), (y.clone(), 0));
        // Walking does not record anything.
        assert_eq!(a.history.len(), 3);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn history_truncates_forward_and_stops_at_the_ends() {
        let (dir, mut a) = files_app("ends");
        let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(a.message, "start of history");
        press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
        assert_eq!(a.message, "end of history");

        a.jump_to(&x, 1);
        a.jump_to(&y, 1);
        a.jump_to(&y, 5);
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        // A jump from the middle of the history drops everything after it.
        a.jump_to(&y, 8);
        assert_eq!(a.history, [(x, 0, 0), (y, 7, 0)]);
        assert_eq!(a.hist_idx, 1);
        press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
        assert_eq!(a.message, "end of history");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn repeated_jumps_to_the_same_spot_are_not_recorded() {
        let (dir, mut a) = files_app("dup");
        let x = dir.join("a.rs");
        a.jump_to(&x, 3);
        a.jump_to(&x, 3);
        a.jump_to(&x, 3);
        assert_eq!(a.history, [(x, 2, 0)]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn at(a: &App) -> (PathBuf, usize) {
        (a.buf.path.clone().unwrap(), a.line)
    }

    #[test]
    fn far_moves_become_stops_and_near_moves_update_the_current_one() {
        let (dir, mut a) = files_app("wander");
        let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
        a.jump_to(&x, 1);
        a.jump_to(&y, 1);
        // Three lines down is still "here": the current stop follows the cursor.
        for _ in 0..3 {
            press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        }
        assert_eq!(a.history, [(x.clone(), 0, 0), (y.clone(), 3, 0)]);
        // The end of the file (36 lines away) is somewhere else: a new stop.
        press(&mut a, KeyCode::End, KeyModifiers::CONTROL);
        assert_eq!(
            a.history,
            [(x.clone(), 0, 0), (y.clone(), 3, 0), (y.clone(), 39, 1)]
        );
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(at(&a), (y.clone(), 3));
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(at(&a), (x.clone(), 0));
        press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
        press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
        assert_eq!(at(&a), (y, 39));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Ctrl+D/U and PageUp/Down are scrolling, not jumps: however far a page is, the current
    /// stop follows the cursor, so one `[` is back at the call site and `]` still works after
    /// reading around it.
    #[test]
    fn paging_moves_the_current_stop_instead_of_adding_stops() {
        let (dir, mut a) = files_app("paging");
        let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
        a.jump_to(&x, 1);
        press(&mut a, KeyCode::Char('d'), KeyModifiers::CONTROL);
        press(&mut a, KeyCode::Char('d'), KeyModifiers::CONTROL);
        a.jump_to(&y, 1);
        press(&mut a, KeyCode::PageDown, KeyModifiers::NONE);
        assert_eq!(a.history, [(x.clone(), 24, 0), (y.clone(), 24, 0)]);
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(at(&a), (x.clone(), 24));
        press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
        press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
        assert_eq!(at(&a), (y.clone(), 24));
        assert_eq!(a.history, [(x, 12, 0), (y, 24, 0)]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn wandering_after_going_back_moves_that_stop() {
        let (dir, mut a) = files_app("stale");
        let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
        a.jump_to(&x, 1);
        a.jump_to(&y, 1);
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        for _ in 0..4 {
            press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        }
        press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(
            at(&a),
            (x, 4),
            "back lands where the cursor was left, not where it arrived"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn find_next_is_a_stop_even_when_near() {
        let (dir, mut a) = files_app("find");
        let x = dir.join("a.rs");
        a.jump_to(&x, 1);
        find(&mut a, "x");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('n'), KeyModifiers::NONE);
        assert_eq!(a.history, [(x.clone(), 0, 0), (x, 1, 0)]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_jump_that_goes_nowhere_keeps_the_forward_history() {
        let (dir, mut a) = files_app("noop");
        let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
        a.jump_to(&x, 1);
        a.jump_to(&y, 1);
        a.jump_to(&y, 5);
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        a.jump_to(&x, 1);
        assert_eq!(a.history.len(), 3);
        press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
        assert_eq!(at(&a), (y, 0));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn enter_on_the_open_file_in_the_tree_keeps_the_cursor() {
        let (dir, mut a) = files_app("tree");
        let x = dir.join("a.rs");
        a.tree = crate::tree::build(&dir).0;
        a.jump_to(&x, 5); // also puts the tree cursor on a.rs
        a.focus = Focus::Tree;
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!((at(&a), a.focus), ((x.clone(), 4), Focus::Code));
        assert_eq!(a.history, [(x, 4, 0)]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Picking the open file in the file picker keeps the cursor, like Enter in the tree: the
    /// picker's items carry no line, and line 1 is not where the user was. The tree cursor
    /// still lands on the file.
    #[test]
    fn picking_the_open_file_keeps_the_cursor() {
        let (dir, mut a) = files_app("pick");
        let x = dir.join("a.rs");
        a.tree = crate::tree::build(&dir).0;
        a.files = vec![PathBuf::from("a.rs"), PathBuf::from("b.rs")];
        a.jump_to(&x, 5);
        a.tree.reveal(Path::new("b.rs"));
        a.focus = Focus::Tree;
        press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
        a.picker.as_mut().unwrap().settle();
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            (at(&a), a.focus, a.mode),
            ((x.clone(), 4), Focus::Code, Mode::Normal)
        );
        assert_eq!(a.tree.selected().unwrap().path, Path::new("a.rs"));
        assert_eq!(a.history, [(x, 4, 0)]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn history_keeps_the_last_fifty_stops() {
        let (dir, mut a) = files_app("cap");
        let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
        for _ in 0..30 {
            a.jump_to(&x, 1);
            a.jump_to(&y, 1);
        }
        assert_eq!((a.history.len(), a.hist_idx), (50, 49));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A scratch project on disk with its startup walk, for the definition and symbol tests.
    fn project_app(tag: &str, files: &[(&str, &str)]) -> (PathBuf, App) {
        let dir = std::env::temp_dir().join(format!("merl-nav-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (name, text) in files {
            let path = dir.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let (tree, files) = crate::tree::build(&dir);
        let app = App::new(dir.clone(), tree, files, Buffer::empty(), None);
        (dir, app)
    }

    #[test]
    fn infra_definitions_stay_in_their_scope() {
        let (dir, mut a) = project_app(
            "scope",
            &[
                (
                    "app/main.tf",
                    "resource \"aws_s3_bucket\" \"logs\" {\n  bucket = var.region\n}\n",
                ),
                ("app/variables.tf", "variable \"region\" {}\n"),
                ("vpc/variables.tf", "variable \"region\" {}\n"),
                (
                    "compose.yml",
                    "services:\n  web:\n    depends_on: [db-main]\n  db-main:\n    image: postgres\n",
                ),
                ("other.yml", "db-main:\n  x: 1\n"),
            ],
        );
        // On `region` in `var.region`: the variable of this module, not the other one.
        a.jump_to(&dir.join("app/main.tf"), 2);
        a.col = 16;
        press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("app/variables.tf"), 0), "{}", a.message);
        // On `db-main`, dash and all: the service in this file only.
        a.jump_to(&dir.join("compose.yml"), 3);
        a.col = 20;
        press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("compose.yml"), 3), "{}", a.message);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_field_has_no_definition_and_locals_must_be_direct() {
        let (dir, mut a) = project_app(
            "fallback",
            &[
                (
                    "order.rs",
                    "pub struct Order {\n    items: Vec<u8>,\n}\nfn add(order: &mut Order) {\n    order.items.push(1);\n}\n",
                ),
                (
                    "main.tf",
                    "locals {\n  name = \"x\"\n  tags = {\n    name = \"y\"\n  }\n}\nresource \"a\" \"b\" {\n  bucket = local.name\n}\n",
                ),
            ],
        );
        // A field is no declaration the Rust rules know, and `u` is the key for its uses.
        a.jump_to(&dir.join("order.rs"), 5);
        a.col = 10;
        press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("order.rs"), 4));
        assert_eq!(a.message, "no definition for items");
        // Of the two `name =` lines, only the one directly inside `locals` is `local.name`.
        a.jump_to(&dir.join("main.tf"), 8);
        a.col = 17;
        press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("main.tf"), 1), "{}", a.message);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// `json.load` is not declared in the project: `d` follows it into the standard library of
    /// the `python3` on this machine and opens it read-only. Skipped where there is none.
    #[test]
    fn definitions_outside_the_project_come_from_the_standard_library() {
        let stdlib = search::external_roots(Kind::Python, Path::new("/"));
        if stdlib.is_empty() {
            eprintln!("no python3, skipped");
            return;
        }
        let (dir, mut a) = project_app(
            "stdlib",
            &[(
                "main.py",
                "import json\n\ndef read(p):\n    return json.load(open(p))\n",
            )],
        );
        a.jump_to(&dir.join("main.py"), 4);
        a.col = 17;
        press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
        let (path, _) = at(&a);
        assert!(
            path.ends_with("json/__init__.py"),
            "{} {}",
            path.display(),
            a.message
        );
        assert!(a.line_str().starts_with("def load("), "{}", a.line_str());
        assert_eq!(a.buf.readonly, Some("outside the project"));
        assert_eq!(a.message, "load: via import json");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// One of the #68 projects in `tests/fixtures`, with nothing outside it, so the standard
    /// library of the machine running the tests has no say in what `d` offers.
    fn fixture_app(name: &str) -> App {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let (tree, files) = crate::tree::build(&dir);
        let mut a = App::new(dir, tree, files, Buffer::empty(), None);
        for kind in [Kind::Python, Kind::TsJs, Kind::Go] {
            a.external.insert(kind, (Vec::new(), Arc::new(Vec::new())));
        }
        a
    }

    /// Presses `d` on the last word of the first line of `file` that contains `code`, or starts
    /// with it when `code` starts with `^`.
    fn d_on(a: &mut App, file: &str, code: &str) {
        let path = a.root.join(file);
        let text = std::fs::read_to_string(&path).unwrap();
        let (n, line) = text
            .lines()
            .enumerate()
            .find(|(_, l)| match code.strip_prefix('^') {
                Some(start) => l.starts_with(start),
                None => l.contains(code),
            })
            .unwrap_or_else(|| panic!("no `{code}` in {file}"));
        let code = code.trim_start_matches('^');
        a.jump_to(&path, n + 1);
        a.col = line.find(code).unwrap() + code.rfind(|c: char| !is_word(c)).map_or(0, |i| i + 1);
        press(a, KeyCode::Char('d'), KeyModifiers::NONE);
    }

    /// The rows of the open picker, each split into its qualified name, its reason and the
    /// `path:line` it points at.
    fn definition_rows(a: &mut App) -> Vec<(String, String, String)> {
        let picker = a.picker.as_mut().expect("a picker");
        picker.settle();
        picker
            .window(50)
            .0
            .into_iter()
            .map(|r| {
                let at = r.item.code_at.unwrap();
                let head = &r.item.label[..at];
                let cols: Vec<&str> = head
                    .split("  ")
                    .map(str::trim)
                    .filter(|c| !c.is_empty())
                    .collect();
                let [name, why, place] = cols[..] else {
                    panic!("{head}");
                };
                (name.into(), why.into(), place.trim_end_matches(':').into())
            })
            .collect()
    }

    /// What `d` shows: the status line, and for a jump where it landed, for a picker its rows.
    #[derive(Debug, PartialEq)]
    enum Shown {
        Jump(String, String),
        Picker(String, Vec<(String, String, String)>),
    }

    fn shown(a: &mut App) -> Shown {
        match a.picker {
            Some(_) => {
                let rows = definition_rows(a);
                press(a, KeyCode::Esc, KeyModifiers::NONE);
                Shown::Picker(a.message.clone(), rows)
            }
            None => Shown::Jump(
                a.message.clone(),
                format!("{}:{}", a.rel_path(), a.line + 1),
            ),
        }
    }

    fn jump(status: &str, place: &str) -> Shown {
        Shown::Jump(status.into(), place.into())
    }

    fn picker(status: &str, rows: &[(&str, &str)]) -> Shown {
        let rows = rows
            .iter()
            .map(|(name, place)| (name.to_string(), "by name".to_string(), place.to_string()))
            .collect();
        Shown::Picker(status.into(), rows)
    }

    /// Steps 7 and 1 of #68 over the same project in three languages: on `x.word` with `x` a
    /// value whose type is not known, one member declaration of the name jumps and says it was
    /// found by name; several open a picker whose rows say what each is declared in and why it
    /// is there.
    #[test]
    fn a_member_of_a_value_is_found_by_name_and_says_so() {
        let two = |status: &str, first: (&str, &str), second: (&str, &str)| {
            picker(status, &[first, second])
        };
        let cases: [(&str, &str, &str, Shown); 6] = [
            // A parameter with no annotation.
            (
                "python",
                "factories.py",
                "repo.find_user",
                jump(
                    "find_user \u{2192} UserRepository.find_user (by name, 1 match)",
                    "repos.py:5",
                ),
            ),
            // Two classes declare the method: a picker, never a guess.
            (
                "python",
                "factories.py",
                "return repo.delete_user",
                two(
                    "delete_user: by name, 2 declarations",
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ),
            ),
            // `any` is no type of the project.
            (
                "typescript",
                "factories.ts",
                "repo.findUser",
                jump(
                    "findUser \u{2192} UserRepository.findUser (by name, 1 match)",
                    "repos.ts:6",
                ),
            ),
            (
                "typescript",
                "factories.ts",
                "void repo.deleteUser",
                two(
                    "deleteUser: by name, 2 declarations",
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ),
            ),
            // A `range` variable.
            (
                "go",
                "factories.go",
                "repo.FindUser",
                jump(
                    "FindUser \u{2192} UserRepository.FindUser (by name, 1 match)",
                    "repos.go:11",
                ),
            ),
            (
                "go",
                "factories.go",
                "^\t\trepo.DeleteUser",
                two(
                    "DeleteUser: by name, 2 declarations",
                    ("UserRepository.DeleteUser", "repos.go:15"),
                    ("AuditLog.DeleteUser", "repos.go:21"),
                ),
            ),
        ];
        for (fixture, file, code, want) in cases {
            let mut a = fixture_app(fixture);
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{fixture}: {code}");
        }
        // The picker is titled with what the status line says.
        let mut a = fixture_app("go");
        d_on(&mut a, "factories.go", "^\t\trepo.DeleteUser");
        assert_eq!(
            a.picker.as_ref().unwrap().title,
            "DeleteUser: by name, 2 declarations"
        );
    }

    /// Found by the acceptance pass of #68. On a declaration, the other declarations of the name
    /// are namesakes nothing ties to it: a second `d` after a proven jump used to leave
    /// `UserRepository.delete_user` for `AuditLog.delete_user` on its own, with `1 match`. They
    /// are offered in a picker that says where the cursor stands, even when there is one.
    #[test]
    fn a_declaration_does_not_jump_to_its_namesake() {
        let cases = [
            (
                "python",
                "repos.py",
                "async def delete_user",
                "delete_user",
                "AuditLog.delete_user",
                "repos.py:13",
            ),
            (
                "typescript",
                "repos.ts",
                "async deleteUser",
                "deleteUser",
                "AuditLog.deleteUser",
                "",
            ),
            (
                "go",
                "repos.go",
                ") DeleteUser",
                "DeleteUser",
                "AuditLog.DeleteUser",
                "repos.go:21",
            ),
        ];
        for (fixture, file, code, word, other, place) in cases {
            let mut a = fixture_app(fixture);
            d_on(&mut a, file, code);
            let Shown::Picker(status, rows) = shown(&mut a) else {
                panic!("{fixture}: a jump to {}", a.message);
            };
            assert_eq!(
                status,
                format!("{word}: at a declaration, 1 other by name"),
                "{fixture}"
            );
            assert_eq!(rows.len(), 1, "{fixture}");
            assert_eq!(rows[0].0, other, "{fixture}");
            assert!(
                place.is_empty() || rows[0].2 == place,
                "{fixture}: {}",
                rows[0].2
            );
        }
    }

    /// Steps 2 and 3 of #68 over the same project in three languages. A receiver whose every
    /// declaration in scope reads one type, directly or through the return type of one call, has
    /// its member looked up in that type: one jump, which says the link it followed. Two
    /// declarations that disagree, or a call whose return type is not written, leave the member
    /// to the search by name: a picker of two.
    #[test]
    fn a_member_of_a_typed_receiver_is_looked_up_in_its_type() {
        let by_name = |status: &str, first: (&str, &str), second: (&str, &str)| {
            picker(status, &[first, second])
        };
        let py_both = || {
            by_name(
                "delete_user: by name, 2 declarations",
                ("UserRepository.delete_user", "repos.py:8"),
                ("AuditLog.delete_user", "repos.py:13"),
            )
        };
        let ts_both = || {
            by_name(
                "deleteUser: by name, 2 declarations",
                ("UserRepository.deleteUser", "repos.ts:10"),
                ("AuditLog.deleteUser", "repos.ts:16"),
            )
        };
        let go_both = || {
            by_name(
                "DeleteUser: by name, 2 declarations",
                ("UserRepository.DeleteUser", "repos.go:15"),
                ("AuditLog.DeleteUser", "repos.go:21"),
            )
        };
        let cases: Vec<(&str, &str, &str, Shown)> = vec![
            // Two same-named methods: each field lands on its own class's.
            (
                "python",
                "service.py",
                "self.repo.delete_user",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via self.repo: UserRepository)",
                    "repos.py:8",
                ),
            ),
            (
                "python",
                "service.py",
                "self.audit.delete_user",
                jump(
                    "delete_user \u{2192} AuditLog.delete_user (via self.audit: AuditLog)",
                    "repos.py:13",
                ),
            ),
            // The declared type's method, not its implementations (step 6).
            (
                "python",
                "service.py",
                "self.notifier.send",
                jump(
                    "send \u{2192} Notifier.send (via self.notifier: Notifier)",
                    "repos.py:18",
                ),
            ),
            // Shadowing: below the nested function only the outer `repo` is in scope; inside it
            // both are, and they disagree.
            (
                "python",
                "service.py",
                "^    repo.delete_user",
                jump(
                    "delete_user \u{2192} AuditLog.delete_user (via repo: AuditLog)",
                    "repos.py:13",
                ),
            ),
            ("python", "service.py", "await repo.delete_user", py_both()),
            (
                "python",
                "factories.py",
                "repo.delete_user",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via make_repo() -> UserRepository)",
                    "repos.py:8",
                ),
            ),
            ("python", "factories.py", "audit.delete_user", py_both()),
            // The return type is resolved where the function is declared.
            (
                "python",
                "factories.py",
                "session.close",
                jump(
                    "close \u{2192} Session.close (via connect() -> Session)",
                    "store/sessions.py:6",
                ),
            ),
            (
                "typescript",
                "service.ts",
                "this.repo.deleteUser",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via this.repo: UserRepository)",
                    "repos.ts:10",
                ),
            ),
            (
                "typescript",
                "service.ts",
                "this.audit.deleteUser",
                jump(
                    "deleteUser \u{2192} AuditLog.deleteUser (via this.audit: AuditLog)",
                    "repos.ts:16",
                ),
            ),
            (
                "typescript",
                "service.ts",
                "this.notifier.send",
                jump(
                    "send \u{2192} Notifier.send (via this.notifier: Notifier)",
                    "repos.ts:22",
                ),
            ),
            (
                "typescript",
                "service.ts",
                "^  repo.deleteUser",
                jump(
                    "deleteUser \u{2192} AuditLog.deleteUser (via repo: AuditLog)",
                    "repos.ts:16",
                ),
            ),
            (
                "typescript",
                "service.ts",
                "await repo.deleteUser",
                ts_both(),
            ),
            (
                "typescript",
                "factories.ts",
                "await repo.deleteUser",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via makeRepo(): UserRepository)",
                    "repos.ts:10",
                ),
            ),
            // No return type, but the body constructs one.
            (
                "typescript",
                "factories.ts",
                "audit.deleteUser",
                jump(
                    "deleteUser \u{2192} AuditLog.deleteUser (via makeAudit() returns new AuditLog())",
                    "repos.ts:16",
                ),
            ),
            (
                "typescript",
                "factories.ts",
                "session.close",
                jump(
                    "close \u{2192} Session.close (via connect(): Session)",
                    "store/sessions.ts:6",
                ),
            ),
            (
                "go",
                "service.go",
                "s.repo.DeleteUser",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via s.repo: UserRepository)",
                    "repos.go:15",
                ),
            ),
            (
                "go",
                "service.go",
                "s.audit.DeleteUser",
                jump(
                    "DeleteUser \u{2192} AuditLog.DeleteUser (via s.audit: AuditLog)",
                    "repos.go:21",
                ),
            ),
            // An interface's method line.
            (
                "go",
                "service.go",
                "s.notifier.Send",
                jump(
                    "Send \u{2192} Notifier.Send (via s.notifier: Notifier)",
                    "repos.go:26",
                ),
            ),
            (
                "go",
                "service.go",
                "^\trepo.DeleteUser",
                jump(
                    "DeleteUser \u{2192} AuditLog.DeleteUser (via repo: AuditLog)",
                    "repos.go:21",
                ),
            ),
            ("go", "service.go", "^\t\trepo.DeleteUser", go_both()),
            (
                "go",
                "factories.go",
                "repo.DeleteUser",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via NewRepo() *UserRepository)",
                    "repos.go:15",
                ),
            ),
            // The first result of two.
            (
                "go",
                "factories.go",
                "audit.DeleteUser",
                jump(
                    "DeleteUser \u{2192} AuditLog.DeleteUser (via NewAudit() AuditLog)",
                    "repos.go:21",
                ),
            ),
            // A function of an imported package, whose result is a type of that package.
            (
                "go",
                "factories.go",
                "session.Close",
                jump(
                    "Close \u{2192} Session.Close (via store.Open() *Session)",
                    "store/store.go:15",
                ),
            ),
        ];
        for (fixture, file, code, want) in cases {
            let mut a = fixture_app(fixture);
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{fixture}: {file}: {code}");
        }
    }

    /// Step 4 of #68 over the same project in three languages. A chain is followed one field at a
    /// time, each through the type before it (a Go field may be promoted from an embedded struct),
    /// and a jump lists the links. A link that cannot be proven, or a seventh name, falls back to
    /// the search by name and says where the chain broke. A chain that hangs off a call has no
    /// names to follow.
    #[test]
    fn a_chain_is_followed_link_by_link() {
        let by_name = |status: &str, rows: &[(&str, &str)]| picker(status, rows);
        let folders = |root: &str, field: &str, ty: &str| {
            let tail = format!(" \u{2192} {field}: {ty}").repeat(4);
            format!("{root} \u{2192} {ty}.{root} (via folder.{field}: {ty}{tail})")
        };
        let py_both = [
            ("UserRepository.delete_user", "repos.py:8"),
            ("AuditLog.delete_user", "repos.py:13"),
        ];
        let ts_both = [
            ("UserRepository.deleteUser", "repos.ts:10"),
            ("AuditLog.deleteUser", "repos.ts:16"),
        ];
        let go_both = [
            ("UserRepository.DeleteUser", "repos.go:15"),
            ("AuditLog.DeleteUser", "repos.go:21"),
        ];
        let cases: Vec<(&str, &str, &str, Shown)> = vec![
            // A parameter, then a field handed on from a constructor parameter, twice.
            (
                "python",
                "service.py",
                "app.services.users.remove",
                jump(
                    "remove \u{2192} UserService.remove (via app.services: Services \u{2192} users: UserService)",
                    "service.py:10",
                ),
            ),
            // Two same-named methods: each field of the unit of work lands on its own.
            (
                "python",
                "chains.py",
                "self.uow.users.delete_user",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via self.uow: UnitOfWork \u{2192} users: UserRepository)",
                    "repos.py:8",
                ),
            ),
            (
                "python",
                "chains.py",
                "self.uow.audit.delete_user",
                jump(
                    "delete_user \u{2192} AuditLog.delete_user (via self.uow: UnitOfWork \u{2192} audit: AuditLog)",
                    "repos.py:13",
                ),
            ),
            // A type parameter is not the type argument.
            (
                "python",
                "chains.py",
                "self.box.item.delete_user",
                by_name(
                    "delete_user: by name, 2 declarations (chain broke at item)",
                    &py_both,
                ),
            ),
            // Not the local `users`: the chain hangs off a call.
            (
                "python",
                "chains.py",
                "make_uow().users.delete_user",
                by_name("delete_user: by name, 2 declarations", &py_both),
            ),
            (
                "python",
                "chains.py",
                "folder.parent.parent.parent.parent.parent.root",
                jump(&folders("root", "parent", "Folder"), "chains.py:23"),
            ),
            (
                "python",
                "chains.py",
                "folder.parent.parent.parent.parent.parent.parent.root",
                jump(
                    "root \u{2192} Folder.root (by name, 1 match, chain broke at parent)",
                    "chains.py:23",
                ),
            ),
            (
                "typescript",
                "service.ts",
                "app.services.users.remove",
                jump(
                    "remove \u{2192} UserService.remove (via app.services: Services \u{2192} users: UserService)",
                    "service.ts:11",
                ),
            ),
            (
                "typescript",
                "chains.ts",
                "this.uow.users.deleteUser",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via this.uow: UnitOfWork \u{2192} users: UserRepository)",
                    "repos.ts:10",
                ),
            ),
            (
                "typescript",
                "chains.ts",
                "this.uow.audit.deleteUser",
                jump(
                    "deleteUser \u{2192} AuditLog.deleteUser (via this.uow: UnitOfWork \u{2192} audit: AuditLog)",
                    "repos.ts:16",
                ),
            ),
            (
                "typescript",
                "chains.ts",
                "this.box.item.deleteUser",
                by_name(
                    "deleteUser: by name, 2 declarations (chain broke at item)",
                    &ts_both,
                ),
            ),
            (
                "typescript",
                "chains.ts",
                "makeUow().users.deleteUser",
                by_name("deleteUser: by name, 2 declarations", &ts_both),
            ),
            (
                "typescript",
                "chains.ts",
                "folder.parent.parent.parent.parent.parent.root",
                jump(&folders("root", "parent", "Folder"), "chains.ts:15"),
            ),
            (
                "typescript",
                "chains.ts",
                "folder.parent.parent.parent.parent.parent.parent.root",
                jump(
                    "root \u{2192} Folder.root (by name, 1 match, chain broke at parent)",
                    "chains.ts:15",
                ),
            ),
            // Pointer fields.
            (
                "go",
                "service.go",
                "app.Services.Users.Remove",
                jump(
                    "Remove \u{2192} UserService.Remove (via app.Services: Services \u{2192} Users: UserService)",
                    "service.go:11",
                ),
            ),
            // A field promoted from the embedded `*Deps`.
            (
                "go",
                "chains.go",
                "h.uow.Users.DeleteUser",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via h.uow: UnitOfWork \u{2192} Users: UserRepository)",
                    "repos.go:15",
                ),
            ),
            // The embedded struct named as a link.
            (
                "go",
                "chains.go",
                "h.Deps.uow.Audit.DeleteUser",
                jump(
                    "DeleteUser \u{2192} AuditLog.DeleteUser (via h.Deps: Deps \u{2192} uow: UnitOfWork \u{2192} Audit: AuditLog)",
                    "repos.go:21",
                ),
            ),
            (
                "go",
                "chains.go",
                "h.box.Item.DeleteUser",
                by_name(
                    "DeleteUser: by name, 2 declarations (chain broke at Item)",
                    &go_both,
                ),
            ),
            (
                "go",
                "chains.go",
                "NewUnitOfWork().Users.DeleteUser",
                by_name("DeleteUser: by name, 2 declarations", &go_both),
            ),
            (
                "go",
                "chains.go",
                "folder.Parent.Parent.Parent.Parent.Parent.Root",
                jump(&folders("Root", "Parent", "Folder"), "chains.go:16"),
            ),
            (
                "go",
                "chains.go",
                "folder.Parent.Parent.Parent.Parent.Parent.Parent.Root",
                jump(
                    "Root \u{2192} Folder.Root (by name, 1 match, chain broke at Parent)",
                    "chains.go:16",
                ),
            ),
        ];
        for (fixture, file, code, want) in cases {
            let mut a = fixture_app(fixture);
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{fixture}: {file}: {code}");
        }
    }

    /// Step 6 of #68 over the same project in three languages: on the declaration of a member of
    /// an interface, a protocol, an abstract or a base class, `d` offers what implements it,
    /// labelled with the member it comes from. A type that inherits the member without declaring
    /// it, a member of another number of parameters and a declaration nothing implements are left
    /// out, and the last of those falls back to the search by name.
    #[test]
    fn implementations_are_offered_on_the_declaration_they_implement() {
        let impls = |status: &str, member: &str, rows: &[(&str, &str)]| {
            let why = format!("implementations of {member}");
            let rows = rows
                .iter()
                .map(|(name, place)| (name.to_string(), why.clone(), place.to_string()))
                .collect();
            Shown::Picker(status.into(), rows)
        };
        let cases: [(&str, &str, &str, Shown); 9] = [
            // A base class: the subclasses that override it, `NightlyJob` two levels down.
            // `QuietJob` inherits `run` without declaring it and is no implementation.
            (
                "python",
                "impls.py",
                "def run",
                impls(
                    "run: implementations of BaseJob.run, 3 declarations",
                    "BaseJob.run",
                    &[
                        ("ImportJob.run", "impls.py:10"),
                        ("ExportJob.run", "impls.py:15"),
                        ("NightlyJob.run", "impls.py:24"),
                    ],
                ),
            ),
            // A protocol is structural: `WebhookNotifier` names nothing and implements it,
            // `Batch.send` takes another parameter and does not.
            (
                "python",
                "repos.py",
                "def send",
                impls(
                    "send: implementations of Notifier.send, 4 declarations",
                    "Notifier.send",
                    &[
                        ("EmailNotifier.send", "repos.py:22"),
                        ("SmsNotifier.send", "repos.py:27"),
                        ("LoudNotifier.send", "impls.py:29"),
                        ("WebhookNotifier.send", "impls.py:34"),
                    ],
                ),
            ),
            (
                "typescript",
                "impls.ts",
                "^  run",
                impls(
                    "run: implementations of BaseJob.run, 4 declarations",
                    "BaseJob.run",
                    &[
                        ("ImportJob.run", "impls.ts:8"),
                        ("ExportJob.run", "impls.ts:12"),
                        // A header prettier wrapped over three lines is read all the same.
                        ("WrappedJob.run", "impls.ts:39"),
                        ("NightlyJob.run", "impls.ts:18"),
                    ],
                ),
            ),
            // `implements` in the same file and behind an import.
            (
                "typescript",
                "repos.ts",
                "^  send",
                impls(
                    "send: implementations of Notifier.send, 4 declarations",
                    "Notifier.send",
                    &[
                        ("EmailNotifier.send", "repos.ts:26"),
                        ("SmsNotifier.send", "repos.ts:32"),
                        ("LoudNotifier.send", "impls.ts:22"),
                        ("WrappedJob.send", "impls.ts:41"),
                    ],
                ),
            ),
            // Go's interfaces are implicit: the method name and the number of parameters are all
            // there is to go on, and `Batch.Run` takes one more.
            (
                "go",
                "impls.go",
                "Run",
                impls(
                    "Run: implementations of Job.Run, 2 declarations",
                    "Job.Run",
                    &[
                        ("ImportJob.Run", "impls.go:11"),
                        ("ExportJob.Run", "impls.go:17"),
                    ],
                ),
            ),
            (
                "go",
                "repos.go",
                "Send",
                impls(
                    "Send: implementations of Notifier.Send, 3 declarations",
                    "Notifier.Send",
                    &[
                        ("EmailNotifier.Send", "repos.go:31"),
                        ("SmsNotifier.Send", "repos.go:37"),
                        ("LoudNotifier.Send", "impls.go:29"),
                    ],
                ),
            ),
            // One implementation is an answer, not a one-row picker, and the status line says
            // where it came from.
            (
                "typescript",
                "impls.ts",
                "^  sweep",
                jump(
                    "sweep \u{2192} NightlySweeper.sweep (implementations of Sweeper.sweep)",
                    "impls.ts:32",
                ),
            ),
            // Nothing implements a Go method beside its type: the search by name answers, and
            // says the cursor is on one of them.
            (
                "go",
                "repos.go",
                "func (e *EmailNotifier) Send",
                picker(
                    "Send: at a declaration, 2 others by name",
                    &[
                        ("SmsNotifier.Send", "repos.go:37"),
                        ("LoudNotifier.Send", "impls.go:29"),
                    ],
                ),
            ),
            // A member of a value still resolves through the type of the receiver, not through
            // the implementations of the interface it lands on.
            (
                "python",
                "service.py",
                "self.notifier.send",
                jump(
                    "send \u{2192} Notifier.send (via self.notifier: Notifier)",
                    "repos.py:18",
                ),
            ),
        ];
        for (fixture, file, code, want) in cases {
            let mut a = fixture_app(fixture);
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{fixture}: {file}: {code}");
        }
    }

    /// The two presses #68 step 6 is reached by: `d` on a call of an interface method lands on
    /// the declaration with the cursor on its name, and a second `d` there lists what implements
    /// it. A jump that left the cursor at the start of the line would answer nothing.
    #[test]
    fn a_second_d_on_the_declaration_a_jump_landed_on_lists_the_implementations() {
        let mut a = fixture_app("python");
        d_on(&mut a, "service.py", "self.notifier.send");
        assert_eq!(
            a.message,
            "send \u{2192} Notifier.send (via self.notifier: Notifier)"
        );
        assert_eq!(format!("{}:{}", a.rel_path(), a.line + 1), "repos.py:18");
        assert_eq!(&a.line_str()[a.col..a.col + 4], "send", "on the word");
        press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(
            a.message,
            "send: implementations of Notifier.send, 4 declarations"
        );
    }

    /// Step 5 of #68 over the same project in three languages: a word or a qualifier an import
    /// binds to a module of the project is looked for in that module, and the status line names
    /// the file or the package directory. A module that does not declare the word re-exports it,
    /// and the search by name answers, as before.
    #[test]
    fn an_import_inside_the_project_is_looked_up_in_its_module() {
        let cases: [(&str, &str, &str, Shown); 13] = [
            // `fakes.py` declares a `UserRepository` too.
            (
                "python",
                "jobs.py",
                "repo: UserRepository",
                jump("UserRepository: via import repos.py", "repos.py:4"),
            ),
            // A package is its `__init__.py`.
            (
                "python",
                "jobs.py",
                "    connect",
                jump(
                    "connect: via import store/__init__.py",
                    "store/__init__.py:6",
                ),
            ),
            (
                "python",
                "jobs.py",
                "    open_session",
                picker(
                    "open_session: by name, 2 declarations",
                    &[
                        ("open_session", "fakes.py:6"),
                        ("open_session", "store/sessions.py:16"),
                    ],
                ),
            ),
            (
                "python",
                "jobs.py",
                "sessions.open_session",
                jump(
                    "open_session: via import store/sessions.py",
                    "store/sessions.py:16",
                ),
            ),
            // Through an aliased class: its own `start`, not `Pool.start` in the same module.
            (
                "python",
                "jobs.py",
                "StoreSession.start",
                jump(
                    "start \u{2192} Session.start (via import store/sessions.py)",
                    "store/sessions.py:3",
                ),
            ),
            (
                "python",
                "store/__init__.py",
                "return open_session",
                jump(
                    "open_session: via import store/sessions.py",
                    "store/sessions.py:16",
                ),
            ),
            (
                "typescript",
                "jobs.ts",
                "repo: UserRepository",
                jump("UserRepository: via import repos.ts", "repos.ts:5"),
            ),
            // A default import under another name: the module's `export default`.
            (
                "typescript",
                "jobs.ts",
                "  connectToStore",
                jump(
                    "connectToStore: via import store/index.ts",
                    "store/index.ts:5",
                ),
            ),
            (
                "typescript",
                "jobs.ts",
                "  openSession",
                picker(
                    "openSession: by name, 2 declarations",
                    &[
                        ("openSession", "fakes.ts:5"),
                        ("openSession", "store/sessions.ts:15"),
                    ],
                ),
            ),
            // A namespace behind the `@/` alias of tsconfig.json.
            (
                "typescript",
                "jobs.ts",
                "sessions.openSession",
                jump(
                    "openSession: via import store/sessions.ts",
                    "store/sessions.ts:15",
                ),
            ),
            (
                "typescript",
                "jobs.ts",
                "StoreSession.start",
                jump(
                    "start \u{2192} Session.start (via import store/sessions.ts)",
                    "store/sessions.ts:2",
                ),
            ),
            (
                "typescript",
                "store/index.ts",
                "return openSession",
                jump(
                    "openSession: via import store/sessions.ts",
                    "store/sessions.ts:15",
                ),
            ),
            // The package function, not the method `Session.Open` nor `fakes.Open`.
            (
                "go",
                "jobs.go",
                "store.Open",
                jump("Open: via import store/", "store/store.go:7"),
            ),
        ];
        for (fixture, file, code, want) in cases {
            let mut a = fixture_app(fixture);
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{fixture}: {code}");
        }
    }

    /// A type that does not declare the member itself hands it to the class it extends or the
    /// struct it embeds; a field comes down from a base class the same way. The status line keeps
    /// the receiver's own type.
    #[test]
    fn a_typed_receiver_finds_what_its_type_extends() {
        type Probe = (&'static str, &'static str, Shown);
        type Project = (Kind, &'static [(&'static str, &'static str)], Vec<Probe>);
        let projects: [Project; 3] = [
            (
                Kind::Python,
                &[
                    (
                        "base.py",
                        "class BaseRepo:\n    def find(self, key):\n        pass\n",
                    ),
                    (
                        "repos.py",
                        "from base import BaseRepo\n\n\nclass UserRepo(BaseRepo):\n    pass\n\n\nclass Other:\n    def find(self, key):\n        pass\n",
                    ),
                    (
                        "main.py",
                        "from repos import UserRepo\n\n\ndef run(repo: UserRepo):\n    repo.find(1)\n\n\nclass BaseService:\n    def __init__(self, repo: UserRepo):\n        self.repo = repo\n\n\nclass Service(BaseService):\n    def run(self):\n        self.repo.find(1)\n",
                    ),
                ],
                vec![
                    (
                        "main.py",
                        "^    repo.find",
                        jump(
                            "find \u{2192} BaseRepo.find (via repo: UserRepo)",
                            "base.py:2",
                        ),
                    ),
                    (
                        "main.py",
                        "self.repo.find",
                        jump(
                            "find \u{2192} BaseRepo.find (via self.repo: UserRepo)",
                            "base.py:2",
                        ),
                    ),
                ],
            ),
            (
                Kind::TsJs,
                &[
                    (
                        "base.ts",
                        "export class BaseRepo {\n  find(key: string): void {}\n}\n",
                    ),
                    (
                        "repos.ts",
                        "import { BaseRepo } from \"./base\";\n\nexport class UserRepo extends BaseRepo {}\n\nexport class Other {\n  find(key: string): void {}\n}\n",
                    ),
                    (
                        "main.ts",
                        "import { UserRepo } from \"./repos\";\n\nexport function run(repo: UserRepo): void {\n  repo.find(\"a\");\n}\n",
                    ),
                ],
                vec![(
                    "main.ts",
                    "repo.find",
                    jump(
                        "find \u{2192} BaseRepo.find (via repo: UserRepo)",
                        "base.ts:2",
                    ),
                )],
            ),
            (
                Kind::Go,
                &[
                    ("go.mod", "module example.com/inherit\n"),
                    (
                        "base.go",
                        "package main\n\ntype Base struct{}\n\nfunc (b *Base) Find(key string) {}\n",
                    ),
                    (
                        "repos.go",
                        "package main\n\ntype UserRepo struct {\n\t*Base\n\tname string\n}\n\ntype Other struct{}\n\nfunc (o Other) Find(key string) {}\n",
                    ),
                    (
                        "main.go",
                        "package main\n\nfunc run(repo *UserRepo) {\n\trepo.Find(\"a\")\n}\n",
                    ),
                ],
                vec![(
                    "main.go",
                    "repo.Find",
                    jump("Find \u{2192} Base.Find (via repo: UserRepo)", "base.go:5"),
                )],
            ),
        ];
        for (kind, files, probes) in projects {
            let (dir, mut a) = project_app(&format!("extends-{kind:?}"), files);
            a.external.insert(kind, (Vec::new(), Arc::new(Vec::new())));
            for (file, code, want) in probes {
                d_on(&mut a, file, code);
                assert_eq!(shown(&mut a), want, "{file}: {code}");
            }
            std::fs::remove_dir_all(&dir).unwrap();
        }
    }

    #[test]
    fn a_member_of_a_value_is_looked_for_outside_the_project_too() {
        let mut a = fixture_app("python");
        let site = std::env::temp_dir().join(format!("merl-site-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&site);
        std::fs::create_dir_all(site.join("client")).unwrap();
        std::fs::write(
            site.join("client/api.py"),
            "class Client:\n    def find_user(self, user_id):\n        pass\n",
        )
        .unwrap();
        a.external.insert(
            Kind::Python,
            (
                vec![site.clone()],
                Arc::new(vec![site.join("client/api.py")]),
            ),
        );
        d_on(&mut a, "factories.py", "repo.find_user");
        // The project first; a dependency shown relative to its root.
        assert_eq!(
            shown(&mut a),
            picker(
                "find_user: by name, 2 declarations",
                &[
                    ("UserRepository.find_user", "repos.py:5"),
                    ("Client.find_user", "client/api.py:2"),
                ],
            )
        );
        // TypeScript outside the project is read from its declarations, not the bundle.
        let mut a = fixture_app("typescript");
        let (dts, js) = (site.join("client/index.d.ts"), site.join("client/index.js"));
        std::fs::write(
            &dts,
            "export declare class Client {\n    findUser(id: number): unknown;\n}\n",
        )
        .unwrap();
        std::fs::write(&js, "class Client {\n  findUser(id) {\n  }\n}\n").unwrap();
        a.external
            .insert(Kind::TsJs, (vec![site.clone()], Arc::new(vec![dts, js])));
        d_on(&mut a, "factories.ts", "repo.findUser");
        assert_eq!(
            shown(&mut a),
            picker(
                "findUser: by name, 2 declarations",
                &[
                    ("UserRepository.findUser", "repos.ts:6"),
                    ("Client.findUser", "client/index.d.ts:2"),
                ],
            )
        );
        std::fs::remove_dir_all(&site).unwrap();
    }

    /// A standard library or dependency root on disk, with `files` in it, for the lookups
    /// outside the project; removed by the caller.
    fn external_root(tag: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("merl-root-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (name, text) in files {
            let path = dir.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        dir
    }

    /// Makes `roots` the only places outside the project `d` looks in for `kind`.
    fn use_roots(a: &mut App, kind: Kind, roots: &[PathBuf]) {
        let files = search::external_files(kind, roots);
        a.external.insert(kind, (roots.to_vec(), Arc::new(files)));
    }

    /// Found by the acceptance pass of #68: three ways `d` claimed more than it knew.
    #[test]
    fn a_local_name_is_not_an_import_and_a_member_is_not_a_module_level_name() {
        let (dir, mut a) = project_app(
            "acceptance",
            &[
                ("pkg/__init__.py", ""),
                ("pkg/b.py", "def helper():\n    pass\n"),
                // A parameter called like an imported module, a local called like an import.
                (
                    "shadow.py",
                    "import json\nfrom pkg.b import helper\n\n\ndef handler(json):\n    return json.loads(1)\n\n\ndef f():\n    helper = 3\n    return helper\n",
                ),
                // Module-level names are no members of a value.
                (
                    "member.py",
                    "zqwidget = 5\n\n\ndef zqmake(x):\n    return x\n\n\ndef g(client):\n    client.zqwidget\n    return client.zqmake(1)\n",
                ),
                // The package's own `pick` is native; a method of that name is not it.
                (
                    "outside.py",
                    "from fakelib import pick\nfrom fakelib.core import make\n\n\ndef run():\n    make()\n    return pick(1)\n",
                ),
            ],
        );
        let root = external_root(
            "acceptance",
            &[
                ("json/__init__.py", "def loads(s):\n    pass\n"),
                ("fakelib/__init__.py", "from fakelib._native import pick\n"),
                (
                    "fakelib/core.py",
                    "class Thing:\n    def pick(self, x):\n        return x\n\n\ndef make():\n    pass\n",
                ),
            ],
        );
        use_roots(&mut a, Kind::Python, std::slice::from_ref(&root));
        d_on(&mut a, "shadow.py", "json.loads");
        assert!(!a.message.contains("via import"), "{}", a.message);
        d_on(&mut a, "shadow.py", "return helper");
        assert!(!a.message.contains("via import"), "{}", a.message);
        assert_eq!(a.rel_path(), "shadow.py");
        for code in ["client.zqwidget", "client.zqmake"] {
            d_on(&mut a, "member.py", code);
            assert!(
                a.message.starts_with("no definition for zq"),
                "{}",
                a.message
            );
        }
        d_on(&mut a, "outside.py", "return pick");
        assert_eq!(a.message, "no definition for pick");
        assert_eq!(a.rel_path(), "outside.py");
        // What the module does declare at its top is still found through the import.
        d_on(&mut a, "outside.py", "    make");
        assert_eq!(a.message, "make: via import fakelib.core");
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Found by the acceptance pass of #68: a line inside a raw string, a docstring or a block
    /// comment declares nothing, however much it reads like a declaration.
    #[test]
    fn a_declaration_inside_a_literal_is_not_one() {
        let (dir, mut a) = project_app(
            "literals",
            &[
                ("go.mod", "module lit\n"),
                (
                    "misc.go",
                    "package lit\n\ntype Target struct{}\n\nconst snippet = `\nfunc (t Target) InString() string { return \"\" }\n`\n\n/*\nfunc (t Target) InBlock() string { return \"\" }\n*/\n",
                ),
                (
                    "use.go",
                    "package lit\n\nfunc Use(t Target) {\n\t_ = t.InString()\n\t_ = t.InBlock()\n}\n",
                ),
                (
                    "a.py",
                    "SRC = \"\"\"\nclass Fake:\n    def ghost(self):\n        pass\n\"\"\"\n\n\nclass Real:\n    def ghost(self):\n        pass\n\n\ndef call(g):\n    return g.ghost()\n",
                ),
            ],
        );
        for kind in [Kind::Python, Kind::Go] {
            a.external.insert(kind, (Vec::new(), Arc::new(Vec::new())));
        }
        d_on(&mut a, "use.go", "t.InString");
        assert_eq!(a.message, "no definition for InString");
        d_on(&mut a, "use.go", "t.InBlock");
        assert_eq!(a.message, "no definition for InBlock");
        d_on(&mut a, "a.py", "g.ghost");
        assert_eq!(
            shown(&mut a),
            jump("ghost \u{2192} Real.ghost (by name, 1 match)", "a.py:9")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Found by the acceptance pass of #68: `Outer.find` names what `Outer` declares, and a
    /// method `find` of some class elsewhere used to take the jump with `1 match`.
    #[test]
    fn a_namespace_or_a_class_in_front_of_the_word_names_where_it_is_declared() {
        let (dir, mut a) = project_app(
            "namespaces",
            &[
                (
                    "ns.ts",
                    "export namespace Outer {\n  export function find(id: string) {\n    return id;\n  }\n}\n\nexport function run() {\n  return Outer.find(\"1\");\n}\n",
                ),
                (
                    "other.ts",
                    "export class Repo {\n  find(id: string) {\n    return id;\n  }\n}\n",
                ),
            ],
        );
        a.external
            .insert(Kind::TsJs, (Vec::new(), Arc::new(Vec::new())));
        d_on(&mut a, "ns.ts", "Outer.find");
        assert_eq!(
            shown(&mut a),
            jump("find \u{2192} Outer.find (via Outer)", "ns.ts:2")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Found by the acceptance pass of #68: a Go method of the same name and as many parameters,
    /// of other types, was the one implementation `d` jumped to.
    #[test]
    fn a_go_method_of_other_types_implements_nothing() {
        let (dir, mut a) = project_app(
            "arity",
            &[
                ("go.mod", "module arity\n"),
                (
                    "iface.go",
                    "package arity\n\ntype Solo interface {\n\tFerry(a string) error\n}\n\ntype NotSolo struct{}\n\nfunc (n NotSolo) Ferry(a int) string { return \"\" }\n\ntype Real struct{}\n\nfunc (r *Real) Ferry(name string) error { return nil }\n",
                ),
            ],
        );
        a.external
            .insert(Kind::Go, (Vec::new(), Arc::new(Vec::new())));
        d_on(&mut a, "iface.go", "\tFerry");
        assert_eq!(
            shown(&mut a),
            jump(
                "Ferry \u{2192} Real.Ferry (implementations of Solo.Ferry)",
                "iface.go:13"
            )
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Found by the acceptance pass of #68: two interfaces of one name in two files, and the
    /// class that implements the other one was the implementation `d` jumped to.
    #[test]
    fn an_implementation_of_a_namesake_interface_is_not_one_of_ours() {
        let iface = "export interface Notifier {\n  send(to: string): void;\n}\n";
        let (dir, mut a) = project_app(
            "dupiface",
            &[
                ("a.ts", iface),
                ("b.ts", iface),
                ("index.ts", "export * from \"./a\";\n"),
                (
                    "impl.ts",
                    "import { Notifier } from \"./a\";\n\nexport class MailNotifier implements Notifier {\n  send(to: string): void {}\n}\n",
                ),
                (
                    "barrel.ts",
                    "import { Notifier } from \"./index\";\n\nexport class SmsNotifier implements Notifier {\n  send(to: string): void {}\n}\n",
                ),
            ],
        );
        a.external
            .insert(Kind::TsJs, (Vec::new(), Arc::new(Vec::new())));
        // Through a barrel the file is not known: the class stays, as it was.
        d_on(&mut a, "b.ts", "  send");
        assert_eq!(
            shown(&mut a),
            jump(
                "send \u{2192} SmsNotifier.send (implementations of Notifier.send)",
                "barrel.ts:4"
            )
        );
        d_on(&mut a, "a.ts", "  send");
        let Shown::Picker(status, rows) = shown(&mut a) else {
            panic!("{}", a.message);
        };
        assert_eq!(
            status,
            "send: implementations of Notifier.send, 2 declarations"
        );
        assert_eq!(rows.len(), 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Found by the acceptance pass of #68 in hono: a member written as a property was missing
    /// from the implementations, and the one left was jumped to as if it were the only one.
    #[test]
    fn a_property_holding_the_function_implements_the_method() {
        let (dir, mut a) = project_app(
            "propmember",
            &[
                (
                    "iface.ts",
                    "export interface Router {\n  match(method: string): string;\n}\n",
                ),
                (
                    "impl.ts",
                    "import type { Router } from \"./iface\";\n\ndeclare const match: (method: string) => string;\n\nexport class RegExpRouter implements Router {\n  match: typeof match = match;\n}\n\nexport class TrieRouter implements Router {\n  match(method: string): string {\n    const o = {\n      match: 1\n    };\n    return method;\n  }\n}\n",
                ),
            ],
        );
        a.external
            .insert(Kind::TsJs, (Vec::new(), Arc::new(Vec::new())));
        d_on(&mut a, "iface.ts", "  match");
        let Shown::Picker(status, rows) = shown(&mut a) else {
            panic!("{}", a.message);
        };
        assert_eq!(
            status,
            "match: implementations of Router.match, 2 declarations"
        );
        let places: Vec<&str> = rows.iter().map(|r| r.2.as_str()).collect();
        assert_eq!(places, ["impl.ts:6", "impl.ts:10"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Found by the acceptance pass of #68: a name imported from two modules, one per branch of
    /// a `try`, jumped to the first as if there were no second.
    #[test]
    fn a_name_imported_from_two_modules_offers_both() {
        let (dir, mut a) = project_app(
            "twice",
            &[
                ("pkg/__init__.py", ""),
                ("pkg/a.py", "def pick(x):\n    return x\n"),
                ("pkg/b.py", "def pick(x):\n    return x\n"),
                (
                    "use.py",
                    "try:\n    from pkg.a import pick\nexcept ImportError:\n    from pkg.b import pick\n\n\ndef run():\n    return pick(1)\n",
                ),
            ],
        );
        a.external
            .insert(Kind::Python, (Vec::new(), Arc::new(Vec::new())));
        d_on(&mut a, "use.py", "return pick");
        let Shown::Picker(status, rows) = shown(&mut a) else {
            panic!("{}", a.message);
        };
        assert_eq!(status, "pick: 2 declarations");
        let places: Vec<&str> = rows.iter().map(|r| r.2.as_str()).collect();
        assert_eq!(places, ["pkg/a.py:1", "pkg/b.py:1"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A qualifier bound by a relative import, or a class of the project, is no value: `d`
    /// finds the module-level declaration and does not add a dependency's same-named methods.
    #[test]
    fn a_module_or_a_class_in_front_of_the_word_is_not_a_value() {
        let (dir, mut a) = project_app(
            "modules",
            &[
                ("shop/views.py", "def index(request):\n    return 1\n"),
                (
                    "shop/urls.py",
                    "from . import views\n\nurlpatterns = [views.index, views.missing]\n",
                ),
                (
                    "web/utils.ts",
                    "export function formatDate(d: Date): string {\n  return d.toISOString();\n}\n",
                ),
                (
                    "web/app.ts",
                    "import * as utils from \"./utils\";\n\nexport const today = utils.formatDate(new Date());\n",
                ),
                (
                    "shapes.py",
                    "class Outer:\n    class Inner:\n        pass\n\n\nx = Outer.Inner()\n",
                ),
            ],
        );
        let root = external_root(
            "modules",
            &[
                (
                    "site/admin.py",
                    "class AdminSite:\n    def index(self, request):\n        pass\n",
                ),
                // A dependency with a `views` package must not answer for the project's `views`.
                (
                    "django/views/generic.py",
                    "def missing(request):\n    pass\n",
                ),
                (
                    "node/fmt.d.ts",
                    "export declare class Fmt {\n  formatDate(d: Date): string;\n}\n",
                ),
            ],
        );
        use_roots(&mut a, Kind::Python, std::slice::from_ref(&root));
        use_roots(&mut a, Kind::TsJs, std::slice::from_ref(&root));
        for (file, code, want) in [
            (
                "shop/urls.py",
                "views.index",
                jump("index: via import shop/views.py", "shop/views.py:1"),
            ),
            (
                "shop/urls.py",
                "views.missing",
                jump("no definition for missing", "shop/urls.py:3"),
            ),
            (
                "web/app.ts",
                "utils.formatDate",
                jump("formatDate: via import web/utils.ts", "web/utils.ts:1"),
            ),
            (
                "shapes.py",
                "Outer.Inner",
                jump("Inner \u{2192} Outer.Inner (via Outer)", "shapes.py:2"),
            ),
        ] {
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{code}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// What `d` says about a lookup outside the project: the module an import or a path names,
    /// and "by name" once the search is no longer inside it.
    #[test]
    fn a_module_lookup_says_which_module_or_that_it_went_by_name() {
        let (dir, mut a) = project_app(
            "outside",
            &[
                (
                    "main.go",
                    "package main\n\nimport (\n\t\"github.com/foo/bar\"\n\t\"gopkg.in/yaml.v3\"\n)\n\nfunc main() {\n\t_ = yaml.Unmarshal(nil, nil)\n\tbar.Baz()\n}\n",
                ),
                (
                    "main.rs",
                    "fn main() {\n    let s = String::new().into_owned();\n    path.join(\"y\");\n    std::fs::read_to_string(\"x\");\n}\n",
                ),
                (
                    "m.py",
                    "import os\nimport jsonx\n\nos.path.join('a', 'b')\njsonx.dumps_x(1)\n",
                ),
                // The project's own `dumps_x` is not the one `import jsonx` names.
                ("codec.py", "def dumps_x(o):\n    return o\n"),
                (
                    "app.ts",
                    "import { Hono } from 'hono';\n\nexport const app = new Hono();\n",
                ),
            ],
        );
        let root = external_root(
            "outside",
            &[
                (
                    "gopkg.in/yaml.v3@v3.0.1/yaml.go",
                    "package yaml\n\nfunc Unmarshal(in []byte, out any) error { return nil }\n",
                ),
                (
                    "google.golang.org/protobuf@v1.34.0/proto/decode.go",
                    "package proto\n\nfunc (o UnmarshalOptions) Unmarshal(b []byte, m any) error { return nil }\n",
                ),
                (
                    "github.com/other/lib@v1.0.0/lib.go",
                    "package lib\n\nfunc Baz() {}\n",
                ),
                (
                    "alloc/src/borrow.rs",
                    "pub trait ToOwned {\n    fn into_owned(self) -> Self;\n}\n",
                ),
                (
                    "std/src/path.rs",
                    "impl Path {\n    pub fn join(&self, p: &str) {}\n}\n",
                ),
                ("std/src/fs.rs", "pub fn read_to_string(path: &str) {}\n"),
                ("os/__init__.py", ""),
                ("os/path.py", "def join(a, *p):\n    return a\n"),
                ("jsonx/a.py", "def dumps_x(o):\n    return o\n"),
                ("jsonx/b.py", "def dumps_x(o):\n    return o\n"),
                (
                    "hono/dist/types/hono.d.ts",
                    "export declare class Hono {\n}\n",
                ),
            ],
        );
        for kind in [Kind::Go, Kind::Rust, Kind::Python, Kind::TsJs] {
            use_roots(&mut a, kind, std::slice::from_ref(&root));
        }
        let at = |p: &str| format!("{}", root.join(p).display());
        for (file, code, want) in [
            // A Go package named without its module path's `.v3`.
            (
                "main.go",
                "yaml.Unmarshal",
                jump(
                    "Unmarshal: via import gopkg.in/yaml.v3",
                    &at("gopkg.in/yaml.v3@v3.0.1/yaml.go:3"),
                ),
            ),
            // `github.com/foo/bar` is not installed: `github.com/other/lib` only shares a prefix.
            (
                "main.go",
                "bar.Baz",
                jump(
                    "Baz: by name, 1 match",
                    &at("github.com/other/lib@v1.0.0/lib.go:3"),
                ),
            ),
            // A Rust call on a value still looks outside the project.
            (
                "main.rs",
                ".into_owned",
                jump(
                    "into_owned \u{2192} ToOwned::into_owned (by name, 1 match)",
                    &at("alloc/src/borrow.rs:2"),
                ),
            ),
            // A value named like a module is found in that module, by name.
            (
                "main.rs",
                "path.join",
                jump(
                    "join \u{2192} Path::join (by name, 1 match)",
                    &at("std/src/path.rs:2"),
                ),
            ),
            (
                "main.rs",
                "std::fs::read_to_string",
                jump("read_to_string: via std::fs", &at("std/src/fs.rs:1")),
            ),
            (
                "m.py",
                "os.path.join",
                jump("join: via import os.path", &at("os/path.py:1")),
            ),
            // The name a TypeScript import takes is not a file of the package.
            (
                "app.ts",
                "new Hono",
                jump("Hono: via import hono", &at("hono/dist/types/hono.d.ts:1")),
            ),
        ] {
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{code}");
        }
        // Rows that share an import say so, and so does the title.
        d_on(&mut a, "m.py", "jsonx.dumps_x");
        let row = |place: &str| {
            (
                "dumps_x".to_string(),
                "via import jsonx".to_string(),
                place.to_string(),
            )
        };
        assert_eq!(
            shown(&mut a),
            Shown::Picker(
                "dumps_x: via import jsonx, 2 declarations".into(),
                vec![row("jsonx/a.py:1"), row("jsonx/b.py:1")],
            )
        );
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// `self.word` alone is the class's own member: a dependency's method of that name is not
    /// a candidate.
    #[test]
    fn a_bare_self_stays_in_the_project() {
        let (dir, mut a) = project_app(
            "own",
            &[(
                "a.py",
                "class A:\n    def stop(self):\n        pass\n\n    def run(self):\n        self.stop()\n",
            )],
        );
        let root = external_root(
            "own",
            &[(
                "threading.py",
                "class Thread:\n    def stop(self):\n        pass\n",
            )],
        );
        use_roots(&mut a, Kind::Python, std::slice::from_ref(&root));
        d_on(&mut a, "a.py", "self.stop");
        assert_eq!(
            shown(&mut a),
            jump("stop \u{2192} A.stop (via self: A)", "a.py:2")
        );
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// An editable install puts the project's own `src` on `sys.path`: a file the project walk
    /// listed stays editable, while a gitignored `.venv` under the root does not.
    #[test]
    fn a_project_file_under_an_external_root_stays_editable() {
        let (dir, mut a) = project_app(
            "editable",
            &[
                (
                    "src/app/repo.py",
                    "class Repo:\n    def save(self):\n        pass\n",
                ),
                ("src/app/cli.py", "def main(r):\n    r.save()\n"),
                (".gitignore", ".venv\n"),
                (".venv/lib/site.py", "x = 1\n"),
            ],
        );
        use_roots(
            &mut a,
            Kind::Python,
            &[dir.join("src"), dir.join(".venv/lib")],
        );
        d_on(&mut a, "src/app/cli.py", "r.save");
        assert_eq!(
            shown(&mut a),
            jump(
                "save \u{2192} Repo.save (by name, 1 match)",
                "src/app/repo.py:2"
            )
        );
        assert_eq!(a.buf.readonly, None);
        a.jump_to(&dir.join(".venv/lib/site.py"), 1);
        assert_eq!(a.buf.readonly, Some("outside the project"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Edits that cannot be saved keep merl on the file; the status line must not claim `d` went.
    #[test]
    fn a_refused_jump_does_not_report_a_resolution() {
        let (dir, mut a) = project_app(
            "refused",
            &[
                ("a.py", "def helper():\n    pass\n"),
                ("b.py", "from a import helper\n\nhelper()\n"),
            ],
        );
        a.jump_to(&dir.join("b.py"), 3);
        (a.dirty, a.conflict) = (true, true);
        press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("b.py"), 2));
        assert_eq!(a.message, "");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_jump_says_how_the_target_was_found() {
        let mut a = fixture_app("go");
        // A bare word declared at the top level: nothing to qualify it with.
        d_on(&mut a, "service.go", "\tHandleDelete");
        assert_eq!(a.message, "HandleDelete: by name, 1 match");
        assert!(a.line_str().starts_with("func HandleDelete("));
        let one = |reason| {
            vec![Candidate {
                hit: Hit {
                    path: PathBuf::from("x"),
                    line: 1,
                    text: String::new(),
                },
                reason,
            }]
        };
        assert_eq!(
            resolution(
                "load",
                Some("json.load"),
                &one(Reason::Import("json".into())),
                None
            ),
            "load \u{2192} json.load (via import json)"
        );
        let mut two = one(Reason::ByName);
        two.extend(one(Reason::Path("std::fs".into())));
        assert_eq!(resolution("read", None, &two, None), "read: 2 declarations");
        // The candidates stop at MAX_HITS: the count is a lower bound.
        let many: Vec<Candidate> = (0..search::MAX_HITS)
            .flat_map(|_| one(Reason::ByName))
            .collect();
        assert_eq!(
            resolution("save", None, &many, None),
            format!("save: by name, {}+ declarations", search::MAX_HITS)
        );
        assert_eq!(
            resolution("read", None, &[], None),
            "no definition for read"
        );
        // A chain in front of the word that could not be followed says where it broke.
        assert_eq!(
            resolution(
                "remove",
                Some("UserService.remove"),
                &one(Reason::ByName),
                Some("users")
            ),
            "remove \u{2192} UserService.remove (by name, 1 match, chain broke at users)"
        );
        assert_eq!(
            resolution("read", None, &[], Some("repo")),
            "no definition for read (chain broke at repo)"
        );
    }

    #[test]
    fn navigation_searches_the_open_file_as_it_is_on_screen() {
        let (dir, mut a) = project_app(
            "unsaved",
            &[(
                "lib.rs",
                "fn main() { target(); }\nfn target() {}\n// end\n",
            )],
        );
        a.jump_to(&dir.join("lib.rs"), 1);
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        // Undo from navigation: until autosave the buffer is a line shorter than the disk.
        press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert!(a.dirty);
        a.col = 12;
        press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(a.line_str(), "fn target() {}", "{}", a.message);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn symbols_read_each_kind_with_its_own_pattern() {
        let (dir, mut a) = project_app(
            "symbols",
            &[
                ("Makefile", "build: deps\n"),
                ("deploy.yaml", "apiVersion: apps/v1\nx-base: &base\n"),
                ("main.tf", "variable \"region\" {}\n"),
                ("Dockerfile", "FROM rust AS build\n"),
                ("app.py", "def serve():\n    pass\n"),
            ],
        );
        press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
        let picker = a.picker.as_mut().unwrap();
        picker.settle();
        let names: Vec<String> = picker
            .window(20)
            .0
            .into_iter()
            .map(|r| r.item.label.split_whitespace().next().unwrap().to_string())
            .collect();
        // `apiVersion:` has the shape of a Makefile target; the target rule only reads Makefiles.
        assert_eq!(names, ["&base", "build", "build", "serve", "var.region"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn tree_focus_keys_do_not_move_the_code_cursor() {
        let mut a = app("aaa\nbbb\nccc\n");
        a.focus = Focus::Tree;
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(a.line, 0, "Down belongs to the tree while it has focus");
        press(&mut a, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(a.focus, Focus::Code);
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(a.line, 1);
        // `t` hides the tree and hands the keys to the code pane for good.
        a.focus = Focus::Tree;
        press(&mut a, KeyCode::Char('t'), KeyModifiers::NONE);
        assert!(!a.show_tree);
        assert_eq!(a.focus, Focus::Code);
        press(&mut a, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(a.focus, Focus::Code);
    }

    fn typed(a: &mut App, text: &str) {
        for c in text.chars() {
            press(a, KeyCode::Char(c), KeyModifiers::NONE);
        }
    }

    fn find(a: &mut App, query: &str) {
        press(a, KeyCode::Char('/'), KeyModifiers::NONE);
        typed(a, query);
    }

    #[test]
    fn emptying_the_query_drops_the_pattern_and_returns_to_the_anchor() {
        let mut a = app("foo\nbar\nbaz\n");
        find(&mut a, "ba");
        assert_eq!(a.line, 1);
        assert!(a.find_re.is_some());
        press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
        press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
        assert!(
            a.find_re.is_none(),
            "an empty query must not keep old highlights"
        );
        assert_eq!(a.line, 0, "cursor returns to the anchor");
    }

    #[test]
    fn find_is_smart_case() {
        let mut a = app("foo\nFoo\nbar\n");
        // An all-lowercase query is case-insensitive: it stops on `Foo` under the anchor.
        a.line = 1;
        find(&mut a, "foo");
        assert_eq!((a.line, a.col), (1, 0));
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(a.mode, Mode::Normal);

        // One uppercase letter makes it case-sensitive: `foo` on line 1 is skipped.
        a.line = 0;
        find(&mut a, "Foo");
        assert_eq!((a.line, a.col), (1, 0));
    }

    #[test]
    fn next_and_prev_wrap_around() {
        let mut a = app("foo\nbar\nfoo\n");
        find(&mut a, "foo");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (0, 0));

        press(&mut a, KeyCode::Char('n'), KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (2, 0));
        assert_eq!(a.message, "");
        press(&mut a, KeyCode::Char('n'), KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (0, 0));
        assert_eq!(a.message, "wrapped");

        press(&mut a, KeyCode::Char('N'), KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (2, 0));
        assert_eq!(a.message, "wrapped");
        press(&mut a, KeyCode::Char('N'), KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (0, 0));
        assert_eq!(a.message, "");

        let mut a = app("nothing here\n");
        find(&mut a, "zzz");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('n'), KeyModifiers::NONE);
        assert_eq!(a.message, "no match");
    }

    #[test]
    fn find_is_literal_not_a_regex() {
        let mut a = app("foo\nMigrator()\nbar\n");
        find(&mut a, "migrator(");
        assert_eq!((a.line, a.col), (1, 0));
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);

        find(&mut a, "f.o");
        let re = a.find_re.as_ref().unwrap();
        assert!(!re.is_match("foo"), "`.` is a dot, not any-char");
        assert!(re.is_match("f.o"));
    }

    #[test]
    fn esc_restores_the_anchor() {
        let mut a = app("foo\nbar\nbaz\n");
        a.line = 2;
        a.col = 1;
        find(&mut a, "foo");
        assert_eq!(
            (a.line, a.col),
            (0, 0),
            "incremental search moved the cursor"
        );
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(a.mode, Mode::Normal);
        assert_eq!((a.line, a.col), (2, 1));
    }

    fn temp_file(name: &str, text: &str) -> (PathBuf, App) {
        let dir = std::env::temp_dir().join(format!("merl-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.py");
        std::fs::write(&path, text).unwrap();
        let mut a = App::new(
            dir,
            Tree::default(),
            Vec::new(),
            Buffer::load(&path).unwrap(),
            None,
        );
        a.view_w = 20;
        a.view_h = 10;
        (path, a)
    }

    #[test]
    fn enter_edits_and_letters_are_text_until_esc() {
        let mut a = app("def f():\n    pass\n");
        typed(&mut a, "s");
        assert_eq!(a.mode, Mode::Search, "letters navigate outside edit mode");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        press(&mut a, KeyCode::End, KeyModifiers::NONE);
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(a.mode, Mode::Edit);
        typed(&mut a, "  # sq");
        assert_eq!(a.line_str(), "def f():  # sq");
        assert!(a.dirty);
        // Enter keeps the indentation of the line it leaves; Backspace at column 0 joins.
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        press(&mut a, KeyCode::End, KeyModifiers::NONE);
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (2, 4));
        assert_eq!(a.buf.lines[2], "    ");
        press(&mut a, KeyCode::Tab, KeyModifiers::NONE);
        typed(&mut a, "x");
        assert_eq!(a.buf.lines[2], "        x");
        press(&mut a, KeyCode::Home, KeyModifiers::NONE);
        press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(a.buf.lines, vec!["def f():  # sq", "    pass        x"]);
        press(&mut a, KeyCode::Delete, KeyModifiers::NONE);
        press(&mut a, KeyCode::Delete, KeyModifiers::NONE);
        assert_eq!(a.buf.lines[1], "    pass      x");
        // Delete at the end of the last line has nothing to join.
        press(&mut a, KeyCode::End, KeyModifiers::NONE);
        press(&mut a, KeyCode::Delete, KeyModifiers::NONE);
        assert_eq!(a.buf.lines.len(), 2);
        assert!(
            !press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE),
            "q types"
        );
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(a.mode, Mode::Normal);
        assert!(
            press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE),
            "q quits again"
        );
    }

    #[test]
    fn tab_follows_the_file_and_unicode_edits_stay_on_boundaries() {
        let mut a = app("\tif x:\n\t\tpass\n");
        assert!(a.buf.tabs);
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Tab, KeyModifiers::NONE);
        typed(&mut a, "é");
        assert_eq!(a.line_str(), "\té\tif x:");
        assert_eq!(a.display_col(), 6);
        press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
        press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(a.line_str(), "\tif x:");
    }

    #[test]
    fn undo_groups_typing_and_redo_replays_it() {
        let mut a = app("ab\ncd\n");
        press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(a.message, "nothing to undo");
        press(&mut a, KeyCode::End, KeyModifiers::NONE);
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "12");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "34");
        press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(a.buf.lines, vec!["ab12", "3", "cd"]);
        // A run of keystrokes on one line is one step; the split and the next run are others.
        press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(a.buf.lines, vec!["ab12", "", "cd"]);
        press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(
            (a.buf.lines.clone(), a.line, a.col),
            (vec!["ab12".to_string(), "cd".into()], 0, 4)
        );
        press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(a.buf.lines, vec!["ab", "cd"]);
        assert!(a.dirty);
        press(&mut a, KeyCode::Char('y'), KeyModifiers::CONTROL);
        press(&mut a, KeyCode::Char('y'), KeyModifiers::CONTROL);
        assert_eq!(
            (a.buf.lines.clone(), a.line, a.col),
            (vec!["ab12".to_string(), "".into(), "cd".into()], 1, 0)
        );
        // A new edit drops the redo stack; a cursor move starts a new step.
        typed(&mut a, "x");
        press(&mut a, KeyCode::Char('y'), KeyModifiers::CONTROL);
        assert_eq!(a.message, "nothing to redo");
        press(&mut a, KeyCode::Up, KeyModifiers::NONE);
        typed(&mut a, "!");
        press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(a.buf.lines, vec!["ab12", "x", "cd"]);
        // Leaving and re-entering edit mode also ends the step; undo works from navigation.
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "y");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(a.buf.lines, vec!["ab12", "x", "cd"]);
        press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(a.buf.lines, vec!["ab12", "", "cd"]);
    }

    #[test]
    fn typing_replaces_the_selection_and_the_clipboard_keys_copy_or_cut() {
        let mut a = app("abc\ndef\nghi\n");
        press(&mut a, KeyCode::Right, KeyModifiers::NONE);
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
        assert_eq!(a.selection(), Some(((0, 1), (1, 1))));
        press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(a.clipboard.take().as_deref(), Some("bc\nd"));
        assert_eq!(a.buf.lines.len(), 3, "copy leaves the text alone");
        assert!(a.selection().is_some(), "and the selection");
        typed(&mut a, "X");
        assert_eq!(a.buf.lines, vec!["aXef", "ghi"]);
        assert_eq!((a.line, a.col, a.selection()), (0, 2, None));
        press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(a.buf.lines, vec!["abc", "def", "ghi"], "one step");
        assert_eq!((a.line, a.col), (1, 1), "undo puts the cursor where it was");
        // Delete on a selection removes it; Ctrl+X without one cuts the line.
        press(&mut a, KeyCode::Up, KeyModifiers::NONE);
        press(
            &mut a,
            KeyCode::Right,
            KeyModifiers::SHIFT | KeyModifiers::CONTROL,
        );
        press(&mut a, KeyCode::Delete, KeyModifiers::NONE);
        assert_eq!(a.buf.lines[0], "a");
        press(&mut a, KeyCode::Char('x'), KeyModifiers::CONTROL);
        assert_eq!(a.clipboard.take().as_deref(), Some("a\n"));
        assert_eq!(a.buf.lines, vec!["def", "ghi"]);
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('x'), KeyModifiers::CONTROL);
        assert_eq!(a.clipboard.take().as_deref(), Some("ghi"));
        assert_eq!(a.buf.lines, vec!["def", ""]);
        // Pasted text is inserted only while editing, with CRLF normalised.
        a.paste("p\r\nq");
        assert_eq!(a.buf.lines, vec!["def", "p", "q"]);
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        a.paste("nope");
        assert_eq!(a.buf.lines, vec!["def", "p", "q"]);
        assert!(
            press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL),
            "Ctrl+C quits again"
        );
    }

    #[test]
    fn readonly_and_clipped_lines_refuse_to_edit() {
        let mut a = App::new(
            PathBuf::from("/tmp"),
            Tree::default(),
            Vec::new(),
            Buffer::from_bytes(PathBuf::from("/tmp/x"), b"a\0b"),
            None,
        );
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            (a.mode, a.message.as_str()),
            (Mode::Normal, "read-only: binary file")
        );
        let mut a = app(&"x".repeat(30_000));
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            (a.mode, a.message.as_str()),
            (Mode::Normal, "line too long to edit")
        );
        let mut a = App::new(
            PathBuf::from("/tmp"),
            Tree::default(),
            Vec::new(),
            Buffer::empty(),
            None,
        );
        a.focus = Focus::Code;
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            a.mode,
            Mode::Normal,
            "nothing to edit on the welcome screen"
        );
    }

    #[test]
    fn autosave_esc_and_quit_reach_the_disk() {
        let (path, mut a) = temp_file("autosave", "a\r\nb\r\n");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "x");
        assert!(a.dirty);
        assert!(!a.tick(), "the delay has not passed");
        a.autosave = Duration::ZERO;
        assert!(a.tick());
        assert!(!a.dirty);
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"xa\r\nb\r\n",
            "CRLF survives"
        );
        a.autosave = Duration::from_secs(60);
        typed(&mut a, "y");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(std::fs::read(&path).unwrap(), b"xya\r\nb\r\n");
        // Undo from navigation dirties the buffer again; quitting flushes it.
        press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert!(a.dirty);
        assert!(press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"a\r\nb\r\n",
            "x and y were one step"
        );
        // merl's own save is not a change to react to.
        a.reload(false);
        assert_eq!(a.message, "");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_change_under_unsaved_edits_is_a_conflict() {
        let (path, mut a) = temp_file("conflict", "one\n");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "mine ");
        std::fs::write(&path, "theirs\n").unwrap();
        a.reload(false);
        assert!(a.conflict);
        assert_eq!(a.line_str(), "mine one", "the buffer keeps the edits");
        a.autosave = Duration::ZERO;
        assert!(!a.tick(), "autosave is off while conflicted");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "theirs\n");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "theirs\n",
            "Esc does not overwrite"
        );
        press(&mut a, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert!(!a.conflict && !a.dirty);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "mine one\n");
        // Ctrl+R takes the disk's version, edits and all.
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "again ");
        std::fs::write(&path, "theirs\n").unwrap();
        a.reload(false);
        assert!(a.conflict);
        press(&mut a, KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert_eq!(
            (a.conflict, a.dirty, a.line_str()),
            (false, false, "theirs")
        );
        press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(
            a.message, "nothing to undo",
            "a reload starts a fresh history"
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn switching_files_flushes_and_leaves_edit_mode() {
        let (path, mut a) = temp_file("switch", "one\n");
        let other = path.with_file_name("g.py");
        std::fs::write(&other, "two\n").unwrap();
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "x");
        a.jump_to(&other, 1);
        assert_eq!((a.mode, a.dirty), (Mode::Normal, false));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "xone\n");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn read_only_buffers_are_never_changed_or_written() {
        let (path, mut a) = temp_file("readonly", "one\n");
        // Another tool re-encodes the file while it is in edit mode with nothing unsaved.
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        std::fs::write(&path, b"caf\xe9\n").unwrap();
        a.reload(false);
        typed(&mut a, "x");
        assert_eq!(
            (a.dirty, a.message.as_str()),
            (false, "read-only: not UTF-8")
        );
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(a.message, "read-only: not UTF-8");
        assert_eq!(std::fs::read(&path).unwrap(), b"caf\xe9\n");
        // Nor does a binary file's placeholder text reach the disk.
        std::fs::write(&path, b"\x89PNG\0\x01").unwrap();
        a.reload(false);
        press(&mut a, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(std::fs::read(&path).unwrap(), b"\x89PNG\0\x01");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn edits_that_touch_or_make_a_clipped_line_are_refused() {
        let mut a = app(&format!("ab\n{}\n", "x".repeat(20_000)));
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        a.paste(&"y".repeat(20_000));
        assert_eq!(
            (a.line_str(), a.message.as_str()),
            ("ab", "line too long to edit")
        );
        // A join into a line exactly as long as the screen shows.
        press(&mut a, KeyCode::End, KeyModifiers::NONE);
        press(&mut a, KeyCode::Delete, KeyModifiers::NONE);
        assert_eq!(
            (a.buf.lines.len(), a.message.as_str()),
            (2, "line too long to edit")
        );
        assert!(!a.dirty);
    }

    #[test]
    fn a_save_never_overwrites_a_change_it_has_not_seen() {
        let (path, mut a) = temp_file("unseen", "one\n");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "mine ");
        // The other write lands before its watcher event is handled, or with no watcher at all.
        std::fs::write(&path, "theirs\n").unwrap();
        a.autosave = Duration::ZERO;
        a.tick();
        assert!(a.conflict);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "theirs\n");
        // With the conflict on screen, Ctrl+S is the answer that overwrites.
        press(&mut a, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert!(!a.conflict && !a.dirty);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "mine one\n");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn edits_that_cannot_be_saved_keep_merl_on_the_file() {
        let (path, mut a) = temp_file("keep", "one\n");
        let other = path.with_file_name("g.py");
        std::fs::write(&other, "two\n").unwrap();
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "mine ");
        std::fs::write(&path, "theirs\n").unwrap();
        a.reload(false);
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        // In a conflict another file does not open over the edits, and the first quit is refused.
        a.jump_to(&other, 1);
        assert_eq!((at(&a).0, a.line_str()), (path.clone(), "mine one"));
        assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(a.message.contains("q again"), "{}", a.message);
        assert!(press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "theirs\n");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();

        // A save that fails holds merl the same way, and a key in between asks again.
        let (path, mut a) = temp_file("keep-gone", "one\n");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "x");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert!(a.message.starts_with("save failed"), "{}", a.message);
        assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
    }

    #[test]
    fn reload_clamps_the_cursor_into_a_shrunken_file() {
        let dir = std::env::temp_dir().join(format!("merl-reload-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.txt");
        std::fs::write(&path, "line\n".repeat(10)).unwrap();
        let mut a = App::new(
            dir.clone(),
            Tree::default(),
            Vec::new(),
            Buffer::load(&path).unwrap(),
            None,
        );
        a.view_w = 20;
        a.view_h = 10;
        a.line = 9;
        a.col = 4;

        // An event that changes nothing must not even report a reload.
        a.reload(false);
        assert_eq!(a.message, "");

        std::fs::write(&path, "a\nb\nc\n").unwrap();
        a.reload(false);
        assert_eq!(a.buf.lines.len(), 3);
        assert_eq!(a.line, 2);
        assert_eq!(a.col, 1);
        assert_eq!(a.message, "reloaded");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Every alias in `KEYS` reaches the same action as its primary key.
    #[test]
    fn aliases_reach_the_same_actions() {
        let mut a = app("foo bar\n");
        press(&mut a, KeyCode::F(12), KeyModifiers::NONE);
        assert_eq!(a.message, "no definition for foo");
        press(&mut a, KeyCode::F(12), KeyModifiers::SHIFT);
        assert_eq!(a.message, "no usages of foo");
        press(&mut a, KeyCode::Char('e'), KeyModifiers::CONTROL);
        assert_eq!(a.mode, Mode::Picker(PickerKind::Files));
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert!(a.picker.is_none());
        press(&mut a, KeyCode::Char('f'), KeyModifiers::CONTROL);
        assert_eq!(a.mode, Mode::Find);
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('g'), KeyModifiers::CONTROL);
        assert_eq!(a.mode, Mode::Goto);
    }

    #[test]
    fn help_scrolls_and_reopens_at_the_top() {
        let mut a = app("foo\n");
        press(&mut a, KeyCode::Char('?'), KeyModifiers::NONE);
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        press(&mut a, KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(a.help_top, 1);
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('?'), KeyModifiers::NONE);
        assert_eq!(a.help_top, 0);
    }

    /// Alt+letter is Esc then the letter. Over a picker or a prompt the Esc closes it and the
    /// letter is dropped: Alt+q must not quit, Alt+d must not run go-to-definition.
    #[test]
    fn alt_letter_over_an_overlay_only_closes_it() {
        let mut a = app("foo\n");
        press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
        assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::ALT));
        assert!((a.picker.is_none(), a.mode) == (true, Mode::Normal));
        press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('d'), KeyModifiers::ALT);
        assert_eq!((a.mode, a.message.as_str()), (Mode::Normal, ""));
        // In normal mode the letter still counts.
        assert!(press(&mut a, KeyCode::Char('q'), KeyModifiers::ALT));
    }

    #[test]
    fn project_search_is_literal_not_a_regex() {
        let (path, mut a) = temp_file("literal-s", "def foo(x):\nself.foo(1)\nfoo_bar\na.b\naXb\n");
        for (query, hits, why) in [
            ("foo(", 2, "a paren is text, not a regex error"),
            ("a.b", 1, "a dot matches only a dot"),
        ] {
            press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
            typed(&mut a, query);
            press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
            let p = a.picker.as_mut().expect(why);
            p.settle();
            assert_eq!(p.counts().1, hits, "{why}: {}", a.message);
            press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        }
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    /// The find anchor, the selection anchor and the history stops are byte positions taken
    /// before a reload; reading them back must clamp to the file as it is now.
    #[test]
    fn stored_positions_survive_a_reload() {
        let dir = std::env::temp_dir().join(format!("merl-stale-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.txt");
        std::fs::write(&path, "abcdefghij\n".repeat(50)).unwrap();
        let mut a = App::new(
            dir.clone(),
            Tree::default(),
            Vec::new(),
            Buffer::load(&path).unwrap(),
            None,
        );
        a.jump_to(&path, 40);
        a.col = 8;
        press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
        press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
        std::fs::write(&path, "éé\ny\n").unwrap();
        a.reload(false);
        // Empty query: back to the (clamped) find anchor.
        typed(&mut a, "z");
        press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (1, 0));
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        // A plain arrow collapses the selection to its clamped anchor.
        press(&mut a, KeyCode::Left, KeyModifiers::NONE);
        assert_eq!((a.line, a.col, a.selection()), (1, 0, None));
        // `[` onto the stop at line 40 lands on the clamped line, rewrites that stop, and keeps
        // the forward history instead of reading the clamp as a new move.
        a.jump_to(&path, 1);
        assert_eq!(a.history.len(), 4);
        assert_eq!(a.history[1], (path.clone(), 40, 0));
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!((a.line, a.col, a.hist_idx), (1, 0, 1));
        assert_eq!(a.history[1], (path.clone(), 1, 0));
        assert_eq!(a.history.len(), 4);
        press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
        assert_eq!(a.hist_idx, 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_stop_whose_file_is_gone_leaves_the_history_alone() {
        let (dir, mut a) = files_app("gone");
        let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
        a.jump_to(&x, 5);
        a.jump_to(&y, 1);
        std::fs::remove_file(&x).unwrap();
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!((at(&a), a.hist_idx, a.history.len()), ((y, 0), 1, 2));
        assert!(a.message.contains("a.rs"), "{}", a.message);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A hidden or ignored file is not in the startup walk, but it is still the file under
    /// the cursor: `u` finds the usages in it.
    #[test]
    fn the_open_file_is_searched_even_when_the_walk_skipped_it() {
        let (dir, mut a) = files_app("hidden");
        let x = dir.join("a.rs");
        assert!(a.files.is_empty());
        a.jump_to(&x, 1);
        press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
        assert_eq!(a.mode, Mode::Picker(PickerKind::Usages));
        let p = a.picker.as_mut().unwrap();
        p.settle();
        assert_eq!(p.counts().1, 40);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_truncated_result_list_says_so_in_its_title() {
        let (dir, mut a) = files_app("truncated");
        let x = dir.join("a.rs");
        std::fs::write(&x, "x\n".repeat(search::MAX_HITS + 1)).unwrap();
        a.jump_to(&x, 1);
        press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
        let p = a.picker.as_mut().unwrap();
        p.settle();
        assert_eq!(p.title, format!("Usages (first {})", search::MAX_HITS));
        assert_eq!(p.counts().1 as usize, search::MAX_HITS);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn help_opens_and_closes_and_esc_clears_the_find() {
        let mut a = app("foo\nbar\n");
        press(&mut a, KeyCode::Char('?'), KeyModifiers::NONE);
        assert_eq!(a.mode, Mode::Help);
        // Keys are inert while the overlay is up; `q` closes it instead of quitting.
        assert!(!press(&mut a, KeyCode::Down, KeyModifiers::NONE));
        assert_eq!((a.mode, a.line), (Mode::Help, 0));
        assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(a.mode, Mode::Normal);

        find(&mut a, "foo");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert!(a.find_re.is_some());
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert!(a.find_re.is_none());
        assert_eq!(a.message, "find cleared");
    }

    /// The README key table and `KEYS` are the same list.
    #[test]
    fn readme_documents_every_key() {
        let readme = include_str!("../README.md");
        let section = readme
            .split("\n## Keys\n")
            .nth(1)
            .expect("README has a Keys section")
            .split("\n## ")
            .next()
            .unwrap();
        let rows: Vec<&str> = section
            .lines()
            .filter(|l| l.starts_with('|'))
            .filter_map(|l| l.split('|').nth(1))
            .map(str::trim)
            .filter(|c| !c.is_empty() && !c.starts_with('-') && *c != "Key")
            .collect();
        for (key, _) in KEYS {
            assert!(rows.contains(key), "README is missing {key:?}");
        }
        for row in &rows {
            assert!(
                KEYS.iter().any(|(k, _)| k == row),
                "README documents {row:?}, which is not a binding"
            );
        }
    }

    #[test]
    fn half_page_moves_cursor_and_viewport_together() {
        let text: String = (0..40).map(|i| format!("line {i}\n")).collect();
        let mut a = app(&text);
        a.view_h = 10;
        press(&mut a, KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_eq!(a.line, 5);
        assert_eq!(a.top_line, 5, "viewport scrolls by the same amount");
        press(&mut a, KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_eq!((a.line, a.top_line), (10, 10));
        press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
        assert_eq!((a.line, a.top_line), (5, 5));
        // Ctrl+D must not be mistaken for go-to-definition.
        assert_eq!(a.message, "");
    }

    #[test]
    fn half_page_counts_screen_rows() {
        // The first line is four rows at 20 columns; half of a 6-row screen is 3 of them.
        let mut a = app(&format!("{}\nnext\n", "word ".repeat(16)));
        a.view_h = 6;
        press(&mut a, KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_eq!((a.line, a.cursor_row()), (0, 3));
        assert_eq!(
            (a.top_line, a.top_row),
            (0, 3),
            "the viewport scrolls the same rows"
        );
    }

    #[test]
    fn up_and_down_walk_screen_rows_and_keep_the_column() {
        // At 20 columns the first line is two rows: "aaaa bbbb cccc dddd " and "eeee ffff".
        let mut a = app("aaaa bbbb cccc dddd eeee ffff\nx\n");
        for _ in 0..7 {
            press(&mut a, KeyCode::Right, KeyModifiers::NONE);
        }
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (0, 27), "the second row of the same line");
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (1, 1), "a shorter row: its end");
        press(&mut a, KeyCode::Up, KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (0, 27), "the column is kept");
        press(&mut a, KeyCode::Up, KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (0, 7));
    }

    #[test]
    fn home_and_end_stop_at_the_screen_row_first() {
        let mut a = app("aaaa bbbb cccc dddd eeee ffff\n");
        press(&mut a, KeyCode::End, KeyModifiers::NONE);
        assert_eq!(a.col, 19, "on the space that ends the first row");
        press(&mut a, KeyCode::End, KeyModifiers::NONE);
        assert_eq!(a.col, 29, "the end of the line");
        press(&mut a, KeyCode::Home, KeyModifiers::NONE);
        assert_eq!(a.col, 20, "the start of the second row");
        press(&mut a, KeyCode::Home, KeyModifiers::NONE);
        assert_eq!(a.col, 0);
    }

    #[test]
    fn paragraph_jumps_to_blank_lines() {
        let mut a = app("a\nb\n\n  \nc\nd\n\ne\nf");
        let go = |a: &mut App, c: char| press(a, KeyCode::Char(c), KeyModifiers::NONE);
        go(&mut a, '}');
        assert_eq!(a.line, 2);
        go(&mut a, '}');
        assert_eq!(a.line, 6, "a run of blank lines is one boundary");
        go(&mut a, '}');
        assert_eq!(a.line, 8, "no blank line left: the last line");
        go(&mut a, '{');
        assert_eq!(a.line, 6);
        go(&mut a, '{');
        assert_eq!(a.line, 3, "whitespace-only lines are blank");
        go(&mut a, '{');
        assert_eq!(a.line, 0, "no blank line left: the first line");
    }

    #[test]
    fn ctrl_end_goes_to_last_line() {
        let mut a = app("a\nbb\nccc\n");
        press(&mut a, KeyCode::End, KeyModifiers::CONTROL);
        assert_eq!((a.line, a.col), (2, 3));
        press(&mut a, KeyCode::Home, KeyModifiers::CONTROL);
        assert_eq!((a.line, a.col), (0, 0));
    }
}
