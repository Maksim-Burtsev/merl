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
