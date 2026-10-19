use super::{App, Candidate, Hit, Kind, Path, PathBuf, Reason, Regex, bound, search};
use std::ops::ControlFlow;

impl App {
    pub(super) fn imported_definitions(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        chain: &[String],
        path: &[String],
    ) -> Option<Vec<Candidate>> {
        self.imported_at(kind, here, word, chain, path, 0)
    }

    pub(super) fn barrel_package(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        chain: &[String],
        path: &[String],
    ) -> Option<Vec<String>> {
        let first = chain.first().map_or(word, String::as_str);
        let package = (kind == Kind::TsJs)
            .then(|| self.package_behind(here, path, 0))
            .flatten()?;
        (package.last().is_some_and(|t| t == first)
            && self
                .imported_definitions(kind, here, word, chain, path)
                .is_some_and(|f| f.is_empty()))
        .then_some(package)
    }

    pub(super) fn package_behind(
        &self,
        here: &Path,
        path: &[String],
        depth: usize,
    ) -> Option<Vec<String>> {
        let (taken, module) = path.split_last()?;
        if taken == "*" || depth >= 4 {
            return None;
        }
        let files = |from: &Path, module: &[String]| {
            search::module_files(Kind::TsJs, &self.root, &self.files, from, module)
        };
        files(here, module).iter().find_map(|f| {
            let text = self.text_of(f)?;
            search::reexported(&text, taken).into_iter().find_map(
                |search::Reexport {
                     mut module,
                     name_in_module,
                 }| {
                    let outside = files(f, &module).is_empty();
                    let package = module
                        .first()
                        .is_some_and(|m| !m.starts_with(['.', '~', '#']) && m != "@");
                    module.push(name_in_module);
                    if outside {
                        package.then_some(module)
                    } else {
                        self.package_behind(f, &module, depth + 1)
                    }
                },
            )
        })
    }

    pub(super) fn renamed_export(&self, word: &str, grep: impl Fn(&str) -> Vec<Hit>) -> Vec<Hit> {
        let cut = self.truncated.get();
        let mut exports: Vec<PathBuf> = grep(&format!(r"\bas\s+{}\b", regex::escape(word)))
            .into_iter()
            .map(|h| h.path)
            .collect();
        self.truncated.set(cut);
        exports.dedup();
        let Some(local) = exports
            .iter()
            .find_map(|f| search::exported_as(&self.text_of(f)?, word))
        else {
            return Vec::new();
        };
        let pattern = search::def_patterns(Kind::TsJs, &local).join("|");
        let mut hits = grep(&pattern);
        hits.retain(|h| {
            self.text_of(&h.path)
                .is_some_and(|text| search::qualified(Kind::TsJs, &text, h.line1, &local).is_none())
        });
        hits
    }

    fn imported_at(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        chain: &[String],
        path: &[String],
        depth: usize,
    ) -> Option<Vec<Candidate>> {
        let mut exports = None;
        let mut instance = None;
        let (files, inside) = match kind {
            Kind::Python => match self.python_target(here, word, chain, path) {
                Some(found) => found,
                None => return self.whole_module(here, word, chain, path),
            },
            Kind::TsJs => {
                let (taken, module) = path.split_last()?;
                let files = search::module_files(kind, &self.root, &self.files, here, module);
                if files.is_empty() {
                    return None;
                }
                if taken == "*" {
                    exports = self.star_exports(&files);
                }
                if let ("default", [head]) = (taken.as_str(), chain)
                    && let Some(found) = self.default_literal_member(&files, head, word)
                {
                    return Some(found);
                }
                let Some(inside) =
                    self.ts_inside(word, chain, taken, &files, exports.as_ref(), &mut instance)
                else {
                    return Some(Vec::new());
                };
                (files, inside)
            }
            Kind::Go => (
                search::module_files(kind, &self.root, &self.files, here, path),
                names_down_to_word(word, chain, Vec::new()),
            ),
            Kind::Julia if path.first().is_some_and(|p| !p.starts_with('.')) => return None,
            _ => return Some(Vec::new()),
        };
        if files.is_empty() {
            return None;
        }
        let Some(name) = inside.last() else {
            return Some(Vec::new());
        };
        let mut hits = self.declared_inside(kind, &files, &inside, name);
        let named = path.last().is_some_and(|t| t != "default");
        if hits.is_empty() && kind == Kind::TsJs && inside.len() <= 1 && named {
            let module = chain.is_empty() && path.last().is_some_and(|t| t == "*");
            hits = self.named_export(name, &files, module);
        }
        if hits.is_empty()
            && chain.is_empty()
            && let Some((file, search::ModuleExports::OneAssignment { line1, .. })) = &exports
        {
            hits = vec![self.export_hit(file, *line1)];
        }
        if hits.is_empty()
            && chain.is_empty()
            && let Some((file, search::ModuleExports::Piecemeal)) = &exports
        {
            return Some(self.module_candidates(vec![file.clone()]));
        }
        if hits.is_empty()
            && kind == Kind::TsJs
            && instance.is_none()
            && path.last().is_some_and(|t| t == "default")
        {
            match self.export_default(kind, &files, depth) {
                ControlFlow::Continue(found) => hits = found,
                ControlFlow::Break(handed) => return Some(handed),
            }
        }
        if hits.is_empty() && !named && files.iter().all(|f| search::component(f)) {
            return Some(self.module_candidates(files));
        }
        if hits.is_empty()
            && kind == Kind::TsJs
            && depth < 4
            && !inside.is_empty()
            && instance.is_none()
        {
            let found = self.ts_reexported(kind, &files, &inside, depth);
            if !found.is_empty() {
                return Some(found);
            }
        }
        if hits.is_empty() && kind == Kind::Python && depth < 4 {
            let found = self.python_handed_on(kind, &files, &inside, depth);
            if !found.is_empty() {
                return Some(found);
            }
        }
        if hits.is_empty()
            && kind == Kind::Python
            && inside
                .first()
                .is_some_and(|name| self.unbound(kind, &files, name))
            && let Some(found) = self.whole_module(here, word, chain, path)
        {
            return Some(found);
        }
        let hits = self.host_built(kind, hits);
        Some(import_candidates(kind, hits, instance.as_ref()))
    }

    fn whole_module(
        &self,
        here: &Path,
        word: &str,
        chain: &[String],
        path: &[String],
    ) -> Option<Vec<Candidate>> {
        let found = self.module_candidates(
            self.project_module(here, &names_down_to_word(word, chain, path.to_vec())),
        );
        (!found.is_empty()).then_some(found)
    }

    fn python_target(
        &self,
        here: &Path,
        word: &str,
        chain: &[String],
        path: &[String],
    ) -> Option<(Vec<PathBuf>, Vec<String>)> {
        let kind = Kind::Python;
        let target = names_down_to_word(word, chain, path.to_vec());
        let floor = path.len().saturating_sub(1).max(1);
        let declares = |name: &String| {
            let pattern = search::def_patterns(kind, name).join("|");
            self.grep(&pattern, false, false, |p| p == here)
                .unwrap_or_default()
                .iter()
                .any(|h| {
                    self.text_of(&h.path)
                        .is_some_and(|text| search::qualified(kind, &text, h.line1, name).is_none())
                })
        };
        (floor..target.len()).rev().find_map(|n| {
            let files = search::module_files(kind, &self.root, &self.files, here, &target[..n]);
            let own = files == [here] && !target[n..].first().is_some_and(declares);
            (!files.is_empty() && !own).then(|| (files, target[n..].to_vec()))
        })
    }

    fn default_literal_member(
        &self,
        files: &[PathBuf],
        head: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let [file] = files else {
            return None;
        };
        let text = self.text_of(file)?;
        let (at, name) = text
            .lines()
            .enumerate()
            .find_map(|(i, l)| Some((i + 1, search::default_name(l)?)))?;
        let [b] = &search::bindings(Kind::TsJs, &text, at, &name)[..] else {
            return None;
        };
        let search::Value::Type(written) = &b.value else {
            return None;
        };
        let (line, literal) = search::ts_literal_member(&text, b.line1, written, word)?;
        let hit = Hit {
            text: text.lines().nth(line - 1)?.to_owned(),
            path: file.clone(),
            line1: line,
            byte_col: None,
            deleted: None,
        };
        Some(vec![Candidate {
            hit,
            reason: Reason::Receiver(format!("{head}: {literal}")),
        }])
    }

    fn ts_inside(
        &self,
        word: &str,
        chain: &[String],
        taken: &str,
        files: &[PathBuf],
        exports: Option<&(PathBuf, search::ModuleExports)>,
        instance: &mut Option<String>,
    ) -> Option<Vec<String>> {
        let kind = Kind::TsJs;
        let class = |n: &String| {
            let pattern = search::def_patterns(kind, n).join("|");
            self.grep(&pattern, false, false, |p| files.iter().any(|f| f == p))
                .unwrap_or_default()
                .iter()
                .any(|h| {
                    self.text_of(&h.path).is_some_and(|t| {
                        search::qualified(kind, &t, h.line1, n).is_none()
                            && search::declares_type(kind, &h.text)
                    })
                })
        };
        Some(match taken {
            "*" => match (exports.map(|(_, e)| e), chain.is_empty()) {
                (Some(search::ModuleExports::OneAssignment { name, .. }), true) => {
                    vec![name.clone().unwrap_or_else(|| word.to_owned())]
                }
                (Some(search::ModuleExports::Piecemeal), true) => {
                    vec![word.to_owned()]
                }
                (
                    Some(search::ModuleExports::OneAssignment {
                        name: Some(name), ..
                    }),
                    false,
                ) if class(name) => {
                    let mut names = vec![name.clone()];
                    names.extend(chain[1..].iter().cloned());
                    names.push(word.to_owned());
                    names
                }
                _ => names_down_to_word(word, chain, Vec::new()),
            },
            "default" if chain.len() == 1 => {
                let class = files
                    .iter()
                    .find_map(|f| search::default_class(&self.text_of(f)?))?;
                *instance = Some(format!("{}: {class}", chain[0]));
                vec![class, word.to_owned()]
            }
            "default" if !chain.is_empty() => return None,
            "default" => vec![word.to_owned()],
            name => names_down_to_word(word, chain, vec![name.to_owned()]),
        })
    }

    fn declared_inside(
        &self,
        kind: Kind,
        files: &[PathBuf],
        inside: &[String],
        name: &str,
    ) -> Vec<Hit> {
        let wanted = |p: &Path| files.iter().any(|f| f == p);
        let mut patterns = search::def_patterns(kind, name);
        let member = kind == Kind::TsJs && inside.len() > 1;
        let declarations = member
            .then(|| Regex::new(&patterns.join("|")).ok())
            .flatten();
        let bare = format!(r"^\s+{}\s*,?\s*$", regex::escape(name));
        if member {
            patterns.extend(search::field_patterns(kind, name).unwrap_or_default());
            patterns.push(bare.clone());
        }
        let hits = self
            .grep(&patterns.join("|"), false, false, wanted)
            .unwrap_or_default();
        let mut hits = self.declaring(kind, name, hits);
        let within = (inside.len() > 1).then(|| inside.join("."));
        let bare = Regex::new(&bare).expect("an escaped name keeps the pattern valid");
        hits.retain(|h| {
            self.text_of(&h.path).is_some_and(|text| {
                let owner = search::owner_line(&text, h.line1).unwrap_or_default();
                let enumed = OWNER_ENUM.is_match(owner);
                let literal = OWNER_LITERAL.is_match(owner);
                let value = OWNER_VALUE.is_match(owner);
                let added = member && !declarations.as_ref().is_some_and(|d| d.is_match(&h.text));
                search::qualified(kind, &text, h.line1, name) == within
                    && (!added || !value || literal)
                    && (!added || !bare.is_match(&h.text) || enumed || literal)
            })
        });
        hits
    }

    fn named_export(&self, name: &str, files: &[PathBuf], module: bool) -> Vec<Hit> {
        let wanted = |p: &Path| files.iter().any(|f| f == p);
        let hits = self.renamed_export(name, |p| {
            self.grep(p, false, false, wanted).unwrap_or_default()
        });
        if hits.is_empty() && !module {
            return self
                .grep(&search::cjs_export(name), false, false, wanted)
                .unwrap_or_default();
        }
        hits
    }

    fn export_default(
        &self,
        kind: Kind,
        files: &[PathBuf],
        depth: usize,
    ) -> ControlFlow<Vec<Candidate>, Vec<Hit>> {
        let wanted = |p: &Path| files.iter().any(|f| f == p);
        let hits = self
            .grep(r"^export\s+default\b", false, false, wanted)
            .unwrap_or_default();
        if let Some(found) = self.default_reexported(kind, &hits, depth) {
            return ControlFlow::Break(found);
        }
        let declared = match hits.as_slice() {
            [hit] => search::default_name(&hit.text).map_or_else(Vec::new, |local| {
                self.declared_inside(
                    kind,
                    std::slice::from_ref(&hit.path),
                    std::slice::from_ref(&local),
                    &local,
                )
            }),
            _ => Vec::new(),
        };
        ControlFlow::Continue(if declared.is_empty() { hits } else { declared })
    }

    fn default_reexported(&self, kind: Kind, hits: &[Hit], depth: usize) -> Option<Vec<Candidate>> {
        let [hit] = hits else {
            return None;
        };
        if depth >= 4 {
            return None;
        }
        let name = search::default_name(&hit.text)?;
        let source = self.text_of(&hit.path).and_then(|t| {
            let imports = search::imports(kind, &t);
            bound(&imports, &name)
        })?;
        self.imported_at(kind, &hit.path, &name, &[], &source, depth + 1)
            .filter(|f| !f.is_empty())
    }

    fn star_exports(&self, files: &[PathBuf]) -> Option<(PathBuf, search::ModuleExports)> {
        files
            .iter()
            .find_map(|f| Some((f.clone(), search::module_exports(&self.text_of(f)?)?)))
    }

    pub(super) fn export_hit(&self, file: &Path, line: usize) -> Hit {
        Hit {
            deleted: None,
            text: self
                .text_of(file)
                .and_then(|t| t.lines().nth(line - 1).map(str::to_owned))
                .unwrap_or_default(),
            path: file.to_path_buf(),
            line1: line,
            byte_col: None,
        }
    }

    fn python_handed_on(
        &self,
        kind: Kind,
        files: &[PathBuf],
        inside: &[String],
        depth: usize,
    ) -> Vec<Candidate> {
        let mut found: Vec<Candidate> = Vec::new();
        for f in files {
            for c in self.handed_on(kind, f, inside, depth) {
                push_new(&mut found, c);
            }
        }
        found
    }

    fn unbound(&self, kind: Kind, files: &[PathBuf], name: &str) -> bool {
        files.iter().all(|f| {
            self.text_of(f)
                .is_none_or(|t| search::bindings(kind, &t, 1, name).is_empty())
        })
    }

    fn ts_reexported(
        &self,
        kind: Kind,
        files: &[PathBuf],
        inside: &[String],
        depth: usize,
    ) -> Vec<Candidate> {
        let (first, rest) = inside.split_first().expect("not empty");
        let (chain, word) = match rest.split_last() {
            Some((word, between)) => {
                let mut chain = vec![first.clone()];
                chain.extend(between.iter().cloned());
                (chain, word.clone())
            }
            None => (Vec::new(), first.clone()),
        };
        let mut found: Vec<Candidate> = Vec::new();
        for f in files {
            let Some(text) = self.text_of(f) else {
                continue;
            };
            for search::Reexport {
                mut module,
                name_in_module,
            } in search::reexported(&text, first)
            {
                module.push(name_in_module);
                let more = self
                    .imported_at(kind, f, &word, &chain, &module, depth + 1)
                    .unwrap_or_default();
                for c in more {
                    push_new(&mut found, c);
                }
            }
        }
        found
    }

    fn handed_on(&self, kind: Kind, file: &Path, names: &[String], depth: usize) -> Vec<Candidate> {
        let (Some(text), Some((first, last))) =
            (self.text_of(file), names.first().zip(names.last()))
        else {
            return Vec::new();
        };
        let lines: Vec<&str> = text.lines().collect();
        let import =
            |l: &str| l.trim_start().starts_with("from ") || l.trim_start().starts_with("import ");
        if search::bindings(kind, &text, 1, first)
            .iter()
            .any(|b| !lines.get(b.line1 - 1).is_some_and(|l| import(l)))
        {
            return Vec::new();
        }
        let chain = &names[..names.len() - 1];
        search::imports_as_written(kind, &search::python_module_level(&text))
            .into_iter()
            .filter_map(|search::Import { name, mut path }| {
                if name == "*" {
                    path.pop();
                    path.push(first.clone());
                } else if name != *first {
                    return None;
                }
                self.imported_at(kind, file, last, chain, &path, depth + 1)
            })
            .flatten()
            .collect()
    }

    pub(super) fn import_files(
        &mut self,
        kind: Kind,
        here: &Path,
        module: &[String],
    ) -> Vec<PathBuf> {
        if kind == Kind::Elixir
            && let Some((lib, path)) = search::erlang_include(self.line_str(), self.col)
        {
            let outside = self.external_files(kind);
            return search::erlang_include_files(lib, &path, here, &self.files, &outside);
        }
        let Some((command, arg)) = (kind == Kind::Cmake)
            .then(|| search::cmake_import(self.line_str(), self.col))
            .flatten()
        else {
            let found = search::module_files(kind, &self.root, &self.files, here, module);
            if !found.is_empty() || !matches!(kind, Kind::EmacsLisp | Kind::Scheme) {
                return found;
            }
            let outside = self.external_files(kind);
            let dir = here.parent().unwrap_or_else(|| Path::new(""));
            return search::lisp_files(kind, dir, &module.join("/"), &outside);
        };
        let dir = here.parent().unwrap_or_else(|| Path::new(""));
        let found = search::cmake_files(&command, &arg, dir, &self.files);
        if !found.is_empty() || command == "add_subdirectory" || arg.contains('/') {
            return found;
        }
        let outside = self.external_files(kind);
        search::cmake_files(&command, &arg, dir, &outside)
    }

    pub(super) fn module_candidates(&self, files: Vec<PathBuf>) -> Vec<Candidate> {
        files
            .into_iter()
            .map(|path| Candidate {
                reason: Reason::Module(path.display().to_string()),
                hit: Hit {
                    deleted: None,
                    text: self.file_text(&path).map_or_else(String::new, |t| {
                        t.lines().next().unwrap_or_default().to_owned()
                    }),
                    path,
                    line1: 1,
                    byte_col: None,
                },
            })
            .collect()
    }
}

fn names_down_to_word(word: &str, chain: &[String], mut taken: Vec<String>) -> Vec<String> {
    if let Some((_, after)) = chain.split_first() {
        taken.extend(after.iter().cloned());
        taken.push(word.to_owned());
    }
    taken
}

fn push_new(found: &mut Vec<Candidate>, c: Candidate) {
    if !found
        .iter()
        .any(|o| (&o.hit.path, o.hit.line1) == (&c.hit.path, c.hit.line1))
    {
        found.push(c);
    }
}

fn import_candidates(kind: Kind, hits: Vec<Hit>, instance: Option<&String>) -> Vec<Candidate> {
    let label = |hit: &Hit| match (kind, hit.path.parent()) {
        (Kind::Go, Some(dir)) if dir != Path::new("") => format!("{}/", dir.display()),
        (Kind::Go, _) => "./".to_owned(),
        _ => hit.path.display().to_string(),
    };
    hits.into_iter()
        .map(|hit| Candidate {
            reason: match instance {
                Some(typed) => Reason::Receiver(typed.clone()),
                None => Reason::Import(label(&hit)),
            },
            hit,
        })
        .collect()
}

static OWNER_ENUM: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^\s*(?:export\s+)?(?:declare\s+)?(?:const\s+)?enum\s")
        .expect("the regex is a literal")
});
static OWNER_VALUE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^\s*(?:export\s+)?(?:declare\s+)?(?:const|let|var)\s")
        .expect("the regex is a literal")
});
static OWNER_LITERAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^\s*(?:export\s+)?const\s+[\w$]+\s*(?::[^=]+)?=\s*\{\s*$")
        .expect("the regex is a literal")
});

pub(super) const JS_GLOBALS: &[&str] = &[
    "JSON",
    "Math",
    "Object",
    "Array",
    "Promise",
    "Reflect",
    "Number",
    "String",
    "Date",
    "RegExp",
    "Symbol",
    "Intl",
    "console",
    "document",
    "window",
    "navigator",
    "globalThis",
    "process",
];
