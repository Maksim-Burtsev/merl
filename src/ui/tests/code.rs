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

/// #185: merl measured `⚠️` one column wide, ratatui draws it two, so the last letter of a
/// full row was on no row and the cursor stood a cell to the left of its char.
#[test]
fn an_emoji_with_a_selector_wraps_and_holds_the_cursor_as_drawn() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(
            PathBuf::from("/tmp/f.md"),
            "\u{26a0}\u{fe0f}abcdefghi\nnext\n".as_bytes(),
        ),
        None,
    );
    app.show_tree = false;
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    // Gutter is two cells, so the text gets ten.
    let mut terminal = Terminal::new(TestBackend::new(12, 5)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        rows(&terminal)[..3],
        ["1 \u{26a0}\u{fe0f} abcdefgh", "i", "2 next"]
    );
    app.key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    app.key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(terminal.get_cursor_position().unwrap().x, 2 + 3);
}

/// #206: a redraw sent the second column of `✔️` as a cell of its own, the backend printed it
/// right after the emoji without moving the cursor (ratatui/ratatui#2651), and the rest of the
/// row landed a column late. That column is never sent, so the backend moves past it.
#[test]
fn a_redraw_never_sends_the_column_under_an_emoji_with_a_selector() {
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(12, 3)).unwrap();
    let mut frame = |text: &str| {
        let mut app = App::new(
            PathBuf::from("/tmp"),
            Tree::default(),
            Vec::new(),
            Buffer::from_bytes(PathBuf::from("/tmp/f.md"), text.as_bytes()),
            None,
        );
        app.show_tree = false;
        let done = terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        done.buffer.clone()
    };
    let before = frame("abcdefgh\n");
    let after = frame("\u{2714}\u{fe0f} Tests\n");
    let sent: Vec<u16> = before
        .diff(&after)
        .into_iter()
        .filter(|&(_, y, _)| y == 0)
        .map(|(x, ..)| x)
        .collect();
    // Gutter is two cells; the emoji covers 2 and 3, and `e` at 6 did not change.
    assert_eq!(sent, [2, 4, 5, 7, 8, 9]);
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
    // On the deleted tint, never greyed by the terminal's `dim` (#144); with no base file to
    // take syntax colours from, in the plain text colour.
    let buf = terminal.backend().buffer();
    let o = (0..8).find(|x| buf[(*x, 1)].symbol() == "o").unwrap();
    assert_eq!((buf[(o, 1)].fg, buf[(o, 1)].bg), (theme.fg, theme.del_bg));
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

/// Down, PgDn and Ctrl+D on the last line scroll in the lines deleted after it, until the last
/// one is on the bottom row; the cursor stays on the text, and Up brings it back on screen (#179).
#[test]
fn every_ghost_after_the_last_line_can_be_scrolled_to() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), b"a\nb\n"),
        None,
    );
    app.show_tree = false;
    app.diff
        .ghosts
        .insert(2, (1..=10).map(|i| format!("g{i}")).collect());
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    // Four rows of code: a, b and ten ghosts are twelve.
    let mut terminal = Terminal::new(TestBackend::new(8, 5)).unwrap();
    let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
    let ctrl = |c| KeyEvent::new(c, KeyModifiers::CONTROL);
    for (k, top) in [
        (key(KeyCode::Down), (0, 0)),
        (key(KeyCode::Down), (1, 0)),
        (key(KeyCode::PageDown), (2, 3)),
        (ctrl(KeyCode::Char('d')), (2, 5)),
        (key(KeyCode::Down), (2, 6)),
        (key(KeyCode::Down), (2, 6)),
        (ctrl(KeyCode::End), (2, 6)),
        (key(KeyCode::Char('w')), (2, 6)),
    ] {
        app.key(k);
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        assert_eq!((app.line, (app.top_line, app.top_row)), (1, top), "{k:?}");
    }
    assert_eq!(
        rows(&terminal)[..4],
        ["\u{258e}g7", "\u{258e}g8", "\u{258e}g9", "\u{258e}g10"]
    );
    // Up leaves the last line, and the view comes back to the cursor.
    app.key(key(KeyCode::Up));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!((app.line, app.top_line, app.top_row), (0, 0, 0));
    assert_eq!(terminal.get_cursor_position().unwrap().y, 0);
}

#[test]
fn a_ghost_longer_than_the_pane_wraps_and_the_text_follows() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), b"a\nb\n"),
        None,
    );
    app.show_tree = false;
    app.diff
        .ghosts
        .insert(1, vec!["  one two three four".into()]);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    // Gutter is two cells, so the text gets twelve: the ghost wraps there like a file line,
    // its second row under its indent, and `b` comes after it.
    let mut terminal = Terminal::new(TestBackend::new(14, 6)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        rows(&terminal)[..4],
        ["1 a", "\u{258e}  one two", "\u{258e}  three four", "2 b"]
    );
    // On the deleted tint on both rows, out to the right edge.
    let buf = terminal.backend().buffer();
    let t = |y: u16| (0..14).find(|x| buf[(*x, y)].symbol() == "t").unwrap();
    assert_eq!(buf[(t(2), 2)].bg, theme.del_bg);
    assert_eq!(buf[(13, 1)].bg, theme.del_bg);
}

#[test]
fn up_and_down_never_land_on_a_wrapped_ghost_row() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), b"a\nb\nc\n"),
        None,
    );
    app.show_tree = false;
    // Twelve cells of text: the ghost is three rows.
    app.diff
        .ghosts
        .insert(1, vec!["one two three four five".into()]);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(14, 6)).unwrap();
    let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    // Down from `a` lands on `b`'s first text row, past the ghost's three.
    app.key(key(KeyCode::Down));
    assert_eq!((app.line, app.cursor_row()), (1, 3));
    app.key(key(KeyCode::Down));
    assert_eq!(app.line, 2);
    // Up from `c` lands on `b`, not on a ghost row; up again on `a`.
    app.key(key(KeyCode::Up));
    assert_eq!((app.line, app.cursor_row()), (1, 3));
    app.key(key(KeyCode::Up));
    assert_eq!(app.line, 0);
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(terminal.get_cursor_position().unwrap().y, 0);
}

#[test]
fn up_brings_scrolled_off_wrapped_ghosts_in_one_row_at_a_time() {
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
        .insert(1, vec!["one two three four five".into()]);
    app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    // Two rows of code: `b` and above it only the ghost's last row.
    let mut terminal = Terminal::new(TestBackend::new(14, 3)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!((app.top_line, app.top_row), (1, 2));
    assert_eq!(rows(&terminal)[..2], ["\u{258e}five", "2 b"]);
    // Up on that line scrolls the hidden ghost rows in one at a time.
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!((app.line, app.top_line, app.top_row), (1, 1, 1));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(rows(&terminal)[..2], ["\u{258e}three four", "\u{258e}five"]);
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!((app.line, app.top_line, app.top_row), (1, 1, 0));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        rows(&terminal)[..2],
        ["\u{258e}one two", "\u{258e}three four"]
    );
    // Then Up leaves for the line above.
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!((app.line, app.top_line, app.top_row), (0, 0, 0));
}

#[test]
fn wrapped_ghosts_after_the_last_line_can_all_be_scrolled_to() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), b"a\nb\n"),
        None,
    );
    app.show_tree = false;
    app.diff.ghosts.insert(
        2,
        vec!["one two three four five".into(), "six seven eight".into()],
    );
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    // Three rows of code: a, b and five ghost rows (three of the first ghost, two of the
    // second) are seven.
    let mut terminal = Terminal::new(TestBackend::new(14, 4)).unwrap();
    let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
    for top in [(0, 0), (1, 0), (2, 0), (2, 1), (2, 2), (2, 2)] {
        app.key(key(KeyCode::Down));
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        assert_eq!((app.line, (app.top_line, app.top_row)), (1, top));
    }
    assert_eq!(
        rows(&terminal)[..3],
        ["\u{258e}five", "\u{258e}six seven", "\u{258e}eight"]
    );
    // Up leaves the last line, and the view comes back to the cursor.
    app.key(key(KeyCode::Up));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!((app.line, app.top_line, app.top_row), (0, 0, 0));
    assert_eq!(terminal.get_cursor_position().unwrap().y, 0);
}

#[test]
fn a_long_ghost_stays_one_row_when_the_file_is_not_wrapped() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), b"a\nb\n"),
        None,
    );
    app.show_tree = false;
    app.diff
        .ghosts
        .insert(1, vec!["one two three four five".into()]);
    app.key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(14, 4)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    // Cut at the pane's right edge like a text line, with `b` right under it.
    assert_eq!(rows(&terminal)[..3], ["1 a", "\u{258e}one two thre", "2 b"]);
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

/// A repository whose `main` has `f.py` and `gone.py`, and a `feature` branch checked out that
/// deleted `gone.py` and changed one operator of `f.py` in the working tree: a real review.
fn review_of_one_operator(tag: &str) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["-c", "user.email=t@t", "-c", "user.name=t"])
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(dir.join("f.py"), "def f(a, b):\n    return a == b\n").unwrap();
    std::fs::write(dir.join("gone.py"), "x = 1\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    git(&["rm", "-q", "gone.py"]);
    git(&["commit", "-q", "-m", "gone"]);
    std::fs::write(dir.join("f.py"), "def f(a, b):\n    return a >= b\n").unwrap();
    let path = dir.join("f.py");
    let mut app = App::new(
        dir.clone(),
        Tree::default(),
        Vec::new(),
        Buffer::load(&path).unwrap(),
        None,
    );
    app.show_tree = false;
    app.start_review(crate::git::Review::open(&dir, None, None).unwrap());
    (dir, app)
}

/// Review paints the diff as GitHub does (#165): the deleted and the added row on their tints out
/// to the right edge, the ghost in the base file's syntax colours, and the word that changed on a
/// stronger tint in `word_fg`. A selection and a find match win over a changed word.
#[test]
fn review_paints_the_word_that_changed_on_both_rows() {
    let (dir, mut app) = review_of_one_operator("words");
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(24, 5)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        rows(&terminal)[..3],
        [
            "1 def f(a, b):",
            "\u{258e}    return a == b",
            "2\u{258e}    return a >= b"
        ]
    );
    // The cursor is on the hunk: the added row is the cursor line.
    assert_eq!(app.line, 1);
    let cell = |t: &Terminal<TestBackend>, x: u16, y: u16| {
        let c = &t.backend().buffer()[(x, y)];
        (c.fg, c.bg)
    };
    let x_of = |t: &Terminal<TestBackend>, y: u16, s: &str| {
        (0..24)
            .find(|x| t.backend().buffer()[(*x, y)].symbol() == s)
            .unwrap()
    };
    // `==` became `>=`: the first `=` is the old word, `>` the new one.
    let (eq, gt, ret) = (
        x_of(&terminal, 1, "="),
        x_of(&terminal, 2, ">"),
        x_of(&terminal, 2, "r"),
    );
    assert_eq!(cell(&terminal, eq, 1), (theme.word_fg, theme.del_word_bg));
    assert_eq!(
        cell(&terminal, gt, 2),
        (theme.word_fg, theme.add_word_bg_hl)
    );
    assert_eq!(cell(&terminal, eq + 1, 1).1, theme.del_bg, "the kept `=`");
    assert_eq!(cell(&terminal, 23, 1).1, theme.del_bg);
    assert_eq!(cell(&terminal, 23, 2).1, theme.add_bg_hl);
    // `return` is a keyword in both rows: the ghost is coloured from the base, like the text.
    let keyword = cell(&terminal, ret, 2).0;
    assert_ne!(keyword, theme.fg);
    assert_eq!(cell(&terminal, ret, 1), (keyword, theme.del_bg));
    assert_eq!(cell(&terminal, ret, 2).1, theme.add_bg_hl);
    // Off the cursor line the added row takes the plain tints.
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(cell(&terminal, gt, 2), (theme.word_fg, theme.add_word_bg));
    assert_eq!(cell(&terminal, 23, 2).1, theme.add_bg);
    // Selected, the changed word shows the selection; a find match on it shows the match.
    app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.col = app.buf.lines[1].find('>').unwrap();
    app.key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(cell(&terminal, gt, 2).1, theme.selection);
    app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.find_re = Some(regex::Regex::new(">").unwrap());
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(cell(&terminal, gt, 2).1, theme.find_bg);
    // A file the branch deleted is all deleted rows.
    app.jump_to(&dir.join("gone.py"), 0);
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(rows(&terminal)[0], "1\u{2581}x = 1");
    assert_eq!(cell(&terminal, 23, 0).1, theme.del_bg_hl);
    let _ = std::fs::remove_dir_all(dir);
}

/// Outside a review the gutter marks lines against the index, and nothing is tinted.
#[test]
fn outside_a_review_added_lines_are_not_tinted() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), b"a\nb\n"),
        None,
    );
    app.show_tree = false;
    app.diff.marks.insert(1, Mark::Added);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(8, 3)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(rows(&terminal)[1], "2\u{258e}b");
    assert_eq!(terminal.backend().buffer()[(2, 1)].bg, theme.bg);
}

/// The changed word of a wrapped ghost is painted on whichever row it lands, and with `w` it
/// moves with the text when the view scrolls sideways.
#[test]
fn a_changed_word_is_painted_on_a_wrapped_or_scrolled_ghost() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), b"a\nalpha beta gamma omega\n"),
        None,
    );
    app.show_tree = false;
    app.diff
        .ghosts
        .insert(1, vec!["alpha beta gamma delta".into()]);
    app.diff.pairs.insert(1, (1, 0));
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    // Twelve cells of text: `delta` is on the ghost's second row.
    let mut terminal = Terminal::new(TestBackend::new(14, 6)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        rows(&terminal)[1..3],
        ["\u{258e}alpha beta", "\u{258e}gamma delta"]
    );
    let buf = terminal.backend().buffer();
    let d = (0..14).find(|x| buf[(*x, 2)].symbol() == "d").unwrap();
    assert_eq!(buf[(d, 2)].bg, theme.del_word_bg);
    assert_eq!(buf[(d - 2, 2)].bg, theme.del_bg, "`gamma` is kept");
    // Not wrapped and scrolled to the end of the line, `delta` is painted where it now stands.
    app.key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
    app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let buf = terminal.backend().buffer();
    let d = (0..14).find(|x| buf[(*x, 1)].symbol() == "d").unwrap();
    assert_eq!(buf[(d, 1)].bg, theme.del_word_bg);
    assert!(app.left > 0);
}
