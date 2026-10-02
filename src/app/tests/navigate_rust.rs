//! `d` in Rust against a standard library and dependencies on disk: what the fixtures, which
//! have none, cannot show.

use super::*;

/// #370: a word inside an attribute is a macro outside the project or nothing, never a project
/// item of its name; a field access with nothing in the project reads the `pub` fields outside.
#[test]
fn attributes_name_macros_and_fields_reach_outside() {
    let (dir, mut a) = project_app(
        "rust-attr",
        &[
            ("Cargo.toml", "[package]\nname = \"repro\"\n"),
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
            // Nothing found leaves the cursor where it was.
            Shown::Jump(status, _) if status.starts_with("no definition") => {
                Shown::Jump(status, String::new())
            }
            Shown::Jump(status, place) => {
                Shown::Jump(status, place.replace(&format!("{}/", std.display()), ""))
            }
            other => other,
        };
        assert_eq!(got, want, "{code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}

/// #358: `x.method()` of a type `d` does not know is every method of the name the cursor can
/// reach, in the project, the standard library and the dependencies, and the one trait's method
/// when every candidate declares or implements it.
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
                "mod extra;\n\npub struct Error {\n    msg: String,\n}\n\nimpl Clone for Error {\n    fn clone(&self) -> Error {\n        Error { msg: self.msg.clone() }\n    }\n}\n\nimpl Error {\n    fn describe(&self) -> u32 {\n        1\n    }\n}\n\npub fn first(v: Option<u32>) -> u32 {\n    v.unwrap()\n}\n\npub fn name(n: &String) -> String {\n    n.clone()\n}\n\npub fn text(s: &str) -> String {\n    s.to_string()\n}\n\npub fn told(e: &Error, x: &extra::Extra) -> u32 {\n    x.secret();\n    e.describe()\n}\n\npub fn peeked(v: Option<u32>) {\n    v.peek()\n}\n",
            ),
            (
                "src/extra.rs",
                "pub struct Extra;\n\nimpl Extra {\n    fn secret(&self) {}\n}\n",
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
                "#[stable(feature = \"rust1\", since = \"1.0.0\")]\npub trait Clone: Sized {\n    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n    fn clone(&self) -> Self;\n}\n\nimpl Clone for u32 {\n    fn clone(&self) -> u32 {\n        *self\n    }\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/alloc/src/string.rs",
                "#[stable(feature = \"rust1\", since = \"1.0.0\")]\npub trait ToString {\n    fn to_string(&self) -> String;\n}\n\nimpl<T: fmt::Display + ?Sized> ToString for T {\n    default fn to_string(&self) -> String {\n        String::new()\n    }\n}\n",
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
        ],
    );
    let library = std.join("lib/rustlib/src/rust/library");
    use_roots(&mut a, Kind::Rust, &[library.clone(), registry.clone()]);
    let row = |name: &str, place: &str| (name.to_owned(), "by name".to_owned(), place.to_owned());
    for (code, want) in [
        (
            "v.unwrap",
            Shown::Picker(
                "unwrap: by name, 3 declarations".into(),
                vec![
                    row("Option::unwrap", "core/src/option.rs:8"),
                    row("Result::unwrap", "core/src/result.rs:8"),
                    row("Thing::unwrap", "dep-1.0.0/src/lib.rs:4"),
                ],
            ),
        ),
        (
            "n.clone",
            jump(
                "clone \u{2192} Clone::clone (via trait Clone)",
                "core/src/clone.rs:4",
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
            "v.peek",
            Shown::Jump("no definition for peek".into(), String::new()),
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

/// #529: a struct literal's key lands on the field of the literal's type, the project's or a
/// dependency's, and never on a method of the name.
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
    assert_eq!(at(&a), (dir.join("src/lib.rs"), 3), "{}", a.message);
    d_on(&mut a, "src/lib.rs", "Opts { depth|: 2");
    assert_eq!(at(&a), (knobs.join("src/lib.rs"), 1), "{}", a.message);
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&registry).unwrap();
}

/// #350: the first name of a path names the crate searched first, a `use` of the file or the
/// path written out, and in it the module the path spells; a project namesake found by name is
/// no answer then.
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
    for (code, place, status) in [
        (
            "File::open|(p)",
            lib("std/src/fs.rs"),
            4,
            "via import std::fs",
        ),
        ("-> io|::Result", lib("std/src/lib.rs"), 2, "via import std"),
        (
            "    norm|(1)",
            dir.join("src/helpers.rs"),
            1,
            "via import crate::helpers",
        ),
        (
            "_e: io::Error",
            lib("std/src/io/error.rs"),
            3,
            "via import std::io",
        ),
        (
            "m: Match",
            dir.join("crates/matcher/src/lib.rs"),
            1,
            "via import grep_matcher",
        ),
        ("-> std::fs|::File", lib("std/src/lib.rs"), 1, "via std"),
        (
            "std::fs::File::create",
            lib("std/src/fs.rs"),
            5,
            "via std::fs",
        ),
        (
            "std::sync::Arc",
            lib("alloc/src/sync.rs"),
            1,
            "via alloc::sync",
        ),
        (
            "usize::MAX",
            lib("core/src/num/uint_macros.rs"),
            3,
            "by name",
        ),
        // A glob `use` of the block hides the file's `use` of the name.
        (
            "        Match|(n)",
            dir.join("src/lib.rs"),
            51,
            "Kind::Match",
        ),
        (
            "dyn StdError",
            lib("core/src/error.rs"),
            1,
            "via import core::error",
        ),
        (
            "memmap2::Mmap::map",
            memmap.join("src/lib.rs"),
            4,
            "via memmap2",
        ),
    ]
    .map(|(c, p, l, s)| (c, (p, l), s))
    {
        d_on(&mut a, "src/lib.rs", code);
        let (path, line) = at(&a);
        assert_eq!((path, line + 1), place, "{code}: {}", a.message);
        assert!(a.message.contains(status), "{code}: {}", a.message);
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&std).unwrap();
}
