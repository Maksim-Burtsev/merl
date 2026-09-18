//! The one-line editor behind the `:` and `/` prompts and the picker query.

use std::ops::{Deref, Range};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::is_word;

/// A line of text with a cursor and a selection. It reads as a `&str`; the keys are the ones the
/// buffer uses, so nothing new has to be learned inside a prompt.
#[derive(Default)]
pub struct LineEdit {
    text: String,
    /// Byte index of the cursor, always on a char boundary.
    cur: usize,
    /// Where the selection started; it runs from here to the cursor.
    anchor: Option<usize>,
}

impl Deref for LineEdit {
    type Target = str;
    fn deref(&self) -> &str {
        &self.text
    }
}

impl LineEdit {
    /// A line opened with `text` in it, all selected: typing replaces it, an arrow edits it.
    pub fn selected(text: &str) -> Self {
        Self {
            text: text.into(),
            cur: text.len(),
            anchor: Some(0),
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn cursor(&self) -> usize {
        self.cur
    }

    /// The selected bytes; None when the cursor stands on the anchor.
    pub fn selection(&self) -> Option<Range<usize>> {
        let a = self.anchor.filter(|&a| a != self.cur)?;
        Some(a.min(self.cur)..a.max(self.cur))
    }

    /// Applies one key. True when the text changed, so the caller knows to search again; a move,
    /// or a key that is not the line's (Enter, Esc, Up), returns false.
    pub fn key(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let end = self.text.len();
        match key.code {
            KeyCode::Char('a') if ctrl => self.go(0, false),
            KeyCode::Char('e') if ctrl => self.go(end, false),
            KeyCode::Char('u') if ctrl => return self.cut(0..end),
            KeyCode::Char('w') if ctrl => {
                let range = self.selection().unwrap_or(self.word_left()..self.cur);
                return self.cut(range);
            }
            KeyCode::Backspace if alt => {
                let range = self.selection().unwrap_or(self.word_left()..self.cur);
                return self.cut(range);
            }
            KeyCode::Char(c) if !ctrl => {
                if let Some(range) = self.selection() {
                    self.cut(range);
                }
                // A selection stretched and shrunk back to nothing leaves its anchor behind.
                self.anchor = None;
                self.text.insert(self.cur, c);
                self.cur += c.len_utf8();
                return true;
            }
            KeyCode::Backspace => {
                let range = self.selection().unwrap_or(self.left()..self.cur);
                return self.cut(range);
            }
            KeyCode::Delete if alt => {
                let range = self.selection().unwrap_or(self.cur..self.word_right());
                return self.cut(range);
            }
            KeyCode::Delete => {
                let range = self.selection().unwrap_or(self.cur..self.right());
                return self.cut(range);
            }
            KeyCode::Home => self.go(0, shift),
            KeyCode::End => self.go(end, shift),
            KeyCode::Left if ctrl => self.go(0, shift),
            KeyCode::Right if ctrl => self.go(end, shift),
            KeyCode::Left if alt => self.go(self.word_left(), shift),
            KeyCode::Right if alt => self.go(self.word_right(), shift),
            KeyCode::Left if shift => self.go(self.left(), true),
            KeyCode::Right if shift => self.go(self.right(), true),
            // A plain arrow on a selection collapses it to the matching end.
            KeyCode::Left => self.go(self.selection().map_or(self.left(), |r| r.start), false),
            KeyCode::Right => self.go(self.selection().map_or(self.right(), |r| r.end), false),
            _ => {}
        }
        false
    }

    fn go(&mut self, to: usize, extend: bool) {
        if extend {
            self.anchor.get_or_insert(self.cur);
        } else {
            self.anchor = None;
        }
        self.cur = to;
    }

    fn cut(&mut self, range: Range<usize>) -> bool {
        self.anchor = None;
        self.cur = range.start;
        let changed = !range.is_empty();
        self.text.replace_range(range, "");
        changed
    }

    fn left(&self) -> usize {
        self.text[..self.cur]
            .chars()
            .next_back()
            .map_or(0, |c| self.cur - c.len_utf8())
    }

    fn right(&self) -> usize {
        self.text[self.cur..]
            .chars()
            .next()
            .map_or(self.cur, |c| self.cur + c.len_utf8())
    }

    fn word_left(&self) -> usize {
        let head = self.text[..self.cur].trim_end_matches(|c| !is_word(c));
        head.trim_end_matches(is_word).len()
    }

    fn word_right(&self) -> usize {
        let tail = &self.text[self.cur..];
        let tail = tail.trim_start_matches(|c| !is_word(c));
        self.text.len() - tail.trim_start_matches(is_word).len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(e: &mut LineEdit, code: KeyCode, m: KeyModifiers) -> bool {
        e.key(KeyEvent::new(code, m))
    }

    fn typed(s: &str) -> LineEdit {
        let mut e = LineEdit::default();
        for c in s.chars() {
            press(&mut e, KeyCode::Char(c), KeyModifiers::NONE);
        }
        e
    }

    #[test]
    fn inserts_and_deletes_at_the_cursor() {
        let mut e = typed("now");
        assert!(!press(&mut e, KeyCode::Home, KeyModifiers::NONE));
        for c in "func ".chars() {
            assert!(press(&mut e, KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!((&*e, e.cursor()), ("func now", 5));
        assert!(press(&mut e, KeyCode::Delete, KeyModifiers::NONE));
        assert!(press(&mut e, KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(&*e, "funcow");
        press(&mut e, KeyCode::Char('a'), KeyModifiers::CONTROL);
        assert!(!press(&mut e, KeyCode::Backspace, KeyModifiers::NONE));
        press(&mut e, KeyCode::Char('e'), KeyModifiers::CONTROL);
        assert!(!press(&mut e, KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(&*e, "funcow");
    }

    #[test]
    fn moves_and_kills_by_word_in_any_script() {
        let mut e = typed("найти func_now(");
        press(&mut e, KeyCode::Left, KeyModifiers::ALT);
        assert_eq!(&e[e.cursor()..], "func_now(");
        press(&mut e, KeyCode::Left, KeyModifiers::ALT);
        assert_eq!(e.cursor(), 0);
        press(&mut e, KeyCode::Right, KeyModifiers::ALT);
        assert_eq!(&e[..e.cursor()], "найти");
        press(&mut e, KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(&e[..e.cursor()], "найт");
        press(&mut e, KeyCode::End, KeyModifiers::NONE);
        assert!(press(&mut e, KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert_eq!(&*e, "найти ");
        // Option+Backspace is the same cut; Option+Delete is its pair forward.
        assert!(press(&mut e, KeyCode::Backspace, KeyModifiers::ALT));
        assert_eq!(&*e, "");
        let mut e = typed("a bc");
        press(&mut e, KeyCode::Home, KeyModifiers::NONE);
        assert!(press(&mut e, KeyCode::Delete, KeyModifiers::ALT));
        assert_eq!(&*e, " bc");
        assert!(press(&mut e, KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!((&*e, e.cursor()), ("", 0));
    }

    #[test]
    fn a_selection_is_replaced_deleted_or_collapsed() {
        let sel = KeyModifiers::ALT | KeyModifiers::SHIFT;
        let mut e = typed("func now");
        press(&mut e, KeyCode::Left, sel);
        assert_eq!(e.selection(), Some(5..8));
        press(&mut e, KeyCode::Char('x'), KeyModifiers::NONE);
        assert_eq!((&*e, e.selection()), ("func x", None));

        press(
            &mut e,
            KeyCode::Left,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        assert_eq!(e.selection(), Some(0..6));
        press(&mut e, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!((e.cursor(), e.selection()), (6, None));

        // Shrunk back to nothing, it does not come alive under the next char typed.
        press(&mut e, KeyCode::Left, KeyModifiers::SHIFT);
        press(&mut e, KeyCode::Right, KeyModifiers::SHIFT);
        press(&mut e, KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!((&*e, e.selection()), ("func xy", None));
        press(&mut e, KeyCode::Backspace, KeyModifiers::NONE);

        press(&mut e, KeyCode::Left, KeyModifiers::SHIFT);
        assert_eq!(e.selection(), Some(5..6));
        press(&mut e, KeyCode::Home, KeyModifiers::SHIFT);
        assert!(press(&mut e, KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!((&*e, e.cursor(), e.selection()), ("", 0, None));
    }
}
