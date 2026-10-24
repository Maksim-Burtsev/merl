use super::{App, Candidate, Hit, Kind, Path, PathBuf, Reason, Regex, Typed, bound, search};

pub(super) enum OutsideClassMember {
    Found(Vec<Candidate>),
    NoSuchClass,
}

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
    ) -> Option<OutsideClassMember> {
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
        let imports = search::imports(kind, &text);
        let class = text
            .lines()
            .any(|l| search::type_name(kind, l).as_deref() == Some(name))
            || imports.iter().any(|i| i.name == *name || i.name == "*");
        if !class {
            return Some(OutsideClassMember::NoSuchClass);
        }
        let mut patterns = search::def_patterns(kind, word);
        patterns.extend(search::field_patterns(kind, word).unwrap_or_default());
        let within = Some(format!("{name}.{word}"));
        let hits = self.external_grep(kind, std::slice::from_ref(&file), &patterns.join("|"));
        let mut hits: Vec<Hit> = (hits.into_iter())
            .filter(|h| search::qualified(kind, &text, h.line1, word) == within)
            .collect();
        if hits.is_empty()
            && let Some(class) = self.outside_class(Path::new(""), path, std::slice::from_ref(name))
            && let Some(declared) = search::type_name(kind, &class.text)
        {
            let ty = Typed {
                name: declared,
                path: class.path,
                line: class.line1,
            };
            hits = self.above(kind, &ty, word, 0).unwrap_or_default();
        }
        let reason = Reason::Import(module.join("."));
        Some(OutsideClassMember::Found(
            hits.into_iter()
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
        let Some(search::ModuleFiles { files, .. }) = self
            .python_module_among(&all, module)
            .filter(|m| m.matched_parts == module.len())
        else {
            return Vec::new();
        };
        let pattern = search::def_patterns(kind, word).join("|");
        let reason = Reason::Import(module.join("."));
        self.external_grep(kind, &files, &pattern)
            .into_iter()
            .filter(|h| starts_by_assigning(&h.text, word))
            .map(|hit| Candidate {
                hit,
                reason: reason.clone(),
            })
            .collect()
    }

    pub(super) fn block_assignments(&self, here: &Path, word: &str) -> Vec<Candidate> {
        let kind = Kind::Python;
        let pattern = format!(r"^\s+{}\s*(:[^=]*)?=", regex::escape(word));
        let opens_block = |l: &str| {
            let keyword = l.split(|c: char| !(c.is_alphanumeric() || c == '_')).next();
            keyword.is_some_and(|k| ["if", "elif", "else", "try", "except", "finally"].contains(&k))
                && l.contains(':')
                && !l.contains("__name__")
        };
        self.project_grep(kind, here, &pattern)
            .into_iter()
            .filter(|h| {
                starts_by_assigning(h.text.trim_start(), word)
                    && self.text_of(&h.path).is_some_and(|text| {
                        let literal = search::file_literal_lines(kind, &h.path, &text);
                        !literal.get(h.line1 - 1).copied().unwrap_or(false)
                            && search::owner_line(&text, h.line1).is_some_and(opens_block)
                    })
            })
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
    }

    pub(super) fn star_imported(
        &mut self,
        word: &str,
        imports: &[search::Import],
    ) -> Vec<Candidate> {
        let kind = Kind::Python;
        let pattern = search::def_patterns(kind, word).join("|");
        let mut found = Vec::new();
        for search::Import { path, .. } in imports.iter().filter(|i| i.name == "*") {
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
                    .filter(|h| search::qualified(kind, &text, h.line1, word).is_none())
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
        let hits = self.module_level_defs(&file, &text, name);
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
                .get(b.line1 - 1)
                .is_some_and(|l| import(l.trim_start()))
        });
        if other || depth >= 4 {
            return Vec::new();
        }
        let mut found: Vec<Candidate> = Vec::new();
        for search::Import {
            name: bound,
            mut path,
        } in search::imports_as_written(kind, &search::python_module_level(&text))
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
                            line1: line,
                            byte_col: None,
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
                    .any(|o| (&o.hit.path, o.hit.line1) == (&c.hit.path, c.hit.line1))
                {
                    found.push(c);
                }
            }
        }
        found
    }

    fn module_level_defs(&self, file: &Path, text: &str, name: &str) -> Vec<Hit> {
        let kind = Kind::Python;
        let pattern = search::def_patterns(kind, name).join("|");
        let hits = self.external_grep(kind, &[file.to_path_buf()], &pattern);
        (self.declaring(kind, name, hits).into_iter())
            .filter(|h| search::qualified(kind, text, h.line1, name).is_none())
            .collect()
    }

    pub(super) fn outside_callee(&self, file: &Path, text: &str, callee: &str) -> Option<Hit> {
        let mut parts = callee.split('.').map(str::to_owned);
        let mut path = bound(&search::imports(Kind::Python, text), &parts.next()?)?;
        path.extend(parts);
        let (name, module) = path.split_last()?;
        if module.first().is_none_or(|m| m.starts_with('.'))
            || !self.project_module(file, module).is_empty()
        {
            return None;
        }
        let returns = |hit: &Hit| {
            let text = self.text_of(&hit.path)?;
            self.returns_of(Kind::Python, &hit.path, &text, hit.line1)
        };
        if let [one] = self.outside_handed_on(module, name, 0).as_slice()
            && let Some(search::Value::Type(_) | search::Value::New(_)) = returns(&one.hit)
        {
            return Some(one.hit.clone());
        }
        let dir = crate::typeshed::stdlib()?;
        let base: PathBuf = module.iter().collect();
        let stub = [base.join("__init__.pyi"), base.with_extension("pyi")]
            .into_iter()
            .map(|f| dir.join(f))
            .find(|f| f.is_file())?;
        let text = std::fs::read_to_string(&stub).ok()?;
        let [hit] = <[Hit; 1]>::try_from(self.module_level_defs(&stub, &text, name)).ok()?;
        Some(hit)
    }

    pub(super) fn runtime_class(&self, ty: Typed) -> Typed {
        let Some(rel) = crate::typeshed::within(&ty.path) else {
            return ty;
        };
        let mut path: Vec<String> = (rel.with_extension("").components())
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        if path.last().is_some_and(|l| l == "__init__") {
            path.pop();
        }
        path.push(ty.name.clone());
        self.outside_class(Path::new(""), &path, std::slice::from_ref(&ty.name))
            .and_then(|class| {
                Some(Typed {
                    name: search::type_name(Kind::Python, &class.text)?,
                    path: class.path,
                    line: class.line1,
                })
            })
            .unwrap_or(ty)
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

    pub(super) fn star_winner(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        found: &mut Vec<Candidate>,
    ) {
        if found.iter().any(|c| c.hit.path == here) {
            return;
        }
        if let Some((module, decls)) = self.star_source(here, text, self.line, word) {
            *found = decls
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Import(module.join(".")),
                })
                .collect();
        }
    }

    pub(super) fn star_source(
        &self,
        here: &Path,
        text: &str,
        line0: usize,
        word: &str,
    ) -> Option<(Vec<String>, Vec<Hit>)> {
        static STAR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"^(\s*)from\s+[\w.]+\s+import\s+\*").expect("the regex is a literal")
        });
        if word.starts_with('_') {
            return None;
        }
        let literal = search::literal_lines(Kind::Python, text);
        let mut stars = Vec::new();
        for (i, l) in text.lines().enumerate() {
            let Some(c) = STAR
                .captures(l)
                .filter(|_| !literal.get(i).copied().unwrap_or(false))
            else {
                continue;
            };
            if !c[1].is_empty() || i >= line0 {
                return None;
            }
            stars.extend(search::imports(Kind::Python, l));
        }
        let mentions = Regex::new(&format!(r"\b{}\b|import\s+\*", regex::escape(word)))
            .expect("an escaped name keeps the pattern valid");
        for search::Import { path, .. } in stars.iter().rev() {
            let module = &path[..path.len() - 1];
            let files = self.project_module(here, module);
            let [file] = files.as_slice() else {
                return None;
            };
            let module_text = self.text_of(file).filter(|t| !t.contains("__all__"))?;
            let decls = self.package_declarations(Kind::Python, file, word, None);
            if !decls.is_empty() {
                return Some((module.to_vec(), decls));
            }
            if mentions.is_match(&module_text) {
                return None;
            }
        }
        None
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
            || (path == here && found.iter().any(|c| c.hit.line1 == self.line + 1))
        {
            return;
        }
        let Some(text) = self.hit_text(&first.hit) else {
            return;
        };
        let name = |c: &Candidate| {
            let t = c.hit.text.trim_start();
            (t.starts_with("def ") || t.starts_with("async def "))
                .then(|| search::qualified(Kind::Python, &text, c.hit.line1, word))
        };
        let one = name(first);
        if one.is_none() || found.iter().any(|c| name(c) != one) {
            return;
        }
        let plain = |c: &Candidate| !search::python_overload(&text, c.hit.line1);
        if found.iter().filter(|c| plain(c)).count() == 1 {
            found.retain(plain);
        }
    }

    fn python_mro(&self, ty: &Typed) -> Vec<Typed> {
        let mut walk = Vec::new();
        self.hierarchy(Kind::Python, ty, 0, &mut |t| {
            walk.push(t.clone());
            None::<()>
        });
        let at = |t: &Typed| (t.path.clone(), t.line);
        (0..walk.len())
            .filter(|&i| walk[i + 1..].iter().all(|u| at(u) != at(&walk[i])))
            .map(|i| walk[i].clone())
            .collect()
    }

    pub(super) fn made_by_class_attribute(&self, ty: &Typed, field: &str) -> Option<Typed> {
        let kind = Kind::Python;
        let mro = self.python_mro(ty);
        let nearest = |name: &str| {
            mro.iter().find_map(|t| {
                let text = self.text_of(&t.path)?;
                let found = search::field_bindings(kind, &text, t.line, name);
                (!found.is_empty()).then(|| (t.path.clone(), found))
            })
        };
        let (_, made) = nearest(field)?;
        let [
            search::Binding {
                value: search::Value::Call(callee),
                ..
            },
        ] = made.as_slice()
        else {
            return None;
        };
        let attribute = (callee
            .strip_prefix("cls.")
            .or_else(|| callee.strip_prefix("self.")))
        .filter(|a| !a.contains('.'))?;
        let (at, held) = nearest(attribute)?;
        let [
            search::Binding {
                value: search::Value::Name(class),
                ..
            },
        ] = held.as_slice()
        else {
            return None;
        };
        self.type_decl(kind, &at, class)
    }
}

pub(super) fn enclosing_def_lines1(text: &str, line1: usize) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let literal = search::literal_lines(Kind::Python, text);
    let indent = |l: &str| l.len() - l.trim_start().len();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
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
        Regex::new(r"(?m)^from\s+[\w.]+\s+import\s+(\([^)]*\)|[^\n]+)")
            .expect("the regex is a literal")
    });
    let word = Regex::new(&format!(r"\b{}\b", regex::escape(name))).ok()?;
    FROM.captures_iter(text).find_map(|c| {
        let names = c.get(1)?;
        let at = word.find(names.as_str())?;
        Some(
            text.get(..names.start() + at.start())?
                .matches('\n')
                .count()
                + 1,
        )
    })
}
pub(super) fn starts_by_assigning(text: &str, word: &str) -> bool {
    text.strip_prefix(word).is_some_and(|rest| {
        let rest = rest.trim_start();
        rest.starts_with(':') || rest.starts_with('=') && !rest.starts_with("==")
    })
}
