//! Tests for [`crate::ui::welcome`].

use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use crate::app::App;
use crate::buffer::Buffer;
use crate::tree::Tree;

use super::rows;

/// Every welcome action still exists in `KEYS`, the screen shows the key next to it, and a
/// pane too small for the logo drops the logo before it drops the keys.
#[test]
fn welcome_screen_lists_keys_and_shrinks_before_it_clips() {
    let hints = super::welcome_hints();
    assert_eq!(hints.len(), super::WELCOME_ACTIONS.len());
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mk = || {
        App::new(
            PathBuf::from("/demo"),
            Tree::default(),
            Vec::new(),
            Buffer::empty(),
            None,
        )
    };

    let mut app = mk();
    app.show_tree = false;
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let text = rows(&terminal).join("\n");
    for (k, a) in &hints {
        assert!(text.contains(k) && text.contains(a), "{k} {a}\n{text}");
    }
    assert!(text.contains(super::LOGO[0].trim()), "no logo\n{text}");

    let mut app = mk();
    app.show_tree = false;
    let mut terminal = Terminal::new(TestBackend::new(80, 8)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let text = rows(&terminal).join("\n");
    assert!(text.contains("Quit"), "keys must survive\n{text}");
    assert!(
        !text.contains(super::LOGO[0].trim()),
        "logo must go first\n{text}"
    );
}
