//! Definitions outside the project: the standard library and the installed dependencies.

use super::*;

impl App {
    /// `pattern` over the standard library and dependencies of `kind`, in the module the file's
    /// imports bind `chain` (or `word` itself) to. The module path is relaxed from the end until
    /// files match: `from json import load` is `json/load`, then `json`. Relaxing past the
    /// module the import names (`github.com/foo/bar` down to `github.com`) is a search by name,
    /// and says so. A relative import names a project file, so nothing outside is searched. A
    /// qualifier no import binds is taken as the module path itself: `std::fs::read` spells one,
    /// while a `dotted` qualifier is a value whose name merely matches a module, found by name. A
    /// name nothing installed declares matches no file and the search stops. A bare word no
    /// import binds (`Vec`, `open`) searches every file, by name. Of a TypeScript package
    /// installed more than once, only the copy Node loads is the import's.
    pub(super) fn external_definitions(
        &mut self,
        kind: Kind,
        word: &str,
        chain: &[String],
        dotted: bool,
        pattern: &str,
        imports: &[(String, Vec<String>)],
    ) -> Vec<Candidate> {
        let mut bound_path = bound(imports, chain.first().map_or(word, String::as_str));
        // What a TypeScript import takes (a name, `default`, `*`) is no part of a file's path.
        if kind == Kind::TsJs {
            bound_path.as_mut().map(Vec::pop);
        }
        let imported = bound_path.is_some();
        if bound_path
            .as_ref()
            .and_then(|p| p.first())
            .is_some_and(|p| p.starts_with('.'))
        {
            return Vec::new();
        }
        // The import's own item may go (`from json import load` is `json`), and so may whatever
        // the chain adds past it; the module itself may not.
        let floor = bound_path
            .as_ref()
            .map_or(chain.len(), Vec::len)
            .saturating_sub(1)
            .max(1);
        // A Go import names its package's directory in full, so down to its length a file has
        // to be in that directory, and anything shorter is a search by name (#100).
        let package = bound_path
            .as_ref()
            .filter(|_| kind == Kind::Go)
            .map(Vec::len);
        let floor = package.unwrap_or(floor);
        let all = self.external_files(kind);
        // Of a package installed more than once, the copy Node loads (#141); a name only another
        // copy declares is found by name below. With no copy among the files, all of them count.
        let copy = match (&bound_path, self.external.get(&kind)) {
            (Some(path), Some((roots, _))) if kind == Kind::TsJs => {
                search::package_copy(&self.root, roots, path)
            }
            _ => Some(Vec::new()),
        };
        // A workspace package linked in is the project's own: the search in the project, by
        // name, finds its source, where a published copy outside would be a stale one.
        let Some(copy) = copy else {
            return Vec::new();
        };
        let copy: Vec<PathBuf> = all
            .iter()
            .filter(|p| search::in_copy(p, &copy))
            .cloned()
            .collect();
        let near: &[PathBuf] = if copy.is_empty() { &all } else { &copy };
        let mut module = match chain.first() {
            // A C++ `std::` or `detail::` qualifier names a namespace, and no directory of the
            // system headers is called that, so narrowing by it would find nothing at all.
            Some(_) if kind == Kind::C => None,
            Some(first) => {
                let mut p = bound_path.unwrap_or_else(|| vec![first.clone()]);
                p.extend(chain[1..].iter().cloned());
                Some(p)
            }
            None => bound_path,
        };
        let mut files: Vec<PathBuf> = Vec::new();
        while let Some(m) = &mut module {
            let within = |p: &PathBuf| match package {
                Some(n) if m.len() >= n => search::in_package(p, m),
                _ => search::in_module(p, m),
            };
            files = near.iter().filter(|p| within(p)).cloned().collect();
            if !files.is_empty() {
                break;
            }
            m.pop();
            if m.is_empty() {
                // An import of something not installed: nothing outside says what it is.
                return Vec::new();
            }
        }
        let by_name = |hits: Vec<Hit>| -> Vec<Candidate> {
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::ByName,
                })
                .collect()
        };
        let Some(module) = module else {
            return by_name(self.external_grep(kind, &all, pattern));
        };
        // `from lib import pick` names something at the top of a module: a method called `pick`
        // is not it, however alone it stands (the real one may be native code).
        let top_level =
            imported && chain.len() <= 1 && matches!(kind, Kind::Python | Kind::TsJs | Kind::Go);
        let at_top = |this: &Self, mut hits: Vec<Hit>| {
            if top_level {
                hits.retain(|h| {
                    this.text_of(&h.path)
                        .is_some_and(|t| search::qualified(kind, &t, h.line, word).is_none())
                });
            }
            hits
        };
        let hits = at_top(self, self.external_grep(kind, &files, pattern));
        // An imported module that does not declare the name re-exports it (`std::sync::Arc`
        // lives in `alloc`, a package's `__init__` pulls from its submodules): look everywhere.
        if hits.is_empty() && imported {
            return by_name(at_top(self, self.external_grep(kind, &all, pattern)));
        }
        let sep = match kind {
            Kind::Python => ".",
            Kind::Rust => "::",
            _ => "/",
        };
        let reason = if module.len() < floor || (!imported && dotted) {
            Reason::ByName
        } else if imported {
            Reason::Import(module.join(sep))
        } else {
            Reason::Path(module.join(sep))
        };
        hits.into_iter()
            .map(|hit| Candidate {
                hit,
                reason: reason.clone(),
            })
            .collect()
    }

    /// A grep that came back full stopped at the cap: what `d` counts from it is a lower bound,
    /// also after a filter has made the list short (#100).
    pub(super) fn note_cut(&self, hits: &[Hit]) {
        if hits.len() >= search::MAX_HITS {
            self.truncated.set(true);
        }
    }

    /// `pattern` over `files` outside the project, standard library first. The paths are
    /// absolute: `root.join` leaves them alone, so a hit opens where it is.
    pub(super) fn external_grep(&self, kind: Kind, files: &[PathBuf], pattern: &str) -> Vec<Hit> {
        let roots = self
            .external
            .get(&kind)
            .map(|(roots, _)| roots.as_slice())
            .unwrap_or_default();
        let mut hits = search::grep_project(&self.root, files, pattern, false, false, None, None)
            .unwrap_or_default();
        self.note_cut(&hits);
        hits.sort_by_cached_key(|h| {
            (
                roots.iter().position(|r| h.path.starts_with(r)),
                h.path.clone(),
                h.line,
            )
        });
        hits
    }

    /// The files of `kind` outside the project, walked once per kind.
    ///
    /// ponytail: lives for the session, unlike the project walk. A `pip install` mid-session
    /// needs a restart.
    pub(super) fn external_files(&mut self, kind: Kind) -> Arc<Vec<PathBuf>> {
        // TypeScript's roots depend on where the open file is (#100). Each `node_modules` is
        // walked once; one inside another already listed adds no file of its own.
        let here = self.buf.path.as_ref().and_then(|p| p.parent());
        if let Some(here) = here.filter(|_| kind == Kind::TsJs)
            && (self.node_modules_of.is_some() || !self.external.contains_key(&kind))
            && self.node_modules_of.as_deref() != Some(here)
        {
            let roots = search::node_modules(&self.root, here);
            let mut files = Vec::new();
            for dir in &roots {
                if roots.iter().any(|o| o != dir && dir.starts_with(o)) {
                    continue;
                }
                let walked = self.node_modules.entry(dir.clone()).or_insert_with(|| {
                    Arc::new(search::external_files(kind, std::slice::from_ref(dir)))
                });
                files.extend(walked.iter().cloned());
            }
            self.external.insert(kind, (roots, Arc::new(files)));
            self.node_modules_of = Some(here.to_path_buf());
        }
        if let Some((_, files)) = self.external.get(&kind) {
            return files.clone();
        }
        let roots = search::external_roots(kind, &self.root);
        let files = Arc::new(search::external_files(kind, &roots));
        // No roots may be a toolchain that failed to answer this once: it is asked again on the
        // next `d`, rather than leave the session without a standard library (#183).
        if !roots.is_empty() {
            self.external.insert(kind, (roots, files.clone()));
        }
        files
    }

    /// The text of `path` as the search read it: the open file as it is on screen.
    pub(super) fn text_of(&self, path: &Path) -> Option<String> {
        if self.rel_current().as_deref() == Some(path) {
            Some(self.buf.lines.join("\n"))
        } else {
            std::fs::read_to_string(self.root.join(path)).ok()
        }
    }

    /// Picker rows for `d`: the qualified name, the reason, then `path:line: code`, the columns
    /// padded so the names and reasons line up. An external hit is shown relative to the root it
    /// came from: `json/__init__.py:278:` rather than the whole path to the interpreter.
    pub(super) fn definition_items(
        &self,
        kind: Kind,
        word: &str,
        found: Vec<Candidate>,
    ) -> Vec<PickItem> {
        let roots = self
            .external
            .get(&kind)
            .map(|(roots, _)| roots.as_slice())
            .unwrap_or_default();
        let mut texts: HashMap<PathBuf, Option<String>> = HashMap::new();
        let named: Vec<(String, String, Candidate)> = found
            .into_iter()
            .map(|c| {
                let text = texts
                    .entry(c.hit.path.clone())
                    .or_insert_with(|| self.text_of(&c.hit.path));
                let name = text
                    .as_deref()
                    .and_then(|text| search::qualified(kind, text, c.hit.line, word))
                    .unwrap_or_else(|| word.to_owned());
                (name, c.reason.to_string(), c)
            })
            .collect();
        let pad = |width: usize, s: &str| " ".repeat(width.saturating_sub(wrap::width(s)));
        let name_w = named
            .iter()
            .map(|(n, ..)| wrap::width(n))
            .max()
            .unwrap_or(0);
        let name_w = name_w.min(MAX_NAME_PAD);
        let why_w = named
            .iter()
            .map(|(_, r, _)| wrap::width(r))
            .max()
            .unwrap_or(0);
        named
            .into_iter()
            .map(|(name, why, c)| {
                // A workspace package's own `node_modules` is named from the project root, so
                // its `lib/index.d.ts` reads apart from the one at the top.
                let shown = match kind {
                    Kind::TsJs => roots.last().into_iter().chain([&self.root]).collect(),
                    _ => roots.iter().collect::<Vec<_>>(),
                }
                .into_iter()
                .find_map(|r| c.hit.path.strip_prefix(r).ok())
                .unwrap_or(&c.hit.path);
                let head = format!(
                    "{name}{}  {why}{}  {}:{}: ",
                    pad(name_w, &name),
                    pad(why_w, &why),
                    shown.display(),
                    c.hit.line
                );
                PickItem {
                    code_at: Some(head.len()),
                    label: head + &clip(c.hit.text.trim(), MAX_LABEL_TEXT),
                    path: c.hit.path,
                    line: c.hit.line,
                }
            })
            .collect()
    }

    /// `d` in a file whose kind has no declaration patterns: not "not found", never looked.
    pub(super) fn no_rules(&self) -> String {
        let ext = self.buf.path.as_deref().and_then(Path::extension);
        match ext {
            Some(ext) => format!("no rules for .{}", ext.to_string_lossy()),
            None => "no rules for this file".into(),
        }
    }
}
