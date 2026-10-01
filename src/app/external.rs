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
    /// installed more than once, only the copy Node loads is the import's when `narrow`; `None`
    /// when that copy is a workspace package linked into the project, whose source is the
    /// project's. Without `narrow` it is every copy, as the import names the module.
    pub(super) fn external_definitions(
        &mut self,
        kind: Kind,
        word: &str,
        chain: &[String],
        dotted: bool,
        imports: &[(String, Vec<String>)],
        narrow: bool,
    ) -> Option<Vec<Candidate>> {
        let mut patterns = search::def_patterns(kind, word);
        self.powershell_spelling(kind, word, &mut patterns);
        // A Ruby local is seen from its own method alone, never found by name (#383): outside
        // the project only a constant's assignment declares.
        if kind == Kind::Ruby && word.starts_with(|c: char| c.is_ascii_lowercase() || c == '_') {
            let assignment = search::ruby_assignment(word);
            patterns.retain(|p| *p != assignment);
        }
        let pattern = &patterns.join("|");
        let mut bound_path = bound(imports, chain.first().map_or(word, String::as_str));
        // What a TypeScript import takes (a name, `default`, `*`) is no part of a file's path.
        let taken = match kind {
            Kind::TsJs => bound_path.as_mut().and_then(Vec::pop),
            _ => None,
        };
        // The word is a name the import takes by that name, or a name in the module a `* as ns`
        // import names: what the module declares, under its own name or another. A default
        // import's name is the importer's own.
        let whole = match taken.as_deref() {
            Some("*") => chain.len() == 1,
            Some("default") | None => false,
            Some(_) => chain.is_empty(),
        };
        let imported = bound_path.is_some();
        if bound_path
            .as_ref()
            .and_then(|p| p.first())
            .is_some_and(|p| p.starts_with('.'))
        {
            return Some(Vec::new());
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
        // `@/lib`, `~/lib`, `#lib`: an alias of the project's own module, no package. The search
        // in the project comes first; after it, by name, never narrowed into `@mui`.
        let alias = kind == Kind::TsJs
            && bound_path
                .as_ref()
                .and_then(|p| p.first())
                .is_some_and(|f| f == "@" || f.starts_with(['~', '#']));
        if alias && narrow {
            return None;
        }
        let all = self.external_files(kind);
        // Of a package installed more than once, the files of the copy Node loads (#141); a name
        // only another copy declares is found by name below.
        let copy = match (&bound_path, self.external.get(&kind)) {
            (Some(path), Some((roots, _))) if kind == Kind::TsJs && narrow => {
                search::package_copy(&self.root, roots, &all, &self.files, path)
            }
            _ => Some(search::PackageCopy::default()),
        };
        // A workspace package linked in is the project's own: the search in the project, by
        // name, finds its source, where a published copy outside would be a stale one; the
        // caller searches outside by name only after it.
        let copy = copy?;
        // Only a lookup narrowed to a copy follows the copy's rules below: a renamed export and
        // the module's other copies. Any other matches modules as it always has.
        let narrowed = !copy.files.is_empty();
        // `node:sqlite` is looked for, and named, as `sqlite`.
        if let Some(first) = bound_path.as_mut().and_then(|p| p.first_mut())
            && let Some(bare) = first.strip_prefix("node:")
        {
            *first = bare.to_owned();
        }
        let mut module = match chain.first() {
            // A C++ `std::` or `detail::` qualifier names a namespace, and no directory of the
            // system headers is called that, so narrowing by it would find nothing at all. Ruby's
            // `require` binds no name and a constant is no path: `I18n` is in `i18n.rb` (#369).
            Some(_) if matches!(kind, Kind::C | Kind::Ruby) => None,
            Some(first) => {
                let mut p = bound_path.unwrap_or_else(|| vec![first.clone()]);
                p.extend(chain[1..].iter().cloned());
                Some(p)
            }
            None => bound_path,
        };
        let named = module.clone();
        let mut files: Vec<PathBuf> = Vec::new();
        if let Some(m) = module.as_mut().filter(|_| !alias) {
            // The copy's files of the module, else, for a copy of no walked files, every file's,
            // as without a copy.
            let found = match kind {
                Kind::Elixir => self.elixir_module(&all, m, pattern),
                _ => copy
                    .module(m)
                    .or_else(|| search::module_among(&all, m, package)),
            };
            // An import of something not installed: nothing outside says what it is. A Go
            // package is its own directory, never one above it (#332).
            let Some((n, found)) = found.filter(|(n, _)| package.is_none_or(|k| *n >= k)) else {
                return Some(Vec::new());
            };
            m.truncate(n);
            files = found;
        }
        let by_name = |hits: Vec<Hit>| -> Vec<Candidate> {
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::ByName,
                })
                .collect()
        };
        // What the patterns match, of the lines that declare the word where they sit.
        let grep = |this: &Self, files: &[PathBuf]| {
            this.declaring(kind, word, this.external_grep(kind, files, pattern))
        };
        let Some(module) = module else {
            return Some(by_name(
                match self.rel_current().filter(|_| kind == Kind::C) {
                    Some(here) => self.c_outside(&here, &all, |h| h, grep),
                    None => grep(self, &all),
                },
            ));
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
        let mut hits = at_top(self, grep(self, &files));
        // `export { parseCookie as parse }` is the import's own `parse`, as in the project.
        if hits.is_empty() && narrowed && whole {
            // The package itself is its entry, not every file in it: a chunk, a legacy module.
            let within = match module.len() <= copy.parts {
                true => copy.entries(),
                false => files,
            };
            hits = self.renamed_export(word, |p| self.external_grep(kind, &within, p));
        }
        // An imported module that does not declare the name re-exports it (`std::sync::Arc`
        // lives in `alloc`, a package's `__init__` pulls from its submodules): look everywhere.
        // Past a copy, first where the module's other copies are, as the module's files among
        // all of them: no slower and no noisier than without the copy.
        // Go has no re-exports: a package that does not declare the name is the answer (#332).
        if hits.is_empty() && imported && kind == Kind::Go {
            return Some(Vec::new());
        }
        if hits.is_empty() && imported {
            if narrowed {
                let others = search::module_among(&all, &named.unwrap_or_default(), package);
                let others = others.map(|(_, files)| files).unwrap_or_default();
                hits = at_top(self, grep(self, &others));
            }
            if hits.is_empty() {
                hits = at_top(self, grep(self, &all));
            }
            return Some(by_name(hits));
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
        let found = hits.into_iter().map(|hit| Candidate {
            hit,
            reason: reason.clone(),
        });
        Some(found.collect())
    }

    /// Where the Elixir module `module`, or the one it is nested in, is among `all`, with how many
    /// of its parts that is (#437): the files declaring it when `pattern` finds the word there,
    /// else every file of their package, where a `use` may inject it. A module is no path:
    /// `Phoenix.LiveView` is `phoenix_live_view/lib/phoenix_live_view.ex`, so the package is the
    /// directory under `deps/` whose file declares it. `None` when none does: `Enum` and `String`
    /// ship compiled, and no dependency's namesake is theirs.
    fn elixir_module(
        &self,
        all: &[PathBuf],
        module: &[String],
        pattern: &str,
    ) -> Option<(usize, Vec<PathBuf>)> {
        let roots = self
            .external
            .get(&Kind::Elixir)
            .map(|(roots, _)| roots.clone())
            .unwrap_or_default();
        (1..=module.len()).rev().find_map(|n| {
            let name = regex::escape(&module[..n].join("."));
            let declares = format!(r"^\s*def(?:module|protocol)\s+{name}\s*(?:,|do\b)");
            let mut own: Vec<PathBuf> = self
                .external_grep(Kind::Elixir, all, &declares)
                .into_iter()
                .map(|h| h.path)
                .collect();
            own.dedup();
            if own.is_empty() {
                return None;
            }
            if !self.external_grep(Kind::Elixir, &own, pattern).is_empty() {
                return Some((n, own));
            }
            let packages: Vec<PathBuf> = own
                .iter()
                .filter_map(|f| {
                    let root = roots.iter().find(|r| f.starts_with(r))?;
                    Some(root.join(f.strip_prefix(root).ok()?.components().next()?))
                })
                .collect();
            let files = all
                .iter()
                .filter(|f| packages.iter().any(|p| f.starts_with(p)));
            Some((n, files.cloned().collect()))
        })
    }

    /// The file outside the project that is the Python module `parts` (#333), matched from the
    /// root it lies under, the deepest that holds it: `a/b/c/__init__.py`, `a/b/c.py` or
    /// `a/b/c.pyi` from there, never a `c.py` deeper in some other package. As Python imports it,
    /// the first root holding it wins, and in a root a package over a module beside it; `.py`
    /// over `.pyi`, which a compiled module has alone.
    pub(super) fn external_module(&mut self, parts: &[String]) -> Option<PathBuf> {
        let files = self.external_files(Kind::Python);
        let roots = self
            .external
            .get(&Kind::Python)
            .map(|(roots, _)| roots.clone())
            .unwrap_or_default();
        let name: PathBuf = parts.iter().collect();
        let forms = [
            name.join("__init__.py"),
            name.join("__init__.pyi"),
            name.with_extension("py"),
            name.with_extension("pyi"),
        ];
        files
            .iter()
            .filter_map(|f| {
                let root = roots
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| f.starts_with(r))
                    .max_by_key(|(_, r)| r.components().count())?;
                let rel = f.strip_prefix(root.1).ok()?;
                let form = forms.iter().position(|m| rel == m)?;
                Some(((root.0, form), f))
            })
            .min_by_key(|(rank, _)| *rank)
            .map(|(_, f)| f.clone())
    }

    /// A grep that came back full stopped at the cap: what `d` counts from it is a lower bound,
    /// also after a filter has made the list short (#100).
    pub(super) fn note_cut(&self, hits: &[Hit]) {
        if hits.len() >= search::MAX_HITS {
            self.truncated.set(true);
        }
    }

    /// Where `word`, a member of a JavaScript or DOM global, `JSON.parse`, is declared (#341):
    /// in the `.d.ts` files outside the project, TypeScript's lib and `@types/node`, and in what
    /// the project declares into the global scope, `interface Window { dataLayer }` or `var
    /// appConfig` in a `declare global` ([`search::ts_global_scope`]). The project's hits are
    /// master's search by name, `members` then `pattern`, first.
    pub(super) fn js_global_members(
        &mut self,
        here: &Path,
        word: &str,
        members: Option<&str>,
        pattern: &str,
    ) -> Vec<Candidate> {
        let kind = Kind::TsJs;
        let files: Vec<PathBuf> = (self.external_files(kind).iter())
            .filter(|f| {
                let f = f.to_string_lossy();
                f.ends_with(".d.ts")
                    && (f.contains("typescript/lib/lib.") || f.contains("@types/node/"))
            })
            .cloned()
            .collect();
        let mut patterns = search::def_patterns(kind, word);
        patterns.extend(search::member_or_signature(kind, word).unwrap_or_default());
        let mut found = members
            .map(|m| self.members_by_name(kind, here, word, m))
            .filter(|hits| !hits.is_empty())
            .unwrap_or_else(|| self.project_definitions(kind, here, word, pattern));
        found.retain(|h| {
            self.text_of(&h.path)
                .is_some_and(|text| search::ts_global_scope(&text, h.line))
        });
        found.extend(self.external_grep(kind, &files, &patterns.join("|")));
        found
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
    }

    /// PowerShell's patterns for `word` cut to what its spelling under the cursor allows
    /// ([`search::powershell_sigil`]), as `d` cuts them in the project (#420): `$Error` outside
    /// is no enum member `Error`.
    pub(super) fn powershell_spelling(&self, kind: Kind, word: &str, patterns: &mut Vec<String>) {
        let line = self.line_str();
        if kind == Kind::PowerShell
            && let Some((r, w)) = search::definition_word(Some(kind), line, self.col)
            && w == word
        {
            search::powershell_sigil(patterns, &line[..r.start], &line[r.end..]);
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
        // A function's locals in a dependency are no one's to import (#339).
        if kind == Kind::TsJs {
            let mut texts: HashMap<PathBuf, Vec<String>> = HashMap::new();
            hits.retain(|h| {
                let lines = texts.entry(h.path.clone()).or_insert_with(|| {
                    std::fs::read_to_string(&h.path)
                        .map(|t| t.lines().map(str::to_owned).collect())
                        .unwrap_or_default()
                });
                !search::ts_nested_local(lines, h.line)
            });
        }
        hits.sort_by_cached_key(|h| {
            (
                roots.iter().position(|r| h.path.starts_with(r)),
                h.path.clone(),
                h.line,
            )
        });
        hits
    }

    /// The methods `members` matches in the Python `files` outside, and whether a field `word`
    /// is declared there too (#342): a class-body `word = …` or `word: T`, a `self.word = …` in a
    /// method, read as [`search::field_rows`] reads the project's. One pass over the files for
    /// both. Fields outside are never listed, there are too many: a few hundred candidate lines
    /// are enough to tell whether one declares a field.
    pub(super) fn external_methods(
        &self,
        files: &[PathBuf],
        members: &str,
        word: &str,
    ) -> (Vec<Hit>, bool) {
        let Some(fields) = search::field_patterns(Kind::Python, word) else {
            return (self.external_grep(Kind::Python, files, members), false);
        };
        let method = Regex::new(members).expect("built-in patterns compile");
        let pattern = format!("{members}|{}", fields.join("|"));
        let kept = std::cell::Cell::new(0);
        // ponytail: the first 500 field-shaped lines; a field past them goes unseen.
        let hits = search::grep_filtered(&self.root, files, &pattern, None, None, |l| {
            method.is_match(l) || {
                kept.set(kept.get() + 1);
                kept.get() <= 500
            }
        })
        .unwrap_or_default();
        let (mut methods, candidates): (Vec<Hit>, Vec<Hit>) =
            hits.into_iter().partition(|h| method.is_match(&h.text));
        // A `def` in a function's body is a local of it, no method (#338).
        let mut texts: HashMap<PathBuf, Option<String>> = HashMap::new();
        methods.retain(|h| {
            let text = texts
                .entry(h.path.clone())
                .or_insert_with(|| std::fs::read_to_string(&h.path).ok());
            !text
                .as_deref()
                .is_some_and(|t| search::python_in_function(t, h.line))
        });
        self.note_cut(&methods);
        let roots = self
            .external
            .get(&Kind::Python)
            .map(|(roots, _)| roots.as_slice())
            .unwrap_or_default();
        methods.sort_by_cached_key(|h| {
            (
                roots.iter().position(|r| h.path.starts_with(r)),
                h.path.clone(),
                h.line,
            )
        });
        let mut by_file: Vec<(PathBuf, Vec<usize>)> = Vec::new();
        for h in candidates {
            match by_file.last_mut() {
                Some((path, lines)) if *path == h.path => lines.push(h.line),
                _ => by_file.push((h.path, vec![h.line])),
            }
        }
        let field = by_file.into_iter().any(|(path, lines)| {
            std::fs::read_to_string(&path).is_ok_and(|t| {
                !search::field_rows(Kind::Python, &t, &lines, word, false).is_empty()
            })
        });
        (methods, field)
    }

    /// Test helper: nothing is installed outside the project, so a lookup reads no library of
    /// the machine the tests run on.
    #[cfg(test)]
    pub(crate) fn no_external(&mut self) {
        // Every kind: a new one fails to compile here until it is in the list below too.
        let every = |kind: Kind| match kind {
            Kind::Python
            | Kind::Go
            | Kind::Rust
            | Kind::TsJs
            | Kind::Jvm
            | Kind::Ruby
            | Kind::C
            | Kind::CSharp
            | Kind::Swift
            | Kind::Php
            | Kind::Lua
            | Kind::Elixir
            | Kind::Zig
            | Kind::Proto
            | Kind::Shell
            | Kind::PowerShell
            | Kind::Sql
            | Kind::Make
            | Kind::Terraform
            | Kind::Docker
            | Kind::Yaml
            | Kind::Markdown
            | Kind::Graphql => kind,
        };
        for kind in [
            Kind::Python,
            Kind::Go,
            Kind::Rust,
            Kind::TsJs,
            Kind::Jvm,
            Kind::Ruby,
            Kind::C,
            Kind::CSharp,
            Kind::Swift,
            Kind::Php,
            Kind::Lua,
            Kind::Elixir,
            Kind::Zig,
            Kind::Proto,
            Kind::Shell,
            Kind::PowerShell,
            Kind::Sql,
            Kind::Make,
            Kind::Terraform,
            Kind::Docker,
            Kind::Yaml,
            Kind::Markdown,
            Kind::Graphql,
        ]
        .map(every)
        {
            self.external
                .insert(kind, (Vec::new(), Arc::new(Vec::new())));
        }
    }

    /// The files of `kind` outside the project, walked once per kind.
    ///
    /// ponytail: lives for the session, unlike the project walk. A `pip install` mid-session
    /// needs a restart.
    pub(super) fn external_files(&mut self, kind: Kind) -> Arc<Vec<PathBuf>> {
        // TypeScript's roots depend on where the open file is (#100). Each `node_modules` is
        // walked once; one inside another already listed adds no file of its own. They are
        // inside the project, so a test's `no_external`, which lists no root, does not hide
        // them either; roots a test lists are kept.
        let here = self.buf.path.as_ref().and_then(|p| p.parent());
        if let Some(here) = here.filter(|_| kind == Kind::TsJs)
            && (self.node_modules_of.is_some()
                || self.external.get(&kind).is_none_or(|(r, _)| r.is_empty()))
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
        // Elixir's are the `deps/` of the Mix project the file is in (#437), inside the project
        // and not of the machine, so a test's `no_external` does not hide them either.
        if let Some(here) = here.filter(|_| kind == Kind::Elixir) {
            let roots = search::mix_deps(&self.root, here);
            if self.external.get(&kind).is_none_or(|(r, _)| *r != roots) {
                let files = Arc::new(search::external_files(kind, &roots));
                self.external.insert(kind, (roots, files));
            }
        }
        let files = match self.external.get(&kind) {
            Some((_, files)) => files.clone(),
            None => {
                let roots = search::external_roots(kind, &self.root);
                let files = Arc::new(search::external_files(kind, &roots));
                // No roots may be a toolchain that failed to answer this once: it is asked
                // again on the next `d`, rather than leave the session without a standard
                // library (#183).
                if !roots.is_empty() {
                    self.external.insert(kind, (roots, files.clone()));
                }
                files
            }
        };
        if kind != Kind::C {
            return files;
        }
        // A `.c` file cannot include a C++ header: no `c++/` directory is its (#382). Only an
        // Objective-C file reads the frameworks and `Pods/` (#417): any other keeps the files
        // it had before them.
        let key = (self.c_source(), self.objc_file());
        if let Some((from, kept)) = self.c_files.get(&key)
            && Arc::ptr_eq(from, &files)
        {
            return kept.clone();
        }
        let away: Vec<&PathBuf> = match self.external.get(&kind) {
            Some((roots, _)) if !key.1 => roots.iter().filter(|r| search::objc_root(r)).collect(),
            _ => Vec::new(),
        };
        let kept: Arc<Vec<PathBuf>> = Arc::new(
            (files.iter())
                .filter(|f| !key.0 || !f.components().any(|c| c.as_os_str() == "c++"))
                .filter(|f| !away.iter().any(|r| f.starts_with(r)))
                .cloned()
                .collect(),
        );
        self.c_files.insert(key, (files, kept.clone()));
        kept
    }

    /// Whether the open file is C source, `.c`, or Objective-C's `.m` (#417), which read no C++
    /// header; `.mm` does.
    pub(super) fn c_source(&self) -> bool {
        (self.buf.path.as_deref())
            .is_some_and(|p| p.extension().is_some_and(|e| e == "c" || e == "m"))
    }

    /// Whether the open file is Objective-C ([`search::objc_file`]).
    pub(super) fn objc_file(&self) -> bool {
        (self.buf.path.as_deref())
            .is_some_and(|p| search::objc_file(p, || self.buf.lines.join("\n")))
    }

    /// Of `hits` of the [`search::def_patterns`] of `word`, the lines that declare it where they
    /// sit ([`search::declares_where`]). An Objective-C line declares for an Objective-C file
    /// alone: a C++ `load` is no `+ (void)load;` of `objc/NSObject.h` (#417).
    pub(super) fn declaring(&self, kind: Kind, word: &str, mut hits: Vec<Hit>) -> Vec<Hit> {
        // One file holds thousands of GraphQL `id` fields, so each file is split once.
        let mut lines: HashMap<PathBuf, Vec<String>> = HashMap::new();
        let objc_only = (kind == Kind::C && !self.objc_file()).then(|| search::objc_only(word));
        hits.retain(|h| {
            objc_only.as_ref().is_none_or(|only| !only(&h.text))
                && search::declares_where(kind, &h.path, word, h.line, &h.text, || {
                    lines.entry(h.path.clone()).or_insert_with(|| {
                        self.text_of(&h.path)
                            .map_or_else(Vec::new, |t| t.lines().map(str::to_owned).collect())
                    })
                })
        });
        hits
    }

    /// The text of `path` as the search read it: the open file as it is on screen. A
    /// component's lines outside its script are blank (#413).
    pub(super) fn text_of(&self, path: &Path) -> Option<String> {
        let text = self.file_text(path)?;
        Some(search::script_text(path, &text, None).into_owned())
    }

    /// The text of `path` as it is, a component's template included: the open file as it is on
    /// screen.
    pub(super) fn file_text(&self, path: &Path) -> Option<String> {
        match self.rel_current().as_deref() == Some(path) {
            true => Some(self.buf.lines.join("\n")),
            false => std::fs::read_to_string(self.root.join(path)).ok(),
        }
    }

    /// The text `h` was read from: the file as the search read it, or for a line the branch
    /// deleted, the file at the base, under the name it had there (#440).
    pub(super) fn hit_text(&self, h: &Hit) -> Option<String> {
        let text = self.hit_file_text(h)?;
        Some(search::script_text(&h.path, &text, None).into_owned())
    }

    /// [`Self::hit_text`] as it is, a component's template included.
    fn hit_file_text(&self, h: &Hit) -> Option<String> {
        match h.deleted {
            None => self.file_text(&h.path),
            Some(_) => self.base_text(&h.path),
        }
    }

    /// The text of `path` at the base of the review, under the name it had there (#440).
    fn base_text(&self, path: &Path) -> Option<String> {
        let r = self.review.as_ref()?;
        let from = r.file(path).and_then(|f| f.old.as_deref());
        let bytes = r.base_bytes(&self.root, from.unwrap_or(path)).ok()?;
        Some(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// [`search::hidden_lines`] of the text `h` was read from: the lines where a
    /// declaration-shaped line declares nothing.
    pub(super) fn hidden_of(&self, kind: Kind, h: &Hit) -> Vec<bool> {
        (self.hit_file_text(h)).map_or_else(Vec::new, |t| search::hidden_lines(kind, &h.path, &t))
    }

    /// The lines of `h`'s file where a declaration-shaped line declares nothing, as `d` filters
    /// its candidates: a component's from the text the hit was read from (#413), any other file's
    /// from its text now.
    pub(super) fn hidden_now(&self, kind: Kind, h: &Hit) -> Vec<bool> {
        match search::component(&h.path) {
            true => self.hidden_of(kind, h),
            false => {
                (self.text_of(&h.path)).map_or_else(Vec::new, |t| search::literal_lines(kind, &t))
            }
        }
    }

    /// The lines the branch deleted that `D` lists: a component's, only those of its script at
    /// the base (#413).
    pub(super) fn symbol_deleted(&self) -> Arc<Vec<git::DeletedLine>> {
        let all = self.deleted_lines();
        let mut code: HashMap<PathBuf, Vec<bool>> = HashMap::new();
        let kept = (all.iter()).filter(|d| {
            !search::component(&d.path)
                || (code.entry(d.path.clone()))
                    .or_insert_with(|| {
                        let text = self.base_text(&d.path).unwrap_or_default();
                        search::script_lines(&d.path, &text).unwrap_or_default()
                    })
                    .get(d.line - 1)
                    .is_some_and(|&c| c)
        });
        Arc::new(kept.cloned().collect())
    }

    /// `path` relative to the standard library or dependency root of `kind` it is under,
    /// `json/__init__.py` rather than the whole path to the interpreter; any other path as it
    /// is. A root that is one package keeps its name, `serde-1.0.200/src/lib.rs`, or its files
    /// would read like the project's own. A workspace package's own `node_modules` is named from
    /// the project root, so its `lib/index.d.ts` reads apart from the one at the top.
    pub(super) fn rel_to_its_root<'a>(&self, kind: Kind, path: &'a Path) -> &'a Path {
        let roots = self
            .external
            .get(&kind)
            .map(|(roots, _)| roots.as_slice())
            .unwrap_or_default();
        match kind {
            Kind::TsJs => roots.last().into_iter().chain([&self.root]).collect(),
            // `deps/` is inside the project: `deps/jason/lib/jason.ex`.
            Kind::Elixir => vec![&self.root],
            _ => roots.iter().collect::<Vec<_>>(),
        }
        .into_iter()
        .find(|r| path.starts_with(r))
        .map_or(path, |r| {
            let keep = if *r == self.root { 0 } else { package_dirs(r) };
            let from = r.ancestors().nth(keep).unwrap_or(r);
            path.strip_prefix(from).unwrap_or(path)
        })
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
        let mut texts: HashMap<(PathBuf, bool), Option<String>> = HashMap::new();
        let named: Vec<(String, String, Candidate)> = found
            .into_iter()
            .map(|c| {
                let text = texts
                    .entry((c.hit.path.clone(), c.hit.deleted.is_some()))
                    .or_insert_with(|| self.hit_text(&c.hit));
                let name = text
                    .as_deref()
                    .filter(|_| !matches!(c.reason, Reason::Module(_)))
                    .and_then(|text| {
                        let word = definition::declared_as(kind, word, &c.hit.text);
                        search::qualified(kind, text, c.hit.line, &word)
                    })
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
                let head = format!(
                    "{name}{}  {why}{}  {}: ",
                    pad(name_w, &name),
                    pad(why_w, &why),
                    at_label(self.rel_to_its_root(kind, &c.hit.path), c.hit.line),
                );
                PickItem {
                    code_at: Some(head.len()),
                    col: word_col(
                        &c.hit.text,
                        &definition::declared_as(kind, word, &c.hit.text),
                        search::word_chars(Some(kind), true),
                    ),
                    label: head + &clip(c.hit.text.trim(), MAX_LABEL_TEXT),
                    deleted: c.hit.deleted.is_some(),
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

/// How many directories at the end of `root` name its package. A root named with a version is
/// one package, and its own name counts: Cargo gives each crate a directory of its own,
/// `serde-1.0.200`, and Go each module, `gin@v1.9.1`. A Go module whose path ends in its major
/// version, `github.com/jackc/pgx/v5`, is `v5@v5.5.0` on disk, so the directory before it counts
/// too, as a gem's directory counts before its `lib`. The other roots hold many packages and
/// have no version in their name: a standard library (`python3.13`, `src`, `library`),
/// `site-packages`, `node_modules`.
fn package_dirs(root: &Path) -> usize {
    let name = root.file_name().unwrap_or_default().to_string_lossy();
    let versioned = |sep: &str| {
        name.match_indices(sep).any(|(i, _)| {
            let v = &name[i + sep.len()..];
            let rest = v.trim_start_matches(|c: char| c.is_ascii_digit());
            rest.len() < v.len() && rest.starts_with('.')
        })
    };
    // A gem's `lib` is the gem's (#369): `rack-attack-6.7.0/lib`.
    if name == "lib" {
        return root.parent().map_or(0, |gem| match package_dirs(gem) {
            0 => 0,
            n => n + 1,
        });
    }
    if !versioned("-") && !versioned("@v") {
        return 0;
    }
    let major = name
        .strip_prefix('v')
        .and_then(|v| v.split_once('@'))
        .is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    1 + usize::from(major)
}
