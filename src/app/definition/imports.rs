use super::{
    App, Asked, Candidate, Hit, Kind, Near, Path, PathBuf, Patterns, Reason, Regex, bound,
    imported, python, search,
};
use crate::app::jvm_typed::java_shaped;

pub(super) struct Imported {
    pub(super) value: Option<Vec<String>>,
    pub(super) outside: bool,
    pub(super) linked: bool,
}

impl App {
    pub(super) fn import_path(
        &self,
        a: &Asked,
        near: &mut Near,
        p: &Patterns,
    ) -> Option<Vec<String>> {
        let import = (!p.on_value)
            .then(|| bound(&near.imports, a.first()))
            .flatten();
        let barrel = import
            .as_deref()
            .and_then(|path| self.barrel_package(a.kind, &a.here, &a.word, &a.chain, path));
        match barrel {
            Some(package) => {
                for i in &mut near.imports {
                    if i.name == a.first() {
                        i.path.clone_from(&package);
                    }
                }
                Some(package)
            }
            None => import,
        }
    }

    pub(super) fn answered_by_missing_package(
        &mut self,
        a: &Asked,
        import: Option<&[String]>,
    ) -> bool {
        if a.kind == Kind::TsJs
            && let Some((item, module)) = import.and_then(<[String]>::split_last)
            && search::package_missing(
                &self.root,
                &self.files,
                a.here.parent().unwrap_or_else(|| Path::new("")),
                module,
            )
            && !self.external_files(a.kind).iter().any(|f| {
                let parts = if module[0].starts_with('@') { 2 } else { 1 };
                search::in_module(f, &module[..parts.min(module.len())])
            })
            && let Some(line) = search::ts_import_line1(&a.text, a.first())
        {
            if let Some(found) = self.ambient_declared(a, item, module) {
                return self.show_found(a, found);
            }
            let hit = self.line_hit(&a.here, line);
            let reason = Reason::Import(format!("{} (not installed)", module.join("/")));
            return self.show_found(a, vec![Candidate { hit, reason }]);
        }
        false
    }

    fn ambient_declared(
        &mut self,
        a: &Asked,
        item: &str,
        module: &[String],
    ) -> Option<Vec<Candidate>> {
        if matches!(item, "*" | "default") {
            return None;
        }
        let name = module.join("/");
        let here = a.here.parent().unwrap_or_else(|| Path::new(""));
        let deps = search::manifest_dependencies(&self.root, here);
        let files: Vec<PathBuf> = (self.external_files(a.kind).iter())
            .filter(|f| f.to_string_lossy().ends_with(".d.ts"))
            .filter(|f| search::package_of(f).is_some_and(|p| deps.contains(&p)))
            .cloned()
            .collect();
        let pattern = search::ambient_module_pattern(&name);
        let hits = self.outside_grep(a.kind, &files, &pattern, None);
        let mut packages: Vec<PathBuf> = hits
            .iter()
            .filter_map(|h| search::package_of(&h.path))
            .collect();
        packages.sort();
        packages.dedup();
        let [_] = packages[..] else {
            return None;
        };
        let dts = &hits.first()?.path;
        let hit = search::ambient_target(dts, &name, item)?;
        if !a.chain.is_empty() {
            let member = match &a.chain[..] {
                [only] if only == item => search::ambient_member(dts, &name, item, &a.word),
                _ => None,
            };
            let reason = Reason::Receiver(item.to_owned());
            return Some(
                member
                    .map(|hit| Candidate { hit, reason })
                    .into_iter()
                    .collect(),
            );
        }
        let reason = Reason::Import(name);
        Some(vec![Candidate { hit, reason }])
    }

    pub(super) fn imported(
        &mut self,
        a: &Asked,
        near: &Near,
        import: Option<Vec<String>>,
    ) -> Option<Imported> {
        let mut imported = Imported {
            value: None,
            outside: false,
            linked: false,
        };
        let mut import_read = false;
        let found = match import {
            Some(path) => {
                let project = self.imported_definitions(a.kind, &a.here, &a.word, &a.chain, &path);
                import_read = project.is_some();
                self.found_by_import(a, near, &path, project, &mut imported)?
            }
            None => Vec::new(),
        };
        let go_qualified = a.kind == Kind::Go
            && a.chain.len() == 1
            && bound(&near.imports, &a.chain[0]).is_some_and(|p| {
                p != ["C"] && (import_read || !self.files.iter().any(|f| search::in_package(f, &p)))
            });
        if !found.is_empty() || go_qualified {
            self.show_found(a, found);
            return None;
        }
        Some(imported)
    }

    fn found_by_import(
        &mut self,
        a: &Asked,
        near: &Near,
        path: &[String],
        project: Option<Vec<Candidate>>,
        imported: &mut Imported,
    ) -> Option<Vec<Candidate>> {
        let (kind, dotted, chain, word) = (a.kind, a.dotted, &a.chain, &a.word);
        if project.is_none()
            && kind == Kind::Python
            && !dotted
            && chain.is_empty()
            && near.imports.iter().filter(|i| i.name == *word).count() == 1
            && let Some(found) = self.bound_module(path)
        {
            self.show_found(a, found);
            return None;
        }
        if project.is_none() && kind == Kind::Python && dotted && chain.len() == 1 {
            match self.outside_class_member(word, path) {
                Some(python::OutsideClassMember::Found(found)) => {
                    self.show_found(a, found);
                    return None;
                }
                Some(python::OutsideClassMember::NoSuchClass) => {
                    imported.value = Some(path[..path.len() - 1].to_vec());
                }
                None => {}
            }
        }
        let stdlib = project.is_none()
            && kind == Kind::Python
            && dotted
            && !chain.is_empty()
            && path
                .first()
                .is_some_and(|m| search::PYTHON_STDLIB_MODULES.contains(&m.as_str()));
        let mut found = project.unwrap_or_else(|| {
            if imported.value.is_some() {
                return Vec::new();
            }
            let found = self.external_definitions(kind, word, chain, dotted, &near.imports, true);
            imported.outside = found.is_some();
            imported.linked = found.is_none();
            found.unwrap_or_default()
        });
        if stdlib
            && found.is_empty()
            && imported.value.is_none()
            && !self.python_assigned_attribute(&a.here, chain, word)
        {
            self.offer_only = false;
            self.truncated.set(false);
            self.message = format!(
                "{word}: builtin, no source (via {}: standard library)",
                path.join(".")
            );
            return None;
        }
        let others: Vec<Vec<String>> = near
            .imports
            .iter()
            .filter(|i| i.name == a.first() && i.path != path)
            .map(|i| i.path.clone())
            .collect();
        for other in others {
            let more = self
                .imported_definitions(kind, &a.here, word, chain, &other)
                .unwrap_or_default();
            found.extend(more);
        }
        Some(found)
    }

    pub(super) fn answered_by_globals(&mut self, a: &Asked, near: &Near, p: &Patterns) -> bool {
        let (kind, dotted, chain, word, locals) =
            (a.kind, a.dotted, &a.chain, &a.word, &near.locals);
        if kind == Kind::TsJs
            && locals.is_empty()
            && let [global] = chain.as_slice()
            && imported::JS_GLOBALS.contains(&global.as_str())
            && bound(&near.imports, global).is_none()
            && self
                .project_definitions(
                    kind,
                    &a.here,
                    global,
                    &search::def_patterns(kind, global).join("|"),
                )
                .is_empty()
        {
            let found = self.js_global_members(&a.here, word, p.members.as_deref(), &p.pattern);
            return self.show_found(a, found);
        }
        if kind == Kind::Rust && !dotted && chain.is_empty() && locals.is_empty() {
            let found = self.rust_glob_variant(&a.here, &a.text, word, &near.imports);
            if !found.is_empty() {
                return self.show_found(a, found);
            }
        }
        if kind == Kind::Rust && !dotted && chain.is_empty() && !a.before.ends_with("::") {
            let found = self.rust_file_items(&a.here, &a.text, word, a.range.clone());
            if !found.is_empty() {
                return self.show_found(a, found);
            }
        }
        let nested_binding = |app: &Self| {
            search::bindings(kind, &a.text, app.line + 1, a.first())
                .iter()
                .any(|b| app.buf.lines[b.line1 - 1].starts_with([' ', '\t']))
        };
        if kind == Kind::Python && locals.is_empty() && !chain.is_empty() && !nested_binding(self) {
            let found = self.class_attribute(kind, &a.here, chain, word);
            if !found.is_empty() {
                return self.show_found(a, found);
            }
        }
        let found = self.jvm_this(a, &p.pattern);
        let found = match a.here.extension() {
            Some(e) if e == "java" => {
                let call = a.tail.trim_start().starts_with('(') || a.before.ends_with("::");
                java_shaped(found, |c| &c.hit, &a.word, call)
            }
            _ => found,
        };
        if !found.is_empty() {
            return self.show_found(a, found);
        }
        false
    }

    fn jvm_this(&self, a: &Asked, pattern: &str) -> Vec<Candidate> {
        let kind = a.kind;
        if kind == Kind::Jvm
            && a.own()
            && (a.dotted || a.before.ends_with("::"))
            && let Some(owner) = search::jvm_this_owner(&a.text, self.line + 1)
        {
            let full = format!("{owner}.{}", a.word);
            let re = Regex::new(pattern).expect("an escaped name keeps the pattern valid");
            return a
                .text
                .lines()
                .enumerate()
                .filter(|&(i, l)| {
                    re.is_match(l)
                        && search::qualified(kind, &a.text, i + 1, &a.word).as_ref() == Some(&full)
                })
                .map(|(i, l)| Candidate {
                    hit: Hit {
                        deleted: None,
                        path: a.here.clone(),
                        line1: i + 1,
                        byte_col: None,
                        text: l.to_owned(),
                    },
                    reason: Reason::Path("this".to_owned()),
                })
                .collect();
        }
        Vec::new()
    }

    fn python_assigned_attribute(&self, here: &Path, chain: &[String], word: &str) -> bool {
        let Some(module) = chain.last() else {
            return false;
        };
        let pattern = format!(
            r"\b{}\.{}\s*=[^=]",
            regex::escape(module),
            regex::escape(word)
        );
        let cut = self.truncated.get();
        let assigned = !self.project_grep(Kind::Python, here, &pattern).is_empty();
        self.truncated.set(cut);
        assigned
    }
}
