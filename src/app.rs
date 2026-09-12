//! All editor state and every key binding. Rendering lives in `ui.rs`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use regex::{Regex, RegexBuilder};

use crate::buffer::Buffer;
use crate::picker::{Pick, PickItem, Picker};
use crate::search::{self, Hit};
use crate::tree::Tree;
use crate::wrap;

/// Shift+Up / Shift+Down jump distance.
const SHIFT_LINES: usize = 3;
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
    ("Arrows", "Move the cursor"),
    ("Shift+Up / Shift+Down", "Move three lines"),
    ("Shift+Left / Shift+Right", "Move one word"),
    ("PgUp / PgDn", "Move one screen"),
    ("Home / End", "Start / end of the line"),
    ("Ctrl+Home / Ctrl+End", "Start / end of the file"),
    ("Esc", "Close an overlay, or clear the find highlights"),
    ("?", "This help"),
    ("q / Ctrl+C", "Quit"),
    ("Tree: Up / Down", "Move"),
    ("Tree: Enter", "Open the file, or expand the directory"),
    ("Tree: Left / Right", "Collapse / expand"),
    ("Picker: Up / Down, Ctrl+P / Ctrl+N", "Move"),
    ("Picker: Enter", "Accept"),
    ("Picker: Esc", "Cancel"),
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Tree,
    Code,
}

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
    /// Visited positions, oldest first; `hist_idx` points at the current one.
    pub history: Vec<(PathBuf, usize, usize)>,
    pub hist_idx: usize,
    /// Wakes the event loop when nucleo has new results. Set by `main`.
    pub wake: Arc<dyn Fn() + Send + Sync>,
    /// Cursor: file line, byte offset into that line, and the display column Up/Down aims for.
    pub line: usize,
    pub col: usize,
    pub want_x: usize,
    /// Top of the viewport: a file line plus which wrapped row of it is first on screen.
    pub top_line: usize,
    pub top_row: usize,
    pub mode: Mode,
    /// What has been typed into the `:`, `/` or `s>` prompt.
    pub prompt: String,
    /// Last pattern that compiled, kept for `n`/`N` and for painting the matches.
    pub find_re: Option<Regex>,
    /// The query in the find prompt does not compile; the prompt goes red.
    pub find_bad: bool,
    /// Where the cursor was when `/` was pressed: the start of the incremental search.
    find_anchor: (usize, usize),
    pub message: String,
    /// Code text area, in cells, written by `ui::draw` before every frame.
    pub view_w: usize,
    pub view_h: usize,
    center: bool,
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
            top_line: 0,
            top_row: 0,
            mode: Mode::Normal,
            prompt: String::new(),
            find_re: None,
            find_bad: false,
            find_anchor: (0, 0),
            message: String::new(),
            view_w: 80,
            view_h: 24,
            center: false,
        };
        if let Some(n) = line {
            app.goto_line(n);
        }
        if let Some(path) = app.buf.path.clone() {
            app.reveal(&path);
        }
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

    /// Wrapped rows of file line `l` at the current viewport width.
    pub fn rows(&self, l: usize) -> Vec<std::ops::Range<usize>> {
        wrap::wrap_line(&self.buf.lines[l], self.view_w)
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

    fn move_line(&mut self, delta: isize) {
        let last = self.buf.lines.len() - 1;
        self.line = self.line.saturating_add_signed(delta).min(last);
        self.apply_want_x();
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
    fn open(&mut self, path: &Path, line: usize) {
        if self.buf.path.as_deref() != Some(path) {
            match Buffer::load(path) {
                Ok(buf) => {
                    self.buf = buf;
                    (self.line, self.col, self.want_x) = (0, 0, 0);
                    (self.top_line, self.top_row) = (0, 0);
                }
                Err(e) => {
                    self.message = format!("{e:#}");
                    return;
                }
            }
        }
        self.goto_line(line.max(1));
        self.reveal(path);
    }

    /// Re-reads the open file after it changed on disk. Cursor, scroll, history and find pattern
    /// survive; the cursor is clamped to whatever the file is now.
    pub fn reload(&mut self) {
        let Some(path) = self.buf.path.clone() else {
            return;
        };
        // Mid-save the file can be briefly gone; the rename that follows sends another event.
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let buf = Buffer::from_bytes(path, &bytes);
        if buf.lines == self.buf.lines {
            return;
        }
        self.buf = buf;
        let last = self.buf.lines.len() - 1;
        self.line = self.line.min(last);
        self.col = self.col.min(self.line_str().len());
        while !self.line_str().is_char_boundary(self.col) {
            self.col -= 1;
        }
        self.top_line = self.top_line.min(last);
        self.top_row = self.top_row.min(self.rows(self.top_line).len() - 1);
        self.sync_want_x();
        self.clamp_scroll();
        self.message = "reloaded".into();
    }

    fn pos(&self) -> Option<(PathBuf, usize, usize)> {
        self.buf.path.clone().map(|p| (p, self.line, self.col))
    }

    fn hist_push(&mut self, pos: Option<(PathBuf, usize, usize)>) {
        if let Some(pos) = pos
            && self.history.last() != Some(&pos)
        {
            self.history.push(pos);
        }
    }

    /// Opens `path` at `line` and records the jump: where we left from, then where we landed.
    pub fn jump_to(&mut self, path: &Path, line: usize) {
        if !self.history.is_empty() {
            self.history.truncate(self.hist_idx + 1);
        }
        let from = self.pos();
        self.open(path, line);
        self.focus = Focus::Code;
        if self.pos() == from {
            return; // the jump went nowhere; nothing to remember
        }
        self.hist_push(from);
        self.hist_push(self.pos());
        self.hist_idx = self.history.len().saturating_sub(1);
    }

    /// `[` and `]`: walks the recorded positions without recording anything new.
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
        self.hist_idx = i;
        let (path, line, col) = self.history[i].clone();
        self.open(&path, line + 1);
        self.focus = Focus::Code;
        self.col = col.min(self.line_str().len());
        self.sync_want_x();
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

    fn show_picker(&mut self, kind: PickerKind, items: Vec<PickItem>) {
        self.picker = Some(Picker::new(kind.title(), items, false, self.wake.clone()));
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
        self.find_bad = false;
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
            KeyCode::Enter => self.mode = Mode::Normal,
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.find_bad = false;
                let (line, col) = self.find_anchor;
                self.line = line.min(self.buf.lines.len() - 1);
                self.col = col.min(self.line_str().len());
                self.sync_want_x();
            }
            _ => {}
        }
    }

    /// Recompiles the query (smart-case) and moves to the first match at or after the anchor.
    /// A query that does not compile leaves the cursor and the last good pattern alone.
    fn refresh_find(&mut self) {
        if self.prompt.is_empty() {
            self.find_bad = false;
            return;
        }
        let insensitive = !self.prompt.chars().any(char::is_uppercase);
        match RegexBuilder::new(&self.prompt)
            .case_insensitive(insensitive)
            .build()
        {
            Ok(re) => {
                self.find_bad = false;
                let (l, c) = self.find_anchor;
                let hit = self
                    .match_at_or_after(&re, l, c)
                    .or_else(|| self.match_at_or_after(&re, 0, 0));
                if let Some((l, c)) = hit {
                    self.go_to_match(l, c);
                }
                self.find_re = Some(re);
            }
            Err(_) => self.find_bad = true,
        }
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

    /// Greps the project, optionally only the files with extension `ext`.
    fn grep(
        &self,
        pattern: &str,
        whole_word: bool,
        smart_case: bool,
        ext: Option<&str>,
    ) -> Vec<Hit> {
        let filtered: Vec<PathBuf>;
        let files = match ext {
            Some(ext) => {
                filtered = self
                    .files
                    .iter()
                    .filter(|p| p.extension().is_some_and(|e| e == ext))
                    .cloned()
                    .collect();
                &filtered
            }
            None => &self.files,
        };
        let current = self.rel_current();
        // The only pattern that can fail to compile is one the user typed; an empty result
        // leaves the picker closed, which is what "no matches" looks like anyway.
        search::grep_project(
            &self.root,
            files,
            pattern,
            whole_word,
            smart_case,
            current.as_deref(),
        )
        .unwrap_or_default()
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

    /// `rel/path:line: text` rows for a result picker.
    fn hit_items(hits: Vec<Hit>) -> Vec<PickItem> {
        hits.into_iter()
            .map(|h| PickItem {
                label: format!(
                    "{}:{}: {}",
                    h.path.display(),
                    h.line,
                    clip(h.text.trim(), MAX_LABEL_TEXT)
                ),
                path: h.path,
                line: h.line,
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
        let hits = self.grep(&query, false, true, None);
        if hits.is_empty() {
            self.message = format!("no results for {query}");
            return;
        }
        self.show_picker(PickerKind::Search, Self::hit_items(hits));
    }

    /// `d` / F12. Python and Go get their declaration patterns; anything else falls back to a
    /// whole-word search for the identifier itself.
    fn goto_definition(&mut self) {
        let Some(word) = self.word_under() else {
            return;
        };
        let ext = self.extension();
        let patterns = search::def_patterns(&ext, &word);
        let mut hits = if patterns.is_empty() {
            self.grep(&regex::escape(&word), true, false, None)
        } else {
            self.grep(&patterns.join("|"), false, false, Some(&ext))
        };
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
        let hits = self.grep(&regex::escape(&word), true, false, None);
        if hits.is_empty() {
            self.message = format!("no usages of {word}");
            return;
        }
        self.show_picker(PickerKind::Usages, Self::hit_items(hits));
    }

    /// `D`: every declaration in the project, recomputed on each press.
    fn symbols(&mut self) {
        let hits = self.grep(search::SYMBOL_PATTERN, false, false, None);
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
                // Opening from the tree keeps the focus there, like VS Code's preview tabs.
                Some(n) => {
                    let path = self.root.join(&n.path);
                    self.jump_to(&path, 1);
                }
                None => {}
            },
            _ => {}
        }
    }

    // ---- keys ------------------------------------------------------------

    /// Handles one key. Returns `true` when merl should quit.
    pub fn key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }
        let mut key = key;
        // Legacy terminals report Alt+X as Esc followed by X; treat it that way so no binding
        // can be reached by accident. merl itself never binds Alt.
        if key.modifiers.contains(KeyModifiers::ALT) {
            key.modifiers.remove(KeyModifiers::ALT);
            self.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            return self.key(key);
        }
        // In kitty mode `:`, `?` and `D` arrive with SHIFT set; legacy sends none.
        if matches!(key.code, KeyCode::Char(_)) {
            key.modifiers.remove(KeyModifiers::SHIFT);
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

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
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
                ) {
                    self.mode = Mode::Normal;
                }
                return false;
            }
            _ => {}
        }

        self.message.clear();
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('?') => self.mode = Mode::Help,
            KeyCode::Esc => {
                self.find_re = None;
                self.message = "find cleared".into();
            }
            KeyCode::Char('g') if ctrl => {
                self.mode = Mode::Goto;
                self.prompt.clear();
            }
            KeyCode::Char(':') => {
                self.mode = Mode::Goto;
                self.prompt.clear();
            }
            KeyCode::Char('/') => self.start_find(),
            KeyCode::Char('f') if ctrl => self.start_find(),
            KeyCode::Char('n') if !ctrl => self.step_find(true),
            KeyCode::Char('N') => self.step_find(false),
            KeyCode::Char('s') => {
                self.mode = Mode::Search;
                self.prompt.clear();
            }
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
            KeyCode::Up if shift => self.move_line(-(SHIFT_LINES as isize)),
            KeyCode::Down if shift => self.move_line(SHIFT_LINES as isize),
            KeyCode::Up => self.move_line(-1),
            KeyCode::Down => self.move_line(1),
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
            KeyCode::Home => {
                self.col = 0;
                self.want_x = 0;
            }
            KeyCode::End => {
                self.col = self.line_str().len();
                self.sync_want_x();
            }
            _ => {}
        }
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
            std::fs::write(dir.join(name), "x\n".repeat(10)).unwrap();
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
    fn invalid_regex_keeps_the_last_good_pattern() {
        let mut a = app("foo\nbar\nfoo\n");
        find(&mut a, "foo");
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        a.line = 1;

        find(&mut a, "[");
        assert!(a.find_bad, "the prompt goes red");
        assert_eq!((a.line, a.col), (1, 0), "the cursor does not move");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert!(!a.find_bad);
        // `n` still walks the matches of the pattern that did compile.
        press(&mut a, KeyCode::Char('n'), KeyModifiers::NONE);
        assert_eq!((a.line, a.col), (2, 0));
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
        a.reload();
        assert_eq!(a.message, "");

        std::fs::write(&path, "a\nb\nc\n").unwrap();
        a.reload();
        assert_eq!(a.buf.lines.len(), 3);
        assert_eq!(a.line, 2);
        assert_eq!(a.col, 1);
        assert_eq!(a.message, "reloaded");
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
    fn ctrl_end_goes_to_last_line() {
        let mut a = app("a\nbb\nccc\n");
        press(&mut a, KeyCode::End, KeyModifiers::CONTROL);
        assert_eq!((a.line, a.col), (2, 3));
        press(&mut a, KeyCode::Home, KeyModifiers::CONTROL);
        assert_eq!((a.line, a.col), (0, 0));
    }
}
