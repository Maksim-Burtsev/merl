//! `p`: the preview of a Markdown file, its keys, and the file changing under it.

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;

/// Rows at 40 columns, by index: 0 `Plan`, 1 its rule, 2 `Intro line.`, 3 blank, 4 `Steps`, 5
/// its rule, 6 `1. one`, 7 `2. two`, 8 blank, 9–13 the table (12 its one row), 14 blank, 15 the
/// code, 16 blank, 17 `End.`.
const PLAN: &str = "# Plan\n\nIntro line.\n\n## Steps\n\n1. one\n2. two\n\n| a | b |\n|---|---|\n\
                    | 1 | 2 |\n\n```py\nx = 1\n```\n\nEnd.\n";

fn md_app(tag: &str, text: &str) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-md-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("PLAN.md");
    std::fs::write(&path, text).unwrap();
    let mut a = App::new(
        dir.clone(),
        Tree::default(),
        Vec::new(),
        Buffer::load(&path).unwrap(),
        None,
    );
    (a.view_w, a.view_h) = (40, 8);
    (dir, a)
}

fn key(a: &mut App, code: KeyCode) {
    press(a, code, KeyModifiers::NONE);
}

/// The cursor row, the top row, and the cursor row's text.
fn at_row(a: &App) -> (usize, usize, String) {
    let p = a.preview.as_ref().expect("a preview");
    (p.row, p.top, p.doc.rows[p.row].text.clone())
}

#[test]
fn p_shows_the_file_rendered_and_the_source_again_at_the_same_place() {
    let (dir, mut a) = md_app("toggle", PLAN);
    // The table's row, fourth on screen.
    a.line = 11;
    (a.top_line, a.top_row) = (8, 0);
    key(&mut a, KeyCode::Char('p'));
    assert!(a.previewing());
    assert_eq!(at_row(&a), (12, 9, "\u{2502} 1 \u{2502} 2 \u{2502}".into()));
    // Down past the table onto the blank row under it.
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Down);
    assert_eq!(at_row(&a).0, 14);
    assert_eq!((a.line, a.col), (12, 0));
    // The source has the cursor on that blank line, sixth on screen as the row was.
    key(&mut a, KeyCode::Char('p'));
    assert!(!a.previewing());
    assert_eq!((a.line, a.top_line, a.top_row), (12, 7, 0));
    // And back to the very row the preview left, the cursor not having moved.
    key(&mut a, KeyCode::Char('p'));
    assert_eq!(at_row(&a).0, 14);
    assert_eq!(a.message, "");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn p_on_a_file_that_is_not_markdown_says_so() {
    let mut a = app("plain\n");
    key(&mut a, KeyCode::Char('p'));
    assert!(!a.previewing());
    assert_eq!(a.message, "not Markdown");
}

#[test]
fn enter_edits_the_source_where_the_preview_stands() {
    let (dir, mut a) = md_app("enter", PLAN);
    key(&mut a, KeyCode::Char('p'));
    for _ in 0..7 {
        key(&mut a, KeyCode::Down);
    }
    assert_eq!(at_row(&a).2, "2. two");
    key(&mut a, KeyCode::Enter);
    assert!(!a.previewing());
    assert_eq!(a.mode, Mode::Edit);
    // On the item's text, past its marker.
    assert_eq!((a.line, a.col), (7, 3));
    key(&mut a, KeyCode::Char('X'));
    assert_eq!(a.buf.lines[7], "2. Xtwo");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn reading_keys_move_the_cursor_row() {
    let (dir, mut a) = md_app("read", PLAN);
    key(&mut a, KeyCode::Char('p'));
    let mut rows = Vec::new();
    for (code, m) in [
        (KeyCode::Char('}'), KeyModifiers::NONE),
        (KeyCode::Char('}'), KeyModifiers::NONE),
        (KeyCode::Char('{'), KeyModifiers::NONE),
        (KeyCode::End, KeyModifiers::CONTROL),
        (KeyCode::Up, KeyModifiers::NONE),
        (KeyCode::Home, KeyModifiers::CONTROL),
        (KeyCode::PageDown, KeyModifiers::NONE),
        (KeyCode::PageUp, KeyModifiers::NONE),
        (KeyCode::Char('d'), KeyModifiers::CONTROL),
        (KeyCode::Char('u'), KeyModifiers::CONTROL),
    ] {
        press(&mut a, code, m);
        let (row, top, _) = at_row(&a);
        rows.push((row, top));
    }
    assert_eq!(
        rows,
        [
            // `}` and `{` stop on the blank rows between blocks.
            (3, 0),
            (8, 1),
            (3, 1),
            (17, 10),
            (16, 10),
            (0, 0),
            (8, 1),
            (0, 0),
            // Half a screen, the view with the cursor.
            (4, 4),
            (0, 0),
        ]
    );
    // The cursor follows in the source, so the status bar, `[` and `]` see where the reader is.
    key(&mut a, KeyCode::Char('}'));
    assert_eq!((a.line, a.col), (3, 0));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn keys_that_act_on_a_word_or_a_column_do_nothing_in_the_preview() {
    let (dir, mut a) = md_app("inert", PLAN);
    key(&mut a, KeyCode::Char('p'));
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Down);
    let before = (a.line, a.col, at_row(&a));
    for c in ['/', 'v', 'd', 'u', 'n', 'w'] {
        key(&mut a, KeyCode::Char(c));
    }
    for code in [KeyCode::Left, KeyCode::Right, KeyCode::End, KeyCode::Home] {
        key(&mut a, code);
    }
    press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    assert_eq!((a.line, a.col, at_row(&a)), before);
    assert_eq!(a.mode, Mode::Normal);
    assert!(a.picker.is_none() && a.selection().is_none() && !a.nowrap());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_rewritten_on_disk_renders_again() {
    let (dir, mut a) = md_app("rewrite", PLAN);
    key(&mut a, KeyCode::Char('p'));
    std::fs::write(dir.join("PLAN.md"), "# Plan\n\nAn agent wrote this.\n").unwrap();
    assert!(a.reload(false));
    a.preview_sync();
    let p = a.preview.as_ref().unwrap();
    let texts: Vec<&str> = p.doc.rows.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(texts[2..], ["An agent wrote this."]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_jump_moves_the_cursor_row_to_its_line() {
    let (dir, mut a) = md_app("jump", PLAN);
    key(&mut a, KeyCode::Char('p'));
    for c in [':', '1', '8'] {
        key(&mut a, KeyCode::Char(c));
    }
    key(&mut a, KeyCode::Enter);
    // What a frame does before it draws.
    a.preview_sync();
    a.preview_clamp();
    assert_eq!(at_row(&a), (17, 10, "End.".into()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_preview_belongs_to_the_file_p_was_pressed_on() {
    let (dir, mut a) = md_app("per-file", PLAN);
    std::fs::write(dir.join("other.md"), "# Other\n").unwrap();
    key(&mut a, KeyCode::Char('p'));
    a.jump_to(&dir.join("other.md"), 1);
    assert!(!a.previewing(), "another Markdown file opens as source");
    key(&mut a, KeyCode::Char('['));
    assert!(a.previewing(), "the plan is still rendered");
    a.preview_sync();
    assert_eq!(at_row(&a).2, "Plan");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A review loses no mark in the preview: the rows of added lines have the green bar and tint.
#[test]
fn the_preview_of_a_review_keeps_its_marks() {
    let (dir, mut a) = review_app_with("preview-review", &[("doc.md", b"# Doc\n\nnew words\n")]);
    a.jump_to(&dir.join("doc.md"), 1);
    key(&mut a, KeyCode::Char('p'));
    assert!(a.previewing());
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    a.show_tree = false;
    let mut terminal = Terminal::new(TestBackend::new(30, 6)).unwrap();
    terminal
        .draw(|f| crate::ui::draw(f, &mut a, &theme))
        .unwrap();
    let buf = terminal.backend().buffer();
    // Row 2 is `new words`, off the cursor row: the bar in the gutter, the tint behind the text.
    assert_eq!(buf[(1, 2)].symbol(), "\u{258e}");
    assert_eq!(buf[(1, 2)].fg, ratatui::style::Color::Green);
    assert_eq!(buf[(2, 2)].symbol(), "n");
    assert_eq!(buf[(2, 2)].bg, theme.add_bg);
    // `C` walks back to the hunk, in the preview as in the source.
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Down);
    assert_eq!(at_row(&a).2, "new words");
    press(&mut a, KeyCode::Char('C'), KeyModifiers::SHIFT);
    a.preview_sync();
    assert!(a.previewing());
    assert_eq!(at_row(&a).2, "Doc");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A branch that changes `doc.md` from `base` to `branch`, under review with `doc.md` open.
fn md_review(tag: &str, base: &str, branch: &str) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-md-review-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "t@t"]);
    git(&["config", "user.name", "t"]);
    std::fs::write(dir.join("doc.md"), base).unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    std::fs::write(dir.join("doc.md"), branch).unwrap();
    git(&["commit", "-q", "-am", "work"]);
    let review = git::Review::open(&dir, None, None).unwrap();
    let (tree, files) = crate::tree::build(&dir, false);
    let path = dir.join("doc.md");
    let mut a = App::new(dir.clone(), tree, files, Buffer::load(&path).unwrap(), None);
    a.start_review(review);
    a.show_tree = false;
    (dir, a)
}

/// The gutter's mark column of each preview row on a pane of 13 columns: a two-column gutter
/// and eleven for the text, which reflows `aaa bbb` / `ccc ddd` / … three words a row.
fn marks(a: &mut App) -> Vec<(String, String)> {
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(13, 5)).unwrap();
    terminal.draw(|f| crate::ui::draw(f, a, &theme)).unwrap();
    let buf = terminal.backend().buffer();
    (0..3)
        .map(|y| {
            let text: String = (2..13).map(|x| buf[(x, y)].symbol()).collect();
            (
                buf[(1, y)].symbol().to_string(),
                text.trim_end().to_string(),
            )
        })
        .collect()
}

const PARA: &str = "aaa bbb\nccc ddd\neee fff\nggg hhh\n";

/// The branch changed the second line of a paragraph: every row that shows part of it has the
/// bar, the first row too, which starts on the line above; `C` lands on a marked row.
#[test]
fn a_row_is_marked_for_every_line_it_shows() {
    let base = "aaa bbb\nxxx yyy\neee fff\nggg hhh\n";
    let (dir, mut a) = md_review("partial", base, PARA);
    key(&mut a, KeyCode::Char('p'));
    let bar = "\u{258e}".to_string();
    assert_eq!(
        marks(&mut a),
        [
            (bar.clone(), "aaa bbb ccc".into()),
            (bar.clone(), "ddd eee fff".into()),
            (" ".into(), "ggg hhh".into()),
        ]
    );
    key(&mut a, KeyCode::Char('}'));
    press(&mut a, KeyCode::Char('C'), KeyModifiers::SHIFT);
    marks(&mut a);
    assert_eq!(at_row(&a).0, 0, "on the hunk's first row");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Lines deleted inside a paragraph are marked on the row that shows where they stood; lines
/// deleted at the end of the file under the last row.
#[test]
fn deleted_lines_are_marked_where_they_stood() {
    let base = "aaa bbb\nxxx\nccc ddd\neee fff\nggg hhh\n";
    let (dir, mut a) = md_review("deleted", base, PARA);
    key(&mut a, KeyCode::Char('p'));
    let m: Vec<String> = marks(&mut a).into_iter().map(|(m, _)| m).collect();
    assert_eq!(m, ["\u{2594}", " ", " "]);
    let _ = std::fs::remove_dir_all(&dir);

    let base = format!("{PARA}zzz\n");
    let (dir, mut a) = md_review("deleted-end", &base, PARA);
    key(&mut a, KeyCode::Char('p'));
    let m: Vec<String> = marks(&mut a).into_iter().map(|(m, _)| m).collect();
    assert_eq!(m, [" ", " ", "\u{2581}"]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Every way into edit mode leaves the preview: Ctrl+N on the file shown rendered edits its
/// source, on screen.
#[test]
fn ctrl_n_on_a_previewed_file_edits_its_source() {
    let (dir, mut a) = md_app("ctrl-n", PLAN);
    key(&mut a, KeyCode::Char('p'));
    press(&mut a, KeyCode::Char('n'), KeyModifiers::CONTROL);
    for c in "PLAN.md".chars() {
        key(&mut a, KeyCode::Char(c));
    }
    key(&mut a, KeyCode::Enter);
    assert_eq!(a.mode, Mode::Edit);
    assert!(!a.previewing(), "the text being typed is on screen");
    key(&mut a, KeyCode::Char('p'));
    assert_eq!(a.buf.lines[0], "p# Plan", "`p` types while editing");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A selection made before `p` goes: the preview cannot show it, and Ctrl+C copies the line of
/// the cursor row, as with nothing selected.
#[test]
fn p_drops_the_selection_so_ctrl_c_copies_the_rows_line() {
    let (dir, mut a) = md_app("copy", PLAN);
    a.line = 6;
    key(&mut a, KeyCode::Char('v'));
    key(&mut a, KeyCode::Char('v'));
    assert!(a.selection().is_some());
    key(&mut a, KeyCode::Char('p'));
    assert!(a.selection().is_none());
    key(&mut a, KeyCode::Down);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(a.clipboard.as_deref(), Some("2. two\n"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A narrower pane lays the file out again: every row fits, and the cursor row stays on its
/// place, as far down the pane.
#[test]
fn a_narrower_pane_lays_the_preview_out_again() {
    let text = "# Plan\n\nA paragraph long enough to take two rows at forty columns wide.\n\n\
                Then the one the cursor reads, also long enough to wrap.\n\nEnd.\n";
    let (dir, mut a) = md_app("narrow", text);
    key(&mut a, KeyCode::Char('p'));
    for _ in 0..5 {
        key(&mut a, KeyCode::Down);
    }
    let (row, top, text) = at_row(&a);
    assert_eq!(
        (row - top, text.as_str()),
        (5, "Then the one the cursor reads, also long")
    );
    a.view_w = 20;
    a.preview_sync();
    a.preview_clamp();
    let p = a.preview.as_ref().unwrap();
    assert!(p.doc.rows.iter().all(|r| wrap::width(&r.text) <= 20));
    assert_eq!(p.row - p.top, 5);
    assert_eq!(p.doc.rows[p.row].text, "Then the one the");
    let _ = std::fs::remove_dir_all(&dir);
}
