use regex::Regex;
use std::path::Path;

use super::*;

pub const SYMBOL_PATTERN: &str = r#"^(?:\s*(?:(?:export|default|declare|async|pub(?:\([a-z]+\))?|static|unsafe|abstract|const|extern(?:\s+"[^"]*")?)\s+)*(?:def|class|func|function\*?|type|fn|struct|enum|impl|trait|interface|mod|union|macro_rules!|namespace)|(?:\s*(?:export|pub(?:\([a-z]+\))?)\s+)?(?:declare\s+)?(?:const|static))(?:<[^>]*>)?\s+(\([^)]*\)\s*)?(?P<name>[A-Za-z_]\w*)"#;

macro_rules! sql_create {
    () => {
        r"(?i)^\s*CREATE\s+(?:OR\s+REPLACE\s+)?(?:(?:TEMP|TEMPORARY|UNLOGGED|MATERIALIZED|UNIQUE)\s+)*(?:TABLE|VIEW|INDEX|FUNCTION|PROCEDURE|TRIGGER|TYPE|SCHEMA|SEQUENCE|DOMAIN|EXTENSION|DATABASE|ROLE|USER|ENUM)(?:\s+IF\s+NOT\s+EXISTS)?\s+"
    };
}
pub(super) use sql_create;

macro_rules! jvm_mods {
    () => {
        jvm_mods!("*")
    };
    (at_least_one) => {
        jvm_mods!("+")
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
macro_rules! scala_mods {
    () => {
        concat!(
            r"^\s*(?:@[\w.]+(?:\([^)]*\))?\s+)*",
            r"(?:(?:public|protected|private|internal|static|final|abstract|sealed|non-sealed",
            r"|strictfp|synchronized|native|default|transient|volatile|open|data|value|inner",
            r"|annotation|companion|enum|const|lateinit|expect|actual|suspend|override|inline",
            r"|operator|infix|tailrec|external|reified|implicit|lazy|case|transparent|opaque",
            r"|(?:private|protected)\[\w+\])\s+)*"
        )
    };
}
pub(super) use scala_mods;
macro_rules! scala_given_tail {
    () => {
        r"\s*(?:\[[^\]]*\]\s*)?(?:\([^)]*\)\s*)*:"
    };
}
pub(super) use scala_given_tail;

macro_rules! jvm_return_type {
    () => {
        concat!(
            r"(?:void|int|long|short|byte|char|boolean|float|double|[\w.]*[A-Z][\w.]*)",
            r"(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?(?:\[\])*"
        )
    };
}
pub(super) use jvm_return_type;
const JVM_DECL_SYMBOL: &str = concat!(
    scala_mods!(),
    r"(?:class|interface|enum|record|typealias|@interface|object|trait|type|package\s+object",
    r"|fun\s+interface|fun|const\s+(?:val|var))\s+(?:<[^>]*>\s*)?",
    r"(?:[\w.]+(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?\??\.)?",
    r"(?P<name>[A-Za-z_]\w*)"
);
const SCALA_GIVEN_SYMBOL: &str = concat!(
    scala_mods!(),
    r"given\s+`?(?P<name>[A-Za-z_]\w*)`?",
    scala_given_tail!()
);
const SCALA_DEF_SYMBOL: &str = concat!(
    "(?:",
    scala_mods!(),
    r"|^\s*extension\b.*?\b)def\s+`?(?P<name>[A-Za-z_]\w*)"
);
const GROOVY_DEF_SYMBOL: &str = concat!(scala_mods!(), r"def\s+(?P<name>[A-Za-z_]\w*)\s*\(");
const GRADLE_TASK_SYMBOL: &str =
    r#"(?:^\s*task\s+|\btasks\.(?:register|create)\s*\(?\s*['"])(?P<name>[A-Za-z_]\w*)"#;
pub fn row_reads(pattern: &str, path: &Path) -> bool {
    match pattern {
        SCALA_DEF_SYMBOL => !groovy(path),
        GROOVY_DEF_SYMBOL | GRADLE_TASK_SYMBOL => groovy(path),
        _ => true,
    }
}
const JAVA_METHOD_SYMBOL: &str = concat!(
    jvm_mods!(),
    r"(?:<[^>]*>\s*)?",
    jvm_return_type!(),
    r"\s+(?P<name>[A-Za-z_]\w*)\s*\("
);

macro_rules! c_mods {
    () => {
        concat!(
            r"(?:template\s*<[^>]*>\s*)?",
            r"(?:(?:static|extern|const|inline|constexpr|thread_local)\s+)*"
        )
    };
    (macros) => {
        r"(?:(?:[A-Z][A-Z0-9_]*|__\w+|_[A-Z][A-Z0-9_]*)\s*(?:\((?:[^()]|\([^()]*\))*\))?\s+)*"
    };
}
pub(super) use c_mods;
const C_FUNC_SYMBOL: &str = r"^(?:\w[^;(){}=]*[\s*&:])?(?P<name>_?[A-Za-z0-9]\w*)\s*\([^;]*$";
const C_METHOD_SYMBOL: &str =
    r"^\s+[^;(){}=]*\w[\s*&]+(?P<name>[A-Za-z_]\w*)\s*\([^;{}]*\)[^;{}=]*\{";
const C_TYPE_SYMBOL: &str = concat!(
    r"^\s*",
    c_mods!(),
    r"(?:struct|class|union|enum\s+class|enum\s+struct|enum|namespace|using)\s+",
    c_mods!(macros),
    r"(?:\w+(?:<[^<>]*>)?::)*(?P<name>[A-Za-z_]\w*)\s*(?:[{=<]|:[^:]|final\b|$)"
);
const C_TYPEDEF_SYMBOL: &str =
    r"^(?:\s*typedef\s+[^;]*?|\}\s*[\w\s,*]*)\b(?P<name>[A-Za-z_]\w*)\s*(?:\[[^\]]*\])?\s*;\s*$";
const C_MACRO_SYMBOL: &str = r"^\s*#\s*define\s+(?P<name>[A-Za-z_]\w*)";

macro_rules! cs_mods {
    () => {
        concat!(
            r"^\s*(?:\[[^\]]*\]\s*)*",
            r"(?:(?:public|private|protected|internal|file|static|readonly|const|sealed|abstract",
            r"|virtual|override|partial|async|extern|unsafe|new|volatile|event|required|fixed",
            r"|implicit|explicit|ref)\s+)*"
        )
    };
    (access) => {
        concat!(
            r"^\s*(?:\[[^\]]*\]\s*)*",
            r"(?:(?:public|private|protected|internal|static|unsafe|extern|partial)\s+)+"
        )
    };
}
pub(super) use cs_mods;

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

macro_rules! cs_generics {
    () => {
        r"(?:<[^<>]*(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>[^<>]*)*>)?"
    };
}
pub(super) use cs_generics;
const CS_DECL_SYMBOL: &str = concat!(
    cs_mods!(),
    r"(?:(?:class|struct|interface|enum|record\s+class|record\s+struct|record)\s+",
    r"|delegate\s+",
    cs_type!(),
    r"\s+|namespace\s+(?:[\w.]+\.)?)",
    r"(?P<name>[A-Za-z_]\w*)"
);
const CS_MEMBER_SYMBOL: &str = concat!(
    cs_mods!(),
    cs_type!(),
    r"\s+(?:[\w.]+\.)?(?P<name>[A-Za-z_]\w*)\s*(?:<[^<>]*>\s*)?(?:\(|\{|=>|$)"
);

macro_rules! swift_mods {
    () => {
        concat!(
            r"^\s*(?:@[\w.]+(?:\([^)]*\))?\s+)*",
            r"(?:(?:public|private|fileprivate|internal|open|package|static|class|final|override",
            r"|mutating|nonmutating|required|convenience|lazy|weak|unowned|dynamic|indirect",
            r"|optional|prefix|postfix|infix|nonisolated|distributed|borrowing|consuming)",
            r"(?:\(set\))?\s+)*"
        )
    };
}
pub(super) use swift_mods;
const SWIFT_DECL_SYMBOL: &str = concat!(
    swift_mods!(),
    r"(?:class|struct|enum|protocol|actor|extension|typealias|associatedtype|func)\s+",
    r"`?(?P<name>[A-Za-z_]\w*)"
);

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
const PHP_DECL_SYMBOL: &str = concat!(
    php_mods!(),
    r"(?:(?:class|interface|trait|enum)\s+|const\s+(?:[\w\\|&?()]+\s+)?|namespace\s+(?:[\w\\]+\\)?)",
    r"(?P<name>[A-Za-z_]\w*)"
);
const PHP_FUNC_SYMBOL: &str = concat!(php_mods!(), r"function\s+&?\s*(?P<name>_?[A-Za-z0-9]\w*)");
const RUBY_SYMBOL: &str =
    r"^\s*(?:def\s+(?:self\.|[A-Z]\w*\.)?|(?:class|module)\s+(?:[\w:]*::)?)(?P<name>[A-Za-z_]\w*)";
const LUA_FUNCTION_SYMBOL: &str =
    r"^\s*(?:local\s+)?function\s+(?:[\w.]+[.:])?(?P<name>[A-Za-z_]\w*)\s*\(";
const LUA_ASSIGNED_SYMBOL: &str =
    r"^\s*(?:local\s+)?(?:[\w.]+[.:])?(?P<name>[A-Za-z_]\w*)\s*=\s*function\b";
const ELIXIR_SYMBOL: &str = concat!(
    r"^\s*def(?:(?:module|protocol)\s+(?:[\w.]+\.)?|(?:p|macro|macrop|guard|guardp|delegate)?\s+)",
    r"(?P<name>[A-Za-z_]\w*[!?]?)"
);
const ZIG_INLINE_FN_SYMBOL: &str = concat!(
    r#"^\s*(?:(?:pub|export|extern(?:\s+"[^"]*")?)\s+)*"#,
    r"(?:inline|noinline)\s+fn\s+(?P<name>[A-Za-z_]\w*)"
);
const ZIG_TEST_SYMBOL: &str = r#"^\s*test\s+"(?P<name>[^"]*)""#;
pub(super) const EXUNIT_AND_MIX_DIRECTIVES: &[&str] = &[
    "describetag",
    "endpoint",
    "moduletag",
    "shortdoc",
    "switches",
    "tag",
];
pub(super) const ELIXIR_DIRECTIVES: &[&str] = &[
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
pub(super) const SQL_NAME_PART: &str = r#"(?:"[^"]+"|`[^`]+`|\w+)"#;
const SQL_CREATE_SYMBOL: &str = concat!(
    sql_create!(),
    r#"(?P<name>(?:"[^"]+"|`[^`]+`|\w+)(?:\.(?:"[^"]+"|`[^`]+`|\w+))?)"#
);
pub const SYMBOLS: &[(Option<Kind>, &str)] = &[
    (None, SYMBOL_PATTERN),
    (Some(Kind::Shell), r"^\s*(?P<name>[A-Za-z_]\w*)\s*\(\s*\)"),
    (Some(Kind::PowerShell), POWERSHELL_SYMBOL),
    (Some(Kind::Dart), DART_TYPE_SYMBOL),
    (Some(Kind::Dart), DART_FUNCTION_SYMBOL),
    (Some(Kind::Dart), DART_ACCESSOR_SYMBOL),
    (Some(Kind::Cmake), CMAKE_FUNCTION_SYMBOL),
    (Some(Kind::Cmake), CMAKE_TARGET_SYMBOL),
    (Some(Kind::Nix), NIX_FUNCTION_SYMBOL),
    (Some(Kind::Haskell), HASKELL_TYPE_SYMBOL),
    (Some(Kind::Haskell), HASKELL_PATTERN_SYMBOL),
    (Some(Kind::Haskell), HASKELL_SIGNATURE_SYMBOL),
    (Some(Kind::Haskell), HASKELL_EQUATION_SYMBOL),
    (Some(Kind::Ocaml), OCAML_SYMBOL),
    (Some(Kind::Fsharp), FSHARP_SYMBOL),
    (Some(Kind::Julia), JULIA_TYPE_SYMBOL),
    (Some(Kind::Julia), JULIA_FUNCTION_SYMBOL),
    (Some(Kind::Julia), JULIA_METHOD_SYMBOL),
    (Some(Kind::R), R_FUNCTION_SYMBOL),
    (Some(Kind::R), R_CLASS_SYMBOL),
    (Some(Kind::R), R6_CLASS_SYMBOL),
    (Some(Kind::Perl), PERL_SUB_SYMBOL),
    (Some(Kind::Perl), PERL_PACKAGE_SYMBOL),
    (Some(Kind::Gdscript), GDSCRIPT_SYMBOL),
    (Some(Kind::Solidity), SOLIDITY_SYMBOL),
    (Some(Kind::Clojure), CLOJURE_SYMBOL),
    (Some(Kind::EmacsLisp), EMACS_LISP_SYMBOL),
    (Some(Kind::Scheme), SCHEME_SYMBOL),
    (Some(Kind::CommonLisp), COMMON_LISP_SYMBOL),
    (Some(Kind::Starlark), STARLARK_RULE_SYMBOL),
    (Some(Kind::Starlark), STARLARK_TARGET_SYMBOL),
    (Some(Kind::Sql), SQL_CREATE_SYMBOL),
    (Some(Kind::Jvm), JVM_DECL_SYMBOL),
    (Some(Kind::Jvm), JAVA_METHOD_SYMBOL),
    (Some(Kind::Jvm), SCALA_GIVEN_SYMBOL),
    (Some(Kind::Jvm), SCALA_DEF_SYMBOL),
    (Some(Kind::Jvm), GROOVY_DEF_SYMBOL),
    (Some(Kind::Jvm), GRADLE_TASK_SYMBOL),
    (Some(Kind::Ruby), RUBY_SYMBOL),
    (Some(Kind::C), C_FUNC_SYMBOL),
    (Some(Kind::C), C_METHOD_SYMBOL),
    (Some(Kind::C), C_TYPE_SYMBOL),
    (Some(Kind::C), C_TYPEDEF_SYMBOL),
    (Some(Kind::C), C_MACRO_SYMBOL),
    (Some(Kind::C), OBJC_TYPE_SYMBOL),
    (Some(Kind::C), OBJC_METHOD_SYMBOL),
    (Some(Kind::CSharp), CS_DECL_SYMBOL),
    (Some(Kind::CSharp), CS_MEMBER_SYMBOL),
    (Some(Kind::Swift), SWIFT_DECL_SYMBOL),
    (Some(Kind::Php), PHP_DECL_SYMBOL),
    (Some(Kind::Php), PHP_FUNC_SYMBOL),
    (Some(Kind::Lua), LUA_FUNCTION_SYMBOL),
    (Some(Kind::Lua), LUA_ASSIGNED_SYMBOL),
    (Some(Kind::Elixir), ELIXIR_SYMBOL),
    (Some(Kind::Elixir), ERLANG_ATTRIBUTE_SYMBOL),
    (Some(Kind::Elixir), ERLANG_CLAUSE_SYMBOL),
    (Some(Kind::Zig), ZIG_INLINE_FN_SYMBOL),
    (Some(Kind::Zig), ZIG_TEST_SYMBOL),
    (
        Some(Kind::Proto),
        r"^\s*(?:message|enum|service|rpc)\s+(?P<name>[A-Za-z_]\w*)",
    ),
    (
        Some(Kind::Make),
        r"^(?P<name>[A-Za-z0-9_][\w./-]*)(\s+[\w./-]+)*\s*::?([^=:]|$)",
    ),
    (
        Some(Kind::Make),
        r"^\s*((export|override)\s+)*define\s+(?P<name>[^\s:=#+?!]+)",
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
    (
        Some(Kind::Graphql),
        r"^(?:(?:type|interface|input|enum|union|scalar|fragment|query|mutation|subscription)\s+|directive\s+@)(?P<name>[A-Za-z_]\w*)",
    ),
    (
        Some(Kind::Css),
        r"^\s*(?:@(?:mixin|function|(?:-[a-z]+-)?keyframes)\s+|%)(?P<name>[A-Za-z_-][\w-]*)",
    ),
];
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
                | Kind::Markdown
                | Kind::Graphql
                | Kind::Proto
                | Kind::Css
                | Kind::PowerShell
                | Kind::Dart
                | Kind::Cmake
                | Kind::Nix
                | Kind::Haskell
                | Kind::Ocaml
                | Kind::Fsharp
                | Kind::Julia
                | Kind::Perl
                | Kind::Gdscript
        )
    )
}
pub fn symbol_name(re: &Regex, line: &str) -> Option<String> {
    symbol_at(re, line).map(|s| s.name)
}
pub struct SymbolAt {
    pub byte_col: usize,
    pub name: String,
}
pub fn symbol_at(re: &Regex, line: &str) -> Option<SymbolAt> {
    let c = re.captures(line)?;
    let group = |name: &str| c.name(name).map(|m| m.as_str());
    let col = ["name", "a", "anchor"]
        .into_iter()
        .find_map(|g| c.name(g))
        .map_or(0, |m| m.start());
    if let (Some(block), Some(a)) = (group("block"), group("a")) {
        let name = match (block, group("b")) {
            ("resource", Some(n)) => format!("{a}.{n}"),
            ("data", Some(n)) => format!("data.{a}.{n}"),
            ("variable", None) => format!("var.{a}"),
            ("module" | "output", None) => format!("{block}.{a}"),
            _ => return None,
        };
        return Some(SymbolAt {
            byte_col: col,
            name,
        });
    }
    group("anchor")
        .map(|a| format!("&{a}"))
        .or_else(|| group("name").map(str::to_owned))
        .map(|name| SymbolAt {
            byte_col: col,
            name,
        })
}
pub fn fuzzy_match(query: &str, name: &str) -> bool {
    let mut left = name.chars();
    query
        .chars()
        .all(|q| left.any(|c| c.to_lowercase().eq(q.to_lowercase())))
}
