//! One project grep, owned by the thread that runs it.

use super::*;

/// How long the `s` query has to stand still before it is grepped.
pub(super) const SEARCH_PAUSE: Duration = Duration::from_millis(80);

/// One project grep with everything it reads, owned: `s` and `D` run it in a thread.
pub struct SearchJob {
    pub seq: u64,
    pub(super) root: PathBuf,
    pub(super) files: Vec<PathBuf>,
    /// The text `s` looks for, escaped; for a `symbols` job, the query as typed.
    pub(super) pattern: String,
    /// `D` past the cap: the job greps the [`search::SYMBOLS`] patterns and keeps the
    /// declarations whose name matches `pattern`, rather than the text of the lines.
    pub(super) symbols: bool,
    pub(super) current: Option<PathBuf>,
    pub(super) unsaved: Option<Vec<u8>>,
}

impl SearchJob {
    pub(super) fn run(&self, whole_word: bool, ignore_case: bool) -> anyhow::Result<Vec<Hit>> {
        search::grep_project(
            &self.root,
            &self.files,
            &self.pattern,
            whole_word,
            ignore_case,
            self.current.as_deref(),
            self.unsaved.as_deref(),
        )
    }

    /// The rows the answer becomes: the lines `s` found, any case and the query anywhere in
    /// them, or the declarations `D` lists.
    pub fn items(&self) -> Vec<PickItem> {
        if self.symbols {
            return App::symbol_items(self.symbol_hits().0);
        }
        App::hit_items(
            self.run(false, true)
                .expect("an escaped literal always compiles"),
        )
    }

    /// Every declaration the [`search::SYMBOLS`] rows read out of the project, as
    /// `(listed name, hit)`, keeping the names `pattern` matches — all of them when it is empty,
    /// which is the press of `D`. Each row is read only from the files it is written for; the
    /// name decides before the [`search::MAX_HITS`] cut, so a query reaches past a cut list.
    pub(super) fn symbol_hits(&self) -> (Vec<(String, Hit)>, bool) {
        let mut named: Vec<(String, Hit)> = Vec::new();
        let mut cut = false;
        for (kind, pattern) in search::SYMBOLS {
            let re = Regex::new(pattern).expect("built-in symbol patterns are valid");
            let files: Vec<PathBuf> = self
                .files
                .iter()
                .filter(|p| match kind {
                    Some(k) => search::kind_of(p) == Some(*k),
                    None => search::shared_symbols(search::kind_of(p)),
                })
                .cloned()
                .collect();
            let hits = search::grep_filtered(
                &self.root,
                &files,
                pattern,
                self.current.as_deref(),
                self.unsaved.as_deref(),
                // The press of `D` reads every declaration, so it pays for no name it will
                // not list: the filter is the query's, and there is none until one is typed.
                |line| {
                    self.pattern.is_empty()
                        || search::symbol_name(&re, line)
                            .is_some_and(|name| search::fuzzy_match(&self.pattern, &name))
                },
            )
            .unwrap_or_default();
            // Each row has the cap to itself, so a cut is this row's, never the total's: on a
            // project whose kinds add up past it with none of them cut, the list is whole.
            cut |= hits.len() >= search::MAX_HITS;
            named.extend(
                hits.into_iter()
                    .filter_map(|h| Some((search::symbol_name(&re, &h.text)?, h))),
            );
        }
        (named, cut)
    }
}

/// A type `d` followed a receiver to: its name and the line that declares it.
#[derive(Debug, Clone)]
pub(super) struct Typed {
    pub(super) name: String,
    pub(super) path: PathBuf,
    pub(super) line: usize,
}
