//! `d` in Python: the modules a name or an import leads to, in the project and outside it,
//! the names a `from m import *` and a package's assignments bring in, and the implementation
//! behind an `@overload` set.

use super::*;

impl App {
    pub(super) fn project_module(&self, here: &Path, module: &[String]) -> Vec<PathBuf> {
        let mut files = search::module_files(Kind::Python, &self.root, &self.files, here, module);
        if files.iter().any(|f| f.ends_with("__init__.py")) {
            files.retain(|f| f.ends_with("__init__.py"));
        }
        files
    }

    pub(super) fn python_module(&mut self, here: &Path, module: &[String]) -> Vec<Candidate> {
        let files = self.project_module(here, module);
        if !files.is_empty() || module[0].starts_with('.') {
            return self.module_candidates(files);
        }
        let file = self.external_module(module);
        file.map(|f| self.outside_module(f)).unwrap_or_default()
    }

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
        let reason = Reason::Import(module.join("."));
        Some(Some(
            hits.into_iter()
                .filter(|h| search::qualified(kind, &text, h.line, word) == within)
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect(),
        ))
    }

    pub(super) fn package_assignments(&mut self, word: &str, module: &[String]) -> Vec<Candidate> {
        let kind = Kind::Python;
        let all = self.external_files(kind);
        let Some((_, files)) =
            search::module_among(&all, module, None).filter(|(n, _)| *n == module.len())
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

    /// The first line of `file`, a module outside the project, named from its root.
    fn outside_module(&self, file: PathBuf) -> Vec<Candidate> {
        let mut found = self.module_candidates(vec![file]);
        for c in &mut found {
            let shown = self.rel_to_its_root(Kind::Python, &c.hit.path);
            c.reason = Reason::Module(shown.display().to_string());
        }
        found
    }

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
/// Whether the Python line `text` assigns `word` at module level: `NAME = …`, `NAME: T = …`.
pub(super) fn assigns(text: &str, word: &str) -> bool {
    text.strip_prefix(word).is_some_and(|rest| {
        let rest = rest.trim_start();
        rest.starts_with(':') || rest.starts_with('=') && !rest.starts_with("==")
    })
}
