use super::*;

#[test]
fn d_on_a_cmake_module_or_package_the_project_lacks_opens_it_outside() {
    let (dir, mut a) = project_app(
        "cmake-outside",
        &[
            (
                "CMakeLists.txt",
                "include(FetchContent)\nfind_package(fmt)\ninclude(Missing)\nFetchContent_Declare(gtest URL x)\n",
            ),
            ("cmake/Local.cmake", "set(X 1)\n"),
        ],
    );
    let root = external_root(
        "cmake",
        &[
            (
                "Modules/FetchContent.cmake",
                "# FetchContent\nfunction(FetchContent_Declare contentName)\nendfunction()\n",
            ),
            ("cellar/fmt/fmt-config.cmake", "# fmt\n"),
        ],
    );
    std::fs::create_dir_all(root.join("lib")).unwrap();
    std::os::unix::fs::symlink(root.join("cellar/fmt"), root.join("lib/fmt")).unwrap();
    use_roots(
        &mut a,
        Kind::Cmake,
        &[root.join("Modules"), root.join("lib")],
    );
    let at = |p: &str| format!("{}", root.join(p).display());
    for (code, want) in [
        (
            "include(FetchContent",
            jump(
                &format!("FetchContent: module {}", at("Modules/FetchContent.cmake")),
                &at("Modules/FetchContent.cmake:1"),
            ),
        ),
        (
            "find_package(fmt",
            jump(
                &format!("fmt: module {}", at("lib/fmt/fmt-config.cmake")),
                &at("lib/fmt/fmt-config.cmake:1"),
            ),
        ),
        (
            "include(Missing",
            jump("no definition for Missing", "CMakeLists.txt:3"),
        ),
        (
            "FetchContent_Declare",
            jump(
                "FetchContent_Declare: by name, 1 match",
                &at("Modules/FetchContent.cmake:2"),
            ),
        ),
    ] {
        d_on(&mut a, "CMakeLists.txt", code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn u_reads_a_cmake_alias_and_a_dashed_target_whole() {
    let (dir, mut a) = project_app(
        "cmake-u",
        &[
            (
                "CMakeLists.txt",
                "add_library(shop-core STATIC a.cpp)\nadd_library(shop-core-x STATIC b.cpp)\nadd_library(Shop::core ALIAS shop-core)\nset(core 1)\n",
            ),
            (
                "app/CMakeLists.txt",
                "target_link_libraries(app PRIVATE Shop::core shop-core-x)\nmessage(${core})\n",
            ),
        ],
    );
    let rows = |a: &mut App, file: &str, line: usize, at: &str| {
        a.jump_to(&dir.join(file), line);
        a.col = a.line_str().find(at).expect(at);
        press(a, KeyCode::Char('u'), KeyModifiers::NONE);
        let picker = a.picker.as_mut().expect("a picker");
        picker.settle();
        let rows: Vec<String> = picker
            .window(50)
            .0
            .into_iter()
            .map(|r| {
                r.item.label[..r.item.code_at.unwrap()]
                    .trim_end()
                    .to_owned()
            })
            .collect();
        press(a, KeyCode::Esc, KeyModifiers::NONE);
        rows
    };
    let core = rows(&mut a, "app/CMakeLists.txt", 1, "core shop");
    assert_eq!(core.len(), 2, "{core:?}");
    assert!(
        core.iter().all(|r| !r.ends_with("CMakeLists.txt:4:")),
        "{core:?}"
    );
    let dashed = rows(&mut a, "CMakeLists.txt", 1, "core STATIC");
    assert_eq!(dashed.len(), 2, "{dashed:?}");
    assert!(dashed.iter().all(|r| !r.contains(":2:")), "{dashed:?}");
    std::fs::remove_dir_all(&dir).unwrap();
}
