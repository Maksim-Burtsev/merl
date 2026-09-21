//! The theme picker.

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
