use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use crate::app::App;
use crate::buffer::Buffer;
use crate::tree::Tree;

use super::rows;

const STDLIB: &str = "/opt/homebrew/Cellar/python@3.13/3.13.15/Frameworks/Python.framework/\
                      Versions/3.13/lib/python3.13/json/__init__.py";

fn status(path: &str, message: &str, width: u16) -> String {
    status_with(path, width, |app| app.message = message.into())
}

fn status_about(path: &str, file: &str, rest: &str, width: u16) -> String {
    status_with(path, width, |app| {
        app.say_about(&PathBuf::from("/work/app").join(file), rest);
    })
}

fn status_with(path: &str, width: u16, say: impl FnOnce(&mut App)) -> String {
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
    say(&mut app);
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(width, 4)).unwrap();
    terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
    rows(&terminal).pop().unwrap()
}

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
    assert_eq!(
        status(
            "/work/app/services/billing/src/providers/stripe/webhooks/handlers/paid.py",
            why,
            80
        ),
        "\u{2026}/webhooks/handlers/paid.py  185:5  [code]  read-only  dumps: via import json",
        "a deep path in the project gives way the same"
    );
    assert_eq!(
        status(STDLIB, "", 80),
        "\u{2026}/Versions/3.13/lib/python3.13/json/__init__.py  185:5  [code]  read-only",
        "with nothing to report, the path takes the whole bar: no hint at its right edge"
    );
    assert_eq!(
        status(STDLIB, why, 50),
        "\u{2026}/__init__.py  185:5  [code]  read-only  dumps: vi",
        "the file's name is never cut: in a pane too narrow for it, the end of the line goes"
    );
}

#[test]
fn a_path_that_fits_is_left_whole() {
    assert_eq!(
        status("/work/lib/json.py", "dumps: via import json", 80),
        "/work/lib/json.py  185:5  [code]  read-only  dumps: via import json"
    );
}

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

#[test]
fn the_rest_of_the_line_is_measured_in_columns_too() {
    assert_eq!(
        status(STDLIB, "\u{30c7}\u{30fc}\u{30bf}: via import json", 86),
        "\u{2026}/lib/python3.13/json/__init__.py  185:5  [code]  read-only  \
         \u{30c7} \u{30fc} \u{30bf} : via import json",
        "a reason naming `データ` (six columns, three characters, nine bytes) leaves the path the \
         room of its columns; each wide character's second cell reads as a space"
    );
}

#[test]
fn a_message_naming_a_long_path_keeps_its_reason() {
    let file = "services/billing/src/providers/stripe/webhooks/locked.txt";
    assert_eq!(
        status_about("/work/app/paid.py", file, ": permission denied", 80),
        "paid.py  185:5  [code]  read-only  \u{2026}/webhooks/locked.txt: permission denied",
        "once the open file's path is down to its name, the message gives way from the left"
    );
    assert_eq!(
        status_about(
            "/work/app/paid.py",
            "src/locked.txt",
            ": permission denied",
            80
        ),
        "paid.py  185:5  [code]  read-only  src/locked.txt: permission denied",
        "a message that fits is left whole"
    );
    assert_eq!(
        status_about(
            "/work/app/paid.py",
            "My Notes/billing/providers/stripe/webhooks/a.md",
            ": permission denied",
            80
        ),
        "paid.py  185:5  [code]  read-only  \u{2026}/stripe/webhooks/a.md: permission denied",
        "the path is the part the message's maker says it is, a space in it included"
    );
    assert_eq!(
        status(
            "/work/app/paid.py",
            "../fragments/shared/user/profile/fields.graphql: path, 1 match",
            80
        ),
        "paid.py  185:5  [code]  read-only  ../fragments/shared/user/profile/fields.graph",
        "any other message is left as it is, a `/` in its first word or not: a GraphQL `#import`"
    );
}

#[test]
fn a_session_with_no_file_watcher_says_auto_reload_is_off() {
    let line = status_with("/work/app/paid.py", 80, |app| app.no_watch = true);
    assert!(line.contains("  no auto-reload"), "{line:?}");
}

#[test]
fn a_read_only_message_is_not_repeated_by_the_read_only_mark() {
    let line = status("/work/app/paid.py", "read-only: not UTF-8", 80);
    assert_eq!(line.matches("read-only").count(), 1, "{line:?}");
}

fn clocked(path: &str, message: &str, width: u16) -> String {
    status_with(path, width, |app| {
        app.message = message.into();
        app.clock = Some("14:32".into());
    })
}

#[test]
fn the_clock_takes_the_right_edge_while_the_path_fits_beside_it() {
    let wide = clocked("/work/lib/json.py", "", 120);
    assert!(
        wide.starts_with("/work/lib/json.py  185:5  [code]  read-only  ")
            && wide.ends_with("  14:32"),
        "{wide:?}"
    );
    assert_eq!(
        clocked("/work/lib/json.py", "", 120).len(),
        119,
        "one blank cell after the time, as after `? help` before it"
    );
    let message = clocked("/work/lib/json.py", "1/70", 80);
    assert!(
        message.contains("  1/70") && message.ends_with("  14:32"),
        "{message:?}"
    );
    let long = "dumps: via import json from the standard library";
    assert_eq!(
        clocked("/work/lib/json.py", long, 80),
        status("/work/lib/json.py", long, 80),
        "a message the clock would cut keeps the bar, and the clock goes"
    );
    assert_eq!(
        status_with("/work/lib/json.py", 80, |app| app.clock =
            Some(String::new())),
        status("/work/lib/json.py", "", 80),
        "a format that prints nothing leaves no box"
    );
}

#[test]
fn the_clock_goes_before_the_path_is_shortened() {
    let path = "/srv/app/services/billing/providers/webhooks/handlers/paid.py";
    let bar = "  185:5  [code]  read-only";
    let fits_bare = u16::try_from(path.len() + bar.len()).unwrap();
    assert_eq!(clocked(path, "", fits_bare), format!("{path}{bar}"));
    assert_eq!(
        clocked(path, "", fits_bare + 8),
        format!("{path}{bar}  14:32"),
        "two cells before the time, one after it"
    );
    assert_eq!(clocked(path, "", fits_bare + 7), format!("{path}{bar}"));
    assert!(
        clocked(STDLIB, "", 80).starts_with('\u{2026}'),
        "a path too long anyway is shortened as before"
    );
}

#[test]
fn a_prompt_keeps_the_right_edge_for_its_count() {
    let line = status_with("/work/lib/json.py", 80, |app| {
        app.clock = Some("14:32".into());
        app.mode = crate::app::Mode::Find;
        app.message = "1/70".into();
    });
    assert!(
        line.starts_with('/') && line.ends_with("1/70") && !line.contains("14:32"),
        "{line:?}"
    );
}

#[test]
fn with_no_file_open_the_bar_holds_only_the_message_and_the_clock() {
    let bar = |say: fn(&mut App)| {
        let mut app = App::new(
            PathBuf::from("/work/app"),
            Tree::default(),
            Vec::new(),
            Buffer::empty(),
            None,
        );
        app.clock = Some("14:32".into());
        say(&mut app);
        let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(60, 4)).unwrap();
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        rows(&terminal).pop().unwrap()
    };
    assert_eq!(bar(|_| {}).trim(), "14:32");
    let said = bar(|app| app.message = "start of history".into());
    assert!(
        said.starts_with("start of history") && said.ends_with("14:32"),
        "{said:?}"
    );
    let about = bar(|app| app.say_about(&PathBuf::from("/work/app/gone.rs"), " deleted"));
    assert!(about.starts_with("gone.rs deleted"), "{about:?}");
}
