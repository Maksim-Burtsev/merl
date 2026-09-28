//! Tests for [`crate::ui::overlays`].

use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Cell;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier};

use crate::app::App;
use crate::buffer::Buffer;
use crate::search::MAX_HITS;
use crate::tree::Tree;

use super::rows;

/// The colour of the first cell of `needle`, the first time it is on screen.
fn at(terminal: &Terminal<TestBackend>, needle: &str) -> Color {
    cell(terminal, needle).fg
}

/// The first cell of `needle`, a char a cell, the first time it is on screen.
fn cell(terminal: &Terminal<TestBackend>, needle: &str) -> Cell {
    let buf = terminal.backend().buffer();
    let n = needle.chars().count() as u16;
    for y in 0..buf.area.height {
        for x in 0..=buf.area.width.saturating_sub(n) {
            let got: String = (0..n).map(|i| buf[(x + i, y)].symbol()).collect();
            if got == needle {
                return buf[(x, y)].clone();
            }
        }
    }
    panic!("{needle:?} is not on screen");
}

/// #157: what `.gitignore` leaves out is dim, in the tree and in `o`.
#[test]
fn ignored_rows_are_dim_in_the_tree_and_in_the_file_picker() {
    let dir = std::env::temp_dir().join(format!("merl-ui-ignored-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("node_modules")).unwrap();
    for (f, text) in [
        (".gitignore", "node_modules/\n.env\n"),
        (".env", "PORT=1\n"),
        ("app.py", "x = 1\n"),
        ("node_modules/pkg.js", "x\n"),
    ] {
        std::fs::write(dir.join(f), text).unwrap();
    }
    let (tree, files) = crate::tree::build(&dir, false);
    let mut app = App::new(dir.clone(), tree, files, Buffer::empty(), None);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(at(&terminal, "node_modules"), theme.ghost_fg);
    assert_eq!(at(&terminal, ".env"), theme.ghost_fg);
    assert_eq!(at(&terminal, "app.py"), theme.fg);

    app.show_tree = false;
    app.key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
    app.picker.as_mut().unwrap().settle();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(at(&terminal, ".env"), theme.ghost_fg);
    assert_eq!(at(&terminal, "app.py"), theme.fg);
    assert!(!rows(&terminal).join("\n").contains("pkg.js"));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Text a reader has to read is never `gutter_fg`: that is the line numbers' colour, under
/// 2:1 against the background in most themes (#146). The `?` overlay's actions and the
/// one-line welcome hint take the theme's readable grey instead.
#[test]
fn help_actions_and_the_narrow_hint_are_drawn_in_the_readable_grey() {
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
    app.mode = crate::app::Mode::Help;
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(at(&terminal, "Open a file (fuzzy)"), theme.ghost_fg);

    // Too short for the five key rows: the pane falls back to the single hint line.
    let mut app = mk();
    app.show_tree = false;
    let mut terminal = Terminal::new(TestBackend::new(40, 4)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(at(&terminal, "o: open file"), theme.ghost_fg);
}

#[test]
fn picker_overlay_renders_border_prompt_and_items() {
    let files = ["src/app.rs", "src/wrap.rs"].map(PathBuf::from).to_vec();
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        files,
        Buffer::empty(),
        None,
    );
    app.open_files_picker();
    app.picker.as_mut().unwrap().settle();

    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();

    let buf = terminal.backend().buffer();
    let text: Vec<String> = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect();
    assert_eq!(text, SNAPSHOT);
}

/// `0 hits` is an answer. Until the grep for the query on screen is back the title says so.
#[test]
fn search_title_hides_the_count_until_the_grep_answers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tutor");
    let mut app = App::new(root, Tree::default(), Vec::new(), Buffer::empty(), None);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
    let mut title = |app: &mut App| {
        terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
        let text = rows(&terminal).join("\n");
        let at = text.find("Search (").expect("the title");
        text[at..].split_inclusive(')').next().unwrap().to_string()
    };

    for c in "sqqzz".chars() {
        app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    assert_eq!(title(&mut app), "Search (…)");
    app.settle_search();
    assert_eq!(title(&mut app), "Search (0 hits)");
    // With the project files in, one line matches.
    app.files = crate::tree::build(&app.root, false).1;
    app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    for c in "sfromisoformat".chars() {
        app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    app.settle_search();
    assert_eq!(title(&mut app), "Search (1 hit)");
}

/// A `D` list the cap cut short never reads as the project's symbols: the title says what
/// the rows are, and once a query is typed it counts the answer to that query.
#[test]
fn symbol_title_says_the_list_is_cut_until_the_query_answers() {
    let dir = std::env::temp_dir().join(format!("merl-ui-cap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let many: String = (0..MAX_HITS)
        .map(|i| format!("func a{i}() {{}}\n"))
        .collect();
    std::fs::write(dir.join("a.go"), many).unwrap();
    std::fs::write(dir.join("z.go"), "func zebra() {}\n").unwrap();
    let (tree, files) = crate::tree::build(&dir, false);
    let mut app = App::new(dir.clone(), tree, files, Buffer::empty(), None);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    let mut title = |app: &mut App| {
        terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
        let text = rows(&terminal).join("\n");
        let at = text.find("Symbols (").expect("the title");
        text[at..].split_inclusive(')').next().unwrap().to_string()
    };

    app.key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE));
    // The event loop ticks an open picker before it draws it.
    app.picker.as_mut().unwrap().settle();
    assert_eq!(
        title(&mut app),
        format!("Symbols (first {MAX_HITS}, type to search all)")
    );
    for c in "zebra".chars() {
        app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    assert_eq!(title(&mut app), "Symbols (…)");
    app.settle_search();
    assert_eq!(title(&mut app), "Symbols (1 hit)");
    // The query greps like `s`, but the picker is `D`'s and its prompt says so.
    let prompt = rows(&terminal)
        .into_iter()
        .find(|r| r.contains("zebra"))
        .expect("the query on screen");
    assert!(prompt.contains("│> zebra"), "not `s> `: {prompt}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The `s` picker has a prompt of its own, and its title counts hits: one, many, or as many
/// as the grep stops at, which is a floor.
#[test]
fn search_picker_prompt_and_hit_count() {
    let dir = std::env::temp_dir().join(format!("merl-ui-hits-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("a.txt"),
        format!("one\n{}", "x\n".repeat(MAX_HITS)),
    )
    .unwrap();
    let files = vec![PathBuf::from("a.txt")];
    let mut app = App::new(dir.clone(), Tree::default(), files, Buffer::empty(), None);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
    for (query, title) in [
        ("one", "Search (1 hit)".to_string()),
        ("x", format!("Search ({MAX_HITS}+ hits)")),
    ] {
        app.key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        for c in query.chars() {
            app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        app.settle_search();
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        let text = rows(&terminal).join("\n");
        assert!(text.contains(&title), "{title}:\n{text}");
        assert!(text.contains(&format!("s> {query} ")), "{text}");
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A usages row is drawn with the colours of the file line it quotes: the `//!` comment
/// in the row must not be painted like the `path:line:` prefix in front of it.
#[test]
fn hit_picker_rows_keep_the_syntax_colours_of_their_line() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut app = App::new(root, Tree::default(), Vec::new(), Buffer::empty(), None);
    let hits = vec![crate::search::Hit {
        path: PathBuf::from("src/wrap.rs"),
        line: 1,
        col: 0,
        text: std::fs::read_to_string(app.root.join("src/wrap.rs"))
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .to_string(),
    }];
    assert!(hits[0].text.starts_with("//!"), "{:?}", hits[0].text);
    app.show_picker(crate::app::PickerKind::Usages, App::hit_items(hits));
    app.picker.as_mut().unwrap().settle();

    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();

    let buf = terminal.backend().buffer();
    let row = 4;
    let x0 = (0..buf.area.width)
        .find(|&x| buf[(x, row)].symbol() == "s")
        .expect("row starts with src/wrap.rs");
    let prefix = "src/wrap.rs:1: ";
    let code_x = x0 + prefix.len() as u16;
    assert_eq!(buf[(code_x, row)].symbol(), "/");
    assert_ne!(buf[(code_x, row)].fg, buf[(x0, row)].fg);
}

#[test]
fn hit_picker_rows_keep_their_colours_past_a_tab() {
    let dir = std::env::temp_dir().join(format!("merl-tab-row-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.c"), "#define MAX\t10\n").unwrap();
    let mut app = App::new(
        dir.clone(),
        Tree::default(),
        Vec::new(),
        Buffer::empty(),
        None,
    );
    let hits = vec![crate::search::Hit {
        path: PathBuf::from("a.c"),
        line: 1,
        col: 0,
        text: "#define MAX\t10".into(),
    }];
    app.show_picker(crate::app::PickerKind::Usages, App::hit_items(hits));
    app.picker.as_mut().unwrap().settle();

    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();

    let buf = terminal.backend().buffer();
    let row = 4;
    let hash = (0..buf.area.width)
        .find(|&x| buf[(x, row)].symbol() == "#")
        .expect("the quoted line");
    // The tab is drawn four wide, and `10` past it keeps the colour the code view gives it.
    let ten = hash + "#define MAX".len() as u16 + 4;
    assert_eq!(buf[(ten, row)].symbol(), "1");
    assert_ne!(buf[(ten, row)].fg, buf[(hash - 2, row)].fg);
}

/// The tree pane and the status bar frame the overlay; the overlay itself is the border,
/// the `>` prompt and the two items.
#[rustfmt::skip]
const SNAPSHOT: [&str; 12] = [
    "┌demo────────────────────────┐ o: open file   ?: help",
    "│                            │",
    "│     ┌Files (2/2)───────────────────────────────────┐",
    "│     │>                                             │",
    "│     │src/app.rs                                    │",
    "│     │src/wrap.rs                                   │",
    "│     │                                              │",
    "│     │                                              │",
    "│     │                                              │",
    "│     └──────────────────────────────────────────────┘",
    "└────────────────────────────┘",
    "demo/  [tree]                                        ? help",
];

/// `?` on a terminal shorter than the key list scrolls instead of clipping.
#[test]
fn help_scrolls_to_the_last_binding() {
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::empty(),
        None,
    );
    app.mode = crate::app::Mode::Help;
    app.help_top = usize::MAX;
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(100, 12)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let text = rows(&terminal).join("\n");
    let (last_key, last_action, last_group) = crate::app::KEYS.last().unwrap();
    assert!(
        text.lines()
            .any(|l| l.contains(last_key) && l.contains(last_action)),
        "{text}"
    );
    assert!(text.contains(last_group), "{text}");
    assert!(text.contains("to scroll"), "{text}");
    let (_, _, first_group) = crate::app::KEYS[0];
    assert!(
        !text.contains(first_group) && !text.contains("Open a file (fuzzy)"),
        "first rows scrolled away\n{text}"
    );
}

#[test]
fn review_panel_counts_end_at_the_border() {
    let dir = PathBuf::from("/tmp");
    let mut app = App::new(
        dir.clone(),
        crate::tree::from_files(&["a.rs".into()]),
        Vec::new(),
        Buffer::from_bytes(dir.join("a.rs"), b"x\n"),
        None,
    );
    app.review = Some(crate::git::Review {
        branch: "feature".into(),
        base: "main".into(),
        merge_base: String::new(),
        files: vec![crate::git::ReviewFile {
            path: "a.rs".into(),
            status: 'M',
            old: None,
            added: 6,
            deleted: 2,
            binary: false,
            untracked: false,
        }],
        note: None,
    });
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(40, 4)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let r = rows(&terminal);
    assert!(
        r[0].starts_with("\u{250c}feature \u{2190} main"),
        "{}",
        r[0]
    );
    assert!(
        r[1].starts_with("\u{2502}  M a.rs               +6 \u{2212}2\u{2502}"),
        "{}",
        r[1]
    );
    // #162: a viewed file has a tick in the column before the status.
    app.viewed.insert("a.rs".into(), 0);
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let r = rows(&terminal);
    assert!(
        r[1].starts_with("\u{2502}\u{2713} M a.rs               +6 \u{2212}2\u{2502}"),
        "{}",
        r[1]
    );
}

#[test]
fn review_panel_cuts_a_long_name_and_keeps_the_counts() {
    let dir = PathBuf::from("/tmp");
    let name = "catppuccin-macchiato-with-a-long-name.png";
    let mut app = App::new(
        dir.clone(),
        crate::tree::from_files(&[name.into()]),
        Vec::new(),
        Buffer::from_bytes(dir.join("a.rs"), b"x\n"),
        None,
    );
    app.review = Some(crate::git::Review {
        branch: "feature".into(),
        base: "main".into(),
        merge_base: String::new(),
        files: vec![crate::git::ReviewFile {
            path: name.into(),
            status: 'A',
            old: None,
            added: 0,
            deleted: 0,
            binary: true,
            untracked: false,
        }],
        note: None,
    });
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(40, 4)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let r = rows(&terminal);
    assert!(r[1].contains("\u{2026} bin\u{2502}"), "{}", r[1]);
}

/// A review of `files`, each `(path, status, added, deleted)`; `bin` for a binary file.
fn review_app(files: &[(&str, char, usize, usize)]) -> App {
    let dir = PathBuf::from("/tmp");
    let paths: Vec<PathBuf> = files.iter().map(|f| f.0.into()).collect();
    let mut app = App::new(
        dir.clone(),
        crate::tree::from_files(&paths),
        Vec::new(),
        Buffer::from_bytes(dir.join("a.rs"), b"x\n"),
        None,
    );
    app.review = Some(crate::git::Review {
        branch: "feature".into(),
        base: "main".into(),
        merge_base: String::new(),
        files: files
            .iter()
            .map(|&(path, status, added, deleted)| crate::git::ReviewFile {
                path: path.into(),
                status,
                old: matches!(status, 'R' | 'C').then(|| "was.rs".into()),
                added,
                deleted,
                binary: path.ends_with(".png"),
                untracked: false,
            })
            .collect(),
        note: None,
    });
    app
}

/// #250: the status letter is bold, in the colours of the gutter marks; a rename is dim. The
/// counts are dim, `bin` too, and the name keeps the text colour.
#[test]
fn review_panel_colours_the_status_and_dims_the_counts() {
    let mut app = review_app(&[
        ("new.rs", 'A', 6, 0),
        ("store.rs", 'M', 7, 1),
        ("gone.rs", 'D', 0, 9),
        ("moved.rs", 'R', 0, 0),
        ("logo.png", 'A', 0, 0),
    ]);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(60, 8)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    for (needle, colour) in [
        ("A new.rs", Color::Green),
        ("M store.rs", Color::Blue),
        ("D gone.rs", Color::Red),
        ("R moved.rs", theme.ghost_fg),
    ] {
        let c = cell(&terminal, needle);
        assert_eq!(c.fg, colour, "{needle}");
        assert!(c.modifier.contains(Modifier::BOLD), "{needle}");
    }
    assert_eq!(at(&terminal, "store.rs"), theme.fg);
    assert_eq!(at(&terminal, "+7 \u{2212}1"), theme.ghost_fg);
    assert_eq!(at(&terminal, "bin"), theme.ghost_fg);
    // #172: the tick column is untouched.
    app.viewed.insert("store.rs".into(), 0);
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(at(&terminal, "\u{2713} M store.rs"), theme.accent);
    assert_eq!(cell(&terminal, "M store.rs").fg, Color::Blue);
}

/// #250: the branch totals sit dim on the bottom border, as `feature ← main` on the top one; a
/// binary file counts as a file and adds no lines.
#[test]
fn review_panel_shows_the_branch_totals_on_the_bottom_border() {
    let mut app = review_app(&[
        ("new.rs", 'A', 6, 0),
        ("store.rs", 'M', 7, 1),
        ("logo.png", 'A', 0, 0),
    ]);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(60, 8)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let r = rows(&terminal);
    let bottom = &r[r.len() - 2];
    assert!(
        bottom.starts_with("\u{2514} 3 files \u{b7} +13 \u{2212}1 \u{2500}"),
        "{bottom}"
    );
    let c = cell(&terminal, "3 files");
    assert_eq!(c.fg, theme.ghost_fg);
    assert!(!c.modifier.contains(Modifier::BOLD));
    assert!(cell(&terminal, "feature").modifier.contains(Modifier::BOLD));

    let mut one = review_app(&[("store.rs", 'M', 7, 1)]);
    terminal.draw(|f| super::draw(f, &mut one, &theme)).unwrap();
    assert!(
        rows(&terminal)
            .join("\n")
            .contains(" 1 file \u{b7} +7 \u{2212}1 ")
    );
}

/// #250: `review_panel_colours = false` draws the panel as before: the letter and the counts in
/// the row's colours, no totals on the bottom border.
#[test]
fn review_panel_colours_off_draws_the_plain_panel() {
    let mut app = review_app(&[("new.rs", 'A', 6, 0), ("store.rs", 'M', 7, 1)]);
    app.review_panel_colours = false;
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(60, 8)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    for needle in ["A new.rs", "M store.rs"] {
        let c = cell(&terminal, needle);
        assert_eq!(c.fg, theme.fg, "{needle}");
        assert!(!c.modifier.contains(Modifier::BOLD), "{needle}");
    }
    assert_eq!(at(&terminal, "+7 \u{2212}1"), theme.fg);
    let r = rows(&terminal);
    assert!(!r[r.len() - 2].contains("file"), "{r:#?}");
}

/// #250: on a border too narrow for them the totals go whole, never cut mid-number.
#[test]
fn review_panel_drops_the_totals_that_do_not_fit() {
    let mut app = review_app(&[("store.rs", 'M', 123_456, 654_321)]);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(60, 8)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let r = rows(&terminal);
    assert!(r[r.len() - 2].contains("+123456 \u{2212}654321"), "{r:?}");

    let mut terminal = Terminal::new(TestBackend::new(24, 8)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let r = rows(&terminal);
    let bottom = &r[r.len() - 2];
    assert!(!bottom.contains("file"), "{bottom}");
    assert!(bottom.starts_with("\u{2514}\u{2500}"), "{bottom}");
}

/// #250: totals exactly as wide as the border are shown whole; on a panel one column narrower
/// they are left out whole, never cut at the corner.
#[test]
fn review_panel_keeps_totals_that_just_fit_and_drops_them_one_column_short() {
    // ` 1 file · +1234567 −7654321 `: 28 columns, the border of the 30-column panel.
    let mut app = review_app(&[("store.rs", 'M', 1_234_567, 7_654_321)]);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(60, 8)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let r = rows(&terminal);
    let fits = "\u{2514} 1 file \u{b7} +1234567 \u{2212}7654321 \u{2518}";
    assert!(r[r.len() - 2].starts_with(fits), "{r:#?}");

    // A 30-column terminal leaves the panel 29 columns, its border 27.
    let mut terminal = Terminal::new(TestBackend::new(30, 8)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let r = rows(&terminal);
    let empty = format!("\u{2514}{}\u{2518}", "\u{2500}".repeat(27));
    assert!(r[r.len() - 2].starts_with(&empty), "{r:#?}");
}

/// #250: a name of wide or multi-byte chars puts the counts at the border, and a long one is
/// cut by columns, never inside a char.
#[test]
fn review_panel_measures_a_non_ascii_name_in_columns() {
    let mut app = review_app(&[
        ("файл.rs", 'M', 1, 0),
        ("文件.rs", 'A', 2, 0),
        ("很长很长很长很长很长很长的名字.rs", 'A', 3, 0),
    ]);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(40, 6)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let buf = terminal.backend().buffer();
    // Every row ends where the header's border does: the counts, then `│`.
    let edge = (0..buf.area.width)
        .find(|&x| buf[(x, 0)].symbol() == "\u{2510}")
        .unwrap();
    // By name: `файл.rs`, the long one, `文件.rs`.
    for (y, counts) in [
        (1, "+1 \u{2212}0"),
        (2, "+3 \u{2212}0"),
        (3, "+2 \u{2212}0"),
    ] {
        let row: String = (0..=edge).map(|x| buf[(x, y)].symbol()).collect();
        assert!(row.ends_with(&format!("{counts}\u{2502}")), "{row}");
    }
    assert_eq!(at(&terminal, "+1 \u{2212}0"), theme.ghost_fg);
    assert!(rows(&terminal)[1].contains("M файл.rs"));
    let long = &rows(&terminal)[2];
    assert!(
        long.contains('\u{2026}') && !long.contains("\u{540d}"),
        "{long}"
    );
}

/// #250: a file below the top level has its counts at the border too: the room for its name
/// counts the indent.
#[test]
fn review_panel_puts_a_nested_file_counts_at_the_border() {
    let mut app = review_app(&[("src/deep/store.rs", 'M', 7, 1), ("top.rs", 'A', 1, 0)]);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(60, 8)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let r = rows(&terminal);
    let nested = r
        .iter()
        .find(|l| l.contains("M store.rs"))
        .expect("store.rs row");
    assert!(nested.contains("+7 \u{2212}1\u{2502}"), "{r:#?}");
    assert!(nested.starts_with("\u{2502}      M store.rs"), "{r:#?}");
}

/// A task can start with the help or a picker open (`Esc`, `Help: Down`, `Picker: PgDn`): the
/// overlay stops above the panel, so the task stays readable.
#[test]
fn overlays_leave_the_tutor_panel_in_sight() {
    let files = ["src/app.rs", "src/wrap.rs"].map(PathBuf::from).to_vec();
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        files,
        Buffer::empty(),
        None,
    );
    app.tutor = Some(crate::tutor::Tutor {
        step: 0,
        dir: PathBuf::from("/demo"),
        drill: None,
    });
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    let mut shows = |app: &mut App, overlay: &str| {
        terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
        let text = rows(&terminal).join("\n");
        assert!(text.contains(overlay), "{overlay}:\n{text}");
        assert!(
            text.contains("Tutor 1/"),
            "{overlay} hides the panel:\n{text}"
        );
    };
    app.mode = crate::app::Mode::Help;
    shows(&mut app, "merl \u{2014} keys");
    app.mode = crate::app::Mode::Normal;
    app.open_files_picker();
    app.picker.as_mut().unwrap().settle();
    shows(&mut app, "Files (");
}
