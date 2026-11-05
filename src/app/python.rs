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

    pub(super) fn outside_instance_member(
        &self,
        module: &[String],
        value: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let kind = Kind::Python;
        let made = self.outside_handed_on(module, value, 0);
        let [made] = made.as_slice() else {
            return None;
        };
        let (name, _) = made.hit.text.split_once('=')?;
        let name = name.trim_end();
        if name.is_empty() || !name.chars().all(super::is_word) {
            return None;
        }
        let text = self.text_of(&made.hit.path)?;
        let bound = search::bindings(kind, &text, 1, name);
        let [
            search::Binding {
                line1,
                value: search::Value::Call(class),
            },
        ] = bound.as_slice()
        else {
            return None;
        };
        let once = |n: &str| {
            let binds = module_binds(n);
            text.lines().filter(|l| binds.is_match(l)).count() == 1
        };
        if *line1 != made.hit.line1 || class.contains('.') || !once(name) || !once(class) {
            return None;
        }
        let ty = self.type_decl(kind, &made.hit.path, class)?;
        let above: Vec<&str> = text.lines().take(ty.line.saturating_sub(1)).collect();
        let decorated = (above.iter().rev())
            .map(|l| l.trim())
            .take_while(|l| l.starts_with('@'))
            .any(|l| !matches!(l, "@final" | "@typing.final"));
        if ty.path != made.hit.path
            || decorated
            || !search::python_rooted(&text, ty.line)
            || DYNAMIC_STORE.is_match(&text)
        {
            return None;
        }
        let start = self.walked_module(module)?;
        let package = self.python_package_files(&[&made.hit.path, &start]);
        let (m, v) = (regex::escape(module.last()?), regex::escape(value));
        let stores = format!(
            r#"{}|\b{m}\s*\.\s*{v}{STORE}|setattr\s*\(\s*{m}\s*,\s*["']{v}["']"#,
            stores_of(&[name, word])
        );
        let in_project = self.grep(&stores, false, false, |p| {
            p.extension().is_some_and(|e| e == "py")
        });
        if !self.grep_in(&stores, &package).is_empty() || !in_project.ok()?.is_empty() {
            return None;
        }
        let hits = match self.own_member(kind, &ty, word) {
            Some(hits) if !hits.is_empty() => hits,
            _ => self.type_checking_member(&ty, word)?,
        };
        let reason = Reason::Receiver(format!("{}.{value}: {}", module.join("."), ty.name));
        (!hits.is_empty()).then(|| {
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect()
        })
    }

    fn type_checking_member(&self, ty: &Typed, word: &str) -> Option<Vec<Hit>> {
        let text = self.text_of(&ty.path)?;
        let lines: Vec<&str> = text.lines().collect();
        let hits: Vec<Hit> = search::type_checking_fields(&text, ty.line, word)
            .into_iter()
            .filter_map(|b| {
                Some(Hit {
                    text: (*lines.get(b.line1 - 1)?).to_owned(),
                    path: ty.path.clone(),
                    line1: b.line1,
                    byte_col: None,
                    deleted: None,
                })
            })
            .collect();
        (!hits.is_empty()).then_some(hits)
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

    pub(super) fn stub_returns_self(&self, ty: &Typed, field: &str) -> bool {
        let rel = self
            .rel_to_its_root(Kind::Python, &ty.path)
            .with_extension("");
        let Some(dir) = crate::typeshed::stdlib() else {
            return false;
        };
        let stdlib = ty
            .path
            .ancestors()
            .nth(rel.components().count())
            .is_some_and(|root| {
                root.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("python"))
                    && root.join("os.py").is_file()
            });
        if !ty.path.is_absolute() || rel.is_absolute() || !stdlib {
            return false;
        }
        let Some((stub, text)) = [rel.with_extension("pyi"), rel.join("__init__.pyi")]
            .into_iter()
            .find_map(|f| Some((dir.join(&f), std::fs::read_to_string(dir.join(f)).ok()?)))
        else {
            return false;
        };
        let defs = self.module_level_defs(&stub, &text, &ty.name);
        let [class] = &defs[..] else {
            return false;
        };
        if !search::declares_type(Kind::Python, &class.text) {
            return false;
        }
        let class = Typed {
            name: ty.name.clone(),
            path: stub,
            line: class.line1,
        };
        let says_self = self.hierarchy(Kind::Python, &class, 0, &mut |t| {
            let text = self.text_of(&t.path)?;
            let found = search::field_bindings(Kind::Python, &text, t.line, field);
            let self_type = search::imports(Kind::Python, &text).iter().any(|i| {
                i.name == "Self"
                    && matches!(&i.path[..], [m, s] if s == "Self" && (m == "typing" || m == "typing_extensions"))
            });
            (!found.is_empty()).then(|| {
                self_type
                    && found
                        .iter()
                        .all(|b| matches!(&b.value, search::Value::Type(s) if s == "Self"))
            })
        });
        says_self == Some(true)
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

    pub(super) fn created_model_field(
        &self,
        here: &Path,
        line0: usize,
        byte_col: usize,
        word: &str,
    ) -> Option<Candidate> {
        let kind = Kind::Python;
        let head = self
            .buf
            .lines
            .get(line0)?
            .get(..=byte_col)?
            .strip_suffix(".objects.create")?;
        let model = head
            .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
            .next()
            .filter(|m| {
                !m.is_empty()
                    && head
                        .strip_suffix(m)
                        .is_some_and(|before| !before.ends_with('.'))
            })?;
        let ty = self.type_decl(kind, here, model)?;
        let mro = self.python_mro(&ty);
        mro.iter()
            .any(|t| {
                self.text_of(&t.path).is_some_and(|text| {
                    search::bases(kind, &text, t.line)
                        .iter()
                        .any(|b| b == "models.Model" || b.ends_with(".models.Model"))
                })
            })
            .then_some(())?;
        let (class, text, made) = mro.into_iter().find_map(|t| {
            let text = self.text_of(&t.path)?;
            let made = search::field_bindings(kind, &text, t.line, word);
            (!made.is_empty()).then_some((t, text, made))
        })?;
        let [
            search::Binding {
                line1,
                value: search::Value::Call(_),
            },
        ] = made.as_slice()
        else {
            return None;
        };
        let line = text.lines().nth(line1 - 1)?;
        (!class.path.is_absolute() && starts_by_assigning(line.trim_start(), word)).then(|| {
            Candidate {
                hit: Hit {
                    deleted: None,
                    path: class.path.clone(),
                    line1: *line1,
                    byte_col: None,
                    text: line.to_owned(),
                },
                reason: Reason::Label(format!("field of {}", class.name)),
            }
        })
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
const STORE: &str = r"\s*(?:[-+*/%@|&^]|//|\*\*|<<|>>)?=(?:[^=]|$)";

static DYNAMIC_STORE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"__dict__\s*\.\s*update|setattr\s*\(\s*(?:self|cls)\s*,\s*[^"'\s]"#)
        .expect("the regex is a literal")
});

fn module_binds(name: &str) -> Regex {
    let n = regex::escape(name);
    Regex::new(&format!(
        r"^\s*(?:(?:global|del|case)\b.*\b{n}\b|(?:async\s+)?(?:def|class)\s+{n}\b|for\b.*\b{n}\b.*\bin\b|[\w\s,.*()\[\]]*\b{n}\b[\w\s,.*()\[\]]*{STORE})|\bimport\b.*\b{n}\b|\bimport\s+\*|\bas\s+{n}\b|\b{n}\s*:=|\bsetattr\s*\(|\bglobals\s*\(\s*\)|\bvars\s*\(\s*\)|\bsys\.modules\b"
    ))
    .expect("an escaped name keeps the pattern valid")
}

fn stores_of(names: &[&str]) -> String {
    let n = (names.iter().map(|n| regex::escape(n)))
        .collect::<Vec<_>>()
        .join("|");
    format!(
        r#"\.\s*(?:{n})\b{STORE}|setattr\s*\(.*["'](?:{n})["']|\[\s*["'](?:{n})["']\s*\]{STORE}|\b(?:{n})\s*\.\s*__(?:dict|setattr)__"#
    )
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

impl App {
    pub(super) fn patched_mock(
        &self,
        kind: Kind,
        text: &str,
        b: &search::Binding,
    ) -> Option<Typed> {
        if kind != Kind::Python
            || !matches!(b.value, search::Value::Call(_) | search::Value::Unknown)
        {
            return None;
        }
        let line1 = b.line1;
        let imports = search::imports(Kind::Python, text);
        let unittest = |callee: &str| {
            let mut parts = callee.split('.').map(str::to_owned);
            (parts.next().and_then(|head| bound(&imports, &head))).is_some_and(|mut path| {
                path.extend(parts);
                path.starts_with(&["unittest".into(), "mock".into(), "patch".into()])
            })
        };
        let from_mock = match search::python_patched_mock(text, line1) {
            Some((callee, true)) => unittest(&callee),
            Some((callee, false)) => {
                let parts: Vec<&str> = callee.split('.').collect();
                bound(&imports, parts[0]).is_none()
                    && parts.get(1) == Some(&"patch")
                    && matches!(search::bindings(Kind::Python, text, line1, parts[0]).as_slice(),
                        [search::Binding { value: search::Value::Type(t), .. }]
                            if t.trim_end().ends_with("MockerFixture"))
            }
            None => {
                let (range, _) = self.word_here(Some(Kind::Python))?;
                let before = self.line_str().get(..range.start)?.strip_suffix('.')?;
                let receiver = before
                    .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .next()?;
                search::python_patched_param(text, line1, receiver)
                    .is_some_and(|patches| patches.iter().all(|c| unittest(c)))
            }
        };
        if !from_mock {
            return None;
        }
        let [magic, other] = ["MagicMock", "AsyncMock"].map(|name| {
            let path = ["unittest", "mock", name].map(str::to_owned);
            let class = self.outside_class(Path::new(""), &path, &path[2..])?;
            Some(Typed {
                name: search::type_name(Kind::Python, &class.text)?,
                path: class.path,
                line: class.line1,
            })
        });
        let (magic, other) = (magic?, other?);
        let word = self.word_here(Some(Kind::Python))?.1.to_owned();
        let declares = |ty: &Typed| -> Vec<(PathBuf, usize)> {
            let kind = Kind::Python;
            let declared =
                self.hierarchy(kind, ty, 0, &mut |t| self.instance_member(kind, t, &word));
            let hits = declared.or_else(|| {
                let mut assigned = None;
                self.hierarchy(kind, ty, 0, &mut |t| {
                    if let Some(hit) = self.field_of(kind, t, &word, true) {
                        assigned = Some(vec![hit]);
                    }
                    None::<()>
                });
                assigned
            });
            (hits.unwrap_or_default().into_iter())
                .map(|h| (h.path, h.line1))
                .collect()
        };
        let found = declares(&magic);
        (!found.is_empty() && found == declares(&other)).then_some(magic)
    }
}
