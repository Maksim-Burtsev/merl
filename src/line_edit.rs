use std::ops::{Deref, Range};

use crate::app::{is_word, next_char, prev_char};
use crate::key::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Default)]
pub struct LineEdit {
    text: String,
    cursor_byte: usize,
    anchor: Option<usize>,
    max_chars: Option<usize>,
}

impl Deref for LineEdit {
    type Target = str;
    fn deref(&self) -> &str {
        &self.text
    }
}

impl LineEdit {
    pub fn selected(text: &str) -> Self {
        Self {
            text: text.into(),
            cursor_byte: text.len(),
            anchor: Some(0),
            max_chars: None,
        }
    }

    pub fn typed(text: &str) -> Self {
        Self {
            anchor: None,
            ..Self::selected(text)
        }
    }

    pub fn capped(self, chars: usize) -> Self {
        Self {
            max_chars: Some(chars),
            ..self
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn cursor(&self) -> usize {
        self.cursor_byte
    }

    pub fn selection(&self) -> Option<Range<usize>> {
        let a = self.anchor.filter(|&a| a != self.cursor_byte)?;
        Some(a.min(self.cursor_byte)..a.max(self.cursor_byte))
    }

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
                let range = self
                    .selection()
                    .unwrap_or_else(|| self.word_left()..self.cursor_byte);
                return self.cut(range);
            }
            KeyCode::Backspace if alt => {
                let range = self
                    .selection()
                    .unwrap_or_else(|| self.word_left()..self.cursor_byte);
                return self.cut(range);
            }
            KeyCode::Char(c) if !ctrl => return self.insert(c.encode_utf8(&mut [0; 4])),
            KeyCode::Backspace => {
                let range = self
                    .selection()
                    .unwrap_or_else(|| self.left()..self.cursor_byte);
                return self.cut(range);
            }
            KeyCode::Delete if alt => {
                let range = self
                    .selection()
                    .unwrap_or_else(|| self.cursor_byte..self.word_right());
                return self.cut(range);
            }
            KeyCode::Delete => {
                let range = self
                    .selection()
                    .unwrap_or_else(|| self.cursor_byte..self.right());
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
            KeyCode::Left => self.go(
                self.selection().map_or_else(|| self.left(), |r| r.start),
                false,
            ),
            KeyCode::Right => self.go(
                self.selection().map_or_else(|| self.right(), |r| r.end),
                false,
            ),
            _ => {}
        }
        false
    }

    pub fn insert(&mut self, text: &str) -> bool {
        if text.is_empty() {
            return false;
        }
        let cut = self.selection().map(|range| self.cut(range)).is_some();
        self.anchor = None;
        let room = self.max_chars.map_or(usize::MAX, |cap| {
            cap.saturating_sub(self.text.chars().count())
        });
        let fits: String = text.chars().take(room).collect();
        self.text.insert_str(self.cursor_byte, &fits);
        self.cursor_byte += fits.len();
        cut || !fits.is_empty()
    }

    pub fn put(&mut self, to: usize) {
        self.go(to, false);
    }

    fn go(&mut self, to: usize, extend: bool) {
        if extend {
            self.anchor.get_or_insert(self.cursor_byte);
        } else {
            self.anchor = None;
        }
        self.cursor_byte = to;
    }

    fn cut(&mut self, range: Range<usize>) -> bool {
        self.anchor = None;
        self.cursor_byte = range.start;
        let changed = !range.is_empty();
        self.text.replace_range(range, "");
        changed
    }

    fn left(&self) -> usize {
        prev_char(&self.text, self.cursor_byte)
    }

    fn right(&self) -> usize {
        next_char(&self.text, self.cursor_byte)
    }

    #[expect(
        clippy::string_slice,
        reason = "LineEdit keeps cursor_byte on a char boundary"
    )]
    fn word_left(&self) -> usize {
        let head = self.text[..self.cursor_byte].trim_end_matches(|c| !is_word(c));
        head.trim_end_matches(is_word).len()
    }

    #[expect(
        clippy::string_slice,
        reason = "LineEdit keeps cursor_byte on a char boundary"
    )]
    fn word_right(&self) -> usize {
        let tail = &self.text[self.cursor_byte..];
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
        assert_eq!(e.get(e.cursor()..), Some("func_now("));
        press(&mut e, KeyCode::Left, KeyModifiers::ALT);
        assert_eq!(e.cursor(), 0);
        press(&mut e, KeyCode::Right, KeyModifiers::ALT);
        assert_eq!(e.get(..e.cursor()), Some("найти"));
        press(&mut e, KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(e.get(..e.cursor()), Some("найт"));
        press(&mut e, KeyCode::End, KeyModifiers::NONE);
        assert!(press(&mut e, KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert_eq!(&*e, "найти ");
        assert!(press(&mut e, KeyCode::Backspace, KeyModifiers::ALT));
        assert_eq!(&*e, "", "Option+Backspace is the same cut");
        let mut e = typed("a bc");
        press(&mut e, KeyCode::Home, KeyModifiers::NONE);
        assert!(press(&mut e, KeyCode::Delete, KeyModifiers::ALT));
        assert_eq!(&*e, " bc", "Option+Delete is its pair forward");
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

        press(&mut e, KeyCode::Left, KeyModifiers::SHIFT);
        press(&mut e, KeyCode::Right, KeyModifiers::SHIFT);
        press(&mut e, KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(
            (&*e, e.selection()),
            ("func xy", None),
            "a selection shrunk back to nothing does not come alive under the next char typed"
        );
        press(&mut e, KeyCode::Backspace, KeyModifiers::NONE);

        press(&mut e, KeyCode::Left, KeyModifiers::SHIFT);
        assert_eq!(e.selection(), Some(5..6));
        press(&mut e, KeyCode::Home, KeyModifiers::SHIFT);
        assert!(press(&mut e, KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!((&*e, e.cursor(), e.selection()), ("", 0, None));
    }

    #[test]
    fn a_line_opens_selected_typed_or_capped() {
        let mut e = LineEdit::selected("old");
        assert_eq!(e.selection(), Some(0..3));
        press(&mut e, KeyCode::Char('n'), KeyModifiers::NONE);
        assert_eq!(&*e, "n", "typing replaces a line opened selected");
        let mut e = LineEdit::selected("old");
        assert!(!press(&mut e, KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(
            (&*e, e.cursor(), e.selection()),
            ("old", 0, None),
            "a plain Left collapses a selection to its start"
        );
        let e = LineEdit::typed("old");
        assert_eq!((e.cursor(), e.selection()), (3, None));
        let mut e = LineEdit::typed("ab").capped(3);
        assert!(press(&mut e, KeyCode::Char('c'), KeyModifiers::NONE));
        assert!(!press(&mut e, KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(&*e, "abc", "typing past the cap does nothing");
        let mut e = LineEdit::typed("a").capped(3);
        assert!(e.insert("bcdef"));
        assert_eq!(&*e, "abc", "a paste is cut to what fits");
    }

    #[test]
    fn only_a_change_of_the_text_reports_true() {
        let mut e = LineEdit::selected("old");
        for code in [KeyCode::Enter, KeyCode::Esc, KeyCode::Up, KeyCode::End] {
            assert!(!press(&mut e, code, KeyModifiers::NONE), "{code:?}");
        }
        let mut e = LineEdit::selected("old");
        assert!(!e.insert(""));
        assert_eq!(
            e.selection(),
            Some(0..3),
            "an empty insert keeps the selection"
        );
    }
}
