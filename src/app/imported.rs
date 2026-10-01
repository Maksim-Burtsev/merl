//! `d` through the imports: the declarations of a name in the project module an import names,
//! followed through the modules that only hand it on, and what a TypeScript module or barrel
//! exports under another name.

use super::*;

impl App {
    /// The declarations of `word` in the project module an import names, where `path` is what
    /// [`search::imports`] binds the word or its qualifier to; `None` when the module is not the
    /// project's. The word sits directly in the module, or in the class the chain goes through
    /// (`UserRepo.create`), as [`search::qualified`] names it: `store.Open` is never a method
    /// `Open`. An alias finds the imported name. A default import finds a declaration of its local
    /// name, else the module's `export default`. An empty list is a module that does not declare
    /// the word, a re-export: the search by name takes over. A Python name that is itself a
    /// module of the project, and that nothing on the way declares, is that module's file.
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

    /// The package a barrel of the project hands the word on from, `export { x } from
    /// "lodash"` (#527), when nothing of the project answers the import `path`.
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

    /// The package a TypeScript barrel of the project hands on what the import `path` takes
    /// (#527), as an import of it is spelled: `["lodash", "x"]` for `export { x } from "lodash"`
    /// in the module `path` names, or in a barrel that module hands the name on from.
    /// ponytail: four modules deep, which also ends a cycle.
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
            search::reexported(&text, taken)
                .into_iter()
                .find_map(|(mut module, taken)| {
                    let outside = files(f, &module).is_empty();
                    let package = module
                        .first()
                        .is_some_and(|m| !m.starts_with(['.', '~', '#']) && m != "@");
                    module.push(taken);
                    match outside {
                        true => package.then_some(module),
                        false => self.package_behind(f, &module, depth + 1),
                    }
                })
        })
    }

    /// What a TypeScript module exports as `word` under another name, `export { Hono as HonoBase }`:
    /// the top-level declarations of `Hono` in the module's files, which `grep` searches. What
    /// it finds of the `export` line says only which files to read, so a cut in it cuts nothing
    /// shown.
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
                .is_some_and(|text| search::qualified(Kind::TsJs, &text, h.line, &local).is_none())
        });
        hits
    }

    /// [`App::imported_definitions`], `depth` modules of the project that only hand the name on
    /// away from the file that asked.
    fn imported_at(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        chain: &[String],
        path: &[String],
        depth: usize,
    ) -> Option<Vec<Candidate>> {
        let module_files =
            |module: &[String]| search::module_files(kind, &self.root, &self.files, here, module);
        // What a CommonJS module bound whole hands out (#328), [`search::module_exports`].
        let mut exports = None;
        // The qualifier and its class, when it is an instance the module exports (#341).
        let mut instance = None;
        // The names from the module down to the word: what the import takes, then the chain.
        let tail = |mut names: Vec<String>| {
            if let Some((_, after)) = chain.split_first() {
                names.extend(after.iter().cloned());
                names.push(word.to_owned());
            }
            names
        };
        // The word itself names a module of the project (#280): `views` in `from shop import
        // views`, `shop` in `import shop`. What Python imports: a package over a module of the
        // same name beside it. Only when nothing the lookup below reads declares or hands on
        // the name, so what a package's `__init__.py` binds keeps its say.
        let whole_module = || {
            let found = self.module_candidates(self.project_module(here, &tail(path.to_vec())));
            (!found.is_empty()).then_some(found)
        };
        let (files, inside) = match kind {
            // `from a import b` takes a module or a name from `a`: the longest module that exists,
            // never shorter than the one the import names.
            Kind::Python => {
                let target = tail(path.to_vec());
                let floor = path.len().saturating_sub(1).max(1);
                // A module importing from itself (`from . import views` in a package's
                // `__init__.py`) takes a module of the package, unless it declares the name
                // itself: then it is read as on master, where the import proves it.
                let declares = |name: &String| {
                    let pattern = search::def_patterns(kind, name).join("|");
                    self.grep(&pattern, false, false, |p| p == here)
                        .unwrap_or_default()
                        .iter()
                        .any(|h| {
                            self.text_of(&h.path).is_some_and(|text| {
                                search::qualified(kind, &text, h.line, name).is_none()
                            })
                        })
                };
                let found = (floor..target.len()).rev().find_map(|n| {
                    let files = module_files(&target[..n]);
                    let own = files == [here] && !target[n..].first().is_some_and(declares);
                    (!files.is_empty() && !own).then(|| (files, target[n..].to_vec()))
                });
                match found {
                    Some(found) => found,
                    None => return whole_module(),
                }
            }
            Kind::TsJs => {
                let (taken, module) = path.split_last()?;
                let files = module_files(module);
                if files.is_empty() {
                    return None;
                }
                // A CommonJS module bound whole, `const X = require("./x")` (#328): `X` is what
                // its one `module.exports =` hands out, the name it assigns or the line itself,
                // and `X.member` a member of the class that name declares there. Where it
                // builds `module.exports` otherwise, only its top-level `X` counts.
                if taken == "*" {
                    exports = files.iter().find_map(|f| {
                        Some((f.clone(), search::module_exports(&self.text_of(f)?)?))
                    });
                }
                let class = |n: &String| {
                    let pattern = search::def_patterns(kind, n).join("|");
                    self.grep(&pattern, false, false, |p| files.iter().any(|f| f == p))
                        .unwrap_or_default()
                        .iter()
                        .any(|h| {
                            self.text_of(&h.path).is_some_and(|t| {
                                search::qualified(kind, &t, h.line, n).is_none()
                                    && search::declares_type(kind, &h.text)
                            })
                        })
                };
                let inside = match taken.as_str() {
                    "*" => match (exports.as_ref().map(|(_, e)| e), chain.is_empty()) {
                        (Some(Some((_, name))), true) => {
                            vec![name.clone().unwrap_or_else(|| word.to_owned())]
                        }
                        (Some(None), true) => vec![word.to_owned()],
                        (Some(Some((_, Some(name)))), false) if class(name) => {
                            let mut names = vec![name.clone()];
                            names.extend(chain[1..].iter().cloned());
                            names.push(word.to_owned());
                            names
                        }
                        _ => tail(Vec::new()),
                    },
                    // An instance the module exports as its default, `export default new
                    // Environment()`: `env.APP_NAME` is a member of its class (#341). Of any
                    // other default export what it is called is known only on its own line.
                    "default" if chain.len() == 1 => {
                        match files
                            .iter()
                            .find_map(|f| search::default_class(&self.text_of(f)?))
                        {
                            Some(class) => {
                                instance = Some(format!("{}: {class}", chain[0]));
                                vec![class, word.to_owned()]
                            }
                            None => return Some(Vec::new()),
                        }
                    }
                    "default" if !chain.is_empty() => return Some(Vec::new()),
                    "default" => vec![word.to_owned()],
                    name => tail(vec![name.to_owned()]),
                };
                (files, inside)
            }
            Kind::Go => (module_files(path), tail(Vec::new())),
            // No module rules: the search by name, then outside the project, as before.
            _ => return Some(Vec::new()),
        };
        if files.is_empty() {
            return None;
        }
        let Some(name) = inside.last() else {
            return Some(Vec::new());
        };
        let wanted = |p: &Path| files.iter().any(|f| f == p);
        let mut patterns = search::def_patterns(kind, name);
        // In a TypeScript type the import names, a field, a static and an enum member declare
        // the name too (#341): `static presetColors = …`, `Admin = "admin",`, and so does a key
        // directly inside a `const X = {` literal, `TwentyFivePerMinute: {`. A `let` or `var`
        // literal may be reassigned, and one behind a call or a cast (`Object.freeze({`) is
        // another value: neither declares. A member with no value, `ZOOMED,`, only directly
        // inside an `enum`, where no array or call's arguments are, or a literal's shorthand.
        let member = kind == Kind::TsJs && inside.len() > 1;
        // What the declaration patterns find is kept as on master, `get(url) {` in `const api =
        // Object.freeze({`: the rules above sort only the hits of the patterns added here.
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
                let owner = search::owner_line(&text, h.line).unwrap_or_default();
                let enumed = OWNER_ENUM.is_match(owner);
                let literal = OWNER_LITERAL.is_match(owner);
                let value = OWNER_VALUE.is_match(owner);
                let added = member && !declarations.as_ref().is_some_and(|d| d.is_match(&h.text));
                search::qualified(kind, &text, h.line, name) == within
                    && (!added || !value || literal)
                    && (!added || !bare.is_match(&h.text) || enumed || literal)
            })
        });
        // `export { Hono as HonoBase }`: the module declares it under another name. A default
        // import's name is the importer's own.
        let named = path.last().is_some_and(|t| t != "default");
        if hits.is_empty() && kind == Kind::TsJs && within.is_none() && named {
            hits = self.renamed_export(name, |p| {
                self.grep(p, false, false, wanted).unwrap_or_default()
            });
        }
        // `const utils = require("./m")` over `module.exports = { helper };`: that line.
        if hits.is_empty()
            && chain.is_empty()
            && let Some((file, Some((line, _)))) = &exports
        {
            let line = *line;
            hits = vec![Hit {
                deleted: None,
                text: self
                    .text_of(file)
                    .and_then(|t| t.lines().nth(line - 1).map(str::to_owned))
                    .unwrap_or_default(),
                path: file.clone(),
                line,
                col: 0,
            }];
        }
        // One that builds its exports line by line hands out itself: its file, as a module.
        if hits.is_empty()
            && chain.is_empty()
            && let Some((file, None)) = &exports
        {
            return Some(self.module_candidates(vec![file.clone()]));
        }
        if hits.is_empty()
            && kind == Kind::TsJs
            && instance.is_none()
            && path.last().is_some_and(|t| t == "default")
        {
            hits = self
                .grep(r"^export\s+default\b", false, false, wanted)
                .unwrap_or_default();
            // `import Text from "../shared/Text"; export default Text;` hands on what it imports
            // (#335): that import is followed. `export default observer(Text)` is what it says.
            if let [hit] = hits.as_slice()
                && depth < 4
                && let Some(name) = search::default_name(&hit.text)
                && let Some(source) = self.text_of(&hit.path).and_then(|t| {
                    let imports = search::imports(kind, &t);
                    bound(&imports, &name)
                })
                && let Some(found) = self
                    .imported_at(kind, &hit.path, &name, &[], &source, depth + 1)
                    .filter(|f| !f.is_empty())
            {
                return Some(found);
            }
        }
        // A component with no `export default`, a `<script setup>`'s, a Svelte or an Astro one,
        // is its file (#413).
        if hits.is_empty() && !named && files.iter().all(|f| search::component(f)) {
            return Some(self.module_candidates(files));
        }
        // A barrel hands the name on (#335): `export { Name } from "./x"`, `export { default as
        // Name }`, `export { x as Name }`, `export * from "./x"`, each source followed as an
        // import of it. Several `export *` sources that declare it are a picker.
        // ponytail: four modules deep, which also ends a cycle.
        if hits.is_empty()
            && kind == Kind::TsJs
            && depth < 4
            && !inside.is_empty()
            && instance.is_none()
        {
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
            for f in &files {
                let Some(text) = self.text_of(f) else {
                    continue;
                };
                for (mut module, taken) in search::reexported(&text, first) {
                    module.push(taken);
                    let more = self
                        .imported_at(kind, f, &word, &chain, &module, depth + 1)
                        .unwrap_or_default();
                    for c in more {
                        if !found
                            .iter()
                            .any(|o| (&o.hit.path, o.hit.line) == (&c.hit.path, c.hit.line))
                        {
                            found.push(c);
                        }
                    }
                }
            }
            if !found.is_empty() {
                return Some(found);
            }
        }
        // A Python module of the project that does not declare the name but imports it hands
        // it on (#100): `from .sessions import open_session` in a package's `__init__.py`.
        // ponytail: four modules deep, which also ends a cycle.
        if hits.is_empty() && kind == Kind::Python && depth < 4 {
            let mut found: Vec<Candidate> = Vec::new();
            for f in &files {
                for c in self.handed_on(kind, f, &inside, depth) {
                    if !found
                        .iter()
                        .any(|o| (&o.hit.path, o.hit.line) == (&c.hit.path, c.hit.line))
                    {
                        found.push(c);
                    }
                }
            }
            if !found.is_empty() {
                return Some(found);
            }
        }
        // A module that binds the name in any way, an import that led nowhere included, is
        // left to the search by name.
        let unbound = |name: &String| {
            files.iter().all(|f| {
                self.text_of(f)
                    .is_none_or(|t| search::bindings(kind, &t, 1, name).is_empty())
            })
        };
        if hits.is_empty()
            && kind == Kind::Python
            && inside.first().is_some_and(unbound)
            && let Some(found) = whole_module()
        {
            return Some(found);
        }
        let hits = self.host_built(kind, hits);
        // A file, or a Go package's directory.
        let label = |hit: &Hit| match (kind, hit.path.parent()) {
            (Kind::Go, Some(dir)) if dir != Path::new("") => format!("{}/", dir.display()),
            (Kind::Go, _) => "./".to_owned(),
            _ => hit.path.display().to_string(),
        };
        Some(
            hits.into_iter()
                .map(|hit| Candidate {
                    reason: match &instance {
                        Some(typed) => Reason::Receiver(typed.clone()),
                        None => Reason::Import(label(&hit)),
                    },
                    hit,
                })
                .collect(),
        )
    }

    /// What the imports of the Python module `file` lead to for `names`, the first of which the
    /// module was asked for and does not declare: an import of the module itself, not of a
    /// function in it, binds that name, or a `from x import *` may. Every source is followed,
    /// and which one the module ends up with is not computed: several are a picker. A name the
    /// module binds in any other way as well (an assignment under an `if`, a loop) is left to
    /// the search by name.
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
            .any(|b| !lines.get(b.line - 1).is_some_and(|l| import(l)))
        {
            return Vec::new();
        }
        let chain = &names[..names.len() - 1];
        search::imports_as_written(kind, &search::python_module_level(&text))
            .into_iter()
            .filter_map(|(name, mut path)| {
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
        let Some((command, arg)) = (kind == Kind::Cmake)
            .then(|| search::cmake_import(self.line_str(), self.col))
            .flatten()
        else {
            return search::module_files(kind, &self.root, &self.files, here, module);
        };
        let dir = here.parent().unwrap_or(Path::new(""));
        let found = search::cmake_files(&command, &arg, dir, &self.files);
        if !found.is_empty() || command == "add_subdirectory" || arg.contains('/') {
            return found;
        }
        let outside = self.external_files(kind);
        search::cmake_files(&command, &arg, dir, &outside)
    }

    /// The first line of each of `files`, a module a name or a path leads to as a whole.
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
                    line: 1,
                    col: 0,
                },
            })
            .collect()
    }
}

/// The line a TypeScript enum's body opens with (#341).
static OWNER_ENUM: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^\s*(?:export\s+)?(?:declare\s+)?(?:const\s+)?enum\s").unwrap()
});
/// The line a TypeScript value's body opens with: a `const`, `let` or `var` (#341).
static OWNER_VALUE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^\s*(?:export\s+)?(?:declare\s+)?(?:const|let|var)\s").unwrap()
});
/// A `const` bound to an object literal itself, `export const X = {` (#341).
static OWNER_LITERAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^\s*(?:export\s+)?const\s+[\w$]+\s*(?::[^=]+)?=\s*\{\s*$").unwrap()
});

/// The JavaScript and DOM globals whose members TypeScript's lib and `@types/node` declare (#341).
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
