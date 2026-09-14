//! All editor state and every key binding. Rendering lives in `ui.rs`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use regex::{Regex, RegexBuilder};

use crate::buffer::{self, Buffer};
use crate::picker::{Pick, PickItem, Picker};
use crate::search::{self, Hit};
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
    ("d / F12", "Go to definition of the word under the cursor"),
    ("D", "Project symbols (fuzzy)"),
    ("u / Shift+F12", "Usages of the word under the cursor"),
    ("[ / ]", "Back / forward in the jump history"),
    (": / Ctrl+G", "Go to line"),
    ("t", "Show or hide the file tree"),
    ("Tab", "Switch focus between tree and code"),
    ("Enter", "Edit at the cursor (Esc returns to navigation)"),
    (
        "Ctrl+S",
        "Save now (edits are saved on their own after a pause)",
    ),
    ("Ctrl+R", "Reload from disk, dropping unsaved edits"),
    ("Arrows", "Move the cursor"),
    ("Shift+Up / Shift+Down", "Extend the selection by a line"),
    ("Shift+Left / Shift+Right", "Move one word"),
    ("Alt+Shift+Left / Right", "Extend the selection by a word"),
    (
        "Ctrl+Shift+Left / Right",
        "Extend the selection to the start / end of the line",
    ),
    ("Ctrl+D / Ctrl+U", "Move half a screen down / up"),
    ("{ / }", "Previous / next paragraph (blank line)"),
    ("PgUp / PgDn", "Move one screen"),
    ("Home / End", "Start / end of the line"),
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
}

impl PickerKind {
    fn title(self) -> &'static str {
        match self {
            Self::Files => "Files",
            Self::Search => "Search",
            Self::Definitions => "Definitions",
            Self::Usages => "Usages",
            Self::Symbols => "Symbols",
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
    pub prompt: String,
    /// The current query, kept for `n`/`N` and for painting the matches.
    pub find_re: Option<Regex>,
    /// Where the cursor was when `/` was pressed: the start of the incremental search.
    find_anchor: (usize, usize),
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
            prompt: String::new(),
            find_re: None,
            find_anchor: (0, 0),
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

    pub fn cursor_row(&self) -> usize {
        wrap::col_to_row(&self.rows(self.line), self.col)
    }

    /// Display column of the cursor inside its wrapped row.
    pub fn cursor_x(&self) -> usize {
        let rows = self.rows(self.line);
        let row = wrap::col_to_row(&rows, self.col);
        wrap::width(&self.line_str()[wrap::row_to_col(&rows, row)..self.col])
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
        let cur = (self.line, self.cursor_row());
        if cur < (self.top_line, self.top_row) {
            (self.top_line, self.top_row) = cur;
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
                row = self.rows(line).len() - 1;
            } else {
                break;
            }
        }
        (line, row)
    }

    // ---- cursor movement -------------------------------------------------

    fn sync_want_x(&mut self) {
        self.want_x = wrap::width(&self.line_str()[..self.col]);
    }

    /// Places the cursor on `self.line` at the byte offset closest to `want_x`.
    fn apply_want_x(&mut self) {
        let mut col = self.buf.lines[self.line].len();
        let mut used = 0usize;
        for (i, c) in self.buf.lines[self.line].char_indices() {
            if used >= self.want_x {
                col = i;
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
    pub fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let a = self.clamp_pos(self.anchor?);
        let b = (self.line, self.col);
        Some((a.min(b), a.max(b)))
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

    /// Shift+move, VS Code style: the anchor is set where the first extending move started and
    /// the selection runs from there to wherever `mv` takes the cursor.
    fn extend(&mut self, mv: fn(&mut Self)) {
        self.anchor.get_or_insert((self.line, self.col));
        mv(self);
    }

    fn line_start(&mut self) {
        self.col = 0;
        self.want_x = 0;
    }

    fn line_end(&mut self) {
        self.col = self.line_str().len();
        self.sync_want_x();
    }

    fn move_line(&mut self, delta: isize) {
        let last = self.buf.lines.len() - 1;
        self.line = self.line.saturating_add_signed(delta).min(last);
        self.apply_want_x();
    }

    /// Ctrl+D / Ctrl+U: cursor and viewport both move half a screen, like vim and less,
    /// so the cursor keeps its place on screen and half the context stays visible.
    fn half_page(&mut self, dir: isize) {
        let half = (self.view_h / 2).max(1);
        let before = (self.line, self.cursor_row());
        self.move_line(dir * half as isize);
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
        self.apply_want_x();
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
            if row + 1 < self.rows(line).len() {
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

    /// Shows `path` at `line` (1-based; 0 means "keep the start of the file"). Reloading is
    /// skipped when the file is already open, so this doubles as a plain cursor move.
    /// Returns `false`, with the error in the status bar, when the file cannot be read.
    fn open(&mut self, path: &Path, line: usize) -> bool {
        if self.buf.path.as_deref() != Some(path) {
            self.flush();
            match Buffer::load(path) {
                Ok(buf) => {
                    self.buf = buf;
                    self.anchor = None;
                    self.dirty = false;
                    self.conflict = false;
                    if self.mode == Mode::Edit {
                        self.mode = Mode::Normal;
                    }
                    (self.line, self.col, self.want_x) = (0, 0, 0);
                    (self.top_line, self.top_row) = (0, 0);
                }
                Err(e) => {
                    self.message = format!("{e:#}");
                    return false;
                }
            }
        }
        self.goto_line(line.max(1));
        self.reveal(path);
        true
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
        let last = self.buf.lines.len() - 1;
        (self.line, self.col) = self.clamp_pos((self.line, self.col));
        self.top_line = self.top_line.min(last);
        self.top_row = self.top_row.min(self.rows(self.top_line).len() - 1);
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
    /// forward history. Standing on the current stop records nothing, so walking with
    /// `[` / `]` is silent.
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

    /// Opens `path` at `line` and makes it a stop in the jump history.
    pub fn jump_to(&mut self, path: &Path, line: usize) {
        if self.open(path, line) {
            self.focus = Focus::Code;
        }
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

    fn picker_key(&mut self, code: KeyCode, ctrl: bool) {
        let Some(picker) = &mut self.picker else {
            return;
        };
        match picker.key(code, ctrl) {
            Pick::Stay => return,
            Pick::Cancel => {}
            Pick::Accept(item) => {
                let path = self.root.join(&item.path);
                self.jump_to(&path, item.line.max(1));
            }
        }
        // Dropping the picker stops nucleo's workers.
        self.picker = None;
        self.mode = Mode::Normal;
    }

    // ---- find in file ----------------------------------------------------

    /// Starts an incremental search from the current cursor position.
    fn start_find(&mut self) {
        self.mode = Mode::Find;
        self.prompt.clear();
        self.find_anchor = (self.line, self.col);
    }

    fn find_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) => {
                self.prompt.push(c);
                self.refresh_find();
            }
            KeyCode::Backspace => {
                self.prompt.pop();
                self.refresh_find();
            }
            // Enter keeps both the position and the pattern, so `n` carries on from here.
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                self.hist_note(true);
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                (self.line, self.col) = self.clamp_pos(self.find_anchor);
                self.sync_want_x();
            }
            _ => {}
        }
    }

    /// Recompiles the query and moves to the first match at or after the anchor.
    /// The query is literal text with smart case, as in VS Code: `migrator(` hits `Migrator()`.
    /// `s>` is the place for regexes.
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

    /// Greps the project, optionally only the files with extension `ext`. The open file is
    /// always searched, even when the startup walk skipped it (hidden or ignored).
    fn grep(
        &self,
        pattern: &str,
        whole_word: bool,
        smart_case: bool,
        ext: Option<&str>,
    ) -> anyhow::Result<Vec<Hit>> {
        let wanted = |p: &Path| ext.is_none_or(|e| p.extension().is_some_and(|x| x == e));
        let mut files: Vec<PathBuf> = self.files.iter().filter(|p| wanted(p)).cloned().collect();
        let current = self.rel_current();
        if let Some(cur) = &current
            && wanted(cur)
            && !files.contains(cur)
        {
            files.push(cur.clone());
        }
        search::grep_project(
            &self.root,
            &files,
            pattern,
            whole_word,
            smart_case,
            current.as_deref(),
        )
    }

    fn word_under(&self) -> Option<String> {
        search::word_at(self.line_str(), self.col).map(|(_, w)| w.to_string())
    }

    fn extension(&self) -> String {
        self.buf
            .path
            .as_ref()
            .and_then(|p| p.extension())
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// `rel/path:line: text` rows for a result picker. The text is normalized like `Buffer`
    /// does (tabs to spaces) so `ui` can line it up with the file's highlighting.
    pub(crate) fn hit_items(hits: Vec<Hit>) -> Vec<PickItem> {
        hits.into_iter()
            .map(|h| {
                let label = format!("{}:{}: ", h.path.display(), h.line);
                let code_at = Some(label.len());
                let text = h.text.replace('\t', buffer::TAB);
                PickItem {
                    label: label + &clip(text.trim(), MAX_LABEL_TEXT),
                    path: h.path,
                    line: h.line,
                    code_at,
                }
            })
            .collect()
    }

    /// Enter in the `s>` prompt: a smart-case regex search over every file.
    fn run_search(&mut self) {
        let query = std::mem::take(&mut self.prompt);
        self.mode = Mode::Normal;
        if query.is_empty() {
            return;
        }
        let hits = match self.grep(&query, false, true, None) {
            Ok(hits) => hits,
            Err(e) => {
                self.message = format!("{e:#}");
                return;
            }
        };
        if hits.is_empty() {
            self.message = format!("no results for {query}");
            return;
        }
        self.show_picker(PickerKind::Search, Self::hit_items(hits));
    }

    /// `d` / F12. Languages with declaration patterns get those, over files of the same
    /// extension; anything else, or a word the patterns do not declare (a field, a variant, a
    /// parameter), falls back to a whole-word search for the identifier itself.
    fn goto_definition(&mut self) {
        let Some(word) = self.word_under() else {
            return;
        };
        let ext = self.extension();
        let patterns = search::def_patterns(&ext, &word);
        // Escaped or built-in patterns always compile.
        let mut hits = if patterns.is_empty() {
            Vec::new()
        } else {
            self.grep(&patterns.join("|"), false, false, Some(&ext))
                .unwrap_or_default()
        };
        if hits.is_empty() {
            hits = self
                .grep(&regex::escape(&word), true, false, None)
                .unwrap_or_default();
        }
        // Standing on one of the definitions is not a reason to go nowhere.
        if hits.len() > 1 {
            let here = self.rel_current();
            hits.retain(|h| h.line != self.line + 1 || Some(&h.path) != here.as_ref());
        }
        match hits.len() {
            0 => self.message = format!("no definition for {word}"),
            1 => {
                let path = self.root.join(&hits[0].path);
                self.jump_to(&path, hits[0].line);
            }
            _ => self.show_picker(PickerKind::Definitions, Self::hit_items(hits)),
        }
    }

    /// `u` / Shift+F12: every whole-word occurrence of the identifier, case-sensitive.
    fn usages(&mut self) {
        let Some(word) = self.word_under() else {
            return;
        };
        let hits = self
            .grep(&regex::escape(&word), true, false, None)
            .unwrap_or_default();
        if hits.is_empty() {
            self.message = format!("no usages of {word}");
            return;
        }
        self.show_picker(PickerKind::Usages, Self::hit_items(hits));
    }

    /// `D`: every declaration in the project, recomputed on each press.
    fn symbols(&mut self) {
        let hits = self
            .grep(search::SYMBOL_PATTERN, false, false, None)
            .unwrap_or_default();
        let mut named: Vec<(String, Hit)> = hits
            .into_iter()
            .filter_map(|h| Some((search::symbol_name(&h.text)?.to_string(), h)))
            .collect();
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
                // The file that is already open keeps its cursor, like VS Code's explorer.
                Some(n) if self.buf.path.as_deref() == Some(&self.root.join(&n.path)) => {
                    self.focus = Focus::Code;
                }
                Some(n) => {
                    let path = self.root.join(&n.path);
                    self.jump_to(&path, 1);
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
        if let Some(why) = self.buf.readonly {
            self.message = format!("read-only: {why}");
            return;
        }
        // The tail of a clipped line is not on screen, so it cannot be edited by sight.
        if self.buf.shown(self.line).len() != self.line_str().len() {
            self.message = "line too long to edit".into();
            return;
        }
        self.mode = Mode::Edit;
    }

    /// Keys that only mean something while editing. Returns `false` for every other key, which
    /// then falls through to the navigation keys: arrows, Home / End, the chord aliases.
    fn edit_key(&mut self, code: KeyCode, ctrl: bool) -> bool {
        match code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.flush();
            }
            KeyCode::Char(c) if !ctrl => self.insert(&c.to_string()),
            KeyCode::Enter => {
                let head = &self.line_str()[..self.col];
                let indent = &head[..head.len() - head.trim_start().len()];
                self.insert(&format!("\n{indent}"));
            }
            KeyCode::Tab => self.insert(if self.buf.tabs { "\t" } else { buffer::TAB }),
            KeyCode::Backspace => {
                if self.col > 0 {
                    let from = prev_char(self.line_str(), self.col);
                    self.buf.lines[self.line].replace_range(from..self.col, "");
                    self.col = from;
                    self.touched();
                } else if self.line > 0 {
                    let tail = self.buf.lines.remove(self.line);
                    self.line -= 1;
                    self.col = self.line_str().len();
                    self.buf.lines[self.line].push_str(&tail);
                    self.touched();
                }
            }
            KeyCode::Delete => {
                if self.col < self.line_str().len() {
                    let to = next_char(self.line_str(), self.col);
                    self.buf.lines[self.line].replace_range(self.col..to, "");
                    self.touched();
                } else if self.line + 1 < self.buf.lines.len() {
                    let tail = self.buf.lines.remove(self.line + 1);
                    self.buf.lines[self.line].push_str(&tail);
                    self.touched();
                }
            }
            _ => return false,
        }
        true
    }

    /// Inserts `text` at the cursor; a `\n` in it splits the line.
    fn insert(&mut self, text: &str) {
        let tail = self.buf.lines[self.line].split_off(self.col);
        let mut parts = text.split('\n');
        self.buf.lines[self.line].push_str(parts.next().unwrap_or_default());
        for part in parts {
            self.line += 1;
            self.buf.lines.insert(self.line, part.to_string());
        }
        self.col = self.line_str().len();
        self.buf.lines[self.line].push_str(&tail);
        self.touched();
    }

    /// After every change to `buf.lines`: the highlighting above the cursor may be stale, the
    /// disk is behind, and the autosave clock restarts.
    fn touched(&mut self) {
        self.buf.edited(self.line);
        self.dirty = true;
        self.last_edit = Some(Instant::now());
        self.sync_want_x();
    }

    /// Writes the buffer to its file, ending any conflict in the buffer's favour.
    pub fn save(&mut self) {
        let Some(path) = &self.buf.path else { return };
        let bytes = self.buf.to_bytes();
        match std::fs::write(path, &bytes) {
            Ok(()) => {
                self.buf.disk = buffer::hash(&bytes);
                self.dirty = false;
                self.conflict = false;
                self.last_edit = None;
            }
            Err(e) => {
                // Retried on the next autosave; the message stays until then.
                self.message = format!("save failed: {e}");
                self.last_edit = Some(Instant::now());
            }
        }
    }

    /// Saves unsaved edits now: leaving edit mode, switching files, quitting.
    pub fn flush(&mut self) {
        if self.dirty && !self.conflict {
            self.save();
        }
    }

    /// Autosave, called by the event loop between events. Returns `true` when it saved.
    pub fn tick(&mut self) -> bool {
        let due = self.last_edit.is_some_and(|t| t.elapsed() >= self.autosave);
        if !due || !self.dirty || self.conflict {
            return false;
        }
        self.save();
        true
    }

    // ---- keys ------------------------------------------------------------

    /// Handles one key. Returns `true` when merl should quit.
    pub fn key(&mut self, key: KeyEvent) -> bool {
        let quit = self.key_inner(key);
        if quit {
            self.flush();
        } else {
            tutor::check(self);
        }
        quit
    }

    fn key_inner(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }
        let mut key = key;
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
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let alt = key.modifiers.contains(KeyModifiers::ALT);

        if ctrl && key.code == KeyCode::Char('c') {
            return true;
        }
        if self.picker.is_some() {
            self.picker_key(key.code, ctrl);
            return false;
        }
        match self.mode {
            Mode::Goto => {
                self.goto_key(key.code);
                return false;
            }
            Mode::Find => {
                self.find_key(key.code);
                return false;
            }
            Mode::Search => {
                self.search_key(key.code);
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
        let extending = shift
            && match key.code {
                KeyCode::Up | KeyCode::Down => true,
                KeyCode::Left | KeyCode::Right => ctrl || alt,
                _ => false,
            };
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
            _ if self.focus == Focus::Tree => self.tree_key(key.code),
            KeyCode::Enter => self.start_edit(),
            KeyCode::Up if shift => self.extend(|s| s.move_line(-1)),
            KeyCode::Down if shift => self.extend(|s| s.move_line(1)),
            KeyCode::Up => self.move_line(-1),
            KeyCode::Down => self.move_line(1),
            KeyCode::Left if shift && ctrl => self.extend(Self::line_start),
            KeyCode::Right if shift && ctrl => self.extend(Self::line_end),
            KeyCode::Left if shift && alt => self.extend(Self::word_left),
            KeyCode::Right if shift && alt => self.extend(Self::word_right),
            // A plain arrow on a selection collapses it to the matching end, VS Code style.
            KeyCode::Left | KeyCode::Right if !shift && self.anchor.is_some() => {
                let (start, end) = self.selection().unwrap();
                (self.line, self.col) = self.clamp_pos(if key.code == KeyCode::Left {
                    start
                } else {
                    end
                });
                self.sync_want_x();
                self.anchor = None;
            }
            KeyCode::Left if shift => self.word_left(),
            KeyCode::Right if shift => self.word_right(),
            KeyCode::Left => self.left(),
            KeyCode::Right => self.right(),
            KeyCode::PageUp => self.move_line(-(self.view_h.max(1) as isize)),
            KeyCode::PageDown => self.move_line(self.view_h.max(1) as isize),
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
        // Any cursor move that is not an extending one drops the selection.
        if (self.line, self.col) != before && !extending {
            self.anchor = None;
        }
        self.hist_note(false);
        false
    }

    fn goto_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) if c.is_ascii_digit() => self.prompt.push(c),
            KeyCode::Backspace => {
                self.prompt.pop();
            }
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
            _ => {}
        }
    }

    fn search_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) => self.prompt.push(c),
            KeyCode::Backspace => {
                self.prompt.pop();
            }
            KeyCode::Enter => self.run_search(),
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.prompt.clear();
            }
            _ => {}
        }
    }
}

/// Cuts `s` to `max` chars, marking the cut.
fn clip(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((i, _)) => format!("{}\u{2026}", &s[..i]),
        None => s.to_string(),
    }
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
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
        let mut a = App::new(
            PathBuf::from("/tmp"),
            Tree::default(),
            Vec::new(),
            Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), text.as_bytes()),
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
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
        assert_eq!(a.col, 3);
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
        assert_eq!(a.col, 9);
        press(&mut a, KeyCode::Left, KeyModifiers::SHIFT);
        assert_eq!(a.col, 4);
        // Past the end of the line, jump to the start of the next one.
        a.col = 13;
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
        assert_eq!((a.line, a.col), (1, 0));
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
        // Back over the anchor: the range flips, it never collapses to nothing.
        for _ in 0..3 {
            press(&mut a, KeyCode::Up, KeyModifiers::SHIFT);
        }
        assert_eq!((a.line, a.selection()), (0, Some(((0, 2), (0, 2)))));
        // Any other cursor move drops it; Esc drops it too.
        press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
        press(&mut a, KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(a.selection(), None);
        press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
        assert!(a.selection().is_some());
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(a.selection(), None);
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
        // Shift+Left alone is still a plain word jump: it drops the selection.
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
        assert_eq!((a.col, a.selection()), (3, None));
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
        // Half a page (12 lines) at once is somewhere else: a new stop.
        press(&mut a, KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_eq!(
            a.history,
            [(x.clone(), 0, 0), (y.clone(), 3, 0), (y.clone(), 15, 0)]
        );
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(at(&a), (y.clone(), 3));
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(at(&a), (x.clone(), 0));
        press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
        press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
        assert_eq!(at(&a), (y, 15));
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
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "z");
        assert!(press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(std::fs::read(&path).unwrap(), b"xyza\r\nb\r\n");
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
    fn a_bad_search_regex_is_reported_as_such() {
        let mut a = app("foo(\n");
        press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
        typed(&mut a, "foo(");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert!(a.message.starts_with("bad pattern"), "{}", a.message);
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
