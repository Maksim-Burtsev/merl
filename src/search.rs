//! Project-wide grep (ripgrep's own crates) and the regex rules behind "go to definition",
//! the symbol list and the word under the cursor.
//!
//! There is no language server here: a definition is whatever a per-language line pattern says
//! it is, and everything merl does not know about falls back to a whole-word search.

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Context, Result};
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{BinaryDetection, Searcher, SearcherBuilder, Sink, SinkMatch};
use regex::Regex;

/// ponytail: a hard stop instead of a streaming picker, taken in file order, so the picker
/// title says "first N" when it hits. Raise it if a picker over the whole result set ever
/// becomes the point.
pub const MAX_HITS: usize = 5_000;

/// Lines that look like a top-level declaration. The name is group 3; group 2 swallows a Go
/// method receiver (`func (i Invoice) Total()`).
pub const SYMBOL_PATTERN: &str = r#"^\s*(?:(?:export|default|async|pub(?:\([a-z]+\))?|static|unsafe|abstract|const|extern(?:\s+"[^"]*")?)\s+)*(def|class|func|function|type|fn|struct|enum|impl|trait|interface|mod|const|static|union|macro_rules!)(?:<[^>]*>)?\s+(\([^)]*\)\s*)?([A-Za-z_]\w*)"#;

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

/// Line patterns that declare `word` in a `.{ext}` file, or an empty list for a language merl
/// has no rules for (the caller then falls back to a whole-word search).
pub fn def_patterns(ext: &str, word: &str) -> Vec<String> {
    let w = regex::escape(word);
    match ext {
        // The optional `: Type` group covers annotated assignments (`X: Final[int] = 1`).
        "py" => vec![
            format!(r"^\s*(def|class)\s+{w}\b"),
            format!(r"^{w}\s*(:[^=]*)?="),
        ],
        "go" => vec![
            format!(r"^func\s+(\([^)]*\)\s*)?{w}\("),
            format!(r"^type\s+{w}\b"),
            format!(r"^(var|const)\s+{w}\b"),
            format!(r"^\s*{w}\s*:="),
        ],
        // `impl X` is a use of `X`, not its definition, so it is left out on purpose.
        "rs" => {
            let vis = r#"^\s*(?:(?:pub(?:\([^)]*\))?|async|unsafe|const|extern(?:\s+"[^"]*")?|default)\s+)*"#;
            vec![
                format!(r"{vis}(?:fn|struct|enum|union|trait|type|const|static|mod)\s+{w}\b"),
                format!(r"^\s*macro_rules!\s+{w}\b"),
                format!(r"^\s*let\s+(?:mut\s+)?{w}\b"),
            ]
        }
        _ => Vec::new(),
    }
}

/// The declared name on a line matched by [`SYMBOL_PATTERN`].
pub fn symbol_name(line: &str) -> Option<&str> {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(SYMBOL_PATTERN).expect("SYMBOL_PATTERN is valid"))
        .captures(line)
        .and_then(|c| c.get(3))
        .map(|m| m.as_str())
}

/// The `[A-Za-z0-9_]` run at byte offset `col`, or the one that ends there when the cursor sits
/// right after a word.
pub fn word_at(line: &str, col: usize) -> Option<(Range<usize>, &str)> {
    let b = line.as_bytes();
    let word = |i: usize| b[i].is_ascii_alphanumeric() || b[i] == b'_';
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
    Some((start..end, &line[start..end]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PY: &str = "class Invoice:\n    def total(self):\n        return 0\n\n\ndef parse(t):\n    return Invoice()\n\n\nDEFAULT_LIMIT = 10\ntotal_foobar = 1\nprint(total_foobar, DEFAULT_LIMIT)\nNAME_RE: Final[re.Pattern[str]] = re.compile(r\"x\")\n";
    const RS: &str = "pub struct Order<T> {\n    items: Vec<T>,\n}\n\nimpl<T> Order<T> {\n    pub fn sum(&self) -> u32 { 0 }\n}\n\npub(crate) const MAX_ORDERS: usize = 10;\n\npub async fn parse_order(s: &str) -> Order<u8> {\n    let mut order = Order { items: vec![] };\n    order.items.push(1);\n    order\n}\n\nmacro_rules! order {\n    () => {};\n}\n\nmod orders;\n";
    const GO: &str = "package main\n\ntype Invoice struct{}\n\nfunc (i Invoice) Total() int { return 0 }\n\nfunc Parse(s string) Invoice { return Invoice{} }\n\nconst Limit = 10\n\nfunc main() {\n\tinv := Parse(\"x\")\n}\n";

    /// A throwaway project on disk; grep needs real files.
    fn project(tag: &str) -> (PathBuf, Vec<PathBuf>) {
        let dir = std::env::temp_dir().join(format!("merl-grep-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.py"), PY).unwrap();
        std::fs::write(dir.join("b.go"), GO).unwrap();
        std::fs::write(dir.join("c.rs"), RS).unwrap();
        (
            dir,
            vec![
                PathBuf::from("a.py"),
                PathBuf::from("b.go"),
                PathBuf::from("c.rs"),
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

    #[test]
    fn python_def_patterns_find_declarations_only() {
        let (dir, files) = project("py");
        let py = files[..1].to_vec();

        let pat = def_patterns("py", "total").join("|");
        // The `def` line, not the `total_foobar` assignment.
        assert_eq!(
            lines(&grep(&dir, &py, &pat, false, false)),
            [("a.py".into(), 2)]
        );

        let pat = def_patterns("py", "parse").join("|");
        assert_eq!(
            lines(&grep(&dir, &py, &pat, false, false)),
            [("a.py".into(), 6)]
        );

        // `^W\s*(:[^=]*)?=` catches module constants, annotated or not.
        let pat = def_patterns("py", "DEFAULT_LIMIT").join("|");
        assert_eq!(
            lines(&grep(&dir, &py, &pat, false, false)),
            [("a.py".into(), 10)]
        );
        let pat = def_patterns("py", "NAME_RE").join("|");
        assert_eq!(
            lines(&grep(&dir, &py, &pat, false, false)),
            [("a.py".into(), 13)]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn go_def_patterns_cover_receivers_types_and_short_vars() {
        let (dir, files) = project("go");
        let go = files[1..2].to_vec();
        for (word, line) in [("Invoice", 3), ("Total", 5), ("Parse", 7), ("Limit", 9)] {
            let pat = def_patterns("go", word).join("|");
            assert_eq!(
                lines(&grep(&dir, &go, &pat, false, false)),
                [("b.go".into(), line)],
                "{word}"
            );
        }
        // `inv := Parse("x")` is the closest thing Go has to a definition of `inv`.
        let pat = def_patterns("go", "inv").join("|");
        assert_eq!(
            lines(&grep(&dir, &go, &pat, false, false)),
            [("b.go".into(), 12)]
        );
        assert!(def_patterns("txt", "inv").is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rust_def_patterns_cover_items_behind_prefixes_and_lets() {
        let (dir, files) = project("rs");
        let rs = files[2..].to_vec();
        for (word, line) in [
            ("Order", 1), // the struct, not the `impl` block or the `Order { .. }` literal
            ("sum", 6),
            ("MAX_ORDERS", 9),
            ("parse_order", 11),
            ("orders", 21),
        ] {
            let pat = def_patterns("rs", word).join("|");
            assert_eq!(
                lines(&grep(&dir, &rs, &pat, false, false)),
                [("c.rs".into(), line)],
                "{word}"
            );
        }
        // Both the `let mut` binding and the macro: the caller shows a picker.
        let pat = def_patterns("rs", "order").join("|");
        assert_eq!(
            lines(&grep(&dir, &rs, &pat, false, false)),
            [("c.rs".into(), 12), ("c.rs".into(), 17)]
        );
        std::fs::remove_dir_all(&dir).unwrap();
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

    #[test]
    fn symbol_names_come_from_the_third_group() {
        assert_eq!(
            symbol_name("def render(inv: Invoice) -> str:"),
            Some("render")
        );
        assert_eq!(symbol_name("    def total(self) -> int:"), Some("total"));
        assert_eq!(
            symbol_name("pub fn wrap_line(s: &str) {"),
            Some("wrap_line")
        );
        assert_eq!(symbol_name("pub(crate) struct Hit {"), Some("Hit"));
        assert_eq!(
            symbol_name("export async function parse(x) {"),
            Some("parse")
        );
        assert_eq!(symbol_name("export default class Foo {"), Some("Foo"));
        assert_eq!(symbol_name("    return total"), None);
        assert_eq!(symbol_name("class Invoice:"), Some("Invoice"));
        assert_eq!(symbol_name("func (i Invoice) Total() int {"), Some("Total"));
        assert_eq!(symbol_name("func main() {"), Some("main"));
        assert_eq!(symbol_name("type Invoice struct{}"), Some("Invoice"));
        assert_eq!(symbol_name("impl<T> Order<T> {"), Some("Order"));
        assert_eq!(
            symbol_name("pub(crate) const MAX_ORDERS: usize = 10;"),
            Some("MAX_ORDERS")
        );
        assert_eq!(symbol_name("static COUNT: u32 = 0;"), Some("COUNT"));
        assert_eq!(symbol_name("const fn zero() -> u32 {"), Some("zero"));
        assert_eq!(symbol_name("extern \"C\" fn c_call() {"), Some("c_call"));
        assert_eq!(symbol_name("macro_rules! order {"), Some("order"));
        assert_eq!(symbol_name("mod orders;"), Some("orders"));
        assert_eq!(symbol_name("pub union Bits {"), Some("Bits"));
        assert_eq!(symbol_name("    return inv.render()"), None);
    }

    #[test]
    fn word_at_covers_the_run_under_and_before_the_cursor() {
        let line = "    inv = parse_it(\"x\")";
        assert_eq!(word_at(line, 4), Some((4..7, "inv")));
        assert_eq!(word_at(line, 6), Some((4..7, "inv")));
        // Right after a word counts as being on it; on a space it does not.
        assert_eq!(word_at(line, 7), Some((4..7, "inv")));
        assert_eq!(word_at(line, 8), None);
        assert_eq!(word_at(line, 10), Some((10..18, "parse_it")));
        assert_eq!(word_at(line, line.len()), None);
        assert_eq!(word_at("", 0), None);
        assert_eq!(word_at("x", 99), Some((0..1, "x")));
    }
}
