//! `d`: where a definition is looked for, and what is out of its scope.

use super::*;

#[test]
fn infra_definitions_stay_in_their_scope() {
    let (dir, mut a) = project_app(
        "scope",
        &[
            (
                "app/main.tf",
                "resource \"aws_s3_bucket\" \"logs\" {\n  bucket = var.region\n}\n",
            ),
            ("app/variables.tf", "variable \"region\" {}\n"),
            ("vpc/variables.tf", "variable \"region\" {}\n"),
            (
                "compose.yml",
                "services:\n  web:\n    depends_on: [db-main]\n  db-main:\n    image: postgres\n",
            ),
            ("other.yml", "db-main:\n  x: 1\n"),
        ],
    );
    // On `region` in `var.region`: the variable of this module, not the other one.
    a.jump_to(&dir.join("app/main.tf"), 2);
    a.col = 16;
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("app/variables.tf"), 0), "{}", a.message);
    // On `db-main`, dash and all: the service in this file only.
    a.jump_to(&dir.join("compose.yml"), 3);
    a.col = 20;
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("compose.yml"), 3), "{}", a.message);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_field_has_no_definition_and_locals_must_be_direct() {
    let (dir, mut a) = project_app(
        "fallback",
        &[
            (
                "order.rs",
                "pub struct Order {\n    items: Vec<u8>,\n}\nfn add(order: &mut Order) {\n    order.items.push(1);\n}\n",
            ),
            (
                "main.tf",
                "locals {\n  name = \"x\"\n  tags = {\n    name = \"y\"\n  }\n}\nresource \"a\" \"b\" {\n  bucket = local.name\n}\n",
            ),
        ],
    );
    // A field is no declaration the Rust rules know, and `u` is the key for its uses.
    a.jump_to(&dir.join("order.rs"), 5);
    a.col = 10;
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("order.rs"), 4));
    assert_eq!(a.message, "no definition for items");
    // Of the two `name =` lines, only the one directly inside `locals` is `local.name`.
    a.jump_to(&dir.join("main.tf"), 8);
    a.col = 17;
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("main.tf"), 1), "{}", a.message);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `json.load` is not declared in the project: `d` follows it into the standard library of
/// the `python3` on this machine and opens it read-only. Skipped where there is none.
#[test]
fn definitions_outside_the_project_come_from_the_standard_library() {
    let stdlib = search::external_roots(Kind::Python, Path::new("/"));
    if stdlib.is_empty() {
        eprintln!("no python3, skipped");
        return;
    }
    let (dir, mut a) = project_app(
        "stdlib",
        &[(
            "main.py",
            "import json\n\ndef read(p):\n    return json.load(open(p))\n",
        )],
    );
    a.jump_to(&dir.join("main.py"), 4);
    a.col = 17;
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    let (path, _) = at(&a);
    assert!(
        path.ends_with("json/__init__.py"),
        "{} {}",
        path.display(),
        a.message
    );
    assert!(a.line_str().starts_with("def load("), "{}", a.line_str());
    assert_eq!(a.buf.readonly, Some("outside the project"));
    assert_eq!(a.message, "load: via import json");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Roots that come back empty are looked for again on the next `d`: a toolchain that failed to
/// answer once does not leave the rest of the session without them (#183).
#[test]
fn empty_external_roots_are_asked_again() {
    let (dir, mut a) = project_app("roots-again", &[("index.php", "<?php\n")]);
    assert!(a.external_files(Kind::Php).is_empty());
    std::fs::create_dir_all(dir.join("vendor/acme")).unwrap();
    std::fs::write(dir.join("vendor/acme/Client.php"), "<?php\n").unwrap();
    assert_eq!(
        *a.external_files(Kind::Php),
        [dir.join("vendor/acme/Client.php")]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Found by the acceptance pass of #68: three ways `d` claimed more than it knew.
#[test]
fn a_local_name_is_not_an_import_and_a_member_is_not_a_module_level_name() {
    let (dir, mut a) = project_app(
        "acceptance",
        &[
            ("pkg/__init__.py", ""),
            ("pkg/b.py", "def helper():\n    pass\n"),
            // A parameter called like an imported module, a local called like an import.
            (
                "shadow.py",
                "import json\nfrom pkg.b import helper\n\n\ndef handler(json):\n    return json.loads(1)\n\n\ndef f():\n    helper = 3\n    return helper\n",
            ),
            // Module-level names are no members of a value.
            (
                "member.py",
                "zqwidget = 5\n\n\ndef zqmake(x):\n    return x\n\n\ndef g(client):\n    client.zqwidget\n    return client.zqmake(1)\n",
            ),
            // The package's own `pick` is native; a method of that name is not it.
            (
                "outside.py",
                "import fakelib\nfrom fakelib import pick\nfrom fakelib.core import make\n\nlimit = 3\n\n\ndef run():\n    make()\n    fakelib.pick(1)\n    make(\n        limit=1,\n    )\n    return pick(1)\n",
            ),
        ],
    );
    let root = external_root(
        "acceptance",
        &[
            ("json/__init__.py", "def loads(s):\n    pass\n"),
            ("fakelib/__init__.py", "from fakelib._native import pick\n"),
            (
                "fakelib/core.py",
                "class Thing:\n    def pick(self, x):\n        return x\n\n\ndef make():\n    pass\n",
            ),
        ],
    );
    use_roots(&mut a, Kind::Python, std::slice::from_ref(&root));
    d_on(&mut a, "shadow.py", "json.loads");
    assert!(!a.message.contains("via import"), "{}", a.message);
    d_on(&mut a, "shadow.py", "return helper");
    assert!(!a.message.contains("via import"), "{}", a.message);
    assert_eq!(a.rel_path(), "shadow.py");
    for code in ["client.zqwidget", "client.zqmake"] {
        d_on(&mut a, "member.py", code);
        assert!(
            a.message.starts_with("no definition for zq"),
            "{}",
            a.message
        );
    }
    d_on(&mut a, "outside.py", "return pick");
    assert_eq!(a.message, "no definition for pick");
    assert_eq!(a.rel_path(), "outside.py");
    // Behind the module's name as well: `fakelib.pick` is no method of a class in it.
    d_on(&mut a, "outside.py", "fakelib.pick");
    assert_eq!(a.message, "no definition for pick");
    // A keyword argument names a parameter: the one variable spelled so is offered.
    d_on(&mut a, "outside.py", "    limit");
    assert!(a.picker.is_some(), "{}", a.message);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    // What the module does declare at its top is still found through the import.
    d_on(&mut a, "outside.py", "    make");
    assert_eq!(a.message, "make: via import fakelib.core");
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// Found by the acceptance pass of #68: a line inside a raw string, a docstring or a block
/// comment declares nothing, however much it reads like a declaration.
#[test]
fn a_declaration_inside_a_literal_is_not_one() {
    let (dir, mut a) = project_app(
        "literals",
        &[
            ("go.mod", "module lit\n"),
            (
                "misc.go",
                "package lit\n\ntype Target struct{}\n\nconst snippet = `\nfunc (t Target) InString() string { return \"\" }\n`\n\n/*\nfunc (t Target) InBlock() string { return \"\" }\n*/\n",
            ),
            (
                "use.go",
                "package lit\n\nfunc Use(t Target) {\n\t_ = t.InString()\n\t_ = t.InBlock()\n}\n",
            ),
            (
                "a.py",
                "SRC = \"\"\"\nclass Fake:\n    def ghost(self):\n        pass\n\"\"\"\n\n\nclass Real:\n    def ghost(self):\n        pass\n\n\ndef call(g):\n    return g.ghost()\n",
            ),
        ],
    );
    for kind in [Kind::Python, Kind::Go] {
        a.external.insert(kind, (Vec::new(), Arc::new(Vec::new())));
    }
    d_on(&mut a, "use.go", "t.InString");
    assert_eq!(a.message, "no definition for InString");
    d_on(&mut a, "use.go", "t.InBlock");
    assert_eq!(a.message, "no definition for InBlock");
    d_on(&mut a, "a.py", "g.ghost");
    assert_eq!(
        shown(&mut a),
        jump("ghost \u{2192} Real.ghost (by name, 1 match)", "a.py:9")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Found by the acceptance pass of #68: `Outer.find` names what `Outer` declares, and a
/// method `find` of some class elsewhere used to take the jump with `1 match`.
#[test]
fn a_namespace_or_a_class_in_front_of_the_word_names_where_it_is_declared() {
    let (dir, mut a) = project_app(
        "namespaces",
        &[
            (
                "ns.ts",
                "export namespace Outer {\n  export function find(id: string) {\n    return id;\n  }\n}\n\nexport function run() {\n  return Outer.find(\"1\");\n}\n",
            ),
            (
                "other.ts",
                "export class Repo {\n  find(id: string) {\n    return id;\n  }\n}\n",
            ),
        ],
    );
    a.external
        .insert(Kind::TsJs, (Vec::new(), Arc::new(Vec::new())));
    d_on(&mut a, "ns.ts", "Outer.find");
    assert_eq!(
        shown(&mut a),
        jump("find \u{2192} Outer.find (via Outer)", "ns.ts:2")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #129. The chain in front of the word is joined as the kind qualifies a name, `Depot::open`
/// in Rust and C++, so a declaration that reads so is the answer there as `Outer.find` is
/// elsewhere. A type nothing declares the word in, `Self`, and a value's method stay by name.
#[test]
fn a_path_in_front_of_the_word_is_joined_as_the_kind_qualifies() {
    let (dir, mut a) = project_app(
        "paths",
        &[
            (
                "depot.rs",
                "pub struct Depot;\n\nimpl Depot {\n    pub fn open() -> Self {\n        Depot\n    }\n\n    pub fn again() -> Self {\n        Self::open()\n    }\n}\n\npub struct Shed;\n\nimpl Shed {\n    pub fn open() -> Self {\n        Shed\n    }\n}\n\npub struct Bare;\n\npub fn run(shed: Shed) {\n    let _ = Depot::open();\n    let _ = depot::Shed::open();\n    let _ = Bare::open();\n    let _ = shed.open();\n    let _ = vendored::Shed::open();\n    let _ = crate::depot::Shed::open();\n    let _ = store::Shelf::stock();\n    let _ = <Shed>::open();\n}\n",
            ),
            (
                "store/shelf.rs",
                "pub struct Shelf;\n\nimpl Shelf {\n    pub fn stock() -> Self {\n        Shelf\n    }\n}\n",
            ),
            ("cli.rs", "#[derive(Parser)]\npub struct Config;\n"),
            (
                "settings.rs",
                "pub struct Config;\n\nimpl Config {\n    pub fn parse() -> Self {\n        Config\n    }\n}\n",
            ),
            ("io.rs", "pub fn read() {}\n"),
            (
                "app/Yard.php",
                "<?php\nnamespace App;\n\nclass Yard\n{\n    public static function open(): void\n    {\n    }\n}\n\nfunction near(): void\n{\n    Yard::open();\n}\n",
            ),
            (
                "app/Far.php",
                "<?php\nnamespace Other;\n\nfunction far(): void\n{\n    \\Vendor\\Pkg\\Yard::open();\n}\n",
            ),
            (
                "app/Grouped.php",
                "<?php\nnamespace Other;\n\nuse Vendor\\Pkg\\{Yard, Shed};\n\nfunction grouped(): void\n{\n    Yard::open( );\n}\n",
            ),
            (
                "glob.rs",
                "use std::io::*;\n\nfn glob() {\n    let _ = Error::new();\n}\n",
            ),
            (
                "using.cpp",
                "using std::filesystem::Depot;\n\nvoid far() {\n    Depot::open();\n}\n",
            ),
            (
                "error.rs",
                "pub struct Error;\n\nimpl Error {\n    pub fn new() -> Self {\n        Error\n    }\n}\n",
            ),
            (
                "main.rs",
                "use crate::cli::Config;\nuse crate::depot::Depot;\nuse std::io;\n\nfn main() {\n    let _ = Depot::open();\n    let _ = Config::parse();\n    let _ = io::Error::new();\n}\n",
            ),
            (
                "depot.cpp",
                "struct Depot {\n    static Depot open() {\n        return Depot{};\n    }\n};\n\nstruct Shed {\n    static Shed open() {\n        return Shed{};\n    }\n};\n\nstruct Bare {};\n\nstruct crate {\n    void open() {}\n};\n\nstruct lid {\n    void open() {}\n};\n\nvoid use(lid crate) {\n    crate.open();\n}\n\nint main() {\n    Depot::open();\n    Bare::open();\n    return 0;\n}\n",
            ),
        ],
    );
    for kind in [Kind::Rust, Kind::C] {
        a.external.insert(kind, (Vec::new(), Arc::new(Vec::new())));
    }
    let rs = [("Depot::open", "depot.rs:4"), ("Shed::open", "depot.rs:16")];
    let cpp = [
        ("Depot::open", "depot.cpp:2"),
        ("Shed::open", "depot.cpp:8"),
        ("crate::open", "depot.cpp:16"),
        ("lid::open", "depot.cpp:20"),
    ];
    let cases = [
        (
            "depot.rs",
            "= Depot::open",
            jump("open \u{2192} Depot::open (via Depot)", "depot.rs:4"),
        ),
        (
            "depot.rs",
            "depot::Shed::open",
            jump("open \u{2192} Shed::open (via depot::Shed)", "depot.rs:16"),
        ),
        (
            "depot.rs",
            "crate::depot::Shed::open",
            jump(
                "open \u{2192} Shed::open (via crate::depot::Shed)",
                "depot.rs:16",
            ),
        ),
        // A directory of the project.
        (
            "depot.rs",
            "store::Shelf::stock",
            jump(
                "stock \u{2192} Shelf::stock (via store::Shelf)",
                "store/shelf.rs:4",
            ),
        ),
        // A `use` of the project's own type is no reason to doubt it.
        (
            "main.rs",
            "= Depot::open",
            jump("open \u{2192} Depot::open (via Depot)", "depot.rs:4"),
        ),
        // Two types called `Config`, and the derive of the imported one supplies `parse`:
        // the name of a type is no proof of which one.
        (
            "main.rs",
            "Config::parse",
            jump(
                "parse \u{2192} Config::parse (by name, 1 match)",
                "settings.rs:4",
            ),
        ),
        // `io` is `std::io` by the file's `use`, whatever `io.rs` the project has.
        (
            "main.rs",
            "io::Error::new",
            jump("new \u{2192} Error::new (by name, 1 match)", "error.rs:4"),
        ),
        // A glob may bring in an `Error` of its own; so may a C++ `using`.
        (
            "glob.rs",
            "Error::new",
            jump("new \u{2192} Error::new (by name, 1 match)", "error.rs:4"),
        ),
        (
            "using.cpp",
            "    Depot::open",
            picker("open: by name, 4 declarations", &cpp),
        ),
        // PHP: the class of the project, one spelled out in another namespace, one a
        // grouped `use` brings in.
        (
            "app/Yard.php",
            "    Yard::open",
            jump("open \u{2192} Yard::open (via Yard)", "app/Yard.php:6"),
        ),
        (
            "app/Far.php",
            "Pkg\\Yard::open",
            jump(
                "open \u{2192} Yard::open (by name, 1 match)",
                "app/Yard.php:6",
            ),
        ),
        (
            "app/Grouped.php",
            "Yard::open|( )",
            jump(
                "open \u{2192} Yard::open (by name, 1 match)",
                "app/Yard.php:6",
            ),
        ),
        // No name in front of the `::`.
        (
            "depot.rs",
            "<Shed>::open",
            picker("open: by name, 2 declarations", &rs),
        ),
        // No file or directory of the project is called `vendored`.
        (
            "depot.rs",
            "vendored::Shed::open",
            picker("open: by name, 2 declarations", &rs),
        ),
        (
            "depot.rs",
            "Bare::open",
            picker("open: by name, 2 declarations", &rs),
        ),
        (
            "depot.rs",
            "Self::open",
            picker("open: by name, 2 declarations", &rs),
        ),
        (
            "depot.rs",
            "shed.open",
            picker("open: by name, 2 declarations", &rs),
        ),
        (
            "depot.cpp",
            "    Depot::open",
            jump("open \u{2192} Depot::open (via Depot)", "depot.cpp:2"),
        ),
        (
            "depot.cpp",
            "Bare::open",
            picker("open: by name, 4 declarations", &cpp),
        ),
        // A value called like a type, behind a `.`: a `lid`, whatever `crate::open` reads.
        (
            "depot.cpp",
            "crate.open",
            picker("open: by name, 4 declarations", &cpp),
        ),
    ];
    for (file, code, want) in cases {
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{file}: {code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #227. A Rust type the project declares twice is proven by the file of the module a
/// `use crate::…` or `use super::…` at the top of the file takes it from. A binary's crate, a
/// second `use` of the name, a type of the name in the file itself, a module that only hands the type on and a module with
/// no file of its own stay by name.
#[test]
fn a_use_of_the_crate_says_which_of_two_types_it_is() {
    let cache = |name: &str| {
        format!(
            "pub struct Cache;\n\nimpl Cache {{\n    pub fn new() -> Self {{\n        Cache\n    }}\n}}\n// {name}\n"
        )
    };
    let (store, cached) = (cache("store"), cache("net"));
    let (dir, mut a) = project_app(
        "use-crate",
        &[
            ("Cargo.toml", "[package]\nname = \"depot\"\n"),
            (
                "src/main.rs",
                "mod app;\nmod net;\nmod store;\nmod shelf;\n\nuse crate::store::Cache;\n\nfn main() {\n    let _ = Cache::new();\n}\n",
            ),
            ("src/store.rs", &store),
            ("src/net/mod.rs", "pub mod cache;\npub mod client;\n"),
            ("src/net/cache.rs", &cached),
            (
                "src/net/client.rs",
                "use super::cache::Cache;\n\nfn open() {\n    let _ = Cache::new();\n}\n",
            ),
            (
                "src/app/mod.rs",
                "use super::store::{self, Cache};\n\nfn open() {\n    let _ = Cache::new();\n}\n",
            ),
            (
                "src/app/grouped.rs",
                "use crate::{\n    net::cache::Cache,\n    store,\n};\n\nfn open() {\n    let _ = Cache::new();\n}\n",
            ),
            (
                "src/bin/tool.rs",
                "use crate::store::Cache;\n\nfn main() {\n    let _ = Cache::new();\n}\n",
            ),
            (
                "src/twice.rs",
                "use crate::store::Cache;\n\nfn open() {\n    let _ = Cache::new();\n}\n\nfn other() {\n    use crate::net::cache::Cache;\n}\n",
            ),
            (
                "src/own.rs",
                "use crate::store::Cache;\n\nmod inner {\n    pub struct Cache;\n}\n\nfn open() {\n    let _ = Cache::new();\n}\n",
            ),
            ("src/shelf.rs", "pub use crate::store::Cache;\n"),
            (
                "src/rack.rs",
                "pub use crate::store::Cache;\n\n#[cfg(test)]\nmod tests {\n    struct Cache;\n\n    impl Cache {\n        fn new() -> Self {\n            Cache\n        }\n    }\n}\n",
            ),
            (
                "src/counter.rs",
                "pub use crate::store::Cache;\n\nimpl Cache {\n    pub fn fresh() -> Self {\n        Cache\n    }\n}\n",
            ),
            (
                "src/till.rs",
                "use crate::counter::Cache;\n\nfn open() {\n    let _ = Cache::fresh();\n}\n",
            ),
            (
                "src/stall.rs",
                "use crate::rack::Cache;\n\nfn open() {\n    let _ = Cache::new();\n}\n",
            ),
            (
                "src/aisle.rs",
                "use crate::shelf::Cache;\n\nfn open() {\n    let _ = Cache::new();\n}\n",
            ),
            (
                "src/far.rs",
                "use crate::depot::Cache;\n\nfn open() {\n    let _ = Cache::new();\n}\n",
            ),
        ],
    );
    a.external
        .insert(Kind::Rust, (Vec::new(), Arc::new(Vec::new())));
    let three = [
        ("Cache::new", "src/net/cache.rs:4"),
        ("tests::Cache::new", "src/rack.rs:8"),
        ("Cache::new", "src/store.rs:4"),
    ];
    let cases = [
        (
            "src/main.rs",
            "Cache::new",
            jump(
                "new \u{2192} Cache::new (via import src/store.rs)",
                "src/store.rs:4",
            ),
        ),
        (
            "src/net/client.rs",
            "Cache::new",
            jump(
                "new \u{2192} Cache::new (via import src/net/cache.rs)",
                "src/net/cache.rs:4",
            ),
        ),
        (
            "src/app/mod.rs",
            "Cache::new",
            jump(
                "new \u{2192} Cache::new (via import src/store.rs)",
                "src/store.rs:4",
            ),
        ),
        (
            "src/app/grouped.rs",
            "Cache::new",
            jump(
                "new \u{2192} Cache::new (via import src/net/cache.rs)",
                "src/net/cache.rs:4",
            ),
        ),
        (
            "src/bin/tool.rs",
            "Cache::new",
            picker("new: by name, 3 declarations", &three),
        ),
        (
            "src/twice.rs",
            "Cache::new",
            picker("new: by name, 3 declarations", &three),
        ),
        (
            "src/own.rs",
            "Cache::new",
            picker("new: by name, 3 declarations", &three),
        ),
        (
            "src/aisle.rs",
            "Cache::new",
            picker("new: by name, 3 declarations", &three),
        ),
        // A module that hands `Cache` on and implements it.
        (
            "src/till.rs",
            "Cache::fresh",
            jump(
                "fresh \u{2192} Cache::fresh (via import src/counter.rs)",
                "src/counter.rs:4",
            ),
        ),
        // `rack.rs` hands `Cache` on and declares another one only in its tests.
        (
            "src/stall.rs",
            "Cache::new",
            picker("new: by name, 3 declarations", &three),
        ),
        (
            "src/far.rs",
            "Cache::new",
            picker("new: by name, 3 declarations", &three),
        ),
    ];
    for (file, code, want) in cases {
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{file}: {code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Found by the acceptance pass of #68: a Go method of the same name and as many parameters,
/// of other types, was the one implementation `d` jumped to.
#[test]
fn a_go_method_of_other_types_implements_nothing() {
    let (dir, mut a) = project_app(
        "arity",
        &[
            ("go.mod", "module arity\n"),
            (
                "iface.go",
                "package arity\n\ntype Solo interface {\n\tFerry(a string) error\n}\n\ntype NotSolo struct{}\n\nfunc (n NotSolo) Ferry(a int) string { return \"\" }\n\ntype Real struct{}\n\nfunc (r *Real) Ferry(name string) error { return nil }\n",
            ),
            (
                "duo.go",
                "package arity\n\ntype Duo interface {\n\tCarry(a string) error\n}\n\nfunc (n NotSolo) Carry(a int) string { return \"\" }\n\nfunc (r *Real) Carry(a []byte) error { return nil }\n",
            ),
        ],
    );
    a.external
        .insert(Kind::Go, (Vec::new(), Arc::new(Vec::new())));
    d_on(&mut a, "iface.go", "\tFerry");
    assert_eq!(
        shown(&mut a),
        jump(
            "Ferry \u{2192} Real.Ferry (implementations of Solo.Ferry)",
            "iface.go:13"
        )
    );
    // With no implementation at all, the methods of other types are namesakes to offer.
    d_on(&mut a, "duo.go", "\tCarry");
    let Shown::Picker(status, rows) = shown(&mut a) else {
        panic!("a jump: {}", a.message);
    };
    assert_eq!(status, "Carry: at a declaration, 2 others by name");
    assert_eq!(rows.len(), 2);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Found by the acceptance pass of #68: two interfaces of one name in two files, and the
/// class that implements the other one was the implementation `d` jumped to.
#[test]
fn an_implementation_of_a_namesake_interface_is_not_one_of_ours() {
    let iface = "export interface Notifier {\n  send(to: string): void;\n}\n";
    let class = |name: &str, from: &str| {
        format!(
            "import {{ Notifier }} from \"./{from}\";\n\nexport class {name} implements Notifier {{\n  send(to: string): void {{}}\n}}\n"
        )
    };
    let looped = class("LoopedNotifier", "loop_a");
    let own = class("OwnNotifier", "own");
    let (mail, sms, push, deep, loose) = (
        class("MailNotifier", "a"),
        class("SmsNotifier", "index"),
        // `type Notifier,` on a line of a wrapped list declares no alias, in the class's
        // file and in the barrel alike.
        class("PushNotifier", "named").replace("{ Notifier }", "{\n  type Notifier,\n}"),
        class("DeepNotifier", "deep"),
        class("LooseNotifier", "renamed"),
    );
    let (dir, mut a) = project_app(
        "dupiface",
        &[
            ("a.ts", iface),
            ("b.ts", iface),
            // Barrels (#100): everything of `a`, the name out of `b`, a barrel of a barrel,
            // and one that hands on another interface under this name, which is not followed.
            ("index.ts", "export * from \"./a\";\n"),
            ("named.ts", "export {\n  type Notifier,\n} from \"./b\";\n"),
            ("deep.ts", "export * from \"./named\";\n"),
            (
                "things.ts",
                "export interface Thing {\n  send(to: string): void;\n}\ninterface Notifier {}\n",
            ),
            (
                "renamed.ts",
                "export { Thing as Notifier } from \"./things\";\n",
            ),
            // Two barrels that export each other lead nowhere, and end.
            ("loop_a.ts", "export * from \"./loop_b\";\n"),
            ("loop_b.ts", "export * from \"./loop_a\";\n"),
            ("looped.ts", &looped),
            // A barrel that declares a `Notifier` of its own, whatever else it hands on.
            (
                "own.ts",
                "export interface Notifier {\n  send(to: string): void;\n}\nexport * from \"./loop_a\";\n",
            ),
            ("own_impl.ts", &own),
            ("impl.ts", &mail),
            ("barrel.ts", &sms),
            ("named_impl.ts", &push),
            ("deep_impl.ts", &deep),
            ("loose.ts", &loose),
        ],
    );
    a.external
        .insert(Kind::TsJs, (Vec::new(), Arc::new(Vec::new())));
    for (file, want) in [
        (
            "a.ts",
            [
                ("SmsNotifier.send", "barrel.ts:4"),
                ("MailNotifier.send", "impl.ts:4"),
                ("LoopedNotifier.send", "looped.ts:4"),
                ("LooseNotifier.send", "loose.ts:4"),
            ],
        ),
        (
            "b.ts",
            [
                ("DeepNotifier.send", "deep_impl.ts:4"),
                ("LoopedNotifier.send", "looped.ts:4"),
                ("LooseNotifier.send", "loose.ts:4"),
                ("PushNotifier.send", "named_impl.ts:6"),
            ],
        ),
    ] {
        d_on(&mut a, file, "  send");
        let Shown::Picker(status, rows) = shown(&mut a) else {
            panic!("{}", a.message);
        };
        assert_eq!(
            status,
            "send: implementations of Notifier.send, 4 declarations"
        );
        let rows: Vec<(&str, &str)> = rows
            .iter()
            .map(|(n, _, at)| (n.as_str(), at.as_str()))
            .collect();
        assert_eq!(rows, want, "{file}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Found by the acceptance pass of #68 in hono: a member written as a property was missing
/// from the implementations, and the one left was jumped to as if it were the only one.
#[test]
fn a_property_holding_the_function_implements_the_method() {
    let (dir, mut a) = project_app(
        "propmember",
        &[
            (
                "iface.ts",
                "export interface Router {\n  match(method: string): string;\n}\n",
            ),
            (
                "impl.ts",
                "import type { Router } from \"./iface\";\n\ndeclare const match: (method: string) => string;\n\nexport class RegExpRouter implements Router {\n  match: typeof match = match;\n}\n\nexport class TrieRouter implements Router {\n  match(method: string): string {\n    const o = {\n      match: 1\n    };\n    return method;\n  }\n}\n",
            ),
        ],
    );
    a.external
        .insert(Kind::TsJs, (Vec::new(), Arc::new(Vec::new())));
    d_on(&mut a, "iface.ts", "  match");
    let Shown::Picker(status, rows) = shown(&mut a) else {
        panic!("{}", a.message);
    };
    assert_eq!(
        status,
        "match: implementations of Router.match, 2 declarations"
    );
    let places: Vec<&str> = rows.iter().map(|r| r.2.as_str()).collect();
    assert_eq!(places, ["impl.ts:6", "impl.ts:10"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Found by the acceptance pass of #68: a name imported from two modules, one per branch of
/// a `try`, jumped to the first as if there were no second.
#[test]
fn a_name_imported_from_two_modules_offers_both() {
    let (dir, mut a) = project_app(
        "twice",
        &[
            ("pkg/__init__.py", ""),
            ("pkg/a.py", "def pick(x):\n    return x\n"),
            ("pkg/b.py", "def pick(x):\n    return x\n"),
            (
                "use.py",
                "try:\n    from pkg.a import pick\nexcept ImportError:\n    from pkg.b import pick\n\n\ndef run():\n    return pick(1)\n",
            ),
        ],
    );
    a.external
        .insert(Kind::Python, (Vec::new(), Arc::new(Vec::new())));
    d_on(&mut a, "use.py", "return pick");
    let Shown::Picker(status, rows) = shown(&mut a) else {
        panic!("{}", a.message);
    };
    assert_eq!(status, "pick: 2 declarations");
    let places: Vec<&str> = rows.iter().map(|r| r.2.as_str()).collect();
    assert_eq!(places, ["pkg/a.py:1", "pkg/b.py:1"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A declaration with no namesake is its own answer, as it was before the namesake rule.
#[test]
fn a_lone_declaration_stays_where_it_is() {
    let (dir, mut a) = project_app(
        "lonely",
        &[(
            "a.ts",
            "export class A {\n  onlyOne(x: number): number {\n    return x;\n  }\n}\n",
        )],
    );
    a.external
        .insert(Kind::TsJs, (Vec::new(), Arc::new(Vec::new())));
    d_on(&mut a, "a.ts", "  onlyOne");
    assert_eq!(
        shown(&mut a),
        jump("onlyOne \u{2192} A.onlyOne (by name, 1 match)", "a.ts:2")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A qualifier bound by a relative import, or a class of the project, is no value: `d`
/// finds the module-level declaration and does not add a dependency's same-named methods.
#[test]
fn a_module_or_a_class_in_front_of_the_word_is_not_a_value() {
    let (dir, mut a) = project_app(
        "modules",
        &[
            ("shop/views.py", "def index(request):\n    return 1\n"),
            (
                "shop/urls.py",
                "from . import views\n\nurlpatterns = [views.index, views.missing]\n",
            ),
            (
                "web/utils.ts",
                "export function formatDate(d: Date): string {\n  return d.toISOString();\n}\n",
            ),
            (
                "web/app.ts",
                "import * as utils from \"./utils\";\n\nexport const today = utils.formatDate(new Date());\n",
            ),
            (
                "shapes.py",
                "class Outer:\n    class Inner:\n        pass\n\n\nx = Outer.Inner()\n",
            ),
        ],
    );
    let root = external_root(
        "modules",
        &[
            (
                "site/admin.py",
                "class AdminSite:\n    def index(self, request):\n        pass\n",
            ),
            // A dependency with a `views` package must not answer for the project's `views`.
            (
                "django/views/generic.py",
                "def missing(request):\n    pass\n",
            ),
            (
                "node/fmt.d.ts",
                "export declare class Fmt {\n  formatDate(d: Date): string;\n}\n",
            ),
        ],
    );
    use_roots(&mut a, Kind::Python, std::slice::from_ref(&root));
    use_roots(&mut a, Kind::TsJs, std::slice::from_ref(&root));
    for (file, code, want) in [
        (
            "shop/urls.py",
            "views.index",
            jump("index: via import shop/views.py", "shop/views.py:1"),
        ),
        (
            "shop/urls.py",
            "views.missing",
            jump("no definition for missing", "shop/urls.py:3"),
        ),
        (
            "web/app.ts",
            "utils.formatDate",
            jump("formatDate: via import web/utils.ts", "web/utils.ts:1"),
        ),
        (
            "shapes.py",
            "Outer.Inner",
            jump("Inner \u{2192} Outer.Inner (via Outer)", "shapes.py:2"),
        ),
    ] {
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}
