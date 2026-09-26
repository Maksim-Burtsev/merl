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

/// `p` in a review scrolled down through the lines the branch deleted at the end of the file,
/// the cursor left above the pane: it returns, the cursor row at the top.
#[test]
fn p_on_a_review_scrolled_past_its_last_line_returns() {
    // A merl that hangs cannot be stopped from here: the App lives on a thread of its own.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let base = format!("aaa\nbbb\nccc\n{}", "gone\n".repeat(60));
        let (dir, mut a) = md_review("scrolled-past", &base, "aaa\nbbb\nccc\n");
        (a.view_w, a.view_h) = (40, 7);
        for _ in 0..70 {
            key(&mut a, KeyCode::Down);
        }
        let scrolled = a.top_line == a.buf.lines.len();
        key(&mut a, KeyCode::Char('p'));
        let p = a.preview.as_ref().unwrap();
        let _ = tx.send((scrolled, a.previewing(), p.row, p.top));
        let _ = std::fs::remove_dir_all(&dir);
    });
    let (scrolled, previewing, row, top) = rx
        .recv_timeout(Duration::from_secs(20))
        .expect("`p` did not return");
    assert!(scrolled, "the pane was on the deleted lines");
    assert!(previewing);
    assert_eq!(row, top, "the cursor row at the top");
}

/// The row a jump or `p` lands on is the line the keys act on, at column 0 of a list item or a
/// heading too: Enter edits it, `C` stands on its mark.
#[test]
fn the_row_of_a_line_is_the_one_that_shows_it() {
    let (dir, mut a) = md_app("row-of-line", PLAN);
    a.line = 7;
    key(&mut a, KeyCode::Char('p'));
    assert_eq!(at_row(&a).2, "2. two");
    for c in [':', '5'] {
        key(&mut a, KeyCode::Char(c));
    }
    key(&mut a, KeyCode::Enter);
    a.preview_sync();
    assert_eq!(at_row(&a).2, "Steps");
    for c in [':', '7'] {
        key(&mut a, KeyCode::Char(c));
    }
    key(&mut a, KeyCode::Enter);
    a.preview_sync();
    assert_eq!(at_row(&a).2, "1. one");
    key(&mut a, KeyCode::Enter);
    assert_eq!((a.mode, a.line), (Mode::Edit, 6));
    let _ = std::fs::remove_dir_all(&dir);

    let (dir, mut a) = md_review("row-of-item", "- a\n- b\n- c\n", "- a\n- B\n- c\n");
    key(&mut a, KeyCode::Char('p'));
    press(&mut a, KeyCode::End, KeyModifiers::CONTROL);
    press(&mut a, KeyCode::Char('C'), KeyModifiers::SHIFT);
    let m = marks(&mut a);
    assert_eq!(at_row(&a).2, "\u{2022} B");
    assert_eq!(m[1], ("\u{258e}".into(), "\u{2022} B".into()));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A change on a line no row shows marks the row it belongs to, never with the deletion mark:
/// a reference definition, a fence's info string, a setext underline.
#[test]
fn a_change_no_row_shows_marks_the_row_it_belongs_to() {
    let bar = "\u{258e}".to_string();
    for (tag, base, branch, row) in [
        (
            "ref",
            "Text [a].\n\n[a]: http://x\n",
            "Text [a].\n\n[a]: http://y\n",
            0,
        ),
        (
            "fence",
            "Go:\n\n```py\nx = 1\n```\n",
            "Go:\n\n```python\nx = 1\n```\n",
            2,
        ),
        (
            "setext",
            "Title\n=====\n\nText\n",
            "Title\n-----\n\nText\n",
            0,
        ),
    ] {
        let (dir, mut a) = md_review(tag, base, branch);
        key(&mut a, KeyCode::Char('p'));
        let m = marks(&mut a);
        assert_eq!(m[row].0, bar, "{tag}: {m:?}");
        assert!(m.iter().all(|(g, _)| g != "\u{2581}"), "{tag}: {m:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Lines deleted at the end of the file are marked under the last row of the last line.
#[test]
fn lines_deleted_at_the_end_are_marked_under_the_last_row() {
    let (dir, mut a) = md_review("end-wrapped", "aaa bbb ccc ddd\nzzz\n", "aaa bbb ccc ddd\n");
    key(&mut a, KeyCode::Char('p'));
    let m: Vec<String> = marks(&mut a).into_iter().map(|(m, _)| m).collect();
    assert_eq!(m[..2], [" ", "\u{2581}"]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// With the tree focused, the tree keeps its keys, and what the preview refuses stays refused.
#[test]
fn from_the_tree_the_preview_still_refuses_what_it_refuses() {
    let (dir, mut a) = md_app("tree-focus", PLAN);
    key(&mut a, KeyCode::Char('p'));
    key(&mut a, KeyCode::Tab);
    assert_eq!(a.focus, Focus::Tree);
    key(&mut a, KeyCode::Char('w'));
    key(&mut a, KeyCode::Char('/'));
    assert!(!a.nowrap() && a.mode == Mode::Normal);
    // `}` reads on in the preview, Down moves the tree.
    key(&mut a, KeyCode::Char('}'));
    assert_eq!(at_row(&a).0, 3);
    key(&mut a, KeyCode::Down);
    assert_eq!(at_row(&a).0, 3);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Reading in the preview adds no stop to the jump history, down into the footnotes either,
/// though their source lines are far from the text above them; Ctrl+End and Ctrl+Home add one
/// as in the source, so `[` goes back.
#[test]
fn reading_the_preview_adds_no_history_stop() {
    let mut text = String::from("Claim[^n].\n\n[^n]: The note.\n\n");
    text += &"More text.\n\n".repeat(12);
    let (dir, mut a) = md_app("history", &text);
    key(&mut a, KeyCode::Char('p'));
    let before = a.history.len();
    for _ in 0..40 {
        key(&mut a, KeyCode::Down);
    }
    assert_eq!(at_row(&a).2, "[1] The note.");
    for _ in 0..40 {
        key(&mut a, KeyCode::Up);
    }
    assert_eq!(a.history.len(), before);
    let _ = std::fs::remove_dir_all(&dir);

    let (dir, mut a) = md_app("history-jump", &"Text.\n\n".repeat(20));
    key(&mut a, KeyCode::Char('p'));
    press(&mut a, KeyCode::End, KeyModifiers::CONTROL);
    assert_eq!(a.line, 38);
    key(&mut a, KeyCode::Char('['));
    a.preview_sync();
    assert_eq!((a.line, at_row(&a).0), (0, 0), "back to the top");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `p`, `p` returns to the very row the preview left, a rule under a heading too.
#[test]
fn p_twice_returns_to_the_row_it_left() {
    let (dir, mut a) = md_app("exact", PLAN);
    key(&mut a, KeyCode::Char('p'));
    key(&mut a, KeyCode::Down);
    assert_eq!(at_row(&a).0, 1, "the rule under `Plan`");
    key(&mut a, KeyCode::Char('p'));
    key(&mut a, KeyCode::Char('p'));
    assert_eq!(at_row(&a).0, 1);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A jump to a line centres its row, as it centres the line in the source.
#[test]
fn a_jump_centres_its_row() {
    let (dir, mut a) = md_app("centre", PLAN);
    key(&mut a, KeyCode::Char('p'));
    for c in [':', '8'] {
        key(&mut a, KeyCode::Char(c));
    }
    key(&mut a, KeyCode::Enter);
    a.preview_sync();
    a.preview_clamp();
    assert_eq!(at_row(&a), (7, 3, "2. two".into()));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The keys of a word or a column say nothing either: no message, no picker, no mode.
#[test]
fn keys_of_a_word_say_nothing_in_the_preview() {
    let (dir, mut a) = md_app("silent", PLAN);
    key(&mut a, KeyCode::Char('p'));
    for (code, m) in [
        (KeyCode::Char('d'), KeyModifiers::NONE),
        (KeyCode::Char('n'), KeyModifiers::NONE),
        (KeyCode::Char('N'), KeyModifiers::SHIFT),
        (KeyCode::F(12), KeyModifiers::NONE),
        (KeyCode::F(12), KeyModifiers::SHIFT),
        (KeyCode::Char('f'), KeyModifiers::CONTROL),
        (KeyCode::Up, KeyModifiers::SHIFT),
        (KeyCode::Right, KeyModifiers::ALT),
    ] {
        press(&mut a, code, m);
        assert_eq!(
            (a.message.as_str(), a.mode, a.picker.is_some()),
            ("", Mode::Normal, false),
            "{code:?} {m:?}"
        );
    }
    assert!(a.selection().is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Back in the source, Up and Down aim at the column the preview's row put the cursor on.
#[test]
fn back_in_the_source_up_keeps_the_column() {
    let (dir, mut a) = md_app("want-x", PLAN);
    key(&mut a, KeyCode::Char('p'));
    for _ in 0..7 {
        key(&mut a, KeyCode::Down);
    }
    key(&mut a, KeyCode::Char('p'));
    key(&mut a, KeyCode::Up);
    assert_eq!((a.line, a.col), (6, 3));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A held arrow in the preview is no missed key: its rows are not the source's.
#[test]
fn a_held_arrow_in_the_preview_misses_nothing() {
    let (dir, mut a) = md_app("missed", &"para\n\n".repeat(40));
    key(&mut a, KeyCode::Char('p'));
    let t0 = Instant::now();
    for i in 0..20 {
        a.key_at(
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            t0 + Duration::from_millis(30 * i),
        );
    }
    a.key_at(
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        t0 + Duration::from_millis(2000),
    );
    assert!(a.missed.is_empty(), "{:?}", a.missed);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Only the keys the preview names do anything in it: no other key, with or without Ctrl,
/// selects, finds, flips the wrapping or changes the text behind it.
#[test]
fn only_the_keys_the_preview_names_act_in_it() {
    let (dir, mut a) = md_app("allow", PLAN);
    let codes = (' '..='~')
        .map(KeyCode::Char)
        .chain((1..=12).map(KeyCode::F))
        .chain([
            KeyCode::Backspace,
            KeyCode::Delete,
            KeyCode::Insert,
            KeyCode::BackTab,
        ]);
    for code in codes {
        for m in [KeyModifiers::NONE, KeyModifiers::CONTROL] {
            // What leaves merl or the preview, and what says so, is not this test's.
            if matches!(code, KeyCode::Char('q' | 'p')) && m.is_empty() {
                continue;
            }
            (a.mode, a.picker) = (Mode::Normal, None);
            a.previewed.insert(a.buf.path.clone().unwrap());
            key(&mut a, KeyCode::Esc);
            press(&mut a, code, m);
            assert!(
                a.selection().is_none() && !a.nowrap() && a.mode != Mode::Find,
                "{code:?} {m:?}: {:?}",
                a.mode
            );
            assert_eq!(a.buf.lines.join("\n") + "\n", PLAN, "{code:?} {m:?}");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A narrower pane keeps the colours of the code on screen: the text is the same.
#[test]
fn another_width_keeps_the_code_highlighted() {
    let (dir, mut a) = md_app("keep-hl", "```rust\nfn main() {}\n```\n");
    key(&mut a, KeyCode::Char('p'));
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let draw = |a: &mut App, w: u16| {
        let mut terminal = Terminal::new(TestBackend::new(w, 6)).unwrap();
        terminal.draw(|f| crate::ui::draw(f, a, &theme)).unwrap();
        a.preview.as_ref().unwrap().code[0].hl.clone()
    };
    a.show_tree = false;
    let before = draw(&mut a, 40);
    assert_eq!(before.len(), 1);
    // Drawn at another width, the spans are the very ones kept, not highlighted again.
    a.preview.as_mut().unwrap().code[0].hl[0].clear();
    assert!(draw(&mut a, 30)[0].is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

/// A new layout keeps the cursor on the row it was on, where rows share a position: a table's
/// header and its top border, a heading and its rule, a bottom border.
#[test]
fn a_new_layout_keeps_the_row() {
    let text = "# Plan\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nEnd.\n";
    let (dir, mut a) = md_app("same-row", text);
    key(&mut a, KeyCode::Char('p'));
    let texts: Vec<String> = a
        .preview
        .as_ref()
        .unwrap()
        .doc
        .rows
        .iter()
        .map(|r| r.text.clone())
        .collect();
    for (i, want) in texts.iter().enumerate().filter(|(_, t)| !t.is_empty()) {
        let first = want.chars().next().unwrap();
        a.view_w = 40;
        a.preview_sync();
        while at_row(&a).0 != i {
            let code = if at_row(&a).0 < i {
                KeyCode::Down
            } else {
                KeyCode::Up
            };
            key(&mut a, code);
        }
        a.view_w = 30;
        a.preview_sync();
        assert!(
            at_row(&a).2.starts_with(first),
            "{want:?} became {:?}",
            at_row(&a).2
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// In a review, a line that replaced one is marked changed, blue, as the gutter marks a
/// changed line; a line only added is green. Both are on the review's tint.
#[test]
fn a_replaced_line_is_marked_changed() {
    let base = "aaa\n\nthe old line\n";
    let (dir, mut a) = md_review("replaced", base, "aaa\n\nthe new line\n\nccc\n");
    key(&mut a, KeyCode::Char('p'));
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(20, 6)).unwrap();
    terminal
        .draw(|f| crate::ui::draw(f, &mut a, &theme))
        .unwrap();
    let buf = terminal.backend().buffer();
    let at = |y: u16| (buf[(1, y)].symbol().to_string(), buf[(1, y)].fg);
    use ratatui::style::Color;
    assert_eq!(
        at(2),
        ("\u{258e}".into(), Color::Blue),
        "the new line replaced the old"
    );
    assert_eq!(at(4), ("\u{258e}".into(), Color::Green), "ccc was added");
    // Both on the review's tint, the cursor's own on the one under the cursor.
    for y in [2, 4] {
        let bg = buf[(2, y)].bg;
        assert!(
            [theme.add_bg, theme.add_bg_hl].contains(&bg),
            "row {y}: {bg:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// `c` from a row that does not start on a hunk's line goes to the next hunk by the source's
/// rule, on the source cursor: here the hunk is on the same row, which stays, and the next `c`
/// goes on. The count follows each press.
#[test]
fn c_from_a_row_that_starts_on_no_hunk_goes_by_the_source() {
    let base = "aaa bbb\nxxx\neee fff\nyyy\n";
    let (dir, mut a) = md_review("c-row", base, PARA);
    key(&mut a, KeyCode::Char('p'));
    press(&mut a, KeyCode::Home, KeyModifiers::CONTROL);
    let mut walk = Vec::new();
    for _ in 0..3 {
        marks(&mut a);
        walk.push((a.line, at_row(&a).2, a.review_status().unwrap()));
        key(&mut a, KeyCode::Char('c'));
    }
    let hunk = |k: usize| format!("hunk {k}/2  file 1/1");
    assert_eq!(
        walk,
        [
            (0, "aaa bbb ccc".to_string(), hunk(0)),
            (1, "aaa bbb ccc".to_string(), hunk(1)),
            (3, "ggg hhh".to_string(), hunk(2)),
        ]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `p` on a code block that cites a file, as Cursor does, shows it coloured as that file.
#[test]
fn a_fence_that_cites_a_file_renders() {
    let (dir, mut a) = md_app("cite", "```12:15:src/main.rs\nfn main() {}\n```\n");
    key(&mut a, KeyCode::Char('p'));
    assert!(a.previewing());
    let p = a.preview.as_ref().unwrap();
    assert_eq!(p.code[0].lines, ["fn main() {}"]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Enter on the row of an empty code block, or on a blank row with no blank line under it,
/// edits where the row is: the block's fence, the start of the block after it.
#[test]
fn enter_on_rows_of_no_text_edits_where_they_are() {
    let (dir, mut a) = md_app("empty-block", "Para.\n\n```\n```\n\nEnd.\n");
    key(&mut a, KeyCode::Char('p'));
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Enter);
    assert_eq!((a.mode, a.line, a.col), (Mode::Edit, 2, 0));
    let _ = std::fs::remove_dir_all(&dir);

    let (dir, mut a) = md_app("tight-gap", "```\ncode\n```\nText after.\n");
    key(&mut a, KeyCode::Char('p'));
    key(&mut a, KeyCode::Char('}'));
    key(&mut a, KeyCode::Enter);
    assert_eq!((a.mode, a.line, a.col), (Mode::Edit, 3, 0));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A hunk on a line no row shows (a reference definition, a second blank line, a setext
/// underline) traps neither `c` nor `C`: they go by the source cursor, which stays on the hunk's
/// line, and the preview shows the row that owns it.
#[test]
fn c_and_big_c_step_past_the_lines_a_row_owns() {
    for (tag, base, branch, owner) in [
        (
            "own-ref",
            "Text [a].\n\n[a]: http://x\n\nMid.\n\nEnd.\n",
            "Text [a].\n\n[a]: http://y\n\nMid.\n\nEnd!\n",
            "Text a.",
        ),
        (
            "own-blank",
            "Aaa.\n\n\nMid.\n\nEnd.\n",
            "Aaa.\n \n\nMid.\n\nEnd!\n",
            "Aaa.",
        ),
        (
            "own-setext",
            "Title\n=====\n\nMid.\n\nEnd.\n",
            "Title\n-----\n\nMid.\n\nEnd!\n",
            "Title",
        ),
    ] {
        let (dir, mut a) = md_review(tag, base, branch);
        key(&mut a, KeyCode::Char('p'));
        // The first row already stands on the first hunk, the line it owns.
        marks(&mut a);
        assert_eq!(at_row(&a).2, owner, "{tag}");
        let mut rows = Vec::new();
        for code in ['c', 'C', 'c'] {
            marks(&mut a);
            let m = if code == 'C' {
                KeyModifiers::SHIFT
            } else {
                KeyModifiers::NONE
            };
            press(&mut a, KeyCode::Char(code), m);
            marks(&mut a);
            rows.push(at_row(&a).2);
        }
        assert_eq!(rows, ["End!", owner, "End!"], "{tag}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The status bar and the missed-keys watch read the source cursor, as `c` does: on a row that
/// shows two hunks the count says which one the cursor is on, and from the last one `o` to the
/// next file of the review is a missed `c`.
#[test]
fn the_hunk_count_and_the_watch_agree_with_c() {
    let dir = std::env::temp_dir().join(format!("merl-md-review-{}-agree", std::process::id()));
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
    std::fs::write(dir.join("doc.md"), "a1\nb\nc1\n").unwrap();
    std::fs::write(dir.join("other.md"), "x\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    std::fs::write(dir.join("doc.md"), "a2\nb\nc2\n").unwrap();
    std::fs::write(dir.join("other.md"), "y\n").unwrap();
    git(&["commit", "-q", "-am", "work"]);
    let review = git::Review::open(&dir, None, None).unwrap();
    let (tree, files) = crate::tree::build(&dir, false);
    let path = dir.join("doc.md");
    let mut a = App::new(dir.clone(), tree, files, Buffer::load(&path).unwrap(), None);
    a.start_review(review);
    a.show_tree = false;
    key(&mut a, KeyCode::Char('p'));
    marks(&mut a);
    // One row shows both hunks of the file; the cursor is on the first, then `c` takes it on.
    assert_eq!(at_row(&a).2, "a2 b c2");
    assert_eq!(a.review_status().unwrap(), "hunk 1/2  file 1/2");
    key(&mut a, KeyCode::Char('c'));
    marks(&mut a);
    assert_eq!(at_row(&a).2, "a2 b c2");
    assert_eq!(a.review_status().unwrap(), "hunk 2/2  file 1/2");
    crate::tutor::press(&mut a, "oother.md<Enter>");
    assert_eq!(a.rel_path(), "other.md");
    assert_eq!(a.missed.get("c"), Some(&1), "{:?}", a.missed);
    let _ = std::fs::remove_dir_all(&dir);
}

/// `c` from a row drawn for no line of its own goes to the next hunk: a table's top border, the
/// rule under a setext heading, the blank row before a list with no blank line above it, the
/// blank row before the footnotes. Each stands where its position is in the source, and `c`
/// goes on from there as in the source.
#[test]
fn c_from_a_row_drawn_for_no_line_goes_on() {
    for (tag, base, branch, onto, to) in [
        (
            "no-line-table",
            "Intro.\n\n| a |\n|---|\n| 1 |\n| 2 |\n",
            "Intro!\n\n| a |\n|---|\n| 1 |\n| 3 |\n",
            "\u{250c}",
            "\u{2502} 3 \u{2502}",
        ),
        (
            "no-line-setext",
            "Title\n=====\n\nMid.\n",
            "Tytle\n=====\n\nMid!\n",
            "\u{2501}",
            "Mid!",
        ),
        (
            "no-line-list",
            "Steps:\n- one\n- two\n",
            "Steps!\n- one\n- two!\n",
            "",
            "\u{2022} two!",
        ),
        (
            "no-line-notes",
            "Top[^n].\n\n[^n]: Old.\n",
            "Top[^n].\n\n[^n]: New.\n",
            "",
            "[1] New.",
        ),
    ] {
        let (dir, mut a) = md_review(tag, base, branch);
        key(&mut a, KeyCode::Char('p'));
        press(&mut a, KeyCode::Home, KeyModifiers::CONTROL);
        marks(&mut a);
        // Down onto the row drawn for no line.
        while !at_row(&a).2.starts_with(onto) || at_row(&a).2.is_empty() != onto.is_empty() {
            key(&mut a, KeyCode::Down);
            marks(&mut a);
        }
        key(&mut a, KeyCode::Char('c'));
        marks(&mut a);
        assert_eq!(at_row(&a).2, to, "{tag}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// With footnotes, and a blank line and reference definitions after them, `c` from the body
/// lands on the footnote's hunk, drawn at the end, before it leaves the file.
#[test]
fn c_stops_on_a_footnotes_hunk() {
    let base = "Body[^1].\n\n[^1]: Old note.\n\n[a]: http://a\n[b]: http://b\n";
    let branch = "Body[^1]!\n\n[^1]: New note.\n\n[a]: http://a\n[b]: http://b\n";
    let (dir, mut a) = md_review("note-hunk", base, branch);
    key(&mut a, KeyCode::Char('p'));
    press(&mut a, KeyCode::Home, KeyModifiers::CONTROL);
    marks(&mut a);
    key(&mut a, KeyCode::Char('c'));
    let m = marks(&mut a);
    assert_eq!((a.line, at_row(&a).2.as_str()), (2, "[1] New"), "{m:?}");
    assert_eq!(a.review_status().unwrap(), "hunk 2/2  file 1/1");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A walk with `c` through hunks of every kind: the cursor goes to each hunk's line in turn,
/// the count says which one it is on, and the row shown is the one that shows or owns the line.
#[test]
fn the_count_follows_c_through_hunks_of_every_kind() {
    let base = "# Plan\n\nText with a word.\nSecond line.\n\n- one\n- two\n\n| a |\n|---|\n| 1 |\n\n\
                ```py\nx = 1\n```\n\nDropped.\n\nSee[^n] [r].\n\n[^n]: Note.\n\n[r]: http://x\n\nGone.\n";
    let branch = "# Plan!\n\nText with a word.\nSecond changed.\n\n- one\n- two\n- three\n\n\
                  | a |\n|---|\n| 2 |\n\n```py\nx = 2\n```\n\nSee[^n] [r].\n\n[^n]: New.\n\n\
                  [r]: http://y\n";
    let (dir, mut a) = md_review("walk", base, branch);
    key(&mut a, KeyCode::Char('p'));
    press(&mut a, KeyCode::Home, KeyModifiers::CONTROL);
    marks(&mut a);
    let hunks = a.diff.hunks.clone();
    // A heading, a line inside a paragraph, a list item, a deleted paragraph, a table row, a
    // code line, a footnote drawn at the end, and a reference with the lines deleted after it.
    assert_eq!(hunks.len(), 8, "{hunks:?}");
    for (k, &h) in hunks.iter().enumerate() {
        if k > 0 || h > 0 {
            key(&mut a, KeyCode::Char('c'));
        }
        marks(&mut a);
        assert_eq!(a.line, h, "hunk {k}");
        let want = format!("hunk {}/{}  file 1/1", k + 1, hunks.len());
        assert_eq!(a.review_status().unwrap(), want);
        let p = a.preview.as_ref().unwrap();
        let r = &p.doc.rows[p.row];
        assert!(
            r.lines.contains(&h) || r.owns.contains(&h) || r.lines.start > h,
            "hunk {k}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A jump to a line shows the row of that line, found anew, not the row the cursor stood on
/// because it shares the line's position: from a table's top border, `:` to its header's line.
#[test]
fn a_jump_shows_the_row_of_its_line() {
    let (dir, mut a) = md_app("jump-row", "Intro.\n\n| a |\n|---|\n| 1 |\n");
    key(&mut a, KeyCode::Char('p'));
    while !at_row(&a).2.starts_with('\u{250c}') {
        key(&mut a, KeyCode::Down);
    }
    assert_eq!((a.line, a.col), (2, 0));
    for c in [':', '3'] {
        key(&mut a, KeyCode::Char(c));
    }
    key(&mut a, KeyCode::Enter);
    a.preview_sync();
    assert_eq!(at_row(&a).2, "\u{2502} a \u{2502}");
    let _ = std::fs::remove_dir_all(&dir);
}
