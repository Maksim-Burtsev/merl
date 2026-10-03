use super::*;

fn d_at(a: &mut App, dir: &Path, file: &str, line: usize, at: &str) {
    a.jump_to(&dir.join(file), line);
    a.col = a.line_str().find(at).expect(at);
    press(a, KeyCode::Char('d'), KeyModifiers::NONE);
}

#[test]
fn d_offers_the_plain_values_folder_before_a_translation_of_any_module() {
    let (dir, mut a) = project_app(
        "android-values-order",
        &[
            (
                "app/src/main/res/values-de/strings.xml",
                "<resources>\n    <string name=\"greeting\">Hallo</string>\n</resources>\n",
            ),
            (
                "core/src/main/res/values/strings.xml",
                "<resources>\n    <string name=\"greeting\">Hello</string>\n</resources>\n",
            ),
            (
                "app/src/main/kotlin/Hello.kt",
                "fun hello() = stringResource(R.string.greeting)\n",
            ),
        ],
    );
    d_at(&mut a, &dir, "app/src/main/kotlin/Hello.kt", 1, "greeting");
    let picker = a.picker.as_mut().expect("a picker");
    picker.settle();
    let rows: Vec<String> = (picker.window(50).0.into_iter())
        .map(|r| r.item.label.clone())
        .collect();
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert!(
        rows[0].contains("core/src/main/res/values/strings.xml"),
        "{rows:?}"
    );
    assert!(
        rows[1].contains("app/src/main/res/values-de/strings.xml"),
        "{rows:?}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn d_leaves_a_local_libs_of_a_build_script_to_the_kotlin_rules() {
    let (dir, mut a) = project_app(
        "android-local-libs",
        &[
            (
                "gradle/libs.versions.toml",
                "[libraries]\ngoogle-oss-licenses = { module = \"a:b\" }\n",
            ),
            (
                "app/build.gradle.kts",
                "val libs = Deps()\n\ndependencies {\n    implementation(libs.google.oss.licenses)\n}\n",
            ),
        ],
    );
    d_at(&mut a, &dir, "app/build.gradle.kts", 4, "licenses");
    assert!(!a.message.contains("libs.versions.toml"), "{}", a.message);
    assert_eq!(
        a.rel_current().as_deref(),
        Some(Path::new("app/build.gradle.kts"))
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
