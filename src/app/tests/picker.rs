use super::*;

#[test]
fn theme_picker_previews_reverts_and_keeps() {
    let names: Vec<&str> = crate::theme::names().collect();
    let mut a = app("x\n");
    a.theme = names[2].to_owned();
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
    assert_eq!(
        (a.shown_theme(), a.picker.is_none()),
        (names[2], true),
        "Esc puts the theme in use back"
    );

    press(&mut a, KeyCode::Char('T'), KeyModifiers::NONE);
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Up, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        (a.shown_theme(), a.theme.as_str()),
        (names[1], names[1]),
        "Enter keeps the new one"
    );
    let kept = format!("theme {}", names[1]);
    assert_eq!((a.mode, a.message.as_str()), (Mode::Normal, kept.as_str()));
}

#[test]
fn enter_in_the_theme_picker_writes_the_config() {
    let dir = std::env::temp_dir().join(format!("merl-theme-config-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
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

#[test]
fn enter_on_a_broken_theme_keeps_the_picker_and_saves_nothing() {
    let dir = std::env::temp_dir().join(format!("merl-theme-broken-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
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
        "theme = \"dayfox\"\n",
        "a theme that does not load is not saved: the next start would exit on it"
    );
    assert_eq!(a.mode, Mode::Picker(PickerKind::Themes));
    assert_eq!((a.picker.is_some(), a.theme.as_str()), (true, "dayfox"));
    assert!(a.message.contains("zz-broken.tmTheme"), "{}", a.message);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[cfg(unix)]
fn a_theme_that_cannot_be_read_or_saved_says_why_in_a_few_words() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("merl-theme-io-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).unwrap();
    let locked = dir.join("zz-locked.tmTheme");
    std::fs::write(&locked, "").unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let config = dir.join("config.toml");
    std::fs::create_dir_all(&config).unwrap();
    let mut a = app("x\n");
    (a.config, a.theme_dir) = (Some(config.clone()), Some(dir.clone()));
    press(&mut a, KeyCode::Char('T'), KeyModifiers::NONE);
    typed(&mut a, "zz-locked");
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
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

fn cursor_on(a: &mut App, file: &str, line: usize, word: &str) {
    a.jump_to(&a.root.join(file), line);
    a.col = a.line_str().find(word).expect(word);
}

fn landed(a: &App) -> (String, usize, usize) {
    (a.rel_path(), a.line + 1, a.col + 1)
}

#[test]
fn enter_on_a_definition_row_lands_on_the_name() {
    let (dir, mut a) = orders_app("pick-d-col");
    cursor_on(&mut a, "app/service.py", 3, "find_by_customer");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    a.picker.as_mut().expect("the by name picker").settle();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        landed(&a),
        ("app/repos.py".into(), 2, 9),
        "lands on the declared name, as a jump with one match does"
    );
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        a.message, "find_by_customer: at a declaration, 1 other by name",
        "the next d asks about that name: its namesake in the fake"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

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
    assert_eq!(
        landed(&a),
        ("app/service.py".into(), 3, 34),
        "the hit found in any case"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

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

#[test]
fn a_row_whose_line_got_shorter_lands_at_its_end() {
    let (dir, mut a) = orders_app("pick-shorter");
    cursor_on(&mut a, "app/service.py", 1, "checkout");
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "find_by_customer(self");
    a.settle_search();
    std::fs::write(dir.join("app/repos.py"), "class OrderRepo:\n  я\n").unwrap();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        landed(&a),
        ("app/repos.py".into(), 2, "  я".len() + 1),
        "the line got shorter after the search: the cursor stops at its end"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

fn noise(alphabet: &[char], n: usize) -> String {
    let mut x: u32 = 1;
    (0..n)
        .map(|_| {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            alphabet[(x >> 16) as usize % alphabet.len()]
        })
        .collect()
}

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
        let (pasted, last) = query.split_at(query.len() - query.chars().last().unwrap().len_utf8());
        a.picker.as_mut().unwrap().query = LineEdit::typed(pasted);
        typed(&mut a, last);
        a.settle_search();
        press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(landed(&a), ("quote.py".into(), line, 6));
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_search_query_ending_in_a_space_lands_on_the_hit() {
    let (dir, mut a) = project_app("pick-s-space", &[("limits.py", "ROWS = 10 \n")]);
    cursor_on(&mut a, "limits.py", 1, "ROWS");
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "10 ");
    a.settle_search();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        landed(&a),
        ("limits.py".into(), 1, 8),
        "the blanks the row's text is trimmed of are still the line's"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

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
    assert_eq!(
        landed(&a),
        ("Makefile".into(), 7, 26),
        "in a Makefile `-` is part of a word: on build-image, not inside build-image-arm"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

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
    assert_eq!(
        landed(&a),
        ("release.sh".into(), 1, 22),
        "build-image from a Makefile, whole in a shell script"
    );
    cursor_on(&mut a, "app.py", 1, "app");
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    let picker = a.picker.as_mut().expect("a picker");
    picker.settle();
    assert_eq!(
        picker.counts().0,
        2,
        "YAML reads my-app as one name: app.py:1 and ci.yml:3"
    );
    enter_on_row(&mut a, "ci.yml", 3);
    assert_eq!(
        landed(&a),
        ("ci.yml".into(), 3, 16),
        "the row is listed for the whole `app`, and lands there, not inside `my-app`"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

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
    assert_eq!(
        landed(&a),
        ("Makefile".into(), 6, 17),
        "from a row of the by name list"
    );
    std::fs::remove_dir_all(&two).unwrap();
}

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
        assert_eq!(
            (a.mode, a.picker.is_none()),
            (Mode::Normal, true),
            "Esc still closes it"
        );
    }
}

#[test]
fn the_file_picker_ranks_a_match_in_the_file_name_first() {
    let (dir, mut a) = project_app("paths", &[("fob-x/y.rs", ""), ("a/fob.rs", "")]);
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
    typed(&mut a, "fob");
    let p = a.picker.as_mut().unwrap();
    p.settle();
    let rows: Vec<String> = p.rows(0, 2).into_iter().map(|r| r.item.label).collect();
    assert_eq!(rows, ["a/fob.rs", "fob-x/y.rs"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

fn files_listed(a: &mut App, query: &str) -> Vec<(String, &'static str)> {
    press(a, KeyCode::Char('o'), KeyModifiers::NONE);
    typed(a, query);
    let p = a.picker.as_mut().unwrap();
    p.settle();
    let labels = p.labels.clone();
    let tag = |i: usize| labels.iter().find(|t| t.0 == i).map_or("", |t| t.1);
    let rows = p.window(10).0.into_iter().enumerate();
    let listed = rows.map(|(i, r)| (r.item.label, tag(i))).collect();
    press(a, KeyCode::Esc, KeyModifiers::NONE);
    listed
}

fn rows(listed: &[(String, &'static str)]) -> Vec<String> {
    listed.iter().map(|r| r.0.clone()).collect()
}

fn recent_app(tag: &str) -> (PathBuf, App) {
    let names = ["src/aa.txt", "src/bb.txt", "cc.txt", "dd.txt"];
    project_app(tag, &names.map(|n| (n, "x\n")))
}

#[test]
fn an_empty_o_lists_the_files_opened_last_first_with_the_cursor_on_the_one_before() {
    let (dir, mut a) = recent_app("recent-empty");
    for f in ["src/bb.txt", "cc.txt", "dd.txt", "cc.txt"] {
        a.jump_to(&dir.join(f), 1);
    }
    let listed = files_listed(&mut a, "");
    assert_eq!(rows(&listed), ["cc.txt", "dd.txt", "src/bb.txt"]);
    assert_eq!(listed[0].1, "recently opened");
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
    a.picker.as_mut().unwrap().settle();
    assert_eq!(a.picker.as_ref().unwrap().selected, 1);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(a.rel_path(), "dd.txt");
    assert_eq!(
        rows(&files_listed(&mut a, "")),
        ["dd.txt", "cc.txt", "src/bb.txt"]
    );
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn a_query_lists_the_recent_matches_by_name_then_the_project_without_repeats() {
    let (dir, mut a) = recent_app("recent-query");
    for f in ["dd.txt", "src/bb.txt"] {
        a.jump_to(&dir.join(f), 1);
    }
    let listed = files_listed(&mut a, "txt");
    assert_eq!(
        listed,
        [
            ("src/bb.txt".to_owned(), "recently opened"),
            ("dd.txt".to_owned(), ""),
            ("cc.txt".to_owned(), "file results"),
            ("src/aa.txt".to_owned(), ""),
        ]
    );
    let by_name = files_listed(&mut a, "src");
    assert_eq!(by_name[0], ("src/aa.txt".to_owned(), "file results"));
    assert_eq!(rows(&by_name), ["src/aa.txt", "src/bb.txt"]);
    let by_path = files_listed(&mut a, "src/");
    assert_eq!(by_path[0], ("src/bb.txt".to_owned(), "recently opened"));
    assert_eq!(rows(&by_path), ["src/bb.txt", "src/aa.txt"]);
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn the_history_outlives_the_session_and_forgets_deleted_files() {
    let (dir, mut a) = recent_app("recent-kept");
    let store = dir.join(".git/merl/recent");
    a.recent_file = Some(store.clone());
    for f in ["cc.txt", "src/aa.txt", "dd.txt"] {
        a.jump_to(&dir.join(f), 1);
    }
    std::fs::remove_file(dir.join("src/aa.txt")).unwrap();
    let (tree, files) = crate::tree::build(&dir, false);
    let mut b = App::new(dir.clone(), tree, files, Buffer::empty(), None);
    b.recent_file = Some(store.clone());
    assert_eq!(rows(&files_listed(&mut b, "")), ["dd.txt", "cc.txt"]);
    b.jump_to(&dir.join("src/bb.txt"), 1);
    let kept = std::fs::read_to_string(&store).unwrap();
    let names: Vec<&str> = kept
        .lines()
        .map(|l| l.rsplit('/').next().unwrap())
        .collect();
    assert_eq!(names, ["bb.txt", "dd.txt", "cc.txt"]);
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn o_in_a_review_keeps_the_review_order() {
    let (dir, mut a) = review_app("recent-review");
    a.jump_to(&dir.join("src/keep.rs"), 1);
    a.jump_to(&dir.join("tail"), 1);
    let listed = files_listed(&mut a, "");
    assert_eq!(rows(&listed)[..3], ["src/a.rs", "crlf.txt", "new"]);
    assert!(listed.iter().all(|r| r.1.is_empty()));
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn a_file_outside_the_project_leaves_the_cursor_on_the_first_row() {
    let (dir, mut a) = recent_app("recent-outside");
    let outside = std::env::temp_dir().join(format!("merl-outside-{}.txt", std::process::id()));
    std::fs::write(&outside, "x\n").unwrap();
    a.jump_to(&dir.join("cc.txt"), 1);
    a.jump_to(&dir.join("dd.txt"), 1);
    a.jump_to(&outside, 1);
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
    a.picker.as_mut().unwrap().settle();
    assert_eq!(a.picker.as_ref().unwrap().selected, 0);
    drop((std::fs::remove_dir_all(dir), std::fs::remove_file(outside)));
}

#[test]
fn the_open_file_stays_on_top_when_another_merl_wrote_last() {
    let (dir, mut a) = recent_app("recent-other");
    let store = dir.join(".git/merl/recent");
    a.recent_file = Some(store.clone());
    a.jump_to(&dir.join("cc.txt"), 1);
    a.jump_to(&dir.join("dd.txt"), 1);
    let theirs = format!("{}\n", dir.join("src/aa.txt").display());
    let ours = std::fs::read_to_string(&store).unwrap();
    std::fs::write(&store, theirs + &ours).unwrap();
    assert_eq!(
        rows(&files_listed(&mut a, "")),
        ["dd.txt", "src/aa.txt", "cc.txt"]
    );
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
    a.picker.as_mut().unwrap().settle();
    assert_eq!(a.picker.as_ref().unwrap().selected, 1);
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn a_recent_file_matched_by_name_is_marked_in_its_name() {
    let (dir, mut a) = project_app("recent-marks", &[("abc/xabc.rs", "x\n"), ("q.rs", "x\n")]);
    a.jump_to(&dir.join("abc/xabc.rs"), 1);
    a.jump_to(&dir.join("q.rs"), 1);
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
    typed(&mut a, "abc");
    let p = a.picker.as_mut().unwrap();
    p.settle();
    let row = p.window(10).0.remove(0);
    assert_eq!(
        (row.item.label.as_str(), row.matched),
        ("abc/xabc.rs", vec![5, 6, 7])
    );
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn an_erased_query_puts_the_cursor_back_on_the_file_before() {
    let (dir, mut a) = recent_app("recent-erased");
    a.jump_to(&dir.join("cc.txt"), 1);
    a.jump_to(&dir.join("dd.txt"), 1);
    press(&mut a, KeyCode::Char('o'), KeyModifiers::NONE);
    typed(&mut a, "x");
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Backspace, KeyModifiers::NONE);
    a.picker.as_mut().unwrap().settle();
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(a.rel_path(), "cc.txt");
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn the_tutor_lists_every_file_as_its_tasks_expect() {
    let (dir, mut a) = recent_app("recent-tutor");
    a.tutor = Some(Tutor {
        step: 0,
        dir: dir.clone(),
        drill: None,
        card: tutor::Card::Down,
    });
    a.jump_to(&dir.join("dd.txt"), 1);
    assert_eq!(
        rows(&files_listed(&mut a, "")),
        ["src/aa.txt", "src/bb.txt", "cc.txt", "dd.txt"]
    );
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn a_definition_row_is_its_place_and_code_with_the_class_and_only_a_differing_reason_before_it() {
    let heads = |a: &mut App| -> Vec<String> {
        let p = a.picker.as_mut().expect("a picker");
        p.settle();
        let rows = p.window(50).0;
        rows.iter()
            .map(|r| {
                r.item
                    .label
                    .get(..r.item.code_at.unwrap())
                    .unwrap()
                    .to_owned()
            })
            .collect()
    };
    let (dir, mut a) = project_app(
        "rows",
        &[
            ("a.py", "class A:\n    def run(self):\n        pass\n"),
            ("b.py", "def run():\n    pass\n"),
            ("c.ts", "class C {\n  run() {}\n}\n"),
            ("d.ts", "class D {\n  run() {}\n}\n"),
            ("main.ts", "export function go(x) {\n  x.run();\n}\n"),
            ("pkg/__init__.py", ""),
            ("pkg/one.py", "def pick(x):\n    return x\n"),
            ("pkg/two.py", "def pick(x):\n    return x\n"),
            (
                "use.py",
                "try:\n    from pkg.one import pick\nexcept ImportError:\n    from pkg.two import pick\n\n\ndef run():\n    return pick(1)\n",
            ),
        ],
    );
    a.external
        .insert(Kind::Python, (Vec::new(), Arc::new(Vec::new())));
    d_on(&mut a, "b.py", "^def run");
    assert_eq!(a.message, "run: at a declaration, 2 others by name");
    assert_eq!(
        heads(&mut a),
        ["A  a.py:2: ", "use.py:7: "],
        "a method's class, nothing for a function at the top level, and no `by name` the title says"
    );
    a.picker = None;
    d_on(&mut a, "main.ts", "x.run");
    assert_eq!(
        heads(&mut a),
        ["C  c.ts:2: ", "D  d.ts:2: "],
        "TypeScript's class too (#1328)"
    );
    a.picker = None;
    d_on(&mut a, "use.py", "return pick");
    assert_eq!(a.message, "pick: 2 declarations");
    assert_eq!(
        heads(&mut a),
        [
            "via import pkg/one.py  pkg/one.py:1: ",
            "via import pkg/two.py  pkg/two.py:1: "
        ],
        "rows found in different ways keep their reason"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_definition_row_s_class_is_its_declaration_s_and_comes_before_a_differing_reason() {
    let (dir, a) = project_app(
        "heads",
        &[
            (
                "lib.rs",
                "struct A;\nimpl A {\n    fn run(&self) {}\n}\nfn run() {}\n",
            ),
            (
                "one.py",
                "class Box:\n    def slugify(self):\n        pass\n",
            ),
            ("two.py", "def slugify(x):\n    return x\n"),
            ("view.ts", "class C {\n  run() {}\n}\n"),
            ("view.rb", "class C\n  def run\n  end\nend\n"),
            ("view.dart", "class C {\n  void run() {}\n}\n"),
            (
                "view.lua",
                "local M = {}\nM.set = function(k)\n  _G.kong = k\nend\n",
            ),
        ],
    );
    let found = |path: &str, line1: usize, reason: Reason| Candidate {
        hit: Hit {
            path: PathBuf::from(path),
            line1,
            byte_col: None,
            text: std::fs::read_to_string(dir.join(path))
                .unwrap()
                .lines()
                .nth(line1 - 1)
                .unwrap()
                .to_owned(),
            deleted: None,
        },
        reason,
    };
    let heads = |kind: Kind, word: &str, found: Vec<Candidate>| -> Vec<String> {
        a.definition_items(kind, word, found)
            .into_iter()
            .map(|i| i.label.get(..i.code_at.unwrap()).unwrap().to_owned())
            .collect()
    };
    assert_eq!(
        heads(
            Kind::Rust,
            "run",
            vec![
                found("lib.rs", 3, Reason::ByName),
                found("lib.rs", 5, Reason::ByName)
            ]
        ),
        ["A  lib.rs:3: ", "lib.rs:5: "],
        "a Rust method's impl, nothing for a free function"
    );
    assert_eq!(
        heads(
            Kind::Python,
            "slug",
            vec![
                found("one.py", 2, Reason::ByName),
                found("two.py", 1, Reason::Import("two.py".into()))
            ]
        ),
        ["by name  one.py:2: ", "via import two.py  two.py:1: "],
        "a name bound by an alias is no declaration of `slug`: its line names no class for it"
    );
    assert_eq!(
        heads(
            Kind::Python,
            "slugify",
            vec![
                found("one.py", 2, Reason::ByName),
                found("two.py", 1, Reason::Import("two.py".into()))
            ]
        ),
        ["Box  by name  one.py:2: ", "via import two.py  two.py:1: "],
        "the class first, then the reason of a list found in different ways"
    );
    assert_eq!(
        heads(Kind::TsJs, "run", vec![found("view.ts", 2, Reason::ByName)]),
        ["C  view.ts:2: "],
        "TypeScript's class is measured (#1328)"
    );
    assert_eq!(
        heads(Kind::Ruby, "run", vec![found("view.rb", 2, Reason::ByName)]),
        ["C  view.rb:2: "],
        "Ruby's class is measured (#1329)"
    );
    assert_eq!(
        heads(
            Kind::Dart,
            "run",
            vec![found("view.dart", 2, Reason::ByName)]
        ),
        ["view.dart:2: "],
        "no class where it is not proven"
    );
    assert_eq!(
        heads(
            Kind::Lua,
            "kong",
            vec![found("view.lua", 3, Reason::ByName)]
        ),
        ["set  view.lua:3: "],
        "Lua's class is measured (#1332)"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
