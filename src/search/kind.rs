//! The kinds of file merl has navigation rules for, told by the file name.

use std::path::Path;

/// A file kind with navigation rules of its own. Told by the file name, since a `Makefile` or a
/// `Dockerfile` has no extension to go by; one kind can span extensions (`.tsx` finds `.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Python,
    Go,
    Rust,
    TsJs,
    Jvm,
    Ruby,
    /// C and C++ together, headers included.
    C,
    CSharp,
    Swift,
    Php,
    Lua,
    Elixir,
    Zig,
    Shell,
    Sql,
    Make,
    Terraform,
    Docker,
    Yaml,
}
pub fn kind_of(path: &Path) -> Option<Kind> {
    let name = path.file_name()?.to_str()?;
    let ext = name.rsplit_once('.').map_or("", |(_, ext)| ext);
    Some(match (name, ext) {
        (_, "py" | "pyi") => Kind::Python,
        (_, "go") => Kind::Go,
        (_, "rs") => Kind::Rust,
        (_, "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs") => Kind::TsJs,
        // Java and Kotlin are one kind: they call each other inside the same project, so `d` in
        // a `.kt` file has to find the `.java` class it uses, as `.tsx` finds `.ts`.
        (_, "java" | "kt" | "kts") => Kind::Jvm,
        (_, "rb" | "rake" | "gemspec" | "podspec" | "rbi" | "ru") => Kind::Ruby,
        // C and C++ are one kind: a header declares what a `.c` or a `.cc` defines, and either
        // language reads the other's headers, so they have to search each other.
        (_, "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx") => Kind::C,
        // `.csx` is a C# script: the same language, run by `dotnet script`.
        (_, "cs" | "csx") => Kind::CSharp,
        (_, "swift") => Kind::Swift,
        (_, "php" | "phtml") => Kind::Php,
        (_, "lua") => Kind::Lua,
        (_, "ex" | "exs") => Kind::Elixir,
        // Not `.zon`: Zig's data format declares nothing the rules look for, and a key of a
        // build manifest is no reason to send `d` into the standard library. bat paints it
        // as Zig all the same.
        (_, "zig") => Kind::Zig,
        (
            "Rakefile" | "rakefile" | "Gemfile" | "Guardfile" | "Capfile" | "Vagrantfile"
            | "Podfile" | "Brewfile" | "Dangerfile" | "Fastfile",
            _,
        ) => Kind::Ruby,
        // Name-only, like every other kind: a shebang-only script with no extension has no kind
        // and so no rules, since `kind_of` never reads a file.
        (_, "sh" | "bash" | "zsh" | "ksh") => Kind::Shell,
        (
            ".bashrc" | ".bash_profile" | ".bash_aliases" | ".zshrc" | ".zshenv" | ".zprofile"
            | ".profile",
            _,
        ) => Kind::Shell,
        (_, "sql" | "psql" | "pgsql" | "mysql" | "ddl" | "dml") => Kind::Sql,
        ("Makefile" | "makefile" | "GNUmakefile", _) | (_, "mk") => Kind::Make,
        (_, "tf" | "tfvars") => Kind::Terraform,
        (_, "yml" | "yaml") => Kind::Yaml,
        (_, "dockerignore") => return None,
        ("Dockerfile" | "Containerfile", _) | (_, "dockerfile" | "Dockerfile") => Kind::Docker,
        _ if name.starts_with("Dockerfile.") || name.starts_with("Containerfile.") => Kind::Docker,
        _ => return None,
    })
}
/// Characters that belong to a name besides `[A-Za-z0-9_]`. Targets, services and Terraform
/// labels are often `kebab-case`; `d` in Terraform reads the whole dotted `var.region` address.
pub fn word_chars(kind: Option<Kind>, address: bool) -> &'static str {
    match kind {
        Some(Kind::Terraform) if address => "-.",
        Some(Kind::Make | Kind::Terraform | Kind::Docker | Kind::Yaml) => "-",
        _ => "",
    }
}
/// What stands between the names of a qualified name of `kind`: `Depot::open`, `Outer.find`.
pub fn separator(kind: Kind) -> &'static str {
    if matches!(kind, Kind::Rust | Kind::C | Kind::Php) {
        "::"
    } else {
        "."
    }
}
/// Whether `path` is a TypeScript declaration file: `.d.ts`, `.d.mts` or `.d.cts`.
pub fn declaration_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| [".d.ts", ".d.mts", ".d.cts"].iter().any(|e| n.ends_with(e)))
}
