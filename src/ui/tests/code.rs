//! Tests for [`crate::ui::code`].

use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use ratatui::style::Color;

use crate::app::App;
use crate::buffer::Buffer;
use crate::git::Mark;
use crate::tree::Tree;

use super::rows;

/// `w` cuts long lines at the edge: `\u{203a}` and `\u{2039}` say there is more, End brings the
/// end of the line into view and Home the start.
#[test]
fn unwrapped_lines_are_cut_at_the_edge_and_follow_the_cursor() {
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(
            PathBuf::from("/demo/f.txt"),
            b"0123456789abcdefghijKLMN\nshort\n",
        ),
        None,
    );
    app.show_tree = false;
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let paint = |app: &mut App, code: KeyCode| -> Vec<String> {
        app.key(KeyEvent::new(code, KeyModifiers::NONE));
        // Gutter is two cells, so the text gets twelve.
        let mut terminal = Terminal::new(TestBackend::new(14, 5)).unwrap();
        terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
        rows(&terminal)[..3].to_vec()
    };
    assert_eq!(
        paint(&mut app, KeyCode::Null),
        ["1 0123456789ab", "cdefghijKLMN", "2 short"]
    );
    assert_eq!(
        paint(&mut app, KeyCode::Char('w')),
        ["1 0123456789a\u{203a}", "2 short", ""]
    );
    // The end of the line stops at the right edge, the cursor in the last cell.
    assert_eq!(
        paint(&mut app, KeyCode::End),
        ["1 \u{2039}efghijKLMN", "2 \u{2039}", ""]
    );
    assert_eq!((app.left, app.cursor_x()), (13, 24));
    assert_eq!(
        paint(&mut app, KeyCode::Home),
        ["1 0123456789a\u{203a}", "2 short", ""]
    );
    // Wrapped again, nothing stays scrolled.
    paint(&mut app, KeyCode::End);
    assert_eq!(
        paint(&mut app, KeyCode::Char('w')),
        ["1 0123456789ab", "cdefghijKLMN", "2 short"]
    );
    assert_eq!(app.left, 0);
}

#[test]
fn selection_background_covers_partial_edges_and_pads_inner_lines() {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/f.txt"), b"zz\nabcd\nef\nghij\n"),
        None,
    );
    app.show_tree = false;
    for (code, m) in [
        (KeyCode::Down, KeyModifiers::NONE),
        (KeyCode::Right, KeyModifiers::NONE),
        (KeyCode::Right, KeyModifiers::NONE),
        (KeyCode::Down, KeyModifiers::SHIFT),
        (KeyCode::Down, KeyModifiers::SHIFT),
    ] {
        app.key(KeyEvent::new(code, m));
    }
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    // Gutter is two cells; `#` marks a cell with the selection background, `-` one with the
    // cursor line highlight.
    let paint = |app: &mut App| -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(12, 5)).unwrap();
        terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
        let buf = terminal.backend().buffer();
        (0..4)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| match buf[(x, y)].bg {
                        bg if bg == theme.selection => '#',
                        bg if bg == theme.line_hl => '-',
                        _ => '.',
                    })
                    .collect()
            })
            .collect()
    };
    // The line above the selection stays clean; the cursor line keeps its highlight.
    assert_eq!(
        paint(&mut app),
        [
            "............",
            "....########",
            "..##########",
            "--##--------"
        ]
    );
    // Ending at the start of the cursor line selects none of its text: highlighted in the
    // gutter only, as in VS Code, or past the selection it would pass for selected text.
    app.key(KeyEvent::new(
        KeyCode::Left,
        KeyModifiers::SHIFT | KeyModifiers::CONTROL,
    ));
    assert_eq!(paint(&mut app)[3], "--..........");
    // Back on the anchor nothing is selected, and the cursor line is highlighted again.
    for (code, m) in [
        (KeyCode::Esc, KeyModifiers::NONE),
        (KeyCode::Up, KeyModifiers::NONE),
        (KeyCode::Up, KeyModifiers::NONE),
        (KeyCode::Down, KeyModifiers::SHIFT),
        (KeyCode::Up, KeyModifiers::SHIFT),
    ] {
        app.key(KeyEvent::new(code, m));
    }
    assert_eq!(paint(&mut app)[1], "------------");
    // With no selection at all, too.
    app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(app.selection(), None);
    assert_eq!(paint(&mut app)[1], "------------");
}

#[test]
fn selecting_inside_a_wrapped_line_keeps_its_highlight_on_the_other_rows() {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    // One 24-column line wraps onto three 8-column rows (the gutter takes two cells).
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(
            PathBuf::from("/demo/f.txt"),
            b"aaaaaaaabbbbbbbbcccccccc
z
",
        ),
        None,
    );
    app.show_tree = false;
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let paint = |app: &mut App| -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(10, 5)).unwrap();
        terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
        let buf = terminal.backend().buffer();
        (0..4)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| match buf[(x, y)].bg {
                        bg if bg == theme.selection => '#',
                        bg if bg == theme.line_hl => '-',
                        _ => '.',
                    })
                    .collect()
            })
            .collect()
    };
    // A first draw sets the pane width the wrapping and the remembered column depend on.
    paint(&mut app);
    // Cursor on the middle row, five cells in.
    for _ in 0..12 {
        app.key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    }
    assert_eq!(
        paint(&mut app)[..3],
        ["----------", "----------", "----------"]
    );
    // Shift+Up selects half a row up to the cursor; the rest of the line stays highlighted
    // instead of flashing back to the plain background.
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT));
    assert_eq!(
        paint(&mut app),
        ["------####", "--####----", "----------", ".........."]
    );
    // The same from below: Shift+Up out of the start of the next line selects the tail of
    // the wrapped line, and its rows above the cursor keep the highlight too.
    for (code, m) in [
        (KeyCode::Esc, KeyModifiers::NONE),
        (KeyCode::Down, KeyModifiers::NONE),
        (KeyCode::Down, KeyModifiers::NONE),
        (KeyCode::Down, KeyModifiers::NONE),
        (KeyCode::Home, KeyModifiers::NONE),
        (KeyCode::Up, KeyModifiers::SHIFT),
    ] {
        app.key(KeyEvent::new(code, m));
    }
    assert_eq!(app.selection(), Some(((0, 16), (1, 0))));
    assert_eq!(
        paint(&mut app),
        ["----------", "----------", "--########", ".........."]
    );
}

#[test]
fn find_spans_cut_the_syntax_spans_they_cover() {
    use ratatui::style::{Color, Style};

    let syntax = Style::new().fg(Color::Blue);
    let find = Style::new().bg(Color::Yellow);
    // "abcdefgh": one syntax span over 0..4, matches at 2..3 (inside it) and 5..6 (in the
    // gap the highlighter left behind).
    let out = super::with_find(&[(syntax, 0..4)], &[2..3, 5..6], find);
    assert_eq!(
        out,
        [(syntax, 0..2), (find, 2..3), (syntax, 3..4), (find, 5..6),]
    );
    // Matches covering a whole span replace it.
    assert_eq!(
        super::with_find(&[(syntax, 0..4)], &[0..2, 2..6], find),
        [(find, 0..2), (find, 2..6)]
    );
    assert_eq!(
        super::with_find(&[(syntax, 0..4)], &[], find),
        [(syntax, 0..4)]
    );
}

/// A line past the render clip: the cursor still sits on a drawn row of that line, not on
/// the row of the next line, so End on a minified file does not lose the cursor.
#[test]
fn cursor_stays_on_a_drawn_row_of_a_clipped_line() {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let text = format!("{}\nsecond\n", "a".repeat(30_000));
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/f.txt"), text.as_bytes()),
        None,
    );
    app.show_tree = false;
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(82, 10)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    app.key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let y = terminal.get_cursor_position().unwrap().y;
    let buf = terminal.backend().buffer();
    assert_eq!(
        buf[(2, y)].symbol(),
        "a",
        "cursor row is a row of the long line"
    );
    assert_eq!(
        buf[(2, 0)].symbol(),
        "a",
        "the long line is what the view shows"
    );
}

#[test]
fn tabs_draw_four_wide_and_the_status_shows_the_edit_state() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.go"), b"\tx := 1\n"),
        None,
    );
    app.show_tree = false;
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(40, 4)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert!(
        rows(&terminal)[0].starts_with("1     x := 1"),
        "{:?}",
        rows(&terminal)[0]
    );
    assert!(rows(&terminal)[3].contains("[code]"));
    app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    app.key(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let status = &rows(&terminal)[3];
    assert!(
        status.contains("f.go \u{25cf}") && status.contains("[edit]  Tab"),
        "{status:?}"
    );
    assert_eq!(terminal.get_cursor_position().unwrap().x, 2 + 4 + 7);
}

#[test]
fn wrapped_rows_and_the_cursor_on_them_start_under_the_text() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(
            PathBuf::from("/tmp/f.md"),
            b"- Celery lost tasks on restart\n",
        ),
        None,
    );
    app.show_tree = false;
    for _ in 0..16 {
        app.key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    }
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(20, 4)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let cursor = terminal.get_cursor_position().unwrap();
    let buf = terminal.backend().buffer();
    let row = |y: u16| (0..20).map(|x| buf[(x, y)].symbol()).collect::<String>();
    // "tasks" fits a row, so it moves down whole, and the row starts past the list marker.
    assert_eq!(row(0), "1 - Celery lost     ");
    assert_eq!(row(1), "    tasks on restart");
    // On the "s" of "tasks": gutter 2, indent 2, "ta" 2.
    assert_eq!((cursor.x, cursor.y), (6, 1));
}

#[test]
fn ghost_lines_draw_above_their_line_and_the_cursor_skips_them() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), b"a\nb\nc\n"),
        None,
    );
    app.show_tree = false;
    app.diff
        .ghosts
        .insert(1, vec!["old1".into(), "old2".into()]);
    app.diff.marks.insert(1, Mark::Added);
    app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(8, 6)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        rows(&terminal)[..5],
        ["1 a", "\u{258e}old1", "\u{258e}old2", "2\u{258e}b", "3 c"]
    );
    // Greyed by the theme's own readable grey, never by the terminal's `dim` (#144).
    let buf = terminal.backend().buffer();
    let o = (0..8).find(|x| buf[(*x, 1)].symbol() == "o").unwrap();
    assert_eq!(buf[(o, 1)].fg, theme.ghost_fg);
    assert!(!buf[(o, 1)].modifier.contains(ratatui::style::Modifier::DIM));
    assert_eq!(terminal.get_cursor_position().unwrap().y, 4);
    // Up from `c` lands on `b`, not on a ghost; up again on `a`.
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(app.line, 1);
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(app.line, 0);
    // Two rows for `b` and what is above it: the last ghost, not the first.
    app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    let mut terminal = Terminal::new(TestBackend::new(8, 3)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(rows(&terminal)[..2], ["\u{258e}old2", "2\u{258e}b"]);
    // Up on that line scrolls the hidden ghost in instead of leaving the line.
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!((app.line, app.top_line, app.top_row), (1, 1, 0));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(rows(&terminal)[..2], ["\u{258e}old1", "\u{258e}old2"]);
    // Then Up leaves for the line above, which comes on screen with row 0.
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!((app.line, app.top_line, app.top_row), (0, 0, 0));
}

#[test]
fn scrolling_up_onto_a_ghosted_line_shows_its_ghosts_first() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), b"a\nb\nc\nd\n"),
        None,
    );
    app.show_tree = false;
    app.diff.ghosts.insert(1, vec!["old".into()]);
    for _ in 0..3 {
        app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(8, 3)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!((app.top_line, app.top_row), (2, 0));
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(rows(&terminal)[..2], ["\u{258e}old", "2 b"]);
    assert_eq!(terminal.get_cursor_position().unwrap().y, 1);
}

#[test]
fn down_onto_a_ghosted_wrapped_line_lands_on_its_first_text_row() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(
            PathBuf::from("/tmp/f.txt"),
            b"a\nword word word word word word word word\n",
        ),
        None,
    );
    app.show_tree = false;
    app.diff.ghosts.insert(1, vec!["g1".into(), "g2".into()]);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(14, 8)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!((app.line, app.col), (1, 0));
    assert_eq!(app.cursor_row(), 2);
}

#[test]
fn ghosts_after_the_last_line_are_drawn_under_it() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), b"a\nb\n"),
        None,
    );
    app.show_tree = false;
    app.diff.ghosts.insert(2, vec!["gone".into()]);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(8, 4)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(rows(&terminal)[..3], ["1 a", "2 b", "\u{258e}gone"]);
}

#[test]
fn git_marks_sit_between_the_number_and_the_text() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), b"a\nb\nc\nd\n"),
        None,
    );
    app.show_tree = false;
    app.diff.marks = [
        (0, Mark::Added),
        (1, Mark::Changed),
        (2, Mark::DeletedBelow),
    ]
    .into();
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(20, 5)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let r = rows(&terminal);
    let head = |i: usize| r[i].chars().take(3).collect::<String>();
    assert_eq!(head(0), "1\u{258e}a");
    assert_eq!(head(1), "2\u{258e}b");
    assert_eq!(head(2), "3\u{2581}c");
    assert_eq!(head(3), "4 d");
    let buf = terminal.backend().buffer();
    assert_eq!(buf[(1, 0)].fg, Color::Green);
    assert_eq!(buf[(1, 1)].fg, Color::Blue);
    assert_eq!(buf[(1, 2)].fg, Color::Red);
}

#[test]
fn gutter_width() {
    assert_eq!(super::digits(0), 1);
    assert_eq!(super::digits(9), 1);
    assert_eq!(super::digits(10), 2);
    assert_eq!(super::digits(999), 3);
    assert_eq!(super::digits(1000), 4);
}
