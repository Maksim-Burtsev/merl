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

fn at(terminal: &Terminal<TestBackend>, needle: &str) -> Color {
    cell(terminal, needle).fg
}

fn cell_at(terminal: &Terminal<TestBackend>, needle: &str) -> (u16, u16) {
    let buf = terminal.backend().buffer();
    let n = needle.chars().count() as u16;
    (0..buf.area.height)
        .find_map(|y| {
            (0..=buf.area.width.saturating_sub(n))
                .find(|&x| (0..n).map(|i| buf[(x + i, y)].symbol()).collect::<String>() == needle)
                .map(|x| (x, y))
        })
        .unwrap_or_else(|| panic!("{needle:?} is not on screen"))
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
    assert_eq!(
        at(&terminal, "node_modules"),
        theme.ghost_fg,
        "what `.gitignore` leaves out is dim"
    );
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
    app.files = crate::tree::build(&app.root, false).1;
    app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    for c in "sfromisoformat".chars() {
        app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    app.settle_search();
    assert_eq!(
        title(&mut app),
        "Search (1 hit)",
        "with the project files in, one line matches"
    );
}

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
        format!("Symbols (first {MAX_HITS}, type to search all)"),
        "a list the cap cut short never reads as the project's symbols"
    );
    for c in "zebra".chars() {
        app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    assert_eq!(title(&mut app), "Symbols (…)");
    app.settle_search();
    assert_eq!(
        title(&mut app),
        "Symbols (1 hit)",
        "the answer to the query"
    );
    let prompt = rows(&terminal)
        .into_iter()
        .find(|r| r.contains("zebra"))
        .expect("the query on screen");
    assert!(
        prompt.contains("│> zebra"),
        "the query greps like `s`, but the prompt is `D`'s, not `s> `: {prompt}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

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

fn rust_file_opening_on_a_doc_comment() -> PathBuf {
    let root = std::env::temp_dir().join(format!("merl-doc-row-{}", std::process::id()));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/wrap.rs"), "//! Soft wrap.\n\nfn wrap() {}\n").unwrap();
    root
}

#[test]
fn hit_picker_rows_keep_the_syntax_colours_of_their_line() {
    let root = rust_file_opening_on_a_doc_comment();
    let mut app = App::new(root, Tree::default(), Vec::new(), Buffer::empty(), None);
    let hits = vec![crate::search::Hit {
        path: PathBuf::from("src/wrap.rs"),
        line1: 1,
        byte_col: None,
        text: std::fs::read_to_string(app.root.join("src/wrap.rs"))
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .to_string(),
        deleted: None,
    }];
    assert!(hits[0].text.starts_with("//!"), "{:?}", hits[0].text);
    app.show_picker(crate::app::PickerKind::Usages, App::hit_items(hits));
    app.picker.as_mut().unwrap().settle();

    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();

    let buf = terminal.backend().buffer();
    let path = cell_at(&terminal, "src/wrap.rs");
    let (x, y) = cell_at(&terminal, "1  //!");
    assert_eq!(path.1 + 1, y);
    let code_x = x + 3;
    assert_eq!(buf[(code_x, y)].symbol(), "/");
    assert_ne!(
        buf[(code_x, y)].fg,
        theme.fg,
        "the `//!` comment is not painted like the `path:line:` prefix"
    );
}

#[test]
fn the_selected_hit_row_takes_the_themes_selected_text_colour() {
    let root = rust_file_opening_on_a_doc_comment();
    let mut app = App::new(root, Tree::default(), Vec::new(), Buffer::empty(), None);
    let hits = vec![crate::search::Hit {
        path: PathBuf::from("src/wrap.rs"),
        line1: 1,
        byte_col: None,
        text: std::fs::read_to_string(app.root.join("src/wrap.rs"))
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .to_string(),
        deleted: None,
    }];
    app.show_picker(crate::app::PickerKind::Usages, App::hit_items(hits));
    app.picker.as_mut().unwrap().settle();

    let theme = crate::theme::load("srcery").unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();

    let buf = terminal.backend().buffer();
    let (x, y) = cell_at(&terminal, "1  //!");
    let code = &buf[(x + 3, y)];
    assert_eq!(code.symbol(), "/");
    assert_eq!(
        (code.fg, code.bg),
        (theme.selection_fg.unwrap(), theme.selection)
    );
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
        line1: 1,
        byte_col: None,
        text: "#define MAX\t10".into(),
        deleted: None,
    }];
    app.show_picker(crate::app::PickerKind::Usages, App::hit_items(hits));
    app.picker.as_mut().unwrap().settle();

    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();

    let buf = terminal.backend().buffer();
    let (hash, row) = cell_at(&terminal, "#define");
    // The tab is drawn four wide, and `10` past it keeps the colour the code view gives it.
    let ten = hash + "#define MAX".len() as u16 + 4;
    assert_eq!(buf[(ten, row)].symbol(), "1");
    assert_ne!(buf[(ten, row)].fg, theme.fg);
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
            generated: false,
            same_bytes: false,
        }],
        deleted: Default::default(),
        added: Default::default(),
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
    app.viewed.insert("a.rs".into(), 0);
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let r = rows(&terminal);
    assert!(
        r[1].starts_with("\u{2502}\u{2713} M a.rs               +6 \u{2212}2\u{2502}"),
        "a viewed file has a tick in the column before the status: {}",
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
            generated: false,
            same_bytes: false,
        }],
        deleted: Default::default(),
        added: Default::default(),
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
                generated: false,
                same_bytes: false,
            })
            .collect(),
        deleted: Default::default(),
        added: Default::default(),
        note: None,
    });
    app
}

#[test]
fn review_panel_dims_the_counts_and_leaves_the_status_plain() {
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
    for needle in ["A new.rs", "M store.rs", "D gone.rs", "R moved.rs"] {
        let c = cell(&terminal, needle);
        assert_eq!(
            c.fg, theme.fg,
            "{needle}: the status letter is plain, like the name"
        );
        assert!(!c.modifier.contains(Modifier::BOLD), "{needle}");
    }
    assert_eq!(at(&terminal, "store.rs"), theme.fg);
    assert_eq!(at(&terminal, "+7 \u{2212}1"), theme.ghost_fg);
    assert_eq!(at(&terminal, "bin"), theme.ghost_fg);
    app.viewed.insert("store.rs".into(), 0);
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(
        at(&terminal, "\u{2713} M store.rs"),
        theme.accent,
        "the tick column is untouched"
    );
    assert_eq!(cell(&terminal, "M store.rs").fg, theme.fg);
}

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
        "a binary file counts as a file and adds no lines: {bottom}"
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
    assert_eq!(
        at(&terminal, "+7 \u{2212}1"),
        theme.fg,
        "the counts in the row's colours"
    );
    let r = rows(&terminal);
    assert!(
        !r[r.len() - 2].contains("file"),
        "no totals on the bottom border: {r:#?}"
    );
}

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
    assert!(
        bottom.starts_with("\u{2514}\u{2500}"),
        "the totals go whole, never cut mid-number: {bottom}"
    );
}

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
    assert!(
        r[r.len() - 2].starts_with(&empty),
        "one column short, left out whole, never cut at the corner: {r:#?}"
    );
}

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
        "a long name is cut by columns, never inside a char: {long}"
    );
}

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
    assert!(
        nested.contains("+7 \u{2212}1\u{2502}"),
        "the room for a nested name counts the indent: {r:#?}"
    );
    assert!(nested.starts_with("\u{2502}      M store.rs"), "{r:#?}");
}

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
            "{overlay} hides the panel, and a task that starts with it open is unreadable:\n{text}"
        );
    };
    app.mode = crate::app::Mode::Help;
    shows(&mut app, "merl \u{2014} keys");
    app.mode = crate::app::Mode::Normal;
    app.open_files_picker();
    app.picker.as_mut().unwrap().settle();
    shows(&mut app, "Files (");
}

#[test]
fn the_lesson_panel_grows_to_its_text() {
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::empty(),
        None,
    );
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut tallest = 0;
    for step in 0..crate::tutor::TUTOR.len() {
        app.tutor = Some(crate::tutor::Tutor {
            step,
            dir: PathBuf::from("/demo"),
            drill: None,
        });
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        let r = rows(&terminal);
        let top = r.iter().position(|l| l.starts_with("Tutor ")).unwrap();
        // The panel runs from its title to the status bar, the last row.
        let panel = r[top + 1..r.len() - 1].join(" ").trim_end().to_string();
        let last = crate::tutor::lesson(step)
            .unwrap()
            .tutor
            .rsplit(' ')
            .next()
            .unwrap();
        assert!(
            panel.ends_with(last),
            "lesson {}: its last word is drawn:\n{r:#?}",
            step + 1
        );
        assert!(
            r.len() - 1 - top >= 3,
            "lesson {}: a title and two rows at least:\n{r:#?}",
            step + 1
        );
        tallest = tallest.max(r.len() - 1 - top);
    }
    assert!(tallest > 3, "no lesson wraps past two rows at 80 columns");
}

fn hits_app(name: &str, hits: &[(&str, usize, &str)]) -> App {
    let dir = std::env::temp_dir().join(format!("merl-{name}-{}", std::process::id()));
    let mut app = App::new(dir, Tree::default(), Vec::new(), Buffer::empty(), None);
    let hits = hits
        .iter()
        .map(|&(path, line, text)| crate::search::Hit {
            path: PathBuf::from(path),
            line1: line,
            byte_col: None,
            text: text.into(),
            deleted: None,
        })
        .collect();
    app.show_picker(crate::app::PickerKind::Usages, App::hit_items(hits));
    app.picker.as_mut().unwrap().settle();
    app
}

fn inside(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let rows: Vec<Vec<String>> = rows(terminal)
        .iter()
        .map(|r| r.chars().map(String::from).collect())
        .collect();
    let (top, left) = rows
        .iter()
        .enumerate()
        .find_map(|(y, r)| {
            (0..r.len().saturating_sub(1))
                .find(|&x| r[x] == "\u{250c}" && r[x + 1] == "U")
                .map(|x| (y, x))
        })
        .expect("the picker");
    rows[top + 1..]
        .iter()
        .take_while(|r| r.get(left).is_some_and(|c| c == "\u{2502}"))
        .map(|r| {
            r[left + 1..]
                .iter()
                .take_while(|c| *c != "\u{2502}")
                .map(String::as_str)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

#[test]
fn hits_of_one_file_sit_under_its_path_and_a_long_one_wraps_whole() {
    let long =
        "total = compute(alpha, beta, gamma, delta, epsilon, zeta, eta, theta, iota, kappa, omega)";
    let mut app = hits_app(
        "grouped",
        &[
            ("pkg/a.py", 1, "x = total"),
            ("pkg/a.py", 20, long),
            ("pkg/b.py", 3, "total"),
        ],
    );
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let list = inside(&terminal);
    let at = list
        .iter()
        .position(|r| r == "pkg/a.py")
        .expect("a.py's header");
    assert_eq!(list[at + 1], "   1  x = total");
    assert!(
        list[at + 2].starts_with("  20  total = compute("),
        "{list:#?}"
    );
    let wrapped: Vec<&str> = list[at + 3..]
        .iter()
        .take_while(|r| r.starts_with("      "))
        .map(|r| r.trim())
        .collect();
    assert!(!wrapped.is_empty(), "{list:#?}");
    assert_eq!(
        format!(
            "{} {}",
            list[at + 2].trim_start_matches("  20  "),
            wrapped.join(" ")
        ),
        long
    );
    let next = at + 3 + wrapped.len();
    assert_eq!(list[next..next + 2], ["pkg/b.py", "   3  total"]);
    assert_eq!(list.iter().filter(|r| r.starts_with("pkg/")).count(), 2);
}

#[test]
fn moving_down_a_grouped_list_keeps_the_row_on_screen_under_its_path() {
    let hits: Vec<(&str, usize, &str)> = (1..=6)
        .map(|n| (["a.py", "b.py", "c.py"][(n - 1) / 2], n, "total"))
        .collect();
    let mut app = hits_app("scroll", &hits);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    for _ in 0..5 {
        app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        let list = inside(&terminal);
        let at = 1;
        assert!(list[at].ends_with(".py"), "{list:#?}");
        let picked = app.picker.as_ref().unwrap().current().unwrap().line;
        assert!(
            list.iter().any(|r| r.trim() == format!("{picked}  total")),
            "{list:#?}"
        );
    }
    let list = inside(&terminal);
    assert!(list.iter().any(|r| r == "c.py"), "{list:#?}");
    assert!(!list.iter().any(|r| r == "a.py"), "{list:#?}");
}

#[test]
fn a_path_with_no_room_for_its_rows_is_not_drawn_last() {
    let mut app = hits_app(
        "dangling",
        &[
            ("a.py", 1, "total"),
            ("a.py", 2, "total"),
            ("a.py", 3, "total"),
            ("b.py", 4, "total"),
        ],
    );
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let list = inside(&terminal);
    assert_eq!(
        list[1..],
        ["a.py", "  1  total", "  2  total", "  3  total", ""]
    );
}

fn three_files() -> App {
    let hits: Vec<(&str, usize, &str)> = (1..=6)
        .map(|n| (["a.py", "b.py", "c.py"][(n - 1) / 2], n, "total"))
        .collect();
    hits_app("three-files", &hits)
}

fn picked_is_shown(app: &mut App, terminal: &mut Terminal<TestBackend>) -> Vec<String> {
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
    let list = inside(terminal);
    let picked = app.picker.as_ref().unwrap().current().unwrap().line;
    assert!(
        list.iter().any(|r| r.trim() == format!("{picked}  total")),
        "{list:#?}"
    );
    list
}

#[test]
fn page_down_from_mid_list_lands_on_a_row_on_screen() {
    let mut app = three_files();
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    picked_is_shown(&mut app, &mut terminal);
    for _ in 0..2 {
        app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    picked_is_shown(&mut app, &mut terminal);
    app.key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
    picked_is_shown(&mut app, &mut terminal);
    assert_eq!(app.picker.as_ref().unwrap().current().unwrap().line, 6);
    app.key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
    let list = picked_is_shown(&mut app, &mut terminal);
    assert_eq!(list[1], "b.py");
}

#[test]
fn moving_up_after_scrolling_down_keeps_the_row_on_screen_under_its_path() {
    let mut app = three_files();
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    for _ in 0..5 {
        app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    picked_is_shown(&mut app, &mut terminal);
    for _ in 0..5 {
        app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        let list = picked_is_shown(&mut app, &mut terminal);
        assert!(list[1].ends_with(".py"), "{list:#?}");
    }
    let list = picked_is_shown(&mut app, &mut terminal);
    assert_eq!(list[1..4], ["a.py", "  1  total", "  2  total"]);
}

#[test]
fn a_file_heads_each_run_of_its_rows() {
    let mut app = hits_app(
        "runs",
        &[
            ("a.py", 1, "total"),
            ("b.py", 2, "total"),
            ("a.py", 3, "total"),
        ],
    );
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let list = picked_is_shown(&mut app, &mut terminal);
    let heads: Vec<&str> = list
        .iter()
        .filter(|r| r.ends_with(".py"))
        .map(|r| r.as_str())
        .collect();
    assert_eq!(heads, ["a.py", "b.py", "a.py"]);
}

#[test]
fn a_list_one_line_tall_shows_the_picked_row() {
    let mut app = three_files();
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 6)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    assert_eq!(inside(&terminal), [">", "a.py:1: total"]);
}

#[test]
fn help_wraps_every_action_whole_at_80_columns() {
    let mut app = App::new(
        PathBuf::from("/demo"),
        Tree::default(),
        Vec::new(),
        Buffer::empty(),
        None,
    );
    app.mode = crate::app::Mode::Help;
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 200)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let words = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let text = words(&rows(&terminal).join(" ").replace('│', " "));
    for (_, action, _) in crate::app::KEYS {
        assert!(text.contains(&words(action)), "{action}\n{text}");
    }
}

#[test]
fn a_deep_review_folds_its_chain_and_keeps_both_file_names() {
    let d = "application/src/main/java/run/halo/app/security/authentication/twofactor/totp";
    let filter = format!("{d}/TotpAuthenticationFilter.java");
    let converter = format!("{d}/TotpCodeAuthenticationConverter.java");
    let mut app = review_app(&[(&filter, 'M', 1, 1), (&converter, 'A', 1, 0)]);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(92, 10)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let r: Vec<String> = rows(&terminal)
        .iter()
        .map(|l| l.chars().take(30).collect())
        .collect();
    assert_eq!(
        r[1..6],
        [
            "\u{2502}  \u{25be} application             \u{2502}",
            "\u{2502}    \u{25be} src                   \u{2502}",
            "\u{2502}      \u{25be} \u{2026}/twofactor/totp    \u{2502}",
            "\u{2502}        M TotpAuthent\u{2026} +1 \u{2212}1\u{2502}",
            "\u{2502}        A TotpCodeAut\u{2026} +1 \u{2212}0\u{2502}",
        ],
        "{r:#?}"
    );
}

#[test]
fn a_chain_that_fits_keeps_a_row_per_directory() {
    let mut app = review_app(&[
        ("binding/form_mapping.go", 'M', 1, 0),
        ("internal/bytesconv/bytesconv.go", 'M', 1, 0),
    ]);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(92, 10)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    let text = rows(&terminal).join("\n");
    assert!(text.contains("\u{25be} internal "), "{text}");
    assert!(text.contains("\u{25be} bytesconv "), "{text}");
}

fn deep_app() -> App {
    let mut files: Vec<String> = Vec::new();
    let mut dir = String::new();
    for d in ["a", "b", "c", "d", "e", "f", "g", "h"] {
        dir.push_str(d);
        dir.push('/');
        files.push(format!("{dir}x.rs"));
    }
    files.push(format!("{dir}TotpAuthenticationManager.java"));
    let mut app = review_app(
        &files
            .iter()
            .map(|f| (f.as_str(), 'M', 1, 1))
            .collect::<Vec<_>>(),
    );
    app.review = None;
    app.buf = Buffer::from_bytes(PathBuf::from("/tmp/a.rs"), "x\n".repeat(40).as_bytes());
    app
}

fn tree_width(app: &mut App, w: u16) -> (usize, String) {
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(w, 30)).unwrap();
    terminal.draw(|f| super::draw(f, app, &theme)).unwrap();
    let r = rows(&terminal);
    let width = r[0].chars().position(|c| c == '\u{2510}').unwrap() + 1;
    (width, r.join("\n"))
}

#[test]
fn the_tree_widens_with_the_keys_to_its_longest_row_on_screen() {
    let mut app = deep_app();
    app.tree.reveal(std::path::Path::new(
        "a/b/c/d/e/f/g/h/TotpAuthenticationManager.java",
    ));
    let (w, text) = tree_width(&mut app, 120);
    assert_eq!(w, 30, "{text}");
    assert!(!text.contains("TotpAuthenticationManager.java"), "{text}");
    app.focus = crate::app::Focus::Tree;
    let (w, text) = tree_width(&mut app, 120);
    assert_eq!(w, 2 + 2 * 8 + 2 + 30, "{text}");
    assert!(
        text.contains("TotpAuthenticationManager.java\u{2502}"),
        "{text}"
    );
    assert!(
        text.lines()
            .skip(1)
            .all(|l| l.chars().nth(29) != Some('\u{2502}')),
        "{text}"
    );
    app.focus = crate::app::Focus::Code;
    assert_eq!(tree_width(&mut app, 120).0, 30);
}

#[test]
fn the_tree_keeps_its_30_columns_where_every_name_fits() {
    let mut app = review_app(&[
        ("binding/form_mapping.go", 'M', 1, 0),
        ("internal/bytesconv/bytesconv.go", 'M', 1, 0),
    ]);
    app.focus = crate::app::Focus::Tree;
    let (w, text) = tree_width(&mut app, 92);
    assert_eq!(w, 30, "{text}");
    assert_eq!(app.tree_width, Some(30));
}

#[test]
fn the_wide_tree_never_passes_half_the_screen() {
    let mut deep = review_app(&[("a/b/c/d/e/f/g/h/i/j/abcdefghij.rs", 'M', 1, 0)]);
    deep.focus = crate::app::Focus::Tree;
    assert_eq!(
        tree_width(&mut deep, 120).0,
        2 + 2 + 2 * 10 + 2 + 13 + 1 + 5
    );
    let long = format!("a/{}.rs", "x".repeat(120));
    let mut long = review_app(&[(&long, 'M', 1, 0)]);
    long.focus = crate::app::Focus::Tree;
    assert_eq!(tree_width(&mut long, 120).0, 60);
}

#[test]
fn the_width_is_chosen_once_a_visit() {
    let mut app = deep_app();
    let top = |app: &mut App| {
        app.tree.cursor = app
            .tree
            .nodes
            .iter()
            .position(|n| n.path == std::path::Path::new("a"))
            .unwrap();
        app.tree.collapse();
    };
    top(&mut app);
    app.focus = crate::app::Focus::Tree;
    assert_eq!(tree_width(&mut app, 92).0, 30);
    app.tree.reveal(std::path::Path::new(
        "a/b/c/d/e/f/g/h/TotpAuthenticationManager.java",
    ));
    assert_eq!(tree_width(&mut app, 92).0, 30);
    app.focus = crate::app::Focus::Code;
    tree_width(&mut app, 92);
    app.focus = crate::app::Focus::Tree;
    assert_eq!(tree_width(&mut app, 92).0, 46);
    top(&mut app);
    assert_eq!(tree_width(&mut app, 92).0, 46);
    app.focus = crate::app::Focus::Code;
    assert_eq!(tree_width(&mut app, 92).0, 30);
}

#[test]
fn a_visit_to_the_wide_tree_leaves_the_code_where_it_was() {
    for (name, text) in [
        ("/tmp/a.csv", format!("{}\n", "x".repeat(110)).repeat(40)),
        ("/tmp/a.rs", format!("{}\n", "x ".repeat(80)).repeat(40)),
    ] {
        let mut app = deep_app();
        app.tree.reveal(std::path::Path::new(
            "a/b/c/d/e/f/g/h/TotpAuthenticationManager.java",
        ));
        app.buf = Buffer::from_bytes(PathBuf::from(name), text.as_bytes());
        app.goto_line(30);
        app.col = 70;
        tree_width(&mut app, 120);
        let before = (app.left, app.top_line, app.top_row);
        app.focus = crate::app::Focus::Tree;
        assert_eq!(tree_width(&mut app, 120).0, 50);
        app.focus = crate::app::Focus::Code;
        assert_eq!(tree_width(&mut app, 120).0, 30);
        assert_eq!((app.left, app.top_line, app.top_row), before, "{name}");
    }
}

#[test]
fn a_visit_to_the_code_leaves_the_tree_cursor_where_it_was() {
    let d = "application/src/main/java/run/halo/app/security/authentication/twofactor/totp";
    let filter = format!("{d}/TotpAuthenticationFilter.java");
    let mut app = review_app(&[(&filter, 'M', 1, 1)]);
    app.focus = crate::app::Focus::Tree;
    assert_eq!(tree_width(&mut app, 120).0, 60);
    let main = std::path::Path::new("application/src/main");
    app.tree.cursor = app.tree.nodes.iter().position(|n| n.path == main).unwrap();
    tree_width(&mut app, 120);
    app.focus = crate::app::Focus::Code;
    assert_eq!(tree_width(&mut app, 120).0, 30);
    app.focus = crate::app::Focus::Tree;
    assert_eq!(tree_width(&mut app, 120).0, 60);
    assert_eq!(app.tree.selected().unwrap().path, main);
}

#[test]
fn a_terminal_too_narrow_to_widen_keeps_the_tree_at_30_columns() {
    let mut app = deep_app();
    app.tree.reveal(std::path::Path::new(
        "a/b/c/d/e/f/g/h/TotpAuthenticationManager.java",
    ));
    app.focus = crate::app::Focus::Tree;
    assert_eq!(tree_width(&mut app, 60).0, 30);
    assert_eq!(app.tree_width, Some(30));
}
