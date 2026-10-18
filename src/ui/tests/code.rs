use std::fmt::Write as _;
use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use ratatui::style::{Color, Modifier};

use crate::app::App;
use crate::buffer::Buffer;
use crate::git::{Mark, TextLine};
use crate::tree::Tree;

use super::rows;

fn no_cursor(name: &str) -> crate::theme::Theme {
    crate::theme::Theme {
        cursor: ratatui::style::Style::new(),
        typing_cursor: ratatui::style::Style::new(),
        ..crate::theme::load(name).unwrap()
    }
}

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
    assert_eq!(
        paint(&mut app, KeyCode::End),
        ["1 \u{2039}efghijKLMN", "2 \u{2039}", ""],
        "End stops the end of the line at the right edge"
    );
    assert_eq!(
        (app.left, app.cursor_x()),
        (13, 24),
        "the cursor in the last cell"
    );
    assert_eq!(
        paint(&mut app, KeyCode::Home),
        ["1 0123456789a\u{203a}", "2 short", ""]
    );
    paint(&mut app, KeyCode::End);
    assert_eq!(
        paint(&mut app, KeyCode::Char('w')),
        ["1 0123456789ab", "cdefghijKLMN", "2 short"]
    );
    assert_eq!(app.left, 0, "wrapped again, nothing stays scrolled");
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
    let theme = no_cursor(crate::theme::DEFAULT);
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
    assert_eq!(
        paint(&mut app),
        [
            "............",
            "....########",
            "..##########",
            "..##........"
        ],
        "the line above the selection stays clean; while a selection is on screen the cursor \
         line has no band, as in Neovim's Visual mode (#554)"
    );
    app.key(KeyEvent::new(
        KeyCode::Left,
        KeyModifiers::SHIFT | KeyModifiers::CONTROL,
    ));
    assert_eq!(
        paint(&mut app)[3],
        "............",
        "ending at the start of the cursor line selects none of its text, and the line has no band"
    );
    for (code, m) in [
        (KeyCode::Esc, KeyModifiers::NONE),
        (KeyCode::Up, KeyModifiers::NONE),
        (KeyCode::Up, KeyModifiers::NONE),
        (KeyCode::Down, KeyModifiers::SHIFT),
        (KeyCode::Up, KeyModifiers::SHIFT),
    ] {
        app.key(KeyEvent::new(code, m));
    }
    assert_eq!(
        paint(&mut app)[1],
        "------------",
        "back on the anchor nothing is selected, and the cursor line is highlighted again"
    );
    app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(app.selection(), None);
    assert_eq!(
        paint(&mut app)[1],
        "------------",
        "with no selection at all, the cursor line is highlighted"
    );
}

#[test]
fn under_a_selection_the_line_number_is_drawn_as_cursor_line_nr_alone() {
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/f.txt"), b"abc\n"),
        None,
    );
    app.show_tree = false;
    let theme = crate::theme::load("iceberg").unwrap();
    assert_eq!(
        (theme.bg, theme.line_hl, theme.cursor_gutter_bg()),
        (
            Color::Rgb(0x16, 0x18, 0x21),
            Color::Rgb(0x1e, 0x21, 0x32),
            Color::Rgb(0x2a, 0x31, 0x58)
        ),
        "iceberg's CursorLineNr has a background of its own"
    );
    let mut terminal = Terminal::new(TestBackend::new(10, 3)).unwrap();
    let mut gutter = |app: &mut App| {
        terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
        let c = &terminal.backend().buffer()[(0, 0)];
        (c.fg, c.bg)
    };
    app.key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT));
    assert_eq!(
        gutter(&mut app),
        (theme.cursor_gutter_fg, theme.cursor_gutter_bg()),
        "as Neovim draws it in Visual mode: CursorLineNr's own colours, not the band (#554)"
    );
    app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(
        gutter(&mut app),
        (theme.cursor_gutter_fg, theme.line_hl),
        "the selection gone, the band is back under the number"
    );
}

#[test]
fn selecting_inside_a_wrapped_line_takes_the_band_off_all_its_rows() {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
    let theme = no_cursor(crate::theme::DEFAULT);
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
    paint(&mut app);
    for _ in 0..12 {
        app.key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    }
    assert_eq!(
        paint(&mut app)[..3],
        ["----------", "----------", "----------"]
    );
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT));
    assert_eq!(
        paint(&mut app),
        ["......####", "..####....", "..........", ".........."],
        "Shift+Up selects half a row up to the cursor, and no row of the line keeps the band"
    );
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
        ["..........", "..........", "..########", ".........."],
        "Shift+Up from the start of the next line: the cursor's rows have no band either"
    );
    app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(
        paint(&mut app)[..3],
        ["----------", "----------", "----------"],
        "the selection gone, the band is back"
    );
}

#[test]
fn find_spans_cut_the_syntax_spans_they_cover() {
    use ratatui::style::{Color, Modifier, Style};

    let syntax = Style::new().fg(Color::Blue).add_modifier(Modifier::ITALIC);
    let find = Style::new().bg(Color::Yellow);
    let lit = syntax.bg(Color::Yellow);
    let out = super::with_find(&[(syntax, 0..4)], &[2..3, 5..6], find);
    assert_eq!(
        out,
        [
            (syntax, 0..2),
            (lit, 2..3),
            (syntax, 3..4),
            (Style::new(), 4..5),
            (find, 5..6),
        ],
        "a match inside a syntax span keeps its colour and style under the tint (#480); one in \
         the gap the highlighter left has none to keep"
    );
    assert_eq!(
        super::with_find(&[(syntax, 0..4)], &[0..2, 2..6], find),
        [(lit, 0..2), (lit, 2..4), (find, 4..6)],
        "a match across a span's end is cut there"
    );
    let named = find.fg(Color::Black);
    assert_eq!(
        super::with_find(&[(syntax, 0..4)], &[1..2, 5..6], named),
        [
            (syntax, 0..1),
            (syntax.patch(named), 1..2),
            (syntax, 2..4),
            (Style::new(), 4..5),
            (named, 5..6)
        ],
        "a theme's own match foreground wins over the syntax colour"
    );
    assert_eq!(
        super::with_find(&[(syntax, 0..4)], &[], find),
        [(syntax, 0..4)]
    );
}

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
    let y = app.cursor_cell.unwrap().y;
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
    let mut terminal = Terminal::new(TestBackend::new(12, 5)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        rows(&terminal)[..3],
        ["1 \u{26a0}\u{fe0f} abcdefgh", "i", "2 next"]
    );
    app.key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    app.key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        app.cursor_cell.unwrap().x,
        2 + 3,
        "on the `b`: the emoji is two columns wide, as ratatui draws it"
    );
}

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
    assert_eq!(
        sent,
        [2, 4, 5, 7, 8, 9],
        "the emoji's second column is never sent: ratatui/ratatui#2651 prints it in place"
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
    assert_eq!(app.cursor_cell.unwrap().x, 2 + 4 + 7);
}

#[test]
fn merl_paints_the_cursor_cell_in_the_theme_s_colours_and_hides_the_terminal_s() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.py"), b"from enum import Enum\n"),
        None,
    );
    app.show_tree = false;
    let theme = crate::theme::load("github-light").unwrap();
    let mut terminal = Terminal::new(TestBackend::new(40, 4)).unwrap();
    let mut cell = |app: &mut App| {
        terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
        assert!(!terminal.backend().cursor_visible());
        let at = app.cursor_cell.unwrap();
        let c = &terminal.backend().buffer()[(at.x, at.y)];
        (c.symbol().to_owned(), c.fg, c.bg)
    };
    let hex = |h: &str| h.parse::<Color>().unwrap();
    assert_eq!(
        cell(&mut app),
        ("f".to_owned(), hex("#ffffff"), hex("#1f2328")),
        "github-light's Cursor group"
    );
    app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let (symbol, _, typing) = cell(&mut app);
    assert_eq!(symbol, "f");
    assert_eq!(Some(typing), theme.typing_cursor.bg);
    assert_ne!(typing, hex("#1f2328"), "typing is told apart by colour");
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
    let cursor = app.cursor_cell.unwrap();
    let buf = terminal.backend().buffer();
    let row = |y: u16| (0..20).map(|x| buf[(x, y)].symbol()).collect::<String>();
    assert_eq!(row(0), "1 - Celery lost     ");
    assert_eq!(
        row(1),
        "    tasks on restart",
        "`tasks` moves down whole, the row starts past the list marker"
    );
    assert_eq!((cursor.x, cursor.y), (6, 1));
}

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
    let buf = terminal.backend().buffer();
    let o = (0..8).find(|x| buf[(*x, 1)].symbol() == "o").unwrap();
    assert_eq!(
        (buf[(o, 1)].fg, buf[(o, 1)].bg),
        (theme.fg, theme.del_bg),
        "with no base file to take syntax colours from, the plain text colour on the deleted tint"
    );
    assert!(
        !buf[(o, 1)].modifier.contains(Modifier::DIM),
        "never greyed by the terminal's `dim` (#144)"
    );
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
        assert_eq!((app.at(), app.cursor_cell.unwrap().y), (at, y));
    }
    key(&mut app, KeyCode::Down);
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let buf = terminal.backend().buffer();
    assert_eq!(
        (buf[(0, 1)].bg, buf[(7, 1)].bg),
        (theme.line_hl, theme.del_bg_hl),
        "the cursor's deleted line is the cursor line, on the deleted tint: gutter and row"
    );
    assert_eq!(buf[(7, 2)].bg, theme.del_bg);
}

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
        assert_eq!(app.cursor_cell.unwrap().y, 0);
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
    assert_eq!(app.cursor_cell.unwrap().y, 3);
    app.key(key(KeyCode::Up));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(app.at(), TextLine::Deleted(2, 8));
    assert_eq!(app.cursor_cell.unwrap().y, 2);
}

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
        let cursor = app.cursor_cell.unwrap().y;
        assert_eq!(((app.at(), app.col), cursor), (at, y), "{c:?}");
    }
}

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
        assert_eq!(app.cursor_cell.unwrap().y, 0);
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
    assert_eq!(app.cursor_cell.unwrap().y, 2);
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
    let mut terminal = Terminal::new(TestBackend::new(14, 6)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        rows(&terminal)[..4],
        ["1 a", "\u{258e}  one two", "\u{258e}  three four", "2 b"]
    );
    let buf = terminal.backend().buffer();
    let t = |y: u16| (0..14).find(|x| buf[(*x, y)].symbol() == "t").unwrap();
    assert_eq!(
        buf[(t(2), 2)].bg,
        theme.del_bg,
        "the second row on the deleted tint"
    );
    assert_eq!(
        buf[(13, 1)].bg,
        theme.del_bg,
        "the deleted tint out to the right edge"
    );
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
    assert_eq!(
        rows(&terminal)[..3],
        ["1 a", "\u{258e}one two thre", "2 b"],
        "cut at the pane's right edge like a text line, with `b` right under it"
    );
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

fn review_of_one_operator(tag: &str) -> (PathBuf, App) {
    review_of(
        tag,
        "def f(a, b):\n    return a == b\n",
        "def f(a, b):\n    return a >= b\n",
    )
}

fn review_of(tag: &str, base: &str, new: &str) -> (PathBuf, App) {
    review_of_file(tag, "f.py", base, new)
}

fn review_of_file(tag: &str, name: &str, base: &str, new: &str) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-{tag}-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
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
    std::fs::write(dir.join(name), base).unwrap();
    std::fs::write(dir.join("gone.py"), "x = 1\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    git(&["rm", "-q", "gone.py"]);
    git(&["commit", "-q", "-m", "gone"]);
    std::fs::write(dir.join(name), new).unwrap();
    let path = dir.join(name);
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
    assert_eq!(app.at(), TextLine::File(1), "on the hunk's added row");
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(app.at(), TextLine::Deleted(1, 0));
    assert_eq!(
        cell(&terminal, 23, 1).1,
        theme.del_bg_hl,
        "Up: the deleted row is the cursor line"
    );
    assert_eq!(cell(&terminal, 23, 2).1, theme.add_bg);
    app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(app.at(), TextLine::File(1));
    let x_of = |t: &Terminal<TestBackend>, y: u16, s: &str| {
        (0..24)
            .find(|x| t.backend().buffer()[(*x, y)].symbol() == s)
            .unwrap()
    };
    let (eq, gt, ret) = (
        x_of(&terminal, 1, "="),
        x_of(&terminal, 2, ">"),
        x_of(&terminal, 2, "r"),
    );
    assert_eq!(
        cell(&terminal, eq, 1),
        (theme.word_fg, theme.del_word_bg),
        "`==` became `>=`: the first `=` is the old word"
    );
    assert_eq!(
        cell(&terminal, gt, 2),
        (theme.word_fg, theme.add_word_bg_hl),
        "`>` is the new word"
    );
    assert_eq!(cell(&terminal, eq + 1, 1).1, theme.del_bg, "the kept `=`");
    assert_eq!(cell(&terminal, 23, 1).1, theme.del_bg);
    assert_eq!(cell(&terminal, 23, 2).1, theme.add_bg_hl);
    let keyword = cell(&terminal, ret, 2).0;
    assert_ne!(keyword, theme.fg);
    assert_eq!(
        cell(&terminal, ret, 1),
        (keyword, theme.del_bg),
        "`return` on the ghost is coloured from the base, a keyword like the text's"
    );
    assert_eq!(cell(&terminal, ret, 2).1, theme.add_bg_hl);
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        cell(&terminal, gt, 2),
        (theme.word_fg, theme.add_word_bg),
        "off the cursor line the added row takes the plain tints"
    );
    assert_eq!(cell(&terminal, 23, 2).1, theme.add_bg);
    app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.col = app.buf.lines[1].find('>').unwrap();
    app.key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        cell(&terminal, gt, 2).1,
        theme.selection,
        "a selection wins over a changed word"
    );
    app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.find_re = Some(regex::Regex::new(&regex::escape(">")).unwrap());
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        cell(&terminal, gt, 2).1,
        theme.find_bg,
        "a find match wins over a changed word"
    );
    app.jump_to(&dir.join("gone.py"), 0);
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        rows(&terminal)[0],
        "1\u{258e}x = 1",
        "a file the branch deleted is all deleted rows"
    );
    assert_eq!(cell(&terminal, 23, 0).1, theme.del_bg_hl);
    drop(std::fs::remove_dir_all(dir));
}

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

#[test]
fn a_capture_s_background_wins_over_the_cursor_line_and_under_a_selection() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.diff"), b"@@ -1 +1 @@\n-a\n+bb\n"),
        None,
    );
    app.show_tree = false;
    let theme = crate::theme::load("tokyonight-moon").unwrap();
    let plus = theme.group("@diff.plus").and_then(|s| s.bg).unwrap();
    app.buf.parse_more(None);
    app.buf.highlight_to(usize::MAX, &theme);
    let paint = |app: &mut App| -> String {
        let mut terminal = Terminal::new(TestBackend::new(8, 4)).unwrap();
        terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
        let buf = terminal.backend().buffer();
        (2..buf.area.width)
            .map(|x| match buf[(x, 2)].bg {
                bg if bg == plus => '+',
                bg if bg == theme.selection => '#',
                bg if bg == theme.line_hl => '-',
                _ => '.',
            })
            .collect()
    };
    app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(
        paint(&mut app),
        "+++---",
        "the cursor line's band yields to @diff.plus behind the text, as in Neovim"
    );
    app.key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT));
    assert_eq!(
        paint(&mut app),
        "#++...",
        "a selection paints over @diff.plus, as Visual does in Neovim, and takes the band off \
         the rest of the line (#554)"
    );
}

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
    app.key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
    app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let buf = terminal.backend().buffer();
    let d = (0..14).find(|x| buf[(*x, 1)].symbol() == "d").unwrap();
    assert_eq!(
        buf[(d, 1)].bg,
        theme.del_word_bg,
        "not wrapped and scrolled to the end, `delta` is painted where it now stands"
    );
    assert!(app.left > 0);
}

#[test]
fn the_enclosing_function_stays_pinned_above_the_code() {
    let mut text = String::from("fn long() {\n");
    for i in 0..40 {
        _ = writeln!(text, "    let a{i} = {i};");
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
    let theme = no_cursor(crate::theme::DEFAULT);
    let mut terminal = Terminal::new(TestBackend::new(30, 12)).unwrap();
    let mut press = |app: &mut App, code: KeyCode| {
        app.key(KeyEvent::new(code, KeyModifiers::NONE));
        terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
        let y = app.cursor_cell.unwrap().y as usize;
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
            assert_eq!(
                band,
                pinned || app.line == 0,
                "unpinned, the top row is lit only as the cursor line"
            );
            assert!(y >= usize::from(pinned), "the cursor is under the band");
            assert!(screen[y].starts_with(&format!("{} ", app.line + 1)));
        }
        if code == KeyCode::Down {
            let (screen, y, _) = press(&mut app, KeyCode::Null);
            assert_eq!((app.line, y), (20, 10));
            assert_eq!(
                screen[1], "12     let a10 = 10;",
                "on the bottom row, the rows above it start right under the band"
            );
        }
    }
    let (screen, _, _) = press(&mut app, KeyCode::Null);
    assert_eq!(screen[..2], ["1 fn long() {", "2     let a0 = 0;"]);
}

#[test]
fn a_hunk_inside_a_long_function_lands_below_the_pinned_header() {
    let dir = std::env::temp_dir().join(format!("merl-pinned-hunk-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
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
            _ = writeln!(t, "    a{i} = {v}");
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
    let y = app.cursor_cell.unwrap().y as usize;
    assert_eq!(app.line, 21);
    assert_eq!(screen[0], "1 def long():");
    let buf = terminal.backend().buffer();
    assert!((0..30).all(|x| buf[(x, 0)].bg == theme.line_hl));
    assert_eq!(
        screen[y - 1..=y],
        ["\u{258e}    a20 = 20", "22\u{258e}    a20 = 200"],
        "on the hunk's added line, its deleted line above it"
    );
    assert!(y > 1, "the deleted line is under the band");
    drop(std::fs::remove_dir_all(dir));
}

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

fn press(
    app: &mut App,
    terminal: &mut Terminal<TestBackend>,
    code: KeyCode,
) -> (Vec<String>, usize) {
    let theme = no_cursor(crate::theme::DEFAULT);
    app.key(KeyEvent::new(code, KeyModifiers::NONE));
    terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
    (rows(terminal), app.cursor_cell.unwrap().y as usize)
}

fn line_no(row: &str) -> usize {
    row.split(' ').next().unwrap().parse().unwrap()
}

#[test]
fn a_page_under_a_pinned_header_skips_no_line() {
    let mut text = String::from("fn long() {\n");
    for i in 0..60 {
        _ = writeln!(text, "    let a{i} = {i};");
    }
    text += "}\n";
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

#[test]
fn a_jump_scrolls_to_the_first_top_that_fits_under_its_pins() {
    let mut text = String::from("impl S {\n    fn m() {\n");
    for i in 0..30 {
        _ = writeln!(text, "        let a{i} = {i};");
    }
    text += "    }\n    fn n() {\n";
    for i in 0..30 {
        _ = writeln!(text, "        let b{i} = {i};");
    }
    text += "    }\n}\n";
    let (mut app, mut terminal) = pinned_app(&text, 40, 18);
    press(&mut app, &mut terminal, KeyCode::Null);
    app.line = 47;
    let (screen, y) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(
        (app.top_line, y),
        (32, 16),
        "the top on the `}}` of `m`, the cursor on the bottom row"
    );
    assert_eq!(screen[..2], ["1 impl S {", "33     }"]);
}

#[test]
fn a_centred_jump_puts_the_cursor_in_the_middle_of_the_rows_under_its_pins() {
    let mut text = String::from("impl S {\n    fn m() {\n");
    for i in 0..60 {
        _ = writeln!(text, "        let a{i} = {i};");
    }
    text += "    }\n}\n";
    let (mut app, mut terminal) = pinned_app(&text, 40, 18);
    press(&mut app, &mut terminal, KeyCode::Null);
    app.goto_line(41);
    let (screen, y) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(screen[..2], ["1 impl S {", "2     fn m() {"]);
    assert_eq!(
        (line_no(&screen[y]), y),
        (41, 9),
        "seven rows of code above the cursor and eight below, under the two pins"
    );
}

#[test]
fn not_wrapped_the_view_scrolls_before_the_cursor_reaches_the_edge() {
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/f.txt"), b"0123456789abcdefghijKLMN\n"),
        None,
    );
    app.show_tree = false;
    let mut terminal = Terminal::new(TestBackend::new(14, 5)).unwrap();
    press(&mut app, &mut terminal, KeyCode::Char('w'));
    for _ in 0..6 {
        press(&mut app, &mut terminal, KeyCode::Right);
    }
    assert_eq!(app.left, 0);
    press(&mut app, &mut terminal, KeyCode::Right);
    assert_eq!(
        (app.cursor_x(), app.left),
        (7, 1),
        "five of the twelve columns stay right of the cursor"
    );
}

#[test]
fn a_short_pane_pins_nothing_and_a_tall_one_the_two_innermost() {
    let mut text = String::from("mod m {\n    impl S {\n        fn f() {\n");
    for i in 0..40 {
        _ = writeln!(text, "            let a{i} = {i};");
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
    assert_eq!(
        screen[..2],
        ["2     impl S {", "3         fn f() {"],
        "the two innermost pinned, not the `mod` around them"
    );
    assert_eq!(line_no(&screen[2]), app.top_line + 1);
}

#[test]
fn an_unwrapped_pinned_header_is_cut_where_the_code_is() {
    let mut text = String::from("fn long(first: usize, second: usize) {\n");
    for i in 0..30 {
        _ = writeln!(text, "    let a{i} = first + second + {i};");
    }
    text += "}\n";
    let (mut app, mut terminal) = pinned_app(&text, 24, 12);
    press(&mut app, &mut terminal, KeyCode::Char('w'));
    for _ in 0..20 {
        press(&mut app, &mut terminal, KeyCode::Down);
    }
    let (screen, _) = press(&mut app, &mut terminal, KeyCode::End);
    let (left, w) = (app.left, app.view_w);
    let header = "fn long(first: usize, second: usize) {";
    assert!(left > 0 && header.len() > left + w);
    assert_eq!(
        screen[0],
        format!(
            "1 \u{2039}{}\u{203a}",
            header.get(left + 1..left + w - 1).unwrap()
        )
    );
}

fn cut_app(text: &str, w: u16, h: u16, line: usize) -> (App, Terminal<TestBackend>) {
    let (mut app, mut terminal) = pinned_app(text, w, h);
    app.line = line;
    press(&mut app, &mut terminal, KeyCode::Null);
    (app, terminal)
}

#[test]
fn a_cut_line_ends_in_a_dim_ellipsis_when_wrapped() {
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let text = format!("{}\nsecond\n", "a".repeat(20_005));
    let (mut app, mut terminal) = cut_app(&text, 9, 4, 1);
    assert_eq!(rows(&terminal)[..3], ["aaaaaaa", "a\u{2026}", "2 second"]);
    let buf = terminal.backend().buffer();
    assert_eq!(
        (buf[(3, 1)].fg, buf[(3, 1)].bg),
        (theme.gutter_fg, theme.bg)
    );
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let buf = terminal.backend().buffer();
    assert_eq!(
        (buf[(3, 1)].symbol(), buf[(3, 1)].bg),
        ("\u{2026}", theme.selection),
        "selected through the line's end, the `…` is on the selection, as the newline is"
    );
    let (mut app, mut terminal) = cut_app(&text, 12, 4, 1);
    assert_eq!(
        rows(&terminal)[..3],
        ["aaaaaaaaaa", "\u{2026}", "2 second"],
        "ten columns: the last row is full, and the `…` stands alone under it"
    );
    let (_, y) = press(&mut app, &mut terminal, KeyCode::Up);
    assert_eq!((app.line, y), (0, 1), "Up lands on the row of the `…`");
    app.col = app.buf.shown(0).len();
    press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(
        app.cursor_cell.unwrap(),
        (2, 1).into(),
        "the end of the drawn part is the `…`"
    );
    let text = format!("    {}\nsecond\n", "a".repeat(19_999));
    let (_, terminal) = cut_app(&text, 10, 4, 1);
    assert_eq!(
        rows(&terminal)[..3],
        ["aaaa", "\u{2026}", "2 second"],
        "a last row full at its indent puts the `…` on a row of its own"
    );
    assert_eq!(
        terminal.backend().buffer()[(6, 1)].symbol(),
        "\u{2026}",
        "the `…` under the text of an indented line"
    );
    let (_, terminal) = cut_app(&"a".repeat(20_000), 9, 3, 0);
    assert!(
        !rows(&terminal).concat().contains('\u{2026}'),
        "a line of exactly what is drawn is not cut"
    );
}

#[test]
fn a_cut_line_ends_in_a_dim_ellipsis_when_not_wrapped() {
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let text = format!("top\n{}\n", "b".repeat(20_005));
    let (mut app, mut terminal) = cut_app(&text, 12, 3, 1);
    press(&mut app, &mut terminal, KeyCode::Char('w'));
    assert_eq!(rows(&terminal)[1], "2 bbbbbbbbb\u{203a}");
    app.col = app.buf.shown(1).len();
    let (screen, _) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(
        screen[1], "2 \u{2039}bbbbbbbb\u{2026}",
        "the end of the drawn part in view: the `…` in the column after it"
    );
    let buf = terminal.backend().buffer();
    assert_eq!(
        (buf[(11, 1)].fg, buf[(11, 1)].bg),
        (theme.gutter_fg, theme.line_hl),
        "the `…` dim, on the cursor line"
    );
    app.left = 19_990;
    app.col = 19_995;
    let (screen, _) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(app.left, 19_990);
    assert_eq!(
        screen[1], "2 \u{2039}bbbbbbbb\u{203a}",
        "the text reaching the right edge exactly: no room for the `…`, so `›` stands there"
    );
}

#[test]
fn a_cut_ghost_ends_in_a_dim_ellipsis() {
    let long = |c: &str| c.repeat(20_005);
    let text = format!("top\n{}\n", long("b"));
    let (mut app, mut terminal) = pinned_app(&text, 9, 4);
    app.diff.ghosts.insert(1, vec![long("a")]);
    app.line = 1;
    press(&mut app, &mut terminal, KeyCode::Null);
    app.top_line = 1;
    app.top_row = app.ghost_rows(1) - 2;
    let (screen, _) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(
        screen[..3],
        ["\u{258e}aaaaaaa", "\u{258e}a\u{2026}", "2 bbbbbbb"],
        "wrapped, the last rows of the ghost are right above its line"
    );
    press(&mut app, &mut terminal, KeyCode::Char('w'));
    app.col = app.buf.shown(1).len();
    app.top_row = 0;
    let (screen, _) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(
        screen[..2],
        ["\u{258e}aaaaaa\u{2026}", "2 \u{2039}bbbbb\u{2026}"],
        "not wrapped and scrolled to the end, the ghost's end is on screen too"
    );
    let (mut app, mut terminal) = pinned_app(&text, 12, 4);
    app.diff.ghosts.insert(1, vec![long("a")]);
    app.line = 1;
    press(&mut app, &mut terminal, KeyCode::Null);
    app.top_line = 1;
    app.top_row = app.ghost_rows(1) - 2;
    let (screen, _) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(
        screen[..3],
        ["\u{258e}aaaaaaaaaa", "\u{258e}\u{2026}", "2 bbbbbbbbbb"],
        "a full last row puts the ghost's `…` on a row of its own"
    );
}

#[test]
fn a_cut_pinned_header_ends_in_a_dim_ellipsis() {
    let mut text = format!("fn long() {{ // {}\n", "x".repeat(20_005));
    for _ in 0..20 {
        text += "    let a = 1;\n";
    }
    _ = write!(text, "    // {}\n}}\n", "b".repeat(20_005));
    let (mut app, mut terminal) = cut_app(&text, 12, 10, 21);
    press(&mut app, &mut terminal, KeyCode::Char('w'));
    app.col = app.buf.shown(21).len();
    let (screen, y) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(screen[0], "1 \u{2039}xxxxxxx\u{2026}");
    assert_eq!(screen[y], "22 \u{2039}bbbbbbb\u{2026}");
}

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
    let mut at = |left: usize, col: usize| {
        (app.left, app.col) = (left, col);
        let (screen, _) = press(&mut app, &mut terminal, KeyCode::Null);
        assert_eq!(app.left, left);
        screen[..2].to_vec()
    };
    assert_eq!(
        at(19_997, 5_002),
        ["1 \u{2039} \u{2026}", "\u{258e}aaa\u{2026}"],
        "the wide char at the end straddles `‹`: the `…` comes after its hidden half"
    );
    assert_eq!(
        at(19_998, 5_003),
        ["1 \u{2039}\u{2026}", "\u{258e}aa\u{2026}"]
    );
    assert_eq!(at(19_999, 5_004), ["1 \u{2039}", "\u{258e}a\u{2026}"]);
    assert_eq!(
        at(20_000, 5_004),
        ["1 \u{2039}", "\u{258e}\u{2026}"],
        "past the end of line 0, `‹` alone; the ghost has no `‹`, so its `…` is in column 0"
    );
}

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
    assert!(
        rows(&terminal)[7].contains("-8:1"),
        "{:?}",
        rows(&terminal)[7]
    );
}

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
    assert_eq!(
        app.at(),
        TextLine::Deleted(0, 0),
        "`[` comes back across the deletion, though no file line was passed"
    );
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn a_wider_pane_draws_the_cursor_on_its_own_line() {
    let words: Vec<String> = (0..60).map(|i| format!("word{i:03}")).collect();
    let mut text = format!("LONG {}\n", words.join(" "));
    for i in 2..40 {
        _ = writeln!(text, "line {i}");
    }
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/a.txt"), text.as_bytes()),
        None,
    );
    let mut terminal = Terminal::new(TestBackend::new(100, 12)).unwrap();
    for _ in 0..17 {
        press(&mut app, &mut terminal, KeyCode::Down);
    }
    assert_eq!((app.line, app.top_line, app.top_row), (10, 0, 7));
    let (screen, y) = press(&mut app, &mut terminal, KeyCode::Char('t'));
    assert_eq!(
        screen[y], "11 line 11",
        "`t`: the cursor drawn on the line the status bar names"
    );
    for code in [KeyCode::Enter, KeyCode::Char('X'), KeyCode::Esc] {
        press(&mut app, &mut terminal, code);
    }
    let (screen, y) = press(&mut app, &mut terminal, KeyCode::Null);
    assert_eq!(screen[y], "11 Xline 11");
    assert_eq!(
        app.buf.lines[10], "Xline 11",
        "a letter typed lands on the line it is drawn on"
    );
}

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
    let h = u16::try_from(all.len()).unwrap() + 3;
    let mut terminal = Terminal::new(TestBackend::new(20, h)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let r = rows(&terminal);
    assert_eq!(r[0], "\u{258e}old<202e>");
    let buf = terminal.backend().buffer();
    for (i, c) in all.iter().enumerate() {
        let y = u16::try_from(i).unwrap() + 1;
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
    assert_eq!(r[y], "16 \u{1f468}\u{200d}\u{1f4bb} !");
    assert_eq!(buf[(5, u16::try_from(y).unwrap())].symbol(), "!");
}

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
    assert_eq!(app.cursor_cell.unwrap().x, 2 + 1 + 6);
    key(&mut app, KeyCode::Left);
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Delete);
    assert_eq!(app.buf.lines[0], "ab");
}

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
    assert_eq!(x, (60 - u16::try_from(note.len()).unwrap()) / 2);
    assert_eq!(buf[(x, 3)].fg, theme.ghost_fg);
}

fn folded(name: &str, text: &str, keys: &str, width: u16) -> Terminal<TestBackend> {
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo").join(name), text.as_bytes()),
        None,
    );
    app.show_tree = false;
    for c in keys.chars() {
        app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(width, 6)).unwrap();
    while app.parsing() {
        app.parse_more();
    }
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    terminal
}

fn items(width: u16) -> Terminal<TestBackend> {
    let body: String = (0..30).map(|i| format!("    \"x{i}\": {i},\n")).collect();
    folded(
        "f.py",
        &format!("ITEMS = {{\n{body}}}\ndef g():\n    pass\n"),
        "f",
        width,
    )
}

#[test]
fn a_fold_draws_its_tail_and_the_lines_under_it_in_colour() {
    let terminal = items(30);
    assert_eq!(
        rows(&terminal)[..2],
        ["1 ITEMS = { 30 lines \u{22ef} }", "33 def g():"]
    );
    let buf = terminal.backend().buffer();
    assert_ne!(
        buf[(3, 1)].fg,
        buf[(7, 1)].fg,
        "line 33 is drawn uncoloured"
    );
}

#[test]
fn a_fold_counts_one_hidden_line_as_one_line() {
    let terminal = folded("f.py", "def g():\n    pass\nx = 1\n", "f", 30);
    assert_eq!(
        rows(&terminal)[..2],
        ["1 def g():  1 line \u{22ef}", "3 x = 1"]
    );
}

#[test]
fn a_fold_whose_count_does_not_fit_the_row_shows_the_dots_alone() {
    assert_eq!(rows(&items(24))[0], "1 ITEMS = { \u{22ef} }");
    assert_eq!(rows(&items(25))[0], "1 ITEMS = { 30 lines \u{22ef} }");
}

#[test]
fn a_fold_that_hides_no_line_shows_the_dots_alone() {
    let terminal = folded("f.rs", "fn f() {\n}\nfn g() {}\n", "f", 30);
    assert_eq!(rows(&terminal)[0], "1 fn f() { \u{22ef} }");
}

#[test]
fn a_fold_without_wrap_says_its_count_and_its_band_reaches_the_edge() {
    let terminal = folded("f.py", "def g():\n    pass\nx = 1\n", "wf", 30);
    assert_eq!(rows(&terminal)[0], "1 def g():  1 line \u{22ef}");
    let buf = terminal.backend().buffer();
    assert_eq!(buf[(29, 0)].bg, buf[(10, 0)].bg, "the cursor line's band");
}

#[test]
fn selected_code_takes_the_themes_selected_text_colour_and_keeps_find_matches() {
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/f.py"), b"x = 1\n"),
        None,
    );
    app.show_tree = false;
    for _ in 0..5 {
        app.key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT));
    }
    let theme = crate::theme::load("srcery").unwrap();
    let selected_fg = theme.selection_fg.expect("srcery sets selectionForeground");
    let mut terminal = Terminal::new(TestBackend::new(12, 3)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let cell = |terminal: &Terminal<TestBackend>, x: u16| {
        let c = &terminal.backend().buffer()[(x, 0)];
        (c.symbol().to_owned(), c.fg, c.bg)
    };
    assert_eq!(
        cell(&terminal, 6),
        ("1".into(), selected_fg, theme.selection)
    );
    app.find_re = Some(regex::Regex::new(&regex::escape("=")).unwrap());
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(cell(&terminal, 4).2, theme.find_bg);
    assert_eq!(
        cell(&terminal, 6),
        ("1".into(), selected_fg, theme.selection)
    );
}

#[test]
fn a_find_match_carries_the_search_groups_underline_as_neovim_draws_it() {
    use ratatui::style::Modifier;
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/f.py"), b"x = 1\nx = 2\n"),
        None,
    );
    app.show_tree = false;
    app.find_re = Some(regex::Regex::new(&regex::escape("=")).unwrap());
    let theme = crate::theme::load("srcery").unwrap();
    let mut terminal = Terminal::new(TestBackend::new(12, 4)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    let row = |y: u16| (0..12).find(|&x| buffer[(x, y)].symbol() == "=").unwrap();
    assert!(buffer[(row(1), 1)].modifier.contains(Modifier::UNDERLINED));
    let strike = crate::theme::load("tokyonight-moon").unwrap().groups["@markup.strikethrough"];
    assert!(strike.add_modifier.contains(Modifier::CROSSED_OUT));
}

#[test]
fn a_rename_without_changes_tints_the_part_of_each_path_that_changed() {
    use ratatui::style::Style;
    use std::path::Path;
    let theme = crate::theme::load_from(None, crate::theme::DEFAULT).unwrap();
    let base = Style::new().bg(theme.bg).fg(theme.fg);
    let shown = |w: u16, old: &str, new: &str| {
        let mut t = Terminal::new(TestBackend::new(w, 12)).unwrap();
        t.draw(|f| {
            let (old, new) = (Path::new(old), Path::new(new));
            super::super::code::draw_renamed(f, &theme, f.area(), base, old, new);
        })
        .unwrap();
        let buf = t.backend().buffer().clone();
        let on = |bg| {
            buf.content()
                .iter()
                .filter(|c| c.bg == bg)
                .map(ratatui::buffer::Cell::symbol)
                .collect::<String>()
        };
        let text: Vec<String> = rows(&t).into_iter().filter(|r| !r.is_empty()).collect();
        (text, on(theme.del_word_bg), on(theme.add_word_bg))
    };
    let tail = ["renamed, no changes", "c  next file"].map(String::from);

    let (text, del, add) = shown(80, "app/util/text.py", "lib/text.py");
    assert_eq!(text[0], "app/util/text.py  \u{2192}  lib/text.py");
    assert_eq!(text[1..], tail);
    assert_eq!((del.as_str(), add.as_str()), ("app/util", "lib"));

    let (_, del, add) = shown(80, "pkg/helpers.py", "pkg/money.py");
    assert_eq!((del.as_str(), add.as_str()), ("helpers", "money"));

    let (_, del, add) = shown(80, "src/old_name.py", "src/new_name.py");
    assert_eq!((del.as_str(), add.as_str()), ("old_name", "new_name"));

    let (_, del, add) = shown(80, "src/user_old.py", "src/user_new.py");
    assert_eq!((del.as_str(), add.as_str()), ("user_old", "user_new"));

    let (text, del, add) = shown(80, "lib/x.py", "lib/x/x.py");
    assert_eq!(text[0], "lib/x.py  \u{2192}  lib/x/x.py");
    assert_eq!((del.as_str(), add.as_str()), ("", "/x"));

    let (text, del, _) = shown(9, "a/foo.py", "b/foo.py");
    assert_eq!((text[0].as_str(), del.as_str()), ("\u{2026}/foo.py", ""));

    let (old, new) = (
        "src/shop/payments/providers/stripe_client.py",
        "src/shop/billing/providers/stripe_client.py",
    );
    let (text, del, add) = shown(80, old, new);
    assert_eq!(text[..2], [old.to_owned(), format!("\u{2192}  {new}")]);
    assert_eq!((del.as_str(), add.as_str()), ("payments", "billing"));

    let (old, new) = (
        "aaaaaaaaaa/bbbbbbbbbb/old_name.py",
        "aaaaaaaaaa/bbbbbbbbbb/new_name.py",
    );
    let (text, del, add) = shown(30, old, new);
    assert_eq!(text[0], "\u{2026}/bbbbbbbbbb/old_name.py");
    assert_eq!(text[1], "\u{2192}  \u{2026}/bbbbbbbbbb/new_name.py");
    assert_eq!((del.as_str(), add.as_str()), ("old_name", "new_name"));
}

#[test]
fn a_rename_band_cuts_each_path_from_its_start_when_the_two_do_not_fit() {
    use ratatui::style::Style;
    use std::path::Path;
    let theme = crate::theme::load_from(None, crate::theme::DEFAULT).unwrap();
    let band = Style::new().bg(theme.line_hl).fg(theme.fg);
    let shown = |w: usize, old: &str, new: &str| {
        let line = super::super::code::rename_band(&theme, band, Path::new(old), Path::new(new), w);
        let on = |bg| {
            line.spans
                .iter()
                .filter(|s| s.style.bg == Some(bg))
                .map(|s| s.content.as_ref())
                .collect::<String>()
        };
        (
            line.to_string(),
            on(theme.del_word_bg),
            on(theme.add_word_bg),
        )
    };

    let (text, del, add) = shown(60, "store.py", "app/store.py");
    assert_eq!(text, " store.py  \u{2192}  app/store.py");
    assert_eq!((del.as_str(), add.as_str()), ("", "app/"));

    let (text, del, add) = shown(40, "lib/a.py", "src/shop/billing/providers/a.py");
    assert_eq!(text, " lib/a.py  \u{2192}  \u{2026}/billing/providers/a.py");
    assert_eq!((del.as_str(), add.as_str()), ("lib", "/billing/providers"));

    let (old, new) = (
        "aaaaaaaaaa/bbbbbbbbbb/old_name.py",
        "aaaaaaaaaa/bbbbbbbbbb/new_name.py",
    );
    let (text, del, add) = shown(50, old, new);
    assert_eq!(
        text,
        " \u{2026}/old_name.py  \u{2192}  \u{2026}/new_name.py"
    );
    assert_eq!((del.as_str(), add.as_str()), ("old_name", "new_name"));
}

#[test]
fn the_match_under_the_cursor_draws_in_cur_search_the_others_in_search() {
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/demo/f.txt"), b"ab ab\n"),
        None,
    );
    app.show_tree = false;
    for c in ['/', 'a', 'b'] {
        app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
    let theme = no_cursor("tokyonight-moon");
    let mut terminal = Terminal::new(TestBackend::new(12, 3)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let bg = |x: u16| terminal.backend().buffer()[(x, 0)].bg;
    let hex = |s: &str| s.parse::<Color>().unwrap();
    assert_eq!((bg(2), bg(3)), (hex("#3e68d7"), hex("#3e68d7")));
    assert_eq!((bg(5), bg(6)), (hex("#ff966c"), hex("#ff966c")));
}

#[test]
fn a_ghost_takes_the_colours_of_its_base_line_down_to_the_last_ghost_on_screen() {
    let mut app = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.py"), b"a = 1\nb = 2\nc = 3\n"),
        None,
    );
    app.show_tree = false;
    let mut base = vec!["x = 0"; 60];
    (base[1], base[50], base[51]) = ("return 1", "return 2", "import x");
    app.base = Some((
        "main".into(),
        Buffer::from_bytes(PathBuf::from("/tmp/f.py"), base.join("\n").as_bytes()),
    ));
    app.diff.ghosts.insert(1, vec!["return 1".into()]);
    app.diff
        .ghosts
        .insert(2, vec!["return 2".into(), "return 3".into()]);
    app.diff.ghost_from.extend([(1, 1), (2, 50)]);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(20, 8)).unwrap();
    while app.parsing() {
        app.parse_more();
    }
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let r = rows(&terminal);
    let buf = terminal.backend().buffer();
    let fg = |y: u16| {
        let x = (0..20).find(|&x| buf[(x, y)].symbol() == "r").unwrap();
        buf[(x, y)].fg
    };
    assert_eq!(r[1], "\u{258e}return 1");
    assert_ne!(fg(1), theme.fg, "a keyword on the first ghost");
    assert_eq!(
        (r[3].as_str(), fg(3)),
        ("\u{258e}return 2", fg(1)),
        "the base is highlighted as far as the last ghost on screen, not the first"
    );
    assert_eq!(
        (r[4].as_str(), fg(4)),
        ("\u{258e}return 3", theme.fg),
        "the base says `import x` there: the colours of another text would paint the wrong words"
    );
}

#[test]
fn a_changed_word_drops_the_bold_and_italic_of_its_syntax() {
    let (dir, mut app) = review_of(
        "word-modifiers",
        "def f():\n    return 1\n",
        "def f():\n    yield 1\n",
    );
    let theme = crate::theme::load("adwaita").unwrap();
    let mut terminal = Terminal::new(TestBackend::new(24, 6)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    drop(std::fs::remove_dir_all(&dir));
    let r = rows(&terminal);
    let buf = terminal.backend().buffer();
    let cell = |y: usize, c: &str| {
        let x = (0..24)
            .find(|&x| buf[(x, u16::try_from(y).unwrap())].symbol() == c)
            .unwrap();
        buf[(x, u16::try_from(y).unwrap())].clone()
    };
    let y_new = r.iter().position(|l| l.ends_with("yield 1")).unwrap();
    let kept = cell(0, "d").modifier;
    assert!(
        kept.intersects(Modifier::BOLD | Modifier::ITALIC),
        "adwaita draws a keyword bold: {kept:?}"
    );
    let changed = cell(y_new, "y");
    assert_eq!(changed.fg, theme.word_fg);
    assert!(
        !changed
            .modifier
            .intersects(Modifier::BOLD | Modifier::ITALIC),
        "{:?}",
        changed.modifier
    );
}

#[test]
fn a_selection_over_a_changed_word_draws_the_selection_not_the_word() {
    let (dir, mut app) = review_of_one_operator("selected-word");
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(24, 5)).unwrap();
    app.key(KeyEvent::new(KeyCode::End, KeyModifiers::SHIFT));
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(rows(&terminal)[2], "2\u{258e}    return a >= b");
    let buf = terminal.backend().buffer();
    let x = (0..24).find(|&x| buf[(x, 2)].symbol() == ">").unwrap();
    let gt = &buf[(x, 2)];
    assert_eq!(gt.bg, theme.selection);
    assert_ne!(gt.fg, theme.word_fg);
    assert_eq!(
        buf[(23, 2)].bg,
        theme.add_bg,
        "past the selection the added line keeps its tint without the band (#554)"
    );
    for (code, m) in [
        (KeyCode::Esc, KeyModifiers::NONE),
        (KeyCode::Up, KeyModifiers::NONE),
        (KeyCode::Left, KeyModifiers::SHIFT),
    ] {
        app.key(KeyEvent::new(code, m));
    }
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    drop(std::fs::remove_dir_all(&dir));
    assert_eq!(app.at(), TextLine::Deleted(1, 0));
    let buf = terminal.backend().buffer();
    assert_eq!(
        (buf[(0, 1)].bg, buf[(23, 1)].bg),
        (theme.cursor_gutter_bg(), theme.del_bg),
        "a selection on a deleted line takes the band off it too"
    );
}

#[test]
fn a_review_s_tint_wins_over_a_capture_s_background_as_diff_add_does_in_neovim() {
    let (dir, mut app) =
        review_of_file("tint-bg", "f.md", "# A\n# Same old\n", "# A\n# Same new\n");
    let theme = crate::theme::load("tokyonight-moon").unwrap();
    let mut terminal = Terminal::new(TestBackend::new(24, 5)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        rows(&terminal)[1..3],
        ["\u{258e}# Same old", "2\u{258e}# Same new"]
    );
    assert_eq!(app.at(), TextLine::File(1));
    let buf = terminal.backend().buffer();
    assert_eq!(buf[(4, 1)].bg, theme.del_bg, "the deleted heading");
    assert_eq!(buf[(4, 2)].bg, theme.add_bg_hl, "the added heading");
    std::fs::remove_dir_all(dir).unwrap();
}
