use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use regex::{Regex, RegexBuilder};
use unicode_segmentation::UnicodeSegmentation;

use crate::buffer::{self, Buffer, shown_str};
use crate::git::{self, TextLine};
use crate::line_edit::LineEdit;
use crate::picker::{Pick, PickItem, Picker};
use crate::search::{self, Candidate, Hit, Kind, Reason, Tier};
use crate::tree::Tree;
use crate::tutor::{self, Tutor};
use crate::wrap;

mod android;
mod at_base;
mod c;
mod collapse;
mod component;
mod cpp;
mod cs_typed;
mod css;
mod cursor;
mod dart;
mod definition;
mod edit;
mod erlang;
mod external;
mod find;
mod gdscript;
mod haskell;
mod imported;
mod json_ref;
mod jvm;
mod jvm_typed;
mod keys;
mod links;
mod lisp;
mod lua;
mod members;
mod missed;
mod ml;
mod open;
mod perl;
mod php;
mod picker;
mod preview;
mod project_search;
mod proto;
mod python;
mod review;
mod ruby;
mod rust;
mod scroll;
mod search_job;
mod solidity;
mod starlark;
mod swift;
mod symbols;
mod tree;
mod typed;
mod usages;

use external::Walked;
pub(crate) use open::error_text;
pub use preview::Preview;
use project_search::at_label;
use review::OldPicture;
pub use review::Side;
pub use search_job::SearchJob;
use search_job::{FileLine, SEARCH_PAUSE, Typed, deleted_hits};

const MAX_LABEL_TEXT: usize = 120;
const MAX_NAME_PAD: usize = 40;

pub const KEYS: &[(&str, &str, &str)] = &[
    ("o / Ctrl+E", "Open a file (fuzzy)", "Files and jumps"),
    (
        "Ctrl+N",
        "New file: type its path, Enter creates and edits it",
        "Files and jumps",
    ),
    (
        "d / F12",
        "Go to definition of the word under the cursor, or its implementations",
        "Files and jumps",
    ),
    ("D", "Project symbols (fuzzy)", "Files and jumps"),
    (
        "u / Shift+F12",
        "Usages of the word under the cursor",
        "Files and jumps",
    ),
    (
        "[ / ]",
        "Back / forward in the jump history",
        "Files and jumps",
    ),
    (": / Ctrl+G", "Go to line", "Files and jumps"),
    ("/ / Ctrl+F", "Find in the open file", "Search"),
    ("n / N", "Next / previous match", "Search"),
    ("s", "Search the project", "Search"),
    (
        "c / C",
        "Review: next / previous hunk, on to the next file",
        "Review",
    ),
    (
        "m",
        "Review: mark the file as viewed, or take the mark off",
        "Review",
    ),
    (
        "Fold: Enter",
        "Review: load the diff of a folded generated file",
        "Review",
    ),
    (
        "Enter",
        "Edit at the cursor (Esc returns to navigation)",
        "Editing",
    ),
    (
        "Ctrl+S",
        "Save now (edits are saved on their own after a pause)",
        "Editing",
    ),
    (
        "Ctrl+R",
        "Reload from disk, dropping unsaved edits",
        "Editing",
    ),
    ("Ctrl+Z / Ctrl+Y", "Undo / redo", "Editing"),
    (
        "Ctrl+C",
        "Copy the selection, or the line, to the clipboard",
        "Editing",
    ),
    ("Edit: Ctrl+X", "Cut the selection, or the line", "Editing"),
    (
        "Edit: Alt+Backspace / Alt+Delete",
        "Delete the word before / after the cursor",
        "Editing",
    ),
    (
        "v",
        "Select the word, then the line, then the paragraph, then the whole file",
        "Selection",
    ),
    (
        "Shift+Up / Shift+Down",
        "Extend the selection by a screen row",
        "Selection",
    ),
    (
        "Shift+Left / Shift+Right",
        "Extend the selection by a char",
        "Selection",
    ),
    (
        "Alt+Shift+Left / Right",
        "Extend the selection by a word",
        "Selection",
    ),
    (
        "Ctrl+Shift+Left / Right",
        "Extend the selection to the start / end of the screen row, then of the line",
        "Selection",
    ),
    (
        "Shift+Home / Shift+End",
        "Extend the selection to the start / end of the screen row, then of the line",
        "Selection",
    ),
    (
        "Shift+PgUp / Shift+PgDn",
        "Extend the selection by a screen",
        "Selection",
    ),
    (
        "Ctrl+Shift+Home / Ctrl+Shift+End",
        "Extend the selection to the start / end of the file",
        "Selection",
    ),
    (
        "Arrows",
        "Move the cursor; Up / Down go by screen row",
        "Movement",
    ),
    ("Alt+Left / Alt+Right", "Move one word", "Movement"),
    (
        "Home / End",
        "Start / end of the screen row, then of the line",
        "Movement",
    ),
    (
        "Ctrl+Home / Ctrl+End",
        "Start / end of the file",
        "Movement",
    ),
    (
        "{ / }",
        "Previous / next paragraph (blank line)",
        "Movement",
    ),
    (
        "Ctrl+D / Ctrl+U",
        "Move half a screen down / up",
        "Movement",
    ),
    ("PgUp / PgDn", "Move one screen", "Movement"),
    ("t", "Show or hide the file tree", "Panels"),
    ("Tab", "Switch focus between tree and code", "Panels"),
    ("Tree: Up / Down", "Move", "Panels"),
    (
        "Tree: Enter",
        "Open the file, or expand the directory",
        "Panels",
    ),
    ("Tree: Left / Right", "Collapse / expand", "Panels"),
    ("Picker: Up / Down", "Move", "Panels"),
    ("Picker: PgUp / PgDn", "Move one page", "Panels"),
    ("Picker: Enter", "Accept", "Panels"),
    ("Picker: Esc", "Cancel", "Panels"),
    ("?", "This help", "Panels"),
    ("Help: Up / Down", "Scroll", "Panels"),
    (
        "w",
        "Wrap long lines, or cut them at the edge and scroll sideways",
        "General",
    ),
    (
        "p",
        "Show a Markdown file rendered, or its source again",
        "General",
    ),
    (
        "f",
        "Fold the function or block at the cursor into its first line, or unfold it",
        "General",
    ),
    ("T", "Pick a theme (live preview)", "General"),
    (
        "Esc",
        "Close an overlay, leave edit mode, or clear selection and find",
        "General",
    ),
    ("q", "Quit", "General"),
];

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
    Find,
    Goto,
    New,
    Picker(PickerKind),
    Help,
    Edit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Tree,
    Code,
}

const SIDE_OFF: usize = 8;
const HIST_NEAR: usize = 10;
const HIST_MAX: usize = 50;

type Paths = Arc<Vec<PathBuf>>;
type CMode = (bool, bool);

pub struct App {
    pub root: PathBuf,
    pub shallow: bool,
    pub tree_order: Option<crate::tree::OrderFile>,
    pub buf: Buffer,
    pub tree: Tree,
    pub files: Vec<PathBuf>,
    pub ignored: Vec<PathBuf>,
    external: HashMap<Kind, (Vec<PathBuf>, Arc<Vec<PathBuf>>)>,
    walked_roots: HashMap<PathBuf, Arc<Vec<PathBuf>>>,
    shaped: search::ShapedFiles,
    warming: HashMap<Kind, Option<std::thread::JoinHandle<Walked>>>,
    node_modules_of: Option<PathBuf>,
    otp: Option<Vec<PathBuf>>,
    c_includes: HashMap<(PathBuf, CMode), Paths>,
    c_files: HashMap<CMode, (Paths, Paths)>,
    go_build: search::GoBuild,
    offer_only: bool,
    probe: Option<Vec<Candidate>>,
    base_app: Option<(String, Box<App>)>,
    truncated: std::cell::Cell<bool>,
    reading: std::cell::RefCell<Vec<(PathBuf, usize)>>,
    pub focus: Focus,
    pub show_tree: bool,
    pub tree_width: Option<u16>,
    pub tree_top: usize,
    pub picker: Option<Picker>,
    search_seq: u64,
    search_due: Option<Instant>,
    search_sent: Option<u64>,
    search_enter: bool,
    pub history: Vec<(PathBuf, TextLine, usize)>,
    pub hist_idx: usize,
    hist_rows: HashMap<(PathBuf, TextLine, usize), usize>,
    pub line: usize,
    pub col: usize,
    pub want_x: usize,
    pub deleted: Option<(usize, usize)>,
    anchor: Option<(TextLine, usize)>,
    pub top_line: usize,
    pub top_row: usize,
    pub left: usize,
    pub collapsed: Vec<(usize, usize)>,
    collapsed_stash: HashMap<PathBuf, Vec<(usize, String)>>,
    wrap_opposite_of_kind: HashSet<PathBuf>,
    previewed: HashSet<PathBuf>,
    pub preview: Option<Preview>,
    pub diagrams: crate::mermaid::Diagrams,
    old_picture: std::cell::RefCell<Option<OldPicture>>,
    pub mode: Mode,
    pub prompt: LineEdit,
    pub find_re: Option<Regex>,
    find_query: String,
    find_anchor: (TextLine, usize),
    find_sel: Option<(TextLine, usize)>,
    pub message: String,
    pub message_path: Option<(String, usize)>,
    pub view_w: usize,
    pub view_h: usize,
    center: bool,
    pub tutor: Option<Tutor>,
    pub help_top: usize,
    pub no_watch: bool,
    pub dirty: bool,
    pub conflict: bool,
    last_edit: Option<Instant>,
    pub autosave: Duration,
    pub review_panel_colours: bool,
    pub review_list_marks: bool,
    pub review_open_files_first: bool,
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    undo_break: bool,
    edit_kind: edit::Kind,
    stash: HashMap<PathBuf, Stashed>,
    resume_edit: bool,
    pub clipboard: Option<String>,
    pub diff: git::Diff,
    pub base: Option<(String, Buffer)>,
    pub want_diff: bool,
    pub review: Option<git::Review>,
    pub marks: HashMap<PathBuf, char>,
    pub viewed: HashMap<PathBuf, u64>,
    hidden: HashMap<PathBuf, u64>,
    unfolded: HashSet<PathBuf>,
    viewed_branch: Option<String>,
    last_hunk: Option<(PathBuf, usize)>,
    pub session: Option<crate::reviews::Session>,
    sessions_closed_by_switch_with_viewed: Vec<(crate::reviews::Session, Vec<PathBuf>)>,
    pub theme: String,
    pub config: Option<PathBuf>,
    pub theme_dir: Option<PathBuf>,
    quit_again_drops_unsaved: bool,
    keys_action_of_key_in_hand: Option<&'static str>,
    pub pressed: HashMap<&'static str, u64>,
    pub missed: HashMap<&'static str, u64>,
    watch: missed::Watch,
}

type Stashed = (Vec<String>, buffer::Format, Vec<Edit>, Vec<Edit>);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Edit {
    line: usize,
    old: Vec<String>,
    new: Vec<String>,
    before: (usize, usize),
    after: (usize, usize),
    format: Option<(buffer::Format, buffer::Format)>,
}

pub fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

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
            tree_order: None,
            buf: Buffer::empty(),
            tree,
            files,
            ignored,
            external: HashMap::new(),
            walked_roots: HashMap::new(),
            shaped: Default::default(),
            warming: HashMap::new(),
            node_modules_of: None,
            otp: None,
            c_includes: HashMap::new(),
            c_files: HashMap::new(),
            go_build: search::GoBuild::host().env(
                std::env::var("CGO_ENABLED").ok().as_deref(),
                std::env::var("GOFLAGS").ok().as_deref(),
            ),
            offer_only: false,
            probe: None,
            base_app: None,
            truncated: Default::default(),
            reading: Default::default(),
            focus,
            show_tree: true,
            tree_width: None,
            tree_top: 0,
            picker: None,
            search_seq: 0,
            search_due: None,
            search_sent: None,
            search_enter: false,
            history: Vec::new(),
            hist_rows: HashMap::new(),
            hist_idx: 0,
            line: 0,
            col: 0,
            deleted: None,
            want_x: 0,
            anchor: None,
            top_line: 0,
            top_row: 0,
            left: 0,
            collapsed: Vec::new(),
            collapsed_stash: HashMap::new(),
            wrap_opposite_of_kind: HashSet::new(),
            previewed: HashSet::new(),
            diagrams: crate::mermaid::Diagrams::default(),
            old_picture: Default::default(),
            preview: None,
            mode: Mode::Normal,
            prompt: LineEdit::default(),
            find_re: None,
            find_query: String::new(),
            find_anchor: (TextLine::File(0), 0),
            find_sel: None,
            message: String::new(),
            message_path: None,
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
            review_panel_colours: true,
            review_list_marks: true,
            review_open_files_first: true,
            undo: Vec::new(),
            redo: Vec::new(),
            undo_break: false,
            edit_kind: edit::Kind::Other,
            stash: HashMap::new(),
            resume_edit: false,
            clipboard: None,
            diff: git::Diff::default(),
            base: None,
            want_diff: true,
            review: None,
            marks: HashMap::new(),
            viewed: HashMap::new(),
            hidden: HashMap::new(),
            unfolded: HashSet::new(),
            viewed_branch: None,
            last_hunk: None,
            session: None,
            sessions_closed_by_switch_with_viewed: Vec::new(),
            theme: crate::theme::DEFAULT.to_string(),
            config: None,
            theme_dir: crate::theme::user_dir(),
            quit_again_drops_unsaved: false,
            keys_action_of_key_in_hand: None,
            pressed: HashMap::new(),
            missed: HashMap::new(),
            watch: Default::default(),
        };
        lock_no_write(&mut buf);
        app.buf = buf;
        if let Some((n, char_col1)) = at {
            app.goto_line(n);
            let s = app.buf.shown(app.line);
            app.col = s
                .char_indices()
                .nth(char_col1.saturating_sub(1))
                .map_or(s.len(), |(i, _)| i);
            app.sync_want_x();
        }
        if let Some(path) = app.buf.path.clone() {
            app.reveal(&path);
        }
        app.hist_note(true);
        app
    }

    pub fn rel_path(&self) -> String {
        match &self.buf.path {
            Some(path) => self.rel_path_of(path),
            None => format!("{}/", self.root_name()),
        }
    }

    pub fn rel_path_of(&self, path: &Path) -> String {
        match (path.strip_prefix(&self.root), search::opened_kind(path)) {
            (Ok(rel), _) => rel,
            (Err(_), Some(kind)) => self.rel_to_its_root(kind, path),
            (Err(_), None) => path,
        }
        .display()
        .to_string()
    }

    pub fn say_about(&mut self, path: &Path, rest: &str) {
        let name = self.rel_path_of(path);
        self.message = format!("{name}{rest}");
        self.message_path = Some((self.message.clone(), name.len()));
    }

    pub fn root_name(&self) -> String {
        self.root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.root.display().to_string())
    }

    pub fn line_str(&self) -> &str {
        self.text(self.at())
    }

    pub fn at(&self) -> TextLine {
        match self.deleted {
            Some((k, i)) => TextLine::Deleted(k, i),
            None => TextLine::File(self.line),
        }
    }

    pub(super) fn set_at(&mut self, t: TextLine) {
        let last = self.buf.lines.len() - 1;
        (self.line, self.deleted) = match t {
            TextLine::File(l) => (l, None),
            TextLine::Deleted(k, i) => (k.min(last), Some((k, i))),
        };
    }

    pub(super) fn go(&mut self, (line, col): (usize, usize)) {
        (self.line, self.col, self.deleted) = (line, col, None);
    }

    pub fn text(&self, t: TextLine) -> &str {
        match t {
            TextLine::File(l) => &self.buf.lines[l],
            TextLine::Deleted(k, i) => &self.diff.ghosts[&k][i],
        }
    }

    pub(super) fn deleted_at(&self, k: usize) -> usize {
        self.diff.ghosts.get(&k).map_or(0, Vec::len)
    }

    pub(super) fn next_line(&self, t: TextLine) -> Option<TextLine> {
        let n = self.buf.lines.len();
        match t {
            TextLine::Deleted(k, i) if i + 1 < self.deleted_at(k) => {
                Some(TextLine::Deleted(k, i + 1))
            }
            TextLine::Deleted(k, _) => (k < n).then_some(TextLine::File(k)),
            TextLine::File(l) if l < n && self.deleted_at(l + 1) > 0 => {
                Some(TextLine::Deleted(l + 1, 0))
            }
            TextLine::File(l) => (l + 1 < n).then_some(TextLine::File(l + 1)),
        }
    }

    pub(super) fn prev_line(&self, t: TextLine) -> Option<TextLine> {
        match t {
            TextLine::Deleted(k, i) if i > 0 => Some(TextLine::Deleted(k, i - 1)),
            TextLine::File(l) if self.deleted_at(l) > 0 => {
                Some(TextLine::Deleted(l, self.deleted_at(l) - 1))
            }
            t => t.key().checked_sub(1).map(TextLine::File),
        }
    }

    pub(super) fn next_shown(&self, t: TextLine) -> Option<TextLine> {
        std::iter::successors(self.next_line(t), |&t| self.next_line(t))
            .find(|t| !self.hidden(t.key()))
    }

    pub(super) fn prev_shown(&self, t: TextLine) -> Option<TextLine> {
        std::iter::successors(self.prev_line(t), |&t| self.prev_line(t))
            .find(|t| !self.hidden(t.key()))
    }

    pub(super) fn first_line(&self) -> TextLine {
        match self.deleted_at(0) {
            0 => TextLine::File(0),
            _ => TextLine::Deleted(0, 0),
        }
    }

    pub(super) fn last_line(&self) -> TextLine {
        let n = self.buf.lines.len();
        match self.deleted_at(n) {
            0 => TextLine::File(n - 1),
            g => TextLine::Deleted(n, g - 1),
        }
    }

    pub(super) fn clamp_line(&self, t: TextLine) -> TextLine {
        let last = self.buf.lines.len() - 1;
        match t {
            TextLine::Deleted(k, i) if k <= last + 1 && i < self.deleted_at(k) => t,
            t => TextLine::File(t.key().min(last)),
        }
    }

    pub fn nowrap(&self) -> bool {
        let Some(path) = &self.buf.path else {
            return false;
        };
        let table = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("csv") || e.eq_ignore_ascii_case("tsv"));
        table != self.wrap_opposite_of_kind.contains(path)
    }

    fn toggle_wrap(&mut self) {
        let Some(path) = self.buf.path.clone() else {
            return;
        };
        if !self.wrap_opposite_of_kind.remove(&path) {
            self.wrap_opposite_of_kind.insert(path);
        }
        self.clamp_top();
        self.sync_want_x();
        self.message = if self.nowrap() {
            "wrap off, the view follows the cursor".into()
        } else {
            "wrap on".into()
        };
    }

    pub fn rows(&self, l: usize) -> Vec<std::ops::Range<usize>> {
        self.text_rows(&self.buf.lines[l])
    }

    pub fn text_rows(&self, text: &str) -> Vec<std::ops::Range<usize>> {
        if self.nowrap() {
            return std::iter::once(0..shown_str(text).len()).collect();
        }
        wrap::wrap_shown(text, self.view_w)
    }

    pub fn ghost_rows(&self, l: usize) -> usize {
        self.diff.ghosts.get(&l).map_or(0, |ghosts| {
            ghosts.iter().map(|g| self.text_rows(g).len()).sum()
        })
    }

    pub fn row_count(&self, l: usize) -> usize {
        if self.hidden(l) {
            return 0;
        }
        let text = match l < self.buf.lines.len() {
            true => self.rows(l).len(),
            false => 0,
        };
        self.ghost_rows(l) + text
    }

    pub fn cursor_row(&self) -> usize {
        let above = match self.deleted {
            Some((k, i)) => self.diff.ghosts[&k][..i]
                .iter()
                .map(|g| self.text_rows(g).len())
                .sum(),
            None => self.ghost_rows(self.line),
        };
        above + wrap::col_to_row(&self.text_rows(self.line_str()), self.col)
    }

    pub fn cursor_at(&self) -> (usize, usize) {
        (self.at().key(), self.cursor_row())
    }

    pub(super) fn line_at_row(&self, key: usize, mut row: usize) -> (TextLine, usize) {
        let ghosts = self.diff.ghosts.get(&key).map_or(&[][..], Vec::as_slice);
        for (i, g) in ghosts.iter().enumerate() {
            let n = self.text_rows(g).len();
            if row < n || (i + 1 == ghosts.len() && key >= self.buf.lines.len()) {
                return (TextLine::Deleted(key, i), row.min(n - 1));
            }
            row -= n;
        }
        (TextLine::File(key.min(self.buf.lines.len() - 1)), row)
    }

    pub fn cursor_x(&self) -> usize {
        let rows = self.text_rows(self.line_str());
        let row = wrap::col_to_row(&rows, self.col);
        let indent = match row {
            0 => 0,
            _ => wrap::indent(shown_str(self.line_str()), self.view_w),
        };
        indent + wrap::width(&self.line_str()[wrap::row_to_col(&rows, row)..self.col])
    }

    pub fn display_col(&self) -> usize {
        wrap::width(&self.line_str()[..self.col]) + 1
    }
}

fn signature(kind: Kind, callee: &str, returns: &str) -> String {
    match kind {
        Kind::Python => format!("{callee}() -> {returns}"),
        Kind::TsJs => format!("{callee}(): {returns}"),
        _ => format!("{callee}() {returns}"),
    }
}

fn bound(imports: &[search::Import], name: &str) -> Option<Vec<String>> {
    imports
        .iter()
        .find(|i| i.name == name)
        .map(|i| i.path.clone())
}

pub(crate) fn clip(s: &str, max: usize) -> String {
    match s.grapheme_indices(true).nth(max) {
        Some((i, _)) => format!("{}\u{2026}", &s[..i]),
        None => s.to_string(),
    }
}

pub fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub(super) fn word_col(line: &str, word: &str, word_chars_past_alnum: &str) -> usize {
    whole_at(line, word, word_chars_past_alnum)
        .or_else(|| whole_at(line, word.strip_suffix('=')?, word_chars_past_alnum))
        .or_else(|| {
            whole_at(
                &line.to_ascii_lowercase(),
                &word.to_ascii_lowercase(),
                word_chars_past_alnum,
            )
        })
        .unwrap_or(0)
}

pub(super) fn whole_at(line: &str, word: &str, word_chars_past_alnum: &str) -> Option<usize> {
    let part = |c: char| is_word(c) || word_chars_past_alnum.contains(c);
    let whole = |(i, _): &(usize, &str)| {
        !line[..*i].ends_with(part) && !line[i + word.len()..].starts_with(part)
    };
    line.match_indices(word).find(whole).map(|(i, _)| i)
}

pub(crate) fn next_char(s: &str, i: usize) -> usize {
    s[i..].graphemes(true).next().map_or(i, |g| i + g.len())
}

pub(crate) fn prev_char(s: &str, i: usize) -> usize {
    s[..i]
        .graphemes(true)
        .next_back()
        .map_or(0, |g| i - g.len())
}

fn names_itself(kind: Kind, line: &str, name: &str) -> bool {
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
    .filter_map(|k| t.strip_prefix(k).map(|rest| (k, rest)))
    .any(|(k, rest)| {
        rest.strip_prefix(name).is_some_and(|after| {
            !after.starts_with(is_word)
                && (kind != Kind::Jvm || *k != "def " || after.trim_start().starts_with('('))
        })
    });
    declares
        || import_line(kind, line)
        || kind == Kind::Swift && search::swift_local_decl(line, name)
}

fn import_line(kind: Kind, line: &str) -> bool {
    let t = line.trim_start();
    match kind {
        Kind::Python => t.starts_with("import ") || t.starts_with("from "),
        Kind::TsJs | Kind::Jvm | Kind::Go | Kind::Swift | Kind::Elixir | Kind::Proto => {
            t.starts_with("import ")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests;
