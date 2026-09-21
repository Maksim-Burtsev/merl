//! All editor state and every key binding. Rendering lives in `ui.rs`.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use regex::{Regex, RegexBuilder};

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
    pub buf: Buffer,
    pub tree: Tree,
    /// Every file under the root, sorted like the tree; the file picker's item list.
    pub files: Vec<PathBuf>,
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
    /// The theme in use, by name. Set by `main`; the theme picker previews others over it.
    pub theme: String,
    /// Where Enter in the theme picker saves the choice. Set by `main`; `None` saves nothing.
    pub config: Option<PathBuf>,
    /// A quit was just refused over edits that could not be saved: quitting again right away
    /// leaves them behind.
    quit_again: bool,
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
        let mut app = Self {
            root,
            buf: Buffer::empty(),
            tree,
            files,
            external: HashMap::new(),
            node_modules: HashMap::new(),
            node_modules_of: None,
            go_build: search::GoBuild::host().env(
                std::env::var("CGO_ENABLED").ok().as_deref(),
                std::env::var("GOFLAGS").ok().as_deref(),
            ),
            offer_only: false,
            truncated: Default::default(),
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
            theme: crate::theme::DEFAULT.to_string(),
            config: None,
            quit_again: false,
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
        self.top_row = self.top_row.min(self.row_count(self.top_line) - 1);
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

fn next_char(s: &str, i: usize) -> usize {
    s[i..].chars().next().map_or(i, |c| i + c.len_utf8())
}

fn prev_char(s: &str, i: usize) -> usize {
    s[..i].chars().next_back().map_or(0, |c| i - c.len_utf8())
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
mod tests {
    use super::definition::resolution;
    use super::open::carried;
    use super::*;

    fn app(text: &str) -> App {
        // Edits get saved, so each test has a file of its own.
        let name = std::thread::current()
            .name()
            .unwrap_or("main")
            .replace("::", "-");
        let path = std::env::temp_dir().join(format!("merl-{name}.txt"));
        std::fs::write(&path, text).unwrap();
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
    fn ctrl_c_copies_in_navigation_and_never_quits() {
        let mut a = app("abc");
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
        press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
        assert!(!press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(a.clipboard.take().as_deref(), Some("ab"));
        assert!(a.selection().is_some(), "copy leaves the selection");
        // Cmd+C, from a terminal that passes it on, copies too.
        assert!(!press(&mut a, KeyCode::Char('c'), KeyModifiers::SUPER));
        assert_eq!(a.clipboard.take().as_deref(), Some("ab"));
        // Without a selection the line goes, and merl stays open.
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert!(!press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(a.clipboard.take().as_deref(), Some("abc"));
        assert_eq!(a.message, "copied 1 line");
        // A prompt has nothing to copy, and Ctrl+C does not quit from it either.
        press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
        assert!(!press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL));
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

    /// `merl FILE:LINE:COL` (#161): the column counts chars, as compilers do, and a column past
    /// the end of the line is its end. The first stop in the jump history has it too.
    #[test]
    fn command_line_column_counts_chars() {
        let path = PathBuf::from("/tmp/merl-column.txt");
        let at = |line, col| {
            let buf = Buffer::from_bytes(path.clone(), "one\nhéllo\n".as_bytes());
            App::new(
                PathBuf::from("/tmp"),
                Tree::default(),
                Vec::new(),
                buf,
                Some((line, col)),
            )
        };
        let a = at(2, 3);
        assert_eq!((a.line, a.col, a.display_col()), (1, 3, 3));
        assert_eq!(a.history, [(path.clone(), 1, 3)]);
        assert_eq!(at(2, 99).col, "héllo".len());
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
        assert_eq!(a.mode, Mode::Picker(PickerKind::Search));
        assert_eq!(&*a.picker.as_ref().unwrap().query, "parse_it");
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

    /// #75: the walk is redone while merl runs. The tree cursor keeps its entry and its screen
    /// row, an open `o` keeps its rows until it is reopened, and the review panel is not the walk.
    #[test]
    fn a_new_walk_keeps_the_cursor_row_and_an_open_picker() {
        let (dir, mut a) = files_app("live");
        let walk = |a: &mut App| {
            let (tree, files) = crate::tree::build(&a.root);
            a.project_walked(tree, files);
        };
        walk(&mut a);
        a.tree.reveal(Path::new("b.rs"));
        std::fs::write(dir.join("a2.rs"), "x\n").unwrap();
        walk(&mut a);
        assert_eq!(
            a.tree_top, 0,
            "a panel showing its first row keeps showing it"
        );
        std::fs::remove_file(dir.join("a2.rs")).unwrap();
        walk(&mut a);
        a.tree_top = 1;
        press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);

        std::fs::write(dir.join("a0.rs"), "x\n").unwrap();
        std::fs::remove_file(dir.join("a.rs")).unwrap();
        std::fs::write(dir.join("a1.rs"), "x\n").unwrap();
        walk(&mut a);
        assert_eq!(a.files, ["a0.rs", "a1.rs", "b.rs"].map(PathBuf::from));
        assert_eq!(a.tree.selected().unwrap().path, Path::new("b.rs"));
        assert_eq!(
            a.tree_top, 2,
            "one row more above the cursor: the scroll follows"
        );
        let p = a.picker.as_mut().unwrap();
        p.settle();
        assert_eq!(p.counts().1, 2, "the open picker still lists a.rs and b.rs");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
        let p = a.picker.as_mut().unwrap();
        p.settle();
        assert_eq!(p.counts().1, 3);
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);

        // Rows leaving above the cursor pull the scroll back, never below the first row.
        a.tree.reveal(Path::new("b.rs"));
        a.tree_top = 2;
        std::fs::remove_file(dir.join("a0.rs")).unwrap();
        walk(&mut a);
        assert_eq!(a.tree_top, 1);
        a.tree_top = 1;
        std::fs::remove_file(dir.join("a1.rs")).unwrap();
        std::fs::write(dir.join("c.rs"), "x\n").unwrap();
        walk(&mut a);
        assert_eq!((a.tree_top, a.tree.cursor), (0, 0));

        let (_, mut r) = review_app("live-review");
        let panel = |r: &App| {
            let rows = r.tree.nodes.iter().map(|n| (n.path.clone(), n.expanded));
            (rows.collect::<Vec<_>>(), r.tree.cursor, r.tree_top)
        };
        let before = panel(&r);
        let (tree, files) = crate::tree::build(&r.root);
        r.project_walked(tree, files.clone());
        assert_eq!((panel(&r), &r.files), (before, &files));
    }

    /// A repository with a `feature` branch checked out: `src/a.rs` changed twice, `new`
    /// added, `gone` deleted, `src/keep.rs` as it was; the app is in review mode on it.
    fn review_app(tag: &str) -> (PathBuf, App) {
        review_app_with(tag, &[])
    }

    /// `extra`: files the branch adds on top of the usual five.
    fn review_app_with(tag: &str, extra: &[(&str, &[u8])]) -> (PathBuf, App) {
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
        std::fs::write(dir.join("src/keep.rs"), "k1\nk2\nk3\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "base"]);
        git(&["switch", "-q", "-c", "feature"]);
        std::fs::write(dir.join("src/a.rs"), "a\nB\nc\nd\ne\nF\n").unwrap();
        std::fs::write(dir.join("new"), "n\n").unwrap();
        std::fs::remove_file(dir.join("gone")).unwrap();
        std::fs::write(dir.join("tail"), "t1\n").unwrap();
        std::fs::write(dir.join("crlf.txt"), "one\r\nTWO\n").unwrap();
        for (name, bytes) in extra {
            std::fs::write(dir.join(name), bytes).unwrap();
        }
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
    fn review_walks_past_files_without_hunks() {
        let png: &[u8] = b"\x89PNG\0\0";
        let (dir, mut a) = review_app_with("reviewbin", &[("a.png", png), ("z.png", png)]);
        // Panel order: src/a.rs, a.png, crlf.txt, gone, new, tail, z.png.
        let r = a.review.as_ref().unwrap();
        assert!(r.files[1].binary && !r.files[1].has_hunks());
        assert!(!r.files[2].binary && r.files[2].has_hunks());
        press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("tail"), 0));
        // Only z.png is left: not a stop.
        press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("tail"), 0));
        assert_eq!(a.message, "last hunk of the review");
        for _ in 0..3 {
            press(&mut a, KeyCode::Char('C'), KeyModifiers::NONE);
        }
        assert_eq!(at(&a), (dir.join("crlf.txt"), 1));
        press(&mut a, KeyCode::Char('C'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("src/a.rs"), 5));
        assert_eq!(a.message, "skipped 1 file without hunks");
        assert_eq!(a.review_status().unwrap(), "hunk 2/2  file 1/7");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// #76: the review is left open next to an agent. What `main` does on a change is
    /// `Review::refresh` and `review_refreshed`; the open file, the cursor and both scrolls stay.
    #[test]
    fn a_live_review_follows_edits_untracked_files_and_commits() {
        let (dir, mut a) = review_app("livereview");
        let git = |args: &[&str]| {
            let mut cmd = std::process::Command::new("git");
            let out = cmd.arg("-C").arg(&dir).args(args).output().unwrap();
            assert!(out.status.success(), "git {args:?}");
        };
        let refresh = |a: &mut App| {
            let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
            a.review_refreshed(fresh)
        };
        let rows = |a: &App| -> Vec<(char, String, usize, usize)> {
            let files = a.review.as_ref().unwrap().files.iter();
            files
                .map(|f| (f.status, f.path.display().to_string(), f.added, f.deleted))
                .collect()
        };
        let row = |s: char, p: &str, n: usize, m: usize| (s, p.to_string(), n, m);
        // Where the reader is: the code pane, the panel cursor and its row on screen.
        let place = |a: &App| {
            let vis = a.tree.visible();
            let row = vis.iter().position(|&i| i == a.tree.cursor).unwrap();
            let selected = a.tree.selected().unwrap().path.clone();
            (at(a), a.col, a.top_line, selected, row - a.tree_top)
        };
        assert!(!refresh(&mut a), "nothing changed: nothing to draw");
        a.tree.reveal(Path::new("new"));
        a.tree_top = 1;
        let before = place(&a);
        assert_eq!(a.review_status().unwrap(), "hunk 1/1  file 4/5");

        // The agent touches a file for the first time, writes a module in a new directory and
        // one at the root, neither added, and a log that is ignored.
        std::fs::write(dir.join("src/keep.rs"), "k1\nk2\nK\nk3\n").unwrap();
        std::fs::create_dir(dir.join("pkg")).unwrap();
        std::fs::write(dir.join("pkg/mod.py"), "x = 1\ny = 2\n").unwrap();
        std::fs::write(dir.join("pkg/logo.png"), b"\x89PNG\0").unwrap();
        std::fs::write(dir.join("zz.py"), "z = 1\nz = 2\nlast").unwrap();
        std::fs::write(dir.join(".gitignore"), "*.log\n").unwrap();
        std::fs::write(dir.join("agent.log"), "noise\n").unwrap();
        assert!(refresh(&mut a));
        assert_eq!(
            rows(&a),
            vec![
                row('A', "pkg/logo.png", 0, 0),
                row('A', "pkg/mod.py", 2, 0),
                row('M', "src/a.rs", 2, 2),
                row('M', "src/keep.rs", 1, 0),
                row('A', ".gitignore", 1, 0),
                row('M', "crlf.txt", 1, 1),
                row('D', "gone", 0, 2),
                row('A', "new", 1, 0),
                row('M', "tail", 0, 2),
                row('A', "zz.py", 3, 0),
            ]
        );
        assert_eq!(a.review_status().unwrap(), "hunk 1/1  file 8/10");
        assert_eq!(place(&a), before, "rows came in above: nothing moved");
        let shown = |a: &App, p: &str| {
            a.tree
                .visible()
                .iter()
                .any(|&i| a.tree.nodes[i].path == Path::new(p))
        };
        assert!(shown(&a, "pkg/mod.py"), "a new directory comes open");
        // A directory the reader closed stays closed.
        a.tree.reveal(Path::new("src"));
        a.tree.collapse();
        a.tree.reveal(Path::new("new"));
        std::fs::write(dir.join("pkg/mod.py"), "x = 1\n").unwrap();
        assert!(refresh(&mut a));
        assert_eq!(rows(&a)[1], row('A', "pkg/mod.py", 1, 0));
        assert!(a.review.as_ref().unwrap().files[0].binary);
        assert!(!shown(&a, "src/a.rs"));

        // `c` walks into the untracked file as into any added one: every line is added.
        press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("zz.py"), 0));
        assert_eq!((a.diff.marks.len(), &a.diff.hunks), (3, &vec![0]));
        assert_eq!(a.review_status().unwrap(), "hunk 1/1  file 10/10");

        // A reverted file leaves the panel. The open one stays open, without marks.
        a.jump_to(&dir.join("src/keep.rs"), 3);
        assert_eq!(a.diff.marks.len(), 1);
        std::fs::write(dir.join("src/keep.rs"), "k1\nk2\nk3\n").unwrap();
        a.reload(false);
        assert!(refresh(&mut a));
        assert!(
            a.review
                .as_ref()
                .unwrap()
                .file(Path::new("src/keep.rs"))
                .is_none()
        );
        assert_eq!(at(&a), (dir.join("src/keep.rs"), 2));
        assert!(a.diff.marks.is_empty());
        assert_eq!(a.review_status().unwrap(), "hunk 0/0  file -/9");

        // The agent commits: the rows are the same ones, tracked now. Then the base moves up to
        // the branch's first commit, and only the second one is left to review; `new`, open
        // and not touched on disk, loses its marks.
        a.jump_to(&dir.join("new"), 1);
        assert_eq!(a.diff.marks.len(), 1);
        let listed = rows(&a);
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "more work"]);
        assert!(
            refresh(&mut a),
            "the rows are tracked now: `untracked` went"
        );
        assert_eq!(rows(&a), listed);
        assert_eq!(a.review_status().unwrap(), "hunk 1/1  file 7/9");
        git(&["branch", "-f", "main", "HEAD~1"]);
        assert!(refresh(&mut a));
        assert_eq!(
            rows(&a),
            vec![
                row('A', "pkg/logo.png", 0, 0),
                row('A', "pkg/mod.py", 1, 0),
                row('A', ".gitignore", 1, 0),
                row('A', "zz.py", 3, 0),
            ]
        );
        assert_eq!(at(&a), (dir.join("new"), 0));
        assert!(a.diff.marks.is_empty());
        assert_eq!(a.review_status().unwrap(), "hunk 0/0  file -/4");
        // The base caught up with the branch: an empty panel, and keys that find no row.
        git(&["branch", "-f", "main", "HEAD"]);
        assert!(refresh(&mut a));
        assert_eq!(a.review_status().unwrap(), "hunk 0/0  file -/0");
        a.focus = Focus::Tree;
        for key in [KeyCode::Down, KeyCode::Enter, KeyCode::Char('c')] {
            press(&mut a, key, KeyModifiers::NONE);
        }
        assert_eq!(at(&a), (dir.join("new"), 0));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// #76: the marks of the open file are read again when the base moves under it or its row
    /// changes kind, with no write to the file itself, which is what `reload` answers.
    #[test]
    fn the_open_file_gets_new_marks_when_only_the_branch_changed() {
        let (dir, mut a) = review_app("livemarks");
        let git = |args: &[&str]| {
            let mut cmd = std::process::Command::new("git");
            let out = cmd.arg("-C").arg(&dir).args(args).output().unwrap();
            assert!(out.status.success(), "git {args:?}");
        };
        let refresh = |a: &mut App| {
            let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
            a.review_refreshed(fresh)
        };
        let keep = dir.join("src/keep.rs");
        std::fs::write(&keep, "k1\nK2\nk3\n").unwrap();
        git(&["commit", "-qam", "second"]);
        std::fs::write(&keep, "k1\nK2\nK3\n").unwrap();
        git(&["commit", "-qam", "third"]);
        assert!(refresh(&mut a));
        a.jump_to(&keep, 1);
        assert_eq!(a.diff.marks.len(), 2);
        // The base moves up to `second`: the row stays `M`, one mark is left.
        git(&["branch", "-f", "main", "HEAD~1"]);
        assert!(refresh(&mut a));
        assert_eq!(a.diff.marks.len(), 1);
        // The file leaves the index: the same merge base, a row of another kind, every line
        // added. One row: the file on disk, not the deletion.
        git(&["rm", "-q", "--cached", "src/keep.rs"]);
        assert!(refresh(&mut a));
        let rows = a.review.as_ref().unwrap().files.iter();
        let rows: Vec<_> = rows
            .filter(|f| f.path == Path::new("src/keep.rs"))
            .collect();
        assert_eq!(
            (rows.len(), rows[0].status, rows[0].untracked),
            (1, 'A', true)
        );
        assert_eq!(a.diff.marks.len(), 3);
        assert_eq!(at(&a), (keep, 0));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// #76: the branch deleted `gone` and the agent writes it again, not added: git lists the
    /// path as deleted and as untracked. It is one row, read from disk, and `c` walks past it.
    #[test]
    fn a_deleted_file_written_again_is_one_row() {
        let (dir, mut a) = review_app("liveagain");
        std::fs::write(dir.join("gone"), "fresh\n").unwrap();
        let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
        assert!(a.review_refreshed(fresh));
        let names = a.tree.nodes.iter().map(|n| n.path.to_str().unwrap());
        assert_eq!(
            names.collect::<Vec<_>>(),
            vec!["src", "src/a.rs", "crlf.txt", "gone", "new", "tail"]
        );
        press(&mut a, KeyCode::Char('C'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("gone"), 0));
        assert_eq!(
            (a.buf.lines.clone(), a.buf.readonly),
            (vec!["fresh".to_string()], None)
        );
        press(&mut a, KeyCode::Char('C'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("crlf.txt"), 1));
        press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("new"), 0));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// #76: the reader is in the second hunk when the agent adds one above it. The cursor and
    /// the scroll go down with the text, so the hunk is still the current one and `c` goes on.
    #[test]
    fn the_current_hunk_stays_current_when_one_is_added_above() {
        let (dir, mut a) = review_app("livehunk");
        a.jump_to(&dir.join("src/a.rs"), 6);
        a.view_h = 4;
        a.clamp_scroll();
        a.top_line = 3;
        a.anchor = Some((4, 0));
        let stop = |a: &App| a.history[a.hist_idx].clone();
        assert_eq!(stop(&a), (dir.join("src/a.rs"), 5, 0));
        assert_eq!(a.review_status().unwrap(), "hunk 2/2  file 1/5");
        std::fs::write(dir.join("src/a.rs"), "new\nnew\na\nB\nc\nd\ne\nF\n").unwrap();
        assert!(a.reload(false));
        assert_eq!(
            (a.line, a.top_line, a.buf.lines[a.line].as_str()),
            (7, 5, "F")
        );
        assert_eq!(a.review_status().unwrap(), "hunk 3/3  file 1/5");
        // What is selected is still selected, and the history stop is still under the cursor.
        assert_eq!(a.anchor, Some((6, 0)));
        assert_eq!(stop(&a), (dir.join("src/a.rs"), 7, 0));
        a.anchor = None;
        // A block deleted above the pane, from a file longer than what is left of it: the
        // scroll is carried from where it was, not from where the shorter file clamps it.
        let text = |r: std::ops::Range<usize>| r.map(|i| format!("l{i}\n")).collect::<String>();
        std::fs::write(dir.join("src/a.rs"), text(0..40)).unwrap();
        a.reload(false);
        (a.view_h, a.line, a.top_line) = (10, 35, 30);
        std::fs::write(dir.join("src/a.rs"), text(25..40)).unwrap();
        a.reload(false);
        assert_eq!(
            (a.line, a.top_line, a.buf.lines[a.line].as_str()),
            (10, 5, "l35")
        );
        std::fs::write(dir.join("src/a.rs"), "new\nnew\na\nB\nc\nd\ne\nF\n").unwrap();
        a.reload(false);
        a.jump_to(&dir.join("src/a.rs"), 8);
        press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("crlf.txt"), 1));
        // Written below the cursor, or the cursor's own line: nothing to follow.
        let text = |n: usize| (0..n).map(|i| format!("{i}\n")).collect::<String>();
        for (old, new, l, want) in [
            (text(6), text(6) + "x\n", 3, 3),
            (text(6), "x\n".to_string() + &text(6), 3, 4),
            (text(6), text(6).replace("3\n", "x\ny\n"), 3, 3),
            (text(6), text(6).replace("1\n", ""), 3, 2),
            ("a\nb\n".into(), "a\na\nb\n".into(), 0, 0),
            // The cursor's own lines went: the line that followed them.
            (text(6), text(6).replace("2\n3\n", ""), 3, 2),
        ] {
            let lines = |s: &str| s.lines().map(String::from).collect::<Vec<_>>();
            assert_eq!(
                carried(&lines(&old), &lines(&new), l),
                want,
                "{old:?} {new:?}"
            );
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    /// A submodule is a gitlink, which `git diff --numstat` counts as one added line: listed,
    /// not a stop. And a file that does not open is walked past with the reason, not retried.
    #[test]
    fn review_walks_past_a_submodule_and_a_file_that_does_not_open() {
        let (dir, mut a) = review_app("reviewsub");
        let git = |at: &Path, args: &[&str]| {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(at)
                .args(["-c", "user.email=t@t", "-c", "user.name=t"])
                .args(args)
                .output()
                .unwrap();
            assert!(out.status.success(), "git {args:?}");
        };
        let sub = dir.join("sub");
        std::fs::create_dir(&sub).unwrap();
        git(&sub, &["init", "-q"]);
        git(&sub, &["commit", "-q", "--allow-empty", "-m", "sub"]);
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "-q", "-m", "submodule"]);
        a.start_review(git::Review::open(&dir, None, None).unwrap());
        // Panel order: src/a.rs, crlf.txt, gone, new, sub, tail.
        let r = a.review.as_ref().unwrap();
        assert_eq!(r.files[4].path, Path::new("sub"));
        assert!(!r.files[4].has_hunks());
        press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("tail"), 0));
        assert_eq!(a.message, "skipped 1 file without hunks");
        // `new` stops opening: `c` from `gone` goes past it to `tail`, and the status says
        // what happened to `new`.
        std::fs::remove_file(dir.join("new")).unwrap();
        std::fs::create_dir(dir.join("new")).unwrap();
        a.jump_to(&dir.join("gone"), 1);
        press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("tail"), 0));
        assert!(a.message.contains("new: "), "{}", a.message);
        // Edits that cannot be saved hold merl on the file; what the status says stays.
        a.jump_to(&dir.join("gone"), 1);
        (a.dirty, a.conflict) = (true, true);
        a.message = "save failed: x".into();
        a.hunk(1);
        assert_eq!(at(&a), (dir.join("gone"), 0));
        assert_eq!(a.message, "save failed: x");
        (a.dirty, a.conflict) = (false, false);
        // Nothing ahead opens: the reason again, not a retry that hides it.
        std::fs::remove_file(dir.join("tail")).unwrap();
        a.jump_to(&dir.join("gone"), 1);
        press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(at(&a), (dir.join("gone"), 0));
        assert!(a.message.contains("tail: "), "{}", a.message);
        let _ = std::fs::remove_dir_all(dir);
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

    /// A file merl cannot write is read-only from the moment it opens, not from the first failed
    /// save (#123). Every load asks again, so a `chmod` outside merl is picked up by Ctrl+R, and
    /// a reason that tells the reader more keeps its place.
    #[test]
    #[cfg(unix)]
    fn a_file_without_write_permission_is_read_only() {
        use std::os::unix::fs::PermissionsExt;
        let (dir, mut a) = project_app(
            "readonly",
            &[("src/a.rs", "fn x() {}\n"), (".env", "API_KEY=\n")],
        );
        let (env, other) = (dir.join(".env"), dir.join("src/a.rs"));
        let open = |a: &mut App, p: &Path| {
            a.jump_to(&other, 1);
            a.jump_to(p, 1);
        };
        let chmod = |p: &Path, mode| {
            std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode)).unwrap()
        };
        open(&mut a, &env);
        assert_eq!(a.buf.readonly, None);

        chmod(&env, 0o444);
        open(&mut a, &env);
        // As root every file is writable, and there is nothing to assert.
        if std::fs::OpenOptions::new().write(true).open(&env).is_ok() {
            std::fs::remove_dir_all(&dir).unwrap();
            return;
        }
        assert_eq!(a.buf.readonly, Some("no write permission"));

        // Where the file is says more than what it is missing: the standard library and a
        // dependency are unwritable on most machines, and `outside the project` is the reason
        // that explains them.
        let outside = std::env::temp_dir().join(format!("merl-ro-out-{}", std::process::id()));
        std::fs::write(&outside, "fn x() {}\n").unwrap();
        chmod(&outside, 0o444);
        open(&mut a, &outside);
        assert_eq!(a.buf.readonly, Some("outside the project"));
        chmod(&outside, 0o644);
        std::fs::remove_file(&outside).unwrap();

        // `merl .env`: the file named on the command line never reaches `open`.
        chmod(&env, 0o444);
        let started = App::new(
            dir.clone(),
            Tree::default(),
            Vec::new(),
            Buffer::load(&env).unwrap(),
            None,
        );
        assert_eq!(started.buf.readonly, Some("no write permission"));

        chmod(&env, 0o644);
        open(&mut a, &env);
        assert_eq!(a.buf.readonly, None);
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
        // The fixtures are read as a plain build reads them, whatever `GOFLAGS` the tests run under.
        a.go_build = search::GoBuild::host();
        for kind in [Kind::Python, Kind::TsJs, Kind::Go] {
            a.external.insert(kind, (Vec::new(), Arc::new(Vec::new())));
        }
        a
    }

    /// Presses `d` on the last word of the first line of `file` that contains `code`, or starts
    /// with it when `code` starts with `^`. A `|` in `code` is not part of the line: it stands
    /// after the word to press `d` on, and what follows it only picks the line.
    fn d_on(a: &mut App, file: &str, code: &str) {
        let (code, rest) = code.split_once('|').unwrap_or((code, ""));
        let whole = format!("{code}{rest}");
        let (code, whole) = (code, whole.as_str());
        let path = a.root.join(file);
        let text = std::fs::read_to_string(&path).unwrap();
        let (n, line) = text
            .lines()
            .enumerate()
            .find(|(_, l)| match whole.strip_prefix('^') {
                Some(start) => l.starts_with(start),
                None => l.contains(whole),
            })
            .unwrap_or_else(|| panic!("no `{whole}` in {file}"));
        let code = code.trim_start_matches('^');
        a.jump_to(&path, n + 1);
        a.col = line.find(whole.trim_start_matches('^')).unwrap()
            + code.rfind(|c: char| !is_word(c)).map_or(0, |i| i + 1);
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
            // The variable of a `range` over a channel, which the rules do not read.
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
            // the inner one hides it (#100).
            (
                "python",
                "service.py",
                "^    repo.delete_user",
                jump(
                    "delete_user \u{2192} AuditLog.delete_user (via repo: AuditLog)",
                    "repos.py:13",
                ),
            ),
            (
                "python",
                "service.py",
                "await repo.delete_user",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via repo: UserRepository)",
                    "repos.py:8",
                ),
            ),
            (
                "python",
                "factories.py",
                "repo.delete_user",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via make_repo() -> UserRepository)",
                    "repos.py:8",
                ),
            ),
            // No return type, but every `return` constructs one (#100).
            (
                "python",
                "factories.py",
                "audit.delete_user",
                jump(
                    "delete_user \u{2192} AuditLog.delete_user (via make_audit() returns AuditLog())",
                    "repos.py:13",
                ),
            ),
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
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via repo: UserRepository)",
                    "repos.ts:10",
                ),
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
            (
                "go",
                "service.go",
                "^\t\trepo.DeleteUser",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via repo: UserRepository)",
                    "repos.go:15",
                ),
            ),
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
            // Not the local `users`: the chain hangs off a call, whose declared return type
            // starts it (#100).
            (
                "python",
                "chains.py",
                "make_uow().users.delete_user",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via make_uow() -> UnitOfWork \u{2192} users: UserRepository)",
                    "repos.py:8",
                ),
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
            // A `@cached_property` with a return type is a field of that type; a `@property`
            // without one is not read, nor the typed one of the base class it overrides.
            (
                "python",
                "chains.py",
                "self.registry.users.delete_user",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via self.registry: Registry \u{2192} users: UserRepository)",
                    "repos.py:8",
                ),
            ),
            (
                "python",
                "chains.py",
                "self.registry.audit.delete_user",
                by_name(
                    "delete_user: by name, 2 declarations (chain broke at audit)",
                    &py_both,
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
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via makeUow(): UnitOfWork \u{2192} users: UserRepository)",
                    "repos.ts:10",
                ),
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
            (
                "typescript",
                "chains.ts",
                "this.registry.users.deleteUser",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via this.registry: Registry \u{2192} users: UserRepository)",
                    "repos.ts:10",
                ),
            ),
            (
                "typescript",
                "chains.ts",
                "this.registry.audit.deleteUser",
                by_name(
                    "deleteUser: by name, 2 declarations (chain broke at audit)",
                    &ts_both,
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
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via NewUnitOfWork() *UnitOfWork \u{2192} Users: UserRepository)",
                    "repos.go:15",
                ),
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

    /// #104 over the same project in three languages. A field is a target: behind a receiver whose
    /// type is proven, the type's declaration of the field, found through the classes it extends
    /// and the structs it embeds, is one jump that names the receiver; an assignment in a method
    /// stands for the field only when no type declares it, and then the base-most one does. On a
    /// value whose type is not known, each type's declaration of a field of that name is a
    /// candidate by name, so a common name is a picker; a local, a literal's key or a `var` block
    /// of that name is none. On the declaration itself the other fields of the name are its
    /// namesakes, but only on the name the line declares.
    #[test]
    fn a_field_is_a_target() {
        let namesakes = |word: &str, row: (&str, &str)| {
            picker(
                &format!("{word}: at a declaration, 1 other by name"),
                &[row],
            )
        };
        let cases: Vec<(&str, &str, &str, Shown)> = vec![
            // Annotated in the class body, with a value and without.
            (
                "python",
                "fields.py",
                "issue.poster_id",
                jump(
                    "poster_id \u{2192} Issue.poster_id (via issue: Issue)",
                    "fields.py:16",
                ),
            ),
            (
                "python",
                "fields.py",
                "issue.title",
                jump(
                    "title \u{2192} Issue.title (via issue: Issue)",
                    "fields.py:15",
                ),
            ),
            // Assigned in the `__init__` of the base class, and again in a method of `Issue`.
            (
                "python",
                "fields.py",
                "issue.audit",
                jump(
                    "audit \u{2192} Base.audit (via issue: Issue)",
                    "fields.py:8",
                ),
            ),
            // A field of `Issue` before a method of `Base`.
            (
                "python",
                "fields.py",
                "issue.summary",
                jump(
                    "summary \u{2192} Issue.summary (via issue: Issue)",
                    "fields.py:17",
                ),
            ),
            (
                "python",
                "fields.py",
                "await self.repo",
                jump("repo \u{2192} Issue.repo (via self: Issue)", "fields.py:21"),
            ),
            // A later assignment is not the declaration.
            (
                "python",
                "fields.py",
                "^            self.labels",
                jump(
                    "labels \u{2192} Issue.labels (via self: Issue)",
                    "fields.py:22",
                ),
            ),
            (
                "python",
                "chains.py",
                "self.uow.users",
                jump(
                    "users \u{2192} UnitOfWork.users (via self.uow: UnitOfWork)",
                    "chains.py:10",
                ),
            ),
            (
                "python",
                "fields.py",
                "comment.poster_id",
                picker(
                    "poster_id: by name, 2 declarations",
                    &[
                        ("Issue.poster_id", "fields.py:16"),
                        ("Comment.poster_id", "fields.py:40"),
                    ],
                ),
            ),
            // `body : str` in the docstring declares nothing.
            (
                "python",
                "fields.py",
                "comment.body",
                jump(
                    "body \u{2192} Comment.body (by name, 1 match)",
                    "fields.py:41",
                ),
            ),
            // `total: int = 0` is a local.
            (
                "python",
                "fields.py",
                "comment.total",
                jump("no definition for total", "fields.py:52"),
            ),
            // The first binding of `Point.offset` is a tuple target.
            (
                "python",
                "fields.py",
                "p.offset",
                picker(
                    "offset: by name, 2 declarations",
                    &[
                        ("Point.offset", "fields.py:68"),
                        ("Cursor.offset", "fields.py:76"),
                    ],
                ),
            ),
            (
                "python",
                "fields.py",
                "^    poster_id",
                namesakes("poster_id", ("Comment.poster_id", "fields.py:40")),
            ),
            (
                "python",
                "fields.py",
                "^        self.poster_id",
                namesakes("poster_id", ("Issue.poster_id", "fields.py:16")),
            ),
            // The parameter handed on, not the field it is handed to, and named so (#100).
            (
                "python",
                "fields.py",
                "self.repo = repo",
                jump("repo \u{2192} Issue.__init__.repo (local)", "fields.py:19"),
            ),
            // A class whose base is not the project's: its declarations by name, fields
            // included, and a nested class among them.
            (
                "python",
                "fields.py",
                "if self.poster_id",
                picker(
                    "poster_id: by name, 2 declarations",
                    &[
                        ("Issue.poster_id", "fields.py:16"),
                        ("Comment.poster_id", "fields.py:40"),
                    ],
                ),
            ),
            (
                "python",
                "fields.py",
                "self.Options",
                jump(
                    "Options \u{2192} Encoder.Options (by name, 1 match)",
                    "fields.py:57",
                ),
            ),
            (
                "typescript",
                "fields.ts",
                "issue.posterId",
                jump(
                    "posterId \u{2192} Issue.posterId (via issue: Issue)",
                    "fields.ts:9",
                ),
            ),
            (
                "typescript",
                "fields.ts",
                "issue.title",
                jump(
                    "title \u{2192} Issue.title (via issue: Issue)",
                    "fields.ts:8",
                ),
            ),
            // `Issue.close` assigns it again.
            (
                "typescript",
                "fields.ts",
                "issue.audit",
                jump(
                    "audit \u{2192} Base.audit (via issue: Issue)",
                    "fields.ts:4",
                ),
            ),
            // A parameter of a constructor wrapped over several lines.
            (
                "typescript",
                "fields.ts",
                "this.repo",
                jump("repo \u{2192} Issue.repo (via this: Issue)", "fields.ts:13"),
            ),
            (
                "typescript",
                "fields.ts",
                "this.title",
                jump(
                    "title \u{2192} Issue.title (via this: Issue)",
                    "fields.ts:8",
                ),
            ),
            // Both sides of `this.close = this.close.bind(this)` are the method.
            (
                "typescript",
                "fields.ts",
                "this.close",
                jump(
                    "close \u{2192} Issue.close (via this: Issue)",
                    "fields.ts:21",
                ),
            ),
            (
                "typescript",
                "fields.ts",
                "= this.close",
                jump(
                    "close \u{2192} Issue.close (via this: Issue)",
                    "fields.ts:21",
                ),
            ),
            // A `case` block is no object literal: `this` is still the class.
            (
                "typescript",
                "fields.ts",
                "String(this.posterId",
                jump(
                    "posterId \u{2192} Issue.posterId (via this: Issue)",
                    "fields.ts:9",
                ),
            ),
            (
                "typescript",
                "fields.ts",
                "return this.title",
                jump(
                    "title \u{2192} Issue.title (via this: Issue)",
                    "fields.ts:8",
                ),
            ),
            // But the literal a `case` returns is: its `this` is not the class.
            (
                "typescript",
                "fields.ts",
                "`${this.posterId",
                picker(
                    "posterId: by name, 2 declarations",
                    &[
                        ("Issue.posterId", "fields.ts:9"),
                        ("Comment.posterId", "fields.ts:47"),
                    ],
                ),
            ),
            (
                "typescript",
                "chains.ts",
                "this.uow.users",
                jump(
                    "users \u{2192} UnitOfWork.users (via this.uow: UnitOfWork)",
                    "chains.ts:4",
                ),
            ),
            (
                "typescript",
                "fields.ts",
                "comment.posterId",
                picker(
                    "posterId: by name, 2 declarations",
                    &[
                        ("Issue.posterId", "fields.ts:9"),
                        ("Comment.posterId", "fields.ts:47"),
                    ],
                ),
            ),
            (
                "typescript",
                "fields.ts",
                "comment.body",
                jump(
                    "body \u{2192} Comment.body (by name, 1 match)",
                    "fields.ts:48",
                ),
            ),
            // `let total` is a local, and `total: 0` the key of an object literal.
            (
                "typescript",
                "fields.ts",
                "comment.total",
                jump("no definition for total", "fields.ts:62"),
            ),
            (
                "typescript",
                "fields.ts",
                "^  posterId",
                namesakes("posterId", ("Comment.posterId", "fields.ts:47")),
            ),
            (
                "go",
                "fields.go",
                "issue.PosterID",
                jump(
                    "PosterID \u{2192} Issue.PosterID (via issue: Issue)",
                    "fields.go:14",
                ),
            ),
            // The second name of `Title, Body string`.
            (
                "go",
                "fields.go",
                "issue.Body",
                jump(
                    "Body \u{2192} Issue.Body (via issue: Issue)",
                    "fields.go:15",
                ),
            ),
            // Promoted from the embedded `Base`, and the embedded struct by its name.
            (
                "go",
                "fields.go",
                "issue.Audit",
                jump(
                    "Audit \u{2192} Base.Audit (via issue: Issue)",
                    "fields.go:9",
                ),
            ),
            (
                "go",
                "fields.go",
                "issue.Base",
                jump(
                    "Base \u{2192} Issue.Base (via issue: Issue)",
                    "fields.go:13",
                ),
            ),
            (
                "go",
                "fields.go",
                "i.repo",
                jump("repo \u{2192} Issue.repo (via i: Issue)", "fields.go:16"),
            ),
            (
                "go",
                "chains.go",
                "h.uow",
                jump("uow \u{2192} Deps.uow (via h: Handler)", "chains.go:25"),
            ),
            (
                "go",
                "chains.go",
                "h.uow.Users",
                jump(
                    "Users \u{2192} UnitOfWork.Users (via h.uow: UnitOfWork)",
                    "chains.go:4",
                ),
            ),
            // The variable of a `range` over a channel, which the rules do not read.
            (
                "go",
                "fields.go",
                "c.PosterID",
                picker(
                    "PosterID: by name, 2 declarations",
                    &[
                        ("Issue.PosterID", "fields.go:14"),
                        ("Comment.PosterID", "fields.go:20"),
                    ],
                ),
            ),
            (
                "go",
                "fields.go",
                "c.Text",
                jump(
                    "Text \u{2192} Comment.Text (by name, 1 match)",
                    "fields.go:21",
                ),
            ),
            // `Host string` in a `var` block is a local.
            (
                "go",
                "fields.go",
                "r.Host",
                jump("no definition for Host", "fields.go:40"),
            ),
            // A field named like its type: on the type, `d` goes to the type.
            (
                "go",
                "fields.go",
                "^\tIssue    *Issue",
                jump("Issue: by name, 1 match", "fields.go:12"),
            ),
            // An embedded struct's name is its type's: `d` there goes to the type.
            (
                "go",
                "fields.go",
                "^\tBase",
                jump("Base: by name, 1 match", "fields.go:8"),
            ),
            (
                "go",
                "fields.go",
                "^\tPosterID",
                namesakes("PosterID", ("Comment.PosterID", "fields.go:20")),
            ),
        ];
        for (fixture, file, code, want) in cases {
            let mut a = fixture_app(fixture);
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{fixture}: {file}: {code}");
        }
    }

    /// #100: `super().m()` / `super.m()` is `self` / `this` with the walk started one level up, so
    /// an override leads to what it overrides and never to itself. Go has no `super`: its
    /// `i.Base.M()` is a chain through the embedded struct. Under several Python bases only what
    /// needs no method resolution order is proven.
    #[test]
    fn super_starts_one_level_up() {
        let cases: Vec<(&str, &str, &str, Shown)> = vec![
            // One base: the nearest declaration above the class, a method or a declared field.
            (
                "python",
                "supers.py",
                "super().__init__",
                jump(
                    "__init__ \u{2192} Archive.__init__ (via super of ColdArchive)",
                    "supers.py:12",
                ),
            ),
            (
                "python",
                "supers.py",
                "super().store|(item)",
                jump(
                    "store \u{2192} Archive.store (via super of ColdArchive)",
                    "supers.py:15",
                ),
            ),
            // An override leads to what it overrides, never to itself; `Generic[T]` declares nothing.
            (
                "python",
                "supers.py",
                "super().store|(item + 1)",
                jump(
                    "store \u{2192} ColdArchive.store (via super of GlacierArchive)",
                    "supers.py:26",
                ),
            ),
            (
                "python",
                "supers.py",
                "super().flush|()",
                jump(
                    "flush \u{2192} Archive.flush (via super of GlacierArchive)",
                    "supers.py:18",
                ),
            ),
            (
                "python",
                "supers.py",
                "super().label",
                jump(
                    "label \u{2192} Archive.label (via super of GlacierArchive)",
                    "supers.py:10",
                ),
            ),
            // A function inside the method: `super()` has no arguments to find there.
            (
                "python",
                "supers.py",
                "super().flush|(), \"no arg",
                picker(
                    "flush: by name, 4 declarations",
                    &[
                        ("Archive.flush", "supers.py:18"),
                        ("GlacierArchive.flush", "supers.py:36"),
                        ("Right.flush", "supers.py:63"),
                        ("Diamond.flush", "supers.py:68"),
                    ],
                ),
            ),
            // Several bases: the first one declaring the member itself is first in any order.
            (
                "python",
                "supers.py",
                "super().store|(item + 2)",
                jump(
                    "store \u{2192} Stamped.store (via super of Mixed)",
                    "supers.py:44",
                ),
            ),
            (
                "python",
                "supers.py",
                "super().stamp",
                jump(
                    "stamp \u{2192} Stamped.stamp (via super of Mixed)",
                    "supers.py:47",
                ),
            ),
            // Only one of the bases leads to a `flush`.
            (
                "python",
                "supers.py",
                "super().flush|(), \"one base",
                jump(
                    "flush \u{2192} Archive.flush (via super of Mixed)",
                    "supers.py:18",
                ),
            ),
            // `Left` leads to `Archive.flush`, `Right` declares its own, and Python asks `Right` first: a
            // walk by depth would jump to the wrong one, so none is proven.
            (
                "python",
                "supers.py",
                "super().flush|(), \"Right",
                picker(
                    "flush: by name, 4 declarations",
                    &[
                        ("Archive.flush", "supers.py:18"),
                        ("GlacierArchive.flush", "supers.py:36"),
                        ("Right.flush", "supers.py:63"),
                        ("Diamond.flush", "supers.py:68"),
                    ],
                ),
            ),
            // `json.JSONEncoder` is outside the project and may declare the member first.
            (
                "python",
                "supers.py",
                "super().store|(item + 3)",
                picker(
                    "store: by name, 6 declarations",
                    &[
                        ("Archive.store", "supers.py:15"),
                        ("ColdArchive.store", "supers.py:26"),
                        ("GlacierArchive.store", "supers.py:31"),
                        ("Stamped.store", "supers.py:44"),
                        ("Mixed.store", "supers.py:52"),
                        ("Wire.store", "supers.py:73"),
                    ],
                ),
            ),
            (
                "python",
                "supers.py",
                "super().default",
                picker(
                    "default: by name, 2 declarations",
                    &[
                        ("Wire.default", "supers.py:76"),
                        ("Encoder.default", "fields.py:60"),
                    ],
                ),
            ),
            // A name between `super()` and the word is not followed: the word is not looked for
            // above the class as if it stood behind `super()` itself.
            (
                "python",
                "supers.py",
                "super().audit.store",
                picker(
                    "store: by name, 6 declarations (chain broke at audit)",
                    &[
                        ("Archive.store", "supers.py:15"),
                        ("ColdArchive.store", "supers.py:26"),
                        ("GlacierArchive.store", "supers.py:31"),
                        ("Stamped.store", "supers.py:44"),
                        ("Mixed.store", "supers.py:52"),
                        ("Wire.store", "supers.py:73"),
                    ],
                ),
            ),
            // TypeScript: one `extends`.
            (
                "typescript",
                "supers.ts",
                "super.store|(item);",
                jump(
                    "store \u{2192} Archive.store (via super of ColdArchive)",
                    "supers.ts:8",
                ),
            ),
            (
                "typescript",
                "supers.ts",
                "super.store|(item + 1)",
                jump(
                    "store \u{2192} ColdArchive.store (via super of GlacierArchive)",
                    "supers.ts:18",
                ),
            ),
            (
                "typescript",
                "supers.ts",
                "super.flush",
                jump(
                    "flush \u{2192} Archive.flush (via super of GlacierArchive)",
                    "supers.ts:10",
                ),
            ),
            // An arrow function passes `super` through as it does `this`.
            (
                "typescript",
                "supers.ts",
                "super.store|(n)",
                jump(
                    "store \u{2192} ColdArchive.store (via super of GlacierArchive)",
                    "supers.ts:18",
                ),
            ),
            // In an object literal `super` is the literal's prototype, not the class around it.
            (
                "typescript",
                "supers.ts",
                "super.toString",
                picker(
                    "toString: by name, 2 declarations",
                    &[
                        ("Archive.toString", "supers.ts:12"),
                        ("toString", "supers.ts:32"),
                    ],
                ),
            ),
            // The same conditions hold at every level the answer is found through: a diamond
            // or an outside base under the one direct base proves nothing either.
            (
                "python",
                "supers.py",
                "super().drain|(), \"a diamond",
                picker(
                    "drain: by name, 6 declarations",
                    &[
                        ("Tank.drain", "supers.py:89"),
                        ("LoudTank.drain", "supers.py:102"),
                        ("LeafTank.drain", "supers.py:111"),
                        ("LeafWire.drain", "supers.py:120"),
                        ("SameTank.drain", "supers.py:125"),
                        ("AbcTank.drain", "supers.py:130"),
                    ],
                ),
            ),
            (
                "python",
                "supers.py",
                "super().drain|(), \"an outside",
                picker(
                    "drain: by name, 6 declarations",
                    &[
                        ("Tank.drain", "supers.py:89"),
                        ("LoudTank.drain", "supers.py:102"),
                        ("LeafTank.drain", "supers.py:111"),
                        ("LeafWire.drain", "supers.py:120"),
                        ("SameTank.drain", "supers.py:125"),
                        ("AbcTank.drain", "supers.py:130"),
                    ],
                ),
            ),
            // Two bases that lead to the same declaration, and `ABC`, which declares nothing.
            (
                "python",
                "supers.py",
                "super().drain|(), \"both",
                jump(
                    "drain \u{2192} Tank.drain (via super of SameTank)",
                    "supers.py:89",
                ),
            ),
            (
                "python",
                "supers.py",
                "super().drain|(), \"ABC",
                jump(
                    "drain \u{2192} Tank.drain (via super of AbcTank)",
                    "supers.py:89",
                ),
            ),
            // `d` on the word itself is what it was: `super` is no local.
            (
                "python",
                "supers.py",
                "super|().stamp",
                jump("no definition for super", "supers.py:54"),
            ),
            (
                "typescript",
                "supers.ts",
                "super|.flush",
                jump("no definition for super", "supers.ts:26"),
            ),
            // `super.open()` returns what the base's `open` declares, not the override's
            // narrower type: a call on `super` is not read as a call on `this`.
            (
                "typescript",
                "supers.ts",
                "opened.store",
                picker(
                    "store: by name, 3 declarations",
                    &[
                        ("Archive.store", "supers.ts:8"),
                        ("ColdArchive.store", "supers.ts:18"),
                        ("GlacierArchive.store", "supers.ts:24"),
                    ],
                ),
            ),
            (
                "typescript",
                "supers.ts",
                "super.open().store",
                picker(
                    "store: by name, 3 declarations",
                    &[
                        ("Archive.store", "supers.ts:8"),
                        ("ColdArchive.store", "supers.ts:18"),
                        ("GlacierArchive.store", "supers.ts:24"),
                    ],
                ),
            ),
        ];
        for (fixture, file, code, want) in cases {
            let mut a = fixture_app(fixture);
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{fixture}: {file}: {code}");
        }
    }

    /// #100: the innermost scope that binds a name decides what it is. A module-level or
    /// package-level name, a variable of the function around a closure and one of the block
    /// around a block are hidden, where they used to disagree with the inner one. What hides may
    /// be unknown, and then nothing behind it answers; two bindings of one scope still disagree.
    #[test]
    fn the_innermost_binding_wins() {
        let cases: Vec<(&str, &str, &str, Shown)> = vec![
            // A parameter named like a module-level `def`.
            (
                "python",
                "scopes.py",
                "save.find_user",
                jump(
                    "find_user \u{2192} UserRepository.find_user (via save: UserRepository)",
                    "repos.py:5",
                ),
            ),
            // A local hides the module's variable, in the function and in a closure inside it.
            (
                "python",
                "scopes.py",
                "ledger.find_user|(user_id)",
                jump(
                    "find_user \u{2192} UserRepository.find_user (via ledger: UserRepository)",
                    "repos.py:5",
                ),
            ),
            (
                "python",
                "scopes.py",
                "ledger.find_user|(user_id + 1)",
                jump(
                    "find_user \u{2192} UserRepository.find_user (via ledger: UserRepository)",
                    "repos.py:5",
                ),
            ),
            // A function that binds no `ledger` reads the module's.
            (
                "python",
                "scopes.py",
                "ledger.delete_user|(user_id + 2)",
                jump(
                    "delete_user \u{2192} AuditLog.delete_user (via ledger: AuditLog)",
                    "repos.py:13",
                ),
            ),
            // Two bindings in one function disagree: which one reaches the line is control flow.
            (
                "python",
                "scopes.py",
                "ledger.delete_user|(3)",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            // A parameter with no annotation hides the module's `ledger`, which must not answer.
            (
                "python",
                "scopes.py",
                "ledger.delete_user|(user_id + 4)",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            // On the name itself, the declaration in its scope.
            (
                "python",
                "scopes.py",
                "    ledger|.find_user(user_id)",
                jump("ledger \u{2192} rotate.ledger (local)", "scopes.py:15"),
            ),
            // TypeScript: a `const` of the function, an arrow function's parameter, a block's `const`
            // over the function's, and the function's own below that block.
            (
                "typescript",
                "scopes.ts",
                "ledger.findUser",
                jump(
                    "findUser \u{2192} UserRepository.findUser (via ledger: UserRepository)",
                    "repos.ts:6",
                ),
            ),
            (
                "typescript",
                "scopes.ts",
                "ledger.deleteUser|(id);",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via ledger: UserRepository)",
                    "repos.ts:10",
                ),
            ),
            (
                "typescript",
                "scopes.ts",
                "ledger.deleteUser|(id + 1)",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via ledger: UserRepository)",
                    "repos.ts:10",
                ),
            ),
            (
                "typescript",
                "scopes.ts",
                "ledger.deleteUser|(id + 2)",
                jump(
                    "deleteUser \u{2192} AuditLog.deleteUser (via ledger: AuditLog)",
                    "repos.ts:16",
                ),
            ),
            // `any` hides the module's `ledger`.
            (
                "typescript",
                "scopes.ts",
                "ledger.deleteUser|(id + 3)",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            // The cursor is not inside an arrow function on its own line, or on the line the
            // statement started on: its parameter counts and hides nothing.
            (
                "typescript",
                "scopes.ts",
                "ledger.deleteUser|(repos.map",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            (
                "typescript",
                "scopes.ts",
                "ledger.deleteUser|(id + 4)",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            (
                "typescript",
                "scopes.ts",
                "  ledger|.findUser(id)",
                jump("ledger \u{2192} rotate.ledger (local)", "scopes.ts:6"),
            ),
            // A literal under a line that also holds an arrow function: the cursor is in the
            // literal, and the arrow's parameter hides nothing.
            (
                "typescript",
                "scopes.ts",
                "ledger.deleteUser|(id + 5)",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            // Go: a `:=` over the package's `var`, a block's over the function's, and the package's
            // where nothing hides it.
            (
                "go",
                "scopes.go",
                "ledger.FindUser",
                jump(
                    "FindUser \u{2192} UserRepository.FindUser (via NewRepo() *UserRepository)",
                    "repos.go:11",
                ),
            ),
            (
                "go",
                "scopes.go",
                "ledger.DeleteUser|(id + 1)",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via NewRepo() *UserRepository)",
                    "repos.go:15",
                ),
            ),
            (
                "go",
                "scopes.go",
                "ledger.DeleteUser|(id + 2)",
                jump(
                    "DeleteUser \u{2192} AuditLog.DeleteUser (via ledger: AuditLog)",
                    "repos.go:21",
                ),
            ),
            (
                "go",
                "scopes.go",
                "ledger.DeleteUser|(id + 3)",
                jump(
                    "DeleteUser \u{2192} AuditLog.DeleteUser (via ledger: AuditLog)",
                    "repos.go:21",
                ),
            ),
            // The second name of a `:=` is unknown and hides the package's.
            (
                "go",
                "scopes.go",
                "ledger.DeleteUser|(id + 4)",
                picker(
                    "DeleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.DeleteUser", "repos.go:15"),
                        ("AuditLog.DeleteUser", "repos.go:21"),
                    ],
                ),
            ),
            // An `if` header and its body are read as one scope, so their two `ledger` disagree.
            (
                "go",
                "scopes.go",
                "ledger.DeleteUser|(id + 5)",
                picker(
                    "DeleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.DeleteUser", "repos.go:15"),
                        ("AuditLog.DeleteUser", "repos.go:21"),
                    ],
                ),
            ),
            (
                "go",
                "scopes.go",
                "	ledger|.FindUser(id)",
                jump(
                    "ledger \u{2192} ScopedRotate.ledger (local)",
                    "scopes.go:10",
                ),
            ),
            // A composite literal under a line that also holds a `func`.
            (
                "go",
                "scopes.go",
                "ledger.DeleteUser|(id + 6)",
                picker(
                    "DeleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.DeleteUser", "repos.go:15"),
                        ("AuditLog.DeleteUser", "repos.go:21"),
                    ],
                ),
            ),
            // A sibling block's callback or loop is not around the cursor of the `else` or the
            // `catch`, which reads the parameter of its own function. A callback on the lines
            // of the header itself counts, as one on the cursor's line does, and hides nothing.
            (
                "go",
                "scopes.go",
                "ledger.DeleteUser|(7)",
                jump(
                    "DeleteUser \u{2192} AuditLog.DeleteUser (via ledger: AuditLog)",
                    "repos.go:21",
                ),
            ),
            (
                "go",
                "scopes.go",
                "ledger.DeleteUser|(8)",
                jump(
                    "DeleteUser \u{2192} AuditLog.DeleteUser (via ledger: AuditLog)",
                    "repos.go:21",
                ),
            ),
            (
                "go",
                "scopes.go",
                "ledger.DeleteUser|(9)",
                picker(
                    "DeleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.DeleteUser", "repos.go:15"),
                        ("AuditLog.DeleteUser", "repos.go:21"),
                    ],
                ),
            ),
            (
                "typescript",
                "scopes.ts",
                "ledger.deleteUser|(id + 6)",
                jump(
                    "deleteUser \u{2192} AuditLog.deleteUser (via ledger: AuditLog)",
                    "repos.ts:16",
                ),
            ),
            (
                "typescript",
                "scopes.ts",
                "ledger.deleteUser|(id + 7)",
                jump(
                    "deleteUser \u{2192} AuditLog.deleteUser (via ledger: AuditLog)",
                    "repos.ts:16",
                ),
            ),
            (
                "typescript",
                "scopes.ts",
                "ledger.deleteUser|(id + 8)",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            (
                "typescript",
                "scopes.ts",
                "ledger.deleteUser|(id + 9",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            // A docstring's example binds nothing: the module's `ledger` is read.
            (
                "python",
                "scopes.py",
                "ledger.delete_user|(user_id + 5)",
                jump(
                    "delete_user \u{2192} AuditLog.delete_user (via ledger: AuditLog)",
                    "repos.py:13",
                ),
            ),
            // The arrow function's line ends in `=>`: its body is the lines below.
            (
                "typescript",
                "scopes.ts",
                "ledger.deleteUser|(id + 10)",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via ledger: UserRepository)",
                    "repos.ts:10",
                ),
            ),
            // A destructured parameter under a header closed by `}: Deps): void {` hides the
            // module's `ledger` and has no type of its own.
            (
                "typescript",
                "scopes.ts",
                "ledger.deleteUser|(id + 11)",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            // A backtick inside a regex opens no template: the `const` under it is read.
            (
                "typescript",
                "scopes.ts",
                "ledger.deleteUser|(id + 12)",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via ledger: UserRepository)",
                    "repos.ts:10",
                ),
            ),
        ];
        for (fixture, file, code, want) in cases {
            let mut a = fixture_app(fixture);
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{fixture}: {file}: {code}");
        }
    }

    /// #100: a loop variable is an element of what it loops over, where that collection's type is
    /// written: `list[T]`, `T[]`, `[]T`, `map[K]T`, as an annotation or as the return type of the
    /// function it came from. The collection itself is no `T`, and keys, pairs, a tuple target
    /// and a collection declared twice stay unknown.
    #[test]
    fn a_loop_variable_is_an_element_of_a_written_collection() {
        let cases: Vec<(&str, &str, &str, Shown)> = vec![
            // An annotated parameter: `list[T]`, and `tuple[T, ...]` in quotes.
            (
                "python",
                "elements.py",
                "repo.delete_user|(user_id)",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via repos: list[UserRepository])",
                    "repos.py:8",
                ),
            ),
            (
                "python",
                "elements.py",
                "log.delete_user",
                jump(
                    "delete_user \u{2192} AuditLog.delete_user (via logs: \"tuple[AuditLog, ...]\")",
                    "repos.py:13",
                ),
            ),
            // The collection came from a function that declares what it returns.
            (
                "python",
                "elements.py",
                "repo.delete_user|(user_id + 3)",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via load_repos() -> list[UserRepository])",
                    "repos.py:8",
                ),
            ),
            // The list itself is no `UserRepository`.
            (
                "python",
                "elements.py",
                "repos.delete_user",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            // A `dict` hands out its keys; a tuple target over a call is not read.
            (
                "python",
                "elements.py",
                "key.delete_user",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            (
                "python",
                "elements.py",
                "repo.delete_user|(user_id + 5)",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            // `repos` is assigned again with no type written: not every declaration says what it holds.
            (
                "python",
                "elements.py",
                "repo.delete_user|(user_id + 6)",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            // Two annotations that disagree.
            (
                "python",
                "elements.py",
                "repo.delete_user|(user_id + 7)",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            // TypeScript: `T[]`, `ReadonlyArray<T>`, a declared return type.
            (
                "typescript",
                "elements.ts",
                "repo.deleteUser|(id);",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via repos: UserRepository[])",
                    "repos.ts:10",
                ),
            ),
            (
                "typescript",
                "elements.ts",
                "log.deleteUser",
                jump(
                    "deleteUser \u{2192} AuditLog.deleteUser (via logs: ReadonlyArray<AuditLog>)",
                    "repos.ts:16",
                ),
            ),
            (
                "typescript",
                "elements.ts",
                "repo.deleteUser|(id + 2)",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via loadRepos(): UserRepository[])",
                    "repos.ts:10",
                ),
            ),
            // The array itself, a `Map`'s pairs and the keys of `for … in`.
            (
                "typescript",
                "elements.ts",
                "repos.deleteUser",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            (
                "typescript",
                "elements.ts",
                "entry.deleteUser",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            (
                "typescript",
                "elements.ts",
                "index.deleteUser",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            // Go: the second variable of a `range` over `[]*T`, over `map[K]T`, over a declared result.
            (
                "go",
                "elements.go",
                "repo.DeleteUser|(id)",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via repos: []*UserRepository)",
                    "repos.go:15",
                ),
            ),
            (
                "go",
                "elements.go",
                "entry.DeleteUser",
                jump(
                    "DeleteUser \u{2192} AuditLog.DeleteUser (via logs: map[string]AuditLog)",
                    "repos.go:21",
                ),
            ),
            (
                "go",
                "elements.go",
                "repo.DeleteUser|(id + 2)",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via LoadRepos() []*UserRepository)",
                    "repos.go:15",
                ),
            ),
            // A single variable is an index or a key, or the element of a channel the rules do not read.
            (
                "go",
                "elements.go",
                "repo.DeleteUser|(id + 3)",
                picker(
                    "DeleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.DeleteUser", "repos.go:15"),
                        ("AuditLog.DeleteUser", "repos.go:21"),
                    ],
                ),
            ),
            // A slice made or written out in place says what it holds.
            (
                "go",
                "elements.go",
                "repo.DeleteUser|(id + 4)",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via made: []*UserRepository)",
                    "repos.go:15",
                ),
            ),
            (
                "go",
                "elements.go",
                "entry.DeleteUser|(id + 5)",
                jump(
                    "DeleteUser \u{2192} AuditLog.DeleteUser (via listed: []AuditLog)",
                    "repos.go:21",
                ),
            ),
            // A named slice type, read where it is declared.
            (
                "go",
                "elements.go",
                "repo.DeleteUser|(id + 6)",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via repos: RepoList)",
                    "repos.go:15",
                ),
            ),
            // An element handed on to another name is one hop too many.
            (
                "python",
                "elements.py",
                "current.delete_user",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            // A named map, a named slice declared in another package, and a `type X = []T`
            // alias, which is not read.
            (
                "go",
                "elements.go",
                "repo.DeleteUser|(id + 7)",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via index: RepoIndex)",
                    "repos.go:15",
                ),
            ),
            (
                "go",
                "elements.go",
                "session.Close",
                jump(
                    "Close \u{2192} Session.Close (via sessions: store.SessionList)",
                    "store/store.go:15",
                ),
            ),
            (
                "go",
                "elements.go",
                "repo.DeleteUser|(id + 8)",
                picker(
                    "DeleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.DeleteUser", "repos.go:15"),
                        ("AuditLog.DeleteUser", "repos.go:21"),
                    ],
                ),
            ),
        ];
        for (fixture, file, code, want) in cases {
            let mut a = fixture_app(fixture);
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{fixture}: {file}: {code}");
        }
    }

    /// #100: one more hop on a call. A method called on a receiver whose type is proven gives
    /// what it declares to return, a Python function or method with no annotation what every
    /// `return` of it constructs, and a chain may hang off a call that starts the expression.
    /// A call of a call, returns that differ and an unproven receiver stay by name.
    #[test]
    fn a_call_is_one_more_hop() {
        let cases: Vec<(&str, &str, &str, Shown)> = vec![
            // A method of a parameter whose type is written, with the return type it declares.
            (
                "python",
                "calls.py",
                "repo.delete_user|(user_id)",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via depot.people_repo() -> UserRepository)",
                    "repos.py:8",
                ),
            ),
            // No annotation, and every `return` constructs an `AuditLog`.
            (
                "python",
                "calls.py",
                "trail.delete_user",
                jump(
                    "delete_user \u{2192} AuditLog.delete_user (via depot.trail() returns AuditLog())",
                    "repos.py:13",
                ),
            ),
            // A chain off a call that starts the expression: a function, a class, and the field itself.
            (
                "python",
                "calls.py",
                "open_depot().people.delete_user",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via open_depot() -> Depot \u{2192} people: UserRepository)",
                    "repos.py:8",
                ),
            ),
            (
                "python",
                "calls.py",
                "Depot().people.delete_user",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via Depot(): Depot \u{2192} people: UserRepository)",
                    "repos.py:8",
                ),
            ),
            (
                "python",
                "calls.py",
                "open_depot().people|.delete_user",
                jump(
                    "people \u{2192} Depot.people (via open_depot() -> Depot)",
                    "calls.py:8",
                ),
            ),
            // The returns differ; one of them is `None`; a decorator may return anything.
            (
                "python",
                "calls.py",
                "either.delete_user",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            (
                "python",
                "calls.py",
                "maybe.delete_user",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            (
                "python",
                "calls.py",
                "shared.delete_user",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            // A call of a call hangs off a value nobody typed.
            (
                "python",
                "calls.py",
                "open_depot().people_repo().delete_user",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            // The receiver of the method is a parameter with no annotation.
            (
                "python",
                "calls.py",
                "repo.delete_user|(user_id + 8)",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            // TypeScript: a method's `): T`, a method whose every `return` is `new T()`, a chain off a
            // function and off `new`, and the method line of an interface.
            (
                "typescript",
                "calls.ts",
                "repo.deleteUser|(id);",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via depot.peopleRepo(): UserRepository)",
                    "repos.ts:10",
                ),
            ),
            (
                "typescript",
                "calls.ts",
                "trail.deleteUser",
                jump(
                    "deleteUser \u{2192} AuditLog.deleteUser (via depot.trail() returns new AuditLog())",
                    "repos.ts:16",
                ),
            ),
            (
                "typescript",
                "calls.ts",
                "openDepot().people.deleteUser",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via openDepot(): Depot \u{2192} people: UserRepository)",
                    "repos.ts:10",
                ),
            ),
            (
                "typescript",
                "calls.ts",
                "new Depot().people.deleteUser",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via new Depot(): Depot \u{2192} people: UserRepository)",
                    "repos.ts:10",
                ),
            ),
            (
                "typescript",
                "calls.ts",
                "sourced.deleteUser",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via src.source(): UserRepository)",
                    "repos.ts:10",
                ),
            ),
            // Overloads declare the method three times: which one is called is not read.
            (
                "typescript",
                "calls.ts",
                "picked.deleteUser",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            // A call of a call, and a receiver typed `any`.
            (
                "typescript",
                "calls.ts",
                "openDepot().peopleRepo().deleteUser",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            (
                "typescript",
                "calls.ts",
                "repo.deleteUser|(id + 6)",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            // Go: a method behind its receiver, the first of two results, a chain off a function and
            // off a package's, and the method line of an interface.
            (
                "go",
                "calls.go",
                "repo.DeleteUser|(id)",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via depot.PeopleRepo() *UserRepository)",
                    "repos.go:15",
                ),
            ),
            (
                "go",
                "calls.go",
                "trail.DeleteUser",
                jump(
                    "DeleteUser \u{2192} AuditLog.DeleteUser (via depot.Trail() AuditLog)",
                    "repos.go:21",
                ),
            ),
            (
                "go",
                "calls.go",
                "OpenDepot().People.DeleteUser",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via OpenDepot() *Depot \u{2192} People: UserRepository)",
                    "repos.go:15",
                ),
            ),
            (
                "go",
                "calls.go",
                "store.Open().Close",
                jump(
                    "Close \u{2192} Session.Close (via store.Open() *Session)",
                    "store/store.go:15",
                ),
            ),
            (
                "go",
                "calls.go",
                "sourced.DeleteUser",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via src.Source() *UserRepository)",
                    "repos.go:15",
                ),
            ),
            // A call of a call, and a receiver that is the second name of a `:=`.
            (
                "go",
                "calls.go",
                "OpenDepot().PeopleRepo().DeleteUser",
                picker(
                    "DeleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.DeleteUser", "repos.go:15"),
                        ("AuditLog.DeleteUser", "repos.go:21"),
                    ],
                ),
            ),
            (
                "go",
                "calls.go",
                "repo.DeleteUser|(id + 5)",
                picker(
                    "DeleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.DeleteUser", "repos.go:15"),
                        ("AuditLog.DeleteUser", "repos.go:21"),
                    ],
                ),
            ),
            // A decorator written over several lines may return anything too.
            (
                "python",
                "calls.py",
                "wrapped.delete_user",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            // A parameter named like a function of the module is a value nobody typed.
            (
                "python",
                "calls.py",
                "open_depot().people.delete_user|(user_id + 10)",
                picker(
                    "delete_user: by name, 2 declarations (chain broke at open_depot())",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            (
                "python",
                "calls.py",
                "made.people.delete_user",
                picker(
                    "delete_user: by name, 2 declarations (chain broke at made)",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            // The method is inherited; the receiver is a field; the chain hangs off a method
            // of a proven receiver.
            (
                "python",
                "calls.py",
                "inherited.delete_user",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via sub.people_repo() -> UserRepository)",
                    "repos.py:8",
                ),
            ),
            (
                "python",
                "calls.py",
                "through.delete_user",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via self.depot.people_repo() -> UserRepository)",
                    "repos.py:8",
                ),
            ),
            (
                "python",
                "calls.py",
                "self.depot.people_repo().delete_user",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via self.depot.people_repo() -> UserRepository)",
                    "repos.py:8",
                ),
            ),
            // Two locals assigned from each other's methods end in a picker, and end.
            (
                "python",
                "calls.py",
                "ahead.delete_user",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            // A `return` behind an `if` on its own line is a return the rules do not read.
            (
                "python",
                "calls.py",
                "picked.delete_user",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            (
                "typescript",
                "calls.ts",
                "inline.deleteUser",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
        ];
        for (fixture, file, code, want) in cases {
            let mut a = fixture_app(fixture);
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{fixture}: {file}: {code}");
        }
    }

    /// #100: a cast writes the type. `typing.cast(T, x)`, `x as T`, `v, ok := i.(T)`, and the
    /// variable of `switch v := x.(type)` inside a `case T:`, assigned to a name or with the chain
    /// hanging off the cast itself. A cast to a type the project does not declare, a `case` of
    /// several types and `default` prove nothing.
    #[test]
    fn a_cast_writes_the_type() {
        let cases: Vec<(&str, &str, &str, Shown)> = vec![
            // `cast(T, x)` and `typing.cast("T", x)` assigned to a name, and with the member hanging
            // off the cast itself.
            (
                "python",
                "casts.py",
                "repo.delete_user",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via repo: UserRepository)",
                    "repos.py:8",
                ),
            ),
            (
                "python",
                "casts.py",
                "audit.delete_user",
                jump(
                    "delete_user \u{2192} AuditLog.delete_user (via audit: AuditLog)",
                    "repos.py:13",
                ),
            ),
            (
                "python",
                "casts.py",
                "cast(UserRepository, found).delete_user",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via cast(UserRepository, found))",
                    "repos.py:8",
                ),
            ),
            // A cast to a type the project does not declare proves nothing.
            (
                "python",
                "casts.py",
                "missing.delete_user",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            (
                "python",
                "casts.py",
                "loose.delete_user",
                picker(
                    "delete_user: by name, 2 declarations",
                    &[
                        ("UserRepository.delete_user", "repos.py:8"),
                        ("AuditLog.delete_user", "repos.py:13"),
                    ],
                ),
            ),
            // TypeScript: `x as T`, the last type of `x as unknown as T`, and `(x as T).member`.
            (
                "typescript",
                "casts.ts",
                "repo.deleteUser",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via repo: UserRepository)",
                    "repos.ts:10",
                ),
            ),
            (
                "typescript",
                "casts.ts",
                "audit.deleteUser",
                jump(
                    "deleteUser \u{2192} AuditLog.deleteUser (via audit: AuditLog)",
                    "repos.ts:16",
                ),
            ),
            (
                "typescript",
                "casts.ts",
                "(found as UserRepository).deleteUser",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via found as UserRepository)",
                    "repos.ts:10",
                ),
            ),
            // `any`, and a generic wrapper that is no type of the project.
            (
                "typescript",
                "casts.ts",
                "loose.deleteUser",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            (
                "typescript",
                "casts.ts",
                "partial.deleteUser",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            // Go: `v, ok := i.(T)`, `v := i.(T)` and `i.(T).Member`.
            (
                "go",
                "casts.go",
                "repo.DeleteUser",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via repo: UserRepository)",
                    "repos.go:15",
                ),
            ),
            (
                "go",
                "casts.go",
                "audit.DeleteUser",
                jump(
                    "DeleteUser \u{2192} AuditLog.DeleteUser (via audit: AuditLog)",
                    "repos.go:21",
                ),
            ),
            (
                "go",
                "casts.go",
                "found.(*UserRepository).DeleteUser",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via found.(*UserRepository))",
                    "repos.go:15",
                ),
            ),
            // `fmt.Stringer` is declared outside the project.
            (
                "go",
                "casts.go",
                "found.(fmt.Stringer).String",
                jump("no definition for String", "casts.go:25"),
            ),
            // The variable of a type switch has the type of the `case` the cursor is in, also from a
            // block inside it.
            (
                "go",
                "casts.go",
                "v.DeleteUser|(id + 3)",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via v: UserRepository)",
                    "repos.go:15",
                ),
            ),
            (
                "go",
                "casts.go",
                "v.DeleteUser|(id + 4)",
                jump(
                    "DeleteUser \u{2192} AuditLog.DeleteUser (via v: AuditLog)",
                    "repos.go:21",
                ),
            ),
            // A second type switch below the first: the first one's `v` is out of scope.
            (
                "go",
                "casts.go",
                "return v.Area|()",
                jump("Area \u{2192} Square.Area (via v: Square)", "casts.go:11"),
            ),
            // A `case` of two types and `default` leave `v` what it was.
            (
                "go",
                "casts.go",
                "v.Area|() + 1",
                picker(
                    "Area: by name, 2 declarations",
                    &[
                        ("Square.Area", "casts.go:11"),
                        ("Circle.Area", "casts.go:15"),
                    ],
                ),
            ),
            (
                "go",
                "casts.go",
                "v.Area|() + 2",
                picker(
                    "Area: by name, 2 declarations",
                    &[
                        ("Square.Area", "casts.go:11"),
                        ("Circle.Area", "casts.go:15"),
                    ],
                ),
            ),
            // A `cast` the project declares itself is a function with a return type.
            (
                "python",
                "casts_own.py",
                "repo.delete_user",
                jump(
                    "delete_user \u{2192} AuditLog.delete_user (via cast() -> AuditLog)",
                    "repos.py:13",
                ),
            ),
        ];
        for (fixture, file, code, want) in cases {
            let mut a = fixture_app(fixture);
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{fixture}: {file}: {code}");
        }
    }

    /// #100, TypeScript: a class header prettier wraps is still the class's header. A list of type
    /// parameters over several lines ends in `> extends Base<K> {`, the clauses may stand on lines
    /// of their own over a lone `{`; `this`, `super`, the fields and what the class extends are
    /// read through both. The parameters of a function behind a wrapped `<…>` hide a module's
    /// namesake.
    #[test]
    fn a_wrapped_class_header_is_a_header() {
        let user = |via: &str| {
            jump(
                &format!("deleteUser \u{2192} UserRepository.deleteUser (via {via})"),
                "repos.ts:10",
            )
        };
        let audit = |via: &str| {
            jump(
                &format!("deleteUser \u{2192} AuditLog.deleteUser (via {via})"),
                "repos.ts:16",
            )
        };
        let seal = |via: &str| {
            jump(
                &format!("seal \u{2192} Crate.seal (via {via})"),
                "headers.ts:12",
            )
        };
        let cases: Vec<(&str, Shown)> = vec![
            // Under `> extends Crate<K> {`: a field of the class, one of its base, a method of
            // the base through `this` and through `super`, a constructor parameter.
            (
                "this.repo.deleteUser|(id)",
                user("this.repo: UserRepository"),
            ),
            (
                "this.audit.deleteUser|(id + 1)",
                audit("this.audit: AuditLog"),
            ),
            ("this.seal|(key)", seal("this: Shelf")),
            ("super.seal|(key)", seal("super of Shelf")),
            (
                "this.spare|)",
                jump(
                    "spare \u{2192} Shelf.spare (via this: Shelf)",
                    "headers.ts:34",
                ),
            ),
            // Under `extends` and `implements` on their own lines and a lone `{`.
            (
                "this.repo.deleteUser|(id + 2)",
                user("this.repo: UserRepository"),
            ),
            (
                "this.audit.deleteUser|(id + 3)",
                audit("this.audit: AuditLog"),
            ),
            (
                "super.seal|(key + \"!\")",
                seal("super of LongNamedShelfOfStrings"),
            ),
            // `implements Sealable` on its own line is read; the constraint `S extends Sealable`
            // of `Bin` implements nothing.
            (
                "seal|(key: string): void;",
                jump(
                    "seal \u{2192} LongNamedShelfOfStrings.seal (implementations of Sealable.seal)",
                    "headers.ts:70",
                ),
            ),
            // A method whose own type parameters are wrapped, `stash<` over `>(a: A, b: B)`; a
            // call written so inside a method is no declaration of it.
            (
                "this.stash|(key, this.spare)",
                jump(
                    "stash \u{2192} Crate.stash (via this: Shelf)",
                    "headers.ts:18",
                ),
            ),
            // What overrides a method of the base, and a field found by name, are told to be
            // the class's under `> extends … {` too.
            (
                "^  open|(): void {}",
                jump(
                    "open \u{2192} Shelf.open (implementations of Crate.open)",
                    "headers.ts:51",
                ),
            ),
            (
                "found.spare|)",
                jump(
                    "spare \u{2192} Shelf.spare (by name, 1 match)",
                    "headers.ts:34",
                ),
            ),
            (
                "found.one|)",
                jump("one \u{2192} Bin.one (by name, 1 match)", "headers.ts:81"),
            ),
            // `other: T` is typed by a parameter of the wrapped list: the chain breaks there.
            (
                "this.other.seal|(key)",
                picker(
                    "seal: by name, 4 declarations (chain broke at other)",
                    &[
                        ("Sealable.seal", "headers.ts:6"),
                        ("Crate.seal", "headers.ts:12"),
                        ("LongNamedShelfOfStrings.seal", "headers.ts:70"),
                        ("Bin.seal", "headers.ts:85"),
                    ],
                ),
            ),
            // `type Loose = any;` has no body: the fields of the class under it are not its.
            (
                "loose.audit.deleteUser|(id + 5)",
                picker(
                    "deleteUser: by name, 2 declarations (chain broke at audit)",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            // A lone `{` under a statement, with no `;`, is a block, and the statement still binds.
            (
                "void repo.deleteUser|(id + 4)",
                user("repo: UserRepository"),
            ),
            // `>(repo: UserRepository, …` binds the parameter: the module's `repo` is hidden.
            (
                "void repo.deleteUser|(key.length)",
                user("repo: UserRepository"),
            ),
            ("^  repo.deleteUser|(id)", audit("repo: AuditLog")),
            // `(repo: AuditLog) => void` among wrapped type parameters, or wrapped type
            // arguments, types the function's `repo` no more than it does on one line.
            (
                "void repo.deleteUser|(visit.length);",
                user("repo: UserRepository"),
            ),
            (
                "void repo.deleteUser|(visit.length + 1)",
                user("repo: UserRepository"),
            ),
        ];
        for (code, want) in cases {
            let mut a = fixture_app("typescript");
            d_on(&mut a, "headers.ts", code);
            assert_eq!(shown(&mut a), want, "{code}");
        }
    }

    /// #100, TypeScript: a member access prettier broke in front of its dots reads as the one line
    /// it is; a comment at the end of the line above names no receiver.
    #[test]
    fn a_member_access_broken_over_lines_is_one_chain() {
        let user = |via: &str| {
            jump(
                &format!("deleteUser \u{2192} UserRepository.deleteUser (via {via})"),
                "repos.ts:10",
            )
        };
        let by_name = || {
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            )
        };
        let cases: Vec<(&str, Shown)> = vec![
            // One break, two breaks with a comment between them, and the name in the middle.
            (
                "      .deleteUser|(id);",
                user("this.uow: UnitOfWork \u{2192} users: UserRepository"),
            ),
            (
                "      .deleteUser|(id + 1);",
                jump(
                    "deleteUser \u{2192} AuditLog.deleteUser (via this.uow: UnitOfWork \u{2192} audit: AuditLog)",
                    "repos.ts:16",
                ),
            ),
            (
                "      .audit",
                jump(
                    "audit \u{2192} UnitOfWork.audit (via this.uow: UnitOfWork)",
                    "chains.ts:5",
                ),
            ),
            // Off a method's call and off a function's, as on one line.
            (
                "      .deleteUser|(id + 2);",
                user("this.depot.peopleRepo(): UserRepository"),
            ),
            (
                "      .peopleRepo|()",
                jump(
                    "peopleRepo \u{2192} Depot.peopleRepo (via this.depot: Depot)",
                    "calls.ts:10",
                ),
            ),
            (
                "      .people.deleteUser|(id + 3);",
                user("openDepot(): Depot \u{2192} people: UserRepository"),
            ),
            (
                "      .people|.deleteUser(id + 3);",
                jump(
                    "people \u{2192} Depot.people (via openDepot(): Depot)",
                    "calls.ts:8",
                ),
            ),
            // `found // note`: the module's `note` is an `AuditLog`, and no receiver here.
            ("      .deleteUser|(id + 4);", by_name()),
            // A call of a call, and a call closed on a line of its own, stay by name.
            ("      .deleteUser|(id + 5);", by_name()),
            ("      .deleteUser|(id + 6);", by_name()),
            (
                "      .length",
                jump("no definition for length", "fluent.ts:39"),
            ),
        ];
        for (code, want) in cases {
            let mut a = fixture_app("typescript");
            d_on(&mut a, "fluent.ts", code);
            assert_eq!(shown(&mut a), want, "{code}");
        }
    }

    /// #100, TypeScript: `r!.m()` and `a?.b.m()` have the type of the plain access for a member
    /// lookup.
    #[test]
    fn a_non_null_or_optional_access_is_the_plain_one() {
        let user = |via: &str| {
            jump(
                &format!("deleteUser \u{2192} UserRepository.deleteUser (via {via})"),
                "repos.ts:10",
            )
        };
        let audit = |via: &str| {
            jump(
                &format!("deleteUser \u{2192} AuditLog.deleteUser (via {via})"),
                "repos.ts:16",
            )
        };
        let users = "this.uow: UnitOfWork \u{2192} users: UserRepository";
        let cases: Vec<(&str, Shown)> = vec![
            (
                "this.repo!.deleteUser|(id)",
                user("this.repo: UserRepository"),
            ),
            (
                "this.repo?.deleteUser|(id + 1)",
                user("this.repo: UserRepository"),
            ),
            ("this.uow?.users.deleteUser|(id + 2)", user(users)),
            (
                "this.uow?.users|.deleteUser(id + 2)",
                jump(
                    "users \u{2192} UnitOfWork.users (via this.uow: UnitOfWork)",
                    "chains.ts:4",
                ),
            ),
            // Two marks in one chain, and two around a name of one letter.
            (
                "this.uow!.audit!.deleteUser|(id + 3)",
                audit("this.uow: UnitOfWork \u{2192} audit: AuditLog"),
            ),
            (
                "u!.users!.deleteUser|(id + 9)",
                user("u.users: UserRepository"),
            ),
            ("spare?.deleteUser|(id + 4)", audit("spare: AuditLog")),
            ("spare!.deleteUser|(id + 5)", audit("spare: AuditLog")),
            // A receiver nobody typed stays by name.
            (
                "found?.deleteUser|(id + 6)",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            ("!note.deleteUser|.length", audit("note: AuditLog")),
        ];
        for (code, want) in cases {
            let mut a = fixture_app("typescript");
            d_on(&mut a, "optional.ts", code);
            assert_eq!(shown(&mut a), want, "{code}");
        }
    }

    /// #100, TypeScript: a `#private` member is the word with its `#`, on the `#` and on the name.
    #[test]
    fn a_private_name_keeps_its_hash() {
        let private = jump(
            "#addRoute \u{2192} Router.#addRoute (via this: Router)",
            "privates.ts:12",
        );
        let cases: Vec<(&str, Shown)> = vec![
            // On the name and on the `#`: the private method, not the public `addRoute`.
            ("this.#addRoute|(path);", private),
            (
                "this.|#addRoute(path + \"/\")",
                jump(
                    "#addRoute \u{2192} Router.#addRoute (via this: Router)",
                    "privates.ts:12",
                ),
            ),
            // A private field, as a target and as a link.
            (
                "this.#repo|.deleteUser(id)",
                jump(
                    "#repo \u{2192} Router.#repo (via this: Router)",
                    "privates.ts:5",
                ),
            ),
            (
                "console.log(this.#audit|)",
                jump(
                    "#audit \u{2192} Router.#audit (via this: Router)",
                    "privates.ts:6",
                ),
            ),
            (
                "other.#repo.deleteUser|(id + 2)",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via other.#repo: UserRepository)",
                    "repos.ts:10",
                ),
            ),
            // The public name never reaches a private one, by name or through a type.
            (
                "found.addRoute|(path)",
                jump(
                    "addRoute \u{2192} Router.addRoute (by name, 1 match)",
                    "privates.ts:16",
                ),
            ),
            (
                "this.addRoute|(path);",
                jump(
                    "addRoute \u{2192} Router.addRoute (via this: SubRouter)",
                    "privates.ts:16",
                ),
            ),
            // `route#addRoute` in a string is no private name.
            (
                "see route#addRoute|",
                jump(
                    "addRoute \u{2192} Router.addRoute (by name, 1 match)",
                    "privates.ts:16",
                ),
            ),
            // A subclass's `#addRoute` is its own, and implements nothing of the base's.
            (
                "this.#addRoute|(path, 1)",
                jump(
                    "#addRoute \u{2192} SubRouter.#addRoute (via this: SubRouter)",
                    "privates.ts:37",
                ),
            ),
            (
                "^  #addRoute|(path: string): void {",
                picker(
                    "#addRoute: at a declaration, 1 other by name",
                    &[("SubRouter.#addRoute", "privates.ts:37")],
                ),
            ),
        ];
        for (code, want) in cases {
            let mut a = fixture_app("typescript");
            d_on(&mut a, "privates.ts", code);
            assert_eq!(shown(&mut a), want, "{code}");
        }
    }

    /// #100, TypeScript: a NestJS service. Its dependencies are constructor parameters wrapped one
    /// to a line, decorated or not; `const { repo } = this` hands fields on; a class may stand
    /// behind namespaces, of an import or of the file itself.
    #[test]
    fn a_nest_service_reads_its_dependencies() {
        let user = |via: &str| {
            jump(
                &format!("deleteUser \u{2192} UserRepository.deleteUser (via {via})"),
                "repos.ts:10",
            )
        };
        let audit = |via: &str| {
            jump(
                &format!("deleteUser \u{2192} AuditLog.deleteUser (via {via})"),
                "repos.ts:16",
            )
        };
        let field = |name: &str, line: usize| {
            jump(
                &format!("{name} \u{2192} AlbumService.{name} (via this: AlbumService)"),
                &format!("nest.ts:{line}"),
            )
        };
        let spin = |via: &str, to: &str, line: usize| {
            jump(
                &format!("spin \u{2192} {to}.spin (via {via})"),
                &format!("nest_parts.ts:{line}"),
            )
        };
        let cases: Vec<(&str, Shown)> = vec![
            // The wrapped constructor: a decorated parameter, a plain one, one under its
            // decorator's line; as a link and as a target.
            (
                "this.repo.deleteUser|(id)",
                user("this.repo: UserRepository"),
            ),
            (
                "this.audit.deleteUser|(id + 1)",
                audit("this.audit: AuditLog"),
            ),
            (
                "this.uow.users.deleteUser|(id + 2)",
                user("this.uow: UnitOfWork \u{2192} users: UserRepository"),
            ),
            ("console.log(this.repo|,", field("repo", 15)),
            ("console.log(this.repo, this.audit|,", field("audit", 16)),
            (
                "console.log(this.repo, this.audit, this.uow|)",
                field("uow", 18),
            ),
            // `const { repo, audit: trail } = this` and `const { users } = this.uow`; the
            // module's `repo` is an `AuditLog`.
            (
                "void repo.deleteUser|(id + 3)",
                user("repo: UserRepository"),
            ),
            ("trail.deleteUser|(id + 4)", audit("trail: AuditLog")),
            (
                "void users.deleteUser|(id + 5)",
                user("users: UserRepository"),
            ),
            // A default may be what the name holds.
            (
                "uow.audit.deleteUser|(id + 6)",
                picker(
                    "deleteUser: by name, 2 declarations (chain broke at uow)",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
            ("^  repo.deleteUser|(id + 7)", audit("repo: AuditLog")),
            // Namespaces of an import, of a module taken whole, and of the file itself, where a
            // `Tool` and a `Widget` outside them are other classes.
            (
                "widget.spin|(id)",
                spin("widget: Widget", "Outer.Inner.Widget", 4),
            ),
            (
                "new Outer.Inner.Widget().spin|(id + 1)",
                spin("new Outer.Inner.Widget(): Widget", "Outer.Inner.Widget", 4),
            ),
            (
                "other.spin|(id + 2)",
                spin("other: Widget", "Outer.Inner.Widget", 4),
            ),
            (
                "gadget.spin|(id + 3)",
                spin("gadget: Gadget", "Outer.Gadget", 11),
            ),
            (
                "tool.turn|(id)",
                jump(
                    "turn \u{2192} Local.Tool.turn (via tool: Tool)",
                    "nest.ts:46",
                ),
            ),
            (
                "new Local.Tool().turn|(id + 1)",
                jump(
                    "turn \u{2192} Local.Tool.turn (via new Local.Tool(): Tool)",
                    "nest.ts:46",
                ),
            ),
            (
                "new Tool().turn|(id + 2)",
                jump(
                    "turn \u{2192} Tool.turn (via new Tool(): Tool)",
                    "nest.ts:53",
                ),
            ),
        ];
        for (code, want) in cases {
            let mut a = fixture_app("typescript");
            d_on(&mut a, "nest.ts", code);
            assert_eq!(shown(&mut a), want, "{code}");
        }
    }

    /// #131, TypeScript: a destructuring prettier wrapped over several lines binds its names, so
    /// the module's `ledger`, an `AuditLog`, does not answer for them.
    #[test]
    fn a_wrapped_destructuring_binds_its_names() {
        let user = jump(
            "deleteUser \u{2192} UserRepository.deleteUser (via ledger: UserRepository)",
            "repos.ts:10",
        );
        let by_name = || {
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            )
        };
        let cases: Vec<(&str, Shown)> = vec![
            // Out of `deps: Deps`, whose `ledger` is a `UserRepository`; from a block below too.
            ("ledger.deleteUser|(id + 13)", user),
            (
                "ledger.deleteUser|(id + 14)",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via ledger: UserRepository)",
                    "repos.ts:10",
                ),
            ),
            // With a type literal behind the pattern, and an array's pattern: nothing is read,
            // and nothing outside answers.
            ("ledger.deleteUser|(count + 15)", by_name()),
            ("ledger.deleteUser|(16)", by_name()),
            // Any statement closed so is read whole, once: `const ledger = {` … `} as T;`.
            (
                "ledger.deleteUser|(id + 17)",
                jump(
                    "deleteUser \u{2192} UserRepository.deleteUser (via ledger: UserRepository)",
                    "repos.ts:10",
                ),
            ),
        ];
        for (code, want) in cases {
            let mut a = fixture_app("typescript");
            d_on(&mut a, "scopes.ts", code);
            assert_eq!(shown(&mut a), want, "{code}");
        }
    }

    /// #100, TypeScript workspaces: a file sees the `node_modules` of every directory above it, the
    /// nearest first, and not those of the package beside it.
    #[test]
    fn a_workspace_package_sees_the_node_modules_above_it() {
        let main = "import { pick } from \"lib\";\n\npick(1);\n";
        let (dir, mut a) = project_app(
            "workspace",
            &[
                ("packages/api/src/main.ts", main),
                ("packages/web/src/main.ts", main),
                ("main.ts", main),
            ],
        );
        // Written after the project walk, which a `.gitignore` keeps out of them.
        for (path, text) in [
            (
                "packages/api/node_modules/lib/index.d.ts",
                "import { deep } from \"deep\";\nexport declare function pick(n: number): number;\nexport declare const made: typeof deep;\n",
            ),
            (
                "packages/api/node_modules/lib/node_modules/deep/index.d.ts",
                "export declare function deep(): void;\n",
            ),
            (
                "node_modules/lib/index.d.ts",
                "// An older one.\nexport declare function pick(n: string): string;\n",
            ),
        ] {
            std::fs::create_dir_all(dir.join(path).parent().unwrap()).unwrap();
            std::fs::write(dir.join(path), text).unwrap();
        }
        let top = || jump("pick: via import lib", "node_modules/lib/index.d.ts:2");
        let both = || {
            Shown::Picker(
                "pick: via import lib, 2 declarations".into(),
                vec![
                    (
                        "pick".into(),
                        "via import lib".into(),
                        "packages/api/node_modules/lib/index.d.ts:2".into(),
                    ),
                    (
                        "pick".into(),
                        "via import lib".into(),
                        "lib/index.d.ts:2".into(),
                    ),
                ],
            )
        };
        // Back in `api` after `web`: each file has its own view, and no directory is walked twice.
        for (file, want) in [
            ("packages/api/src/main.ts", both()),
            ("packages/web/src/main.ts", top()),
            ("main.ts", top()),
            ("packages/api/src/main.ts", both()),
        ] {
            d_on(&mut a, file, "^pick");
            assert_eq!(shown(&mut a), want, "{file}");
        }
        assert_eq!(a.node_modules.len(), 2);
        // From inside a dependency, its own `node_modules` is the nearest, and the one it lies
        // in is not listed twice.
        d_on(
            &mut a,
            "packages/api/node_modules/lib/index.d.ts",
            "made: typeof deep",
        );
        assert_eq!(
            shown(&mut a),
            jump(
                "deep: via import deep",
                "packages/api/node_modules/lib/node_modules/deep/index.d.ts:1"
            )
        );
        assert_eq!(a.buf.readonly, Some("outside the project"));
        // A dependency of `api` is outside the project from wherever `d` was pressed last.
        d_on(&mut a, "packages/web/src/main.ts", "^pick");
        a.jump_to(&dir.join("packages/api/node_modules/lib/index.d.ts"), 1);
        assert_eq!(a.buf.readonly, Some("outside the project"));
        d_on(
            &mut a,
            "packages/api/node_modules/lib/index.d.ts",
            "made: typeof deep",
        );
        let (roots, files) = a.external[&Kind::TsJs].clone();
        assert_eq!(
            roots,
            [
                dir.join("packages/api/node_modules/lib/node_modules"),
                dir.join("packages/api/node_modules"),
                dir.join("node_modules"),
            ]
        );
        assert_eq!(files.len(), 3);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// #100, TypeScript: `export { Trunk as TrunkBase }` is followed to the class the module
    /// declares under its own name, as hono exports the `HonoBase` its `Hono` extends. A re-export
    /// from another module under a new name is not.
    #[test]
    fn an_export_under_another_name_is_followed() {
        let cases: Vec<(&str, Shown)> = vec![
            (
                "super.lock|()",
                jump(
                    "lock \u{2192} Trunk.lock (via super of Boot)",
                    "aliased.ts:7",
                ),
            ),
            (
                "this.audit.deleteUser|(1)",
                jump(
                    "deleteUser \u{2192} AuditLog.deleteUser (via this.audit: AuditLog)",
                    "repos.ts:16",
                ),
            ),
            (
                "trunk.lock|()",
                jump(
                    "lock \u{2192} Trunk.lock (via trunk: Trunk)",
                    "aliased.ts:7",
                ),
            ),
            (
                "extends TrunkBase|",
                jump("TrunkBase: via import aliased.ts", "aliased.ts:4"),
            ),
            (
                "import { HatchBase|",
                jump("no definition for HatchBase", "aliased_use.ts:1"),
            ),
            // `HatchBase` is the `UserRepository` of `repos`, not the one `aliased` declares.
            (
                "hatch.deleteUser|(2)",
                picker(
                    "deleteUser: by name, 2 declarations",
                    &[
                        ("UserRepository.deleteUser", "repos.ts:10"),
                        ("AuditLog.deleteUser", "repos.ts:16"),
                    ],
                ),
            ),
        ];
        for (code, want) in cases {
            let mut a = fixture_app("typescript");
            d_on(&mut a, "aliased_use.ts", code);
            assert_eq!(shown(&mut a), want, "{code}");
        }
    }

    /// What the Punchcard review of the TypeScript items (#100, #131) found: each row was a wrong
    /// jump, a lost one or a picker of doubles on the first build of them.
    #[test]
    fn what_the_review_of_the_typescript_items_found() {
        let user = |via: &str| {
            jump(
                &format!("deleteUser \u{2192} UserRepository.deleteUser (via {via})"),
                "repos.ts:10",
            )
        };
        let by_name = || {
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            )
        };
        let public = || {
            jump(
                "addRoute \u{2192} Router.addRoute (by name, 1 match)",
                "privates.ts:16",
            )
        };
        let cases: Vec<(&str, &str, Shown)> = vec![
            // `svc.list()` is the namespace's function, until a parameter `svc` hides it: as a
            // callee and as the namespace of `new svc.Tool()`.
            (
                "shadowed.ts",
                "log.deleteUser|(id)",
                jump(
                    "deleteUser \u{2192} AuditLog.deleteUser (via svc.list(): AuditLog[])",
                    "repos.ts:16",
                ),
            ),
            ("shadowed.ts", "r.deleteUser|(id + 1)", by_name()),
            (
                "shadowed.ts",
                "tool.turn|(String(id + 2))",
                picker(
                    "turn: by name, 3 declarations",
                    &[
                        ("svc.Tool.turn", "shadowed.ts:10"),
                        ("Local.Tool.turn", "nest.ts:46"),
                        ("Tool.turn", "nest.ts:53"),
                    ],
                ),
            ),
            // A statement closed by `}` is one declaration, not its first line and itself.
            (
                "scopes.ts",
                "void ledger|.deleteUser(id + 17)",
                jump(
                    "ledger \u{2192} wrappedCast.ledger (local)",
                    "scopes.ts:129",
                ),
            ),
            // `this.stash<` over its type arguments over `>(key, this.spare);` is a call.
            (
                "headers.ts",
                "found.stash|(1, {})",
                jump(
                    "stash \u{2192} Crate.stash (by name, 1 match)",
                    "headers.ts:18",
                ),
            ),
            // #131 under a name commented out at the margin, and behind another name's default.
            (
                "scopes.ts",
                "ledger.deleteUser|(18)",
                user("ledger: UserRepository"),
            ),
            ("scopes.ts", "ledger?.deleteUser|(id + 19)", by_name()),
            // A `#` in a comment or a string starts no private name.
            ("privates.ts", "// As #addRoute|", public()),
            ("privates.ts", "console.log(\"#addRoute|", public()),
        ];
        for (file, code, want) in cases {
            let mut a = fixture_app("typescript");
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{file}: {code}");
        }
        // A JSX tag's `>` closes no header: the parameter of an attribute's callback is not the
        // children's `repo`.
        let (dir, mut a) = project_app(
            "jsx",
            &[
                (
                    "repos.ts",
                    "export class UserRepository {\n  deleteUser(id: number): void {}\n}\nexport class AuditLog {\n  deleteUser(id: number): void {}\n}\n",
                ),
                (
                    "page.tsx",
                    "import { AuditLog, UserRepository } from \"./repos\";\n\nexport function Page(repo: UserRepository, id: number) {\n  return (\n    <List\n      title=\"every user of the long named list\"\n      render={(repo: AuditLog) => repo.deleteUser(id)}\n    >\n      {repo.deleteUser(id + 1)}\n    </List>\n  );\n}\n",
                ),
            ],
        );
        a.external
            .insert(Kind::TsJs, (Vec::new(), Arc::new(Vec::new())));
        d_on(&mut a, "page.tsx", "{repo.deleteUser|(id + 1)");
        assert_eq!(
            shown(&mut a),
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via repo: UserRepository)",
                "repos.ts:2"
            )
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The rows of the Go section of #100, each on the `go` fixture.
    fn go_rows(cases: Vec<(&str, &str, Shown)>) {
        for (file, code, want) in cases {
            let mut a = fixture_app("go");
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{file}: {code}");
        }
    }

    const BOTH_DELETE_USER: [(&str, &str); 2] = [
        ("UserRepository.DeleteUser", "repos.go:15"),
        ("AuditLog.DeleteUser", "repos.go:21"),
    ];

    /// #100. A Go name no scope of the file declares is the package's: a `var` of another file,
    /// of a `var (` block, or below the cursor. A local of the name hides it, readable or not.
    #[test]
    fn a_go_package_level_name_is_read_in_every_file_of_the_package() {
        let repo = |via: &str| {
            jump(
                &format!("DeleteUser \u{2192} UserRepository.DeleteUser (via {via})"),
                "repos.go:15",
            )
        };
        let audit = |via: &str| {
            jump(
                &format!("DeleteUser \u{2192} AuditLog.DeleteUser (via {via})"),
                "repos.go:21",
            )
        };
        go_rows(vec![
            (
                "globals.go",
                "defaultRepo.DeleteUser|(id + 10",
                repo("defaultRepo: UserRepository"),
            ),
            // The `var` inside `globalsInner` and the raw string's line are not the package's.
            (
                "globals.go",
                "sharedAudit.DeleteUser|(id + 11",
                audit("sharedAudit: AuditLog"),
            ),
            (
                "globals.go",
                "sharedRepo.DeleteUser",
                repo("NewRepo() *UserRepository"),
            ),
            (
                "globals.go",
                "lateRepo.DeleteUser",
                repo("lateRepo: UserRepository"),
            ),
            (
                "globals.go",
                "spareAudit.DeleteUser",
                picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
            ),
            // Declared twice under build tags, as two types.
            (
                "globals.go",
                "taggedRepo.DeleteUser",
                picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
            ),
            // Two files that agree, and two of which one cannot be read.
            (
                "globals.go",
                "twinRepo.DeleteUser",
                repo("twinRepo: UserRepository"),
            ),
            (
                "globals.go",
                "mixedRepo.DeleteUser",
                picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
            ),
            (
                "globals.go",
                "defaultRepo.DeleteUser|(id + 15",
                audit("defaultRepo: AuditLog"),
            ),
            (
                "globals.go",
                "sharedAudit.DeleteUser|(id + 16",
                picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
            ),
            // Locals the scope walk does not read: nothing is proven from their absence.
            (
                "globals.go",
                "defaultRepo.DeleteUser|(18",
                picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
            ),
            (
                "globals.go",
                "defaultRepo.DeleteUser|(19",
                picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
            ),
            (
                "globals.go",
                "defaultRepo.DeleteUser|(20",
                picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
            ),
            (
                "globals.go",
                "defaultRepo.DeleteUser|(21",
                picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
            ),
            (
                "globals.go",
                "hop.DeleteUser",
                picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
            ),
            // An import of the external test package is no variable of `package main`.
            (
                "globals_x_test.go",
                "session.Close",
                jump(
                    "Close \u{2192} Session.Close (via defaultRepo.Open() *Session)",
                    "store/store.go:15",
                ),
            ),
        ]);
    }

    /// #100. Go's `type X = Y` is followed to `Y`, through a second alias and into another
    /// package; `type X Y` declares a type with methods of its own.
    #[test]
    fn a_go_alias_is_the_type_it_names() {
        go_rows(vec![
            (
                "aliases.go",
                "first.DeleteUser",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via first: UserRepository)",
                    "repos.go:15",
                ),
            ),
            (
                "aliases.go",
                "again.DeleteUser",
                jump(
                    "DeleteUser \u{2192} UserRepository.DeleteUser (via again: UserRepository)",
                    "repos.go:15",
                ),
            ),
            (
                "aliases.go",
                "session.Close",
                jump(
                    "Close \u{2192} Session.Close (via session: Session)",
                    "store/store.go:15",
                ),
            ),
            (
                "aliases.go",
                "twin.Close",
                jump(
                    "Close \u{2192} Session.Close (via twin: Session)",
                    "store/store.go:15",
                ),
            ),
            (
                "aliases.go",
                "kind.Flush",
                jump(
                    "Flush \u{2192} AuditKind.Flush (via kind: AuditKind)",
                    "aliases.go:19",
                ),
            ),
        ]);
        let mut a = fixture_app("go");
        d_on(&mut a, "aliases.go", "twin.Seal");
        assert_eq!(
            shown(&mut a),
            jump(
                "Seal \u{2192} AuditTwin.Seal (via twin: AuditTwin)",
                "aliases.go:39"
            )
        );
        // Two aliases of each other, which no compiler accepts, end.
        let (dir, mut a) = project_app(
            "alias-cycle",
            &[(
                "a.go",
                "package a\n\ntype A = B\n\ntype B = A\n\ntype C struct{}\n\nfunc (c C) Run() {}\n\nfunc f(x A) {\n\tx.Run()\n}\n",
            )],
        );
        a.external
            .insert(Kind::Go, (Vec::new(), Arc::new(Vec::new())));
        d_on(&mut a, "a.go", "x.Run");
        assert_eq!(a.message, "Run \u{2192} C.Run (by name, 1 match)");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// #100. A Go type declared once per platform (`clock_windows.go` beside a
    /// `//go:build !windows` file) is the one the host builds, and one declared under a tag of
    /// the project's own is the one a plain `go build` compiles, or the one `-tags` asks for
    /// (#137). From inside a file that is not built nothing is preferred.
    #[test]
    fn a_go_declaration_per_platform_is_the_hosts() {
        let (mine, other) = match cfg!(windows) {
            true => ("clock_windows.go", "clock_other.go"),
            false => ("clock_other.go", "clock_windows.go"),
        };
        let host = if cfg!(windows) {
            ("windows", 3)
        } else {
            ("other", 5)
        };
        let gauge = if cfg!(windows) {
            ("fast", 8)
        } else {
            ("other", 7)
        };
        let now = |file: &str| format!("platform/{file}:11");
        let via = |via: &str| format!("Now \u{2192} Clock.Now (via {via})");
        let both = |status: &str, name: &str, a: &str, b: &str| {
            Shown::Picker(
                status.into(),
                vec![
                    (name.into(), "by name".into(), a.into()),
                    (name.into(), "by name".into(), b.into()),
                ],
            )
        };
        go_rows(vec![
            (
                "platforms.go",
                "clock.Now",
                jump(&via("clock: Clock"), &now(mine)),
            ),
            (
                "platforms.go",
                "made.Now",
                jump(&via("platform.NewClock() *Clock"), &now(mine)),
            ),
            (
                "platforms.go",
                "platform.NewClock",
                jump(
                    "NewClock: via import platform/",
                    &format!("platform/{mine}:7"),
                ),
            ),
            (
                "platforms.go",
                "codec.Encode",
                jump(
                    "Encode \u{2192} Codec.Encode (via codec: Codec)",
                    "platform/codec_slow.go:7",
                ),
            ),
            (
                "platform/codec_fast.go",
                "c.Encode",
                both(
                    "Encode: by name, 2 declarations",
                    "Codec.Encode",
                    "platform/codec_fast.go:8",
                    "platform/codec_slow.go:7",
                ),
            ),
            // The type is declared once and its method per platform.
            (
                "platforms.go",
                "timer.Tick",
                jump(
                    "Tick \u{2192} Timer.Tick (via timer: Timer)",
                    &format!("platform/timer_{}.go:{}", host.0, host.1),
                ),
            ),
            // A platform and a tag of the project's own: `windows && !slow`.
            (
                "platforms.go",
                "gauge.Read",
                jump(
                    "Read \u{2192} Gauge.Read (via gauge: Gauge)",
                    &format!("platform/gauge_{}.go:{}", gauge.0, gauge.1),
                ),
            ),
            (
                &format!("platform/{other}"),
                "c.Now",
                both(
                    "Now: by name, 2 declarations",
                    "Clock.Now",
                    &now(other),
                    &now(mine),
                ),
            ),
        ]);
        // `GOFLAGS=-tags=fast` builds the other one.
        let mut a = fixture_app("go");
        a.go_build.tags = vec!["fast".into()];
        d_on(&mut a, "platforms.go", "codec.Encode");
        assert_eq!(
            shown(&mut a),
            jump(
                "Encode \u{2192} Codec.Encode (via codec: Codec)",
                "platform/codec_fast.go:8",
            )
        );
        // Asked from inside the file the host does not build, a method per platform is both;
        // and where no declaration is built (`gate_windows.go`, `gate_plan9.go`), all stay.
        if !cfg!(windows) {
            let mut a = fixture_app("go");
            d_on(&mut a, "platform/timer_windows.go", "t.Tick");
            let row = |place: &str| ("Timer.Tick".into(), "via t: Timer".into(), place.into());
            assert_eq!(
                shown(&mut a),
                Shown::Picker(
                    "Tick: via t: Timer, 2 declarations".into(),
                    vec![
                        row("platform/timer_windows.go:3"),
                        row("platform/timer_other.go:5")
                    ],
                )
            );
            d_on(&mut a, "platforms_gate.go", "gate.Lift");
            assert_eq!(
                shown(&mut a),
                both(
                    "Lift: by name, 2 declarations",
                    "Gate.Lift",
                    "platform/gate_plan9.go:5",
                    "platform/gate_windows.go:6",
                )
            );
            d_on(&mut a, "platforms_gate.go", "platform.NewGate");
            assert_eq!(a.message, "NewGate: via import platform/, 2 declarations");
            press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
            // `meter_fast.go` (`// +build`) is not known to be built, so `meter_windows.go` loses
            // to nothing.
            d_on(&mut a, "platforms_gate.go", "meter.Sample");
            assert_eq!(
                shown(&mut a),
                both(
                    "Sample: by name, 2 declarations",
                    "Meter.Sample",
                    "platform/meter_fast.go:8",
                    "platform/meter_windows.go:5",
                )
            );
        }
    }

    /// #100. `d` on a Go package qualifier is the import line of the open file. A local of the
    /// name is the local, and a name the package declares itself is not the import its path
    /// happens to spell.
    #[test]
    fn a_go_package_qualifier_is_its_import_line() {
        go_rows(vec![
            (
                "qualifiers.go",
                "depot|.Open",
                jump(
                    "depot: via import example.com/fixture/store",
                    "qualifiers.go:7",
                ),
            ),
            (
                "qualifiers.go",
                "fmt|.Println",
                jump("fmt: via import fmt", "qualifiers.go:4"),
            ),
            // On the import line itself the name is no qualifier.
            (
                "qualifiers.go",
                "depot| \"example",
                jump("no definition for depot", "qualifiers.go:7"),
            ),
            (
                "qualifiers.go",
                "depot|.Remove",
                jump(
                    "depot \u{2192} QualifierHidden.depot (local)",
                    "qualifiers.go:19",
                ),
            ),
            (
                "qualifiers.go",
                "depot|.Remove(2",
                jump("no definition for depot", "qualifiers.go:32"),
            ),
            (
                "qualifiers.go",
                "h.depot|.Remove",
                jump(
                    "depot \u{2192} qualifierHolder.depot (via h: qualifierHolder)",
                    "qualifiers.go:37",
                ),
            ),
            (
                "qualifiers.go",
                "ledger|.DeleteUser",
                picker(
                    "ledger: by name, 5 declarations",
                    &[
                        ("ledger", "scopes.go:3"),
                        ("ScopedRotate.ledger", "scopes.go:10"),
                        ("ScopedNested.ledger", "scopes.go:15"),
                        ("ledger", "scopes.go:17"),
                        ("ledger", "scopes.go:34"),
                    ],
                ),
            ),
        ]);
    }

    /// #100. A named result or a parameter of a Go function is named after the function, as a
    /// local of its body is: `Reload.err`, not `UserRepository.err`, which would be a field.
    #[test]
    fn a_go_named_result_is_named_after_its_function() {
        go_rows(vec![
            (
                "results.go",
                "return user, err",
                jump("err \u{2192} Reload.err (local)", "results.go:4"),
            ),
            (
                "results.go",
                "count| == 0",
                jump("count \u{2192} Reload.count (local)", "results.go:5"),
            ),
            (
                "results.go",
                "FindUser(id|)",
                jump("id \u{2192} Reload.id (local)", "results.go:4"),
            ),
            (
                "results.go",
                "\treturn err",
                jump("err \u{2192} ReloadPlain.err (local)", "results.go:12"),
            ),
        ]);
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
                    "run: implementations of BaseJob.run, 5 declarations",
                    "BaseJob.run",
                    &[
                        ("ImportJob.run", "impls.py:10"),
                        ("ExportJob.run", "impls.py:15"),
                        // A header black wrapped, one base to a line (#100), and a class
                        // below it. `Roster` has a `BaseJob,` line too, an argument of a call.
                        ("WrappedJob.run", "impls.py:57"),
                        ("NightlyJob.run", "impls.py:24"),
                        ("DeepJob.run", "impls.py:81"),
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

    fn py_rows(cases: Vec<(&str, &str, Shown)>) {
        for (file, code, want) in cases {
            let mut a = fixture_app("python");
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{file}: {code}");
        }
    }

    const EVERY_SEAL: [(&str, &str); 7] = [
        ("Crate.seal", "depot/crates.py:2"),
        ("Lid.seal", "depot/crates.py:7"),
        ("Pallet.seal", "depot/crates.py:12"),
        ("Tray.seal", "depot/crates.py:17"),
        ("Hook.seal", "depot/crates.py:22"),
        ("Label.seal", "depot/labels.py:5"),
        ("Pallet.seal", "depot/labels.py:10"),
    ];

    /// #100. `from x import y as z` types a receiver as `y`, and a module of the project that
    /// imports a name without declaring it hands it on: a package's `__init__.py`, by a relative
    /// import, under another name, from another package that hands it on in turn, through
    /// `import *`. What the module may not end up with stays by name: two sources, a name it
    /// also assigns, the import of a function in it, a cycle.
    #[test]
    fn a_python_alias_and_a_module_that_hands_a_name_on_are_followed() {
        py_rows(vec![
            (
                "aliases.py",
                "repo.delete_user|(1",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via repo: UserRepository)",
                    "repos.py:8",
                ),
            ),
            (
                "aliases.py",
                "log.delete_user|(2",
                jump(
                    "delete_user \u{2192} AuditLog.delete_user (via log: AuditLog)",
                    "repos.py:13",
                ),
            ),
            // The alias called: the class it names.
            (
                "aliases.py",
                "Users().find_user",
                jump(
                    "find_user \u{2192} UserRepository.find_user (via Users(): UserRepository)",
                    "repos.py:5",
                ),
            ),
            (
                "aliases.py",
                "repo.delete_user|(4",
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via repo: UserRepository)",
                    "repos.py:8",
                ),
            ),
            // `depot/__init__.py` declares none of these.
            (
                "aliases.py",
                "crate.seal",
                jump(
                    "seal \u{2192} Crate.seal (via crate: Crate)",
                    "depot/crates.py:2",
                ),
            ),
            // `from .crates import Lid as Cover`.
            (
                "aliases.py",
                "cover.seal",
                jump(
                    "seal \u{2192} Lid.seal (via cover: Lid)",
                    "depot/crates.py:7",
                ),
            ),
            // Two modules on: `depot` has it from `store`, which has it from `.sessions`.
            (
                "aliases.py",
                "conn.close",
                jump(
                    "close \u{2192} Session.close (via conn: Session)",
                    "store/sessions.py:6",
                ),
            ),
            (
                "aliases.py",
                "trail.delete_user",
                jump(
                    "delete_user \u{2192} AuditLog.delete_user (via trail: AuditLog)",
                    "repos.py:13",
                ),
            ),
            (
                "aliases.py",
                "dial().close",
                jump(
                    "close \u{2192} Session.close (via dial() -> Session)",
                    "store/sessions.py:6",
                ),
            ),
            // `from .labels import *`.
            (
                "aliases.py",
                "label.seal",
                jump(
                    "seal \u{2192} Label.seal (via label: Label)",
                    "depot/labels.py:5",
                ),
            ),
            // `fakes.UserRepository`, not the one of `repos`.
            (
                "aliases.py",
                "users.users",
                jump(
                    "users \u{2192} UserRepository.users (via users: UserRepository)",
                    "fakes.py:3",
                ),
            ),
            // A parameter called like the alias is a value of its own type.
            (
                "aliases.py",
                "Users.delete_user|(7",
                jump(
                    "delete_user \u{2192} AuditLog.delete_user (via Users: AuditLog)",
                    "repos.py:13",
                ),
            ),
            // `d` on the imported word itself. `Crate` comes by two routes, the import and
            // the `*` of a module that imports it too, to one declaration.
            (
                "aliases.py",
                "crate: Crate",
                jump("Crate: via import depot/crates.py", "depot/crates.py:1"),
            ),
            (
                "aliases.py",
                "cover: Cover",
                jump("Cover: via import depot/crates.py", "depot/crates.py:6"),
            ),
            (
                "aliases.py",
                "conn: Session",
                jump(
                    "Session: via import store/sessions.py",
                    "store/sessions.py:1",
                ),
            ),
            // Four modules that hand the name on are followed, five are not.
            (
                "relays.py",
                "parcel.wrap_up",
                jump(
                    "wrap_up \u{2192} Parcel.wrap_up (via parcel: Parcel)",
                    "relay5.py:2",
                ),
            ),
            (
                "relays.py",
                "bundle.wrap_up",
                jump(
                    "wrap_up \u{2192} Parcel.wrap_up (by name, 1 match)",
                    "relay5.py:2",
                ),
            ),
            // An import in a docstring's example is no source.
            (
                "docstring_import.py",
                "repo: UserRepository",
                jump("UserRepository: via import repos.py", "repos.py:4"),
            ),
            // … but for the reader of that example, as it was.
            (
                "docstring_import.py",
                "from fakes import UserRepository",
                Shown::Picker(
                    "UserRepository: 2 declarations".into(),
                    vec![
                        (
                            "UserRepository".into(),
                            "via import repos.py".into(),
                            "repos.py:4".into(),
                        ),
                        (
                            "UserRepository".into(),
                            "via import fakes.py".into(),
                            "fakes.py:1".into(),
                        ),
                    ],
                ),
            ),
            // `try` / `except ImportError` names two sources: both are offered, neither typed.
            (
                "aliases.py",
                "pallet: Pallet",
                Shown::Picker(
                    "Pallet: 2 declarations".into(),
                    vec![
                        (
                            "Pallet".into(),
                            "via import depot/labels.py".into(),
                            "depot/labels.py:9".into(),
                        ),
                        (
                            "Pallet".into(),
                            "via import depot/crates.py".into(),
                            "depot/crates.py:11".into(),
                        ),
                    ],
                ),
            ),
            (
                "aliases.py",
                "pallet.seal",
                picker("seal: by name, 7 declarations", &EVERY_SEAL),
            ),
            // `Tray` is imported and, under an `if`, assigned.
            (
                "aliases.py",
                "tray: Tray",
                jump("Tray: by name, 1 match", "depot/crates.py:16"),
            ),
            (
                "aliases.py",
                "tray.seal",
                picker("seal: by name, 7 declarations", &EVERY_SEAL),
            ),
            // An import inside a function of the module binds nothing of the module.
            (
                "aliases.py",
                "hook: Hook",
                jump("Hook: by name, 1 match", "depot/crates.py:21"),
            ),
            (
                "aliases.py",
                "hook.seal",
                picker("seal: by name, 7 declarations", &EVERY_SEAL),
            ),
            // `depot` has `Ring` from `depot.loop`, which has it from `depot`.
            (
                "aliases.py",
                "ring: Ring",
                jump("no definition for Ring", "aliases.py:35"),
            ),
        ]);
    }

    /// #100. `Cls.CONST`, an `Enum` member and a dataclass field are what the class body
    /// declares, in the class the qualifier names or one above it: `via Cls`. An attribute a
    /// method assigns to `self`, a member of a member, a function's attribute, a class declared
    /// inside the function and a parameter of the class's name are not that. A `with … as x`
    /// target is a local, which hides the module's name whatever its type (so on master).
    #[test]
    fn a_python_class_attribute_is_looked_up_in_the_class() {
        let both_delete_user = [
            ("UserRepository.delete_user", "repos.py:8"),
            ("AuditLog.delete_user", "repos.py:13"),
        ];
        py_rows(vec![
            (
                "consts.py",
                "Limits.MAX_USERS",
                jump(
                    "MAX_USERS \u{2192} Limits.MAX_USERS (via Limits)",
                    "consts.py:12",
                ),
            ),
            (
                "consts.py",
                "Limits.timeout",
                jump(
                    "timeout \u{2192} Limits.timeout (via Limits)",
                    "consts.py:13",
                ),
            ),
            // Two enums with a `RED`.
            (
                "consts.py",
                "    Color.RED",
                jump("RED \u{2192} Color.RED (via Color)", "consts.py:27"),
            ),
            (
                "consts.py",
                "    Shade.RED",
                jump("RED \u{2192} Shade.RED (via Shade)", "consts.py:32"),
            ),
            // A dataclass field; `Archive.label` is a namesake.
            (
                "consts.py",
                "Point.label",
                jump("label \u{2192} Point.label (via Point)", "consts.py:38"),
            ),
            // Inherited, overridden, and a method as before.
            (
                "consts.py",
                "Tight.MAX_USERS",
                jump(
                    "MAX_USERS \u{2192} Limits.MAX_USERS (via Tight)",
                    "consts.py:12",
                ),
            ),
            (
                "consts.py",
                "Tight.timeout",
                jump("timeout \u{2192} Tight.timeout (via Tight)", "consts.py:20"),
            ),
            (
                "consts.py",
                "Tight.check",
                jump("check \u{2192} Limits.check (via Tight)", "consts.py:15"),
            ),
            // Behind an import, an alias and a module.
            (
                "consts_use.py",
                "Color.GREEN",
                jump("GREEN \u{2192} Color.GREEN (via Color)", "consts.py:28"),
            ),
            (
                "consts_use.py",
                "Caps.MAX_USERS",
                jump(
                    "MAX_USERS \u{2192} Limits.MAX_USERS (via Caps)",
                    "consts.py:12",
                ),
            ),
            (
                "consts_use.py",
                "consts.Shade.RED",
                jump("RED \u{2192} Shade.RED (via consts.Shade)", "consts.py:32"),
            ),
            // `self.count = 0` is an instance's.
            (
                "consts.py",
                "Tight.count",
                jump(
                    "count \u{2192} Tight.count (by name, 1 match)",
                    "consts.py:23",
                ),
            ),
            (
                "consts.py",
                "Limits.missing",
                jump("no definition for missing", "consts.py:48"),
            ),
            (
                "consts.py",
                "Color.RED.value",
                jump(
                    "no definition for value (chain broke at Color)",
                    "consts.py:53",
                ),
            ),
            (
                "consts.py",
                "scan.cache",
                jump("no definition for cache", "consts.py:54"),
            ),
            // The function's own `class Color`, which the rules do not read (#101).
            (
                "consts.py",
                "Color.RED|  # the class",
                picker(
                    "RED: by name, 3 declarations",
                    &[
                        ("Color.RED", "consts.py:27"),
                        ("Shade.RED", "consts.py:32"),
                        ("inner.Color.RED", "consts.py:59"),
                    ],
                ),
            ),
            (
                "consts.py",
                "Color.RED|  # a parameter",
                jump("RED \u{2192} Shade.RED (via Color: Shade)", "consts.py:32"),
            ),
            (
                "consts.py",
                "Limits.MAX_USERS|  # a parameter",
                jump(
                    "MAX_USERS \u{2192} Limits.MAX_USERS (by name, 1 match)",
                    "consts.py:12",
                ),
            ),
            // The class binds the word in a shape the rules do not read: a tuple, a `def` under
            // an `if`, a `for`. Not reading it is no proof that the base's is meant.
            (
                "consts.py",
                "    Unread.RANK",
                jump(
                    "RANK \u{2192} Plain.RANK (by name, 1 match)",
                    "consts.py:101",
                ),
            ),
            (
                "consts.py",
                "    Unread.check",
                picker(
                    "check: by name, 3 declarations",
                    &[
                        ("Limits.check", "consts.py:15"),
                        ("Plain.check", "consts.py:105"),
                        // Under an `if` the walk of `qualified` names nothing.
                        ("check", "consts.py:112"),
                    ],
                ),
            ),
            (
                "consts.py",
                "    Unread.CODE",
                jump(
                    "CODE \u{2192} Plain.CODE (by name, 1 match)",
                    "consts.py:103",
                ),
            ),
            // The body goes on past a comment and a string at column 0, and may share the
            // header's line.
            (
                "consts.py",
                "    Noted.Meta",
                jump("Meta \u{2192} Noted.Meta (via Noted)", "consts.py:133"),
            ),
            (
                "consts.py",
                "    Queried.RANK",
                jump(
                    "RANK \u{2192} Plain.RANK (by name, 1 match)",
                    "consts.py:101",
                ),
            ),
            (
                "consts.py",
                "    Short.CODE",
                jump(
                    "CODE \u{2192} Plain.CODE (by name, 1 match)",
                    "consts.py:103",
                ),
            ),
            // The header's own lines at the margin end nothing either.
            (
                "consts.py",
                "    Flush.CODE",
                jump(
                    "CODE \u{2192} Plain.CODE (by name, 1 match)",
                    "consts.py:103",
                ),
            ),
            // … and a nested class, which is what the class declares.
            (
                "consts.py",
                "    Unread.Meta",
                jump("Meta \u{2192} Unread.Meta (via Unread)", "consts.py:115"),
            ),
            // Two bases that disagree: the order Python reads them in is not computed.
            (
                "consts.py",
                "Diamond.LEVEL",
                picker(
                    "LEVEL: by name, 2 declarations",
                    &[
                        ("Root.LEVEL", "consts.py:80"),
                        ("Right.LEVEL", "consts.py:88"),
                    ],
                ),
            ),
            // `with … as`: one line, two targets, and wrapped in brackets.
            (
                "consts.py",
                "        conn|.delete",
                jump("conn \u{2192} scan.conn (local)", "consts.py:70"),
            ),
            (
                "consts.py",
                "        handle|.read",
                jump("handle \u{2192} scan.handle (local)", "consts.py:70"),
            ),
            (
                "consts.py",
                "        wrapped|.delete",
                jump("wrapped: local", "consts.py:74"),
            ),
            (
                "consts.py",
                "conn.delete_user",
                picker("delete_user: by name, 2 declarations", &both_delete_user),
            ),
            (
                "consts.py",
                "wrapped.delete_user",
                picker("delete_user: by name, 2 declarations", &both_delete_user),
            ),
        ]);
    }

    /// #100. A Python class header wrapped over several lines: its members are the class's,
    /// its bases are read, and from inside it a member's implementations are found, the
    /// bases of the implementing class sharing one line under theirs.
    #[test]
    fn a_wrapped_python_class_header_keeps_its_name_and_its_bases() {
        py_rows(vec![
            (
                "impls.py",
                "self.mop",
                jump(
                    "mop \u{2192} WrappedJob.mop (via self: WrappedJob)",
                    "impls.py:60",
                ),
            ),
            (
                "impls.py",
                "job.mop",
                jump(
                    "mop \u{2192} WrappedJob.mop (via job: WrappedJob)",
                    "impls.py:60",
                ),
            ),
            (
                "impls.py",
                "def tick",
                jump(
                    "tick \u{2192} Narrow.tick (implementations of WideBase.tick)",
                    "impls.py:94",
                ),
            ),
            (
                "impls.py",
                "self.sweep",
                jump(
                    "sweep \u{2192} Sweeper.sweep (via self: Narrow)",
                    "impls.py:44",
                ),
            ),
        ]);
    }

    /// #100. A parameter of a Python function is named after the function, as a local of its
    /// body is: `RecipeController.get_one.slug`, not `RecipeController.slug`, a field's name.
    #[test]
    fn a_python_parameter_is_named_after_its_function() {
        py_rows(vec![
            (
                "params.py",
                "found = slug",
                jump(
                    "slug \u{2192} RecipeController.get_one.slug (local)",
                    "params.py:2",
                ),
            ),
            (
                "params.py",
                "return found",
                jump(
                    "found \u{2192} RecipeController.get_one.found (local)",
                    "params.py:3",
                ),
            ),
            // A signature wrapped over several lines: the parameter, and a local under its `)`.
            (
                "params.py",
                "kept = slugs",
                jump(
                    "slugs \u{2192} RecipeController.get_many.slugs (local)",
                    "params.py:6",
                ),
            ),
            (
                "params.py",
                "return kept",
                jump(
                    "kept \u{2192} RecipeController.get_many.kept (local)",
                    "params.py:10",
                ),
            ),
            (
                "params.py",
                "        return slug",
                jump(
                    "slug \u{2192} RecipeController.get_later.slug (local)",
                    "params.py:13",
                ),
            ),
            (
                "params.py",
                "return level",
                jump("level \u{2192} top.level (local)", "params.py:17"),
            ),
        ]);
    }

    /// #131. A Python binding that does not start its line is a binding: behind the `:` of a
    /// header on the same line, behind a `;`, annotated, chained. It used to be unseen, and the
    /// module's `ledger`, an `AuditLog`, was proven in its place. One the rules cannot read hides
    /// the module's all the same; a comparison binds nothing.
    #[test]
    fn a_python_binding_need_not_start_its_line() {
        let users = |n: &str| {
            (
                "scopes.py",
                format!("ledger.delete_user|({n}"),
                jump(
                    "delete_user \u{2192} UserRepository.delete_user (via ledger: UserRepository)",
                    "repos.py:8",
                ),
            )
        };
        let unproven = |n: &str, status: &str| {
            let rows = [
                ("UserRepository.delete_user", "repos.py:8"),
                ("AuditLog.delete_user", "repos.py:13"),
            ];
            (
                "scopes.py",
                format!("ledger.delete_user|({n}"),
                picker(status, &rows),
            )
        };
        let module = |n: &str| {
            (
                "scopes.py",
                format!("ledger.delete_user|({n}"),
                jump(
                    "delete_user \u{2192} AuditLog.delete_user (via ledger: AuditLog)",
                    "repos.py:13",
                ),
            )
        };
        let by_name = "delete_user: by name, 2 declarations";
        let cases = vec![
            // `if fresh: ledger = UserRepository()`.
            users("10"),
            // … and `else: ledger = open("ledger")`.
            unproven("11", by_name),
            // `count = 1; ledger = UserRepository()`.
            users("12 + count"),
            // `for name in names["a:b"]: ledger = …`: the `:` of the string ends no header.
            users("13"),
            // `with … as source: ledger: UserRepository = source`.
            users("14"),
            // The `:` of a header wrapped over two lines, the first ending in `and`, in `(`.
            users("19"),
            users("20"),
            // `first = ledger = UserRepository()`.
            unproven("15 + len", by_name),
            // `try: from fakes import ledger`: only the statement starts with `from`.
            unproven("16", by_name),
            // `if cold: self.ledger = UserRepository()` beside `self.ledger = AuditLog()`.
            unproven(
                "18",
                "delete_user: by name, 2 declarations (chain broke at ledger)",
            ),
            // `if ledger == flag: print(ledger)` binds nothing, and neither does a keyword
            // argument on a line that continues a call: the module's.
            module("17"),
            module("21"),
            // A lambda's `:` behind the end of a call's arguments is no header's.
            module("22"),
        ];
        for (file, code, want) in cases {
            let mut a = fixture_app("python");
            d_on(&mut a, file, &code);
            assert_eq!(shown(&mut a), want, "{file}: {code}");
        }
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
                // `store/__init__.py` imports it from `.sessions` and hands it on (#100).
                jump(
                    "open_session: via import store/sessions.py",
                    "store/sessions.py:16",
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
                    "import fakelib\nfrom fakelib import pick\nfrom fakelib.core import make\n\nlimit = 3\n\n\ndef run():\n    make()\n    fakelib.pick(1)\n    make(\n        limit=1,\n    )\n    return pick(1)\n",
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
        // Behind the module's name as well: `fakelib.pick` is no method of a class in it.
        d_on(&mut a, "outside.py", "fakelib.pick");
        assert_eq!(a.message, "no definition for pick");
        // A keyword argument names a parameter: the one variable spelled so is offered.
        d_on(&mut a, "outside.py", "    limit");
        assert!(a.picker.is_some(), "{}", a.message);
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
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

    /// #129. The chain in front of the word is joined as the kind qualifies a name, `Depot::open`
    /// in Rust and C++, so a declaration that reads so is the answer there as `Outer.find` is
    /// elsewhere. A type nothing declares the word in, `Self`, and a value's method stay by name.
    #[test]
    fn a_path_in_front_of_the_word_is_joined_as_the_kind_qualifies() {
        let (dir, mut a) = project_app(
            "paths",
            &[
                (
                    "depot.rs",
                    "pub struct Depot;\n\nimpl Depot {\n    pub fn open() -> Self {\n        Depot\n    }\n\n    pub fn again() -> Self {\n        Self::open()\n    }\n}\n\npub struct Shed;\n\nimpl Shed {\n    pub fn open() -> Self {\n        Shed\n    }\n}\n\npub struct Bare;\n\npub fn run(shed: Shed) {\n    let _ = Depot::open();\n    let _ = depot::Shed::open();\n    let _ = Bare::open();\n    let _ = shed.open();\n    let _ = vendored::Shed::open();\n    let _ = crate::depot::Shed::open();\n    let _ = store::Shelf::stock();\n    let _ = <Shed>::open();\n}\n",
                ),
                (
                    "store/shelf.rs",
                    "pub struct Shelf;\n\nimpl Shelf {\n    pub fn stock() -> Self {\n        Shelf\n    }\n}\n",
                ),
                ("cli.rs", "#[derive(Parser)]\npub struct Config;\n"),
                (
                    "settings.rs",
                    "pub struct Config;\n\nimpl Config {\n    pub fn parse() -> Self {\n        Config\n    }\n}\n",
                ),
                ("io.rs", "pub fn read() {}\n"),
                (
                    "app/Yard.php",
                    "<?php\nnamespace App;\n\nclass Yard\n{\n    public static function open(): void\n    {\n    }\n}\n\nfunction near(): void\n{\n    Yard::open();\n}\n",
                ),
                (
                    "app/Far.php",
                    "<?php\nnamespace Other;\n\nfunction far(): void\n{\n    \\Vendor\\Pkg\\Yard::open();\n}\n",
                ),
                (
                    "app/Grouped.php",
                    "<?php\nnamespace Other;\n\nuse Vendor\\Pkg\\{Yard, Shed};\n\nfunction grouped(): void\n{\n    Yard::open( );\n}\n",
                ),
                (
                    "glob.rs",
                    "use std::io::*;\n\nfn glob() {\n    let _ = Error::new();\n}\n",
                ),
                (
                    "using.cpp",
                    "using std::filesystem::Depot;\n\nvoid far() {\n    Depot::open();\n}\n",
                ),
                (
                    "error.rs",
                    "pub struct Error;\n\nimpl Error {\n    pub fn new() -> Self {\n        Error\n    }\n}\n",
                ),
                (
                    "main.rs",
                    "use crate::cli::Config;\nuse crate::depot::Depot;\nuse std::io;\n\nfn main() {\n    let _ = Depot::open();\n    let _ = Config::parse();\n    let _ = io::Error::new();\n}\n",
                ),
                (
                    "depot.cpp",
                    "struct Depot {\n    static Depot open() {\n        return Depot{};\n    }\n};\n\nstruct Shed {\n    static Shed open() {\n        return Shed{};\n    }\n};\n\nstruct Bare {};\n\nstruct crate {\n    void open() {}\n};\n\nstruct lid {\n    void open() {}\n};\n\nvoid use(lid crate) {\n    crate.open();\n}\n\nint main() {\n    Depot::open();\n    Bare::open();\n    return 0;\n}\n",
                ),
            ],
        );
        for kind in [Kind::Rust, Kind::C] {
            a.external.insert(kind, (Vec::new(), Arc::new(Vec::new())));
        }
        let rs = [("Depot::open", "depot.rs:4"), ("Shed::open", "depot.rs:16")];
        let cpp = [
            ("Depot::open", "depot.cpp:2"),
            ("Shed::open", "depot.cpp:8"),
            ("crate::open", "depot.cpp:16"),
            ("lid::open", "depot.cpp:20"),
        ];
        let cases = [
            (
                "depot.rs",
                "= Depot::open",
                jump("open \u{2192} Depot::open (via Depot)", "depot.rs:4"),
            ),
            (
                "depot.rs",
                "depot::Shed::open",
                jump("open \u{2192} Shed::open (via depot::Shed)", "depot.rs:16"),
            ),
            (
                "depot.rs",
                "crate::depot::Shed::open",
                jump(
                    "open \u{2192} Shed::open (via crate::depot::Shed)",
                    "depot.rs:16",
                ),
            ),
            // A directory of the project.
            (
                "depot.rs",
                "store::Shelf::stock",
                jump(
                    "stock \u{2192} Shelf::stock (via store::Shelf)",
                    "store/shelf.rs:4",
                ),
            ),
            // A `use` of the project's own type is no reason to doubt it.
            (
                "main.rs",
                "= Depot::open",
                jump("open \u{2192} Depot::open (via Depot)", "depot.rs:4"),
            ),
            // Two types called `Config`, and the derive of the imported one supplies `parse`:
            // the name of a type is no proof of which one.
            (
                "main.rs",
                "Config::parse",
                jump(
                    "parse \u{2192} Config::parse (by name, 1 match)",
                    "settings.rs:4",
                ),
            ),
            // `io` is `std::io` by the file's `use`, whatever `io.rs` the project has.
            (
                "main.rs",
                "io::Error::new",
                jump("new \u{2192} Error::new (by name, 1 match)", "error.rs:4"),
            ),
            // A glob may bring in an `Error` of its own; so may a C++ `using`.
            (
                "glob.rs",
                "Error::new",
                jump("new \u{2192} Error::new (by name, 1 match)", "error.rs:4"),
            ),
            (
                "using.cpp",
                "    Depot::open",
                picker("open: by name, 4 declarations", &cpp),
            ),
            // PHP: the class of the project, one spelled out in another namespace, one a
            // grouped `use` brings in.
            (
                "app/Yard.php",
                "    Yard::open",
                jump("open \u{2192} Yard::open (via Yard)", "app/Yard.php:6"),
            ),
            (
                "app/Far.php",
                "Pkg\\Yard::open",
                jump(
                    "open \u{2192} Yard::open (by name, 1 match)",
                    "app/Yard.php:6",
                ),
            ),
            (
                "app/Grouped.php",
                "Yard::open|( )",
                jump(
                    "open \u{2192} Yard::open (by name, 1 match)",
                    "app/Yard.php:6",
                ),
            ),
            // No name in front of the `::`.
            (
                "depot.rs",
                "<Shed>::open",
                picker("open: by name, 2 declarations", &rs),
            ),
            // No file or directory of the project is called `vendored`.
            (
                "depot.rs",
                "vendored::Shed::open",
                picker("open: by name, 2 declarations", &rs),
            ),
            (
                "depot.rs",
                "Bare::open",
                picker("open: by name, 2 declarations", &rs),
            ),
            (
                "depot.rs",
                "Self::open",
                picker("open: by name, 2 declarations", &rs),
            ),
            (
                "depot.rs",
                "shed.open",
                picker("open: by name, 2 declarations", &rs),
            ),
            (
                "depot.cpp",
                "    Depot::open",
                jump("open \u{2192} Depot::open (via Depot)", "depot.cpp:2"),
            ),
            (
                "depot.cpp",
                "Bare::open",
                picker("open: by name, 4 declarations", &cpp),
            ),
            // A value called like a type, behind a `.`: a `lid`, whatever `crate::open` reads.
            (
                "depot.cpp",
                "crate.open",
                picker("open: by name, 4 declarations", &cpp),
            ),
        ];
        for (file, code, want) in cases {
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{file}: {code}");
        }
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
                (
                    "duo.go",
                    "package arity\n\ntype Duo interface {\n\tCarry(a string) error\n}\n\nfunc (n NotSolo) Carry(a int) string { return \"\" }\n\nfunc (r *Real) Carry(a []byte) error { return nil }\n",
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
        // With no implementation at all, the methods of other types are namesakes to offer.
        d_on(&mut a, "duo.go", "\tCarry");
        let Shown::Picker(status, rows) = shown(&mut a) else {
            panic!("a jump: {}", a.message);
        };
        assert_eq!(status, "Carry: at a declaration, 2 others by name");
        assert_eq!(rows.len(), 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Found by the acceptance pass of #68: two interfaces of one name in two files, and the
    /// class that implements the other one was the implementation `d` jumped to.
    #[test]
    fn an_implementation_of_a_namesake_interface_is_not_one_of_ours() {
        let iface = "export interface Notifier {\n  send(to: string): void;\n}\n";
        let class = |name: &str, from: &str| {
            format!(
                "import {{ Notifier }} from \"./{from}\";\n\nexport class {name} implements Notifier {{\n  send(to: string): void {{}}\n}}\n"
            )
        };
        let looped = class("LoopedNotifier", "loop_a");
        let own = class("OwnNotifier", "own");
        let (mail, sms, push, deep, loose) = (
            class("MailNotifier", "a"),
            class("SmsNotifier", "index"),
            // `type Notifier,` on a line of a wrapped list declares no alias, in the class's
            // file and in the barrel alike.
            class("PushNotifier", "named").replace("{ Notifier }", "{\n  type Notifier,\n}"),
            class("DeepNotifier", "deep"),
            class("LooseNotifier", "renamed"),
        );
        let (dir, mut a) = project_app(
            "dupiface",
            &[
                ("a.ts", iface),
                ("b.ts", iface),
                // Barrels (#100): everything of `a`, the name out of `b`, a barrel of a barrel,
                // and one that hands on another interface under this name, which is not followed.
                ("index.ts", "export * from \"./a\";\n"),
                ("named.ts", "export {\n  type Notifier,\n} from \"./b\";\n"),
                ("deep.ts", "export * from \"./named\";\n"),
                (
                    "things.ts",
                    "export interface Thing {\n  send(to: string): void;\n}\ninterface Notifier {}\n",
                ),
                (
                    "renamed.ts",
                    "export { Thing as Notifier } from \"./things\";\n",
                ),
                // Two barrels that export each other lead nowhere, and end.
                ("loop_a.ts", "export * from \"./loop_b\";\n"),
                ("loop_b.ts", "export * from \"./loop_a\";\n"),
                ("looped.ts", &looped),
                // A barrel that declares a `Notifier` of its own, whatever else it hands on.
                (
                    "own.ts",
                    "export interface Notifier {\n  send(to: string): void;\n}\nexport * from \"./loop_a\";\n",
                ),
                ("own_impl.ts", &own),
                ("impl.ts", &mail),
                ("barrel.ts", &sms),
                ("named_impl.ts", &push),
                ("deep_impl.ts", &deep),
                ("loose.ts", &loose),
            ],
        );
        a.external
            .insert(Kind::TsJs, (Vec::new(), Arc::new(Vec::new())));
        for (file, want) in [
            (
                "a.ts",
                [
                    ("SmsNotifier.send", "barrel.ts:4"),
                    ("MailNotifier.send", "impl.ts:4"),
                    ("LoopedNotifier.send", "looped.ts:4"),
                    ("LooseNotifier.send", "loose.ts:4"),
                ],
            ),
            (
                "b.ts",
                [
                    ("DeepNotifier.send", "deep_impl.ts:4"),
                    ("LoopedNotifier.send", "looped.ts:4"),
                    ("LooseNotifier.send", "loose.ts:4"),
                    ("PushNotifier.send", "named_impl.ts:6"),
                ],
            ),
        ] {
            d_on(&mut a, file, "  send");
            let Shown::Picker(status, rows) = shown(&mut a) else {
                panic!("{}", a.message);
            };
            assert_eq!(
                status,
                "send: implementations of Notifier.send, 4 declarations"
            );
            let rows: Vec<(&str, &str)> = rows
                .iter()
                .map(|(n, _, at)| (n.as_str(), at.as_str()))
                .collect();
            assert_eq!(rows, want, "{file}");
        }
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

    /// A declaration with no namesake is its own answer, as it was before the namesake rule.
    #[test]
    fn a_lone_declaration_stays_where_it_is() {
        let (dir, mut a) = project_app(
            "lonely",
            &[(
                "a.ts",
                "export class A {\n  onlyOne(x: number): number {\n    return x;\n  }\n}\n",
            )],
        );
        a.external
            .insert(Kind::TsJs, (Vec::new(), Arc::new(Vec::new())));
        d_on(&mut a, "a.ts", "  onlyOne");
        assert_eq!(
            shown(&mut a),
            jump("onlyOne \u{2192} A.onlyOne (by name, 1 match)", "a.ts:2")
        );
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
                    "package main\n\nimport (\n\t\"database/sql\"\n\t\"database/sql/pq\"\n\t\"errors\"\n\t\"github.com/foo/bar\"\n\t\"gopkg.in/yaml.v3\"\n)\n\nfunc main() {\n\t_ = yaml.Unmarshal(nil, nil)\n\tbar.Baz()\n\tsql.Open(\"\", \"\")\n\t_ = errors.New(\"\")\n\tpq.Open(\"\")\n}\n",
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
                // A Go package is one directory: `database/sql` is not `database/sql/driver`,
                // and the standard library's `errors` is not a module's.
                (
                    "src/database/sql/sql.go",
                    "package sql\n\nfunc Open(driver, dsn string) {}\n",
                ),
                (
                    "src/database/sql/driver/driver.go",
                    "package driver\n\nfunc Open(name string) {}\n",
                ),
                (
                    "src/errors/errors.go",
                    "package errors\n\nfunc New(text string) error { return nil }\n",
                ),
                (
                    "github.com/pkg/errors@v0.9.1/errors.go",
                    "package errors\n\nfunc New(message string) error { return nil }\n",
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
            (
                "main.go",
                "sql.Open",
                jump(
                    "Open: via import database/sql",
                    &at("src/database/sql/sql.go:3"),
                ),
            ),
            (
                "main.go",
                "errors.New",
                jump("New: via import errors", &at("src/errors/errors.go:3")),
            ),
            // A package that is not installed: its parent directory is no proof.
            (
                "main.go",
                "pq.Open",
                picker(
                    "Open: by name, 2 declarations",
                    &[
                        ("Open", "src/database/sql/driver/driver.go:3"),
                        ("Open", "src/database/sql/sql.go:3"),
                    ],
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

    /// #104. A `self.word` whose class extends one outside the project gets the project's
    /// declarations of the word and never the dependency's, although the dependency declares it.
    #[test]
    fn a_self_word_past_a_base_outside_the_project_stays_in_it() {
        let (dir, mut a) = project_app(
            "own-outside",
            &[(
                "a.py",
                "from threading import Thread\n\n\nclass A(Thread):\n    def run(self):\n        self.stop()\n",
            )],
        );
        let root = external_root(
            "own-outside",
            &[(
                "threading.py",
                "class Thread:\n    def stop(self):\n        pass\n",
            )],
        );
        use_roots(&mut a, Kind::Python, std::slice::from_ref(&root));
        d_on(&mut a, "a.py", "self.stop");
        assert_eq!(shown(&mut a), jump("no definition for stop", "a.py:6"));
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// #100. A grep that stopped at the cap counts a lower bound also after a filter has made
    /// the list short: the one top-level `Pick` left of a cut list is offered with a `+`, not
    /// jumped to as the package's only one.
    #[test]
    fn a_count_behind_a_cut_grep_says_so() {
        let (dir, mut a) = project_app(
            "cut-count",
            &[(
                "main.go",
                "package main\n\nimport \"example.com/lib\"\n\nfunc main() {\n\tlib.Pick()\n}\n",
            )],
        );
        let methods = "func (o Row) Pick() {}\n".repeat(search::MAX_HITS);
        let root = external_root(
            "cut-count",
            &[(
                "example.com/lib@v1.0.0/lib.go",
                &format!("package lib\n\nfunc Pick() {{}}\n\n{methods}"),
            )],
        );
        use_roots(&mut a, Kind::Go, std::slice::from_ref(&root));
        d_on(&mut a, "main.go", "lib.Pick");
        assert_eq!(
            shown(&mut a),
            Shown::Picker(
                "Pick: via import example.com/lib, 1+ declarations".into(),
                vec![(
                    "Pick".into(),
                    "via import example.com/lib".into(),
                    "example.com/lib@v1.0.0/lib.go:3".into()
                )],
            )
        );
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// A cut in a grep whose hits are all dropped (`x.String` looked for as `x`'s own `String`)
    /// does not mark the members that are then listed: one match is a jump, as it was.
    #[test]
    fn a_cut_grep_that_is_dropped_marks_nothing() {
        let plain = "func String() {}\n".repeat(search::MAX_HITS);
        let (dir, mut a) = project_app(
            "cut-dropped",
            &[
                ("a/many.go", &format!("package a\n\n{plain}")),
                (
                    "main.go",
                    "package main\n\ntype T struct{}\n\nfunc (t T) String() {}\n\nfunc main() {\n\tx.String()\n}\n",
                ),
            ],
        );
        a.external
            .insert(Kind::Go, (Vec::new(), Arc::new(Vec::new())));
        d_on(&mut a, "main.go", "x.String");
        assert_eq!(
            shown(&mut a),
            jump("String \u{2192} T.String (by name, 1 match)", "main.go:5")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// #104. The field lines of a name are grepped apart from its methods: a file of object-literal
    /// keys that fills the grep leaves the method a candidate, and the count says the field side
    /// was cut, so a single candidate is offered rather than jumped to.
    #[test]
    fn a_field_grep_cut_at_the_cap_keeps_the_methods_and_says_so() {
        let keys = format!(
            "export const rows = {{\n{}}};\n",
            "  id: 1,\n".repeat(search::MAX_HITS)
        );
        let (dir, mut a) = project_app(
            "field-cap",
            &[
                ("a/rows.ts", keys.as_str()),
                (
                    "z/model.ts",
                    "export class Model {\n  id(): number {\n    return 1;\n  }\n}\n",
                ),
                (
                    "main.ts",
                    "export function f(u: any): number {\n  return u.id();\n}\n",
                ),
            ],
        );
        for kind in [Kind::Python, Kind::TsJs, Kind::Go] {
            a.external.insert(kind, (Vec::new(), Arc::new(Vec::new())));
        }
        d_on(&mut a, "main.ts", "u.id");
        assert_eq!(
            shown(&mut a),
            picker(
                "id: by name, 1+ declarations",
                &[("Model.id", "z/model.ts:2")]
            )
        );
        std::fs::remove_dir_all(&dir).unwrap();
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
                None,
                false
            ),
            "load \u{2192} json.load (via import json)"
        );
        let mut two = one(Reason::ByName);
        two.extend(one(Reason::Path("std::fs".into())));
        assert_eq!(
            resolution("read", None, &two, None, false),
            "read: 2 declarations"
        );
        // The candidates stop at MAX_HITS: the count is a lower bound.
        let many: Vec<Candidate> = (0..search::MAX_HITS)
            .flat_map(|_| one(Reason::ByName))
            .collect();
        assert_eq!(
            resolution("save", None, &many, None, false),
            format!("save: by name, {}+ declarations", search::MAX_HITS)
        );
        // So is a list a search cut short, one candidate or more.
        assert_eq!(
            resolution("id", None, &one(Reason::ByName), None, true),
            "id: by name, 1+ declarations"
        );
        assert_eq!(
            resolution("read", None, &[], None, false),
            "no definition for read"
        );
        // A chain in front of the word that could not be followed says where it broke.
        assert_eq!(
            resolution(
                "remove",
                Some("UserService.remove"),
                &one(Reason::ByName),
                Some("users"),
                false
            ),
            "remove \u{2192} UserService.remove (by name, 1 match, chain broke at users)"
        );
        assert_eq!(
            resolution("read", None, &[], Some("repo"), false),
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
                // C is read from its own rows: `struct` must not be listed by the shared
                // pattern as well, and a prototype is not a symbol.
                (
                    "invoice.h",
                    "#define LIMIT 10\nstruct invoice {\n    int total;\n};\nint sum(struct invoice *i);\n",
                ),
                // Lua from its own rows too: the shared pattern reads `function M.setup(` as a
                // declaration of `M`.
                (
                    "init.lua",
                    "local M = {}\n\nfunction M.setup(opts)\n  return opts\nend\n",
                ),
                // Elixir from its own rows: the shared pattern knows `def` and nothing else of
                // the family, and `defp` would be missing.
                (
                    "ledger.ex",
                    "defmodule Ledger do\n  @timeout 5\n\n  defp normalise(raw), do: raw\nend\n",
                ),
                // Zig keeps the shared pattern and complements it: `pub fn` comes from there,
                // the test from a row of its own.
                (
                    "ledger.zig",
                    "pub fn total() u32 {\n    return 0;\n}\n\ntest \"it adds up\" {}\n",
                ),
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
        assert_eq!(
            names,
            [
                "&base",
                "build",
                "build",
                "invoice",
                // The first word of `it adds up`, the Zig test's description.
                "it",
                "Ledger",
                "LIMIT",
                "normalise",
                "serve",
                "setup",
                "total",
                "var.region"
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A project past the cap: `a.go` fills the list on its own, so the walk stops before
    /// `z.go` and `zebra` is behind the cut. `main.tf` is there for a name with a dot in it.
    fn capped_project(tag: &str) -> (PathBuf, App) {
        let many: String = (0..search::MAX_HITS)
            .map(|i| format!("func a{i}() {{}}\n"))
            .collect();
        project_app(
            tag,
            &[
                ("a.go", &many),
                ("main.tf", "variable \"region\" {}\n"),
                ("z.go", "func zebra() {}\nfunc Zebra() {}\n"),
            ],
        )
    }

    /// Past the cap the rows are the declarations found before it, not the project's, so the
    /// query greps for a name instead of filtering them: what the cut never reached is found by
    /// typing it, in either case. An answer to a query that has changed since is dropped, and an
    /// emptied query brings the opening list back. Under the cap nothing of this happens.
    #[test]
    fn symbols_past_the_cap_are_grepped_not_filtered() {
        let (small, mut b) = project_app("cap-under", &[("z.go", "func zebra() {}\n")]);
        press(&mut b, KeyCode::Char('D'), KeyModifiers::NONE);
        let p = b.picker.as_ref().unwrap();
        assert_eq!(
            (p.title.as_str(), p.live),
            ("Symbols", false),
            "the whole list"
        );
        std::fs::remove_dir_all(&small).unwrap();

        let (dir, mut a) = capped_project("cap-symbols");
        // A grep of the search that was open answers to a number `D` must not be waiting for.
        press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
        typed(&mut a, "zebra");
        std::thread::sleep(SEARCH_PAUSE);
        let of_the_search = a.search_tick().expect("the grep for the `s` query").seq;
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);

        press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
        let p = a.picker.as_mut().unwrap();
        p.settle();
        assert_eq!(
            p.counts().1 as usize,
            search::MAX_HITS + 1,
            "a.go fills the cap of the shared pattern; main.tf is another pattern's row"
        );
        assert!(
            p.live,
            "the query greps the project, it does not filter these rows"
        );
        assert!(
            !a.search_done(of_the_search, App::hit_items(Vec::new())),
            "the search's answer is not the symbol list"
        );

        typed(&mut a, "zebr");
        std::thread::sleep(SEARCH_PAUSE);
        let stale = a.search_tick().expect("the grep for zebr").seq;
        typed(&mut a, "a");
        assert!(!a.search_done(stale, Vec::new()), "zebr is not on screen");
        let cut = a.picker.as_ref().unwrap().counts().1 as usize;
        assert_eq!(cut, search::MAX_HITS + 1, "a stale answer settles nothing");

        // Smart case, as everywhere else: an all-lowercase query finds both spellings.
        a.settle_search();
        let p = a.picker.as_mut().unwrap();
        assert_eq!(p.counts(), (2, 2));
        let rows = p.window(5).0;
        let mut labels: Vec<&str> = rows.iter().map(|r| r.item.label.as_str()).collect();
        labels.sort_unstable();
        assert_eq!(labels, ["Zebra  z.go:2", "zebra  z.go:1"]);
        // nucleo ranks and marks what the grep brought back, as it does under the cap.
        assert_eq!(rows[0].matched, [0, 1, 2, 3, 4]);

        // One capital of its own makes the query case-sensitive, before the picker sees it.
        press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
        typed(&mut a, "Zebra");
        a.settle_search();
        let p = a.picker.as_mut().unwrap();
        assert_eq!(p.counts(), (1, 1));
        assert_eq!(p.window(5).0[0].item.label, "Zebra  z.go:2");

        press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
        a.settle_search();
        let p = a.picker.as_ref().unwrap();
        assert_eq!(
            p.counts().1 as usize,
            search::MAX_HITS + 1,
            "the list it opened on"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The cap belongs to each declaration pattern, not to the list: a project whose kinds add
    /// up past it with none of them cut has its whole list, and the query filters it as ever.
    #[test]
    fn a_list_no_pattern_cut_short_is_the_whole_list() {
        let half = search::MAX_HITS / 2 + 100;
        let go: String = (0..half).map(|i| format!("func a{i}() {{}}\n")).collect();
        let rb: String = (0..half).map(|i| format!("def b{i}\nend\n")).collect();
        let (dir, mut a) = project_app("cap-two-kinds", &[("a.go", &go), ("b.rb", &rb)]);
        press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
        let p = a.picker.as_mut().unwrap();
        p.settle();
        assert!(2 * half > search::MAX_HITS, "past the cap in total");
        assert_eq!(p.counts().1 as usize, 2 * half, "both kinds, whole");
        assert_eq!(
            (p.title.as_str(), p.live),
            ("Symbols", false),
            "nothing was cut, so the query still filters the rows"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The query past the cap is a name to find, not a pattern: a regex would read `a1.*` as
    /// a thousand of the names in `a.go`, and an escaped one would look for a backslash in
    /// `var.region`.
    #[test]
    fn a_symbol_query_is_literal_not_a_regex() {
        let (dir, mut a) = capped_project("cap-regex");
        press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
        for (query, hits, why) in [
            ("a1.*", 0, "a dot and a star are characters of a name"),
            (
                "var.region",
                1,
                "and a name written with a dot is typed with it",
            ),
        ] {
            typed(&mut a, query);
            a.settle_search();
            let p = a.picker.as_ref().expect(why);
            assert_eq!(p.counts().1, hits, "{why}: {}", a.message);
            press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
            a.settle_search();
        }
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
    fn find_reopens_with_the_active_query_selected() {
        let mut a = app("now\nfunc now\nx\n");
        find(&mut a, "now");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
        assert_eq!((&*a.prompt, a.prompt.selection()), ("now", Some(0..3)));
        // Home edits the old query; the search follows.
        press(&mut a, KeyCode::Home, KeyModifiers::NONE);
        typed(&mut a, "func ");
        assert_eq!((&*a.prompt, a.line), ("func now", 1));
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        // Typing replaces it whole.
        find(&mut a, "x");
        assert_eq!((&*a.prompt, a.line), ("x", 2));
        // Esc in navigation clears the pattern, so the next `/` opens empty.
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
        assert_eq!(&*a.prompt, "");
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
        assert_eq!(a.message, "2/2");
        press(&mut a, KeyCode::Char('n'), KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (0, 0));
        assert_eq!(a.message, "1/2");

        press(&mut a, KeyCode::Char('N'), KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (2, 0));
        assert_eq!(a.message, "2/2");
        press(&mut a, KeyCode::Char('N'), KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (0, 0));
        assert_eq!(a.message, "1/2");

        let mut a = app("nothing here\n");
        find(&mut a, "zzz");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('n'), KeyModifiers::NONE);
        assert_eq!(a.message, "no match");
    }

    /// #82: a key that cannot act says why, so "not found" never reads as "not pressed".
    #[test]
    fn no_key_is_silent_on_an_empty_line() {
        let mut a = app("\nfoo\n");
        for key in ['d', 'u', 'D', 'n', 'N', '[', ']', 'c', 'C'] {
            press(&mut a, KeyCode::Char(key), KeyModifiers::NONE);
            assert!(!a.message.is_empty(), "`{key}` said nothing");
            assert_eq!(a.mode, Mode::Normal, "`{key}`");
        }
        assert_eq!(a.message, "not in review mode");
        // Esc with nothing to clear no longer claims `find cleared`.
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(a.message, "");
        // `/` typing a query the file does not have, and one it has.
        find(&mut a, "zzz");
        assert_eq!(a.message, "no match");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(a.message, "");
        find(&mut a, "foo");
        assert_eq!(a.message, "1/1");
        // Reopened with the query still active, the count is there before a key is typed.
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
        assert_eq!(a.message, "1/1");
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

    /// A project on disk, `src/a.py` and `README.md`, walked as `main` walks it; `src/a.py` open.
    fn new_file_project(name: &str) -> (PathBuf, App) {
        let dir = std::env::temp_dir().join(format!("merl-new-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/a.py"), "x = 1\n").unwrap();
        std::fs::write(dir.join("README.md"), "hi\n").unwrap();
        let dir = dir.canonicalize().unwrap();
        let (tree, files) = crate::tree::build(&dir);
        let buf = Buffer::load(&dir.join("src/a.py")).unwrap();
        let mut a = App::new(dir.clone(), tree, files, buf, None);
        a.focus = Focus::Code;
        a.view_w = 40;
        a.view_h = 10;
        (dir, a)
    }

    fn ctrl_n(a: &mut App) {
        press(a, KeyCode::Char('n'), KeyModifiers::CONTROL);
    }

    #[test]
    fn ctrl_n_creates_a_file_next_to_the_open_one_and_edits_it() {
        let (dir, mut a) = new_file_project("code");
        ctrl_n(&mut a);
        assert_eq!((a.mode, &*a.prompt), (Mode::New, "src/"));
        typed(&mut a, "sub/b.py");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        let made = dir.join("src/sub/b.py");
        assert_eq!(std::fs::read_to_string(&made).unwrap(), "");
        assert_eq!((a.buf.path.as_deref(), a.mode), (Some(&*made), Mode::Edit));
        // In the file list and the tree at once, with no watcher to wait for.
        let rel = Path::new("src/sub/b.py");
        assert!(a.files.iter().any(|f| f == rel));
        assert_eq!(a.tree.selected().map(|n| &*n.path), Some(rel));
        // `[` is the way back.
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(a.buf.path.as_deref(), Some(&*dir.join("src/a.py")));
    }

    #[test]
    fn ctrl_n_in_the_tree_starts_from_the_selected_row() {
        let (_, mut a) = new_file_project("tree");
        a.focus = Focus::Tree;
        a.tree.reveal(Path::new("README.md"));
        ctrl_n(&mut a);
        assert_eq!(&*a.prompt, "");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        a.tree.reveal(Path::new("src/a.py"));
        ctrl_n(&mut a);
        assert_eq!(&*a.prompt, "src/");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        a.tree.reveal(Path::new("src"));
        ctrl_n(&mut a);
        assert_eq!(&*a.prompt, "src/");
        typed(&mut a, "c.py");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!((a.focus, a.mode), (Focus::Code, Mode::Edit));
    }

    #[test]
    fn ctrl_n_from_edit_mode_saves_the_open_file_and_esc_resumes_it() {
        let (dir, mut a) = new_file_project("edit");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "y");
        ctrl_n(&mut a);
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(a.mode, Mode::Edit);
        ctrl_n(&mut a);
        typed(&mut a, "b.py");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            std::fs::read_to_string(dir.join("src/a.py")).unwrap(),
            "yx = 1\n"
        );
        assert_eq!(a.buf.path.as_deref(), Some(&*dir.join("src/b.py")));
    }

    #[test]
    fn ctrl_n_never_overwrites_and_never_leaves_the_project() {
        let (dir, mut a) = new_file_project("refuse");
        let new = |a: &mut App, path: &str| {
            ctrl_n(a);
            press(a, KeyCode::Char('u'), KeyModifiers::CONTROL);
            typed(a, path);
            press(a, KeyCode::Enter, KeyModifiers::NONE);
        };
        // A file that is there opens as it is.
        new(&mut a, "README.md");
        assert_eq!(a.buf.path.as_deref(), Some(&*dir.join("README.md")));
        assert_eq!(
            std::fs::read_to_string(dir.join("README.md")).unwrap(),
            "hi\n"
        );
        for (path, why) in [
            ("../out.py", "outside the project"),
            ("/tmp/merl-out.py", "outside the project"),
            ("src/../../out.py", "outside the project"),
            ("src/", "no file name"),
            ("src", "src is a directory"),
        ] {
            new(&mut a, path);
            assert_eq!(a.message, why, "{path}");
        }
        assert!(!dir.parent().unwrap().join("out.py").exists());
        // An empty prompt is a cancel, as in `:`.
        new(&mut a, "");
        assert_eq!(a.message, "");
    }

    #[test]
    fn ctrl_n_over_an_overlay_does_nothing() {
        let (_, mut a) = new_file_project("overlay");
        for open in ['o', ':', '/', '?'] {
            press(&mut a, KeyCode::Char(open), KeyModifiers::NONE);
            ctrl_n(&mut a);
            assert_ne!(a.mode, Mode::New, "{open}");
            press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        }
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
        assert_eq!(
            a.mode,
            Mode::Picker(PickerKind::Search),
            "letters navigate outside edit mode"
        );
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
    fn alt_backspace_and_alt_delete_take_a_word_in_one_undo_step() {
        let mut a = app("x_1 = да мир;\nnext\n");
        press(&mut a, KeyCode::End, KeyModifiers::NONE);
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "!");
        press(&mut a, KeyCode::Backspace, KeyModifiers::ALT);
        assert_eq!(a.buf.lines[0], "x_1 = да ");
        // The word alone comes back, with the cursor where the key was pressed.
        press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(
            (a.buf.lines[0].as_str(), a.col),
            ("x_1 = да мир;!", "x_1 = да мир;!".len())
        );
        press(&mut a, KeyCode::Home, KeyModifiers::NONE);
        press(&mut a, KeyCode::Delete, KeyModifiers::ALT);
        press(&mut a, KeyCode::Delete, KeyModifiers::ALT);
        assert_eq!(a.buf.lines[0], " мир;!");
        // At the start of a line it joins the line above, as Alt+Left goes there.
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
        press(&mut a, KeyCode::Home, KeyModifiers::NONE);
        press(&mut a, KeyCode::Backspace, KeyModifiers::ALT);
        assert_eq!(a.buf.lines, vec![" мир;!next"]);
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
        assert_eq!(a.message, "copied 2 lines");
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
    fn overlays_opened_while_editing_go_back_to_editing() {
        let mut a = app("one\ntwo\nthree two\n");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        // Find, Enter: editing goes on at the match.
        press(&mut a, KeyCode::Char('f'), KeyModifiers::CONTROL);
        typed(&mut a, "two");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!((a.mode, a.line), (Mode::Edit, 1));
        typed(&mut a, "d");
        assert_eq!(a.buf.lines[1], "dtwo");
        // Find, Esc: the cursor goes back, and so does the mode.
        press(&mut a, KeyCode::Char('f'), KeyModifiers::CONTROL);
        typed(&mut a, "three");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!((a.mode, a.line, a.col), (Mode::Edit, 1, 1));
        // Goto and a cancelled prompt.
        press(&mut a, KeyCode::Char('g'), KeyModifiers::CONTROL);
        typed(&mut a, "3");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!((a.mode, a.line), (Mode::Edit, 2));
        press(&mut a, KeyCode::Char('g'), KeyModifiers::CONTROL);
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(a.mode, Mode::Edit);
        // Opened from navigation, find still ends in navigation.
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('/'), KeyModifiers::NONE);
        typed(&mut a, "one");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!((a.mode, a.line), (Mode::Normal, 0));
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
        assert!(press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
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
        let dropped = a.line_str().to_string();
        std::fs::write(&path, "theirs\n").unwrap();
        a.reload(false);
        assert!(a.conflict);
        press(&mut a, KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert_eq!(
            (a.conflict, a.dirty, a.line_str()),
            (false, false, "theirs")
        );
        // #122: the reload is a step, so the edits Ctrl+R dropped are one Ctrl+Z away, and then
        // they are edits like any other: the autosave writes them over the disk's version.
        press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!((a.dirty, a.line_str()), (true, dropped.as_str()));
        assert!(a.tick());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            format!("{dropped}\n")
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

    /// README: a file that changes on disk under unsaved edits is "neither reloaded nor
    /// overwritten". Deleted or renamed is changed: only Ctrl+S puts it back.
    #[test]
    fn autosave_does_not_recreate_a_file_that_is_gone() {
        let (path, mut a) = temp_file("gone-save", "one\n");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "X");
        std::fs::remove_file(&path).unwrap();
        a.last_edit = Some(Instant::now() - a.autosave);
        assert!(a.tick());
        assert!(!a.flush());
        assert!(!path.exists());
        assert!(a.conflict && a.dirty);
        press(&mut a, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "Xone\n");
        assert!(!a.conflict && !a.dirty);
        // Back as merl last saw it (a writer's delete-then-write) is no conflict any more.
        typed(&mut a, "Y");
        std::fs::remove_file(&path).unwrap();
        a.save();
        assert!(a.conflict);
        std::fs::write(&path, "Xone\n").unwrap();
        a.reload(false);
        assert!(!a.conflict && a.dirty);
        // Ctrl+R has no disk version to take; the edits go all the same, or nothing but
        // writing the file back would let merl off it.
        std::fs::remove_file(&path).unwrap();
        a.save();
        press(&mut a, KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert!(!a.conflict && !a.dirty && !path.exists());
        assert!(a.flush());
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

        // Outside a review the cursor keeps its line number when lines are written above it.
        a.line = 3;
        std::fs::write(&path, "new\n".to_string() + &"line\n".repeat(10)).unwrap();
        a.reload(false);
        assert_eq!(a.line, 3);
        a.line = 9;

        std::fs::write(&path, "a\nb\nc\n").unwrap();
        a.reload(false);
        assert_eq!(a.buf.lines.len(), 3);
        assert_eq!(a.line, 2);
        assert_eq!(a.col, 1);
        assert_eq!(a.message, "reloaded");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn ctrl(a: &mut App, c: char) {
        press(a, KeyCode::Char(c), KeyModifiers::CONTROL);
    }

    /// #122, the issue's steps: a write from outside is one step of the history, as in VS Code.
    /// Ctrl+Z takes it back first, then the edits before it, and Ctrl+Y replays both.
    #[test]
    fn a_reload_is_an_undo_step_over_the_edits_before_it() {
        let (path, mut a) = temp_file("undo-reload", "one\ntwo\n");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "MINE");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        std::fs::write(&path, "MINEone\ntwo\nagent line\n").unwrap();
        assert!(a.reload(false));
        assert_eq!(a.message, "reloaded");
        let step = a.undo.last().unwrap();
        assert_eq!(
            (step.line, step.old.len(), step.new.len()),
            (2, 0, 1),
            "the step holds what changed, not the file"
        );
        ctrl(&mut a, 'z');
        assert_eq!(
            (a.buf.lines.join("|"), a.line),
            ("MINEone|two".into(), 1),
            "undo lands where the file changed"
        );
        ctrl(&mut a, 'z');
        assert_eq!(a.buf.lines.join("|"), "one|two");
        ctrl(&mut a, 'y');
        ctrl(&mut a, 'y');
        assert_eq!(a.buf.lines.join("|"), "MINEone|two|agent line");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    /// #163, the issue's steps: a file left keeps its history, as a VS Code tab does. Written on
    /// disk while away, it comes back with the write as one more step on top.
    #[test]
    fn leaving_a_file_keeps_its_undo_history() {
        let (dir, mut a) = project_app(
            "undo-away",
            &[
                ("a.py", "x = 1\nhelper()\n"),
                ("b.py", "def helper():\n    pass\n"),
            ],
        );
        a.external
            .insert(Kind::Python, (Vec::new(), Arc::new(Vec::new())));
        let away_and_back = |a: &mut App| {
            a.jump_to(&dir.join("a.py"), 2);
            press(a, KeyCode::Char('d'), KeyModifiers::NONE);
            assert_eq!(at(a), (dir.join("b.py"), 0), "{}", a.message);
            press(a, KeyCode::Char('['), KeyModifiers::NONE);
            assert_eq!(at(a).0, dir.join("a.py"));
        };
        a.jump_to(&dir.join("a.py"), 1);
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        typed(&mut a, "MINE");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        away_and_back(&mut a);
        ctrl(&mut a, 'z');
        assert_eq!(a.buf.lines.join("|"), "x = 1|helper()");
        away_and_back(&mut a);
        ctrl(&mut a, 'y');
        assert_eq!(a.buf.lines.join("|"), "MINEx = 1|helper()", "redo too");
        a.jump_to(&dir.join("b.py"), 1);
        std::fs::write(dir.join("a.py"), "MINEx = 1\nhelper()\nagent line\n").unwrap();
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        ctrl(&mut a, 'z');
        assert_eq!(a.buf.lines.join("|"), "MINEx = 1|helper()");
        ctrl(&mut a, 'z');
        assert_eq!(a.buf.lines.join("|"), "x = 1|helper()");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// What is typed after a reload is a step of its own, also where the reload's step ends.
    #[test]
    fn typing_after_a_reload_is_a_step_of_its_own() {
        let (path, mut a) = temp_file("undo-reload-typing", "one\ntwo\n");
        a.autosave = Duration::ZERO;
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert!(a.tick());
        std::fs::write(&path, "\nONE\ntwo\n").unwrap();
        a.reload(false);
        assert_eq!((a.line, a.col), (1, 0));
        typed(&mut a, "x");
        ctrl(&mut a, 'z');
        assert_eq!(a.buf.lines.join("|"), "|ONE|two");
        ctrl(&mut a, 'z');
        assert_eq!(a.buf.lines.join("|"), "|one|two");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    /// Undoing a reload puts the file's format back with its text. A reload to what no save can
    /// write back as it came is undone, never redone; one from it starts a new history.
    #[test]
    fn undoing_a_reload_puts_the_format_back() {
        let (path, mut a) = temp_file("undo-format", "a\r\nb\r\n");
        a.autosave = Duration::ZERO;
        std::fs::write(&path, "a\nb\nc").unwrap();
        a.reload(false);
        ctrl(&mut a, 'z');
        assert_eq!(
            a.buf.to_bytes(),
            b"a\r\nb\r\n",
            "CRLF and the final newline"
        );
        ctrl(&mut a, 'y');
        assert_eq!(a.buf.to_bytes(), b"a\nb\nc");
        assert!(a.tick());
        std::fs::write(&path, "a\r\nb\nc\r\n").unwrap();
        a.reload(false);
        assert_eq!(a.buf.readonly, Some("mixed line endings"));
        ctrl(&mut a, 'z');
        assert_eq!(
            (a.buf.readonly, a.buf.to_bytes()),
            (None, b"a\nb\nc".to_vec())
        );
        ctrl(&mut a, 'y');
        assert_eq!(a.message, "nothing to redo");
        assert!(a.tick());
        std::fs::write(&path, "\0").unwrap();
        a.reload(false);
        std::fs::write(&path, "t\n").unwrap();
        a.reload(false);
        ctrl(&mut a, 'z');
        assert_eq!(a.message, "nothing to undo");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    /// A dependency changed on disk (a `pip install -U`) or reloaded with Ctrl+R stays read-only.
    #[test]
    fn a_reload_keeps_a_file_outside_the_project_read_only() {
        let (path, mut a) = temp_file("reload-outside", "x\n");
        let outside = std::env::temp_dir().join(format!("merl-dep-{}.py", std::process::id()));
        std::fs::write(&outside, "def x(): pass\n").unwrap();
        a.jump_to(&outside, 1);
        std::fs::write(&outside, "def y(): pass\n").unwrap();
        a.reload(false);
        assert_eq!(a.buf.readonly, Some("outside the project"));
        std::fs::remove_file(&outside).unwrap();
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    /// Every alias in `KEYS` reaches the same action as its primary key.
    #[test]
    fn aliases_reach_the_same_actions() {
        let mut a = app("foo bar\n");
        // Not on disk: the grep for usages has nothing to read.
        std::fs::remove_file(a.buf.path.as_ref().unwrap()).unwrap();
        press(&mut a, KeyCode::F(12), KeyModifiers::NONE);
        assert_eq!(a.message, "no rules for .txt");
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

    /// Edit mode is not an overlay for an Esc to close: Option+letter mid-word must not drop
    /// into navigation, where the rest of the word runs as commands and its `q` quits.
    #[test]
    fn an_unbound_alt_letter_while_editing_is_ignored() {
        let mut a = app("foo\n");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('p'), KeyModifiers::ALT);
        assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::ALT));
        assert!(!press(&mut a, KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!((a.mode, a.buf.lines[0].as_str()), (Mode::Edit, "qfoo"));
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
            a.settle_search();
            let p = a.picker.as_ref().expect(why);
            assert_eq!(p.counts().1, hits, "{why}: {}", a.message);
            press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        }
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    /// The hits follow the query. An answer to a query that has changed since is dropped, and
    /// the cursor stays on its hit while the new query still finds it.
    #[test]
    fn project_search_refreshes_while_typing() {
        let (path, mut a) = temp_file(
            "live-s",
            "foo
food
foo
",
        );
        press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
        assert!(!a.search_pending(), "nothing asked yet");
        typed(&mut a, "foo");
        assert!(
            a.search_tick().is_none(),
            "the grep waits for a pause in the typing"
        );
        assert!(a.search_pending(), "pending through the pause");
        std::thread::sleep(SEARCH_PAUSE);
        let stale = a.search_tick().expect("the grep for foo").seq;
        assert!(a.search_pending(), "and while the grep runs");
        typed(&mut a, "d");
        assert!(!a.search_done(stale, Vec::new()), "foo is not on screen");
        assert!(a.search_pending(), "a stale answer settles nothing");
        a.settle_search();
        assert!(!a.search_pending());
        assert_eq!(a.picker.as_ref().unwrap().counts().1, 1);

        press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
        a.settle_search();
        let p = a.picker.as_ref().unwrap();
        assert_eq!((p.counts().1, p.current().unwrap().line), (3, 2));
        // An emptied query empties the list at once.
        press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL);
        assert!(a.search_tick().is_none());
        a.settle_search();
        assert_eq!(a.picker.as_ref().unwrap().counts().1, 0);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    /// Enter before the hits of the query on screen: the jump waits for them.
    #[test]
    fn enter_waits_for_the_project_search() {
        let (path, mut a) = temp_file(
            "enter-s", "one
two
",
        );
        press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
        typed(&mut a, "two");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert!(a.picker.is_some(), "nothing to jump to yet");
        a.settle_search();
        assert_eq!(
            (a.picker.is_none(), a.mode, a.line),
            (true, Mode::Normal, 1)
        );

        press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
        typed(&mut a, "three");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        a.settle_search();
        assert_eq!(
            (a.picker.is_none(), &*a.message),
            (true, "no results for three")
        );

        // Past the pause, with the grep running: the list on screen is still the older one.
        press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
        typed(&mut a, "one");
        std::thread::sleep(SEARCH_PAUSE);
        let job = a.search_tick().expect("the grep for one");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert!(a.picker.is_some(), "the grep for one is still running");
        a.search_done(job.seq, job.items());
        assert_eq!((a.picker.is_none(), a.line), (true, 0));
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    /// Enter once the hits are in: the same jump, and the same word for a query that found
    /// nothing, as an Enter that came before them. An empty query closes without a word.
    #[test]
    fn enter_after_the_project_search_answered() {
        let (path, mut a) = temp_file("enter-late-s", "one\ntwo\n");
        // The very next key after the answer, before the event loop has ticked or drawn.
        press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
        typed(&mut a, "two");
        std::thread::sleep(SEARCH_PAUSE);
        let job = a.search_tick().expect("the grep for two");
        a.search_done(job.seq, job.items());
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            (a.picker.is_none(), a.mode, a.line, &*a.message),
            (true, Mode::Normal, 1, "")
        );

        press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
        typed(&mut a, "three");
        a.settle_search();
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            (a.picker.is_none(), &*a.message),
            (true, "no results for three")
        );

        for keys in ["", "x\u{15}"] {
            a.message.clear();
            press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
            for c in keys.chars() {
                match c {
                    '\u{15}' => press(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL),
                    c => press(&mut a, KeyCode::Char(c), KeyModifiers::NONE),
                };
            }
            press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
            assert_eq!(
                (a.picker.is_none(), a.mode, &*a.message),
                (true, Mode::Normal, ""),
                "{keys:?}"
            );
        }
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    /// The grep stops at MAX_HITS. The open file's hits sort to the top, so they are read first
    /// and never the ones cut, even when other files alone fill the cap.
    #[test]
    fn the_cap_never_cuts_the_open_files_hits() {
        let dir = std::env::temp_dir().join(format!("merl-cap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.py"), "x\n".repeat(search::MAX_HITS)).unwrap();
        std::fs::write(dir.join("z.py"), "one\nx\n").unwrap();
        let files = vec![PathBuf::from("a.py"), PathBuf::from("z.py")];
        let buf = Buffer::load(&dir.join("z.py")).unwrap();
        let mut a = App::new(dir.clone(), Tree::default(), files, buf, None);
        press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
        typed(&mut a, "x");
        a.settle_search();
        let p = a.picker.as_ref().unwrap();
        let top = p.current().unwrap();
        assert_eq!((top.path.as_path(), top.line), (Path::new("z.py"), 2));
        assert_eq!(p.counts().1 as usize, search::MAX_HITS);
        std::fs::remove_dir_all(&dir).unwrap();
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

    /// An agent renames a file: its stops are dropped and `[` goes on to the next one that
    /// opens, instead of retrying the dead stop with everything behind it out of reach.
    #[test]
    fn a_stop_whose_file_is_gone_is_dropped_and_walked_past() {
        let (dir, mut a) = files_app("gone");
        let (x, y, z) = (dir.join("a.rs"), dir.join("b.rs"), dir.join("c.rs"));
        std::fs::write(&z, "x\n".repeat(40)).unwrap();
        a.jump_to(&z, 3);
        a.jump_to(&x, 5);
        a.jump_to(&x, 30);
        a.jump_to(&y, 1);
        std::fs::remove_file(&x).unwrap();
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(
            (at(&a), a.hist_idx, a.history.len()),
            ((z.clone(), 2), 0, 2)
        );
        assert_eq!(a.message, "a.rs gone");
        press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
        assert_eq!(
            (at(&a), a.hist_idx, a.message.as_str()),
            ((y.clone(), 0), 1, "")
        );
        // `]` drops them the same way, two files at once; a walk that runs off the end says
        // what it dropped first, and then that this is the end.
        let w = dir.join("d.rs");
        for p in [&x, &w] {
            std::fs::write(p, "x\n".repeat(40)).unwrap();
            a.jump_to(p, 9);
        }
        a.jump_to(&y, 20);
        for _ in 0..3 {
            press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        }
        assert_eq!(
            (at(&a), a.hist_idx, a.history.len()),
            ((y.clone(), 0), 1, 5)
        );
        std::fs::remove_file(&x).unwrap();
        std::fs::remove_file(&w).unwrap();
        press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
        assert_eq!(
            (at(&a), a.hist_idx, a.history.len()),
            ((y.clone(), 19), 2, 3)
        );
        assert_eq!(a.message, "2 files gone");
        std::fs::remove_file(&z).unwrap();
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(
            (at(&a), a.hist_idx, a.history.len()),
            ((y.clone(), 0), 0, 2)
        );
        assert_eq!(a.message, "c.rs gone");
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(a.message, "start of history");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// b, a, b with `a` gone would leave `b` next to itself: a press that moves nothing.
    #[test]
    fn dropping_a_stop_does_not_leave_its_neighbours_as_twins() {
        let (dir, mut a) = files_app("twins");
        let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
        a.jump_to(&y, 1);
        a.jump_to(&x, 5);
        a.jump_to(&y, 1);
        std::fs::remove_file(&x).unwrap();
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!((at(&a), a.hist_idx, a.history.len()), ((y, 0), 0, 1));
        assert_eq!(a.message, "a.rs gone");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Edits that cannot be saved keep merl on the file; that is no reason to drop a stop.
    #[test]
    fn a_stop_that_does_not_open_for_another_reason_leaves_the_history_alone() {
        let (dir, mut a) = files_app("stuck");
        let (x, y) = (dir.join("a.rs"), dir.join("b.rs"));
        a.jump_to(&x, 5);
        a.jump_to(&y, 1);
        (a.dirty, a.conflict) = (true, true);
        std::fs::remove_file(&x).unwrap();
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!(
            (at(&a), a.hist_idx, a.history.len()),
            ((y.clone(), 0), 1, 2)
        );
        // Nor is a file that is there and does not open.
        (a.dirty, a.conflict) = (false, false);
        std::fs::create_dir(&x).unwrap();
        press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
        assert_eq!((at(&a), a.hist_idx, a.history.len()), ((y, 0), 1, 2));
        assert!(a.message.contains("a.rs: "), "{}", a.message);
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
        // The split counts what the list holds, and the cut still says so behind it.
        assert_eq!(
            p.title,
            format!(
                "Usages of x: {} in code (first {})",
                search::MAX_HITS,
                search::MAX_HITS
            )
        );
        assert_eq!(p.counts().1 as usize, search::MAX_HITS);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The rows of the open result picker, each split into its mark and the `path:line` it
    /// points at.
    fn usage_rows(a: &mut App) -> Vec<(String, String)> {
        let picker = a.picker.as_mut().expect("a picker");
        picker.settle();
        picker
            .window(50)
            .0
            .into_iter()
            .map(|r| {
                let head = r.item.label[..r.item.code_at.unwrap()].trim_end();
                let (mark, place) = head.rsplit_once(' ').unwrap_or(("", head));
                (mark.trim().into(), place.trim_end_matches(':').into())
            })
            .collect()
    }

    /// Puts the cursor on `word` in `file` at `line` and presses `u`.
    fn usages_at(a: &mut App, dir: &Path, file: &str, line: usize, word: &str) {
        a.jump_to(&dir.join(file), line);
        a.col = a.line_str().find(word).expect(word);
        press(a, KeyCode::Char('u'), KeyModifiers::NONE);
    }

    /// #81: the declaration first and marked as one, then the open file, then the rest of the
    /// project's code nearest first, then the tests — a test file next door still sorts below
    /// code a package away.
    #[test]
    fn usages_put_the_declaration_first_and_the_tests_last() {
        let (dir, mut a) = project_app(
            "u-order",
            &[
                (
                    "src/users/repo.py",
                    "class Repo:\n    def delete_user(self, id):\n        pass\n",
                ),
                (
                    "src/users/admin.py",
                    "def purge(repo, id):\n    repo.delete_user(id)\n",
                ),
                (
                    "src/users/repo_test.py",
                    "def check(repo):\n    repo.delete_user(2)\n",
                ),
                (
                    "src/api/view.py",
                    "def view(repo):\n    repo.delete_user(1)\n",
                ),
                (
                    "tests/test_repo.py",
                    "def test_delete(repo):\n    repo.delete_user(1)\n",
                ),
            ],
        );
        usages_at(&mut a, &dir, "src/users/admin.py", 2, "delete_user");
        assert_eq!(
            usage_rows(&mut a),
            [
                ("declaration".to_string(), "src/users/repo.py:2".to_string()),
                (String::new(), "src/users/admin.py:2".into()),
                (String::new(), "src/api/view.py:2".into()),
                (String::new(), "src/users/repo_test.py:2".into()),
                (String::new(), "tests/test_repo.py:2".into()),
            ]
        );
        assert_eq!(
            a.picker.as_ref().unwrap().title,
            "Usages of delete_user: 1 declaration, 2 in code, 2 in tests"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The title says how the list splits, and a part with no hits is left out rather than
    /// printed as a zero.
    #[test]
    fn the_usages_title_splits_the_counts_and_omits_what_is_not_there() {
        let (dir, mut a) = project_app(
            "u-title",
            &[
                (
                    "src/repo.py",
                    "class Repo:\n    def delete_user(self, id):\n        pass\n",
                ),
                ("src/api/repo.py", "class Repo:\n    pass\n"),
                (
                    "src/admin.py",
                    "import log\n\ndef purge(repo, id):\n    log.info(\"purge\")\n    repo.delete_user(id)\n",
                ),
                (
                    "tests/test_repo.py",
                    "import log\n\ndef test_delete(repo):\n    log.info(\"t\")\n    repo.delete_user(1)\n",
                ),
            ],
        );
        for (file, line, word, title) in [
            (
                "src/admin.py",
                5,
                "delete_user",
                "Usages of delete_user: 1 declaration, 1 in code, 1 in tests",
            ),
            // Two declarations and nothing else: only the one part, and it is plural.
            ("src/repo.py", 1, "Repo", "Usages of Repo: 2 declarations"),
            // A name no rule declares: no declaration part at all.
            (
                "src/admin.py",
                4,
                "info",
                "Usages of info: 1 in code, 1 in tests",
            ),
        ] {
            usages_at(&mut a, &dir, file, line, word);
            let p = a.picker.as_mut().unwrap();
            p.settle();
            assert_eq!(p.title, title);
            press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// #81: the candidates of `d` share the demotion — the copy of the declaration under `spec/`
    /// is offered last, though its path sorts first.
    #[test]
    fn d_offers_the_test_copy_of_a_declaration_last() {
        let (dir, mut a) = project_app(
            "d-tests",
            &[
                ("spec/repo.py", "def delete_user(id):\n    pass\n"),
                ("src/repo.py", "def delete_user(id):\n    pass\n"),
                ("src/admin.py", "def purge(id):\n    delete_user(id)\n"),
            ],
        );
        let rows = |a: &mut App| {
            definition_rows(a)
                .into_iter()
                .map(|(_, _, place)| place)
                .collect::<Vec<_>>()
        };
        a.jump_to(&dir.join("src/admin.py"), 2);
        a.col = a.line_str().find("delete_user").unwrap();
        press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(rows(&mut a), ["src/repo.py:1", "spec/repo.py:1"]);
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        // Read from inside `spec/` and that file is not a test copy, it is what is on screen:
        // its declaration stays first.
        a.jump_to(&dir.join("spec/repo.py"), 2);
        a.col = 4;
        a.buf.lines[1] = "    delete_user(id)".into();
        press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(rows(&mut a), ["spec/repo.py:1", "src/repo.py:1"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// #81: the mark says what `d` would call a declaration — not a `def` line inside a
    /// docstring, nothing at all where the kind has no rule for the word, and nothing in a file
    /// the list demotes, whose count belongs to the tests.
    #[test]
    fn the_usages_mark_is_only_for_a_declaration_d_would_offer() {
        let (dir, mut a) = project_app(
            "u-marks",
            &[
                (
                    "src/repo.py",
                    "class Repo:\n    def delete_user(self, id):\n        pass\n",
                ),
                (
                    "docs/guide.py",
                    "HELP = \"\"\"\nUsage:\n\ndef delete_user(id):\n\"\"\"\n",
                ),
                (
                    "src/admin.py",
                    "def purge(repo, id):\n    repo.delete_user(id)\n\ndef build():\n    return make_repo()\n",
                ),
                ("tests/test_repo.py", "def make_repo():\n    return None\n"),
                (
                    "main.tf",
                    "resource \"aws_s3_bucket\" \"logs\" {\n  count = 2\n}\n",
                ),
                (
                    "other.tf",
                    "resource \"aws_s3_bucket\" \"data\" {\n  count = 3\n}\n",
                ),
            ],
        );
        usages_at(&mut a, &dir, "src/admin.py", 2, "delete_user");
        assert_eq!(
            usage_rows(&mut a),
            [
                ("declaration".to_string(), "src/repo.py:2".to_string()),
                (String::new(), "src/admin.py:2".into()),
                (String::new(), "docs/guide.py:4".into()),
            ]
        );
        assert_eq!(
            a.picker.as_ref().unwrap().title,
            "Usages of delete_user: 1 declaration, 2 in code"
        );
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        // Terraform declares no bare `count`: with no rule there is no mark and no column for it.
        usages_at(&mut a, &dir, "main.tf", 2, "count");
        assert_eq!(
            usage_rows(&mut a),
            [
                (String::new(), "main.tf:2".to_string()),
                (String::new(), "other.tf:2".into()),
            ]
        );
        assert_eq!(
            a.picker.as_ref().unwrap().title,
            "Usages of count: 2 in code"
        );
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        // A word declared only in a test file: the declaration is a test row, marked as neither.
        usages_at(&mut a, &dir, "src/admin.py", 5, "make_repo");
        assert_eq!(
            usage_rows(&mut a),
            [
                (String::new(), "src/admin.py:5".to_string()),
                (String::new(), "tests/test_repo.py:1".into()),
            ]
        );
        assert_eq!(
            a.picker.as_ref().unwrap().title,
            "Usages of make_repo: 1 in code, 1 in tests"
        );
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
        let readme = include_str!("../../README.md");
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
