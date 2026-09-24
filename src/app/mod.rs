//! All editor state and every key binding. Rendering lives in `ui/`.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use regex::{Regex, RegexBuilder};
use unicode_segmentation::UnicodeSegmentation;

use crate::buffer::{self, Buffer};
use crate::git;
use crate::line_edit::LineEdit;
use crate::picker::{Pick, PickItem, Picker};
use crate::search::{self, Candidate, Hit, Kind, Reason, Tier};
use crate::tree::Tree;
use crate::tutor::{self, Tutor};
use crate::wrap;

mod cursor;
mod definition;
mod edit;
mod external;
mod find;
mod keys;
mod members;
mod missed;
mod open;
mod picker;
mod project_search;
mod review;
mod scroll;
mod search_job;
mod symbols;
mod tree;
mod typed;
mod usages;

pub use search_job::SearchJob;
use search_job::{SEARCH_PAUSE, Typed};

/// Longest hit text kept in a picker label; the rest is off the screen anyway.
const MAX_LABEL_TEXT: usize = 120;
/// Longest symbol name the symbol list pads to.
const MAX_NAME_PAD: usize = 40;

/// Every binding, in the order the `?` overlay and the README table list them. Single source of
/// truth: a test checks that the README says exactly this.
pub const KEYS: &[(&str, &str)] = &[
    ("o / Ctrl+E", "Open a file (fuzzy)"),
    (
        "Ctrl+N",
        "New file: type its path, Enter creates and edits it",
    ),
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
    ("m", "Review: mark the file as viewed, or take the mark off"),
    (": / Ctrl+G", "Go to line"),
    ("t", "Show or hide the file tree"),
    ("T", "Pick a theme (live preview)"),
    (
        "w",
        "Wrap long lines, or cut them at the edge and scroll sideways",
    ),
    ("Tab", "Switch focus between tree and code"),
    ("Enter", "Edit at the cursor (Esc returns to navigation)"),
    (
        "Ctrl+S",
        "Save now (edits are saved on their own after a pause)",
    ),
    ("Ctrl+R", "Reload from disk, dropping unsaved edits"),
    ("Ctrl+Z / Ctrl+Y", "Undo / redo"),
    (
        "Ctrl+C",
        "Copy the selection, or the line, to the clipboard",
    ),
    ("Edit: Ctrl+X", "Cut the selection, or the line"),
    (
        "Edit: Alt+Backspace / Alt+Delete",
        "Delete the word before / after the cursor",
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
    ("q", "Quit"),
    ("Tree: Up / Down", "Move"),
    ("Tree: Enter", "Open the file, or expand the directory"),
    ("Tree: Left / Right", "Collapse / expand"),
    ("Picker: Up / Down", "Move"),
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
    Goto,
    /// Ctrl+N: the path of a file to create.
    New,
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
/// Columns kept between the cursor and the pane edge while scrolling sideways.
const SIDE_OFF: usize = 8;
const HIST_NEAR: usize = 10;
const HIST_MAX: usize = 50;

pub struct App {
    pub root: PathBuf,
    /// Set by `main` for a file opened outside any repository: the project is the files right
    /// in the root, and every walk stops there (#182).
    pub shallow: bool,
    pub buf: Buffer,
    pub tree: Tree,
    /// Every file under the root that is not ignored, sorted like the tree: what `o` offers
    /// and `s`, `u` and `d` search.
    pub files: Vec<PathBuf>,
    /// The ignored files of the walked directories (`.env`): `o` offers them after `files`,
    /// dim, and nothing searches them.
    pub ignored: Vec<PathBuf>,
    /// Per kind, the standard library and dependency roots outside the project and the files of
    /// that kind under them; filled the first time `d` leaves the project.
    external: HashMap<Kind, (Vec<PathBuf>, Arc<Vec<PathBuf>>)>,
    /// The walk of each `node_modules`, and the file the TypeScript entry of `external` was put
    /// together for: a workspace has one per package, and each file sees those above it.
    node_modules: HashMap<PathBuf, Arc<Vec<PathBuf>>>,
    node_modules_of: Option<PathBuf>,
    /// What `go build` compiles here, which picks among a Go declaration's twins.
    go_build: search::GoBuild,
    /// The candidates of this `d` are to be offered, not jumped to, however few: the word is a
    /// keyword argument, which names a parameter no rule reads.
    offer_only: bool,
    /// Set by a grep of this `d` that stopped at [`search::MAX_HITS`], whatever was filtered out
    /// of it afterwards: the candidates are a lower bound, so the count says `+` and a single
    /// one is offered, not jumped to.
    truncated: std::cell::Cell<bool>,
    /// The bindings (`path`, `line`) a type is being read from, outermost first: a binding met
    /// again inside its own reading, `this.close = this.close.bind(this)`, is left out (#175).
    reading: std::cell::RefCell<Vec<(PathBuf, usize)>>,
    pub focus: Focus,
    pub show_tree: bool,
    /// First visible row of the tree pane, clamped by `ui`.
    pub tree_top: usize,
    pub picker: Option<Picker>,
    /// `s`: the number of the query on screen. Every change of the query takes the next one, so
    /// the answer to an older query is recognised and dropped.
    search_seq: u64,
    /// When the query last changed and has not been grepped yet: the grep waits for a pause.
    search_due: Option<Instant>,
    /// The `search_seq` a thread is grepping for.
    search_sent: Option<u64>,
    /// Enter came before the answer to the query on screen: jump when it arrives.
    search_enter: bool,
    /// Stops of the jump history, oldest first; `hist_idx` is the current one and follows
    /// the cursor (see `hist_note`).
    pub history: Vec<(PathBuf, usize, usize)>,
    pub hist_idx: usize,
    /// Cursor: file line, byte offset into that line, and the display column Up/Down aims for.
    pub line: usize,
    pub col: usize,
    pub want_x: usize,
    /// (line, col) where the selection started; it runs from here to the cursor.
    anchor: Option<(usize, usize)>,
    /// Top of the viewport: a file line plus which wrapped row of it is first on screen.
    pub top_line: usize,
    pub top_row: usize,
    /// Display columns scrolled off to the left while the file is not wrapped; follows the
    /// cursor in `clamp_scroll`.
    pub left: usize,
    /// Files `w` was pressed on: their wrapping is the opposite of what their kind gets.
    wrap_toggled: HashSet<PathBuf>,
    pub mode: Mode,
    /// What has been typed into the `:` or `/` prompt.
    pub prompt: LineEdit,
    /// The current query, kept for `n`/`N` and for painting the matches.
    pub find_re: Option<Regex>,
    /// The text `find_re` was built from: `/` opens with it again while the pattern is active.
    find_query: String,
    /// Where the cursor was when `/` was pressed: the start of the incremental search.
    find_anchor: (usize, usize),
    /// The selection anchor, set aside while `/` moves the cursor; Esc puts it back.
    find_sel: Option<(usize, usize)>,
    pub message: String,
    /// Code text area, in cells, written by `ui::draw` before every frame.
    pub view_w: usize,
    pub view_h: usize,
    center: bool,
    /// `--tutor` and `--drill` only: the running tutorial or drill. `None` in a normal session.
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
    /// The history of each file left with one, by path (#163): coming back is like coming back
    /// to a VS Code tab that stayed open.
    stash: HashMap<PathBuf, Stashed>,
    /// The overlay on screen (find, goto, a prompt, a picker) was opened from edit mode with a
    /// chord alias: closing it without leaving the file goes back to editing.
    resume_edit: bool,
    /// Text for the system clipboard, taken by `main` and sent to the terminal (OSC 52).
    pub clipboard: Option<String>,
    /// Lines that differ from the git index, painted in the gutter. Refreshed by `main` after
    /// every load, save and reload; between an edit and its autosave they lag by a second.
    /// In review mode: against the branch's base, with ghosts, taken on open (see `refresh_diff`).
    pub diff: git::Diff,
    pub want_diff: bool,
    /// `--review`: the branch under review. The tree pane then lists its files.
    pub review: Option<git::Review>,
    /// Review: the files marked as viewed, each with the hash of what was on disk then. A file
    /// that has changed since is not viewed any more (`drop_stale_viewed`). Kept for the session.
    pub viewed: HashMap<PathBuf, u64>,
    /// The theme in use, by name. Set by `main`; the theme picker previews others over it.
    pub theme: String,
    /// Where Enter in the theme picker saves the choice. Set by `main`; `None` saves nothing.
    pub config: Option<PathBuf>,
    /// A quit was just refused over edits that could not be saved: quitting again right away
    /// leaves them behind.
    quit_again: bool,
    /// The `KEYS` action the key being handled was routed to, named by `key_inner` for `key`.
    action: Option<&'static str>,
    /// Real work's presses by action, since merl started: `main` adds them to the key stats on
    /// exit. The tutorial counts nothing.
    pub pressed: HashMap<&'static str, u64>,
    /// Real work's missed keys by action (#210), added to the key stats with the presses.
    pub missed: HashMap<&'static str, u64>,
    watch: missed::Watch,
}

/// A file's lines and format as it was left, and its undo and redo stacks.
type Stashed = (Vec<String>, buffer::Format, Vec<Edit>, Vec<Edit>);

/// One undoable change: `old` lines from `line` on became `new`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Edit {
    line: usize,
    old: Vec<String>,
    new: Vec<String>,
    /// Cursor before and after, for undo and redo to put it back.
    before: (usize, usize),
    after: (usize, usize),
    /// A reload's: the file's format before and after, which undo and redo put back too.
    format: Option<(buffer::Format, buffer::Format)>,
}

/// `1 hit`, `2 hits`.
pub fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// A file merl has no permission to write is read-only before the first keystroke, not after the
/// first failed save (#123). The test is an open for writing, not the permission bits: the bits
/// lie in both directions — root writes a 444 file, an ACL or a read-only mount refuses a 644 one
/// — and a save is `fs::write` on this very path, so the open it would do is the honest question.
/// Nothing is truncated, so the file is left as it was. A file that is not there is not a file
/// that refuses to be written: merl keeps the text of one deleted under it, and Ctrl+S puts it
/// back. Every other reason says more, so this one never takes a place.
fn lock_no_write(buf: &mut Buffer) {
    let Some(path) = buf.path.as_deref() else {
        return;
    };
    if let Err(e) = std::fs::OpenOptions::new().write(true).open(path)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        buf.readonly.get_or_insert("no write permission");
    }
}

impl App {
    pub fn new(
        root: PathBuf,
        tree: Tree,
        files: Vec<PathBuf>,
        mut buf: Buffer,
        at: Option<(usize, usize)>,
    ) -> Self {
        let focus = if buf.path.is_some() {
            Focus::Code
        } else {
            Focus::Tree
        };
        let ignored = tree.ignored_files();
        let mut app = Self {
            root,
            shallow: false,
            buf: Buffer::empty(),
            tree,
            files,
            ignored,
            external: HashMap::new(),
            node_modules: HashMap::new(),
            node_modules_of: None,
            go_build: search::GoBuild::host().env(
                std::env::var("CGO_ENABLED").ok().as_deref(),
                std::env::var("GOFLAGS").ok().as_deref(),
            ),
            offer_only: false,
            truncated: Default::default(),
            reading: Default::default(),
            focus,
            show_tree: true,
            tree_top: 0,
            picker: None,
            search_seq: 0,
            search_due: None,
            search_sent: None,
            search_enter: false,
            history: Vec::new(),
            hist_idx: 0,
            line: 0,
            col: 0,
            want_x: 0,
            anchor: None,
            top_line: 0,
            top_row: 0,
            left: 0,
            wrap_toggled: HashSet::new(),
            mode: Mode::Normal,
            prompt: LineEdit::default(),
            find_re: None,
            find_query: String::new(),
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
            stash: HashMap::new(),
            resume_edit: false,
            clipboard: None,
            diff: git::Diff::default(),
            want_diff: true,
            review: None,
            viewed: HashMap::new(),
            theme: crate::theme::DEFAULT.to_string(),
            config: None,
            quit_again: false,
            action: None,
            pressed: HashMap::new(),
            missed: HashMap::new(),
            watch: Default::default(),
        };
        // The file named on the command line is asked the same question as one opened later.
        lock_no_write(&mut buf);
        app.buf = buf;
        if let Some((n, c)) = at {
            app.goto_line(n);
            // 1-based in chars, as compilers count; past the end of the line is its end.
            let s = app.line_str();
            app.col = s
                .char_indices()
                .nth(c.saturating_sub(1))
                .map_or(s.len(), |(i, _)| i);
            app.sync_want_x();
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

    /// Whether long lines are cut at the pane edge instead of wrapped: `w` flips it for the open
    /// file. Tables of values start out cut, since wrapping takes their columns apart.
    pub fn nowrap(&self) -> bool {
        let Some(path) = &self.buf.path else {
            return false;
        };
        let table = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("csv") || e.eq_ignore_ascii_case("tsv"));
        table != self.wrap_toggled.contains(path)
    }

    /// `w`: wrap the open file or stop wrapping it.
    fn toggle_wrap(&mut self) {
        let Some(path) = self.buf.path.clone() else {
            return;
        };
        if !self.wrap_toggled.remove(&path) {
            self.wrap_toggled.insert(path);
        }
        // The top row and the column Up / Down aim at were counted in the other layout.
        self.clamp_top();
        self.sync_want_x();
        self.message = if self.nowrap() {
            "wrap off, the view follows the cursor".into()
        } else {
            "wrap on".into()
        };
    }

    /// Screen rows of file line `l` at the current viewport width, over the text `ui` draws:
    /// the wrapped rows, or the whole line as one row when the file is not wrapped.
    pub fn rows(&self, l: usize) -> Vec<std::ops::Range<usize>> {
        let shown = self.buf.shown(l);
        if self.nowrap() {
            return std::iter::once(0..shown.len()).collect();
        }
        wrap::wrap_line(shown, self.view_w)
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
    /// are drawn with. Not wrapped, the row is the line: `left` of these columns are off screen.
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
}

/// A call and the return type its function declares, spelled as the language writes it.
fn signature(kind: Kind, callee: &str, returns: &str) -> String {
    match kind {
        Kind::Python => format!("{callee}() -> {returns}"),
        Kind::TsJs => format!("{callee}(): {returns}"),
        _ => format!("{callee}() {returns}"),
    }
}

/// The module path an import in `imports` binds `name` to.
fn bound(imports: &[(String, Vec<String>)], name: &str) -> Option<Vec<String>> {
    imports
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, p)| p.clone())
}

/// Cuts `s` to `max` grapheme clusters, marking the cut.
pub(crate) fn clip(s: &str, max: usize) -> String {
    match s.grapheme_indices(true).nth(max) {
        Some((i, _)) => format!("{}\u{2026}", &s[..i]),
        None => s.to_string(),
    }
}

/// A word for the cursor: letters of any script, so a comment in Russian moves by word too.
pub fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The byte after the grapheme cluster at `i`: an emoji with its selector, skin tone or ZWJ
/// parts is one step, as it is one cell on screen, and so is a letter with its accents.
pub(crate) fn next_char(s: &str, i: usize) -> usize {
    s[i..].graphemes(true).next().map_or(i, |g| i + g.len())
}

/// The start of the grapheme cluster before `i`, as [`next_char`] steps.
pub(crate) fn prev_char(s: &str, i: usize) -> usize {
    s[..i]
        .graphemes(true)
        .next_back()
        .map_or(0, |g| i - g.len())
}

/// Whether a line `search::bindings` gave for `name` is an import, or the declaration of a class,
/// a function or a namespace of that name: what a value of the name would hide, and no value
/// itself. `Outer.Inner` reads a declaration, not a member.
fn names_itself(line: &str, name: &str) -> bool {
    let t = line.trim_start();
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
        rest.strip_prefix(name)
            .is_some_and(|after| !after.starts_with(is_word))
    });
    declares || t.starts_with("import ") || t.starts_with("from ")
}

#[cfg(test)]
mod tests;
