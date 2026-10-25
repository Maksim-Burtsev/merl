use super::{
    App, Candidate, FileLine, HashSet, Hit, Kind, Path, PathBuf, Reason, Regex, Typed, bound,
    is_word, search,
};

impl App {
    pub(super) fn declaration(&self, kind: Kind, file: &Path, parts: &[String]) -> Option<Hit> {
        self.declared(kind, file, parts, false)
    }

    pub(super) fn type_declaration(
        &self,
        kind: Kind,
        file: &Path,
        parts: &[String],
    ) -> Option<Hit> {
        self.declared(kind, file, parts, true)
    }

    fn declared(&self, kind: Kind, file: &Path, parts: &[String], types_only: bool) -> Option<Hit> {
        let (name, chain) = parts.split_last()?;
        let interface = Regex::new(&format!(
            r"^\s*(?:export\s+)?(?:declare\s+)?interface\s+{}\b",
            regex::escape(name)
        ))
        .expect("an escaped name keeps the pattern valid");
        let merged = |hits: &[Hit]| {
            kind == Kind::TsJs
                && hits
                    .iter()
                    .all(|h| h.path == hits[0].path && interface.is_match(&h.text))
        };
        let namespace = |h: &Hit| {
            let t = h.text.trim_start().trim_start_matches("export ");
            t.trim_start_matches("declare ").starts_with("namespace ")
        };
        let beside_its_namespaces = |hits: &[Hit]| {
            kind == Kind::TsJs
                && hits.iter().all(|h| h.path == hits[0].path)
                && hits.iter().filter(|h| !namespace(h)).count() == 1
        };
        let one = |hits: Vec<Hit>| match <[Hit; 1]>::try_from(hits) {
            Ok([hit]) => Some(hit),
            Err(hits) if merged(&hits) => hits.into_iter().min_by_key(|h| h.line1),
            Err(hits) if beside_its_namespaces(&hits) => hits.into_iter().find(|h| !namespace(h)),
            Err(hits) if types_only => {
                let types = hits
                    .into_iter()
                    .filter(|h| search::declares_type(kind, &h.text));
                <[Hit; 1]>::try_from(types.collect::<Vec<_>>())
                    .ok()
                    .map(|[hit]| hit)
            }
            Err(_) => None,
        };
        if chain.is_empty() || kind == Kind::TsJs {
            let within = (!chain.is_empty()).then(|| parts.join("."));
            let hits = self.package_declarations(kind, file, name, within.as_deref());
            if !hits.is_empty() {
                return one(self.host_built(kind, hits));
            }
        }
        let text = self.text_of(file)?;
        let imports = search::imports(kind, &text);
        let Some(path) = bound(&imports, chain.first().unwrap_or(name)) else {
            return (kind == Kind::Python && chain.is_empty())
                .then(|| self.star_source(file, &text, usize::MAX, name))
                .flatten()
                .and_then(|(_, decls)| one(decls));
        };
        if kind == Kind::Python && file.is_absolute() {
            return self.outside_class(file, &path, parts);
        }
        let found = match self.imported_definitions(kind, file, name, chain, &path) {
            Some(found) => found,
            None if kind == Kind::Go && chain.len() == 1 => self.outside_declarations(&path, name),
            None if kind == Kind::Python => return self.outside_class(file, &path, parts),
            None => return None,
        };
        one(found
            .into_iter()
            .map(|c| c.hit)
            .filter(|h| self.in_code(kind, h))
            .collect())
    }

    fn outside_declarations(&self, path: &[String], name: &str) -> Vec<Candidate> {
        let kind = Kind::Go;
        let (Some((_, all)), Some(last)) = (self.external.get(&kind), path.last()) else {
            return Vec::new();
        };
        let spelled = !last.contains(|c: char| c.is_ascii_uppercase());
        let near: Vec<&PathBuf> = all
            .iter()
            .filter(|f| !spelled || f.as_os_str().to_string_lossy().contains(last.as_str()))
            .collect();
        let Some(search::ModuleFiles { files, .. }) =
            search::module_among(&near, path, Some(path.len()))
        else {
            return Vec::new();
        };
        let files: Vec<PathBuf> = files.into_iter().cloned().collect();
        let pattern = search::def_patterns(kind, name).join("|");
        let hits = self.declaring(kind, name, self.grep_in(&pattern, &files));
        let hits = hits
            .into_iter()
            .filter(|h| {
                self.text_of(&h.path)
                    .is_some_and(|t| search::qualified(kind, &t, h.line1, name).is_none())
            })
            .collect();
        self.host_built(kind, hits)
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::Import(path.join("/")),
            })
            .collect()
    }

    pub(super) fn grep_in(&self, pattern: &str, files: &[PathBuf]) -> Vec<Hit> {
        if files.first().is_some_and(|f| f.is_absolute()) {
            search::grep_project(&self.root, files, pattern, false, false, None, None)
                .unwrap_or_default()
        } else {
            self.grep(pattern, false, false, |p| files.iter().any(|f| f == p))
                .unwrap_or_default()
        }
    }

    pub(super) fn in_code(&self, kind: Kind, hit: &Hit) -> bool {
        if search::component(&hit.path) {
            self.hidden_of(kind, hit).get(hit.line1 - 1) != Some(&true)
        } else {
            (self.text_of(&hit.path)).is_some_and(|text| {
                search::literal_lines(kind, &text).get(hit.line1 - 1) != Some(&true)
            })
        }
    }

    pub(super) fn doc_tag(&self, kind: Kind, hit: &Hit) -> bool {
        let Some(text) = self.text_of(&hit.path) else {
            return false;
        };
        let lines: Vec<&str> = text.lines().collect();
        match kind {
            Kind::Php => search::php_tag_class(&lines, hit.line1 - 1).is_some(),
            Kind::TsJs => {
                search::reads_jsdoc(kind, &hit.path)
                    && (search::jsdoc_owner(&lines, hit.line1 - 1).is_some()
                        || search::jsdoc_typedef_line(lines[hit.line1 - 1])
                        || search::jsdoc_returned_literal(lines[hit.line1 - 1]).is_some())
            }
            _ => false,
        }
    }

    pub(super) fn package_declarations(
        &self,
        kind: Kind,
        file: &Path,
        name: &str,
        within: Option<&str>,
    ) -> Vec<Hit> {
        let own = self.package_files(kind, file);
        let pattern = search::def_patterns(kind, name).join("|");
        let hits = self.grep_in(&pattern, &own);
        self.declaring(kind, name, hits)
            .into_iter()
            .filter(|h| {
                self.in_code(kind, h)
                    && !(kind == Kind::Go && search::go_short_var(&h.text, name))
                    && self.text_of(&h.path).is_some_and(|text| {
                        search::qualified(kind, &text, h.line1, name).as_deref() == within
                    })
            })
            .collect()
    }

    pub(super) fn package_files(&self, kind: Kind, file: &Path) -> Vec<PathBuf> {
        if kind == Kind::Go
            && file.is_absolute()
            && let Some((_, all)) = self.external.get(&kind)
        {
            let dir = file.parent().map(Path::as_os_str);
            return all
                .iter()
                .filter(|f| f.parent().map(Path::as_os_str) == dir)
                .cloned()
                .collect();
        }
        let mut files = vec![file.to_path_buf()];
        if kind == Kind::Go {
            let test = |f: &Path| f.to_string_lossy().ends_with("_test.go");
            let package = |f: &Path| {
                self.text_of(f).and_then(|t| {
                    t.lines()
                        .find_map(|l| l.strip_prefix("package "))
                        .map(|p| p.split_whitespace().next().unwrap_or_default().to_owned())
                })
            };
            let own = test(file).then(|| package(file)).flatten();
            let external = own.as_ref().is_some_and(|p| p.ends_with("_test"));
            files.extend(
                self.files
                    .iter()
                    .filter(|f| f.parent() == file.parent() && f.as_path() != file)
                    .filter(|f| search::kind_of(f) == Some(Kind::Go) && (test(file) || !test(f)))
                    .filter(|f| {
                        if test(f) {
                            own.is_none() || package(f) == own
                        } else {
                            !external
                        }
                    })
                    .cloned(),
            );
        }
        files
    }

    pub(super) fn members_of(&self, kind: Kind, ty: &Typed, word: &str) -> Vec<Hit> {
        let Some(text) = self.text_of(&ty.path) else {
            return Vec::new();
        };
        let decl = text.lines().nth(ty.line.saturating_sub(1)).unwrap_or("");
        if kind == Kind::TsJs && search::js_constructor(decl).is_some() {
            return Vec::new();
        }
        let named = search::type_name(kind, decl).filter(|_| kind == Kind::TsJs);
        let anonymous = kind == Kind::TsJs
            && ((named.is_none() && search::declares_type(kind, decl))
                || search::ts_assigned_class(decl).is_some());
        if anonymous {
            self.anonymous_named.set(Some(ty.clone()));
        }
        let named = named.unwrap_or_else(|| ty.name.clone());
        let owner = search::qualified(kind, &text, ty.line, &named).unwrap_or(named);
        let want = Some(format!("{owner}.{word}"));
        let patterns = search::member_or_signature(kind, word).unwrap_or_default();
        let files = self.package_files(kind, &ty.path);
        let hits = self
            .grep_in(&patterns.join("|"), &files)
            .into_iter()
            .filter(|h| match self.text_of(&h.path) {
                Some(text) if anonymous => {
                    h.path == ty.path && search::owner_decl(kind, &text, h.line1) == Some(ty.line)
                }
                text => text.and_then(|text| search::qualified(kind, &text, h.line1, word)) == want,
            })
            .collect();
        self.host_built(kind, hits)
    }

    pub(super) fn host_built(&self, kind: Kind, hits: Vec<Hit>) -> Vec<Hit> {
        if kind != Kind::Go || hits.len() < 2 {
            return hits;
        }
        let built = |p: &Path| {
            self.text_of(p).map_or(search::GoBuilt::Unread, |t| {
                search::go_built(p, &t, &self.go_build)
            })
        };
        if self
            .rel_current()
            .is_some_and(|here| built(&here) == search::GoBuilt::Excluded)
        {
            return hits;
        }
        let built: Vec<search::GoBuilt> = hits.iter().map(|h| built(&h.path)).collect();
        if built.contains(&search::GoBuilt::Unread) || !built.contains(&search::GoBuilt::Compiled) {
            return hits;
        }
        hits.into_iter()
            .zip(built)
            .filter(|(_, b)| *b == search::GoBuilt::Compiled)
            .map(|(hit, _)| hit)
            .collect()
    }

    pub(super) fn field_of(
        &self,
        kind: Kind,
        ty: &Typed,
        word: &str,
        assigned: bool,
    ) -> Option<Hit> {
        let text = self.text_of(&ty.path)?;
        let decl = text.lines().nth(ty.line.saturating_sub(1)).unwrap_or("");
        if kind == Kind::TsJs && search::js_constructor(decl).is_some() {
            return None;
        }
        let search::FieldLine {
            line,
            assigned_in_method,
        } = search::field_line(kind, &text, ty.line, word)?;
        (assigned_in_method == assigned).then(|| Hit {
            path: ty.path.clone(),
            line1: line,
            byte_col: None,
            text: text.lines().nth(line - 1).unwrap_or_default().to_owned(),
            deleted: None,
        })
    }

    pub(super) fn members_by_name(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        members: &str,
    ) -> Vec<Hit> {
        let mut hits = self.project_definitions(kind, here, word, members);
        if kind == Kind::Python {
            hits.retain(|h| !self.python_nested_def(h));
        }
        let Some(fields) = search::field_patterns(kind, word) else {
            return hits;
        };
        let raw = self.project_grep(kind, here, &fields.join("|"));
        let mut by_file: Vec<(PathBuf, Vec<usize>)> = Vec::new();
        for h in raw {
            match by_file.last_mut() {
                Some((path, lines)) if *path == h.path => lines.push(h.line1),
                _ => by_file.push((h.path, vec![h.line1])),
            }
        }
        for (path, lines) in by_file {
            let Some(text) = self.text_of(&path) else {
                continue;
            };
            let jsdoc = search::reads_jsdoc(kind, &path);
            for line in search::field_rows(kind, &text, &lines, word, jsdoc) {
                if !hits.iter().any(|h| h.path == path && h.line1 == line) {
                    hits.push(Hit {
                        text: text.lines().nth(line - 1).unwrap_or_default().to_owned(),
                        path: path.clone(),
                        line1: line,
                        byte_col: None,
                        deleted: None,
                    });
                }
            }
        }
        let current = self.rel_current();
        hits.sort_by_cached_key(|h| (current.as_ref() != Some(&h.path), h.path.clone(), h.line1));
        hits
    }

    pub(super) fn python_nested_def(&self, h: &Hit) -> bool {
        let t = h.text.trim_start();
        (t.starts_with("def ") || t.starts_with("async def "))
            && self
                .hit_text(h)
                .is_some_and(|text| search::python_in_function(&text, h.line1))
    }

    pub(super) fn field_namesakes(&self, kind: Kind, here: &Path, word: &str) -> Vec<Candidate> {
        let members = search::member_patterns(kind, word)
            .unwrap_or_default()
            .join("|");
        self.members_by_name(kind, here, word, &members)
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
    }

    pub(super) fn implementations(
        &self,
        kind: Kind,
        here: &Path,
        text: &str,
        word: &str,
    ) -> Vec<Candidate> {
        if !matches!(kind, Kind::Python | Kind::TsJs | Kind::Go) {
            return Vec::new();
        }
        let line = self.line + 1;
        let declares = search::member_or_signature(kind, word)
            .and_then(|p| Regex::new(&p.join("|")).ok())
            .is_some_and(|re| re.is_match(self.line_str()));
        let declares = declares && self.on_declared_name(kind, word, true);
        let Some(owner_line) = search::owner_decl(kind, text, line).filter(|_| declares) else {
            return Vec::new();
        };
        let Some(member) = search::qualified(kind, text, line, word) else {
            return Vec::new();
        };
        let owner = Typed {
            name: member.rsplit('.').nth(1).unwrap_or_default().to_owned(),
            path: here.to_path_buf(),
            line: owner_line,
        };
        let decl = text.lines().nth(owner_line - 1).unwrap_or_default();
        let structural = match kind {
            Kind::Go => decl.contains("interface"),
            Kind::Python => is_protocol(text, owner_line),
            _ => false,
        };
        if owner.name.is_empty() || (kind == Kind::Go && !structural) {
            return Vec::new();
        }
        let hits = if structural {
            self.structural_impls(kind, here, word, text, line, owner_line)
        } else {
            self.subtype_impls(kind, here, word, &owner)
        };
        hits.into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::Implementation(member.clone()),
            })
            .collect()
    }

    fn subtype_impls(&self, kind: Kind, here: &Path, word: &str, owner: &Typed) -> Vec<Hit> {
        self.subtypes(kind, here, owner)
            .into_iter()
            .filter_map(|FileLine { path, line1 }| {
                let text = self.text_of(&path)?;
                let at = search::member_decl(kind, &text, line1, word)?;
                Some(Hit {
                    deleted: None,
                    text: text.lines().nth(at - 1).unwrap_or_default().to_owned(),
                    path,
                    line1: at,
                    byte_col: None,
                })
            })
            .collect()
    }

    pub(super) fn subclass_members(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        pattern: &str,
        ty: &Typed,
    ) -> Vec<Candidate> {
        let mut owners = self.subtypes(kind, here, ty);
        owners.push(ty.decl());
        self.members_by_name(kind, here, word, pattern)
            .into_iter()
            .filter(|h| {
                self.text_of(&h.path).is_some_and(|t| {
                    let lines: Vec<&str> = t.lines().collect();
                    search::enclosing_type(kind, &lines, h.line1 - 1)
                        .is_some_and(|d| owners.iter().any(|o| o.path == h.path && o.line1 == d))
                        || kind == Kind::TsJs && search::prototype_mixed(&t, h.line1, &ty.name)
                })
            })
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
    }

    pub(super) fn subtypes(&self, kind: Kind, here: &Path, owner: &Typed) -> Vec<FileLine> {
        let mut names = vec![owner.name.clone()];
        let mut types = vec![(owner.name.clone(), owner.path.clone())];
        let mut seen = vec![(owner.path.clone(), owner.line)];
        let mut out = Vec::new();
        for _ in 0..4 {
            let Some(pattern) = search::subtype_patterns(kind, &names) else {
                break;
            };
            let hits = self
                .grep(&pattern, false, false, |p| self.in_def_scope(kind, here, p))
                .unwrap_or_default();
            let mut next = Vec::new();
            let mut found = Vec::new();
            for hit in hits {
                let Some(text) = self.text_of(&hit.path) else {
                    continue;
                };
                let Some(decl) = search::type_decl_at(kind, &text, hit.line1) else {
                    continue;
                };
                let name = text
                    .lines()
                    .nth(decl - 1)
                    .and_then(|l| search::type_name(kind, l));
                let Some(name) = name else {
                    continue;
                };
                if seen.contains(&(hit.path.clone(), decl)) {
                    continue;
                }
                let declares = |text: &str, name: &str| {
                    text.lines()
                        .any(|l| search::type_name(kind, l).as_deref() == Some(name))
                };
                let another = |first: &str, module: &[String]| {
                    let module = match kind {
                        Kind::TsJs => &module[..module.len().saturating_sub(1)],
                        _ => module,
                    };
                    let shortest = match kind {
                        Kind::TsJs => module.len(),
                        _ => 1,
                    };
                    (shortest..=module.len()).rev().any(|n| {
                        let (files, cut) =
                            self.behind_barrels(kind, &hit.path, &module[..n], first, &[]);
                        let ours = |f: &PathBuf| types.iter().any(|(t, p)| t == first && p == f);
                        !files.iter().any(ours)
                            && (cut
                                || files
                                    .iter()
                                    .any(|f| self.text_of(f).is_some_and(|t| declares(&t, first))))
                    })
                };
                let sees = |first: &str| {
                    if declares(&text, first) {
                        return !types.iter().any(|(t, _)| t == first)
                            || types.iter().any(|(t, p)| t == first && *p == hit.path);
                    }
                    bound(&search::imports(kind, &text), first)
                        .is_some_and(|module| !another(first, &module))
                };
                let derives = search::bases(kind, &text, decl)
                    .into_iter()
                    .chain(search::interfaces(kind, &text, decl))
                    .filter_map(|b| search::type_path(kind, &b))
                    .any(|p| p.last().is_some_and(|n| names.contains(n)) && sees(&p[0]));
                if !derives {
                    continue;
                }
                seen.push((hit.path.clone(), decl));
                found.push((name.clone(), hit.path.clone()));
                next.push(name);
                out.push(FileLine {
                    path: hit.path,
                    line1: decl,
                });
            }
            if next.is_empty() || out.len() >= search::MAX_HITS {
                break;
            }
            names = next;
            types = found;
        }
        out
    }

    fn behind_barrels(
        &self,
        kind: Kind,
        from: &Path,
        module: &[String],
        name: &str,
        chain: &[PathBuf],
    ) -> (Vec<PathBuf>, bool) {
        let files = search::module_files(kind, &self.root, &self.files, from, module);
        if kind != Kind::TsJs {
            return (files, false);
        }
        let (mut out, mut cut) = (Vec::new(), false);
        for f in files {
            let modules = self
                .text_of(&f)
                .filter(|t| {
                    !t.lines()
                        .any(|l| search::type_name(kind, l).as_deref() == Some(name))
                })
                .map(|t| search::reexports(&t, name))
                .unwrap_or_default();
            if modules.is_empty() || chain.contains(&f) {
                out.push(f);
                continue;
            }
            if chain.len() == 4 {
                cut = true;
                continue;
            }
            let chain = [chain, std::slice::from_ref(&f)].concat();
            let mut behind = Vec::new();
            for m in &modules {
                let (files, deeper) = self.behind_barrels(kind, &f, m, name, &chain);
                behind.extend(files);
                cut |= deeper;
            }
            if behind.is_empty() {
                out.push(f);
            } else {
                out.extend(behind);
            }
        }
        (out, cut)
    }

    fn structural_impls(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        text: &str,
        line: usize,
        owner_line: usize,
    ) -> Vec<Hit> {
        let want = search::params(kind, text, line);
        let signature = search::go_signature(text, line);
        let patterns = match kind {
            Kind::Go => search::member_patterns(kind, word),
            _ => search::member_or_signature(kind, word),
        };
        let (Some(want), Some(patterns)) = (want, patterns) else {
            return Vec::new();
        };
        self.project_definitions(kind, here, word, &patterns.join("|"))
            .into_iter()
            .filter(|h| h.line1 != line || h.path != here)
            .filter(|h| {
                let Some(text) = self.text_of(&h.path) else {
                    return false;
                };
                if search::params(kind, &text, h.line1) != Some(want) {
                    return false;
                }
                if kind == Kind::Go
                    && let (Some(a), Some(b)) = (&signature, search::go_signature(&text, h.line1))
                    && *a != b
                {
                    return false;
                }
                kind != Kind::Python
                    || search::owner_decl(kind, &text, h.line1)
                        .filter(|&d| d != owner_line || h.path != here)
                        .is_some_and(|d| !is_protocol(&text, d))
            })
            .collect()
    }

    pub(super) fn own_member(&self, kind: Kind, t: &Typed, word: &str) -> Option<Vec<Hit>> {
        let members = self.members_of(kind, t, word);
        if members.is_empty() {
            self.field_of(kind, t, word, false).map(|hit| vec![hit])
        } else {
            Some(members)
        }
    }

    fn constructor_name(&self, t: &Typed) -> Option<String> {
        let text = self.text_of(&t.path)?;
        search::js_constructor(text.lines().nth(t.line.saturating_sub(1))?).map(str::to_owned)
    }

    fn constructed_member(&self, t: &Typed, name: &str, word: &str) -> Option<Vec<Hit>> {
        let text = self.text_of(&t.path)?;
        let hit = |line1: usize| Hit {
            text: text.lines().nth(line1 - 1).unwrap_or_default().to_owned(),
            path: t.path.clone(),
            line1,
            byte_col: None,
            deleted: None,
        };
        if let Some(line1) = search::constructor_field(&text, t.line, word) {
            return Some(vec![hit(line1)]);
        }
        let methods = search::prototype_members(&text, name, word);
        (!methods.is_empty()).then(|| methods.into_iter().map(hit).collect())
    }

    pub(super) fn instance_member(&self, kind: Kind, t: &Typed, word: &str) -> Option<Vec<Hit>> {
        if kind != Kind::TsJs {
            return self.own_member(kind, t, word);
        }
        if let Some(name) = self.constructor_name(t) {
            return self.constructed_member(t, &name, word);
        }
        let mut members = self.members_of(kind, t, word);
        let field = self.field_of(kind, t, word, false);
        if members.is_empty() {
            return field
                .map(|hit| vec![hit])
                .or_else(|| self.alias_literal_member(t, word));
        }
        let Some(field) = field.filter(|f| {
            !ts_modifiers(&f.text)
                .iter()
                .any(|m| matches!(*m, "static" | "get" | "set"))
                && members.iter().all(|m| {
                    (m.path != f.path || m.line1 != f.line1)
                        && !ts_modifiers(&m.text).contains(&"static")
                })
        }) else {
            return Some(members);
        };
        if members
            .iter()
            .all(|m| m.path == field.path && m.line1 > field.line1)
        {
            return Some(vec![field]);
        }
        members.push(field);
        Some(members)
    }

    fn alias_literal_member(&self, t: &Typed, word: &str) -> Option<Vec<Hit>> {
        let text = self.text_of(&t.path)?;
        let decl = text.lines().nth(t.line.checked_sub(1)?)?;
        search::ts_alias_literal_member(decl, word).then(|| {
            vec![Hit {
                text: decl.to_owned(),
                path: t.path.clone(),
                line1: t.line,
                byte_col: None,
                deleted: None,
            }]
        })
    }

    pub(super) fn above(
        &self,
        kind: Kind,
        ty: &Typed,
        word: &str,
        depth: usize,
    ) -> Result<Vec<Hit>, String> {
        let broke = || "super".to_owned();
        let find = |t: &Typed| self.own_member(kind, t, word);
        let text = self
            .text_of(&ty.path)
            .filter(|_| depth < 8)
            .ok_or_else(broke)?;
        let plain = |b: &str| {
            let b = b.split('[').next().unwrap_or(b);
            let b = b.rsplit('.').next().unwrap_or(b);
            matches!(b, "Generic" | "Protocol" | "ABC" | "object")
        };
        let written = search::bases(kind, &text, ty.line);
        let bases: Vec<&String> = written.iter().filter(|b| !plain(b)).collect();
        let mut answers: Vec<Vec<Hit>> = Vec::new();
        for (i, base) in bases.iter().enumerate() {
            let Some(base) = self.type_decl(kind, &ty.path, base) else {
                return Err(broke());
            };
            let hits = match find(&base) {
                Some(own) if i == 0 => return Ok(own),
                Some(own) => own,
                None => self.above(kind, &base, word, depth + 1)?,
            };
            let lines = |hits: &[Hit]| -> Vec<(PathBuf, usize)> {
                hits.iter().map(|h| (h.path.clone(), h.line1)).collect()
            };
            if !hits.is_empty() && answers.iter().all(|a| lines(a) != lines(&hits)) {
                answers.push(hits);
            }
        }
        match answers.len() {
            0 | 1 => Ok(answers.pop().unwrap_or_default()),
            _ => Err(broke()),
        }
    }

    pub(super) fn hierarchy<R>(
        &self,
        kind: Kind,
        ty: &Typed,
        depth: usize,
        find: &mut dyn FnMut(&Typed) -> Option<R>,
    ) -> Option<R> {
        if kind == Kind::Go {
            return self.go_promoted(ty, find);
        }
        if let Some(found) = find(ty) {
            return Some(found);
        }
        if depth == 8 {
            return None;
        }
        let text = self.text_of(&ty.path)?;
        search::bases(kind, &text, ty.line)
            .iter()
            .filter_map(|base| self.type_decl(kind, &ty.path, base))
            .find_map(|base| self.hierarchy(kind, &base, depth + 1, &mut *find))
    }

    fn go_promoted<R>(&self, ty: &Typed, find: &mut dyn FnMut(&Typed) -> Option<R>) -> Option<R> {
        let mut level = vec![ty.clone()];
        for _ in 0..=8 {
            let mut found = level.iter().filter_map(&mut *find);
            if let Some(first) = found.next() {
                return found.next().is_none().then_some(first);
            }
            level = level
                .iter()
                .flat_map(|t| {
                    let text = self.text_of(&t.path).unwrap_or_default();
                    search::bases(Kind::Go, &text, t.line)
                        .iter()
                        .filter_map(|base| self.type_decl(Kind::Go, &t.path, base))
                        .collect::<Vec<_>>()
                })
                .collect();
            let mut seen = HashSet::new();
            level.retain(|t| seen.insert((t.path.clone(), t.line)));
        }
        None
    }

    pub(super) fn on_declared_name(&self, kind: Kind, word: &str, exact: bool) -> bool {
        let line = self.line_str();
        let Some((r, _)) = search::definition_word(Some(kind), line, self.col) else {
            return true;
        };
        if kind == Kind::Shell
            && (line.get(..r.start)).is_some_and(|b| b.ends_with('$') || b.ends_with("${"))
        {
            return false;
        }
        let part = |c: char| is_word(c) || search::word_chars(Some(kind), true).contains(c);
        let around = |i: usize| Some((line.get(..i)?, line.get(i + word.len()..)?));
        let at: Vec<usize> = line
            .match_indices(word)
            .map(|(i, _)| i)
            .filter(|&i| around(i).is_some_and(|(h, t)| !h.ends_with(part) && !t.starts_with(part)))
            .collect();
        if !at.contains(&r.start) {
            return true;
        }
        let mut patterns = search::def_patterns(kind, word);
        patterns.extend(search::member_or_signature(kind, word).unwrap_or_default());
        let re = Regex::new(&patterns.join("|"))
            .ok()
            .filter(|re| re.is_match(line));
        let same_shape_word: String = word
            .chars()
            .map(|c| match c {
                'q' => 'z',
                'Q' => 'Z',
                '0' => '1',
                c if c.is_lowercase() => 'q',
                c if c.is_uppercase() => 'Q',
                c if c.is_numeric() => '0',
                c => c,
            })
            .collect();
        let declared: Vec<usize> = at
            .iter()
            .copied()
            .filter(|&i| {
                re.as_ref()
                    .zip(around(i))
                    .is_some_and(|(re, (h, t))| !re.is_match(&format!("{h}{same_shape_word}{t}")))
            })
            .collect();
        let first = declared.first().copied().unwrap_or(at[0]);
        if declared.contains(&r.start) || (declared.is_empty() && first == r.start) {
            return true;
        }
        if exact {
            return false;
        }
        let called = |i: usize| around(i).is_some_and(|(_, t)| t.trim_start().starts_with('('));
        let Some((before, _)) = around(r.start) else {
            return true;
        };
        let before = before.trim_end();
        let member = before.ends_with(['.', '>']) || before.ends_with("::");
        !(r.start < first || member || (called(r.start) && !called(first)))
    }
}

fn ts_modifiers(line: &str) -> Vec<&str> {
    let words: Vec<&str> = line.split_whitespace().collect();
    words
        .windows(2)
        .take_while(|w| {
            matches!(
                w[0],
                "public"
                    | "private"
                    | "protected"
                    | "readonly"
                    | "static"
                    | "declare"
                    | "override"
                    | "abstract"
                    | "accessor"
                    | "async"
                    | "get"
                    | "set"
            ) && !w[1].starts_with(['=', ';', ':', '(', '?', '!'])
        })
        .map(|w| w[0])
        .collect()
}

fn is_protocol(text: &str, decl1: usize) -> bool {
    search::bases(Kind::Python, text, decl1)
        .iter()
        .filter_map(|b| search::type_path(Kind::Python, b))
        .any(|p| p.last().is_some_and(|n| n == "Protocol"))
}
