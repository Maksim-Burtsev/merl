use super::*;

fn declares(line: &str, word: &str) -> bool {
    Regex::new(&cmake_patterns(word).join("|"))
        .unwrap()
        .is_match(line)
}

#[test]
fn cmake_declaration_forms() {
    for (line, word) in [
        ("function(shop_add_library name)", "shop_add_library"),
        ("FUNCTION(Shop_Add_Library name)", "shop_add_library"),
        ("Function (shop_add_library)", "SHOP_ADD_LIBRARY"),
        ("  macro(shop_option name default)", "shop_option"),
        ("set(SHOP_WARNINGS -Wall -Wextra)", "SHOP_WARNINGS"),
        (r#"set(SHOP_VERSION 1.2 CACHE STRING "v")"#, "SHOP_VERSION"),
        ("SET(SHOP_LIST)", "SHOP_LIST"),
        (r#"option(SHOP_TESTS "Build the tests" ON)"#, "SHOP_TESTS"),
        ("add_library(shop_core STATIC a.cpp)", "shop_core"),
        ("add_library(shop-core)", "shop-core"),
        ("add_executable(shop_app main.cpp)", "shop_app"),
        ("add_custom_target(docs ALL)", "docs"),
        ("add_library(Shop::core ALIAS shop_core)", "Shop::core"),
        ("add_library(Foo::foo UNKNOWN IMPORTED)", "Foo::foo"),
    ] {
        assert!(declares(line, word), "{line}: {word}");
    }
}

#[test]
fn cmake_refusals() {
    for (line, word) in [
        ("shop_add_library(shop_core a.cpp)", "shop_core"),
        ("shop_add_library(shop_core a.cpp)", "shop_add_library"),
        ("target_link_libraries(app PRIVATE shop_core)", "shop_core"),
        (
            "target_compile_options(app PRIVATE ${SHOP_WARNINGS})",
            "SHOP_WARNINGS",
        ),
        ("if(SHOP_TESTS)", "SHOP_TESTS"),
        ("add_library(Shop::core ALIAS shop_core)", "shop_core"),
        ("add_library(shop_core STATIC a.cpp)", "STATIC"),
        (
            "set_target_properties(shop_core PROPERTIES X 1)",
            "shop_core",
        ),
        ("set_property(TARGET shop_core PROPERTY X 1)", "TARGET"),
        ("list(APPEND SHOP_SOURCES a.cpp)", "SHOP_SOURCES"),
        ("unset(SHOP_SOURCES)", "SHOP_SOURCES"),
        ("set(ENV{PATH} /bin)", "PATH"),
        ("endfunction(shop_add_library)", "shop_add_library"),
        ("set(shop_warnings 1)", "SHOP_WARNINGS"),
        ("add_library(Shop_Core STATIC a.cpp)", "shop_core"),
        ("add_library(shop-core-x STATIC a.cpp)", "shop-core"),
        ("function(shop_add_library_x)", "shop_add_library"),
    ] {
        assert!(!declares(line, word), "{line}: {word}");
    }
}

#[test]
fn cmake_symbol_names() {
    let cmake = |line| one(Kind::Cmake, line);
    for (line, name) in [
        ("function(shop_add_library name)", "shop_add_library"),
        ("MACRO(Shop_Option name)", "Shop_Option"),
        ("add_library(shop_core STATIC a.cpp)", "shop_core"),
        ("add_library(shop-core)", "shop-core"),
        ("add_library(Foo::foo UNKNOWN IMPORTED)", "Foo::foo"),
        ("add_library(shop_core", "shop_core"),
        ("add_library(alias_free ALIASED.cpp)", "alias_free"),
        ("add_executable(shop.cli main.cpp)", "shop.cli"),
        ("add_custom_target(docs COMMAND doxygen)", "docs"),
    ] {
        assert_eq!(cmake(line).as_deref(), Some(name), "{line}");
    }
    for line in [
        "add_library(Shop::core ALIAS shop_core)",
        "add_library(shop_alias ALIAS shop_core)",
        "set(SHOP_WARNINGS -Wall)",
        r#"option(SHOP_TESTS "x" ON)"#,
        "shop_add_library(shop_core a.cpp)",
        "endfunction(shop_add_library)",
    ] {
        assert_eq!(cmake(line), None, "{line}");
    }
}

#[test]
fn cmake_literals_hide_declarations() {
    let text = "#[[ comment\nfunction(a)\n]]\n#[==[ level\nfunction(b) ]]\n]==]\nset(X [=[\nfunction(c)\n]=])\nmessage(\"over\nfunction(d)\n\")\nfunction(e) # 'quote\nfunction(f)\nstring(REPLACE \"\\\\\" \"/\" p ${p})\nfunction(g)\nmessage(\"a \\\" b\nfunction(h)\n\")\n";
    let hidden: Vec<usize> = literal_lines(Kind::Cmake, text)
        .iter()
        .enumerate()
        .filter(|(_, h)| **h)
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(hidden, [2, 3, 5, 6, 8, 9, 11, 12, 18, 19]);
}

#[test]
fn a_cmake_name_holds_its_dashes_dots_and_double_colons() {
    let line = "target_link_libraries(shop-core PRIVATE Shop::core $<TARGET_FILE:shop.cli>)";
    let word = |col| definition_word(Some(Kind::Cmake), line, col).map(|(_, w)| w);
    assert_eq!(word(24), Some("shop-core"));
    assert_eq!(word(41), Some("Shop::core"));
    assert_eq!(word(48), Some("Shop::core"));
    assert_eq!(word(46), Some("Shop::core"));
    assert_eq!(word(68), Some("shop.cli"));
    assert_eq!(word(56), Some("TARGET_FILE"));
}

#[test]
fn cmake_commands_name_files() {
    let files: Vec<PathBuf> = [
        "CMakeLists.txt",
        "app/CMakeLists.txt",
        "cmake/ShopHelpers.cmake",
        "cmake/warnings.cmake",
        "cmake/FindBoost.cmake",
        "deps/fmt-config.cmake",
    ]
    .map(PathBuf::from)
    .to_vec();
    let at = |line: &str, col| cmake_import(line, col);
    let open = |line: &str, here: &str| {
        let (command, arg) = cmake_import(line, line.find('(').unwrap() + 1).unwrap();
        let dir = Path::new(here).parent().unwrap();
        cmake_files(&command, &arg, dir, &files)
    };
    assert_eq!(
        at("include(ShopHelpers)", 9),
        Some(("include".into(), "ShopHelpers".into()))
    );
    assert_eq!(at("INCLUDE(ShopHelpers)", 2), None);
    assert_eq!(at("include(${X}/a.cmake)", 9), None);
    assert_eq!(
        open("include(ShopHelpers)", "CMakeLists.txt"),
        [PathBuf::from("cmake/ShopHelpers.cmake")]
    );
    assert_eq!(
        open("include(cmake/warnings.cmake)", "CMakeLists.txt"),
        [PathBuf::from("cmake/warnings.cmake")]
    );
    assert_eq!(
        open("include(../cmake/warnings.cmake)", "app/CMakeLists.txt"),
        [PathBuf::from("cmake/warnings.cmake")]
    );
    assert_eq!(
        open("add_subdirectory(app)", "CMakeLists.txt"),
        [PathBuf::from("app/CMakeLists.txt")]
    );
    assert_eq!(
        open("find_package(Boost 1.80 REQUIRED)", "CMakeLists.txt"),
        [PathBuf::from("cmake/FindBoost.cmake")]
    );
    assert_eq!(
        open("find_package(fmt)", "app/CMakeLists.txt"),
        [PathBuf::from("deps/fmt-config.cmake")]
    );
    assert!(open("include(FetchContent)", "CMakeLists.txt").is_empty());
}

#[test]
fn cmake_roots_follow_the_cmake_on_the_path() {
    let dir = std::env::temp_dir().join(format!("merl-cmake-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("cellar/cmake/4.1/bin")).unwrap();
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    std::fs::write(dir.join("cellar/cmake/4.1/bin/cmake"), "").unwrap();
    std::os::unix::fs::symlink(
        dir.join("cellar/cmake/4.1/bin/cmake"),
        dir.join("bin/cmake"),
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("cellar/cmake/4.1/share/cmake-4.1")).unwrap();
    let roots = cmake_roots(Some(dir.join("bin/cmake")));
    let share = std::fs::canonicalize(dir.join("cellar/cmake/4.1/share")).unwrap();
    assert_eq!(roots[0], share.join("cmake/Modules"));
    assert_eq!(roots[1], share.join("cmake-4.1/Modules"));
    assert!(roots.contains(&PathBuf::from("/opt/homebrew/lib/cmake")));
    assert!(roots.contains(&PathBuf::from("/usr/lib/cmake")));
    assert!(!cmake_roots(None).iter().any(|r| r.ends_with("Modules")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cmake_is_told_by_its_file_names() {
    assert_eq!(kind_of(Path::new("CMakeLists.txt")), Some(Kind::Cmake));
    assert_eq!(kind_of(Path::new("cmake/Shop.cmake")), Some(Kind::Cmake));
    assert_eq!(kind_of(Path::new("notes.txt")), None);
    assert_eq!(word_chars(Some(Kind::Cmake), false), "-.");
}
