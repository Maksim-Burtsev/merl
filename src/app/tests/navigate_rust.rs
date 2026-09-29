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
                "pub trait Clone: Sized {\n    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n    fn clone(&self) -> Self;\n}\n\nimpl Clone for u32 {\n    fn clone(&self) -> u32 {\n        *self\n    }\n}\n",
            ),
            (
                "lib/rustlib/src/rust/library/alloc/src/string.rs",
                "pub trait ToString {\n    fn to_string(&self) -> String;\n}\n\nimpl<T: fmt::Display + ?Sized> ToString for T {\n    default fn to_string(&self) -> String {\n        String::new()\n    }\n}\n",
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
                "core/src/clone.rs:3",
            ),
        ),
        (
            "s.to_string",
            jump(
                "to_string \u{2192} ToString::to_string (via trait ToString)",
                "alloc/src/string.rs:2",
            ),
        ),
        (
            "e.describe",
            jump(
                "describe \u{2192} Error::describe (by name, 1 match)",
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
