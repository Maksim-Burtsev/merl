//! Tests for [`crate::ui::preview`].

use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use crate::buffer::Buffer;
use crate::tree::Tree;

use super::rows;

/// A code block is painted with the syntaxes and the theme of the files merl opens, and the
/// theme `T` picks repaints it; the status bar names the view.
#[test]
fn code_blocks_take_the_theme_and_another_theme_repaints_them() {
    let text = b"# Code\n\n```rust\nfn main() {}\n```\n";
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/notes.md"), text),
        None,
    );
    app.show_tree = false;
    app.key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
    let mut seen = Vec::new();
    for name in ["tokyonight-moon", "github-light"] {
        app.theme = name.into();
        let theme = crate::theme::load(name).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(40, 5)).unwrap();
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        let shown = rows(&terminal);
        assert_eq!(shown[2], "fn main() {}");
        assert!(shown[4].contains("[preview]"), "{shown:?}");
        // The gutter is two columns and the code one more in from its tint.
        let fg = terminal.backend().buffer()[(3, 2)].fg;
        let lines = ["fn main() {}".to_string()];
        let want = crate::buffer::highlight_lines("rust", &lines, &theme).unwrap()[0][0]
            .0
            .fg;
        assert_eq!(Some(fg), want, "{name}");
        seen.push(fg);
    }
    assert_ne!(seen[0], seen[1]);
}
