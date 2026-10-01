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
    /// C, C++, Objective-C and Objective-C++ together, headers included.
    C,
    CSharp,
    Swift,
    Php,
    Lua,
    Elixir,
    Zig,
    /// Protocol Buffers: `.proto` schemas, never the `.textproto` data they describe.
    Proto,
    Shell,
    /// PowerShell scripts and modules; names ignore case in `d`, as the language does (#420).
    PowerShell,
    /// Dart and Flutter; `$` is a name character, `_$UserFromJson` one name (#414).
    Dart,
    Cmake,
    Sql,
    Make,
    Terraform,
    Docker,
    Yaml,
    /// `d` follows a link or a path in a code span to the file or the heading it names (#421).
    Markdown,
    Graphql,
    /// CSS, SCSS, Sass and Less: a class is styled in any of them, so one kind searches them all
    /// (#415).
    Css,
    /// `d` follows a class, an id or a path of an HTML file to the rule or the file it names
    /// (#415); an inline `<script>` is not read as JavaScript.
    Html,
}
/// The kind of a file open on screen: [`kind_of`], and an RBS signature, `hash.rbs` of Ruby's
/// core that `d` opened (#369), read as Ruby. The search never takes a `.rbs` for the project's.
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
        // A component's script calls `.ts` modules and they import components, so the two
        // search each other; only the script's lines are code (`script_lines`, #413).
        (_, "vue" | "svelte" | "astro") => Kind::TsJs,
        // Java, Kotlin and Scala are one kind: they call each other inside the same project, so
        // `d` in a `.kt` file has to find the `.java` class it uses, as `.tsx` finds `.ts`. A
        // `.sc` is a Scala script, a `.sbt` and a `.mill` the build Scala's tools read (#416).
        (_, "java" | "kt" | "kts" | "scala" | "sc" | "sbt" | "mill") => Kind::Jvm,
        (_, "rb" | "rake" | "gemspec" | "podspec" | "rbi" | "ru") => Kind::Ruby,
        // C and C++ are one kind: a header declares what a `.c` or a `.cc` defines, and either
        // language reads the other's headers, so they have to search each other. Objective-C
        // (`.m`) and Objective-C++ (`.mm`) too: C with messages, reading C headers and read from
        // them, and an Objective-C project's `.h` is this kind's already (#417).
        (_, "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" | "m" | "mm") => Kind::C,
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
        // Not `.textproto` or `.pbtxt`: the text format is data, a message written out, and
        // declares nothing.
        (_, "proto") => Kind::Proto,
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
        // A module manifest declares nothing, but `d` from its `FunctionsToExport` finds them.
        (_, "ps1" | "psm1" | "psd1") => Kind::PowerShell,
        (_, "dart") => Kind::Dart,
        ("CMakeLists.txt", _) | (_, "cmake") => Kind::Cmake,
        (_, "sql" | "psql" | "pgsql" | "mysql" | "ddl" | "dml") => Kind::Sql,
        ("Makefile" | "makefile" | "GNUmakefile", _) | (_, "mk") => Kind::Make,
        (_, "tf" | "tfvars") => Kind::Terraform,
        (_, "yml" | "yaml") => Kind::Yaml,
        // MDX writes its links as Markdown does.
        (_, "md" | "markdown" | "mdx") => Kind::Markdown,
        // `.graphqls` is a schema by convention, `.gql` the short form of either.
        (_, "graphql" | "graphqls" | "gql") => Kind::Graphql,
        (_, "css" | "scss" | "sass" | "less") => Kind::Css,
        (_, "html" | "htm") => Kind::Html,
        (_, "dockerignore") => return None,
        ("Dockerfile" | "Containerfile", _) | (_, "dockerfile" | "Dockerfile") => Kind::Docker,
        _ if name.starts_with("Dockerfile.") || name.starts_with("Containerfile.") => Kind::Docker,
        // libc++'s and libstdc++'s own headers have no extension: `c++/v1/string`,
        // `c++/13/vector` (#382).
        _ if !name.contains('.') && cpp_library(path) => Kind::C,
        _ => return None,
    })
}
/// Whether `path` is a Scala file of the Jvm kind (#416): what Scala writes differently from
/// Java and Kotlin, a body with no brace or a type parameter in `[…]`, is read only there.
pub fn scala(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| matches!(e.to_str(), Some("scala" | "sc" | "sbt" | "mill")))
}
/// Whether `path` is under a `c++/<dir>/` directory, where a C++ standard library keeps its
/// headers.
fn cpp_library(path: &Path) -> bool {
    let mut parts = path.parent().into_iter().flat_map(Path::components);
    parts.any(|c| c.as_os_str() == "c++") && parts.next().is_some()
}
pub fn word_chars(kind: Option<Kind>, address: bool) -> &'static str {
    match kind {
        Some(Kind::Terraform) if address => "-.",
        // `Get-ShopUser` is one PowerShell name, for `d` and `u` alike (#420).
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
