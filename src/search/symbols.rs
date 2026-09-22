//! The line patterns `D` lists a project's declarations with: one row per language,
//! and the name read out of a row that matched.

use regex::Regex;

use super::*;

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
pub(super) use sql_create;

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
pub(super) use jvm_mods;

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
pub(super) use jvm_return_type;
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
pub(super) use c_mods;
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
pub(super) use cs_mods;

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
pub(super) use cs_type;

/// A generic argument list, nesting three levels: a regex counts no brackets, and
/// `Task<ActionResult<QueryResult<T>>>` is what an ASP.NET controller action returns.
macro_rules! cs_generics {
    () => {
        r"(?:<[^<>]*(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>[^<>]*)*>)?"
    };
}
pub(super) use cs_generics;
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
pub(super) use swift_mods;
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
pub(super) use php_mods;
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
pub(super) const ELIXIR_DIRECTIVES: &[&str] = &[
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
pub(super) const SQL_NAME: &str = r#"(?:"[^"]+"|`[^`]+`|\w+)"#;
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
/// Whether `name` matches `query` as a name: the query's characters in order, ignoring case (as
/// `/` and `s`: `sameCancel` finds `SameCancel`). `D` past the cap
/// narrows its grep by this, so what comes back is what the picker then ranks. It is a name, not
/// a pattern: the picker's matcher also reads `^`, `!`, `'` and spaces as syntax of its own,
/// folds the accents off a letter, and matches the path beside the name — none of that reaches
/// the grep, so past the cap those keystrokes are characters of a name like any other.
pub fn fuzzy_match(query: &str, name: &str) -> bool {
    let mut left = name.chars();
    query
        .chars()
        .all(|q| left.any(|c| c.to_lowercase().eq(q.to_lowercase())))
}
