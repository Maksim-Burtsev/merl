use super::php::{php_constants, php_method, php_namespace_line, php_properties, php_tags};
use super::*;
use regex::Regex;
use std::path::{Path, PathBuf};

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
    /// The file of the module of the project an import binds the word to: `views` in `from
    /// shop import views`.
    Module(String),
    File,
    Trait(String),
    /// What a named argument, a literal's key or a JSX attribute names (#316): `parameter of
    /// followTopic`, `field of Opts`.
    Label(String),
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
            Self::Module(file) => write!(f, "module {file}"),
            Self::File => write!(f, "in this file"),
            Self::Trait(name) => write!(f, "via trait {name}"),
            Self::Label(what) => write!(f, "{what}"),
        }
    }
}
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
            // ASCII classes: Unicode's `\w` makes each grep build a DFA many times the size.
            format!(
                r"^\t(?:[A-Za-z_][A-Za-z0-9_]*[ \t]*,[ \t]*)*{w}(?:[ \t]*,[ \t]*[A-Za-z_][A-Za-z0-9_]*)*(?:[ \t]*=[^=]|[ \t]+[^ \t:=]|\[|[ \t]*$)"
            ),
        ],
        // `impl X` is a use of `X`, not its definition, so it is left out on purpose. A `let` is
        // a local of its block, which [`bindings`] reads: another function's never declares the
        // word (#353).
        Kind::Rust => {
            let vis = r#"^\s*(?:(?:pub(?:\([^)]*\))?|async|unsafe|const|extern(?:\s+"[^"]*")?|default)\s+)*"#;
            vec![
                format!(r"{vis}(?:fn|struct|enum|union|trait|type|const|static|mod)\s+{w}\b"),
                format!(r"^\s*macro_rules!\s+{w}\b"),
            ]
        }
        Kind::TsJs => {
            let pre = r"^\s*(?:(?:export|default|declare|abstract|async)\s+)*";
            let mut patterns = vec![
                format!(
                    r"{pre}(?:function\*?|class|interface|(?:const\s+)?enum|namespace|module)\s+{w}\b"
                ),
                format!(r"{pre}type\s+{w}\s*[=<]"),
                // Arrow functions assigned to a name land here too.
                format!(r"{pre}(?:const|let|var)\s+{w}\b"),
            ];
            patterns.extend(member_patterns(kind, word).unwrap_or_default());
            patterns
        }
        Kind::Jvm => {
            let (mods, ret) = (jvm_mods!(), jvm_return_type!());
            let mods_at_least_one = jvm_mods!(at_least_one);
            let mut patterns = vec![
                format!(
                    r"{mods}(?:class|interface|fun\s+interface|enum|record|@interface|object|typealias)\s+{w}\b"
                ),
                // Kotlin: a function, with its generics and, for an extension, its receiver.
                format!(
                    r"{mods}fun\s+(?:<[^>]*>\s*)?(?:[\w.]+(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?\??\.)?{w}\s*\("
                ),
                // A property, with an extension's receiver in front of its name: the receiver
                // itself, `Topic` in `val Topic.testTag`, is no declaration (#362).
                format!(
                    r"{mods}(?:val|var)\s+(?:<[^>]*>\s*)?(?:[\w.]+(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?\??\.)?{w}(?:[^\w.]|$)"
                ),
                // Java: a constructor, behind at least one modifier. With nothing in front,
                // `Card(title) {` is a Kotlin call with a trailing lambda and `new Runnable() {`
                // an anonymous class, so a bare name before `(` is never a declaration here; a
                // method is found by the return type in the rule below, brace or no brace.
                format!(r"{mods_at_least_one}{w}\s*\([^;]*\)\s*(?:throws [\w.,\s]+)?\{{\s*$"),
                // Java: a constructor whose parameters wrap, `Name(` at the end of the line or
                // followed by parameters that end in `,` (#367).
                format!(r"{mods_at_least_one}{w}\s*\((?:[^;()]*,)?\s*$"),
                // Java: a record's component, on a one-line header (#367), or on a line of a
                // header wrapped over lines, which [`declares_where`] checks.
                format!(
                    r"{mods}record\s+\w+\s*(?:<[^>]*>)?\s*\((?:[^)]*,)?\s*(?:@[\w.]+(?:\([^)]*\))?\s+)*{ret}\s+{w}\s*[,)]"
                ),
                format!(
                    r"{RECORD_COMPONENT_ANNOTATIONS}{ret}\s+{w}\s*(?:,\s*$|\)\s*(?:implements\b[^{{]*)?\{{)"
                ),
                // Kotlin: a property of a primary constructor on its class's line, `val` or
                // `var` after the `(` or a `,`; a plain parameter is no property (#367).
                format!(
                    r"{mods}(?:enum\s+|data\s+|value\s+)?class\s+\w+[^(]*\((?:.*[(,])?\s*(?:@[\w.]+(?:\([^)]*\))?\s+)*(?:(?:private|public|protected|internal|override|open|final)\s+)*(?:val|var)\s+{w}\s*:"
                ),
                // Java: an abstract or interface method, and a field: a return type, the name,
                // and the `(`, `;` or `=` that follows it.
                format!(r"{mods}(?:<[^>]*>\s*)?{ret}(?:\.\.\.)?\s+{w}\s*[(;=]"),
            ];
            patterns.extend(scala_patterns(word));
            patterns.extend(groovy_patterns(word));
            patterns
        }
        Kind::Ruby => {
            let end = r"(?:[^\w?!=]|$)";
            // `delegate :a, :b, to: :x` declares `a` and `b`, on a line of its own that closes
            // it: `prefix:` renames them (`user_email`), and a `delegate` wrapped over lines may
            // say so on a line below.
            let option = r"(?:(?:to|allow_nil|private):|prefix:\s*false\b)[^,#]*";
            let method = vec![
                format!(r"^\s*def\s+(?:self\??\.|[A-Z]\w*\.)?{w}{end}"),
                format!(r"^\s*alias(?:_method)?\s+:?{w}{end}"),
                format!(
                    r"^\s*delegate\s*\(?\s*(?::[\w?!]+\s*,\s*)*:{w}\s*,\s*(?::[\w?!]+\s*,\s*)*{option}(?:,\s*{option})*(?:#.*)?$"
                ),
            ];
            let attr = |which: &str, name: &str| {
                format!(
                    r"^\s*(?:[mc]?attr_(?:accessor|{which})|config_accessor)\s+(?:[:\w]+\s*,\s*)*:{name}\b"
                )
            };
            match word.strip_suffix('=') {
                // The reader and writer methods a class declares for its attributes, wherever
                // the name sits in the list.
                Some(name) => [method, vec![attr("writer", &regex::escape(name))]].concat(),
                None if word.ends_with(['?', '!']) => method,
                None if word.starts_with('@') => vec![ruby_assignment(word)],
                None => [
                    method,
                    vec![
                        // `class A::B` declares `B`, not `A`.
                        format!(r"^\s*(?:class|module)\s+(?:[\w:]+::)?{w}(?:[^\w:]|$)"),
                        ruby_assignment(word),
                        attr("reader", &w),
                        // An association, a scope, an attachment, an attribute or an enum.
                        format!(
                            r"^\s*(?:has_many|has_one|belongs_to|has_and_belongs_to_many|scope|has_one_attached|has_many_attached|has_attached_file|attribute|alias_attribute|enum)\s*\(?\s*:{w}{end}"
                        ),
                        format!(r"^\s*enum\s*\(?\s*{w}:"),
                        // Every name after the store's.
                        format!(r"^\s*store_accessor\s*\(?\s*:\w+\s*(?:,\s*:\w+\s*)*,\s*:{w}{end}"),
                        // A column, `t.string "language"`: [`ruby_column`] says where it counts.
                        format!(r#"^\s*t\.\w+\s*\(?\s*(?:"{w}"|:{w}{end})"#),
                    ],
                ]
                .concat(),
            }
        }
        Kind::C => {
            let (mods, macros) = (c_mods!(), c_mods!(macros));
            vec![
                format!(
                    r"^(?:\w[^;(){{}}=]*(?:\b(?:__\w+|_[A-Z][A-Z0-9_]*)\s*\([^()]*\)[^;(){{}}=]*)?[\s*&:])?{w}\s*\("
                ),
                // The same indented — a method in a class body, a function in an indented
                // namespace — when the body opens on the line.
                format!(r"^[^;(){{}}=]*\w[\s*&]+{w}\s*\([^;{{}}]*\)[^;{{}}=]*\{{"),
                format!(
                    r"^\s*{mods}(?:typedef\s+)?(?:struct|class|union|enum\s+class|enum\s+struct|enum|namespace)\s+{macros}(?:\w+(?:<[^<>]*>)?::)*{w}\s*(?:[{{;<]|:(?:[^:]|$)|final\b|$)"
                ),
                // A nested namespace, `namespace outer::inner {`, opens every name on its path.
                format!(r"^\s*(?:inline\s+)?namespace\s+(?:\w+::)*{w}\s*::"),
                // `typedef unsigned long ull;`, `typedef int (*cb)(void);`, and the name a
                // `typedef struct { … } client;` closes with, whose brace is in column zero: an
                // indented one closes a nested anonymous struct, and that name is a field. The
                // run before a `typedef`'s name crosses no `{`: `typedef struct client { int
                // flags; } client;` declares `client` alone (#382), by the brace's rule, which
                // reads a body opened on the line from column zero, or behind a `typedef` at any
                // indentation (one in a class body).
                format!(
                    r"^\s*typedef\s+[^;{{]*(?:\(\s*\*+\s*{w}\s*\)|\b{w}\s*(?:\[[^\]]*\])*\s*;)"
                ),
                format!(
                    r"^(?:\}}|(?:[^\s{{}}]|\s*typedef\b)[^{{}}]*\{{[^{{}}]*\}})\s*[\w\s,*]*\b{w}\s*[,;]"
                ),
                format!(r"^\s*(?:template\s*<[^>]*>\s*)?using\s+{w}\s*="),
                format!(r"^\s*#\s*define\s+{w}\b"),
                // A global, with the array bounds it can carry. In column zero again, so an
                // assignment inside a function body is not one; no angle brackets, or a
                // `template <typename T = U>` head would read as a declaration of `T`.
                format!(r"^\w[^;(){{}}=<>]*[\s*&]{w}\s*(?:\[[^\]]*\])*\s*(?:=[^=]|;)"),
                // An enum constant, on a line of its own or in a one-line `enum X { A, B };`,
                // and a member function declared with no body (#373). Callers index the rules
                // above, so these stay last, Objective-C's after them (#417).
                c_enumerators(Some(word)),
                c_enum_line(word),
                c_member_decl(Some(word)),
            ]
            .into_iter()
            .chain(objc_patterns(word))
            .collect()
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
                // Its body may stand on its line, `{ Id = id; }`, and its parameters wrap onto the
                // lines below, past a `(` or a `,` at the end of this one (#360).
                format!(
                    r"{access}{w}\s*\((?:[^;]*\)\s*(?::\s*(?:base|this)\b.*)?[{{=]?|[^;{{}}]*\)\s*(?::\s*(?:base|this)\b[^{{]*)?\{{.*\}}|[^;)]*)\s*$"
                ),
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
                // A function, past its generic parameters. A name in backticks, as a keyword
                // has to be written, is the name (#463).
                format!(r"{mods}func\s+`?{w}`?\s*[(<]"),
                format!(r"{mods}(?:let|var)\s+`?{w}\b"),
                // An enum case, alone or among several on one line, with the associated values or
                // the raw value it can carry. A `case .open:` or a `case let .open(x):` of a
                // `switch` is a pattern, and a `case open:` there matches against a constant, so
                // what follows the name must not be a `:`.
                format!(
                    r"^\s*(?:indirect\s+)?case\s+(?:`?\w+`?(?:\([^)]*\))?\s*,\s*)*`?{w}`?\s*(?:\(|=[^=]|,|$)"
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
            let [property, promoted] = php_properties(&w);
            let [property_tag, method_tag] = php_tags(&w);
            let [constant, case] = php_constants(&w);
            vec![
                php_method(&w),
                format!(r"{mods}(?:class|interface|trait|enum)\s+{w}\b"),
                php_namespace_line(&w),
                constant,
                format!(r#"^\s*define\s*\(\s*['"]{w}['"]"#),
                case,
                property,
                promoted,
                // An assignment that opens a line, `.=` and `??=` included. `==` compares, `=>`
                // is a key in an array literal, and `$rows['x'] =` writes to an element.
                format!(r"^\s*\${w}\s*(?:\.|\?\?|\+)?=(?:$|[^=>])"),
                property_tag,
                method_tag,
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
                format!(
                    r"^\s*def(?:p|macro|macrop|guard|guardp|delegate)?\s+{w}\s*(?:\(|,|do\b|$)"
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
            if !ELIXIR_DIRECTIVES.contains(&word) && !EXUNIT_AND_MIX_DIRECTIVES.contains(&word) {
                patterns.push(format!(r"^\s*@{w}\s+[^\s|]"));
            }
            patterns.extend(erlang_patterns(&w));
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
        // Protocol Buffers declares with a keyword, or with the `= N` of a number a message or an
        // enum gives each name. A field writes its type before the name and an enum value none.
        // An `extend` adds fields to a message declared elsewhere, as a Rust `impl` is a use of
        // its type, and an `option` sets a string, a bool or a name.
        // ponytail: `option x = 1;` would read as a field `x`; no option protoc knows takes a
        // number, and a custom one is `(x)`, so no rule tells them apart.
        Kind::Proto => vec![
            format!(r"^\s*(?:message|enum|service|oneof)\s+{w}\s*(?:\{{|$)"),
            // Braces or not, `stream` arguments too: the name and the `(` of the argument.
            format!(r"^\s*rpc\s+{w}\s*\("),
            format!(r"^\s*{w}\s*=\s*-?\d"),
            // `string id = 1;`, `repeated Order orders = 2;`, `map<string, int32> counts = 3;`,
            // `.shop.v1.User owner = 4;`.
            format!(
                r"^\s*(?:(?:repeated|optional|required)\s+)?(?:map\s*<[^>]*>|\.?[A-Za-z_][\w.]*)\s+{w}\s*=\s*\d"
            ),
        ],
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
                format!(r#"{create}(?:{SQL_NAME_PART}\.)?(?:"{w}"|`{w}`|{w}\b)(?:[^.]|$)"#),
                sql_cte(word),
            ]
        }
        // A target, alone or among others before the colon (`build test: deps`), a variable, or
        // a multi-line one, `define NAME` … `endef`, with the operator GNU make allows after it.
        Kind::Make => vec![
            format!(r"^([^:=#\s]+\s+)*{w}(\s+[^:=#\s]+)*\s*::?([^=:]|$)"),
            format!(r"^\s*(export\s+|override\s+)?{w}\s*[:?!]{{0,3}}="),
            format!(r"^\s*((export|override)\s+)*define\s+{w}\s*(\+=|[:?!]{{0,3}}=)?\s*(#|$)"),
        ],
        Kind::Terraform => terraform_patterns(word),
        Kind::PowerShell => powershell_patterns(word),
        Kind::Dart => dart_patterns(word),
        Kind::Cmake => cmake_patterns(word),
        Kind::Nix => nix_patterns(word),
        Kind::Haskell => haskell_patterns(word),
        Kind::Ocaml | Kind::Fsharp => ml_patterns(kind, word),
        Kind::Julia => julia_patterns(word),
        Kind::R => r_patterns(word),
        Kind::Perl => perl_patterns(word),
        Kind::Gdscript => gdscript_patterns(word),
        Kind::Solidity => solidity_patterns(word),
        Kind::Clojure | Kind::EmacsLisp | Kind::Scheme | Kind::CommonLisp => {
            lisp_patterns(kind, word)
        }
        Kind::Starlark => starlark_patterns(word),
        // `FROM image AS name`, with any flags before the image. Stage names ignore case.
        Kind::Docker => vec![format!(r"(?i)^\s*FROM\s+(\S+\s+)+AS\s+{w}\s*$")],
        // An anchor, or a key that opens a block: compose services, CI jobs, GitLab's `.hidden`
        // jobs.
        Kind::Yaml => vec![
            format!(r"(^|\s)&{w}(\s|$)"),
            format!(r"^\s*\.?{w}:\s*(#.*)?$"),
        ],
        Kind::Markdown => Vec::new(),
        // A definition of the type system, an operation or a fragment starts its line. `extend
        // type X {` is a use of `X`, as a Rust `impl` is. The indented rule is a field, one whose
        // arguments wrap included, or an enum value, and so is every selection of an operation:
        // [`declares_where`] keeps only the lines directly inside a type, an interface, an input
        // or an enum.
        Kind::Graphql => vec![
            format!(
                r"^(?:type|interface|input|enum|union|scalar|fragment|query|mutation|subscription)\s+{w}\b"
            ),
            format!(r"^directive\s+@{w}\b"),
            format!(r"^\s+{w}\s*(?:[:(@#,]|$)"),
        ],
        Kind::Css => css_patterns(word),
        // An HTML file declares an id, which `d` reads from the attribute under the cursor.
        Kind::Html => Vec::new(),
    }
}
pub fn narrow_patterns(
    kind: Kind,
    p: &mut Vec<String>,
    text: &str,
    row: usize,
    line: &str,
    r: std::ops::Range<usize>,
) {
    match kind {
        Kind::Php => php_namespace_patterns(p, php_block(text, row), line, r),
        Kind::PowerShell => powershell_sigil(p, &line[..r.start], &line[r.end..]),
        Kind::Dart => dart_narrow(p, line, r),
        Kind::Julia => julia_narrow(p, line, r),
        _ => {}
    }
}
/// Of the C and C++ candidates for `word` found by name, the ones that are the type itself
/// (#368), when a type line is among them:
/// - a forward declaration at namespace scope (`class X;`) yields to the one body of `X`; one
///   inside a class body declares a nested type and stays, and two bodies keep every row;
/// - a constructor or destructor (`X::X(`, `explicit X(` in the class) is not the type, unless
///   the word under the cursor is called or braced;
/// - C's `typedef struct X { … } X;` is one type: a bare `X` lands on the `} X;` line, the
///   tag `struct X` on the opening one.
///
/// The caller runs it only where a type can be meant: never behind `.` or `->` (#359).
#[derive(Clone, Copy)]
pub struct CTypeWord {
    pub constructs: bool,
    pub struct_tag: bool,
}
impl CTypeWord {
    pub const NAMED: CTypeWord = CTypeWord {
        constructs: false,
        struct_tag: false,
    };
}
pub fn c_type_rows(
    word: &str,
    hits: Vec<Hit>,
    text_of: impl Fn(&Path) -> Option<String>,
    used: CTypeWord,
) -> Vec<Hit> {
    let (w, mods, macros) = (regex::escape(word), c_mods!(), c_mods!(macros));
    let ty = format!(
        r"^\s*{mods}(?:typedef\s+)?(?:struct|class|union|enum\s+class|enum\s+struct|enum)\s+{macros}(?:\w+(?:<[^<>]*>)?::)*{w}\s*"
    );
    let re = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let forward = re(format!(r"{ty}(?::[^:{{;][^{{;]*)?;"));
    let body = re(format!(r"{ty}(?:[{{<]|:(?:[^:]|$)|final\b|$)"));
    let ctor = re(format!(
        r"\b{w}(?:<[^;()]*>)?::~?{w}\s*\(|^\s+(?:(?:explicit|constexpr|consteval|inline)\s+)*~?{w}\s*\("
    ));
    let opens = re(format!(
        r"^\s*typedef\s+(?:struct|union|enum)\s+{macros}{w}\s*(?:\{{[^}}]*)?$"
    ));
    let closes = re(format!(r"^\}}\s*[\w\s,*]*\b{w}\s*[,;]"));
    let is_body = |h: &Hit| body.is_match(&h.text) && !forward.is_match(&h.text);
    let typed = |h: &Hit| forward.is_match(&h.text) || body.is_match(&h.text);
    if !hits.iter().any(|h| typed(h) || closes.is_match(&h.text)) {
        return hits;
    }
    let one_body = hits.iter().filter(|h| is_body(h)).count() == 1;
    let typedef_halves_to_drop: Vec<(PathBuf, usize)> = hits
        .iter()
        .filter(|o| opens.is_match(&o.text))
        .filter_map(|o| {
            let close = hits
                .iter()
                .filter(|c| c.path == o.path && c.line > o.line && closes.is_match(&c.text))
                .min_by_key(|c| c.line)?;
            Some(match used.struct_tag {
                true => (close.path.clone(), close.line),
                false => (o.path.clone(), o.line),
            })
        })
        .collect();
    hits.into_iter()
        .filter(|h| {
            if typedef_halves_to_drop
                .iter()
                .any(|(p, l)| *p == h.path && *l == h.line)
            {
                return false;
            }
            if !used.constructs && ctor.is_match(&h.text) && !typed(h) {
                return false;
            }
            !(one_body
                && forward.is_match(&h.text)
                && !text_of(&h.path).is_some_and(|t| in_class_body(&t, h.line)))
        })
        .collect()
}
fn in_class_body(text: &str, line1: usize) -> bool {
    static CLASS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:template\s*<.*>\s*)?(?:class|struct|union)\b[^;]*$").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(depth) = line1
        .checked_sub(1)
        .and_then(|k| lines.get(k))
        .map(|l| indent(l))
    else {
        return false;
    };
    lines[..line1 - 1]
        .iter()
        .rev()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty()
                && t != "{"
                && !t.starts_with(['#', '/', '*'])
                && !matches!(t, "public:" | "private:" | "protected:")
        })
        .find(|l| indent(l) < depth)
        .is_some_and(|l| CLASS.is_match(l))
}
/// Of the C and C++ candidates for `word` found by name, the ones the file `here` can see (#364).
/// A source file (`.c`, `.cc`, `.cpp`, `.cxx`, `.m`, `.mm`) is compiled alone, so what another
/// one declares `static` at file scope, with `#define` or inside an unnamed `namespace {` is
/// visible in no other file: those rows go, unless `here` `#include`s that file or that file
/// `#include`s `here`. A header's rows stay, as do types (an opaque struct's body lives in one source
/// file). In `here` itself a file-scope `static` hides every other declaration of the name, so
/// it is the answer — unless the cursor stands on a candidate, where the others are offered as
/// namesakes.
pub fn c_file_local(
    word: &str,
    here: &Path,
    here_text: &str,
    hits: Vec<Hit>,
    text_of: impl Fn(&Path) -> Option<String>,
    cursor_on_candidate: bool,
) -> Vec<Hit> {
    static INCLUDE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r#"^\s*#\s*include\s*"([^"]+)""#).unwrap());
    static STATIC: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^(?:inline\s+)?static\b").unwrap());
    static DEFINE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*#\s*define\b").unwrap());
    let (w, macros) = (regex::escape(word), c_mods!(macros));
    let ty = Regex::new(&format!(
        r"\b(?:struct|class|union|enum)\s+{macros}{w}\b|^\s*typedef\b"
    ))
    .expect("an escaped name keeps the pattern valid");
    let local = |h: &Hit| STATIC.is_match(&h.text) && !ty.is_match(&h.text);
    if !cursor_on_candidate && hits.iter().any(|h| h.path == here && local(h)) {
        return hits
            .into_iter()
            .filter(|h| h.path == here && local(h))
            .collect();
    }
    let included: Vec<&str> = here_text
        .lines()
        .filter_map(|l| INCLUDE.captures(l).and_then(|c| c.get(1)))
        .map(|m| m.as_str())
        .collect();
    let source = |p: &Path| {
        p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e, "c" | "cc" | "cpp" | "cxx" | "m" | "mm"))
    };
    // The X-macro idiom: a source file that `#define`s a name and then `#include`s `here` hands
    // it what it declares. A quoted include is looked up next to the file first, so a namesake
    // there is the one it means (hiredis's `async.c` includes its own `dict.c`, not redis's).
    let includes_here = |file: &Path, t: &str| {
        let dir = file.parent().unwrap_or(Path::new(""));
        t.lines()
            .filter_map(|l| INCLUDE.captures(l).and_then(|c| c.get(1)))
            .any(|m| {
                let next = dir.join(m.as_str());
                next == here || here.ends_with(m.as_str()) && text_of(&next).is_none()
            })
    };
    hits.into_iter()
        .filter(|h| {
            if h.path == here || !source(&h.path) || included.iter().any(|i| h.path.ends_with(i)) {
                return true;
            }
            let text = text_of(&h.path);
            if text.as_deref().is_some_and(|t| includes_here(&h.path, t)) {
                return true;
            }
            if DEFINE.is_match(&h.text) {
                return false;
            }
            ty.is_match(&h.text)
                || !local(h) && !text.is_some_and(|t| in_unnamed_namespace(&t, h.line))
        })
        .collect()
}
fn in_unnamed_namespace(text: &str, line1: usize) -> bool {
    static UNNAMED: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*namespace\s*(?:\{|$)").unwrap());
    let literal = literal_lines(Kind::C, text);
    let (mut open, mut unnamed) = (Vec::new(), false);
    for (i, l) in text.lines().enumerate().take(line1.saturating_sub(1)) {
        let directive_or_its_body =
            l.trim_start().starts_with('#') || literal.get(i).copied().unwrap_or(false);
        if directive_or_its_body {
            continue;
        }
        unnamed |= UNNAMED.is_match(l);
        for (_, c) in code(Kind::C, l) {
            match c {
                b'{' => {
                    open.push(unnamed);
                    unnamed = false;
                }
                b'}' => {
                    open.pop();
                }
                _ => {}
            }
        }
    }
    open.contains(&true)
}
/// Whether the C line `line1` of `text` defines `word` only where nothing else does: a
/// `#define word` right under `#ifndef word` or `#if !defined(word)` (#364).
pub fn c_fallback(text: &str, line1: usize, word: &str) -> bool {
    let w = regex::escape(word);
    let re = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let define = re(format!(r"^\s*#\s*define\s+{w}\b"));
    let guard = re(format!(
        r"^\s*#\s*(?:ifndef\s+{w}\b|if\s+!\s*defined\s*\(?\s*{w}\b)"
    ));
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = line1.checked_sub(1).filter(|&k| k < lines.len()) else {
        return false;
    };
    define.is_match(lines[k])
        && lines[..k]
            .iter()
            .rev()
            .find(|l| {
                let t = l.trim();
                !t.is_empty() && !t.starts_with("//") && !t.starts_with("/*") && !t.starts_with('*')
            })
            .is_some_and(|l| guard.is_match(l))
}
/// A C parameter as its type alone, to tell a prototype of a function from an overload:
/// `const char *name = "x"` is `constchar*`. The name is the last word after another word, a `*`
/// or a `&`, unless it is a word of a built-in type (`unsigned long`). A type written in a way
/// this misreads only keeps a picker.
fn c_param_type(p: &str) -> String {
    static NAME: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"[\w*&\s]\b([A-Za-z_]\w*)\s*$").unwrap());
    let mut p = p.split('=').next().unwrap_or(p).trim();
    while let Some(open) = p.strip_suffix(']').and_then(|q| q.rfind('[')) {
        p = p[..open].trim_end();
    }
    let builtin = [
        "int", "char", "short", "long", "float", "double", "void", "signed", "unsigned", "bool",
    ];
    let p = match NAME.captures(p).and_then(|c| c.get(1)) {
        Some(name) if !builtin.contains(&name.as_str()) => &p[..name.start()],
        _ => p,
    };
    p.split_whitespace().collect()
}
struct CShape {
    declares: CDeclares,
    defines: bool,
}
#[derive(PartialEq)]
enum CDeclares {
    Function { params: Vec<String> },
    Variable,
}
pub struct COneDefinition {
    pub hit_index: usize,
    pub set_aside_note: String,
}
/// When the C or C++ candidates for `word` are one function — its definition and its prototypes,
/// all taking the same parameters — or one variable — its definition and its `extern`
/// declarations (#364). `None` for anything else: overloads, two definitions (`#if` / `#else`
/// variants), a macro, a type.
pub fn c_one_definition(
    word: &str,
    hits: &[Hit],
    text_of: impl Fn(&Path) -> Option<String>,
) -> Option<COneDefinition> {
    let w = regex::escape(word);
    let re = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let call = re(format!(r"\b{w}\s*\("));
    let global = re(format!(
        r"^\w[^;(){{}}=<>]*[\s*&]{w}\s*(?:\[[^\]]*\])*\s*(?:=[^=]|;)"
    ));
    let macros = c_mods!(macros);
    let other = re(format!(
        r"^\s*#|\b(?:struct|class|union|enum|namespace)\s+{macros}{w}\b|^\s*(?:typedef|using)\b"
    ));
    if hits.len() < 2 || hits.iter().any(|h| other.is_match(&h.text)) {
        return None;
    }
    let mut shapes = Vec::new();
    for h in hits {
        shapes.push(match call.find(&h.text) {
            Some(m) => {
                let text = text_of(&h.path)?;
                let s: String = text
                    .lines()
                    .skip(h.line - 1)
                    .take(30)
                    .collect::<Vec<_>>()
                    .join("\n");
                let open = m.end() - 1;
                let close = close_of(Kind::C, &s, open)?;
                let inner = &s[open + 1..close - 1];
                let params: Vec<String> = match inner.trim() {
                    "" | "void" => Vec::new(),
                    _ => split_top(Kind::C, inner, b',')
                        .into_iter()
                        .map(c_param_type)
                        .collect(),
                };
                let body = code(Kind::C, &s[close..])
                    .find(|(_, c)| matches!(c, b'{' | b';'))
                    .is_some_and(|(_, c)| c == b'{');
                CShape {
                    declares: CDeclares::Function { params },
                    defines: body,
                }
            }
            None if global.is_match(&h.text) => CShape {
                declares: CDeclares::Variable,
                defines: !h.text.starts_with("extern"),
            },
            None => return None,
        });
    }
    if shapes.iter().any(|s| s.declares != shapes[0].declares) {
        return None;
    }
    let mut defs = shapes.iter().enumerate().filter(|(_, s)| s.defines);
    let (Some((at, _)), None) = (defs.next(), defs.next()) else {
        return None;
    };
    let n = hits.len() - 1;
    let function = matches!(shapes[0].declares, CDeclares::Function { .. });
    let what = match (function, n) {
        (true, 1) => "prototype",
        (true, _) => "prototypes",
        (false, 1) => "declaration",
        (false, _) => "declarations",
    };
    Some(COneDefinition {
        hit_index: at,
        set_aside_note: format!("1 definition, {n} {what}"),
    })
}
pub fn c_declaration_only(word: &str, text: &str, line1: usize) -> bool {
    let w = regex::escape(word);
    let re = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let Some(l) = text.lines().nth(line1.saturating_sub(1)) else {
        return false;
    };
    let forward = re(format!(
        r"^\s*(?:template\s*<[^>]*>\s*)?(?:class|struct|union|enum)\s+(?:\w+\s+)*{w}\s*;"
    ));
    if forward.is_match(l) || (l.trim_start().starts_with("extern ") && !l.contains('(')) {
        return true;
    }
    let Some(m) = re(format!(r"\b{w}\s*\(")).find(l) else {
        return false;
    };
    let s: String = text
        .lines()
        .skip(line1 - 1)
        .take(30)
        .collect::<Vec<_>>()
        .join("\n");
    let Some(close) = close_of(Kind::C, &s, m.end() - 1) else {
        return false;
    };
    code(Kind::C, &s[close..])
        .find(|(_, c)| matches!(c, b'{' | b';'))
        .is_some_and(|(_, c)| c == b';')
}
/// Whether `word` is a parameter of the C function line `line1` of `text` stands in: named
/// after the `(` of the nearest line above in column zero that opens no brace of its own, as
/// a function's header does, the return type on the line above or not. The rules read no C
/// parameter yet (#378), and one definition must not hide that a parameter was meant.
pub fn c_parameter(text: &str, line1: usize, word: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = line1.checked_sub(1).filter(|&k| k < lines.len()) else {
        return false;
    };
    lines[..=k]
        .iter()
        .rev()
        .find(|l| {
            l.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && !l.trim_end().ends_with(';')
        })
        .and_then(|l| l.split_once('('))
        .is_some_and(|(_, params)| names(params, word))
}
/// ponytail: a flow list wrapped over several lines is not read.
pub fn stage_entries(before: &str, text: &str, word: &str) -> Option<Vec<usize>> {
    let key = regex::Regex::new(r#"^\s*stage:\s*["']?$"#).expect("a fixed pattern is valid");
    if !key.is_match(before) {
        return None;
    }
    let w = regex::escape(word);
    let flow = regex::Regex::new(&format!(r#"^stages:\s*\[(.*,)?\s*["']?{w}["']?\s*[,\]]"#))
        .expect("an escaped name keeps the pattern valid");
    let entry = regex::Regex::new(&format!(r#"^\s*-\s*["']?{w}["']?\s*(#.*)?$"#))
        .expect("an escaped name keeps the pattern valid");
    let opens = regex::Regex::new(r"^stages:\s*(#.*)?$").expect("a fixed pattern is valid");
    let mut found = Vec::new();
    let mut in_block = false;
    let continues_block = |l: &str| l.trim().is_empty() || l.starts_with([' ', '\t', '-', '#']);
    for (i, l) in text.lines().enumerate() {
        in_block &= continues_block(l);
        if in_block && entry.is_match(l) || flow.is_match(l) {
            found.push(i + 1);
        }
        in_block |= opens.is_match(l);
    }
    Some(found)
}
const SHAPE_WORD: &str = "MerlShapeWord";
fn ts_shapes(word: &str) -> [String; 3] {
    let def = def_patterns(Kind::TsJs, word);
    let members = member_patterns(Kind::TsJs, word).unwrap_or_default();
    let mut global = def.clone();
    global.extend(member_or_signature(Kind::TsJs, word).unwrap_or_default());
    [def.join("|"), members.join("|"), global.join("|")]
}
pub fn ts_shape(word: &str, pattern: &str) -> Option<&'static grep_regex::RegexMatcher> {
    static SHAPE: std::sync::LazyLock<grep_regex::RegexMatcher> = std::sync::LazyLock::new(|| {
        let [.., every] = ts_shapes(SHAPE_WORD);
        super::shape_matcher(&every.replace(SHAPE_WORD, "[A-Za-z0-9_$]+"))
    });
    let plain = !word.is_empty()
        && (word.bytes()).all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$');
    (plain && ts_shapes(word).iter().any(|p| p == pattern)).then(|| &*SHAPE)
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
fn ruby_column_elsewhere(path: &Path, text: &str) -> bool {
    text.trim_start().starts_with("t.") && !path.ends_with("db/schema.rb")
}
pub fn ruby_assignment(word: &str) -> String {
    format!(r"^\s*{}\s*(?:\|\|)?=($|[^=~>])", regex::escape(word))
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
                format!(
                    r"{mods}{w}\??\s*(?:<.*>)?\((?:(?:[^;]|:\s*\{{[^{{}}]*\}})*\{{\s*\}}?)?\s*$"
                ),
                // A method whose type parameters prettier wrapped: `route<` over `  T,` over
                // `>(path: T): this {` (#100).
                format!(r"{mods}{w}\??\s*<\s*$"),
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
        | Kind::Proto
        | Kind::Shell
        | Kind::PowerShell
        | Kind::Dart
        | Kind::Cmake
        | Kind::Nix
        | Kind::Haskell
        | Kind::Ocaml
        | Kind::Fsharp
        | Kind::Julia
        | Kind::R
        | Kind::Perl
        | Kind::Gdscript
        | Kind::Solidity
        | Kind::Clojure
        | Kind::EmacsLisp
        | Kind::Scheme
        | Kind::CommonLisp
        | Kind::Starlark
        | Kind::Sql
        | Kind::Make
        | Kind::Terraform
        | Kind::Docker
        | Kind::Yaml
        | Kind::Markdown
        | Kind::Graphql
        | Kind::Css
        | Kind::Html => return None,
    })
}
/// Line patterns that can declare `word` as a field, for the search by name: more than the fields,
/// since a line counts only when [`field_decl`] finds the type it is the first declaration of the
/// field in. `None` for a kind whose fields have no rules.
pub fn field_patterns(kind: Kind, word: &str) -> Option<Vec<String>> {
    let w = regex::escape(word);
    Some(match kind {
        Kind::Python => vec![
            format!(r"^\s+(?:[^#]*[:;]\s*)?(?:self\.)?{w}\s*(?::|=[^=])"),
            format!(r"\bself\.{w}\b.*[^=!<>]=[^=]|\bas\s+self\.{w}\b|^\s*for\s.*\bself\.{w}\b"),
        ],
        // A member behind any modifiers, bare or not, a constructor parameter behind one and
        // any decorators, `this.name = …`, a JSDoc `@property`.
        Kind::TsJs => vec![
            format!(
                r"^\s+(?:@[\w$.]+(?:\([^)]*\))?\s*)*(?:(?:public|private|protected|readonly|static|declare|override|abstract|accessor)\s+)*{w}\s*[?!]?\s*(?::|=[^=>]|;|$)"
            ),
            format!(r"^\s+this\.{w}\s*=[^=]"),
            // A `@property {T} name` of a JSDoc `@typedef {Object}` (#347).
            format!(r"^\s*\*\s*@prop(?:erty)?\s*\{{.*\}}\s*\[?{w}(?:[\]=\s]|$)"),
        ],
        // A struct field, alone or among others (`a, name T`), and an embedded `*pkg.Name`.
        Kind::Go => vec![
            format!(r"^\s+(?:[A-Za-z_]\w*\s*,\s*)*{w}(?:\s*,\s*[A-Za-z_]\w*)*\s+[^\s:=,(/]"),
            format!(r"^\s+\*?(?:[A-Za-z_]\w*\.)?{w}(?:\[.*\])?\s*(?:$|//|`)"),
        ],
        _ => return None,
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
pub fn declares_where<'a, S: AsRef<str> + 'a>(
    kind: Kind,
    path: &Path,
    word: &str,
    line: usize,
    line_text: &str,
    lines: impl FnOnce() -> &'a [S],
) -> bool {
    if component(path) {
        let lines = lines();
        let code = lines
            .get(line - 1)
            .is_some_and(|l| !l.as_ref().trim().is_empty());
        return code && declares_by_kind(kind, path, word, line, line_text, || lines);
    }
    declares_by_kind(kind, path, word, line, line_text, lines)
}
fn declares_by_kind<'a, S: AsRef<str> + 'a>(
    kind: Kind,
    path: &Path,
    word: &str,
    line: usize,
    line_text: &str,
    lines: impl FnOnce() -> &'a [S],
) -> bool {
    match kind {
        Kind::Graphql => !line_text.starts_with([' ', '\t']) || graphql_member(lines(), line),
        Kind::Css => css_declares(word, line, line_text, || {
            lines()
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>()
                .join("\n")
        }),
        Kind::Go if line_text.starts_with([' ', '\t']) => {
            let local = line_text
                .trim_start()
                .strip_prefix(word)
                .is_some_and(|rest| rest.trim_start().starts_with(":="));
            local || {
                let lines = lines();
                ["const (", "var (", "type ("]
                    .iter()
                    .any(|o| directly_inside(lines, line, o))
            }
        }
        Kind::Jvm if shaped_as_record_component(line_text) => in_record_header(lines(), line),
        Kind::Jvm => groovy_declares(path, word, line, line_text, lines).unwrap_or(true),
        Kind::C => c_declares_where(line, line_text, lines),
        Kind::Ruby if ruby_column_elsewhere(path, line_text) => false,
        Kind::PowerShell => powershell_declares(lines(), line, line_text),
        Kind::Dart => dart_declares(lines(), line, line_text),
        Kind::Nix => nix_declares(lines(), line, word, false),
        Kind::Elixir if erlang_head(line_text) => erlang_clause(lines(), line),
        Kind::Haskell => haskell_declares(lines(), line, word),
        Kind::Ocaml | Kind::Fsharp => ml_declares(kind, lines(), line, word),
        Kind::Julia => julia_declares(lines(), line, word),
        Kind::Perl => perl_declares(lines(), line, word, line_text),
        Kind::Gdscript => gdscript_declares(lines(), line, line_text),
        Kind::Solidity => solidity_declares(lines(), line, line_text),
        Kind::Clojure => lisp_declares(kind, lines(), line, line_text),
        _ => def_block(kind, word).is_none_or(|block| directly_inside(lines(), line, block)),
    }
}
const RECORD_COMPONENT_ANNOTATIONS: &str = r"^\s*(?:@[\w.]+(?:\([^)]*\))?\s+)*";
fn shaped_as_record_component(line: &str) -> bool {
    static SHAPE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(&format!(
            r"{RECORD_COMPONENT_ANNOTATIONS}{}\s+\w+\s*(?:,\s*$|\)\s*(?:implements\b[^{{]*)?\{{)",
            jvm_return_type!()
        ))
        .unwrap()
    });
    SHAPE.is_match(line) && !line.trim_start().starts_with("return ")
}
fn in_record_header<S: AsRef<str>>(lines: &[S], line1: usize) -> bool {
    static RECORD: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(concat!(
            jvm_mods!(),
            r"record\s+\w+\s*(?:<[^>]*>)?\s*\(\s*$"
        ))
        .unwrap()
    });
    let above = lines[..line1.saturating_sub(1).min(lines.len())]
        .iter()
        .rev();
    for l in above.take(40) {
        let l = l.as_ref();
        if RECORD.is_match(l) {
            return true;
        }
        let t = l.trim();
        if !(t.is_empty()
            || t.starts_with("//")
            || t.starts_with('@')
            || (shaped_as_record_component(l) && t.ends_with(',')))
        {
            return false;
        }
    }
    false
}
/// The block a definition of `word` has to sit directly inside, when its line pattern cannot
/// tell on its own: a Terraform local is `x = ...` inside `locals { }`, and so is every other
/// attribute.
pub fn def_block(kind: Kind, word: &str) -> Option<&'static str> {
    (kind == Kind::Terraform && word.starts_with("local.")).then_some("locals")
}
pub fn ts_nested_local<S: AsRef<str>>(lines: &[S], line: usize) -> bool {
    static DECL: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(
            r"^\s+(?:(?:async|abstract|declare)\s+)*(?:const|let|var|function\*?|class)\s",
        )
        .unwrap()
    });
    static SCOPE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(
            r"^\s*(?:export\s+)?(?:declare\s+)?(?:namespace|module)\s|^\s*(?:export\s+)?declare\s+global\b",
        )
        .unwrap()
    });
    static AT_LOAD: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(
            r"^\s*(?:[;!+~]\s*\(?|\(\s*|\}\)?\s*\(.*,\s*\(?)(?:async\s+)?(?:function\b|(?:\([^)]*\)|\w+)\s*=>)",
        )
        .unwrap()
    });
    let Some(target) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let target = target.as_ref();
    if !DECL.is_match(target) {
        return false;
    }
    let depth = indent(target);
    !lines[..line - 1]
        .iter()
        .map(AsRef::as_ref)
        .rev()
        .find(|l| {
            let t = l.trim_start();
            !t.is_empty()
                && indent(l) < depth
                && (!t.starts_with(['}', ')', ']', '/', '*']) || AT_LOAD.is_match(l))
        })
        .is_some_and(|l| SCOPE.is_match(l) || AT_LOAD.is_match(l))
}
pub fn ts_literal_key<S: AsRef<str>>(
    lines: &[S],
    line: usize,
    name: &str,
    key: &str,
) -> Option<usize> {
    let head = lines.get(line.checked_sub(1)?)?.as_ref();
    let open = Regex::new(&format!(
        r"^\s*(?:export\s+)?const\s+{}\s*(?::[^=]+)?=\s*\{{",
        regex::escape(name)
    ))
    .ok()?
    .find(head)?
    .end();
    let k = regex::escape(key);
    let form = Regex::new(&format!(
        r"^(?:(?:async|get|set|static)\s+)*\*?(?:{k}|'{k}'|\x22{k}\x22)\s*(?:[:(,}}]|$)"
    ))
    .ok()?;
    let literal_on_head = &head[open..];
    let mut depth = 0i32;
    let mut part = 0;
    for (i, c) in literal_on_head
        .char_indices()
        .chain([(literal_on_head.len(), ',')])
    {
        match c {
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' if depth > 0 => depth -= 1,
            ',' | '}' if depth == 0 => {
                if form.is_match(literal_on_head[part..i].trim()) {
                    return Some(line);
                }
                if c == '}' {
                    return None;
                }
                part = i + 1;
            }
            _ => {}
        }
    }
    key_on_wrapped_lines(lines, line, &form)
}
fn key_on_wrapped_lines<S: AsRef<str>>(
    lines: &[S],
    head_line1: usize,
    form: &Regex,
) -> Option<usize> {
    let outer = indent(lines[head_line1 - 1].as_ref());
    let mut level = None;
    for (n, l) in lines.iter().enumerate().skip(head_line1) {
        let l = l.as_ref();
        if l.trim().is_empty() {
            continue;
        }
        let d = indent(l);
        if d <= outer {
            return None;
        }
        if *level.get_or_insert(d) == d && form.is_match(l.trim_start()) {
            return Some(n + 1);
        }
    }
    None
}
pub fn owner_line(text: &str, line1: usize) -> Option<&str> {
    let lines: Vec<&str> = text.lines().collect();
    let depth = indent(lines.get(line1.checked_sub(1)?)?);
    lines[..line1 - 1]
        .iter()
        .rev()
        .find(|l| !l.trim().is_empty() && indent(l) < depth)
        .copied()
}
pub fn ts_global_scope(text: &str, line: usize) -> bool {
    static MODULE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"(?m)^(?:import|export)\b").unwrap());
    static GLOBAL: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*declare\s+global\s*\{").unwrap());
    static SCOPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:export\s+)?(?:declare\s+)?(?:interface|namespace)\s").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = lines.get(line.wrapping_sub(1)) else {
        return false;
    };
    let mut depth = indent(at);
    for l in lines[..line - 1].iter().rev() {
        if l.trim().is_empty() || indent(l) >= depth {
            continue;
        }
        depth = indent(l);
        if GLOBAL.is_match(l) {
            return true;
        }
        if !SCOPE.is_match(l) {
            return false;
        }
    }
    !MODULE.is_match(text)
}
pub fn directly_inside<S: AsRef<str>>(lines: &[S], line1: usize, opener: &str) -> bool {
    let Some(target) = line1.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let depth = indent(target.as_ref());
    lines[..line1 - 1]
        .iter()
        .map(AsRef::as_ref)
        .rev()
        .find(|l| !l.trim().is_empty() && indent(l) < depth)
        .is_some_and(|l| l.trim_start().starts_with(opener))
}
pub fn zig_visible(text: &str, line: usize, same_file: bool) -> bool {
    static CONTAINER: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(
            r"\b(?:struct|enum|union|opaque)\s*(?:\((?:[^()]|\([^()]*\))*\))?\s*\{\s*(?://.*)?$",
        )
        .unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(target) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let (mut depth, mut header_wraps_above) = (indent(target), false);
    for l in lines[..line - 1].iter().rev() {
        let code = uncommented(Kind::Zig, l);
        let t = code.trim();
        if t.is_empty() || indent(l) > depth || (indent(l) == depth && !header_wraps_above) {
            continue;
        }
        depth = indent(l);
        if CONTAINER.is_match(t) {
            break;
        }
        if ZIG_FN_OR_TEST_HEAD.is_match(t) {
            return false;
        }
        header_wraps_above = t.starts_with(')');
    }
    let open = target.trim_start();
    same_file || open.starts_with("pub ") || open.starts_with("export ")
}
/// Whether line `line1` of `lines`, a GraphQL file, declares a field or an enum value: the
/// nearest line above it indented less, past blanks and `#` comments, opens a `type`, an
/// `interface`, an `input` or an `enum`, or an `extend` of one, which declares fields all the
/// same. A selection of an operation or a fragment is the same line one level down elsewhere.
pub fn graphql_member<S: AsRef<str>>(lines: &[S], line1: usize) -> bool {
    static OPENER: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"^(?:extend\s+)?(?:type|interface|input|enum)\s").unwrap()
    });
    let Some(target) = line1.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let depth = indent(target.as_ref());
    lines[..line1 - 1]
        .iter()
        .map(AsRef::as_ref)
        .rev()
        .find(|l| {
            let t = l.trim_start();
            !t.is_empty() && !t.starts_with('#') && indent(l) < depth
        })
        .is_some_and(|l| OPENER.is_match(l))
}

/// A grep for the line that names one of `names` as a base: `class X(Base)` in Python,
/// `class X extends Base`, `class X implements Base` and `interface I extends Base` in
/// TypeScript, where the clause may also stand on a line of its own under a wrapped header —
/// [`type_decl_at`] walks up to the declaration it belongs to — as a Python base does under a
/// wrapped `class X(`. `None` for Go, whose types
/// implement an interface by carrying its methods and never name it.
pub fn subtype_patterns(kind: Kind, names: &[String]) -> Option<String> {
    let any: Vec<String> = names.iter().map(|n| regex::escape(n)).collect();
    let any = any.join("|");
    match kind {
        Kind::Python => Some(format!(
            r#"^\s*class\s+\w+\s*\(.*\b({any})\b|^\s+[\w.\[\]"', ]*\b({any})\b[\w.\[\]"', ]*(?:\)\s*:)?\s*(?:#.*)?$"#
        )),
        // Not only the header line: prettier writes `extends Base` on a line of its own.
        Kind::TsJs => Some(format!(r"\b(?:extends|implements)\s[^;]*\b({any})\b")),
        _ => None,
    }
}
