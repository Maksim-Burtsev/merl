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
        let mut block = Buffer::block("rust", vec!["fn main() {}".to_string()]);
        block.highlight_to(0, &theme);
        let want = block.hl[0][0].0.fg;
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
    let mut block = Buffer::block("rust", vec![line.to_string()]);
    block.highlight_to(0, &theme);
    let spans = &block.hl;
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

#[test]
fn a_long_code_block_is_highlighted_as_far_as_it_is_drawn() {
    let body = "let x = 1;\n".repeat(1000);
    let text = format!("```rust\n{body}```\n");
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/long.md"), text.as_bytes()),
        None,
    );
    app.show_tree = false;
    app.key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let hl = app.preview.as_ref().unwrap().code[0].hl.len();
    assert!(
        (9..100).contains(&hl),
        "{hl} lines highlighted for a screen of 9"
    );
}

/// The status bar leaves out `nowrap` in the preview, which always wraps.
#[test]
fn the_preview_status_says_no_nowrap() {
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/notes.md"), b"# Notes\n"),
        None,
    );
    app.show_tree = false;
    for c in ['w', 'p'] {
        app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(60, 4)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let status = rows(&terminal)[3].clone();
    assert!(
        status.contains("[preview]") && !status.contains("nowrap"),
        "{status}"
    );
}

fn settle(app: &mut App, terminal: &mut Terminal<TestBackend>, theme: &crate::theme::Theme) {
    for _ in 0..3 {
        terminal.draw(|f| super::draw(f, app, theme)).unwrap();
        for job in app.diagrams.jobs() {
            app.diagrams.done(crate::mermaid::render(job));
        }
    }
    terminal.draw(|f| super::draw(f, app, theme)).unwrap();
}

#[test]
fn a_mermaid_picture_is_placed_on_its_rows_cut_by_the_pane_and_hidden_under_an_overlay() {
    let text = "# Flow\n\n> ```mermaid\n> graph TD\n>   A[Start] --> B[Done]\n> ```\n";
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/flow.md"), text.as_bytes()),
        None,
    );
    app.show_tree = false;
    app.diagrams = crate::mermaid::Diagrams::with_cell(Some((10, 20)));
    let theme = crate::theme::load("tokyonight-moon").unwrap();
    let mut terminal = Terminal::new(TestBackend::new(60, 8)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    app.key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert!(app.diagrams.want.is_empty());
    assert!(rows(&terminal).iter().any(|r| r.contains("A[Start]")));

    settle(&mut app, &mut terminal, &theme);
    let shown = rows(&terminal);
    assert!(!shown.iter().any(|r| r.contains("A[Start]")), "{shown:?}");
    let want = app.diagrams.want.clone();
    let [place] = &want[..] else {
        panic!("one picture: {want:?}");
    };
    assert_eq!(
        (place.placement, place.x, place.y, place.crop_y),
        (1, 5, 2, 0)
    );
    assert_eq!(place.rows, 5);
    assert!(place.cols > 3);
    let pic = app
        .diagrams
        .pic("graph TD\n  A[Start] --> B[Done]")
        .unwrap();
    assert!(
        place.crop_h < pic.height(),
        "the pane's bottom cuts the picture"
    );

    app.key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
    settle(&mut app, &mut terminal, &theme);
    assert!(app.diagrams.want.is_empty());
}
