#![expect(clippy::too_many_lines, reason = "test tables")]
use super::*;

#[test]
fn attributes_name_macros_and_fields_reach_outside() {
    let (dir, mut a) = project_app(
        "rust-attr",
        &[
            ("Cargo.toml", "[package]\nname = \"repro\"\n"),
            (
                "src/span.rs",
                "pub struct Span {\n    ticks: u64,\n}\n\npub fn span(t: &dyn Fn()) -> u64 {\n    t.ticks\n}\n",
            ),
            (
                "src/lib.rs",
                "struct Debug;\n\npub struct Tester;\n\nimpl Tester {\n    pub fn test(&self) {}\n    pub fn main(&self) {}\n}\n\n#[derive(Debug, serde::Deserialize)]\nstruct Column;\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn works() {}\n}\n\n#[tokio::main]\nasync fn run(d: std::time::Duration) -> bool {\n    if cfg!(test) {}\n    d.secs > 0\n}\n\n#[allow(dead_code)]\nfn quiet() {}\n",
            ),
        ],
    );
    let std = external_root(
        "rust-attr-std",
        &[
            (
                "core/src/fmt/mod.rs",
                "pub trait Debug {}\n\npub(crate) mod macros {\n    pub macro Debug($item:item) {}\n}\n",
            ),
            (
                "core/src/macros/mod.rs",
                "pub(crate) mod builtin {\n    pub macro test($item:item) {}\n    pub macro derive($item:item) {}\n}\n",
            ),
            (
                "core/src/time.rs",
                "pub struct Duration {\n    secs: u64,\n}\n\npub struct Instant {\n    pub secs: u64,\n}\n",
            ),
            (
                "core/src/tick.rs",
                "pub struct Tick {\n    pub ticks: u64,\n}\n",
            ),
            (
                "serde_derive-1.0.0/src/lib.rs",
                "#[proc_macro_derive(Deserialize, attributes(serde))]\npub fn derive_deserialize(input: TokenStream) -> TokenStream {\n    input\n}\n",
            ),
            (
                "tokio-macros-2.0.0/src/lib.rs",
                "#[proc_macro_attribute]\npub fn main(args: TokenStream, item: TokenStream) -> TokenStream {\n    item\n}\n\npub fn test(x: u32) {}\n",
            ),
        ],
    );
    use_roots(&mut a, Kind::Rust, std::slice::from_ref(&std));
    for (code, want) in [
        (
            "derive(Debug",
            jump(
                "Debug \u{2192} macros::Debug (by name, 1 match)",
                "core/src/fmt/mod.rs:4",
            ),
        ),
        (
            "serde::Deserialize",
            jump(
                "Deserialize: by name, 1 match",
                "serde_derive-1.0.0/src/lib.rs:1",
            ),
        ),
        (
            "    #[test",
            jump(
                "test \u{2192} builtin::test (by name, 1 match)",
                "core/src/macros/mod.rs:2",
            ),
        ),
        (
            "#[derive",
            jump(
                "derive \u{2192} builtin::derive (by name, 1 match)",
                "core/src/macros/mod.rs:3",
            ),
        ),
        (
            "tokio::main",
            jump("main: by name, 1 match", "tokio-macros-2.0.0/src/lib.rs:2"),
        ),
        (
            "cfg(test",
            Shown::Jump("no definition for test".into(), String::new()),
        ),
        (
            "cfg!(test",
            Shown::Jump("no definition for test".into(), String::new()),
        ),
        (
            "#[allow",
            Shown::Jump("no definition for allow".into(), String::new()),
        ),
        (
            "dead_code",
            Shown::Jump("no definition for dead_code".into(), String::new()),
        ),
        (
            "#[tokio",
            Shown::Jump("no definition for tokio".into(), String::new()),
        ),
        (
            "d.secs",
            jump(
                "secs \u{2192} Instant::secs (by name, 1 match)",
                "core/src/time.rs:6",
            ),
        ),
    ] {
        d_on(&mut a, "src/lib.rs", code);
        let got = match shown(&mut a) {
            Shown::Jump(status, _) if status.starts_with("no definition") => {
                Shown::Jump(status, String::new())
            }
            Shown::Jump(status, place) => {
                Shown::Jump(status, place.replace(&format!("{}/", std.display()), ""))
            }
            other @ Shown::Picker(..) => other,
        };
        assert_eq!(got, want, "{code}");
    }
    d_on(&mut a, "src/span.rs", "t.ticks");
    assert_eq!(
        shown(&mut a),
        jump(
            "ticks \u{2192} Span::ticks (by name, 1 match)",
            "src/span.rs:2"
        ),
        "the project's field, and none of the standard library's"
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn a_method_of_an_unknown_type_is_every_reachable_one() {
    let lock = "version = 3\n\n[[package]]\nname = \"dep\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n\n[[package]]\nname = \"far\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n\n[[package]]\nname = \"other\"\nversion = \"0.1.0\"\ndependencies = [\n \"far\",\n]\n\n[[package]]\nname = \"repro\"\nversion = \"0.1.0\"\ndependencies = [\n \"dep\",\n]\n";
    let (dir, mut a) = project_app(
        "rust-358",
        &[
            ("Cargo.toml", "[package]\nname = \"repro\"\n"),
            ("Cargo.lock", lock),
            ("other/Cargo.toml", "[package]\nname = \"other\"\n"),
            (
                "other/src/lib.rs",
                "pub struct Query;\n\nimpl Query {\n    fn unwrap(self) -> Query {\n        self\n    }\n\n    pub fn to_string(&self) -> String {\n        String::new()\n    }\n}\n",
            ),
            (
                "src/lib.rs",
                "mod extra;\n\npub struct Error {\n    msg: String,\n}\n\nimpl Clone for Error {\n    fn clone(&self) -> Error {\n        Error { msg: self.msg.clone() }\n    }\n}\n\nimpl Error {\n    fn describe(&self) -> u32 {\n        1\n    }\n}\n\npub fn first(v: Opaque) -> u32 {\n    v.unwrap()\n}\n\npub fn name(n: &String) -> String {\n    n.clone()\n}\n\npub fn text(s: &str) -> String {\n    s.to_string()\n}\n\npub fn told(e: &Error, x: &extra::Extra) -> u32 {\n    x.secret();\n    e.describe()\n}\n\npub fn peeked(v: Option<u32>) {\n    v.peek()\n}\n\npub fn drained(q: Vec<u32>) {\n    q.into_iter();\n}\n\npub fn opted(w: Option<u32>) -> u32 {\n    w.unwrap()\n}\n\npub struct Bag;\n\nimpl Bag {\n    pub fn len(&self) -> usize {\n        0\n    }\n}\n\npub fn sized(m: &String) -> usize {\n    m.len()\n}\n\npub fn lit() -> usize {\n    let k = \"x\";\n    k.len()\n}\n\nimpl Bag {\n    pub fn frob(&self) {}\n}\n\npub fn frobbed(f: &String) {\n    f.frob()\n}\n\npub fn arr() -> bool {\n    [1, 2].contains(&1)\n}\n\npub fn joined() -> usize {\n    [\"a\", \"b\"].concat().len()\n}\n",
            ),
            (
                "src/extra.rs",
                "pub struct Extra;\n\nimpl Extra {\n    fn secret(&self) {}\n}\n",
            ),
            (
                "src/strs.rs",
                "use dep::String;\n\npub fn sized(m: &String) -> usize {\n    m.len()\n}\n",
            ),
        ],
    );
    let std = external_root(
        "rust-358-std",
        &[
            (
                "lib/rustlib/src/rust/library/core/src/option.rs",
                "pub enum Option<T> {\n    None,\n    Some(T),\n}\n\nimpl<T> Option<T> {\n    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n    pub fn unwrap(self) -> T {\n        loop {}\n    }\n\n    pub(crate) fn peek(&self) {}\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/core/src/result.rs",
                "pub enum Result<T, E> {\n    Ok(T),\n    Err(E),\n}\n\nimpl<T, E> Result<T, E> {\n    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n    pub fn unwrap(self) -> T {\n        loop {}\n    }\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/core/src/clone.rs",
                "#[stable(feature = \"rust1\", since = \"1.0.0\")]\npub const trait Clone: Sized {\n    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n    fn clone(&self) -> Self;\n}\n\nconst impl Clone for u32 {\n    fn clone(&self) -> u32 {\n        *self\n    }\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/alloc/src/string.rs",
                "#[stable(feature = \"rust1\", since = \"1.0.0\")]\npub trait ToString {\n    fn to_string(&self) -> String;\n}\n\nimpl<T: fmt::Display + ?Sized> ToString for T {\n    default fn to_string(&self) -> String {\n        String::new()\n    }\n}\n\npub struct String;\n\nimpl Clone for String {\n    fn clone(&self) -> String {\n        String\n    }\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/core/src/str/mod.rs",
                "impl str {\n    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n    pub fn len(&self) -> usize {\n        0\n    }\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/core/src/slice/mod.rs",
                "impl<T> [T] {\n    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n    pub fn contains(&self, x: &T) -> bool {\n        false\n    }\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/core/src/iter/traits/collect.rs",
                "#[stable(feature = \"rust1\", since = \"1.0.0\")]\npub const trait IntoIterator {\n    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n    fn into_iter(self) -> Self::IntoIter;\n}\n\nconst impl<I: Iterator> IntoIterator for I {\n    fn into_iter(self) -> I {\n        self\n    }\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/alloc/src/vec/mod.rs",
                "pub struct Vec<T>;\n\nmacro_rules! into_iter_impls {\n    () => {\n        const impl<T> IntoIterator for Vec<T> {\n            fn into_iter(self) -> IntoIter<T> {\n                loop {}\n            }\n        }\n    };\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/core/src/spec.rs",
                "trait SpecUnwrap {\n    fn unwrap(self);\n}\n\nimpl<T> SpecUnwrap for Option<T> {\n    fn unwrap(self) {}\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/std/src/sys/process.rs",
                "pub struct Command;\n\nimpl Command {\n    pub fn unwrap(&self) {}\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/coretests/tests/option.rs",
                "fn probe() {\n    trait Probe {\n        fn unwrap(self);\n    }\n}\n",
            ),
        ],
    );
    let registry = external_root(
        "rust-358-registry",
        &[
            (
                "dep-1.0.0/src/lib.rs",
                "pub struct Thing;\n\nimpl Thing {\n    pub fn unwrap(self) {}\n}\n",
            ),
            (
                "dep-1.0.0/tests/it.rs",
                "struct Probe;\n\nimpl Probe {\n    pub fn unwrap(self) {}\n}\n",
            ),
            (
                "far-1.0.0/src/lib.rs",
                "pub struct Far;\n\nimpl Far {\n    pub fn unwrap(self) {}\n}\n",
            ),
            (
                "dep-1.0.0/benches/b.rs",
                "struct Bench;\n\nimpl Bench {\n    pub fn unwrap(self) {}\n}\n",
            ),
            (
                "dep-1.0.0/examples/e.rs",
                "struct Example;\n\nimpl Example {\n    pub fn unwrap(self) {}\n}\n",
            ),
            (
                "dep-1.0.0/src/sealed.rs",
                "trait Hidden {\n    fn unwrap(self);\n}\n",
            ),
            (
                "dep-1.0.0/src/open.rs",
                "pub trait Hidden {\n    fn unwrap(self);\n}\n",
            ),
            (
                "dep-1.0.0/src/imp.rs",
                "impl Hidden for Thing {\n    fn unwrap(self) {}\n}\n",
            ),
        ],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, &[library.clone(), registry.clone()]);
    let row = |name: &str, place: &str| (name.to_owned(), "by name".to_owned(), place.to_owned());
    for (code, want) in [
        (
            "v.unwrap",
            Shown::Picker(
                "unwrap: by name, 5 declarations".into(),
                vec![
                    row("Hidden::unwrap", "dep-1.0.0/src/open.rs:2"),
                    row("Option::unwrap", "core/src/option.rs:8"),
                    row("Result::unwrap", "core/src/result.rs:8"),
                    row("Thing::unwrap", "dep-1.0.0/src/imp.rs:2"),
                    row("Thing::unwrap", "dep-1.0.0/src/lib.rs:4"),
                ],
            ),
        ),
        (
            "n.clone",
            jump(
                "clone \u{2192} String::clone (via n: String)",
                "alloc/src/string.rs:15",
            ),
        ),
        (
            "w.unwrap",
            jump(
                "unwrap \u{2192} Option::unwrap (via w: Option)",
                "core/src/option.rs:8",
            ),
        ),
        (
            "m.len",
            jump(
                "len \u{2192} str::len (via m: String)",
                "core/src/str/mod.rs:3",
            ),
        ),
        (
            "k.len",
            jump(
                "len \u{2192} str::len (via k: str)",
                "core/src/str/mod.rs:3",
            ),
        ),
        (
            "s.to_string",
            jump(
                "to_string \u{2192} ToString::to_string (via trait ToString)",
                "alloc/src/string.rs:3",
            ),
        ),
        (
            "e.describe",
            jump(
                "describe \u{2192} Error::describe (via e: Error)",
                "src/lib.rs:14",
            ),
        ),
        (
            "x.secret",
            Shown::Jump("no definition for secret".into(), String::new()),
        ),
        (
            "q.into_iter",
            jump(
                "into_iter \u{2192} Vec::into_iter (via q: Vec)",
                "alloc/src/vec/mod.rs:6",
            ),
        ),
        (
            "v.peek",
            Shown::Jump("no definition for peek".into(), String::new()),
        ),
        (
            "f.frob",
            Shown::Jump("no definition for frob".into(), String::new()),
        ),
        (
            "[1, 2].contains",
            jump("contains: via [..]: [T; N]", "core/src/slice/mod.rs:3"),
        ),
        (
            "concat().len",
            Shown::Picker(
                "len: by name, 2 declarations".into(),
                vec![
                    row("Bag::len", "src/lib.rs:51"),
                    row("str::len", "core/src/str/mod.rs:3"),
                ],
            ),
        ),
    ] {
        d_on(&mut a, "src/lib.rs", code);
        let strip = |p: &str| {
            p.replace(&format!("{}/", library.display()), "")
                .replace(&format!("{}/", registry.display()), "")
        };
        let got = match shown(&mut a) {
            Shown::Jump(status, _) if status.starts_with("no definition") => {
                Shown::Jump(status, String::new())
            }
            Shown::Jump(status, place) => Shown::Jump(status, strip(&place)),
            Shown::Picker(status, rows) => Shown::Picker(
                status,
                rows.into_iter()
                    .map(|(n, w, p)| (n, w, strip(&p)))
                    .collect(),
            ),
        };
        assert_eq!(got, want, "{code}");
    }
    d_on(&mut a, "src/strs.rs", "m.len");
    assert!(
        !matches!(shown(&mut a), Shown::Jump(s, _) if s.contains("str::len")),
        "a String imported from a crate is not the standard library's"
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
    std::fs::remove_dir_all(&registry).unwrap();
}

#[test]
fn each_reach_rule_keeps_or_drops_its_namesake() {
    let stable = "#[stable(feature = \"rust1\", since = \"1.0.0\")]";
    let lock = "version = 3\n\n[[package]]\nname = \"dep\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\ndependencies = [\"mid 1.0.0 (registry+https://github.com/rust-lang/crates.io-index)\"]\n\n[[package]]\nname = \"far\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n\n[[package]]\nname = \"mid\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n\n[[package]]\nname = \"near\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"repro\"\nversion = \"0.1.0\"\ndependencies = [\n \"dep\",\n \"near\",\n]\n";
    let (dir, mut a) = project_app(
        "rust-358-rules",
        &[
            (
                "Cargo.toml",
                "[package]\nversion = \"0.1.0\"\nname=\"repro\"\n",
            ),
            ("Cargo.lock", lock),
            ("near/Cargo.toml", "[package]\nname = \"near\"\n"),
            (
                "near/src/lib.rs",
                "pub struct N;\n\nimpl N {\n    pub(crate) fn inner(&self) {}\n    pub fn kick(&self) {}\n}\n",
            ),
            ("vend/Cargo.toml", "[package]\nname = \"vend\"\n"),
            (
                "vend/src/lib.rs",
                "pub struct V;\n\nimpl V {\n    pub fn kick(&self) {}\n}\n",
            ),
            (
                "src/lib.rs",
                "mod extra;\n\ntrait Local {\n    fn loc(&self);\n}\n\nimpl Local for u32 {\n    fn loc(&self) {}\n}\n\npub fn calls(x: &near::N, n: u32, v: Vec<u32>) {\n    x.inner();\n    x.kick();\n    n.loc();\n    n.dup();\n    n.wr();\n    n.mac();\n    n.wide();\n    v.unst();\n}\n\npub struct W(Vec<u32>);\n\nimpl W {\n    pub fn is_empty(&self) -> bool { self.0.is_empty() }\n    pub(crate) fn shared(&self) {}\n}\n",
            ),
            (
                "src/extra.rs",
                "pub struct E;\n\nimpl E {\n    fn hidden(&self) {}\n}\n\nimpl<T>\n    Wrapped for T {\n    fn wr(&self) {}\n}\n\nmacro_rules! methods {\n    () => {\n        fn mac(&self) {}\n    };\n}\n",
            ),
            (
                "src/extra/sub.rs",
                "pub fn go(e: &super::E, w: &crate::W) {\n    e.hidden();\n    w.shared();\n}\n",
            ),
            (
                "tests/it.rs",
                "mod helper;\n\npub struct T;\n\nimpl T {\n    fn zorb(&self) {}\n}\n",
            ),
            (
                "tests/helper.rs",
                "pub fn go(t: &super::T) {\n    t.zorb();\n}\n",
            ),
        ],
    );
    let std = external_root(
        "rust-358-rules-std",
        &[
            (
                "lib/rustlib/src/rust/library/alloc/src/vec.rs",
                &format!(
                    "impl<T> Vec<T> {{\n    {stable}\n    pub fn is_empty(&self) -> bool {{\n        true\n    }}\n\n    #[unstable(feature = \"x\", issue = \"1\")]\n    pub fn unst(&self) {{}}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/core/src/dup.rs",
                &format!("{stable}\npub trait Dup {{\n    fn dup(&self);\n}}\n"),
            ),
            (
                "lib/rustlib/src/rust/library/compiler-builtins/src/int.rs",
                "pub trait Int {\n    fn wide(&self);\n}\n\nimpl Int for u64 {\n    fn wide(&self) {}\n}\n",
            ),
        ],
    );
    let registry = external_root(
        "rust-358-rules-registry",
        &[
            (
                "dep-1.0.0/src/lib.rs",
                "pub trait Dup {\n    fn dup(&self);\n}\n",
            ),
            (
                "mid-1.0.0/src/lib.rs",
                "pub struct M;\n\nimpl M {\n    pub fn kick(&self) {}\n}\n",
            ),
            (
                "near-0.1.0/src/lib.rs",
                "pub struct N;\n\nimpl N {\n    pub fn kick(&self) {}\n}\n",
            ),
            (
                "far-1.0.0/src/lib.rs",
                "pub struct F;\n\nimpl F {\n    pub fn kick(&self) {}\n}\n",
            ),
        ],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, &[library.clone(), registry.clone()]);
    let row = |name: &str, place: &str| (name.to_owned(), "by name".to_owned(), place.to_owned());
    let none = |w: &str| Shown::Jump(format!("no definition for {w}"), String::new());
    for (name, file, code, want) in [
        (
            "`pub(crate)` of another crate of the project",
            "src/lib.rs",
            "x.inner",
            none("inner"),
        ),
        (
            "the project's own, a package the lock does not list (`vend`) and a transitive dependency (`mid`); not the registry copy of the workspace's `near`, nor `far`",
            "src/lib.rs",
            "x.kick",
            Shown::Picker(
                "kick: by name, 3 declarations".into(),
                vec![
                    row("N::kick", "near/src/lib.rs:5"),
                    row("V::kick", "vend/src/lib.rs:4"),
                    row("M::kick", "mid-1.0.0/src/lib.rs:4"),
                ],
            ),
        ),
        (
            "a private trait of the project",
            "src/lib.rs",
            "n.loc",
            jump("loc \u{2192} Local::loc (via trait Local)", "src/lib.rs:4"),
        ),
        (
            "two traits of one name: no jump",
            "src/lib.rs",
            "n.dup",
            Shown::Picker(
                "dup: by name, 2 declarations".into(),
                vec![
                    row("Dup::dup", "core/src/dup.rs:3"),
                    row("Dup::dup", "dep-1.0.0/src/lib.rs:2"),
                ],
            ),
        ),
        (
            "a header wrapped before its `for`: read as unreadable, kept however private",
            "src/lib.rs",
            "n.wr",
            jump("wr: by name, 1 match", "src/extra.rs:9"),
        ),
        (
            "a method a macro writes: read as unreadable, kept however private",
            "src/lib.rs",
            "n.mac",
            jump("mac: by name, 1 match", "src/extra.rs:14"),
        ),
        (
            "the standard library's own trait, with no stability attribute, and its `impl`",
            "src/lib.rs",
            "n.wide",
            none("wide"),
        ),
        (
            "`#[unstable]` is offered",
            "src/lib.rs",
            "v.unst",
            jump(
                "unst \u{2192} Vec::unst (by name, 1 match)",
                "alloc/src/vec.rs:8",
            ),
        ),
        (
            "the call on a one-line delegation is no declaration of its own",
            "src/lib.rs",
            "self.0.is_empty",
            jump(
                "is_empty \u{2192} Vec::is_empty (by name, 1 match)",
                "alloc/src/vec.rs:3",
            ),
        ),
        (
            "a child module sees its parent's private items",
            "src/extra/sub.rs",
            "e.hidden",
            jump("hidden \u{2192} E::hidden (via e: E)", "src/extra.rs:4"),
        ),
        (
            "a child module sees its crate's `pub(crate)` items",
            "src/extra/sub.rs",
            "w.shared",
            jump("shared \u{2192} W::shared (via w: W)", "src/lib.rs:26"),
        ),
        (
            "a crate root of `tests/` has its directory",
            "tests/helper.rs",
            "t.zorb",
            jump("zorb \u{2192} T::zorb (via t: T)", "tests/it.rs:6"),
        ),
    ] {
        d_on(&mut a, file, code);
        let strip = |p: &str| {
            p.replace(&format!("{}/", library.display()), "")
                .replace(&format!("{}/", registry.display()), "")
        };
        let got = match shown(&mut a) {
            Shown::Jump(status, _) if status.starts_with("no definition") => {
                Shown::Jump(status, String::new())
            }
            Shown::Jump(status, place) => Shown::Jump(status, strip(&place)),
            Shown::Picker(status, rows) => Shown::Picker(
                status,
                rows.into_iter()
                    .map(|(n, w, p)| (n, w, strip(&p)))
                    .collect(),
            ),
        };
        assert_eq!(got, want, "{name}: {code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
    std::fs::remove_dir_all(&registry).unwrap();
}

#[test]
fn a_literal_key_is_its_struct_field_inside_and_out() {
    let (dir, mut a) = project_app(
        "rust-literal-key",
        &[
            ("Cargo.toml", "[package]\nname = \"shop\"\n"),
            (
                "src/lib.rs",
                "use knobs::Opts;\n\npub struct Printer {\n    hyperlink: u32,\n}\n\nimpl Printer {\n    pub fn hyperlink(&self) -> u32 {\n        self.hyperlink\n    }\n\n    pub fn depth(&self) -> u32 {\n        0\n    }\n}\n\npub fn made() -> Printer {\n    Printer { hyperlink: 1 }\n}\n\npub fn opts() -> Opts {\n    Opts { depth: 2 }\n}\n",
            ),
        ],
    );
    let registry = external_root(
        "rust-literal-key",
        &[(
            "knobs-1.0.0/src/lib.rs",
            "pub struct Opts {\n    pub depth: u32,\n}\n",
        )],
    );
    let knobs = registry.join("knobs-1.0.0");
    use_roots(&mut a, Kind::Rust, std::slice::from_ref(&knobs));
    d_on(&mut a, "src/lib.rs", "Printer { hyperlink|: 1");
    assert_eq!(
        at(&a),
        (dir.join("src/lib.rs"), 3),
        "the field of the literal's type, never a method of the name: {}",
        a.message
    );
    d_on(&mut a, "src/lib.rs", "Opts { depth|: 2");
    assert_eq!(
        at(&a),
        (knobs.join("src/lib.rs"), 1),
        "a dependency's field: {}",
        a.message
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&registry).unwrap();
}

#[test]
fn a_path_is_looked_up_in_the_crate_its_first_name_names() {
    let (dir, mut a) = project_app(
        "rust-crate-path",
        &[
            ("Cargo.toml", "[package]\nname = \"shop\"\n"),
            (
                "src/lib.rs",
                "mod helpers;\nmod other;\n\nuse std::fs::File;\nuse std::io;\nuse crate::helpers::norm;\nuse grep_matcher::Match;\nuse std::error::Error as StdError;\n\npub struct Mmap;\n\nimpl Mmap {\n    pub fn open(&self) {}\n}\n\npub struct CommandError;\n\nimpl CommandError {\n    pub(crate) fn io(_e: io::Error) -> CommandError {\n        CommandError\n    }\n}\n\npub fn read(p: &str) -> io::Result<File> {\n    File::open(p)\n}\n\npub fn g() -> u32 {\n    norm(1)\n}\n\npub fn made(m: Match) -> std::fs::File {\n    std::fs::File::create(\"x\")\n}\n\npub fn shared() -> std::sync::Arc<u32> {\n    todo!()\n}\n\npub fn most() -> usize {\n    usize::MAX\n}\n\npub fn map() {\n    memmap2::Mmap::map()\n}\n\npub fn boxed(e: Box<dyn StdError>) {}\n\npub enum Kind {\n    Match(u32),\n}\n\npub fn pick(k: Kind) -> u32 {\n    use self::Kind::*;\n    match k {\n        Match(n) => n,\n    }\n}\n",
            ),
            ("src/helpers.rs", "pub fn norm(x: u32) -> u32 {\n    x\n}\n"),
            (
                "src/other.rs",
                "fn norm(y: u8) -> u8 {\n    y\n}\n\npub struct Match;\n\npub fn create() {}\n",
            ),
            (
                "crates/matcher/Cargo.toml",
                "[package]\nname = \"grep-matcher\"\n",
            ),
            ("crates/matcher/src/lib.rs", "pub struct Match;\n"),
        ],
    );
    let std = external_root(
        "rust-crate-path",
        &[
            (
                "library/std/src/lib.rs",
                "pub mod fs;\npub mod io;\npub mod os;\npub mod sync;\npub mod error;\n",
            ),
            (
                "library/std/src/fs.rs",
                "pub struct File;\n\nimpl File {\n    pub fn open(p: &str) {}\n    pub fn create(p: &str) {}\n}\n\npub struct OpenOptions;\n\nimpl OpenOptions {\n    pub fn open(&self) {}\n}\n",
            ),
            (
                "library/std/src/io/mod.rs",
                "mod error;\npub use self::error::{Error, Result};\n",
            ),
            (
                "library/std/src/io/error.rs",
                "pub type Result<T> = core::result::Result<T, Error>;\n\npub struct Error;\n",
            ),
            ("library/std/src/os/aix/fs.rs", "pub fn create() {}\n"),
            (
                "library/std/src/sync/mod.rs",
                "pub use alloc_crate::sync::Arc;\n",
            ),
            ("library/std/src/error.rs", "pub use core::error::Error;\n"),
            ("library/core/src/error.rs", "pub trait Error {}\n"),
            ("library/alloc/src/sync.rs", "pub struct Arc<T>(T);\n"),
            (
                "library/core/src/num/uint_macros.rs",
                "macro_rules! uint_impl {\n    () => {\n        pub const MAX: Self = 0;\n    };\n}\n",
            ),
            (
                "registry/memmap2-0.9.0/src/lib.rs",
                "pub struct Mmap;\n\nimpl Mmap {\n    pub fn map() {}\n}\n",
            ),
        ],
    );
    let library = std.join("library");
    let memmap = std.join("registry/memmap2-0.9.0");
    use_roots(&mut a, Kind::Rust, &[library.clone(), memmap.clone()]);
    let lib = |p: &str| library.join(p);
    for (name, code, place, status) in [
        (
            "",
            "File::open|(p)",
            lib("std/src/fs.rs"),
            4,
            "via import std::fs",
        ),
        (
            "",
            "-> io|::Result",
            lib("std/src/lib.rs"),
            2,
            "via import std",
        ),
        (
            "",
            "    norm|(1)",
            dir.join("src/helpers.rs"),
            1,
            "via import crate::helpers",
        ),
        (
            "",
            "_e: io::Error",
            lib("std/src/io/error.rs"),
            3,
            "via import std::io",
        ),
        (
            "",
            "m: Match",
            dir.join("crates/matcher/src/lib.rs"),
            1,
            "via import grep_matcher",
        ),
        ("", "-> std::fs|::File", lib("std/src/lib.rs"), 1, "via std"),
        (
            "",
            "std::fs::File::create",
            lib("std/src/fs.rs"),
            5,
            "via std::fs",
        ),
        (
            "",
            "std::sync::Arc",
            lib("alloc/src/sync.rs"),
            1,
            "via alloc::sync",
        ),
        (
            "",
            "usize::MAX",
            lib("core/src/num/uint_macros.rs"),
            3,
            "by name",
        ),
        (
            "A glob `use` of the block hides the file's `use` of the name",
            "        Match|(n)",
            dir.join("src/lib.rs"),
            51,
            "Kind::Match",
        ),
        (
            "",
            "dyn StdError",
            lib("core/src/error.rs"),
            1,
            "via import core::error",
        ),
        (
            "",
            "memmap2::Mmap::map",
            memmap.join("src/lib.rs"),
            4,
            "via memmap2",
        ),
    ]
    .map(|(n, c, p, l, s)| (n, c, (p, l), s))
    {
        d_on(&mut a, "src/lib.rs", code);
        let (path, line) = at(&a);
        assert_eq!((path, line + 1), place, "{name}: {code}: {}", a.message);
        assert!(a.message.contains(status), "{name}: {code}: {}", a.message);
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn a_private_method_is_seen_from_its_module_and_below() {
    let private =
        |ty: &str, m: &str| format!("struct {ty};\n\nimpl {ty} {{\n    fn {m}(&self) {{}}\n}}\n");
    let calls = |ms: &[&str]| {
        let body: String = ms
            .iter()
            .map(|m| format!("    v.iter().for_each(|x| x.{m}());\n"))
            .collect();
        format!("pub fn go(v: Vec<u32>) {{\n{body}}}\n")
    };
    let files = [
        (
            "Cargo.toml".to_owned(),
            "[package]\nname = \"reach\"\n".to_owned(),
        ),
        ("build.rs".to_owned(), private("B", "from_build")),
        ("src/lib.rs".to_owned(), private("L", "from_lib")),
        ("src/main.rs".to_owned(), private("M", "from_main")),
        ("src/belt.rs".to_owned(), private("Bt", "from_belt")),
        ("src/belt/strap.rs".to_owned(), calls(&["from_belt"])),
        ("src/gears/mod.rs".to_owned(), private("G", "from_mod")),
        ("src/gears/cog.rs".to_owned(), calls(&["from_mod"])),
        (
            "src/user.rs".to_owned(),
            calls(&[
                "from_build",
                "from_lib",
                "from_main",
                "from_belt",
                "from_mod",
            ]),
        ),
        ("tests/wind.rs".to_owned(), private("T", "from_tests")),
        ("tests/unwind.rs".to_owned(), calls(&["from_tests"])),
        ("benches/wind.rs".to_owned(), private("Bn", "from_benches")),
        ("benches/unwind.rs".to_owned(), calls(&["from_benches"])),
        ("examples/wind.rs".to_owned(), private("E", "from_examples")),
        ("examples/unwind.rs".to_owned(), calls(&["from_examples"])),
        ("src/bin/wind.rs".to_owned(), private("Bi", "from_bin")),
        ("src/bin/unwind.rs".to_owned(), calls(&["from_bin"])),
    ];
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    let (dir, mut a) = project_app("rust-private-reach", &files);
    for (file, method, want) in [
        ("src/user.rs", "from_build", Some("build.rs")),
        ("src/user.rs", "from_lib", Some("src/lib.rs")),
        ("src/user.rs", "from_main", Some("src/main.rs")),
        ("src/user.rs", "from_belt", None),
        ("src/belt/strap.rs", "from_belt", Some("src/belt.rs")),
        ("src/user.rs", "from_mod", None),
        ("src/gears/cog.rs", "from_mod", Some("src/gears/mod.rs")),
        ("tests/unwind.rs", "from_tests", Some("tests/wind.rs")),
        ("benches/unwind.rs", "from_benches", Some("benches/wind.rs")),
        (
            "examples/unwind.rs",
            "from_examples",
            Some("examples/wind.rs"),
        ),
        ("src/bin/unwind.rs", "from_bin", Some("src/bin/wind.rs")),
    ] {
        d_on(&mut a, file, &format!("x.{method}"));
        let got = if a.message.starts_with("no definition") {
            None
        } else {
            Some(at(&a))
        };
        assert_eq!(
            got,
            want.map(|p| (dir.join(p), 3)),
            "{method} from {file}: {}",
            a.message
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_path_follows_the_modules_and_uses_of_its_crate() {
    let (dir, mut a) = project_app(
        "rust-crate-uses",
        &[
            ("Cargo.toml", "[package]\nname = \"yard\"\n"),
            (
                "src/lib.rs",
                "mod tally;\nmod shapes;\nmod geo;\n\npub fn counted() -> u32 {\n    tally::count()\n}\n\npub fn walked() {\n    std::fs::Walker::new()\n}\n\npub fn synced(f: &str) {\n    std::fs::File::sync(f)\n}\n\npub fn squared(s: crate::shapes::Square) {}\n\npub fn circled(c: crate::shapes::inner::Circle) {}\n\npub fn drummed(d: winchlib::Drum) {}\n\npub fn knotted() {\n    rope_knots::Knot::tie()\n}\n\npub fn spun() {\n    pulley_kit::Sheave::spin()\n}\n",
            ),
            ("src/tally.rs", "pub fn count() -> u32 {\n    0\n}\n"),
            (
                "src/shapes.rs",
                "mod inner;\n\npub fn local() {\n    use crate::other::Square;\n}\n\npub use crate::geo::Square;\n",
            ),
            ("src/shapes/inner.rs", "pub use super::round::Circle;\n"),
            ("src/shapes/round.rs", "pub struct Circle;\n"),
            ("src/geo.rs", "pub struct Square;\n"),
            ("src/other.rs", "pub struct Square;\n"),
            (
                "crates/winch/Cargo.toml",
                "[package]\nname = \"winch-core\"\n\n[lib]\nname = \"winchlib\"\n",
            ),
            ("crates/winch/src/lib.rs", "pub struct Drum;\n"),
            (
                "crates/pulley/Cargo.toml",
                "[package]\nname = \"pulley-kit\"\n",
            ),
            (
                "crates/pulley/src/lib.rs",
                "mod spin;\n\npub struct Sheave;\n",
            ),
            (
                "crates/pulley/src/spin.rs",
                "impl Sheave {\n    pub fn spin() {}\n}\n",
            ),
        ],
    );
    let std = external_root(
        "rust-crate-uses",
        &[
            ("library/std/src/lib.rs", "pub mod fs;\n"),
            ("library/std/src/fs.rs", "mod walk;\n\npub struct File;\n"),
            (
                "library/std/src/fs/walk.rs",
                "pub struct Walker;\n\nimpl Walker {\n    pub fn new() {}\n}\n",
            ),
            (
                "library/std/src/sys/file_ext.rs",
                "impl File {\n    pub fn sync(f: &str) {}\n}\n",
            ),
            (
                "registry/tally-1.0.0/src/lib.rs",
                "pub fn count() -> u32 {\n    1\n}\n",
            ),
            (
                "registry/rope-knots-1.0.0/src/lib.rs",
                "mod tie;\n\npub struct Knot;\n",
            ),
            (
                "registry/rope-knots-1.0.0/src/tie.rs",
                "impl Knot {\n    pub fn tie() {}\n}\n",
            ),
        ],
    );
    let library = std.join("library");
    use_roots(
        &mut a,
        Kind::Rust,
        &[
            library,
            std.join("registry/tally-1.0.0"),
            std.join("registry/rope-knots-1.0.0"),
        ],
    );
    for (code, place, status) in [
        ("tally::count|()", "src/tally.rs:1", "count: by name"),
        (
            "std::fs::Walker::new",
            "library/std/src/fs/walk.rs:4",
            "via std::fs",
        ),
        (
            "File::sync",
            "library/std/src/sys/file_ext.rs:2",
            "File::sync (by name",
        ),
        ("shapes::Square", "src/geo.rs:1", "via crate::geo"),
        (
            "inner::Circle",
            "src/shapes/round.rs:1",
            "via crate::shapes::round",
        ),
        (
            "winchlib::Drum",
            "crates/winch/src/lib.rs:1",
            "via winchlib",
        ),
        (
            "rope_knots::Knot::tie",
            "registry/rope-knots-1.0.0/src/tie.rs:2",
            "via rope_knots",
        ),
        (
            "pulley_kit::Sheave::spin",
            "crates/pulley/src/spin.rs:2",
            "via pulley_kit",
        ),
    ] {
        d_on(&mut a, "src/lib.rs", code);
        let (path, line) = at(&a);
        let path = path
            .strip_prefix(&dir)
            .or_else(|_| path.strip_prefix(&std))
            .unwrap_or(&path)
            .to_owned();
        assert_eq!(
            format!("{}:{}", path.display(), line + 1),
            place,
            "{code}: {}",
            a.message
        );
        assert!(a.message.contains(status), "{code}: {}", a.message);
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn a_vec_macro_of_the_project_is_not_the_standard_vec() {
    let (dir, mut a) = project_app(
        "rust-vec-macro",
        &[
            (
                "Cargo.toml",
                "[package]\nname = \"m\"\nversion = \"0.1.0\"\n",
            ),
            (
                "src/lib.rs",
                "macro_rules! vec {\n    [] => { Bag };\n}\n\npub struct Bag;\n\nimpl Bag {\n    pub fn push(&self) {}\n}\n\npub fn f() {\n    let b = vec![];\n    b.push();\n}\n",
            ),
        ],
    );
    d_on(&mut a, "src/lib.rs", "b.push");
    let got = shown(&mut a);
    assert!(
        matches!(&got, Shown::Jump(s, p) if s.contains("Bag::push") && p.ends_with("src/lib.rs:8"))
            || matches!(&got, Shown::Picker(_, rows) if rows.iter().any(|r| r.0 == "Bag::push")),
        "the project's own vec! makes a Bag: {got:?}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_indexed_value_is_its_element_and_a_vec_reaches_its_slice() {
    let stable = "    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n";
    let range = "pub struct Range<Idx> {\n    pub start: Idx,\n    pub end: Idx,\n}\n";
    let lib = "use std::ops::Range;\n\npub struct Bag {\n    pub end: usize,\n}\n\nimpl Bag {\n    pub fn len(&self) -> usize {\n        0\n    }\n\n    pub fn contains(&self) -> bool {\n        false\n    }\n}\n\nfn spans() -> Vec<Range<usize>> {\n    Vec::new()\n}\n\npub fn ends(v: &[std::ops::Range<usize>]) -> usize {\n    v[0].end\n}\n\npub fn last() -> usize {\n    let got = spans();\n    got[1].end\n}\n\npub fn bags(b: Vec<Bag>, i: usize) -> usize {\n    b[0].len() + b[i].end\n}\n\npub fn names(n: &[String]) -> usize {\n    n[0].len()\n}\n\npub fn has(w: Vec<u8>) -> bool {\n    w.contains(&1) && w.to_owned().is_empty()\n}\n\npub fn spans_len(s: Vec<Range<usize>>, i: usize) -> usize {\n    s[i].len() + s[1..].len()\n}\n";
    let (dir, mut a) = project_app(
        "rust-indexed",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
        ],
    );
    let std = external_root(
        "rust-indexed-std",
        &[
            ("lib/rustlib/src/rust/library/core/src/ops/range.rs", range),
            ("lib/rustlib/src/rust/library/core/src/range.rs", range),
            (
                "lib/rustlib/src/rust/library/std/src/lib.rs",
                "pub use core::ops;\n",
            ),
            (
                "lib/rustlib/src/rust/library/alloc/src/borrow.rs",
                "impl<T: Clone> ToOwned for T {\n    fn to_owned(&self) -> T {\n        loop {}\n    }\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/core/src/slice/mod.rs",
                &format!(
                    "impl<T> [T] {{\n{stable}    pub fn contains(&self, x: &T) -> bool {{\n        false\n    }}\n}}\n\nimpl<T: Clone> ToOwned for [T] {{\n    fn to_owned(&self) -> Vec<T> {{\n        loop {{}}\n    }}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/core/src/str/mod.rs",
                &format!(
                    "impl str {{\n{stable}    pub fn len(&self) -> usize {{\n        0\n    }}\n\n{stable}    pub fn contains(&self) -> bool {{\n        false\n    }}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/alloc/src/vec/mod.rs",
                &format!(
                    "pub struct Vec<T>;\n\nimpl<T> Vec<T> {{\n{stable}    pub fn len(&self) -> usize {{\n        0\n    }}\n}}\n"
                ),
            ),
        ],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, std::slice::from_ref(&library));
    for (code, want) in [
        ("v[0].end", "core/src/ops/range.rs:3"),
        ("got[1].end", "core/src/ops/range.rs:3"),
        ("b[0].len", "src/lib.rs:8"),
        ("b[i].end", "src/lib.rs:4"),
        ("n[0].len", "core/src/str/mod.rs:3"),
        ("w.contains", "core/src/slice/mod.rs:3"),
    ] {
        d_on(&mut a, "src/lib.rs", code);
        let got = match shown(&mut a) {
            Shown::Jump(_, place) => place.replace(&format!("{}/", library.display()), ""),
            other @ Shown::Picker(..) => format!("{other:?}"),
        };
        assert_eq!(got, want, "{code}");
    }
    for code in ["s[i].len", "s[1..].len", "w.to_owned"] {
        d_on(&mut a, "src/lib.rs", code);
        let got = shown(&mut a);
        assert!(
            !matches!(&got, Shown::Jump(s, p) if !s.starts_with("no definition") && !p.is_empty()),
            "{code} is not typed by its index: {got:?}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn a_std_value_keeps_only_project_traits_it_can_implement() {
    let lib = "pub struct Rack;\n\npub trait Panes {\n    fn windows(&self) -> u32;\n}\n\nimpl Panes for Rack {\n    fn windows(&self) -> u32 {\n        0\n    }\n}\n\npub trait Sniff {\n    fn sniff(&self) -> u32;\n}\n\nimpl Sniff for str {\n    fn sniff(&self) -> u32 {\n        0\n    }\n}\n\npub trait Every {\n    fn every(&self) -> u32;\n}\n\nimpl<T: ?Sized> Every for T {\n    fn every(&self) -> u32 {\n        0\n    }\n}\n\npub trait Halves {\n    fn halves(&self) -> u32;\n}\n\nimpl<T> Halves for [T] {\n    fn halves(&self) -> u32 {\n        0\n    }\n}\n\npub fn used(s: &str, n: String) -> u32 {\n    let v = vec![1, 2, 3];\n    let a = v.windows(2);\n    let b = [1, 2].windows(1);\n    let c = s.sniff() + n.sniff();\n    let d = v.sniff();\n    let e = v.every() + v.halves();\n    let f = [1].halves();\n    a + b + c + d + e + f\n}\n";
    let (dir, mut a) = project_app(
        "rust-std-traits",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
        ],
    );
    use_roots(&mut a, Kind::Rust, &[]);
    for (code, want) in [
        ("s.sniff", "src/lib.rs:18"),
        ("n.sniff", "src/lib.rs:14"),
        ("v.every", "src/lib.rs:24"),
        ("v.halves", "src/lib.rs:34"),
        ("[1].halves", "src/lib.rs:34"),
    ] {
        d_on(&mut a, "src/lib.rs", code);
        let got = match shown(&mut a) {
            Shown::Jump(_, place) => place,
            other @ Shown::Picker(..) => format!("{other:?}"),
        };
        assert_eq!(got, want, "{code}");
    }
    for code in ["v.windows", "[1, 2].windows", "v.sniff"] {
        d_on(&mut a, "src/lib.rs", code);
        let got = shown(&mut a);
        assert!(
            matches!(&got, Shown::Jump(s, _) if s.starts_with("no definition")),
            "{code}: {got:?}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_std_method_s_declared_return_type_types_the_next_link() {
    let stable = "    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n";
    let lib = "use std::collections::HashMap;\n\nconst TEMPLATE: &'static str = \"a\";\n\npub struct Lines {\n    pub n: usize,\n}\n\nimpl Lines {\n    pub fn nth(&self) -> usize {\n        0\n    }\n}\n\npub struct Def {\n    pub globs: Vec<String>,\n}\n\nimpl Def {\n    pub fn clone(&self) -> usize {\n        0\n    }\n\n    pub fn len(&self) -> usize {\n        0\n    }\n}\n\npub struct Bag {\n    pub types: HashMap<String, Def>,\n}\n\nfn contents() -> String {\n    String::new()\n}\n\npub fn second() -> bool {\n    let got = contents();\n    got.lines().nth(1).unwrap().is_empty()\n}\n\npub fn help() -> String {\n    let mut out =\n        TEMPLATE.replace(\"a\", \"b\");\n    out = out.replace(\"b\", \"c\");\n    out\n}\n\nimpl Bag {\n    pub fn globs(&self, name: &str) -> Vec<String> {\n        self.types.get(name).unwrap().globs.clone()\n    }\n\n    pub fn size(&self, name: &str) -> usize {\n        self.types.get(name).unwrap().len()\n    }\n}\n";
    let (dir, mut a) = project_app(
        "rust-std-returns",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
        ],
    );
    let std = external_root(
        "rust-std-returns-std",
        &[
            (
                "lib/rustlib/src/rust/library/core/src/str/mod.rs",
                &format!(
                    "impl str {{\n{stable}    pub fn lines(&self) -> Lines<'_> {{\n        loop {{}}\n    }}\n\n{stable}    pub fn is_empty(&self) -> bool {{\n        false\n    }}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/core/src/str/iter.rs",
                "pub struct Lines<'a>(&'a str);\n\nimpl<'a> Iterator for Lines<'a> {\n    type Item = &'a str;\n\n    fn next(&mut self) -> Option<&'a str> {\n        None\n    }\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/core/src/iter/traits/iterator.rs",
                "pub trait Iterator {\n    type Item;\n\n    fn next(&mut self) -> Option<Self::Item>;\n\n    fn nth(&mut self, n: usize) -> Option<Self::Item> {\n        None\n    }\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/core/src/option.rs",
                &format!(
                    "pub enum Option<T> {{\n    None,\n    Some(T),\n}}\n\nimpl<T> Option<T> {{\n{stable}    pub fn unwrap(self) -> T {{\n        loop {{}}\n    }}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/alloc/src/str.rs",
                &format!(
                    "impl str {{\n{stable}    pub fn replace(&self, from: &str, to: &str) -> String {{\n        loop {{}}\n    }}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/alloc/src/vec/mod.rs",
                "pub struct Vec<T>;\n\nimpl<T: Clone> Clone for Vec<T> {\n    fn clone(&self) -> Self {\n        loop {}\n    }\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/std/src/collections/hash/map.rs",
                &format!(
                    "pub struct HashMap<K, V, S = RandomState>;\n\nimpl<K, V, S> HashMap<K, V, S> {{\n{stable}    pub fn get(&self, k: &K) -> Option<&V> {{\n        None\n    }}\n}}\n"
                ),
            ),
        ],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, std::slice::from_ref(&library));
    for (code, want) in [
        ("got.lines().nth(1).unwrap", "core/src/option.rs:8"),
        ("got.lines().nth", "core/src/iter/traits/iterator.rs:6"),
        ("out.replace", "alloc/src/str.rs:3"),
        ("unwrap().globs.clone", "alloc/src/vec/mod.rs:4"),
        ("unwrap().len", "src/lib.rs:24"),
    ] {
        d_on(&mut a, "src/lib.rs", code);
        let got = match shown(&mut a) {
            Shown::Jump(_, place) => place.replace(&format!("{}/", library.display()), ""),
            other @ Shown::Picker(..) => format!("{other:?}"),
        };
        assert_eq!(got, want, "{code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn a_project_receiver_answers_before_the_toolchain_is_walked() {
    let lib = "pub struct Payout {\n    pub amount: u64,\n}\n\nimpl Payout {\n    pub fn line(&self) -> String {\n        String::new()\n    }\n}\n\npub fn latest() -> Payout {\n    Payout { amount: 1250 }\n}\n\npub fn print_latest() -> String {\n    let payout = latest();\n    payout.line()\n}\n\npub fn chained() -> String {\n    latest().line()\n}\n";
    let (dir, mut a) = project_app(
        "rust-project-first",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
        ],
    );
    for (code, want) in [
        (
            "payout.line",
            jump(
                "line \u{2192} Payout::line (via latest() -> Payout)",
                "src/lib.rs:6",
            ),
        ),
        (
            "latest().line",
            jump(
                "line \u{2192} Payout::line (via latest(): Payout)",
                "src/lib.rs:6",
            ),
        ),
    ] {
        let walk = std::thread::spawn(|| (Vec::new(), Vec::new()));
        a.warming.insert(Kind::Rust, Some(walk));
        d_on(&mut a, "src/lib.rs", code);
        assert_eq!(shown(&mut a), want, "{code}");
        assert!(
            a.warming[&Kind::Rust].is_some(),
            "{code} waited for the walk outside the project"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_std_value_keeps_a_project_trait_whose_impl_merl_cannot_read() {
    let lib = "pub trait Shout {\n    fn shout(&self) -> u32;\n}\n\ntype Name = String;\n\nimpl Shout for Name {\n    fn shout(&self) -> u32 {\n        0\n    }\n}\n\npub trait Whisper {\n    fn whisper(&self) -> u32;\n}\n\nmacro_rules! whisper_for {\n    ($t:ty) => {\n        impl Whisper for $t {\n            fn whisper(&self) -> u32 {\n                0\n            }\n        }\n    };\n}\n\nwhisper_for!(String);\n\npub trait Murmur {\n    fn murmur(&self) -> u32;\n}\n\nimpl<\n    T: std::fmt::Display + std::fmt::Debug + Clone + Send + Sync + PartialEq + 'static,\n> Murmur for Vec<T>\n{\n    fn murmur(&self) -> u32 {\n        0\n    }\n}\n\npub fn used(s: String, w: Vec<String>) -> u32 {\n    let v = vec![1, 2, 3];\n    s.shout() + s.whisper() + w[0].shout() + v.murmur()\n}\n";
    let (dir, mut a) = project_app(
        "rust-std-unread-impls",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
        ],
    );
    use_roots(&mut a, Kind::Rust, &[]);
    for (code, want) in [
        ("s.shout", "src/lib.rs:2"),
        ("s.whisper", "src/lib.rs:14"),
        ("w[0].shout", "src/lib.rs:2"),
        ("v.murmur", "src/lib.rs:30"),
    ] {
        d_on(&mut a, "src/lib.rs", code);
        let got = match shown(&mut a) {
            Shown::Jump(_, place) => place,
            other @ Shown::Picker(..) => format!("{other:?}"),
        };
        assert_eq!(got, want, "{code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_for_variable_and_an_as_ref_of_a_bound_take_their_std_type() {
    let stable = "#[stable(feature = \"rust1\", since = \"1.0.0\")]\n";
    let lib = "use std::{\n    ffi::OsString,\n    path::{Path, PathBuf},\n};\n\npub struct Leaf;\n\nimpl Leaf {\n    pub fn join(&self) {}\n\n    pub fn display(&self) {}\n}\n\npub fn find<P: AsRef<Path>>(\n    prog: P,\n    paths: OsString,\n) -> Option<PathBuf> {\n    use std::env;\n\n    fn is_exe(path: &Path) -> bool {\n        false\n    }\n\n    let prog = prog.as_ref();\n    for dir in env::split_paths(&paths) {\n        dir.join(prog);\n    }\n    prog.display();\n    None\n}\n\npub fn by_impl(q: impl AsRef<Path>) {\n    let q = q.as_ref();\n    q.display();\n}\n\npub fn by_where<W>(w: W)\nwhere\n    W: AsRef<Path>,\n{\n    let w = w.as_ref();\n    w.display();\n}\n\npub fn by_two<T: AsRef<Path> + AsRef<str>>(t: T) {\n    let t = t.as_ref();\n    t.display();\n}\n\npub struct Bag;\n\nimpl Iterator for Bag {\n    type Item = Leaf;\n    fn next(&mut self) -> Option<Leaf> {\n        None\n    }\n}\n\npub fn walk(bag: Bag) {\n    for leaf in bag {\n        leaf.join();\n    }\n}\n";
    let (dir, mut a) = project_app(
        "rust-std-bound",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
        ],
    );
    let std = external_root(
        "rust-std-bound-std",
        &[
            (
                "lib/rustlib/src/rust/library/std/src/env.rs",
                &format!(
                    "use crate::path::PathBuf;\n\n{stable}pub fn split_paths<T: AsRef<OsStr> + ?Sized>(unparsed: &T) -> SplitPaths<'_> {{\n    loop {{}}\n}}\n\npub struct SplitPaths<'a> {{\n    inner: &'a str,\n}}\n\nimpl<'a> Iterator for SplitPaths<'a> {{\n    type Item = PathBuf;\n    fn next(&mut self) -> Option<PathBuf> {{\n        None\n    }}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/std/src/sys/paths.rs",
                "pub struct SplitPaths<'a>(&'a str);\n",
            ),
            (
                "lib/rustlib/src/rust/library/std/src/path.rs",
                &format!(
                    "pub struct PathBuf;\n\npub struct Path;\n\nimpl Path {{\n    {stable}    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {{\n        loop {{}}\n    }}\n\n    {stable}    pub fn display(&self) -> Display<'_> {{\n        loop {{}}\n    }}\n}}\n"
                ),
            ),
        ],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, std::slice::from_ref(&library));
    for (code, want) in [
        ("dir.join", "std/src/path.rs:7"),
        ("    prog.display", "std/src/path.rs:12"),
        ("    q.display", "std/src/path.rs:12"),
        ("    w.display", "std/src/path.rs:12"),
        ("    t.display", "picker"),
        ("leaf.join", "picker"),
    ] {
        d_on(&mut a, "src/lib.rs", code);
        let got = match shown(&mut a) {
            Shown::Jump(_, place) => place.replace(&format!("{}/", library.display()), ""),
            Shown::Picker(..) if want == "picker" => want.to_owned(),
            other @ Shown::Picker(..) => format!("{other:?}"),
        };
        assert_eq!(got, want, "{code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn a_variant_pattern_or_a_for_over_a_vec_binds_the_field_or_element() {
    let stable = "    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n";
    let lib = "pub struct Glob;\n\nimpl Glob {\n    pub fn clone(&self) -> u32 {\n        0\n    }\n\n    pub fn len(&self) -> u32 {\n        0\n    }\n}\n\npub enum Error {\n    Glob {\n        glob: Option<String>,\n        err: String,\n    },\n    Io(u32),\n}\n\nimpl Error {\n    pub fn copy(&self) -> String {\n        match *self {\n            Error::Glob { ref glob, ref err } => err.clone(),\n            Error::Io(_) => String::new(),\n        }\n    }\n\n    pub fn size(&self) -> usize {\n        if let Self::Glob { err: ref text, .. } = *self {\n            return text.len();\n        }\n        0\n    }\n}\n\npub fn total(names: Vec<String>, more: &[String]) -> usize {\n    let mut n = 0;\n    for name in &names {\n        n += name.len();\n    }\n    for each in more {\n        n += each.clone().len();\n    }\n    n\n}\n";
    let (dir, mut a) = project_app(
        "rust-variant-field",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
        ],
    );
    let std = external_root(
        "rust-variant-field-std",
        &[(
            "lib/rustlib/src/rust/library/alloc/src/string.rs",
            &format!(
                "pub struct String;\n\nimpl String {{\n{stable}    pub fn len(&self) -> usize {{\n        0\n    }}\n}}\n\nimpl Clone for String {{\n    fn clone(&self) -> Self {{\n        loop {{}}\n    }}\n}}\n"
            ),
        )],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, std::slice::from_ref(&library));
    for (code, want) in [
        ("err.clone", "alloc/src/string.rs:11"),
        ("text.len", "alloc/src/string.rs:5"),
        ("name.len", "alloc/src/string.rs:5"),
        ("each.clone", "alloc/src/string.rs:11"),
    ] {
        d_on(&mut a, "src/lib.rs", code);
        let got = match shown(&mut a) {
            Shown::Jump(_, place) => place.replace(&format!("{}/", library.display()), ""),
            other @ Shown::Picker(..) => format!("{other:?}"),
        };
        assert_eq!(got, want, "{code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn a_slice_method_on_a_vec_a_slice_or_an_array_binds_its_t_to_the_element() {
    let stable = "    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n";
    let lib = "pub struct Leaf {\n    pub name: u32,\n}\n\npub struct Cup {\n    pub name: u32,\n}\n\npub fn first_name(v: Vec<Leaf>) -> u32 {\n    v.first().unwrap().name\n}\n\npub fn sliced(s: &[Leaf]) -> u32 {\n    s.first().unwrap().name\n}\n\npub fn arrayed(r: [Leaf; 2]) -> u32 {\n    r.first().unwrap().name\n}\n\npub static LEAVES: &[Leaf] = &[];\n\npub fn leaves() -> &'static [Leaf] {\n    LEAVES\n}\n\npub fn heads(v: Vec<Leaf>) -> u32 {\n    LEAVES.first().unwrap().name + leaves().first().unwrap().name + v.as_slice().first().unwrap().name\n}\n";
    let (dir, mut a) = project_app(
        "rust-slice-t",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
        ],
    );
    let std = external_root(
        "rust-slice-t-std",
        &[
            (
                "lib/rustlib/src/rust/library/core/src/slice/mod.rs",
                &format!(
                    "impl<T> [T] {{\n{stable}    pub fn first(&self) -> Option<&T> {{\n        None\n    }}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/core/src/option.rs",
                &format!(
                    "pub enum Option<T> {{\n    None,\n    Some(T),\n}}\n\nimpl<T> Option<T> {{\n{stable}    pub fn unwrap(self) -> T {{\n        loop {{}}\n    }}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/alloc/src/vec/mod.rs",
                &format!(
                    "pub struct Vec<T>;\n\nimpl<T> Vec<T> {{\n{stable}    pub fn as_slice(&self) -> &[T] {{\n        loop {{}}\n    }}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/std/src/lib.rs",
                "pub use core::*;\n",
            ),
        ],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, std::slice::from_ref(&library));
    for code in [
        "v.first().unwrap().name",
        "s.first().unwrap().name",
        "r.first().unwrap().name",
        "LEAVES.first().unwrap().name",
        "leaves().first().unwrap().name",
        "v.as_slice().first().unwrap().name",
    ] {
        d_on(&mut a, "src/lib.rs", code);
        let got = match shown(&mut a) {
            Shown::Jump(_, place) => place,
            other @ Shown::Picker(..) => format!("{other:?}"),
        };
        assert_eq!(got, "src/lib.rs:2", "{code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn a_variant_field_typed_elsewhere_or_rebound_keeps_master_s_answer() {
    let stable = "    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n";
    let lib = "pub mod seen;\n\nuse seen::Seen;\n\npub struct Glob;\n\nimpl Glob {\n    pub fn clone(&self) -> u32 {\n        0\n    }\n\n    pub fn len(&self) -> u32 {\n        0\n    }\n}\n\npub enum Error {\n    Glob {\n        glob: Option<String>,\n        err: String,\n    },\n    Io(u32),\n}\n\npub fn key_len(s: &Seen) -> u32 {\n    match *s {\n        Seen::Dup { ref key } => key.len(),\n    }\n}\n\npub fn rebound(e: &Error, names: &[Glob]) -> u32 {\n    let err = match *e { Error::Glob { ref err, .. } => 1, Error::Io(n) => n };\n    let a = err.clone();\n    match *e {\n        Error::Glob { ref glob, .. } => names.iter().map(|glob| glob.len()).sum::<u32>() + a,\n        Error::Io(_) => 0,\n    }\n}\n";
    let (dir, mut a) = project_app(
        "rust-variant-neighbours",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
            (
                "src/seen.rs",
                "use hashbrown::String;\n\npub enum Seen {\n    Dup {\n        key: String,\n    },\n}\n",
            ),
        ],
    );
    let std = external_root(
        "rust-variant-neighbours-std",
        &[(
            "lib/rustlib/src/rust/library/alloc/src/string.rs",
            &format!(
                "pub struct String;\n\nimpl String {{\n{stable}    pub fn len(&self) -> usize {{\n        0\n    }}\n}}\n\nimpl Clone for String {{\n    fn clone(&self) -> Self {{\n        loop {{}}\n    }}\n}}\n"
            ),
        )],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, std::slice::from_ref(&library));
    for code in ["err.clone", "glob.len|()).sum", "key.len"] {
        d_on(&mut a, "src/lib.rs", code);
        let got = shown(&mut a);
        assert!(
            matches!(&got, Shown::Picker(s, rows) if s.ends_with("2 declarations") && rows.len() == 2),
            "{code}: {got:?}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn an_or_pattern_of_variants_binds_the_field_every_alternative_types_alike() {
    let stable = "    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n";
    let lib = "pub struct Glob;\n\nimpl Glob {\n    pub fn clone(&self) -> u32 {\n        0\n    }\n\n    pub fn len(&self) -> u32 {\n        0\n    }\n}\n\npub enum Error {\n    Glob {\n        glob: Option<String>,\n        err: String,\n    },\n    Parse {\n        err: String,\n    },\n    Wide {\n        err: Glob,\n    },\n    Io(u32),\n}\n\npub fn copy(e: &Error) -> String {\n    match *e {\n        Error::Glob { ref err, .. } | Error::Parse { ref err } => err.clone(),\n        _ => String::new(),\n    }\n}\n\npub fn mixed(e: &Error) -> u32 {\n    match *e {\n        Error::Parse { ref err } | Error::Wide { ref err } => err.len(),\n        Error::Glob { err: ref text, .. } | Error::Io(ref text) => text.len(),\n        _ => 0,\n    }\n}\n";
    let (dir, mut a) = project_app(
        "rust-variant-or",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
        ],
    );
    let std = external_root(
        "rust-variant-or-std",
        &[(
            "lib/rustlib/src/rust/library/alloc/src/string.rs",
            &format!(
                "pub struct String;\n\nimpl String {{\n{stable}    pub fn len(&self) -> usize {{\n        0\n    }}\n}}\n\nimpl Clone for String {{\n    fn clone(&self) -> Self {{\n        loop {{}}\n    }}\n}}\n"
            ),
        )],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, std::slice::from_ref(&library));
    d_on(&mut a, "src/lib.rs", "err.clone");
    let got = match shown(&mut a) {
        Shown::Jump(_, place) => place.replace(&format!("{}/", library.display()), ""),
        other @ Shown::Picker(..) => format!("{other:?}"),
    };
    assert_eq!(got, "alloc/src/string.rs:11");
    for code in ["err.len", "text.len"] {
        d_on(&mut a, "src/lib.rs", code);
        let got = shown(&mut a);
        assert!(
            matches!(&got, Shown::Picker(s, rows) if s.ends_with("2 declarations") && rows.len() == 2),
            "{code}: {got:?}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn an_or_pattern_split_over_lines_binds_the_field_as_on_one_line() {
    let stable = "    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n";
    let lib = "pub struct Glob;\n\nimpl Glob {\n    pub fn clone(&self) -> u32 {\n        0\n    }\n\n    pub fn len(&self) -> u32 {\n        0\n    }\n}\n\npub enum Error {\n    Glob {\n        glob: Option<String>,\n        err: String,\n    },\n    Parse {\n        err: String,\n    },\n    Wide {\n        err: Glob,\n    },\n}\n\npub fn lead(e: &Error) -> String {\n    match *e {\n        Error::Glob { err: ref one, .. }\n        | Error::Parse { err: ref one } => one.clone(),\n        _ => String::new(),\n    }\n}\n\npub fn trail(e: &Error) -> String {\n    match *e {\n        Error::Glob { err: ref two, .. } |\n        Error::Parse { err: ref two } => two.clone(),\n        _ => String::new(),\n    }\n}\n\npub fn block(e: &Error) -> String {\n    match *e {\n        Error::Glob { err: ref three, .. }\n        | Error::Parse { err: ref three } => {\n            three.clone()\n        }\n        _ => String::new(),\n    }\n}\n\npub fn mixed(e: &Error) -> u32 {\n    match *e {\n        Error::Parse { err: ref four }\n        | Error::Wide { err: ref four } => four.len(),\n        _ => 0,\n    }\n}\n";
    let (dir, mut a) = project_app(
        "rust-variant-or-lines",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
        ],
    );
    let std = external_root(
        "rust-variant-or-lines-std",
        &[(
            "lib/rustlib/src/rust/library/alloc/src/string.rs",
            &format!(
                "pub struct String;\n\nimpl String {{\n{stable}    pub fn len(&self) -> usize {{\n        0\n    }}\n}}\n\nimpl Clone for String {{\n    fn clone(&self) -> Self {{\n        loop {{}}\n    }}\n}}\n"
            ),
        )],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, std::slice::from_ref(&library));
    for code in ["one.clone", "two.clone", "three.clone"] {
        d_on(&mut a, "src/lib.rs", code);
        let got = match shown(&mut a) {
            Shown::Jump(_, place) => place.replace(&format!("{}/", library.display()), ""),
            other @ Shown::Picker(..) => format!("{other:?}"),
        };
        assert_eq!(got, "alloc/src/string.rs:11", "{code}");
    }
    d_on(&mut a, "src/lib.rs", "four.len");
    let got = shown(&mut a);
    assert!(
        matches!(&got, Shown::Picker(s, rows) if s.ends_with("2 declarations") && rows.len() == 2),
        "{got:?}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn a_tuple_or_one_line_variant_pattern_binds_the_type_at_its_place() {
    let stable = "    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n";
    let lib = "pub struct Glob;\n\nimpl Glob {\n    pub fn clone(&self) -> u32 {\n        0\n    }\n\n    pub fn len(&self) -> u32 {\n        0\n    }\n}\n\npub enum Error {\n    Io(String),\n    Pair(u32, Vec<(u8, u8)>, String),\n    Num(Glob),\n    Line { cups: Vec<(u8, u8)>, note: String },\n    Flat { flat: Glob },\n}\n\npub fn copy(e: &Error) -> String {\n    match *e {\n        Error::Io(ref err) => err.clone(),\n        Error::Line { ref note, .. } => note.clone(),\n        Error::Pair(_, _, ref s) | Error::Io(ref s) => s.clone(),\n        Error::Num(ref g) => g.clone().to_string(),\n    }\n}\n\npub fn size(e: &Error) -> u32 {\n    if let Error::Pair(n, ref pairs, ref text) = *e {\n        return n + text.len() as u32;\n    }\n    match *e {\n        Error::Num(ref g) => g.len(),\n        Error::Flat { ref flat } => flat.len(),\n        Error::Pair(.., ref tail) => tail.len() as u32,\n        Error::Pair(_, ref p) => p.len() as u32,\n        _ => 0,\n    }\n}\n";
    let (dir, mut a) = project_app(
        "rust-variant-tuple",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
        ],
    );
    let std = external_root(
        "rust-variant-tuple-std",
        &[(
            "lib/rustlib/src/rust/library/alloc/src/string.rs",
            &format!(
                "pub struct String;\n\nimpl String {{\n{stable}    pub fn len(&self) -> usize {{\n        0\n    }}\n}}\n\nimpl Clone for String {{\n    fn clone(&self) -> Self {{\n        loop {{}}\n    }}\n}}\n"
            ),
        )],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, std::slice::from_ref(&library));
    for (code, want) in [
        ("err.clone", "alloc/src/string.rs:11"),
        ("s.clone", "alloc/src/string.rs:11"),
        ("note.clone", "alloc/src/string.rs:11"),
        ("text.len", "alloc/src/string.rs:5"),
        ("g.len", "src/lib.rs:8"),
        ("flat.len", "src/lib.rs:8"),
    ] {
        d_on(&mut a, "src/lib.rs", code);
        let got = match shown(&mut a) {
            Shown::Jump(_, place) => place.replace(&format!("{}/", library.display()), ""),
            other @ Shown::Picker(..) => format!("{other:?}"),
        };
        assert_eq!(got, want, "{code}");
    }
    for code in ["tail.len", "p.len"] {
        d_on(&mut a, "src/lib.rs", code);
        let got = shown(&mut a);
        assert!(
            matches!(&got, Shown::Picker(s, rows) if s.ends_with("2 declarations") && rows.len() == 2),
            "{code}: {got:?}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn a_variant_place_is_std_s_only_when_its_written_path_is_std_s() {
    let stable = "    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n";
    let lib = "mod hb;\nuse std::collections;\n\npub struct Cache;\n\nimpl Cache {\n    pub fn get(&self, _: &u32) -> Option<&u32> {\n        None\n    }\n\n    pub fn len(&self) -> usize {\n        0\n    }\n}\n\npub enum Store {\n    Map(hashbrown::HashMap<u32, u32>),\n    Named {\n        m: hashbrown::HashMap<u32, u32>,\n    },\n    Std(std::collections::HashMap<u32, u32>),\n    Pair(HashMap<fn() -> u8, String>, Vec<u8>),\n    List(u32, Vec<String>),\n    Note {\n        text: String,\n    },\n    Tag(String),\n    Coll(collections::HashMap<u32, u32>),\n}\n\npub fn crate_map(s: Store) {\n    if let Store::Map(m) = s {\n        let _ = m.get(&1);\n    }\n}\n\npub fn crate_named(s: Store) {\n    if let Store::Named { m } = s {\n        let _ = m.get(&2);\n    }\n}\n\npub fn std_map(s: Store) {\n    if let Store::Std(m) = s {\n        let _ = m.get(&3);\n    }\n}\n\npub fn coll_map(s: Store) {\n    if let Store::Coll(m) = s {\n        let _ = m.get(&4);\n    }\n}\n\npub fn size(s: &Store) -> usize {\n    match *s {\n        Store::Pair(_, ref b) => b.len(),\n        Store::List(_, ref v) => v.len(),\n        Store::Note { ref text } | Store::Tag(ref text) => text.len(),\n        _ => 0,\n    }\n}\n";
    let (dir, mut a) = project_app(
        "rust-variant-path",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
            (
                "src/hb.rs",
                "use hashbrown::HashMap;\nuse hashbrown as collections;\n\npub enum Hb {\n    Bare(HashMap<u32, u32>),\n    Path(collections::HashMap<u32, u32>),\n}\n\npub fn bare(h: Hb) {\n    if let Hb::Bare(m) = h {\n        let _ = m.get(&5);\n    }\n}\n\npub fn path(h: Hb) {\n    if let Hb::Path(m) = h {\n        let _ = m.get(&6);\n    }\n}\n",
            ),
        ],
    );
    let std = external_root(
        "rust-variant-path-std",
        &[
            ("lib/rustlib/src/rust/library/core/src/lib.rs", "\n"),
            (
                "lib/rustlib/src/rust/library/alloc/src/string.rs",
                &format!(
                    "pub struct String;\n\nimpl String {{\n{stable}    pub fn len(&self) -> usize {{\n        0\n    }}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/alloc/src/vec/mod.rs",
                &format!(
                    "pub struct Vec<T>;\n\nimpl<T> Vec<T> {{\n{stable}    pub fn len(&self) -> usize {{\n        0\n    }}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/std/src/collections/hash/map.rs",
                &format!(
                    "pub struct HashMap<K, V>;\n\nimpl<K, V> HashMap<K, V> {{\n{stable}    pub fn get(&self, k: &K) -> Option<&V> {{\n        None\n    }}\n}}\n"
                ),
            ),
        ],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, std::slice::from_ref(&library));
    for (code, want) in [
        ("m.get|(&3)", "std/src/collections/hash/map.rs:5"),
        ("m.get|(&4)", "std/src/collections/hash/map.rs:5"),
        ("b.len", "alloc/src/vec/mod.rs:5"),
        ("v.len", "alloc/src/vec/mod.rs:5"),
        ("text.len", "alloc/src/string.rs:5"),
    ] {
        d_on(&mut a, "src/lib.rs", code);
        let got = match shown(&mut a) {
            Shown::Jump(_, place) => place.replace(&format!("{}/", library.display()), ""),
            other @ Shown::Picker(..) => format!("{other:?}"),
        };
        assert_eq!(got, want, "{code}");
    }
    for (file, code) in [
        ("src/lib.rs", "m.get|(&1)"),
        ("src/lib.rs", "m.get|(&2)"),
        ("src/hb.rs", "m.get|(&5)"),
        ("src/hb.rs", "m.get|(&6)"),
    ] {
        d_on(&mut a, file, code);
        let got = shown(&mut a);
        assert!(
            matches!(&got, Shown::Picker(s, rows) if s.ends_with("2 declarations") && rows.len() == 2),
            "{code}: {got:?}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn a_std_value_built_by_an_associated_call_heads_a_let_chain() {
    let stable = "    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n";
    let chains = "pub struct Cache;\n\nimpl Cache {\n    pub fn len(&self) -> usize {\n        0\n    }\n}\n\npub fn named() -> usize {\n    let n = String::from(\"a\");\n    let t = n.to_string();\n    t.len()\n}\n\npub fn hashed() -> usize {\n    let m = HashMap::new();\n    let k = m.clone();\n    k.len()\n}\n\npub fn qualified() -> usize {\n    let q = hashbrown::HashMap::new();\n    let r = q.clone();\n    r.len()\n}\n";
    let std = external_root(
        "rust-assoc-head-std",
        &[
            ("lib/rustlib/src/rust/library/core/src/lib.rs", "\n"),
            (
                "lib/rustlib/src/rust/library/alloc/src/string.rs",
                &format!(
                    "pub struct String;\n\nimpl String {{\n{stable}    pub fn len(&self) -> usize {{\n        0\n    }}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/std/src/collections/hash/map.rs",
                &format!(
                    "pub struct HashMap<K, V>;\n\nimpl<K, V> HashMap<K, V> {{\n{stable}    pub fn len(&self) -> usize {{\n        0\n    }}\n}}\n\nimpl<K: Clone, V: Clone> Clone for HashMap<K, V> {{\n    fn clone(&self) -> Self {{\n        loop {{}}\n    }}\n}}\n"
                ),
            ),
        ],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    for (name, head, own, want) in [
        (
            "rust-assoc-head",
            "use std::collections::HashMap;\n\n",
            "\n",
            [
                "alloc/src/string.rs:5",
                "std/src/collections/hash/map.rs:5",
                "not std",
            ],
        ),
        (
            "rust-assoc-head-namesakes",
            "mod own;\n\nuse crate::own::String;\nuse hashbrown::HashMap;\n\n",
            "pub struct String;\n",
            ["not std", "not std", "not std"],
        ),
    ] {
        let lib = format!("{head}{chains}");
        let (dir, mut a) = project_app(
            name,
            &[
                ("Cargo.toml", "[package]\nname = \"m\"\n"),
                ("src/lib.rs", &lib),
                ("src/own.rs", own),
            ],
        );
        use_roots(&mut a, Kind::Rust, std::slice::from_ref(&library));
        for (code, want) in [("t.len", want[0]), ("k.len", want[1]), ("r.len", want[2])] {
            d_on(&mut a, "src/lib.rs", code);
            let got = match shown(&mut a) {
                Shown::Jump(_, place) if place.starts_with(&*library.to_string_lossy()) => {
                    place.replace(&format!("{}/", library.display()), "")
                }
                _ => "not std".to_owned(),
            };
            assert_eq!(got, want, "{name}: {code}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn a_tuple_field_of_self_is_typed_and_a_project_alias_of_std_s_is_std_s() {
    let stable = "    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n";
    let lib = "mod fnv;\n\npub struct Cache;\n\nimpl Cache {\n    pub fn entry(&self, _: u32) {}\n}\n\nstruct Lit(fnv::HashMap<u32, u32>);\n\nimpl Lit {\n    fn add(&mut self) {\n        self.0.entry(1);\n    }\n}\n\nstruct Dep(hashbrown::HashMap<u32, u32>);\n\nimpl Dep {\n    fn add(&mut self) {\n        self.0.entry(2);\n    }\n}\n\nstruct Named {\n    m: hashbrown::HashMap<u32, u32>,\n}\n\nimpl Named {\n    fn add(&mut self) {\n        self.m.entry(3);\n    }\n}\n\nstruct Own(pub Cache, u32);\n\nimpl Own {\n    fn add(&self) {\n        self.0.entry(4);\n    }\n}\n\nstruct Other(fnv::Map<u32, u32>);\n\nimpl Other {\n    fn add(&mut self) {\n        self.0.entry(5);\n    }\n}\n\nstruct Pair(u32, std::collections::HashMap<u32, u32>);\n\nimpl Pair {\n    fn add(&mut self) {\n        self.1.entry(6);\n    }\n}\n\nstruct Wrap<T>(T);\n\nimpl<T> Wrap<T> {\n    fn add(&self) {\n        self.0.entry(7);\n    }\n}\n\nstruct Full(crate::fnv::HashMap<u32, u32>);\n\nimpl Full {\n    fn add(&mut self) {\n        self.0.entry(8);\n    }\n}\n\nuse std::collections;\n\nstruct Coll(collections::HashMap<u32, u32>);\n\nimpl Coll {\n    fn add(&mut self) {\n        self.0.entry(9);\n    }\n}\n\nstruct Dflt(fnv::Dflt);\n\nimpl Dflt {\n    fn add(&mut self) {\n        self.0.reserve(11);\n    }\n}\n";
    let fnv = "pub(crate) type HashMap<K, V> =\n    std::collections::HashMap<K, V, std::hash::BuildHasherDefault<Hasher>>;\n\npub(crate) type Map<K, V> = hashbrown::HashMap<K, V>;\n\npub(crate) struct Hasher(u64);\n\npub(crate) type Dflt<V = String> = std::collections::HashMap<String, V>;\n";
    let bare = "use hashbrown::HashMap;\n\nstruct Bare(HashMap<u32, u32>);\n\nimpl Bare {\n    fn add(&mut self) {\n        self.0.entry(10);\n    }\n}\n\nstruct Spelled(std::collections::HashMap<u32, u32>);\n\nimpl Spelled {\n    fn add(&mut self) {\n        self.0.entry(12);\n    }\n}\n";
    let (dir, mut a) = project_app(
        "rust-tuple-alias",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
            ("src/fnv.rs", fnv),
            ("src/bare.rs", bare),
        ],
    );
    let std = external_root(
        "rust-tuple-alias-std",
        &[
            ("lib/rustlib/src/rust/library/core/src/lib.rs", "\n"),
            ("lib/rustlib/src/rust/library/alloc/src/lib.rs", "\n"),
            (
                "lib/rustlib/src/rust/library/alloc/src/string.rs",
                &format!(
                    "pub struct String;\n\nimpl String {{\n{stable}    pub fn reserve(&mut self, _: usize) {{}}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/std/src/collections/hash/map.rs",
                &format!(
                    "pub struct HashMap<K, V, S>;\n\nimpl<K, V, S> HashMap<K, V, S> {{\n{stable}    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {{\n        todo!()\n    }}\n{stable}    pub fn reserve(&mut self, _: usize) {{}}\n}}\n"
                ),
            ),
        ],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, std::slice::from_ref(&library));
    let map = "std/src/collections/hash/map.rs:5";
    for (code, want) in [
        ("entry|(1)", map),
        ("entry|(4)", "src/lib.rs:6"),
        ("entry|(6)", map),
        ("entry|(8)", map),
        ("entry|(9)", map),
        ("reserve|(11)", "std/src/collections/hash/map.rs:9"),
    ] {
        d_on(&mut a, "src/lib.rs", code);
        let got = match shown(&mut a) {
            Shown::Jump(_, place) => place.replace(&format!("{}/", library.display()), ""),
            other @ Shown::Picker(..) => format!("{other:?}"),
        };
        assert_eq!(got, want, "{code}");
    }
    d_on(&mut a, "src/bare.rs", "entry|(12)");
    let got = match shown(&mut a) {
        Shown::Jump(_, place) => place.replace(&format!("{}/", library.display()), ""),
        other @ Shown::Picker(..) => format!("{other:?}"),
    };
    assert_eq!(got, map, "entry(12) under use hashbrown::HashMap");
    for (file, code) in [
        ("src/lib.rs", "entry|(2)"),
        ("src/lib.rs", "entry|(3)"),
        ("src/lib.rs", "entry|(5)"),
        ("src/lib.rs", "entry|(7)"),
        ("src/bare.rs", "entry|(10)"),
    ] {
        d_on(&mut a, file, code);
        let got = shown(&mut a);
        assert!(
            matches!(&got, Shown::Picker(s, rows) if s.contains("2 declarations") && rows.len() == 2),
            "{code}: {got:?}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn a_match_s_value_takes_the_std_type_its_typed_arms_agree_on() {
    let stable = "    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n";
    let lib = "pub struct Glob;\n\nimpl Glob {\n    pub fn clone(&self) -> u32 {\n        0\n    }\n\n    pub fn len(&self) -> u32 {\n        0\n    }\n}\n\npub fn label(note: Option<&str>) -> usize {\n    let name = String::from(\"oolong\");\n    let label = match note {\n        None => name.clone(),\n        Some(ref n) => n.to_string(),\n    };\n    label.len()\n}\n\npub fn mixed(flag: bool) -> usize {\n    let name = String::from(\"a\");\n    let list = Vec::new();\n    let both = match flag {\n        true => name.clone(),\n        false => list.clone(),\n    };\n    both.len()\n}\n\npub fn caught(note: Option<String>) -> usize {\n    let name = Vec::new();\n    let got = match note {\n        Some(name) => name.clone(),\n        None => String::new(),\n    };\n    got.len()\n}\n\npub fn blocked(note: Option<&str>) -> usize {\n    let name = String::from(\"a\");\n    let block = match note {\n        None => {\n            name.clone()\n        }\n        Some(n) => n.to_string(),\n    };\n    block.len()\n}\n\npub fn spelled(name: &str, note: Option<&str>) -> usize {\n    let text = name.to_string();\n    let spelled = match note {\n        None => text.clone(),\n        Some(n) => n.to_string(),\n    };\n    spelled.len()\n}\n";
    let (dir, mut a) = project_app(
        "rust-match-arms",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
        ],
    );
    let std = external_root(
        "rust-match-arms-std",
        &[
            ("lib/rustlib/src/rust/library/core/src/lib.rs", "\n"),
            ("lib/rustlib/src/rust/library/std/src/lib.rs", "\n"),
            (
                "lib/rustlib/src/rust/library/alloc/src/string.rs",
                &format!(
                    "pub struct String;\n\nimpl String {{\n{stable}    pub fn len(&self) -> usize {{\n        0\n    }}\n}}\n\nimpl Clone for String {{\n    fn clone(&self) -> Self {{\n        loop {{}}\n    }}\n}}\n\nimpl<T: fmt::Display + ?Sized> ToString for T {{\n    fn to_string(&self) -> String {{\n        loop {{}}\n    }}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/alloc/src/vec/mod.rs",
                &format!(
                    "pub struct Vec<T>;\n\nimpl<T> Vec<T> {{\n{stable}    pub fn len(&self) -> usize {{\n        0\n    }}\n}}\n\nimpl<T: Clone> Clone for Vec<T> {{\n    fn clone(&self) -> Self {{\n        loop {{}}\n    }}\n}}\n"
                ),
            ),
        ],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, std::slice::from_ref(&library));
    for code in ["label.len", "spelled.len"] {
        d_on(&mut a, "src/lib.rs", code);
        let got = match shown(&mut a) {
            Shown::Jump(_, place) => place.replace(&format!("{}/", library.display()), ""),
            other @ Shown::Picker(..) => format!("{other:?}"),
        };
        assert_eq!(got, "alloc/src/string.rs:5", "{code}");
    }
    for code in ["both.len", "got.len", "block.len"] {
        d_on(&mut a, "src/lib.rs", code);
        let got = shown(&mut a);
        assert!(matches!(&got, Shown::Picker(..)), "{code}: {got:?}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn a_variant_binding_of_a_registry_enum_is_typed_in_its_one_locked_version() {
    let lib = "use {\n    regex_syntax::hir::{self, Hir, HirKind},\n};\n\npub struct Near {\n    pub sub: u32,\n    pub bits: u32,\n    pub ranges: u32,\n}\n\npub struct Far {\n    pub sub: u32,\n    pub bits: u32,\n    pub ranges: u32,\n}\n\npub fn strip(h: Hir) -> u32 {\n    match h.into_kind() {\n        HirKind::Repetition(mut x) => x.sub.len(),\n        HirKind::Look(y) => y.bits,\n        HirKind::Class(z) => z.ranges,\n        _ => 0,\n    }\n}\n";
    let hir = "mod look;\nmod class;\n\nuse self::look::Look;\nuse self::class::*;\n\npub enum HirKind {\n    Empty,\n    Look(Look),\n    Repetition(Repetition),\n    Class(Class),\n}\n\npub struct Repetition {\n    pub min: u32,\n    pub sub: Box<Hir>,\n}\n\npub struct Hir;\n";
    let look = "pub struct Look {\n    pub bits: u32,\n}\n";
    let class = "pub struct Class {\n    pub ranges: u32,\n}\n";
    let reg = external_root(
        "rust-registry-variant",
        &[
            ("registry/regex-syntax-0.8.0/src/lib.rs", "pub mod hir;\n"),
            ("registry/regex-syntax-0.8.0/src/hir/mod.rs", hir),
            ("registry/regex-syntax-0.8.0/src/hir/look.rs", look),
            ("registry/regex-syntax-0.8.0/src/hir/class.rs", class),
            ("registry/regex-syntax-0.6.0/src/lib.rs", "pub mod hir;\n"),
            ("registry/regex-syntax-0.6.0/src/hir/mod.rs", hir),
            ("registry/regex-syntax-0.6.0/src/hir/look.rs", look),
            ("registry/regex-syntax-0.6.0/src/hir/class.rs", class),
        ],
    );
    let (one, two, missing) = (
        reg.join("registry/regex-syntax-0.8.0"),
        reg.join("registry/regex-syntax-0.6.0"),
        reg.join("registry/regex-syntax-0.9.0"),
    );
    let plain = "[package]\nname = \"m\"\n\n[dependencies]\nregex-syntax = \"0.8\"\n";
    let renamed = "[package]\nname = \"m\"\n\n[dependencies]\nregex-syntax = { version = \"0.8\", package = \"regex-syntax-ng\" }\n";
    for (tag, toml, roots, jumps) in [
        ("one", plain, vec![one.clone()], true),
        ("two", plain, vec![one.clone(), two], false),
        ("missing", plain, vec![missing], false),
        ("renamed", renamed, vec![one.clone()], false),
    ] {
        let (dir, mut a) = project_app(
            &format!("rust-registry-variant-{tag}"),
            &[("Cargo.toml", toml), ("src/lib.rs", lib)],
        );
        use_roots(&mut a, Kind::Rust, &roots);
        d_on(&mut a, "src/lib.rs", "x.sub");
        let got = shown(&mut a);
        if jumps {
            assert!(
                matches!(&got, Shown::Jump(s, place) if s.contains("via x: Repetition") && *place == format!("{}/src/hir/mod.rs:16", one.display())),
                "{tag}: {got:?}"
            );
        } else {
            assert!(
                matches!(&got, Shown::Picker(s, rows) if s.ends_with("2 declarations") && rows.len() == 2),
                "{tag}: {got:?}"
            );
        }
        for (code, file) in [("y.bits", "look.rs"), ("z.ranges", "class.rs")] {
            d_on(&mut a, "src/lib.rs", code);
            let got = shown(&mut a);
            if jumps {
                let want = format!("{}/src/hir/{file}:2", one.display());
                assert!(
                    matches!(&got, Shown::Jump(_, place) if *place == want),
                    "{tag} {code}: {got:?}"
                );
            } else {
                assert!(
                    matches!(&got, Shown::Picker(s, rows) if s.ends_with("2 declarations") && rows.len() == 2),
                    "{tag} {code}: {got:?}"
                );
            }
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::remove_dir_all(&reg).unwrap();
}

#[test]
fn a_registry_glob_types_a_name_only_when_no_globbed_module_imports_it() {
    let lib = "use regex_syntax::hir::kind::HirKind;\n\npub struct Near {\n    pub ranges: u32,\n}\n\npub struct Far {\n    pub ranges: u32,\n}\n\npub fn strip(k: HirKind) -> u32 {\n    match k {\n        HirKind::Value(z) => z.ranges,\n        _ => 0,\n    }\n}\n";
    let kind = "pub enum HirKind {\n    Empty,\n    Value(Value),\n}\n";
    let with_prelude = "pub mod kind;\npub mod prelude;\n";
    for (tag, hir, prelude, globs, jumps) in [
        (
            "crate",
            "pub mod kind;\n",
            "",
            "use crate::json::*;\n\n",
            true,
        ),
        (
            "parent-import",
            "pub mod kind;\n\nuse serde_json::Value;\n",
            "",
            "use super::*;\n\n",
            false,
        ),
        (
            "prelude-reexport",
            with_prelude,
            "pub use serde_json::Value;\n",
            "use super::prelude::*;\n\n",
            false,
        ),
        (
            "prelude-own-glob",
            with_prelude,
            "pub use crate::json::*;\n",
            "use super::prelude::*;\n\n",
            true,
        ),
        (
            "prelude-foreign-glob",
            with_prelude,
            "pub use serde_json::*;\n",
            "use super::prelude::*;\n\n",
            false,
        ),
        (
            "indented",
            "pub mod kind;\n",
            "",
            "mod tests {\n    use crate::json::*;\n}\n\n",
            false,
        ),
    ] {
        let kind = format!("{globs}{kind}");
        let reg = external_root(
            &format!("rust-registry-glob-{tag}"),
            &[
                (
                    "registry/regex-syntax-0.8.0/src/lib.rs",
                    "pub mod hir;\npub mod json;\n",
                ),
                (
                    "registry/regex-syntax-0.8.0/src/json.rs",
                    "pub struct Value {\n    pub ranges: u32,\n}\n",
                ),
                ("registry/regex-syntax-0.8.0/src/hir/mod.rs", hir),
                ("registry/regex-syntax-0.8.0/src/hir/kind.rs", &kind),
                ("registry/regex-syntax-0.8.0/src/hir/prelude.rs", prelude),
            ],
        );
        let one = reg.join("registry/regex-syntax-0.8.0");
        let (dir, mut a) = project_app(
            &format!("rust-registry-glob-{tag}"),
            &[
                (
                    "Cargo.toml",
                    "[package]\nname = \"m\"\n\n[dependencies]\nregex-syntax = \"0.8\"\n",
                ),
                ("src/lib.rs", lib),
            ],
        );
        use_roots(&mut a, Kind::Rust, std::slice::from_ref(&one));
        d_on(&mut a, "src/lib.rs", "z.ranges");
        let got = shown(&mut a);
        if jumps {
            let want = format!("{}/src/json.rs:2", one.display());
            assert!(
                matches!(&got, Shown::Jump(_, place) if *place == want),
                "{tag}: {got:?}"
            );
        } else {
            assert!(
                matches!(&got, Shown::Picker(s, rows) if s.ends_with("2 declarations") && rows.len() == 2),
                "{tag}: {got:?}"
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&reg).unwrap();
    }
}

#[test]
fn a_for_item_over_a_vec_takes_its_element_or_the_map_it_collects() {
    let lib = "pub struct Near;\n\nimpl Near {\n    pub fn matches(&self, s: &str) -> bool {\n        true\n    }\n}\n\npub struct Far;\n\nimpl Far {\n    pub fn matches(&self, s: &str) -> bool {\n        true\n    }\n}\n\nfn new_glob(pat: &str) -> glob::Pattern {\n    glob::Pattern::new(pat)\n}\n\nfn near(pat: glob::Pattern) -> Near {\n    Near\n}\n\npub fn count(globs: &[&str], nears: Vec<Near>) -> usize {\n    let pats: Vec<_> = globs.iter().map(|&s| new_glob(s)).collect();\n    let neared: Vec<_> = globs.iter().map(|&s| new_glob(s)).map(|p| near(p)).collect();\n    let kept: Vec<_> = globs.iter().map(|&s| new_glob(s)).filter(|p| true).collect();\n    let mut n = 0;\n    for pat in &pats {\n        n += usize::from(pat.matches(\"a\"));\n    }\n    for one in &nears {\n        n += usize::from(one.matches(\"b\"));\n    }\n    for last in neared {\n        n += usize::from(last.matches(\"c\"));\n    }\n    for k in &kept {\n        n += usize::from(k.matches(\"d\"));\n    }\n    n\n}\n";
    let glob = "pub struct Pattern {\n    original: String,\n}\n\nimpl Pattern {\n    pub fn new(p: &str) -> Pattern {\n        Pattern { original: p.to_owned() }\n    }\n\n    pub fn matches(&self, s: &str) -> bool {\n        true\n    }\n}\n";
    let reg = external_root(
        "rust-registry-for",
        &[
            ("registry/glob-0.3.4/src/lib.rs", glob),
            ("registry/glob-0.2.0/src/lib.rs", glob),
        ],
    );
    let (one, two) = (
        reg.join("registry/glob-0.3.4"),
        reg.join("registry/glob-0.2.0"),
    );
    let pattern = format!("{}/src/lib.rs:10", one.display());
    for (tag, roots, pat) in [
        ("one", vec![one.clone()], Some(pattern.as_str())),
        ("two", vec![one, two], None),
    ] {
        let (dir, mut a) = project_app(
            &format!("rust-registry-for-{tag}"),
            &[
                (
                    "Cargo.toml",
                    "[package]\nname = \"m\"\n\n[dependencies]\nglob = \"0.3\"\n",
                ),
                ("src/lib.rs", lib),
            ],
        );
        use_roots(&mut a, Kind::Rust, &roots);
        for (code, want) in [
            ("pat.matches", pat),
            ("one.matches", Some("src/lib.rs:4")),
            ("last.matches", Some("src/lib.rs:4")),
            ("k.matches", None),
        ] {
            d_on(&mut a, "src/lib.rs", code);
            let got = shown(&mut a);
            match want {
                Some(want) => assert!(
                    matches!(&got, Shown::Jump(_, place) if place == want),
                    "{tag} {code}: {got:?}"
                ),
                None => assert!(matches!(&got, Shown::Picker(..)), "{tag} {code}: {got:?}"),
            }
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::remove_dir_all(&reg).unwrap();
}

#[test]
fn a_closure_on_a_shadowing_let_s_line_does_not_bind_its_head() {
    let stable = "    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n";
    let lib = "pub struct Cache;\n\nimpl Cache {\n    pub fn len(&self) -> usize {\n        0\n    }\n}\n\npub fn trimmed(s: String) -> usize {\n    let s = s.trim_matches(|s: char| s == '-');\n    s.len()\n}\n";
    let (dir, mut a) = project_app(
        "rust-shadow-closure",
        &[
            ("Cargo.toml", "[package]\nname = \"m\"\n"),
            ("src/lib.rs", lib),
        ],
    );
    let std = external_root(
        "rust-shadow-closure-std",
        &[
            ("lib/rustlib/src/rust/library/core/src/lib.rs", "\n"),
            ("lib/rustlib/src/rust/library/std/src/lib.rs", "\n"),
            (
                "lib/rustlib/src/rust/library/alloc/src/string.rs",
                "pub struct String;\n",
            ),
            (
                "lib/rustlib/src/rust/library/core/src/str/mod.rs",
                &format!(
                    "impl str {{\n{stable}    pub fn trim_matches<P>(&self, pat: P) -> &str {{\n        self\n    }}\n\n{stable}    pub fn len(&self) -> usize {{\n        0\n    }}\n}}\n"
                ),
            ),
        ],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, std::slice::from_ref(&library));
    d_on(&mut a, "src/lib.rs", "s.len");
    let got = match shown(&mut a) {
        Shown::Jump(_, place) => place.replace(&format!("{}/", library.display()), ""),
        other @ Shown::Picker(..) => format!("{other:?}"),
    };
    assert_eq!(got, "core/src/str/mod.rs:8");
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

#[test]
fn a_std_associated_call_heads_a_chain_by_its_declared_return_type() {
    let stable = "    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n";
    let chains = "pub struct Cache;\n\nimpl Cache {\n    pub fn len(&self) -> usize {\n        0\n    }\n\n    pub fn display(&self) {}\n}\n\npub fn decoded(b: Vec<u8>) -> Result<usize, FromUtf8Error> {\n    let s = String::from_utf8(b)?;\n    Ok(s.len())\n}\n\npub fn joined(p: &str) {\n    let full = PathBuf::from(p).join(\"x\");\n    full.display();\n}\n\npub fn direct(b: Vec<u8>) -> usize {\n    String::from_utf8(b).unwrap().len()\n}\n\npub fn lossy(w: &[u16]) -> usize {\n    let t = String::from_utf16_lossy(w);\n    t.len()\n}\n";
    let std = external_root(
        "rust-assoc-chain-std",
        &[
            ("lib/rustlib/src/rust/library/core/src/lib.rs", "\n"),
            (
                "lib/rustlib/src/rust/library/alloc/src/string.rs",
                &format!(
                    "pub struct String;\n\npub struct FromUtf8Error;\n\nimpl String {{\n{stable}    pub fn len(&self) -> usize {{\n        0\n    }}\n{stable}    pub fn from_utf8(vec: Vec<u8>) -> Result<String, FromUtf8Error> {{\n        loop {{}}\n    }}\n{stable}    pub fn from_utf16_lossy(v: &[u16]) -> String {{\n        loop {{}}\n    }}\n}}\n"
                ),
            ),
            (
                "lib/rustlib/src/rust/library/std/src/path.rs",
                &format!(
                    "pub struct PathBuf;\n\npub struct Path;\n\nimpl Path {{\n{stable}    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {{\n        loop {{}}\n    }}\n{stable}    pub fn display(&self) -> Display<'_> {{\n        loop {{}}\n    }}\n}}\n"
                ),
            ),
        ],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    let len = "alloc/src/string.rs:7";
    for (name, head, own, want) in [
        (
            "rust-assoc-chain",
            "use std::path::PathBuf;\nuse std::string::FromUtf8Error;\n\n",
            "\n",
            [len, "std/src/path.rs:11", len, len],
        ),
        (
            "rust-assoc-chain-namesakes",
            "mod own;\n\nuse camino::Utf8PathBuf as PathBuf;\nuse crate::own::{FromUtf8Error, String};\n\n",
            "pub struct String;\n\npub struct FromUtf8Error;\n",
            ["not std"; 4],
        ),
    ] {
        let lib = format!("{head}{chains}");
        let (dir, mut a) = project_app(
            name,
            &[
                ("Cargo.toml", "[package]\nname = \"m\"\n"),
                ("src/lib.rs", &lib),
                ("src/own.rs", own),
            ],
        );
        use_roots(&mut a, Kind::Rust, std::slice::from_ref(&library));
        for (code, want) in [
            ("s.len", want[0]),
            ("full.display", want[1]),
            ("unwrap().len", want[2]),
            ("t.len", want[3]),
        ] {
            d_on(&mut a, "src/lib.rs", code);
            let got = match shown(&mut a) {
                Shown::Jump(_, place) if place.starts_with(&*library.to_string_lossy()) => {
                    place.replace(&format!("{}/", library.display()), "")
                }
                _ => "not std".to_owned(),
            };
            assert_eq!(got, want, "{name}: {code}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::remove_dir_all(&std).unwrap();
}
