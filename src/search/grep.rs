use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};

use crate::git::TextLine;
use grep_matcher::Matcher;
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use grep_searcher::{BinaryDetection, Searcher, SearcherBuilder, Sink, SinkMatch};

pub const MAX_HITS: usize = 5_000;
const MAX_THREADS: usize = 8;
const FILES_PER_THREAD: usize = 64;
/// One matching line. `path` is relative to the project root.
#[derive(Debug, Clone)]
pub struct Hit {
    pub path: PathBuf,
    pub line1: usize,
    /// Where the pattern's match starts: where `s` lands on the row.
    pub byte_col: Option<usize>,
    pub text: String,
    /// A line the branch under review deleted (#440): where the review draws it. `line1` is then
    /// its number in the file at the base.
    pub deleted: Option<TextLine>,
}

impl Hit {
    /// The line of the text the hit is on, as a review orders them.
    pub fn place(&self) -> TextLine {
        self.deleted.unwrap_or(TextLine::File(self.line1 - 1))
    }
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
    Ok(collect(
        root,
        files,
        &matcher,
        current,
        unsaved,
        None,
        |_| true,
    ))
}
pub type ShapedFiles = Mutex<HashMap<PathBuf, Option<Arc<Shaped>>>>;
pub struct Shaped {
    text: Vec<u8>,
    lines: Vec<usize>,
}
const SHAPED_BYTES: usize = 1 << 18;
pub fn grep_shaped(
    files: &[PathBuf],
    pattern: &str,
    shape: &RegexMatcher,
    shaped: &ShapedFiles,
) -> Result<Vec<Hit>> {
    let matcher = RegexMatcherBuilder::new()
        .build(pattern)
        .with_context(|| format!("bad pattern `{pattern}`"))?;
    let root = Path::new("");
    Ok(collect(
        root,
        files,
        &matcher,
        None,
        None,
        Some((shape, shaped)),
        |_| true,
    ))
}
pub fn shape_matcher(pattern: &str) -> RegexMatcher {
    RegexMatcherBuilder::new()
        .build(pattern)
        .expect("built-in patterns compile")
}
fn shaped_lines(
    searcher: &mut Searcher,
    shape: &RegexMatcher,
    shaped: &ShapedFiles,
    path: &Path,
) -> Option<Arc<Shaped>> {
    if let Some(known) = shaped.lock().ok()?.get(path) {
        return known.clone();
    }
    let read = std::fs::read(path).ok().and_then(|bytes| {
        let plain = !bytes.contains(&0);
        let mut kept = Shaped {
            text: Vec::new(),
            lines: Vec::new(),
        };
        let sink = grep_searcher::sinks::Bytes(|n, line| {
            kept.text.extend_from_slice(line);
            if !line.ends_with(b"\n") {
                kept.text.push(b'\n');
            }
            kept.lines.push(n as usize);
            Ok(kept.text.len() <= SHAPED_BYTES)
        });
        searcher.search_slice(shape, &bytes, sink).ok()?;
        (plain && kept.text.len() <= SHAPED_BYTES).then(|| Arc::new(kept))
    });
    shaped.lock().ok()?.insert(path.to_path_buf(), read.clone());
    read
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
    keep: impl Fn(&str) -> bool + Sync,
) -> Result<Vec<Hit>> {
    let matcher = RegexMatcherBuilder::new()
        .build(pattern)
        .with_context(|| format!("bad pattern `{pattern}`"))?;
    Ok(collect(root, files, &matcher, current, unsaved, None, keep))
}
#[allow(clippy::too_many_arguments)]
fn collect(
    root: &Path,
    files: &[PathBuf],
    matcher: &RegexMatcher,
    current: Option<&Path>,
    unsaved: Option<&[u8]>,
    shape: Option<(&RegexMatcher, &ShapedFiles)>,
    keep: impl Fn(&str) -> bool + Sync,
) -> Vec<Hit> {
    let next_file = AtomicUsize::new(0);
    let cap_reached = AtomicBool::new(false);
    let search_until_cap = || {
        let mut searcher = SearcherBuilder::new()
            .line_number(true)
            .binary_detection(BinaryDetection::quit(0))
            .build();
        let mut hits = Vec::new();
        while !cap_reached.load(Ordering::Relaxed) {
            let file = next_file.fetch_add(1, Ordering::Relaxed);
            let Some(rel) = files.get(file) else {
                break;
            };
            let shaped =
                shape.and_then(|(m, known)| shaped_lines(&mut searcher, m, known, &root.join(rel)));
            let sink = Collect {
                file,
                path: rel,
                matcher,
                hits: &mut hits,
                keep: &keep,
                lines: shaped.as_ref().map(|s| s.lines.as_slice()),
            };
            let _ = match (unsaved.filter(|_| current == Some(rel.as_path())), &shaped) {
                (Some(text), _) => searcher.search_slice(matcher, text, sink),
                (None, Some(s)) => searcher.search_slice(matcher, &s.text, sink),
                (None, None) => searcher.search_path(matcher, root.join(rel), sink),
            };
            if hits.len() >= MAX_HITS {
                cap_reached.store(true, Ordering::Relaxed);
            }
        }
        hits
    };
    let threads = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(MAX_THREADS)
        .min(files.len().div_ceil(FILES_PER_THREAD));
    let mut hits: Vec<(usize, Hit)> = std::thread::scope(|s| {
        let others: Vec<_> = (1..threads).map(|_| s.spawn(search_until_cap)).collect();
        let mut hits = search_until_cap();
        for t in others {
            hits.extend(t.join().unwrap_or_else(|e| std::panic::resume_unwind(e)));
        }
        hits
    });
    hits.sort_by_key(|(file, h)| (*file, h.line1));
    hits.truncate(MAX_HITS);
    let mut hits: Vec<Hit> = hits.into_iter().map(|(_, h)| h).collect();
    hits.sort_by_cached_key(|h| (current != Some(h.path.as_path()), h.path.clone(), h.line1));
    hits
}
struct Collect<'a> {
    file: usize,
    path: &'a Path,
    matcher: &'a RegexMatcher,
    hits: &'a mut Vec<(usize, Hit)>,
    keep: &'a dyn Fn(&str) -> bool,
    lines: Option<&'a [usize]>,
}
impl Sink for Collect<'_> {
    type Error = std::io::Error;

    fn matched(&mut self, _searcher: &Searcher, m: &SinkMatch<'_>) -> std::io::Result<bool> {
        let line = String::from_utf8_lossy(m.bytes());
        let text = line.trim_end();
        if (self.keep)(text) {
            // The matcher that found the line, run again over it as `Buffer` reads it (lossy, its
            // trailing blanks kept), finds the column as ripgrep does, with nothing to compile.
            let col = self.matcher.find(line.as_bytes()).ok().flatten();
            self.hits.push((
                self.file,
                Hit {
                    path: self.path.to_path_buf(),
                    line1: {
                        let n = m.line_number().unwrap_or(0) as usize;
                        self.lines
                            .map_or(n, |l| l.get(n.wrapping_sub(1)).copied().unwrap_or(0))
                    },
                    byte_col: col.map(|m| m.start()),
                    text: text.to_string(),
                    deleted: None,
                },
            ));
        }
        Ok(self.hits.len() < MAX_HITS)
    }
}
