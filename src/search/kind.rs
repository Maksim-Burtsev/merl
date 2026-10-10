use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Python,
    Go,
    Rust,
    TsJs,
    Jvm,
    Ruby,
    C,
    CSharp,
    Swift,
    Php,
    Lua,
    Elixir,
    Zig,
    Proto,
    Shell,
    PowerShell,
    Dart,
    Cmake,
    Nix,
    Haskell,
    Ocaml,
    Fsharp,
    Julia,
    R,
    Perl,
    Gdscript,
    Solidity,
    Clojure,
    EmacsLisp,
    Scheme,
    CommonLisp,
    Starlark,
    Sql,
    Make,
    Terraform,
    Docker,
    Yaml,
    Markdown,
    Graphql,
    Css,
    Html,
}
pub fn opened_kind(path: &Path) -> Option<Kind> {
    match path.extension() {
        Some(e) if e == "rbs" => Some(Kind::Ruby),
        _ => kind_of(path),
    }
}
pub fn kind_of(path: &Path) -> Option<Kind> {
    let name = path.file_name()?.to_str()?;
    let ext = name.rsplit_once('.').map_or("", |(_, ext)| ext);
    Some(match (name, ext) {
        (_, "py" | "pyi") => Kind::Python,
        (_, "go") => Kind::Go,
        (_, "rs") => Kind::Rust,
        (_, "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs") => Kind::TsJs,
        (_, "vue" | "svelte" | "astro") => Kind::TsJs,
        (_, "java" | "kt" | "kts" | "scala" | "sc" | "sbt" | "mill") => Kind::Jvm,
        (_, "groovy" | "gvy" | "gradle" | "jenkinsfile") | ("Jenkinsfile", _) => Kind::Jvm,
        (_, "rb" | "rake" | "gemspec" | "podspec" | "rbi" | "ru") => Kind::Ruby,
        (_, "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" | "m" | "mm") => Kind::C,
        (_, "cs" | "csx") => Kind::CSharp,
        (_, "swift") => Kind::Swift,
        (_, "php" | "phtml") => Kind::Php,
        (_, "lua") => Kind::Lua,
        (_, "ex" | "exs" | "erl" | "hrl" | "escript") => Kind::Elixir,
        (_, "zig") => Kind::Zig,
        (_, "proto") => Kind::Proto,
        (
            "Rakefile" | "rakefile" | "Gemfile" | "Guardfile" | "Capfile" | "Vagrantfile"
            | "Podfile" | "Brewfile" | "Dangerfile" | "Fastfile",
            _,
        ) => Kind::Ruby,
        (_, "sh" | "bash" | "zsh" | "ksh") => Kind::Shell,
        (
            ".bashrc" | ".bash_profile" | ".bash_aliases" | ".zshrc" | ".zshenv" | ".zprofile"
            | ".profile",
            _,
        ) => Kind::Shell,
        (_, "ps1" | "psm1" | "psd1") => Kind::PowerShell,
        (_, "dart") => Kind::Dart,
        ("CMakeLists.txt", _) | (_, "cmake") => Kind::Cmake,
        (_, "nix") => Kind::Nix,
        (_, "hs") => Kind::Haskell,
        (_, "ml" | "mli") => Kind::Ocaml,
        (_, "fs" | "fsi" | "fsx") => Kind::Fsharp,
        (_, "jl") => Kind::Julia,
        (_, "R" | "r") | (".Rprofile", _) => Kind::R,
        (_, "pl" | "pm" | "t") => Kind::Perl,
        (_, "gd") => Kind::Gdscript,
        (_, "sol") => Kind::Solidity,
        (_, "clj" | "cljs" | "cljc" | "bb") => Kind::Clojure,
        (".emacs", _) | (_, "el") => Kind::EmacsLisp,
        (_, "scm" | "ss" | "sld" | "rkt") => Kind::Scheme,
        (_, "lisp" | "cl" | "lsp" | "asd") => Kind::CommonLisp,
        ("BUILD" | "WORKSPACE" | "Tiltfile" | "BUCK", _) | (_, "bazel" | "bzl" | "star") => {
            Kind::Starlark
        }
        (_, "sql" | "psql" | "pgsql" | "mysql" | "ddl" | "dml") => Kind::Sql,
        ("Makefile" | "makefile" | "GNUmakefile", _) | (_, "mk") => Kind::Make,
        (_, "tf" | "tfvars") => Kind::Terraform,
        (_, "yml" | "yaml") => Kind::Yaml,
        (_, "md" | "markdown" | "mdx") => Kind::Markdown,
        (_, "graphql" | "graphqls" | "gql") => Kind::Graphql,
        (_, "css" | "scss" | "sass" | "less") => Kind::Css,
        (_, "html" | "htm") => Kind::Html,
        (_, "dockerignore") => return None,
        ("Dockerfile" | "Containerfile", _) | (_, "dockerfile" | "Dockerfile") => Kind::Docker,
        _ if name.starts_with("Dockerfile.") || name.starts_with("Containerfile.") => Kind::Docker,
        _ if !name.contains('.') && cpp_library(path) => Kind::C,
        _ => return None,
    })
}
pub fn scala(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| matches!(e.to_str(), Some("scala" | "sc" | "sbt" | "mill")))
}
pub fn groovy(path: &Path) -> bool {
    path.file_name().is_some_and(|n| n == "Jenkinsfile")
        || path.extension().is_some_and(|e| {
            matches!(
                e.to_str(),
                Some("groovy" | "gvy" | "gradle" | "jenkinsfile")
            )
        })
}
pub fn erlang(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| matches!(e.to_str(), Some("erl" | "hrl" | "escript")))
}
fn cpp_library(path: &Path) -> bool {
    let mut parts = path.parent().into_iter().flat_map(Path::components);
    parts.any(|c| c.as_os_str() == "c++") && parts.next().is_some()
}
pub fn word_chars(kind: Option<Kind>, address: bool) -> &'static str {
    match kind {
        Some(Kind::Terraform) if address => "-.",
        Some(
            Kind::Make
            | Kind::Terraform
            | Kind::Docker
            | Kind::Yaml
            | Kind::PowerShell
            | Kind::Css
            | Kind::Html,
        ) => "-",
        Some(Kind::Cmake) => "-.",
        Some(Kind::Nix) => "-'",
        Some(Kind::Haskell) => "'",
        Some(Kind::Ocaml | Kind::Fsharp) => "'",
        Some(Kind::R) => ".",
        Some(Kind::Clojure) => "-?!*+<>=",
        Some(Kind::EmacsLisp) => "-?!*+<>=/",
        Some(Kind::Scheme) => "-?!*+<>=/:",
        Some(Kind::CommonLisp) => "-?!*+<>=/%",
        _ => "",
    }
}
pub fn lisp(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Clojure | Kind::EmacsLisp | Kind::Scheme | Kind::CommonLisp
    )
}
pub fn separator(kind: Kind) -> &'static str {
    if matches!(kind, Kind::Rust | Kind::C | Kind::Php | Kind::Perl) {
        "::"
    } else {
        "."
    }
}
pub fn declaration_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| [".d.ts", ".d.mts", ".d.cts"].iter().any(|e| n.ends_with(e)))
}
