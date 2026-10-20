#[cfg(not(target_family = "wasm"))]
pub use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[cfg(target_family = "wasm")]
pub use shim::*;

#[cfg(target_family = "wasm")]
mod shim {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub enum KeyCode {
        Backspace,
        Enter,
        Left,
        Right,
        Up,
        Down,
        Home,
        End,
        PageUp,
        PageDown,
        Tab,
        BackTab,
        Delete,
        Insert,
        F(u8),
        Char(char),
        Null,
        Esc,
    }

    impl KeyCode {
        pub fn is_char(&self, c: char) -> bool {
            *self == Self::Char(c)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
    pub struct KeyModifiers(u8);

    impl KeyModifiers {
        pub const NONE: Self = Self(0);
        pub const SHIFT: Self = Self(1);
        pub const CONTROL: Self = Self(2);
        pub const ALT: Self = Self(4);
        pub const SUPER: Self = Self(8);
        pub const fn union(self, o: Self) -> Self {
            Self(self.0 | o.0)
        }
        pub const fn bits(self) -> u8 {
            self.0
        }
        pub const fn from_bits_truncate(b: u8) -> Self {
            Self(b & 15)
        }
        pub fn contains(self, o: Self) -> bool {
            self.0 & o.0 == o.0
        }
        pub fn intersects(self, o: Self) -> bool {
            self.0 & o.0 != 0
        }
        pub fn is_empty(self) -> bool {
            self.0 == 0
        }
        pub const fn difference(self, o: Self) -> Self {
            Self(self.0 & !o.0)
        }
        pub fn remove(&mut self, o: Self) {
            self.0 &= !o.0;
        }
        pub fn insert(&mut self, o: Self) {
            self.0 |= o.0;
        }
    }

    impl std::ops::BitOr for KeyModifiers {
        type Output = Self;
        fn bitor(self, o: Self) -> Self {
            Self(self.0 | o.0)
        }
    }

    impl std::ops::BitAnd for KeyModifiers {
        type Output = Self;
        fn bitand(self, o: Self) -> Self {
            Self(self.0 & o.0)
        }
    }

    impl std::ops::Not for KeyModifiers {
        type Output = Self;
        fn not(self) -> Self {
            Self(!self.0 & 15)
        }
    }

    impl std::ops::BitOrAssign for KeyModifiers {
        fn bitor_assign(&mut self, o: Self) {
            self.0 |= o.0;
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum KeyEventKind {
        Press,
        Repeat,
        Release,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct KeyEvent {
        pub code: KeyCode,
        pub modifiers: KeyModifiers,
        pub kind: KeyEventKind,
    }

    impl KeyEvent {
        pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
            Self {
                code,
                modifiers,
                kind: KeyEventKind::Press,
            }
        }
    }

    impl From<KeyCode> for KeyEvent {
        fn from(code: KeyCode) -> Self {
            Self::new(code, KeyModifiers::NONE)
        }
    }
}
