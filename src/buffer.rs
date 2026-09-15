//! File contents as lines, plus its syntect highlighting and what is needed to write it back.

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Context, Result};
use ratatui::style::Style;
use syntect::highlighting::{HighlightIterator, HighlightState, Highlighter};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

use crate::theme::Theme;

/// What a tab is drawn as. Lines keep their `\t`; only the renderer and [`crate::wrap`] expand.
pub const TAB: &str = "    ";
/// Every `CHECKPOINT` lines the parser state is snapshotted, so an edit only re-highlights from
/// the last snapshot instead of from line 1.
const CHECKPOINT: usize = 64;
/// How far in we look for a NUL before calling a file binary.
const SNIFF: usize = 8 * 1024;
/// ponytail: syntect is sequential, so a huge file would have to be parsed from line 1 before
/// anything can be drawn. Past these limits merl shows plain text instead of stalling.
const MAX_HL_LINES: usize = 30_000;
const MAX_HL_BYTES: usize = 4 * 1024 * 1024;
/// ponytail: a single line longer than this is shown truncated. Both the renderer and the
/// cursor arithmetic wrap [`Buffer::shown`], so they agree on how many rows the line has.
const MAX_SHOWN_BYTES: usize = 20_000;

/// bat's syntax set, with the `\n`-terminated variants `highlight_line` expects.
fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(two_face::syntax::extra_newlines)
}

/// One line's highlighting: styles over byte ranges, in order.
pub type Spans = Vec<(Style, Range<usize>)>;

pub struct Buffer {
    pub path: Option<PathBuf>,
    pub lines: Vec<String>,
    /// Why the buffer cannot be edited, when it cannot: what was loaded is not what would be
    /// written back.
    pub readonly: Option<&'static str>,
    /// Lines end with `\r\n` on disk.
    crlf: bool,
    /// The file ends with a line terminator (every sane file does; an empty file does not).
    trailing_newline: bool,
    /// Indentation uses tabs: what Tab inserts, as VS Code's `detectIndentation`.
    pub tabs: bool,
    /// [`hash`] of the bytes last read from or written to `path`: tells merl's own saves and
    /// no-op events apart from a change made by someone else.
    pub disk: u64,
    /// Highlighted prefix: one entry per already-highlighted line, spans in byte ranges.
    pub hl: Vec<Spans>,
    /// `None` when this buffer is not highlighted at all (binary, or too large).
    syntax: Option<&'static SyntaxReference>,
    /// syntect's carry-over state at the end of `hl`.
    state: Option<(ParseState, HighlightState)>,
    /// `checkpoints[i]` is the state before line `i * CHECKPOINT`.
    checkpoints: Vec<(ParseState, HighlightState)>,
}

impl Buffer {
    /// An empty scratch buffer, used when merl is opened on a directory.
    pub fn empty() -> Self {
        Self::new(None, vec![String::new()], None)
    }

    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).with_context(|| format!("{}", path.display()))?;
        Ok(Self::from_bytes(path.to_path_buf(), &bytes))
    }

    pub fn from_bytes(path: PathBuf, bytes: &[u8]) -> Self {
        if bytes[..bytes.len().min(SNIFF)].contains(&0) {
            let mut b = Self::new(Some(path), vec!["binary file".to_string()], None);
            b.readonly = Some("binary file");
            return b;
        }
        let text = String::from_utf8_lossy(bytes);
        let lossy = matches!(text, std::borrow::Cow::Owned(_));
        let crlf = text.contains("\r\n");
        let trailing_newline = text.ends_with('\n');
        let mut lines: Vec<String> = text
            .split('\n')
            .map(|l| l.trim_end_matches('\r').to_string())
            .collect();
        // A trailing newline is a terminator, not an empty last line.
        if lines.len() > 1 && lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        let syntax = (lines.len() <= MAX_HL_LINES && bytes.len() <= MAX_HL_BYTES)
            .then(|| syntax_for(&path, &lines[0]));
        let mut b = Self::new(Some(path), lines, syntax);
        b.readonly = lossy.then_some("not UTF-8");
        b.crlf = crlf;
        b.trailing_newline = trailing_newline;
        b.tabs = b.lines.iter().any(|l| l.starts_with('\t'));
        b.disk = hash(bytes);
        b
    }

    /// The file as it is written back: the lines joined with the line ending they came with.
    pub fn to_bytes(&self) -> Vec<u8> {
        let nl = if self.crlf { "\r\n" } else { "\n" };
        let mut out = self.lines.join(nl);
        if self.trailing_newline {
            out.push_str(nl);
        }
        out.into_bytes()
    }

    /// Forgets the highlighting from line `l` on: the next `highlight_to` re-parses from the
    /// last checkpoint above it.
    pub fn edited(&mut self, l: usize) {
        let n = l / CHECKPOINT;
        if self.hl.len() <= n * CHECKPOINT {
            return;
        }
        self.hl.truncate(n * CHECKPOINT);
        self.checkpoints.truncate(n + 1);
        self.state = self.checkpoints.last().cloned();
    }

    fn new(
        path: Option<PathBuf>,
        lines: Vec<String>,
        syntax: Option<&'static SyntaxReference>,
    ) -> Self {
        Self {
            path,
            lines,
            readonly: None,
            crlf: false,
            trailing_newline: true,
            tabs: false,
            disk: 0,
            hl: Vec::new(),
            syntax,
            state: None,
            checkpoints: Vec::new(),
        }
    }

    /// Line `l` as it is shown: the whole line, or its first [`MAX_SHOWN_BYTES`] bytes.
    pub fn shown(&self, l: usize) -> &str {
        let s = &self.lines[l];
        &s[..floor_boundary(s, MAX_SHOWN_BYTES)]
    }

    /// Drops the highlighting, so the next [`Buffer::highlight_to`] paints with another theme:
    /// spans carry the colours of the theme they were made with.
    pub fn clear_hl(&mut self) {
        self.hl.clear();
        self.checkpoints.clear();
        self.state = None;
    }

    /// Extends the highlighted prefix so that `last` (a file line index) is covered.
    ///
    /// syntect's parser is sequential: line N needs the state left by line N-1, so the view can
    /// only be drawn after everything above it has been parsed. The result is cached in `hl`.
    pub fn highlight_to(&mut self, last: usize, theme: &Theme) {
        let Some(syntax) = self.syntax else { return };
        let last = last.min(self.lines.len().saturating_sub(1));
        if self.hl.len() > last {
            return;
        }
        let set = syntaxes();
        let highlighter = Highlighter::new(&theme.syntect);
        let state = self.state.get_or_insert_with(|| {
            (
                ParseState::new(syntax),
                HighlightState::new(&highlighter, ScopeStack::new()),
            )
        });
        while self.hl.len() <= last {
            if self.hl.len().is_multiple_of(CHECKPOINT)
                && self.checkpoints.len() == self.hl.len() / CHECKPOINT
            {
                self.checkpoints.push(state.clone());
            }
            let raw = &self.lines[self.hl.len()];
            let line = format!("{raw}\n");
            let ops = state.0.parse_line(&line, set).unwrap_or_default();
            let mut spans = Vec::new();
            let mut at = 0usize;
            for (style, text) in HighlightIterator::new(&mut state.1, &ops, &line, &highlighter) {
                let start = at;
                at += text.len();
                // The trailing "\n" we added is not part of the line.
                let end = at.min(raw.len());
                if start < end {
                    spans.push((crate::theme::style(style), start..end));
                }
            }
            self.hl.push(spans);
        }
    }
}

pub fn hash(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}

/// Largest byte index <= `max` that is a char boundary of `s`.
fn floor_boundary(s: &str, max: usize) -> usize {
    if s.len() <= max {
        return s.len();
    }
    let mut i = max;
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Syntax by file name, then by the first line (shebangs, `<?xml`), then plain text.
fn syntax_for(path: &Path, first_line: &str) -> &'static SyntaxReference {
    let set = syntaxes();
    let found = known_name(path)
        .and_then(|name| set.find_syntax_by_name(name))
        .or_else(|| set.find_syntax_for_file(path).ok().flatten())
        .or_else(|| set.find_syntax_by_first_line(first_line))
        .unwrap_or_else(|| set.find_syntax_plain_text());
    // bat's plain Dockerfile grammar leaves every instruction's arguments unscoped, so most of
    // the file would be drawn in the default colour; the bash variant scopes them.
    if found.name == "Dockerfile" {
        return set
            .find_syntax_by_name("Dockerfile (with bash)")
            .unwrap_or(found);
    }
    found
}

/// Infrastructure files bat's set has no name pattern for, mapped to the grammar that fits.
fn known_name(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?;
    let ext = name.rsplit_once('.').map_or("", |(_, ext)| ext);
    Some(match (name, ext) {
        // `.dockerignore`, and BuildKit's per-Dockerfile `app.Dockerfile.dockerignore`.
        (_, "dockerignore") | ("CODEOWNERS", _) => "Git Ignore",
        ("Containerfile", _) => "Dockerfile",
        _ if name.starts_with("Dockerfile.") || name.starts_with("Containerfile.") => "Dockerfile",
        (_, "jsonc") => "JSON",
        // bat's SQL grammar owns `.sql`, `.ddl` and `.dml`, but not the dialect extensions.
        (_, "psql" | "pgsql" | "mysql") => "SQL",
        (".npmrc", _) | (_, "service" | "timer" | "socket") => "INI",
        ("Procfile" | "yarn.lock", _) => "YAML",
        // Starlark.
        ("WORKSPACE" | "Tiltfile", _) => "Python",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(bytes: &[u8]) -> Buffer {
        Buffer::from_bytes(PathBuf::from("x"), bytes)
    }

    #[test]
    fn keeps_tabs_and_round_trips_line_endings() {
        let b = load(b"a\tb\r\nc\n");
        assert_eq!(b.lines, vec!["a\tb", "c"]);
        assert_eq!(b.to_bytes(), b"a\tb\r\nc\r\n");
        assert_eq!(load(b"x").to_bytes(), b"x");
        assert_eq!(load(b"").to_bytes(), b"");
        assert_eq!(load(b"\n").to_bytes(), b"\n");
        assert!(load(b"\tx\n").tabs);
        assert!(!load(b"    x\n").tabs);
    }

    #[test]
    fn edits_re_highlight_from_the_last_checkpoint() {
        let theme = crate::theme::load("tokyonight-moon").unwrap();
        let src = "x = 1\n".repeat(200);
        let mut b = Buffer::from_bytes(PathBuf::from("a.py"), src.as_bytes());
        b.highlight_to(199, &theme);
        assert_eq!(b.checkpoints.len(), 4);
        b.lines[150] = "# comment".into();
        b.edited(150);
        assert_eq!(b.hl.len(), 128);
        assert_eq!(b.checkpoints.len(), 3);
        b.highlight_to(199, &theme);
        assert_eq!(b.hl.len(), 200);
        assert_ne!(b.hl[150][0].0.fg, b.hl[151][0].0.fg);
        // Editing below the highlighted prefix forgets nothing.
        b.edited(5000);
        assert_eq!(b.hl.len(), 200);
    }

    #[test]
    fn empty_file_has_one_line() {
        assert_eq!(load(b"").lines, vec![""]);
        assert_eq!(load(b"\n").lines, vec![""]);
    }

    #[test]
    fn binary_is_one_line() {
        let b = load(b"ELF\0\x01\x02");
        assert_eq!(b.lines, vec!["binary file"]);
        assert!(b.syntax.is_none());
        assert_eq!(b.readonly, Some("binary file"));
    }

    #[test]
    fn invalid_utf8_is_lossy() {
        let b = load(b"a\xffb");
        assert_eq!(b.lines, vec!["a\u{fffd}b"]);
        assert_eq!(b.readonly, Some("not UTF-8"));
    }

    #[test]
    fn huge_files_are_not_highlighted() {
        let text = "x\n".repeat(MAX_HL_LINES + 1);
        let mut b = Buffer::from_bytes(PathBuf::from("big.rs"), text.as_bytes());
        assert!(b.syntax.is_none());
        b.highlight_to(10, &crate::theme::load("tokyonight-moon").unwrap());
        assert!(b.hl.is_empty());
    }

    #[test]
    fn highlights_a_prefix_with_in_range_spans() {
        let theme = crate::theme::load("tokyonight-moon").unwrap();
        let src = "// hi\nfn main() {\n    let s = \"x\";\n}\n";
        let mut b = Buffer::from_bytes(PathBuf::from("a.rs"), src.as_bytes());
        b.highlight_to(1, &theme);
        assert_eq!(b.hl.len(), 2, "only the requested prefix is parsed");
        b.highlight_to(3, &theme);
        assert_eq!(b.hl.len(), 4);
        for (line, spans) in b.lines.iter().zip(&b.hl) {
            assert!(!spans.is_empty() || line.is_empty());
            assert!(spans.iter().all(|(_, r)| r.end <= line.len()), "{line:?}");
        }
        // A comment and a keyword must not end up the same colour.
        assert_ne!(b.hl[0][0].0.fg, b.hl[1][0].0.fg);
    }

    #[test]
    fn rust_highlights_with_every_shipped_theme() {
        let src = "/// doc\npub struct Order { n: u32 }\nfn main() { let s = \"x\"; }\n";
        for name in crate::theme::names() {
            let theme = crate::theme::load(name).unwrap();
            let mut b = Buffer::from_bytes(PathBuf::from("a.rs"), src.as_bytes());
            assert_eq!(b.syntax.map(|s| s.name.as_str()), Some("Rust"), "{name}");
            b.highlight_to(2, &theme);
            let colours: std::collections::HashSet<_> =
                b.hl.iter().flatten().map(|(s, _)| s.fg).collect();
            assert!(colours.len() > 1, "{name}: everything is one colour");
        }
    }

    #[test]
    fn sql_highlights_with_every_shipped_theme() {
        let src =
            "-- doc\nCREATE TABLE public.orders (id serial);\nSELECT 'x' FROM public.orders;\n";
        for file in ["a.sql", "b.psql", "c.pgsql", "d.mysql", "e.ddl", "f.dml"] {
            for name in crate::theme::names() {
                let theme = crate::theme::load(name).unwrap();
                let mut b = Buffer::from_bytes(PathBuf::from(file), src.as_bytes());
                assert_eq!(
                    b.syntax.map(|s| s.name.as_str()),
                    Some("SQL"),
                    "{file} {name}"
                );
                b.highlight_to(2, &theme);
                let colours: std::collections::HashSet<_> =
                    b.hl.iter().flatten().map(|(s, _)| s.fg).collect();
                assert!(colours.len() > 1, "{file} {name}: everything is one colour");
            }
        }
    }

    #[test]
    fn ts_and_js_highlight_with_every_shipped_theme() {
        let src = "// doc\nexport class Order { n = 1 }\nfunction main() { const s = \"x\"; }\n";
        for (file, lang) in [
            ("a.ts", "TypeScript"),
            ("b.tsx", "TypeScriptReact"),
            ("c.js", "JavaScript (Babel)"),
        ] {
            for name in crate::theme::names() {
                let theme = crate::theme::load(name).unwrap();
                let mut b = Buffer::from_bytes(PathBuf::from(file), src.as_bytes());
                assert_eq!(
                    b.syntax.map(|s| s.name.as_str()),
                    Some(lang),
                    "{file} {name}"
                );
                b.highlight_to(2, &theme);
                let colours: std::collections::HashSet<_> =
                    b.hl.iter().flatten().map(|(s, _)| s.fg).collect();
                assert!(colours.len() > 1, "{file} {name}: everything is one colour");
            }
        }
    }

    #[test]
    fn shell_highlights_with_every_shipped_theme() {
        let src = "#!/usr/bin/env bash\n# doc\nbuild() { echo \"$ROOT\"; }\n";
        // bat's own name and shebang patterns cover all of these; none needs a mapping.
        for file in [
            "a.sh", "b.bash", "c.zsh", "d.ksh", ".bashrc", ".zshrc", ".profile", "install",
        ] {
            for name in crate::theme::names() {
                let theme = crate::theme::load(name).unwrap();
                let mut b = Buffer::from_bytes(PathBuf::from(file), src.as_bytes());
                assert_eq!(
                    b.syntax.map(|s| s.name.as_str()),
                    Some("Bourne Again Shell (bash)"),
                    "{file} {name}"
                );
                b.highlight_to(2, &theme);
                let colours: std::collections::HashSet<_> =
                    b.hl.iter().flatten().map(|(s, _)| s.fg).collect();
                assert!(colours.len() > 1, "{file} {name}: everything is one colour");
            }
        }
    }

    #[test]
    fn shown_clips_a_huge_line_on_a_char_boundary() {
        let text = "漢".repeat(MAX_SHOWN_BYTES / 3 + 1);
        let b = load(text.as_bytes());
        assert_eq!(b.shown(0).len(), MAX_SHOWN_BYTES / 3 * 3);
        assert_eq!(load(b"short").shown(0), "short");
    }

    #[test]
    fn first_line_picks_the_syntax_when_the_name_cannot() {
        let b = Buffer::from_bytes(PathBuf::from("script"), b"#!/usr/bin/env python3\nx = 1\n");
        assert_eq!(b.syntax.unwrap().name, "Python");
    }

    #[test]
    fn infra_files_get_their_grammar() {
        for (name, syntax) in [
            (".dockerignore", "Git Ignore"),
            ("app.Dockerfile.dockerignore", "Git Ignore"),
            ("CODEOWNERS", "Git Ignore"),
            ("Dockerfile", "Dockerfile (with bash)"),
            ("Dockerfile.prod", "Dockerfile (with bash)"),
            ("prod.Dockerfile", "Dockerfile (with bash)"),
            ("api.dockerfile", "Dockerfile (with bash)"),
            ("Containerfile", "Dockerfile (with bash)"),
            ("Containerfile.dev", "Dockerfile (with bash)"),
            ("tsconfig.jsonc", "JSON"),
            (".npmrc", "INI"),
            ("merl.service", "INI"),
            ("merl.timer", "INI"),
            ("merl.socket", "INI"),
            ("Procfile", "YAML"),
            ("yarn.lock", "YAML"),
            ("WORKSPACE", "Python"),
            ("Tiltfile", "Python"),
            // Already in bat's set; listed so a two-face upgrade cannot drop them silently.
            ("docker-compose.yml", "YAML"),
            ("Makefile", "Makefile"),
            ("common.mk", "Makefile"),
            ("main.tf", "Terraform"),
            ("prod.tfvars", "Terraform"),
            ("nginx.conf", "nginx"),
            (".env", "DotENV"),
            (".env.local", "DotENV"),
            (".editorconfig", "INI"),
            ("go.sum", "Gosum"),
            ("poetry.lock", "TOML"),
            ("Jenkinsfile", "Groovy"),
        ] {
            let b = Buffer::from_bytes(PathBuf::from(name), b"x\n");
            assert_eq!(b.syntax.unwrap().name, syntax, "{name}");
        }
        // The first line still finds a Dockerfile, and it gets the bash variant too.
        let b = Buffer::from_bytes(PathBuf::from("image"), b"FROM rust:1.80\n");
        assert_eq!(b.syntax.unwrap().name, "Dockerfile (with bash)");
    }
}
