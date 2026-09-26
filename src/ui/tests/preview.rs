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

/// A code line wider than the pane keeps its colours on the rows it wraps onto: each row takes
/// the colours of the part of the line it shows.
#[test]
fn a_wrapped_code_line_keeps_its_colours() {
    let line = "let answer = compute(1, 2) + \"forty\";";
    let text = format!("```rust\n{line}\n```\n");
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/notes.md"), text.as_bytes()),
        None,
    );
    app.show_tree = false;
    app.key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(30, 4)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let shown = rows(&terminal);
    assert_eq!(shown[..2], ["let answer = compute(1, 2)", "+ \"forty\";"]);
    // The string on the second row: past two columns of gutter, one of padding and `+ `.
    let lines = [line.to_string()];
    let spans = crate::buffer::highlight_lines("rust", &lines, &theme).unwrap();
    let at = line.find('"').unwrap();
    let want = spans[0].iter().find(|(_, r)| r.contains(&at)).unwrap().0.fg;
    assert_eq!(Some(terminal.backend().buffer()[(5, 1)].fg), want);
    assert_ne!(
        want, spans[0][0].0.fg,
        "the string is not in the colour of `let`"
    );
}

/// A megabyte of JSON on one line, in a code block: drawn plain past what a line shows, as in the
/// source, so the first frame and every theme the `T` picker passes over come at once.
#[test]
fn a_megabyte_line_in_a_code_block_draws_at_once() {
    let huge = format!("{{\"a\": \"{}\"}}", "x y".repeat(370_000));
    let text = format!("# Dump\n\n```json\n{huge}\n```\n");
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/dump.md"), text.as_bytes()),
        None,
    );
    app.show_tree = false;
    app.key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
    for name in ["tokyonight-moon", "github-light"] {
        app.theme = name.into();
        let theme = crate::theme::load(name).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(60, 6)).unwrap();
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        assert!(rows(&terminal)[2].starts_with("{\"a\": \"x yx y"), "{name}");
    }
}
