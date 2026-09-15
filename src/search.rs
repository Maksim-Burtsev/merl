//! Project-wide grep (ripgrep's own crates) and the regex rules behind "go to definition",
//! the symbol list and the word under the cursor.
//!
//! There is no language server here: a definition is whatever a per-kind line pattern says it
//! is, and everything merl does not know about falls back to a whole-word search.

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

/// Lines that look like a top-level declaration in code. Group 2 swallows a Go method receiver
/// (`func (i Invoice) Total()`), and the group after it a Kotlin extension receiver
/// (`fun String.slug()`).
///
/// `static` and `const` are a branch of their own, with the name in `cname`: they are followed by
/// the name in Rust, JavaScript and Kotlin (`static COUNT: u32`, `const val LIMIT = 10`) but by a
/// type in Java, where `static final int LIMIT = 10;` would otherwise be listed as `final`.
/// Requiring the `:` or `=` that follows the name keeps a Java field or method off the list.
pub const SYMBOL_PATTERN: &str = r#"^\s*(?:(?:export|default|declare|async|pub(?:\([a-z]+\))?|static|unsafe|abstract|const|extern(?:\s+"[^"]*")?|public|protected|private|internal|final|open|sealed|data|annotation|companion|inner|value|enum|suspend|override|inline|operator|infix)\s+)*(?:(def|class|func|function\*?|type|fn|fun|struct|enum|impl|trait|interface|mod|module|object|record|typealias|union|macro_rules!|namespace)(?:<[^>]*>)?\s+(\([^)]*\)\s*)?(?:[A-Za-z_][\w.]*\.)?(?P<name>[A-Za-z_]\w*)|(?:static|const)\s+(?:(?:val|var)\s+)?(?P<cname>[A-Za-z_]\w*)\s*[:=])"#;

/// The head of a SQL `CREATE` statement, up to the name it declares: the optional `OR REPLACE`,
/// the modifiers that can sit before the object word, the object itself and `IF NOT EXISTS`.
/// A macro, so the symbol pattern below and [`def_patterns`] share one spelling of it.
macro_rules! sql_create {
    () => {
        r"(?i)^\s*CREATE\s+(?:OR\s+REPLACE\s+)?(?:(?:TEMP|TEMPORARY|UNLOGGED|MATERIALIZED|UNIQUE)\s+)*(?:TABLE|VIEW|INDEX|FUNCTION|PROCEDURE|TRIGGER|TYPE|SCHEMA|SEQUENCE|DOMAIN|EXTENSION|DATABASE|ROLE|USER|ENUM)(?:\s+IF\s+NOT\s+EXISTS)?\s+"
    };
}

/// A name in a `CREATE` statement, as written: bare, `"quoted"` or `` `backticked` ``, and
/// optionally schema-qualified (`public.orders`).
const SQL_NAME: &str = r#"(?:"[^"]+"|`[^`]+`|\w+)"#;

/// The SQL half of [`SYMBOLS`]: every `CREATE`d object, listed under its written name.
const SQL_CREATE_SYMBOL: &str = concat!(
    sql_create!(),
    r#"(?P<name>(?:"[^"]+"|`[^`]+`|\w+)(?:\.(?:"[^"]+"|`[^`]+`|\w+))?)"#
);

/// What `D` lists: a line pattern and the kind of file it runs over (`None`: every file).
/// The infrastructure patterns need their kind, since a Makefile target and a YAML key are the
/// same shape. [`symbol_name`] reads the listed name from the named groups.
pub const SYMBOLS: &[(Option<Kind>, &str)] = &[
    (None, SYMBOL_PATTERN),
    // `name ()`, the form without the `function` keyword: a `function name` line is already
    // listed by [`SYMBOL_PATTERN`], and requiring no keyword here keeps it off the list twice.
    (Some(Kind::Shell), r"^\s*(?P<name>[A-Za-z_]\w*)\s*\(\s*\)"),
    // Every `CREATE` object, with the name as written, schema and quotes included. CTEs are a
    // query's own scaffolding, not a symbol of the project, so they are left out.
    (Some(Kind::Sql), SQL_CREATE_SYMBOL),
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

/// A file kind with navigation rules of its own. Told by the file name, since a `Makefile` or a
/// `Dockerfile` has no extension to go by; one kind can span extensions (`.tsx` finds `.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        (_, "py") => Kind::Python,
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
        // Name-only, like every other kind: a shebang-only script with no extension is left to
        // the whole-word fallback, since `kind_of` never reads a file.
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
/// line. Files that cannot be read are skipped — this is a viewer, not a linter.
pub fn grep_project(
    root: &Path,
    files: &[PathBuf],
    pattern: &str,
    whole_word: bool,
    smart_case: bool,
    current: Option<&Path>,
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
        let _ = searcher.search_path(
            &matcher,
            root.join(rel),
            Collect {
                path: rel,
                hits: &mut hits,
            },
        );
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

/// Line patterns that declare `word` in a file of `kind`, or an empty list when there is no rule
/// for it (the caller then falls back to a whole-word search). In Terraform `word` is the dotted
/// address under the cursor.
pub fn def_patterns(kind: Kind, word: &str) -> Vec<String> {
    let w = regex::escape(word);
    match kind {
        // The optional `: Type` group covers annotated assignments (`X: Final[int] = 1`).
        Kind::Python => vec![
            format!(r"^\s*(def|class)\s+{w}\b"),
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
            let mods = r"^\s*(?:(?:public|private|protected|static|readonly|abstract|override|async|get|set)\s+)*";
            vec![
                format!(
                    r"{pre}(?:function\*?|class|interface|type|(?:const\s+)?enum|namespace|module)\s+{w}\b"
                ),
                // Arrow functions assigned to a name land here too.
                format!(r"{pre}(?:const|let|var)\s+{w}\b"),
                // A class or object-literal method: `foo(` at the end of the line, or
                // `foo(..) {`. A `;` on the line means it was a call statement.
                format!(r"{mods}{w}\s*(?:<[^>]*>)?\((?:[^;]*\{{)?\s*$"),
                // A property holding a function: `foo = () =>`, `foo: async (x) =>`,
                // `foo: function`.
                format!(
                    r"{mods}{w}\s*[?!]?\s*(?::[^=]*)?[=:]\s*(?:async\s+)?(?:function\b|\(|[\w$]+\s*=>)"
                ),
            ]
        }
        Kind::Jvm => {
            // Everything that can stand before a declaration in either language: annotations,
            // Java's access and class modifiers, Kotlin's own.
            let mods = concat!(
                r"^\s*(?:@[\w.]+(?:\([^)]*\))?\s+)*",
                r"(?:(?:public|protected|private|internal|static|final|abstract|sealed|non-sealed",
                r"|strictfp|synchronized|native|default|transient|volatile|open|data|value|inner",
                r"|annotation|companion|enum|const|lateinit|expect|actual|suspend|override|inline",
                r"|operator|infix|tailrec|external|reified)\s+)*"
            );
            // A return type: a primitive, or a name with a capital in it. Java names its types
            // that way, and requiring one keeps `return parse(x);` from looking like a method.
            let ret = r"(?:void|int|long|short|byte|char|boolean|float|double|[\w.]*[A-Z][\w.]*)";
            vec![
                format!(
                    r"{mods}(?:class|interface|enum|record|@interface|object|typealias)\s+{w}\b"
                ),
                // Kotlin: a function, with its generics and, for an extension, its receiver.
                format!(r"{mods}fun\s+(?:<[^>]*>\s*)?(?:[\w.]+\.)?{w}\s*\("),
                format!(r"{mods}(?:val|var)\s+{w}\b"),
                // Java: a method or constructor with a body. The line ends with the brace that
                // opens it, and what may stand before the name rules out `if (parse(x)) {`.
                format!(r"^\s*[\w.<>\[\],?@\s]*\b{w}\s*\([^;]*\)\s*(?:throws [\w.,\s]+)?\{{\s*$"),
                // Java: an abstract or interface method, and a field: a return type, the name,
                // and the `(`, `;` or `=` that follows it.
                format!(
                    r"{mods}(?:<[^>]*>\s*)?{ret}(?:<[^>]*>)?(?:\[\])*(?:\.\.\.)?\s+{w}\s*[(;=]"
                ),
            ]
        }
        // Ruby declares everything on one line. A constant lives indented inside its class, so
        // the assignment rule is not anchored at column zero as Python's is, and it takes the
        // `@`/`@@` of an instance or class variable with it. The Rails-style DSL (`scope`,
        // `has_many`, `define_method`) is left to the whole-word fallback.
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
        // line is any column of any table, so `d` falls back to the whole-word search.
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
        .or_else(|| group("name").or_else(|| group("cname")).map(str::to_owned))
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
        grep_project(dir, files, pat, word, smart, None).unwrap()
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
        // A plain field is not a declaration the rules know; the caller falls back.
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

object Registry {
    val all = listOf<Order>()
}

enum class Status {
    OPEN,
}

typealias Rows = List<Order>

const val LIMIT = 10
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
        assert_eq!(d("Store"), [21]);
        assert_eq!(d("save"), [22], "an interface method has no body");
        assert_eq!(d("Point"), [25]);
        assert_eq!(d("Status"), [27]);
        assert_eq!(d("rows"), Vec::<usize>::new(), "a parameter falls back");
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
        assert_eq!(d("Registry"), [17]);
        assert_eq!(d("all"), [18]);
        assert_eq!(d("Status"), [21]);
        assert_eq!(d("Rows"), [25]);
        assert_eq!(d("LIMIT"), [27]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn java_and_kotlin_find_each_other() {
        let (dir, files) = scratch("jvm", &[("Invoice.java", JAVA), ("app.kt", KT)]);
        let pat = def_patterns(Kind::Jvm, "Store").join("|");
        assert_eq!(
            lines(&grep(&dir, &files, &pat, false, false)),
            [("Invoice.java".into(), 21)]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    const RB: &str = r#"module Billing
  LIMIT = 10

  class Invoice
    attr_accessor :total
    attr_reader :id, :customer

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

    def empty?
      @rows.empty?
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
        // The accessor, the setter and the assignment behind it.
        assert_eq!(d("total"), [5, 17, 18]);
        assert_eq!(d("customer"), [6], "second in the `attr_reader` list");
        assert_eq!(d("id"), [6, 9]);
        assert_eq!(d("initialize"), [8]);
        assert_eq!(d("parse"), [13], "`def self.parse`");
        // `?` is not part of the word under the cursor, and `@rows.empty?` is a call.
        assert_eq!(d("empty"), [21]);
        assert_eq!(d("blank"), [25]);
        assert_eq!(d("rows"), [10]);
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
        // `customers` is only ever used, never created: the caller falls back.
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
            // A shebang-only script: `kind_of` goes by the name, so `d` falls back.
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
            ("public final class Invoice {", Some("Invoice")),
            ("interface Store {", Some("Store")),
            ("record Point(int x, int y) {}", Some("Point")),
            ("enum Status {", Some("Status")),
            ("data class Order(val id: String)", Some("Order")),
            ("enum class Status {", Some("Status")),
            ("annotation class Json", Some("Json")),
            ("    suspend fun load(id: String): Order {", Some("load")),
            ("fun String.slug(): String = lowercase()", Some("slug")),
            ("object Registry {", Some("Registry")),
            ("typealias Rows = List<Order>", Some("Rows")),
            // A method without a keyword in front, as in TypeScript: the regex cannot tell it
            // from a call. So is a field, and `companion object` has no name of its own.
            ("    public int total() {", None),
            ("    companion object {", None),
            ("module Billing", Some("Billing")),
            ("class Invoice < Base", Some("Invoice")),
            ("    def self.parse(text)", Some("parse")),
            ("    def total=(value)", Some("total")),
            // A Ruby constant and the `attr_*` methods have no keyword to go by, and one
            // `attr_accessor` line can declare several names.
            ("  LIMIT = 10", None),
            ("    attr_accessor :total", None),
            ("module.exports = helpers;", None),
            // A `static` or `const` line is listed when the name follows the keyword, as in
            // Rust, JavaScript and Kotlin -- not when a Java type stands in between.
            ("const val LIMIT = 10", Some("LIMIT")),
            ("    private static final int LIMIT = 10;", None),
            (
                "    static Map<String, Integer> compute(Map<String, Integer> rows) {",
                None,
            ),
            ("    static final Invoice EMPTY = new Invoice(0);", None),
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
