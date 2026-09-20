//! The line patterns `d` looks a declaration up with, per kind and per word, and the
//! reason a candidate is offered under.

use super::*;

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
