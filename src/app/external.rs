use super::{
    App, Arc, Candidate, HashMap, HashSet, Hit, Kind, MAX_LABEL_TEXT, MAX_NAME_PAD, Path, PathBuf,
    PickItem, Reason, Regex, at_label, bound, clip, git, jvm, search, word_col, wrap,
};

pub(super) struct OutsideMembers {
    pub methods: Vec<Hit>,
    pub unlisted_field_declares_it: bool,
}
impl OutsideMembers {
    fn listed(methods: Vec<Hit>) -> Self {
        Self {
            methods,
            unlisted_field_declares_it: false,
        }
    }
}

struct ModuleLookup<'a> {
    kind: Kind,
    word: &'a str,
    chain: &'a [String],
    pattern: &'a str,
    all: &'a [PathBuf],
    files: Vec<PathBuf>,
    copy: &'a search::PackageCopy,
    word_is_export_name: bool,
    imported: bool,
    named: Option<Vec<String>>,
    package: Option<usize>,
    floor: usize,
    reason: Reason,
    narrow: bool,
}

fn by_name(hits: Vec<Hit>) -> Vec<Candidate> {
    hits.into_iter()
        .map(|hit| Candidate {
            hit,
            reason: Reason::ByName,
        })
        .collect()
}

fn handed_on_name(
    named: &[String],
    module_len: usize,
    chain: &[String],
    word: &str,
) -> Option<String> {
    let mut inside = named[module_len..].to_vec();
    if !chain.is_empty() {
        inside.push(word.to_owned());
    }
    match inside.as_slice() {
        [name] => Some(name.clone()),
        _ => None,
    }
}

fn module_reason(
    kind: Kind,
    module: &[String],
    floor: usize,
    (imported, dotted): (bool, bool),
) -> Reason {
    let sep = match kind {
        Kind::Python => ".",
        Kind::Rust => "::",
        _ => "/",
    };
    if module.len() < floor || (!imported && dotted) {
        Reason::ByName
    } else if imported {
        Reason::Import(module.join(sep))
    } else {
        Reason::Path(module.join(sep))
    }
}

impl App {
    pub(super) fn external_definitions(
        &mut self,
        kind: Kind,
        word: &str,
        chain: &[String],
        dotted: bool,
        imports: &[search::Import],
        narrow: bool,
    ) -> Option<Vec<Candidate>> {
        let pattern = &self.external_pattern(kind, word);
        let mut bound_path = bound(imports, chain.first().map_or(word, String::as_str));
        let ts_import_item = match kind {
            Kind::TsJs => bound_path.as_mut().and_then(Vec::pop),
            _ => None,
        };
        let word_is_export_name = match ts_import_item.as_deref() {
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
        let floor = bound_path
            .as_ref()
            .map_or(chain.len(), Vec::len)
            .saturating_sub(1)
            .max(1);
        let package = bound_path
            .as_ref()
            .filter(|_| kind == Kind::Go)
            .map(Vec::len);
        let floor = package.unwrap_or(floor);
        let alias = kind == Kind::TsJs
            && bound_path
                .as_ref()
                .and_then(|p| p.first())
                .is_some_and(|f| f == "@" || f.starts_with(['~', '#']));
        if alias && narrow {
            return None;
        }
        let all = self.external_files(kind);
        let copy = match (&bound_path, self.external.get(&kind)) {
            (Some(path), Some((roots, _))) if kind == Kind::TsJs && narrow => {
                search::package_copy(&self.root, roots, &all, &self.files, path)
            }
            _ => Some(search::PackageCopy::default()),
        };
        let copy = copy?;
        if let Some(first) = bound_path.as_mut().and_then(|p| p.first_mut())
            && let Some(bare) = first.strip_prefix("node:")
        {
            *first = bare.to_owned();
        }
        let mut module = match chain.first() {
            Some(_) if matches!(kind, Kind::C | Kind::Ruby | Kind::Dart) => None,
            Some(first) => {
                let mut p = bound_path.unwrap_or_else(|| vec![first.clone()]);
                p.extend(chain[1..].iter().cloned());
                Some(p)
            }
            None => bound_path,
        };
        let named = module.clone();
        let files = match module.as_mut().filter(|_| !alias) {
            Some(m) => match self.module_files(kind, m, (&all, &copy), pattern, package) {
                Some(files) => files,
                None => return Some(Vec::new()),
            },
            None => Vec::new(),
        };
        let Some(module) = module else {
            return Some(by_name(
                match self.rel_current().filter(|_| kind == Kind::C) {
                    Some(here) => self.c_outside(
                        &here,
                        &all,
                        |h| h,
                        |this, files| this.declarations_among(kind, word, pattern, files),
                    ),
                    None => self.declarations_among(kind, word, pattern, &all),
                },
            ));
        };
        let lookup = ModuleLookup {
            kind,
            word,
            chain,
            pattern,
            all: &all,
            files,
            copy: &copy,
            word_is_export_name,
            imported,
            named,
            package,
            floor,
            reason: module_reason(kind, &module, floor, (imported, dotted)),
            narrow,
        };
        Some(self.module_definitions(&lookup, &module))
    }

    fn module_files(
        &self,
        kind: Kind,
        m: &mut Vec<String>,
        (all, copy): (&[PathBuf], &search::PackageCopy),
        pattern: &str,
        package: Option<usize>,
    ) -> Option<Vec<PathBuf>> {
        let found = match kind {
            Kind::Elixir => self.elixir_module(all, m, pattern),
            Kind::Python => self.python_module_among(all, m),
            _ => copy
                .module(m)
                .or_else(|| search::module_among(all, m, package)),
        };
        let found = found.filter(|f| package.is_none_or(|k| f.matched_parts >= k))?;
        m.truncate(found.matched_parts);
        Some(found.files)
    }

    fn external_pattern(&self, kind: Kind, word: &str) -> String {
        let mut patterns = search::def_patterns(kind, word);
        self.spelling_cut(kind, word, &mut patterns);
        if kind == Kind::Ruby && word.starts_with(|c: char| c.is_ascii_lowercase() || c == '_') {
            let assignment = search::ruby_assignment(word);
            patterns.retain(|p| *p != assignment);
        }
        patterns.join("|")
    }

    fn declarations_among(
        &self,
        kind: Kind,
        word: &str,
        pattern: &str,
        files: &[PathBuf],
    ) -> Vec<Hit> {
        let erlang = self.buf.path.as_deref().is_some_and(search::erlang);
        let mut hits = self.declaring(
            kind,
            word,
            self.outside_grep(kind, files, pattern, Some(word)),
        );
        hits.retain(|h| kind != Kind::Elixir || search::erlang(&h.path) == erlang);
        hits
    }

    fn module_definitions(&mut self, q: &ModuleLookup, module: &[String]) -> Vec<Candidate> {
        let (kind, word, imported) = (q.kind, q.word, q.imported);
        let top_level =
            imported && q.chain.len() <= 1 && matches!(kind, Kind::Python | Kind::TsJs | Kind::Go);
        let at_top = |this: &Self, mut hits: Vec<Hit>| {
            if top_level {
                hits.retain(|h| {
                    this.text_of(&h.path)
                        .is_some_and(|t| search::qualified(kind, &t, h.line1, word).is_none())
                });
            }
            hits
        };
        let narrowed_to_copy = !q.copy.files.is_empty();
        let mut hits = at_top(
            self,
            self.declarations_among(kind, word, q.pattern, &q.files),
        );
        if hits.is_empty() && narrowed_to_copy && q.word_is_export_name {
            let within = if module.len() <= q.copy.name_parts {
                q.copy.entries()
            } else {
                q.files.clone()
            };
            hits = self.renamed_export(word, |p| self.external_grep(kind, &within, p));
        }
        let walked = if hits.is_empty()
            && imported
            && kind == Kind::Python
            && module.len() >= q.floor
            && let Some(name) = q
                .named
                .as_deref()
                .and_then(|named| handed_on_name(named, module.len(), q.chain, word))
        {
            let found = self.outside_handed_on(module, &name, 0);
            if !found.is_empty() {
                return found;
            }
            true
        } else {
            false
        };
        if hits.is_empty() && imported && kind == Kind::Go {
            return Vec::new();
        }
        if hits.is_empty() && imported {
            if narrowed_to_copy {
                let named = q.named.clone().unwrap_or_default();
                let others = search::module_among(q.all, &named, q.package);
                let others = others.map(|m| m.files).unwrap_or_default();
                hits = at_top(
                    self,
                    self.declarations_among(kind, word, q.pattern, &others),
                );
            }
            if hits.is_empty() {
                hits = at_top(self, self.declarations_among(kind, word, q.pattern, q.all));
            }
            self.offer_only |= walked && !hits.is_empty();
            return by_name(hits);
        }
        let mut found: Vec<Candidate> = hits
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: q.reason.clone(),
            })
            .collect();
        if kind == Kind::Julia
            && imported
            && q.narrow
            && q.chain.is_empty()
            && !found.is_empty()
            && let Some(here) = self.rel_current()
        {
            let own = self.project_definitions(kind, &here, word, q.pattern);
            found.extend(own.into_iter().map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            }));
        }
        found
    }

    fn elixir_module(
        &self,
        all: &[PathBuf],
        module: &[String],
        pattern: &str,
    ) -> Option<search::ModuleFiles<PathBuf>> {
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
                return Some(search::ModuleFiles {
                    matched_parts: n,
                    files: own,
                });
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
            Some(search::ModuleFiles {
                matched_parts: n,
                files: files.cloned().collect(),
            })
        })
    }

    pub(super) fn python_module_among(
        &self,
        all: &[PathBuf],
        module: &[String],
    ) -> Option<search::ModuleFiles<PathBuf>> {
        let roots = self
            .external
            .get(&Kind::Python)
            .map(|(roots, _)| roots.as_slice())
            .unwrap_or_default();
        let rels: Vec<(&Path, &PathBuf)> = all
            .iter()
            .filter_map(|f| Some((python_rel(roots, f)?.1, f)))
            .collect();
        (1..=module.len()).rev().find_map(|n| {
            let name: PathBuf = module[..n].iter().collect();
            let forms = [name.with_extension("py"), name.with_extension("pyi")];
            let found: Vec<PathBuf> = (rels.iter())
                .filter(|(rel, _)| {
                    (rel.starts_with(&name) && *rel != name) || forms.contains(&rel.to_path_buf())
                })
                .map(|(_, f)| (*f).clone())
                .collect();
            (!found.is_empty()).then_some(search::ModuleFiles {
                matched_parts: n,
                files: found,
            })
        })
    }

    pub(super) fn outside_members(
        &self,
        kind: Kind,
        all: &[PathBuf],
        files: &[PathBuf],
        imports: &[search::Import],
        members: &str,
        word: &str,
    ) -> OutsideMembers {
        match kind {
            Kind::Python => self.python_members(files, imports, members, word),
            Kind::TsJs => match self.ts_imported_members(all, imports, members) {
                Some(hits) => OutsideMembers::listed(hits),
                None => OutsideMembers::listed(self.outside_grep(kind, files, members, Some(word))),
            },
            _ => OutsideMembers::listed(self.external_grep(kind, files, members)),
        }
    }

    fn ts_imported_members(
        &self,
        all: &[PathBuf],
        imports: &[search::Import],
        members: &str,
    ) -> Option<Vec<Hit>> {
        let (roots, _) = self.external.get(&Kind::TsJs)?;
        let mut packages: Vec<&[String]> = (imports.iter())
            .filter_map(|search::Import { path, .. }| {
                let first = path.first().filter(|f| !f.starts_with(['.', '/']))?;
                let parts = 1 + usize::from(first.starts_with('@'));
                path.get(..parts).filter(|_| path.len() > parts)
            })
            .collect();
        packages.sort();
        packages.dedup();
        let mut reached: Vec<PathBuf> = (packages.into_iter())
            .filter_map(|p| search::package_copy(&self.root, roots, all, &self.files, p))
            .flat_map(|copy| copy.files)
            .filter(|f| search::declaration_file(f))
            .collect();
        reached.sort();
        reached.dedup();
        let hits = self.external_grep(Kind::TsJs, &reached, members);
        (hits.len() > 1).then_some(hits)
    }

    fn python_members(
        &self,
        files: &[PathBuf],
        imports: &[search::Import],
        members: &str,
        word: &str,
    ) -> OutsideMembers {
        let reached = self.python_imported(files, imports);
        match self.external_methods(&reached, members, word) {
            found if found.methods.len() > 1 => found,
            _ => self.external_methods(files, members, word),
        }
    }

    fn python_imported(&self, files: &[PathBuf], imports: &[search::Import]) -> Vec<PathBuf> {
        let mut packages: Vec<&String> = (imports.iter())
            .filter_map(|i| i.path.first())
            .filter(|p| !p.starts_with('.'))
            .collect();
        packages.sort();
        packages.dedup();
        (packages.into_iter())
            .filter_map(|p| self.python_module_among(files, std::slice::from_ref(p)))
            .flat_map(|m| m.files)
            .collect()
    }

    pub(super) fn external_module(&mut self, parts: &[String]) -> Option<PathBuf> {
        self.external_files(Kind::Python);
        self.walked_module(parts)
    }

    pub(super) fn walked_module(&self, parts: &[String]) -> Option<PathBuf> {
        let (roots, files) = self.external.get(&Kind::Python)?;
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
                let (root, rel) = python_rel(roots, f)?;
                let form = forms.iter().position(|m| rel == m)?;
                Some(((root, form), f))
            })
            .min_by_key(|(rank, _)| *rank)
            .map(|(_, f)| f.clone())
    }

    pub(super) fn note_cut(&self, hits: &[Hit]) {
        if hits.len() >= search::MAX_HITS {
            self.truncated.set(true);
        }
    }

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
                .is_some_and(|text| search::ts_global_scope(&text, h.line1))
        });
        found.extend(self.outside_grep(kind, &files, &patterns.join("|"), Some(word)));
        found
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
    }

    pub(super) fn spelling_cut(&self, kind: Kind, word: &str, patterns: &mut Vec<String>) {
        if matches!(kind, Kind::PowerShell | Kind::Dart | Kind::Julia)
            && let Some((r, w)) = self.word_here(Some(kind))
            && w == word
        {
            self.narrow(kind, patterns, "", r);
        }
    }

    pub(super) fn narrow(
        &self,
        kind: Kind,
        p: &mut Vec<String>,
        text: &str,
        r: std::ops::Range<usize>,
    ) {
        search::narrow_patterns(kind, p, text, self.line, self.line_str(), r);
    }

    pub(super) fn external_grep(&self, kind: Kind, files: &[PathBuf], pattern: &str) -> Vec<Hit> {
        self.outside_grep(kind, files, pattern, None)
    }

    pub(super) fn outside_grep(
        &self,
        kind: Kind,
        files: &[PathBuf],
        pattern: &str,
        word: Option<&str>,
    ) -> Vec<Hit> {
        let roots = self
            .external
            .get(&kind)
            .map(|(roots, _)| roots.as_slice())
            .unwrap_or_default();
        let shape = word
            .filter(|_| kind == Kind::TsJs)
            .and_then(|w| search::ts_shape(w, pattern));
        let mut hits = match shape {
            Some(shape) => search::grep_shaped(files, pattern, shape, &self.shaped),
            None => search::grep_project(&self.root, files, pattern, false, false, None, None),
        }
        .unwrap_or_default();
        self.note_cut(&hits);
        if kind == Kind::TsJs {
            let mut texts: HashMap<PathBuf, Vec<String>> = HashMap::new();
            hits.retain(|h| {
                let lines = texts.entry(h.path.clone()).or_insert_with(|| {
                    std::fs::read_to_string(&h.path)
                        .map(|t| t.lines().map(str::to_owned).collect())
                        .unwrap_or_default()
                });
                !search::ts_nested_local(lines, h.line1)
            });
            hits = search::drop_farther_copies(roots, hits);
        }
        hits.sort_by_cached_key(|h| {
            (
                roots.iter().position(|r| h.path.starts_with(r)),
                h.path.clone(),
                h.line1,
            )
        });
        hits
    }

    pub(super) fn external_methods(
        &self,
        files: &[PathBuf],
        members: &str,
        word: &str,
    ) -> OutsideMembers {
        let Some(fields) = search::field_patterns(Kind::Python, word) else {
            return OutsideMembers::listed(self.external_grep(Kind::Python, files, members));
        };
        let method = Regex::new(members).expect("built-in patterns compile");
        let pattern = format!("{members}|{}", fields.join("|"));
        let kept = std::sync::atomic::AtomicUsize::new(0);
        let hits = search::grep_filtered(&self.root, files, &pattern, None, None, |l| {
            method.is_match(l) || kept.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 500
        })
        .unwrap_or_default();
        let (mut methods, candidates): (Vec<Hit>, Vec<Hit>) =
            hits.into_iter().partition(|h| method.is_match(&h.text));
        let mut texts: HashMap<PathBuf, Option<String>> = HashMap::new();
        methods.retain(|h| {
            let text = texts
                .entry(h.path.clone())
                .or_insert_with(|| std::fs::read_to_string(&h.path).ok());
            !text
                .as_deref()
                .is_some_and(|t| search::python_in_function(t, h.line1))
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
                h.line1,
            )
        });
        let mut by_file: Vec<(PathBuf, Vec<usize>)> = Vec::new();
        for h in candidates {
            match by_file.last_mut() {
                Some((path, lines)) if *path == h.path => lines.push(h.line1),
                _ => by_file.push((h.path, vec![h.line1])),
            }
        }
        let unlisted_field_declares_it = by_file.into_iter().any(|(path, lines)| {
            std::fs::read_to_string(&path).is_ok_and(|t| {
                !search::field_rows(Kind::Python, &t, &lines, word, false).is_empty()
            })
        });
        OutsideMembers {
            methods,
            unlisted_field_declares_it,
        }
    }

    #[cfg(test)]
    pub(crate) fn no_external(&mut self) {
        let new_kind_fails_to_compile_until_listed = |kind: Kind| match kind {
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
            | Kind::Dart
            | Kind::Cmake
            | Kind::Nix
            | Kind::Haskell
            | Kind::Ocaml
            | Kind::Fsharp
            | Kind::Julia
            | Kind::R
            | Kind::Perl
            | Kind::Gdscript
            | Kind::Solidity
            | Kind::Clojure
            | Kind::EmacsLisp
            | Kind::Scheme
            | Kind::CommonLisp
            | Kind::Starlark
            | Kind::Sql
            | Kind::Make
            | Kind::Terraform
            | Kind::Docker
            | Kind::Yaml
            | Kind::Markdown
            | Kind::Graphql
            | Kind::Css
            | Kind::Html => kind,
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
            Kind::Dart,
            Kind::Cmake,
            Kind::Nix,
            Kind::Haskell,
            Kind::Ocaml,
            Kind::Fsharp,
            Kind::Julia,
            Kind::R,
            Kind::Perl,
            Kind::Gdscript,
            Kind::Solidity,
            Kind::Clojure,
            Kind::EmacsLisp,
            Kind::Scheme,
            Kind::CommonLisp,
            Kind::Starlark,
            Kind::Sql,
            Kind::Make,
            Kind::Terraform,
            Kind::Docker,
            Kind::Yaml,
            Kind::Markdown,
            Kind::Graphql,
            Kind::Css,
            Kind::Html,
        ]
        .map(new_kind_fails_to_compile_until_listed)
        {
            self.external
                .insert(kind, (Vec::new(), Arc::new(Vec::new())));
        }
        self.otp = Some(Vec::new());
    }

    pub fn warm_up(&mut self) {
        let Some(kind) = self.kind() else { return };
        if matches!(kind, Kind::TsJs | Kind::Elixir | Kind::Solidity)
            || self.external.contains_key(&kind)
            || self.warming.contains_key(&kind)
        {
            return;
        }
        let root = self.root.clone();
        let walk = std::thread::spawn(move || walk_outside(kind, &root));
        self.warming.insert(kind, Some(walk));
    }

    pub(super) fn external_files(&mut self, kind: Kind) -> Arc<Vec<PathBuf>> {
        if kind == Kind::Scheme
            && self
                .buf
                .path
                .as_ref()
                .and_then(|p| p.extension())
                .is_none_or(|e| e != "rkt")
            && !self
                .buf
                .lines
                .first()
                .is_some_and(|l| l.starts_with("#lang"))
        {
            return Arc::new(Vec::new());
        }
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
                let walked = self.walked_roots.entry(dir.clone()).or_insert_with(|| {
                    Arc::new(search::external_files(kind, std::slice::from_ref(dir)))
                });
                files.extend(walked.iter().cloned());
            }
            self.external.insert(kind, (roots, Arc::new(files)));
            self.node_modules_of = Some(here.to_path_buf());
        }
        if let Some(here) =
            here.filter(|_| matches!(kind, Kind::Elixir | Kind::Solidity | Kind::Css))
        {
            let roots = match kind {
                Kind::Elixir => elixir_roots(&mut self.otp, &self.root, here),
                _ => search::node_modules(&self.root, here),
            };
            if self.external.get(&kind).is_none_or(|(r, _)| *r != roots) {
                let mut files = Vec::new();
                for dir in &roots {
                    let walked = self.walked_roots.entry(dir.clone()).or_insert_with(|| {
                        Arc::new(search::external_files(kind, std::slice::from_ref(dir)))
                    });
                    files.extend(walked.iter().cloned());
                }
                self.external.insert(kind, (roots, Arc::new(files)));
            }
        }
        let files = if let Some((_, files)) = self.external.get(&kind) {
            Arc::clone(files)
        } else {
            let roots = search::external_roots(kind, &self.root);
            let warm = self.warming.insert(kind, None).flatten();
            let files = match warm.and_then(|w| w.join().ok()) {
                Some((walked, files)) if walked == roots => files,
                _ => search::external_files(kind, &roots),
            };
            let files = Arc::new(files);
            if !roots.is_empty() {
                self.external.insert(kind, (roots, Arc::clone(&files)));
            }
            files
        };
        if kind != Kind::C {
            return files;
        }
        self.c_kept(files)
    }

    fn c_kept(&mut self, files: Arc<Vec<PathBuf>>) -> Arc<Vec<PathBuf>> {
        let kind = Kind::C;
        let key = (self.c_source(), self.objc_file());
        if let Some((from, kept)) = self.c_files.get(&key)
            && Arc::ptr_eq(from, &files)
        {
            return Arc::clone(kept);
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
        self.c_files.insert(key, (files, Arc::clone(&kept)));
        kept
    }

    pub(super) fn c_source(&self) -> bool {
        (self.buf.path.as_deref())
            .is_some_and(|p| p.extension().is_some_and(|e| e == "c" || e == "m"))
    }

    pub(super) fn objc_file(&self) -> bool {
        (self.buf.path.as_deref())
            .is_some_and(|p| search::objc_file(p, || self.buf.lines.join("\n")))
    }
    pub(super) fn c_dialect(&self) -> search::CDialect {
        if self.objc_file() {
            search::CDialect::ObjectiveC
        } else {
            search::CDialect::COrCpp
        }
    }

    pub(super) fn declaring(&self, kind: Kind, word: &str, mut hits: Vec<Hit>) -> Vec<Hit> {
        let mut lines: HashMap<PathBuf, Vec<String>> = HashMap::new();
        let objc_only = (kind == Kind::C && !self.objc_file()).then(|| search::objc_only(word));
        hits.retain(|h| {
            objc_only.as_ref().is_none_or(|only| !only(&h.text))
                && search::declares_where(kind, &h.path, word, h.line1, &h.text, || {
                    lines.entry(h.path.clone()).or_insert_with(|| {
                        self.text_of(&h.path)
                            .map_or_else(Vec::new, |t| t.lines().map(str::to_owned).collect())
                    })
                })
        });
        if kind == Kind::Scheme
            && let Some(here) = self.rel_current()
        {
            hits.retain(|h| {
                let dir = h.path.parent().unwrap_or_else(|| Path::new(""));
                !self.text_of(&h.path).is_some_and(|text| {
                    search::scheme_foreign_branch(&text, h.line1, |f| dir.join(f) == here)
                })
            });
        }
        hits
    }

    pub(super) fn text_of(&self, path: &Path) -> Option<String> {
        let text = self.file_text(path)?;
        Some(search::script_text(path, &text, None).into_owned())
    }

    pub(super) fn file_text(&self, path: &Path) -> Option<String> {
        if self.rel_current().as_deref() == Some(path) {
            Some(self.buf.lines.join("\n"))
        } else {
            std::fs::read_to_string(self.root.join(path)).ok()
        }
    }

    pub(super) fn hit_text(&self, h: &Hit) -> Option<String> {
        let text = self.hit_file_text(h)?;
        Some(search::script_text(&h.path, &text, None).into_owned())
    }

    fn hit_file_text(&self, h: &Hit) -> Option<String> {
        match h.deleted {
            None => self.file_text(&h.path),
            Some(_) => self.base_text(&h.path),
        }
    }

    pub(super) fn base_text(&self, path: &Path) -> Option<String> {
        let r = self.review.as_ref()?;
        let from = r.file(path).and_then(|f| f.old.as_deref());
        let bytes = r.base_bytes(&self.root, from.unwrap_or(path)).ok()?;
        Some(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub(super) fn hidden_of(&self, kind: Kind, h: &Hit) -> Vec<bool> {
        (self.hit_file_text(h)).map_or_else(Vec::new, |t| search::hidden_lines(kind, &h.path, &t))
    }

    pub(super) fn hidden_now(&self, kind: Kind, h: &Hit) -> Vec<bool> {
        if search::android_source(&h.path) {
            return Vec::new();
        }
        if search::component(&h.path) && kind != Kind::Css {
            self.hidden_of(kind, h)
        } else {
            (self.hit_file_text(h)).map_or_else(Vec::new, |t| match kind {
                Kind::C | Kind::CSharp => self.c_hidden(kind, &h.path, &t),
                _ => search::file_literal_lines(kind, &h.path, &t),
            })
        }
    }

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

    pub(super) fn rel_to_its_root<'a>(&self, kind: Kind, path: &'a Path) -> &'a Path {
        let roots = self
            .external
            .get(&kind)
            .map(|(roots, _)| roots.as_slice())
            .unwrap_or_default();
        let mut roots = match kind {
            Kind::TsJs => roots.last().into_iter().chain([&self.root]).collect(),
            Kind::Elixir => std::iter::once(&self.root)
                .chain(roots.iter().filter(|r| !r.starts_with(&self.root)))
                .collect(),
            _ => roots.iter().collect::<Vec<_>>(),
        };
        if kind == Kind::Python {
            roots.sort_by_key(|r| std::cmp::Reverse(r.components().count()));
        }
        roots
            .into_iter()
            .find(|r| path.starts_with(r))
            .map_or(path, |r| {
                let keep = if *r == self.root { 0 } else { package_dirs(r) };
                let from = r.ancestors().nth(keep).unwrap_or(r);
                path.strip_prefix(from).unwrap_or(path)
            })
    }

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
                        let word = jvm::declared_as(kind, word, &c.hit.text);
                        search::qualified(kind, text, c.hit.line1, &word)
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
                let lead = format!("{name}{}  {why}{}  ", pad(name_w, &name), pad(why_w, &why));
                let shown = self.rel_to_its_root(kind, &c.hit.path);
                let head = format!("{lead}{}: ", at_label(shown, c.hit.line1));
                let path_at = lead.len()..lead.len() + shown.display().to_string().len();
                PickItem {
                    code_at: Some(head.len()),
                    path_at: Some(path_at),
                    col: word_col(
                        &c.hit.text,
                        &jvm::declared_as(kind, word, &c.hit.text),
                        search::word_chars(Some(kind), true),
                    ),
                    label: head + &clip(c.hit.text.trim(), MAX_LABEL_TEXT),
                    deleted: c.hit.deleted.is_some(),
                    path: c.hit.path,
                    line: c.hit.line1,
                }
            })
            .collect()
    }

    pub(super) fn no_rules(&self) -> String {
        let ext = self.buf.path.as_deref().and_then(Path::extension);
        match ext {
            Some(ext) => format!("no rules for .{}", ext.to_string_lossy()),
            None => "no rules for this file".into(),
        }
    }
}

pub(super) type Walked = (Vec<PathBuf>, Vec<PathBuf>);

fn walk_outside(kind: Kind, root: &Path) -> Walked {
    let roots = search::external_roots(kind, root);
    let files = search::external_files(kind, &roots);
    (roots, files)
}

fn python_rel<'a>(roots: &[PathBuf], f: &'a Path) -> Option<(usize, &'a Path)> {
    let (i, root) = (roots.iter().enumerate())
        .filter(|(_, r)| f.starts_with(r))
        .max_by_key(|(_, r)| r.components().count())?;
    Some((i, f.strip_prefix(root).ok()?))
}

fn package_dirs(root: &Path) -> usize {
    let name = root.file_name().unwrap_or_default().to_string_lossy();
    let versioned = |sep: &str| {
        name.match_indices(sep).any(|(i, _)| {
            let Some(v) = name.get(i + sep.len()..) else {
                return false;
            };
            let rest = v.trim_start_matches(|c: char| c.is_ascii_digit());
            rest.len() < v.len() && rest.starts_with('.')
        })
    };
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

fn elixir_roots(otp_cache: &mut Option<Vec<PathBuf>>, root: &Path, here: &Path) -> Vec<PathBuf> {
    let otp = match otp_cache {
        Some(otp) => otp.clone(),
        None => search::external_roots(Kind::Elixir, root),
    };
    if !otp.is_empty() {
        *otp_cache = Some(otp.clone());
    }
    let mut roots = [
        search::mix_deps(root, here),
        search::rebar_deps(root, here),
        otp,
    ]
    .concat();
    let mut seen = HashSet::new();
    roots.retain(|r| seen.insert(r.clone()));
    roots
}
