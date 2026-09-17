//! Project-wide grep (ripgrep's own crates) and the regex rules behind "go to definition",
//! the symbol list and the word under the cursor.
//!
//! There is no language server here: a definition is whatever a per-kind line pattern says it
//! is, searched in the project first and then in the standard library and the installed
//! dependencies the toolchain on this machine knows about.

use std::ops::Range;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use grep_regex::RegexMatcherBuilder;
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

/// The Ruby half of [`SYMBOLS`]: a method, including the `self.` form and the `name=` setter, and
/// a class or module under the namespace it is written with. A constant and the names an
/// `attr_accessor` line declares stay off the list: there is no keyword to go by, and one such
/// line can declare several.
const RUBY_SYMBOL: &str =
    r"^\s*(?:def\s+(?:self\.|[A-Z]\w*\.)?|(?:class|module)\s+(?:[\w:]*::)?)(?P<name>[A-Za-z_]\w*)";

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

/// Whether [`SYMBOL_PATTERN`] is read from a file of `kind`. Java, Kotlin and Ruby have rows of
/// their own in [`SYMBOLS`], written for what those languages declare and how they name it, so
/// reading the all-language pattern over them too would list a declaration twice.
pub fn shared_symbols(kind: Option<Kind>) -> bool {
    !matches!(kind, Some(Kind::Jvm | Kind::Ruby))
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
        };
        let _ = match unsaved.filter(|_| current == Some(rel.as_path())) {
            Some(text) => searcher.search_slice(&matcher, text, sink),
            None => searcher.search_path(&matcher, root.join(rel), sink),
        };
    }
    hits.sort_by_cached_key(|h| (current != Some(h.path.as_path()), h.path.clone(), h.line));
    Ok(hits)
}

/// Collects one `Hit` per matching line, stopping the whole search at [`MAX_HITS`].
struct Collect<'a> {
    path: &'a Path,
    hits: &'a mut Vec<Hit>,
}

impl Sink for Collect<'_> {
    type Error = std::io::Error;

    fn matched(&mut self, _searcher: &Searcher, m: &SinkMatch<'_>) -> std::io::Result<bool> {
        self.hits.push(Hit {
            path: self.path.to_path_buf(),
            line: m.line_number().unwrap_or(0) as usize,
            text: String::from_utf8_lossy(m.bytes()).trim_end().to_string(),
        });
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
            format!(r"^func\s+(\([^)]*\)\s*)?{w}\("),
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
                format!(r"{mods}{w}\s*(?:<[^>]*>)?\((?:[^;]*\{{\s*\}}?)?\s*$"),
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
                    r"{mods}{w}\??\s*(?:<[^>]*>)?\((?:\s*|\s*(?:\.\.\.)?[\w$]+\??\s*:[^;{{}}]*)\)\s*:[^;{{}}=]*;?\s*$"
                ),
            ]
        }
        Kind::Rust
        | Kind::Jvm
        | Kind::Ruby
        | Kind::Shell
        | Kind::Sql
        | Kind::Make
        | Kind::Terraform
        | Kind::Docker
        | Kind::Yaml => return None,
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
    let sep = if kind == Kind::Rust { "::" } else { "." };
    let lines: Vec<&str> = text.lines().collect();
    let target = *lines.get(line.checked_sub(1)?)?;
    if let Some(c) = RECEIVER.captures(target).filter(|_| kind == Kind::Go) {
        return Some(format!("{}{sep}{name}", &c[1]));
    }
    let indent = |s: &str| s.len() - s.trim_start().len();
    let mut depth = indent(target);
    let mut names = vec![name.to_owned()];
    for l in lines[..line - 1].iter().rev() {
        if depth == 0 {
            break;
        }
        let t = l.trim_start();
        if t.is_empty()
            || indent(l) >= depth
            || ["#", "//", "/*", "*"].iter().any(|c| t.starts_with(c))
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
        // Java, Kotlin and Ruby have no roots yet: the JDK and Gradle caches, and a gem path,
        // are their own lookups. `d` stays inside the project for them, as for the rest.
        Kind::Jvm
        | Kind::Ruby
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
        // Nothing to bind without roots to resolve an `import` or a `require` against.
        Kind::Jvm
        | Kind::Ruby
        | Kind::Shell
        | Kind::Sql
        | Kind::Make
        | Kind::Terraform
        | Kind::Docker
        | Kind::Yaml => {}
    }
    out
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
    let e = uncommented(kind, expr);
    let e = e.trim().trim_end_matches(';').trim_end();
    // What comes after the bracket that opens at `open` must be nothing, or the next lines.
    let ends = |open: usize| close_of(kind, e, open).is_none_or(|end| e[end..].trim().is_empty());
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
    if NAME.is_match(e) && !matches!(e, "None" | "null" | "undefined" | "nil" | "this" | "self") {
        return Value::Name(e.to_owned());
    }
    Value::Unknown
}

/// The declarations of `name` visible on 1-based `line` of `text`, a file of `kind`, with what
/// each gives it. Every one counts, the enclosing scopes' too: the caller trusts them only when
/// they agree.
///
/// - Python: a name belongs to its function, so its parameters and every binding in the body
///   count, before the cursor or after it, then the enclosing functions', then the module's.
///   Nested functions and classes are scopes of their own. A comprehension or a `lambda` counts
///   on the cursor line only. `self` / `cls` is the class a method sits in.
/// - TypeScript and Go: `const`, `let` and `:=` belong to their block, so the declarations above
///   the cursor count, in the blocks around it, told by indentation: a function's parameters, a Go
///   receiver, the statements at each block's level. `this` is the class around it, unless a
///   `function` or an object literal comes first.
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

    let mut out = Vec::new();
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
            if t.is_empty() {
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
            let value = if unknown.is_match(t) || (i == at && inline.is_match(t)) {
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
    let this = kind == Kind::TsJs && name == "this";
    let mut out = Vec::new();
    if !this {
        opener_bindings(kind, &uncommented(kind, lines[at]), at + 1, name, &mut out);
    }
    let mut depth = indent(lines[at]);
    let mut i = at;
    while i > 0 {
        i -= 1;
        let code = uncommented(kind, lines[i]);
        let t = code.trim();
        let ind = indent(lines[i]);
        if t.is_empty() || comment(kind, t) || ind > depth {
            continue;
        }
        if ind == depth {
            if !this {
                statement_bindings(kind, t, i + 1, name, &mut out);
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
        let header: Vec<&str> = lines[i..=end].iter().map(|l| l.trim()).collect();
        let header = uncommented(kind, &header.join("\n"));
        depth = ind.min(indent(lines[i]));
        if !this {
            opener_bindings(kind, &header, i + 1, name, &mut out);
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
    let object = before.is_some_and(|b| {
        b.ends_with(['=', '(', '[', ',', '?', ':', '|', '&'])
            || b.ends_with("return")
            || b.ends_with("default")
    });
    (object || names(header, "function")).then_some(Value::Unknown)
}

/// The bindings of `name` a block's header makes for the lines inside it: the parameters of a
/// function, a method or an arrow, a Go receiver and named results, the variables of a loop, a
/// `catch` or a Go `if x := …;`.
fn opener_bindings(kind: Kind, header: &str, line: usize, name: &str, out: &mut Vec<Binding>) {
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
    match kind {
        Kind::TsJs => {
            if TS_LOOP
                .captures_iter(header)
                .any(|c| c.iter().skip(1).flatten().any(|m| names(m.as_str(), name)))
            {
                unknown(out);
            }
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
                    ts_params(&header[open + 1..close - 1], line, name, out);
                }
            }
            let arrow = Regex::new(&format!(r"(?:^|[^\w$.]){}\s*=>", regex::escape(name)))
                .expect("an escaped name keeps the pattern valid");
            if arrow.is_match(header) {
                unknown(out);
            }
        }
        Kind::Go => {
            if GO_ASSIGN.captures_iter(header).any(|c| names(&c[1], name)) {
                unknown(out);
            }
            if let Some(c) = GO_FUNC.captures_iter(header).last() {
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
                }
            }
        }
        _ => {}
    }
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
                .is_some_and(|rest| names(rest, name))
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
            !t.is_empty() && !comment(kind, t) && indent(lines[i]) <= base
        })
        .unwrap_or(lines.len());
    start..end.max(start)
}

/// The declarations of the field `name` of the class, interface or struct declared on 1-based
/// `decl` of `text`:
/// - Python: `name: T` or `name = …` in the class body, `self.name: T = …` or `self.name = …` in
///   its methods;
/// - TypeScript: a member `name: T` or `name = …` behind any modifiers, a constructor parameter
///   with one (`private name: T`), `this.name = …`;
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
                }
            }
        }
        Kind::TsJs => {
            let member = rule(format!(
                r"^(?:(?:public|private|protected|readonly|static|declare|override|abstract|accessor)\s+)*{n}\s*[?!]?\s*(?::\s*([^=;]+?))?\s*(?:=\s*(.+?))?\s*;?$"
            ));
            let param = rule(format!(
                r"(?:^|[(,])\s*(?:(?:public|private|protected|readonly|override)\s+)+{n}\s*\??\s*:\s*([^,)=]+)"
            ));
            let assigned = rule(format!(r"^this\.{n}\s*=\s*([^=].*?)\s*;?$"));
            for i in body {
                let (code, ind) = (uncommented(kind, lines[i]), indent(lines[i]));
                let t = code.trim();
                if let Some(c) = member.captures(t).filter(|_| ind == base) {
                    push(
                        i,
                        match (c.get(1), c.get(2)) {
                            (Some(ty), _) => Value::Type(ty.as_str().trim().to_owned()),
                            (None, Some(v)) => value_of(kind, v.as_str()),
                            (None, None) => Value::Unknown,
                        },
                    );
                } else if let Some(c) = param.captures(t) {
                    push(i, Value::Type(c[1].trim().to_owned()));
                } else if let Some(c) = assigned.captures(t).filter(|_| ind > base) {
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

/// What a call of the function declared on 1-based `decl` of `text` gives: its declared return
/// type (Python `-> T`, TypeScript `): T`, Go's result or its first one), or in TypeScript without
/// one, the `new T()` that every `return` of the body, or an arrow's expression, agrees on.
pub fn returns(kind: Kind, text: &str, decl: usize) -> Option<Value> {
    static PY_DEF: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*(?:async\s+)?def\s+\w+\s*\(").unwrap());
    static PY_RETURN: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*->\s*(.+?)\s*:\s*(?:#.*)?$").unwrap());
    static TS_FUNCTION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:export|default|declare|async)\s+)*(?:function\*?\s*[\w$]*|(?:const|let|var)\s+[\w$]+\s*(?::[^=]+)?=\s*(?:async\s+)?(?:function\*?\s*[\w$]*)?)\s*(?:<[^>]*>)?\s*\(").unwrap()
    });
    static TS_RETURN: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*:\s*(.+?)\s*(?:\{|=>)").unwrap());
    static GO_FUNC: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^func\s+[A-Za-z_]\w*\s*(?:\[[^\]]*\])?\s*\(").unwrap()
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
        Kind::Python => PY_RETURN
            .captures(after)
            .map(|c| Value::Type(c[1].to_owned())),
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
                        .filter_map(|l| l.trim().strip_prefix("return "))
                        .map(|e| value_of(kind, e))
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
        !t.is_empty() && !comment(kind, t) && indent(l) < depth
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

/// A grep for a type declaration that names one of `names` as a base: `class X(Base)` in Python,
/// `class X extends Base`, `class X implements Base` and `interface I extends Base` in
/// TypeScript. `None` for Go, whose types implement an interface by carrying its methods and
/// never name it. A header wrapped over several lines keeps its bases off the line the grep
/// matches and is missed, as [`bases`] misses it.
pub fn subtype_patterns(kind: Kind, names: &[String]) -> Option<String> {
    let any: Vec<String> = names.iter().map(|n| regex::escape(n)).collect();
    let any = any.join("|");
    match kind {
        Kind::Python => Some(format!(r"^\s*class\s+\w+\s*\(.*\b({any})\b")),
        Kind::TsJs => Some(format!(
            r"\b(?:class|interface)\s.*\b(?:extends|implements)\b.*\b({any})\b"
        )),
        _ => None,
    }
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

/// Whether `path` is (in) the module spelled by `parts`: every part is a directory or file
/// stem on it, in order. A package directory carries a version (`regex-1.11.1`,
/// `toml@v1.2.3`), a Go module escapes upper case (`!burnt!sushi`) and a crate name spells
/// `_` as `-`: those are ignored.
pub fn in_module(path: &Path, parts: &[String]) -> bool {
    let norm = |s: &str| -> String {
        let s = s.split('@').next().unwrap_or(s);
        // `fs.d.ts`, `python3.13`, `github.com`: the stem before every extension.
        let s = s.split('.').next().unwrap_or(s);
        // `name-1.2.3`: the version after the first `-` followed by a digit.
        let s = s
            .match_indices('-')
            .find(|(i, _)| s[i + 1..].starts_with(|c: char| c.is_ascii_digit()))
            .map_or(s, |(i, _)| &s[..i]);
        s.replace('!', "").replace('-', "_").to_ascii_lowercase()
    };
    let mut want = parts.iter().map(|p| norm(p)).peekable();
    for c in path.components() {
        let c = c.as_os_str().to_string_lossy();
        if want.peek().is_some_and(|w| *w == norm(&c)) {
            want.next();
        }
    }
    want.peek().is_none()
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

    const TS_MEMBERS: &str = "export interface Repo {\n  deleteUser(id: string): Promise<void>;\n  findUser?<T>(id: string): T | undefined\n  onChange: (id: string) => void;\n  name: string;\n}\n\nexport abstract class Base {\n  abstract deleteUser(id: string): Promise<void>;\n  get size(): number;\n}\n\nconst deleteUser = (id: string) => id;\nfindUser(id);\nrun(x).then((y): void => y);\nconst n = cond ? findUser(a) : b;\nexport const helpers = {\n  deleteUser(id) {\n    return id;\n  },\n};\nappend(target, visitor ? visitNode(s) : s);\nlog(\"(while reading XRef): \" + e);\ndeclare class Emitter {\n  on(event: string, cb: (x: T) => void): this;\n  append(...items: string[]): void;\n}\nclass Session {\n  close(): void {}\n}\nnoop(() => {})\n";

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
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn go_members_need_a_receiver() {
        let go = "package main\n\ntype Repo struct{}\n\nfunc (r *Repo[T]) Delete(id int) {}\n\nfunc (Repo) Find(id int) {}\n\nfunc Delete(id int) {}\n\nfunc main() {\n\tDelete := 1\n}\n";
        let (dir, files) = scratch("go-members", &[("repo.go", go)]);
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
        // A spread and a range are no member access.
        assert_eq!(q("f(...this.repo.find())", 15), ["this", "repo"]);
        assert_eq!(q("for i in 0..v.len() {", 14), ["v"]);
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
        // A parameter over a multi-line signature, and the module's `repo` above.
        assert_eq!(
            at(14, "repo"),
            [
                (9, ty("UserRepository")),
                (3, Value::Call("UserRepository".into()))
            ]
        );
        assert_eq!(at(14, "cache"), [(9, ty("\"Cache | None\""))]);
        assert_eq!(at(14, "local"), [(15, ty("Optional[Repo]"))]);
        assert_eq!(at(24, "self"), [(21, Value::Class(6))]);
        // A static method's first parameter is no `self`, and an unannotated one is unknown.
        assert_eq!(at(19, "first"), [(18, Value::Unknown)]);
        assert_eq!(at(19, "second"), [(18, ty("int"))]);
        assert_eq!(at(23, "item"), [(22, Value::Unknown)]);
        // A comprehension or a lambda binds on its own line only.
        assert_eq!(at(25, "r"), [(25, Value::Unknown)]);
        assert_eq!(at(24, "r"), []);
        assert_eq!(
            at(26, "repo"),
            [
                (26, Value::Unknown),
                (3, Value::Call("UserRepository".into()))
            ]
        );
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
        assert_eq!(
            at(6, "repo"),
            [
                (5, call("UserRepository")),
                (2, call("AuditLog")),
                (9, call("make_repo"))
            ]
        );
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
        assert_eq!(at(18, "item"), [(17, Value::Unknown)]);
        assert_eq!(at(28, "a"), [(20, Value::Unknown)]);
        assert_eq!(at(21, "x"), [(21, ty("Item"))]);
        // Only the blocks around the cursor: the inner `repo` is gone below its arrow.
        assert_eq!(
            at(36, "repo"),
            [(35, new("UserRepository")), (33, new("AuditLog"))]
        );
        assert_eq!(at(38, "repo"), [(33, new("AuditLog"))]);
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
        assert_eq!(at(12, "item"), [(11, Value::Unknown)]);
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
        let py = "def make_repo() -> UserRepository:\n    return UserRepository()\n\nasync def connect(\n    url: str,\n) -> \"Session\":\n    ...\n\ndef untyped():\n    return Repo()\n";
        assert_eq!(returns(Kind::Python, py, 1), Some(ty("UserRepository")));
        assert_eq!(returns(Kind::Python, py, 4), Some(ty("\"Session\"")));
        assert_eq!(returns(Kind::Python, py, 9), None);
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
    fn the_shared_pattern_skips_the_kinds_with_rows_of_their_own() {
        // Java, Kotlin and Ruby are listed from their own rows only, so nothing is listed twice
        // and `def self.parse` is not `self`.
        assert!(!shared_symbols(Some(Kind::Jvm)));
        assert!(!shared_symbols(Some(Kind::Ruby)));
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
