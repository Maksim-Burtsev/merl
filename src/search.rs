//! Project-wide grep (ripgrep's own crates) and the regex rules behind "go to definition",
//! the symbol list and the word under the cursor.
//!
//! There is no language server here: a definition is whatever a per-kind line pattern says it
//! is, searched in the project first and then in the standard library and the installed
//! dependencies the toolchain on this machine knows about.

use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use grep_searcher::{BinaryDetection, Searcher, SearcherBuilder, Sink, SinkMatch};
use regex::Regex;

/// ponytail: a hard stop instead of a streaming picker, taken in file order, so the picker
/// title says "first N" when it hits. Raise it if a picker over the whole result set ever
/// becomes the point.
pub const MAX_HITS: usize = 5_000;

/// Lines that look like a declaration in code. `const` and `static` count only unindented or
/// behind `export` / `pub`: indented and bare, they are the locals of a function body. The group
/// before the name swallows a Go method receiver (`func (i Invoice) Total()`).
pub const SYMBOL_PATTERN: &str = r#"^(?:\s*(?:(?:export|default|declare|async|pub(?:\([a-z]+\))?|static|unsafe|abstract|const|extern(?:\s+"[^"]*")?)\s+)*(?:def|class|func|function\*?|type|fn|struct|enum|impl|trait|interface|mod|union|macro_rules!|namespace)|(?:\s*(?:export|pub(?:\([a-z]+\))?)\s+)?(?:declare\s+)?(?:const|static))(?:<[^>]*>)?\s+(\([^)]*\)\s*)?(?P<name>[A-Za-z_]\w*)"#;

/// The head of a SQL `CREATE` statement, up to the name it declares: the optional `OR REPLACE`,
/// the modifiers that can sit before the object word, the object itself and `IF NOT EXISTS`.
/// A macro, so the symbol pattern below and [`def_patterns`] share one spelling of it.
macro_rules! sql_create {
    () => {
        r"(?i)^\s*CREATE\s+(?:OR\s+REPLACE\s+)?(?:(?:TEMP|TEMPORARY|UNLOGGED|MATERIALIZED|UNIQUE)\s+)*(?:TABLE|VIEW|INDEX|FUNCTION|PROCEDURE|TRIGGER|TYPE|SCHEMA|SEQUENCE|DOMAIN|EXTENSION|DATABASE|ROLE|USER|ENUM)(?:\s+IF\s+NOT\s+EXISTS)?\s+"
    };
}

/// Everything that can stand before a declaration in Java or Kotlin: annotations, Java's access
/// and class modifiers, Kotlin's own. The argument is the repetition the run takes: `"*"` for a
/// rule that reads them if they are there, `"+"` for one that needs at least one of them. A
/// macro, so [`def_patterns`] and the [`SYMBOLS`] rows share one spelling of it.
macro_rules! jvm_mods {
    () => {
        jvm_mods!("*")
    };
    ($rep:literal) => {
        concat!(
            r"^\s*(?:@[\w.]+(?:\([^)]*\))?\s+)*",
            r"(?:(?:public|protected|private|internal|static|final|abstract|sealed|non-sealed",
            r"|strictfp|synchronized|native|default|transient|volatile|open|data|value|inner",
            r"|annotation|companion|enum|const|lateinit|expect|actual|suspend|override|inline",
            r"|operator|infix|tailrec|external|reified)\s+)",
            $rep
        )
    };
}

/// A Java return type: a primitive, or a name with a capital in it. Java names its types that
/// way, and requiring one keeps `return parse(x);` from reading as a declaration. The generics
/// nest one level, for a `Map<String, List<Integer>>`.
macro_rules! jvm_return_type {
    () => {
        concat!(
            r"(?:void|int|long|short|byte|char|boolean|float|double|[\w.]*[A-Z][\w.]*)",
            r"(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?(?:\[\])*"
        )
    };
}

/// The Java and Kotlin half of [`SYMBOLS`]: what either language declares with a keyword.
/// `fun interface` stands before `fun`, so a Kotlin functional interface is listed under its own
/// name; `companion object` has no name and falls out on the `\s+` before it. A `val` counts
/// behind `const` only: the rest are fields and locals, which no kind lists.
const JVM_DECL_SYMBOL: &str = concat!(
    jvm_mods!(),
    r"(?:class|interface|enum|record|typealias|@interface|object",
    r"|fun\s+interface|fun|const\s+(?:val|var))\s+(?:<[^>]*>\s*)?",
    r"(?:[\w.]+(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?\??\.)?",
    r"(?P<name>[A-Za-z_]\w*)"
);

/// The other Java half: a method, told from a call by the return type before its name. Kotlin
/// writes its types after the name, so nothing of Kotlin's lands here twice. A field is left out,
/// as in every other kind.
const JAVA_METHOD_SYMBOL: &str = concat!(
    jvm_mods!(),
    r"(?:<[^>]*>\s*)?",
    jvm_return_type!(),
    r"\s+(?P<name>[A-Za-z_]\w*)\s*\("
);

/// Everything C and C++ can write between the start of a declaration and its keyword: a template
/// head, the storage specifiers, and the attribute or export macro a library puts there
/// (`class FMT_API name`, `struct __attribute__ ((__packed__)) sdshdr8`). The macro's arguments
/// nest one level. A macro, so [`def_patterns`] and the [`SYMBOLS`] rows share one spelling of it.
macro_rules! c_mods {
    () => {
        concat!(
            r"(?:template\s*<[^>]*>\s*)?",
            r"(?:(?:static|extern|const|inline|constexpr|thread_local)\s+)*"
        )
    };
    (macros) => {
        r"(?:(?:[A-Z][A-Z0-9_]*|__\w+)\s*(?:\((?:[^()]|\([^()]*\))*\))?\s+)*"
    };
}

/// The C and C++ half of [`SYMBOLS`], first half: a function at the top level, where the language
/// has no statements, so anything shaped like a declaration is one. A line that ends its
/// parameters with a `;` is a prototype and is left out — every function of a header would be
/// listed twice — which leaves the definition, brace on the line or on the next one, and the
/// signature that wraps. The return type may sit on the line above, as GNU style writes it, so
/// the run before the name is optional here. A name opening with two underscores is the
/// implementation's, not the project's, and sits exactly where a function name would
/// (`struct __attribute__ ((__packed__)) sdshdr8 {`), so it is left out.
const C_FUNC_SYMBOL: &str = r"^(?:\w[^;(){}=]*[\s*&:])?(?P<name>_?[A-Za-z0-9]\w*)\s*\([^;]*$";

/// The second half: a method in a class body or a function in an indented namespace, told from a
/// call by the body it opens on the line. Indented, so it never lists what [`C_FUNC_SYMBOL`] does.
const C_METHOD_SYMBOL: &str =
    r"^\s+[^;(){}=]*\w[\s*&]+(?P<name>[A-Za-z_]\w*)\s*\([^;{}]*\)[^;{}=]*\{";

/// A type, a namespace and a C++ `using` alias. What follows the name keeps `struct dict *d;` out;
/// a `<` is a template specialization (`struct formatter<path, Char> {`), and a lone `:` a base
/// list, where the `::` of a `using a::b;` names an imported symbol, not a declared one. A
/// `typedef struct name { … }` is listed from the line it closes on instead, under the name the
/// project uses.
const C_TYPE_SYMBOL: &str = concat!(
    r"^\s*",
    c_mods!(),
    r"(?:struct|class|union|enum\s+class|enum\s+struct|enum|namespace|using)\s+",
    c_mods!(macros),
    r"(?P<name>[A-Za-z_]\w*)\s*(?:[{=<]|:[^:]|final\b|$)"
);

/// The name a `typedef` or a `} name;` gives a type. The closing brace is in column zero: an
/// indented one closes a nested anonymous struct, and that name is a field. A global stays off the
/// list, as a field does in every other kind.
const C_TYPEDEF_SYMBOL: &str =
    r"^(?:\s*typedef\s+[^;]*?|\}\s*[\w\s,*]*)\b(?P<name>[A-Za-z_]\w*)\s*(?:\[[^\]]*\])?\s*;\s*$";

/// An object- or function-like macro.
const C_MACRO_SYMBOL: &str = r"^\s*#\s*define\s+(?P<name>[A-Za-z_]\w*)";

/// Everything that can stand before a C# declaration: the attribute lists written on the same
/// line (`[Fact] public void …`) and the modifiers, which come in any order. The argument is the
/// repetition the run takes: `"*"` for a rule that reads them if they are there, `"+"` for one
/// that needs at least one. A macro, so [`def_patterns`] and the [`SYMBOLS`] rows share one
/// spelling of it.
macro_rules! cs_mods {
    () => {
        cs_mods!("*")
    };
    ($rep:literal) => {
        concat!(
            r"^\s*(?:\[[^\]]*\]\s*)*",
            r"(?:(?:public|private|protected|internal|file|static|readonly|const|sealed|abstract",
            r"|virtual|override|partial|async|extern|unsafe|new|volatile|event|required|fixed",
            r"|implicit|explicit|ref)\s+)",
            $rep
        )
    };
    // A constructor is told from a call by its modifiers alone, so its run is the access ones
    // only: `new` is a member modifier too, and a bare `new Invoice(id)` must not read as one.
    (access) => {
        concat!(
            r"^\s*(?:\[[^\]]*\]\s*)*",
            r"(?:(?:public|private|protected|internal|static|unsafe|extern|partial)\s+)+"
        )
    };
}

/// A C# type as it stands before the name it declares: a predefined type, `var`, or a name with a
/// capital in it, which is how C# names its types — the same trick Java's rules use, and what
/// keeps `return Compute(x);` from reading as a declaration. A tuple type counts too, and needs
/// the comma it is written with: without it, `if (x) Run();` would read as a declaration of `Run`.
/// Generics nest one level, and the nullable `?`, the array `[]` and a qualified name all count.
macro_rules! cs_type {
    () => {
        concat!(
            r"(?:void|var|bool|byte|sbyte|char|decimal|double|float|int|uint|long|ulong|short",
            r"|ushort|object|string|dynamic|nint|nuint|\([^()]*,[^()]*\)|[\w.]*[A-Z][\w.]*)",
            cs_generics!(),
            r"\??(?:\[[,\s]*\])*\??"
        )
    };
}

/// A generic argument list, nesting three levels: a regex counts no brackets, and
/// `Task<ActionResult<QueryResult<T>>>` is what an ASP.NET controller action returns.
macro_rules! cs_generics {
    () => {
        r"(?:<[^<>]*(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>[^<>]*)*>)?"
    };
}

/// The C# half of [`SYMBOLS`], first half: what the language declares with a keyword. A
/// `delegate` carries its return type between the keyword and the name, and a `namespace` is
/// listed under its last part, the one `d` finds it by.
const CS_DECL_SYMBOL: &str = concat!(
    cs_mods!(),
    r"(?:(?:class|struct|interface|enum|record\s+class|record\s+struct|record)\s+",
    r"|delegate\s+",
    cs_type!(),
    r"\s+|namespace\s+(?:[\w.]+\.)?)",
    r"(?P<name>[A-Za-z_]\w*)"
);

/// The other C# half: a member with no keyword at all, told from a call by the type before its
/// name — a method by the `(` of its parameters, a property by the `{` of its accessors, the `=>`
/// of its expression body or the end of the line, where they open on the next one. A field
/// (`… name;`, `… name = 1;`) is left out, as in every other kind, and so is a constructor, which
/// is listed under its class.
const CS_MEMBER_SYMBOL: &str = concat!(
    cs_mods!(),
    cs_type!(),
    r"\s+(?:[\w.]+\.)?(?P<name>[A-Za-z_]\w*)\s*(?:<[^<>]*>\s*)?(?:\(|\{|=>|$)"
);

/// Everything that can stand before a Swift declaration: its attributes and property wrappers,
/// and the modifiers, which come in any order. `class` is one of them — `class func load()` is
/// Swift's static method — and the keyword alternations below read past it. The argument is the
/// repetition the run takes, as [`cs_mods`]. A macro, so [`def_patterns`] and the [`SYMBOLS`] row
/// share one spelling of it.
macro_rules! swift_mods {
    () => {
        swift_mods!("*")
    };
    ($rep:literal) => {
        concat!(
            r"^\s*(?:@[\w.]+(?:\([^)]*\))?\s+)*",
            r"(?:(?:public|private|fileprivate|internal|open|package|static|class|final|override",
            r"|mutating|nonmutating|required|convenience|lazy|weak|unowned|dynamic|indirect",
            r"|optional|prefix|postfix|infix|nonisolated|distributed|borrowing|consuming)",
            // `private(set)`: the setter's own access, the one place Swift parenthesises a
            // modifier. Without it the run stops at the `(` and the declaration is never read.
            r"(?:\(set\))?\s+)",
            $rep
        )
    };
}

/// The Swift half of [`SYMBOLS`]: what the language declares with a keyword. An `extension` is
/// listed under the type it extends, since that is where a project keeps its own members of one —
/// often the only place, when the type itself comes from a framework. A `let`, a `var` and an
/// `enum` case are what a type holds, which no kind lists, and an `init` is listed under its type.
const SWIFT_DECL_SYMBOL: &str = concat!(
    swift_mods!(),
    r"(?:class|struct|enum|protocol|actor|extension|typealias|associatedtype|func)\s+",
    r"`?(?P<name>[A-Za-z_]\w*)"
);

/// Everything that can stand before a PHP declaration: its attributes and the modifiers a class
/// member carries. The argument is the repetition the run takes, as [`cs_mods`]. A macro, so
/// [`def_patterns`] and the [`SYMBOLS`] row share one spelling of it.
macro_rules! php_mods {
    () => {
        php_mods!("*")
    };
    ($rep:literal) => {
        concat!(
            r"^\s*(?:#\[[^\]]*\]\s*)*",
            r"(?:(?:public|private|protected|static|final|abstract|readonly|var)\s+)",
            $rep
        )
    };
}

/// The PHP half of [`SYMBOLS`], first half: what the language declares with a keyword other than
/// `function`, behind the modifiers a member carries. A namespace is listed under its last part,
/// the one `d` finds it by. A property is a field, which no kind lists, and an `enum` case is what
/// a type holds, as in every other kind; `define('X', …)` has no keyword before the name and is
/// left out with them.
const PHP_DECL_SYMBOL: &str = concat!(
    php_mods!(),
    r"(?:(?:class|interface|trait|enum)\s+|const\s+|namespace\s+(?:[\w\\]+\\)?)",
    r"(?P<name>[A-Za-z_]\w*)"
);

/// The other half: a function or a method. A row of its own because a project holds far more of
/// them than types and [`MAX_HITS`] is counted per row — one shared row would let the methods of
/// the first files crowd every later class off the list. A name opening with two underscores is
/// the language's own hook rather than the project's (`__construct`, `__toString`), and is left
/// out the way C leaves out the implementation's names.
const PHP_FUNC_SYMBOL: &str = concat!(php_mods!(), r"function\s+&?\s*(?P<name>_?[A-Za-z0-9]\w*)");

/// The Ruby half of [`SYMBOLS`]: a method, including the `self.` form and the `name=` setter, and
/// a class or module under the namespace it is written with. A constant and the names an
/// `attr_accessor` line declares stay off the list: there is no keyword to go by, and one such
/// line can declare several.
const RUBY_SYMBOL: &str =
    r"^\s*(?:def\s+(?:self\.|[A-Z]\w*\.)?|(?:class|module)\s+(?:[\w:]*::)?)(?P<name>[A-Za-z_]\w*)";

/// The Lua half of [`SYMBOLS`], first half: a function written with the keyword, `local` and the
/// table it hangs off included. The table prefix is dropped, as Ruby's `def self.parse` and an
/// out-of-line C++ definition are, so the row is listed under the name the language calls it by.
const LUA_FUNCTION_SYMBOL: &str =
    r"^\s*(?:local\s+)?function\s+(?:[\w.]+[.:])?(?P<name>[A-Za-z_]\w*)\s*\(";

/// The second half: a function literal bound to a name, the other way Lua writes a declaration —
/// `M.name = function(`, and the `name = function(` of a table of handlers. A value that is not a
/// function is left out: `name = 1` in a table constructor and a re-assignment inside a body are
/// the same line, and Lua has no keyword to tell them apart.
const LUA_ASSIGNED_SYMBOL: &str =
    r"^\s*(?:local\s+)?(?:[\w.]+[.:])?(?P<name>[A-Za-z_]\w*)\s*=\s*function\b";

/// The Elixir half of [`SYMBOLS`]: a module, a protocol and every `def` form, under the name
/// alone — a `defmodule A.B.C` is listed as `C`, the way Ruby's `class A::B` is. A `defimpl` is
/// left out: it declares the module `Protocol.Type`, and neither the protocol nor the type is a
/// name of its own there, as a Rust `impl` is not. A `defstruct` is left out too, since one line
/// declares every field, and so is a module attribute: `@doc`, `@spec` and `@moduledoc` are the
/// language's own and would fill the list.
const ELIXIR_SYMBOL: &str = concat!(
    r"^\s*def(?:(?:module|protocol)\s+(?:[\w.]+\.)?|(?:p|macro|macrop|guard|guardp|delegate)?\s+)",
    r"(?P<name>[A-Za-z_]\w*[!?]?)"
);

/// The Zig half of [`SYMBOLS`], first half: what the shared pattern has no word for. Zig declares
/// with `fn` and `const`, which [`SYMBOL_PATTERN`] already reads, so this row adds only the
/// function behind `inline` or `noinline` — modifiers that pattern's run does not know. A `var` is
/// a global, and globals stay off the list, as in every other kind.
const ZIG_INLINE_FN_SYMBOL: &str = concat!(
    r#"^\s*(?:(?:pub|export|extern(?:\s+"[^"]*")?)\s+)*"#,
    r"(?:inline|noinline)\s+fn\s+(?P<name>[A-Za-z_]\w*)"
);

/// The second half: a test, under the description it is written with. Half a Zig file is its
/// tests, and the description is the only name one has. [`def_patterns`] has no rule for it, so
/// `d` can never jump to a test: a word inside a description declares nothing.
const ZIG_TEST_SYMBOL: &str = r#"^\s*test\s+"(?P<name>[^"]*)""#;

/// The module attributes Elixir and the libraries everyone uses give a meaning to, rather than a
/// project. They are directives, so [`def_patterns`] has no rule for the name itself. A list of
/// known names is all a line pattern can have here: any library may define an attribute, and
/// `@tag :slow` and `@timeout 5_000` are the same line.
const ELIXIR_DIRECTIVES: &[&str] = &[
    // ExUnit and Mix, which every project in the language meets.
    "describetag",
    "endpoint",
    "moduletag",
    "shortdoc",
    "switches",
    "tag",
    "after_compile",
    "before_compile",
    "behaviour",
    "callback",
    "compile",
    "deprecated",
    "derive",
    "dialyzer",
    "doc",
    "enforce_keys",
    "external_resource",
    "file",
    "impl",
    "macrocallback",
    "moduledoc",
    "on_definition",
    "on_load",
    "opaque",
    "optional_callbacks",
    "spec",
    "type",
    "typedoc",
    "typep",
    "vsn",
];

/// A name in a `CREATE` statement, as written: bare, `"quoted"` or `` `backticked` ``, and
/// optionally schema-qualified (`public.orders`).
const SQL_NAME: &str = r#"(?:"[^"]+"|`[^`]+`|\w+)"#;

/// The SQL half of [`SYMBOLS`]: every `CREATE`d object, listed under its written name.
const SQL_CREATE_SYMBOL: &str = concat!(
    sql_create!(),
    r#"(?P<name>(?:"[^"]+"|`[^`]+`|\w+)(?:\.(?:"[^"]+"|`[^`]+`|\w+))?)"#
);

/// What `D` lists: a line pattern and the kind of file it runs over (`None`: every file
/// [`shared_symbols`] reads it from). The infrastructure patterns need their kind, since a
/// Makefile target and a YAML key are the same shape. [`symbol_name`] reads the listed name from
/// the named groups.
pub const SYMBOLS: &[(Option<Kind>, &str)] = &[
    (None, SYMBOL_PATTERN),
    // `name ()`, the form without the `function` keyword: a `function name` line is already
    // listed by [`SYMBOL_PATTERN`], and requiring no keyword here keeps it off the list twice.
    (Some(Kind::Shell), r"^\s*(?P<name>[A-Za-z_]\w*)\s*\(\s*\)"),
    // Every `CREATE` object, with the name as written, schema and quotes included. CTEs are a
    // query's own scaffolding, not a symbol of the project, so they are left out.
    (Some(Kind::Sql), SQL_CREATE_SYMBOL),
    // Java and Kotlin are listed from their own rows only: their modifiers, annotations and
    // receivers are not the shared pattern's business, and a method carries no keyword at all.
    (Some(Kind::Jvm), JVM_DECL_SYMBOL),
    (Some(Kind::Jvm), JAVA_METHOD_SYMBOL),
    // Ruby likewise: `def self.parse` is `parse`, which the shared pattern would call `self`.
    (Some(Kind::Ruby), RUBY_SYMBOL),
    // C and C++ likewise: a function carries no keyword at all, and `struct dict *d;` is a use of
    // a type the shared pattern would list as its declaration.
    (Some(Kind::C), C_FUNC_SYMBOL),
    (Some(Kind::C), C_METHOD_SYMBOL),
    (Some(Kind::C), C_TYPE_SYMBOL),
    (Some(Kind::C), C_TYPEDEF_SYMBOL),
    (Some(Kind::C), C_MACRO_SYMBOL),
    // C# likewise: `public sealed partial class Foo<T>` stands behind modifiers the shared
    // pattern does not know, and a method or a property carries no keyword at all.
    (Some(Kind::CSharp), CS_DECL_SYMBOL),
    (Some(Kind::CSharp), CS_MEMBER_SYMBOL),
    // Swift likewise: a declaration stands behind its attributes and modifiers, and `extension`,
    // `protocol` and `actor` are no keywords of the shared pattern.
    (Some(Kind::Swift), SWIFT_DECL_SYMBOL),
    // PHP likewise: a method stands behind `final public static`, which the shared pattern does
    // not read, so `function` alone would be the only form it listed.
    (Some(Kind::Php), PHP_DECL_SYMBOL),
    (Some(Kind::Php), PHP_FUNC_SYMBOL),
    // Lua likewise: the shared pattern reads `function M.name(` as a declaration of `M`, and has
    // no word for `local function` at all.
    (Some(Kind::Lua), LUA_FUNCTION_SYMBOL),
    (Some(Kind::Lua), LUA_ASSIGNED_SYMBOL),
    // Elixir likewise: the shared pattern knows `def` and nothing else of the family, and reads
    // the `x` of an anonymous `fn x -> …` as a declaration.
    (Some(Kind::Elixir), ELIXIR_SYMBOL),
    // Zig fits the shared pattern — it declares with `fn` and `const` — so these two rows only
    // complement it, the way Shell's and SQL's do.
    (Some(Kind::Zig), ZIG_INLINE_FN_SYMBOL),
    (Some(Kind::Zig), ZIG_TEST_SYMBOL),
    // A target: not `.PHONY`-style special targets, `%` pattern rules or `:=` / `::=`.
    (
        Some(Kind::Make),
        r"^(?P<name>[A-Za-z0-9_][\w./-]*)(\s+[\w./-]+)*\s*::?([^=:]|$)",
    ),
    (
        Some(Kind::Terraform),
        r#"^(?P<block>resource|data|module|variable|output)\s+"(?P<a>[^"]+)"(\s+"(?P<b>[^"]+)")?"#,
    ),
    (
        Some(Kind::Docker),
        r"(?i)^\s*FROM\s+(\S+\s+)+AS\s+(?P<name>[\w.-]+)",
    ),
    (Some(Kind::Yaml), r"(^|\s)&(?P<anchor>[\w.-]+)"),
];

/// Whether [`SYMBOL_PATTERN`] is read from a file of `kind`. Java, Kotlin, Ruby, C, C++, C#,
/// Swift, PHP, Lua and Elixir have rows of their own in [`SYMBOLS`], written for what those
/// languages declare and how they name it, so reading the all-language pattern over them too
/// would list a declaration twice.
pub fn shared_symbols(kind: Option<Kind>) -> bool {
    !matches!(
        kind,
        Some(
            Kind::Jvm
                | Kind::Ruby
                | Kind::C
                | Kind::CSharp
                | Kind::Swift
                | Kind::Php
                | Kind::Lua
                | Kind::Elixir
        )
    )
}

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

/// One matching line. `path` is relative to the project root, `line` is 1-based.
#[derive(Debug, Clone)]
pub struct Hit {
    pub path: PathBuf,
    pub line: usize,
    pub text: String,
}

/// Greps `pattern` over `files` (paths relative to `root`).
///
/// `current` is the file the cursor is in; its hits sort first, everything else by path and
/// line. `unsaved` is its text when that is ahead of the disk, searched in place of the file.
/// Files that cannot be read are skipped — this is a viewer, not a linter.
pub fn grep_project(
    root: &Path,
    files: &[PathBuf],
    pattern: &str,
    whole_word: bool,
    smart_case: bool,
    current: Option<&Path>,
    unsaved: Option<&[u8]>,
) -> Result<Vec<Hit>> {
    let matcher = RegexMatcherBuilder::new()
        .case_smart(smart_case)
        .word(whole_word)
        .build(pattern)
        .with_context(|| format!("bad pattern `{pattern}`"))?;
    Ok(collect(root, files, &matcher, current, unsaved, |_| true))
}

/// Greps `pattern` over `files`, keeping only the lines `keep` takes. The filter runs before the
/// [`MAX_HITS`] cut, so what the cut drops are matches of the query, not whatever the walk
/// reached first: `D` past the cap searches with this.
pub fn grep_filtered(
    root: &Path,
    files: &[PathBuf],
    pattern: &str,
    current: Option<&Path>,
    unsaved: Option<&[u8]>,
    keep: impl Fn(&str) -> bool,
) -> Result<Vec<Hit>> {
    let matcher = RegexMatcherBuilder::new()
        .build(pattern)
        .with_context(|| format!("bad pattern `{pattern}`"))?;
    Ok(collect(root, files, &matcher, current, unsaved, keep))
}

/// The file walk both greps share: one `Hit` per line `matcher` matches and `keep` takes.
fn collect(
    root: &Path,
    files: &[PathBuf],
    matcher: &RegexMatcher,
    current: Option<&Path>,
    unsaved: Option<&[u8]>,
    keep: impl Fn(&str) -> bool,
) -> Vec<Hit> {
    let mut searcher = SearcherBuilder::new()
        .line_number(true)
        .binary_detection(BinaryDetection::quit(0))
        .build();

    let mut hits = Vec::new();
    for rel in files {
        if hits.len() >= MAX_HITS {
            break;
        }
        let sink = Collect {
            path: rel,
            hits: &mut hits,
            keep: &keep,
        };
        let _ = match unsaved.filter(|_| current == Some(rel.as_path())) {
            Some(text) => searcher.search_slice(matcher, text, sink),
            None => searcher.search_path(matcher, root.join(rel), sink),
        };
    }
    hits.sort_by_cached_key(|h| (current != Some(h.path.as_path()), h.path.clone(), h.line));
    hits
}

/// Collects one `Hit` per matching line `keep` takes, stopping the whole search at [`MAX_HITS`].
struct Collect<'a> {
    path: &'a Path,
    hits: &'a mut Vec<Hit>,
    keep: &'a dyn Fn(&str) -> bool,
}

impl Sink for Collect<'_> {
    type Error = std::io::Error;

    fn matched(&mut self, _searcher: &Searcher, m: &SinkMatch<'_>) -> std::io::Result<bool> {
        let text = String::from_utf8_lossy(m.bytes()).trim_end().to_string();
        if (self.keep)(&text) {
            self.hits.push(Hit {
                path: self.path.to_path_buf(),
                line: m.line_number().unwrap_or(0) as usize,
                text,
            });
        }
        Ok(self.hits.len() < MAX_HITS)
    }
}

/// How `d` found a declaration. Every step that narrows the search adds a variant here; the
/// status line and the picker rows print it, so a guess never passes for a resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// A declaration pattern matched the name, and nothing narrowed which declaration it is.
    ByName,
    /// Inside the module an import binds the word or its qualifier to: `numpy` for `np.array`
    /// behind `import numpy as np`.
    Import(String),
    /// Inside the module a path spells out, with no import: `std::fs` for
    /// `std::fs::read_to_string`.
    Path(String),
    /// Inside the type the receiver is declared with, or the call it is assigned from returns:
    /// `self.repo: UserRepository`, `NewRepo() *Repo`.
    Receiver(String),
    /// A type that implements the interface, protocol, abstract or base class the cursor stands
    /// in, and declares the same member: `implementations of Notifier.send`.
    Implementation(String),
    /// A parameter or a variable of the scope the cursor is in.
    Local,
}

impl Reason {
    /// Whether the reason alone picks the declaration. `by name` only says the name matched, so
    /// the status line adds how many did.
    pub fn proven(&self) -> bool {
        !matches!(self, Self::ByName)
    }
}

impl std::fmt::Display for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ByName => write!(f, "by name"),
            Self::Import(module) => write!(f, "via import {module}"),
            Self::Path(module) | Self::Receiver(module) => write!(f, "via {module}"),
            Self::Implementation(member) => write!(f, "implementations of {member}"),
            Self::Local => write!(f, "local"),
        }
    }
}

/// A declaration `d` can land on, and why it is offered.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub hit: Hit,
    pub reason: Reason,
}

/// Line patterns that declare `word` in a file of `kind`, or an empty list when there is no rule
/// for it. In Terraform `word` is the dotted address under the cursor.
pub fn def_patterns(kind: Kind, word: &str) -> Vec<String> {
    let w = regex::escape(word);
    match kind {
        // The optional `: Type` group covers annotated assignments (`X: Final[int] = 1`).
        Kind::Python => vec![
            format!(r"^\s*(?:async\s+)?(def|class)\s+{w}\b"),
            format!(r"^{w}\s*(:[^=]*)?="),
        ],
        Kind::Go => vec![
            // `func Get[T any](` is a function too.
            format!(r"^func\s+(\([^)]*\)\s*)?{w}(?:\[[^\]]*\])?\("),
            format!(r"^type\s+{w}\b"),
            format!(r"^(var|const)\s+{w}\b"),
            format!(r"^\s*{w}\s*:="),
        ],
        // `impl X` is a use of `X`, not its definition, so it is left out on purpose.
        Kind::Rust => {
            let vis = r#"^\s*(?:(?:pub(?:\([^)]*\))?|async|unsafe|const|extern(?:\s+"[^"]*")?|default)\s+)*"#;
            vec![
                format!(r"{vis}(?:fn|struct|enum|union|trait|type|const|static|mod)\s+{w}\b"),
                format!(r"^\s*macro_rules!\s+{w}\b"),
                format!(r"^\s*let\s+(?:mut\s+)?{w}\b"),
            ]
        }
        Kind::TsJs => {
            let pre = r"^\s*(?:(?:export|default|declare|abstract|async)\s+)*";
            let mut patterns = vec![
                format!(
                    r"{pre}(?:function\*?|class|interface|type|(?:const\s+)?enum|namespace|module)\s+{w}\b"
                ),
                // Arrow functions assigned to a name land here too.
                format!(r"{pre}(?:const|let|var)\s+{w}\b"),
            ];
            patterns.extend(member_patterns(kind, word).unwrap_or_default());
            patterns
        }
        Kind::Jvm => {
            let (mods, ret) = (jvm_mods!(), jvm_return_type!());
            let mods_one = jvm_mods!("+");
            vec![
                format!(
                    r"{mods}(?:class|interface|fun\s+interface|enum|record|@interface|object|typealias)\s+{w}\b"
                ),
                // Kotlin: a function, with its generics and, for an extension, its receiver.
                format!(
                    r"{mods}fun\s+(?:<[^>]*>\s*)?(?:[\w.]+(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?\??\.)?{w}\s*\("
                ),
                format!(r"{mods}(?:val|var)\s+{w}\b"),
                // Java: a constructor, behind at least one modifier. With nothing in front,
                // `Card(title) {` is a Kotlin call with a trailing lambda and `new Runnable() {`
                // an anonymous class, so a bare name before `(` is never a declaration here; a
                // method is found by the return type in the rule below, brace or no brace.
                format!(r"{mods_one}{w}\s*\([^;]*\)\s*(?:throws [\w.,\s]+)?\{{\s*$"),
                // Java: an abstract or interface method, and a field: a return type, the name,
                // and the `(`, `;` or `=` that follows it.
                format!(r"{mods}(?:<[^>]*>\s*)?{ret}(?:\.\.\.)?\s+{w}\s*[(;=]"),
            ]
        }
        // Ruby declares everything on one line. A constant lives indented inside its class, so
        // the assignment rule is not anchored at column zero as Python's is, and it takes the
        // `@`/`@@` of an instance or class variable with it. The Rails-style DSL (`scope`,
        // `has_many`, `define_method`) has no rule.
        Kind::Ruby => vec![
            // A method: `def name`, `def self.name`, `def Klass.name`, and the `name=` setter.
            format!(r"^\s*def\s+(?:self\.|[A-Z]\w*\.)?{w}\b"),
            format!(r"^\s*(?:class|module)\s+(?:[\w:]+::)?{w}\b"),
            // An assignment, `||=` included; `==`, `=~` and `=>` are not one.
            format!(r"^\s*@{{0,2}}{w}\s*(?:\|\|)?=($|[^=~>])"),
            // The reader and writer methods a class declares for its attributes, wherever the
            // name sits in the list.
            format!(r"^\s*attr_(?:accessor|reader|writer)\s+(?:[:\w]+\s*,\s*)*:{w}\b"),
            format!(r"^\s*alias(?:_method)?\s+:?{w}\b"),
        ],
        // C and C++ have no statements at the top level, so a line in column zero that is shaped
        // like a declaration is one — a definition, a prototype and a signature that wraps alike.
        // Indented, only a line that opens a body can be told from a call. An enum constant has no
        // rule: `NAME,` in an `enum` body and in an initializer list are the same line, and C
        // writes tables of callbacks that way everywhere.
        Kind::C => {
            let (mods, macros) = (c_mods!(), c_mods!(macros));
            vec![
                // A function, and the out-of-line definition of a method (`Type::name(`). GNU
                // style puts the return type on the line above, so the run before the name is
                // optional: in column zero a bare `name(` is a declaration all the same.
                format!(r"^(?:\w[^;(){{}}=]*[\s*&:])?{w}\s*\("),
                // The same indented — a method in a class body, a function in an indented
                // namespace — when the body opens on the line.
                format!(r"^[^;(){{}}=]*\w[\s*&]+{w}\s*\([^;{{}}]*\)[^;{{}}=]*\{{"),
                // A type. What follows the name — the body, a base list, a `<` of a template
                // specialization, a `;` or the end of the line — keeps the `struct dict *d;` that
                // uses one out.
                format!(
                    r"^\s*{mods}(?:typedef\s+)?(?:struct|class|union|enum\s+class|enum\s+struct|enum|namespace)\s+{macros}{w}\s*(?:[:{{;<]|final\b|$)"
                ),
                // `typedef unsigned long ull;`, `typedef int (*cb)(void);`, and the name a
                // `typedef struct { … } client;` closes with, whose brace is in column zero: an
                // indented one closes a nested anonymous struct, and that name is a field.
                format!(r"^\s*typedef\s+[^;]*(?:\(\s*\*+\s*{w}\s*\)|\b{w}\s*(?:\[[^\]]*\])*\s*;)"),
                format!(r"^\}}\s*[\w\s,*]*\b{w}\s*[,;]"),
                format!(r"^\s*(?:template\s*<[^>]*>\s*)?using\s+{w}\s*="),
                // An object- or function-like macro.
                format!(r"^\s*#\s*define\s+{w}\b"),
                // A global, with the array bounds it can carry. In column zero again, so an
                // assignment inside a function body is not one; no angle brackets, or a
                // `template <typename T = U>` head would read as a declaration of `T`.
                format!(r"^\w[^;(){{}}=<>]*[\s*&]{w}\s*(?:\[[^\]]*\])*\s*(?:=[^=]|;)"),
            ]
        }
        // C# writes its modifiers and its attributes in front of everything and its type before
        // the name, as Java does, so a member is told from a call by that type: a primitive,
        // `var`, or a name with a capital in it. A field and a local land here too — a `;` or an
        // `=` after the name is as much a declaration as the `(` of a method.
        Kind::CSharp => {
            let (mods, ty, access) = (cs_mods!(), cs_type!(), cs_mods!(access));
            vec![
                // A type, past the generic parameters it declares; the `(` is a record's or a
                // class's primary constructor, `where` its first constraint.
                format!(
                    r"{mods}(?:class|struct|interface|enum|record\s+class|record\s+struct|record)\s+{w}\s*(?:<[^>]*>)?\s*(?:[:{{;(]|where\b|$)"
                ),
                format!(r"{mods}delegate\s+{ty}\s+{w}\s*(?:<[^>]*>)?\s*\("),
                // A file-scoped or a block namespace, under its last part, as it is read.
                format!(r"^\s*namespace\s+(?:[\w.]+\.)?{w}\s*[;{{]?\s*$"),
                // `using Rows = List<int>;`: the alias form, where a plain `using` imports a
                // namespace and declares nothing.
                format!(r"^\s*(?:global\s+)?using\s+(?:unsafe\s+)?{w}\s*="),
                // A constructor, behind at least one access modifier. With nothing in front,
                // `Invoice(n);` is a call, so a bare name before `(` is never a declaration here.
                format!(r"{access}{w}\s*\([^;]*\)\s*(?::\s*(?:base|this)\b.*)?[{{=]?\s*$"),
                // A method, a property, an event and a field: the type, the name, and the `(` of
                // the parameters, the `{` of the accessors, the `=>` of an expression body, the
                // `=` of an initialiser, the `;` of a declaration with none — or the end of the
                // line, where a property's accessors open on the next one.
                format!(r"{mods}{ty}\s+(?:[\w.]+\.)?{w}\s*(?:<[^>]*>\s*)?(?:[({{;=]|$)"),
            ]
        }
        // Swift writes its attributes and modifiers in front of a keyword, and every declaration
        // has one, so there is no need to guess at a type. What has no rule is a binding made by
        // an `if let` or a `guard let`, which is a shadowing rebind of a name declared elsewhere,
        // and a parameter, as in every kind.
        Kind::Swift => {
            let mods = swift_mods!();
            let mut patterns = vec![
                // A type; `extension Foo` counts, since a project's own members of a type live
                // there and often the type itself does not.
                format!(
                    r"{mods}(?:class|struct|enum|protocol|actor|extension|typealias|associatedtype)\s+`?{w}\b"
                ),
                // A function, past its generic parameters.
                format!(r"{mods}func\s+`?{w}\s*[(<]"),
                format!(r"{mods}(?:let|var)\s+`?{w}\b"),
                // An enum case, alone or among several on one line, with the associated values or
                // the raw value it can carry. A `case .open:` or a `case let .open(x):` of a
                // `switch` is a pattern, and a `case open:` there matches against a constant, so
                // what follows the name must not be a `:`.
                format!(
                    r"^\s*(?:indirect\s+)?case\s+(?:\w+(?:\([^)]*\))?\s*,\s*)*{w}\s*(?:\(|=[^=]|,|$)"
                ),
            ];
            // `init` and `subscript` are keywords, so the word under the cursor is the keyword
            // itself and there is no name to read past.
            if matches!(word, "init" | "subscript" | "deinit") {
                patterns.push(format!(r"{mods}{w}\s*[?!(<{{]"));
            }
            patterns
        }
        // PHP declares with a keyword too, and what has none — a property, a promoted constructor
        // parameter — carries the modifiers that tell it from a use. A parameter and a `foreach`
        // target have no rule, as in every kind, and neither has `$this->name = …`, which writes
        // to a property the class declares elsewhere.
        Kind::Php => {
            let mods = php_mods!();
            let mods_one = php_mods!("+");
            vec![
                // A function or a method; `&` returns by reference.
                format!(r"{mods}function\s+&?\s*{w}\s*\("),
                format!(r"{mods}(?:class|interface|trait|enum)\s+{w}\b"),
                format!(r"^\s*namespace\s+(?:[\w\\]+\\)?{w}\s*[;{{]"),
                // A constant: the `const` of a class or a file, and the `define()` of a global.
                format!(r"{mods}const\s+{w}\b"),
                format!(r#"^\s*define\s*\(\s*['"]{w}['"]"#),
                // An enum case. A `case X:` of a `switch` matches against a constant, so what
                // follows the name must not be a `:`.
                format!(r"^\s*case\s+{w}\s*(?:=[^=]|;|$)"),
                // A property, with the type it can carry between its modifiers and the `$`.
                format!(r"{mods_one}(?:\??[\w\\|]+\s+)?\${w}\b"),
                // A constructor parameter promoted to one, wherever it sits in the list.
                format!(
                    r"function\s+__construct\s*\(.*\b(?:public|private|protected|readonly)\s+(?:\??[\w\\|]+\s+)?\${w}\b"
                ),
                // An assignment that opens a line, `.=` and `??=` included. `==` compares, `=>`
                // is a key in an array literal, and `$rows['x'] =` writes to an element.
                format!(r"^\s*\${w}\s*(?:\.|\?\?|\+)?=(?:$|[^=>])"),
            ]
        }
        // Lua declares with `function` and `local`, and with nothing else: a bare `name = value`
        // is an assignment to whatever `name` already is, and a field of a table constructor is
        // written exactly the same way, so only a function literal on the right counts.
        Kind::Lua => vec![
            // `function name(`, `local function name(`, and the table forms `function M.name(`,
            // `function M:name(`, `function a.b.name(`.
            format!(r"^\s*(?:local\s+)?function\s+(?:[\w.]+[.:])?{w}\s*\("),
            // A function literal bound to a name: `name = function(`, `M.name = function(`, and
            // the `name = function(` of a table of handlers.
            format!(r"^\s*(?:local\s+)?(?:[\w.]+[.:])?{w}\s*=\s*function\b"),
            // A local, the one declaration keyword the language has; `local a, b = f()`
            // declares both.
            format!(r"^\s*local\s+(?:[\w\s,]*,\s*)?{w}\b"),
        ],
        // Elixir declares with a `def` macro and with nothing else. Several clauses of one
        // function are several declarations, so `d` offers them all and the picker's rows say
        // which is which, the way a C++ overload set is offered.
        Kind::Elixir => {
            let mut patterns = vec![
                // Every `def` form. A name can end in `?` or `!`, and a clause is written
                // `def name(x) do`, `def name do` or `def name, do: x`.
                format!(
                    r"^\s*def(?:p|macro|macrop|guard|guardp|delegate)?\s+{w}[!?]?\s*(?:\(|,|do\b|$)"
                ),
                // A module or a protocol, under the namespace it is written with. The word has
                // to be the last part: `defmodule MyApp.Repo` declares `MyApp.Repo` and nothing
                // called `MyApp`. A `defimpl` declares the module `Protocol.Type`, where neither
                // name is its own, as a Rust `impl` is a use of the trait and the type.
                format!(r"^\s*def(?:module|protocol)\s+(?:[\w.]+\.)?{w}\s+do\b"),
                // A field of the struct, in the atom list or the keyword form.
                format!(r"^\s*defstruct\b.*(?::{w}\b|\b{w}:)"),
            ];
            // A module attribute is a declaration where it is given a value: `@timeout 5_000`.
            // The attributes the language itself gives a meaning to are directives, not names a
            // project declares — `@spec parse(t) :: t` is a promise about `parse`, not a
            // declaration of `spec` — so `d` on one of them has nothing to find, and says so.
            if !ELIXIR_DIRECTIVES.contains(&word) {
                patterns.push(format!(r"^\s*@{w}\s+[^\s|]"));
            }
            patterns
        }
        // Zig writes every declaration behind a keyword: `fn`, or the `const` a type, a constant
        // and an imported module alike are bound with. A struct field (`total: u32,`) has no rule,
        // as a C field has none: it is the shape of a value in a struct literal.
        Kind::Zig => {
            let mods = r#"^\s*(?:(?:pub|export|extern(?:\s+"[^"]*")?|inline|noinline|threadlocal|comptime)\s+)*"#;
            vec![
                format!(r"{mods}fn\s+{w}\s*\("),
                // `const Name = struct {`, `const Name = enum {` and a plain constant are one
                // form; a `var` and a local inside a body are declarations all the same. A
                // `const` at the start of a line always declares the name after it — the
                // `[]const Row` of a type never starts one — so nothing has to follow the name,
                // and the first name of a destructuring `const a, const b = t;` is found too.
                format!(r"{mods}(?:const|var)\s+{w}\b"),
            ]
        }
        // A function in either form, an assignment behind the declaration keywords that can
        // precede it (`+=` appends to one), or an alias. A shell has no declaration for the rest,
        // so a `$w` use or a `[ "$w" = x ]` test must not look like one.
        Kind::Shell => vec![
            format!(r"^\s*(function\s+)?{w}\s*\(\s*\)"),
            format!(r"^\s*function\s+{w}\b"),
            format!(
                r"^\s*(export\s+|declare\s+(-\w+\s+)*|local\s+(-\w+\s+)*|readonly\s+|typeset\s+(-\w+\s+)*)?{w}\+?="
            ),
            format!(r"^\s*alias\s+{w}="),
        ],
        // Everything here ignores case: SQL keywords are written both ways in the same file.
        // A column inside a `CREATE TABLE` body is deliberately not a rule — `name` alone on a
        // line is any column of any table, so `d` has no rule for one and `u` lists its uses.
        Kind::Sql => {
            let create = sql_create!();
            vec![
                format!(r#"{create}(?:{SQL_NAME}\.)?(?:"{w}"|`{w}`|{w}\b)"#),
                // A common table expression: opening the `WITH`, or continuing it after the
                // comma that follows the previous one's closing `)`.
                format!(r"(?i)^\s*(?:\)\s*)?(?:WITH\s+(?:RECURSIVE\s+)?|,\s*)?{w}\s+AS\s*\("),
            ]
        }
        // A target, alone or among others before the colon (`build test: deps`), or a variable.
        Kind::Make => vec![
            format!(r"^([^:=#\s]+\s+)*{w}(\s+[^:=#\s]+)*\s*::?([^=:]|$)"),
            format!(r"^\s*(export\s+|override\s+)?{w}\s*[:?!]{{0,3}}="),
        ],
        Kind::Terraform => terraform_patterns(word),
        // `FROM image AS name`, with any flags before the image. Stage names ignore case.
        Kind::Docker => vec![format!(r"(?i)^\s*FROM\s+(\S+\s+)+AS\s+{w}\s*$")],
        // An anchor, or a key that opens a block: compose services, CI jobs, GitLab's `.hidden`
        // jobs.
        Kind::Yaml => vec![
            format!(r"(^|\s)&{w}(\s|$)"),
            format!(r"^\s*\.?{w}:\s*(#.*)?$"),
        ],
    }
}

/// [`member_patterns`] and, in Go, the method lines of an interface, which carry no receiver:
/// every form in which a type declares a member called `word`.
pub fn member_or_signature(kind: Kind, word: &str) -> Option<Vec<String>> {
    let mut patterns = member_patterns(kind, word)?;
    if kind == Kind::Go {
        patterns.push(format!(r"^\s+{}\s*\(", regex::escape(word)));
    }
    // Inside a type that is known, an annotated property is a member whatever it holds: hono's
    // `match: typeof match = match` implements `Router.match`. By name it would be every
    // `name: string` of the project, so [`member_patterns`] leaves it out.
    if kind == Kind::TsJs {
        patterns.push(format!(
            r"^\s+(?:(?:public|private|protected|static|readonly|override|declare)\s+)*{}\s*[?!]?\s*:[^=;(]*(?:=|;|$)",
            regex::escape(word)
        ));
    }
    Some(patterns)
}

/// Line patterns that declare `word` as a member of a class, an interface, an object literal or
/// a receiver type: what `x.word` can reach when `x` is a value. A local, a module-level name or a
/// type is not a member, so those rules are left out. `None` for a kind whose members have no
/// rules of their own yet.
pub fn member_patterns(kind: Kind, word: &str) -> Option<Vec<String>> {
    let w = regex::escape(word);
    Some(match kind {
        // Indented: a function at the top of a module is reached through an import, not a value.
        Kind::Python => vec![format!(r"^\s+(?:async\s+)?def\s+{w}\b")],
        Kind::Go => vec![format!(r"^func\s+\([^)]*\)\s*{w}\(")],
        Kind::TsJs => {
            let mods = r"^\s*(?:(?:public|private|protected|static|readonly|abstract|override|async|get|set)\s+)*";
            vec![
                // A class or object-literal method: `foo(` at the end of the line, `foo(..) {`,
                // or an empty `foo(): void {}`. A `;` on the line means it was a call statement.
                format!(r"{mods}{w}\s*(?:<.*>)?\((?:[^;]*\{{\s*\}}?)?\s*$"),
                // A property holding a function: `foo = () =>`, `foo: async (x) =>`,
                // `foo: function`.
                format!(
                    r"{mods}{w}\s*[?!]?\s*(?::[^=]*)?[=:]\s*(?:async\s+)?(?:function\b|\(|[\w$]+\s*=>)"
                ),
                // A signature with no body, as an interface, an abstract class, an overload and
                // every class in a `.d.ts` write one: `foo(id: string): User;`. A signature's
                // parameters are none or start with an annotated name, where a call's arguments
                // are expressions (`foo(a ? b(c) : d);`); no `=` in the return type keeps an
                // annotated arrow argument out.
                format!(
                    r"{mods}{w}\??\s*(?:<.*>)?\((?:\s*|\s*(?:\.\.\.)?[\w$]+\??\s*:[^;{{}}]*)\)\s*:[^;{{}}=]*;?\s*$"
                ),
                // A constructor parameter behind an access modifier is a property of the class:
                // `@Inject(W) private worker: Worker,`. So is a field written the same way.
                format!(
                    r"^\s*(?:@[\w$.]+(?:\([^)]*\))?\s*)*(?:(?:public|private|protected|readonly|override)\s+)+{w}\s*[?!]?\s*:"
                ),
                // The same on the constructor's own line: `constructor(private worker: Worker) {}`.
                format!(
                    r"^\s*constructor\s*\(.*\b(?:public|private|protected|readonly)\s+{w}\s*[?!]?\s*:"
                ),
            ]
        }
        Kind::Rust
        | Kind::Jvm
        | Kind::Ruby
        | Kind::C
        | Kind::CSharp
        | Kind::Swift
        | Kind::Php
        | Kind::Lua
        | Kind::Elixir
        | Kind::Zig
        | Kind::Shell
        | Kind::Sql
        | Kind::Make
        | Kind::Terraform
        | Kind::Docker
        | Kind::Yaml => return None,
    })
}

/// Line patterns that can declare `word` as a field, for the search by name: more than the fields,
/// since a line counts only when [`field_decl`] finds the type it is the first declaration of the
/// field in. `None` for a kind whose fields have no rules.
pub fn field_patterns(kind: Kind, word: &str) -> Option<Vec<String>> {
    let w = regex::escape(word);
    Some(match kind {
        // `name: T` or `name = …` in a class body, `self.name = …` in a method, `self.name` in a
        // tuple, a `with` or a `for` target.
        Kind::Python => vec![
            format!(r"^\s+(?:self\.)?{w}\s*(?::|=[^=])"),
            format!(r"\bself\.{w}\b.*[^=!<>]=[^=]|\bas\s+self\.{w}\b|^\s*for\s.*\bself\.{w}\b"),
        ],
        // A member behind any modifiers, bare or not, a constructor parameter behind one and
        // any decorators, `this.name = …`.
        Kind::TsJs => vec![
            format!(
                r"^\s+(?:@[\w$.]+(?:\([^)]*\))?\s*)*(?:(?:public|private|protected|readonly|static|declare|override|abstract|accessor)\s+)*{w}\s*[?!]?\s*(?::|=[^=>]|;|$)"
            ),
            format!(r"^\s+this\.{w}\s*=[^=]"),
        ],
        // A struct field, alone or among others (`a, name T`), and an embedded `*pkg.Name`.
        Kind::Go => vec![
            format!(r"^\s+(?:[A-Za-z_]\w*\s*,\s*)*{w}(?:\s*,\s*[A-Za-z_]\w*)*\s+[^\s:=,(/]"),
            format!(r"^\s+\*?(?:[A-Za-z_]\w*\.)?{w}(?:\[.*\])?\s*(?:$|//|`)"),
        ],
        _ => return None,
    })
}

/// The name a reader knows the declaration on 1-based `line` of `text` by: `name` behind what it
/// is declared in, `UserRepository.delete_user` for a method, `Outer.Inner.run` for a nested
/// one, the receiver type of a Go method, the type a Rust `impl … for Type` is for. `None` at
/// the top level. The enclosing declarations are the lines above indented less, named by the
/// [`SYMBOLS`] rows a file of `kind` is read with; the walk stops at the first enclosing line
/// they name nothing on (a `return {`, an `if`), so a declaration stays unqualified rather than
/// wrongly qualified. A YAML anchor names a value, not a container: YAML is never qualified.
pub fn qualified(kind: Kind, text: &str, line: usize, name: &str) -> Option<String> {
    static RECEIVER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^func\s+\(\s*(?:\w+\s+)?\*?\s*([A-Za-z_]\w*)").unwrap()
    });
    static FUNC: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^func\s+(?:\([^)]*\)\s*)?([A-Za-z_]\w*)").unwrap()
    });
    static IMPL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:unsafe\s+)?impl\b(?:\s*<[^{]*?>)?\s+(?:[\w:]+(?:<[^{]*?>)?\s+for\s+)?&?(?:\w+::)*([A-Za-z_]\w*)").unwrap()
    });
    static ROWS: std::sync::LazyLock<Vec<(Option<Kind>, Regex)>> = std::sync::LazyLock::new(|| {
        SYMBOLS
            .iter()
            .map(|(k, p)| {
                (
                    *k,
                    Regex::new(p).expect("built-in symbol patterns are valid"),
                )
            })
            .collect()
    });
    if kind == Kind::Yaml {
        return None;
    }
    let sep = if matches!(kind, Kind::Rust | Kind::C | Kind::Php) {
        "::"
    } else {
        "."
    };
    let lines: Vec<&str> = text.lines().collect();
    let target = *lines.get(line.checked_sub(1)?)?;
    // Any other name on a Go function's line is the function's, a parameter or a named result,
    // and reads as a local of its body does (#100): `Load.err`, not the field `Issue.err`.
    if kind == Kind::Go
        && let Some(c) = FUNC.captures(target).filter(|c| &c[1] != name)
    {
        return Some(format!("{}{sep}{name}", &c[1]));
    }
    if let Some(c) = RECEIVER.captures(target).filter(|_| kind == Kind::Go) {
        return Some(format!("{}{sep}{name}", &c[1]));
    }
    // A field declared inside a method or in a constructor's parameters is the class's:
    // `Issue.repo`, not `Issue.__init__.repo`.
    if field_like(kind, target.trim_start(), name)
        && let Some(decl) = enclosing_type(kind, &lines, line - 1)
        && field_bindings(kind, text, decl, name)
            .iter()
            .any(|b| b.line == line)
        && let Some(ty) = type_name(kind, lines[decl - 1])
    {
        let owner = qualified(kind, text, decl, &ty).unwrap_or(ty);
        return Some(format!("{owner}{sep}{name}"));
    }
    let indent = |s: &str| s.len() - s.trim_start().len();
    let mut depth = indent(target);
    let mut names = vec![name.to_owned()];
    for l in lines[..line - 1].iter().rev() {
        if depth == 0 {
            break;
        }
        let t = l.trim_start();
        // A lone `{` opens the body of a declaration wrapped over the lines above it, as
        // prettier writes a long TypeScript class header; it names nothing itself. Neither does
        // a C++ access specifier, which is a label inside the class, not a wall in front of it.
        let access = kind == Kind::C && matches!(t, "public:" | "private:" | "protected:");
        if t.is_empty()
            || t == "{"
            || access
            || indent(l) >= depth
            || ["#", "//", "/*", "*", "--"]
                .iter()
                .any(|c| t.starts_with(c))
        {
            continue;
        }
        depth = indent(l);
        let named = IMPL
            .captures(l)
            .filter(|_| kind == Kind::Rust)
            .map(|c| c[1].to_owned())
            .or_else(|| {
                ROWS.iter()
                    .filter(|(k, _)| {
                        *k == Some(kind) || (k.is_none() && shared_symbols(Some(kind)))
                    })
                    .find_map(|(_, re)| symbol_name(re, l))
            });
        match named {
            Some(n) => names.push(n),
            None => break,
        }
    }
    names.reverse();
    (names.len() > 1).then(|| names.join(sep))
}

/// Whether the trimmed line `t` can declare the field `name` of a class from inside one of its
/// functions: it assigns `self.name` or `this.name`, or, in TypeScript, it is a parameter behind
/// an access modifier. A cheap test before [`field_bindings`] decides.
fn field_like(kind: Kind, t: &str, name: &str) -> bool {
    static MODIFIED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:@[\w$.]+(?:\([^)]*\))?\s*)*(?:(?:public|private|protected|readonly|override)\s+)+([\w$]+)").unwrap()
    });
    match kind {
        Kind::Python => in_method(t, name),
        Kind::TsJs => in_method(t, name) || MODIFIED.captures(t).is_some_and(|c| &c[1] == name),
        _ => false,
    }
}

/// Whether line `t` reaches the field `name` through `self.` or `this.`: an assignment inside a
/// method, as against a declaration in the class body or a constructor's parameters.
fn in_method(t: &str, name: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    ["self.", "this."].iter().any(|p| {
        let at = format!("{p}{name}");
        t.match_indices(&at)
            .any(|(i, m)| !t[..i].ends_with(ident) && !t[i + m.len()..].starts_with(ident))
    })
}

/// `var.x`, `module.x`, `local.x`, `data.T.N` and `T.N` (a resource), with anything after the
/// address ignored. A bare name, as in a `.tfvars` file or on a block's own label, is any block
/// with that name.
fn terraform_patterns(address: &str) -> Vec<String> {
    let parts: Vec<String> = address.split('.').map(regex::escape).collect();
    let parts: Vec<&str> = parts.iter().map(String::as_str).collect();
    let pattern = match parts.as_slice() {
        ["var", x, ..] => format!(r#"^variable\s+"{x}""#),
        ["module", x, ..] => format!(r#"^module\s+"{x}""#),
        ["local", x, ..] => format!(r"^\s+{x}\s*="),
        ["data", t, n, ..] => format!(r#"^data\s+"{t}"\s+"{n}""#),
        ["each" | "count" | "self" | "path" | "terraform", ..] => return Vec::new(),
        [t, n, ..] => format!(r#"^resource\s+"{t}"\s+"{n}""#),
        [x] => format!(r#"^(variable|module|output)\s+"{x}"|^(resource|data)\s+"[^"]+"\s+"{x}""#),
        [] => return Vec::new(),
    };
    vec![pattern]
}

/// The block a definition of `word` has to sit directly inside, when its line pattern cannot
/// tell on its own: a Terraform local is `x = ...` inside `locals { }`, and so is every other
/// attribute.
pub fn def_block(kind: Kind, word: &str) -> Option<&'static str> {
    (kind == Kind::Terraform && word.starts_with("local.")).then_some("locals")
}

/// Whether 1-based `line` of `text` sits directly inside a block whose first line starts with
/// `opener`: the nearest non-blank line above it that is indented less. `terraform fmt` indents
/// every block, so the indentation is the nesting.
pub fn directly_inside(text: &str, line: usize, opener: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let indent = |s: &str| s.len() - s.trim_start().len();
    let Some(target) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    lines[..line - 1]
        .iter()
        .rev()
        .find(|l| !l.trim().is_empty() && indent(l) < indent(target))
        .is_some_and(|l| l.trim_start().starts_with(opener))
}

/// Whether `path` is searched for a definition asked for in `here`, a file of `kind`. Stages,
/// anchors, compose services and CI jobs belong to their file; a Terraform module is a
/// directory; everything else is every file of the same kind, so `.tsx` finds `.ts`.
pub fn in_def_scope(kind: Kind, here: &Path, path: &Path) -> bool {
    match kind {
        Kind::Docker | Kind::Yaml => path == here,
        Kind::Terraform => kind_of(path) == Some(kind) && path.parent() == here.parent(),
        Kind::Python
        | Kind::Go
        | Kind::Rust
        | Kind::TsJs
        | Kind::Jvm
        | Kind::Ruby
        | Kind::C
        | Kind::CSharp
        | Kind::Swift
        | Kind::Php
        | Kind::Lua
        | Kind::Elixir
        | Kind::Zig
        | Kind::Shell
        | Kind::Sql
        | Kind::Make => kind_of(path) == Some(kind),
    }
}

/// Where the standard library and the dependencies of the project at `root` live on this
/// machine, for a file of `kind`; empty when the toolchain is not installed. Each is asked the
/// way it answers itself: `sys.path` of the project's interpreter, `rustc --print sysroot` plus
/// the registry crates `Cargo.lock` names, `GOROOT` plus the `go.mod` requirements in the module
/// cache, `node_modules`. `pip install -e` and vendored code inside `root` are project files
/// already, so `root` itself is never returned.
///
/// ponytail: spawns the toolchain on every `d` that leaves the project. Cache per root if it
/// ever shows.
pub fn external_roots(kind: Kind, root: &Path) -> Vec<PathBuf> {
    let run = |cmd: &str, args: &[&str]| -> Option<String> {
        let out = std::process::Command::new(cmd)
            .args(args)
            .current_dir(root)
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    };
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let mut dirs: Vec<PathBuf> = match kind {
        Kind::Python => {
            let venv = root.join(".venv/bin/python");
            let python = if venv.is_file() {
                venv
            } else {
                PathBuf::from("python3")
            };
            run(
                &python.to_string_lossy(),
                &["-c", "import sys; print('\\n'.join(sys.path))"],
            )
            .unwrap_or_default()
            .lines()
            .map(PathBuf::from)
            .collect()
        }
        Kind::Rust => {
            let mut dirs: Vec<PathBuf> = run("rustc", &["--print", "sysroot"])
                .map(|s| PathBuf::from(s.trim()).join("lib/rustlib/src/rust/library"))
                .into_iter()
                .collect();
            let cargo = std::env::var_os("CARGO_HOME")
                .map_or_else(|| home.join(".cargo"), PathBuf::from)
                .join("registry/src");
            // `[[package]]` tables: `name = "x"` then `version = "y"` is the directory `x-y`.
            let lock = std::fs::read_to_string(root.join("Cargo.lock")).unwrap_or_default();
            let mut name = None;
            for line in lock.lines() {
                if let Some(n) = line.strip_prefix("name = ") {
                    name = Some(n.trim_matches('"').to_owned());
                } else if let (Some(v), Some(n)) = (line.strip_prefix("version = "), name.take()) {
                    let crate_dir = format!("{n}-{}", v.trim_matches('"'));
                    for index in std::fs::read_dir(&cargo).into_iter().flatten().flatten() {
                        dirs.push(index.path().join(&crate_dir));
                    }
                }
            }
            dirs
        }
        Kind::Go => {
            let env = run("go", &["env", "GOROOT", "GOMODCACHE"]).unwrap_or_default();
            let mut env = env.lines();
            let mut dirs = Vec::new();
            if let Some(goroot) = env.next() {
                dirs.push(PathBuf::from(goroot).join("src"));
            }
            let cache = env.next().map(PathBuf::from).unwrap_or_default();
            // `require` lines, `path vX.Y.Z`; the cache escapes upper case as `!x`.
            let gomod = std::fs::read_to_string(root.join("go.mod")).unwrap_or_default();
            for line in gomod.lines() {
                let mut parts = line
                    .trim()
                    .trim_start_matches("require ")
                    .split_whitespace();
                if let (Some(path), Some(v)) = (parts.next(), parts.next())
                    && v.starts_with('v')
                {
                    let escaped: String = path
                        .chars()
                        .flat_map(|c| {
                            if c.is_ascii_uppercase() {
                                vec!['!', c.to_ascii_lowercase()]
                            } else {
                                vec![c]
                            }
                        })
                        .collect();
                    dirs.push(cache.join(format!("{escaped}@{v}")));
                }
            }
            dirs
        }
        Kind::TsJs => vec![root.join("node_modules")],
        // Zig's standard library, where its own `zig env` says it is. The dependencies of a
        // project live in the global package cache under hashed directory names no source line
        // spells out, so they are left out.
        Kind::Zig => zig_roots(&run("zig", &["env"]).unwrap_or_default()),
        // The system headers, which is where a C or C++ project's standard library and most of
        // its dependencies are: the SDK the toolchain reports on macOS, `/usr/include` on Linux,
        // and the two prefixes a package manager installs into. There is no per-project manifest
        // to read — what a build system was told with `-I` is not in the source — so the
        // directories are the same for every project, and the ones that do not exist fall out
        // below.
        Kind::C => {
            let mut dirs = vec![PathBuf::from("/usr/include")];
            if let Some(sdk) = run("xcrun", &["--show-sdk-path"]) {
                dirs.push(PathBuf::from(sdk.trim()).join("usr/include"));
            }
            dirs.push(PathBuf::from("/usr/local/include"));
            dirs.push(PathBuf::from("/opt/homebrew/include"));
            dirs
        }
        // Composer installs a project's dependencies into `vendor/`, as source, and gitignores
        // it, so the project walk does not list it: it is outside in the same way `node_modules`
        // is. PHP's own library is built into the interpreter and has no source to read.
        Kind::Php => vec![root.join("vendor")],
        // Where SwiftPM checks a package's dependencies out, as source. The standard library is
        // not there: the toolchain ships it compiled, with `.swiftinterface` stubs beside it and
        // no `.swift` file to read.
        Kind::Swift => vec![root.join(".build/checkouts")],
        // Java, Kotlin and Ruby have no roots yet: the JDK and Gradle caches, and a gem path,
        // are their own lookups. C# has nothing to point at: a NuGet package is compiled
        // assemblies, and the runtime's own source is not on the machine at all. Lua has no root
        // to ask for either: `package.path` is whatever the interpreter embedding it was built
        // with, and a Neovim or a LuaRocks tree is not a standard library any project can be
        // assumed to use. Elixir needs none: `mix` puts both the dependencies and their sources
        // in `deps/` inside the project, so they are project files already, and the standard
        // library ships compiled — an installed Elixir has `.beam` files, not `.ex`. `d` stays
        // inside the project for all of them, as for the rest.
        Kind::Jvm
        | Kind::Ruby
        | Kind::CSharp
        | Kind::Lua
        | Kind::Elixir
        | Kind::Shell
        | Kind::Sql
        | Kind::Make
        | Kind::Terraform
        | Kind::Docker
        | Kind::Yaml => Vec::new(),
    };
    dirs.retain(|d| d.is_dir() && d != root && !d.as_os_str().is_empty());
    dirs.dedup();
    dirs
}

/// The standard library directory in the output of `zig env`, which is JSON on some versions and
/// ZON on others: the value is the first quoted string after the key either way. A version that
/// reports no `std_dir` still reports the library directory it sits in. Empty when `zig` is not
/// on the PATH, as every root is when its toolchain is not installed.
fn zig_roots(env: &str) -> Vec<PathBuf> {
    let value = |key: &str| {
        let rest = env.split_once(key)?.1.trim_start_matches('"');
        let rest = rest.split_once('"')?.1;
        Some(PathBuf::from(rest.split_once('"')?.0))
    };
    value("std_dir")
        .or_else(|| value("lib_dir").map(|d| d.join("std")))
        .into_iter()
        .collect()
}

/// Every file of `kind` under `dirs`, as absolute paths. Nothing is ignored: `node_modules`
/// and `site-packages` are gitignored by design and are exactly what is wanted here. What no Go
/// import can reach is left out: a `_test.go` file, a `testdata` directory, and a nested module
/// (GOROOT's `cmd`, the toolchain's own source), which is a root of its own when it is a
/// dependency.
pub fn external_files(kind: Kind, dirs: &[PathBuf]) -> Vec<PathBuf> {
    let go = kind == Kind::Go;
    let mut files = Vec::new();
    for dir in dirs {
        // Homebrew's Rust ships the sysroot `library` with a copy of itself inside; every
        // definition would come up twice.
        let copy = dir.file_name().map(std::ffi::OsStr::to_owned);
        let walk = ignore::WalkBuilder::new(dir)
            .filter_entry(move |e| {
                let unreachable = go
                    && e.depth() > 0
                    && e.file_type().is_some_and(|t| t.is_dir())
                    && (e.file_name() == "testdata" || e.path().join("go.mod").is_file());
                !unreachable && (e.depth() != 1 || Some(e.file_name()) != copy.as_deref())
            })
            .hidden(false)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .ignore(false)
            .parents(false)
            .build();
        files.extend(
            walk.filter_map(Result::ok)
                .map(ignore::DirEntry::into_path)
                .filter(|p| p.is_file() && kind_of(p) == Some(kind))
                .filter(|p| !(go && p.to_string_lossy().ends_with("_test.go"))),
        );
    }
    files
}

/// Whether `path` is a TypeScript declaration file: `.d.ts`, `.d.mts` or `.d.cts`.
pub fn declaration_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| [".d.ts", ".d.mts", ".d.cts"].iter().any(|e| n.ends_with(e)))
}

/// The files a reader is shown last: tests, mocks, fixtures, generated code and vendored copies
/// (#81). A row ending in `/` is a directory anywhere in the path; the rest match the file name,
/// where a `*` at either end stands for any run of characters. One table, so `u` and the
/// candidates of `d` agree on what a test file is.
const LAST: &[&str] = &[
    "test/",
    "tests/",
    "__tests__/",
    "spec/",
    "specs/",
    "testdata/",
    "fixtures/",
    "__fixtures__/",
    "mocks/",
    "__mocks__/",
    "vendor/",
    "third_party/",
    // A test file, by the naming every language settled on: `test_user.py`, `user_test.go`,
    // `user_spec.rb`, `user.test.ts`, `user.spec.ts`.
    "test_*",
    "conftest.py",
    "*_test.*",
    "*_spec.*",
    "*.test.*",
    "*.spec.*",
    // Generated: protobuf, and the `.gen.`/`.generated.` convention the code generators use.
    "*_pb2.py",
    "*_pb2_grpc.py",
    "*.pb.go",
    "*.gen.go",
    "*.generated.*",
];

/// Where a hit sorts in a result list: what the reader came for first (#81).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// A line that declares the word, by the [`def_patterns`] of the file's kind.
    Declaration,
    /// The file on screen.
    Open,
    /// The rest of the project's code.
    Code,
    /// A file of [`LAST`].
    Tests,
}

/// How a hit in `path` sorts, with the open file at `here` and `declaration` telling whether the
/// line declares the word: its [`Tier`], then how many directories away from the open file it
/// lives, the nearest first. The caller breaks a tie by path and line.
///
/// `u` ranks its hits with this, and `d` demotes the candidates in [`Tier::Tests`] with it, so
/// one table decides for both. The open file is never demoted: it is what the reader is reading.
pub fn rank(path: &Path, here: Option<&Path>, declaration: bool) -> (Tier, usize) {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let last = LAST.iter().any(|row| match row.strip_suffix('/') {
        Some(dir) => path
            .parent()
            .is_some_and(|p| p.iter().any(|c| c == std::ffi::OsStr::new(dir))),
        None => match (row.strip_prefix('*'), row.strip_suffix('*')) {
            (Some(_), Some(_)) => name.contains(row.trim_matches('*')),
            (Some(suffix), None) => name.ends_with(suffix),
            (None, Some(prefix)) => name.starts_with(prefix),
            (None, None) => name == *row,
        },
    });
    let tier = if last && here != Some(path) {
        Tier::Tests
    } else if declaration {
        Tier::Declaration
    } else if here == Some(path) {
        Tier::Open
    } else {
        Tier::Code
    };
    let dirs = |p: &Path| p.parent().map_or(0, |d| d.components().count());
    let steps = here.map_or(0, |h| {
        let shared = path
            .parent()
            .unwrap_or(Path::new(""))
            .components()
            .zip(h.parent().unwrap_or(Path::new("")).components())
            .take_while(|(a, b)| a == b)
            .count();
        dirs(path) + dirs(h) - 2 * shared
    });
    (tier, steps)
}

/// The dotted or `::` chain in front of the word under the cursor: `["json"]` for
/// `json.load(`, `["os", "path"]` for `os.path.join(`, `["fs"]` for `fs::read(`. Empty when the
/// word stands alone. A TypeScript private field keeps its `#`: `["this", "#root"]`. A chain that
/// hangs off a call, an index or `?.` is empty too: `users` in `make_uow().users.x` is no name of
/// its own.
pub fn qualifier(line: &str, word_start: usize) -> Vec<String> {
    let mut before = &line[..word_start];
    let mut chain = Vec::new();
    while let Some(rest) = before
        .strip_suffix("::")
        .or_else(|| before.strip_suffix('.'))
    {
        // Python's `super().` reads as TypeScript's `super.` does: one name.
        if let Some(head) = rest.strip_suffix("super()")
            && !head.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        {
            chain.insert(0, "super".to_owned());
            before = head;
            break;
        }
        let mut start = rest
            .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .map_or(0, |i| i + 1);
        if start == rest.len() {
            break;
        }
        if rest[..start].ends_with(".#") {
            start -= 1;
        }
        chain.insert(0, rest[start..].to_owned());
        before = &rest[..start];
    }
    // `f().`, `a[0].`, `a?.` or a line starting with `.` are left; a spread `...` or a range `..`
    // is not a member access.
    if before.ends_with('.') && !before.ends_with("..") {
        return Vec::new();
    }
    chain
}

/// The call a member access hangs off, where [`qualifier`] has no name to start from:
/// `pkg.New(x).word`, `make_uow().users.word`, `new Repo().word`, or the cast: `(x as T).word`,
/// `i.(T).word`, `cast(T, x).word`. Gives the call without its arguments (a cast as written), what
/// it is worth ([`Value::Call`], [`Value::New`] or the [`Value::Type`] of a cast) and the names
/// between it and the word. Only a call that starts the expression: `a.b().c().word` hangs off a
/// call of a value nobody typed.
pub fn call_head(
    kind: Kind,
    line: &str,
    word_start: usize,
) -> Option<(String, Value, Vec<String>)> {
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    let mut before = line[..word_start].strip_suffix('.')?;
    let mut fields = Vec::new();
    while !before.ends_with(')') {
        let start = before.rfind(|c: char| !is_name(c)).map_or(0, |i| i + 1);
        if start == before.len() {
            return None;
        }
        fields.insert(0, before[start..].to_owned());
        before = before[..start].strip_suffix('.')?;
    }
    // The `(` this `)` closes, as a scan that knows strings pairs them.
    let open = code(kind, before)
        .filter(|(_, c)| *c == b'(')
        .map(|(i, _)| i)
        .find(|&i| close_of(kind, before, i) == Some(before.len()))?;
    let mut start = before[..open]
        .rfind(|c: char| !(is_name(c) || c == '.'))
        .map_or(0, |i| i + 1);
    // `.c` in `a.b().c()` is no callee [`value_of`] reads, and nor is nothing at all.
    let callee = &before[start..open];
    if kind == Kind::TsJs
        && let Some(rest) = before[..start].trim_end().strip_suffix("new")
        && !rest.ends_with(is_name)
    {
        start = rest.len();
    }
    // `(x as T)` is read without its brackets.
    let written = &before[start..];
    let inner = match callee.is_empty() {
        true => &written[1..written.len() - 1],
        false => written,
    };
    match value_of(kind, inner) {
        Value::Call(name) => Some((format!("{name}()"), Value::Call(name), fields)),
        Value::New(name) => Some((format!("new {name}()"), Value::New(name), fields)),
        // A cast the chain hangs off, as written: `(x as T)`, `i.(T)`, `cast(T, x)`.
        value @ (Value::Type(_) | Value::Cast(..)) => {
            Some((inner.trim().to_owned(), value, fields))
        }
        _ => None,
    }
}

/// The names a file binds by importing, each with the module path it comes from, as the parts
/// a file system would spell it in. `import numpy as np` binds `np` to `[numpy]`; `from json
/// import load` binds `load` to `[json, load]` (a module or a name in one, the caller relaxes
/// the path until a file matches). A relative import (`from ..models import X`, `./utils`) binds
/// too, with its dots as the leading part: it names a project file, never one outside. A
/// TypeScript path ends in what the import takes from the module: the name, `default`, or `*`
/// for the whole module (`* as ns`, `require`). Rust's in-crate `crate::` and `super::` paths are
/// left out.
pub fn imports(kind: Kind, text: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let parts = |module: &str, sep: &str| -> Vec<String> {
        module
            .split(sep)
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect()
    };
    // `name`, or `name as alias`: the alias is what the file uses.
    let bound = |item: &str| -> Option<(String, String)> {
        let mut it = item.split_whitespace();
        let name = it
            .next()?
            .trim_matches(|c| c == '{' || c == '}' || c == ',')
            .to_owned();
        let alias = match (it.next(), it.next()) {
            (Some("as"), Some(a)) => a.trim_end_matches(',').to_owned(),
            _ => name.clone(),
        };
        (!name.is_empty()).then_some((alias, name))
    };
    match kind {
        Kind::Python => {
            static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(
                    r"(?m)^\s*(?:from\s+([\w.]+)\s+import\s+(\([^)]*\)|[^\n]+)|import\s+([^\n]+))",
                )
                .unwrap()
            });
            for c in IMPORT.captures_iter(text) {
                if let (Some(module), Some(names)) = (c.get(1), c.get(2)) {
                    let module = module.as_str();
                    let relative = module.trim_start_matches('.');
                    let mut base = parts(relative, ".");
                    if relative.len() < module.len() {
                        base.insert(0, ".".repeat(module.len() - relative.len()));
                    }
                    for item in names
                        .as_str()
                        .trim_matches(|c| c == '(' || c == ')')
                        .split(',')
                    {
                        if let Some((alias, name)) = bound(item.trim()) {
                            let mut path = base.clone();
                            path.push(name);
                            out.push((alias, path));
                        }
                    }
                } else if let Some(list) = c.get(3) {
                    for item in list.as_str().split(',') {
                        if let Some((alias, name)) = bound(item.trim()) {
                            // `import a.b.c` binds `a`; `import a.b.c as d` binds `d` to `a.b.c`.
                            if alias == name {
                                let first = name.split('.').next().unwrap_or(&name).to_owned();
                                out.push((first.clone(), vec![first]));
                            } else {
                                out.push((alias, parts(&name, ".")));
                            }
                        }
                    }
                }
            }
        }
        Kind::Rust => {
            static USE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(r"(?ms)^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+([^;]+);").unwrap()
            });
            for c in USE.captures_iter(text) {
                use_tree(&c[1], &[], &mut out);
            }
        }
        Kind::Go => {
            static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(r#"(?m)^\s*(?:import\s+)?(?:(\w+|\.|_)\s+)?"([^"]+)"\s*$"#).unwrap()
            });
            let version =
                |p: &str| p.starts_with('v') && p[1..].chars().all(|c| c.is_ascii_digit());
            for c in IMPORT.captures_iter(text) {
                let path = parts(&c[2], "/");
                if let Some(alias) = c.get(1) {
                    out.push((alias.as_str().to_owned(), path));
                    continue;
                }
                // The package name is the last element that is not a major version, without the
                // decorations a module path carries: `yaml.v3`, `nats.go`, `go-sqlite3`,
                // `bar-go`. A major version last can be the package itself
                // (`k8s.io/api/core/v1`), so it binds as well.
                let Some(last) = path.iter().rev().find(|p| !version(p)) else {
                    continue;
                };
                let mut name = last.as_str();
                if let Some((stem, suffix)) = name.rsplit_once('.')
                    && (version(suffix) || suffix == "go")
                {
                    name = stem;
                }
                name = name.strip_prefix("go-").unwrap_or(name);
                name = name.strip_suffix("-go").unwrap_or(name);
                out.push((name.to_owned(), path.clone()));
                if let Some(v) = path.last().filter(|p| version(p)) {
                    out.push((v.clone(), path.clone()));
                }
            }
        }
        Kind::TsJs => {
            static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(
                    r#"(?ms)^\s*(?:import\s+(?:type\s+)?([^'"]*?)\s*from\s*|(?:const|let|var)\s+([^=]+?)\s*=\s*(?:await\s+)?(?:require|import)\s*\(\s*|import\s+([\w$]+)\s*=\s*require\s*\(\s*)['"]([^'"]+)['"]"#,
                )
                .unwrap()
            });
            for c in IMPORT.captures_iter(text) {
                let module = c[4].strip_prefix("node:").unwrap_or(&c[4]);
                // `./x` and `../x` keep their dots as the first part; an absolute path gets one.
                let mut path = parts(module, "/");
                if module.starts_with('/') {
                    path.insert(0, ".".to_owned());
                }
                // A bare `x` is the default export after `import`, the whole module after
                // `require`. `x`, `* as x`, `{a, type b as c}` and `x, {a}` in one clause.
                let (clause, whole) = match (c.get(1), c.get(2).or_else(|| c.get(3))) {
                    (Some(m), _) => (m.as_str(), "default"),
                    (None, m) => (m.map_or("", |m| m.as_str()), "*"),
                };
                let (outside, named) = clause.split_once('{').map_or((clause, ""), |(o, n)| {
                    (o, n.split('}').next().unwrap_or(""))
                });
                let with = |taken: &str| {
                    let mut p = path.clone();
                    p.push(taken.to_owned());
                    p
                };
                for item in outside.split(',').map(str::trim).filter(|i| !i.is_empty()) {
                    match item.strip_prefix("* as ") {
                        Some(ns) => out.push((ns.trim().to_owned(), with("*"))),
                        None => out.push((item.to_owned(), with(whole))),
                    }
                }
                for item in named.split(',') {
                    let item = item.trim();
                    if let Some((alias, name)) = bound(item.strip_prefix("type ").unwrap_or(item)) {
                        out.push((alias, with(&name)));
                    }
                }
            }
        }
        // A `use` names one class, function or constant, and PSR-4 spells a namespace the way a
        // file system does, so the path is the name split on `\`: `use Illuminate\Support\Str`
        // binds `Str` to `vendor/…/Illuminate/Support/Str.php`. In column zero only — indented,
        // `use` pulls a trait into a class body and names no file — and a group `use A\{B, C}`
        // is left out, since one clause then binds several.
        Kind::Php => {
            static USE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(r"(?m)^use\s+(?:function\s+|const\s+)?([^;{]+);").unwrap()
            });
            for c in USE.captures_iter(text) {
                for item in c[1].split(',') {
                    if let Some((alias, name)) = bound(item.trim()) {
                        let path = parts(&name, "\\");
                        if let Some(last) = path.last().cloned() {
                            out.push((if alias == name { last } else { alias }, path));
                        }
                    }
                }
            }
        }
        // Nothing to bind without roots to resolve an `import` or a `require` against. A C
        // `#include` binds no name of its own either: it pastes a file in, and everything the
        // file declares is then visible unqualified, and a C# `using` opens a whole namespace
        // the same way. Zig's `const std = @import("std")` does bind one, but `std` is the root
        // itself, not a directory inside it, so narrowing by it would find nothing.
        Kind::Jvm
        | Kind::Ruby
        | Kind::C
        | Kind::CSharp
        | Kind::Swift
        | Kind::Lua
        | Kind::Elixir
        | Kind::Zig
        | Kind::Shell
        | Kind::Sql
        | Kind::Make
        | Kind::Terraform
        | Kind::Docker
        | Kind::Yaml => {}
    }
    out
}

/// The 1-based line of the Go file `text` whose import binds `name`, as [`imports`] reads it.
pub fn go_import_line(text: &str, name: &str) -> Option<usize> {
    let binds = |l: &str| imports(Kind::Go, l).iter().any(|(n, _)| n == name);
    text.lines().position(binds).map(|i| i + 1)
}

/// One `use` tree: `a::b::{c, d as e, f::*}` binds `c`, `e` and every name of `f`. A `crate`,
/// `self` or `super` root is the project.
fn use_tree(tree: &str, prefix: &[String], out: &mut Vec<(String, Vec<String>)>) {
    let tree = tree.trim();
    if tree.is_empty() {
        return;
    }
    if let Some((head, rest)) = tree.split_once('{') {
        let rest = rest.trim_end().trim_end_matches('}');
        let mut path = prefix.to_vec();
        path.extend(
            head.split("::")
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(String::from),
        );
        // Split the list at the commas outside nested braces.
        let (mut depth, mut start) = (0, 0);
        for (i, ch) in rest.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                ',' if depth == 0 => {
                    use_tree(&rest[start..i], &path, out);
                    start = i + 1;
                }
                _ => {}
            }
        }
        use_tree(&rest[start..], &path, out);
        return;
    }
    let mut path = prefix.to_vec();
    let (tree, alias) = tree
        .split_once(" as ")
        .map_or((tree, None), |(t, a)| (t.trim(), Some(a.trim().to_owned())));
    path.extend(
        tree.split("::")
            .map(str::trim)
            .filter(|p| !p.is_empty() && *p != "self" && *p != "*")
            .map(String::from),
    );
    if matches!(
        path.first().map(String::as_str),
        None | Some("crate" | "super")
    ) {
        return;
    }
    let Some(last) = path.last().cloned() else {
        return;
    };
    out.push((alias.unwrap_or(last), path));
}

/// The project files of the module an import in `here` spells as `module`, the parts
/// [`imports`] gives without what a TypeScript import takes from it; empty when the module is not
/// the project's. Paths are relative to `root`, and only what the project walk listed in `files`
/// counts.
///
/// - Python: `a/b.py` or the package `a/b/__init__.py`. Relative to the directory of `here` behind
///   dots, one directory up per dot past the first; otherwise at any depth (the root, `src/`, a
///   folder of a monorepo) but not inside a package, since `json` is not `myapp/json.py`.
/// - TypeScript: `./x` from the directory of `here`, an alias from the `paths` of the nearest
///   `tsconfig.json` or a name under its `baseUrl`, as `x.ts`, `x.tsx`, `x.d.ts`, the JavaScript
///   forms, then `x/index.*`. `./x.js` is `x.ts` first, as ESM projects write it.
/// - Go: the directory under the `go.mod` whose `module` the import path starts with (the longest,
///   for a module nested in another), without its `_test.go` files.
pub fn module_files(
    kind: Kind,
    root: &Path,
    files: &[PathBuf],
    here: &Path,
    module: &[String],
) -> Vec<PathBuf> {
    let dir = here.parent().unwrap_or(Path::new(""));
    match kind {
        Kind::Python => {
            let (base, parts) = match module.split_first() {
                Some((dots, rest)) if dots.starts_with('.') => {
                    match dir.ancestors().nth(dots.len() - 1) {
                        Some(base) => (Some(base), rest),
                        None => return Vec::new(),
                    }
                }
                _ => (None, module),
            };
            let name: PathBuf = parts.iter().collect();
            let mut forms = vec![name.join("__init__.py")];
            if !parts.is_empty() {
                forms.insert(0, name.with_extension("py"));
            } else if base.is_none() {
                return Vec::new();
            }
            files
                .iter()
                .filter(|f| {
                    forms.iter().any(|m| match base {
                        Some(base) => **f == base.join(m),
                        None => {
                            f.ends_with(m)
                                && f.ancestors()
                                    .nth(m.components().count())
                                    .is_some_and(|top| !files.contains(&top.join("__init__.py")))
                        }
                    })
                })
                .cloned()
                .collect()
        }
        Kind::TsJs => {
            let listed: std::collections::HashSet<&Path> =
                files.iter().map(PathBuf::as_path).collect();
            let spec = module.join("/");
            let bases = if module.first().is_some_and(|p| p.starts_with('.')) {
                vec![dir.join(&spec)]
            } else {
                ts_aliases(root, dir, &spec)
            };
            const EXTENSIONS: [&str; 9] =
                ["ts", "tsx", "d.ts", "js", "jsx", "mts", "cts", "mjs", "cjs"];
            bases
                .iter()
                .filter_map(|b| lexical(b))
                .find_map(|base| {
                    let s = base.to_string_lossy();
                    let stem = [".js", ".jsx", ".mjs", ".cjs"]
                        .iter()
                        .find_map(|e| s.strip_suffix(e))
                        .unwrap_or(&s);
                    EXTENSIONS
                        .iter()
                        .map(|e| PathBuf::from(format!("{stem}.{e}")))
                        .chain([base.clone()])
                        .chain(EXTENSIONS.iter().map(|e| base.join(format!("index.{e}"))))
                        .find(|c| listed.contains(c.as_path()) && kind_of(c) == Some(Kind::TsJs))
                })
                .into_iter()
                .collect()
        }
        Kind::Go => {
            let import = module.join("/");
            let package = files
                .iter()
                .filter(|f| f.file_name().is_some_and(|n| n == "go.mod"))
                .filter_map(|gomod| {
                    let text = std::fs::read_to_string(root.join(gomod)).ok()?;
                    let name = text
                        .lines()
                        .find_map(|l| l.trim().strip_prefix("module "))?
                        .split_whitespace()
                        .next()?
                        .trim_matches('"')
                        .to_owned();
                    let rest = match import.strip_prefix(&name)? {
                        "" => "",
                        rest => rest.strip_prefix('/')?,
                    };
                    Some((name.len(), gomod.parent()?.join(rest)))
                })
                .max_by_key(|(n, _)| *n);
            let Some((_, package)) = package else {
                return Vec::new();
            };
            files
                .iter()
                .filter(|f| f.parent() == Some(package.as_path()))
                .filter(|f| {
                    kind_of(f) == Some(Kind::Go) && !f.to_string_lossy().ends_with("_test.go")
                })
                .cloned()
                .collect()
        }
        Kind::Rust
        | Kind::Jvm
        | Kind::Ruby
        | Kind::C
        | Kind::CSharp
        | Kind::Swift
        | Kind::Php
        | Kind::Lua
        | Kind::Elixir
        | Kind::Zig
        | Kind::Shell
        | Kind::Sql
        | Kind::Make
        | Kind::Terraform
        | Kind::Docker
        | Kind::Yaml => Vec::new(),
    }
}

/// Where an aliased TypeScript specifier points, most specific first: the targets of the
/// `compilerOptions.paths` entries it matches, then the specifier under `baseUrl`. Read from the
/// `tsconfig.json` nearest above `dir` and the configs it `extends` by a relative path, the
/// nearest setting winning; `paths` are relative to `baseUrl` when there is one, else to the
/// config that declares them. Comments and trailing commas are fine: only these keys are read.
fn ts_aliases(root: &Path, dir: &Path, spec: &str) -> Vec<PathBuf> {
    static COMMENT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#""(?:[^"\\]|\\.)*"|//[^\n]*|/\*(?s:.*?)\*/"#).unwrap()
    });
    let key = |name: &str, value: &str| {
        Regex::new(&format!(r#""{name}"\s*:\s*{value}"#)).expect("a fixed key pattern")
    };
    let (base_url, paths, extends) = (
        key("baseUrl", r#""([^"]*)""#),
        key("paths", r"\{([^}]*)\}"),
        key("extends", r#""(\.[^"]*)""#),
    );
    let entry = Regex::new(r#""([^"]+)"\s*:\s*\[([^\]]*)\]"#).expect("a fixed pattern");
    let string = Regex::new(r#""([^"]*)""#).expect("a fixed pattern");

    let mut config = dir
        .ancestors()
        .map(|d| d.join("tsconfig.json"))
        .find(|c| root.join(c).is_file());
    let (mut url, mut table): (Option<PathBuf>, Option<(PathBuf, String)>) = (None, None);
    // ponytail: eight `extends` hops, which also ends a cycle.
    for _ in 0..8 {
        let Some(file) = config.take() else { break };
        let Ok(text) = std::fs::read_to_string(root.join(&file)) else {
            break;
        };
        let text = COMMENT.replace_all(&text, |c: &regex::Captures| {
            if c[0].starts_with('"') {
                c[0].to_owned()
            } else {
                String::new()
            }
        });
        let at = file.parent().unwrap_or(Path::new("")).to_path_buf();
        if url.is_none() {
            url = base_url.captures(&text).map(|c| at.join(&c[1]));
        }
        if table.is_none() {
            table = paths.captures(&text).map(|c| (at.clone(), c[1].to_owned()));
        }
        // `"extends": "./base"` is `./base.json`.
        config = extends.captures(&text).map(|c| {
            if c[1].ends_with(".json") {
                at.join(&c[1])
            } else {
                at.join(format!("{}.json", &c[1]))
            }
        });
    }
    let mut targets: Vec<(usize, PathBuf)> = Vec::new();
    if let Some((at, table)) = &table {
        let from = url.as_ref().unwrap_or(at);
        for e in entry.captures_iter(table) {
            // An exact key outranks every wildcard; among wildcards the longer prefix wins.
            let matched = match e[1].split_once('*') {
                Some((pre, post)) => spec
                    .strip_prefix(pre)
                    .and_then(|s| s.strip_suffix(post))
                    .map(|s| (pre.len(), s)),
                None => (e[1] == *spec).then_some((usize::MAX, "")),
            };
            if let Some((rank, star)) = matched {
                targets.extend(
                    string
                        .captures_iter(&e[2])
                        .map(|t| (rank, from.join(t[1].replace('*', star)))),
                );
            }
        }
    }
    targets.sort_by_key(|(rank, _)| std::cmp::Reverse(*rank));
    targets
        .into_iter()
        .map(|(_, t)| t)
        .chain(url.map(|u| u.join(spec)))
        .collect()
}

/// `path` with its `.` and `..` parts folded away, `None` when it climbs above where it starts.
fn lexical(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            c => out.push(c),
        }
    }
    Some(out)
}

// ---- the type of a receiver (#68, steps 2 and 3) --------------------------------------------

/// What a declaration gives a name, as far as the declaration itself tells. The caller resolves a
/// type in the file that wrote it, a call through the declaration of what it calls, and another
/// name at the declaration's line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A type as written: `UserRepository`, `*Repo`, `Optional[Repo]`.
    Type(String),
    /// A construction of the named type: `new Repo()`, `Repo{}`, `&Repo{}`, `new(Repo)`.
    New(String),
    /// A call of the named function, or of a Python class: `make_repo`, `store.Open`.
    Call(String),
    /// Another name, as `self.repo = repo` hands on a parameter.
    Name(String),
    /// The class declared on this 1-based line: Python's `self` and `cls`, TypeScript's `this`.
    Class(usize),
    /// Python's `cast(T, x)` as written: the callee and `T`. It writes the type when the callee
    /// is `typing`'s, which only the caller can tell: a project may declare a `cast` of its own.
    Cast(String, String),
    /// An element of the named collection, as a loop hands it out: `for r in repos`,
    /// `for (const r of repos)`, `for _, r := range repos`. [`element_type`] reads it off the
    /// collection's written type.
    Element(String),
    /// A declaration whose type the rules cannot read: `for repo in`, a tuple, a parameter with
    /// no annotation.
    Unknown,
}

/// A declaration of a name on 1-based `line`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub line: usize,
    pub value: Value,
}

fn indent(s: &str) -> usize {
    s.len() - s.trim_start().len()
}

/// Whether a trimmed line of `kind` is a comment. TypeScript's `#private` is a name.
fn comment(kind: Kind, t: &str) -> bool {
    match kind {
        Kind::Python => t.starts_with('#'),
        _ => ["//", "/*", "*"].iter().any(|c| t.starts_with(c)),
    }
}

/// The 1-based lines of `text` that start inside a literal or a comment running over several
/// lines: a Python or Elixir triple-quoted string (a docstring or an `@moduledoc` with an example
/// in it), which Swift and C# write with `"` alone, a Go raw string, a TypeScript template, a Lua
/// `[[ ]]` or `[==[ ]==]` long string or block comment, a C# verbatim `@"…"`, a PHP heredoc, a
/// `/* */` block. A line there that reads like a declaration declares nothing — the SQL a
/// migration embeds in one is the common case. Strings of one line end with their line, whatever
/// they hold.
///
/// Each kind says which forms it has rather than inheriting another language's: Zig has none at
/// all — a `\\` string ends with its line — and reading it with the backtick and `/* */` of the C
/// family would take the ``` ``` ``` fences of the markdown a `\\` block holds for a literal and
/// hide the rest of the file behind them.
///
/// ponytail: Elixir's `~S"""` sigil is read from its `"""`, and its one-line `~s(…)` forms not at
/// all.
pub fn literal_lines(kind: Kind, text: &str) -> Vec<bool> {
    // What this kind writes: the comment that runs to the end of a line, the triple quote of a
    // heredoc, Lua's long bracket, and the backtick template with the `/* */` block of the C
    // family. Elixir writes its heredocs and its comments exactly as Python does; Swift and C#
    // write the same `"""` block with the C family's comments around it.
    let (heredoc, long_bracket, template, line_comment): (bool, bool, bool, &[u8]) = match kind {
        Kind::Python | Kind::Elixir => (true, false, false, b"#"),
        Kind::Lua => (false, true, false, b"--"),
        Kind::Zig => (false, false, false, b"//"),
        Kind::Swift | Kind::CSharp => (true, false, true, b"//"),
        _ => (false, false, true, b"//"),
    };
    // The two forms one language each has: C#'s verbatim string, which closes on a `"` that no
    // second `"` follows, since `""` is how it writes a quote, and PHP's `<<<ID`, which closes on
    // the line that repeats its label.
    let (verbatim_strings, labelled) = (kind == Kind::CSharp, kind == Kind::Php);
    let b = text.as_bytes();
    let mut out = vec![false];
    // The multi-line literal the scan is in, by its closing bytes, and, for a long bracket, the
    // number of `=` its closer carries; a one-line quote. A heredoc has no closing bytes at all:
    // `label` holds the word its last line repeats, and `verbatim` marks a `@"…"`.
    let (mut block, mut level, mut quote, mut i): (Option<&[u8]>, usize, Option<u8>, usize) =
        (None, 0, None, 0);
    let (mut verbatim, mut label) = (false, Vec::new());
    // A long bracket opening at `at` — `[[` or `[==[`, behind `--` or not: how many `=` it
    // carries, and how far past `at` its second `[` sits. A `[` that opens nothing, as the one in
    // the `\[[A-Za-z]\+\]` of a Vim regex, is no opener, so the `[=[` around it has to be read.
    let opens = |at: usize| -> Option<(usize, usize)> {
        let open = if b[at..].starts_with(b"--[") {
            at + 2
        } else {
            at
        };
        if b.get(open) != Some(&b'[') {
            return None;
        }
        let eq = b[open + 1..].iter().take_while(|&&c| c == b'=').count();
        (b.get(open + 1 + eq) == Some(&b'[')).then_some((eq, open + 1 + eq - at))
    };
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            quote = None;
            // A heredoc ends on the line that repeats its label, as `ID;`, `ID,` or `ID)`.
            if !label.is_empty() {
                let rest = &b[i + 1..];
                let word = rest
                    .iter()
                    .position(|c| !c.is_ascii_whitespace())
                    .map_or(rest, |n| &rest[n..]);
                if word.starts_with(&label[..])
                    && !word[label.len()..]
                        .first()
                        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
                {
                    label.clear();
                    block = None;
                }
            }
            out.push(block.is_some());
        } else if let Some(end) = block {
            // A long bracket closes on `]`, the `=` its opener carried, and `]`; a verbatim
            // string on a `"` that no second `"` follows; a heredoc only on its label, above.
            let doubled = verbatim && c == b'"' && b.get(i + 1) == Some(&b'"');
            let closes = if long_bracket {
                c == b']'
                    && b[i + 1..]
                        .iter()
                        .take(level)
                        .filter(|&&c| c == b'=')
                        .count()
                        == level
                    && b.get(i + 1 + level) == Some(&b']')
            } else {
                label.is_empty()
                    && !doubled
                    && b[i..].starts_with(end)
                    && (end.len() > 1 || b[i - 1] != b'\\')
            };
            if closes {
                block = None;
                verbatim = false;
                i += if long_bracket {
                    level + 1
                } else {
                    end.len() - 1
                };
            } else if doubled {
                i += 1;
            }
        } else if let Some(q) = quote {
            if c == b'\\' {
                i += 1;
            } else if c == q {
                quote = None;
            }
        } else if heredoc
            && (b[i..].starts_with(b"\"\"\"") || (!template && b[i..].starts_with(b"'''")))
        {
            // `'''` is Python's and Elixir's alone; Swift and C# write the block with `"` only.
            block = Some(if c == b'"' { b"\"\"\"" } else { b"'''" });
            i += 2;
        } else if verbatim_strings && b[i..].starts_with(b"@\"") {
            block = Some(b"\"");
            verbatim = true;
            i += 1;
        } else if labelled && b[i..].starts_with(b"<<<") {
            // `<<<SQL`, `<<<"SQL"` or `<<<'SQL'`: the label is what ends it.
            let word: Vec<u8> = b[i + 3..]
                .iter()
                .skip_while(|c| **c == b'"' || **c == b'\'')
                .take_while(|c| c.is_ascii_alphanumeric() || **c == b'_')
                .copied()
                .collect();
            if !word.is_empty() {
                i += 2;
                block = Some(b"");
                label = word;
            }
        } else if let Some((eq, skip)) = long_bracket.then(|| opens(i)).flatten() {
            block = Some(b"]]");
            level = eq;
            i += skip;
        } else if template && c == b'`' {
            // ponytail: `/`/` is a regex, told by the slash in front; a division by a template
            // is not written.
            if i == 0 || b[i - 1] != b'/' {
                block = Some(b"`");
            }
        } else if template && b[i..].starts_with(b"/*") {
            block = Some(b"*/");
            i += 1;
        } else if c == b'"' || c == b'\'' {
            quote = Some(c);
        } else if b[i..].starts_with(line_comment) {
            while i + 1 < b.len() && b[i + 1] != b'\n' {
                i += 1;
            }
        }
        i += 1;
    }
    out
}

/// The bytes of `s` a scan for brackets and separators reads, with their indexes. String literals
/// are skipped; a comment (`#` in Python, `//` elsewhere) yields its first byte as `0` and is
/// skipped to the end of its line.
fn code(kind: Kind, s: &str) -> impl Iterator<Item = (usize, u8)> + '_ {
    let b = s.as_bytes();
    let (mut quote, mut comment) = (None, false);
    (0..b.len()).filter_map(move |i| {
        let c = b[i];
        if comment {
            comment = c != b'\n';
            return None;
        }
        if let Some(q) = quote {
            if c == q && b[i - 1] != b'\\' {
                quote = None;
            }
            return None;
        }
        match c {
            b'"' | b'\'' | b'`' => {
                quote = Some(c);
                None
            }
            b'#' if kind == Kind::Python => {
                comment = true;
                Some((i, 0))
            }
            b'/' if kind != Kind::Python && b.get(i + 1) == Some(&b'/') => {
                comment = true;
                Some((i, 0))
            }
            _ => Some((i, c)),
        }
    })
}

/// `s` without its comments.
fn uncommented(kind: Kind, s: &str) -> String {
    let (mut out, mut from) = (String::with_capacity(s.len()), 0);
    for (i, _) in code(kind, s).filter(|(_, c)| *c == 0) {
        out.push_str(&s[from..i]);
        from = s[i..].find('\n').map_or(s.len(), |n| i + n);
    }
    out.push_str(&s[from..]);
    out
}

/// The byte index past the bracket that closes the one at `open`; `None` when `s` ends first.
fn close_of(kind: Kind, s: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in code(kind, s).skip_while(|(i, _)| *i < open) {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// `s` cut at every `sep` outside brackets and strings; TypeScript's generics `<…>` count as
/// brackets. An `=` in `=>`, `==`, `<=`, `>=` or `!=` is no separator.
fn split_top(kind: Kind, s: &str, sep: u8) -> Vec<&str> {
    let b = s.as_bytes();
    let (mut depth, mut start, mut out) = (0i32, 0, Vec::new());
    for (i, c) in code(kind, s) {
        let arrow = c == b'>' && i > 0 && b[i - 1] == b'=';
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b'<' if kind == Kind::TsJs => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'>' if kind == Kind::TsJs && !arrow => depth -= 1,
            c if c == sep && depth == 0 => {
                let part_of = |j: usize| b.get(j).is_some_and(|c| b"=<>!".contains(c));
                if sep != b'=' || !(part_of(i + 1) || (i > 0 && part_of(i - 1))) {
                    out.push(&s[start..i]);
                    start = i + 1;
                }
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

/// The text inside the bracket that opens at byte `open` of line `at`, over the lines it spans and
/// without comments, and the index of the line it closes on with what follows it there.
fn group<'a>(
    kind: Kind,
    lines: &[&'a str],
    at: usize,
    open: usize,
) -> Option<(String, usize, &'a str)> {
    // ponytail: a bracket still open 2000 lines on is not read; fastapi's signatures run to 350.
    let text = lines[at..lines.len().min(at + 2000)].join("\n");
    let end = close_of(kind, &text, open)?;
    let line_start = text[..end].rfind('\n').map_or(0, |n| n + 1);
    let last = at + text[..end].matches('\n').count();
    let inner = uncommented(kind, &text[open + 1..end - 1]);
    Some((inner, last, &lines[last][end - line_start..]))
}

/// Whether `word` stands in `s` as a whole name, not as the tail of `a.word`.
fn names(s: &str, word: &str) -> bool {
    s.match_indices(word).any(|(i, _)| {
        let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.';
        !s[..i].ends_with(is_name) && !s[i + word.len()..].starts_with(is_name)
    })
}

/// The value an expression gives a name: a call, a construction, another name, or unknown.
fn value_of(kind: Kind, expr: &str) -> Value {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:await\s+)?(new\s+)?([A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*)*)\s*(?:<[^()]*>)?\s*\(").unwrap()
    });
    static LITERAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(&)?([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)?)\s*(?:\[[^\]]*\])?\s*\{").unwrap()
    });
    static NAME: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^[A-Za-z_$][\w$]*$").unwrap());
    static ASSERTION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^[A-Za-z_][\w.]*\.\(\s*(\*?[A-Za-z_][\w.]*)\s*\)$").unwrap()
    });
    let e = uncommented(kind, expr);
    let e = e.trim().trim_end_matches(';').trim_end();
    // What comes after the bracket that opens at `open` must be nothing, or the next lines.
    let ends = |open: usize| close_of(kind, e, open).is_none_or(|end| e[end..].trim().is_empty());
    // A cast writes the type (#100): `x as T` (the last one of `x as unknown as T`), Go's
    // `i.(T)`, and below Python's `cast(T, x)`.
    if kind == Kind::TsJs {
        let mut depth = 0i32;
        // The first and the last ` as ` outside brackets.
        let mut cast: Option<(usize, usize)> = None;
        for (i, c) in code(kind, e) {
            match c {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                b' ' if depth == 0 && e[i..].starts_with(" as ") => {
                    cast = Some((cast.map_or(i, |(first, _)| first), i + 4));
                }
                _ => {}
            }
        }
        // `as` binds tighter than `?:`, `||`, `??` and `=>`: only one operand in front of it, a
        // name, a call or a literal, is what the cast types.
        let operand = |head: &str| {
            let head = head.trim_start_matches("await ").trim_start_matches("new ");
            let mut depth = 0i32;
            code(kind, head).all(|(_, c)| {
                match c {
                    b'(' | b'[' | b'{' => depth += 1,
                    b')' | b']' | b'}' => depth -= 1,
                    _ => {}
                }
                c != b' ' || depth > 0
            })
        };
        if let Some((first, i)) = cast {
            return match operand(&e[..first]) {
                true => Value::Type(e[i..].trim().to_owned()),
                false => Value::Unknown,
            };
        }
    }
    if kind == Kind::Go
        && let Some(c) = ASSERTION.captures(e)
    {
        return Value::Type(c[1].to_owned());
    }
    if let Some(c) = CALL.captures(e) {
        let open = c.get(0).unwrap().end() - 1;
        let name = c[2].to_owned();
        return match (ends(open), c.get(1).is_some()) {
            (false, _) => Value::Unknown,
            (true, true) if kind == Kind::TsJs => Value::New(name),
            (true, false) if kind == Kind::Go && name == "new" => {
                let inner = &e[open + 1..e.len().saturating_sub(1)];
                Value::New(inner.trim().to_owned())
            }
            (true, false) if kind == Kind::Python && matches!(&*name, "cast" | "typing.cast") => {
                let inner = &e[open + 1..e.len().saturating_sub(1)];
                Value::Cast(name, split_top(kind, inner, b',')[0].trim().to_owned())
            }
            // `make([]*Repo, 0, n)` writes the type of what it makes.
            (true, false) if kind == Kind::Go && name == "make" => {
                let inner = &e[open + 1..e.len().saturating_sub(1)];
                Value::Type(split_top(kind, inner, b',')[0].trim().to_owned())
            }
            (true, false) => Value::Call(name),
            (true, true) => Value::Unknown,
        };
    }
    if kind == Kind::Go
        && let Some(c) = LITERAL.captures(e)
        && ends(c.get(0).unwrap().end() - 1)
    {
        return Value::New(c[2].to_owned());
    }
    // A slice, array or map literal writes its type in front of its `{`: `[]Repo{…}`.
    if kind == Kind::Go
        && (e.starts_with('[') || e.starts_with("map["))
        && let Some(end) = e.find('[').and_then(|i| close_of(kind, e, i))
        && let Some(open) = e[end..].find('{').map(|i| end + i)
        && ends(open)
    {
        return Value::Type(e[..open].trim().to_owned());
    }
    if NAME.is_match(e) && !matches!(e, "None" | "null" | "undefined" | "nil" | "this" | "self") {
        return Value::Name(e.to_owned());
    }
    Value::Unknown
}

/// The declarations of `name` that 1-based `line` of `text`, a file of `kind`, reads, with what
/// each gives it: those of the innermost scope that declares it, which hides the scopes around
/// it (#100). Every one of that scope counts, and the caller trusts them only when they agree.
///
/// - Python: a name belongs to its function, so its parameters and every binding in the body
///   count, before the cursor or after it; a function that binds none reads the enclosing
///   function's, then the module's. Nested functions and classes are scopes of their own. A comprehension or a `lambda` counts
///   on the cursor line only. `self` / `cls` is the class a method sits in, and so is `super`
///   (what [`qualifier`] makes of `super()`): the caller starts above that class.
/// - TypeScript and Go: `const`, `let` and `:=` belong to their block, so the declarations above
///   the cursor count, in the nearest block around it that has any, told by indentation: the
///   statements at the block's level and what its header binds (a function's parameters, a Go
///   receiver). Only what a header binds for the block under it hides: another function on its
///   lines, or on the cursor's own line, binds without hiding. `this` is the class around it, unless a
///   `function` or an object literal comes first, and `super` reads as `this` does.
pub fn bindings(kind: Kind, text: &str, line: usize, name: &str) -> Vec<Binding> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    match kind {
        Kind::Python => python_bindings(&lines, at, name),
        Kind::TsJs | Kind::Go => block_bindings(kind, &lines, at, name),
        _ => Vec::new(),
    }
}

/// Whether the word at `range` of 1-based `line` names a keyword argument of a Python call:
/// `recipe_yield=…` behind a `(` or a `,`, or at the start of a line that continues a call. It
/// names a parameter of whatever is called, and no variable of that spelling.
pub fn keyword_argument(text: &str, line: usize, range: &Range<usize>) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(l) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let after = l[range.end..].trim_start();
    let before = l[..range.start].trim_end();
    after.starts_with('=')
        && !after.starts_with("==")
        && (before.ends_with(['(', ','])
            || (before.is_empty() && continued(Kind::Python, &lines, line - 1)))
}

/// Whether line `i` continues the statement above it: that line ends in an open bracket, a comma
/// or a backslash.
fn continued(kind: Kind, lines: &[&str], i: usize) -> bool {
    lines[..i]
        .iter()
        .rev()
        .map(|l| uncommented(kind, l))
        .find(|t| !t.trim().is_empty() && !comment(kind, t.trim()))
        .is_some_and(|t| t.trim_end().ends_with(['(', '[', '{', ',', '\\']))
}

fn python_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    static DEF: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(?:async\s+)?def\s+(\w+)\s*\(").unwrap());
    static SCOPE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(?:(?:async\s+)?def|class)\s+(\w+)").unwrap());
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let annotated = rule(format!(r"^{n}\s*:\s*([^=]+?)\s*(?:=.*)?$"));
    let assigned = rule(format!(r"^{n}\s*=\s*([^=].*)$"));
    // An import binds the names it imports, not the modules on their path: `from .guild import
    // Guild` leaves a parameter `guild` alone.
    let unknown = rule(format!(
        r"^(?:async\s+)?for\s+[^=]*\b{n}\b.*\sin\s|\bas\s+{n}\b|\b{n}\s*:=|^(?:global|nonlocal)\s.*\b{n}\b|^from\s+\S+\s+import\s.*\b{n}\b|^import\s(?:.*[\s,])?{n}\b"
    ));
    let inline = rule(format!(
        r"\bfor\s+[^=]*?\b{n}\b[^=]*?\s+in\b|\blambda\b[^:]*\b{n}\b"
    ));
    // A plain loop over a plain name; `async for`, a tuple target and a call are unknown.
    let element = rule(format!(r"^for\s+{n}\s+in\s+([A-Za-z_]\w*)\s*:"));
    // `a, repo = …` or `(a, repo) = …`, not the keyword argument of a call.
    let tuple = Regex::new(r"^(\(?[\w\s,.*\[\]]+\)?)\s*=[^=]").unwrap();

    // The functions around the cursor, innermost first; `None` is the module.
    let mut scopes = Vec::new();
    let mut depth = indent(lines[at]);
    for i in (0..at).rev() {
        let t = lines[i].trim_start();
        if depth == 0 {
            break;
        }
        // A closer at a lower indent ends a multi-line signature; its `def` is further up.
        if t.is_empty() || t.starts_with(['#', ')', ']']) || indent(lines[i]) >= depth {
            continue;
        }
        depth = indent(lines[i]);
        if DEF.is_match(t) {
            scopes.push(Some(i));
        }
    }
    scopes.push(None);

    let literal = literal_lines(Kind::Python, &lines.join("\n"));
    let mut out = Vec::new();
    // `super()` is the class of the method it is written in, and nothing in a function inside
    // that method, where the call has no arguments to find.
    if name == "super" {
        if let Some(line) = scopes[0].and_then(|d| python_class_of(lines, d)) {
            let value = Value::Class(line);
            out.push(Binding { line, value });
        }
        return out;
    }
    for scope in scopes {
        let (start, base) = match scope {
            Some(d) => {
                let open = lines[d].find('(').expect("a def has a parameter list");
                let Some((params, end, _)) = group(Kind::Python, lines, d, open) else {
                    continue;
                };
                python_params(lines, d, &params, name, &mut out);
                (end + 1, Some(indent(lines[d])))
            }
            None => (0, None),
        };
        // A nested function or class is skipped down to the lines back at its indent.
        let mut skip: Option<usize> = None;
        for (i, l) in lines.iter().enumerate().skip(start) {
            let code = uncommented(Kind::Python, l);
            let t = code.trim();
            // A docstring's example binds nothing.
            if t.is_empty() || literal[i] {
                continue;
            }
            let ind = indent(l);
            if base.is_some_and(|b| ind <= b) {
                break;
            }
            if let Some(k) = skip {
                if ind > k || t.starts_with([')', ']']) {
                    continue;
                }
                skip = None;
            }
            if let Some(c) = SCOPE.captures(t) {
                if &c[1] == name {
                    out.push(Binding {
                        line: i + 1,
                        value: Value::Unknown,
                    });
                }
                skip = Some(ind);
                continue;
            }
            let value = if let Some(c) = element.captures(t) {
                Some(Value::Element(c[1].to_owned()))
            } else if unknown.is_match(t) || (i == at && inline.is_match(t)) {
                Some(Value::Unknown)
            } else if continued(Kind::Python, lines, i) {
                None
            } else if let Some(c) = annotated.captures(t) {
                Some(Value::Type(c[1].to_owned()))
            } else if let Some(c) = assigned.captures(t) {
                Some(value_of(Kind::Python, &c[1]))
            } else {
                tuple
                    .captures(t)
                    .filter(|c| c[1].contains(',') && names(&c[1], name))
                    .map(|_| Value::Unknown)
            };
            if let Some(value) = value {
                out.push(Binding { line: i + 1, value });
            }
        }
        // The innermost function that binds the name is the one the cursor reads.
        if !out.is_empty() {
            break;
        }
    }
    out
}

/// The binding of `name` among the parameters of the `def` on line `d`: its annotation, the class
/// for the first parameter of a method, else unknown.
fn python_params(lines: &[&str], d: usize, params: &str, name: &str, out: &mut Vec<Binding>) {
    for (i, p) in split_top(Kind::Python, params, b',')
        .into_iter()
        .enumerate()
    {
        let head = split_top(Kind::Python, p, b'=')[0]
            .trim()
            .trim_start_matches('*');
        let (pname, annotation) = match head.split_once(':') {
            Some((pname, t)) => (pname.trim(), Some(t.trim())),
            None => (head, None),
        };
        if pname != name {
            continue;
        }
        let value = match annotation {
            Some(t) => Value::Type(t.to_owned()),
            None if i == 0 => python_class_of(lines, d).map_or(Value::Unknown, Value::Class),
            None => Value::Unknown,
        };
        out.push(Binding { line: d + 1, value });
    }
}

/// The 1-based line of the class the `def` on line `d` is a method of, unless it is a
/// `@staticmethod`.
fn python_class_of(lines: &[&str], d: usize) -> Option<usize> {
    let ind = indent(lines[d]);
    let mut decorators = true;
    for i in (0..d).rev() {
        let t = lines[i].trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if indent(lines[i]) < ind && !t.starts_with([')', ']']) {
            return t.starts_with("class ").then_some(i + 1);
        }
        if decorators && indent(lines[i]) == ind && t.starts_with('@') {
            if t == "@staticmethod" {
                return None;
            }
            continue;
        }
        decorators = false;
    }
    None
}

/// Walks up from line `at` through the blocks around it: a line at the cursor's block level is a
/// statement, a line indented less opens the block the walk is in, and deeper lines belong to
/// blocks already closed.
fn block_bindings(kind: Kind, lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let this = kind == Kind::TsJs && (name == "this" || name == "super");
    let mut out = Vec::new();
    if !this {
        opener_bindings(kind, &uncommented(kind, lines[at]), at + 1, name, &mut out);
    }
    // A raw string, a template or a block comment over several lines declares nothing.
    let literal = literal_lines(kind, &lines.join("\n"));
    let mut depth = indent(lines[at]);
    let mut i = at;
    // Whether a declaration of the block the walk is in, or of its header, has been found.
    let mut scoped = false;
    // The types of the innermost Go `case` around the cursor, for the `switch v := x.(type)` it
    // may belong to (#100): gofmt writes the two at one indent, so the `switch` is met as a
    // statement right after its `case`.
    let mut arm: Option<Vec<String>> = None;
    let type_switch = Regex::new(&format!(
        r"^switch\s+(?:[^;{{]*;\s*)?{}\s*:=\s*[^;{{]+\.\(type\)\s*\{{$",
        regex::escape(name)
    ))
    .expect("an escaped name keeps the pattern valid");
    while i > 0 {
        i -= 1;
        let code = uncommented(kind, lines[i]);
        let t = code.trim();
        let ind = indent(lines[i]);
        if t.is_empty() || comment(kind, t) || ind > depth || literal[i] {
            continue;
        }
        if ind == depth {
            if !this {
                let before = out.len();
                statement_bindings(kind, t, i + 1, name, &mut out);
                // In `case *Repo:` the variable of a type switch is a `*Repo`; under several
                // types or `default` it is whatever came in. A `switch` met with no `case` on
                // the way up is one the cursor is not in.
                if let Some(types) = arm.as_deref().filter(|_| type_switch.is_match(t)) {
                    let value = match types {
                        [one] => Value::Type(one.clone()),
                        _ => Value::Unknown,
                    };
                    out.push(Binding { line: i + 1, value });
                }
                if t.starts_with("switch ") || t.starts_with("select ") {
                    arm = None;
                }
                scoped |= out.len() > before;
            }
            continue;
        }
        // A header over several lines ends in a closer (`) {`, `} else {`) and starts at the
        // line above that is back at its indent.
        let end = i;
        if t.starts_with([')', '}', ']']) {
            while i > 0 && (lines[i - 1].trim().is_empty() || indent(lines[i - 1]) > ind) {
                i -= 1;
            }
            i = i.saturating_sub(1);
        }
        // Between an `if` and its `} else {` lies a block the cursor is not in: only the two
        // lines are the header, where a signature closed by `) {` or `}: Deps) {` is all of
        // its lines.
        let sibling = ["else", "catch", "finally"]
            .iter()
            .any(|k| t.trim_start_matches('}').trim_start().starts_with(k));
        let header: Vec<&str> = match sibling && end > i {
            true => vec![lines[i].trim(), t],
            false => lines[i..=end].iter().map(|l| l.trim()).collect(),
        };
        let header = uncommented(kind, &header.join("\n"));
        depth = ind.min(indent(lines[i]));
        if !this {
            if kind == Kind::Go {
                let types = header
                    .strip_prefix("case ")
                    .and_then(|h| h.strip_suffix(':'));
                arm = match (types, header.starts_with("default")) {
                    (Some(types), _) => Some(type_list(kind, types)),
                    (None, true) => Some(Vec::new()),
                    (None, false) => arm,
                };
            }
            // The innermost block that declares the name hides the ones around it: a statement
            // of the block the walk leaves here, or what the header binds for that block. A
            // callback elsewhere on the header's lines (`if (xs.some((repo: Repo) => …)) {`,
            // `register((repo: Repo) => repo, {`) is a function the cursor is not in, as one on
            // the cursor's own line may be: its parameters count, and hide nothing.
            if opener_bindings(kind, &header, i + 1, name, &mut out) || scoped {
                break;
            }
        } else if let Some(value) = this_opener(&header, i + 1) {
            out.push(Binding { line: i + 1, value });
            break;
        }
    }
    out
}

/// What `this` is inside the block a header opens: the class it declares, unknown under a
/// `function` or an object literal, `None` for a block `this` passes through (a method, an arrow
/// function, an `if`).
fn this_opener(header: &str, line: usize) -> Option<Value> {
    static CLASS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:(?:export|default|declare|abstract)\s+)*class\b").unwrap()
    });
    if CLASS.is_match(header) {
        return Some(Value::Class(line));
    }
    let before = header.strip_suffix('{').map(str::trim_end);
    // `case X: {` and `default: {` open statements, whatever the colon suggests; the `{` of a
    // `case X: return {` on the same line is a literal's.
    let case = (header.starts_with("case ") || header.starts_with("default"))
        && before.is_some_and(|b| b.ends_with(':'));
    let object = !case
        && before.is_some_and(|b| {
            b.ends_with(['=', '(', '[', ',', '?', ':', '|', '&'])
                || b.ends_with("return")
                || b.ends_with("yield")
                || b.ends_with("default")
        });
    (object || names(header, "function")).then_some(Value::Unknown)
}

/// The bindings of `name` a block's header makes for the lines inside it: the parameters of a
/// function, a method or an arrow, a Go receiver and named results, the variables of a loop, a
/// `catch` or a Go `if x := …;`. Returns whether one of them is made for the block under the
/// header, and so hides the scopes around it.
fn opener_bindings(
    kind: Kind,
    header: &str,
    line: usize,
    name: &str,
    out: &mut Vec<Binding>,
) -> bool {
    static TS_LOOP: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bfor\s*\(\s*(?:const|let|var)\s+([^;]+?)\s+(?:of|in)\s|\bfor\s*\(\s*(?:const|let|var)\s+([^;]*);|\bcatch\s*\(([^)]*)\)").unwrap()
    });
    static TS_ARROW: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(?::\s*[^=]+?)?\s*=>").unwrap());
    static TS_FUNCTION: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\bfunction\*?\s*[\w$]*\s*(?:<[^>]*>)?$").unwrap());
    static TS_METHOD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:(?:public|private|protected|static|readonly|abstract|override|async|get|set)\s+)*\*?#?([\w$]+)\s*(?:<[^>]*>)?$").unwrap()
    });
    static TS_BODY: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(?::.*)?\{").unwrap());
    // What follows the parameters of the function whose body ends the header: a return type,
    // an arrow, the brace.
    static TS_OPENS: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(?::[^(){}]+?)?\s*(?:=>)?\s*\{?\s*$").unwrap());
    static GO_ASSIGN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|\belse\s+)(?:if|for|switch|case)\s+([^;{]*?)\s*:=").unwrap()
    });
    static GO_FUNC: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bfunc\b\s*(?:\(([^)]*)\)\s*)?(?:[A-Za-z_]\w*\s*)?(?:\[[^\]]*\]\s*)?\(")
            .unwrap()
    });
    let unknown = |out: &mut Vec<Binding>| {
        out.push(Binding {
            line,
            value: Value::Unknown,
        })
    };
    let n = regex::escape(name);
    let element = |p: String| {
        let rule = Regex::new(&p).expect("an escaped name keeps the pattern valid");
        let value = Value::Element(rule.captures(header)?[1].to_owned());
        Some(Binding { line, value })
    };
    // Whether a binding is made for the block under the header: by the loop, the `catch` or the
    // Go statement the header is, or by the function whose body ends it. A callback elsewhere on
    // these lines, `if (xs.some((repo: Repo) => …)) {`, binds for a body the cursor is not in.
    let mut own = false;
    match kind {
        Kind::TsJs => {
            let starts = |l: &str| {
                let l = l.trim_start_matches('}').trim_start();
                l.starts_with("for") || l.starts_with("catch")
            };
            let is_loop = starts(header) || header.rsplit('\n').next().is_some_and(starts);
            let count = out.len();
            // `for (const r of repos)` over a plain name hands out elements; `in` hands out keys.
            if let Some(b) = element(format!(
                r"\bfor\s*\(\s*(?:const|let|var)\s+{n}\s+of\s+([A-Za-z_$][\w$]*)\s*\)"
            )) {
                out.push(b);
            } else if TS_LOOP
                .captures_iter(header)
                .any(|c| c.iter().skip(1).flatten().any(|m| names(m.as_str(), name)))
            {
                unknown(out);
            }
            own |= is_loop && out.len() > count;
            for (open, _) in header.match_indices('(') {
                let Some(close) = close_of(kind, header, open) else {
                    continue;
                };
                let (before, after) = (header[..open].trim_end(), header[close..].trim_start());
                let method = TS_METHOD.captures(before).is_some_and(|c| {
                    !matches!(
                        &c[1],
                        "if" | "for" | "while" | "switch" | "catch" | "with" | "return" | "super"
                    ) && TS_BODY.is_match(after)
                });
                if TS_ARROW.is_match(after) || TS_FUNCTION.is_match(before) || method {
                    let count = out.len();
                    ts_params(&header[open + 1..close - 1], line, name, out);
                    own |= out.len() > count && TS_OPENS.is_match(after);
                }
            }
            let arrow = Regex::new(&format!(
                r"(?:^|[^\w$.]){}\s*=>(\s*\{{?\s*$)?",
                regex::escape(name)
            ))
            .expect("an escaped name keeps the pattern valid");
            if let Some(c) = arrow.captures_iter(header).last() {
                unknown(out);
                own |= c.get(1).is_some();
            }
        }
        Kind::Go => {
            let count = out.len();
            // The second variable of a `range` over a plain name is an element of a slice, an
            // array or a map; the first is an index or a key.
            if let Some(b) = element(format!(
                r"^for\s+[A-Za-z_]\w*\s*,\s*{n}\s*:=\s*range\s+([A-Za-z_]\w*)\s*\{{"
            )) {
                out.push(b);
            } else if GO_ASSIGN.captures_iter(header).any(|c| names(&c[1], name)) {
                unknown(out);
            }
            own |= out.len() > count;
            if let Some(c) = GO_FUNC.captures_iter(header).last() {
                let count = out.len();
                if let Some(receiver) = c.get(1) {
                    go_params(receiver.as_str(), line, name, out);
                }
                let open = c.get(0).unwrap().end() - 1;
                if let Some(close) = close_of(kind, header, open) {
                    go_params(&header[open + 1..close - 1], line, name, out);
                    let results = header[close..].trim_start();
                    if results.starts_with('(')
                        && let Some(end) = close_of(kind, results, 0)
                    {
                        go_params(&results[1..end - 1], line, name, out);
                    }
                    // Between the parameters and the `{` that ends the header stand results
                    // only: another brace there closes this function or opens a literal.
                    let body = results.trim_end().strip_suffix('{');
                    own |= out.len() > count && body.is_some_and(|r| !r.contains(['{', '}']));
                }
            }
        }
        _ => {}
    }
    own
}

/// The binding of `name` among TypeScript parameters: its annotation, else its default value.
fn ts_params(params: &str, line: usize, name: &str, out: &mut Vec<Binding>) {
    static MODS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:(?:public|private|protected|readonly|override)\s+)*(?:\.\.\.)?").unwrap()
    });
    for p in split_top(Kind::TsJs, params, b',') {
        let p = MODS.replace(p.trim(), "");
        if p.starts_with(['{', '[']) {
            if names(&p, name) {
                out.push(Binding {
                    line,
                    value: Value::Unknown,
                });
            }
            continue;
        }
        let parts = split_top(Kind::TsJs, &p, b'=');
        let (pname, annotation) = match parts[0].split_once(':') {
            Some((pname, t)) => (pname, Some(t.trim())),
            None => (parts[0], None),
        };
        if pname.trim().trim_end_matches('?') != name {
            continue;
        }
        let value = match (annotation, parts.get(1)) {
            (Some(t), _) => Value::Type(t.to_owned()),
            (None, Some(default)) => value_of(Kind::TsJs, default),
            (None, None) => Value::Unknown,
        };
        out.push(Binding { line, value });
    }
}

/// The binding of `name` among Go parameters, a receiver or named results: `a, b *T` gives both
/// names the type written after the last of them. A list of bare types names nothing.
fn go_params(params: &str, line: usize, name: &str, out: &mut Vec<Binding>) {
    let items: Vec<&str> = split_top(Kind::Go, params, b',')
        .into_iter()
        .map(str::trim)
        .filter(|i| !i.is_empty())
        .collect();
    if !items.iter().any(|i| i.contains(char::is_whitespace)) {
        return;
    }
    let mut ty: Option<&str> = None;
    for item in items.iter().rev() {
        let pname = match item.split_once(char::is_whitespace) {
            Some((pname, t)) => {
                ty = Some(t.trim());
                pname
            }
            None => item,
        };
        if pname == name {
            let value = match ty {
                Some(t) if !t.starts_with("...") => Value::Type(t.to_owned()),
                _ => Value::Unknown,
            };
            out.push(Binding { line, value });
        }
    }
}

/// The binding of `name` a statement at a block's level makes: a TypeScript `const` / `let` /
/// `var`, a Go `:=` or `var`. A destructuring, a second name of a Go `:=` or a declaration the
/// rules cannot type is unknown.
fn statement_bindings(kind: Kind, t: &str, line: usize, name: &str, out: &mut Vec<Binding>) {
    static GO_SHORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)*)\s*:=\s*(.*)$").unwrap()
    });
    static GO_VAR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"^var\s+([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)*)(?:\s+([^=]+?))?\s*(?:=\s*(.*))?$",
        )
        .unwrap()
    });
    static TS_DESTRUCTURE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:export\s+)?(?:const|let|var)\s*[\[{]([^=]*)").unwrap()
    });
    let n = regex::escape(name);
    let value = match kind {
        Kind::TsJs => {
            let declared = Regex::new(&format!(
                r"^(?:export\s+)?(?:declare\s+)?(?:const|let|var)\s+{n}\s*!?\s*(?::\s*([^=]+?))?\s*(?:=\s*(.*?))?\s*;?$"
            ))
            .expect("an escaped name keeps the pattern valid");
            let named = Regex::new(&format!(
                r"^(?:export\s+)?(?:default\s+)?(?:declare\s+)?(?:abstract\s+)?(?:async\s+)?(?:function\*?|class|enum|namespace|interface|type)\s+{n}(?:[^\w$]|$)"
            ))
            .expect("an escaped name keeps the pattern valid");
            if let Some(c) = declared.captures(t) {
                match (c.get(1), c.get(2).filter(|v| !v.as_str().is_empty())) {
                    (Some(ty), _) => Value::Type(ty.as_str().to_owned()),
                    (None, Some(v)) => value_of(kind, v.as_str()),
                    (None, None) => Value::Unknown,
                }
            } else if named.is_match(t)
                || TS_DESTRUCTURE
                    .captures(t)
                    .is_some_and(|c| names(&c[1], name))
            {
                Value::Unknown
            } else {
                return;
            }
        }
        Kind::Go => {
            let list =
                |s: &str| -> Vec<String> { s.split(',').map(|p| p.trim().to_owned()).collect() };
            if let Some(c) = GO_SHORT.captures(t) {
                match list(&c[1]).iter().position(|p| p == name) {
                    Some(0) => value_of(kind, &c[2]),
                    Some(_) => Value::Unknown,
                    None => return,
                }
            } else if let Some(c) = GO_VAR.captures(t) {
                match (
                    list(&c[1]).iter().position(|p| p == name),
                    c.get(2),
                    c.get(3),
                ) {
                    (None, ..) => return,
                    (Some(_), Some(ty), _) => Value::Type(ty.as_str().to_owned()),
                    (Some(0), None, Some(v)) => value_of(kind, v.as_str()),
                    _ => Value::Unknown,
                }
            } else if t
                .strip_prefix("const ")
                // The names, not the value: `const csp = "… http://…"` declares no `http`.
                .is_some_and(|rest| names(rest.split('=').next().unwrap_or(rest), name))
            {
                Value::Unknown
            } else {
                return;
            }
        }
        _ => return,
    };
    out.push(Binding { line, value });
}

/// Whether the Go function around 1-based `line` of `text` may declare `name` (#100): the name
/// stands as a whole word, outside strings and comments, somewhere from the function's `func`
/// line down to the line above `line`, other than in front of a `.`. A declaration never writes
/// its name in front of a `.`, whatever its form, so `false` proves there is no local of the
/// name, which an empty [`bindings`] does not: its walk misses a header with a function-typed
/// parameter, a `var (` block inside a function, the lines above a label. Uses of the name as
/// an argument say `true` as well, and the caller then proves nothing.
pub fn go_may_declare(text: &str, line: usize, name: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(Kind::Go, text);
    let at = line.saturating_sub(1).min(lines.len());
    let mentions = |l: &str| {
        let code: String = {
            let mut bytes = vec![b' '; l.len()];
            for (i, c) in code(Kind::Go, l) {
                bytes[i] = if c == 0 { b' ' } else { c };
            }
            String::from_utf8_lossy(&bytes).into_owned()
        };
        code.match_indices(name).any(|(i, m)| {
            let ident = |c: char| c.is_alphanumeric() || c == '_';
            !code[..i].ends_with(ident)
                && !code[i + m.len()..].starts_with(ident)
                && !code[i + m.len()..].trim_start().starts_with('.')
        })
    };
    for i in (0..at).rev() {
        let l = lines[i];
        if literal[i] || l.trim().is_empty() {
            continue;
        }
        if mentions(l) {
            return true;
        }
        // The function starts at the `func` in column 0. Above it a header's closer `) error {`
        // and a label still belong to the function; anything else in column 0 is the package's,
        // and so is the cursor.
        let label = l.trim_end().ends_with(':') && !l.contains(' ');
        if indent(l) == 0 && !comment(Kind::Go, l) && !label {
            if l.starts_with("func") {
                return false;
            }
            if !(l.starts_with(')') && l.trim_end().ends_with('{')) {
                return false;
            }
        }
    }
    false
}

/// What the Go file `text` declares `name` as at the level of its package: `var name T`,
/// `var name = …`, alone or as a line of a `var (` block, anywhere in the file (#100). The scope
/// walk of [`bindings`] reads the open file upwards only, so another file of the package and the
/// lines below the cursor are read through this.
pub fn package_bindings(text: &str, name: &str) -> Vec<Binding> {
    let literal = literal_lines(Kind::Go, text);
    let mut out = Vec::new();
    let mut block = false;
    // The indent of the open block's entries: that of its first line.
    let mut level = None;
    for (i, l) in text.lines().enumerate() {
        let code = uncommented(Kind::Go, l);
        let t = code.trim();
        if literal[i] || t.is_empty() {
            continue;
        }
        match (indent(l), block) {
            (0, _) if t == "var (" => (block, level) = (true, None),
            (0, _) => {
                block = false;
                statement_bindings(Kind::Go, t, i + 1, name, &mut out);
            }
            // gofmt aligns the `=` of a block with spaces, which one `var` line never has. A
            // deeper line belongs to an entry's value or type, `cfg struct {` / `repo *Repo`.
            (n, true) if Some(n) == level.or(Some(n)) => {
                level = Some(n);
                let words: Vec<&str> = t.split_whitespace().collect();
                let t = format!("var {}", words.join(" "));
                statement_bindings(Kind::Go, &t, i + 1, name, &mut out);
            }
            _ => {}
        }
    }
    out
}

/// The lines of the body of the class, interface or struct declared on line `k`: past a Python
/// header over several lines, up to the first line back at the declaration's indent.
fn body_of(kind: Kind, lines: &[&str], k: usize) -> std::ops::Range<usize> {
    let start = match (kind, lines[k].find('(')) {
        (Kind::Python, Some(open)) => {
            group(kind, lines, k, open).map_or(k + 1, |(_, end, _)| end + 1)
        }
        _ => k + 1,
    };
    let base = indent(lines[k]);
    let end = (start..lines.len())
        .find(|&i| {
            let t = lines[i].trim();
            // A lone `{` at the declaration's own indentation opens its body, as prettier
            // writes a wrapped class header; it does not end it.
            !t.is_empty() && t != "{" && !comment(kind, t) && indent(lines[i]) <= base
        })
        .unwrap_or(lines.len());
    start..end.max(start)
}

/// The declarations of the field `name` of the class, interface or struct declared on 1-based
/// `decl` of `text`:
/// - Python: `name: T` or `name = …` in the class body, `self.name: T = …` or `self.name = …` in
///   its methods, a `def name` under `@property`, `@cached_property` or
///   `@functools.cached_property`, read as its `-> T` (a setter declares nothing);
/// - TypeScript: a member `name: T` or `name = …` behind any modifiers, a constructor parameter
///   with one (`private name: T`), `this.name = …`, a getter `get name(): T`;
/// - Go: a struct field `name T` or `a, name T`, and an embedded `*Name` under its type's name.
pub fn field_bindings(kind: Kind, text: &str, decl: usize, name: &str) -> Vec<Binding> {
    static GO_FIELD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)*)\s+(\S.*)$").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = decl.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    let body = body_of(kind, &lines, k);
    let Some(base) = lines[body.clone()]
        .iter()
        .find(|l| !l.trim().is_empty() && !comment(kind, l.trim()))
        .map(|l| indent(l))
    else {
        return Vec::new();
    };
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    // A docstring or a block comment declares nothing, whatever its lines look like.
    let literal = literal_lines(kind, text);
    let body = body.filter(|&i| !literal.get(i).copied().unwrap_or(false));
    let mut out = Vec::new();
    let mut push = |i: usize, value: Value| out.push(Binding { line: i + 1, value });
    match kind {
        Kind::Python => {
            let annotated = rule(format!(r"^(self\.)?{n}\s*:\s*([^=]+?)\s*(?:=.*)?$"));
            let assigned = rule(format!(r"^(self\.)?{n}\s*=\s*([^=].*)$"));
            // A tuple target (`self.a, self.repo = …`), not an argument of a call.
            let unknown = rule(format!(
                r"^\(?[\w\s,.*\[\]]*,[\w\s,.*\[\]]*\bself\.{n}\b[\w\s,.*\[\]]*\)?\s*=[^=]|^\(?[\w\s.*\[\]]*\bself\.{n}\s*,[\w\s,.*\[\]]*\)?\s*=[^=]|\bas\s+self\.{n}\b|^for\s+.*\bself\.{n}\b.*\sin\s"
            ));
            let def = rule(format!(r"^def\s+{n}\s*\("));
            // The decorators right above line `i` at its indent include a property's.
            let property = |i: usize, ind: usize| {
                lines[..i]
                    .iter()
                    .rev()
                    .map(|l| uncommented(kind, l))
                    .filter(|l| !l.trim().is_empty())
                    .take_while(|l| indent(l) == ind && l.trim_start().starts_with('@'))
                    .any(|l| {
                        matches!(
                            l.trim(),
                            "@property" | "@cached_property" | "@functools.cached_property"
                        )
                    })
            };
            let mut skip: Option<usize> = None;
            for i in body {
                let (code, ind) = (uncommented(kind, lines[i]), indent(lines[i]));
                let t = code.trim();
                if t.is_empty() || skip.is_some_and(|k| ind > k) {
                    continue;
                }
                skip = t.starts_with("class ").then_some(ind);
                if skip.is_some() || continued(kind, &lines, i) {
                    continue;
                }
                // `x: T` belongs to the class body, `self.x: T` to a method.
                let right = |c: &regex::Captures| c.get(1).is_some() == (ind > base);
                if unknown.is_match(t) {
                    push(i, Value::Unknown);
                } else if let Some(c) = annotated.captures(t).filter(right) {
                    push(i, Value::Type(c[2].to_owned()));
                } else if let Some(c) = assigned.captures(t).filter(right) {
                    push(i, value_of(kind, &c[2]));
                } else if ind == base && def.is_match(t) && property(i, ind) {
                    push(i, returns(kind, text, i + 1).unwrap_or(Value::Unknown));
                }
            }
        }
        Kind::TsJs => {
            let member = rule(format!(
                r"^(?:(?:public|private|protected|readonly|static|declare|override|abstract|accessor)\s+)*{n}\s*[?!]?\s*(?::\s*([^=;]+?))?\s*(?:=\s*(.+?))?\s*;?$"
            ));
            let param = rule(format!(
                r"(?:^|[(,])\s*(?:@[\w$.]+(?:\([^)]*\))?\s*)*(?:(?:public|private|protected|readonly|override)\s+)+{n}\s*\??\s*:\s*([^,)=]+)"
            ));
            let assigned = rule(format!(r"^this\.{n}\s*=\s*([^=].*?)\s*;?$"));
            let getter = rule(format!(
                r"^(?:(?:public|private|protected|static|override|abstract|declare)\s+)*get\s+{n}\s*\(\s*\)\s*(?::\s*(.+?))?\s*(?:\{{.*)?;?$"
            ));
            // A parameter behind a modifier is a field only in the constructor's list: in a type
            // literal or another method's parameters it is nothing of the class.
            let in_constructor = |i: usize| {
                let ind = indent(lines[i]);
                lines[i].trim_start().starts_with("constructor")
                    || lines[..i]
                        .iter()
                        .rev()
                        .find(|l| !l.trim().is_empty() && indent(l) < ind)
                        .is_some_and(|l| l.trim_start().starts_with("constructor"))
            };
            // `this` there is another object: a literal, a `function`, a nested class.
            let foreign = |i: usize| {
                bindings(kind, text, i + 1, "this")
                    .iter()
                    .any(|b| b.value != Value::Class(decl))
            };
            for i in body {
                let (code, ind) = (uncommented(kind, lines[i]), indent(lines[i]));
                let t = code.trim();
                if let Some(c) = getter.captures(t).filter(|_| ind == base) {
                    push(
                        i,
                        c.get(1)
                            .map_or(Value::Unknown, |ty| Value::Type(ty.as_str().to_owned())),
                    );
                } else if let Some(c) = member.captures(t).filter(|_| ind == base) {
                    push(
                        i,
                        match (c.get(1), c.get(2)) {
                            (Some(ty), _) => Value::Type(ty.as_str().trim().to_owned()),
                            (None, Some(v)) => value_of(kind, v.as_str()),
                            (None, None) => Value::Unknown,
                        },
                    );
                } else if let Some(c) = param.captures(t).filter(|_| in_constructor(i)) {
                    push(i, Value::Type(c[1].trim().to_owned()));
                } else if let Some(c) = assigned.captures(t).filter(|_| ind > base && !foreign(i)) {
                    push(i, value_of(kind, &c[1]));
                }
            }
        }
        Kind::Go if lines[k].contains("struct") => {
            for i in body.filter(|&i| indent(lines[i]) == base) {
                let t = lines[i].split("//").next().unwrap_or("");
                let t = t.split('`').next().unwrap_or("").trim();
                if let Some(embedded) = go_embedded(t) {
                    if embedded.rsplit('.').next() == Some(name) {
                        push(i, Value::Type(t.to_owned()));
                    }
                } else if let Some(c) = GO_FIELD.captures(t)
                    && c[1].split(',').any(|p| p.trim() == name)
                {
                    push(i, Value::Type(c[2].to_owned()));
                }
            }
        }
        _ => {}
    }
    out
}

/// The type an embedded Go field names, `sync.Mutex` for `*sync.Mutex`: a line of nothing else.
fn go_embedded(t: &str) -> Option<&str> {
    static EMBEDDED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\*?((?:[A-Za-z_]\w*\.)?[A-Za-z_]\w*)(?:\[.*\])?$").unwrap()
    });
    EMBEDDED.captures(t).map(|c| c.get(1).unwrap().as_str())
}

/// The 1-based line of the type declaration 0-based line `k` of `lines` sits in, however deep:
/// the enclosing lines, each indented less than the last, until one declares a type. `None` when
/// the walk reaches the top level first.
fn enclosing_type(kind: Kind, lines: &[&str], k: usize) -> Option<usize> {
    let mut depth = indent(lines[k]);
    for i in (0..k).rev() {
        if depth == 0 {
            break;
        }
        let t = lines[i].trim();
        // A closer at a lower indent ends a signature or a header wrapped over several lines.
        if t.is_empty()
            || t == "{"
            || comment(kind, t)
            || t.starts_with([')', ']'])
            || indent(lines[i]) >= depth
        {
            continue;
        }
        depth = indent(lines[i]);
        if declares_type(kind, lines[i]) {
            return Some(i + 1);
        }
    }
    None
}

/// The line among `bindings`, the [`field_bindings`] of one type over `lines`, that declares the
/// field: the first that is no assignment inside a method, else the first assignment, which the
/// second value says.
fn declaring(lines: &[&str], bindings: &[Binding], name: &str) -> Option<(usize, bool)> {
    let assigned = |b: &&Binding| in_method(lines[b.line - 1], name);
    bindings
        .iter()
        .find(|b| !assigned(b))
        .map(|b| (b.line, false))
        .or_else(|| bindings.first().map(|b| (b.line, true)))
}

/// The line on which the type declared on 1-based `decl` of `text` declares its field `name` —
/// the class-body annotation, a constructor parameter, the struct field, else the first
/// `self.name = …` — and whether that line is an assignment inside a method. `None` when the type
/// has no field of that name.
pub fn field_line(kind: Kind, text: &str, decl: usize, name: &str) -> Option<(usize, bool)> {
    let lines: Vec<&str> = text.lines().collect();
    declaring(&lines, &field_bindings(kind, text, decl, name), name)
}

/// The fields `name` the 1-based `hits` of `text` stand for: a hit that is one of the
/// [`field_bindings`] of the type around it gives that type's [`field_line`], once, in line
/// order. What [`field_patterns`] greps is more than the fields; this keeps the fields.
pub fn field_rows(kind: Kind, text: &str, hits: &[usize], name: &str) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let mut types: HashMap<usize, Vec<Binding>> = HashMap::new();
    let mut out = Vec::new();
    for &line in hits {
        let Some(k) = line.checked_sub(1).filter(|&k| k < lines.len()) else {
            continue;
        };
        let Some(decl) = enclosing_type(kind, &lines, k) else {
            continue;
        };
        let bindings = types
            .entry(decl)
            .or_insert_with(|| field_bindings(kind, text, decl, name));
        if bindings.iter().any(|b| b.line == line)
            && let Some((first, _)) = declaring(&lines, bindings, name)
            && !out.contains(&first)
        {
            out.push(first);
        }
    }
    out.sort_unstable();
    out
}

/// Whether the word `name` at byte `start` of 1-based `line` of `text` is the name a field's
/// declaration gives it ([`field_rows`]): the first time the line spells it, as `self.repo = repo`
/// and `this.f = this.f.bind(this)` spell it twice, and never a Go embedded struct, whose name
/// is its type's.
pub fn field_decl_at(kind: Kind, text: &str, line: usize, start: usize, name: &str) -> bool {
    let Some(l) = line.checked_sub(1).and_then(|k| text.lines().nth(k)) else {
        return false;
    };
    let ident = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    let first = l
        .match_indices(name)
        .map(|(i, _)| i)
        .find(|&i| !l[..i].ends_with(ident) && !l[i + name.len()..].starts_with(ident));
    let bare = l.split("//").next().unwrap_or("");
    let embedded =
        kind == Kind::Go && go_embedded(bare.split('`').next().unwrap_or("").trim()).is_some();
    first == Some(start) && !embedded && field_rows(kind, text, &[line], name) == [line]
}

/// What a call of the function declared on 1-based `decl` of `text` gives: its declared return
/// type (Python `-> T`, TypeScript `): T`, Go's result or its first one), or in TypeScript without
/// one, the `new T()` that every `return` of the body, or an arrow's expression, agrees on.
pub fn returns(kind: Kind, text: &str, decl: usize) -> Option<Value> {
    static PY_DEF: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*(?:async\s+)?def\s+\w+\s*\(").unwrap());
    // A function, a `const` holding one, or a method of a class or an interface.
    static TS_FUNCTION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:export|default|declare|async)\s+)*(?:function\*?\s*[\w$]*|(?:const|let|var)\s+[\w$]+\s*(?::[^=]+)?=\s*(?:async\s+)?(?:function\*?\s*[\w$]*)?)\s*(?:<[^>]*>)?\s*\(|^\s*(?:(?:public|private|protected|static|override|abstract|async)\s+)*#?[\w$]+\??\s*(?:<[^>]*>)?\s*\(").unwrap()
    });
    static TS_RETURN: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*:\s*(.+?)\s*(?:\{|=>|;|$)").unwrap());
    // A function, a method behind its receiver, or the method line of an interface.
    static GO_FUNC: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:func\s*(?:\([^)]*\)\s*)?|\s+)[A-Za-z_]\w*\s*(?:\[[^\]]*\])?\s*\(").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let k = decl.checked_sub(1).filter(|&i| i < lines.len())?;
    let opener = match kind {
        Kind::Python => &PY_DEF,
        Kind::TsJs => &TS_FUNCTION,
        Kind::Go => &GO_FUNC,
        _ => return None,
    };
    let open = opener.find(lines[k])?.end() - 1;
    let (_, end, after) = group(kind, &lines, k, open)?;
    match kind {
        // `-> T:` ends at the first colon outside strings and brackets; a body may follow it.
        Kind::Python => {
            let parts = split_top(kind, after, b':');
            match parts[0].trim().strip_prefix("->") {
                Some(t) => (parts.len() > 1 && !t.trim().is_empty())
                    .then(|| Value::Type(t.trim().to_owned())),
                None => python_constructs(text, &lines, k, end),
            }
        }
        Kind::TsJs => {
            if let Some(c) = TS_RETURN.captures(after) {
                return Some(Value::Type(c[1].to_owned()));
            }
            let expression = after.trim_start().strip_prefix("=>").map(str::trim);
            let returned: Vec<Value> = match expression {
                Some(e) if !e.starts_with('{') => vec![value_of(kind, e)],
                _ => {
                    let base = indent(lines[k]);
                    lines[end + 1..]
                        .iter()
                        .take_while(|l| !(indent(l) <= base && l.trim_start().starts_with('}')))
                        .map(|l| uncommented(kind, l))
                        .filter(|l| names(l, "return"))
                        // `if (x) return new A();` returns behind something the rules do not
                        // read: unknown, which no other `return` can agree with.
                        .map(|l| match l.trim().strip_prefix("return ") {
                            Some(e) => value_of(kind, e),
                            None => Value::Unknown,
                        })
                        .collect()
                }
            };
            match returned.first() {
                Some(Value::New(t)) if returned.iter().all(|v| *v == returned[0]) => {
                    Some(Value::New(t.clone()))
                }
                _ => None,
            }
        }
        _ => {
            let result = after.split('{').next().unwrap_or("").trim();
            let first = match result.strip_prefix('(') {
                Some(list) => {
                    let first = split_top(kind, list.strip_suffix(')')?, b',')[0].trim();
                    // `(r *Repo, err error)` names its results.
                    first
                        .split_once(char::is_whitespace)
                        .map_or(first, |(_, t)| t.trim())
                }
                None => result,
            };
            (!first.is_empty()).then(|| Value::Type(first.to_owned()))
        }
    }
}

/// What an undecorated Python `def` on line `k`, its signature ending on line `end`, returns when
/// it declares nothing: the class every `return` of its body calls, `return Repo(…)`. A bare
/// `return`, a `yield` or any other value leaves it unknown; a decorator may return anything.
fn python_constructs(text: &str, lines: &[&str], k: usize, end: usize) -> Option<Value> {
    let base = indent(lines[k]);
    // The nearest line above at the `def`'s indent that is no closer: `@retry(` of a decorator
    // written over several lines, whose arguments and `)` come first on the way up.
    let decorated = lines[..k]
        .iter()
        .rev()
        .map(|l| (indent(l), l.trim()))
        .find(|(ind, t)| !t.is_empty() && !t.starts_with(['#', ')', ']', '}']) && *ind <= base)
        .is_some_and(|(ind, t)| ind == base && t.starts_with('@'));
    if decorated {
        return None;
    }
    let literal = literal_lines(Kind::Python, text);
    let mut constructed: Option<String> = None;
    // A function or a class inside the body returns for itself.
    let mut skip: Option<usize> = None;
    for (i, l) in lines.iter().enumerate().skip(end + 1) {
        let code = uncommented(Kind::Python, l);
        let (t, ind) = (code.trim(), indent(l));
        if t.is_empty() || literal.get(i).copied().unwrap_or(false) {
            continue;
        }
        if ind <= base {
            break;
        }
        if skip.is_some_and(|s| ind > s) {
            continue;
        }
        let inner = ["def ", "async def ", "class "];
        skip = inner.iter().any(|p| t.starts_with(p)).then_some(ind);
        if names(t, "yield") {
            return None;
        }
        // `if flag: return A()` returns behind something the rules do not read.
        if names(t, "return") && !t.starts_with("return") {
            return None;
        }
        if t == "return" || t.starts_with("return ") {
            let Value::Call(name) = value_of(Kind::Python, t.strip_prefix("return")?) else {
                return None;
            };
            if constructed.as_ref().is_some_and(|c| *c != name) {
                return None;
            }
            constructed = Some(name);
        }
    }
    constructed.map(Value::New)
}

/// The written types a class header lists between its commas: `Base, Generic[T]`. A default
/// (`T = int`) is a type parameter, not a base.
fn type_list(kind: Kind, s: &str) -> Vec<String> {
    split_top(kind, s, b',')
        .into_iter()
        .map(str::trim)
        .filter(|b| !b.is_empty() && !b.contains('='))
        .map(str::to_owned)
        .collect()
}

/// The types the class, interface or struct declared on 1-based `decl` of `text` extends or
/// embeds, as written: Python's bases, TypeScript's `extends`, Go's embedded fields.
pub fn bases(kind: Kind, text: &str, decl: usize) -> Vec<String> {
    static TS_EXTENDS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bextends\s+(.+?)\s*(?:\bimplements\b|\{|$)").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = decl.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    let list = |s: &str| type_list(kind, s);
    match kind {
        Kind::Python => lines[k]
            .trim_start()
            .starts_with("class ")
            .then(|| lines[k].find('('))
            .flatten()
            .and_then(|open| group(kind, &lines, k, open))
            .map_or_else(Vec::new, |(inner, ..)| list(&inner)),
        Kind::TsJs => TS_EXTENDS
            .captures(lines[k])
            .map_or_else(Vec::new, |c| list(&c[1])),
        Kind::Go if lines[k].contains("struct") || lines[k].contains("interface") => {
            let body = body_of(kind, &lines, k);
            let base = lines[body.clone()]
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|l| indent(l))
                .min();
            body.filter(|&i| Some(indent(lines[i])) == base)
                .filter_map(|i| {
                    let t = lines[i].split("//").next().unwrap_or("");
                    go_embedded(t.split('`').next().unwrap_or("").trim()).map(str::to_owned)
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// The interfaces the TypeScript class or interface declared on 1-based `decl` of `text` says it
/// implements. [`bases`] reads `extends` alone, since that is where a member is inherited from;
/// an `implements` list declares no member and is read only to find the implementations of one
/// (#68 step 6). A Python class lists everything it derives from in [`bases`], and a Go type
/// names no interface at all.
pub fn interfaces(kind: Kind, text: &str, decl: usize) -> Vec<String> {
    static TS_IMPLEMENTS: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\bimplements\s+(.+?)\s*(?:\{|$)").unwrap());
    if kind != Kind::TsJs {
        return Vec::new();
    }
    decl.checked_sub(1)
        .and_then(|k| text.lines().nth(k))
        .and_then(|l| TS_IMPLEMENTS.captures(l))
        .map_or_else(Vec::new, |c| type_list(kind, &c[1]))
}

/// The 1-based line of the type declaration the member on 1-based `line` of `text` is written
/// inside: the nearest line above indented less, when it [`declares_type`]. `None` for a Go
/// method, which stands beside its type at the top level, and for a function nested in another
/// one, whose nearest enclosing line declares no type.
pub fn owner_decl(kind: Kind, text: &str, line: usize) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let depth = indent(lines.get(line.checked_sub(1)?)?);
    let (i, above) = lines[..line - 1].iter().enumerate().rev().find(|(_, l)| {
        let t = l.trim_start();
        !t.is_empty() && t.trim_end() != "{" && !comment(kind, t) && indent(l) < depth
    })?;
    declares_type(kind, above).then_some(i + 1)
}

/// The name a line declaring a type gives it: `Foo` for `class Foo(Base):`,
/// `export abstract class Foo<T> extends Bar {`, `interface Foo {` and `type Foo struct {`.
pub fn type_name(kind: Kind, line: &str) -> Option<String> {
    static NAME: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:class|interface|type|enum)\s+([A-Za-z_$][\w$]*)").unwrap()
    });
    declares_type(kind, line)
        .then(|| NAME.captures(line))
        .flatten()
        .map(|c| c[1].to_owned())
}

/// A grep for the line that names one of `names` as a base: `class X(Base)` in Python,
/// `class X extends Base`, `class X implements Base` and `interface I extends Base` in
/// TypeScript, where the clause may also stand on a line of its own under a wrapped header —
/// [`type_decl_at`] walks up to the declaration it belongs to. A Python header wrapped over
/// several lines keeps its bases off the `class` line and is missed. `None` for Go, whose types
/// implement an interface by carrying its methods and never name it.
pub fn subtype_patterns(kind: Kind, names: &[String]) -> Option<String> {
    let any: Vec<String> = names.iter().map(|n| regex::escape(n)).collect();
    let any = any.join("|");
    match kind {
        Kind::Python => Some(format!(r"^\s*class\s+\w+\s*\(.*\b({any})\b")),
        // Not only the header line: prettier writes `extends Base` on a line of its own.
        Kind::TsJs => Some(format!(r"\b(?:extends|implements)\s[^;]*\b({any})\b")),
        _ => None,
    }
}

/// The 1-based line the type declaration covering 1-based `line` of `text` starts on: `line`
/// itself when it [`declares_type`], else the nearest line above it that does, when nothing but
/// the rest of a header stands in between — `export class X` over `    extends Y` over `{`, as
/// prettier wraps a long one. `None` when the lines above end a statement first.
pub fn type_decl_at(kind: Kind, text: &str, line: usize) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let k = line.checked_sub(1).filter(|&k| k < lines.len())?;
    // ponytail: eight lines of header, which covers the widest prettier writes.
    for i in (k.saturating_sub(8)..=k).rev() {
        if declares_type(kind, lines[i]) {
            return Some(i + 1);
        }
        let t = lines[i].trim();
        if t.is_empty() || t.ends_with([';', '}']) || comment(kind, t) {
            return None;
        }
    }
    None
}

/// The 1-based line on which the type declared on 1-based `decl` of `text` declares the member
/// `word` itself: a method, or a signature with no body. `None` when it does not declare one, so
/// a subclass that inherits the member is no implementation of it.
pub fn member_decl(kind: Kind, text: &str, decl: usize, word: &str) -> Option<usize> {
    let patterns = member_or_signature(kind, word)?;
    let re = Regex::new(&patterns.join("|")).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let k = decl.checked_sub(1).filter(|&k| k < lines.len())?;
    body_of(kind, &lines, k)
        .find(|&i| re.is_match(lines[i]) && owner_decl(kind, text, i + 1) == Some(decl))
        .map(|i| i + 1)
}

/// How many parameters the declaration on 1-based `line` of `text` writes: what stands between
/// its brackets, cut at the commas outside brackets and strings, over the lines they span. Go's
/// `a, b string` counts two, as an implementation of the same interface method writes two of its
/// own, and a Go method's receiver is not a parameter.
pub fn params(kind: Kind, text: &str, line: usize) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let k = line.checked_sub(1).filter(|&k| k < lines.len())?;
    let mut opens = code(kind, lines[k])
        .filter(|&(_, c)| c == b'(')
        .map(|(i, _)| i);
    let receiver = kind == Kind::Go && lines[k].trim_start().starts_with("func (");
    let open = if receiver { opens.nth(1) } else { opens.next() }?;
    let (inner, ..) = group(kind, &lines, k, open)?;
    Some(
        split_top(kind, &inner, b',')
            .iter()
            .filter(|p| !p.trim().is_empty())
            .count(),
    )
}

/// The types a Go declaration on 1-based `line` takes and what it returns, as written but for
/// the package in front of a name and the spaces: `a, b string` is two `string`s, `ctx
/// context.Context` is `Context`. An implementation of an interface method writes the same ones,
/// whatever it calls its parameters. `None` when the result names its values or cannot be read.
pub fn go_signature(text: &str, line: usize) -> Option<(Vec<String>, String)> {
    static PKG: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\b\w+\.").unwrap());
    let kind = Kind::Go;
    let lines: Vec<&str> = text.lines().collect();
    let k = line.checked_sub(1).filter(|&k| k < lines.len())?;
    let mut opens = code(kind, lines[k])
        .filter(|&(_, c)| c == b'(')
        .map(|(i, _)| i);
    let receiver = lines[k].trim_start().starts_with("func (");
    let open = if receiver { opens.nth(1) } else { opens.next() }?;
    let (inner, _, rest) = group(kind, &lines, k, open)?;
    let plain = |t: &str| {
        PKG.replace_all(t, "")
            .split_whitespace()
            .collect::<String>()
    };
    let parts: Vec<&str> = split_top(kind, &inner, b',')
        .into_iter()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    fn typed(p: &str) -> Option<&str> {
        p.split_once(char::is_whitespace).map(|(_, t)| t.trim())
    }
    let named = parts.iter().any(|p| typed(p).is_some());
    let mut types: Vec<String> = Vec::new();
    // Backwards: a name with no type of its own takes the one that follows it.
    for p in parts.iter().rev() {
        let t = match (named, typed(p)) {
            (true, Some(t)) => plain(t),
            (true, None) => types.last()?.clone(),
            (false, _) => plain(p),
        };
        types.push(t);
    }
    types.reverse();
    let result = rest.split('{').next().unwrap_or_default().trim();
    // `(n int, err error)` names its values, and an implementation may not.
    let names_values = result.starts_with('(')
        && split_top(kind, result.trim_matches(['(', ')']), b',')
            .iter()
            .any(|p| p.trim().contains(char::is_whitespace));
    (!names_values).then(|| (types, plain(result)))
}

/// Whether `line` declares a type: a Python class, a TypeScript class, interface, type alias or
/// enum, a Go `type`.
pub fn declares_type(kind: Kind, line: &str) -> bool {
    static TS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"^\s*(?:(?:export|default|declare|abstract)\s+)*(?:class|interface|type|enum)\s",
        )
        .unwrap()
    });
    match kind {
        Kind::Python => line.trim_start().starts_with("class "),
        Kind::TsJs => TS.is_match(line),
        Kind::Go => line.starts_with("type "),
        _ => false,
    }
}

/// The `GOOS` and `GOARCH` of the machine merl runs on, which is what `go build` targets there.
pub fn go_host() -> (&'static str, &'static str) {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        os => os,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "x86" => "386",
        arch => arch,
    };
    (os, arch)
}

const GO_UNIX: &[&str] = &[
    "aix",
    "android",
    "darwin",
    "dragonfly",
    "freebsd",
    "hurd",
    "illumos",
    "ios",
    "linux",
    "netbsd",
    "openbsd",
    "solaris",
];
const GO_OS: &[&str] = &["js", "nacl", "plan9", "wasip1", "windows", "zos"];
const GO_ARCH: &[&str] = &[
    "386",
    "amd64",
    "amd64p32",
    "arm",
    "arm64",
    "arm64be",
    "armbe",
    "loong64",
    "mips",
    "mips64",
    "mips64le",
    "mips64p32",
    "mips64p32le",
    "mipsle",
    "ppc",
    "ppc64",
    "ppc64le",
    "riscv",
    "riscv64",
    "s390",
    "s390x",
    "sparc",
    "sparc64",
    "wasm",
];

/// Whether `go build` for `goos` / `goarch` compiles the Go file `path` with the text `text`, as
/// far as the platform decides it: the `_GOOS`, `_GOARCH` and `_GOOS_GOARCH` endings of its name
/// and its `//go:build` line. `None` when the line names a tag that is no platform (`gogit`,
/// `cgo`, `ignore`): what a build sets is not written in the source.
pub fn go_built(path: &Path, text: &str, goos: &str, goarch: &str) -> Option<bool> {
    let is_os = |t: &str| GO_UNIX.contains(&t) || GO_OS.contains(&t);
    let tag = |t: &str| -> Option<bool> {
        match t {
            "unix" => Some(GO_UNIX.contains(&goos)),
            t if is_os(t) => Some(t == goos),
            t if GO_ARCH.contains(&t) => Some(t == goarch),
            _ => None,
        }
    };
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let stem = stem.strip_suffix("_test").unwrap_or(stem);
    let mut parts: Vec<&str> = stem.split('_').skip(1).collect();
    let mut named = true;
    if let Some(arch) = parts.pop_if(|p| GO_ARCH.contains(p)) {
        named &= arch == goarch;
    }
    if let Some(os) = parts.pop_if(|p| is_os(p)) {
        named &= os == goos;
    }
    if !named {
        return Some(false);
    }
    let head = || text.lines().take_while(|l| !l.starts_with("package "));
    let Some(expr) = head().find_map(|l| l.strip_prefix("//go:build ")) else {
        // The constraint of before Go 1.17 is not read: undecided, never "no constraint".
        return (!head().any(|l| l.starts_with("// +build"))).then_some(true);
    };
    // `!` binds tightest, then `&&`, then `||`. A tag that is no platform is unknown, and
    // decides nothing only where the platforms around it have not decided already.
    static TOKEN: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"&&|\|\||[!()]|[\w.]+").unwrap());
    let tokens: Vec<&str> = TOKEN.find_iter(expr).map(|m| m.as_str()).collect();
    type Tag<'a> = &'a dyn Fn(&str) -> Option<bool>;
    fn any(t: &[&str], i: &mut usize, tag: Tag) -> Option<bool> {
        let mut value = all(t, i, tag);
        while t.get(*i) == Some(&"||") {
            *i += 1;
            value = match (value, all(t, i, tag)) {
                (Some(true), _) | (_, Some(true)) => Some(true),
                (Some(false), Some(false)) => Some(false),
                _ => None,
            };
        }
        value
    }
    fn all(t: &[&str], i: &mut usize, tag: Tag) -> Option<bool> {
        let mut value = one(t, i, tag);
        while t.get(*i) == Some(&"&&") {
            *i += 1;
            value = match (value, one(t, i, tag)) {
                (Some(false), _) | (_, Some(false)) => Some(false),
                (Some(true), Some(true)) => Some(true),
                _ => None,
            };
        }
        value
    }
    fn one(t: &[&str], i: &mut usize, tag: Tag) -> Option<bool> {
        let token = *t.get(*i)?;
        *i += 1;
        match token {
            "!" => one(t, i, tag).map(|v| !v),
            "(" => {
                let value = any(t, i, tag);
                *i += 1;
                value
            }
            name => tag(name),
        }
    }
    any(&tokens, &mut 0, &tag)
}

/// What the Go line `type X = Y` names, `Y` as written; `None` for a defined type, `type X Y`,
/// and for an alias with type parameters, whose arguments the rules do not carry.
pub fn go_alias(kind: Kind, line: &str) -> Option<&str> {
    static ALIAS: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^type\s+[A-Za-z_]\w*\s*=\s*([^/]+)").unwrap());
    let named = ALIAS.captures(line).filter(|_| kind == Kind::Go)?;
    Some(named.get(1)?.as_str().trim())
}

/// The name a written type comes down to, as its dotted parts: `["UserRepository"]`,
/// `["store", "Session"]`. `T | None`, `Optional[T]`, `Annotated[T, …]`, `T | null | undefined`, a
/// quoted forward reference, a Go pointer and generic arguments read as `T`. A list, a function
/// type, a union of two types or an inline object type has no one name.
pub fn type_path(kind: Kind, written: &str) -> Option<Vec<String>> {
    static NAME: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^[A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*)*$").unwrap()
    });
    let mut t = written.trim().trim_end_matches([';', ',']).trim();
    if kind == Kind::Python {
        t = t.trim_matches(['"', '\'']);
    }
    let left: Vec<&str> = split_top(kind, t, b'|')
        .into_iter()
        .map(str::trim)
        .filter(|p| !matches!(*p, "None" | "null" | "undefined"))
        .collect();
    let [one] = left[..] else { return None };
    t = one.trim_start_matches('*');
    if kind == Kind::Python {
        for wrapper in [
            "Optional[",
            "Annotated[",
            "typing.Optional[",
            "typing.Annotated[",
        ] {
            if let Some(inner) = t.strip_prefix(wrapper).and_then(|s| s.strip_suffix(']')) {
                return type_path(kind, split_top(kind, inner, b',')[0]);
            }
        }
    }
    let (open, close) = if kind == Kind::TsJs {
        ('<', '>')
    } else {
        ('[', ']')
    };
    if let Some(i) = t.find(open).filter(|_| t.ends_with(close)) {
        t = t[..i].trim_end();
    }
    NAME.is_match(t)
        .then(|| t.split('.').map(str::to_owned).collect())
}

/// The written type of an element of the collection written as `written`, where the type says so:
/// Python's `list[T]`, `Sequence[T]`, `set[T]`, `tuple[T, ...]` and the like, TypeScript's `T[]`,
/// `Array<T>`, `Set<T>`, Go's `[]T`, `[4]T` and `map[K]T`. A Python `dict` and a TypeScript `Map`
/// hand out keys or pairs, and anything else is not known to hand out anything.
pub fn element_type(kind: Kind, written: &str) -> Option<String> {
    // `Head<A, B>` or `Head[A, B]` as the last name of its head and its arguments.
    fn generic(kind: Kind, t: &str, open: char, close: char) -> Option<(&str, Vec<&str>)> {
        let i = t.find(open).filter(|_| t.ends_with(close))?;
        let head = t[..i].trim_end();
        let args = split_top(kind, &t[i + 1..t.len() - 1], b',');
        Some((head.rsplit('.').next().unwrap_or(head), args))
    }
    let t = written.trim().trim_end_matches([';', ',']).trim();
    let element = match kind {
        Kind::Python => match generic(kind, t.trim_matches(['"', '\'']), '[', ']')? {
            ("tuple" | "Tuple", args) if args.len() == 2 && args[1].trim() == "..." => args[0],
            (
                "list" | "List" | "Sequence" | "MutableSequence" | "Iterable" | "Iterator"
                | "Collection" | "set" | "Set" | "frozenset" | "FrozenSet" | "AbstractSet"
                | "deque",
                args,
            ) => args[0],
            _ => return None,
        },
        Kind::TsJs => {
            let t = t.strip_prefix("readonly ").unwrap_or(t).trim();
            match (t.strip_suffix("[]"), generic(kind, t, '<', '>')) {
                (Some(one), _) => one,
                (
                    None,
                    Some((
                        "Array" | "ReadonlyArray" | "Set" | "ReadonlySet" | "Iterable"
                        | "IterableIterator",
                        args,
                    )),
                ) => args[0],
                _ => return None,
            }
        }
        Kind::Go => {
            let rest = t.strip_prefix("map").unwrap_or(t);
            let close = close_of(kind, rest, 0).filter(|_| rest.starts_with('['))?;
            &rest[close..]
        }
        _ => return None,
    };
    let element = element.trim();
    (!element.is_empty()).then(|| element.to_owned())
}

/// Whether `path` is (in) the module spelled by `parts`: every part is a directory or file
/// stem on it, in order. A package directory carries a version (`regex-1.11.1`,
/// `toml@v1.2.3`), a Go module escapes upper case (`!burnt!sushi`) and a crate name spells
/// `_` as `-`: those are ignored.
pub fn in_module(path: &Path, parts: &[String]) -> bool {
    let mut want = parts.iter().map(|p| module_part(p)).peekable();
    for c in path.components() {
        let c = c.as_os_str().to_string_lossy();
        if want.peek().is_some_and(|w| *w == module_part(&c)) {
            want.next();
        }
    }
    want.peek().is_none()
}

/// A directory or a part of a module path without what only one of the two carries.
fn module_part(s: &str) -> String {
    let s = s.split('@').next().unwrap_or(s);
    // `fs.d.ts`, `python3.13`, `github.com`: the stem before every extension.
    let s = s.split('.').next().unwrap_or(s);
    // `name-1.2.3`: the version after the first `-` followed by a digit.
    let s = s
        .match_indices('-')
        .find(|(i, _)| s[i + 1..].starts_with(|c: char| c.is_ascii_digit()))
        .map_or(s, |(i, _)| &s[..i]);
    s.replace('!', "").replace('-', "_").to_ascii_lowercase()
}

/// Whether the Go file `path` is of the package imported as `parts` (#100): a Go package is one
/// directory, so the file's own directory ends with the import path, and `database/sql` is not
/// `database/sql/driver`. A path with no dot in its first part is the standard library's, which
/// sits right under GOROOT's `src` (or a `vendor` there): `errors` is not `github.com/pkg/errors`.
pub fn in_package(path: &Path, parts: &[String]) -> bool {
    let dirs: Vec<String> = path
        .parent()
        .into_iter()
        .flat_map(Path::components)
        .map(|c| module_part(&c.as_os_str().to_string_lossy()))
        .collect();
    let want: Vec<String> = parts.iter().map(|p| module_part(p)).collect();
    let std = parts.first().is_some_and(|p| !p.contains('.'));
    dirs.strip_suffix(want.as_slice())
        .is_some_and(|above| !std || above.last().is_some_and(|d| d == "src" || d == "vendor"))
}

/// The name `D` lists for a line matched by `re`, one of the [`SYMBOLS`] patterns: Terraform
/// blocks by the address they are referenced with, YAML anchors with their `&`.
pub fn symbol_name(re: &Regex, line: &str) -> Option<String> {
    let c = re.captures(line)?;
    let group = |name: &str| c.name(name).map(|m| m.as_str());
    if let (Some(block), Some(a)) = (group("block"), group("a")) {
        return Some(match (block, group("b")) {
            ("resource", Some(n)) => format!("{a}.{n}"),
            ("data", Some(n)) => format!("data.{a}.{n}"),
            ("variable", None) => format!("var.{a}"),
            ("module" | "output", None) => format!("{block}.{a}"),
            _ => return None,
        });
    }
    group("anchor")
        .map(|a| format!("&{a}"))
        .or_else(|| group("name").map(str::to_owned))
}

/// Whether `name` matches `query` as a name: the query's characters in order, case-insensitively
/// until the query has a capital of its own (smart case, as `/` and `s`). `D` past the cap
/// narrows its grep by this, so what comes back is what the picker then ranks. It is a name, not
/// a pattern: the picker's matcher also reads `^`, `!`, `'` and spaces as syntax of its own,
/// folds the accents off a letter, and matches the path beside the name — none of that reaches
/// the grep, so past the cap those keystrokes are characters of a name like any other.
pub fn fuzzy_match(query: &str, name: &str) -> bool {
    let exact = query.chars().any(char::is_uppercase);
    let mut left = name.chars();
    query
        .chars()
        .all(|q| left.any(|c| c == q || (!exact && c.to_lowercase().eq(q.to_lowercase()))))
}

/// The run of `[A-Za-z0-9_]` and `extra` characters at byte offset `col`, or the one that ends
/// there when the cursor sits right after a word. `extra` characters do not start or end a word.
pub fn word_at<'a>(line: &'a str, col: usize, extra: &str) -> Option<(Range<usize>, &'a str)> {
    let b = line.as_bytes();
    let word =
        |i: usize| b[i].is_ascii_alphanumeric() || b[i] == b'_' || extra.contains(b[i] as char);
    let mut i = col.min(b.len());
    if i == b.len() || !word(i) {
        if i > 0 && word(i - 1) {
            i -= 1;
        } else {
            return None;
        }
    }
    let mut start = i;
    while start > 0 && word(start - 1) {
        start -= 1;
    }
    let mut end = i + 1;
    while end < b.len() && word(end) {
        end += 1;
    }
    while start < end && extra.contains(b[start] as char) {
        start += 1;
    }
    while end > start && extra.contains(b[end - 1] as char) {
        end -= 1;
    }
    (start < end).then(|| (start..end, &line[start..end]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PY: &str = "class Invoice:\n    def total(self):\n        return 0\n\n\ndef parse(t):\n    return Invoice()\n\n\nDEFAULT_LIMIT = 10\ntotal_foobar = 1\nprint(total_foobar, DEFAULT_LIMIT)\nNAME_RE: Final[re.Pattern[str]] = re.compile(r\"x\")\n";
    const RS: &str = "pub struct Order<T> {\n    items: Vec<T>,\n}\n\nimpl<T> Order<T> {\n    pub fn sum(&self) -> u32 { 0 }\n}\n\npub(crate) const MAX_ORDERS: usize = 10;\n\npub async fn parse_order(s: &str) -> Order<u8> {\n    let mut order = Order { items: vec![] };\n    order.items.push(1);\n    order\n}\n\nmacro_rules! order {\n    () => {};\n}\n\nmod orders;\n";
    const TS: &str = "export interface Order {\n  id: Id;\n}\n\nexport type Id = string;\n\nconst enum Status {\n  Open,\n}\n\nexport default class OrderService {\n  private cache = new Map();\n\n  async load(id: Id): Promise<Order> {\n    render(o);\n    return parse(id);\n  }\n\n  sum = (o: Order) => 0;\n}\n\nexport const parseOrder = (s: string): Order => JSON.parse(s);\n\nexport function render(o: Order) {\n  const n = 1;\n}\n\nfunction* ids() {}\n";
    const JS: &str = "const helpers = {\n  parse(s) {\n    return s;\n  },\n  format: function (o) {\n    return o;\n  },\n};\nmodule.exports = helpers;\n";
    const GO: &str = "package main\n\ntype Invoice struct{}\n\nfunc (i Invoice) Total() int { return 0 }\n\nfunc Parse(s string) Invoice { return Invoice{} }\n\nconst Limit = 10\n\nfunc main() {\n\tinv := Parse(\"x\")\n}\n";

    /// A throwaway project on disk; grep needs real files.
    fn scratch(tag: &str, files: &[(&str, &str)]) -> (PathBuf, Vec<PathBuf>) {
        let dir = std::env::temp_dir().join(format!("merl-grep-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, text) in files {
            std::fs::create_dir_all(dir.join(name).parent().unwrap()).unwrap();
            std::fs::write(dir.join(name), text).unwrap();
        }
        (dir, files.iter().map(|(n, _)| PathBuf::from(n)).collect())
    }

    fn project(tag: &str) -> (PathBuf, Vec<PathBuf>) {
        scratch(
            tag,
            &[
                ("a.py", PY),
                ("b.go", GO),
                ("c.rs", RS),
                ("d.ts", TS),
                ("e.js", JS),
            ],
        )
    }

    fn grep(dir: &Path, files: &[PathBuf], pat: &str, word: bool, smart: bool) -> Vec<Hit> {
        grep_project(dir, files, pat, word, smart, None, None).unwrap()
    }

    fn lines(hits: &[Hit]) -> Vec<(String, usize)> {
        hits.iter()
            .map(|h| (h.path.display().to_string(), h.line))
            .collect()
    }

    /// The lines `d`'s patterns match for `word` in `files`.
    fn defs(dir: &Path, files: &[PathBuf], kind: Kind, word: &str) -> Vec<usize> {
        let pat = def_patterns(kind, word).join("|");
        grep(dir, files, &pat, false, false)
            .iter()
            .map(|h| h.line)
            .collect()
    }

    #[test]
    fn python_def_patterns_find_declarations_only() {
        let (dir, files) = project("py");
        let py = files[..1].to_vec();
        // The `def` line, not the `total_foobar` assignment.
        assert_eq!(defs(&dir, &py, Kind::Python, "total"), [2]);
        assert_eq!(defs(&dir, &py, Kind::Python, "parse"), [6]);
        // `^W\s*(:[^=]*)?=` catches module constants, annotated or not.
        assert_eq!(defs(&dir, &py, Kind::Python, "DEFAULT_LIMIT"), [10]);
        assert_eq!(defs(&dir, &py, Kind::Python, "NAME_RE"), [13]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The lines `d`'s member patterns match for `word` in `files`: what `x.word` reaches.
    fn members(dir: &Path, files: &[PathBuf], kind: Kind, word: &str) -> Vec<usize> {
        let pat = member_patterns(kind, word).unwrap().join("|");
        grep(dir, files, &pat, false, false)
            .iter()
            .map(|h| h.line)
            .collect()
    }

    const PY_MEMBERS: &str = "import asyncio\n\n\nclass UserRepository:\n    async def delete_user(self, user_id: int) -> None:\n        pass\n\n    def find_user(self, user_id):\n        return user_id\n\n\ndef find_user(user_id):\n    return user_id\n\n\nasync def main():\n    pass\n\n\ndelete_user = None\n";

    #[test]
    fn python_members_are_indented_defs_sync_or_async() {
        let (dir, files) = scratch("py-members", &[("repos.py", PY_MEMBERS)]);
        // `async def` is a declaration for a bare word too, at the top level or in a class.
        assert_eq!(defs(&dir, &files, Kind::Python, "delete_user"), [5, 20]);
        assert_eq!(defs(&dir, &files, Kind::Python, "main"), [16]);
        // `x.delete_user` reaches the method, not the module-level name of the same spelling.
        assert_eq!(members(&dir, &files, Kind::Python, "delete_user"), [5]);
        // `x.find_user` reaches the method; the module-level function takes an import.
        assert_eq!(members(&dir, &files, Kind::Python, "find_user"), [8]);
        assert_eq!(defs(&dir, &files, Kind::Python, "find_user"), [8, 12]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    const TS_MEMBERS: &str = "export interface Repo {\n  deleteUser(id: string): Promise<void>;\n  findUser?<T>(id: string): T | undefined\n  onChange: (id: string) => void;\n  name: string;\n}\n\nexport abstract class Base {\n  abstract deleteUser(id: string): Promise<void>;\n  get size(): number;\n}\n\nconst deleteUser = (id: string) => id;\nfindUser(id);\nrun(x).then((y): void => y);\nconst n = cond ? findUser(a) : b;\nexport const helpers = {\n  deleteUser(id) {\n    return id;\n  },\n};\nappend(target, visitor ? visitNode(s) : s);\nlog(\"(while reading XRef): \" + e);\ndeclare class Emitter {\n  on(event: string, cb: (x: T) => void): this;\n  append(...items: string[]): void;\n}\nclass Session {\n  close(): void {}\n}\nnoop(() => {})\nclass Svc {\n  constructor(\n    @Inject(W) private worker: Worker,\n  ) {}\n}\nclass One {\n  constructor(private readonly inline: Dep, public other: Dep) {}\n}\ndeclare class Wide {\n  pong<T extends Record<string, number>>(x: T): T;\n}\n";

    #[test]
    fn ts_members_include_signatures_without_a_body() {
        let (dir, files) = scratch("ts-members", &[("repo.ts", TS_MEMBERS)]);
        let m = |w| members(&dir, &files, Kind::TsJs, w);
        // The interface signature, the abstract one and the object-literal method; not the
        // `const` arrow function, a local to whoever reads `deleteUser` bare.
        assert_eq!(m("deleteUser"), [2, 9, 18]);
        assert_eq!(
            defs(&dir, &files, Kind::TsJs, "deleteUser"),
            [2, 9, 13, 18],
            "a bare word reads the `const` too"
        );
        // An optional generic signature with no `;`, not the call statement or the ternary.
        assert_eq!(m("findUser"), [3]);
        assert_eq!(m("onChange"), [4], "a property holding a function");
        assert_eq!(m("size"), [10], "a getter signature");
        // A plain field has no rule, and an annotated arrow argument is not a signature.
        assert_eq!(m("name"), Vec::<usize>::new());
        assert_eq!(m("run"), Vec::<usize>::new());
        // A call whose arguments hold `) :` or `):` is not one either; a signature whose
        // parameter is a function type, and a rest parameter, are.
        assert_eq!(m("append"), [26], "not the call on line 22");
        assert_eq!(m("log"), Vec::<usize>::new());
        assert_eq!(m("on"), [25]);
        // An empty body on the method's line, not a call whose last argument is one.
        assert_eq!(m("close"), [29]);
        assert_eq!(m("noop"), Vec::<usize>::new());
        // A constructor parameter behind an access modifier is a property of the class.
        assert_eq!(m("worker"), [34]);
        assert_eq!(m("inline"), [38]);
        assert_eq!(m("other"), [38]);
        // Type parameters may nest.
        assert_eq!(m("pong"), [41]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn lines_inside_a_literal_or_a_block_comment_are_told() {
        let inside = |kind, text: &str| -> Vec<usize> {
            let lines = literal_lines(kind, text);
            (1..=lines.len()).filter(|&n| lines[n - 1]).collect()
        };
        let py = "SRC = \"\"\"\ndef ghost(x):\n    pass\n\"\"\"\n\n\ndef real(a=\"# no\", b='\"\"\"'):  # it's fine\n    \"\"\"Doc.\n\n    def example():\n    \"\"\"\n    return 1\n";
        assert_eq!(inside(Kind::Python, py), [2, 3, 4, 9, 10, 11]);
        let go = "const s = `\nfunc (t T) InString() {}\n`\n\n/*\nfunc (t T) InBlock() {}\n*/\nfunc (t T) Real() { _ = \"/*\" } // it's `fine\nfunc (t T) Next() {}\n";
        assert_eq!(inside(Kind::Go, go), [2, 3, 6, 7]);
        let ts = "const q = `\n  find(id: string): User;\n  ${x}`;\nclass A {\n  find(id: string): User {}\n}\n";
        assert_eq!(inside(Kind::TsJs, ts), [2, 3]);
        // A migration embeds SQL, and a raw or a verbatim string is where it puts it. `""` is how
        // a verbatim string writes a quote, so it does not close one.
        let cs = "var q = \"\"\"\n    WHERE EXISTS(SELECT 1 FROM t)\n    \"\"\";\nvar v = @\"\n    SELECT MIN(\"\"rowid\"\") FROM t\n    \";\npublic int Real() => 1;\n";
        assert_eq!(inside(Kind::CSharp, cs), [2, 3, 5, 6]);
        let sw = "let doc = \"\"\"\n    class Ghost {}\n    \"\"\"\nclass Real {}\n";
        assert_eq!(inside(Kind::Swift, sw), [2, 3]);
        // A heredoc ends on the line that repeats its label, and only there.
        let php = "$sql = <<<SQL\n    function ghost() {}\n    class Ghost {}\nSQL;\n$n = <<<'TXT'\n    class Nowdoc {}\nTXT;\nclass Real {}\n";
        assert_eq!(inside(Kind::Php, php), [2, 3, 6]);
    }

    #[test]
    fn a_go_signature_is_its_types_whatever_the_names() {
        let go = "type I interface {\n\tFerry(a, b string, ctx context.Context) (*Row, error)\n\tSolo(a string) error\n\tBare(string, int)\n}\nfunc (r *R) Ferry(x string, y string, c context.Context) (*db.Row, error) {\nfunc (n N) Solo(a int) string { return \"\" }\nfunc (n N) Bare(s string, i int) {}\nfunc (n N) Named(a int) (n int, err error) {\n";
        assert_eq!(go_signature(go, 2), go_signature(go, 6));
        assert!(go_signature(go, 2).is_some());
        assert_ne!(go_signature(go, 3), go_signature(go, 7));
        assert_eq!(go_signature(go, 4), go_signature(go, 8));
        assert_eq!(go_signature(go, 9), None);
    }

    #[test]
    fn go_members_need_a_receiver() {
        let go = "package main\n\ntype Repo struct{}\n\nfunc (r *Repo[T]) Delete(id int) {}\n\nfunc (Repo) Find(id int) {}\n\nfunc Delete(id int) {}\n\nfunc main() {\n\tDelete := 1\n}\n\nfunc Get[T any](id int) {}\n";
        let (dir, files) = scratch("go-members", &[("repo.go", go)]);
        assert_eq!(
            defs(&dir, &files, Kind::Go, "Get"),
            [15],
            "a generic function"
        );
        assert_eq!(members(&dir, &files, Kind::Go, "Delete"), [5]);
        assert_eq!(members(&dir, &files, Kind::Go, "Find"), [7]);
        assert_eq!(defs(&dir, &files, Kind::Go, "Delete"), [5, 9, 12]);
        assert!(member_patterns(Kind::Rust, "len").is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn qualified_names_come_from_the_declarations_around() {
        let py = "class Outer:\n    # a comment at the class level\n    class Inner:\n        def run(self):\n\n            pass\n\n    async def stop(self):\n        pass\n\ndef main():\n    def helper():\n        pass\n";
        let q = |kind, text, line, name| qualified(kind, text, line, name);
        assert_eq!(
            q(Kind::Python, py, 4, "run").as_deref(),
            Some("Outer.Inner.run")
        );
        assert_eq!(
            q(Kind::Python, py, 8, "stop").as_deref(),
            Some("Outer.stop")
        );
        assert_eq!(
            q(Kind::Python, py, 12, "helper").as_deref(),
            Some("main.helper")
        );
        assert_eq!(q(Kind::Python, py, 11, "main"), None, "the top level");
        assert_eq!(q(Kind::Python, py, 99, "x"), None);
        let ts = "export default class OrderService {\n  load(id: Id) {\n  }\n}\nexport interface Repo {\n  deleteUser(id: string): void;\n}\nexport const helpers = {\n  parse(s) {\n    return s;\n  },\n};\n";
        assert_eq!(
            q(Kind::TsJs, ts, 2, "load").as_deref(),
            Some("OrderService.load")
        );
        assert_eq!(
            q(Kind::TsJs, ts, 6, "deleteUser").as_deref(),
            Some("Repo.deleteUser")
        );
        assert_eq!(
            q(Kind::TsJs, ts, 9, "parse").as_deref(),
            Some("helpers.parse")
        );
        let go = "package main\n\nfunc (r *UserRepository) DeleteUser(id int) {}\nfunc (AuditLog) DeleteUser(id int) {}\nfunc (r *Repo[T]) Get() {}\nfunc Parse() {}\ntype Notifier interface {\n\tSend(text string)\n}\n";
        assert_eq!(
            q(Kind::Go, go, 3, "DeleteUser").as_deref(),
            Some("UserRepository.DeleteUser")
        );
        assert_eq!(
            q(Kind::Go, go, 4, "DeleteUser").as_deref(),
            Some("AuditLog.DeleteUser")
        );
        assert_eq!(q(Kind::Go, go, 5, "Get").as_deref(), Some("Repo.Get"));
        assert_eq!(q(Kind::Go, go, 6, "Parse"), None);
        assert_eq!(q(Kind::Go, go, 8, "Send").as_deref(), Some("Notifier.Send"));
        assert_eq!(q(Kind::Rust, RS, 6, "sum").as_deref(), Some("Order::sum"));
        // A Rust `impl` is named after the type it is for.
        let rs = "impl std::fmt::Display for Reason {\n    fn fmt(&self) {}\n}\nimpl<T> From<T> for Order {\n    fn from(t: T) -> Self {}\n}\n";
        assert_eq!(q(Kind::Rust, rs, 2, "fmt").as_deref(), Some("Reason::fmt"));
        assert_eq!(q(Kind::Rust, rs, 5, "from").as_deref(), Some("Order::from"));
        // An enclosing line that names nothing stops the walk: the object literal's `get` is
        // not `Api.get`.
        let lit =
            "class Api {\n  build() {\n    return {\n      get() {\n      },\n    };\n  }\n}\n";
        assert_eq!(q(Kind::TsJs, lit, 4, "get"), None);
        // A comment indented less than the method is skipped, not taken for a container.
        let commented = "class A:\n# note\n    def run(self):\n        pass\n";
        assert_eq!(
            q(Kind::Python, commented, 3, "run").as_deref(),
            Some("A.run")
        );
        let yaml = "x-common: &defaults\n  env: prod\n";
        assert_eq!(q(Kind::Yaml, yaml, 2, "env"), None);
        let rb = "module Billing\n  class Invoice\n    def total\n    end\n  end\nend\n";
        assert_eq!(
            q(Kind::Ruby, rb, 3, "total").as_deref(),
            Some("Billing.Invoice.total")
        );
    }

    /// #100. A `var (` block declares what stands at its own level; a function may declare a
    /// name wherever it mentions it other than in front of a `.`.
    #[test]
    fn a_go_package_block_is_read_at_its_level_and_a_mention_may_declare() {
        let block = "var (\n\tcfg struct {\n\t\trepo *B\n\t}\n\n\t// the audit log\n\taudit AuditLog // shared\n)\n\nfunc f() {\n\trepo := 1\n}\n\ntype T struct {\n\taudit *B\n}\n";
        assert_eq!(package_bindings(block, "repo"), vec![]);
        assert_eq!(
            package_bindings(block, "audit"),
            vec![Binding {
                line: 7,
                value: Value::Type("AuditLog".into())
            }]
        );
        let text = "package p\n\nvar x = repo.Top()\n\nfunc Run(repo *A, fn func() error) {\n\trepo.Do()\n}\n\nfunc Wide(\n\trepo *A,\n) error {\n\trepo.Do()\n\treturn nil\n}\n\nfunc Free(id int) {\n\t// repo is a word here\n\tlog(\"repo\")\n\trepo.Do()\n\tsave(repo)\n\trepo.Do()\n}\n\nfunc Label() {\n\trepo := 1\nretry:\n\trepo.Do()\n}\n";
        for (line, want) in [
            (3, false),
            (6, true),
            (12, true),
            (19, false),
            (21, true),
            (27, true),
        ] {
            assert_eq!(go_may_declare(text, line, "repo"), want, "line {line}");
        }
    }

    /// #100. A Go package is the one directory its import path ends at.
    #[test]
    fn a_go_package_is_one_directory() {
        let parts = |p: &str| -> Vec<String> { p.split('/').map(str::to_owned).collect() };
        for (file, import, want) in [
            ("/go/src/database/sql/sql.go", "database/sql", true),
            (
                "/go/src/database/sql/driver/driver.go",
                "database/sql",
                false,
            ),
            (
                "/go/src/database/sql/driver/driver.go",
                "database/sql/driver",
                true,
            ),
            (
                "/go/src/vendor/golang.org/x/net/http2/h.go",
                "golang.org/x/net/http2",
                true,
            ),
            (
                "/mod/gopkg.in/yaml.v3@v3.0.1/yaml.go",
                "gopkg.in/yaml.v3",
                true,
            ),
            (
                "/mod/github.com/!burnt!sushi/toml@v1.2.3/lex.go",
                "github.com/BurntSushi/toml",
                true,
            ),
            (
                "/mod/github.com/foo/bar/v2@v2.1.0/sub/s.go",
                "github.com/foo/bar/v2/sub",
                true,
            ),
            (
                "/mod/github.com/foo/bar/v2@v2.1.0/sub/s.go",
                "github.com/foo/bar/v2",
                false,
            ),
            ("/go/src/errors/errors.go", "errors", true),
            (
                "/mod/github.com/pkg/errors@v0.9.1/errors.go",
                "errors",
                false,
            ),
            ("/go/src/internal/errors/e.go", "errors", false),
            ("/proj/vendor/errors/e.go", "errors", true),
            ("/proj/lib/errors/e.go", "errors", false),
        ] {
            assert_eq!(
                in_package(Path::new(file), &parts(import)),
                want,
                "{file} as {import}"
            );
        }
    }

    /// #100. A Go file is compiled for a platform by its name and its `//go:build` line; a tag
    /// that is no platform leaves it undecided.
    #[test]
    fn a_go_file_is_built_for_a_platform_by_its_name_and_its_build_line() {
        let built = |name: &str, line: &str, os: &str, arch: &str| {
            let text = format!("// Copyright\n\n{line}\n\npackage p\n");
            go_built(Path::new(name), &text, os, arch)
        };
        for (name, line, os, arch, want) in [
            ("clock.go", "", "linux", "amd64", Some(true)),
            ("clock_linux.go", "", "linux", "amd64", Some(true)),
            ("clock_linux.go", "", "darwin", "arm64", Some(false)),
            ("clock_linux_test.go", "", "darwin", "arm64", Some(false)),
            ("clock_arm64.go", "", "darwin", "arm64", Some(true)),
            ("clock_arm64.go", "", "darwin", "amd64", Some(false)),
            ("clock_linux_arm64.go", "", "linux", "arm64", Some(true)),
            ("clock_linux_arm64.go", "", "darwin", "arm64", Some(false)),
            // The name of a file is no ending of it, and `unix` is no `GOOS` a name can spell.
            ("linux.go", "", "darwin", "arm64", Some(true)),
            ("clock_unix.go", "", "windows", "amd64", Some(true)),
            (
                "clock_unix.go",
                "//go:build unix",
                "windows",
                "amd64",
                Some(false),
            ),
            (
                "clock_unix.go",
                "//go:build unix",
                "darwin",
                "arm64",
                Some(true),
            ),
            (
                "clock_other.go",
                "//go:build !windows",
                "linux",
                "amd64",
                Some(true),
            ),
            (
                "clock_other.go",
                "//go:build !windows",
                "windows",
                "amd64",
                Some(false),
            ),
            (
                "c.go",
                "//go:build linux || darwin",
                "darwin",
                "arm64",
                Some(true),
            ),
            (
                "c.go",
                "//go:build linux && arm64",
                "linux",
                "amd64",
                Some(false),
            ),
            (
                "c.go",
                "//go:build !(js && wasm)",
                "linux",
                "amd64",
                Some(true),
            ),
            (
                "c.go",
                "//go:build (linux || darwin) && !amd64",
                "darwin",
                "amd64",
                Some(false),
            ),
            (
                "c.go",
                "//go:build linux || darwin && amd64",
                "linux",
                "arm64",
                Some(true),
            ),
            // The name and the line both have to hold.
            (
                "c_linux.go",
                "//go:build arm64",
                "linux",
                "amd64",
                Some(false),
            ),
            ("c.go", "//go:build gogit", "linux", "amd64", None),
            ("c.go", "//go:build !gogit && linux", "linux", "amd64", None),
            (
                "c.go",
                "//go:build !gogit && linux",
                "darwin",
                "arm64",
                Some(false),
            ),
            ("c.go", "//go:build ignore", "linux", "amd64", None),
            // A platform that has decided is not undone by a tag that is none.
            (
                "c.go",
                "//go:build windows && cgo",
                "darwin",
                "arm64",
                Some(false),
            ),
            (
                "c.go",
                "//go:build cgo && windows",
                "darwin",
                "arm64",
                Some(false),
            ),
            (
                "c.go",
                "//go:build darwin || cgo",
                "darwin",
                "arm64",
                Some(true),
            ),
            ("c.go", "//go:build darwin && cgo", "darwin", "arm64", None),
            ("c.go", "//go:build windows || cgo", "darwin", "arm64", None),
            (
                "c.go",
                "//go:build !(windows && cgo)",
                "darwin",
                "arm64",
                Some(true),
            ),
            // The old spelling is not read, and an architecture is one whatever the host.
            ("c.go", "// +build windows", "darwin", "arm64", None),
            ("c_sparc64.go", "", "darwin", "arm64", Some(false)),
        ] {
            assert_eq!(
                built(name, line, os, arch),
                want,
                "{name} {line} on {os}/{arch}"
            );
        }
        // The host is spelled as Go spells it.
        let (os, arch) = go_host();
        assert!(GO_UNIX.contains(&os) || GO_OS.contains(&os), "{os}");
        assert!(GO_ARCH.contains(&arch), "{arch}");
        // A `//go:build` under the package clause is a comment.
        let late = "package p\n\n//go:build windows\n";
        assert_eq!(
            go_built(Path::new("c.go"), late, "linux", "amd64"),
            Some(true)
        );
    }

    /// #104. What the search by name offers for a field — the lines [`field_patterns`] match,
    /// through [`field_rows`] — is each type's declaration of it once, named after the type and
    /// not after the method it is assigned in. A later assignment stands for the declaration; a
    /// local, the key of a literal, a docstring, a `var` block, a field of an anonymous struct, a
    /// member of a type literal and a `this` that is no class are none.
    #[test]
    fn a_field_by_name_is_the_declaration_in_its_type() {
        let found = |kind: Kind, text: &str, name: &str| -> Vec<(usize, String)> {
            let re = Regex::new(&field_patterns(kind, name).unwrap().join("|")).unwrap();
            let hits: Vec<usize> = text
                .lines()
                .enumerate()
                .filter(|(_, l)| re.is_match(l))
                .map(|(i, _)| i + 1)
                .collect();
            field_rows(kind, text, &hits, name)
                .into_iter()
                .map(|n| (n, qualified(kind, text, n, name).unwrap_or_default()))
                .collect()
        };
        let one = |n: usize, q: &str| vec![(n, q.to_owned())];
        let py = "class Issue(Base):\n    poster_id: int = 0\n\n    def __init__(\n        self,\n        repo: Repo,\n    ) -> None:\n        self.repo = repo\n        self.poster_id = 1\n\n    def close(self) -> None:\n        self.repo = None\n        total: int = 0\n        counts = {\n            total: 1,\n        }\n\n\ndef tally() -> None:\n    repo: Repo = make()\n";
        assert_eq!(
            found(Kind::Python, py, "poster_id"),
            one(2, "Issue.poster_id")
        );
        assert_eq!(found(Kind::Python, py, "repo"), one(8, "Issue.repo"));
        assert_eq!(found(Kind::Python, py, "total"), vec![]);
        assert_eq!(
            qualified(Kind::Python, py, 12, "repo").as_deref(),
            Some("Issue.repo")
        );
        // Outside a class `self` is a parameter like any other.
        let def = "def build(self):\n    self.x = 1\n";
        assert_eq!(found(Kind::Python, def, "x"), vec![]);
        assert_eq!(
            qualified(Kind::Python, def, 2, "x").as_deref(),
            Some("build.x")
        );
        // A tuple target is a declaration, and the later plain assignment stands for it.
        let tuple = "class Point:\n    def __init__(self):\n        self.x, self.offset = 0, 0\n\n    def move(self):\n        self.offset = 5\n";
        assert_eq!(found(Kind::Python, tuple, "offset"), one(3, "Point.offset"));
        let doc = "class Comment:\n    \"\"\"\n    body : str\n    \"\"\"\n\n    def __init__(self, body):\n        self.body = body\n";
        assert_eq!(found(Kind::Python, doc, "body"), one(7, "Comment.body"));
        let ts = "export class Issue extends Base {\n  posterId = 0;\n\n  constructor(\n    private repo: Repo,\n    @Inject(Log) private log: Log,\n  ) {\n    super();\n    this.title = \"\";\n  }\n\n  resize(opts: {\n    readonly width: number;\n  }): void {\n    const box = {\n      grow() {\n        this.height = 1;\n      },\n    };\n  }\n}\nexport function tally(): void {\n  const sums = {\n    total: 0,\n  };\n  total = 2;\n}\nexport const config: {\n  readonly timeout: number;\n} = { timeout: 1 };\n";
        assert_eq!(found(Kind::TsJs, ts, "posterId"), one(2, "Issue.posterId"));
        assert_eq!(found(Kind::TsJs, ts, "repo"), one(5, "Issue.repo"));
        assert_eq!(found(Kind::TsJs, ts, "log"), one(6, "Issue.log"));
        assert_eq!(found(Kind::TsJs, ts, "title"), one(9, "Issue.title"));
        assert_eq!(found(Kind::TsJs, ts, "width"), vec![]);
        assert_eq!(found(Kind::TsJs, ts, "height"), vec![]);
        // Nor are they named after the class.
        assert_eq!(qualified(Kind::TsJs, ts, 13, "width"), None);
        assert_eq!(qualified(Kind::TsJs, ts, 17, "height"), None);
        assert_eq!(found(Kind::TsJs, ts, "total"), vec![]);
        assert_eq!(found(Kind::TsJs, ts, "timeout"), vec![]);
        assert_eq!(
            qualified(Kind::TsJs, ts, 29, "timeout").as_deref(),
            Some("config.timeout")
        );
        let go = "package main\n\ntype Issue struct {\n\t*store.Base\n\tPosterID    int\n\tTitle, Body string `json:\"t\"`\n\tStats       struct {\n\t\tTotal int\n\t}\n}\n\nfunc Serve() {\n\tvar (\n\t\tHost string\n\t)\n}\n";
        assert_eq!(found(Kind::Go, go, "PosterID"), one(5, "Issue.PosterID"));
        assert_eq!(found(Kind::Go, go, "Body"), one(6, "Issue.Body"));
        assert_eq!(found(Kind::Go, go, "Base"), one(4, "Issue.Base"));
        assert_eq!(found(Kind::Go, go, "Stats"), one(7, "Issue.Stats"));
        assert_eq!(found(Kind::Go, go, "Total"), vec![]);
        assert_eq!(found(Kind::Go, go, "Host"), vec![]);
    }

    /// #104. The search by name greps for [`member_patterns`] and [`field_patterns`] and then keeps
    /// the fields [`field_bindings`] reads; a form the grep does not know would drop its type from
    /// the list, so every line the rules read must be one the grep finds.
    #[test]
    fn the_grep_finds_every_line_the_field_rules_read() {
        let py = "class A:\n    a: int\n    b = 1\n\n    def __init__(self):\n        self.c = 1\n        self.d: int = 1\n        self.e, self.f = 1, 2\n        (self.g, x) = 1, 2\n        with open(p) as self.h:\n            pass\n        for self.i in xs:\n            pass\n\n    @property\n    def j(self) -> int:\n        return 1\n";
        let ts = "export class A {\n  a: number;\n  b = 1;\n  static c = 1;\n  declare d: D;\n  accessor e = 1;\n  f;\n  readonly g?: G;\n  get h(): H {\n    return new H();\n  }\n\n  constructor(\n    private i: I,\n    @Inject(J) protected j: J,\n  ) {\n    this.k = 1;\n  }\n}\nexport class B {\n  constructor(private l: L) {}\n}\n";
        let go = "package main\n\ntype A struct {\n\tA int\n\tB, C string\n\t*Base\n\tpkg.Mixin\n\tD func(x int) error\n\tE map[string]int `json:\"e\"`\n}\n";
        let cases: [(Kind, &str, &[&str]); 3] = [
            (
                Kind::Python,
                py,
                &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"],
            ),
            (
                Kind::TsJs,
                ts,
                &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"],
            ),
            (Kind::Go, go, &["A", "B", "C", "Base", "Mixin", "D", "E"]),
        ];
        for (kind, text, names) in cases {
            let lines: Vec<&str> = text.lines().collect();
            let decls: Vec<usize> = (1..=lines.len())
                .filter(|&n| declares_type(kind, lines[n - 1]))
                .collect();
            for name in names {
                let mut patterns = member_patterns(kind, name).unwrap();
                patterns.extend(field_patterns(kind, name).unwrap());
                let re = Regex::new(&patterns.join("|")).unwrap();
                let read: Vec<usize> = decls
                    .iter()
                    .flat_map(|&d| field_bindings(kind, text, d, name))
                    .map(|b| b.line)
                    .collect();
                assert!(!read.is_empty(), "{kind:?}: no field {name}");
                for n in read {
                    assert!(
                        re.is_match(lines[n - 1]),
                        "{kind:?}: {name}: {}",
                        lines[n - 1]
                    );
                }
            }
        }
    }

    #[test]
    fn a_reason_says_whether_it_proves_the_target() {
        assert_eq!(Reason::ByName.to_string(), "by name");
        assert!(!Reason::ByName.proven());
        let import = Reason::Import("numpy".into());
        assert_eq!(import.to_string(), "via import numpy");
        assert!(import.proven());
        assert_eq!(Reason::Path("std::fs".into()).to_string(), "via std::fs");
    }

    #[test]
    fn go_def_patterns_cover_receivers_types_and_short_vars() {
        let (dir, files) = project("go");
        let go = files[1..2].to_vec();
        for (word, line) in [("Invoice", 3), ("Total", 5), ("Parse", 7), ("Limit", 9)] {
            assert_eq!(defs(&dir, &go, Kind::Go, word), [line], "{word}");
        }
        // `inv := Parse("x")` is the closest thing Go has to a definition of `inv`.
        assert_eq!(defs(&dir, &go, Kind::Go, "inv"), [12]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rust_def_patterns_cover_items_behind_prefixes_and_lets() {
        let (dir, files) = project("rs");
        let rs = files[2..3].to_vec();
        for (word, line) in [
            ("Order", 1), // the struct, not the `impl` block or the `Order { .. }` literal
            ("sum", 6),
            ("MAX_ORDERS", 9),
            ("parse_order", 11),
            ("orders", 21),
        ] {
            assert_eq!(defs(&dir, &rs, Kind::Rust, word), [line], "{word}");
        }
        // Both the `let mut` binding and the macro: the caller shows a picker.
        assert_eq!(defs(&dir, &rs, Kind::Rust, "order"), [12, 17]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ts_def_patterns_cover_declarations_methods_and_arrows() {
        let (dir, files) = project("ts");
        // `.ts` and `.js` are one kind: the JS helpers are found from the TS file.
        let ts_js = files[3..].to_vec();
        for (word, file, line) in [
            ("Order", "d.ts", 1),
            ("Id", "d.ts", 5),
            ("Status", "d.ts", 7),
            ("OrderService", "d.ts", 11),
            ("load", "d.ts", 14),
            ("sum", "d.ts", 19),
            ("parseOrder", "d.ts", 22),
            ("render", "d.ts", 24), // the declaration, not the `render(o);` call
            ("n", "d.ts", 25),
            ("ids", "d.ts", 28),
            ("parse", "e.js", 2),
            ("format", "e.js", 5),
        ] {
            let pat = def_patterns(Kind::TsJs, word).join("|");
            assert_eq!(
                lines(&grep(&dir, &ts_js, &pat, false, false)),
                [(file.into(), line)],
                "{word}"
            );
        }
        // A plain field is not a declaration the rules know: `d` has no definition for it.
        assert!(defs(&dir, &ts_js, Kind::TsJs, "cache").is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    const JAVA: &str = r#"package app;

public final class Invoice {
    private static final int LIMIT = 10;
    private Map<String, Integer> items;

    public Invoice(int n) {
        this.n = n;
    }

    @Override
    public int total() {
        return compute(items);
    }

    static Map<String, Integer> compute(Map<String, Integer> rows) {
        return rows;
    }

    Runnable onSave() {
        return new Runnable() {
            public void run() {}
        };
    }
}

interface Store {
    void save(Invoice inv);
}

record Point(int x, int y) {}

enum Status {
    OPEN,
}
"#;

    const KT: &str = r#"package app

data class Order(val id: String)

class Service(private val repo: Repo) {
    private val cache = mutableMapOf<String, Order>()

    suspend fun load(id: String): Order {
        return parse(id)
    }

    fun parse(id: String): Order = Order(id)
}

fun String.slug(): String = lowercase()

fun Card(title: String, content: () -> Unit) {
}

fun screen() {
    Card(title = "Invoice") {
        withContext(Dispatchers.IO) {
        }
    }
    Card(title = "Order") {
    }
}

object Registry {
    val all = listOf<Order>()

    private const val TAG = "Invoice"
}

enum class Status {
    OPEN,
}

typealias Rows = List<Order>

const val LIMIT = 10

fun interface Handler {
    fun handle(order: Order)
}
"#;

    #[test]
    fn java_def_patterns_cover_types_methods_and_fields() {
        let (dir, files) = scratch("java", &[("Invoice.java", JAVA)]);
        let d = |w| defs(&dir, &files, Kind::Jvm, w);
        // The class and the constructor; the caller shows a picker.
        assert_eq!(d("Invoice"), [3, 7]);
        assert_eq!(d("LIMIT"), [4]);
        assert_eq!(d("items"), [5], "not the `compute(items)` call");
        assert_eq!(d("total"), [12], "behind its annotation and modifiers");
        // The declaration, not the `return compute(items);` call above it.
        assert_eq!(d("compute"), [16]);
        assert_eq!(d("onSave"), [20], "no modifier, but a return type");
        assert_eq!(d("run"), [22]);
        assert_eq!(d("Store"), [27]);
        assert_eq!(d("save"), [28], "an interface method has no body");
        assert_eq!(d("Point"), [31]);
        assert_eq!(d("Status"), [33]);
        assert_eq!(d("rows"), Vec::<usize>::new(), "a parameter has no rule");
        // `new Runnable() {` opens an anonymous class: a use of the interface, not a
        // declaration of it.
        assert_eq!(d("Runnable"), Vec::<usize>::new());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn kotlin_def_patterns_cover_declarations_and_receivers() {
        let (dir, files) = scratch("kt", &[("app.kt", KT)]);
        let d = |w| defs(&dir, &files, Kind::Jvm, w);
        assert_eq!(d("Order"), [3], "not the `Order(id)` calls");
        assert_eq!(d("Service"), [5]);
        assert_eq!(d("cache"), [6]);
        assert_eq!(d("load"), [8], "behind `suspend`");
        assert_eq!(d("parse"), [12], "not the `return parse(id)` call");
        assert_eq!(d("slug"), [15], "an extension function, past its receiver");
        assert_eq!(d("screen"), [20]);
        // The `fun`, not the two `Card(title = …) {` calls with a trailing lambda.
        assert_eq!(d("Card"), [17]);
        assert_eq!(
            d("withContext"),
            Vec::<usize>::new(),
            "a call with a trailing lambda is not a declaration"
        );
        assert_eq!(d("Registry"), [29]);
        assert_eq!(d("all"), [30]);
        assert_eq!(d("TAG"), [32], "behind `private const`");
        assert_eq!(d("Status"), [35]);
        assert_eq!(d("Rows"), [39]);
        assert_eq!(d("LIMIT"), [41]);
        assert_eq!(d("Handler"), [43], "a `fun interface`");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn java_and_kotlin_find_each_other() {
        let (dir, files) = scratch("jvm", &[("Invoice.java", JAVA), ("app.kt", KT)]);
        let pat = def_patterns(Kind::Jvm, "Store").join("|");
        assert_eq!(
            lines(&grep(&dir, &files, &pat, false, false)),
            [("Invoice.java".into(), 27)]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    const RB: &str = r#"module Billing
  LIMIT = 10

  class Invoice
    attr_accessor :total
    attr_reader :id, :customer

    @@count = 0

    def initialize(id)
      @id = id
      @rows = []
    end

    def self.parse(text)
      new(text)
    end

    def total=(value)
      @total = value
    end

    def cache
      @cache ||= {}
    end

    def empty?
      @rows.empty?
    end

    def ==(other)
      total == other.total && name =~ /x/
    end

    def to_h
      {
        id => 1,
      }
    end

    alias_method :blank?, :empty?
  end
end
"#;

    #[test]
    fn ruby_def_patterns_find_methods_attributes_and_assignments() {
        let (dir, files) = scratch("rb", &[("invoice.rb", RB)]);
        let d = |w| defs(&dir, &files, Kind::Ruby, w);
        assert_eq!(d("Billing"), [1]);
        assert_eq!(d("LIMIT"), [2], "a constant, indented in its module");
        assert_eq!(d("Invoice"), [4]);
        // The accessor, the setter and the assignment behind it -- not `total == other.total`.
        assert_eq!(d("total"), [5, 19, 20]);
        assert_eq!(d("customer"), [6], "second in the `attr_reader` list");
        // `id => 1,` is a hash pair, not an assignment.
        assert_eq!(d("id"), [6, 11]);
        assert_eq!(d("count"), [8], "a class variable");
        assert_eq!(d("initialize"), [10]);
        assert_eq!(d("parse"), [15], "`def self.parse`");
        assert_eq!(
            d("cache"),
            [23, 24],
            "the method and the `||=` it memoises with"
        );
        // `?` is not part of the word under the cursor, and `@rows.empty?` is a call.
        assert_eq!(d("empty"), [27]);
        assert_eq!(d("rows"), [12]);
        assert_eq!(d("blank"), [41]);
        assert_eq!(d("name"), Vec::<usize>::new(), "`name =~ /x/` is a match");
        assert_eq!(d("new"), Vec::<usize>::new());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    const C_H: &str = r#"#ifndef INVOICE_H
#define INVOICE_H

#define LRU_BITS 24
#define serverLog(level, ...) do { emit(level); } while (0)

typedef char *sds;
typedef int (*compare_fn)(const void *a, const void *b);

struct client;

struct __attribute__ ((__packed__)) sdshdr8 {
    uint8_t len;
};

static struct config {
    int port;
} server_config;

typedef struct invoice {
    sds name;
    int total;
    struct {
        int index;
    } offset;
} invoice;

typedef enum {
    STATE_NONE = 0,
    STATE_OPEN,
} state;

union value {
    int n;
    sds s;
};

extern struct invoice *current;

int invoice_total(struct invoice *inv);

#endif
"#;

    const C_C: &str = r#"#include "invoice.h"

struct invoice *current = NULL;
static int counter;

int invoice_total(struct invoice *inv) {
    if (invoice_valid(inv)) {
        return compute(inv);
    }
    return counter;
}

static int compute(struct invoice *inv)
{
    struct invoice *copy = inv;
    return copy->total;
}

static unsigned
invoice_index(const struct invoice *inv)
{
    return 0;
}

void invoice_free(struct invoice *inv,
                  int deep)
{
    free(inv);
}
"#;

    const CPP: &str = r#"#include "invoice.h"

namespace billing {

using Rows = std::vector<int>;
using std::swap;

template <typename T> struct Box : Base {
  T value;
};

template <typename T> struct Box<T *> : Base {
};

template <typename T = int>
class LEDGER_API Ledger : public Base {
 public:
  explicit Ledger(int n) : total_(n) {}

  auto total() const -> int { return total_; }

  void append(Rows rows);

  using Row = int;

 private:
  int total_;
};

void Ledger::append(Rows rows) {
  if (check(rows)) {
    log::write(rows);
  }
}

enum class Status {
  Open,
};

}  // namespace billing
"#;

    #[test]
    fn c_def_patterns_find_types_macros_functions_and_globals() {
        let (dir, files) = scratch("c-h", &[("invoice.h", C_H)]);
        let d = |w| defs(&dir, &files, Kind::C, w);
        assert_eq!(d("LRU_BITS"), [4]);
        assert_eq!(d("serverLog"), [5], "a function-like macro");
        assert_eq!(d("sds"), [7], "not the `sds name;` field it types");
        assert_eq!(d("compare_fn"), [8], "a function pointer");
        assert_eq!(d("client"), [10], "a forward declaration");
        assert_eq!(d("sdshdr8"), [12], "behind a lower-case attribute");
        assert_eq!(d("config"), [16], "behind a storage specifier");
        assert_eq!(d("server_config"), [18], "the name the block closes with");
        // The `typedef struct invoice {` and the `} invoice;` it closes with: `d` offers both,
        // where `D` lists the type once, under the name the project uses.
        assert_eq!(d("invoice"), [20, 26]);
        assert_eq!(d("state"), [31]);
        assert_eq!(d("value"), [33]);
        assert_eq!(d("current"), [38]);
        assert_eq!(d("invoice_total"), [40], "the prototype");
        assert_eq!(d("total"), Vec::<usize>::new(), "a field has no rule");
        assert_eq!(
            d("offset"),
            Vec::<usize>::new(),
            "an indented closing brace ends a nested anonymous struct: a field, not a type"
        );
        assert_eq!(
            d("STATE_OPEN"),
            Vec::<usize>::new(),
            "an enum constant has no rule: `NAME,` is also a line of an initializer list"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn c_def_patterns_tell_a_definition_from_a_call() {
        let (dir, files) = scratch("c-c", &[("invoice.c", C_C)]);
        let d = |w| defs(&dir, &files, Kind::C, w);
        assert_eq!(d("current"), [3]);
        assert_eq!(d("counter"), [4], "a global, not the `return counter;`");
        assert_eq!(d("invoice_total"), [6]);
        assert_eq!(d("compute"), [13], "the brace opens on the next line");
        assert_eq!(
            d("invoice_index"),
            [20],
            "the return type is on the line above"
        );
        assert_eq!(d("invoice_free"), [25], "the parameters wrap");
        // Column zero is where C declares; indented, only a body opening on the line counts.
        assert_eq!(
            d("invoice_valid"),
            Vec::<usize>::new(),
            "a call inside `if`"
        );
        assert_eq!(d("free"), Vec::<usize>::new(), "a call statement");
        assert_eq!(d("copy"), Vec::<usize>::new(), "a local");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cpp_def_patterns_cover_classes_methods_and_aliases() {
        let (dir, files) = scratch("cpp", &[("ledger.cc", CPP)]);
        let d = |w| defs(&dir, &files, Kind::C, w);
        assert_eq!(d("billing"), [3], "not the closing comment");
        assert_eq!(d("Rows"), [5]);
        assert_eq!(d("Box"), [8, 12], "the template and its specialization");
        assert_eq!(
            d("Ledger"),
            [16, 18],
            "past the template head; and its constructor"
        );
        assert_eq!(d("total"), [20], "a method defined in the class body");
        assert_eq!(d("Row"), [24], "a `using` alias indented in a class body");
        // The out-of-line definition. The declaration on line 22 has no rule: indented, it is
        // the shape of a call, and the definition is what `d` is asked for anyway.
        assert_eq!(d("append"), [30]);
        assert_eq!(d("Status"), [36]);
        assert_eq!(
            d("T"),
            Vec::<usize>::new(),
            "a template parameter is no global: `template <typename T = int>` declares nothing"
        );
        assert_eq!(
            d("swap"),
            Vec::<usize>::new(),
            "`using std::swap;` imports a name"
        );
        assert_eq!(d("check"), Vec::<usize>::new(), "a call inside `if`");
        assert_eq!(d("write"), Vec::<usize>::new(), "a qualified call");
        assert_eq!(d("Open"), Vec::<usize>::new(), "an enum constant");
        assert_eq!(d("total_"), Vec::<usize>::new(), "a field");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn c_and_cpp_scope_roots_and_names() {
        // One kind: a `.cc` is searched for a definition asked for in a `.h`, and a file of
        // another kind is not.
        let here = Path::new("ledger.hpp");
        assert!(in_def_scope(Kind::C, here, Path::new("src/invoice.c")));
        assert!(in_def_scope(Kind::C, here, Path::new("ledger.cc")));
        assert!(!in_def_scope(Kind::C, here, Path::new("main.rs")));
        // `#include` binds no name, and a member of a value has no rule of its own.
        assert!(imports(Kind::C, C_C).is_empty());
        assert!(member_patterns(Kind::C, "total").is_none());
        // The system headers, wherever this machine keeps them: the SDK on a Mac,
        // `/usr/include` on Linux. Both exist on CI, so the list is never empty there.
        let roots = external_roots(Kind::C, Path::new("/"));
        assert!(
            roots.iter().any(|r| r.ends_with("usr/include")),
            "no system include directory among {roots:?}"
        );
        // C++ writes a member behind `::`, as Rust does, and an access specifier is a label
        // inside the class, not a wall in front of it. The enclosing `namespace billing {` is
        // not indented, so, as in every kind, the walk ends at the first column-zero declaration.
        assert_eq!(
            qualified(Kind::C, CPP, 20, "total").as_deref(),
            Some("Ledger::total")
        );
        assert_eq!(qualified(Kind::C, CPP, 3, "billing"), None);
    }

    #[test]
    fn c_and_cpp_files_find_each_other() {
        let (dir, files) = scratch(
            "c-family",
            &[("invoice.h", C_H), ("invoice.c", C_C), ("ledger.cc", CPP)],
        );
        // The header's prototype and the definition that follows it are both offered; the
        // picker's rows say which file each is in.
        let pat = def_patterns(Kind::C, "invoice_total").join("|");
        assert_eq!(
            lines(&grep(&dir, &files, &pat, false, false)),
            [("invoice.c".into(), 6), ("invoice.h".into(), 40)],
            "the picker's order is by path, as everywhere: it is not definition before prototype"
        );
        // A type of the C header, reached from the C++ file that includes it.
        let pat = def_patterns(Kind::C, "sds").join("|");
        assert_eq!(
            lines(&grep(&dir, &files, &pat, false, false)),
            [("invoice.h".into(), 7)]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    const CS: &str = r#"namespace Billing.Core;

using Rows = System.Collections.Generic.List<int>;

[Serializable]
public sealed partial class Invoice<T> : Base, IEnumerable<T>
{
    private const int Limit = 10;
    private readonly ILogger<Invoice<T>> _logger;
    public static event EventHandler? Saved;

    public Invoice(int n)
    {
        _logger = Create(n);
    }

    public int Total { get; private set; }

    public string Name => _name;

    [HttpGet("{id}")]
    public async Task<Invoice<T>> LoadAsync(int id)
    {
        var rows = Compute(id);
        if (Check(rows))
        {
            Console.WriteLine(rows);
        }
        return new Invoice<T>(id);
    }

    int IComparable.CompareTo(object? other) => 0;

    private static Rows Compute(int id) => new Rows();
}

public interface IStore
{
    void Save(Invoice<int> inv);
}

public record struct Point(int X, int Y);

public record Money(decimal Amount);

public enum Status
{
    Open,
}

public delegate int Comparison<T>(T a, T b);

public static class Registry
{
    public static Dictionary<string, Invoice<int>> All = new();
}
"#;

    #[test]
    fn csharp_def_patterns_find_types_members_and_fields() {
        let (dir, files) = scratch("cs", &[("Invoice.cs", CS)]);
        let d = |w| defs(&dir, &files, Kind::CSharp, w);
        assert_eq!(d("Core"), [1], "a file-scoped namespace, by its last part");
        assert_eq!(
            d("Rows"),
            [3],
            "a `using` alias, not the `Rows` it is used as"
        );
        // The class past its generic parameters and its attribute, and the constructor; the
        // caller shows a picker.
        assert_eq!(d("Invoice"), [6, 12]);
        assert_eq!(d("Limit"), [8]);
        assert_eq!(d("_logger"), [9], "not the `_logger = Create(n);` write");
        assert_eq!(d("Saved"), [10], "an event");
        assert_eq!(d("Total"), [17], "a property, by its accessor block");
        assert_eq!(d("Name"), [19], "an expression-bodied property");
        assert_eq!(
            d("LoadAsync"),
            [22],
            "behind an attribute and `public async`"
        );
        assert_eq!(d("rows"), [24], "a local");
        assert_eq!(d("Compute"), [34], "not the `Compute(id)` call above it");
        assert_eq!(d("CompareTo"), [32], "an explicit interface implementation");
        assert_eq!(d("IStore"), [37]);
        assert_eq!(d("Save"), [39], "an interface method has no modifiers");
        assert_eq!(d("Point"), [42], "a positional `record struct`");
        assert_eq!(d("Money"), [44]);
        assert_eq!(d("Status"), [46]);
        assert_eq!(d("Comparison"), [51], "a delegate, past its return type");
        assert_eq!(d("Registry"), [53]);
        assert_eq!(d("All"), [55]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn csharp_def_patterns_tell_a_declaration_from_a_call() {
        let (dir, files) = scratch("cs-calls", &[("Invoice.cs", CS)]);
        let d = |w| defs(&dir, &files, Kind::CSharp, w);
        let none = Vec::<usize>::new();
        assert_eq!(d("Check"), none, "a call inside `if`");
        assert_eq!(d("Console"), none);
        assert_eq!(d("WriteLine"), none, "a call statement");
        assert_eq!(d("Create"), none, "a call on the right of an assignment");
        assert_eq!(d("Base"), none, "a base list is a use of the type");
        assert_eq!(d("IEnumerable"), none);
        assert_eq!(d("id"), none, "a parameter has no rule");
        assert_eq!(
            d("Open"),
            none,
            "an enum member has no rule: `Open,` is also a line of a collection initialiser"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn csharp_scope_stays_in_the_project() {
        let here = Path::new("Invoice.cs");
        assert!(in_def_scope(Kind::CSharp, here, Path::new("src/Store.csx")));
        assert!(!in_def_scope(Kind::CSharp, here, Path::new("main.rs")));
        // A `using` opens a whole namespace, so it binds no name of its own, and there is nothing
        // to bind it to: a NuGet package ships assemblies, not source.
        assert!(imports(Kind::CSharp, CS).is_empty());
        assert!(external_roots(Kind::CSharp, Path::new("/")).is_empty());
        assert!(member_patterns(Kind::CSharp, "Total").is_none());
        // A member is named by the type it is declared in, as the picker rows show it.
        assert_eq!(
            qualified(Kind::CSharp, CS, 22, "LoadAsync").as_deref(),
            Some("Invoice.LoadAsync")
        );
    }

    const SWIFT: &str = r#"import Foundation

public protocol RequestDelegate: AnyObject {
    associatedtype Value

    func didFinish(_ request: Request)
}

@objc(AFSession)
public final class Session: NSObject {
    public static let `default` = Session()

    private let queue: DispatchQueue
    public var isRunning = false

    public init(queue: DispatchQueue = .main) {
        self.queue = queue
    }

    convenience init?(name: String) {
        self.init()
    }

    public func request<T: Encodable>(_ url: URL, with body: T) -> Request {
        let request = Request(url)
        queue.async {
            self.start(request)
        }
        if let delegate = delegate {
            delegate.didFinish(request)
        }
        return request
    }

    class func shared() -> Session {
        return Session()
    }
}

extension Session: RequestDelegate {
    public func didFinish(_ request: Request) {
        switch request.state {
        case .finished:
            break
        case let .failed(error):
            print(error)
        }
    }
}

public enum State {
    case initialized
    case resumed(Int), suspended
    case failed(Error)
}

public struct Response<Value> {
    let value: Value
}

actor Cache {
    var entries: [String: Data] = [:]
}

public typealias Rows = [Int]

public final class Store {
    public private(set) weak var owner: Session?
}

let opened = 0

func describe(_ code: Int) -> String {
    switch code {
    case opened:
        return "opened"
    default:
        return ""
    }
}
"#;

    #[test]
    fn swift_def_patterns_find_declarations_behind_attributes_and_modifiers() {
        let (dir, files) = scratch("swift", &[("Session.swift", SWIFT)]);
        let d = |w| defs(&dir, &files, Kind::Swift, w);
        assert_eq!(d("RequestDelegate"), [3], "a protocol");
        assert_eq!(d("Value"), [4], "an `associatedtype`");
        // The class and the extension of it: a project's own members of a type live in one.
        assert_eq!(d("Session"), [10, 40], "not the `Session()` calls");
        assert_eq!(d("didFinish"), [6, 41], "not the `delegate.didFinish` call");
        assert_eq!(d("default"), [11], "a backticked name");
        assert_eq!(d("queue"), [13], "not the `self.queue = queue` write");
        assert_eq!(d("isRunning"), [14]);
        assert_eq!(
            d("init"),
            [16, 20],
            "`init?` too, not the `self.init()` call"
        );
        assert_eq!(d("request"), [24, 25], "the function and the local");
        assert_eq!(d("shared"), [35], "behind `class`, Swift's static method");
        assert_eq!(d("initialized"), [52], "an enum case");
        assert_eq!(d("resumed"), [53], "with its associated value");
        assert_eq!(d("suspended"), [53], "second on the line");
        assert_eq!(
            d("failed"),
            [54],
            "not the `case let .failed(error):` pattern"
        );
        assert_eq!(d("State"), [51]);
        assert_eq!(d("Response"), [57], "past the generic parameters");
        assert_eq!(d("Cache"), [61], "an actor");
        assert_eq!(d("entries"), [62]);
        assert_eq!(d("Rows"), [65], "a `typealias`");
        assert_eq!(d("Store"), [67]);
        assert_eq!(d("owner"), [68], "behind `public private(set) weak`");
        // The `let`, not the `case opened:` of the `switch` below it, which matches against
        // that constant: a bare name there is a pattern, not a declaration.
        assert_eq!(d("opened"), [71]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn swift_def_patterns_tell_a_declaration_from_a_call_or_a_pattern() {
        let (dir, files) = scratch("swift-calls", &[("Session.swift", SWIFT)]);
        let d = |w| defs(&dir, &files, Kind::Swift, w);
        let none = Vec::<usize>::new();
        assert_eq!(d("finished"), none, "`case .finished:` is a pattern");
        assert_eq!(d("start"), none, "a call on `self`");
        assert_eq!(d("print"), none);
        assert_eq!(d("Request"), none, "a type this file only uses");
        assert_eq!(
            d("delegate"),
            none,
            "an `if let` rebinds a name declared elsewhere: no rule, so `u` answers"
        );
        assert_eq!(d("body"), none, "a parameter has no rule");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn swift_scope_roots_and_names() {
        let here = Path::new("Session.swift");
        assert!(in_def_scope(
            Kind::Swift,
            here,
            Path::new("Source/Request.swift")
        ));
        assert!(!in_def_scope(Kind::Swift, here, Path::new("main.rs")));
        // An `import` names a module and makes everything in it visible unqualified, so it binds
        // no name of its own.
        assert!(imports(Kind::Swift, SWIFT).is_empty());
        // Outside the project is where SwiftPM checks the dependencies out; nothing else on the
        // machine holds Swift source, so an absent directory leaves the list empty.
        let (dir, _) = scratch(
            "swift-roots",
            &[(".build/checkouts/nio/Sources/a.swift", "")],
        );
        assert_eq!(
            external_roots(Kind::Swift, &dir),
            [dir.join(".build/checkouts")]
        );
        assert!(external_roots(Kind::Swift, Path::new("/")).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
        // A member is named by the type its extension extends.
        assert_eq!(
            qualified(Kind::Swift, SWIFT, 41, "didFinish").as_deref(),
            Some("Session.didFinish")
        );
    }

    const PHP: &str = r#"<?php

namespace App\Services;

use Illuminate\Support\Str;
use App\Models\User as Account;

define('BILLING_LIMIT', 10);

abstract class Invoice implements Arrayable
{
    public const STATUS_OPEN = 'open';

    protected array $rows = [];

    private ?Logger $logger;

    public function __construct(private readonly Account $account, string $name)
    {
        $this->logger = null;
        $total = 0;
        foreach ($this->rows as $key => $value) {
            $total += $value;
        }
        $this->name = $name;
    }

    final public static function parse(string $text): static
    {
        return new static($text);
    }

    public function &rows(): array
    {
        return $this->rows;
    }

    abstract protected function compute(): int;
}

interface Arrayable
{
    public function toArray(): array;
}

trait Macroable
{
    public function macro(string $name): void
    {
    }
}

enum Status: string
{
    case Open = 'open';
    case Closed;
}

function billing_total(Invoice $invoice): int
{
    $sum = 0;
    return $sum;
}

function billing_report(array $rows, int $total): array
{
    $map = [
        $key => $value,
    ];

    return
        $total == 0 ? $map : $rows;
}
"#;

    #[test]
    fn php_def_patterns_find_declarations_behind_modifiers() {
        let (dir, files) = scratch("php", &[("Invoice.php", PHP)]);
        let d = |w| defs(&dir, &files, Kind::Php, w);
        assert_eq!(d("Services"), [3], "a namespace, by its last part");
        assert_eq!(d("BILLING_LIMIT"), [8], "a `define()` constant");
        assert_eq!(d("Invoice"), [10], "not the `Invoice $invoice` parameter");
        assert_eq!(d("STATUS_OPEN"), [12], "a class constant");
        // The property and the method that returns it, not the `$this->rows` uses.
        assert_eq!(d("rows"), [14, 33]);
        assert_eq!(d("logger"), [16], "not the `$this->logger = null;` write");
        assert_eq!(d("account"), [18], "a promoted constructor parameter");
        assert_eq!(
            d("total"),
            [21, 23],
            "the assignment and the `+=` that follows"
        );
        assert_eq!(d("parse"), [28], "behind `final public static`");
        assert_eq!(d("compute"), [38], "an abstract method has no body");
        assert_eq!(d("Arrayable"), [41], "not the `implements Arrayable`");
        assert_eq!(d("toArray"), [43]);
        assert_eq!(d("Macroable"), [46], "a trait");
        assert_eq!(d("macro"), [48]);
        assert_eq!(d("Status"), [53], "a backed enum");
        assert_eq!(d("Open"), [55], "an enum case with its value");
        assert_eq!(d("Closed"), [56]);
        assert_eq!(d("billing_total"), [59], "a function at the top level");
        assert_eq!(d("sum"), [61], "not the `return $sum;`");
        assert_eq!(d("map"), [67]);
        // Not the `$total == 0` that opens line 72: `==` compares, it declares nothing.
        assert_eq!(d("total"), [21, 23]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn php_def_patterns_tell_a_declaration_from_a_use() {
        let (dir, files) = scratch("php-uses", &[("Invoice.php", PHP)]);
        let d = |w| defs(&dir, &files, Kind::Php, w);
        let none = Vec::<usize>::new();
        assert_eq!(d("Str"), none, "a `use` imports a name, it declares none");
        assert_eq!(d("Account"), none, "nor does the alias of one");
        assert_eq!(d("Logger"), none, "a type a property is written with");
        assert_eq!(d("text"), none, "a parameter has no rule");
        assert_eq!(
            d("name"),
            none,
            "`$this->name = $name;` writes to a property declared elsewhere"
        );
        assert_eq!(
            d("key"),
            none,
            "a `foreach` target has no rule, and `$key => $value,` is an array pair"
        );
        assert_eq!(d("value"), none);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn php_scope_roots_imports_and_names() {
        let here = Path::new("src/Invoice.php");
        assert!(in_def_scope(Kind::Php, here, Path::new("views/show.phtml")));
        assert!(!in_def_scope(Kind::Php, here, Path::new("main.rs")));
        // A `use` in column zero binds the last part of the path, or its alias; PSR-4 spells the
        // namespace the way the file system does, so the path is the name split on `\`.
        assert_eq!(
            imports(Kind::Php, PHP),
            [
                (
                    "Str".to_owned(),
                    vec!["Illuminate".into(), "Support".into(), "Str".into()]
                ),
                (
                    "Account".to_owned(),
                    vec!["App".into(), "Models".into(), "User".into()]
                ),
            ]
        );
        // An indented `use` pulls a trait into a class body and names no file.
        assert!(imports(Kind::Php, "class X {\n    use Macroable;\n}\n").is_empty());
        // Composer installs the dependencies into `vendor/`, which is gitignored and so outside
        // the project walk, the way `node_modules` is.
        let (dir, _) = scratch("php-roots", &[("vendor/laravel/framework/src/a.php", "")]);
        assert_eq!(external_roots(Kind::Php, &dir), [dir.join("vendor")]);
        assert!(external_roots(Kind::Php, Path::new("/")).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
        // PHP writes a method of a class behind `::`, as its own documentation does.
        assert_eq!(
            qualified(Kind::Php, PHP, 28, "parse").as_deref(),
            Some("Invoice::parse")
        );
    }

    const LUA: &str = r#"local uv = vim.uv

local M = {}
local cache, hits = {}, 0

function M.setup(opts)
  local defaults = { limit = 10 }
  cache = defaults
  return M.normalise(opts)
end

function M:render(row)
  return row
end

function normalise(opts)
  return opts
end

local function trim(s)
  return s
end

M.format = function(row)
  return trim(row)
end

local handlers = {
  open = function(id)
    return id
  end,
  limit = 10,
}

--[[
function M.ghost(x)
  return x
end
]]

M.setup({ limit = 1 })
return M

local pat = [=[
^\s*\%(\[[A-Za-z]\+\]\)* ]-] x
function M.ghosted(x)
end
]=]

function M.after(x)
  return x
end

local sql = [[
function M.ghost2(x)
end
]]

function M.last() end
"#;

    #[test]
    fn lua_def_patterns_find_functions_and_locals() {
        let (dir, files) = scratch("lua", &[("init.lua", LUA)]);
        let d = |w| defs(&dir, &files, Kind::Lua, w);
        assert_eq!(d("setup"), [6], "the declaration, not the call on line 41");
        assert_eq!(d("render"), [12], "the `M:name` form");
        assert_eq!(d("normalise"), [16], "not the `M.normalise(opts)` call");
        assert_eq!(d("trim"), [20], "`local function`");
        assert_eq!(d("format"), [24], "`M.name = function`");
        assert_eq!(d("open"), [29], "a function in a table of handlers");
        assert_eq!(d("M"), [3]);
        assert_eq!(d("uv"), [1]);
        // `local a, b = …` declares both, and a later bare `cache = …` is an assignment to the
        // local already declared, not a declaration of its own.
        assert_eq!(d("cache"), [4]);
        assert_eq!(d("hits"), [4]);
        assert_eq!(d("defaults"), [7], "a local inside a body");
        assert_eq!(
            d("limit"),
            Vec::<usize>::new(),
            "a table field holding a value has no rule: the line is also an assignment"
        );
        assert_eq!(d("opts"), Vec::<usize>::new(), "a parameter");
        assert_eq!(d("row"), Vec::<usize>::new());
        assert_eq!(
            d("vim"),
            Vec::<usize>::new(),
            "the right-hand side of a local"
        );
        // A `[=[ … ]=]` long string closes on the `=` it was opened with, so neither the
        // `\[[` of the Vim regex inside it nor the `]-]` closes it, and what follows the
        // string is still read as code.
        assert_eq!(d("pat"), [44]);
        assert_eq!(d("after"), [50]);
        assert_eq!(d("last"), [59]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn lua_long_brackets_hide_what_they_hold() {
        // The `--[[ … ]]` block comment on lines 35-39, the `[=[ … ]=]` string on 44-48 and
        // the `[[ … ]]` one on 54-57: the functions inside them declare nothing, the way a
        // Python docstring's example does not.
        let lit = literal_lines(Kind::Lua, LUA);
        assert_eq!(
            lit.iter()
                .enumerate()
                .filter(|(_, l)| **l)
                .map(|(i, _)| i + 1)
                .collect::<Vec<_>>(),
            [36, 37, 38, 39, 45, 46, 47, 48, 55, 56, 57]
        );
        // A `--` line comment is still one line, whatever quote it holds.
        assert!(
            literal_lines(Kind::Lua, "-- don't\nlocal x = 1\n")[1..]
                .iter()
                .all(|l| !l)
        );
    }

    #[test]
    fn lua_scope_roots_and_names() {
        let here = Path::new("lua/config/init.lua");
        assert!(in_def_scope(
            Kind::Lua,
            here,
            Path::new("lua/plugins/ui.lua")
        ));
        assert!(!in_def_scope(Kind::Lua, here, Path::new("main.c")));
        // `require "x"` binds a name, but there is no root to resolve it against and Lua's own
        // `package.path` is the embedding interpreter's, so nothing is bound and nothing is
        // searched outside the project.
        assert!(imports(Kind::Lua, LUA).is_empty());
        assert!(external_roots(Kind::Lua, Path::new("/")).is_empty());
        assert!(member_patterns(Kind::Lua, "setup").is_none());
        // A function nested in another is named under it, as in every kind, and a `--`
        // comment in between is a comment, not a declaration that names nothing.
        assert_eq!(
            qualified(
                Kind::Lua,
                "function M.setup()\n-- a note\n  local function inner() end\nend\n",
                3,
                "inner"
            )
            .as_deref(),
            Some("setup.inner")
        );
    }

    #[test]
    fn lua_symbol_names() {
        let lua = |line| one(Kind::Lua, line);
        for (line, name) in [
            ("function setup(opts)", Some("setup")),
            ("function M.setup(opts)", Some("setup")),
            ("function M:render(row)", Some("render")),
            ("function vim.lsp.util.clamp(x)", Some("clamp")),
            ("local function trim(s)", Some("trim")),
            ("  local function inner()", Some("inner")),
            ("M.format = function(row)", Some("format")),
            ("local format = function(row)", Some("format")),
            ("  open = function(id)", Some("open")),
            // Not a declaration: a call, a field holding a value, a local, a return.
            ("M.setup({ limit = 1 })", None),
            ("  limit = 10,", None),
            ("local M = {}", None),
            ("local cache, hits = {}, 0", None),
            ("  return M.normalise(opts)", None),
            ("  end,", None),
            ("-- function ghost(x)", None),
        ] {
            assert_eq!(lua(line).as_deref(), name, "{line}");
        }
    }

    const EX: &str = r#"defmodule MyApp.Ledger do
  @moduledoc """
  Examples:

      def ghost(x), do: x
  """

  @timeout 5_000
  @derive {Jason.Encoder, only: [:id]}

  defstruct [:id, :total, currency: "EUR"]

  @type t :: %__MODULE__{}

  @spec parse(String.t()) :: t
  def parse(nil), do: nil

  def parse(raw) when is_binary(raw) do
    %__MODULE__{id: raw}
  end

  defp normalise(raw) do
    String.trim(raw)
  end

  defmacro with_total(do: block) do
    block
  end

  defguard is_positive(n) when n > 0

  defdelegate encode(value), to: Jason

  def timeout, do: @timeout
end

defprotocol Renderable do
  def render(value)
end

defimpl Renderable, for: MyApp.Ledger do
  def render(ledger), do: ledger.id
end

defmodule MyApp.LedgerTest do
  @moduletag :slow
  @tag :external

  defmacrop guard!(x), do: x
  defguardp is_even(n) when rem(n, 2) == 0

  def empty?(rows), do: rows == []
  def put!(row), do: row
end
"#;

    #[test]
    fn elixir_def_patterns_find_every_def_form() {
        let (dir, files) = scratch("ex", &[("ledger.ex", EX)]);
        let d = |w| defs(&dir, &files, Kind::Elixir, w);
        assert_eq!(
            d("Ledger"),
            [1],
            "the last part of `defmodule MyApp.Ledger`"
        );
        assert_eq!(d("Renderable"), [37], "not the `defimpl` that uses it");
        // Two clauses of one function are two declarations, so both are offered; the `@spec`
        // above them is a promise about `parse`, not its definition.
        assert_eq!(d("parse"), [16, 18]);
        assert_eq!(d("normalise"), [22], "`defp`");
        assert_eq!(d("with_total"), [26], "`defmacro`");
        assert_eq!(d("is_positive"), [30], "`defguard`");
        assert_eq!(d("encode"), [32], "`defdelegate`");
        assert_eq!(d("render"), [38, 42], "the protocol and its implementation");
        // The attribute and the function of the same name are both declarations, of different
        // things, so `d` offers both rather than guessing.
        assert_eq!(d("timeout"), [8, 34]);
        assert_eq!(d("id"), [11], "a struct field, atom list form");
        assert_eq!(d("currency"), [11], "the keyword form of the same line");
        // The attributes the language owns, and the names they talk about.
        assert_eq!(d("t"), Vec::<usize>::new(), "`@type t ::` declares no `t`");
        assert_eq!(d("spec"), Vec::<usize>::new());
        assert_eq!(d("type"), Vec::<usize>::new());
        assert_eq!(d("moduledoc"), Vec::<usize>::new());
        assert_eq!(d("derive"), Vec::<usize>::new());
        assert_eq!(d("MyApp"), Vec::<usize>::new(), "a namespace, not a module");
        assert_eq!(d("raw"), Vec::<usize>::new(), "a parameter");
        assert_eq!(d("block"), Vec::<usize>::new());
        assert_eq!(d("Jason"), Vec::<usize>::new());
        assert_eq!(d("guard"), [49], "`defmacrop`, past the trailing `!`");
        assert_eq!(d("is_even"), [50], "`defguardp`");
        // A name Elixir spells with a trailing `?` or `!` is found from the bare word, as
        // Ruby's is: the cursor on `empty` in `empty?(rows)` reaches `def empty?`.
        assert_eq!(d("empty"), [52]);
        assert_eq!(d("put"), [53]);
        // ExUnit's and Mix's attributes are directives too, so `d` on one has nothing to find
        // rather than a picker of every place the directive is written.
        assert_eq!(d("tag"), Vec::<usize>::new());
        assert_eq!(d("moduletag"), Vec::<usize>::new());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn elixir_heredocs_hide_what_they_hold() {
        // `@moduledoc """ … """` on lines 2-6: the `def ghost(x)` of its example declares
        // nothing, as a Python docstring's does not.
        let lit = literal_lines(Kind::Elixir, EX);
        assert_eq!(
            lit.iter()
                .enumerate()
                .filter(|(_, l)| **l)
                .map(|(i, _)| i + 1)
                .collect::<Vec<_>>(),
            [3, 4, 5, 6]
        );
    }

    #[test]
    fn elixir_scope_roots_and_names() {
        let here = Path::new("lib/my_app/ledger.ex");
        assert!(in_def_scope(
            Kind::Elixir,
            here,
            Path::new("test/ledger_test.exs")
        ));
        assert!(!in_def_scope(Kind::Elixir, here, Path::new("mix.lock")));
        // `alias` and `import` bind names, but `mix` puts the dependencies in `deps/` inside the
        // project, so they are project files already and there is no root to leave for.
        assert!(imports(Kind::Elixir, EX).is_empty());
        assert!(external_roots(Kind::Elixir, Path::new("/")).is_empty());
        assert!(member_patterns(Kind::Elixir, "parse").is_none());
        // A function is named under the module it is written in, as in every kind.
        assert_eq!(
            qualified(Kind::Elixir, EX, 22, "normalise").as_deref(),
            Some("Ledger.normalise")
        );
        assert_eq!(qualified(Kind::Elixir, EX, 1, "Ledger"), None);
    }

    #[test]
    fn elixir_symbol_names() {
        let ex = |line| one(Kind::Elixir, line);
        for (line, name) in [
            ("defmodule MyApp.Ledger do", Some("Ledger")),
            ("defmodule Ledger do", Some("Ledger")),
            ("defprotocol Renderable do", Some("Renderable")),
            ("  def parse(nil), do: nil", Some("parse")),
            ("  def timeout, do: @timeout", Some("timeout")),
            ("  defp normalise(raw) do", Some("normalise")),
            ("  def empty?(rows), do: rows == []", Some("empty?")),
            ("  def put!(row), do: row", Some("put!")),
            ("  defmacro with_total(do: block) do", Some("with_total")),
            ("  defmacrop guard!(x), do: x", Some("guard!")),
            ("  defguard is_positive(n) when n > 0", Some("is_positive")),
            (
                "  defguardp is_even(n) when rem(n, 2) == 0",
                Some("is_even"),
            ),
            ("  defdelegate encode(value), to: Jason", Some("encode")),
            // A `defimpl` names the module `Protocol.Type`, and neither half is its own name;
            // `defstruct` declares every field on one line; an attribute belongs to the language.
            ("defimpl Renderable, for: MyApp.Ledger do", None),
            ("  defstruct [:id, :total]", None),
            ("  @spec parse(String.t()) :: t", None),
            ("  @type t :: %__MODULE__{}", None),
            ("  @moduledoc \"\"\"", None),
            ("  @timeout 5_000", None),
            // The shared pattern called this a declaration of `x`.
            ("    Enum.map(rows, fn x -> x.id end)", None),
            ("    String.trim(raw)", None),
            ("  end", None),
        ] {
            assert_eq!(ex(line).as_deref(), name, "{line}");
        }
    }

    const ZIG: &str = r#"const std = @import("std");
const Allocator = std.mem.Allocator;

pub const Error = error{OutOfRange};

pub const Ledger = struct {
    total: u32,
    rows: []const Row,

    const empty: Ledger = .{ .total = 0, .rows = &.{} };

    pub fn init(allocator: Allocator) Ledger {
        var self = Ledger{ .total = 0, .rows = &.{} };
        return self;
    }

    pub inline fn isEmpty(self: Ledger) bool {
        return self.rows.len == 0;
    }

    fn compute(self: Ledger) u32 {
        return self.total;
    }
};

pub const Row = struct { id: u32 };

const Status = enum { open, closed };

const Value = union(enum) { n: u32, s: []const u8 };

pub var counter: u32 = 0;
threadlocal var scratch: [16]u8 = undefined;

export fn ledger_total(l: *Ledger) u32 {
    return l.total;
}

pub extern "c" fn strlen(s: [*:0]const u8) usize;

noinline fn slow(x: u32) u32 {
    return x;
}

test "a ledger starts empty" {
    const l = Ledger.init(std.testing.allocator);
    try std.testing.expect(l.isEmpty());
}

const first, const second = .{ 1, 2 };

extern fn puts(s: [*:0]const u8) c_int;

export inline fn fast(x: u32) u32 {
    comptime var seen: u32 = 0;
    seen += x;
    return seen;
}

const help =
    \\```zig
    \\const x = 1;
    \\```
;

pub fn after() void {}
"#;

    #[test]
    fn zig_def_patterns_find_functions_types_and_constants() {
        let (dir, files) = scratch("zig", &[("ledger.zig", ZIG)]);
        let d = |w| defs(&dir, &files, Kind::Zig, w);
        assert_eq!(d("std"), [1]);
        assert_eq!(d("Allocator"), [2]);
        assert_eq!(d("Error"), [4]);
        assert_eq!(
            d("Ledger"),
            [6],
            "not the literal on line 13 or the call on line 46"
        );
        assert_eq!(d("empty"), [10], "a constant in a struct body");
        assert_eq!(d("init"), [12], "not the `Ledger.init(…)` call on line 46");
        assert_eq!(d("isEmpty"), [17], "`pub inline fn`");
        assert_eq!(d("compute"), [21]);
        assert_eq!(
            d("Row"),
            [26],
            "not the `rows: []const Row` field that uses it"
        );
        assert_eq!(d("Status"), [28], "`const X = enum`");
        assert_eq!(d("Value"), [30], "`const X = union(enum)`");
        assert_eq!(d("counter"), [32], "`pub var`");
        assert_eq!(d("scratch"), [33], "`threadlocal var`");
        assert_eq!(d("ledger_total"), [35], "`export fn`");
        assert_eq!(d("strlen"), [39], r#"`pub extern "c" fn`"#);
        assert_eq!(d("slow"), [41], "`noinline fn`");
        assert_eq!(
            d("self"),
            [13],
            "a local; the parameters of lines 17 and 21 are not"
        );
        assert_eq!(d("l"), [46]);
        assert_eq!(
            d("total"),
            Vec::<usize>::new(),
            "a struct field has no rule"
        );
        assert_eq!(d("id"), Vec::<usize>::new());
        assert_eq!(
            d("ledger"),
            Vec::<usize>::new(),
            "a word inside a test description declares nothing"
        );
        assert_eq!(d("open"), Vec::<usize>::new(), "an enum field");
        // A destructuring declares both names, but only the first one starts the line, and every
        // rule here is anchored there.
        assert_eq!(d("first"), [50]);
        assert_eq!(d("second"), Vec::<usize>::new());
        assert_eq!(d("puts"), [52], "`extern fn`, with no calling convention");
        assert_eq!(d("fast"), [54], "`export inline fn`");
        assert_eq!(d("seen"), [55], "`comptime var`");
        // Zig has no literal that runs over lines: a `\\` string ends with its line, so the
        // markdown fences on 61-63 open nothing and the declaration below them is still found.
        assert_eq!(d("after"), [66]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn zig_has_no_literal_that_runs_over_lines() {
        // A `\\` string holding markdown is idiomatic in Zig, and its ``` fences are not a
        // TypeScript template: reading Zig with the C family's rules would hide every line
        // after the first fence, and `d` would say `no definition` over code it can see.
        assert!(
            literal_lines(Kind::Zig, ZIG).iter().all(|l| !l),
            "a Zig line was taken for the inside of a literal"
        );
    }

    #[test]
    fn zig_scope_roots_and_names() {
        let here = Path::new("src/main.zig");
        assert!(in_def_scope(Kind::Zig, here, Path::new("src/ledger.zig")));
        assert!(!in_def_scope(Kind::Zig, here, Path::new("build.zig.zon")));
        assert!(imports(Kind::Zig, ZIG).is_empty());
        assert!(member_patterns(Kind::Zig, "init").is_none());
        // The standard library `zig env` reports, on a machine that has a `zig`; nothing at all
        // on one that does not, as for every kind whose toolchain is not installed.
        assert!(
            external_roots(Kind::Zig, Path::new("/"))
                .iter()
                .all(|r| r.is_dir() && r.ends_with("std")),
            "a Zig root that is not an existing `std` directory"
        );
        // A method is named under the type it is declared in, as in every kind.
        assert_eq!(
            qualified(Kind::Zig, ZIG, 12, "init").as_deref(),
            Some("Ledger.init")
        );
        assert_eq!(qualified(Kind::Zig, ZIG, 6, "Ledger"), None);
    }

    #[test]
    fn zig_std_comes_from_zig_env() {
        // What `zig env` prints: JSON on some versions, ZON on others.
        let json = "{\n \"zig_exe\": \"/opt/homebrew/bin/zig\",\n \"lib_dir\": \"/opt/lib/zig\",\n \"std_dir\": \"/opt/lib/zig/std\"\n}\n";
        assert_eq!(zig_roots(json), [PathBuf::from("/opt/lib/zig/std")]);
        let zon = ".{ .zig_exe = \"/usr/bin/zig\", .lib_dir = \"/usr/lib/zig\", .std_dir = \"/usr/lib/zig/std\" }\n";
        assert_eq!(zig_roots(zon), [PathBuf::from("/usr/lib/zig/std")]);
        // A version that reports only the library directory the standard library sits in.
        assert_eq!(
            zig_roots("{\"lib_dir\": \"/usr/lib/zig\"}"),
            [PathBuf::from("/usr/lib/zig/std")]
        );
        // No `zig` on this machine: nothing to search outside the project.
        assert!(zig_roots("").is_empty());
    }

    #[test]
    fn zig_symbol_names() {
        let zig = |line| one(Kind::Zig, line);
        for (line, name) in [
            // The shared pattern reads these; the rows of this kind must not list them again.
            ("pub const Ledger = struct {", Some("Ledger")),
            ("const Status = enum { open, closed };", Some("Status")),
            ("const Value = union(enum) { n: u32 };", Some("Value")),
            (
                "    pub fn init(allocator: Allocator) Ledger {",
                Some("init"),
            ),
            ("    fn compute(self: Ledger) u32 {", Some("compute")),
            (
                "export fn ledger_total(l: *Ledger) u32 {",
                Some("ledger_total"),
            ),
            (
                "pub extern \"c\" fn strlen(s: [*:0]const u8) usize;",
                Some("strlen"),
            ),
            // These it has no word for.
            (
                "    pub inline fn isEmpty(self: Ledger) bool {",
                Some("isEmpty"),
            ),
            ("noinline fn slow(x: u32) u32 {", Some("slow")),
            (
                "export inline fn ledger_total(l: *Ledger) u32 {",
                Some("ledger_total"),
            ),
            (
                "pub extern \"c\" inline fn strlen(s: [*:0]const u8) usize;",
                Some("strlen"),
            ),
            (
                "test \"a ledger starts empty\" {",
                Some("a ledger starts empty"),
            ),
            // A global, a local and a field stay off the list, as in every other kind.
            ("pub var counter: u32 = 0;", None),
            ("threadlocal var scratch: [16]u8 = undefined;", None),
            ("        var self = Ledger{ .total = 0 };", None),
            ("    const empty: Ledger = .{ .total = 0 };", None),
            ("    total: u32,", None),
            ("    return self.total;", None),
            ("    try std.testing.expect(l.isEmpty());", None),
        ] {
            assert_eq!(zig(line).as_deref(), name, "{line}");
        }
    }

    const SH: &str = "#!/usr/bin/env bash\nset -eu\n\nexport ROOT=/srv\nlocal -i tries=3\ndeclare -r -x LIMIT=10\nreadonly NAME=app\nPATH+=:/opt/bin\nalias ll='ls -l'\n\nbuild() {\n  echo \"$ROOT\"\n}\n\nfunction deploy {\n  build\n}\n\nfunction check() {\n  [ \"$NAME\" = app ]\n}\n\nbuild \"$ROOT\"\n";

    #[test]
    fn shell_def_patterns_find_functions_assignments_and_aliases() {
        let (dir, files) = scratch("sh", &[("run.sh", SH)]);
        let d = |w| defs(&dir, &files, Kind::Shell, w);
        // The definition, not the `build` call on line 16 or line 23.
        assert_eq!(d("build"), [11]);
        assert_eq!(d("deploy"), [15]);
        assert_eq!(d("check"), [19], "`function name()` counts once");
        assert_eq!(d("ROOT"), [4], "not the `\"$ROOT\"` uses");
        assert_eq!(d("tries"), [5]);
        assert_eq!(d("LIMIT"), [6], "behind `declare` and its flags");
        // The `readonly` assignment, not the `[ \"$NAME\" = app ]` test.
        assert_eq!(d("NAME"), [7]);
        assert_eq!(d("PATH"), [8], "`+=` appends to a variable");
        assert_eq!(d("ll"), [9]);
        assert_eq!(d("echo"), Vec::<usize>::new());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    const SQL: &str = r#"CREATE TABLE public.orders (
  id serial PRIMARY KEY,
  customer_id int REFERENCES customers(id)
);

CREATE OR REPLACE FUNCTION total(o int) RETURNS int AS $$ SELECT 0 $$ LANGUAGE sql;

CREATE UNIQUE INDEX orders_id_idx ON public.orders (id);

create materialized view daily_totals as select 1;

CREATE TYPE mood AS ENUM ('ok', 'bad');

CREATE TABLE IF NOT EXISTS billing.invoices (id int);

CREATE TABLE "user" (id int);

WITH recent AS (
  SELECT * FROM public.orders
), older AS (
  SELECT * FROM archive
)
SELECT * FROM recent JOIN older ON true;

SELECT * FROM customers;
JOIN customers ON true
INSERT INTO customers VALUES (1);
ALTER TABLE customers ADD COLUMN x int;
DROP TABLE customers;
"#;

    #[test]
    fn sql_def_patterns_find_create_statements_and_ctes() {
        let (dir, files) = scratch("sql", &[("schema.sql", SQL)]);
        let d = |w| defs(&dir, &files, Kind::Sql, w);
        // The bare name finds the schema-qualified `CREATE TABLE`.
        assert_eq!(d("orders"), [1]);
        assert_eq!(d("total"), [6]);
        assert_eq!(d("orders_id_idx"), [8]);
        // Lower-case keywords read the same.
        assert_eq!(d("daily_totals"), [10]);
        assert_eq!(d("mood"), [12]);
        assert_eq!(d("invoices"), [14]);
        assert_eq!(d("user"), [16], "a quoted name");
        // The `WITH` and the `,` continuation both open a CTE.
        assert_eq!(d("recent"), [18]);
        assert_eq!(d("older"), [20]);
        // `customers` is only ever used, never created: no definition.
        assert_eq!(d("customers"), Vec::<usize>::new());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    const MAKE: &str = ".PHONY: build test\nCC ?= gcc\nexport CFLAGS := -O2\nbuild test-all: deps\n\t$(CC) -o app\ndeps::\n\t@echo deps\n%.o: %.c\n";

    #[test]
    fn make_def_patterns_find_targets_and_variables() {
        let (dir, files) = scratch("make", &[("Makefile", MAKE)]);
        let d = |w| defs(&dir, &files, Kind::Make, w);
        assert_eq!(d("build"), [4]);
        assert_eq!(d("test-all"), [4], "one of several targets");
        // The `deps::` rule, not the prerequisite on line 4.
        assert_eq!(d("deps"), [6]);
        assert_eq!(d("CC"), [2]);
        assert_eq!(d("CFLAGS"), [3]);
        assert_eq!(d("gcc"), Vec::<usize>::new());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    const TF: &str = r#"variable "region" {
  default = "eu"
}
locals {
  name = "logs-${var.region}"
  tags = {
    name = "x"
  }
}
resource "aws_s3_bucket" "logs" {
  bucket = local.name
}
data "aws_ami" "logs" {
  most_recent = true
}
module "vpc" {
  source = "./vpc"
}
output "bucket" {
  value = aws_s3_bucket.logs.id
}
"#;

    #[test]
    fn terraform_def_patterns_resolve_the_address() {
        let (dir, files) = scratch("tf", &[("main.tf", TF)]);
        let d = |w| defs(&dir, &files, Kind::Terraform, w);
        assert_eq!(d("var.region"), [1]);
        assert_eq!(d("aws_s3_bucket.logs.id"), [10]);
        assert_eq!(d("data.aws_ami.logs"), [13]);
        assert_eq!(d("module.vpc.cidr"), [16]);
        // A bare name is any block with that label, as from a `.tfvars` file.
        assert_eq!(d("logs"), [10, 13]);
        assert_eq!(d("region"), [1]);
        // `local.name` matches every `name =`; only the one directly in `locals` survives.
        assert_eq!(d("local.name"), [5, 7]);
        assert_eq!(def_block(Kind::Terraform, "local.name"), Some("locals"));
        assert_eq!(def_block(Kind::Terraform, "var.name"), None);
        assert!(directly_inside(TF, 5, "locals"));
        assert!(!directly_inside(TF, 7, "locals"), "nested in `tags`");
        assert!(!directly_inside(TF, 11, "locals"));
        assert!(!directly_inside(TF, 1, "locals"));
        assert!(!directly_inside(TF, 0, "locals"));
        assert!(def_patterns(Kind::Terraform, "each.key").is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn docker_and_yaml_def_patterns() {
        let docker = "FROM rust:1.80 AS build\nRUN cargo build\nFROM --platform=$BUILDPLATFORM debian AS Runtime\nCOPY --from=build /x /y\n";
        let yaml = "x-common: &common\n  restart: always\nservices:\n  web:\n    <<: *common\n    depends_on: [db-main]\n  db-main:  # the database\n    image: postgres\n.base:\n  script: make\n";
        let (dir, files) = scratch("dy", &[("Dockerfile", docker), ("compose.yml", yaml)]);
        let (docker, yaml) = (files[..1].to_vec(), files[1..].to_vec());
        assert_eq!(defs(&dir, &docker, Kind::Docker, "build"), [1]);
        assert_eq!(defs(&dir, &docker, Kind::Docker, "runtime"), [3]);
        assert_eq!(defs(&dir, &yaml, Kind::Yaml, "common"), [1]);
        assert_eq!(defs(&dir, &yaml, Kind::Yaml, "db-main"), [7]);
        assert_eq!(defs(&dir, &yaml, Kind::Yaml, "web"), [4]);
        assert_eq!(defs(&dir, &yaml, Kind::Yaml, "base"), [9]);
        // A key with a value on its line is data, not a definition.
        assert_eq!(defs(&dir, &yaml, Kind::Yaml, "image"), Vec::<usize>::new());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn kinds_come_from_the_file_name() {
        for (name, kind) in [
            ("a.py", Some(Kind::Python)),
            ("a.go", Some(Kind::Go)),
            ("lib.rs", Some(Kind::Rust)),
            ("app.tsx", Some(Kind::TsJs)),
            ("util.cjs", Some(Kind::TsJs)),
            ("Invoice.java", Some(Kind::Jvm)),
            ("invoice.rb", Some(Kind::Ruby)),
            ("Rakefile", Some(Kind::Ruby)),
            ("Gemfile", Some(Kind::Ruby)),
            ("config.ru", Some(Kind::Ruby)),
            ("merl.gemspec", Some(Kind::Ruby)),
            ("tasks.rake", Some(Kind::Ruby)),
            ("invoice.rbi", Some(Kind::Ruby)),
            ("invoice.c", Some(Kind::C)),
            ("invoice.h", Some(Kind::C)),
            ("ledger.cc", Some(Kind::C)),
            ("ledger.cpp", Some(Kind::C)),
            ("ledger.cxx", Some(Kind::C)),
            ("ledger.hpp", Some(Kind::C)),
            ("ledger.hh", Some(Kind::C)),
            ("ledger.hxx", Some(Kind::C)),
            ("Invoice.cs", Some(Kind::CSharp)),
            ("build.csx", Some(Kind::CSharp)),
            ("Session.swift", Some(Kind::Swift)),
            ("Invoice.php", Some(Kind::Php)),
            ("show.phtml", Some(Kind::Php)),
            ("init.lua", Some(Kind::Lua)),
            ("ledger.ex", Some(Kind::Elixir)),
            ("mix.exs", Some(Kind::Elixir)),
            ("ledger.zig", Some(Kind::Zig)),
            // Zig's data format: painted as Zig, but it declares nothing.
            ("build.zig.zon", None),
            ("app.kt", Some(Kind::Jvm)),
            ("build.gradle.kts", Some(Kind::Jvm)),
            ("run.sh", Some(Kind::Shell)),
            ("run.bash", Some(Kind::Shell)),
            ("run.zsh", Some(Kind::Shell)),
            ("run.ksh", Some(Kind::Shell)),
            (".bashrc", Some(Kind::Shell)),
            (".bash_profile", Some(Kind::Shell)),
            (".bash_aliases", Some(Kind::Shell)),
            (".zshrc", Some(Kind::Shell)),
            (".zshenv", Some(Kind::Shell)),
            (".zprofile", Some(Kind::Shell)),
            (".profile", Some(Kind::Shell)),
            // A shebang-only script: `kind_of` goes by the name, so it has no kind.
            ("install", None),
            ("schema.sql", Some(Kind::Sql)),
            ("dump.psql", Some(Kind::Sql)),
            ("init.pgsql", Some(Kind::Sql)),
            ("seed.mysql", Some(Kind::Sql)),
            ("001.ddl", Some(Kind::Sql)),
            ("002.dml", Some(Kind::Sql)),
            ("Makefile", Some(Kind::Make)),
            ("GNUmakefile", Some(Kind::Make)),
            ("rules.mk", Some(Kind::Make)),
            ("main.tf", Some(Kind::Terraform)),
            ("prod.tfvars", Some(Kind::Terraform)),
            ("Dockerfile", Some(Kind::Docker)),
            ("Dockerfile.prod", Some(Kind::Docker)),
            ("api.Dockerfile", Some(Kind::Docker)),
            ("Containerfile", Some(Kind::Docker)),
            ("Dockerfile.dockerignore", None),
            ("compose.yaml", Some(Kind::Yaml)),
            (".gitlab-ci.yml", Some(Kind::Yaml)),
            ("README", None),
            ("notes.txt", None),
        ] {
            assert_eq!(kind_of(Path::new(name)), kind, "{name}");
        }
    }

    #[test]
    fn qualifier_is_the_chain_in_front_of_the_word() {
        let q = |l: &str, i| qualifier(l, i);
        assert_eq!(q("    return json.load(fp)", 16), ["json"]);
        assert_eq!(q("x = os.path.join(a)", 12), ["os", "path"]);
        assert_eq!(q("let s = fs::read_to_string(p)", 12), ["fs"]);
        assert!(q("load(fp)", 0).is_empty());
        assert!(q("x = load(fp)", 4).is_empty());
        assert_eq!(q("this.#root.insert(p)", 11), ["this", "#root"]);
        // A chain that hangs off a call, an index or `?.` has no name to start from.
        assert!(q("x = make_uow().users.delete_user()", 21).is_empty());
        assert!(q("a[0].users.find()", 11).is_empty());
        assert!(q("a?.users.find()", 9).is_empty());
        assert!(q("  .users.find()", 9).is_empty());
        // Python's `super()` is one name, as TypeScript's `super` is; `my_super()` is a call.
        assert_eq!(q("        super().store(item)", 16), ["super"]);
        assert_eq!(q("    print(super().label)", 18), ["super"]);
        assert!(q("    my_super().store(item)", 15).is_empty());
        assert!(q("    a.super().store(item)", 14).is_empty());
        // A spread and a range are no member access.
        assert_eq!(q("f(...this.repo.find())", 15), ["this", "repo"]);
        assert_eq!(q("for i in 0..v.len() {", 14), ["v"]);
    }

    #[test]
    fn a_loop_hands_out_elements_only_where_the_type_says_so() {
        let cases = [
            (Kind::Python, "list[Repo]", Some("Repo")),
            (
                Kind::Python,
                "typing.Sequence[models.Repo]",
                Some("models.Repo"),
            ),
            (Kind::Python, "\"tuple[Repo, ...]\"", Some("Repo")),
            (Kind::Python, "set[Repo | None]", Some("Repo | None")),
            // Keys, a fixed tuple, a type of the project's own, no arguments at all.
            (Kind::Python, "dict[str, Repo]", None),
            (Kind::Python, "tuple[Repo, Audit]", None),
            (Kind::Python, "Page[Repo]", None),
            (Kind::Python, "list", None),
            (Kind::TsJs, "Repo[]", Some("Repo")),
            (Kind::TsJs, "readonly Repo[]", Some("Repo")),
            (Kind::TsJs, "Array<Box<Repo>>", Some("Box<Repo>")),
            (Kind::TsJs, "Set<Repo>;", Some("Repo")),
            (Kind::TsJs, "Map<string, Repo>", None),
            (Kind::TsJs, "Promise<Repo[]>", None),
            (Kind::TsJs, "Repo", None),
            (Kind::Go, "[]*Repo", Some("*Repo")),
            (Kind::Go, "[4]Repo", Some("Repo")),
            (Kind::Go, "map[Key]*models.Repo", Some("*models.Repo")),
            (Kind::Go, "map[[2]int][]Repo", Some("[]Repo")),
            (Kind::Go, "chan *Repo", None),
            (Kind::Go, "*[]Repo", None),
            (Kind::Go, "Repos", None),
        ];
        for (kind, written, want) in cases {
            let got = element_type(kind, written);
            assert_eq!(got.as_deref(), want, "{written}");
        }
        // The loops that hand out something else are unknown.
        let element = |name: &str| Value::Element(name.into());
        let py = "async def f(repos):\n    for r in repos:\n        r\n    async for r in repos:\n        r\n    for i, r in pairs:\n        r\n    for r in load():\n        r\n";
        let at = |kind, text, line, name| bound_at(kind, text, line, name);
        assert_eq!(
            at(Kind::Python, py, 3, "r"),
            [
                (2, element("repos")),
                (4, Value::Unknown),
                (6, Value::Unknown),
                (8, Value::Unknown)
            ]
        );
        let ts = "for (const r of repos) {\n  r;\n}\nfor (const k in repos) {\n  k;\n}\nfor (const [k, r] of pairs) {\n  r;\n}\nfor (const r of load()) {\n  r;\n}\n";
        assert_eq!(at(Kind::TsJs, ts, 2, "r"), [(1, element("repos"))]);
        assert_eq!(at(Kind::TsJs, ts, 5, "k"), [(4, Value::Unknown)]);
        assert_eq!(at(Kind::TsJs, ts, 8, "r"), [(7, Value::Unknown)]);
        assert_eq!(at(Kind::TsJs, ts, 11, "r"), [(10, Value::Unknown)]);
        let go = "func f() {\n\tfor _, r := range repos {\n\t\tr.Go()\n\t}\n\tfor i, r := range repos {\n\t\ti.Go()\n\t}\n\tfor r := range repos {\n\t\tr.Go()\n\t}\n\tfor _, r := range s.repos {\n\t\tr.Go()\n\t}\n}\n";
        assert_eq!(at(Kind::Go, go, 3, "r"), [(2, element("repos"))]);
        assert_eq!(at(Kind::Go, go, 6, "i"), [(5, Value::Unknown)]);
        assert_eq!(at(Kind::Go, go, 9, "r"), [(8, Value::Unknown)]);
        assert_eq!(at(Kind::Go, go, 12, "r"), [(11, Value::Unknown)]);
    }

    /// The bindings of `name` on `line` of `text`, as (line, value) pairs.
    fn bound_at(kind: Kind, text: &str, line: usize, name: &str) -> Vec<(usize, Value)> {
        bindings(kind, text, line, name)
            .into_iter()
            .map(|b| (b.line, b.value))
            .collect()
    }

    fn ty(t: &str) -> Value {
        Value::Type(t.into())
    }

    #[test]
    fn python_bindings_cover_parameters_locals_and_the_class_of_self() {
        let text = r#"from repos import UserRepository

repo = UserRepository()


class Service:
    audit: AuditLog

    def __init__(
        self,
        repo: UserRepository,
        cache: "Cache | None" = None,
    ) -> None:
        self.repo = repo
        local: Optional[Repo] = make()

    @staticmethod
    def build(first, second: int):
        return first.x(second)

    def run(self, items):
        for item in items:
            item.go()
        self.repo.delete_user(1)
        [r.go() for r in items]
        done = lambda repo: repo.close()
        a, b = items
        with open() as fh:
            fh.read()
        log(a, level=1)

import os.path, store.sessions as sessions
"#;
        let at = |line, name| bound_at(Kind::Python, text, line, name);
        // A parameter over a multi-line signature hides the module's `repo` above.
        assert_eq!(at(14, "repo"), [(9, ty("UserRepository"))]);
        assert_eq!(at(1, "repo"), [(3, Value::Call("UserRepository".into()))]);
        assert_eq!(at(14, "cache"), [(9, ty("\"Cache | None\""))]);
        assert_eq!(at(14, "local"), [(15, ty("Optional[Repo]"))]);
        assert_eq!(at(24, "self"), [(21, Value::Class(6))]);
        // A static method's first parameter is no `self`, and an unannotated one is unknown.
        assert_eq!(at(19, "first"), [(18, Value::Unknown)]);
        assert_eq!(at(19, "second"), [(18, ty("int"))]);
        assert_eq!(at(23, "item"), [(22, Value::Element("items".into()))]);
        // A comprehension or a lambda binds on its own line only.
        assert_eq!(at(25, "r"), [(25, Value::Unknown)]);
        assert_eq!(at(24, "r"), []);
        // Unknown where it is written, it hides the module's `repo` and proves nothing.
        assert_eq!(at(26, "repo"), [(26, Value::Unknown)]);
        // A tuple and `as` are unknown; a keyword argument is no binding.
        assert_eq!(at(29, "a"), [(27, Value::Unknown)]);
        assert_eq!(at(29, "fh"), [(28, Value::Unknown)]);
        assert_eq!(at(30, "level"), []);
        // An import binds the names it imports, not the modules on their path.
        assert_eq!(at(14, "UserRepository"), [(1, Value::Unknown)]);
        assert_eq!(at(14, "repos"), []);
        assert_eq!(at(14, "os"), [(32, Value::Unknown)]);
        assert_eq!(at(14, "path"), []);
        assert_eq!(at(14, "sessions"), [(32, Value::Unknown)]);
    }

    /// Comments and strings hold brackets, quotes and commas that are no code: fastapi writes a
    /// `# type: ignore` on a signature line and `Doc("""…""")` texts through its signatures.
    #[test]
    fn bindings_read_past_comments_strings_and_long_signatures() {
        let filler: String = (0..70).map(|i| format!("    p{i}: int,\n")).collect();
        let py = format!(
            "class Basic(Base):
    async def __call__(  # type: ignore
        self, request: Request  # the request (
    ) -> None:
        repo: Repo  # a comment, with a bracket (
        repo.find(self)


def delete(
    path: Annotated[str, Doc(\"\"\"
        The path (see the docs, it's here.
    \"\"\")] = \"a,(\",
{filler}    repo: Repo = None,
) -> None:
    repo.find()
"
        );
        let at = |line, name| bound_at(Kind::Python, &py, line, name);
        assert_eq!(at(6, "self"), [(2, Value::Class(1))]);
        assert_eq!(at(6, "repo"), [(5, ty("Repo"))]);
        assert_eq!(at(85, "repo"), [(9, ty("Repo"))]);
        assert_eq!(
            at(85, "path"),
            [(
                9,
                ty(
                    "Annotated[str, Doc(\"\"\"\n        The path (see the docs, it's here.\n    \"\"\")]"
                )
            )]
        );
        let ts =
            "export function run(sep = \"a,(\", repo: Repo) {\n  repo.find(sep); // a call (\n}\n";
        assert_eq!(bound_at(Kind::TsJs, ts, 2, "repo"), [(1, ty("Repo"))]);
        assert_eq!(bound_at(Kind::TsJs, ts, 2, "sep"), [(1, Value::Unknown)]);
        let go = "func (s *Service) Do(\n\tctx context.Context, // the context (\n\trepo *Repo,\n) {\n\trepo.Find(ctx)\n}\n";
        assert_eq!(bound_at(Kind::Go, go, 5, "repo"), [(1, ty("*Repo"))]);
        assert_eq!(bound_at(Kind::Go, go, 5, "s"), [(1, ty("*Service"))]);
    }

    #[test]
    fn python_bindings_see_the_enclosing_functions_but_not_the_nested_ones() {
        let text = "def cleanup(user_id: int) -> None:\n    repo = AuditLog()\n\n    async def purge() -> None:\n        repo = UserRepository()\n        await repo.delete_user(user_id)\n\n    repo.delete_user(user_id)\n    repo = make_repo()  # later\n";
        let at = |line, name| bound_at(Kind::Python, text, line, name);
        let call = |c: &str| Value::Call(c.into());
        // The innermost function binding the name hides the one around it.
        assert_eq!(at(6, "repo"), [(5, call("UserRepository"))]);
        // One that does not bind it reads the enclosing function's.
        assert_eq!(at(6, "user_id"), [(1, ty("int"))]);
        // The whole function counts, the lines after the cursor too.
        assert_eq!(
            at(8, "repo"),
            [(2, call("AuditLog")), (9, call("make_repo"))]
        );
        assert_eq!(at(6, "user_id"), [(1, ty("int"))]);
    }

    #[test]
    fn ts_bindings_follow_the_blocks_around_the_cursor() {
        let text = r#"import { AuditLog, UserRepository } from "./repos";

const shared = new AuditLog();

export class UserService {
  private audit = new AuditLog();

  constructor(
    private repo: UserRepository,
  ) {
    this.repo.findUser(1);
  }

  async remove(id: number, cache = new Cache()): Promise<void> {
    const user: User = this.repo.findUser(id);
    const made = await createRepo();
    for (const item of items) {
      item.go();
    }
    const { a, b } = user;
    items.forEach((x: Item) => x.go());
    const handler = function () {
      this.go();
    };
    const later = () => {
      this.audit.deleteUser(id);
    };
    shared.go(made, a);
  }
}

export function cleanup(id: number): void {
  const repo = new AuditLog();
  const purge = async () => {
    const repo = new UserRepository();
    await repo.deleteUser(id);
  };
  repo.deleteUser(id);
}
"#;
        let at = |line, name| bound_at(Kind::TsJs, text, line, name);
        let new = |t: &str| Value::New(t.into());
        // `this` passes a constructor over several lines, an arrow and a method.
        assert_eq!(at(11, "this"), [(5, Value::Class(5))]);
        assert_eq!(at(26, "this"), [(5, Value::Class(5))]);
        assert_eq!(at(23, "this"), [(22, Value::Unknown)]);
        assert_eq!(at(11, "repo"), [(8, ty("UserRepository"))]);
        assert_eq!(at(15, "id"), [(14, ty("number"))]);
        assert_eq!(at(15, "cache"), [(14, new("Cache"))]);
        assert_eq!(at(16, "user"), [(15, ty("User"))]);
        assert_eq!(at(28, "made"), [(16, Value::Call("createRepo".into()))]);
        assert_eq!(at(28, "shared"), [(3, new("AuditLog"))]);
        assert_eq!(at(18, "item"), [(17, Value::Element("items".into()))]);
        assert_eq!(at(28, "a"), [(20, Value::Unknown)]);
        assert_eq!(at(21, "x"), [(21, ty("Item"))]);
        // The innermost block around the cursor that declares the name; the inner `repo` is
        // gone below its arrow.
        assert_eq!(at(36, "repo"), [(35, new("UserRepository"))]);
        assert_eq!(at(38, "repo"), [(33, new("AuditLog"))]);
    }

    #[test]
    fn what_a_header_binds_for_another_body_hides_nothing() {
        let new = |t: &str| Value::New(t.into());
        // A loop and a one-name arrow inside a callback on the line that opens a literal or
        // another callback: the cursor below is in neither, and the outer `repo` still counts.
        let ts = "const repo = new AuditLog();\nrun(() => { for (const repo of repos) { use(repo); } }, {\n  done: repo.x(),\n});\nrepos.map(repo => repo.id).forEach((id) => {\n  repo.y(id);\n});\nrepos.forEach(repo => {\n  repo.z();\n});\n";
        let at = |line| bound_at(Kind::TsJs, ts, line, "repo");
        assert_eq!(
            at(3),
            [(2, Value::Element("repos".into())), (1, new("AuditLog"))]
        );
        assert_eq!(at(6), [(5, Value::Unknown), (1, new("AuditLog"))]);
        // The arrow whose body the line opens is the cursor's own.
        assert_eq!(at(9), [(8, Value::Unknown)]);
        // A declaration inside a raw string over several lines is none.
        let go = "func f() {\n\trepo := NewAudit()\n\tif ok {\n\t\tconst doc = `\n\t\trepo := NewRepo()\n\t\t`\n\t\trepo.Go(doc)\n\t}\n}\n";
        assert_eq!(
            bound_at(Kind::Go, go, 7, "repo"),
            [(2, Value::Call("NewAudit".into()))]
        );
    }

    #[test]
    fn go_bindings_cover_receivers_parameters_and_short_declarations() {
        let text = "package main

var shared = &AuditLog{}

func (s *UserService) Remove(id int, a, b *Repo) (n int, err error) {
\tuser := s.repo.FindUser(id)
\tmade, err := NewRepo()
\tvar typed store.Session
\tvar built = Repo{ID: 1}
\tfresh := new(Repo)
\tfor _, item := range items {
\t\titem.Go()
\t}
\tif r, ok := x.(*Repo); ok {
\t\tr.Go()
\t}
\tpurge := func(repo *UserRepository) {
\t\trepo.DeleteUser(id)
\t}
\t_, other := pair()
\tshared.Go(user, made, typed, built, fresh, other, purge)
}
";
        let at = |line, name| bound_at(Kind::Go, text, line, name);
        let new = |t: &str| Value::New(t.into());
        assert_eq!(at(6, "s"), [(5, ty("*UserService"))]);
        assert_eq!(at(21, "a"), [(5, ty("*Repo"))]);
        assert_eq!(at(21, "b"), [(5, ty("*Repo"))]);
        assert_eq!(at(21, "n"), [(5, ty("int"))]);
        assert_eq!(at(21, "made"), [(7, Value::Call("NewRepo".into()))]);
        // The second name of a `:=` is not the call's first result.
        assert_eq!(at(21, "err"), [(7, Value::Unknown), (5, ty("error"))]);
        assert_eq!(at(21, "typed"), [(8, ty("store.Session"))]);
        assert_eq!(at(21, "built"), [(9, new("Repo"))]);
        assert_eq!(at(21, "fresh"), [(10, new("Repo"))]);
        assert_eq!(at(21, "shared"), [(3, new("AuditLog"))]);
        assert_eq!(at(12, "item"), [(11, Value::Element("items".into()))]);
        assert_eq!(at(15, "r"), [(14, Value::Unknown)]);
        assert_eq!(at(18, "repo"), [(17, ty("*UserRepository"))]);
        assert_eq!(at(21, "other"), [(20, Value::Unknown)]);
    }

    fn fields(kind: Kind, text: &str, decl: usize, name: &str) -> Vec<(usize, Value)> {
        field_bindings(kind, text, decl, name)
            .into_iter()
            .map(|b| (b.line, b.value))
            .collect()
    }

    #[test]
    fn python_fields_come_from_the_class_body_and_self_in_its_methods() {
        let text = "class Service(Base, metaclass=Meta):
    audit: AuditLog
    cache = Cache()

    def __init__(self, repo: UserRepository) -> None:
        self.repo = repo
        self.store: Store | None = None
        self.a, self.b = 1, 2
        log(self.repo, self.audit)

    class Inner:
        def __init__(self):
            self.repo = Other()

    def reset(self):
        self.repo = make_repo()
";
        let at = |name| fields(Kind::Python, text, 1, name);
        // An argument of a call is no binding.
        assert_eq!(
            at("repo"),
            [
                (6, Value::Name("repo".into())),
                (16, Value::Call("make_repo".into()))
            ]
        );
        assert_eq!(at("audit"), [(2, ty("AuditLog"))]);
        assert_eq!(at("cache"), [(3, Value::Call("Cache".into()))]);
        assert_eq!(at("store"), [(7, ty("Store | None"))]);
        assert_eq!(at("a"), [(8, Value::Unknown)]);
        assert_eq!(at("b"), [(8, Value::Unknown)]);
        assert_eq!(bases(Kind::Python, text, 1), ["Base"]);
    }

    #[test]
    fn a_python_property_is_a_field_of_its_declared_return_type() {
        let text = "class Registry(Base):
    @property
    def users(self) -> UserRepository:
        return UserRepository()

    @users.setter
    def users(self, value: Other) -> None:
        self._users = value

    @cached_property  # built once
    def audit(
        self,
    ) -> \"AuditLog\":
        return AuditLog()

    @functools.cached_property
    @override
    def jobs(self) -> Jobs: ...

    @property
    def mixins(self):
        return HttpRepo()

    def plain(self) -> Plain: ...

    class Inner:
        @property
        def inner(self) -> Inner: ...

    def make(self):
        @property
        def nested(self) -> Nested: ...
";
        let at = |name| fields(Kind::Python, text, 1, name);
        // The setter declares no type of its own.
        assert_eq!(at("users"), [(3, ty("UserRepository"))]);
        assert_eq!(at("audit"), [(11, ty("\"AuditLog\""))]);
        assert_eq!(at("jobs"), [(18, ty("Jobs"))]);
        assert_eq!(at("mixins"), [(21, Value::Unknown)]);
        assert_eq!(at("plain"), []);
        assert_eq!(at("inner"), []);
        // A property nested in a method is no field of the class.
        assert_eq!(at("nested"), []);
    }

    #[test]
    fn a_ts_getter_is_a_field_of_its_declared_type() {
        let text = "export abstract class Registry {
  get users(): UserRepository {
    return this.cache.users;
  }
  set users(value: Other) {
    this.cache.users = value;
  }
  public static get audit(): AuditLog | null { return null; }
  protected abstract get jobs(): Jobs;
  get mixins() {
    return new HttpRepo();
  }
  make() {
    return {
      get nested(): Other { return x; },
    };
  }
}
";
        let at = |name| fields(Kind::TsJs, text, 1, name);
        assert_eq!(at("users"), [(2, ty("UserRepository"))]);
        assert_eq!(at("audit"), [(8, ty("AuditLog | null"))]);
        assert_eq!(at("jobs"), [(9, ty("Jobs"))]);
        assert_eq!(at("mixins"), [(10, Value::Unknown)]);
        // An object literal's getter inside a method is no field of the class.
        assert_eq!(at("nested"), []);
    }

    #[test]
    fn ts_fields_come_from_members_constructor_parameters_and_this() {
        let text = "export class UserService extends Base<Repo> implements Service {
  private readonly audit = new AuditLog();
  #root: Node;
  store?: Store | null;

  constructor(
    private repo: UserRepository,
    notifier: Notifier,
  ) {
    super();
    this.notifier = notifier;
    this.#root = new Node();
  }

  audit2(): void {}
}

export interface Store extends Reader, Writer<T> {
  readonly db: Database;
  send(text: string): void;
}
";
        let at = |decl, name| fields(Kind::TsJs, text, decl, name);
        assert_eq!(at(1, "audit"), [(2, Value::New("AuditLog".into()))]);
        assert_eq!(
            at(1, "#root"),
            [(3, ty("Node")), (12, Value::New("Node".into()))]
        );
        assert_eq!(at(1, "store"), [(4, ty("Store | null"))]);
        assert_eq!(at(1, "repo"), [(7, ty("UserRepository"))]);
        // A parameter without a modifier is no field; the assignment is.
        assert_eq!(at(1, "notifier"), [(11, Value::Name("notifier".into()))]);
        assert_eq!(at(18, "db"), [(19, ty("Database"))]);
        assert_eq!(at(18, "send"), []);
        assert_eq!(bases(Kind::TsJs, text, 1), ["Base<Repo>"]);
        assert_eq!(bases(Kind::TsJs, text, 18), ["Reader", "Writer<T>"]);
    }

    #[test]
    fn go_fields_are_struct_fields_and_embedded_types() {
        let text = "type Context struct {
\t*Engine
\tsync.Mutex
\trepo, audit *UserRepository
\thandlers HandlersChain `json:\"-\"` // the chain

\tinner struct {
\t\trepo Other
\t}
}

type Store interface {
\tReader
\tGet(key string) string
}
";
        let at = |name| fields(Kind::Go, text, 1, name);
        assert_eq!(at("repo"), [(4, ty("*UserRepository"))]);
        assert_eq!(at("audit"), [(4, ty("*UserRepository"))]);
        assert_eq!(at("handlers"), [(5, ty("HandlersChain"))]);
        assert_eq!(at("Engine"), [(2, ty("*Engine"))]);
        assert_eq!(at("Mutex"), [(3, ty("sync.Mutex"))]);
        assert_eq!(fields(Kind::Go, text, 12, "Reader"), []);
        assert_eq!(bases(Kind::Go, text, 1), ["Engine", "sync.Mutex"]);
        assert_eq!(bases(Kind::Go, text, 12), ["Reader"]);
    }

    /// #68 step 6: the forms the implementations of a member are found through.
    const PY_IMPLS: &str = "class Notifier(Protocol):
    def send(self, text: str) -> None: ...


class EmailNotifier(Notifier):
    def send(
        self,
        text: str,
    ) -> None:
        def inner() -> None:
            pass


class Quiet(Notifier):
    pass


def send(text: str) -> None:
    pass
";

    const TS_IMPLS: &str = "export interface Notifier {
  send(text: string): void;
}

export abstract class Base<T> extends Other implements Notifier, Logger<T> {
  send(text: string): void {
    const inner = () => {};
  }
}

export class Quiet extends Base {}

export class Wrapped
  extends Base
  implements Notifier
{
  send(text: string): void {}
}
";

    const GO_IMPLS: &str = "type Notifier interface {
\tSend(text string)
}

func (e *EmailNotifier) Send(text string) {
}

func (b Batch) Send(text string, retries int) {
}
";

    #[test]
    fn owner_decl_names_the_type_a_member_is_written_in() {
        let py = |line| owner_decl(Kind::Python, PY_IMPLS, line);
        assert_eq!(py(2), Some(1), "a protocol's signature");
        assert_eq!(py(6), Some(5), "a method");
        assert_eq!(py(10), None, "a function nested in a method is no member");
        assert_eq!(py(18), None, "a function at the top of the module");
        let ts = |line| owner_decl(Kind::TsJs, TS_IMPLS, line);
        assert_eq!(ts(2), Some(1), "an interface signature");
        assert_eq!(ts(6), Some(5));
        assert_eq!(ts(7), None);
        assert_eq!(ts(17), Some(13), "past the brace of a wrapped header");
        let go = |line| owner_decl(Kind::Go, GO_IMPLS, line);
        assert_eq!(go(2), Some(1), "an interface's method line");
        assert_eq!(go(5), None, "a method stands beside its type");
    }

    #[test]
    fn interfaces_read_what_a_typescript_class_implements() {
        assert_eq!(
            interfaces(Kind::TsJs, TS_IMPLS, 5),
            ["Notifier", "Logger<T>"]
        );
        assert_eq!(bases(Kind::TsJs, TS_IMPLS, 5), ["Other"], "extends alone");
        assert_eq!(interfaces(Kind::TsJs, TS_IMPLS, 11), [] as [String; 0]);
        // A wrapped header carries each clause on a line of its own.
        assert_eq!(bases(Kind::TsJs, TS_IMPLS, 14), ["Base"]);
        assert_eq!(interfaces(Kind::TsJs, TS_IMPLS, 15), ["Notifier"]);
        assert_eq!(interfaces(Kind::Python, PY_IMPLS, 5), [] as [String; 0]);
        assert_eq!(bases(Kind::Python, PY_IMPLS, 5), ["Notifier"]);
    }

    #[test]
    fn subtype_patterns_match_a_header_that_names_a_base() {
        let names = ["Notifier".to_owned(), "Base".to_owned()];
        let re = |kind| Regex::new(&subtype_patterns(kind, &names).unwrap()).unwrap();
        let py = re(Kind::Python);
        assert!(py.is_match("class EmailNotifier(Notifier):"));
        assert!(py.is_match("class X(Generic[T], Base):"));
        assert!(!py.is_match("class Notifier(Protocol):"), "its own header");
        assert!(!py.is_match("    notifier: Notifier"));
        let ts = re(Kind::TsJs);
        assert!(ts.is_match("export class EmailNotifier implements Notifier {"));
        assert!(ts.is_match("export interface Admin extends Notifier {"));
        assert!(ts.is_match("class X extends Base<T> implements Other {"));
        assert!(ts.is_match("    extends Base"), "a wrapped header's clause");
        assert!(!ts.is_match("export interface Notifier {"));
        assert!(!ts.is_match("export class NotifierFactory {"));
        assert_eq!(
            subtype_patterns(Kind::Go, &names),
            None,
            "implicit interfaces"
        );
    }

    #[test]
    fn member_decl_finds_only_what_the_type_declares_itself() {
        assert_eq!(member_decl(Kind::Python, PY_IMPLS, 1, "send"), Some(2));
        assert_eq!(member_decl(Kind::Python, PY_IMPLS, 5, "send"), Some(6));
        assert_eq!(member_decl(Kind::Python, PY_IMPLS, 5, "inner"), None);
        assert_eq!(
            member_decl(Kind::Python, PY_IMPLS, 14, "send"),
            None,
            "inherits it"
        );
        assert_eq!(member_decl(Kind::TsJs, TS_IMPLS, 1, "send"), Some(2));
        assert_eq!(member_decl(Kind::TsJs, TS_IMPLS, 5, "send"), Some(6));
        assert_eq!(member_decl(Kind::TsJs, TS_IMPLS, 11, "send"), None);
        assert_eq!(member_decl(Kind::TsJs, TS_IMPLS, 13, "send"), Some(17));
    }

    #[test]
    fn type_decl_at_walks_up_a_header_wrapped_over_several_lines() {
        let at = |line| type_decl_at(Kind::TsJs, TS_IMPLS, line);
        assert_eq!(at(13), Some(13), "the header itself");
        assert_eq!(at(14), Some(13), "`extends Base` on its own line");
        assert_eq!(at(15), Some(13), "`implements Notifier` on its own");
        assert_eq!(at(16), Some(13), "the brace below them");
        assert_eq!(at(8), None, "a line that ends a block declares nothing");
        assert_eq!(
            qualified(Kind::TsJs, TS_IMPLS, 17, "send"),
            Some("Wrapped.send".into()),
            "the member of a wrapped class is named by it"
        );
    }

    #[test]
    fn params_count_what_a_declaration_takes() {
        assert_eq!(params(Kind::Python, PY_IMPLS, 2), Some(2));
        assert_eq!(
            params(Kind::Python, PY_IMPLS, 6),
            Some(2),
            "over three lines"
        );
        assert_eq!(params(Kind::Python, PY_IMPLS, 18), Some(1));
        assert_eq!(params(Kind::TsJs, TS_IMPLS, 2), Some(1));
        assert_eq!(
            params(Kind::Go, GO_IMPLS, 2),
            Some(1),
            "an interface's line"
        );
        assert_eq!(params(Kind::Go, GO_IMPLS, 5), Some(1), "past the receiver");
        assert_eq!(params(Kind::Go, GO_IMPLS, 8), Some(2));
        assert_eq!(params(Kind::Python, PY_IMPLS, 1), Some(1), "a class header");
    }

    #[test]
    fn type_name_reads_the_name_a_declaration_gives() {
        let name = |kind, line| type_name(kind, line);
        assert_eq!(name(Kind::Python, "class Foo(Base):"), Some("Foo".into()));
        assert_eq!(name(Kind::Python, "    def send(self):"), None);
        assert_eq!(
            name(Kind::TsJs, "export abstract class Foo<T> extends Bar {"),
            Some("Foo".into())
        );
        assert_eq!(
            name(Kind::TsJs, "export interface Foo {"),
            Some("Foo".into())
        );
        assert_eq!(name(Kind::TsJs, "  send(text: string): void;"), None);
        assert_eq!(name(Kind::Go, "type Foo struct{}"), Some("Foo".into()));
        assert_eq!(name(Kind::Go, "func (f Foo) Send() {"), None);
    }

    #[test]
    fn returns_read_the_declared_type_or_what_typescript_constructs() {
        let py = "def make_repo() -> UserRepository:\n    return UserRepository()\n\nasync def connect(\n    url: str,\n) -> \"Session\":\n    ...\n\ndef untyped():\n    return Repo()\n\ndef stub() -> Repo: ...\n\ndef documented() -> Annotated[Repo, \"doc: x\"]: ...\n";
        assert_eq!(returns(Kind::Python, py, 1), Some(ty("UserRepository")));
        assert_eq!(returns(Kind::Python, py, 4), Some(ty("\"Session\"")));
        // No annotation: what every `return` constructs (#100).
        assert_eq!(
            returns(Kind::Python, py, 9),
            Some(Value::New("Repo".into()))
        );
        assert_eq!(returns(Kind::Python, py, 12), Some(ty("Repo")));
        // A `: ` inside a string is not the end of the annotation.
        assert_eq!(
            returns(Kind::Python, py, 14),
            Some(ty("Annotated[Repo, \"doc: x\"]"))
        );
        let ts = "export function makeRepo(): UserRepository {
  return new UserRepository();
}
export async function load(id: number): Promise<Repo> {
  return fetchRepo(id);
}
export function makeAudit() {
  if (x) {
    return new AuditLog();
  }
  return new AuditLog();
}
export const createRepo = (): Repo => new Repo();
export const inferred = () => new Repo();
export function mixed() {
  if (x) {
    return new AuditLog();
  }
  return new UserRepository();
}
export default function connect(): Session {
  return openSession();
}
";
        let new = |t: &str| Some(Value::New(t.into()));
        assert_eq!(returns(Kind::TsJs, ts, 1), Some(ty("UserRepository")));
        assert_eq!(returns(Kind::TsJs, ts, 4), Some(ty("Promise<Repo>")));
        assert_eq!(returns(Kind::TsJs, ts, 7), new("AuditLog"));
        assert_eq!(returns(Kind::TsJs, ts, 13), Some(ty("Repo")));
        assert_eq!(returns(Kind::TsJs, ts, 14), new("Repo"));
        assert_eq!(returns(Kind::TsJs, ts, 15), None);
        assert_eq!(returns(Kind::TsJs, ts, 21), Some(ty("Session")));
        let go = "func NewRepo() *UserRepository {
\treturn &UserRepository{}
}
func NewAudit(url string) (AuditLog, error) {
\treturn AuditLog{}, nil
}
func Open() (s *Session, err error) {
\treturn
}
func Close() {
}
";
        assert_eq!(returns(Kind::Go, go, 1), Some(ty("*UserRepository")));
        assert_eq!(returns(Kind::Go, go, 4), Some(ty("AuditLog")));
        assert_eq!(returns(Kind::Go, go, 7), Some(ty("*Session")));
        assert_eq!(returns(Kind::Go, go, 10), None);
    }

    #[test]
    fn returns_read_methods_and_what_an_unannotated_python_def_constructs() {
        let new = |t: &str| Some(Value::New(t.into()));
        let py = r#"class Depot:
    def repo(self) -> Repo:
        return self.people

    def trail(self):
        """Doc.

        return Wrong()
        """
        def inner():
            return Other()

        if self.people:
            return Trail(
                self,
            )
        return Trail()  # again

    def either(self):
        if self.people:
            return Trail()
        return Repo()

    def maybe(self):
        if self.people:
            return Trail()
        return

    def stream(self):
        yield Trail()
        return Trail()

    @cache
    def cached(self):
        return Trail()

    def handed_on(self):
        return self.people
"#;
        assert_eq!(returns(Kind::Python, py, 2), Some(ty("Repo")));
        assert_eq!(returns(Kind::Python, py, 5), new("Trail"));
        assert_eq!(returns(Kind::Python, py, 19), None);
        assert_eq!(returns(Kind::Python, py, 24), None);
        assert_eq!(returns(Kind::Python, py, 29), None);
        assert_eq!(returns(Kind::Python, py, 34), None);
        assert_eq!(returns(Kind::Python, py, 37), None);
        let ts = "export class Depot {\n  async repo<T>(id: T): Promise<Repo> {\n    return load(id);\n  }\n  private static trail() {\n    return new Trail();\n  }\n  find = (id: number): Repo => load(id);\n}\ninterface Source {\n  source(): Repo;\n  maybe?(): Repo\n}\n";
        assert_eq!(returns(Kind::TsJs, ts, 2), Some(ty("Promise<Repo>")));
        assert_eq!(returns(Kind::TsJs, ts, 5), new("Trail"));
        // A property holding a function is not read.
        assert_eq!(returns(Kind::TsJs, ts, 8), None);
        assert_eq!(returns(Kind::TsJs, ts, 11), Some(ty("Repo")));
        assert_eq!(returns(Kind::TsJs, ts, 12), Some(ty("Repo")));
        let go = "func (d *Depot) Repo(id int) *Repo {\n\treturn d.people\n}\nfunc (d Depot) Trail() (Trail, error) {\n\treturn Trail{}, nil\n}\ntype Source interface {\n\tSource() *Repo\n\tClose()\n}\n";
        assert_eq!(returns(Kind::Go, go, 1), Some(ty("*Repo")));
        assert_eq!(returns(Kind::Go, go, 4), Some(ty("Trail")));
        assert_eq!(returns(Kind::Go, go, 8), Some(ty("*Repo")));
        assert_eq!(returns(Kind::Go, go, 9), None);
    }

    #[test]
    fn a_cast_is_read_as_the_type_it_writes() {
        let v = |kind, e| value_of(kind, e);
        let pycast = |callee: &str, t: &str| Value::Cast(callee.into(), t.into());
        assert_eq!(v(Kind::Python, "cast(Repo, row)"), pycast("cast", "Repo"));
        assert_eq!(
            v(Kind::Python, "typing.cast(\"models.Repo\", rows[0])"),
            pycast("typing.cast", "\"models.Repo\"")
        );
        assert_eq!(v(Kind::Python, "cast(Repo, row).other"), Value::Unknown);
        assert_eq!(
            v(Kind::Python, "recast(Repo, row)"),
            Value::Call("recast".into())
        );
        assert_eq!(v(Kind::TsJs, "row as Repo;"), ty("Repo"));
        assert_eq!(v(Kind::TsJs, "load(id) as unknown as Repo"), ty("Repo"));
        assert_eq!(v(Kind::TsJs, "{ a: 1 } as const"), ty("const"));
        assert_eq!(v(Kind::TsJs, "await load(a, b) as Repo"), ty("Repo"));
        // `as` takes the operand next to it, not the whole expression.
        assert_eq!(v(Kind::TsJs, "ok ? a : b as Repo"), Value::Unknown);
        assert_eq!(v(Kind::TsJs, "a ?? b as Repo"), Value::Unknown);
        assert_eq!(v(Kind::TsJs, "() => row as Repo"), Value::Unknown);
        assert_eq!(
            v(Kind::TsJs, "check(x as Foo) ? other : found as Repo"),
            Value::Unknown
        );
        assert_eq!(v(Kind::TsJs, "new Wrapper(x as Foo) as Repo"), ty("Repo"));
        // An `as` inside brackets or a string casts something else.
        assert_eq!(
            v(Kind::TsJs, "load(row as Repo)"),
            Value::Call("load".into())
        );
        assert_eq!(
            v(Kind::TsJs, "pick(\"x as Repo\")"),
            Value::Call("pick".into())
        );
        assert_eq!(v(Kind::Go, "i.(*Repo)"), ty("*Repo"));
        assert_eq!(v(Kind::Go, "ctx.Value.(models.Repo)"), ty("models.Repo"));
        assert_eq!(v(Kind::Go, "i.(*Repo).Owner"), Value::Unknown);

        let go = "func f(x any) {\n\tswitch v := x.(type) {\n\tcase *Repo:\n\t\tv.Go()\n\tcase A, B:\n\t\tv.Go()\n\tdefault:\n\t\tv.Go()\n\t}\n\tswitch v := x.(type) {\n\tcase *Audit:\n\t\tswitch x {\n\t\tcase 1:\n\t\t\tv.Go()\n\t\t}\n\t}\n\tswitch v := pick(); v {\n\tcase 1:\n\t\tv.Go()\n\t}\n}\n";
        let at = |line| bound_at(Kind::Go, go, line, "v");
        assert_eq!(at(4), [(2, ty("*Repo"))]);
        assert_eq!(at(6), [(2, Value::Unknown)]);
        assert_eq!(at(8), [(2, Value::Unknown)]);
        // Past a plain `switch` inside the arm, and not the closed type switch above.
        assert_eq!(at(14), [(10, ty("*Audit"))]);
        // No type switch: nothing the rules read binds `v` here.
        assert_eq!(at(19), []);
        // A `select` ends its `case` as a `switch` does: the closed type switch above it is not
        // read with the `case` of the `select`, and the parameter `v` stays what it is.
        let chans = "func f(v *Repo, x any, ch chan int) {\n\tswitch v := x.(type) {\n\tcase *Audit:\n\t\tv.Go()\n\t}\n\tselect {\n\tcase <-ch:\n\t\tv.Go()\n\t}\n}\n";
        assert_eq!(bound_at(Kind::Go, chans, 8, "v"), [(1, ty("*Repo"))]);
    }

    #[test]
    fn a_chain_may_hang_off_the_call_that_starts_it() {
        let head = |kind, line: &str| {
            let at = line.rfind('.').unwrap() + 1;
            call_head(kind, line, at).map(|(label, value, fields)| (label, value, fields.join(".")))
        };
        let call = |label: &str, callee: &str, fields: &str| {
            Some((
                label.to_owned(),
                Value::Call(callee.into()),
                fields.to_owned(),
            ))
        };
        assert_eq!(
            head(Kind::Go, "\treturn pkg.New(x, y).Run()"),
            call("pkg.New()", "pkg.New", "")
        );
        assert_eq!(
            head(
                Kind::Python,
                "    await make_uow(\")\").users.delete_user(1)"
            ),
            call("make_uow()", "make_uow", "users")
        );
        assert_eq!(
            head(Kind::Python, "x = self.repos.users.get_one(f(a), b).name"),
            call("self.repos.users.get_one()", "self.repos.users.get_one", "")
        );
        assert_eq!(
            head(Kind::TsJs, "  void new Depot(a).people.deleteUser(1);"),
            Some((
                "new Depot()".to_owned(),
                Value::New("Depot".into()),
                "people".to_owned()
            ))
        );
        let cast = |written: &str, t: &str| Some((written.to_owned(), ty(t), String::new()));
        assert_eq!(
            head(Kind::TsJs, "  void (found as Repo).find()"),
            cast("found as Repo", "Repo")
        );
        assert_eq!(
            head(Kind::Go, "\tfound.(*Repo).Find()"),
            cast("found.(*Repo)", "*Repo")
        );
        assert_eq!(
            head(Kind::Python, "    cast(Repo, found).find()"),
            Some((
                "cast(Repo, found)".to_owned(),
                Value::Cast("cast".into(), "Repo".into()),
                String::new()
            ))
        );
        // Brackets around a call are read through, as an `await` in front of one is.
        assert_eq!(
            head(Kind::TsJs, "  (await load()).find()"),
            call("load()", "load", "")
        );
        // A call of a call, an index, a generic call, a condition, a plain name.
        assert_eq!(head(Kind::Go, "\tOpen().Repo().Delete()"), None);
        assert_eq!(head(Kind::Python, "    make()[0].delete()"), None);
        assert_eq!(head(Kind::Python, "    items[0].load().delete()"), None);
        assert_eq!(head(Kind::TsJs, "  load<Repo>(id).find()"), None);
        assert_eq!(head(Kind::TsJs, "  if (ok).find()"), None);
        assert_eq!(head(Kind::Python, "    repo.find()"), None);
    }

    #[test]
    fn a_written_type_comes_down_to_one_name() {
        let path = |kind, t| type_path(kind, t).map(|p| p.join("."));
        let some = |s: &str| Some(s.to_owned());
        for (t, want) in [
            ("UserRepository", some("UserRepository")),
            ("\"Session\"", some("Session")),
            ("Optional[Repo]", some("Repo")),
            ("Repo | None", some("Repo")),
            ("Annotated[Repo, Depends(get_repo)]", some("Repo")),
            ("repos.UserRepository", some("repos.UserRepository")),
            ("list[Repo]", some("list")),
            ("Repo | Other", None),
        ] {
            assert_eq!(path(Kind::Python, t), want, "{t}");
        }
        for (t, want) in [
            ("Repo<User>", some("Repo")),
            ("Repo | null | undefined", some("Repo")),
            ("Repo[]", None),
            ("(a: A) => B", None),
            ("{ a: number }", None),
        ] {
            assert_eq!(path(Kind::TsJs, t), want, "{t}");
        }
        for (t, want) in [
            ("*Repo", some("Repo")),
            ("store.Session", some("store.Session")),
            ("Set[T]", some("Set")),
            ("[]Repo", None),
            ("map[string]Repo", None),
        ] {
            assert_eq!(path(Kind::Go, t), want, "{t}");
        }
    }

    #[test]
    fn imports_bind_names_to_module_paths() {
        let p = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let py = "import json\nimport numpy as np\nimport os.path\nfrom collections import OrderedDict, deque as dq\nfrom . import local\nfrom ..models import Note\nfrom typing import (\n    Any,\n    Final,\n)\n";
        let got = imports(Kind::Python, py);
        assert_eq!(
            got,
            [
                ("json".into(), p(&["json"])),
                ("np".into(), p(&["numpy"])),
                ("os".into(), p(&["os"])),
                ("OrderedDict".into(), p(&["collections", "OrderedDict"])),
                ("dq".into(), p(&["collections", "deque"])),
                // A relative import is a project file, marked by its leading `.` part.
                ("local".into(), p(&[".", "local"])),
                // Each dot past the first is a directory up.
                ("Note".into(), p(&["..", "models", "Note"])),
                ("Any".into(), p(&["typing", "Any"])),
                ("Final".into(), p(&["typing", "Final"])),
            ]
        );
        let rs = "use std::fs;\nuse std::collections::{HashMap, hash_map::Entry};\nuse regex::Regex as Re;\nuse crate::buffer::Buffer;\npub(crate) use anyhow::{self, Context};\nuse std::{\n    io::Write,\n    path::Path,\n};\n";
        let got = imports(Kind::Rust, rs);
        assert_eq!(
            got,
            [
                ("fs".into(), p(&["std", "fs"])),
                ("HashMap".into(), p(&["std", "collections", "HashMap"])),
                (
                    "Entry".into(),
                    p(&["std", "collections", "hash_map", "Entry"])
                ),
                ("Re".into(), p(&["regex", "Regex"])),
                ("anyhow".into(), p(&["anyhow"])),
                ("Context".into(), p(&["anyhow", "Context"])),
                ("Write".into(), p(&["std", "io", "Write"])),
                ("Path".into(), p(&["std", "path", "Path"])),
            ]
        );
        let go = "package x\n\nimport \"strings\"\n\nimport (\n\t\"fmt\"\n\ttoml \"github.com/BurntSushi/toml\"\n\t\"github.com/go-chi/chi/v5\"\n\t\"gopkg.in/yaml.v3\"\n\t\"github.com/mattn/go-sqlite3\"\n\t\"github.com/nats-io/nats.go\"\n\t\"k8s.io/api/core/v1\"\n)\n";
        let got = imports(Kind::Go, go);
        assert_eq!(
            got,
            [
                ("strings".into(), p(&["strings"])),
                ("fmt".into(), p(&["fmt"])),
                ("toml".into(), p(&["github.com", "BurntSushi", "toml"])),
                ("chi".into(), p(&["github.com", "go-chi", "chi", "v5"])),
                ("v5".into(), p(&["github.com", "go-chi", "chi", "v5"])),
                // The package name without the decorations its module path carries.
                ("yaml".into(), p(&["gopkg.in", "yaml.v3"])),
                ("sqlite3".into(), p(&["github.com", "mattn", "go-sqlite3"])),
                ("nats".into(), p(&["github.com", "nats-io", "nats.go"])),
                ("core".into(), p(&["k8s.io", "api", "core", "v1"])),
                ("v1".into(), p(&["k8s.io", "api", "core", "v1"])),
            ]
        );
        let ts = "import fs from 'node:fs';\nimport { join, resolve as res } from \"path\";\nimport * as React from 'react';\nimport type { Foo } from '@scope/pkg/sub';\nimport local from './local';\nconst chalk = require('chalk');\nconst { a, b } = await import('lib');\nimport cp = require('child_process');\nconst utils = require('../lib/utils');\nimport def, { type Bar, baz as qux } from './mixed';\n";
        let got = imports(Kind::TsJs, ts);
        assert_eq!(
            got,
            [
                // The last part is what the import takes: the default export, a name, or the
                // whole module.
                ("fs".into(), p(&["fs", "default"])),
                ("join".into(), p(&["path", "join"])),
                ("res".into(), p(&["path", "resolve"])),
                ("React".into(), p(&["react", "*"])),
                ("Foo".into(), p(&["@scope", "pkg", "sub", "Foo"])),
                ("local".into(), p(&[".", "local", "default"])),
                ("chalk".into(), p(&["chalk", "*"])),
                ("a".into(), p(&["lib", "a"])),
                ("b".into(), p(&["lib", "b"])),
                ("cp".into(), p(&["child_process", "*"])),
                ("utils".into(), p(&["..", "lib", "utils", "*"])),
                ("def".into(), p(&[".", "mixed", "default"])),
                ("Bar".into(), p(&[".", "mixed", "Bar"])),
                ("qux".into(), p(&[".", "mixed", "baz"])),
            ]
        );
    }

    #[test]
    fn module_files_are_the_project_files_an_import_names() {
        let (dir, files) = scratch(
            "modules",
            &[
                // Python: a src layout, a package, a module inside a package.
                ("src/app/__init__.py", ""),
                ("src/app/repos.py", ""),
                ("src/app/json.py", ""),
                ("src/app/store/__init__.py", ""),
                ("src/app/store/backends/memory.py", ""),
                // TypeScript: `paths` and `baseUrl` in the config a nested one extends.
                (
                    "tsconfig.base.json",
                    "{\n  // shared\n  \"compilerOptions\": {\n    \"baseUrl\": \"web\",\n    \"paths\": {\"@/*\": [\"src/*\"], \"@lib\": [\"lib/index.ts\"]},\n  },\n}\n",
                ),
                (
                    "web/tsconfig.json",
                    "{ \"$schema\": \"https://json.schemastore.org/tsconfig\", \"extends\": \"../tsconfig.base\" }\n",
                ),
                ("web/src/page.ts", ""),
                ("web/src/x.ts", ""),
                ("web/src/x.js", ""),
                ("web/src/types.d.ts", ""),
                ("web/src/ui/index.tsx", ""),
                ("web/lib/index.ts", ""),
                // Go: a module nested in another.
                ("go.mod", "module example.com/app\n\ngo 1.22\n"),
                ("internal/repo/repo.go", ""),
                ("internal/repo/repo_test.go", ""),
                ("internal/repo/sub/sub.go", ""),
                (
                    "tools/go.mod",
                    "module example.com/app/tools // generators\n",
                ),
                ("tools/gen/gen.go", ""),
            ],
        );
        let found = |kind, here: &str, module: &[&str]| -> Vec<String> {
            let module: Vec<String> = module.iter().map(|s| s.to_string()).collect();
            module_files(kind, &dir, &files, Path::new(here), &module)
                .iter()
                .map(|f| f.display().to_string())
                .collect()
        };
        let (py, ts, go) = (Kind::Python, Kind::TsJs, Kind::Go);
        assert_eq!(
            found(py, "main.py", &["app", "repos"]),
            ["src/app/repos.py"]
        );
        assert_eq!(
            found(py, "main.py", &["app", "store"]),
            ["src/app/store/__init__.py"]
        );
        // A module inside the `app` package is `app.json`, not the top-level `json`.
        assert!(found(py, "main.py", &["json"]).is_empty());
        let memory = "src/app/store/backends/memory.py";
        assert_eq!(found(py, memory, &[".."]), ["src/app/store/__init__.py"]);
        assert_eq!(found(py, memory, &["...", "repos"]), ["src/app/repos.py"]);
        assert!(found(py, "main.py", &["..", "repos"]).is_empty());
        let page = "web/src/page.ts";
        // The TypeScript file before the JavaScript one, also behind a `.js` specifier.
        assert_eq!(found(ts, page, &[".", "x"]), ["web/src/x.ts"]);
        assert_eq!(found(ts, page, &[".", "x.js"]), ["web/src/x.ts"]);
        assert_eq!(found(ts, page, &[".", "types"]), ["web/src/types.d.ts"]);
        assert_eq!(found(ts, page, &[".", "ui"]), ["web/src/ui/index.tsx"]);
        assert_eq!(found(ts, page, &["@", "ui"]), ["web/src/ui/index.tsx"]);
        assert_eq!(found(ts, page, &["@lib"]), ["web/lib/index.ts"]);
        // A name no alias matches, under `baseUrl`.
        assert_eq!(found(ts, page, &["src", "x"]), ["web/src/x.ts"]);
        assert!(found(ts, page, &["react"]).is_empty());
        assert!(found(ts, "page.ts", &["..", "x"]).is_empty());
        assert_eq!(
            found(go, "main.go", &["example.com", "app", "internal", "repo"]),
            ["internal/repo/repo.go"]
        );
        assert_eq!(
            found(go, "main.go", &["example.com", "app", "tools", "gen"]),
            ["tools/gen/gen.go"]
        );
        assert!(found(go, "main.go", &["example.com", "application"]).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn in_module_follows_the_parts_through_versions_and_escapes() {
        let p = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let m = |path: &str, parts: &[&str]| in_module(Path::new(path), &p(parts));
        assert!(m("/lib/python3.13/json/__init__.py", &["json"]));
        assert!(!m("/lib/python3.13/jsonschema/x.py", &["json"]));
        assert!(m("/rust/library/std/src/fs.rs", &["std", "fs"]));
        assert!(m(
            "/registry/src/idx/grep-regex-0.1.13/src/lib.rs",
            &["grep_regex"]
        ));
        assert!(!m(
            "/registry/src/idx/grep-searcher-0.1.13/src/lib.rs",
            &["grep_regex"]
        ));
        assert!(m(
            "/mod/github.com/!burnt!sushi/toml@v1.4.0/decode.go",
            &["github.com", "BurntSushi", "toml"]
        ));
        assert!(m("/go/src/strings/builder.go", &["strings"]));
        assert!(m("/node_modules/@types/node/fs.d.ts", &["fs"]));
        assert!(m(
            "/node_modules/@scope/pkg/sub/index.d.ts",
            &["@scope", "pkg", "sub"]
        ));
    }

    #[test]
    fn external_files_ignore_no_gitignore_and_keep_the_kind() {
        let dir = std::env::temp_dir().join(format!("merl-ext-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("pkg")).unwrap();
        std::fs::write(dir.join(".gitignore"), "pkg\n").unwrap();
        std::fs::write(dir.join("pkg/mod.py"), "").unwrap();
        std::fs::write(dir.join("pkg/mod.pyi"), "").unwrap();
        std::fs::write(dir.join("pkg/mod.so"), "").unwrap();
        let mut files = external_files(Kind::Python, std::slice::from_ref(&dir));
        files.sort();
        assert_eq!(files, [dir.join("pkg/mod.py"), dir.join("pkg/mod.pyi")]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn external_go_files_are_what_an_import_reaches() {
        let dir = std::env::temp_dir().join(format!("merl-ext-go-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for f in [
            "strings/strings.go",
            "strings/strings_test.go",
            "go/parser/testdata/x.go",
            "cmd/go.mod",
            "cmd/compile/main.go",
        ] {
            std::fs::create_dir_all(dir.join(f).parent().unwrap()).unwrap();
            std::fs::write(dir.join(f), "").unwrap();
        }
        assert_eq!(
            external_files(Kind::Go, std::slice::from_ref(&dir)),
            [dir.join("strings/strings.go")]
        );
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(declaration_file(Path::new(
            "node_modules/@types/node/fs.d.ts"
        )));
        assert!(declaration_file(Path::new("x/index.d.mts")));
        assert!(!declaration_file(Path::new("x/index.mjs")));
        assert!(!declaration_file(Path::new("x/index.ts")));
    }

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

    #[test]
    fn whole_word_excludes_longer_identifiers() {
        let (dir, files) = project("word");
        let py = files[..1].to_vec();
        assert_eq!(
            lines(&grep(&dir, &py, "total", true, false)),
            [("a.py".into(), 2)],
            "total_foobar is not the word `total`"
        );
        assert_eq!(lines(&grep(&dir, &py, "total", false, false)).len(), 3);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn smart_case_only_ignores_case_for_lowercase_patterns() {
        let (dir, files) = project("case");
        // Lowercase: `total`, `total_foobar` and Go's `Total`.
        assert_eq!(
            lines(&grep(&dir, &files, "total", false, true)),
            [
                ("a.py".into(), 2),
                ("a.py".into(), 11),
                ("a.py".into(), 12),
                ("b.go".into(), 5)
            ]
        );
        // One uppercase letter makes the whole pattern case-sensitive.
        assert_eq!(
            lines(&grep(&dir, &files, "Total", false, true)),
            [("b.go".into(), 5)]
        );
        // Without smart case a lowercase pattern stays case-sensitive too.
        assert_eq!(lines(&grep(&dir, &files, "invoice", false, false)), []);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn current_file_sorts_first_then_path_and_line() {
        let (dir, files) = project("sort");
        let hits = grep_project(
            &dir,
            &files,
            "Invoice",
            false,
            false,
            Some(Path::new("b.go")),
            None,
        )
        .unwrap();
        assert_eq!(
            lines(&hits),
            [
                ("b.go".into(), 3),
                ("b.go".into(), 5),
                ("b.go".into(), 7),
                ("a.py".into(), 1),
                ("a.py".into(), 7)
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Every pattern of [`LAST`], and the near-misses that carry the same letters but are code.
    #[test]
    fn tests_mocks_fixtures_and_generated_files_rank_last() {
        let here = Path::new("src/users/service.py");
        for (path, last) in [
            ("test/test_users.py", true),
            ("tests/users.py", true),
            ("src/__tests__/users.ts", true),
            ("spec/users_spec.rb", true),
            ("specs/users.rb", true),
            ("pkg/testdata/golden.go", true),
            ("tests/fixtures/users.json", true),
            ("src/__fixtures__/users.ts", true),
            ("src/mocks/store.ts", true),
            ("src/__mocks__/store.ts", true),
            ("vendor/github.com/x/y/y.go", true),
            ("third_party/x/y.py", true),
            ("src/test_users.py", true),
            ("src/conftest.py", true),
            ("pkg/users_test.go", true),
            ("src/users_spec.rb", true),
            ("src/users.test.ts", true),
            ("src/users.spec.ts", true),
            ("api/users_pb2.py", true),
            ("api/users_pb2_grpc.py", true),
            ("api/users.pb.go", true),
            ("api/users.gen.go", true),
            ("api/users.generated.ts", true),
            // The near-misses: the letters are there, the pattern is not.
            ("src/contest.rs", false),
            ("latest/users.py", false),
            ("src/testimonials.py", false),
            ("docs/spec.md", false),
            ("src/users/service.py", false),
            ("src/protest.go", false),
            ("src/specs.py", false),
        ] {
            let tier = rank(Path::new(path), Some(here), false).0;
            assert_eq!(tier == Tier::Tests, last, "{path} ranked {tier:?}");
        }
    }

    #[test]
    fn a_declaration_comes_first_and_the_nearest_directory_next() {
        let here = Path::new("src/users/service.py");
        let paths = [
            "src/api/admin.py",
            "tests/test_service.py",
            "src/users/repo.py",
            "src/users/service.py",
            "src/users/admin/view.py",
            "src/users/repo.py",
        ];
        // The last row is the declaration; the open file is the one that equals `here`.
        let declaration = |p: &str, i: usize| p == "src/users/repo.py" && i == 5;
        let mut order: Vec<(usize, &str)> = paths.iter().copied().enumerate().collect();
        order.sort_by_key(|&(i, p)| (rank(Path::new(p), Some(here), declaration(p, i)), p, i));
        assert_eq!(
            order.iter().map(|&(_, p)| p).collect::<Vec<_>>(),
            [
                "src/users/repo.py",       // the declaration
                "src/users/service.py",    // the open file
                "src/users/repo.py",       // the same directory
                "src/users/admin/view.py", // one below
                "src/api/admin.py",        // one up and one down
                "tests/test_service.py",   // a test file, whatever its distance
            ]
        );
        // The open file is never demoted, even when it is a test file itself.
        let test = Path::new("tests/test_service.py");
        assert_eq!(rank(test, Some(test), false).0, Tier::Open);
        // `d` asks for the tier alone: every candidate of its own is a declaration.
        assert_eq!(rank(test, None, true).0, Tier::Tests);
        assert_eq!(rank(here, None, true).0, Tier::Declaration);
    }

    fn symbol(kind: Option<Kind>, line: &str) -> Option<String> {
        let (_, pattern) = SYMBOLS.iter().find(|(k, _)| *k == kind).unwrap();
        symbol_name(&Regex::new(pattern).unwrap(), line)
    }

    /// Every name `D` lists for `line` in a file of `kind`: each [`SYMBOLS`] row such a file is
    /// read with, the all-language one included when [`shared_symbols`] takes it. A line listed
    /// twice comes back twice.
    fn listed(kind: Kind, line: &str) -> Vec<String> {
        SYMBOLS
            .iter()
            .filter(|(k, _)| *k == Some(kind) || (k.is_none() && shared_symbols(Some(kind))))
            .filter_map(|(_, p)| symbol_name(&Regex::new(p).unwrap(), line))
            .collect()
    }

    /// The one name `D` lists, and no second one.
    fn one(kind: Kind, line: &str) -> Option<String> {
        let names = listed(kind, line);
        assert!(names.len() <= 1, "{line}: listed as {names:?}");
        names.into_iter().next()
    }

    #[test]
    fn code_symbol_names() {
        for (line, name) in [
            ("def render(inv: Invoice) -> str:", Some("render")),
            ("    def total(self) -> int:", Some("total")),
            ("pub fn wrap_line(s: &str) {", Some("wrap_line")),
            ("pub(crate) struct Hit {", Some("Hit")),
            ("export async function parse(x) {", Some("parse")),
            ("export default class Foo {", Some("Foo")),
            ("    return total", None),
            ("class Invoice:", Some("Invoice")),
            ("func (i Invoice) Total() int {", Some("Total")),
            ("func main() {", Some("main")),
            ("type Invoice struct{}", Some("Invoice")),
            ("impl<T> Order<T> {", Some("Order")),
            (
                "pub(crate) const MAX_ORDERS: usize = 10;",
                Some("MAX_ORDERS"),
            ),
            ("static COUNT: u32 = 0;", Some("COUNT")),
            // Indented and bare, `const` and `static` are locals; exported, they are symbols.
            ("  const n = 1;", None),
            ("    static COUNT: u32 = 0;", None),
            ("  export const LIMIT = 10;", Some("LIMIT")),
            ("    pub const MAX: u32 = 1;", Some("MAX")),
            ("const fn zero() -> u32 {", Some("zero")),
            ("extern \"C\" fn c_call() {", Some("c_call")),
            ("macro_rules! order {", Some("order")),
            ("mod orders;", Some("orders")),
            ("pub union Bits {", Some("Bits")),
            (
                "export declare function load(id: string): void;",
                Some("load"),
            ),
            ("function* ids() {", Some("ids")),
            ("export namespace Orders {", Some("Orders")),
            ("export const parse = (s) => s;", Some("parse")),
            ("    render(o);", None),
            ("    return inv.render()", None),
            // A Go constant with a type or with several names, and a JavaScript name with a `$`:
            // the name is whatever follows the keyword, with nothing required after it.
            ("const Limit int = 10", Some("Limit")),
            ("const A, B = 1, 2", Some("A")),
            (
                "export const user$ = new BehaviorSubject(null);",
                Some("user"),
            ),
            // A dotted name is the first part: the namespace, not what is nested in it.
            ("export namespace Validation.Rules {", Some("Validation")),
            // Java, Kotlin and Ruby have rows of their own; their keywords are not here, so a Go
            // struct field and a line of prose are not declarations.
            ("\tmodule module.Version", None),
            ("module in general is to provide", None),
            ("module.exports = helpers;", None),
        ] {
            assert_eq!(symbol(None, line).as_deref(), name, "{line}");
        }
    }

    #[test]
    fn shell_symbol_names() {
        let sh = |line| symbol(Some(Kind::Shell), line);
        assert_eq!(sh("build() {").as_deref(), Some("build"));
        assert_eq!(sh("run_all ( ) {").as_deref(), Some("run_all"));
        assert_eq!(
            sh("  helper() {").as_deref(),
            Some("helper"),
            "nested, like the rest"
        );
        // The `function` forms are listed by the generic pattern instead, so each shows once.
        assert_eq!(sh("function deploy {"), None);
        assert_eq!(sh("function check() {"), None);
        assert_eq!(symbol(None, "function deploy {").as_deref(), Some("deploy"));
        assert_eq!(symbol(None, "function check() {").as_deref(), Some("check"));
        assert_eq!(symbol(None, "build() {"), None);
        for not_a_function in ["  build \"$ROOT\"", "x=$(build)", "start)", "ROOT=/srv"] {
            assert_eq!(sh(not_a_function), None, "{not_a_function}");
        }
    }

    #[test]
    fn sql_symbol_names() {
        let sql = |line| symbol(Some(Kind::Sql), line);
        assert_eq!(
            sql("CREATE TABLE public.orders (").as_deref(),
            Some("public.orders"),
            "the schema stays part of the name"
        );
        assert_eq!(
            sql("CREATE OR REPLACE FUNCTION total(o int)").as_deref(),
            Some("total")
        );
        assert_eq!(
            sql("CREATE UNIQUE INDEX orders_id_idx ON orders (id);").as_deref(),
            Some("orders_id_idx")
        );
        assert_eq!(
            sql("create materialized view daily_totals as").as_deref(),
            Some("daily_totals")
        );
        assert_eq!(
            sql("CREATE TABLE IF NOT EXISTS billing.invoices (").as_deref(),
            Some("billing.invoices")
        );
        assert_eq!(
            sql(r#"CREATE TABLE "user" ("#).as_deref(),
            Some(r#""user""#)
        );
        assert_eq!(
            sql("CREATE TYPE mood AS ENUM ('ok');").as_deref(),
            Some("mood")
        );
        for not_a_definition in [
            "SELECT * FROM orders;",
            "ALTER TABLE orders ADD COLUMN x int;",
            "DROP TABLE orders;",
            "INSERT INTO orders VALUES (1);",
            "CREATE TABLESPACE fast LOCATION '/x';",
        ] {
            assert_eq!(sql(not_a_definition), None, "{not_a_definition}");
        }
        // A CTE is a query's own scaffolding, not a project symbol.
        assert_eq!(sql("WITH recent AS ("), None);
        // The all-language pattern must not list SQL lines a second time: its keywords are
        // matched case-sensitively at the start of the line.
        for line in [
            "CREATE TYPE mood AS ENUM ('ok');",
            "create type mood as enum ('ok');",
            "CREATE TABLE public.orders (",
        ] {
            assert_eq!(symbol(None, line), None, "{line}");
        }
    }

    #[test]
    fn jvm_symbol_names() {
        let jvm = |line| one(Kind::Jvm, line);
        // What either language declares with a keyword, behind annotations and modifiers.
        for (line, name) in [
            ("public final class Invoice {", Some("Invoice")),
            ("interface Store {", Some("Store")),
            ("record Point(int x, int y) {}", Some("Point")),
            ("enum Status {", Some("Status")),
            ("public @interface Json {", Some("Json")),
            ("data class Order(val id: String)", Some("Order")),
            ("enum class Status {", Some("Status")),
            ("annotation class Json", Some("Json")),
            ("    suspend fun load(id: String): Order {", Some("load")),
            ("@Test fun parsesInvoice() {", Some("parsesInvoice")),
            ("external fun nativeInit()", Some("nativeInit")),
            ("object Registry {", Some("Registry")),
            ("typealias Rows = List<Order>", Some("Rows")),
            ("fun interface Handler {", Some("Handler")),
            // An extension function is listed under its own name, past the receiver.
            ("fun String.slug(): String = lowercase()", Some("slug")),
            ("fun <T> List<T>.second(): T = this[1]", Some("second")),
            (
                "fun String?.orEmpty(): String = this ?: \"\"",
                Some("orEmpty"),
            ),
            // A method: the return type before the name is what tells it from a call.
            ("    public int total() {", Some("total")),
            (
                "    static Map<String, Integer> compute(Map<String, Integer> rows) {",
                Some("compute"),
            ),
            (
                "    Map<String, List<Integer>> group(List<Integer> xs) {",
                Some("group"),
            ),
            ("    void save(Invoice inv);", Some("save")),
            (
                "    @Override public static <T> List<T> of(T one) {",
                Some("of"),
            ),
            // A constant behind `const`; a plain property or field is not a symbol.
            ("const val LIMIT = 10", Some("LIMIT")),
            ("    private const val TAG = \"Invoice\"", Some("TAG")),
            ("        const val MAX = 1", Some("MAX")),
            ("    val all = listOf<Order>()", None),
            ("    private static final int LIMIT = 10;", None),
            ("    static final Invoice EMPTY = new Invoice(0);", None),
            // `companion object` names nothing, and a call is not a declaration.
            ("    companion object {", None),
            ("    return compute(items);", None),
            ("    Map<String, Integer> rows = compute(items);", None),
            ("    System.out.println(x);", None),
            ("    } catch (IOException e) {", None),
            ("    if (parse(x)) {", None),
            ("    Card(title = \"Invoice\") {", None),
            ("        withContext(Dispatchers.IO) {", None),
            ("        return new Runnable() {", None),
            // The constructor is listed under its class.
            ("    public Invoice(int n) {", None),
        ] {
            assert_eq!(jvm(line).as_deref(), name, "{line}");
        }
    }

    #[test]
    fn ruby_symbol_names() {
        let rb = |line| one(Kind::Ruby, line);
        for (line, name) in [
            ("module Billing", Some("Billing")),
            ("  class Invoice < Base", Some("Invoice")),
            ("  class Billing::Invoice < Base", Some("Invoice")),
            ("    def initialize(id)", Some("initialize")),
            ("    def self.parse(text)", Some("parse")),
            ("    def Invoice.build(text)", Some("build")),
            ("    def total=(value)", Some("total")),
            ("    def empty?", Some("empty")),
            // No keyword to go by: a constant, and one `attr_accessor` line can declare
            // several names at once.
            ("  LIMIT = 10", None),
            ("    attr_accessor :total", None),
            ("    @cache ||= {}", None),
            ("    class << self", None),
            ("    Invoice.new(1)", None),
            ("  end", None),
        ] {
            assert_eq!(rb(line).as_deref(), name, "{line}");
        }
    }

    #[test]
    fn c_symbol_names() {
        let c = |line| one(Kind::C, line);
        for (line, name) in [
            // A function: in column zero, where C has no statements, anything but a prototype.
            (
                "int invoice_total(struct invoice *inv) {",
                Some("invoice_total"),
            ),
            ("static int compute(struct invoice *inv)", Some("compute")),
            (
                "void invoice_free(struct invoice *inv,",
                Some("invoice_free"),
            ),
            (
                "sds *sdssplitlen(const char *s, ssize_t len)",
                Some("sdssplitlen"),
            ),
            ("void Ledger::append(Rows rows) {", Some("append")),
            // GNU style: the return type is on the line above, so the name starts the line.
            (
                "edata_ind_get(const edata_t *edata) {",
                Some("edata_ind_get"),
            ),
            ("static unsigned", None),
            (
                "FMT_FUNC auto vformat(string_view f) -> std::string {",
                Some("vformat"),
            ),
            ("int invoice_total(struct invoice *inv);", None),
            // A method, indented, told from a call by the body it opens.
            (
                "  auto total() const -> int { return total_; }",
                Some("total"),
            ),
            ("  explicit Ledger(int n) : total_(n) {}", Some("Ledger")),
            (
                "  template <typename T> void write(T value) {",
                Some("write"),
            ),
            ("  void append(Rows rows);", None),
            ("    if (check(rows)) {", None),
            ("    log::write(rows);", None),
            ("    return compute(inv);", None),
            ("        fmt::format_to(out, \"{}\", 42);", None),
            ("template <typename Context = context, typename... T,", None),
            // A type, a namespace and an alias.
            ("struct invoice {", Some("invoice")),
            (
                "struct __attribute__ ((__packed__)) sdshdr8 {",
                Some("sdshdr8"),
            ),
            ("static struct config {", Some("config")),
            ("template <typename T> struct Box : Base {", Some("Box")),
            (
                "struct formatter<std::filesystem::path, Char> {",
                Some("formatter"),
            ),
            ("struct client;", None),
            ("union value {", Some("value")),
            ("enum class Status {", Some("Status")),
            ("namespace billing {", Some("billing")),
            ("class LEDGER_API Ledger : public Base {", Some("Ledger")),
            (
                "class basic_memory_buffer : public detail::buffer<T> {",
                Some("basic_memory_buffer"),
            ),
            ("using Rows = std::vector<int>;", Some("Rows")),
            ("using namespace detail;", None),
            // `using a::b;` imports a name; the row would otherwise be called `a`.
            ("using std::swap;", None),
            ("  using fmt::buffered_file;", None),
            ("struct invoice *current = NULL;", None),
            // The name a typedef gives a type, once: the opening `typedef struct invoice {` is
            // not listed, so the type is one row, under the name the project writes.
            ("typedef char *sds;", Some("sds")),
            ("typedef struct redisObject robj;", Some("robj")),
            ("} invoice;", Some("invoice")),
            ("typedef struct invoice {", None),
            // Indented, a closing brace ends a nested anonymous struct: that name is a field.
            ("    } offset;", None),
            ("} while (0);", None),
            ("};", None),
            // A macro, function-like or not.
            ("#define LRU_BITS 24", Some("LRU_BITS")),
            ("#  define FMT_THROW(x) throw x", Some("FMT_THROW")),
            ("#ifndef INVOICE_H", None),
        ] {
            assert_eq!(c(line).as_deref(), name, "{line}");
        }
    }

    #[test]
    fn csharp_symbol_names() {
        let cs = |line| one(Kind::CSharp, line);
        for (line, name) in [
            // A type, past its attributes, its modifiers and the generics it declares.
            (
                "public sealed partial class Invoice<T> : Base, IEnumerable<T>",
                Some("Invoice"),
            ),
            ("internal readonly struct Tag", Some("Tag")),
            ("public interface IStore<T> where T : class", Some("IStore")),
            ("public enum Status", Some("Status")),
            ("public record Money(decimal Amount);", Some("Money")),
            ("public record struct Point(int X, int Y);", Some("Point")),
            (
                "public delegate int Comparison<T>(T a, T b);",
                Some("Comparison"),
            ),
            // A namespace under its last part, the one `d` finds it by.
            ("namespace Billing.Core;", Some("Core")),
            ("namespace Billing", Some("Billing")),
            // A member: the type before the name is what tells it from a call.
            (
                "    public async Task<Invoice<T>> LoadAsync(int id)",
                Some("LoadAsync"),
            ),
            ("    void Save(Invoice<int> inv);", Some("Save")),
            (
                "    [Fact] public void Handles_Empty() {",
                Some("Handles_Empty"),
            ),
            (
                "    int IComparable.CompareTo(object? other) => 0;",
                Some("CompareTo"),
            ),
            (
                "    private static Rows Compute(int id) => new Rows();",
                Some("Compute"),
            ),
            // A property, by its accessors, its expression body, or the brace on the next line.
            ("    public int Total { get; private set; }", Some("Total")),
            ("    public string Name => _name;", Some("Name")),
            ("    public IReadOnlyList<int> Rows", Some("Rows")),
            // A field is not a symbol, in this kind as in every other.
            ("    private const int Limit = 10;", None),
            ("    private readonly ILogger<Invoice<T>> _logger;", None),
            ("    public static event EventHandler? Saved;", None),
            (
                "    public static Dictionary<string, Invoice<int>> All = new();",
                None,
            ),
            // The constructor is listed under its class, and a `using` alias is file-local.
            ("    public Invoice(int n)", None),
            ("using Rows = System.Collections.Generic.List<int>;", None),
            ("using System.Text.Json;", None),
            // A call, a statement and a block header are not declarations.
            ("        var rows = Compute(id);", None),
            ("        if (Check(rows))", None),
            ("        Console.WriteLine(rows);", None),
            ("        services.AddSingleton<IFoo, Foo>();", None),
            ("        return new Invoice<T>(id);", None),
            ("        foreach (var row in rows)", None),
            ("        catch (InvalidOperationException ex)", None),
            ("        using (var scope = provider.CreateScope())", None),
            ("        await client.SendAsync(request);", None),
            ("    Open,", None),
        ] {
            assert_eq!(cs(line).as_deref(), name, "{line}");
        }
    }

    #[test]
    fn swift_symbol_names() {
        let sw = |line| one(Kind::Swift, line);
        for (line, name) in [
            ("public final class Session: NSObject {", Some("Session")),
            (
                "@MainActor public struct Response<Value> {",
                Some("Response"),
            ),
            ("actor Cache {", Some("Cache")),
            ("public enum State {", Some("State")),
            (
                "public protocol RequestDelegate: AnyObject {",
                Some("RequestDelegate"),
            ),
            ("public typealias Rows = [Int]", Some("Rows")),
            ("    associatedtype Value", Some("Value")),
            // An extension is listed under the type it extends, which is what a project's own
            // members of that type sit in.
            ("extension Session: RequestDelegate {", Some("Session")),
            ("extension Array where Element: Hashable {", Some("Array")),
            // A function, past its generics; `class func` is a static method, not a class.
            (
                "    public func request<T: Encodable>(_ url: URL) -> Request {",
                Some("request"),
            ),
            ("    class func shared() -> Session {", Some("shared")),
            ("    mutating func append(_ row: Int) {", Some("append")),
            (
                "    @discardableResult func resume() -> Self {",
                Some("resume"),
            ),
            ("    func `default`() {", Some("default")),
            // What a type holds is not a symbol, in this kind as in every other, and an `init` is
            // listed under its type.
            ("    public static let `default` = Session()", None),
            ("    private let queue: DispatchQueue", None),
            ("    public var isRunning = false", None),
            ("    case initialized", None),
            ("    public init(queue: DispatchQueue = .main) {", None),
            // An operator has no name a reader would look it up by.
            (
                "    public static func == (lhs: Self, rhs: Self) -> Bool {",
                None,
            ),
            // A call, a binding and a pattern are not declarations.
            ("        let request = Request(url)", None),
            ("        queue.async {", None),
            ("        if let delegate = delegate {", None),
            ("        guard let url = url else { return }", None),
            ("        return Session()", None),
            ("        case let .failed(error):", None),
            ("        switch request.state {", None),
        ] {
            assert_eq!(sw(line).as_deref(), name, "{line}");
        }
    }

    #[test]
    fn php_symbol_names() {
        let php = |line| one(Kind::Php, line);
        for (line, name) in [
            (
                "abstract class Invoice implements Arrayable",
                Some("Invoice"),
            ),
            ("#[Attribute] final class Money", Some("Money")),
            ("interface Arrayable", Some("Arrayable")),
            ("trait Macroable", Some("Macroable")),
            ("enum Status: string", Some("Status")),
            ("namespace App\\Services;", Some("Services")),
            (
                "function billing_total(Invoice $invoice): int",
                Some("billing_total"),
            ),
            (
                "    final public static function parse(string $text): static",
                Some("parse"),
            ),
            (
                "    abstract protected function compute(): int;",
                Some("compute"),
            ),
            ("    public function &rows(): array", Some("rows")),
            (
                "    public const STATUS_OPEN = 'open';",
                Some("STATUS_OPEN"),
            ),
            // A property is a field, an enum case is what a type holds, and a `define()` has no
            // keyword before the name: none of them is a symbol.
            ("    protected array $rows = [];", None),
            ("    private ?Logger $logger;", None),
            ("    case Open = 'open';", None),
            ("define('BILLING_LIMIT', 10);", None),
            // A magic method is the language's hook, not the project's, as in C.
            (
                "    public function __construct(private readonly Account $account)",
                None,
            ),
            ("    public function __toString(): string", None),
            // A `use` imports, an anonymous function has no name, and a call is not a
            // declaration.
            ("use Illuminate\\Support\\Str;", None),
            ("    use Macroable;", None),
            ("$handler = function ($x) use ($y) {", None),
            ("        return $this->rows;", None),
            ("        foreach ($this->rows as $key => $value) {", None),
            ("        $total = 0;", None),
        ] {
            assert_eq!(php(line).as_deref(), name, "{line}");
        }
    }

    #[test]
    fn the_shared_pattern_skips_the_kinds_with_rows_of_their_own() {
        // Java, Kotlin, Ruby, C, C++, Lua and Elixir are listed from their own rows only, so
        // nothing is listed twice, `def self.parse` is not `self` and `function M.setup(` is
        // not `M`.
        assert!(!shared_symbols(Some(Kind::Jvm)));
        assert!(!shared_symbols(Some(Kind::Ruby)));
        assert!(!shared_symbols(Some(Kind::C)));
        assert!(!shared_symbols(Some(Kind::CSharp)));
        assert!(!shared_symbols(Some(Kind::Swift)));
        assert!(!shared_symbols(Some(Kind::Php)));
        assert!(!shared_symbols(Some(Kind::Lua)));
        assert!(!shared_symbols(Some(Kind::Elixir)));
        // Shell and SQL rows complement the shared pattern instead, and it reads every other
        // file, known kind or not.
        assert!(shared_symbols(Some(Kind::Shell)));
        assert!(shared_symbols(Some(Kind::Sql)));
        assert!(shared_symbols(Some(Kind::Python)));
        assert!(shared_symbols(None));
    }

    #[test]
    fn infra_symbol_names() {
        let make = |line| symbol(Some(Kind::Make), line);
        assert_eq!(make("build test: deps $(SRC)").as_deref(), Some("build"));
        assert_eq!(make("deps::").as_deref(), Some("deps"));
        assert_eq!(make("build-release:").as_deref(), Some("build-release"));
        for not_a_target in [
            ".PHONY: build",
            "%.o: %.c",
            "CC := gcc",
            "CC ::= gcc",
            "CC ?= gcc",
            "\t@echo a: b",
            "ifeq ($(OS),Windows_NT)",
            "$(OBJ): x",
        ] {
            assert_eq!(make(not_a_target), None, "{not_a_target}");
        }

        let tf = |line| symbol(Some(Kind::Terraform), line);
        assert_eq!(
            tf(r#"resource "aws_s3_bucket" "logs" {"#).as_deref(),
            Some("aws_s3_bucket.logs")
        );
        assert_eq!(
            tf(r#"data "aws_ami" "ubuntu" {"#).as_deref(),
            Some("data.aws_ami.ubuntu")
        );
        assert_eq!(tf(r#"variable "region" {"#).as_deref(), Some("var.region"));
        assert_eq!(tf(r#"module "vpc" {"#).as_deref(), Some("module.vpc"));
        assert_eq!(tf(r#"output "url" {"#).as_deref(), Some("output.url"));
        assert_eq!(tf(r#"  dynamic "ingress" {"#), None);

        let docker = |line| symbol(Some(Kind::Docker), line);
        assert_eq!(docker("FROM rust:1.80 AS build").as_deref(), Some("build"));
        assert_eq!(
            docker("from --platform=$P debian as runtime").as_deref(),
            Some("runtime")
        );
        assert_eq!(docker("FROM debian"), None);

        let yaml = |line| symbol(Some(Kind::Yaml), line);
        assert_eq!(yaml("x-common: &common").as_deref(), Some("&common"));
        assert_eq!(yaml("  <<: *common"), None);
        assert_eq!(yaml("apiVersion: v1"), None);
    }

    #[test]
    fn word_at_covers_the_run_under_and_before_the_cursor() {
        let line = "    inv = parse_it(\"x\")";
        assert_eq!(word_at(line, 4, ""), Some((4..7, "inv")));
        assert_eq!(word_at(line, 6, ""), Some((4..7, "inv")));
        // Right after a word counts as being on it; on a space it does not.
        assert_eq!(word_at(line, 7, ""), Some((4..7, "inv")));
        assert_eq!(word_at(line, 8, ""), None);
        assert_eq!(word_at(line, 10, ""), Some((10..18, "parse_it")));
        assert_eq!(word_at(line, line.len(), ""), None);
        assert_eq!(word_at("", 0, ""), None);
        assert_eq!(word_at("x", 99, ""), Some((0..1, "x")));
    }

    #[test]
    fn extra_word_chars_join_names_but_do_not_start_them() {
        let line = "  bucket = var.my-region[0]";
        assert_eq!(word_at(line, 17, ""), Some((15..17, "my")));
        assert_eq!(word_at(line, 17, "-"), Some((15..24, "my-region")));
        assert_eq!(word_at(line, 12, "-."), Some((11..24, "var.my-region")));
        // A flag's dashes and a trailing dot are not part of the name.
        assert_eq!(word_at("COPY --from=build", 8, "-"), Some((7..11, "from")));
        assert_eq!(word_at("x = var.", 6, "-."), Some((4..7, "var")));
        assert_eq!(word_at("- db", 0, "-"), None);
        assert_eq!(word_chars(Some(Kind::Terraform), true), "-.");
        assert_eq!(word_chars(Some(Kind::Terraform), false), "-");
        assert_eq!(word_chars(Some(Kind::Python), true), "");
        assert_eq!(word_chars(None, false), "");
    }
}
