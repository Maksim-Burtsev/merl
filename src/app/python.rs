//! `d` in Python: the modules a name or an import leads to, in the project and outside it,
//! the names a `from m import *` and a package's assignments bring in, and the implementation
//! behind an `@overload` set.

use super::*;

impl App {
    /// The files of the Python module `module` in the project: a package over a module of the
    /// same name beside it, as Python imports it (#280).
    pub(super) fn project_module(&self, here: &Path, module: &[String]) -> Vec<PathBuf> {
        let mut files = search::module_files(Kind::Python, &self.root, &self.files, here, module);
        if files.iter().any(|f| f.ends_with("__init__.py")) {
            files.retain(|f| f.ends_with("__init__.py"));
        }
        files
    }

    /// The Python module `module` as a whole (#333): the project's, else, for an absolute path,
    /// the one outside, named from its root as the picker names it.
    pub(super) fn python_module(&mut self, here: &Path, module: &[String]) -> Vec<Candidate> {
        let files = self.project_module(here, module);
        if !files.is_empty() || module[0].starts_with('.') {
            return self.module_candidates(files);
        }
        let file = self.external_module(module);
        file.map(|f| self.outside_module(f)).unwrap_or_default()
    }

    /// `word` qualified by a name an import binds to `path` outside the project, when the path
    /// resolves to a module only without its last part, the name the import takes
    /// (#342): the declarations of `Name.word` in that module, `Name` as the module calls it. An
    /// empty list is a member the lookup does not find there, which no namesake elsewhere
    /// answers for. `Some(None)` is a name the module neither declares a class of nor imports
    /// (#560): a value it holds, such as Django's `settings = LazySettings()`, whose
    /// `__getattr__` reads the project's settings module, or a name a module `__getattr__` makes
    /// up; nothing outside is read for its members. `None` leaves it to the lookup by import: a
    /// relative import, a name that is a module itself, a module not found.
    pub(super) fn outside_class_member(
        &mut self,
        word: &str,
        path: &[String],
    ) -> Option<Option<Vec<Candidate>>> {
        let (name, module) = path.split_last()?;
        if module.is_empty() || path[0].starts_with('.') {
            return None;
        }
        if self.external_module(path).is_some() {
            return None;
        }
        let file = self.external_module(module)?;
        let kind = Kind::Python;
        let text = std::fs::read_to_string(&file).ok()?;
        // A class the module declares, under an `if` or a `try` too, or a name it imports, `*`
        // included: a class re-exported, as `django.test` does `TestCase`.
        let imports = search::imports(kind, &text);
        let class = text
            .lines()
            .any(|l| search::type_name(kind, l).as_deref() == Some(name))
            || imports.iter().any(|(n, _)| n == name || n == "*");
        if !class {
            return Some(None);
        }
        let mut patterns = search::def_patterns(kind, word);
        patterns.extend(search::field_patterns(kind, word).unwrap_or_default());
        let within = Some(format!("{name}.{word}"));
        let hits = self.external_grep(kind, std::slice::from_ref(&file), &patterns.join("|"));
        let mut hits: Vec<Hit> = (hits.into_iter())
            .filter(|h| search::qualified(kind, &text, h.line, word) == within)
            .collect();
        if hits.is_empty()
            && let Some(class) = self.outside_class(Path::new(""), path, std::slice::from_ref(name))
            && let Some(declared) = search::type_name(kind, &class.text)
        {
            let ty = Typed {
                name: declared,
                path: class.path,
                line: class.line,
            };
            hits = self.above(kind, &ty, word, 0).unwrap_or_default();
        }
        let reason = Reason::Import(module.join("."));
        Some(Some(
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect(),
        ))
    }

    /// The module-level assignments of `word` in the files of the Python package `module` outside
    /// the project, `django.conf` for `django/conf/global_settings.py` (#560).
    pub(super) fn package_assignments(&mut self, word: &str, module: &[String]) -> Vec<Candidate> {
        let kind = Kind::Python;
        let all = self.external_files(kind);
        let Some((_, files)) = self
            .python_module_among(&all, module)
            .filter(|(n, _)| *n == module.len())
        else {
            return Vec::new();
        };
        let pattern = search::def_patterns(kind, word).join("|");
        let reason = Reason::Import(module.join("."));
        self.external_grep(kind, &files, &pattern)
            .into_iter()
            .filter(|h| assigns(&h.text, word))
            .map(|hit| Candidate {
                hit,
                reason: reason.clone(),
            })
            .collect()
    }

    /// The top-level declarations of `word` in the modules outside the project that the file's
    /// `from m import *` lines name (#336).
    pub(super) fn star_imported(
        &mut self,
        word: &str,
        imports: &[(String, Vec<String>)],
    ) -> Vec<Candidate> {
        let kind = Kind::Python;
        let pattern = search::def_patterns(kind, word).join("|");
        let mut found = Vec::new();
        for (_, path) in imports.iter().filter(|(name, _)| name == "*") {
            let module = &path[..path.len() - 1];
            if module.is_empty() || module[0].starts_with('.') {
                continue;
            }
            let Some(file) = self.external_module(module) else {
                continue;
            };
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            let hits = self.external_grep(kind, std::slice::from_ref(&file), &pattern);
            found.extend(
                hits.into_iter()
                    .filter(|h| search::qualified(kind, &text, h.line, word).is_none())
                    .map(|hit| Candidate {
                        hit,
                        reason: Reason::Import(module.join(".")),
                    }),
            );
        }
        found
    }

    pub(super) fn outside_handed_on(
        &self,
        module: &[String],
        name: &str,
        depth: usize,
    ) -> Vec<Candidate> {
        let kind = Kind::Python;
        let Some(file) = self.walked_module(module) else {
            return Vec::new();
        };
        let Ok(text) = std::fs::read_to_string(&file) else {
            return Vec::new();
        };
        let reason = Reason::Import(module.join("."));
        let pattern = search::def_patterns(kind, name).join("|");
        let hits = self.external_grep(kind, std::slice::from_ref(&file), &pattern);
        let hits: Vec<Hit> = (self.declaring(kind, name, hits).into_iter())
            .filter(|h| search::qualified(kind, &text, h.line, name).is_none())
            .collect();
        if !hits.is_empty() {
            return hits
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect();
        }
        let lines: Vec<&str> = text.lines().collect();
        let import = |l: &str| l.starts_with("from ") || l.starts_with("import ");
        let other = search::bindings(kind, &text, 1, name).iter().any(|b| {
            !lines
                .get(b.line - 1)
                .is_some_and(|l| import(l.trim_start()))
        });
        if other || depth >= 4 {
            return Vec::new();
        }
        let mut found: Vec<Candidate> = Vec::new();
        for (bound, mut path) in
            search::imports_as_written(kind, &search::python_module_level(&text))
        {
            let taken = match bound.as_str() {
                "*" => name.to_owned(),
                b if b == name => path.last().cloned().unwrap_or_default(),
                _ => continue,
            };
            path.pop();
            let Some(path) = python_absolute(&file, module, path) else {
                continue;
            };
            let more = match self.walked_module(&path) {
                Some(_) => self.outside_handed_on(&path, &taken, depth + 1),
                None if bound != "*" => python_import_line(&text, name)
                    .map(|line| Candidate {
                        hit: Hit {
                            deleted: None,
                            path: file.clone(),
                            line,
                            col: 0,
                            text: lines[line - 1].to_owned(),
                        },
                        reason: reason.clone(),
                    })
                    .into_iter()
                    .collect(),
                None => Vec::new(),
            };
            for c in more {
                if !found
                    .iter()
                    .any(|o| (&o.hit.path, o.hit.line) == (&c.hit.path, c.hit.line))
                {
                    found.push(c);
                }
            }
        }
        found
    }

    pub(super) fn outside_class(
        &self,
        file: &Path,
        path: &[String],
        parts: &[String],
    ) -> Option<Hit> {
        let (name, chain) = parts.split_last()?;
        let (module, taken) = match chain {
            [] => {
                let (taken, module) = path.split_last()?;
                (module.to_vec(), taken.clone())
            }
            [_, rest @ ..] => ([path, rest].concat(), name.clone()),
        };
        let module = match file.is_absolute() {
            true => {
                let rel = self.rel_to_its_root(Kind::Python, file).with_extension("");
                let mut own: Vec<String> = (rel.components())
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                if own.last().is_some_and(|l| l == "__init__") {
                    own.pop();
                }
                python_absolute(file, &own, module)?
            }
            false if module.first().is_none_or(|m| m.starts_with('.')) => return None,
            false => module,
        };
        let found = self.outside_handed_on(&module, &taken, 0);
        let [one] = found.as_slice() else {
            return None;
        };
        search::declares_type(Kind::Python, &one.hit.text).then(|| one.hit.clone())
    }

    /// The first line of `file`, a module outside the project, named from its root.
    fn outside_module(&self, file: PathBuf) -> Vec<Candidate> {
        let mut found = self.module_candidates(vec![file]);
        for c in &mut found {
            let shown = self.rel_to_its_root(Kind::Python, &c.hit.path);
            c.reason = Reason::Module(shown.display().to_string());
        }
        found
    }

    /// What a Python name bound by an import to `path`, outside the project, is as a whole
    /// (#333): the module, when the whole path is one and the package above does not bind the
    /// name itself (that binding keeps its say, through the lookup by import). The name a plain
    /// `import x` binds is a module or nothing, never a namesake found by name. `None` leaves the
    /// name to the lookup by import.
    pub(super) fn bound_module(&mut self, path: &[String]) -> Option<Vec<Candidate>> {
        if path[0].starts_with('.') {
            return None;
        }
        let Some(file) = self.external_module(path) else {
            return (path.len() == 1).then(Vec::new);
        };
        if let Some((name, above)) = path.split_last().filter(|(_, above)| !above.is_empty())
            && let Some(package) = self.external_module(above)
            && std::fs::read_to_string(&package)
                .is_ok_and(|t| !search::bindings(Kind::Python, &t, 1, name).is_empty())
        {
            return None;
        }
        Some(self.outside_module(file))
    }

    /// An `@overload` set is one function: when every candidate declares the same name in one
    /// `.py` file and all but one are overloads, that one, the implementation, is the answer
    /// (#338). A stub file, and a set of overloads only, has none and keeps its picker. Standing
    /// on one of them leaves them all, as on any declaration.
    pub(super) fn python_implementation(
        &self,
        word: &str,
        here: &Path,
        found: &mut Vec<Candidate>,
    ) {
        let [first, ..] = found.as_slice() else {
            return;
        };
        let path = first.hit.path.clone();
        if found.len() < 2
            || path.extension().is_some_and(|e| e == "pyi")
            || found.iter().any(|c| c.hit.path != path)
            || (path == here && found.iter().any(|c| c.hit.line == self.line + 1))
        {
            return;
        }
        let Some(text) = self.hit_text(&first.hit) else {
            return;
        };
        let name = |c: &Candidate| {
            let t = c.hit.text.trim_start();
            (t.starts_with("def ") || t.starts_with("async def "))
                .then(|| search::qualified(Kind::Python, &text, c.hit.line, word))
        };
        let one = name(first);
        if one.is_none() || found.iter().any(|c| name(c) != one) {
            return;
        }
        let plain = |c: &Candidate| !search::python_overload(&text, c.hit.line);
        if found.iter().filter(|c| plain(c)).count() == 1 {
            found.retain(plain);
        }
    }
}

/// The `def` lines, 1-based and innermost first, of the Python functions 1-based `line` sits
/// in, told by indentation: a name bound in one of them is seen from `line`, and from nowhere
/// outside it.
pub(super) fn python_functions(text: &str, line: usize) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let literal = search::literal_lines(Kind::Python, text);
    let indent = |l: &str| l.len() - l.trim_start().len();
    let Some(at) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    let (mut depth, mut out) = (indent(lines[at]), Vec::new());
    for i in (0..at).rev() {
        let t = lines[i].trim_start();
        if t.is_empty() || t.starts_with(['#', ')', ']']) || literal[i] || indent(lines[i]) >= depth
        {
            continue;
        }
        depth = indent(lines[i]);
        if t.starts_with("def ") || t.starts_with("async def ") {
            out.push(i + 1);
        }
    }
    out
}
pub(super) fn python_absolute(
    file: &Path,
    module: &[String],
    mut path: Vec<String>,
) -> Option<Vec<String>> {
    if let Some(dots) = path.first().filter(|p| p.starts_with('.')).map(String::len) {
        let init = file.file_stem().is_some_and(|s| s == "__init__");
        let package = &module[..module.len() - usize::from(!init)];
        let up = package.len().checked_sub(dots - 1)?;
        path.splice(..1, package[..up].iter().cloned());
    }
    (!path.is_empty()).then_some(path)
}
fn python_import_line(text: &str, name: &str) -> Option<usize> {
    static FROM: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?m)^from\s+[\w.]+\s+import\s+(\([^)]*\)|[^\n]+)").unwrap()
    });
    let word = Regex::new(&format!(r"\b{}\b", regex::escape(name))).ok()?;
    FROM.captures_iter(text).find_map(|c| {
        let names = c.get(1)?;
        let at = word.find(names.as_str())?;
        Some(text[..names.start() + at.start()].matches('\n').count() + 1)
    })
}
/// Whether the Python line `text` assigns `word` at module level: `NAME = …`, `NAME: T = …`.
pub(super) fn assigns(text: &str, word: &str) -> bool {
    text.strip_prefix(word).is_some_and(|rest| {
        let rest = rest.trim_start();
        rest.starts_with(':') || rest.starts_with('=') && !rest.starts_with("==")
    })
}
