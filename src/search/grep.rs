//! Project-wide grep: ripgrep's own crates behind one call, and the lines it gives back.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use grep_searcher::{BinaryDetection, Searcher, SearcherBuilder, Sink, SinkMatch};

/// ponytail: a hard stop instead of a streaming picker, taken in file order after the open file,
/// so the picker title says it is cut ("first N", or "N+ hits" for `s`) when it hits. Raise it
/// if a picker over the whole result set ever becomes the point.
pub const MAX_HITS: usize = 5_000;
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
    ignore_case: bool,
    current: Option<&Path>,
    unsaved: Option<&[u8]>,
) -> Result<Vec<Hit>> {
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(ignore_case)
        .word(whole_word)
        .build(pattern)
        .with_context(|| format!("bad pattern `{pattern}`"))?;
    Ok(collect(root, files, &matcher, current, unsaved, |_| true))
}
/// Greps `pattern` over `files`, keeping only the lines `keep` takes. The filter runs before the
/// [`MAX_HITS`] cut, so what the cut drops are matches of the query, not whatever the walk
/// reached first: `D` past the cap searches with this.
pub fn grep_filtered(
    root: &Path,
    files: &[PathBuf],
    pattern: &str,
    current: Option<&Path>,
    unsaved: Option<&[u8]>,
    keep: impl Fn(&str) -> bool,
) -> Result<Vec<Hit>> {
    let matcher = RegexMatcherBuilder::new()
        .build(pattern)
        .with_context(|| format!("bad pattern `{pattern}`"))?;
    Ok(collect(root, files, &matcher, current, unsaved, keep))
}
/// The file walk both greps share: one `Hit` per line `matcher` matches and `keep` takes.
fn collect(
    root: &Path,
    files: &[PathBuf],
    matcher: &RegexMatcher,
    current: Option<&Path>,
    unsaved: Option<&[u8]>,
    keep: impl Fn(&str) -> bool,
) -> Vec<Hit> {
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
            keep: &keep,
        };
        let _ = match unsaved.filter(|_| current == Some(rel.as_path())) {
            Some(text) => searcher.search_slice(matcher, text, sink),
            None => searcher.search_path(matcher, root.join(rel), sink),
        };
    }
    hits.sort_by_cached_key(|h| (current != Some(h.path.as_path()), h.path.clone(), h.line));
    hits
}
/// Collects one `Hit` per matching line `keep` takes, stopping the whole search at [`MAX_HITS`].
struct Collect<'a> {
    path: &'a Path,
    hits: &'a mut Vec<Hit>,
    keep: &'a dyn Fn(&str) -> bool,
}
impl Sink for Collect<'_> {
    type Error = std::io::Error;

    fn matched(&mut self, _searcher: &Searcher, m: &SinkMatch<'_>) -> std::io::Result<bool> {
        let text = String::from_utf8_lossy(m.bytes()).trim_end().to_string();
        if (self.keep)(&text) {
            self.hits.push(Hit {
                path: self.path.to_path_buf(),
                line: m.line_number().unwrap_or(0) as usize,
                text,
            });
        }
        Ok(self.hits.len() < MAX_HITS)
    }
}
