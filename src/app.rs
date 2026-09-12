//! All editor state and every key binding. Rendering lives in `ui.rs`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::buffer::Buffer;
use crate::picker::{Pick, PickItem, Picker};
use crate::tree::Tree;
use crate::wrap;

/// Shift+Up / Shift+Down jump distance.
const SHIFT_LINES: usize = 3;

/// Which picker is open. Later steps add symbols, search results and definitions here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerKind {
    Files,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Goto,
    Picker(PickerKind),
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
    pub goto: String,
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
            goto: String::new(),
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

    pub fn open_picker(&mut self, kind: PickerKind) {
        let PickerKind::Files = kind;
        let items = self
            .files
            .iter()
            .map(|p| PickItem {
                label: p.display().to_string(),
                path: p.clone(),
                line: 0,
            })
            .collect();
        self.picker = Some(Picker::new("Files", items, true, self.wake.clone()));
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
        if self.mode == Mode::Goto {
            self.goto_key(key.code);
            return false;
        }

        self.message.clear();
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('g') if ctrl => {
                self.mode = Mode::Goto;
                self.goto.clear();
            }
            KeyCode::Char(':') => {
                self.mode = Mode::Goto;
                self.goto.clear();
            }
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
            KeyCode::Char('o') if !ctrl => self.open_picker(PickerKind::Files),
            KeyCode::Char('e') if ctrl => self.open_picker(PickerKind::Files),
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
            KeyCode::Char(c) if c.is_ascii_digit() => self.goto.push(c),
            KeyCode::Backspace => {
                self.goto.pop();
            }
            KeyCode::Enter => {
                if let Ok(n) = self.goto.parse::<usize>() {
                    match self.buf.path.clone() {
                        Some(path) => self.jump_to(&path, n),
                        None => self.goto_line(n),
                    }
                }
                self.mode = Mode::Normal;
                self.goto.clear();
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.goto.clear();
            }
            _ => {}
        }
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

    #[test]
    fn ctrl_end_goes_to_last_line() {
        let mut a = app("a\nbb\nccc\n");
        press(&mut a, KeyCode::End, KeyModifiers::CONTROL);
        assert_eq!((a.line, a.col), (2, 3));
        press(&mut a, KeyCode::Home, KeyModifiers::CONTROL);
        assert_eq!((a.line, a.col), (0, 0));
    }
}
