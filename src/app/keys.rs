use std::ops::ControlFlow;

use super::{App, Focus, Instant, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, Mode, tutor};

impl App {
    pub fn key(&mut self, key: KeyEvent) -> bool {
        self.key_at(key, Instant::now())
    }

    pub(crate) fn key_at(&mut self, key: KeyEvent, at: Instant) -> bool {
        if tutor::card_key(self, key) {
            return false;
        }
        let was = self.mode;
        let work = self.tutor.is_none() && key.kind == KeyEventKind::Press;
        let had_picker = self.picker.is_some();
        if work {
            self.watch_before(key, at);
        }
        self.note_hist_row();
        let from = (work && self.session.is_some()).then(|| self.review_spot());
        let quit = self.key_inner(key);
        if self.mode != Mode::Find {
            self.reveal_cursor();
        }
        let action = self.keys_action_of_key_in_hand.take();
        if let Some(from) = from {
            self.review_count(Some(at), &from, action, quit);
        }
        if let Some(action) = action
            && self.tutor.is_none()
        {
            *self.pressed.entry(action).or_default() += 1;
        }
        if work {
            self.watch_after(key, was, had_picker);
        }
        if matches!(was, Mode::Edit | Mode::Normal) && self.mode != was {
            self.resume_edit = was == Mode::Edit && self.mode != Mode::Normal;
        }
        if !quit {
            self.quit_again_drops_unsaved = false;
            tutor::check(self, action);
            return false;
        }
        if self.flush() || std::mem::replace(&mut self.quit_again_drops_unsaved, true) {
            return true;
        }
        self.message = "unsaved edits: Ctrl+S saves, Ctrl+R drops them, q again quits".into();
        false
    }

    pub(super) fn key_inner(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }
        let key = match self.plain_key(key) {
            ControlFlow::Continue(key) => key,
            ControlFlow::Break(quit) => return quit,
        };
        if self.modal_key(key) {
            return false;
        }
        self.message.clear();
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        if self.mode == Mode::Edit && self.edit_key(key.code, ctrl, alt) {
            self.keys_action_of_key_in_hand = named("Edit: ", key)
                .or_else(|| named("", key).filter(|a| matches!(*a, "Esc" | "Ctrl+C")));
            self.hist_note(false);
            return false;
        }
        self.normal_key(key)
    }

    fn plain_key(&mut self, key: KeyEvent) -> ControlFlow<bool, KeyEvent> {
        let mut key = key;
        if key.modifiers == KeyModifiers::ALT {
            match key.code {
                KeyCode::Char('b') => key.code = KeyCode::Left,
                KeyCode::Char('f') => key.code = KeyCode::Right,
                _ => {}
            }
        }
        if self.does_nothing(key) {
            self.message.clear();
            return ControlFlow::Break(false);
        }
        if key.modifiers.contains(KeyModifiers::ALT) && matches!(key.code, KeyCode::Char(_)) {
            if matches!(self.mode, Mode::Edit | Mode::Note) {
                return ControlFlow::Break(false);
            }
            key.modifiers.remove(KeyModifiers::ALT);
            let overlay = self.picker.is_some() || self.mode != Mode::Normal;
            let session = self.session.take();
            self.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            self.session = session;
            return ControlFlow::Break(!overlay && self.key(key));
        }
        if matches!(key.code, KeyCode::Char(_)) {
            key.modifiers.remove(KeyModifiers::SHIFT);
        }
        if key.modifiers == KeyModifiers::SUPER && matches!(key.code, KeyCode::Char('c' | 'x')) {
            key.modifiers = KeyModifiers::CONTROL;
        }
        ControlFlow::Continue(key)
    }

    fn modal_key(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') && !matches!(self.mode, Mode::Edit | Mode::Note) {
            self.keys_action_of_key_in_hand = named("", key);
            let shown = !self.preview_blank() && !self.text_hidden();
            if let Some(i) = self.notes.focus.filter(|_| self.mode == Mode::Normal) {
                self.clipboard = Some(format!("{}\n", self.notes.list[i].text));
                self.message = "copied note".into();
            } else if self.mode == Mode::Normal && self.picker.is_none() && shown {
                self.copy();
            }
            return true;
        }
        if self.picker.is_some() {
            self.keys_action_of_key_in_hand = named("Picker: ", key);
            self.picker_key(key);
            return true;
        }
        if matches!(self.mode, Mode::Goto | Mode::Find | Mode::New | Mode::Note) {
            self.keys_action_of_key_in_hand = named("", key).filter(|a| *a == "Esc");
        }
        match self.mode {
            Mode::Goto => self.goto_key(key),
            Mode::Find => self.find_key(key),
            Mode::New => self.new_key(key),
            Mode::Note => self.note_key(key),
            Mode::Help => self.help_key(key),
            _ => return false,
        }
        true
    }

    fn help_key(&mut self, key: KeyEvent) {
        self.keys_action_of_key_in_hand = named("Help: ", key)
            .or_else(|| named("", key).filter(|a| matches!(*a, "Esc" | "?" | "q")));
        match key.code {
            KeyCode::Esc | KeyCode::Char('?' | 'q') => {
                self.mode = Mode::Normal;
            }
            KeyCode::Up => self.help_top = self.help_top.saturating_sub(1),
            KeyCode::Down => self.help_top += 1,
            _ => {}
        }
    }

    fn settle_notes(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let keeps_focus = matches!(
            key.code,
            KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown | KeyCode::Enter
        ) || matches!(key.code, KeyCode::Char('a' | 'v'))
            || ctrl && matches!(key.code, KeyCode::Char('d' | 'u' | 'z' | 'y'));
        if !keeps_focus || shift {
            self.notes.focus = None;
        }
        if !matches!(key.code, KeyCode::Char('v') | KeyCode::Enter) {
            self.notes.selected = false;
        }
    }

    fn normal_key(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        self.settle_notes(key);
        let far = is_far(key.code);
        let extending = matches!(key.code, KeyCode::Char('v' | 'V'))
            || shift
                && (far
                    || matches!(
                        key.code,
                        KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right
                    ));
        let paging = matches!(key.code, KeyCode::PageUp | KeyCode::PageDown)
            || ctrl && matches!(key.code, KeyCode::Char('d' | 'u'));
        let before = (self.at(), self.col);
        let fold = self.focus == Focus::Code && self.folded_here().is_some();
        let hidden = self.focus == Focus::Code && self.text_hidden();
        if self.focus == Focus::Tree {
            self.tree.settle();
        }
        self.keys_action_of_key_in_hand = (self.focus == Focus::Tree)
            .then(|| named("Tree: ", key).or_else(|| named("Tree: ", key.code.into())))
            .flatten()
            .or_else(|| fold.then(|| named("Fold: ", key)).flatten())
            .or_else(|| named("", key));
        if self.command_key(key, fold, hidden) {
            return true;
        }
        if !extending {
            self.drop_selection_if_moved(before);
        }
        let moves = paging
            || matches!(
                key.code,
                KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Home
                    | KeyCode::End
            );
        if moves || (self.at(), self.col) != before {
            self.undo_break = true;
        }
        if paging && let (Some(pos), Some(cur)) = (self.pos(), self.history.get_mut(self.hist_idx))
        {
            *cur = pos;
        }
        self.hist_note(false);
        false
    }

    fn command_key(&mut self, key: KeyEvent, fold: bool, hidden: bool) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            _ if self.preview_key(key) => {}
            _ if self.focus == Focus::Tree && super::tree::moves_the_tree(key) => {
                self.tree_key(key);
            }
            KeyCode::Char('q') => return true,
            KeyCode::Char('?') => {
                self.mode = Mode::Help;
                self.help_top = 0;
            }
            KeyCode::Enter if fold => self.unfold(),
            _ if hidden && !leaves_the_text(key) => {}
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
            KeyCode::Char('*') if self.focus == Focus::Code => self.star(),
            KeyCode::Char('s') => self.start_search(),
            KeyCode::Char('d') if ctrl => self.half_page(1),
            KeyCode::Char('u') if ctrl => self.half_page(-1),
            KeyCode::Char('{') => self.paragraph(-1),
            KeyCode::Char('}') => self.paragraph(1),
            KeyCode::Char('d') => self.goto_definition(),
            KeyCode::Char('u') => self.usages(),
            KeyCode::Char('D') => self.symbols(),
            _ => self.view_key(key),
        }
        false
    }

    fn view_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('t') => {
                self.show_tree = !self.show_tree;
                if !self.show_tree {
                    self.focus = Focus::Code;
                }
            }
            KeyCode::Char('T') => self.open_themes_picker(),
            KeyCode::Char('w') => self.toggle_wrap(),
            KeyCode::Char('f') if !ctrl && self.focus == Focus::Code => self.toggle_collapse(),
            KeyCode::Char('p') => self.toggle_preview(),
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
            KeyCode::Char('m') => self.toggle_viewed(),
            KeyCode::Char('a') if self.notes.on => self.add_note(),
            KeyCode::Char('v') if self.notes.focus.is_some() => self.notes.selected = true,
            KeyCode::Char('v') if self.focus == Focus::Code => self.grow(),
            KeyCode::Char('V') if self.focus == Focus::Code => self.shrink(),
            _ if self.focus == Focus::Tree => {}
            KeyCode::Enter if self.notes.focus.is_some() => self.start_note(),
            KeyCode::Enter => self.start_edit(),
            _ => self.cursor_key(key),
        }
    }

    fn cursor_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        match key.code {
            KeyCode::Up if shift => self.extend(|s| s.move_rows(-1)),
            KeyCode::Down if shift => self.extend(|s| s.move_rows(1)),
            KeyCode::Up => self.move_rows(-1),
            KeyCode::Down => self.move_rows(1),
            KeyCode::Left if shift && ctrl => self.extend(Self::line_start),
            KeyCode::Right if shift && ctrl => self.extend(Self::line_end),
            KeyCode::Left if shift && alt => self.extend(Self::word_left),
            KeyCode::Right if shift && alt => self.extend(Self::word_right),
            KeyCode::Left | KeyCode::Right if !shift && !alt && self.selection().is_some() => {
                let (start, end) = self.selection().expect("the guard saw a selection");
                let (t, col) = if key.code == KeyCode::Left {
                    start
                } else {
                    end
                };
                self.set_at(t);
                self.col = col;
                self.sync_want_x();
                self.anchor = None;
            }
            KeyCode::Left if shift => self.extend(Self::left),
            KeyCode::Right if shift => self.extend(Self::right),
            KeyCode::Left if alt => self.word_left(),
            KeyCode::Right if alt => self.word_right(),
            KeyCode::Left => self.left(),
            KeyCode::Right => self.right(),
            code if is_far(code) => self.far_key(key),
            _ => {}
        }
    }

    fn far_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if key.modifiers.contains(KeyModifiers::SHIFT) && self.anchor.is_none() {
            self.anchor = Some((self.at(), self.col));
        }
        match key.code {
            KeyCode::PageUp => {
                self.move_rows(-isize::try_from(self.page_rows(false)).unwrap_or(isize::MAX));
            }
            KeyCode::PageDown => {
                self.move_rows(isize::try_from(self.page_rows(true)).unwrap_or(isize::MAX));
            }
            KeyCode::Home if ctrl => {
                self.set_at(self.first_line());
                self.col = 0;
                self.want_x = 0;
            }
            KeyCode::End if ctrl => {
                self.set_at(self.last_line());
                self.col = self.shown_len();
                self.sync_want_x();
            }
            KeyCode::Home => self.line_start(),
            _ => self.line_end(),
        }
    }

    fn does_nothing(&self, key: KeyEvent) -> bool {
        let KeyCode::Char(c) = key.code else {
            return false;
        };
        if key.modifiers.contains(KeyModifiers::SUPER) {
            return key.modifiers.difference(KeyModifiers::SHIFT) != KeyModifiers::SUPER
                || !matches!(c, 'c' | 'x');
        }
        let mut chord = key;
        chord.modifiers.remove(KeyModifiers::ALT);
        chord.modifiers.contains(KeyModifiers::CONTROL)
            && (chord.modifiers.contains(KeyModifiers::SHIFT)
                || !c.is_ascii_lowercase()
                || (self.letters_are_keys() || key.modifiers.contains(KeyModifiers::ALT))
                    && !self.binds(chord))
    }

    fn letters_are_keys(&self) -> bool {
        self.picker.is_none() && !matches!(self.mode, Mode::Goto | Mode::Find | Mode::New)
    }

    fn binds(&self, key: KeyEvent) -> bool {
        named("", key).is_some()
            || matches!(self.mode, Mode::Edit | Mode::Note) && named("Edit: ", key).is_some()
    }

    pub(super) fn goto_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                match self.prompt.parse::<usize>() {
                    Ok(n) => match self.buf.path.clone() {
                        Some(path) => self.jump_to(&path, n),
                        None => self.goto_line(n),
                    },
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
            KeyCode::Char(c) if !c.is_ascii_digit() && key.modifiers.is_empty() => {}
            _ => {
                self.prompt.key(key);
            }
        }
    }
}

fn is_far(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::PageUp | KeyCode::PageDown | KeyCode::Home | KeyCode::End
    )
}

fn leaves_the_text(key: KeyEvent) -> bool {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Char('e' | 'n') => ctrl,
        KeyCode::Char('c' | 'C' | 'm' | 't' | 'T' | 'o' | 's' | 'D' | '[' | ']') => !ctrl,
        KeyCode::Esc | KeyCode::Tab => true,
        _ => false,
    }
}

pub(super) fn named(scope: &str, key: KeyEvent) -> Option<&'static str> {
    crate::stats::action(&format!("{scope}{}", crate::stats::name(key)))
}
