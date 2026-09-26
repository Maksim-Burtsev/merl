//! Tests for [`crate::ui::status`].

use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use crate::app::App;
use crate::buffer::Buffer;
use crate::tree::Tree;

use super::rows;

const STDLIB: &str = "/opt/homebrew/Cellar/python@3.13/3.13.15/Frameworks/Python.framework/\
                      Versions/3.13/lib/python3.13/json/__init__.py";

/// The status line in a pane `width` columns wide, of `path` open read-only outside the
/// project, the cursor on line 185 column 5, the status bar saying `message`.
fn status(path: &str, message: &str, width: u16) -> String {
    let text = "\n".repeat(184) + "def dumps(obj):\n";
    let mut app = App::new(
        PathBuf::from("/work/app"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(PathBuf::from(path), text.as_bytes()),
        None,
    );
    app.show_tree = false;
    app.buf.readonly = Some("outside the project");
    (app.line, app.col) = (184, 4);
    app.message = message.into();
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(width, 4)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    rows(&terminal).pop().unwrap()
}

/// #235: a path too long for the pane is cut from the left at a `/`, so the column,
/// `read-only` and the reason `d` gave stay on the line, at 120 columns and at 80.
#[test]
fn a_long_path_gives_way_to_the_column_read_only_and_the_reason() {
    let why = "dumps: via import json";
    assert_eq!(
        status(STDLIB, why, 120),
        "\u{2026}/Python.framework/Versions/3.13/lib/python3.13/json/__init__.py  \
         185:5  [code]  read-only  dumps: via import json"
    );
    assert_eq!(
        status(STDLIB, why, 80),
        "\u{2026}/python3.13/json/__init__.py  185:5  [code]  read-only  dumps: via import json"
    );
    // A deep path in the project gives way the same.
    assert_eq!(
        status(
            "/work/app/services/billing/src/providers/stripe/webhooks/handlers/paid.py",
            why,
            80
        ),
        "\u{2026}/webhooks/handlers/paid.py  185:5  [code]  read-only  dumps: via import json"
    );
    // With nothing to report, the `? help` at the right edge is kept clear as well.
    let bare = status(STDLIB, "", 80);
    assert!(
        bare.starts_with(
            "\u{2026}/3.13/lib/python3.13/json/__init__.py  185:5  [code]  read-only  "
        ) && bare.ends_with("  ? help"),
        "{bare:?}"
    );
    // The file's name is never cut: in a pane too narrow for it, the end of the line goes.
    assert_eq!(
        status(STDLIB, why, 50),
        "\u{2026}/__init__.py  185:5  [code]  read-only  dumps: vi"
    );
}

/// A path that fits is drawn whole.
#[test]
fn a_path_that_fits_is_left_whole() {
    assert_eq!(
        status("/work/lib/json.py", "dumps: via import json", 80),
        "/work/lib/json.py  185:5  [code]  read-only  dumps: via import json"
    );
}

/// The path is measured in columns, not in characters: each of these takes two.
#[test]
fn a_wide_path_is_cut_by_the_columns_it_takes() {
    let line = status(
        "/opt/\u{30c7}\u{30fc}\u{30bf}/\u{30e9}\u{30a4}\u{30d6}\u{30e9}\u{30ea}/json/__init__.py",
        "dumps: via import json",
        80,
    );
    assert_eq!(
        line.replace(' ', ""),
        "\u{2026}/\u{30e9}\u{30a4}\u{30d6}\u{30e9}\u{30ea}/json/__init__.py185:5[code]read-onlydumps:viaimportjson",
        "{line:?}"
    );
}
