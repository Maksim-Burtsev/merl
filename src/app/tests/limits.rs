use super::*;

#[test]
fn a_reload_carries_lines_past_one_rewritten_region_only() {
    let old: Vec<String> = (0..10).map(|i| i.to_string()).collect();
    let mut new = old.clone();
    new.insert(6, "y".into());
    new.insert(2, "x".into());
    assert_eq!(new[9], "7");
    assert_eq!(carried(&old, &new, 7), 9);
    assert_eq!(new[5], "4");
    assert_eq!(carried(&old, &new, 4), 4);
}

#[test]
fn the_files_outside_the_project_are_walked_once_per_session() {
    let (dir, mut a) = project_app("outside-once", &[("app.py", "import vendored\n")]);
    let root = external_root("outside-once", &[("vendored.py", "X = 1\n")]);
    use_roots(&mut a, Kind::Python, std::slice::from_ref(&root));
    assert_eq!(a.external_files(Kind::Python).len(), 1);
    std::fs::write(root.join("installed.py"), "Y = 1\n").unwrap();
    assert_eq!(a.external_files(Kind::Python).len(), 1);
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_hunk_left_is_its_index_so_a_hunk_written_above_it_takes_its_place() {
    let (dir, mut a) = review_app("limits-hunk-left");
    let src = dir.join("src/a.rs");
    a.jump_to(&src, 3);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(a.line_str(), "F");
    a.jump_to(&dir.join("src/keep.rs"), 2);
    std::fs::write(&src, "A\nB\nc\nD\ne\nF\n").unwrap();
    let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    a.review_refreshed(fresh);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!((at(&a), a.line_str()), ((src, 3), "D"));
    let _ = std::fs::remove_dir_all(dir);
}

fn d_in(tag: &str, files: &[(String, String)], file: &str, code: &str) -> Shown {
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let (dir, mut a) = project_app(tag, &files);
    a.no_external();
    d_on(&mut a, file, code);
    let found = shown(&mut a);
    let _ = std::fs::remove_dir_all(dir);
    found
}

fn ts_relays(modules: usize, last: &str) -> Vec<(String, String)> {
    let mut files = vec![(
        "app.ts".to_string(),
        "import { Tariff } from './r1';\n\nconst t = new Tariff();\n".to_string(),
    )];
    for i in 1..modules {
        let relay = format!("export {{ Tariff }} from './r{}';\n", i + 1);
        files.push((format!("r{i}.ts"), relay));
    }
    files.push((format!("r{modules}.ts"), last.to_string()));
    files
}

#[test]
fn a_typescript_name_is_followed_through_four_modules_that_hand_it_on() {
    let at = |n: usize| {
        let files = ts_relays(n, "export class Tariff {}\n");
        d_in(&format!("ts-relay-{n}"), &files, "app.ts", "new Tariff")
    };
    assert_eq!(at(5), jump("Tariff: via import r5.ts", "r5.ts:1"));
    assert_eq!(at(6), jump("Tariff: by name, 1 match", "r6.ts:1"));
}

#[test]
fn a_typescript_package_is_found_behind_four_barrels() {
    let at = |n: usize| {
        let files = ts_relays(n, "export { Tariff } from 'tariffs';\n");
        d_in(&format!("ts-barrel-{n}"), &files, "app.ts", "new Tariff")
    };
    let missing = "Tariff: via import tariffs (not installed)";
    assert_eq!(at(4), jump(missing, "app.ts:1"));
    assert_eq!(at(5), jump("no definition for Tariff", "app.ts:3"));
}

#[test]
fn two_typescript_modules_handing_a_name_to_each_other_end() {
    let mut files = ts_relays(2, "export { Tariff } from './r1';\n");
    let none = jump("no definition for Tariff", "app.ts:3");
    assert_eq!(d_in("ts-cycle", &files, "app.ts", "new Tariff"), none);
    files.push(("tariffs.ts".into(), "export class Tariff {}\n".into()));
    let by_name = jump("Tariff: by name, 1 match", "tariffs.ts:1");
    assert_eq!(
        d_in("ts-cycle-named", &files, "app.ts", "new Tariff"),
        by_name
    );
}

fn python_bases(levels: usize, top: &str) -> Vec<(String, String)> {
    let mut text = format!("from outside import Base\n\n\nclass C{levels}({top}):\n    pass\n");
    for i in (0..levels).rev() {
        text += &format!("\n\nclass C{i}(C{}):\n    pass\n", i + 1);
    }
    text +=
        "\n\nclass Other:\n    def save(self):\n        pass\n\n\ndef f(x: C0):\n    x.save()\n";
    vec![("app.py".to_string(), text)]
}

#[test]
fn a_python_receiver_reaches_outside_the_project_through_eight_classes() {
    let at = |n: usize| {
        d_in(
            &format!("py-up-{n}"),
            &python_bases(n, "Base"),
            "app.py",
            "x.save",
        )
    };
    assert_eq!(at(7), jump("no definition for save", "app.py:42"));
    assert_eq!(
        at(8),
        jump("save \u{2192} Other.save (by name, 1 match)", "app.py:41")
    );
}

#[test]
fn of_two_versions_of_a_crate_the_first_root_answers() {
    let (dir, mut a) = project_app(
        "rust-two-versions",
        &[
            ("Cargo.toml", "[package]\nname = \"repro\"\n"),
            (
                "src/lib.rs",
                "use dep::Thing;\n\npub fn made(t: Thing) {}\n",
            ),
        ],
    );
    let registry = external_root(
        "rust-two-versions-registry",
        &[
            ("dep-2.0.0/src/lib.rs", "pub struct Thing;\n"),
            ("dep-1.0.0/src/lib.rs", "\npub struct Thing;\n"),
        ],
    );
    a.no_external();
    for (first, other, line) in [("dep-1.0.0", "dep-2.0.0", 2), ("dep-2.0.0", "dep-1.0.0", 1)] {
        use_roots(
            &mut a,
            Kind::Rust,
            &[registry.join(first), registry.join(other)],
        );
        d_on(&mut a, "src/lib.rs", "t: Thing");
        let Shown::Jump(status, place) = shown(&mut a) else {
            panic!("a picker");
        };
        assert_eq!(status, "Thing: via import dep");
        assert!(
            place.ends_with(&format!("{first}/src/lib.rs:{line}")),
            "{place}"
        );
    }
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(registry);
}

fn chain(head: &str, links: usize) -> String {
    format!("{head}{}", ".parent".repeat(links))
}

fn chain_in(lang: &str, links: usize) -> Shown {
    let tag = format!("{lang}-chain-{links}");
    let files = |name: &str, text: String| vec![(name.to_string(), text)];
    match lang {
        "rust" => {
            let rs = format!(
                "pub struct Folder {{\n    pub parent: Folder,\n}}\n\nimpl Folder {{\n    pub fn root(&self) -> u32 {{\n        0\n    }}\n}}\n\npub struct Other;\n\nimpl Other {{\n    pub fn root(&self) -> u32 {{\n        1\n    }}\n}}\n\npub fn walk(folder: &Folder) -> u32 {{\n    {}.root()\n}}\n",
                chain("folder", links)
            );
            d_in(&tag, &files("src/lib.rs", rs), "src/lib.rs", ".root")
        }
        "csharp" => {
            let cs = format!(
                "namespace Shop;\n\npublic class Folder\n{{\n    public Folder Parent;\n\n    public int Root() => 0;\n}}\n\npublic class Other\n{{\n    public int Root() => 1;\n}}\n\npublic class Walker\n{{\n    public int Walk(Folder folder) => {}.Root();\n}}\n",
                chain("folder", links).replace(".parent", ".Parent")
            );
            d_in(&tag, &files("Shop.cs", cs), "Shop.cs", ".Root")
        }
        "php" => {
            let php = format!(
                "<?php\n\nclass Folder\n{{\n    public Folder $parent;\n\n    public function root(): int\n    {{\n        return 0;\n    }}\n}}\n\nclass Other\n{{\n    public function root(): int\n    {{\n        return 1;\n    }}\n}}\n\nfunction walk(Folder $folder): int\n{{\n    return {}->root();\n}}\n",
                chain("$folder", links).replace(".parent", "->parent")
            );
            d_in(&tag, &files("shop.php", php), "shop.php", "->root")
        }
        _ => {
            let swift = format!(
                "class Folder {{\n    var parent: Folder\n\n    func root() -> Int {{\n        return 0\n    }}\n}}\n\nclass Other {{\n    func root() -> Int {{\n        return 1\n    }}\n}}\n\nfunc walk(folder: Folder) -> Int {{\n    return {}.root()\n}}\n",
                chain("folder", links)
            );
            d_in(&tag, &files("Shop.swift", swift), "Shop.swift", ".root")
        }
    }
}

#[test]
fn a_chain_of_six_names_is_followed_and_a_seventh_breaks_it_in_rust_csharp_php_and_swift() {
    for (lang, root) in [
        ("rust", "src/lib.rs:6"),
        ("csharp", "Shop.cs:7"),
        ("php", "shop.php:7"),
        ("swift", "Shop.swift:4"),
    ] {
        assert!(
            matches!(chain_in(lang, 5), Shown::Jump(s, at) if at == root && s.contains("via")),
            "{lang}"
        );
        assert!(
            matches!(chain_in(lang, 6), Shown::Picker(s, _) if s.contains("by name, 2 declarations")),
            "{lang}"
        );
    }
}

fn ladder(lang: &str, n: usize) -> (String, String, String) {
    let mut t = String::new();
    match lang {
        "py" | "pysuper" => {
            t += &format!("class C{n}:\n    def root(self):\n        pass\n");
            for i in (0..n).rev() {
                t += &format!("\n\nclass C{i}(C{}):\n    pass\n", i + 1);
            }
            t += "\n\nclass Other:\n    def root(self):\n        pass\n";
            if lang == "py" {
                t += "\n\ndef f(x: C0):\n    x.root()\n";
            } else {
                t += "\n\nclass Top(C0):\n    def root(self):\n        super().root()\n";
            }
            (
                "app.py".into(),
                t,
                if lang == "py" {
                    "x.root".into()
                } else {
                    "super().root".into()
                },
            )
        }
        "ts" => {
            t += &format!("export class C{n} {{\n  root(): void {{}}\n}}\n");
            for i in (0..n).rev() {
                t += &format!("\nexport class C{i} extends C{} {{}}\n", i + 1);
            }
            t += "\nexport class Other {\n  root(): void {}\n}\n\nexport function f(x: C0): void {\n  x.root();\n}\n";
            ("app.ts".into(), t, "x.root".into())
        }
        "cs" => {
            t += &format!("public class C{n}\n{{\n    public void Root() {{ }}\n}}\n");
            for i in (0..n).rev() {
                t += &format!("\npublic class C{i} : C{}\n{{\n}}\n", i + 1);
            }
            t += "\npublic class Other\n{\n    public void Root() { }\n}\n\npublic class W\n{\n    public void F(C0 x)\n    {\n        x.Root();\n    }\n}\n";
            ("Shop.cs".into(), t, "x.Root".into())
        }
        "php" => {
            t += &format!(
                "<?php\n\nclass C{n}\n{{\n    public function root(): void\n    {{\n    }}\n}}\n"
            );
            for i in (0..n).rev() {
                t += &format!("\nclass C{i} extends C{}\n{{\n}}\n", i + 1);
            }
            t += "\nclass Other\n{\n    public function root(): void\n    {\n    }\n}\n\nfunction f(C0 $x): void\n{\n    $x->root();\n}\n";
            ("shop.php".into(), t, "$x->root".into())
        }
        _ => {
            t += &format!("package shop\n\ntype C{n} struct{{}}\n\nfunc (c C{n}) Root() {{}}\n");
            for i in (0..n).rev() {
                t += &format!("\ntype C{i} struct {{\n\tC{}\n}}\n", i + 1);
            }
            t += "\ntype Other struct{}\n\nfunc (o Other) Root() {}\n\nfunc F(x C0) {\n\tx.Root()\n}\n";
            ("shop.go".into(), t, "x.Root".into())
        }
    }
}

fn up_the_ladder(lang: &str, n: usize) -> Shown {
    let (file, text, code) = ladder(lang, n);
    let mut files = vec![(file.clone(), text)];
    if lang == "go" {
        files.push(("go.mod".into(), "module shop\n".into()));
    }
    d_in(&format!("ladder-{lang}-{n}"), &files, &file, &code)
}

#[test]
fn a_member_is_looked_for_nine_types_up_and_super_eight() {
    for lang in ["py", "ts", "cs", "php", "go"] {
        assert!(
            matches!(up_the_ladder(lang, 8), Shown::Jump(s, _) if s.contains("C8")),
            "{lang}"
        );
        assert!(
            !matches!(up_the_ladder(lang, 9), Shown::Jump(s, _) if s.contains("C9")),
            "{lang}"
        );
    }
    let found = up_the_ladder("pysuper", 7);
    assert_eq!(
        found,
        jump("root \u{2192} C7.root (via super of Top)", "app.py:2")
    );
    assert!(matches!(up_the_ladder("pysuper", 8), Shown::Picker(..)));
}

fn go_types(n: usize, sep: &str) -> Vec<(String, String)> {
    let mut t = String::from(
        "package shop\n\ntype Real struct {\n\tName string\n}\n\nfunc (r Real) Root() {}\n",
    );
    t += &format!("\ntype A{n}{sep}Real\n");
    for i in (0..n).rev() {
        t += &format!("\ntype A{i}{sep}A{}\n", i + 1);
    }
    t += "\ntype Other struct {\n\tName string\n}\n\nfunc (o Other) Root() {}\n\nfunc F(x A0) {\n\tx.Root()\n\t_ = x.Name\n\t_ = A0{Name: \"\"}\n}\n";
    vec![
        ("go.mod".into(), "module shop\n".into()),
        ("shop.go".into(), t),
    ]
}

#[test]
fn go_reads_eight_aliases_and_eight_defined_types_down_to_a_struct() {
    let alias = |n: usize| {
        d_in(
            &format!("go-alias-{n}"),
            &go_types(n, " = "),
            "shop.go",
            "x.Root",
        )
    };
    let root = jump("Root \u{2192} Real.Root (via x: Real)", "shop.go:7");
    assert_eq!(alias(7), root);
    assert!(matches!(alias(8), Shown::Picker(..)));
    let defined = |n: usize| {
        d_in(
            &format!("go-defined-{n}"),
            &go_types(n, " "),
            "shop.go",
            "A0{Name",
        )
    };
    let name = jump("Name \u{2192} Real.Name (via A0{\u{2026}})", "shop.go:4");
    assert_eq!(defined(7), name);
    assert_eq!(defined(8), jump("no definition for Name", "shop.go:36"));
}

fn py_subtypes(n: usize) -> Vec<(String, String)> {
    let mut t = String::from(
        "class Job:\n    def run(self):\n        pass\n\n\nclass S1(Job):\n    def run(self):\n        pass\n",
    );
    for i in 2..=n {
        t += &format!(
            "\n\nclass S{i}(S{}):\n    def run(self):\n        pass\n",
            i - 1
        );
    }
    t += "\n\ndef f(x: Job):\n    x.run()\n";
    vec![("app.py".into(), t)]
}

#[test]
fn implementations_are_looked_for_four_subtypes_down() {
    let at = |n: usize| {
        d_in(
            &format!("py-sub-{n}"),
            &py_subtypes(n),
            "app.py",
            "^    def run",
        )
    };
    let count = |found: Shown| match found {
        Shown::Picker(_, rows) => rows.len(),
        Shown::Jump(..) => 1,
    };
    assert_eq!(count(at(4)), 4);
    assert_eq!(count(at(5)), 4);
}

fn ts_barrel_subtype(barrels: usize) -> Vec<(String, String)> {
    let mut files = vec![
        ("base.ts".to_string(), "export class Base {\n  run(): void {}\n}\n".to_string()),
        (
            "sub.ts".to_string(),
            "import { Base } from './b1';\n\nexport class Sub extends Base {\n  run(): void {}\n}\n".to_string(),
        ),
    ];
    for i in 1..=barrels {
        files.push((
            format!("b{i}.ts"),
            format!("export * from './b{}';\n", i + 1),
        ));
    }
    files.push((
        format!("b{}.ts", barrels + 1),
        "export class Base {}\n".to_string(),
    ));
    files
}

#[test]
fn implementations_stop_at_four_barrels() {
    let at = |n: usize| {
        d_in(
            &format!("ts-bar-{n}"),
            &ts_barrel_subtype(n),
            "base.ts",
            "  run",
        )
    };
    let other = vec![("Sub.run".into(), "by name".into(), "sub.ts:4".into())];
    assert_eq!(
        at(4),
        Shown::Picker("run: at a declaration, 1 other by name".into(), other)
    );
    let wrong = jump(
        "run \u{2192} Sub.run (implementations of Base.run)",
        "sub.ts:4",
    );
    assert_eq!(at(5), wrong, "#794");
}

#[test]
fn super_passes_over_the_bases_that_declare_nothing_worth_a_jump() {
    for plain in ["Generic[T]", "typing.Protocol", "ABC", "object"] {
        let text = format!(
            "import typing\nfrom abc import ABC\nfrom typing import Generic, TypeVar\n\nT = TypeVar(\"T\")\n\n\nclass Base:\n    def root(self):\n        pass\n\n\nclass Other:\n    def root(self):\n        pass\n\n\nclass Top({plain}, Base):\n    def root(self):\n        super().root()\n"
        );
        let found = d_in(
            "py-plain",
            &[("app.py".into(), text)],
            "app.py",
            "super().root",
        );
        assert_eq!(
            found,
            jump("root \u{2192} Base.root (via super of Top)", "app.py:9"),
            "{plain}"
        );
    }
}

#[test]
fn a_php_assignment_is_read_over_twenty_lines() {
    let at = |blank: usize| {
        let php = format!(
            "<?php\n\nclass Folder\n{{\n    public function root(): int\n    {{\n        return 0;\n    }}\n}}\n\nclass Other\n{{\n    public function root(): int\n    {{\n        return 1;\n    }}\n}}\n\nfunction walk(): int\n{{\n    $folder =\n{}        new Folder();\n    return $folder->root();\n}}\n",
            "\n".repeat(blank)
        );
        d_in(
            &format!("php-stmt-{blank}"),
            &[("shop.php".into(), php)],
            "shop.php",
            "->root",
        )
    };
    let typed = jump(
        "root \u{2192} Folder::root (via $folder: Folder)",
        "shop.php:5",
    );
    assert_eq!(at(18), typed);
    assert!(matches!(at(19), Shown::Picker(..)));
}

#[test]
fn what_a_php_class_outside_the_project_inherits_is_not_read() {
    let (dir, mut a) = project_app(
        "php-vendor",
        &[
            (
                "app/Song.php",
                "<?php\n\nnamespace App;\n\nuse Lib\\Model;\n\nclass Song extends Model\n{\n}\n\nfunction keep(Song $song): void\n{\n    $song->save();\n    $song->fill();\n}\n",
            ),
            (
                "app/Other.php",
                "<?php\n\nclass Other\n{\n    public function save(): void\n    {\n    }\n}\n",
            ),
        ],
    );
    let vendor = external_root(
        "php-vendor-lib",
        &[
            (
                "lib/Model.php",
                "<?php\n\nnamespace Lib;\n\nclass Model extends Base\n{\n    public function fill(): void\n    {\n    }\n}\n",
            ),
            (
                "lib/Base.php",
                "<?php\n\nnamespace Lib;\n\nclass Base\n{\n    public function save(): void\n    {\n    }\n}\n",
            ),
        ],
    );
    a.no_external();
    use_roots(&mut a, Kind::Php, std::slice::from_ref(&vendor));
    d_on(&mut a, "app/Song.php", "$song->fill");
    let model = vendor.join("lib/Model.php:7").display().to_string();
    assert_eq!(
        shown(&mut a),
        jump("fill \u{2192} Model::fill (via $song: Song)", &model)
    );
    d_on(&mut a, "app/Song.php", "$song->save");
    assert!(
        matches!(shown(&mut a), Shown::Picker(s, rows) if s == "save: by name, 2 declarations" && rows.len() == 2)
    );
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(vendor);
}

#[test]
fn an_angled_include_reads_every_directory_that_has_the_header() {
    let (dir, mut a) = project_app(
        "c-include-next",
        &[(
            "main.c",
            "#include <stdio.h>\n\nint p(void) { return printf(\"x\"); }\n",
        )],
    );
    let wrapper = external_root(
        "c-include-next-cxx",
        &[("stdio.h", "#include_next <stdio.h>\n")],
    );
    let libc = external_root(
        "c-include-next-libc",
        &[
            ("stdio.h", "int printf(const char *, ...);\n"),
            ("libintl.h", "int printf(const char *, ...);\n"),
        ],
    );
    use_roots(&mut a, Kind::C, &[wrapper.clone(), libc.clone()]);
    d_on(&mut a, "main.c", "return printf");
    let at = format!("{}:1", libc.join("stdio.h").display());
    assert_eq!(shown(&mut a), jump("printf: by name, 1 match", &at));
    for d in [dir, wrapper, libc] {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
fn a_rust_name_its_module_does_not_declare_is_left_to_the_search_by_name() {
    let files = [
        ("Cargo.toml", "[package]\nname = \"shop\"\n"),
        (
            "src/lib.rs",
            "mod a;\nmod b;\nuse crate::a::Thing;\n\npub fn f(t: Thing) {}\n",
        ),
        ("src/a.rs", "pub struct Other;\n"),
        ("src/b.rs", "pub struct Thing;\n"),
        ("other/Cargo.toml", "[package]\nname = \"other\"\n"),
        ("other/src/lib.rs", "pub struct Thing;\n"),
    ]
    .map(|(n, t)| (n.to_string(), t.to_string()));
    let found = d_in("rust-not-in-module", &files, "src/lib.rs", "t: Thing");
    let rows = [("Thing", "other/src/lib.rs:1"), ("Thing", "src/b.rs:1")];
    assert_eq!(found, picker("Thing: by name, 2 declarations", &rows));
}
