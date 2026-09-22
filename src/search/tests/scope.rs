//! How far a definition is looked for.

use super::*;

#[test]
fn definition_scope_follows_the_kind() {
    let here = Path::new("infra/app/main.tf");
    assert!(in_def_scope(
        Kind::Terraform,
        here,
        Path::new("infra/app/vars.tf")
    ));
    assert!(!in_def_scope(
        Kind::Terraform,
        here,
        Path::new("infra/vpc/vars.tf")
    ));
    assert!(!in_def_scope(
        Kind::Terraform,
        here,
        Path::new("infra/app/notes.md")
    ));
    let compose = Path::new("compose.yml");
    assert!(in_def_scope(Kind::Yaml, compose, compose));
    assert!(!in_def_scope(Kind::Yaml, compose, Path::new("other.yml")));
    assert!(in_def_scope(
        Kind::Make,
        Path::new("Makefile"),
        Path::new("tests/rules.mk")
    ));
    assert!(!in_def_scope(
        Kind::Make,
        Path::new("Makefile"),
        Path::new("a.py")
    ));
    // `.tsx` finds its types in `.ts` and its helpers in `.js`.
    assert!(in_def_scope(
        Kind::TsJs,
        Path::new("ui/app.tsx"),
        Path::new("lib/types.ts")
    ));
    assert!(in_def_scope(
        Kind::TsJs,
        Path::new("ui/app.tsx"),
        Path::new("e.js")
    ));
    assert!(!in_def_scope(
        Kind::Rust,
        Path::new("src/main.rs"),
        Path::new("build.py")
    ));
    // A Kotlin file finds the Java class it calls, and the other way round.
    assert!(in_def_scope(
        Kind::Jvm,
        Path::new("app/src/App.kt"),
        Path::new("lib/src/Invoice.java")
    ));
    assert!(!in_def_scope(
        Kind::Jvm,
        Path::new("app/src/App.kt"),
        Path::new("lib/src/invoice.py")
    ));
    // A Rakefile finds the class it drives in the library it loads.
    assert!(in_def_scope(
        Kind::Ruby,
        Path::new("Rakefile"),
        Path::new("lib/invoice.rb")
    ));
    // A migration finds the table it alters in whatever file created it.
    assert!(in_def_scope(
        Kind::Sql,
        Path::new("migrations/002.sql"),
        Path::new("schema.ddl")
    ));
    assert!(!in_def_scope(
        Kind::Sql,
        Path::new("migrations/002.sql"),
        Path::new("notes.md")
    ));
}
