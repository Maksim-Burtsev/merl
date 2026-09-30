//! The theme picker, and where Enter on a row of the other pickers lands.

use super::*;

/// `T` previews the theme under the cursor without committing to it: Esc puts the one in
/// use back, Enter keeps the new one.
#[test]
fn theme_picker_previews_reverts_and_keeps() {
    let names: Vec<&str> = crate::theme::names().collect();
    let mut a = app("x\n");
    a.theme = names[2].to_string();
    press(&mut a, KeyCode::Char('T'), KeyModifiers::SHIFT);
    a.picker.as_mut().unwrap().settle();
    assert_eq!(a.mode, Mode::Picker(PickerKind::Themes));
    assert_eq!(
        a.shown_theme(),
        names[2],
        "the cursor starts on the theme in use"
    );

    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!((a.shown_theme(), a.theme.as_str()), (names[3], names[2]));
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!((a.shown_theme(), a.picker.is_none()), (names[2], true));

    press(&mut a, KeyCode::Char('T'), KeyModifiers::NONE);
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Up, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((a.shown_theme(), a.theme.as_str()), (names[1], names[1]));
    // Named from the table, so reordering or renaming a theme cannot fail this test.
    let kept = format!("theme {}", names[1]);
    assert_eq!((a.mode, a.message.as_str()), (Mode::Normal, kept.as_str()));
}

#[test]
fn enter_in_the_theme_picker_writes_the_config() {
    let dir = std::env::temp_dir().join(format!("merl-theme-config-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut a = app("x\n");
    a.config = Some(dir.join("config.toml"));
    press(&mut a, KeyCode::Char('T'), KeyModifiers::NONE);
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    let name = crate::theme::names().nth(1).unwrap();
    assert_eq!(a.message, format!("theme {name} saved"));
    assert_eq!(
        std::fs::read_to_string(dir.join("config.toml")).unwrap(),
        format!("theme = \"{name}\"\n")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Enter on a user theme that does not load saves nothing: the next start would exit on it
/// (#277). The picker stays open with the load error in the status bar.
#[test]
fn enter_on_a_broken_theme_keeps_the_picker_and_saves_nothing() {
    let dir = std::env::temp_dir().join(format!("merl-theme-broken-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("zz-broken.tmTheme"), "not a plist").unwrap();
    let config = dir.join("config.toml");
    std::fs::write(&config, "theme = \"dayfox\"\n").unwrap();
    let mut a = app("x\n");
    (a.theme, a.config, a.theme_dir) = ("dayfox".into(), Some(config.clone()), Some(dir.clone()));
    press(&mut a, KeyCode::Char('T'), KeyModifiers::NONE);
    typed(&mut a, "zz-broken");
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        std::fs::read_to_string(&config).unwrap(),
        "theme = \"dayfox\"\n"
    );
    assert_eq!(a.mode, Mode::Picker(PickerKind::Themes));
    assert_eq!((a.picker.is_some(), a.theme.as_str()), (true, "dayfox"));
    assert!(a.message.contains("zz-broken.tmTheme"), "{}", a.message);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #516: a theme file merl may not read, and a config it cannot write, say why in a few words
/// after the path, never with the OS text's `(os error N)`.
#[test]
#[cfg(unix)]
fn a_theme_that_cannot_be_read_or_saved_says_why_in_a_few_words() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("merl-theme-io-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let locked = dir.join("zz-locked.tmTheme");
    std::fs::write(&locked, "").unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    // The config's path is a folder: it cannot be read, so the theme is not saved.
    let config = dir.join("config.toml");
    std::fs::create_dir_all(&config).unwrap();
    let mut a = app("x\n");
    (a.config, a.theme_dir) = (Some(config.clone()), Some(dir.clone()));
    press(&mut a, KeyCode::Char('T'), KeyModifiers::NONE);
    typed(&mut a, "zz-locked");
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    // As root every file is readable, and the empty theme fails to parse instead.
    if std::fs::read(&locked).is_err() {
        let want = format!("{}: permission denied", locked.display());
        assert_eq!(a.message, want);
    }
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('T'), KeyModifiers::NONE);
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(a.message, format!("{}: is a directory", config.display()));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A repository class, a fake of it in the tests and a service that calls both of its methods:
/// the project of #236's table, cut down.
fn orders_app(tag: &str) -> (PathBuf, App) {
    project_app(
        tag,
        &[
            (
                "app/repos.py",
                "class OrderRepo:\n    def find_by_customer(self, customer_id):\n        \
                 return []\n\n    def save(self, order):\n        pass\n\n    \
                 def delete(self, order):\n        pass\n",
            ),
            (
                "tests/fake_repo.py",
                "class FakeRepo:\n    def find_by_customer(self, customer_id):\n        \
                 return []\n",
            ),
            (
                "app/service.py",
                "def checkout(repo, order):\n    repo.save(order)\n    \
                 return repo.find_by_customer(order.customer_id)\n",
            ),
        ],
    )
}

/// Puts the cursor on `word` in `file` at `line`.
fn cursor_on(a: &mut App, file: &str, line: usize, word: &str) {
    a.jump_to(&a.root.join(file), line);
    a.col = a.line_str().find(word).expect(word);
}

/// Where the cursor is, as the status line says it: the file, then line and column from 1.
fn landed(a: &App) -> (String, usize, usize) {
    (a.rel_path(), a.line + 1, a.col + 1)
}

/// Enter on a row of `d`'s `by name` picker lands on the declared name, as a jump with one
/// match does (#236), so the next `d` asks about that name: here, its namesake in the fake.
#[test]
fn enter_on_a_definition_row_lands_on_the_name() {
    let (dir, mut a) = orders_app("pick-d-col");
    cursor_on(&mut a, "app/service.py", 3, "find_by_customer");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    a.picker.as_mut().expect("the by name picker").settle();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(landed(&a), ("app/repos.py".into(), 2, 9));
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        a.message,
        "find_by_customer: at a declaration, 1 other by name"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Enter on a row of `u` lands on the word in that line: the name on the declaration row, the
/// use on the others, so `u` there lists the same uses again (#236).
#[test]
fn enter_on_a_usage_row_lands_on_the_use() {
    let (dir, mut a) = orders_app("pick-u-col");
    cursor_on(&mut a, "app/service.py", 2, "save");
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    a.picker.as_mut().expect("the usages").settle();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(landed(&a), ("app/repos.py".into(), 5, 9), "the declaration");
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    a.picker.as_mut().expect("the usages again").settle();
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(landed(&a), ("app/service.py".into(), 2, 10), "the call");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Enter on a row of `D` lands on the name the row lists (#236).
#[test]
fn enter_on_a_symbol_row_lands_on_the_name() {
    let (dir, mut a) = orders_app("pick-sym-col");
    cursor_on(&mut a, "app/service.py", 1, "checkout");
    press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
    typed(&mut a, "find_by_cus");
    a.picker.as_mut().expect("the symbols").settle();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(landed(&a), ("app/repos.py".into(), 2, 9));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Enter on a row of `s` lands where the hit starts, found as the grep found it, in any case
/// (#236).
#[test]
fn enter_on_a_search_row_lands_on_the_hit() {
    let (dir, mut a) = orders_app("pick-s-col");
    cursor_on(&mut a, "app/service.py", 1, "checkout");
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "def delete");
    a.settle_search();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(landed(&a), ("app/repos.py".into(), 8, 5));
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "Order.Customer");
    a.settle_search();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(landed(&a), ("app/service.py".into(), 3, 34));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `[` and `]` come back to the word a row landed on, not to the start of its line: the column
/// is set before the jump's stop is made.
#[test]
fn back_and_forward_return_to_the_word_a_row_landed_on() {
    let (dir, mut a) = orders_app("pick-hist-col");
    cursor_on(&mut a, "app/service.py", 3, "find_by_customer");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    a.picker.as_mut().expect("the by name picker").settle();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(landed(&a), ("app/service.py".into(), 3, 17));
    press(&mut a, KeyCode::Char(']'), KeyModifiers::NONE);
    assert_eq!(landed(&a), ("app/repos.py".into(), 2, 9));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A `D` row whose name is not a `name` of its pattern lands on it too: a Terraform block on
/// its first label, a YAML anchor on its name after the `&`.
#[test]
fn enter_on_a_terraform_or_yaml_symbol_row_lands_on_its_name() {
    let (dir, mut a) = project_app(
        "pick-sym-infra",
        &[
            (
                "main.tf",
                "variable \"region\" {}\n\nresource \"aws_instance\" \"web\" {\n}\n",
            ),
            ("ci.yml", "defaults: &base\n  image: rust\n"),
        ],
    );
    cursor_on(&mut a, "ci.yml", 2, "image");
    for (query, place) in [
        ("var.region", ("main.tf", 1, 11)),
        ("aws_instance.web", ("main.tf", 3, 11)),
        ("&base", ("ci.yml", 1, 12)),
    ] {
        press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
        typed(&mut a, query);
        a.picker.as_mut().expect("the symbols").settle();
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(landed(&a), (place.0.into(), place.1, place.2), "{query}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A file cut shorter between the search and Enter: the row's column is past its line's end,
/// and the cursor stops at the end rather than aborting merl.
#[test]
fn a_row_whose_line_got_shorter_lands_at_its_end() {
    let (dir, mut a) = orders_app("pick-shorter");
    cursor_on(&mut a, "app/service.py", 1, "checkout");
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "find_by_customer(self");
    a.settle_search();
    std::fs::write(dir.join("app/repos.py"), "class OrderRepo:\n  я\n").unwrap();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(landed(&a), ("app/repos.py".into(), 2, "  я".len() + 1));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Letters of `alphabet` in an order that does not repeat itself, so a long run of them overlaps
/// itself nowhere and the grep stays linear.
fn noise(alphabet: &[char], n: usize) -> String {
    let mut x: u32 = 1;
    (0..n)
        .map(|_| {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            alphabet[(x >> 16) as usize % alphabet.len()]
        })
        .collect()
}

/// A query the grep takes, however long, lands on its hit. It was compiled a second time for
/// the column, under the `regex` crate's smaller size limit, and that compile aborted the search.
#[test]
fn a_huge_search_query_lands_on_its_hit() {
    let cyrillic = noise(&('а'..='я').collect::<Vec<_>>(), 70_000);
    let ascii = noise(&('a'..='z').collect::<Vec<_>>(), 200_000);
    let (dir, mut a) = project_app(
        "pick-s-huge",
        &[(
            "quote.py",
            &format!("x = \"{cyrillic}\"\ny = \"{ascii}\"\n"),
        )],
    );
    cursor_on(&mut a, "quote.py", 1, "x");
    for (query, line) in [(&cyrillic, 1), (&ascii, 2)] {
        press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
        // Pasted, as a person would: typing it would take one press per letter.
        let (head, last) = query.split_at(query.len() - query.chars().last().unwrap().len_utf8());
        a.picker.as_mut().unwrap().query = LineEdit::typed(head);
        typed(&mut a, last);
        a.settle_search();
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(landed(&a), ("quote.py".into(), line, 6));
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A query that ends in a space lands on its hit at the end of a line: the blanks the row's text
/// is trimmed of are still the line's.
#[test]
fn a_search_query_ending_in_a_space_lands_on_the_hit() {
    let (dir, mut a) = project_app("pick-s-space", &[("limits.py", "ROWS = 10 \n")]);
    cursor_on(&mut a, "limits.py", 1, "ROWS");
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "10 ");
    a.settle_search();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(landed(&a), ("limits.py".into(), 1, 8));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Moves the open picker to the row of `file` at `line`, from 1, and presses Enter on it.
fn enter_on_row(a: &mut App, file: &str, line: usize) {
    let picker = a.picker.as_mut().expect("a picker");
    picker.settle();
    for _ in 0..picker.counts().0 {
        if picker
            .current()
            .is_some_and(|it| it.path.as_os_str() == file && it.line == line)
        {
            break;
        }
        picker.key(KeyCode::Down.into());
    }
    assert_eq!(
        picker.current().map(|it| (it.path.clone(), it.line)),
        Some((PathBuf::from(file), line)),
        "no row {file}:{line}"
    );
    press(a, KeyCode::Enter, KeyModifiers::NONE);
}

/// In a Makefile `-` is part of a word, as `u` reads the word under the cursor there: a row for
/// `build-image` lands on that target, not on the start of `build-image-arm` before it.
#[test]
fn enter_on_a_usage_row_in_a_makefile_lands_on_the_whole_target() {
    let (dir, mut a) = project_app(
        "pick-u-make",
        &[(
            "Makefile",
            "build-image-arm:\n\tdocker build --platform arm64 .\n\nbuild-image:\n\t\
             docker build .\n\nrelease: build-image-arm build-image\n\techo done\n",
        )],
    );
    cursor_on(&mut a, "Makefile", 4, "build-image");
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    enter_on_row(&mut a, "Makefile", 7);
    assert_eq!(landed(&a), ("Makefile".into(), 7, 26));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A `u` row in a file of another language lands on the word whole as `u` read it under the
/// cursor and as the row's own file reads it: `build-image` from a Makefile in a shell script.
/// The row's own language decides whether it is listed: YAML reads `my-app` as one name, so
/// `app` from Python has no row for it, and a row listed for a whole `app` beside it lands on
/// that one (#281).
#[test]
fn a_usage_row_in_a_file_of_another_language_lands_on_the_word() {
    let (dir, mut a) = project_app(
        "pick-u-kinds",
        &[
            (
                "Makefile",
                "build-image-arm:\n\ttrue\n\nbuild-image:\n\ttrue\n",
            ),
            ("release.sh", "make build-image-arm build-image\n"),
            ("app.py", "app = object()\n"),
            (
                "ci.yml",
                "build:\n  image: my-app:latest\n  name: my-app app\n",
            ),
        ],
    );
    cursor_on(&mut a, "Makefile", 4, "build-image");
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    enter_on_row(&mut a, "release.sh", 1);
    assert_eq!(landed(&a), ("release.sh".into(), 1, 22));
    cursor_on(&mut a, "app.py", 1, "app");
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    let picker = a.picker.as_mut().expect("a picker");
    picker.settle();
    assert_eq!(picker.counts().0, 2, "app.py:1 and ci.yml:3");
    // The row is listed for the whole `app`, and lands there, not inside `my-app`.
    enter_on_row(&mut a, "ci.yml", 3);
    assert_eq!(landed(&a), ("ci.yml".into(), 3, 16));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `d` reads a Makefile target with its `-`, and lands on it by the same rule: on `build-image`
/// of `build-image-arm build-image:`, not on the start of the line, whether it jumps at once or
/// from a row of its `by name` list.
#[test]
fn d_lands_on_a_makefile_target_after_a_longer_one() {
    let rule = "build-image-arm build-image:\n\tdocker build .\n\n";
    let (one, mut a) = project_app(
        "d-make-one",
        &[("Makefile", &format!("{rule}all: build-image\n"))],
    );
    cursor_on(&mut a, "Makefile", 4, "build-image");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert!(a.picker.is_none(), "one match: {}", a.message);
    assert_eq!(landed(&a), ("Makefile".into(), 1, 17));
    std::fs::remove_dir_all(&one).unwrap();

    let (two, mut a) = project_app(
        "d-make-two",
        &[("Makefile", &format!("{rule}all: build-image\n\n{rule}"))],
    );
    cursor_on(&mut a, "Makefile", 4, "build-image");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    enter_on_row(&mut a, "Makefile", 6);
    assert_eq!(landed(&a), ("Makefile".into(), 6, 17));
    std::fs::remove_dir_all(&two).unwrap();
}

/// Enter in a list with nothing matched does nothing: the list and its query stay, and Esc
/// still closes it (#288).
#[test]
fn enter_with_nothing_matched_keeps_the_list_and_its_query() {
    let mut a = app("x\n");
    for (key, kind) in [('o', PickerKind::Files), ('T', PickerKind::Themes)] {
        press(&mut a, KeyCode::Char(key), KeyModifiers::NONE);
        typed(&mut a, "qqqqzz");
        a.picker.as_mut().unwrap().settle();
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        let query = a.picker.as_ref().map(|p| p.query.to_string());
        assert_eq!(
            (a.mode, query.as_deref()),
            (Mode::Picker(kind), Some("qqqqzz"))
        );
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!((a.mode, a.picker.is_none()), (Mode::Normal, true));
    }
}
