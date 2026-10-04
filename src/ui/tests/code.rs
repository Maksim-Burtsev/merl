//! Tests for [`crate::ui::code`].

use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use ratatui::style::Color;

use crate::app::App;
use crate::buffer::Buffer;
use crate::git::{Mark, TextLine};
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
    assert_eq!(app.file_selection(), Some(((0, 16), (1, 0))));
    assert_eq!(
        paint(&mut app),
        ["----------", "----------", "--########", ".........."]
    );
}

#[test]
fn find_spans_cut_the_syntax_spans_they_cover() {
    use ratatui::style::{Color, Modifier, Style};

    let syntax = Style::new().fg(Color::Blue).add_modifier(Modifier::ITALIC);
    let find = Style::new().bg(Color::Yellow);
    let lit = syntax.bg(Color::Yellow);
    // "abcdefgh": one syntax span over 0..4, matches at 2..3 (inside it) and 5..6 (in the
    // gap the highlighter left behind). The match keeps the syntax colour and style under its
    // tint (#480); in the gap there is none to keep.
    let out = super::with_find(&[(syntax, 0..4)], &[2..3, 5..6], find);
    assert_eq!(
        out,
        [
            (syntax, 0..2),
            (lit, 2..3),
            (syntax, 3..4),
            (Style::new(), 4..5),
            (find, 5..6),
        ]
    );
    // A match across a span's end is cut there, each part over its own text.
    assert_eq!(
        super::with_find(&[(syntax, 0..4)], &[0..2, 2..6], find),
        [(lit, 0..2), (lit, 2..4), (find, 4..6)]
    );
    // A theme's own match foreground wins over the syntax colour.
    let named = find.fg(Color::Black);
    assert_eq!(
        super::with_find(&[(syntax, 0..4)], &[1..2, 5..6], named),
        [
            (syntax, 0..1),
            (syntax.patch(named), 1..2),
            (syntax, 2..4),
            (Style::new(), 4..5),
            (named, 5..6)
        ]
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

/// The lines a review deleted are drawn above the line that replaced them, and the cursor walks
/// them as it walks the file's own (#439).
#[test]
fn ghost_lines_draw_above_their_line_and_the_cursor_walks_them() {
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
    // Down steps onto each deleted line, then onto `b`; Up comes back the same way.
    let key = |app: &mut App, c| app.key(KeyEvent::new(c, KeyModifiers::NONE));
    for (c, at, y) in [
        (KeyCode::Down, TextLine::Deleted(1, 0), 1),
        (KeyCode::Down, TextLine::Deleted(1, 1), 2),
        (KeyCode::Down, TextLine::File(1), 3),
        (KeyCode::Up, TextLine::Deleted(1, 1), 2),
        (KeyCode::Up, TextLine::Deleted(1, 0), 1),
        (KeyCode::Up, TextLine::File(0), 0),
    ] {
        key(&mut app, c);
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        assert_eq!(
            (app.at(), terminal.get_cursor_position().unwrap().y),
            (at, y)
        );
    }
    // The cursor's deleted line is the cursor line, on the deleted tint: gutter and row.
    key(&mut app, KeyCode::Down);
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let buf = terminal.backend().buffer();
    assert_eq!(
        (buf[(0, 1)].bg, buf[(7, 1)].bg),
        (theme.line_hl, theme.del_bg_hl)
    );
    assert_eq!(buf[(7, 2)].bg, theme.del_bg);
}

/// Up in a pane too short for the ghosts scrolls them in one row at a time, the cursor on each.
#[test]
fn up_scrolls_onto_the_ghosts_one_row_at_a_time() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), b"a\nb\nc\nd\n"),
        None,
    );
    app.show_tree = false;
    app.diff.ghosts.insert(1, vec!["old".into()]);
    for _ in 0..4 {
        app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(8, 3)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!((app.top_line, app.top_row), (2, 0));
    for (top, shown) in [
        ((2, 0), ["3 c", "4 d"]),
        ((1, 1), ["2 b", "3 c"]),
        ((1, 0), ["\u{258e}old", "2 b"]),
        ((0, 0), ["1 a", "\u{258e}old"]),
    ] {
        app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        assert_eq!((app.top_line, app.top_row), top);
        assert_eq!(rows(&terminal)[..2], shown);
        assert_eq!(terminal.get_cursor_position().unwrap().y, 0);
    }
}

#[test]
fn down_walks_the_ghosts_then_lands_on_the_first_text_row() {
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
    for row in 0..3 {
        app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.cursor_at(), (1, row));
    }
    assert_eq!((app.at(), app.col), (TextLine::File(1), 0));
}

/// Down, PgDn and Ctrl+D go on from the last line onto the lines deleted after it, down to the
/// last of them, which Ctrl+End lands on (#179).
#[test]
fn every_ghost_after_the_last_line_can_be_walked_to() {
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
    for (k, at) in [
        (key(KeyCode::Down), TextLine::File(1)),
        (key(KeyCode::Down), TextLine::Deleted(2, 0)),
        (key(KeyCode::PageDown), TextLine::Deleted(2, 4)),
        (ctrl(KeyCode::Char('d')), TextLine::Deleted(2, 6)),
        (key(KeyCode::Down), TextLine::Deleted(2, 7)),
        (ctrl(KeyCode::Home), TextLine::File(0)),
        (ctrl(KeyCode::End), TextLine::Deleted(2, 9)),
        (key(KeyCode::Down), TextLine::Deleted(2, 9)),
        (key(KeyCode::Char('w')), TextLine::Deleted(2, 9)),
    ] {
        app.key(k);
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        assert_eq!(app.at(), at, "{k:?}");
    }
    assert_eq!(
        rows(&terminal)[..4],
        ["\u{258e}g7", "\u{258e}g8", "\u{258e}g9", "\u{258e}g10"]
    );
    assert_eq!(terminal.get_cursor_position().unwrap().y, 3);
    // The status bar reads the deleted line's own number, and Up takes the cursor off it.
    app.key(key(KeyCode::Up));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(app.at(), TextLine::Deleted(2, 8));
    assert_eq!(terminal.get_cursor_position().unwrap().y, 2);
}

/// Up and Down walk a ghost that wraps row by row, as they walk a wrapped line of the file.
#[test]
fn up_and_down_walk_a_wrapped_ghost_row_by_row() {
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
    for (c, at, y) in [
        (KeyCode::Down, (TextLine::Deleted(1, 0), 0), 1),
        (KeyCode::Down, (TextLine::Deleted(1, 0), 8), 2),
        (KeyCode::Down, (TextLine::Deleted(1, 0), 19), 3),
        (KeyCode::Down, (TextLine::File(1), 0), 4),
        (KeyCode::Up, (TextLine::Deleted(1, 0), 19), 3),
    ] {
        app.key(key(c));
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        let cursor = terminal.get_cursor_position().unwrap().y;
        assert_eq!(((app.at(), app.col), cursor), (at, y), "{c:?}");
    }
}

/// Up in a pane shorter than a wrapped ghost scrolls its rows in one at a time, the cursor on
/// each.
#[test]
fn up_scrolls_a_wrapped_ghost_in_one_row_at_a_time() {
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
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    // Two rows of code.
    let mut terminal = Terminal::new(TestBackend::new(14, 3)).unwrap();
    for _ in 0..4 {
        app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    }
    assert_eq!(rows(&terminal)[..2], ["\u{258e}five", "2 b"]);
    for shown in [
        ["\u{258e}five", "2 b"],
        ["\u{258e}three four", "\u{258e}five"],
        ["\u{258e}one two", "\u{258e}three four"],
        ["1 a", "\u{258e}one two"],
    ] {
        app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        assert_eq!(rows(&terminal)[..2], shown);
        assert_eq!(terminal.get_cursor_position().unwrap().y, 0);
    }
}

#[test]
fn wrapped_ghosts_after_the_last_line_can_all_be_walked_to() {
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
    for _ in 0..8 {
        app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    }
    assert_eq!((app.at(), app.cursor_row()), (TextLine::Deleted(2, 1), 4));
    assert_eq!(
        rows(&terminal)[..3],
        ["\u{258e}five", "\u{258e}six seven", "\u{258e}eight"]
    );
    assert_eq!(terminal.get_cursor_position().unwrap().y, 2);
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
        (3, Mark::Deleted),
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
    assert_eq!(head(3), "4\u{258e}d");
    let buf = terminal.backend().buffer();
    assert_eq!(buf[(1, 0)].fg, Color::Green);
    assert_eq!(buf[(1, 1)].fg, Color::Blue);
    assert_eq!(buf[(1, 2)].fg, Color::Red);
    assert_eq!(buf[(1, 3)].fg, Color::Red);
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
    let cell = |t: &Terminal<TestBackend>, x: u16, y: u16| {
        let c = &t.backend().buffer()[(x, y)];
        (c.fg, c.bg)
    };
    // The cursor is on the hunk, which starts with the deleted row (#439): that row is the
    // cursor line. Down, and the added row is.
    assert_eq!(app.at(), TextLine::Deleted(1, 0));
    assert_eq!(cell(&terminal, 23, 1).1, theme.del_bg_hl);
    assert_eq!(cell(&terminal, 23, 2).1, theme.add_bg);
    app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(app.at(), TextLine::File(1));
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
    assert_eq!(rows(&terminal)[0], "1\u{258e}x = 1");
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

/// #248: inside a long function its first line stays pinned on top on a band of the cursor
/// line's colour, and the code starts below it. The cursor walking down and back up is always on
/// a row of its own line under the band, so no line is ever behind the pinned ones; on the
/// function's own line the pin goes.
#[test]
fn the_enclosing_function_stays_pinned_above_the_code() {
    let mut text = String::from("fn long() {\n");
    for i in 0..40 {
        text += &format!("    let a{i} = {i};\n");
    }
    text += "}\n";
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/f.rs"), text.as_bytes()),
        None,
    );
    app.show_tree = false;
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    // Eleven rows of code, the first of them the pinned declaration.
    let mut terminal = Terminal::new(TestBackend::new(30, 12)).unwrap();
    let mut press = |app: &mut App, code: KeyCode| {
        app.key(KeyEvent::new(code, KeyModifiers::NONE));
        terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
        let y = terminal.get_cursor_position().unwrap().y as usize;
        let band = (0..30).all(|x| terminal.backend().buffer()[(x, 0)].bg == theme.line_hl);
        (rows(&terminal), y, band)
    };
    let (screen, _, _) = press(&mut app, KeyCode::Null);
    assert_eq!(screen[..2], ["1 fn long() {", "2     let a0 = 0;"]);
    for code in [KeyCode::Down, KeyCode::Up] {
        for _ in 0..20 {
            let (screen, y, band) = press(&mut app, code);
            let pinned = app.top_line > 0;
            assert_eq!(screen[0], "1 fn long() {");
            // Unpinned, the top row is lit only as the cursor line.
            assert_eq!(band, pinned || app.line == 0);
            assert!(
                y >= if pinned { 1 } else { 0 },
                "the cursor is under the band"
            );
            assert!(screen[y].starts_with(&format!("{} ", app.line + 1)));
        }
        if code == KeyCode::Down {
            // On the bottom row, the rows above it start right under the band.
            let (screen, y, _) = press(&mut app, KeyCode::Null);
            assert_eq!((app.line, y), (20, 10));
            assert_eq!(screen[1], "12     let a10 = 10;");
        }
    }
    let (screen, _, _) = press(&mut app, KeyCode::Null);
    assert_eq!(screen[..2], ["1 fn long() {", "2     let a0 = 0;"]);
}

/// #248: `c` puts the hunk under the pinned header, its deleted line with it, not behind it.
#[test]
fn a_hunk_inside_a_long_function_lands_below_the_pinned_header() {
    let dir = std::env::temp_dir().join(format!("merl-pinned-hunk-{}", std::process::id()));
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
    let body = |changed: &[usize]| {
        let mut t = String::from("def long():\n");
        for i in 0..30 {
            let v = if changed.contains(&i) { i * 10 } else { i };
            t += &format!("    a{i} = {v}\n");
        }
        t
    };
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(dir.join("f.py"), body(&[])).unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    std::fs::write(dir.join("f.py"), body(&[2, 20])).unwrap();
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
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(30, 12)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    app.key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let screen = rows(&terminal);
    let y = terminal.get_cursor_position().unwrap().y as usize;
    assert_eq!(app.line, 21);
    assert_eq!(screen[0], "1 def long():");
    let buf = terminal.backend().buffer();
    assert!((0..30).all(|x| buf[(x, 0)].bg == theme.line_hl));
    // On the deleted line the hunk starts with (#439), under the band.
    assert_eq!(
        screen[y..=y + 1],
        ["\u{258e}    a20 = 20", "22\u{258e}    a20 = 200"]
    );
    assert!(y > 0, "the deleted line is under the band");
    let _ = std::fs::remove_dir_all(dir);
}

/// An app on `text` as a Rust file, the tree hidden, and a `w`×`h` terminal to draw it on.
fn pinned_app(text: &str, w: u16, h: u16) -> (App, Terminal<TestBackend>) {
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/f.rs"), text.as_bytes()),
        None,
    );
    app.show_tree = false;
    (app, Terminal::new(TestBackend::new(w, h)).unwrap())
}

/// Presses `code`, draws, and gives back the screen and the cursor's row on it.
fn press(
    app: &mut App,
    terminal: &mut Terminal<TestBackend>,
    code: KeyCode,
) -> (Vec<String>, usize) {
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    app.key(KeyEvent::new(code, KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
    (
        rows(terminal),
        terminal.get_cursor_position().unwrap().y as usize,
    )
}

/// The number a screen row starts with, the line it shows.
fn line_no(row: &str) -> usize {
    row.split(' ').next().unwrap().parse().unwrap()
}

/// #248: under a pinned header a page is the rows of code, not the pane: paging down and back up
/// through a long function shows every line of it, each page starting where the last one ended.
#[test]
fn a_page_under_a_pinned_header_skips_no_line() {
    let mut text = String::from("fn long() {\n");
    for i in 0..60 {
        text += &format!("    let a{i} = {i};\n");
    }
    text += "}\n";
    // Eleven rows of code under the status line, one pinned inside the function.
    let (mut app, mut terminal) = pinned_app(&text, 30, 12);
    let (mut screen, _) = press(&mut app, &mut terminal, KeyCode::Null);
    for (code, pages) in [(KeyCode::PageDown, 4), (KeyCode::PageUp, 4)] {
        for _ in 0..pages {
            let code_rows = |s: &[String]| {
                let first = usize::from(s[0] == "1 fn long() {" && line_no(&s[1]) > 2);
                (line_no(&s[first]), line_no(&s[10]))
            };
            let (top, bottom) = code_rows(&screen);
            (screen, _) = press(&mut app, &mut terminal, code);
            let (next_top, next_bottom) = code_rows(&screen);
            match code {
                KeyCode::PageDown => assert!(next_top <= bottom + 1, "{bottom} then {next_top}"),
                _ => assert!(next_bottom + 1 >= top, "{top} then {next_bottom}"),
            }
        }
    }
    assert_eq!(app.top_line, 0);
}

/// #248: the top a jump scrolls to is the first that shows the cursor under its own pins, even
/// where they fall as the top moves down: from the last line of one method (two pins) to its
/// `}` (one). A jump from the top of the file to `b13` in the next method puts the top on that
/// `}`, the cursor on the bottom row, not a row further with the cursor a row above it.
#[test]
fn a_jump_scrolls_to_the_first_top_that_fits_under_its_pins() {
    let mut text = String::from("impl S {\n    fn m() {\n");
    for i in 0..30 {
        text += &format!("        let a{i} = {i};\n");
    }
    text += "    }\n    fn n() {\n";
    for i in 0..30 {
        text += &format!("        let b{i} = {i};\n");
    }
    text += "    }\n}\n";
    // Seventeen rows of code: two pins at most.
    let (mut app, mut terminal) = pinned_app(&text, 40, 18);
    press(&mut app, &mut terminal, KeyCode::Null);
    // 0-based 47 is `let b13`; the `}` of `m` is 32.
    app.line = 47;
    let (screen, y) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!((app.top_line, y), (32, 16));
    assert_eq!(screen[..2], ["1 impl S {", "33     }"]);
}

/// #248: a pane under 8 rows keeps them all for the code; from 16 up it pins two, the innermost:
/// the `impl` and the `fn`, not the `mod` around them.
#[test]
fn a_short_pane_pins_nothing_and_a_tall_one_the_two_innermost() {
    let mut text = String::from("mod m {\n    impl S {\n        fn f() {\n");
    for i in 0..40 {
        text += &format!("            let a{i} = {i};\n");
    }
    text += "        }\n    }\n}\n";
    let (mut app, mut terminal) = pinned_app(&text, 40, 7);
    for _ in 0..20 {
        press(&mut app, &mut terminal, KeyCode::Down);
    }
    let (screen, _) = press(&mut app, &mut terminal, KeyCode::Null);
    assert!(app.top_line > 3);
    assert_eq!(line_no(&screen[0]), app.top_line + 1, "code on the top row");
    let (mut app, mut terminal) = pinned_app(&text, 40, 19);
    for _ in 0..30 {
        press(&mut app, &mut terminal, KeyCode::Down);
    }
    let (screen, _) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(screen[..2], ["2     impl S {", "3         fn f() {"]);
    assert_eq!(line_no(&screen[2]), app.top_line + 1);
}

/// #248: not wrapped and scrolled sideways, the pinned header is cut at the columns the code
/// under it is, with `‹` and `›` where it goes on past the edges.
#[test]
fn an_unwrapped_pinned_header_is_cut_where_the_code_is() {
    let mut text = String::from("fn long(first: usize, second: usize) {\n");
    for i in 0..30 {
        text += &format!("    let a{i} = first + second + {i};\n");
    }
    text += "}\n";
    let (mut app, mut terminal) = pinned_app(&text, 24, 12);
    press(&mut app, &mut terminal, KeyCode::Char('w'));
    for _ in 0..20 {
        press(&mut app, &mut terminal, KeyCode::Down);
    }
    let (screen, _) = press(&mut app, &mut terminal, KeyCode::End);
    // One column at either edge is the marker's, as on every row of code.
    let (left, w) = (app.left, app.view_w);
    let header = "fn long(first: usize, second: usize) {";
    assert!(left > 0 && header.len() > left + w);
    assert_eq!(
        screen[0],
        format!("1 \u{2039}{}\u{203a}", &header[left + 1..left + w - 1])
    );
}

/// An app on `text`, the tree hidden, drawn on a `w`×`h` terminal with the cursor at the start
/// of `line`.
fn cut_app(text: &str, w: u16, h: u16, line: usize) -> (App, Terminal<TestBackend>) {
    let (mut app, mut terminal) = pinned_app(text, w, h);
    app.line = line;
    press(&mut app, &mut terminal, KeyCode::Null);
    (app, terminal)
}

/// #283: a line longer than merl draws ends in a dim `…` right after its last drawn char. When
/// its last wrapped row is full, the `…` takes a row of its own rather than cover a char.
#[test]
fn a_cut_line_ends_in_a_dim_ellipsis_when_wrapped() {
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let text = format!("{}\nsecond\n", "a".repeat(20_005));
    // Seven columns of text: 20,000 is 2,857 full rows and one `a`.
    let (mut app, mut terminal) = cut_app(&text, 9, 4, 1);
    assert_eq!(rows(&terminal)[..3], ["aaaaaaa", "a\u{2026}", "2 second"]);
    let buf = terminal.backend().buffer();
    assert_eq!(
        (buf[(3, 1)].fg, buf[(3, 1)].bg),
        (theme.gutter_fg, theme.bg)
    );
    // Selected through the line's end, the `…` is on the selection, as the newline is.
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let buf = terminal.backend().buffer();
    assert_eq!(
        (buf[(3, 1)].symbol(), buf[(3, 1)].bg),
        ("\u{2026}", theme.selection)
    );
    // Ten columns: the last row is full, and the `…` stands alone under it.
    let (mut app, mut terminal) = cut_app(&text, 12, 4, 1);
    assert_eq!(rows(&terminal)[..3], ["aaaaaaaaaa", "\u{2026}", "2 second"]);
    // That row is one of the line's: Up lands on it, and the end of the drawn part is the `…`.
    let (_, y) = press(&mut app, &mut terminal, KeyCode::Up);
    assert_eq!((app.line, y), (0, 1));
    app.col = app.buf.shown(0).len();
    press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(terminal.get_cursor_position().unwrap(), (2, 1).into());
    // An indented line's rows after the first start under its text: a last row full at that
    // indent puts the `…` on a row of its own, under the text too.
    let text = format!("    {}\nsecond\n", "a".repeat(19_999));
    let (_, terminal) = cut_app(&text, 10, 4, 1);
    assert_eq!(rows(&terminal)[..3], ["aaaa", "\u{2026}", "2 second"]);
    assert_eq!(terminal.backend().buffer()[(6, 1)].symbol(), "\u{2026}");
    // A line of exactly what is drawn is not cut and has no `…`.
    let (_, terminal) = cut_app(&"a".repeat(20_000), 9, 3, 0);
    assert!(!rows(&terminal).concat().contains('\u{2026}'));
}

/// #283: not wrapped, the `…` shows while the end of the cut line is on screen, and counts as a
/// column of the line: where it would fall past the edge, `›` says the line goes on.
#[test]
fn a_cut_line_ends_in_a_dim_ellipsis_when_not_wrapped() {
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let text = format!("top\n{}\n", "b".repeat(20_005));
    let (mut app, mut terminal) = cut_app(&text, 12, 3, 1);
    press(&mut app, &mut terminal, KeyCode::Char('w'));
    assert_eq!(rows(&terminal)[1], "2 bbbbbbbbb\u{203a}");
    // The end of the drawn part in view: the `…` in the column after it, on the cursor line.
    app.col = app.buf.shown(1).len();
    let (screen, _) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(screen[1], "2 \u{2039}bbbbbbbb\u{2026}");
    let buf = terminal.backend().buffer();
    assert_eq!(
        (buf[(11, 1)].fg, buf[(11, 1)].bg),
        (theme.gutter_fg, theme.line_hl)
    );
    // The text reaching the right edge exactly: no room for the `…`, so `›` stands there.
    app.left = 19_990;
    app.col = 19_995;
    let (screen, _) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(app.left, 19_990);
    assert_eq!(screen[1], "2 \u{2039}bbbbbbbb\u{203a}");
}

/// #283: a deleted line in a review is cut as a text line is, and ends in the same `…`, wrapped
/// or not.
#[test]
fn a_cut_ghost_ends_in_a_dim_ellipsis() {
    let long = |c: &str| c.repeat(20_005);
    let text = format!("top\n{}\n", long("b"));
    let (mut app, mut terminal) = pinned_app(&text, 9, 4);
    app.diff.ghosts.insert(1, vec![long("a")]);
    // Wrapped, the last rows of the ghost are right above its line.
    app.line = 1;
    press(&mut app, &mut terminal, KeyCode::Null);
    app.top_line = 1;
    app.top_row = app.ghost_rows(1) - 2;
    let (screen, _) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(
        screen[..3],
        ["\u{258e}aaaaaaa", "\u{258e}a\u{2026}", "2 bbbbbbb"]
    );
    // Not wrapped and scrolled to the end, the ghost's end is on screen too.
    press(&mut app, &mut terminal, KeyCode::Char('w'));
    app.col = app.buf.shown(1).len();
    app.top_row = 0;
    let (screen, _) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(
        screen[..2],
        ["\u{258e}aaaaaa\u{2026}", "2 \u{2039}bbbbb\u{2026}"]
    );
    // A full last row puts the ghost's `…` on a row of its own too.
    let (mut app, mut terminal) = pinned_app(&text, 12, 4);
    app.diff.ghosts.insert(1, vec![long("a")]);
    app.line = 1;
    press(&mut app, &mut terminal, KeyCode::Null);
    app.top_line = 1;
    app.top_row = app.ghost_rows(1) - 2;
    let (screen, _) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(
        screen[..3],
        ["\u{258e}aaaaaaaaaa", "\u{258e}\u{2026}", "2 bbbbbbbbbb"]
    );
}

/// #283: not wrapped, a pinned header cut at 20 KB ends in the `…` as its line does.
#[test]
fn a_cut_pinned_header_ends_in_a_dim_ellipsis() {
    let mut text = format!("fn long() {{ // {}\n", "x".repeat(20_005));
    for _ in 0..20 {
        text += "    let a = 1;\n";
    }
    text += &format!("    // {}\n}}\n", "b".repeat(20_005));
    let (mut app, mut terminal) = cut_app(&text, 12, 10, 21);
    press(&mut app, &mut terminal, KeyCode::Char('w'));
    app.col = app.buf.shown(21).len();
    let (screen, y) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(screen[0], "1 \u{2039}xxxxxxx\u{2026}");
    assert_eq!(screen[y], "22 \u{2039}bbbbbbb\u{2026}");
}

/// #283: not wrapped, the `…` stands on the column after the text whenever that column is in
/// view, with no char of the line left in view too: `left` follows the cursor on a wider line.
#[test]
fn a_cut_line_scrolled_to_its_end_still_shows_its_ellipsis() {
    let text = format!(
        "{}\u{6f22}bbbbb\n{}{}\n",
        "a".repeat(19_997),
        "\t".repeat(5_000),
        "x".repeat(100)
    );
    let (mut app, mut terminal) = cut_app(&text, 12, 4, 1);
    app.diff.ghosts.insert(1, vec!["a".repeat(20_005)]);
    press(&mut app, &mut terminal, KeyCode::Char('w'));
    // Line 1 is 5,000 tabs, 20,000 columns, then `x`s: col 5,000 + n is column 20,000 + n.
    let mut at = |left: usize, col: usize| {
        (app.left, app.col) = (left, col);
        let (screen, _) = press(&mut app, &mut terminal, KeyCode::Null);
        assert_eq!(app.left, left);
        screen[..2].to_vec()
    };
    // The wide char at the end straddles `‹`: the `…` comes after its hidden half.
    assert_eq!(
        at(19_997, 5_002),
        ["1 \u{2039} \u{2026}", "\u{258e}aaa\u{2026}"]
    );
    assert_eq!(
        at(19_998, 5_003),
        ["1 \u{2039}\u{2026}", "\u{258e}aa\u{2026}"]
    );
    // Past the end of line 0, `‹` alone; the ghost has no `‹`, so its `…` is in column 0.
    assert_eq!(at(19_999, 5_004), ["1 \u{2039}", "\u{258e}a\u{2026}"]);
    assert_eq!(at(20_000, 5_004), ["1 \u{2039}", "\u{258e}\u{2026}"]);
}

/// A deletion taller than the pane is read line by line: Down draws every one of its lines
/// (#279), and PgDn, PgUp, Ctrl+D and Ctrl+U move a page or half of one over it (#296). On
/// one, the status bar reads its number in the file at the base, negative.
#[test]
fn a_deletion_taller_than_the_pane_is_walked_and_paged() {
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
        .insert(1, (1..=30).map(|i| format!("d{i}")).collect());
    app.diff.ghost_from.insert(1, 1);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    // Seven rows of code.
    let mut terminal = Terminal::new(TestBackend::new(40, 8)).unwrap();
    let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
    let ctrl = |c| KeyEvent::new(c, KeyModifiers::CONTROL);
    let mut seen = std::collections::HashSet::new();
    for _ in 0..31 {
        app.key(key(KeyCode::Down));
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        seen.extend(rows(&terminal));
    }
    assert_eq!(app.at(), TextLine::File(1));
    let unseen: Vec<_> = (1..=30)
        .map(|i| format!("\u{258e}d{i}"))
        .filter(|r| !seen.iter().any(|row| row.trim_end() == r))
        .collect();
    assert!(unseen.is_empty(), "never drawn: {unseen:?}");
    for (k, at) in [
        (ctrl(KeyCode::Home), TextLine::File(0)),
        (key(KeyCode::PageDown), TextLine::Deleted(1, 6)),
        (key(KeyCode::PageDown), TextLine::Deleted(1, 13)),
        (key(KeyCode::PageUp), TextLine::Deleted(1, 6)),
        (ctrl(KeyCode::Char('d')), TextLine::Deleted(1, 9)),
        (ctrl(KeyCode::Char('u')), TextLine::Deleted(1, 6)),
    ] {
        app.key(k);
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        assert_eq!(app.at(), at, "{k:?}");
    }
    // d7 is line 8 of the file at the base: the deletion starts at its line 2.
    assert!(
        rows(&terminal)[7].contains("-8:1"),
        "{:?}",
        rows(&terminal)[7]
    );
}

/// The lines deleted above the first line of a file: Ctrl+Home lands on the first of them, Up
/// from the first line goes onto the last of them.
#[test]
fn a_deletion_above_the_first_line_is_the_top_of_the_text() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), b"a\nb\n"),
        None,
    );
    app.show_tree = false;
    app.diff.ghosts.insert(0, vec!["x".into(), "y".into()]);
    let key = |app: &mut App, c, m| app.key(KeyEvent::new(c, m));
    key(&mut app, KeyCode::End, KeyModifiers::CONTROL);
    assert_eq!(app.at(), TextLine::File(1));
    key(&mut app, KeyCode::Home, KeyModifiers::CONTROL);
    assert_eq!(app.at(), TextLine::Deleted(0, 0));
    key(&mut app, KeyCode::Down, KeyModifiers::NONE);
    key(&mut app, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(app.at(), TextLine::File(0));
    key(&mut app, KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(app.at(), TextLine::Deleted(0, 1));
}

/// A move across a tall deletion is a far one for the jump history: `[` comes back to where it
/// started, however few file lines it passed.
#[test]
fn a_move_across_a_tall_deletion_is_a_stop_of_its_own() {
    let dir = std::env::temp_dir().join(format!("merl-439-hist-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("f.txt");
    std::fs::write(&path, "a\n").unwrap();
    let mut app = App::new(
        dir.clone(),
        Tree::default(),
        Vec::new(),
        Buffer::load(&path).unwrap(),
        None,
    );
    app.show_tree = false;
    app.diff
        .ghosts
        .insert(0, (1..=30).map(|i| format!("d{i}")).collect());
    let key = |app: &mut App, c, m| app.key(KeyEvent::new(c, m));
    key(&mut app, KeyCode::Home, KeyModifiers::CONTROL);
    assert_eq!(app.at(), TextLine::Deleted(0, 0));
    key(&mut app, KeyCode::End, KeyModifiers::CONTROL);
    assert_eq!(app.at(), TextLine::File(0));
    key(&mut app, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(app.at(), TextLine::Deleted(0, 0));
    let _ = std::fs::remove_dir_all(dir);
}

/// #412: scrolled into a line that wraps, `t` widens the pane and the line takes fewer rows. The
/// top inside it is made valid for the new wrap first, so the cursor is drawn on the line the
/// status bar names, and a letter typed there lands on the line it is drawn on.
#[test]
fn a_wider_pane_draws_the_cursor_on_its_own_line() {
    let words: Vec<String> = (0..60).map(|i| format!("word{i:03}")).collect();
    let mut text = format!("LONG {}\n", words.join(" "));
    for i in 2..40 {
        text += &format!("line {i}\n");
    }
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/a.txt"), text.as_bytes()),
        None,
    );
    // Next to the tree the first line wraps into eight rows; the view scrolls into them.
    let mut terminal = Terminal::new(TestBackend::new(100, 12)).unwrap();
    for _ in 0..17 {
        press(&mut app, &mut terminal, KeyCode::Down);
    }
    assert_eq!((app.line, app.top_line, app.top_row), (10, 0, 7));
    let (screen, y) = press(&mut app, &mut terminal, KeyCode::Char('t'));
    assert_eq!(screen[y], "11 line 11");
    for code in [KeyCode::Enter, KeyCode::Char('X'), KeyCode::Esc] {
        press(&mut app, &mut terminal, code);
    }
    let (screen, y) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(screen[y], "11 Xline 11");
    assert_eq!(app.buf.lines[10], "Xline 11");
}

/// Every hidden char on #401's list is drawn as its tag, on the tag's amber, and takes the
/// columns of it; the ZWJ inside an emoji stays part of the emoji. A deleted line in a review
/// shows its tags too.
#[test]
fn hidden_chars_are_drawn_as_their_tags() {
    let all = [
        '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}', '\u{2066}', '\u{2067}',
        '\u{2068}', '\u{2069}', '\u{200e}', '\u{200f}', '\u{061c}', '\u{200b}', '\u{2060}',
        '\u{feff}',
    ];
    let mut text: String = all.iter().map(|c| format!("x{c}\n")).collect();
    text.push_str("\u{1f468}\u{200d}\u{1f4bb}!\n");
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), text.as_bytes()),
        None,
    );
    app.show_tree = false;
    app.diff.ghosts.insert(0, vec!["old\u{202e}".into()]);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let h = all.len() as u16 + 3;
    let mut terminal = Terminal::new(TestBackend::new(20, h)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let r = rows(&terminal);
    assert_eq!(r[0], "\u{258e}old<202e>");
    let buf = terminal.backend().buffer();
    for (i, c) in all.iter().enumerate() {
        let y = i as u16 + 1;
        assert_eq!(r[y as usize], format!("{} x<{:04x}>", i + 1, *c as u32));
        for x in 4..10 {
            assert_eq!(
                (buf[(x, y)].fg, buf[(x, y)].bg),
                (theme.tag_fg, theme.tag_bg)
            );
        }
        assert_ne!(buf[(3, y)].bg, theme.tag_bg);
    }
    let y = all.len() + 1;
    // The emoji keeps its two cells (the second is blank in the buffer), `!` after them.
    assert_eq!(r[y], "16 \u{1f468}\u{200d}\u{1f4bb} !");
    assert_eq!(buf[(5, y as u16)].symbol(), "!");
}

/// The cursor steps over a tag in one press and lands after all of its columns; Delete takes
/// the hidden char out whole (#401).
#[test]
fn the_cursor_steps_over_a_tag_and_delete_removes_it() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.txt"), "a\u{202e}b\n".as_bytes()),
        None,
    );
    app.show_tree = false;
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();
    let key = |app: &mut App, code| app.key(KeyEvent::new(code, KeyModifiers::NONE));
    key(&mut app, KeyCode::Right);
    key(&mut app, KeyCode::Right);
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    // Gutter 2, `a` 1, the tag 6: on the `b`.
    assert_eq!(terminal.get_cursor_position().unwrap().x, 2 + 1 + 6);
    key(&mut app, KeyCode::Left);
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Delete);
    assert_eq!(app.buf.lines[0], "ab");
}

/// #287: a binary file shows no line of text and no gutter, only a dimmed note in the middle
/// of the pane; the status bar says `read-only` as before.
#[test]
fn a_binary_file_is_an_empty_pane_with_a_centred_note() {
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/logo.png"), b"\x89PNG\0\x01"),
        None,
    );
    app.show_tree = false;
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(60, 11)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let rows = rows(&terminal);
    let note = "binary file, not shown";
    assert_eq!(rows.iter().position(|r| r == note), Some(3), "{rows:#?}");
    assert!(
        rows[..10].iter().all(|r| r.is_empty() || r == note),
        "{rows:#?}"
    );
    assert!(rows[10].contains("read-only"), "{}", rows[10]);
    let buf = terminal.backend().buffer();
    let x = (0..60).find(|&x| buf[(x, 3)].symbol() == "b").unwrap();
    assert_eq!(x, (60 - note.len() as u16) / 2);
    assert_eq!(buf[(x, 3)].fg, theme.ghost_fg);
}

#[test]
fn a_fold_draws_its_tail_and_the_lines_under_it_in_colour() {
    let body: String = (0..30).map(|i| format!("    \"x{i}\": {i},\n")).collect();
    let text = format!("ITEMS = {{\n{body}}}\ndef g():\n    pass\n");
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/f.py"), text.as_bytes()),
        None,
    );
    app.show_tree = false;
    app.key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(30, 6)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        rows(&terminal)[..2],
        ["1 ITEMS = { \u{22ef} }", "33 def g():"]
    );
    let buf = terminal.backend().buffer();
    assert_ne!(
        buf[(3, 1)].fg,
        buf[(7, 1)].fg,
        "line 33 is drawn uncoloured"
    );
}
