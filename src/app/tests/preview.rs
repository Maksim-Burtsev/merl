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

/// A branch that changes `doc.md` from `base` to `branch` and leaves `notes.md` as it was, under
/// review with `doc.md` open. `notes.md` is `NOTES` and sixty more lines.
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
    std::fs::write(
        dir.join("notes.md"),
        format!("{NOTES}{}", "more\n".repeat(60)),
    )
    .unwrap();
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

/// Ctrl+C on a row that shows no line, the rule under a heading, copies nothing. A jump off it,
/// before the preview has found the row of the new line, and `p` back to the source copy the
/// cursor's line again.
#[test]
fn ctrl_c_on_a_row_of_no_line_copies_nothing() {
    let (dir, mut a) = md_app("copy-rule", PLAN);
    key(&mut a, KeyCode::Char('p'));
    key(&mut a, KeyCode::Down);
    assert!(at_row(&a).2.starts_with('\u{2501}'));
    press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(a.clipboard, None);
    crate::tutor::press(&mut a, ":3<Enter>");
    press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(a.clipboard.as_deref(), Some("Intro line.\n"));
    let _ = std::fs::remove_dir_all(&dir);

    let (dir, mut a) = md_app("copy-back", PLAN);
    key(&mut a, KeyCode::Char('p'));
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Char('p'));
    press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(a.clipboard.as_deref(), Some("Intro line.\n"));
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

/// The row a jump or `p` lands on is the line the keys act on, at column 0 of a list item or a
/// heading too: Enter edits it.
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

/// Reading in the preview adds no stop to the jump history: the rows are in source order, a
/// footnote in its place, so a row moves the source line as little as a line does in the
/// source. Ctrl+End and Ctrl+Home add one as in the source, so `[` goes back.
#[test]
fn reading_the_preview_adds_no_history_stop() {
    let mut text = String::from("Claim[^n].\n\n[^n]: The note.\n\n");
    text += &"More text.\n\n".repeat(12);
    let (dir, mut a) = md_app("history", &text);
    key(&mut a, KeyCode::Char('p'));
    let before = a.history.len();
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Down);
    assert_eq!(at_row(&a).2, "[1] The note.");
    for _ in 0..40 {
        key(&mut a, KeyCode::Down);
    }
    for _ in 0..40 {
        key(&mut a, KeyCode::Up);
    }
    assert_eq!(a.history.len(), before);
    let _ = std::fs::remove_dir_all(&dir);

    // A row that stands for a dozen lines no row shows: a step over it is still reading.
    let mut text = String::from("Top [r0].\n\n");
    text += &(0..12)
        .map(|i| format!("[r{i}]: http://x\n"))
        .collect::<String>();
    text += "\nBelow.\n";
    let (dir, mut a) = md_app("history-hidden", &text);
    key(&mut a, KeyCode::Char('p'));
    let before = a.history.len();
    key(&mut a, KeyCode::Down);
    assert_eq!((a.line, a.history.len()), (14, before));
    key(&mut a, KeyCode::Up);
    assert_eq!((a.line, a.history.len()), (0, before));
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

/// Enter on the row of an empty code block edits its fence; on a row that shows no line (a blank
/// row with no blank line under it, a table's border) it edits the first line the block below
/// shows, a fence, a table's header, never the end of the fence above nor a line no row shows;
/// with nothing below, at the end of the file.
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

    let (dir, mut a) = md_app("end-border", "Text.\n\n| a |\n|---|\n| 1 |\n");
    key(&mut a, KeyCode::Char('p'));
    press(&mut a, KeyCode::End, KeyModifiers::CONTROL);
    assert!(at_row(&a).2.starts_with('\u{2514}'), "{:?}", at_row(&a));
    key(&mut a, KeyCode::Enter);
    assert_eq!((a.mode, a.line, a.col), (Mode::Edit, 4, 5));
    let _ = std::fs::remove_dir_all(&dir);

    let (dir, mut a) = md_app("fence-below", "Intro.\n```py\nx = 1\n```\n");
    key(&mut a, KeyCode::Char('p'));
    key(&mut a, KeyCode::Down);
    assert_eq!(at_row(&a).2, "");
    key(&mut a, KeyCode::Enter);
    assert_eq!((a.mode, a.line, a.col), (Mode::Edit, 1, 0));
    let _ = std::fs::remove_dir_all(&dir);

    let (dir, mut a) = md_app("top-border", "[x]: http://a\n\n| a |\n|---|\n| 1 |\n");
    key(&mut a, KeyCode::Char('p'));
    press(&mut a, KeyCode::Home, KeyModifiers::CONTROL);
    assert!(at_row(&a).2.starts_with('\u{250c}'), "{:?}", at_row(&a));
    key(&mut a, KeyCode::Enter);
    assert_eq!((a.mode, a.line, a.col), (Mode::Edit, 2, 0));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A jump to a line shows the row of that line, found anew, not the row the cursor stood on
/// because it shares the line's position: from the second row of a wrapped table row, `:` to
/// its line shows its first row.
#[test]
fn a_jump_shows_the_row_of_its_line() {
    let text = "| a | b |\n|---|---|\n| x | a cell long enough to wrap in two rows |\n";
    let (dir, mut a) = md_app("jump-row", text);
    key(&mut a, KeyCode::Char('p'));
    let first = a.preview.as_ref().unwrap().doc.row_at((2, 0));
    while at_row(&a).0 != first + 1 {
        key(&mut a, KeyCode::Down);
    }
    assert_eq!((a.line, a.col), (2, 0));
    for c in [':', '3'] {
        key(&mut a, KeyCode::Char(c));
    }
    key(&mut a, KeyCode::Enter);
    a.preview_sync();
    assert_eq!(at_row(&a).0, first);
    let _ = std::fs::remove_dir_all(&dir);
}

/// In `--review` the diff lives on the source: `p` on a file of the review does nothing and says
/// so, and a file outside the review renders as anywhere.
#[test]
fn p_on_a_file_of_the_review_says_in_review() {
    let (dir, mut a) = md_review("in-review", "# Doc\n\nOld.\n", "# Doc\n\nNew.\n");
    key(&mut a, KeyCode::Char('p'));
    assert_eq!((a.previewing(), a.message.as_str()), (false, "in review"));
    crate::tutor::press(&mut a, "onotes.md<Enter>");
    assert_eq!(a.rel_path(), "notes.md");
    key(&mut a, KeyCode::Char('p'));
    assert!(a.previewing());
    let _ = std::fs::remove_dir_all(&dir);
}

const NOTES: &str = "# Notes\n\nKept.\n";

/// A file shown rendered that an agent changes joins the review: its preview is off for good, as
/// if `p` had been pressed, the source at the place the preview showed. When the file leaves the
/// review again it stays source, in edit mode or not, until `p`; so does a rendered file that was
/// not open when it joined.
#[test]
fn a_file_that_joins_the_review_leaves_the_preview_for_good() {
    let (dir, mut a) = md_review("joins", "# Doc\n\nOld.\n", "# Doc\n\nNew.\n");
    let kept = std::fs::read_to_string(dir.join("notes.md")).unwrap();
    let refresh = |a: &mut App| a.review_refreshed(git::Review::open(&dir, None, None).unwrap());
    (a.view_w, a.view_h) = (40, 7);
    crate::tutor::press(&mut a, "onotes.md<Enter>");
    key(&mut a, KeyCode::Char('p'));
    for _ in 0..6 {
        key(&mut a, KeyCode::Down);
    }
    assert!(a.previewing() && a.line > 7, "{}", a.line);
    std::fs::write(dir.join("notes.md"), format!("# Changed\n{kept}")).unwrap();
    refresh(&mut a);
    assert!(!a.previewing(), "joined: source");
    assert!(
        (a.top_line..a.top_line + 7).contains(&a.line),
        "the cursor on screen"
    );
    key(&mut a, KeyCode::Enter);
    std::fs::write(dir.join("notes.md"), &kept).unwrap();
    refresh(&mut a);
    assert_eq!(
        (a.mode, a.previewing()),
        (Mode::Edit, false),
        "left, editing: source"
    );
    key(&mut a, KeyCode::Esc);
    key(&mut a, KeyCode::Char('p'));
    assert!(a.previewing(), "`p` renders it again");

    crate::tutor::press(&mut a, "odoc.md<Enter>");
    std::fs::write(dir.join("notes.md"), format!("# Changed\n{kept}")).unwrap();
    refresh(&mut a);
    std::fs::write(dir.join("notes.md"), &kept).unwrap();
    refresh(&mut a);
    crate::tutor::press(&mut a, "onotes.md<Enter>");
    assert!(!a.previewing(), "joined while not open: source");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `p` on a file outside the review list whose diff has lines deleted after its last one, the
/// pane scrolled onto them, returns on the row of the cursor's line: the list lags the file (it
/// waits out a debounce, or auto-reload is off and Ctrl+R took the file).
#[test]
fn p_past_the_last_line_of_a_file_the_list_lags_returns() {
    // A merl that hangs cannot be stopped from here: the App lives on a thread of its own.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let (dir, mut a) = md_review("lags", "# Doc\n\nOld.\n", "# Doc\n\nNew.\n");
        (a.view_w, a.view_h) = (40, 7);
        crate::tutor::press(&mut a, "onotes.md<Enter>");
        std::fs::write(dir.join("notes.md"), NOTES).unwrap();
        a.reload(true);
        for _ in 0..70 {
            key(&mut a, KeyCode::Down);
        }
        let scrolled = a.top_line == a.buf.lines.len();
        key(&mut a, KeyCode::Char('p'));
        let row = a.preview.as_ref().map(|p| p.doc.rows[p.row].text.clone());
        let _ = tx.send((scrolled, a.previewing(), row));
        let _ = std::fs::remove_dir_all(&dir);
    });
    let (scrolled, previewing, row) = rx
        .recv_timeout(Duration::from_secs(20))
        .expect("`p` did not return");
    assert!(scrolled, "the pane was on the deleted lines");
    assert!(previewing);
    assert_eq!(row.as_deref(), Some("Kept."));
}

/// `c` in the preview of a file outside the review leaves the preview and walks on from the
/// source: to the review's hunk, and `[` comes back to the source.
#[test]
fn c_in_a_preview_leaves_it_for_the_source() {
    let (dir, mut a) = md_review("c-leaves", "# Doc\n\nOld.\n", "# Doc\n\nNew.\n");
    crate::tutor::press(&mut a, "onotes.md<Enter>");
    key(&mut a, KeyCode::Char('p'));
    assert!(a.previewing());
    key(&mut a, KeyCode::Char('c'));
    assert_eq!((a.rel_path().as_str(), a.line), ("doc.md", 2));
    key(&mut a, KeyCode::Char('['));
    assert_eq!(a.rel_path(), "notes.md");
    assert!(!a.previewing());
    let _ = std::fs::remove_dir_all(&dir);
}
