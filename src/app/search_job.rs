use super::*;
use symbols::{NamedSymbol, SymbolHits};

pub(super) const SEARCH_PAUSE: Duration = Duration::from_millis(80);

pub struct SearchJob {
    pub seq: u64,
    pub(super) root: PathBuf,
    pub(super) files: Vec<PathBuf>,
    pub(super) pattern: String,
    pub(super) symbols: bool,
    pub(super) current: Option<PathBuf>,
    pub(super) unsaved: Option<Vec<u8>>,
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

    pub fn items(&self) -> Vec<PickItem> {
        if self.symbols {
            return App::symbol_items(self.symbol_hits().named);
        }
        // An escaped literal always compiles, but a pasted query can outgrow the matcher's size
        // limit: it finds nothing then, rather than killing the thread the answer is awaited from.
        let mut hits = self.run(false, true).unwrap_or_default();
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

    pub(super) fn symbol_hits(&self) -> SymbolHits {
        let mut named: Vec<NamedSymbol> = Vec::new();
        let mut cut = false;
        let mut haskell_code: HashMap<PathBuf, search::HaskellCode> = HashMap::new();
        for (kind, pattern) in search::SYMBOLS {
            let re = Regex::new(pattern).expect("built-in symbol patterns are valid");
            let wanted = |p: &Path| match kind {
                Some(k) => search::kind_of(p) == Some(*k) && search::row_reads(pattern, p),
                None => search::shared_symbols(search::kind_of(p)),
            };
            let files: Vec<PathBuf> = self.files.iter().filter(|p| wanted(p)).cloned().collect();
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
            cut |= hits.len() >= search::MAX_HITS;
            let read = |path: &Path| match (&self.unsaved, &self.current) {
                (Some(t), Some(c)) if c.as_path() == path => {
                    String::from_utf8_lossy(t).into_owned()
                }
                _ => std::fs::read_to_string(self.root.join(path)).unwrap_or_default(),
            };
            let mut script: HashMap<PathBuf, Vec<bool>> = HashMap::new();
            hits.retain(|h| {
                !search::component(&h.path)
                    || (script.entry(h.path.clone()).or_insert_with(|| {
                        search::script_lines(&h.path, &read(&h.path)).unwrap_or_default()
                    }))
                    .get(h.line1 - 1)
                    .copied()
                    .unwrap_or(false)
            });
            let haskell = *kind == Some(Kind::Haskell);
            if haskell {
                hits.retain(|h| {
                    let code = haskell_code.entry(h.path.clone()).or_insert_with(|| {
                        let text = read(&h.path);
                        search::HaskellCode::new(&text.lines().collect::<Vec<_>>())
                    });
                    search::symbol_name(&re, &h.text)
                        .is_some_and(|n| search::haskell_symbol(code, h.line1, &n))
                });
            }
            let reserved = |t: &str| {
                haskell
                    && search::symbol_name(&re, t)
                        .is_some_and(|n| search::HASKELL_RESERVED.contains(&n.as_str()))
            };
            if let Some(k @ (search::Kind::Ocaml | search::Kind::Fsharp)) = *kind {
                let mut texts: HashMap<PathBuf, String> = HashMap::new();
                hits.retain(|h| {
                    let text = texts.entry(h.path.clone()).or_insert_with(|| {
                        match (&self.unsaved, &self.current) {
                            (Some(t), Some(c)) if *c == h.path => {
                                String::from_utf8_lossy(t).into_owned()
                            }
                            _ => {
                                std::fs::read_to_string(self.root.join(&h.path)).unwrap_or_default()
                            }
                        }
                    });
                    let lines: Vec<&str> = text.lines().collect();
                    search::ml_symbol_kept(k, &h.path, &lines, h.line1)
                });
            }
            hits.extend(deleted_hits(&self.deleted, wanted, |t| {
                (re.is_match(t) && keep(t) && !reserved(t)).then_some(0)
            }));
            named.extend(hits.into_iter().filter_map(|h| {
                search::symbol_at(&re, &h.text).map(|s| NamedSymbol {
                    name: s.name,
                    byte_col: s.byte_col,
                    hit: h,
                })
            }));
        }
        SymbolHits { named, cut }
    }
}

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
                byte_col: Some(find(&d.text)?),
                path: d.path.clone(),
                line1: d.line,
                text: d.text.clone(),
                deleted: Some(d.at),
            })
        })
        .collect()
}

pub(super) struct FileLine {
    pub(super) path: PathBuf,
    pub(super) line1: usize,
}

#[derive(Debug, Clone)]
pub(super) struct Typed {
    pub(super) name: String,
    pub(super) path: PathBuf,
    pub(super) line: usize,
}

impl Typed {
    pub(super) fn decl(&self) -> FileLine {
        FileLine {
            path: self.path.clone(),
            line1: self.line,
        }
    }
}
