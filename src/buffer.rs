//! File contents, normalized for display, plus its syntect highlighting. Read-only.

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Context, Result};
use ratatui::style::Style;
use syntect::highlighting::{HighlightIterator, HighlightState, Highlighter};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

use crate::theme::Theme;

const TAB: &str = "    ";
/// How far in we look for a NUL before calling a file binary.
const SNIFF: usize = 8 * 1024;
/// ponytail: syntect is sequential, so a huge file would have to be parsed from line 1 before
/// anything can be drawn. Past these limits merl shows plain text instead of stalling.
const MAX_HL_LINES: usize = 30_000;
const MAX_HL_BYTES: usize = 4 * 1024 * 1024;

/// bat's syntax set, with the `\n`-terminated variants `highlight_line` expects.
fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(two_face::syntax::extra_newlines)
}

pub struct Buffer {
    pub path: Option<PathBuf>,
    pub lines: Vec<String>,
    /// Highlighted prefix: one entry per already-highlighted line, spans in byte ranges.
    pub hl: Vec<Vec<(Style, Range<usize>)>>,
    /// `None` when this buffer is not highlighted at all (binary, or too large).
    syntax: Option<&'static SyntaxReference>,
    /// syntect's carry-over state at the end of `hl`.
    state: Option<(ParseState, HighlightState)>,
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
            return Self::new(Some(path), vec!["binary file".to_string()], None);
        }
        let text = String::from_utf8_lossy(bytes);
        let mut lines: Vec<String> = text
            .split('\n')
            .map(|l| l.trim_end_matches('\r').replace('\t', TAB))
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
        Self::new(Some(path), lines, syntax)
    }

    fn new(
        path: Option<PathBuf>,
        lines: Vec<String>,
        syntax: Option<&'static SyntaxReference>,
    ) -> Self {
        Self {
            path,
            lines,
            hl: Vec::new(),
            syntax,
            state: None,
        }
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

/// Syntax by file name, then by the first line (shebangs, `<?xml`), then plain text.
fn syntax_for(path: &Path, first_line: &str) -> &'static SyntaxReference {
    let set = syntaxes();
    set.find_syntax_for_file(path)
        .ok()
        .flatten()
        .or_else(|| set.find_syntax_by_first_line(first_line))
        .unwrap_or_else(|| set.find_syntax_plain_text())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(bytes: &[u8]) -> Buffer {
        Buffer::from_bytes(PathBuf::from("x"), bytes)
    }

    #[test]
    fn normalizes_text() {
        assert_eq!(load(b"a\tb\r\nc\n").lines, vec!["a    b", "c"]);
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
    }

    #[test]
    fn invalid_utf8_is_lossy() {
        assert_eq!(load(b"a\xffb").lines, vec!["a\u{fffd}b"]);
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
    fn first_line_picks_the_syntax_when_the_name_cannot() {
        let b = Buffer::from_bytes(PathBuf::from("script"), b"#!/usr/bin/env python3\nx = 1\n");
        assert_eq!(b.syntax.unwrap().name, "Python");
    }
}
