//! The key table: every press routed to what it does.

use super::*;

impl App {
    /// Handles one key. Returns `true` when merl should quit. A quit over edits that could not be
    /// saved is refused once, with the ways out in the status bar; quitting again right away
    /// leaves the edits behind.
    pub fn key(&mut self, key: KeyEvent) -> bool {
        let was = self.mode;
        let quit = self.key_inner(key);
        if matches!(was, Mode::Edit | Mode::Normal) && self.mode != was {
            self.resume_edit = was == Mode::Edit && self.mode != Mode::Normal;
        }
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
        // Edit mode is no overlay and binds no Alt+letter: there the chord does nothing, rather
        // than an Esc nobody pressed turning the rest of the word into commands.
        if key.modifiers.contains(KeyModifiers::ALT) && matches!(key.code, KeyCode::Char(_)) {
            if self.mode == Mode::Edit {
                return false;
            }
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
        // they are the Ctrl chords then.
        if key.modifiers == KeyModifiers::SUPER && matches!(key.code, KeyCode::Char('c' | 'x')) {
            key.modifiers = KeyModifiers::CONTROL;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let alt = key.modifiers.contains(KeyModifiers::ALT);

        if ctrl && key.code == KeyCode::Char('c') && self.mode != Mode::Edit {
            // Ctrl+C is copy everywhere and never quits; a prompt or picker has nothing to copy.
            if self.mode == Mode::Normal && self.picker.is_none() {
                self.copy();
            }
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
            Mode::New => {
                self.new_key(key);
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
        if self.mode == Mode::Edit && self.edit_key(key.code, ctrl, alt) {
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
                if self.find_re.take().is_some() {
                    self.message = "find cleared".into();
                }
                self.anchor = None;
            }
            KeyCode::Char('g') if ctrl => {
                self.mode = Mode::Goto;
                self.prompt.clear();
            }
            KeyCode::Char('n') if ctrl => self.start_new(),
            KeyCode::Char('s') if ctrl => self.save(),
            KeyCode::Char('r') if ctrl => {
                self.reload(true);
            }
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
            KeyCode::Char('s') => self.start_search(),
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
            KeyCode::Char('w') => self.toggle_wrap(),
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
                match self.prompt.parse::<usize>() {
                    Ok(n) => match self.buf.path.clone() {
                        Some(path) => self.jump_to(&path, n),
                        None => self.goto_line(n),
                    },
                    // An empty prompt is a cancel, as Esc.
                    Err(_) if self.prompt.is_empty() => {}
                    Err(_) => self.message = format!("no line {}", &*self.prompt),
                }
                self.close_overlay();
                self.prompt.clear();
            }
            KeyCode::Esc => {
                self.close_overlay();
                self.prompt.clear();
            }
            // Only digits are typed here; Ctrl+letter still edits the line.
            KeyCode::Char(c) if !c.is_ascii_digit() && key.modifiers.is_empty() => {}
            _ => {
                self.prompt.key(key);
            }
        }
    }
}
