//! The line patterns `d` looks a declaration up with, per kind and per word, and the
//! reason a candidate is offered under.

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
    /// An item the file on screen declares where a bare Rust name sees it: no other file's item
    /// is in sight without a `use` or a path (#363).
    File,
    /// The method of the one trait every candidate of `x.word()` declares or implements
    /// (#358): whatever `x` is, the call reaches that declaration.
    Trait(String),
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
        // the assignment rule is not anchored at column zero as Python's is. An instance or class
        // variable is its own word, `@name` or `@@name`, and its assignment declares nothing else
        // (#383). The Rails-style DSL (`scope`, `has_many`, `define_method`) has no rule.
        // `name?`, `name!` and the setter `name=` are methods of their own (#387): the word
        // carries its suffix, and a bare `name` is none of them.
        Kind::Ruby => {
            let end = r"(?:[^\w?!=]|$)";
            let method = vec![
                // A method: `def name`, `def self.name`, `def Klass.name`.
                format!(r"^\s*def\s+(?:self\.|[A-Z]\w*\.)?{w}{end}"),
                format!(r"^\s*alias(?:_method)?\s+:?{w}{end}"),
            ];
            let attr = |which: &str, name: &str| {
                format!(r"^\s*attr_(?:accessor|{which})\s+(?:[:\w]+\s*,\s*)*:{name}\b")
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
                    ],
                ]
                .concat(),
            }
        }
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
                // uses one out. A nested type defined through its outer one, `struct Table::Rep {`,
                // declares `Rep` (#368): the `:` of a `::` is no base list.
                format!(
                    r"^\s*{mods}(?:typedef\s+)?(?:struct|class|union|enum\s+class|enum\s+struct|enum|namespace)\s+{macros}(?:\w+(?:<[^<>]*>)?::)*{w}\s*(?:[{{;<]|:(?:[^:]|$)|final\b|$)"
                ),
                // A nested namespace, `namespace outer::inner {`, opens every name on its path.
                format!(r"^\s*(?:inline\s+)?namespace\s+(?:\w+::)*{w}\s*::"),
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
                // Every `def` form. A clause is written `def name(x) do`, `def name do` or
                // `def name, do: x`. A trailing `?` or `!` is part of the word (#459), so
                // `ship` never finds `def ship!`.
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
                // The name, not a schema: `shop.tariffs` declares `tariffs`, so no `.` may follow (#471).
                format!(r#"{create}(?:{SQL_NAME}\.)?(?:"{w}"|`{w}`|{w}\b)(?:[^.]|$)"#),
                sql_cte(word),
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
        // A link is followed before any pattern is asked for (#421); a heading is prose.
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
    }
}
/// Of the C and C++ candidates for `word` found by name, the ones that are the type itself
/// (#368), when a type line is among them:
/// - a forward declaration at namespace scope (`class X;`) yields to the one body of `X`; one
///   inside a class body declares a nested type and stays, and two bodies keep every row;
/// - a constructor or destructor (`X::X(`, `explicit X(` in the class) is not the type, unless
///   the word under the cursor is `construction`, called or braced;
/// - C's `typedef struct X { … } X;` is one type: a bare `X` lands on the `} X;` line, the
///   `tag` `struct X` on the opening one.
///
/// The caller runs it only where a type can be meant: never behind `.` or `->` (#359).
pub fn c_type_rows(
    word: &str,
    hits: Vec<Hit>,
    text_of: impl Fn(&Path) -> Option<String>,
    construction: bool,
    tag: bool,
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
    // `typedef struct X {` and the `} X;` after it in the same file: the one of the pair to drop.
    let paired: Vec<(PathBuf, usize)> = hits
        .iter()
        .filter(|o| opens.is_match(&o.text))
        .filter_map(|o| {
            let close = hits
                .iter()
                .filter(|c| c.path == o.path && c.line > o.line && closes.is_match(&c.text))
                .min_by_key(|c| c.line)?;
            Some(match tag {
                true => (close.path.clone(), close.line),
                false => (o.path.clone(), o.line),
            })
        })
        .collect();
    hits.into_iter()
        .filter(|h| {
            if paired.iter().any(|(p, l)| *p == h.path && *l == h.line) {
                return false;
            }
            if !construction && ctor.is_match(&h.text) && !typed(h) {
                return false;
            }
            !(one_body
                && forward.is_match(&h.text)
                && !text_of(&h.path).is_some_and(|t| in_class_body(&t, h.line)))
        })
        .collect()
}
/// Whether 1-based `line` of a C++ `text` sits in a class, struct or union body: the nearest line
/// above indented less, past blanks, comments, directives and access specifiers, opens one.
fn in_class_body(text: &str, line: usize) -> bool {
    static CLASS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:template\s*<.*>\s*)?(?:class|struct|union)\b[^;]*$").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(depth) = line
        .checked_sub(1)
        .and_then(|k| lines.get(k))
        .map(|l| indent(l))
    else {
        return false;
    };
    lines[..line - 1]
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
/// A source file (`.c`, `.cc`, `.cpp`, `.cxx`) is compiled alone, so what another one declares
/// `static` at file scope, with `#define` or inside an unnamed `namespace {` is visible in no
/// other file: those rows go, unless `here` `#include`s that file or that file `#include`s
/// `here`. A header's rows stay, as do types (an opaque struct's body lives in one source
/// file). In `here` itself a file-scope `static` hides every other declaration of the name, so
/// it is the answer — unless the cursor stands `on` a candidate, where the others are offered as
/// namesakes.
pub fn c_file_local(
    word: &str,
    here: &Path,
    here_text: &str,
    hits: Vec<Hit>,
    text_of: impl Fn(&Path) -> Option<String>,
    on: bool,
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
    if !on && hits.iter().any(|h| h.path == here && local(h)) {
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
            .is_some_and(|e| matches!(e, "c" | "cc" | "cpp" | "cxx"))
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
/// Whether 1-based `line` of a C++ `text` sits inside an unnamed `namespace {`, by the braces
/// above it outside strings and comments.
fn in_unnamed_namespace(text: &str, line: usize) -> bool {
    static UNNAMED: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*namespace\s*(?:\{|$)").unwrap());
    let literal = literal_lines(Kind::C, text);
    let (mut open, mut unnamed) = (Vec::new(), false);
    for (i, l) in text.lines().enumerate().take(line.saturating_sub(1)) {
        if literal.get(i).copied().unwrap_or(false) {
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
/// Whether the C line `line` of `text` defines `word` only where nothing else does: a
/// `#define word` right under `#ifndef word` or `#if !defined(word)` (#364).
pub fn c_fallback(text: &str, line: usize, word: &str) -> bool {
    let w = regex::escape(word);
    let re = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let define = re(format!(r"^\s*#\s*define\s+{w}\b"));
    let guard = re(format!(
        r"^\s*#\s*(?:ifndef\s+{w}\b|if\s+!\s*defined\s*\(?\s*{w}\b)"
    ));
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = line.checked_sub(1).filter(|&k| k < lines.len()) else {
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
/// When the C or C++ candidates for `word` are one function — its definition and its prototypes,
/// all taking the same parameters — or one variable — its definition and its `extern`
/// declarations — the index of the one definition, and what was set aside for the status line:
/// `1 definition, 2 prototypes` (#364). `None` for anything else: overloads, two definitions
/// (`#if` / `#else` variants), a macro, a type.
pub fn c_one_definition(
    word: &str,
    hits: &[Hit],
    text_of: impl Fn(&Path) -> Option<String>,
) -> Option<(usize, String)> {
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
    // Per candidate: the parameters it takes (`None` for a variable), and whether it defines.
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
                (Some(params), body)
            }
            None if global.is_match(&h.text) => (None, !h.text.starts_with("extern")),
            None => return None,
        });
    }
    if shapes.iter().any(|(p, _)| *p != shapes[0].0) {
        return None;
    }
    let mut defs = shapes.iter().enumerate().filter(|(_, (_, body))| *body);
    let (Some((at, _)), None) = (defs.next(), defs.next()) else {
        return None;
    };
    let n = hits.len() - 1;
    let what = match (shapes[0].0.is_some(), n) {
        (true, 1) => "prototype",
        (true, _) => "prototypes",
        (false, 1) => "declaration",
        (false, _) => "declarations",
    };
    Some((at, format!("1 definition, {n} {what}")))
}
/// Whether `word` is a parameter of the C function 1-based `line` of `text` stands in: named
/// after the `(` of the nearest line above in column zero that opens no brace of its own, as
/// a function's header does, the return type on the line above or not. The rules read no C
/// parameter yet (#378), and one definition must not hide that a parameter was meant.
pub fn c_parameter(text: &str, line: usize, word: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = line.checked_sub(1).filter(|&k| k < lines.len()) else {
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
/// GitLab CI's `stage: build` names an entry of the file's top-level `stages:` list, not a job
/// named after its stage (#473): the 1-based lines listing `word`, in the flow form (`stages:
/// [build, test]`) or the block form (`- build` under `stages:`), none when the file lists no
/// such stage. `None` when `before`, the line in front of the word, is no `stage:` key.
///
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
    for (i, l) in text.lines().enumerate() {
        // The block runs while its lines are indented, entries at the key's own column, blank or
        // comments.
        in_block &= l.trim().is_empty() || l.starts_with([' ', '\t', '-', '#']);
        if in_block && entry.is_match(l) || flow.is_match(l) {
            found.push(i + 1);
        }
        in_block |= opens.is_match(l);
    }
    Some(found)
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
/// A Ruby assignment of `word`, `||=` included; `==`, `=~` and `=>` are not one. The word
/// carries its sigil: `@name =` assigns `@name`, never `name` (#383).
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
                // A class or object-literal method: `foo(` at the end of the line, `foo(..) {`,
                // or an empty `foo(): void {}`. A `;` on the line means it was a call statement.
                format!(r"{mods}{w}\s*(?:<.*>)?\((?:[^;]*\{{\s*\}}?)?\s*$"),
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
        | Kind::Sql
        | Kind::Make
        | Kind::Terraform
        | Kind::Docker
        | Kind::Yaml
        | Kind::Markdown
        | Kind::Graphql => return None,
    })
}
/// Line patterns that can declare `word` as a field, for the search by name: more than the fields,
/// since a line counts only when [`field_decl`] finds the type it is the first declaration of the
/// field in. `None` for a kind whose fields have no rules.
pub fn field_patterns(kind: Kind, word: &str) -> Option<Vec<String>> {
    let w = regex::escape(word);
    Some(match kind {
        // `name: T` or `name = …` in a class body, `self.name = …` in a method, `self.name` in a
        // tuple, a `with` or a `for` target; behind a header or a `;` on its line (#131).
        Kind::Python => vec![
            format!(r"^\s+(?:[^#]*[:;]\s*)?(?:self\.)?{w}\s*(?::|=[^=])"),
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
/// Whether a line a pattern of [`def_patterns`] for `word` matched, 1-based `line` of a file of
/// `kind` whose text is `line_text`, declares the word where it sits. Every consumer of the
/// patterns asks it, so `d` and `u` agree on what declares (#419). Where the line alone cannot
/// tell, the lines above it can: a Terraform local is `x = ...` directly inside `locals { }`,
/// as every other attribute is inside its block, and an indented GraphQL line is a field or an
/// enum value directly inside a type and a selection anywhere else. `lines` reads the file, and
/// only for those.
pub fn declares_where<'a, S: AsRef<str> + 'a>(
    kind: Kind,
    word: &str,
    line: usize,
    line_text: &str,
    lines: impl FnOnce() -> &'a [S],
) -> bool {
    match kind {
        Kind::Graphql => !line_text.starts_with([' ', '\t']) || graphql_member(lines(), line),
        _ => def_block(kind, word).is_none_or(|block| directly_inside(lines(), line, block)),
    }
}
/// The block a definition of `word` has to sit directly inside, when its line pattern cannot
/// tell on its own: a Terraform local is `x = ...` inside `locals { }`, and so is every other
/// attribute.
pub fn def_block(kind: Kind, word: &str) -> Option<&'static str> {
    (kind == Kind::Terraform && word.starts_with("local.")).then_some("locals")
}
/// Whether 1-based `line` of `lines` sits directly inside a block whose first line starts with
/// `opener`: the nearest non-blank line above it that is indented less. `terraform fmt` indents
/// every block, so the indentation is the nesting.
pub fn directly_inside<S: AsRef<str>>(lines: &[S], line: usize, opener: &str) -> bool {
    let Some(target) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let depth = indent(target.as_ref());
    lines[..line - 1]
        .iter()
        .map(AsRef::as_ref)
        .rev()
        .find(|l| !l.trim().is_empty() && indent(l) < depth)
        .is_some_and(|l| l.trim_start().starts_with(opener))
}
/// A common table expression named `word`: opening the `WITH`, or continuing it after the
/// comma that follows the previous one's closing `)`. The match ends on the `(` of its body.
pub fn sql_cte(word: &str) -> String {
    let w = regex::escape(word);
    format!(r"(?i)^\s*(?:\)\s*)?(?:WITH\s+(?:RECURSIVE\s+)?|,\s*)?{w}\s+AS\s*\(")
}
/// Whether byte `col` of 1-based `line` of `text` sees the common table expression `word` that
/// 1-based `cte` opens (#472). A CTE lives in its own statement: after its `AS (…)` up to the
/// `;` that ends it, and inside its own body too under `WITH RECURSIVE`. A bracket or a `;` in
/// a comment or a `'string'` counts for nothing. `None` past its body when no `;` ends the
/// statement (a script of T-SQL batches split by `GO`, say): where its scope ends is not known.
pub fn sql_cte_sees(text: &str, cte: usize, word: &str, line: usize, col: usize) -> Option<bool> {
    let start = |l: usize| -> usize { text.split_inclusive('\n').take(l - 1).map(str::len).sum() };
    let head = Regex::new(&sql_cte(word)).expect("an escaped name keeps the pattern valid");
    let Some(m) = text.lines().nth(cte - 1).and_then(|l| head.find(l)) else {
        return Some(false);
    };
    let (open, at, b) = (start(cte) + m.end() - 1, start(line) + col, text.as_bytes());
    // The statement's first byte, the depth inside the body, and the body's closing `)`.
    let (mut first, mut depth, mut close, mut i) = (0, None::<usize>, None, 0);
    let mut ended = false;
    while i < b.len() {
        let skip = match &b[i..] {
            [b'-', b'-', ..] => "\n",
            [b'/', b'*', ..] => "*/",
            [b'\'', ..] => "'",
            [b'(', ..] if i >= open => {
                depth = Some(depth.map_or(1, |d| d + 1));
                ""
            }
            [b')', ..] if close.is_none() => {
                depth = depth.map(|d| d - 1);
                if depth == Some(0) {
                    close = Some(i);
                }
                ""
            }
            [b';', ..] if i < open => {
                first = i + 1;
                ""
            }
            [b';', ..] => {
                ended = true;
                break;
            }
            _ => "",
        };
        i += match skip {
            "" => 1,
            s => text[i + 1..].find(s).map_or(b.len(), |n| n + 1 + s.len()),
        };
    }
    let recursive = Regex::new(r"(?i)\bWITH\s+RECURSIVE\b").expect("a valid pattern");
    let from = if recursive.is_match(&text[first..open]) {
        open
    } else {
        close.unwrap_or(i)
    };
    match at > from {
        false => Some(false),
        true => ended.then_some(at <= i),
    }
}
/// Whether the Zig declaration on 1-based `line` of `text` can be what a name elsewhere names
/// (#469): no function or test holds it, whose local it would be — the blocks around it, told by
/// indentation, lead to a container (the file, a `struct`, an `enum`, a `union`, an `opaque`)
/// before any `fn` or `test`; a block the walk cannot name (`comptime {`) is looked past. From
/// another file (`same_file` false) it is `pub`, since each file is a struct only whose `pub`
/// members leave it. An `export` is a symbol of the whole program, the one thing its name can
/// mean anywhere, and stays.
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
    // The header of each block around the line, innermost first; `) u32 {` closes a header
    // wrapped over the lines above it, which starts back at its indent.
    let (mut depth, mut wrapped) = (indent(target), false);
    for l in lines[..line - 1].iter().rev() {
        let code = uncommented(Kind::Zig, l);
        let t = code.trim();
        if t.is_empty() || indent(l) > depth || (indent(l) == depth && !wrapped) {
            continue;
        }
        depth = indent(l);
        if CONTAINER.is_match(t) {
            break;
        }
        if ZIG_BODY.is_match(t) {
            return false;
        }
        wrapped = t.starts_with(')');
    }
    let open = target.trim_start();
    same_file || open.starts_with("pub ") || open.starts_with("export ")
}
/// Whether 1-based `line` of `lines`, a GraphQL file, declares a field or an enum value: the
/// nearest line above it indented less, past blanks and `#` comments, opens a `type`, an
/// `interface`, an `input` or an `enum`, or an `extend` of one, which declares fields all the
/// same. A selection of an operation or a fragment is the same line one level down elsewhere.
pub fn graphql_member<S: AsRef<str>>(lines: &[S], line: usize) -> bool {
    static OPENER: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"^(?:extend\s+)?(?:type|interface|input|enum)\s").unwrap()
    });
    let Some(target) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let depth = indent(target.as_ref());
    lines[..line - 1]
        .iter()
        .map(AsRef::as_ref)
        .rev()
        .find(|l| {
            let t = l.trim_start();
            !t.is_empty() && !t.starts_with('#') && indent(l) < depth
        })
        .is_some_and(|l| OPENER.is_match(l))
}

/// Line patterns that set the Makefile variable `word` only in addition or for some targets:
/// `X += …`, which make reads as `=` on a variable nothing set before, and a target-specific
/// `release: X := 1.0`, behind `override`, `export` or `private`. Its targets are words and
/// whole references, `$(SRC:.c=.o)`, so the text of `$(error usage: X=1)` is none. `d` falls
/// back to them only when no line of [`def_patterns`] declares the word (#499).
pub fn make_fallback_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    let target = r"(?:[^\s:=#$(){}]+|\$[({][^)}]*[)}])+";
    vec![
        format!(r"^\s*(export\s+|override\s+)?{w}\s*\+="),
        format!(
            r"^{target}(?:\s+{target})*\s*::?\s*((export|override|private)\s+)*{w}\s*(\+|[:?!]{{0,3}})="
        ),
    ]
}
/// When 1-based `line` of a Makefile is a recipe line, a shell command that declares nothing
/// make knows, the line its command starts on: a line that starts with a tab after a rule, until
/// a line that is neither a recipe line, a blank, a comment nor a conditional ends the rule, as
/// GNU make reads it, and the lines a `\` continues it over, which one shell runs. A tab-indented
/// assignment inside an `ifeq` before any rule is make's own, and so is the continuation of one.
pub fn make_recipe_command(text: &str, line: usize) -> Option<usize> {
    let mut in_rule = false;
    // The previous line ended in `\`: where the command it belongs to starts, if it is a recipe.
    let mut continues = None;
    for (i, l) in text.lines().take(line).enumerate() {
        let recipe = match continues {
            Some(recipe) => recipe,
            None if l.starts_with('\t') => in_rule.then_some(i + 1),
            None => {
                let t = l.trim_start();
                let conditional = ["ifeq", "ifneq", "ifdef", "ifndef", "else", "endif"]
                    .iter()
                    .any(|d| {
                        t.strip_prefix(d)
                            .is_some_and(|r| r.is_empty() || r.starts_with([' ', '\t', '(']))
                    });
                if !t.is_empty() && !t.starts_with('#') && !conditional {
                    in_rule = starts_rule(t);
                }
                None
            }
        };
        if i + 1 == line {
            return recipe;
        }
        continues = l.ends_with('\\').then_some(recipe);
    }
    None
}
/// Whether a Makefile line outside a recipe is a rule, `targets: prerequisites`, and not an
/// assignment, `x = a:b` or `x := y`: its first `:` outside a `$(…)` comes before any `=` and is
/// not the start of `:=` or `::=`.
fn starts_rule(line: &str) -> bool {
    let mut depth = 0usize;
    let mut chars = line.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            '$' if matches!(chars.peek(), Some((_, '(' | '{'))) => {
                chars.next();
                depth += 1;
            }
            ')' | '}' if depth > 0 => depth -= 1,
            '=' if depth == 0 => return false,
            ':' if depth == 0 => {
                let rest = line[i..].trim_start_matches(':');
                return !rest.starts_with('=');
            }
            _ => {}
        }
    }
    false
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
        // Not only the `class` line: black writes one base to a line under it (#100). Such a
        // line holds names and commas alone, and the caller reads the header it belongs to.
        Kind::Python => Some(format!(
            r#"^\s*class\s+\w+\s*\(.*\b({any})\b|^\s+[\w.\[\]"', ]*\b({any})\b[\w.\[\]"', ]*(?:\)\s*:)?\s*(?:#.*)?$"#
        )),
        // Not only the header line: prettier writes `extends Base` on a line of its own.
        Kind::TsJs => Some(format!(r"\b(?:extends|implements)\s[^;]*\b({any})\b")),
        _ => None,
    }
}
