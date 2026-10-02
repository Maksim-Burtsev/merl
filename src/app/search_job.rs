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
    /// In a review, the lines the branch deleted: `s` and `D` list them too (#440).
    pub(super) deleted: Arc<Vec<git::DeletedLine>>,
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
        // An escaped literal always compiles, but a pasted query can outgrow the matcher's size
        // limit: it finds nothing then, rather than killing the thread the answer is awaited from.
        let mut hits = self.run(false, true).unwrap_or_default();
        // The lines the branch deleted join their file's, in the order the review draws them,
        // and the list is cut where the grep's would be.
        if let (false, Ok(re)) = (
            self.deleted.is_empty(),
            RegexBuilder::new(&self.pattern)
                .case_insensitive(true)
                .build(),
        ) {
            hits.extend(deleted_hits(
                &self.deleted,
                |_| true,
                |t| re.find(t).map(|m| m.start()),
            ));
            let current = self.current.as_deref();
            hits.sort_by(|a, b| {
                let key = |h: &Hit| (Some(h.path.as_path()) != current, h.path.clone(), h.place());
                key(a).cmp(&key(b))
            });
            hits.truncate(search::MAX_HITS);
        }
        App::hit_items(hits)
    }

    /// Every declaration the [`search::SYMBOLS`] rows read out of the project, as
    /// `(listed name, the byte its line has it at, hit)`, keeping the names `pattern` matches —
    /// all of them when it is empty, which is the press of `D`. Each row is read only from the
    /// files it is written for; the name decides before the [`search::MAX_HITS`] cut, so a query
    /// reaches past a cut list.
    pub(super) fn symbol_hits(&self) -> (Vec<(String, usize, Hit)>, bool) {
        let mut named: Vec<(String, usize, Hit)> = Vec::new();
        let mut cut = false;
        for (kind, pattern) in search::SYMBOLS {
            let re = Regex::new(pattern).expect("built-in symbol patterns are valid");
            let wanted = |p: &Path| match kind {
                Some(k) => search::kind_of(p) == Some(*k) && search::row_reads(pattern, p),
                None => search::shared_symbols(search::kind_of(p)),
            };
            let files: Vec<PathBuf> = self.files.iter().filter(|p| wanted(p)).cloned().collect();
            // The press of `D` reads every declaration, so it pays for no name it will not list:
            // the filter is the query's, and there is none until one is typed.
            let keep = |line: &str| {
                self.pattern.is_empty()
                    || search::symbol_name(&re, line)
                        .is_some_and(|name| search::fuzzy_match(&self.pattern, &name))
            };
            let mut hits = search::grep_filtered(
                &self.root,
                &files,
                pattern,
                self.current.as_deref(),
                self.unsaved.as_deref(),
                keep,
            )
            .unwrap_or_default();
            // Each row has the cap to itself, so a cut is this row's, never the total's: on a
            // project whose kinds add up past it with none of them cut, the list is whole.
            cut |= hits.len() >= search::MAX_HITS;
            // A component's rows are its script's (#413).
            let mut script: HashMap<PathBuf, Vec<bool>> = HashMap::new();
            hits.retain(|h| {
                !search::component(&h.path)
                    || (script.entry(h.path.clone()).or_insert_with(|| {
                        let text = match (&self.unsaved, &self.current) {
                            (Some(t), Some(c)) if *c == h.path => {
                                String::from_utf8_lossy(t).into_owned()
                            }
                            _ => {
                                std::fs::read_to_string(self.root.join(&h.path)).unwrap_or_default()
                            }
                        };
                        search::script_lines(&h.path, &text).unwrap_or_default()
                    }))
                    .get(h.line - 1)
                    .copied()
                    .unwrap_or(false)
            });
            // The declarations the branch deleted, of the files this row is written for.
            hits.extend(deleted_hits(&self.deleted, wanted, |t| {
                (re.is_match(t) && keep(t)).then_some(0)
            }));
            named.extend(
                hits.into_iter()
                    .filter_map(|h| search::symbol_at(&re, &h.text).map(|(col, n)| (n, col, h))),
            );
        }
        (named, cut)
    }
}

/// The lines the branch deleted, in files `wanted` takes, where `find` finds the byte a row lands
/// on (#440). Nothing outside a review.
pub(super) fn deleted_hits(
    lines: &[git::DeletedLine],
    wanted: impl Fn(&Path) -> bool,
    find: impl Fn(&str) -> Option<usize>,
) -> Vec<Hit> {
    lines
        .iter()
        .filter(|d| wanted(&d.path))
        .filter_map(|d| {
            Some(Hit {
                col: find(&d.text)?,
                path: d.path.clone(),
                line: d.line,
                text: d.text.clone(),
                deleted: Some(d.at),
            })
        })
        .collect()
}

/// A type `d` followed a receiver to: its name and the line that declares it.
#[derive(Debug, Clone)]
pub(super) struct Typed {
    pub(super) name: String,
    pub(super) path: PathBuf,
    pub(super) line: usize,
}
