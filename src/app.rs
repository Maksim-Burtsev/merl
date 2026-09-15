//! All editor state and every key binding. Rendering lives in `ui.rs`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use regex::{Regex, RegexBuilder};

use crate::buffer::{self, Buffer};
use crate::picker::{Pick, PickItem, Picker};
use crate::search::{self, Hit, Kind};
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
    ("T", "Pick a theme (live preview)"),
    ("Tab", "Switch focus between tree and code"),
    ("Enter", "Edit at the cursor (Esc returns to navigation)"),
    (
        "Ctrl+S",
        "Save now (edits are saved on their own after a pause)",
    ),
    ("Ctrl+R", "Reload from disk, dropping unsaved edits"),
    ("Ctrl+Z / Ctrl+Y", "Undo / redo"),
    (
        "Edit: Ctrl+C / Ctrl+X",
        "Copy / cut the selection, or the line, to the clipboard",
    ),
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
    /// Linear per-file undo history, oldest first, and what undo took back.
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    /// Set when the next edit must start its own undo step even if it continues the last one.
    undo_break: bool,
    /// Text for the system clipboard, taken by `main` and sent to the terminal (OSC 52).
    pub clipboard: Option<String>,
    /// Lines that differ from the git index, painted in the gutter. Refreshed by `main` after
    /// every load, save and reload; between an edit and its autosave they lag by a second.
    pub marks: HashMap<usize, crate::git::Mark>,
    pub want_diff: bool,
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
            undo: Vec::new(),
            redo: Vec::new(),
            undo_break: false,
            clipboard: None,
            marks: HashMap::new(),
            want_diff: true,
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
    /// Returns `false`, with the reason in the status bar, when the file cannot be read or the
    /// open one has edits that could not be saved.
    fn open(&mut self, path: &Path, line: usize) -> bool {
        if self.buf.path.as_deref() != Some(path) {
            // Replacing the buffer would drop edits the disk does not have; the status bar
            // already says why they are not there (a conflict, a failed save).
            if !self.flush() {
                return false;
            }
            match Buffer::load(path) {
                Ok(mut buf) => {
                    // The standard library and dependencies are read here, never edited. A
                    // `.venv` or `node_modules` sits inside the root, so the roots decide.
                    let external = !path.starts_with(&self.root)
                        || self
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
                    self.marks.clear();
                    self.want_diff = true;
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
        self.undo.clear();
        self.redo.clear();
        self.want_diff = true;
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

    /// `T`: every theme, with the cursor on the one in use.
    fn open_themes_picker(&mut self) {
        let items = crate::theme::names()
            .map(|name| PickItem {
                label: name.to_string(),
                path: PathBuf::from(name),
                line: 0,
                code_at: None,
            })
            .collect();
        self.show_picker(PickerKind::Themes, items);
        if let Some(p) = &mut self.picker {
            p.selected = crate::theme::names()
                .position(|n| n == self.theme)
                .unwrap_or(0);
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

    fn picker_key(&mut self, code: KeyCode, ctrl: bool) {
        let Some(picker) = &mut self.picker else {
            return;
        };
        match picker.key(code, ctrl) {
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

    /// Enter in the `s>` prompt: a smart-case regex search over every file.
    fn run_search(&mut self) {
        let query = std::mem::take(&mut self.prompt);
        self.mode = Mode::Normal;
        if query.is_empty() {
            return;
        }
        let hits = match self.grep(&query, false, true, |_| true) {
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

    /// `d` / F12. A file of a known [`Kind`] gets its declaration patterns, searched only where
    /// such a definition can live (`.tsx` finds `.ts`, a Terraform variable stays in its
    /// module). Nothing in the project means the word comes from outside it: the same patterns
    /// run over the standard library and the installed dependencies
    /// ([`search::external_roots`]), narrowed to the module the file's imports bind the word or
    /// its qualifier to (`np.array` looks in `numpy`, `from json import load` in `json`). A
    /// field, a variant or a parameter has no declaration the rules know and gets "no
    /// definition": `u` lists the uses.
    fn goto_definition(&mut self) {
        let kind = self.kind();
        let extra = search::word_chars(kind, true);
        let Some((range, word)) = search::word_at(self.line_str(), self.col, extra) else {
            return;
        };
        let word = word.to_owned();
        let chain = search::qualifier(self.line_str(), range.start);
        let here = self.rel_current();
        let patterns = kind.map_or_else(Vec::new, |k| search::def_patterns(k, &word));
        let (Some(kind), Some(here)) = (kind, here) else {
            self.message = format!("no definition for {word}");
            return;
        };
        if patterns.is_empty() {
            self.message = format!("no definition for {word}");
            return;
        }
        let pattern = patterns.join("|");
        // Escaped or built-in patterns always compile.
        let mut hits = self
            .grep(&pattern, false, false, |p| {
                search::in_def_scope(kind, &here, p)
            })
            .unwrap_or_default();
        if let Some(block) = search::def_block(kind, &word) {
            hits.retain(|h| {
                std::fs::read_to_string(self.root.join(&h.path))
                    .is_ok_and(|text| search::directly_inside(&text, h.line, block))
            });
        }
        // Standing on one of the definitions is not a reason to go nowhere.
        if hits.len() > 1 {
            hits.retain(|h| h.line != self.line + 1 || h.path != here);
        }
        if hits.is_empty()
            && !matches!(
                chain.first().map(String::as_str),
                Some("self" | "cls" | "this")
            )
        {
            hits = self.external_definitions(kind, &word, &chain, &pattern);
        }
        match hits.len() {
            0 => self.message = format!("no definition for {word}"),
            1 => {
                let path = self.root.join(&hits[0].path);
                self.jump_to(&path, hits[0].line);
            }
            _ => {
                let items = self.external_items(kind, hits);
                self.show_picker(PickerKind::Definitions, items);
            }
        }
    }

    /// `pattern` over the standard library and dependencies of `kind`, in the module the file's
    /// imports bind `chain` (or `word` itself) to. The module path is relaxed from the end until
    /// files match: `from json import load` is `json/load`, then `json`. A qualifier no import
    /// binds is taken as the module path itself (`std::fs::read`, `os.path` behind a bare
    /// `import os`); a local variable in that position matches no file and the search stops.
    /// A bare word no import binds (`Vec`, `open`) searches every file.
    fn external_definitions(
        &mut self,
        kind: Kind,
        word: &str,
        chain: &[String],
        pattern: &str,
    ) -> Vec<Hit> {
        let text = self.buf.lines.join("\n");
        let imports = search::imports(kind, &text);
        let bound = |name: &str| {
            imports
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, p)| p.clone())
        };
        let imported = chain
            .first()
            .map_or(bound(word).is_some(), |f| bound(f).is_some());
        let mut module = match chain.first() {
            Some(first) => {
                let mut p = bound(first).unwrap_or_else(|| vec![first.clone()]);
                p.extend(chain[1..].iter().cloned());
                Some(p)
            }
            None => bound(word),
        };
        let all = self.external_files(kind);
        let roots = self.external[&kind].0.clone();
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
        // Absolute paths: `root.join` leaves them alone, so a hit opens where it is. The
        // standard library is the first root, and its hits come first.
        let grep = |files: &[PathBuf]| {
            let mut hits =
                search::grep_project(&self.root, files, pattern, false, false, None, None)
                    .unwrap_or_default();
            hits.sort_by_cached_key(|h| {
                (
                    roots.iter().position(|r| h.path.starts_with(r)),
                    h.path.clone(),
                    h.line,
                )
            });
            hits
        };
        if module.is_none() {
            return grep(&all);
        }
        let hits = grep(&files);
        // An imported module that does not declare the name re-exports it (`std::sync::Arc`
        // lives in `alloc`, a package's `__init__` pulls from its submodules): look everywhere.
        if hits.is_empty() && imported {
            grep(&all)
        } else {
            hits
        }
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

    /// [`Self::hit_items`] with an external hit shown relative to the root it came from:
    /// `json/__init__.py:278:` rather than the whole path to the interpreter.
    fn external_items(&mut self, kind: Kind, hits: Vec<Hit>) -> Vec<PickItem> {
        let roots = self
            .external
            .get(&kind)
            .map(|(roots, _)| roots.clone())
            .unwrap_or_default();
        let mut items = Self::hit_items(hits);
        for item in &mut items {
            if let Some(root) = roots.iter().find(|r| item.path.starts_with(r))
                && let Ok(rel) = item.path.strip_prefix(root)
            {
                let full = format!("{}:{}: ", item.path.display(), item.line);
                let short = format!("{}:{}: ", rel.display(), item.line);
                item.label = short.clone() + &item.label[full.len()..];
                item.code_at = Some(short.len());
            }
        }
        items
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
            let wanted = |p: &Path| kind.is_none() || search::kind_of(p) == *kind;
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
            KeyCode::Backspace | KeyCode::Delete if self.anchor.is_some() => self.insert(""),
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
                self.want_diff = true;
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

        if ctrl && key.code == KeyCode::Char('c') && self.mode != Mode::Edit {
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
        assert_eq!(
            (a.mode, a.message.as_str()),
            (Mode::Normal, "theme kanagawa-wave")
        );
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
        assert_eq!((a.mode, a.prompt.as_str()), (Mode::Search, "parse_it"));
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
        a.paste("app.rs");
        assert_eq!(a.picker.as_ref().unwrap().query, "app.rs");
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
        std::fs::remove_dir_all(&dir).unwrap();
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
